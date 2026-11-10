//! Acquired four-stage active-response block inside the Vimp primary. Earlier
//! movement/commands and later look, physics and ANM remain caller-owned phases.
use crate::{actor_session_predicates as predicates, audio_parameters::Random, vimp_response};
#[derive(Clone, Copy, Debug)]
pub struct State {
    pub owner_word: u32,
    pub position: [f32; 3],
    pub wander_target: [f32; 3],
    pub heading: f32,
    pub angular_speed: f32,
    pub previous_angular_speed: f32,
    /// Controls from the preceding movement producer; this block does not clear them.
    pub commands: [u8; 8],
    pub selected_heading: f32,
    pub turn_timer: f32,
    pub delay: f32,
    pub stage: u32,
    pub active: u32,
    pub kind: u32,
    pub priority: i32,
}
#[derive(Clone, Copy)]
pub struct Parameters {
    pub turn_rate: f32,
    pub wait_low: f32,
    pub wait_high: f32,
    pub delay_low: f32,
    pub delay_high: f32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    Heading(f32),
    Finished,
}
pub trait Services {
    fn session(
        &mut self,
        state: &State,
        query: vimp_response::Predicate,
    ) -> Result<predicates::Snapshot<'_>, String>;
    fn seconds(&mut self, state: &State) -> Result<f64, String>;
    /// Execute the acquired heading producer on current controls. Callback/owner
    /// effects must be published and retained by an eventual Session adapter.
    fn orient(&mut self, state: &mut State) -> Result<(), String>;
    fn action(&mut self, state: &mut State, action: Action) -> Result<(), String>;
}
fn predicate(
    s: &State,
    p: &mut impl Services,
    query: vimp_response::Predicate,
) -> Result<bool, String> {
    let snapshot = p.session(s, query)?;
    match query {
        vimp_response::Predicate::First => predicates::first(&snapshot, s.owner_word),
        vimp_response::Predicate::Second => predicates::second(&snapshot, s.owner_word),
    }
}
fn seconds(s: &State, p: &mut impl Services) -> Result<f64, String> {
    let t = p.seconds(s)?;
    if !t.is_finite() || t < 0. {
        return Err("Vimp active response clock outside finite nonnegative profile".into());
    }
    Ok(t)
}
fn turn(s: &mut State, parameters: Parameters, p: &mut impl Services) -> Result<(), String> {
    let amount = (seconds(s, p)? * f64::from(parameters.turn_rate)) as f32;
    if ![s.heading, s.selected_heading, amount]
        .into_iter()
        .all(f32::is_finite)
        || s.heading.abs() > 1_000_000.
        || s.selected_heading.abs() > 1_000_000.
    {
        return Err("Vimp active response angle outside finite profile".into());
    }
    s.heading = crate::actor_angle_approach::approach(s.heading, s.selected_heading, amount);
    Ok(())
}
/// Mutates the acquired prefix in order; errors retain earlier stores and draws.
/// This is the active block, not a primary, command producer or world invocation.
pub fn advance(
    s: &mut State,
    parameters: Parameters,
    random: &mut Random,
    p: &mut impl Services,
) -> Result<(), String> {
    if s.active == 0 {
        return Err("Vimp active response block requires an active response".into());
    }
    match s.stage {
        0 => {
            turn(s, parameters, p)?;
            if s.selected_heading == s.heading {
                s.turn_timer =
                    vimp_response::random_delay(random, parameters.wait_low, parameters.wait_high)?;
                s.stage = 1;
            }
        }
        1 => {
            if predicate(s, p, vimp_response::Predicate::First)? {
                if !s.turn_timer.is_finite() {
                    return Err("Vimp response wait outside finite profile".into());
                }
                let remaining = f64::from(s.turn_timer) - seconds(s, p)?;
                s.turn_timer = remaining as f32;
                if remaining <= 0. {
                    s.stage = 2;
                    if !s
                        .position
                        .into_iter()
                        .chain(s.wander_target)
                        .all(f32::is_finite)
                    {
                        return Err("Vimp response bearing outside finite position profile".into());
                    }
                    let bearing = crate::actor_volume::heading_wide(s.position, s.wander_target);
                    if !bearing.is_finite() {
                        return Err("Vimp response bearing outside finite profile".into());
                    }
                    s.selected_heading = bearing as f32;
                    if bearing > 360. {
                        s.selected_heading = (bearing - 360.) as f32;
                    } else if bearing < 0. {
                        s.selected_heading = (bearing + 360.) as f32;
                    }
                    if predicate(s, p, vimp_response::Predicate::Second)? {
                        p.action(s, Action::Heading(s.selected_heading))?;
                    }
                }
            }
        }
        2 => {
            turn(s, parameters, p)?;
            if s.selected_heading == s.heading {
                s.delay = vimp_response::random_delay(
                    random,
                    parameters.delay_low,
                    parameters.delay_high,
                )?;
                s.stage = 3;
            }
        }
        3 => {
            p.orient(s)?;
            if predicate(s, p, vimp_response::Predicate::First)? {
                if !s.delay.is_finite() {
                    return Err("Vimp response delay outside finite profile".into());
                }
                let remaining = f64::from(s.delay) - seconds(s, p)?;
                s.delay = remaining as f32;
                if remaining <= 0. {
                    s.kind = 0;
                    s.active = 0;
                    s.priority = 0;
                    if predicate(s, p, vimp_response::Predicate::Second)? {
                        p.action(s, Action::Finished)?;
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Provider {
        seconds: f64,
        blocked: bool,
        notify: bool,
        orient_error: bool,
        calls: Vec<&'static str>,
        actions: Vec<Action>,
    }
    impl Services for Provider {
        fn session(
            &mut self,
            _: &State,
            q: vimp_response::Predicate,
        ) -> Result<predicates::Snapshot<'_>, String> {
            self.calls.push(match q {
                vimp_response::Predicate::First => "first",
                vimp_response::Predicate::Second => "second",
            });
            Ok(predicates::Snapshot {
                mode: if self.blocked || self.notify { 4 } else { 0 },
                declared_count: 0,
                current_key: 0,
                slots: &[],
            })
        }
        fn seconds(&mut self, _: &State) -> Result<f64, String> {
            self.calls.push("clock");
            Ok(self.seconds)
        }
        fn orient(&mut self, s: &mut State) -> Result<(), String> {
            self.calls.push("orient");
            s.heading = 45.;
            if self.orient_error {
                Err("required heading service failed after write".into())
            } else {
                Ok(())
            }
        }
        fn action(&mut self, s: &mut State, action: Action) -> Result<(), String> {
            self.calls.push("action");
            self.actions.push(action);
            s.priority = 91;
            Err("required action failed after write".into())
        }
    }
    fn fixture(stage: u32) -> (State, Parameters, Provider) {
        (
            State {
                owner_word: 0,
                position: [0.; 3],
                wander_target: [1., 0., 0.],
                heading: 0.,
                angular_speed: 0.,
                previous_angular_speed: 0.,
                commands: [7, 0, 4, 0, 1, 2, 3, 4],
                selected_heading: 0.,
                turn_timer: 0.25,
                delay: 0.25,
                stage,
                active: 1,
                kind: 2,
                priority: 5,
            },
            Parameters {
                turn_rate: 60.,
                wait_low: 2.,
                wait_high: 2.,
                delay_low: 3.,
                delay_high: 3.,
            },
            Provider {
                seconds: 0.25,
                blocked: false,
                notify: false,
                orient_error: false,
                calls: Vec::new(),
                actions: Vec::new(),
            },
        )
    }
    #[test]
    fn pending_360_is_not_rewritten_to_normalized_zero_and_equal_bounds_still_draw() {
        let (mut s, parameters, mut p) = fixture(0);
        s.selected_heading = 360.;
        let mut random = Random(1);
        advance(&mut s, parameters, &mut random, &mut p).unwrap();
        assert_eq!(s.heading, 0.);
        assert_eq!(s.selected_heading, 360.);
        assert_eq!(s.stage, 0);
        assert_eq!(random.0, 1);
        s.selected_heading = 0.;
        advance(&mut s, parameters, &mut random, &mut p).unwrap();
        let mut expected = Random(1);
        expected.next_15();
        assert_eq!(random.0, expected.0);
        assert_eq!((s.stage, s.turn_timer), (1, 2.));
        assert_eq!(s.commands, [7, 0, 4, 0, 1, 2, 3, 4]);
    }
    #[test]
    fn zero_expires_but_positive_wide_remainder_rounded_to_zero_does_not() {
        let (mut s, parameters, mut p) = fixture(1);
        let mut random = Random(1);
        advance(&mut s, parameters, &mut random, &mut p).unwrap();
        assert_eq!((s.stage, s.turn_timer, s.selected_heading), (2, 0., 90.));
        assert_eq!(random.0, 1);
        let (mut s, parameters, mut p) = fixture(1);
        s.turn_timer = f32::from_bits(1);
        p.seconds = f64::from(s.turn_timer) * 0.75;
        advance(&mut s, parameters, &mut random, &mut p).unwrap();
        assert_eq!(s.turn_timer.to_bits(), 0);
        assert_eq!(s.stage, 1);
        assert_eq!(p.calls, ["first", "clock"]);
    }
    #[test]
    fn required_action_failure_retains_transition_and_callback_writes() {
        let (mut s, parameters, mut p) = fixture(1);
        s.owner_word = 255;
        p.notify = true;
        let mut random = Random(1);
        assert!(advance(&mut s, parameters, &mut random, &mut p).is_err());
        assert_eq!(s.stage, 2);
        assert_eq!(s.selected_heading, 90.);
        assert_eq!(s.priority, 91);
        assert_eq!(p.actions, [Action::Heading(90.)]);
        assert_eq!(random.0, 1);
        assert_eq!(p.calls, ["first", "clock", "second", "action"]);
    }
    #[test]
    fn orientation_precedes_gate_and_error_and_blocked_timer_are_retained() {
        let (mut s, parameters, mut p) = fixture(3);
        let mut random = Random(1);
        p.orient_error = true;
        assert!(advance(&mut s, parameters, &mut random, &mut p).is_err());
        assert_eq!(s.heading, 45.);
        assert_eq!(s.delay, 0.25);
        assert_eq!(p.calls, ["orient"]);
        p.orient_error = false;
        p.blocked = true;
        p.calls.clear();
        advance(&mut s, parameters, &mut random, &mut p).unwrap();
        assert_eq!(s.active, 1);
        assert_eq!(s.delay, 0.25);
        assert_eq!(p.calls, ["orient", "first"]);
        p.blocked = false;
        p.calls.clear();
        advance(&mut s, parameters, &mut random, &mut p).unwrap();
        assert_eq!((s.kind, s.active, s.priority), (0, 0, 0));
        assert_eq!(s.stage, 3);
        assert_eq!(s.delay, 0.);
        assert_eq!(p.calls, ["orient", "first", "clock", "second"]);
        assert_eq!(random.0, 1);
        assert_eq!(s.commands, [7, 0, 4, 0, 1, 2, 3, 4]);
    }
    #[test]
    fn finish_action_failure_keeps_clears_and_the_callback_changes() {
        let (mut s, parameters, mut p) = fixture(3);
        s.owner_word = 255;
        p.notify = true;
        let mut random = Random(1);
        assert!(advance(&mut s, parameters, &mut random, &mut p).is_err());
        assert_eq!((s.kind, s.active, s.priority), (0, 0, 91));
        assert_eq!(s.stage, 3);
        assert_eq!(s.delay, 0.);
        assert_eq!(p.actions, [Action::Finished]);
        assert_eq!(p.calls, ["orient", "first", "clock", "second", "action"]);
        assert_eq!(random.0, 1);
    }
}
