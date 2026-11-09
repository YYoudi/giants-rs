//! Dry physical transition after first ANM, before model-rate production.
//! The current held actor and positional stop are required owned services.
#[derive(Clone, Copy, Debug)]
pub struct State {
    pub controls: u32,
    pub internal: u32,
    pub second_context: u32,
    pub timed_word: f32,
}
pub struct Input {
    pub physics_flags: u32,
    pub object_flags: u32,
    pub mode: i32,
    pub first_context: u32,
    pub equipment_context: u32,
    pub view_mode: i32,
    pub damage: f32,
    pub limit: f32,
    pub reset: f32,
}
pub trait World {
    type Held: Copy;
    /// First live reference-one child in the actual held-list order.
    fn held(&mut self) -> Result<Option<Self::Held>, String>;
    fn reveal(&mut self, held: Self::Held) -> Result<(), String>;
    /// Positional cue stop bound to this owner; no acknowledgement fallback.
    fn stop_sound(&mut self, cue: u32) -> Result<(), String>;
}
pub fn advance(state: &mut State, i: Input, world: &mut impl World) -> Result<(), String> {
    if i.physics_flags & 0x40 != 0 {
        return Err("Delphi physical immersion reset requires its water producer".into());
    }
    let requested = state.controls & 0x2000000 != 0;
    let object_reset = i.object_flags & 0x80000 != 0;
    let ordinary_mode = !(49..56).contains(&i.mode) && i.mode != 76;
    let ordinary = !requested
        && !object_reset
        && state.internal & 2 != 0
        && state.second_context == 0
        && ordinary_mode;
    let requested_reset =
        requested && i.object_flags & 0x100 == 0 && i.first_context == 0 && ordinary_mode;
    if requested_reset || !requested && (object_reset || ordinary) {
        let visible = i.view_mode != 4 || i.damage < i.limit;
        let check_held = ordinary || visible && i.equipment_context == 0;
        let held = if check_held { world.held()? } else { None };
        if let Some(held) = held.filter(|_| visible) {
            world.reveal(held)?;
        }
        world.stop_sound(0x34c)?;
        if !i.reset.is_finite() {
            return Err("Delphi physical reset requires its finite external parameter".into());
        }
        state.timed_word = i.reset;
        state.controls &= !0xc000;
        state.second_context = 0;
        if !ordinary || held.is_some() {
            state.internal &= !2;
        }
    }
    if requested {
        state.controls &= !0x2000000;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Services(bool);
    impl World for Services {
        type Held = ();
        fn held(&mut self) -> Result<Option<()>, String> {
            Ok(Some(()))
        }
        fn reveal(&mut self, _: ()) -> Result<(), String> {
            self.0 = true;
            Ok(())
        }
        fn stop_sound(&mut self, cue: u32) -> Result<(), String> {
            assert_eq!(cue, 0x34c);
            Err("declared stop failure".into())
        }
    }
    fn input(mode: i32) -> Input {
        Input {
            physics_flags: 0,
            object_flags: 0x80000,
            mode,
            first_context: 0,
            equipment_context: 0,
            view_mode: 0,
            damage: 0.,
            limit: 10.,
            reset: f32::NAN,
        }
    }
    #[test]
    fn requested_jump_clears_request_without_consuming_reset_or_services() {
        let mut state = State {
            controls: 0x200c000,
            internal: 2,
            second_context: 123,
            timed_word: -1.,
        };
        let mut world = Services(false);
        advance(&mut state, input(49), &mut world).unwrap();
        assert_eq!(
            (
                state.controls,
                state.internal,
                state.second_context,
                state.timed_word
            ),
            (0xc000, 2, 123, -1.)
        );
        assert!(!world.0);
    }
    #[test]
    fn stop_failure_retains_revealed_held_actor_before_reset_writes() {
        let mut state = State {
            controls: 0xc000,
            internal: 2,
            second_context: 123,
            timed_word: -1.,
        };
        let mut world = Services(false);
        assert!(
            advance(&mut state, input(3), &mut world)
                .unwrap_err()
                .contains("stop failure")
        );
        assert!(world.0);
        assert_eq!(
            (
                state.controls,
                state.internal,
                state.second_context,
                state.timed_word
            ),
            (0xc000, 2, 123, -1.)
        );
    }
}
