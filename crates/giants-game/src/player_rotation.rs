//! Acquired manual heading/look helpers, with explicit supplied boundaries.
//! Uses the measured PC Double53 arithmetic profile. Automatic target heading,
//! input preparation and full character/world scheduling remain separate.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct State {
    pub heading: f32,
    pub look: f32,
    pub rate: f32,
    /// Produced by the entered primary prefix, before this helper runs.
    pub previous_rate: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Parameters {
    pub acceleration: f32,
    pub maximum_rate: f32,
    pub deceleration: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Input {
    pub turn: i8,
    pub look: i8,
    pub seconds: f64,
    pub sensitivity: f32,
    pub local_owner: bool,
    pub paused: bool,
    pub automatic_heading: bool,
}

/// Evaluate the two helpers without mutating caller state on refusal.
/// The caller supplies actual object time and input preferences, not outcomes.
pub fn advance(mut state: State, p: Parameters, i: Input) -> Result<State, &'static str> {
    if i.automatic_heading
        || ![state.heading, state.look, state.rate, state.previous_rate]
            .into_iter()
            .all(f32::is_finite)
        || ![
            p.acceleration,
            p.maximum_rate,
            p.deceleration,
            i.sensitivity,
        ]
        .into_iter()
        .all(|v| v.is_finite() && v >= 0.)
        || !i.seconds.is_finite()
        || i.seconds < 0.
        || !(i.seconds as f32).is_finite()
    {
        return Err("Player rotation outside acquired finite manual profile");
    }
    if !i.paused {
        let intent = (f64::from(i.turn) * 0.03125 * f64::from(i.sensitivity)) as f32;
        let step = (i.seconds * f64::from(p.acceleration) * f64::from(intent)) as f32;
        let target = intent * p.maximum_rate;
        if !intent.is_finite() || !step.is_finite() || !target.is_finite() {
            return Err("Player rotation rate exceeds acquired finite profile");
        }
        if target > 0. {
            if state.rate < 0. {
                state.rate = 0.;
            }
            if state.rate < target {
                state.rate = (state.rate + step).min(target);
            } else if target < state.rate {
                let wide = f64::from(state.rate) - i.seconds * f64::from(p.deceleration);
                state.rate = if wide < f64::from(target) {
                    target
                } else {
                    wide as f32
                };
            }
        } else if target < 0. {
            if state.rate > 0. {
                state.rate = 0.;
            }
            if target < state.rate {
                state.rate = (state.rate + step).max(target);
            } else if state.rate < target {
                let wide = f64::from(state.rate) + i.seconds * f64::from(p.deceleration);
                state.rate = if f64::from(target) < wide {
                    target
                } else {
                    wide as f32
                };
            }
        } else if state.rate > 0. {
            let wide = f64::from(state.rate) - i.seconds * f64::from(p.deceleration);
            state.rate = if wide < 0. { 0. } else { wide as f32 };
        } else if state.rate < 0. {
            let wide = f64::from(state.rate) + i.seconds * f64::from(p.deceleration);
            state.rate = if wide > 0. { 0. } else { wide as f32 };
        }
        let delta =
            ((f64::from(state.previous_rate) + f64::from(state.rate)) * i.seconds * 0.5) as f32;
        let heading = f64::from(state.heading) + f64::from(delta);
        // Upper correction compares the wide sum and retains it through subtraction.
        // Lower correction reads the stored float. Each correction runs only once.
        state.heading = if heading >= 360. {
            (heading - 360.) as f32
        } else {
            heading as f32
        };
        if state.heading < 0. {
            state.heading += 360.;
        }
    }
    if i.look != 0 {
        let look = f64::from(i.look)
            * 0.03125
            * f64::from(i.sensitivity)
            * f64::from(i.seconds as f32)
            * 90.
            + f64::from(state.look);
        state.look = look as f32;
        if i.local_owner && look > 80. {
            state.look = 80.;
        }
        if state.look < -90. {
            state.look = -90.;
        }
    }
    if ![state.heading, state.look, state.rate]
        .into_iter()
        .all(f32::is_finite)
    {
        return Err("Player rotation result exceeds acquired finite profile");
    }
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pause_preserves_heading_but_nonzero_look_still_clamps() {
        let state = State {
            heading: 720.,
            look: 100.,
            rate: 20.,
            previous_rate: -10.,
        };
        let p = Parameters {
            acceleration: 360.,
            maximum_rate: 180.,
            deceleration: 120.,
        };
        let i = Input {
            turn: 32,
            look: 0,
            seconds: 1. / 60.,
            sensitivity: 1.,
            local_owner: true,
            paused: true,
            automatic_heading: false,
        };
        assert_eq!(advance(state, p, i).unwrap(), state);
        let changed = advance(state, p, Input { look: 1, ..i }).unwrap();
        assert_eq!(changed, State { look: 80., ..state });
        assert!(
            advance(
                state,
                p,
                Input {
                    automatic_heading: true,
                    ..i
                }
            )
            .is_err()
        );
    }
}
