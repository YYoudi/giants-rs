//! Ordinary state23 mode-zero entry up to its perception continuation.
//! This is neither a complete behavior callback nor mission readiness.
#[derive(Clone, Copy, Debug)]
pub struct Target {
    pub state: u32,
    pub flags: u32,
}
#[derive(Clone, Copy, Debug)]
pub struct Player {
    pub state: u32,
    pub control: u32,
    pub position: [f32; 3],
    pub motion: [f32; 2],
}
#[derive(Clone, Copy, Debug)]
pub struct State {
    pub class: i32,
    pub context: u32,
    pub mode: u32,
    pub action: u32,
    pub flags: u32,
    pub control: u32,
    pub packed: u32,
    pub position: [f32; 3],
    pub parameter: f32,
    pub active_limit: f32,
    pub saved_parameter: f32,
    pub retained_timer: f32,
    pub target: Option<Target>,
}
#[derive(Clone, Copy, Debug)]
pub struct Context {
    pub paused: bool,
    pub native_mode: u32,
    pub parent_state: Option<u32>,
    pub player: Option<Player>,
    pub delta: f64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Skipped,
    /// The source requests the cue before returning to its perception phase.
    /// The caller still owes the sound service and the rest of the callback.
    Continue {
        cue: u32,
    },
}
/// Ordered state snapshots around the required sound service. Producing a plan
/// acknowledges no service and publishes no resident or mission state.
#[derive(Clone, Debug)]
pub struct Plan {
    pub outcome: Outcome,
    pub before_sound: Option<State>,
    pub after: State,
}
/// Shared ordinary action-zero prefix, acquired by the mode0 and mode32
/// consumers. No mode selection, sound service or context reassignment here.
pub(crate) fn ordinary_prefix(
    state: State,
    context: Context,
) -> Result<Option<State>, &'static str> {
    if context.native_mode == 4 {
        return Err("state23 entry requires the network callback gate service");
    }
    if context.paused || state.control & 1 != 0 || matches!(context.parent_state, Some(1 | 76)) {
        return Ok(None);
    }
    if !state
        .position
        .into_iter()
        .chain([
            state.parameter,
            state.active_limit,
            state.saved_parameter,
            state.retained_timer,
        ])
        .all(f32::is_finite)
        || !context.delta.is_finite()
        || context
            .player
            .is_some_and(|p| !p.position.into_iter().chain(p.motion).all(f32::is_finite))
    {
        return Err("non-finite state23 entry input");
    }
    let mut output = state;
    let remaining = f64::from(output.retained_timer) - context.delta;
    output.retained_timer = if remaining < 0. { 0. } else { remaining as f32 };
    match context.player {
        Some(p) if p.state == 2 && p.control & 0x0200_0000 != 0 => {
            if p.motion.into_iter().any(|v| v != 0.) {
                let delta = std::array::from_fn::<_, 3, _>(|i| {
                    f64::from(output.position[i]) - f64::from(p.position[i])
                });
                let distance = (delta[0] * delta[0] + delta[1] * delta[1]) + delta[2] * delta[2];
                if distance < 57_600. {
                    output.retained_timer =
                        (context.delta * 2. + f64::from(output.retained_timer)) as f32;
                }
            }
        }
        _ => output.retained_timer = 0.,
    }
    if output
        .target
        .is_some_and(|t| t.flags & 0x8000 != 0 || !matches!(t.state, 47 | 49))
    {
        output.target = None;
    }
    output.flags &= 0xfcff_ffff;
    output.packed &= 0xff00_ff00;
    Ok(Some(output))
}
pub fn plan(state: State, context: Context) -> Result<Plan, &'static str> {
    if !matches!(state.class, 73 | 230) || state.mode != 0 || state.action != 0 {
        return Err("state23 entry requires the acquired ordinary mode-zero profile");
    }
    let Some(mut output) = ordinary_prefix(state, context)? else {
        return Ok(Plan {
            outcome: Outcome::Skipped,
            before_sound: None,
            after: state,
        });
    };
    output.context = 100;
    let before_sound = Some(output);
    output.active_limit = 0.;
    let cue = if output.class == 73 {
        output.mode = 31;
        output.saved_parameter = output.parameter;
        1258
    } else {
        output.mode = 61;
        1235
    };
    Ok(Plan {
        outcome: Outcome::Continue { cue },
        before_sound,
        after: output,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn state() -> State {
        State {
            class: 73,
            context: 7,
            mode: 0,
            action: 0,
            flags: u32::MAX,
            control: 0,
            packed: u32::MAX,
            position: [0.; 3],
            parameter: 45.,
            active_limit: 100.,
            saved_parameter: 0.,
            retained_timer: 1.,
            target: None,
        }
    }
    fn context() -> Context {
        Context {
            paused: false,
            native_mode: 0,
            parent_state: None,
            player: None,
            delta: 0.1,
        }
    }
    #[test]
    fn skipped_entry_keeps_context_and_unsupported_profile_is_atomic() {
        let s = state();
        assert_eq!(
            plan(
                s,
                Context {
                    paused: true,
                    ..context()
                }
            )
            .unwrap()
            .outcome,
            Outcome::Skipped
        );
        assert_eq!(s.context, 7);
        assert_eq!(s.mode, 0);
        assert!(
            plan(
                s,
                Context {
                    native_mode: 4,
                    ..context()
                }
            )
            .is_err()
        );
        assert_eq!(s.context, 7);
    }
    #[test]
    fn moving_mecc_uses_strict_three_dimensional_radius() {
        for (z, expected) in [(239., 1.1f32), (240., 0.9f32), (241., 0.9f32)] {
            let s = state();
            let ctx = Context {
                player: Some(Player {
                    state: 2,
                    control: 0x0200_0000,
                    position: [0., 0., z],
                    motion: [1., 0.],
                }),
                ..context()
            };
            let prepared = plan(s, ctx).unwrap();
            assert_eq!(prepared.outcome, Outcome::Continue { cue: 1258 });
            let before = prepared.before_sound.unwrap();
            assert_eq!(before.mode, 0);
            assert_eq!(before.context, 100);
            assert_eq!(before.active_limit, 100.);
            assert_eq!(before.saved_parameter, 0.);
            let s = prepared.after;
            assert_eq!(s.retained_timer, expected);
            assert_eq!(s.context, 100);
            assert_eq!(s.saved_parameter, 45.);
        }
    }
}
