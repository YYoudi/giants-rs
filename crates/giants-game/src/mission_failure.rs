//! Acquired PC mission-failure producer, separate from failure-menu ownership.
//! Prototype PS2 `mission_fail` is an unknown correspondence/search route.
//! PC debug-reference-1.1 entry 0x00497370; prior aliases remain in the private
//! identity-bound record. See docs/SYMBOL_NAVIGATION.md.
//! Supplied state is not a live-world initialization or availability receipt.

/// Actual shared failure state. Callers supply its lifecycle; there is no
/// default constructor asserting a clear mission or an initialized clock.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct State {
    pub failed: u32,
    pub elapsed: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Context {
    pub counter: i32,
    pub required: i32,
    pub completed: u32,
    /// The actual global frame sample, including a possible negative sample.
    pub seconds: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Completed/count-satisfied calls preserve both existing failure fields.
    Suppressed,
    Pending,
    /// The native tail enters its failure-menu owner here. This request does
    /// not acknowledge that owner, its flag resets, UI or restart effects.
    FailureMenuRequired,
}

impl State {
    /// One actual failure-producer invocation, not a general frame update.
    /// Preserve repeated calls and negative clock samples. The original stores
    /// rounded elapsed time but compares the wide sum against its strict delay.
    /// This bounded contract refuses nonfinite active arithmetic before writes;
    /// suppressed calls do not inspect or normalize the unused clock fields.
    pub fn request(&mut self, context: Context) -> Result<Outcome, &'static str> {
        if context.counter >= context.required || context.completed != 0 {
            return Ok(Outcome::Suppressed);
        }
        let sum = f64::from(self.elapsed) + f64::from(context.seconds);
        if !self.elapsed.is_finite() || !context.seconds.is_finite() || !(sum as f32).is_finite() {
            return Err("mission failure requires finite active clock arithmetic");
        }
        self.failed = 1;
        self.elapsed = sum as f32;
        Ok(if sum > 5. {
            Outcome::FailureMenuRequired
        } else {
            Outcome::Pending
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(seconds: f32) -> Context {
        Context {
            counter: 0,
            required: 1,
            completed: 0,
            seconds,
        }
    }

    #[test]
    fn signed_counts_and_completion_preserve_existing_failure() {
        for (counter, required, completed) in [(0, -1, 0), (1, 1, 0), (0, 1, 7)] {
            let mut state = State {
                failed: 9,
                elapsed: f32::from_bits(0x7fc00001),
            };
            assert_eq!(
                state.request(Context {
                    counter,
                    required,
                    completed,
                    seconds: f32::NAN
                }),
                Ok(Outcome::Suppressed)
            );
            assert_eq!(state.failed, 9);
            assert_eq!(state.elapsed.to_bits(), 0x7fc00001);
        }
        let mut state = State {
            failed: 9,
            elapsed: 0.,
        };
        assert_eq!(
            state.request(Context {
                counter: -1,
                required: 0,
                ..context(1.)
            }),
            Ok(Outcome::Pending)
        );
        assert_eq!(
            state,
            State {
                failed: 1,
                elapsed: 1.
            }
        );
    }

    #[test]
    fn strict_wide_delay_differs_from_rounded_storage() {
        let mut state = State {
            failed: 0,
            elapsed: 5.,
        };
        assert_eq!(state.request(context(0.)), Ok(Outcome::Pending));
        assert_eq!(
            state.request(context(f32::EPSILON)),
            Ok(Outcome::FailureMenuRequired)
        );
        assert_eq!(
            state,
            State {
                failed: 1,
                elapsed: 5.
            }
        );
    }

    #[test]
    fn repeated_invocations_and_negative_clock_are_preserved() {
        let mut state = State {
            failed: 0,
            elapsed: 4.,
        };
        assert_eq!(state.request(context(1.)), Ok(Outcome::Pending));
        assert_eq!(state.request(context(1.)), Ok(Outcome::FailureMenuRequired));
        assert_eq!(
            state,
            State {
                failed: 1,
                elapsed: 6.
            }
        );
        assert_eq!(state.request(context(-2.)), Ok(Outcome::Pending));
        assert_eq!(
            state,
            State {
                failed: 1,
                elapsed: 4.
            }
        );
    }

    #[test]
    fn invalid_active_arithmetic_is_not_published() {
        for (elapsed, seconds) in [(1., f32::INFINITY), (f32::MAX, f32::MAX)] {
            let mut state = State { failed: 9, elapsed };
            assert!(state.request(context(seconds)).is_err());
            assert_eq!(state, State { failed: 9, elapsed });
        }
    }

    #[test]
    #[ignore = "requires private identity-bound original mission failure controls"]
    fn original_failure_producer_controls() {
        let path = std::env::var("GIANTS_MISSION_FAILURE_CONTROLS").expect("private control path");
        let rows = std::fs::read_to_string(path).unwrap();
        let mut count = 0;
        for row in rows.lines() {
            let w: Vec<u32> = row
                .split_whitespace()
                .map(|v| u32::from_str_radix(v, 16).unwrap())
                .collect();
            assert_eq!(w.len(), 9);
            let mut state = State {
                failed: w[0],
                elapsed: f32::from_bits(w[1]),
            };
            let result = state
                .request(Context {
                    counter: w[2] as i32,
                    required: w[3] as i32,
                    completed: w[4],
                    seconds: f32::from_bits(w[5]),
                })
                .unwrap();
            assert_eq!(
                [
                    state.failed,
                    state.elapsed.to_bits(),
                    u32::from(result == Outcome::FailureMenuRequired)
                ],
                [w[6], w[7], w[8]]
            );
            count += 1;
        }
        assert!(count >= 50);
    }
}
