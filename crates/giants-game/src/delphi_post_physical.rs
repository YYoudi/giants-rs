//! Acquired post-physical timing, inhibited cleanup and ordinary non-strike
//! sword route. Equipment, active context retirement, spells, strike windows
//! and hit services remain required boundaries, with earlier effects retained.
use crate::object_time::{Sample, SlowArea};

#[derive(Clone, Copy, Debug)]
pub struct State {
    /// Observed subtraction word; no gameplay meaning is assigned here.
    pub timed_word: f32,
    pub controls: u32,
    pub response: u32,
    pub buttons: u32,
    pub internal: u32,
    pub attack_serial: u32,
    pub cleanup_context: u32,
}
pub struct Input<'a> {
    pub global_seconds: f32,
    /// Actual post-placement position and opaque time-area owner.
    pub actor: Sample,
    pub areas: &'a [SlowArea],
    pub local_owner: bool,
    pub paused: bool,
    pub inhibited: bool,
    /// The actual inline physical block's flags, not model metadata.
    pub physics_flags: u32,
    pub spell: i32,
    pub mode: i32,
    pub busy_contexts: [u32; 3],
}
#[derive(Clone, Debug, Default)]
pub struct World {
    /// Acquired selected-owner global word, without an inferred gameplay role.
    pub local_word: u32,
}
pub struct Context<'a> {
    pub world: &'a mut World,
    /// Actual shared PC view mode, used only by the physical held-visibility
    /// gate. Its gameplay meaning is not inferred from the value four.
    pub view_mode: i32,
    /// Required positional stop service bound to this owned actor. A native
    /// absent-bank control must supply that boundary explicitly.
    pub stop_sound: &'a mut dyn FnMut(u32) -> Result<(), String>,
}

/// The caller owns phase scheduling and quarantine. On a service refusal this
/// retains completed prefix writes; it must not be retried as a fresh callback.
pub fn advance(state: &mut State, input: Input<'_>, world: &mut World) -> Result<(), &'static str> {
    if !state.timed_word.is_finite()
        || !input.global_seconds.is_finite()
        || input.global_seconds < 0.
        || !input.actor.position.into_iter().all(f32::is_finite)
    {
        return Err("Delphi post-physical timing requires finite source inputs");
    }
    let seconds =
        crate::object_time::seconds(input.global_seconds, Some(input.actor), None, input.areas);
    let next = (f64::from(state.timed_word) - seconds) as f32;
    if !seconds.is_finite() || seconds < 0. || !next.is_finite() {
        return Err("Delphi post-physical timing outside acquired finite profile");
    }
    if input.local_owner {
        world.local_word = 0;
    }
    state.timed_word = next;
    if state.buttons & 0xc0 != 0 {
        return Err("Delphi post-physical equipment switch producer pending");
    }
    if input.inhibited || input.paused {
        state.controls &= !0x80;
        if state.cleanup_context != 0 {
            return Err("Delphi post-physical active context retirement pending");
        }
        state.controls &= !0x40;
        state.internal &= !1;
        state.buttons &= !0x2000000;
        if input.paused {
            return Ok(());
        }
    }
    if input.physics_flags & 0x40 != 0 {
        return Err("Delphi post-physical water profile pending");
    }
    if input.spell > 0 {
        return Err("Delphi post-physical spell producer pending");
    }
    if input.inhibited {
        return Ok(());
    }
    if input.busy_contexts.into_iter().any(|v| v != 0)
        || (49..56).contains(&input.mode)
        || input.mode == 76
    {
        state.buttons &= !0x2000100;
    }
    if state.buttons & 0x100 != 0 && state.response & 0x8000000 == 0 {
        state.controls |= 0x200;
        state.attack_serial = state.attack_serial.wrapping_add(1);
        return Err("Delphi sword attack notification producer pending");
    }
    if state.buttons & 0x100 == 0 {
        state.response &= !0x8000000;
    } else {
        state.response |= 0x8000000;
    }
    if state.controls & 0x400 != 0 {
        return Ok(());
    }
    if matches!(input.mode, 7 | 12..=16 | 18 | 22 | 26 | 31) {
        return Err("Delphi sword strike-window and hit producers pending");
    }
    state.controls &= !0x2000;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn state() -> State {
        State {
            timed_word: 1.,
            controls: 0xc0,
            response: 0,
            buttons: 0x2000000,
            internal: 1,
            attack_serial: 0,
            cleanup_context: 0,
        }
    }
    fn input() -> Input<'static> {
        Input {
            global_seconds: 0.25,
            actor: Sample {
                owner: 1,
                position: [0.; 3],
            },
            areas: &[],
            local_owner: true,
            paused: true,
            inhibited: false,
            physics_flags: 0x40,
            spell: 1,
            mode: 12,
            busy_contexts: [0; 3],
        }
    }
    #[test]
    fn paused_cleanup_follows_timing_and_skips_later_water_spell_and_strike_routes() {
        let mut state = state();
        let mut world = World { local_word: 27 };
        advance(&mut state, input(), &mut world).unwrap();
        assert_eq!((world.local_word, state.timed_word), (0, 0.75));
        assert_eq!((state.controls, state.internal, state.buttons), (0, 0, 0));
    }
    #[test]
    fn retirement_refusal_retains_only_the_completed_prefix() {
        let mut state = State {
            cleanup_context: 123,
            ..state()
        };
        let mut world = World { local_word: 27 };
        assert!(
            advance(&mut state, input(), &mut world)
                .unwrap_err()
                .contains("retirement")
        );
        assert_eq!((world.local_word, state.timed_word), (0, 0.75));
        assert_eq!(state.controls, 0x40);
        assert_eq!(
            (state.internal, state.buttons, state.cleanup_context),
            (1, 0x2000000, 123)
        );
    }
}
