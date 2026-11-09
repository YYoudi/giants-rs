//! Acquired target decision and size pursuit within the primary. Threshold
//! production and complete primary/world scheduling remain separate.
use crate::object_time_precision;

#[derive(Clone, Copy, Debug)]
pub struct DecisionState {
    /// Source state threshold input; its producer and gameplay meaning are open.
    pub trigger: f32,
    pub target: f32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecisionEvent {
    /// Existing actor-sound service invocation, not a device-play guarantee.
    Sound(u32),
    SoundNotification(u32),
    TargetNotification([u8; 2]),
}
/// Acquired threshold and wrapped target decision before size pursuit. The
/// observer receives the state at each original service/notification boundary.
pub fn decide(
    state: &mut DecisionState,
    context: &crate::kabuto_geometry_mode::NotificationContext<'_>,
    mut observe: impl FnMut(DecisionEvent, DecisionState),
) -> Result<bool, &'static str> {
    if state.trigger.partial_cmp(&100.).is_none_or(|o| o.is_lt()) {
        return Ok(false);
    }
    if !state.target.is_finite() {
        return Err("Kabuto target decision outside finite size profile");
    }
    let index = (crate::arithmetic::integer_low_word(f64::from(state.target) + 1.) as u8) & 3;
    let next = f32::from(index);
    let changed = state.target != next;
    if changed {
        let eligible = context.eligible()?;
        observe(DecisionEvent::Sound(0x6e4), *state);
        if eligible {
            observe(DecisionEvent::SoundNotification(0x6e4), *state);
        }
        state.target = next;
        if eligible {
            observe(DecisionEvent::TargetNotification([index, 0]), *state);
        }
    }
    state.trigger = 0.;
    Ok(changed)
}

pub struct Frame<'a> {
    /// Output of the preceding decision or an explicit isolated control input.
    /// This is not a keyboard or feeding rule.
    pub target: f32,
    pub global_seconds: f32,
    pub areas: &'a [object_time_precision::Area],
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    pub current: f32,
    /// The source invokes its size refresh whenever the incoming sizes differ,
    /// including a zero timestep that leaves the current size unchanged.
    pub refreshed: bool,
}

/// Preserve the wide actor-local timestep through the acquired Double53 step.
/// Equal sizes bypass time consumption and preserve the current word's sign.
pub fn approach(current: f32, target: f32, seconds: f64) -> Result<View, &'static str> {
    if !current.is_finite() || !target.is_finite() {
        return Err("Kabuto growth requires finite current and target sizes");
    }
    if current == target {
        return Ok(View {
            current,
            refreshed: false,
        });
    }
    if !seconds.is_finite() || seconds < 0. {
        return Err("Kabuto growth requires a finite nonnegative timestep");
    }
    let step = seconds * f64::from(0.4_f32);
    let value = if current < target {
        f64::from(current) + step
    } else {
        f64::from(current) - step
    };
    let stored = value as f32;
    let current = if (current < target && value > f64::from(target))
        || (current > target && value < f64::from(target))
    {
        target
    } else {
        stored
    };
    Ok(View {
        current,
        refreshed: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn context() -> crate::kabuto_geometry_mode::NotificationContext<'static> {
        crate::kabuto_geometry_mode::NotificationContext {
            session_mode: 4,
            peer_ids: &[],
            local_peer: 0,
            owner_word: 255,
        }
    }
    #[test]
    fn threshold_bypasses_target_and_nan_trigger_preserves_words() {
        for trigger in [99.99999, f32::NEG_INFINITY, f32::NAN, -0.] {
            let mut state = DecisionState {
                trigger,
                target: f32::NAN,
            };
            assert!(
                !decide(&mut state, &context(), |_, _| panic!(
                    "inactive decision emitted"
                ))
                .unwrap()
            );
            assert_eq!(state.trigger.to_bits(), trigger.to_bits());
            assert!(state.target.is_nan());
        }
        let mut state = DecisionState {
            trigger: 100.,
            target: f32::NAN,
        };
        assert!(
            decide(&mut state, &context(), |_, _| panic!(
                "invalid decision emitted"
            ))
            .is_err()
        );
        assert_eq!(state.trigger, 100.);
    }
    #[test]
    fn target_wraps_low_byte_with_wide_conversion_and_ordered_notifications() {
        for (target, next) in [(0., 1.), (3., 0.), (-2., 3.), (16777216., 1.)] {
            let mut state = DecisionState {
                trigger: 100.,
                target,
            };
            let mut events = Vec::new();
            assert!(
                decide(&mut state, &context(), |event, state| events.push((
                    event,
                    state.target,
                    state.trigger
                )))
                .unwrap()
            );
            assert_eq!(state.target, next);
            assert_eq!(state.trigger.to_bits(), 0);
            assert_eq!(
                events,
                vec![
                    (DecisionEvent::Sound(0x6e4), target, 100.),
                    (DecisionEvent::SoundNotification(0x6e4), target, 100.),
                    (
                        DecisionEvent::TargetNotification([next as u8, 0]),
                        next,
                        100.
                    ),
                ]
            );
        }
    }
    #[test]
    fn pursuit_clamps_both_directions_and_refreshes_a_zero_step() {
        assert_eq!(approach(0., 1., 10.).unwrap().current, 1.);
        assert_eq!(approach(3., 1., 10.).unwrap().current, 1.);
        assert_eq!(
            approach(3., 1., 0.).unwrap(),
            View {
                current: 3.,
                refreshed: true
            }
        );
        assert!(approach(0., 1., -1.).is_err());
    }
    #[test]
    fn equal_sizes_bypass_time_and_keep_signed_zero() {
        let result = approach(-0., 0., f64::NAN).unwrap();
        assert!(!result.refreshed);
        assert_eq!(result.current.to_bits(), (-0f32).to_bits());
        assert!(approach(f32::NAN, 0., 0.).is_err());
    }
}
