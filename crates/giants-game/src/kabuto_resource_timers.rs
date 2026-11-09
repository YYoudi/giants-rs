//! Acquired activity/resource timers and next-frame speed latch. The preceding
//! alive/global guard, following action selection and live scheduling are separate.
use crate::animation_frames;
use giants_formats::global_data::GlobalData;

pub struct Input {
    pub channels: crate::animation_channels::Channels,
    pub descriptor: animation_frames::Descriptor,
    pub model: [f32; 4],
    pub mode: u32,
    pub buttons: u32,
    pub damage: f32,
    pub limit: f32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    StartSound(u32),
    NotifySound(u32),
}
/// These services must execute or refuse. The predicate is the actual world
/// notification decision, not inferred from actor ownership or a peer count.
pub trait World {
    fn seconds(&mut self) -> Result<f64, String>;
    fn event(&mut self, event: Event, payload: &mut [u8]) -> Result<(), String>;
    fn notifications(&mut self) -> Result<bool, String>;
}
#[derive(Clone, Copy, Debug)]
pub struct View {
    pub previous_frame: i32,
    pub current_frame: i32,
    pub activity: u32,
    pub initial_mode: u32,
    pub previous_selection: u32,
    pub slot: u32,
    pub reverse: bool,
    pub wrapped: bool,
    pub charge: f32,
    pub interval: f32,
}
fn word(p: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(p[at..at + 4].try_into().unwrap())
}
fn put_word(p: &mut [u8], at: usize, v: u32) {
    p[at..at + 4].copy_from_slice(&v.to_le_bytes());
}
fn scalar(p: &[u8], at: usize) -> Result<f32, String> {
    let v = f32::from_bits(word(p, at));
    if v.is_finite() {
        Ok(v)
    } else {
        Err("Kabuto timer input outside finite profile".into())
    }
}
fn put(p: &mut [u8], at: usize, v: f64) -> Result<(), String> {
    let v = v as f32;
    if !v.is_finite() {
        return Err("Kabuto timer output outside finite profile".into());
    }
    put_word(p, at, v.to_bits());
    Ok(())
}
fn seconds(w: &mut impl World) -> Result<f64, String> {
    let v = w.seconds()?;
    if v.is_finite() && v >= 0. {
        Ok(v)
    } else {
        Err("Kabuto timer clock outside finite nonnegative profile".into())
    }
}
fn interval(g: &GlobalData, damage: f32, limit: f32, at: [usize; 2]) -> Result<f64, String> {
    let a = f64::from(g.scalar(at[0]).map_err(|e| e.to_string())?);
    let b = f64::from(g.scalar(at[1]).map_err(|e| e.to_string())?);
    Ok(((f64::from(limit) - f64::from(damage)) * a + b * f64::from(damage)) / f64::from(limit))
}
/// Prefix and service effects remain on failure. The caller must quarantine
/// replay and must not acknowledge the enclosing primary/frame on success.
pub fn advance(p: &mut [u8], i: Input, g: &GlobalData, w: &mut impl World) -> Result<View, String> {
    if p.len() != 844
        || !i
            .model
            .into_iter()
            .chain([i.damage, i.limit])
            .all(f32::is_finite)
        || i.limit <= 0.
    {
        return Err("Kabuto timers require the acquired payload and finite positive limit".into());
    }
    let v = f64::from(scalar(p, 0xe0)?) + seconds(w)?;
    put(p, 0xe0, v)?;
    let previous_selection = word(p, 0x90);
    let previous_frame =
        animation_frames::frame_at_time(i.descriptor, f32::from_bits(i.channels[0][2]))?;
    let current_frame =
        animation_frames::frame_at_time(i.descriptor, f32::from_bits(i.channels[0][1]))?;
    let history = u16::from_le_bytes(p[0x8e..0x90].try_into().unwrap());
    let history = history.wrapping_mul(2) | u16::from(i.model[3] != 0.);
    let old = u16::from_le_bytes(p[0x8c..0x8e].try_into().unwrap());
    let count = old.wrapping_add((history & 1).wrapping_sub(history >> 4));
    p[0x8c..0x8e].copy_from_slice(&count.to_le_bytes());
    p[0x8e..0x90].copy_from_slice(&(history & 15).to_le_bytes());
    let delay = f64::from(scalar(p, 4)?) - seconds(w)?;
    put(p, 4, delay)?;
    if delay < 0. {
        put_word(p, 4, 0);
        put_word(p, 0, word(p, 0) & !0x40);
    }
    let activity = u32::from(i.model[1] != 0.)
        | (u32::from(i.model[2] != 0.) << 1)
        | (u32::from(count > 2) << 2)
        | (u32::from(scalar(p, 4)? != 0.) << 3);
    if i.buttons & 0x800 != 0 && scalar(p, 0x128)? >= g.scalar(0x9e8).map_err(|e| e.to_string())? {
        put_word(p, 0x12c, 1);
    }
    let mut t;
    if word(p, 0x12c) != 0 && scalar(p, 0x128)? > 0. {
        let wide = interval(g, i.damage, i.limit, [0x10bc, 0x10b8])?;
        t = wide as f32;
        let reserve = if wide != 0. {
            f64::from(scalar(p, 0x128)?) - seconds(w)? / f64::from(t)
        } else {
            0.
        };
        put(p, 0x128, reserve)?;
        if scalar(p, 0x128)? < 0. {
            put_word(p, 0x128, 0);
        }
    } else {
        if word(p, 0x12c) != 0 {
            w.event(Event::StartSound(0x40a), p)?;
            if w.notifications()? {
                w.event(Event::NotifySound(0x40a), p)?;
            }
        }
        put_word(p, 0x12c, 0);
        let wide = interval(g, i.damage, i.limit, [0x10b4, 0x10b0])?;
        t = wide as f32;
        let reserve = if wide != 0. {
            f64::from(scalar(p, 0x128)?) + seconds(w)? / f64::from(t)
        } else {
            1.
        };
        put(p, 0x128, reserve)?;
        if scalar(p, 0x128)? > 1. {
            put(p, 0x128, 1.)?;
        }
    }
    let cooldown = f64::from(scalar(p, 0x140)?) - seconds(w)?;
    put(p, 0x140, cooldown)?;
    if cooldown < 0. {
        put_word(p, 0x140, 0);
    }
    if scalar(p, 0x140)? == 0. && scalar(p, 0x15c)? >= 3. {
        let wide = interval(g, i.damage, i.limit, [0x11a0, 0x11a4])?;
        t = wide as f32;
        let cap = if wide != 0. {
            f64::from(scalar(p, 0x134)?) + seconds(w)? / f64::from(t)
        } else {
            1.
        };
        put(p, 0x134, cap)?;
        if scalar(p, 0x134)? > 1. {
            put(p, 0x134, 1.)?;
        }
    }
    if i.buttons & 0x10 != 0 && scalar(p, 0x140)? == 0. && scalar(p, 0x15c)? >= 3. {
        let charge = f64::from(scalar(p, 0x13c)?) + seconds(w)?;
        put(p, 0x13c, charge)?;
        let upper = g.scalar(0x11a8).map_err(|e| e.to_string())?;
        if charge > f64::from(upper) {
            put(p, 0x13c, f64::from(upper))?;
        }
        let charge = scalar(p, 0x13c)?;
        let a = g.scalar(0x9e0).map_err(|e| e.to_string())?;
        let value = if charge <= a {
            Some(f64::from(charge) / (f64::from(a) * 3.))
        } else {
            let b = g.scalar(0x9e4).map_err(|e| e.to_string())?;
            if charge <= b {
                Some(
                    (f64::from(charge) - f64::from(a)) / ((f64::from(b) - f64::from(a)) * 3.)
                        + f64::from(1f32 / 3.),
                )
            } else if charge <= upper {
                Some(
                    (f64::from(charge) - f64::from(b)) / ((f64::from(upper) - f64::from(b)) * 3.)
                        + f64::from(2f32 / 3.),
                )
            } else {
                None
            }
        };
        if let Some(value) = value {
            put(p, 0x138, value)?;
        }
        if scalar(p, 0x138)? >= 0.99f32 {
            put(p, 0x138, 1.)?;
        }
    } else {
        put_word(p, 0x13c, 0);
        put_word(p, 0x138, 0);
    }
    let mut charge = scalar(p, 0x138)?;
    if charge > scalar(p, 0x134)? {
        charge = scalar(p, 0x134)?;
        put(p, 0x138, f64::from(charge))?;
    }
    let at = if word(p, 0x12c) != 0 { 0x9ec } else { 0x9f0 };
    let duration = g.scalar(at).map_err(|e| e.to_string())?;
    let fraction = if duration != 0. {
        let step = seconds(w)? / f64::from(duration);
        if word(p, 0x12c) != 0 {
            f64::from(scalar(p, 0x130)?) + step
        } else {
            f64::from(scalar(p, 0x130)?) - step
        }
    } else if word(p, 0x12c) != 0 {
        1.
    } else {
        0.
    };
    put(p, 0x130, fraction)?;
    if word(p, 0x12c) != 0 {
        if scalar(p, 0x130)? > 1. {
            put(p, 0x130, 1.)?;
        }
    } else if scalar(p, 0x130)? < 0. {
        put_word(p, 0x130, 0);
    }
    let speed = (f64::from(scalar(p, 0x170)?) - f64::from(scalar(p, 0x164)?))
        * f64::from(scalar(p, 0x130)?)
        + f64::from(scalar(p, 0x164)?);
    put(p, 0x160, speed)?;
    Ok(View {
        previous_frame,
        current_frame,
        activity,
        initial_mode: i.mode,
        previous_selection,
        slot: i.channels[0][0],
        reverse: i.channels[0][8] & 0x10 != 0,
        wrapped: i.channels[0][8] & 2 != 0,
        charge,
        interval: t,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Services {
        fail_sound: bool,
        calls: Vec<Event>,
        clock_calls: usize,
        predicates: usize,
    }
    impl World for Services {
        fn seconds(&mut self) -> Result<f64, String> {
            self.clock_calls += 1;
            Ok(0.1)
        }
        fn notifications(&mut self) -> Result<bool, String> {
            self.predicates += 1;
            Ok(true)
        }
        fn event(&mut self, event: Event, p: &mut [u8]) -> Result<(), String> {
            self.calls.push(event);
            match event {
                Event::StartSound(0x40a) => {
                    assert_eq!(word(p, 0) & 0x40, 0);
                    assert_eq!(word(p, 0x12c), 1);
                    if self.fail_sound {
                        Err("sound provider failed".into())
                    } else {
                        put(p, 0x128, 0.25)?;
                        Ok(())
                    }
                }
                Event::NotifySound(0x40a) => {
                    assert_eq!(scalar(p, 0x128)?, 0.25);
                    Err("transport failed".into())
                }
                _ => panic!("unexpected event"),
            }
        }
    }
    fn fixture() -> (Vec<u8>, Input, GlobalData) {
        let mut p = vec![0; 844];
        put_word(&mut p, 0, 0x41);
        put_word(&mut p, 0x12c, 1);
        put(&mut p, 0xe0, 7.).unwrap();
        put(&mut p, 0x130, 0.75).unwrap();
        put(&mut p, 0x160, 123.).unwrap();
        let channels = [[1, 0, 0, 0, 0, 0, 0, 0, 0]; 4];
        (
            p,
            Input {
                channels,
                descriptor: animation_frames::Descriptor {
                    timeline: [0, 60, 30, 1],
                    flags: 0,
                },
                model: [0.; 4],
                mode: 0,
                buttons: 0,
                damage: 0.,
                limit: 100.,
            },
            GlobalData {
                words: vec![1f32.to_bits(); 0x1200 / 4],
                array_words: vec![],
            },
        )
    }
    #[test]
    fn sound_failure_retains_prefix_and_does_not_recharge_or_publish_speed() {
        let (mut p, i, g) = fixture();
        let mut w = Services {
            fail_sound: true,
            calls: vec![],
            clock_calls: 0,
            predicates: 0,
        };
        assert_eq!(
            advance(&mut p, i, &g, &mut w).unwrap_err(),
            "sound provider failed"
        );
        assert_eq!(w.calls, vec![Event::StartSound(0x40a)]);
        assert_eq!((w.clock_calls, w.predicates), (2, 0));
        assert_eq!(scalar(&p, 0xe0).unwrap(), 7.1);
        assert_eq!(scalar(&p, 0x128).unwrap(), 0.);
        assert_eq!(word(&p, 0x12c), 1);
        assert_eq!(scalar(&p, 0x130).unwrap(), 0.75);
        assert_eq!(scalar(&p, 0x160).unwrap(), 123.);
    }
    #[test]
    fn transport_failure_preserves_completed_sound_mutation_before_latch_clear() {
        let (mut p, i, g) = fixture();
        let mut w = Services {
            fail_sound: false,
            calls: vec![],
            clock_calls: 0,
            predicates: 0,
        };
        assert_eq!(
            advance(&mut p, i, &g, &mut w).unwrap_err(),
            "transport failed"
        );
        assert_eq!(
            w.calls,
            vec![Event::StartSound(0x40a), Event::NotifySound(0x40a)]
        );
        assert_eq!((w.clock_calls, w.predicates), (2, 1));
        assert_eq!(scalar(&p, 0x128).unwrap(), 0.25);
        assert_eq!(word(&p, 0x12c), 1);
        assert_eq!(scalar(&p, 0x160).unwrap(), 123.);
    }
}
