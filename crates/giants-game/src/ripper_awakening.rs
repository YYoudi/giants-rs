//! Ordinary Ripper mode31 branch after perception. Ground sampling, FX context
//! creation and both audio routes require actual providers. No mission tick is
//! implied; mode34/44 continuations remain separate.
use crate::{audio_parameters::Random, ripper_steering::Orbit};

#[derive(Clone, Copy, Debug)]
pub struct Parameters {
    pub camera_range: f32,
    pub slope_decision: f32,
    pub orbit_radius: [f32; 2],
    pub seconds: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Refuse {
        requests: Vec<Audio>,
        contexts: usize,
    }
    impl Services<u32> for Refuse {
        fn ground_normal_z(&mut self, _: &State<u32>) -> Result<f32, String> {
            Err("ground provider absent".into())
        }
        fn prepare_dirt(&mut self, _: &State<u32>, _: &mut Random) -> Result<Option<u32>, String> {
            self.contexts += 1;
            Ok(Some(7))
        }
        fn audio(&mut self, _: &State<u32>, request: Audio, _: &mut Random) -> Result<(), String> {
            self.requests.push(request);
            Err("audio provider absent".into())
        }
    }
    fn state() -> State<u32> {
        State {
            mode: 31,
            position: [0.; 3],
            visibility: 0.,
            target: Some([10., 0., 0.]),
            effect: None,
            orbit: Orbit {
                angle: 0.,
                radius: 0.,
            },
            timer: 0.,
        }
    }
    fn parameters() -> Parameters {
        Parameters {
            camera_range: 50.,
            slope_decision: 101.,
            orbit_radius: [20., 40.],
            seconds: 5.,
        }
    }
    #[test]
    fn partial_context_and_audio_attempt_cannot_replay() {
        let mut m = Machine::new(state());
        let mut random = Random(1);
        let mut services = Refuse {
            requests: vec![],
            contexts: 0,
        };
        assert!(
            m.advance([0.; 3], parameters(), &mut random, &mut services)
                .is_err()
        );
        assert_eq!((m.state().mode, m.state().effect), (34, Some(7)));
        assert_eq!(services.requests, [Audio::Play(1256)]);
        let seed = random.0;
        assert!(
            m.advance([0.; 3], parameters(), &mut random, &mut services)
                .is_err()
        );
        assert_eq!(
            (services.contexts, services.requests.len(), random.0),
            (1, 1, seed)
        );
    }
    #[test]
    fn absent_ground_backend_does_not_become_an_assumed_flat_surface() {
        let mut m = Machine::new(state());
        let mut random = Random(1);
        let mut services = Refuse {
            requests: vec![],
            contexts: 0,
        };
        let mut p = parameters();
        p.slope_decision = -1.;
        assert!(m.advance([0.; 3], p, &mut random, &mut services).is_err());
        assert_eq!(
            (m.state().mode, m.state().effect, services.contexts),
            (31, None, 0)
        );
        assert!(services.requests.is_empty());
        let seed = random.0;
        assert!(m.advance([0.; 3], p, &mut random, &mut services).is_err());
        assert_eq!(random.0, seed);
    }
}
impl Parameters {
    pub fn from_globals(
        data: &giants_formats::global_data::GlobalData,
    ) -> Result<Self, giants_formats::Error> {
        Ok(Self {
            camera_range: data.scalar(0x144)?,
            slope_decision: data.scalar(0x3e4)?,
            orbit_radius: [data.scalar(0xc8)?, data.scalar(0xcc)?],
            seconds: data.scalar(0xd0)?,
        })
    }
}
#[derive(Clone, Debug)]
pub struct State<E> {
    pub mode: u32,
    pub position: [f32; 3],
    pub visibility: f32,
    pub target: Option<[f32; 3]>,
    pub effect: Option<E>,
    pub orbit: Orbit,
    pub timer: f32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Audio {
    Play(u32),
    Stop(u32),
}
pub trait Services<E> {
    /// Native query's normal.z. This is not a supplied boolean slope decision.
    fn ground_normal_z(&mut self, state: &State<E>) -> Result<f32, String>;
    /// Named attached context with flags0x24, tag=-1 and actor ownership.
    /// None is only an actual rejected preparation, never missing implementation.
    fn prepare_dirt(&mut self, state: &State<E>, random: &mut Random) -> Result<Option<E>, String>;
    fn audio(
        &mut self,
        state: &State<E>,
        request: Audio,
        random: &mut Random,
    ) -> Result<(), String>;
}
#[derive(Clone, Debug)]
pub struct Machine<E> {
    state: State<E>,
    failed: Option<String>,
}
impl<E: Copy> Machine<E> {
    pub fn new(state: State<E>) -> Self {
        Self {
            state,
            failed: None,
        }
    }
    pub fn state(&self) -> &State<E> {
        &self.state
    }
    pub fn failure(&self) -> Option<&str> {
        self.failed.as_deref()
    }
    pub fn advance(
        &mut self,
        camera: [f32; 3],
        parameters: Parameters,
        random: &mut Random,
        services: &mut impl Services<E>,
    ) -> Result<(), String> {
        if let Some(error) = &self.failed {
            return Err(format!("awakening service already failed: {error}"));
        }
        if self.state.mode != 31 {
            return Err("awakening requires ordinary class73 mode31 after perception".into());
        }
        if !self
            .state
            .position
            .into_iter()
            .chain(camera)
            .chain(self.state.target.into_iter().flatten())
            .chain([
                self.state.visibility,
                self.state.orbit.angle,
                self.state.orbit.radius,
                self.state.timer,
                parameters.camera_range,
                parameters.slope_decision,
                parameters.orbit_radius[0],
                parameters.orbit_radius[1],
                parameters.seconds,
            ])
            .all(f32::is_finite)
        {
            return Err("awakening outside finite profile".into());
        }
        let result = self.inner(camera, parameters, random, services);
        if let Err(error) = &result {
            self.failed = Some(error.clone());
        }
        result
    }
    fn inner(
        &mut self,
        camera: [f32; 3],
        p: Parameters,
        random: &mut Random,
        services: &mut impl Services<E>,
    ) -> Result<(), String> {
        let s = &mut self.state;
        let delta =
            std::array::from_fn::<_, 3, _>(|i| f64::from(camera[i]) - f64::from(s.position[i]));
        let squared = (delta[0] * delta[0] + delta[1] * delta[1]) + delta[2] * delta[2];
        s.visibility = if squared > f64::from(p.camera_range) * f64::from(p.camera_range) {
            1.
        } else {
            0.
        };
        if s.target.is_none() {
            return Ok(());
        }
        let decision = (random.next_15() * 100) >> 15;
        if f64::from(p.slope_decision) <= f64::from(decision) {
            let normal = services.ground_normal_z(s)?;
            if !normal.is_finite() {
                return Err("awakening ground normal is not finite".into());
            }
            if normal >= f32::from_bits(0x3f248d3b) {
                s.mode = 44;
                services.audio(s, Audio::Stop(1258), random)?;
                return Ok(());
            }
        }
        if s.effect.is_none() {
            s.effect = services.prepare_dirt(s, random)?;
        }
        s.mode = 34;
        if let Some(target) = s.target {
            let bearing = crate::actor_volume::heading_wide(target, s.position);
            let interval = |r: &mut Random, low: f32, high: f32| {
                (f64::from(r.next_15()) * (f64::from(high) - f64::from(low)) * (1. / 32768.)
                    + f64::from(low)) as f32
            };
            s.orbit.angle = crate::ripper_steering::normalize(interval(
                random,
                (bearing - 50.) as f32,
                (bearing + 50.) as f32,
            ));
            s.orbit.radius = interval(random, p.orbit_radius[0], p.orbit_radius[1]);
            s.timer = p.seconds;
        }
        s.visibility = 1.;
        services.audio(s, Audio::Play(1256), random)?;
        services.audio(s, Audio::Stop(1258), random)
    }
}
