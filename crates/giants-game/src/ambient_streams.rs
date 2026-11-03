//! Stream-only slice of state58. Device admission is an explicit service.
//! Sampled sound IDs and live scheduling remain outside this profile.
use crate::{arithmetic::Precision, audio_parameters::Random};
#[derive(Clone)]
pub struct Config {
    pub beds: Vec<String>,
    pub accents: Vec<String>,
    pub delays: [f32; 2],
    pub region: Option<[f32; 2]>,
}
impl Config {
    /// Legacy Bevy caller remains global-only until its regional adapter is bound.
    pub fn from_state(bytes: &[u8]) -> Result<Self, String> {
        let config = Self::from_stream_state(bytes)?;
        if config.region.is_some() || config.dummy() {
            return Err("ambient outside global real-stream profile".into());
        }
        Ok(config)
    }
    pub fn dummy(&self) -> bool {
        self.beds
            .first()
            .is_some_and(|n| n.eq_ignore_ascii_case("dummy"))
    }
    pub fn from_stream_state(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() != 700 {
            return Err("ambient state size outside measured profile".into());
        }
        let word = |at| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
        if word(0) > 1 || [4, 8, 12, 16].into_iter().any(|at| word(at) != 0) {
            return Err("ambient outside stream-only profile".into());
        }
        let names = |count_at, base| -> Result<Vec<String>, String> {
            let count = word(count_at) as usize;
            if count > 10 {
                return Err("ambient stream count exceeds ten".into());
            }
            (0..count)
                .map(|i| {
                    let raw = &bytes[base + i * 32..base + (i + 1) * 32];
                    let end = raw
                        .iter()
                        .position(|v| *v == 0)
                        .ok_or("unterminated ambient name")?;
                    if end == 0
                        || !raw[..end]
                            .iter()
                            .all(|b| b.is_ascii_alphanumeric() || *b == b'_')
                    {
                        return Err("invalid ambient stream identifier".into());
                    }
                    Ok(std::str::from_utf8(&raw[..end]).unwrap().to_owned())
                })
                .collect()
        };
        let beds = names(0x34, 0x3c)?;
        let accents = names(0x38, 0x17c)?;
        let delays = [f32::from_bits(word(0x1c)), f32::from_bits(word(0x20))];
        if beds.is_empty()
            || !delays.into_iter().all(|v| v.is_finite() && v >= 0.)
            || delays[0] > delays[1]
        {
            return Err("ambient stream configuration outside measured profile".into());
        }
        let region = if word(0) == 0 {
            let ranges = [f32::from_bits(word(0x14)), f32::from_bits(word(0x18))];
            if !ranges.into_iter().all(f32::is_finite) || ranges[0] < 0. || ranges[1] <= ranges[0] {
                return Err("ambient regional range outside finite ordered profile".into());
            }
            Some(ranges)
        } else {
            None
        };
        Ok(Self {
            beds,
            accents,
            delays,
            region,
        })
    }
}
pub trait Backend {
    fn play(&mut self, name: &str, argument: u32, looping: bool) -> u32;
    fn playing(&mut self, handle: u32) -> bool;
    fn stop(&mut self, handle: u32);
    fn volume(&mut self, _handle: u32, _gain: f32) {}
}
#[derive(Clone)]
pub struct Driver {
    pub phase: u32,
    pub timer: f32,
    pub random: Random,
    pub bed: u32,
    pub accent: u32,
    pub gain: f32,
    pub dummy: bool,
}
impl Driver {
    pub fn new(seed: u32) -> Self {
        Self {
            phase: 0,
            timer: 0.,
            random: Random(seed),
            bed: 0,
            accent: 0,
            gain: 0.,
            dummy: false,
        }
    }
    fn delay(&mut self, c: &Config, p: Precision) -> f32 {
        let r = |v| p.round(v);
        let span = r(f64::from(c.delays[1]) - f64::from(c.delays[0]));
        r(
            r(r(f64::from(self.random.next_15()) * span) * f64::from(1f32 / 32768.))
                + f64::from(c.delays[0]),
        ) as f32
    }
    pub fn update(
        &mut self,
        c: &Config,
        dt: f32,
        argument: u32,
        p: Precision,
        backend: &mut impl Backend,
    ) -> Result<(), String> {
        self.update_at(c, dt, argument, p, None, backend)
    }
    /// Supplied (listener, emitter) positions are native-coordinate inputs.
    /// Regional gain is deliberately not clamped here; the stream backend has
    /// its own global-gain multiplication and hardware-volume saturation.
    #[allow(clippy::too_many_arguments)]
    pub fn update_at(
        &mut self,
        c: &Config,
        dt: f32,
        argument: u32,
        p: Precision,
        positions: Option<([f32; 3], [f32; 3])>,
        backend: &mut impl Backend,
    ) -> Result<(), String> {
        if !dt.is_finite() || !(0.0..=1.).contains(&dt) {
            return Err("ambient delta outside finite bounded profile".into());
        }
        if self.phase < 5 {
            if self.phase == 0 {
                self.timer = self.delay(c, p);
                self.dummy = c.dummy();
            }
            self.phase += 1;
            return Ok(());
        }
        let distance = if let Some([_, maximum]) = c.region {
            let (listener, emitter) =
                positions.ok_or("regional ambience requires listener and emitter positions")?;
            let distance = crate::lod::distance_from_positions(listener, emitter, 0., p)
                .map_err(str::to_owned)?;
            if distance > maximum {
                if (self.bed as i32) > 0 {
                    if !self.dummy {
                        backend.stop(self.bed);
                    }
                    self.bed = 0;
                }
                if (self.accent as i32) > 0 {
                    backend.stop(self.accent);
                    self.accent = 0;
                }
                return Ok(());
            }
            Some(distance)
        } else {
            None
        };
        if (self.bed as i32) <= 0 {
            self.bed = if self.dummy {
                1
            } else {
                let index = (self.random.next_15() * c.beds.len() as u32) >> 15;
                backend.play(&c.beds[index as usize], argument, true)
            };
            self.timer = self.delay(c, p);
        }
        if (self.bed as i32) <= 0 {
            return Ok(());
        }
        if let (Some([minimum, maximum]), Some(distance)) = (c.region, distance) {
            let numerator = p.round(f64::from(distance) - f64::from(minimum));
            let denominator = p.round(f64::from(maximum) - f64::from(minimum));
            self.gain = p.round(1. - p.round(numerator / denominator)) as f32;
            if !self.dummy {
                backend.volume(self.bed, self.gain);
            }
        }
        if (self.accent as i32) <= 0 {
            if !c.accents.is_empty() {
                let next = p.round(f64::from(self.timer) - f64::from(dt));
                self.timer = next as f32;
                if next < 0. {
                    let index = (self.random.next_15() * c.accents.len() as u32) >> 15;
                    self.accent = backend.play(&c.accents[index as usize], argument, false);
                    if c.region.is_some() && (self.accent as i32) > 0 {
                        backend.volume(self.accent, self.gain);
                    }
                    self.timer = self.delay(c, p);
                }
            }
        } else if !backend.playing(self.accent) {
            self.accent = 0;
        } else if c.region.is_some() {
            backend.volume(self.accent, self.gain);
        }
        Ok(())
    }
    /// Native destructor also passes dummy sentinel1 to stop. A real backend must
    /// validate opaque handles; the sentinel is not an admitted audio voice.
    /// Native callback destructor stops the primary loop. Mission ownership must
    /// additionally retire any outstanding one-shots and their asset handles.
    pub fn destroy(&self, backend: &mut impl Backend) {
        if (self.bed as i32) > 0 {
            backend.stop(self.bed);
        }
    }
}
