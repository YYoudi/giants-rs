//! Complete ordinary mode32 branch up to the shared primary tail. Real effect
//! removal is mandatory when an owned effect is present; no backend is implied.
use crate::{
    audio_parameters::Random,
    ripper_steering::{self, Orbit},
};

#[derive(Clone, Copy, Debug)]
pub struct Parameters {
    pub radius: [f32; 2],
    pub seconds: f32,
}
impl Parameters {
    pub fn from_globals(
        data: &giants_formats::global_data::GlobalData,
    ) -> Result<Self, giants_formats::Error> {
        Ok(Self {
            radius: [data.scalar(0xc8)?, data.scalar(0xcc)?],
            seconds: data.scalar(0xd0)?,
        })
    }
}
#[derive(Clone, Debug)]
pub struct State<E> {
    pub class: i32,
    pub mode: u32,
    pub steering: ripper_steering::State,
    pub orbit: Orbit,
    pub timer: f32,
    /// Owned effect identity, never an actor/native pointer cast.
    pub effect: Option<E>,
    /// Actual retained target view after perception, not a selected replacement.
    pub target: Option<[f32; 3]>,
}
pub trait Services<E> {
    fn remove_effect(
        &mut self,
        state: &State<E>,
        effect: E,
        random: &mut Random,
    ) -> Result<(), String>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    WaitingForAnimation,
    ContinuePrimaryTail,
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
        primary_flags: u32,
        parameters: Parameters,
        random: &mut Random,
        services: &mut impl Services<E>,
    ) -> Result<Outcome, String> {
        if let Some(error) = &self.failed {
            return Err(format!("turn completion service already failed: {error}"));
        }
        if self.state.class != 73 || self.state.mode != 32 {
            return Err("turn completion requires acquired ordinary class73/mode32".into());
        }
        let finished = primary_flags & 2 != 0;
        if ![
            self.state.orbit.angle,
            self.state.orbit.radius,
            self.state.timer,
        ]
        .into_iter()
        .all(f32::is_finite)
            || (finished
                && self.state.target.is_some()
                && !parameters
                    .radius
                    .into_iter()
                    .chain([parameters.seconds])
                    .all(f32::is_finite))
        {
            return Err("turn completion outside finite parameter profile".into());
        }
        let target = self.state.target.unwrap_or([99_999.; 3]);
        let steering = ripper_steering::turn_to_point(self.state.steering, target)?;
        self.state.steering = steering;
        if !finished {
            return Ok(Outcome::WaitingForAnimation);
        }
        if let Some(effect) = self.state.effect
            && let Err(error) = services.remove_effect(&self.state, effect, random)
        {
            self.failed = Some(error.clone());
            return Err(error);
        }
        self.state.effect = None;
        self.state.mode = 33;
        if let Some(target) = self.state.target {
            // Native bearing stays wide until the two interval endpoints store.
            let bearing = crate::actor_volume::heading_wide(target, self.state.steering.position);
            let low = (bearing - 50.) as f32;
            let high = (bearing + 50.) as f32;
            let interval = |random: &mut Random, low: f32, high: f32| {
                (f64::from(random.next_15()) * (f64::from(high) - f64::from(low)) * (1. / 32768.)
                    + f64::from(low)) as f32
            };
            self.state.orbit.angle = ripper_steering::normalize(interval(random, low, high));
            self.state.orbit.radius = interval(random, parameters.radius[0], parameters.radius[1]);
            self.state.timer = parameters.seconds;
        }
        Ok(Outcome::ContinuePrimaryTail)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Refuse {
        calls: u32,
    }
    impl Services<u32> for Refuse {
        fn remove_effect(
            &mut self,
            state: &State<u32>,
            effect: u32,
            _: &mut Random,
        ) -> Result<(), String> {
            self.calls += 1;
            assert_eq!((state.mode, state.effect, effect), (32, Some(7), 7));
            assert_eq!(state.steering.personality & 0x40, 0);
            Err("owned effect removal unavailable".into())
        }
    }
    fn state() -> State<u32> {
        State {
            class: 73,
            mode: 32,
            steering: ripper_steering::State {
                position: [0.; 3],
                heading: 0.,
                flags: 0,
                personality: 0x40,
                commands: [0; 8],
                target_heading: 0.,
                target_position: [0.; 3],
            },
            orbit: Orbit {
                angle: 15.,
                radius: 20.,
            },
            timer: 3.,
            effect: Some(7),
            target: Some([10., 0., 0.]),
        }
    }
    #[test]
    fn started_removal_failure_cannot_replay_or_draw_or_transition() {
        let mut machine = Machine::new(state());
        let mut random = Random(12);
        let mut services = Refuse { calls: 0 };
        let parameters = Parameters {
            radius: [20., 40.],
            seconds: 3.,
        };
        for _ in 0..2 {
            assert!(
                machine
                    .advance(2, parameters, &mut random, &mut services)
                    .is_err()
            );
        }
        assert_eq!(services.calls, 1);
        assert_eq!(random.0, 12);
        assert_eq!(
            (machine.state().mode, machine.state().effect),
            (32, Some(7))
        );
        assert_eq!(machine.state().orbit.angle, 15.);
        assert!(machine.failure().is_some());
    }
    #[test]
    fn preflight_failure_is_retryable_and_waiting_never_calls_removal() {
        let mut machine = Machine::new(state());
        let mut random = Random(12);
        let mut services = Refuse { calls: 0 };
        assert!(
            machine
                .advance(
                    2,
                    Parameters {
                        radius: [f32::NAN, 2.],
                        seconds: 1.
                    },
                    &mut random,
                    &mut services
                )
                .is_err()
        );
        assert!(machine.failure().is_none());
        assert_eq!(
            machine
                .advance(
                    0,
                    Parameters {
                        radius: [1., 2.],
                        seconds: 1.
                    },
                    &mut random,
                    &mut services
                )
                .unwrap(),
            Outcome::WaitingForAnimation
        );
        assert_eq!((services.calls, random.0), (0, 12));
    }
}
