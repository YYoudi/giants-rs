//! Ordinary class73 mode34 after perception. Terrain and audio services stay
//! required; this branch does not implement the subsequent mode44 lifecycle.
use crate::{audio_parameters::Random, ripper_perception::Frame, ripper_pursuit, ripper_steering};
#[derive(Clone, Debug)]
pub struct State<K, E> {
    pub actor: ripper_pursuit::State<K>,
    pub effect: Option<E>,
}
pub trait Services<K, E> {
    fn remove_effect(
        &mut self,
        state: &State<K, E>,
        effect: E,
        random: &mut Random,
    ) -> Result<(), String>;
    fn ground_normal_z(&mut self, state: &State<K, E>) -> Result<f32, String>;
    fn audio(
        &mut self,
        state: &State<K, E>,
        request: crate::ripper_awakening::Audio,
        random: &mut Random,
    ) -> Result<(), String>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Dormant,
    Approaching,
    Surfacing,
}
#[derive(Clone, Debug)]
pub struct Machine<K, E> {
    state: State<K, E>,
    failed: Option<String>,
    consumed: bool,
}
impl<K: Copy + Ord, E: Copy> Machine<K, E> {
    pub fn new(state: State<K, E>) -> Self {
        Self {
            state,
            failed: None,
            consumed: false,
        }
    }
    pub fn state(&self) -> &State<K, E> {
        &self.state
    }
    pub fn failure(&self) -> Option<&str> {
        self.failed.as_deref()
    }
    pub fn advance(
        &mut self,
        frame: &Frame<'_, K>,
        random: &mut Random,
        services: &mut impl Services<K, E>,
    ) -> Result<Outcome, String> {
        if self.failed.is_some() || self.consumed {
            return Err("emergence branch already consumed or failed".into());
        }
        if self.state.actor.class != 73 || self.state.actor.mode != 34 {
            return Err("emergence requires ordinary class73/mode34".into());
        }
        let target = self
            .state
            .actor
            .retaliation
            .map(|id| frame.candidate(id))
            .transpose()?;
        let (lost, control) = ripper_pursuit::target_control(
            target,
            self.state.actor.retained_timer,
            self.state.actor.control,
        )?;
        if lost {
            self.state.actor.control = control;
            if let Some(effect) = self.state.effect {
                let result = services.remove_effect(&self.state, effect, random);
                if let Err(error) = result {
                    self.failed = Some(error.clone());
                    return Err(error);
                }
            }
            self.state.effect = None;
            let result = services.audio(
                &self.state,
                crate::ripper_awakening::Audio::Play(1258),
                random,
            );
            if let Err(error) = result {
                self.failed = Some(error.clone());
                return Err(error);
            }
            self.state.actor.mode = 31;
            self.state.actor.retaliation = None;
            self.consumed = true;
            return Ok(Outcome::Dormant);
        }
        let target = target.unwrap();
        let steering = ripper_steering::pursue(
            self.state.actor.steering,
            target.position,
            self.state.actor.orbit,
        )?;
        let dx = f64::from(target.position[0]) - f64::from(self.state.actor.steering.position[0]);
        let dy = f64::from(target.position[1]) - f64::from(self.state.actor.steering.position[1]);
        let distance = (dx * dx + dy * dy) as f32;
        let radius = f64::from(self.state.actor.orbit.radius) * f64::from(1.1_f32);
        if !distance.is_finite() {
            return Err("emergence target distance overflow".into());
        }
        self.state.actor.control = control;
        self.state.actor.steering = steering;
        if f64::from(distance) < radius * radius {
            let normal = match services.ground_normal_z(&self.state) {
                Ok(n) if n.is_finite() => n,
                Ok(_) => {
                    let e = "emergence ground normal is not finite".to_string();
                    self.failed = Some(e.clone());
                    return Err(e);
                }
                Err(e) => {
                    self.failed = Some(e.clone());
                    return Err(e);
                }
            };
            if normal >= f32::from_bits(0x3f248d3b) {
                self.state.actor.mode = 44;
                if let Err(error) = services.audio(
                    &self.state,
                    crate::ripper_awakening::Audio::Stop(1256),
                    random,
                ) {
                    self.failed = Some(error.clone());
                    return Err(error);
                }
                self.consumed = true;
                return Ok(Outcome::Surfacing);
            }
        }
        self.consumed = true;
        Ok(Outcome::Approaching)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Backend {
        removes: u32,
        ground: u32,
        audio: u32,
    }
    impl Services<u32, u32> for Backend {
        fn remove_effect(
            &mut self,
            state: &State<u32, u32>,
            effect: u32,
            _: &mut Random,
        ) -> Result<(), String> {
            assert_eq!((state.actor.mode, state.effect, effect), (34, Some(7), 7));
            self.removes += 1;
            Ok(())
        }
        fn ground_normal_z(&mut self, _: &State<u32, u32>) -> Result<f32, String> {
            self.ground += 1;
            Err("ground service absent".into())
        }
        fn audio(
            &mut self,
            state: &State<u32, u32>,
            request: crate::ripper_awakening::Audio,
            _: &mut Random,
        ) -> Result<(), String> {
            assert_eq!(request, crate::ripper_awakening::Audio::Play(1258));
            assert_eq!(
                (
                    state.actor.mode,
                    state.effect,
                    state.actor.retaliation,
                    state.actor.control
                ),
                (34, None, Some(42), 0x10)
            );
            self.audio += 1;
            Err("audio service absent".into())
        }
    }
    fn state() -> State<u32, u32> {
        State {
            actor: ripper_pursuit::State {
                class: 73,
                mode: 34,
                steering: ripper_steering::State {
                    position: [0.; 3],
                    heading: 0.,
                    flags: 0,
                    personality: 0,
                    commands: [0; 8],
                    target_heading: 0.,
                    target_position: [0.; 3],
                },
                control: 0,
                retained_timer: 0.,
                orbit: ripper_steering::Orbit {
                    angle: 0.,
                    radius: 20.,
                },
                retaliation: Some(42),
            },
            effect: Some(7),
        }
    }
    fn candidate() -> crate::ripper_perception::Candidate<u32> {
        crate::ripper_perception::Candidate {
            id: 42,
            class: 2,
            state: 2,
            active: 0,
            personality: 1,
            control: 0,
            position: [5., 0., 0.],
            damage: 0.,
            limit: 100.,
            parent_present: false,
        }
    }
    #[test]
    fn required_ground_failure_cannot_become_a_successful_transition_or_replay() {
        let mut machine = Machine::new(state());
        let candidates = [candidate()];
        let frame = Frame::new(&candidates, &[42]).unwrap();
        let mut r = Random(1);
        let mut backend = Backend {
            removes: 0,
            ground: 0,
            audio: 0,
        };
        for _ in 0..2 {
            assert!(machine.advance(&frame, &mut r, &mut backend).is_err());
        }
        assert_eq!(
            (
                backend.ground,
                backend.removes,
                backend.audio,
                machine.state().actor.mode,
                r.0
            ),
            (1, 0, 0, 34, 1)
        );
    }
    #[test]
    fn successful_retirement_then_failed_audio_preserves_partial_state_and_cannot_replay() {
        let mut machine = Machine::new(state());
        let mut target = candidate();
        target.state = 3;
        target.control = 8;
        let candidates = [target];
        let frame = Frame::new(&candidates, &[42]).unwrap();
        let mut r = Random(1);
        let mut backend = Backend {
            removes: 0,
            ground: 0,
            audio: 0,
        };
        for _ in 0..2 {
            assert!(machine.advance(&frame, &mut r, &mut backend).is_err());
        }
        assert_eq!((backend.ground, backend.removes, backend.audio), (0, 1, 1));
        assert_eq!(
            (machine.state().actor.mode, machine.state().effect),
            (34, None)
        );
    }
}
