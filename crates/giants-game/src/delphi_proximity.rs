//! Acquired state102 proximity continuation after Delphi's item/physical work.
//! The caller owns the current common population, anchor/placement services and
//! actual interaction/transport effects. This does not admit world interactions.

#[derive(Clone, Debug)]
pub struct State {
    pub remaining: f32,
    pub controls: u32,
    pub heading: f32,
    pub position: [f32; 3],
    /// Four signed axes followed by the independent command-button word.
    pub commands: [u8; 8],
}

#[derive(Clone, Copy, Debug)]
pub struct Input {
    pub owner_context: u32,
    /// Actual source identity for the required request, not a Session serial.
    pub owner_word: Option<u32>,
    /// Immutable acquired source scalar, supplied separately from world state.
    pub squared_distance_limit: f32,
}

/// Explicit owned component sequence; enclosing primary gates remain separate.
pub struct Context<'a> {
    pub items: crate::delphi_items::Context<'a>,
    pub squared_distance_limit: f32,
}

pub trait World<K> {
    fn authority(&mut self) -> Result<bool, String>;
    fn global_seconds(&mut self) -> Result<f64, String>;
    /// Current common-list order. Read only when the countdown crosses zero.
    fn actors(&mut self) -> Result<Vec<K>, String>;
    fn state(&mut self, actor: K) -> Result<u32, String>;
    fn context(&mut self, actor: K) -> Result<u32, String>;
    fn damage_and_limit(&mut self, actor: K) -> Result<(f32, f32), String>;
    /// Actual current reference/affine anchor; not an assumed actor position.
    fn anchor(&mut self, actor: K) -> Result<[f32; 3], String>;
    fn network_guard(&mut self) -> Result<bool, String>;
    fn notify(&mut self, actor: K, owner_word: &mut u32) -> Result<(), String>;
    fn activate(&mut self, actor: K, owner_word: &mut u32) -> Result<(), String>;
    fn select(&mut self, actor: K) -> Result<(), String>;
    fn alignment_angle(&mut self, actor: K) -> Result<f32, String>;
    fn alignment_position(&mut self, state: &mut State) -> Result<(), String>;
    fn move_owner(&mut self, state: &mut State) -> Result<(), String>;
}

