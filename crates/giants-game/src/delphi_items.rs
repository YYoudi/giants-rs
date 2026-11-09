//! Acquired item/selection tail after reserve. Action execution and eligible
//! interaction targets remain required services, not acknowledged requests.
#[derive(Clone, Debug)]
pub struct State {
    pub buttons: u32,
    pub controls: u32,
    pub physics_flags: u32,
    pub movement_ready: bool,
    pub selected: i32,
    pub items: [i32; 5],
    pub spells: [[i32; 2]; 2],
    pub elapsed: f32,
    pub scan_remaining: f32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Use(i32),
    TargetMovement,
    Activate(i32),
    Cast,
}
pub trait World {
    fn actor_seconds(&mut self) -> Result<f64, String>;
    fn global_seconds(&mut self) -> Result<f64, String>;
    fn eligible(&mut self) -> Result<bool, String>;
    fn action(&mut self, action: Action, state: &mut State) -> Result<(), String>;
    /// Traverse the actual ordered actor list and execute its required services.
    /// Return true only when the source's interaction branch takes its early exit.
    fn scan(&mut self, state: &mut State) -> Result<bool, String>;
}
pub struct Context<'a> {
    pub reserve: crate::delphi_reserve::Context<'a>,
    pub session_mode: u32,
    pub peer_ids: &'a [u32],
    pub local_peer: u32,
    /// Required externally acquired header identity for mode four.
    pub owner_word: Option<u32>,
}
fn seconds(value: f64) -> Result<f64, String> {
    if value.is_finite() && value >= 0. {
        Ok(value)
    } else {
        Err("Delphi item tail requires finite nonnegative time".into())
    }
}
fn selected(state: &State) -> Result<i32, String> {
    usize::try_from(state.selected)
        .ok()
        .and_then(|index| state.items.get(index).copied())
        .ok_or_else(|| "Delphi item selection outside acquired inventory".into())
}
/// Invoke only after the actual reserve callback. The owner retains completed
/// writes on refusal and quarantines retry; this is not native error-path parity.
pub fn advance(state: &mut State, threshold: f32, world: &mut impl World) -> Result<(), String> {
    if state.buttons & 8 != 0 && state.selected >= 0 {
        world.action(Action::Use(selected(state)?), state)?;
    }
    if state.buttons & 0x10 != 0 {
        world.action(Action::TargetMovement, state)?;
    }
    if state.buttons & 0x20 != 0 {
        world.action(Action::Activate(selected(state)?), state)?;
        state.buttons &= !0x7c00;
    }
    if !state.elapsed.is_finite() {
        return Err("Delphi item elapsed word outside finite profile".into());
    }
    let elapsed = f64::from(state.elapsed) + seconds(world.actor_seconds()?)?;
    state.elapsed = elapsed as f32;
    if state.buttons & 0x400000 != 0 {
        if !threshold.is_finite() {
            return Err("Delphi item threshold outside finite profile".into());
        }
        if f64::from(threshold) <= elapsed
            && state.spells.iter().any(|s| s[0] == 0x3f && s[1] > 0)
            && state.physics_flags & 0x40 == 0
        {
            world.action(Action::Activate(0x3f), state)?;
        }
    }
    if state.buttons & 0x800000 != 0 && state.spells.iter().any(|s| s[0] == 0x48 && s[1] > 0) {
        world.action(Action::Cast, state)?;
    }
    for index in 0..5 {
        if state.buttons & (0x400 << index) != 0 && state.items[index] != -1 {
            if state.selected == index as i32 {
                world.action(Action::Activate(state.items[index]), state)?;
            } else {
                state.selected = index as i32;
            }
        }
    }
    if world.eligible()? {
        if !state.scan_remaining.is_finite() {
            return Err("Delphi item scan word outside finite profile".into());
        }
        let remaining = f64::from(state.scan_remaining) - seconds(world.global_seconds()?)?;
        state.scan_remaining = remaining as f32;
        if remaining < 0. {
            if !world.scan(state)? {
                state.controls &= !2;
            }
            state.scan_remaining = 0.5;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Provider {
        events: Vec<Action>,
        fail_scan: bool,
    }
    impl World for Provider {
        fn actor_seconds(&mut self) -> Result<f64, String> {
            Ok(0.25)
        }
        fn global_seconds(&mut self) -> Result<f64, String> {
            Ok(0.5)
        }
        fn eligible(&mut self) -> Result<bool, String> {
            Ok(true)
        }
        fn action(&mut self, a: Action, s: &mut State) -> Result<(), String> {
            self.events.push(a);
            if a == Action::TargetMovement {
                s.selected = 3;
            }
            Ok(())
        }
        fn scan(&mut self, _: &mut State) -> Result<bool, String> {
            if self.fail_scan {
                Err("scan service unavailable".into())
            } else {
                Ok(false)
            }
        }
    }
    fn state() -> State {
        State {
            buttons: 0,
            controls: 2,
            physics_flags: 0,
            movement_ready: false,
            selected: 0,
            items: [10, 11, 12, 13, 14],
            spells: [[0x3f, 1], [0x48, 1]],
            elapsed: 0.,
            scan_remaining: 0.,
        }
    }
    #[test]
    fn action_changes_are_reread_and_selection_bits_clear_only_after_activation() {
        let mut s = state();
        s.buttons = 8 | 0x10 | 0x20 | 0x7c00 | 0xc00000;
        let mut world = Provider {
            events: vec![],
            fail_scan: false,
        };
        advance(&mut s, 0.25, &mut world).unwrap();
        assert_eq!(
            world.events,
            [
                Action::Use(10),
                Action::TargetMovement,
                Action::Activate(13),
                Action::Activate(0x3f),
                Action::Cast
            ]
        );
        assert_eq!(s.buttons & 0x7c00, 0);
        assert_eq!(
            (s.selected, s.elapsed, s.scan_remaining, s.controls),
            (3, 0.25, 0.5, 0)
        );
    }
    #[test]
    fn late_scan_refusal_retains_selection_and_both_timing_prefixes() {
        let mut s = state();
        s.buttons = 0x800;
        let mut world = Provider {
            events: vec![],
            fail_scan: true,
        };
        assert!(advance(&mut s, 0., &mut world).is_err());
        assert_eq!(
            (s.selected, s.elapsed, s.scan_remaining, s.controls),
            (1, 0.25, -0.5, 2)
        );
    }
}
