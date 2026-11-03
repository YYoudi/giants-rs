//! Complete ordinary class73 mode33 branch, after conditional perception.
//! The common prefix/tail, animation and motion remain separate phases.
use crate::{
    audio_parameters::Random,
    enemy_projectile::SoundOutcome,
    ripper_perception::Frame,
    ripper_steering::{self, Orbit, State as Steering},
};

#[derive(Clone, Debug)]
pub struct State<K> {
    pub class: i32,
    pub mode: u32,
    pub steering: Steering,
    pub control: u32,
    pub retained_timer: f32,
    pub orbit: Orbit,
    pub retaliation: Option<K>,
}
pub(crate) fn target_control<K>(
    target: Option<&crate::ripper_perception::Candidate<K>>,
    retained_timer: f32,
    mut control: u32,
) -> Result<(bool, u32), &'static str> {
    if !retained_timer.is_finite()
        || target.is_some_and(|a| !a.damage.is_finite() || !a.limit.is_finite())
    {
        return Err("target/timer outside finite profile");
    }
    let mut lost = target.is_none_or(|a| {
        a.class == 0
            || a.active & 0x4000_c000 != 0
            || a.damage >= a.limit
            || (a.state == 2 && a.control & 0x0200_0000 != 0 && retained_timer < 4.)
    });
    if !lost && let Some(target) = target.filter(|a| a.state == 3) {
        if target.control & 8 != 0 {
            if control & 0x10 == 0 {
                control |= 0x10;
                lost = true;
            }
        } else {
            control &= !0x10;
        }
    }
    Ok((lost, control))
}
pub trait Services<K> {
    fn owner_sound(
        &mut self,
        state: &State<K>,
        cue: u32,
        random: &mut Random,
    ) -> Result<SoundOutcome, String>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    TargetLost,
    Pursuing,
    AttackSelected,
}
#[derive(Clone, Debug)]
pub struct Machine<K> {
    state: State<K>,
    failed: Option<String>,
    consumed: bool,
}
impl<K: Copy + Ord> Machine<K> {
    pub fn new(state: State<K>) -> Self {
        Self {
            state,
            failed: None,
            consumed: false,
        }
    }
    pub fn state(&self) -> &State<K> {
        &self.state
    }
    pub fn failure(&self) -> Option<&str> {
        self.failed.as_deref()
    }
    /// Consume one actual mode33 branch, including target validation and its
    /// conditional random draw. Missing identities are errors, not null views.
    /// `attack_percent` is the current class-global parameter, not a roll.
    pub fn advance(
        &mut self,
        frame: &Frame<'_, K>,
        attack_percent: f32,
        random: &mut Random,
        services: &mut impl Services<K>,
    ) -> Result<Outcome, String> {
        if self.failed.is_some() || self.consumed {
            return Err("pursuit branch already consumed or failed".into());
        }
        if self.state.class != 73 || self.state.mode != 33 {
            return Err("pursuit requires acquired ordinary class73/mode33".into());
        }
        let mut state = self.state.clone();
        let target = state
            .retaliation
            .map(|id| frame.candidate(id))
            .transpose()?;
        let (lost, control) = target_control(target, state.retained_timer, state.control)?;
        state.control = control;
        if lost {
            // Sound sees the old mode and retained target, but the updated
            // control toggle. A failure may have external effects; never replay.
            if let Err(error) = services.owner_sound(&state, 1269, random) {
                self.state = state;
                self.failed = Some(error.clone());
                return Err(error);
            }
            state.mode = 43;
            state.retaliation = None;
            self.state = state;
            self.consumed = true;
            return Ok(Outcome::TargetLost);
        }
        let target = target.unwrap();
        let steering = ripper_steering::pursue(state.steering, target.position, state.orbit)?;
        let dx = f64::from(target.position[0]) - f64::from(state.steering.position[0]);
        let dy = f64::from(target.position[1]) - f64::from(state.steering.position[1]);
        // The preamble stores planar squared distance as f32. The comparison
        // radius product stays wide and its 1.1 multiplier is a native f32.
        let distance = (dx * dx + dy * dy) as f32;
        let radius = f64::from(state.orbit.radius) * f64::from(1.1_f32);
        if !distance.is_finite() || !attack_percent.is_finite() {
            return Err("pursuit decision outside finite profile".into());
        }
        state.steering = steering;
        let outcome = if f64::from(distance) < radius * radius {
            let roll = (random.next_15() * 100) >> 15;
            state.mode = if f64::from(attack_percent) > f64::from(roll) {
                37
            } else {
                39
            };
            Outcome::AttackSelected
        } else {
            Outcome::Pursuing
        };
        self.state = state;
        self.consumed = true;
        Ok(outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ripper_perception::Candidate;
    struct Refuse(u32);
    impl Services<u32> for Refuse {
        fn owner_sound(
            &mut self,
            state: &State<u32>,
            cue: u32,
            random: &mut Random,
        ) -> Result<SoundOutcome, String> {
            assert_eq!(
                (state.mode, state.retaliation, state.control & 0x10, cue),
                (33, Some(2), 0x10, 1269)
            );
            self.0 += 1;
            random.next_15();
            Err("owned sound unavailable".into())
        }
    }
    fn state() -> State<u32> {
        State {
            class: 73,
            mode: 33,
            steering: Steering {
                position: [0.; 3],
                heading: 0.,
                flags: 0,
                personality: 0,
                commands: [0; 8],
                target_heading: 0.,
                target_position: [0.; 3],
            },
            control: 0,
            retained_timer: 4.,
            orbit: Orbit {
                angle: 0.,
                radius: 10.,
            },
            retaliation: Some(2),
        }
    }
    #[test]
    fn missing_view_is_retryable_but_started_sound_preserves_toggle_and_latches() {
        let mut machine = Machine::new(state());
        let mut random = Random(123);
        let mut backend = Refuse(0);
        assert!(
            machine
                .advance(
                    &Frame::new(&[], &[]).unwrap(),
                    50.,
                    &mut random,
                    &mut backend
                )
                .is_err()
        );
        assert!(machine.failure().is_none());
        assert_eq!(random.0, 123);
        let actors = [Candidate {
            id: 2,
            class: 3,
            state: 3,
            active: 0,
            personality: 1,
            control: 8,
            position: [0.; 3],
            damage: 0.,
            limit: 1.,
            parent_present: false,
        }];
        let frame = Frame::new(&actors, &[2]).unwrap();
        assert!(
            machine
                .advance(&frame, 50., &mut random, &mut backend)
                .is_err()
        );
        let seed = random.0;
        assert_ne!(seed, 123);
        assert_eq!(machine.state().mode, 33);
        assert_eq!(machine.state().control & 0x10, 0x10);
        assert!(
            machine
                .advance(&frame, 50., &mut random, &mut backend)
                .is_err()
        );
        assert_eq!((backend.0, random.0), (1, seed));
    }
}
