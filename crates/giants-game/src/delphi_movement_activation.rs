//! Acquired targeted-movement activation, including its required sound sequence,
//! target resolution, held visibility and positional stop services.
use crate::delphi_target_movement as movement;

pub struct State<K, H> {
    pub ready: bool,
    pub pending_target: Option<K>,
    pub active_target: Option<K>,
    pub sequence: Option<H>,
    pub controls: u32,
    pub internal: u32,
    pub gravity_mass: f32,
    pub mass: f32,
    pub movement_seconds: f32,
    pub reset_word: f32,
    pub second_context_present: bool,
    pub suppress_held_reveal: bool,
    pub position: [f32; 3],
    pub fallback: [f32; 3],
    pub movement: movement::State,
}
pub struct Parameters {
    pub gravity_mass: f32,
    pub mass: f32,
    pub reset_word: f32,
    pub movement: movement::Parameters,
    pub sequence: crate::mecc_audio::Sequence,
}
pub trait World<K, H> {
    fn create_sequence(
        &mut self,
        definition: crate::mecc_audio::Sequence,
    ) -> Result<Option<H>, String>;
    fn target_position(&mut self, target: K) -> Result<[f32; 3], String>;
    /// Actual notification guard and transport, including the offline return.
    fn notify(
        &mut self,
        target: Option<K>,
        fallback: [f32; 3],
        position: [f32; 3],
    ) -> Result<(), String>;
    fn reveal_held(&mut self) -> Result<(), String>;
    fn stop_sound(&mut self, sound: u32) -> Result<(), String>;
}
/// Prefix effects remain on refusal. Its owner must prevent replay, including
/// after sequence construction. This does not implement the readiness producer.
pub fn advance<K: Copy, H>(
    state: &mut State<K, H>,
    p: Parameters,
    world: &mut impl World<K, H>,
) -> Result<(), String> {
    if !state.ready {
        return Ok(());
    }
    if !p.gravity_mass.is_finite() || !p.mass.is_finite() || p.mass <= 0. {
        return Err("Delphi movement activation physical parameters outside finite profile".into());
    }
    state.gravity_mass = p.gravity_mass;
    state.mass = p.mass;
    if state.controls & 0x100 == 0 {
        state.sequence = world.create_sequence(p.sequence)?;
    }
    state.active_target = state.pending_target;
    let linked_position = state
        .active_target
        .map(|target| world.target_position(target))
        .transpose()?;
    let duration = movement::advance(
        &mut state.movement,
        movement::Input {
            position: state.position,
            fallback: state.fallback,
            linked_position,
            gravity_mass: state.gravity_mass,
            mass: state.mass,
        },
        p.movement,
    )?;
    state.movement_seconds = if state.active_target.is_some() {
        (f64::from(duration) * f64::from(p.movement.constants.half)) as f32
    } else {
        duration
    };
    world.notify(state.active_target, state.fallback, state.position)?;
    if !state.suppress_held_reveal {
        world.reveal_held()?;
    }
    world.stop_sound(0x34c)?;
    state.reset_word = p.reset_word;
    state.controls &= !0xc000;
    state.second_context_present = false;
    state.internal &= !2;
    state.controls |= 0x100;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Provider(Vec<&'static str>);
    impl World<u32, u32> for Provider {
        fn create_sequence(
            &mut self,
            _: crate::mecc_audio::Sequence,
        ) -> Result<Option<u32>, String> {
            self.0.push("sequence");
            Ok(Some(42))
        }
        fn target_position(&mut self, _: u32) -> Result<[f32; 3], String> {
            self.0.push("target");
            Ok([4., 2., 3.])
        }
        fn notify(&mut self, _: Option<u32>, _: [f32; 3], _: [f32; 3]) -> Result<(), String> {
            self.0.push("notify");
            Ok(())
        }
        fn reveal_held(&mut self) -> Result<(), String> {
            self.0.push("held");
            Ok(())
        }
        fn stop_sound(&mut self, _: u32) -> Result<(), String> {
            self.0.push("stop");
            Err("late stop failure".into())
        }
    }
    #[test]
    fn late_stop_failure_retains_sequence_target_and_motion_before_final_flags() {
        let mut s = State {
            ready: true,
            pending_target: Some(7),
            active_target: None,
            sequence: None,
            controls: 0xc000,
            internal: 2,
            gravity_mass: 1.,
            mass: 1.,
            movement_seconds: 0.,
            reset_word: 9.,
            second_context_present: true,
            suppress_held_reveal: false,
            position: [0.; 3],
            fallback: [1.; 3],
            movement: movement::State {
                cached_target: [0.; 3],
                force: [9.; 3],
                velocity: [0.; 3],
            },
        };
        let mut world = Provider(vec![]);
        assert!(
            advance(
                &mut s,
                Parameters {
                    gravity_mass: 2.,
                    mass: 3.,
                    reset_word: 10.,
                    movement: movement::Parameters {
                        fallback_rate: 1.,
                        linked_rate: 2.,
                        constants: movement::Constants {
                            exponent: 0.25,
                            acceleration_scale: -1.,
                            half: 0.5
                        }
                    },
                    sequence: crate::mecc_audio::Sequence {
                        start: 1,
                        continuous: 2,
                        release: 3,
                        flags: 3
                    }
                },
                &mut world
            )
            .is_err()
        );
        assert_eq!(world.0, ["sequence", "target", "notify", "held", "stop"]);
        assert_eq!((s.active_target, s.sequence), (Some(7), Some(42)));
        assert!(s.movement_seconds > 0.);
        assert_eq!(s.movement.force, [0.; 3]);
        assert_eq!((s.controls, s.internal, s.reset_word), (0xc000, 2, 9.));
        assert!(s.second_context_present);
    }
}
