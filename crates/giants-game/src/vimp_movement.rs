//! Acquired mode1 scalar/command prefix. Earlier growth, route and admission,
//! and later active response/orientation, physics and ANM remain caller phases.
use crate::{audio_parameters::Random, vimp_living_scalars as scalars, vimp_wander};
#[derive(Clone, Copy, Debug)]
pub struct State {
    pub uid: u32,
    pub position: [f32; 3],
    pub heading: f32,
    pub target: [f32; 3],
    pub damage: f32,
    pub scale: f32,
    pub gait: u32,
    pub speed: f32,
    pub parameter_6c: f32,
    pub parameter_70: f32,
    pub commands: [u8; 8],
    /// Supplied nonzero-header predicate; this is not a collision producer.
    pub obstructed: bool,
    pub recovery_timer: f32,
    pub retry_timer: f32,
    pub actor_delay: f32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Clock {
    Actor,
    Global,
}
pub trait Services {
    fn seconds(&mut self, state: &State, clock: Clock) -> Result<f64, String>;
}
fn seconds(s: &State, p: &mut impl Services, clock: Clock) -> Result<f64, String> {
    let value = p.seconds(s, clock)?;
    if !value.is_finite() || value < 0. {
        return Err("Vimp movement clock outside finite nonnegative profile".into());
    }
    Ok(value)
}
/// Ordered prefix stores survive a later required-clock failure. The producer
/// gate is an earlier caller result, not inferred from active response state.
pub fn advance(
    s: &mut State,
    produce_commands: bool,
    globals: &giants_formats::global_data::GlobalData,
    random: &mut Random,
    services: &mut impl Services,
) -> Result<(), String> {
    let p = scalars::movement_parameters(globals, s.damage, s.scale, s.gait)?;
    s.parameter_6c = p.parameter_6c;
    s.parameter_70 = p.parameter_70;
    let dt = seconds(s, services, Clock::Actor)?;
    s.speed = scalars::approach_speed(s.speed, p.desired_speed, dt, p.acceleration)?;
    s.commands = [0; 8];
    if !produce_commands {
        return Ok(());
    }
    if !s
        .position
        .into_iter()
        .chain(s.target)
        .chain([s.heading])
        .all(f32::is_finite)
        || s.heading.abs() > 1_000_000.
    {
        return Err("Vimp command pose outside finite acquired profile".into());
    }
    s.commands[0] = vimp_wander::turn_command(s.position, s.heading, s.target);
    if s.obstructed {
        if !s.retry_timer.is_finite() {
            return Err("Vimp retry timer outside finite profile".into());
        }
        let remaining = f64::from(s.retry_timer) - seconds(s, services, Clock::Global)?;
        s.retry_timer = remaining as f32;
        // This caller uses strict negativity, unlike the active wait's <=0.
        if remaining < 0. {
            s.recovery_timer = (f64::from(s.uid % 10) + 2.) as f32;
            s.actor_delay = 0.;
            s.retry_timer = vimp_wander::interval(random, 0.5, 1.5);
        } else {
            s.recovery_timer = 0.;
        }
    } else {
        // The original draw precedes the recovery clear, then its result stores.
        let retry = vimp_wander::interval(random, 0.5, 1.5);
        s.recovery_timer = 0.;
        s.retry_timer = retry;
    }
    s.commands[2] = vimp_wander::forward_command(s.position, s.target);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Provider {
        fail: Option<Clock>,
        calls: Vec<Clock>,
        actor: f64,
        global: f64,
    }
    impl Services for Provider {
        fn seconds(&mut self, s: &State, clock: Clock) -> Result<f64, String> {
            self.calls.push(clock);
            // Native parameters precede either clock; command clear follows
            // Actor, and the turn byte precedes Global.
            assert_eq!((s.parameter_6c, s.parameter_70), (4., 5.));
            if self.fail == Some(clock) {
                Err("required clock failed".into())
            } else {
                Ok(if clock == Clock::Actor {
                    self.actor
                } else {
                    self.global
                })
            }
        }
    }
    fn fixture() -> (State, giants_formats::global_data::GlobalData, Provider) {
        let mut g = giants_formats::global_data::GlobalData {
            words: vec![0; 0xa00 / 4],
            array_words: vec![],
        };
        for (at, v) in [
            (0x940, 3f32),
            (0x944, 4.),
            (0x948, 5.),
            (0x994, 0.5),
            (0x6dc, 10.),
        ] {
            g.words[at / 4] = v.to_bits();
        }
        (
            State {
                uid: u32::MAX,
                position: [0.; 3],
                heading: 0.,
                target: [100., 0., 0.],
                damage: 0.,
                scale: 1.,
                gait: 0,
                speed: 17.,
                parameter_6c: 0.,
                parameter_70: 0.,
                commands: [7; 8],
                obstructed: true,
                recovery_timer: 3.,
                retry_timer: 0.5,
                actor_delay: 4.,
            },
            g,
            Provider {
                fail: None,
                calls: vec![],
                actor: 0.,
                global: 0.5,
            },
        )
    }
    #[test]
    fn suppressed_commands_still_advance_scalars_and_clear_all_bytes_without_timer_or_draw() {
        let (mut s, g, mut p) = fixture();
        let mut r = Random(1);
        advance(&mut s, false, &g, &mut r, &mut p).unwrap();
        assert_eq!(s.commands, [0; 8]);
        assert_eq!(p.calls, [Clock::Actor]);
        assert_eq!(
            (s.recovery_timer, s.retry_timer, s.actor_delay),
            (3., 0.5, 4.)
        );
        assert_eq!(r.0, 1);
    }
    #[test]
    fn obstruction_uses_separate_global_clock_and_strict_negative_expiry_with_unsigned_uid() {
        let (mut s, g, mut p) = fixture();
        let mut r = Random(1);
        advance(&mut s, true, &g, &mut r, &mut p).unwrap();
        assert_eq!(
            (s.retry_timer, s.recovery_timer, s.actor_delay),
            (0., 0., 4.)
        );
        assert_eq!(r.0, 1);
        p.global = 0.1;
        advance(&mut s, true, &g, &mut r, &mut p).unwrap();
        assert_eq!((s.recovery_timer, s.actor_delay), (7., 0.));
        let mut expected = Random(1);
        expected.next_15();
        assert_eq!(r.0, expected.0);
        assert_eq!(s.speed, 17.);
        assert_eq!(
            p.calls,
            [Clock::Actor, Clock::Global, Clock::Actor, Clock::Global]
        );
    }
    #[test]
    fn clock_errors_keep_distinct_prefixes_and_never_replay_random() {
        for clock in [Clock::Actor, Clock::Global] {
            let (mut s, g, mut p) = fixture();
            p.fail = Some(clock);
            let mut r = Random(1);
            assert!(advance(&mut s, true, &g, &mut r, &mut p).is_err());
            assert_eq!((s.parameter_6c, s.parameter_70), (4., 5.));
            assert_eq!(
                (s.speed, s.retry_timer, s.recovery_timer, s.actor_delay),
                (17., 0.5, 3., 4.)
            );
            assert_eq!(
                s.commands,
                if clock == Clock::Actor {
                    [7; 8]
                } else {
                    [32, 0, 0, 0, 0, 0, 0, 0]
                }
            );
            assert_eq!(r.0, 1);
        }
    }
}