/// Preserve completed stores and provider effects on refusal; the caller must
/// quarantine replay. Finite timer/distance inputs and nonnegative elapsed time
/// are acquired. Rust errors are not native fault/invalid-input equivalence.
pub fn advance<K: Copy>(
    state: &mut State,
    input: Input,
    world: &mut impl World<K>,
) -> Result<(), String> {
    if !world.authority()? {
        return Ok(());
    }
    let seconds = world.global_seconds()?;
    if !seconds.is_finite() || seconds < 0. || !state.remaining.is_finite() {
        return Err("Delphi proximity countdown outside acquired finite profile".into());
    }
    let remaining = f64::from(state.remaining) - seconds;
    state.remaining = remaining as f32;
    // Compare the wide result, after storing its narrowed value.
    if remaining >= 0. {
        return Ok(());
    }
    for actor in world.actors()? {
        if world.state(actor)? != 102 {
            continue;
        }
        let context = world.context(actor)?;
        if context != input.owner_context && context != 0 {
            continue;
        }
        let (damage, limit) = world.damage_and_limit(actor)?;
        if !damage.is_finite() || !limit.is_finite() {
            return Err("Delphi proximity candidate outside acquired finite profile".into());
        }
        if damage >= limit {
            continue;
        }
        let point = world.anchor(actor)?;
        if !point.into_iter().chain(state.position).all(f32::is_finite)
            || !input.squared_distance_limit.is_finite()
            || input.squared_distance_limit < 0.
        {
            return Err("Delphi proximity anchor outside acquired finite profile".into());
        }
        let delta: [f64; 3] =
            std::array::from_fn(|i| f64::from(point[i]) - f64::from(state.position[i]));
        let squared = (delta[1] * delta[1] + delta[0] * delta[0]) + delta[2] * delta[2];
        if squared > f64::from(input.squared_distance_limit) {
            continue;
        }
        if state.controls & 1 == 0 {
            if world.network_guard()? {
                let mut owner_word = input
                    .owner_word
                    .ok_or("Delphi proximity request source identity absent")?;
                world.notify(actor, &mut owner_word)?;
                world.activate(actor, &mut owner_word)?;
            } else if world.authority()? {
                world.select(actor)?;
                let heading = world.alignment_angle(actor)?;
                if !heading.is_finite() {
                    return Err("Delphi proximity alignment angle is not finite".into());
                }
                state.heading = heading;
                world.alignment_position(state)?;
                world.move_owner(state)?;
                state.commands[2] = 0;
                state.commands[3] = 0;
                state.commands[0] = 0;
            }
        }
        // The first nearby candidate also terminates the scan when bit0 is set.
        state.remaining = 0.5;
        return Ok(());
    }
    state.controls &= !1;
    state.remaining = 0.5;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Provider {
        seconds: f64,
        population: bool,
        fail_move: bool,
        reads: Vec<&'static str>,
    }
    impl World<u8> for Provider {
        fn authority(&mut self) -> Result<bool, String> {
            self.reads.push("authority");
            Ok(true)
        }
        fn global_seconds(&mut self) -> Result<f64, String> {
            self.reads.push("seconds");
            Ok(self.seconds)
        }
        fn actors(&mut self) -> Result<Vec<u8>, String> {
            self.reads.push("actors");
            if self.population {
                Ok(vec![0, 1])
            } else {
                Err("population unavailable".into())
            }
        }
        fn state(&mut self, actor: u8) -> Result<u32, String> {
            if actor == 0 {
                Ok(102)
            } else {
                Err("later actor unavailable".into())
            }
        }
        fn context(&mut self, _: u8) -> Result<u32, String> {
            Ok(0)
        }
        fn damage_and_limit(&mut self, _: u8) -> Result<(f32, f32), String> {
            Ok((0., 100.))
        }
        fn anchor(&mut self, _: u8) -> Result<[f32; 3], String> {
            Ok([0.; 3])
        }
        fn network_guard(&mut self) -> Result<bool, String> {
            Ok(false)
        }
        fn notify(&mut self, _: u8, _: &mut u32) -> Result<(), String> {
            unreachable!()
        }
        fn activate(&mut self, _: u8, _: &mut u32) -> Result<(), String> {
            unreachable!()
        }
        fn select(&mut self, _: u8) -> Result<(), String> {
            self.reads.push("select");
            Ok(())
        }
        fn alignment_angle(&mut self, _: u8) -> Result<f32, String> {
            Ok(90.)
        }
        fn alignment_position(&mut self, state: &mut State) -> Result<(), String> {
            assert_eq!(state.heading, 90.);
            state.position = [1., 2., 3.];
            Ok(())
        }
        fn move_owner(&mut self, _: &mut State) -> Result<(), String> {
            if self.fail_move {
                Err("placement unavailable".into())
            } else {
                Ok(())
            }
        }
    }
    fn provider() -> Provider {
        Provider {
            seconds: 0.25,
            population: true,
            fail_move: false,
            reads: Vec::new(),
        }
    }
    fn state() -> State {
        State {
            remaining: 0.,
            controls: 0,
            heading: 0.,
            position: [0.; 3],
            commands: [7; 8],
        }
    }
    fn input() -> Input {
        Input {
            owner_context: 3,
            owner_word: Some(123),
            squared_distance_limit: 0.,
        }
    }
    #[test]
    fn zero_countdown_skips_population_but_wide_negative_zero_does_not() {
        let mut p = provider();
        p.population = false;
        let mut s = state();
        s.remaining = 0.25;
        advance(&mut s, input(), &mut p).unwrap();
        assert_eq!(p.reads, ["authority", "seconds"]);
        assert_eq!(s.remaining.to_bits(), 0);

        p.reads.clear();
        p.seconds = f64::from(f32::from_bits(1)) * 0.5;
        assert!(advance(&mut s, input(), &mut p).is_err());
        assert_eq!(s.remaining.to_bits(), (-0_f32).to_bits());
        assert_eq!(p.reads, ["authority", "seconds", "actors"]);
    }
    #[test]
    fn first_nearby_active_candidate_prevents_later_actor_access() {
        let mut p = provider();
        let mut s = state();
        s.controls = 1;
        advance(&mut s, input(), &mut p).unwrap();
        assert_eq!(s.remaining, 0.5);
        assert_eq!(s.controls, 1);
        assert_eq!(s.commands, [7; 8]);
        assert!(!p.reads.contains(&"select"));
    }
    #[test]
    fn late_placement_failure_keeps_alignment_without_clearing_commands() {
        let mut p = provider();
        p.fail_move = true;
        let mut s = state();
        assert!(advance(&mut s, input(), &mut p).is_err());
        assert!(p.reads.contains(&"select"));
        assert_eq!(s.remaining, -0.25);
        assert_eq!(s.heading, 90.);
        assert_eq!(s.position, [1., 2., 3.]);
        assert_eq!(s.commands, [7; 8]);
    }
}
