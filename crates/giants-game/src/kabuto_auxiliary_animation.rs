//! Distance-driven selection for Kabuto's second auxiliary. Positions and list
//! cadence are supplied by preceding world phases; this does not move an actor.
use crate::animation_channels::{Blending, Channels, Request, select_primary};

pub enum Observers<'a> {
    Remote([f32; 3]),
    Local {
        first: &'a [[f32; 3]],
        second: &'a [[f32; 3]],
    },
}
pub struct Input<'a> {
    pub position: [f32; 3],
    pub radius: f32,
    pub observers: Observers<'a>,
}
pub struct Prepared {
    pub channels: Channels,
    pub normalized_clock: f32,
}

/// Preserve the absent-child bypass before reading other inputs. Finite poses
/// and a nonzero finite radius are the owned profile; invalid input is refused
/// before publication. This neither advances ANM nor evaluates references.
pub fn prepare(
    channels: Option<&Channels>,
    input: Input<'_>,
) -> Result<Option<Prepared>, &'static str> {
    let Some(channels) = channels else {
        return Ok(None);
    };
    if !input.position.into_iter().all(f32::is_finite)
        || !input.radius.is_finite()
        || input.radius == 0.
    {
        return Err("Kabuto auxiliary animation requires finite poses and nonzero radius");
    }
    let squared = |position: [f32; 3]| -> Result<f64, &'static str> {
        if !position.into_iter().all(f32::is_finite) {
            return Err("Kabuto auxiliary observer pose is nonfinite");
        }
        let d: [f64; 3] =
            std::array::from_fn(|i| f64::from(position[i]) - f64::from(input.position[i]));
        Ok((d[0] * d[0] + d[1] * d[1]) + d[2] * d[2])
    };
    let radius_squared = f64::from(input.radius) * f64::from(input.radius);
    let ratio = match input.observers {
        Observers::Remote(position) => squared(position)? / radius_squared,
        Observers::Local { first, second } => {
            let mut nearest = None::<f32>;
            for &position in first.iter().chain(second) {
                let distance = squared(position)?;
                // This store precedes the next candidate comparison. Remote
                // distance remains wide until the final clock store.
                if nearest.is_none_or(|old| f64::from(old) > distance) {
                    nearest = Some(distance as f32);
                }
            }
            nearest.map_or(1., |distance| f64::from(distance) / radius_squared)
        }
    };
    let normalized_clock = (1. - ratio.min(1.)) as f32;
    let mut next = *channels;
    next[0][1] = normalized_clock.to_bits();
    select_primary(
        &mut next,
        Request {
            slot: 1,
            flags: 0x84,
            speed_source: 0,
            reference_speed: 0.,
            blend_seconds: 0.,
        },
        Blending::Disabled,
    )?;
    Ok(Some(Prepared {
        channels: next,
        normalized_clock,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn absent_child_bypasses_invalid_inputs_and_present_child_refuses_them() {
        let invalid = || Input {
            position: [f32::NAN; 3],
            radius: 0.,
            observers: Observers::Remote([f32::NAN; 3]),
        };
        assert!(prepare(None, invalid()).unwrap().is_none());
        let channels = [[0; 9]; 4];
        assert!(prepare(Some(&channels), invalid()).is_err());
        assert!(
            prepare(
                Some(&channels),
                Input {
                    position: [0.; 3],
                    radius: 1.,
                    observers: Observers::Local {
                        first: &[[f32::NAN; 3]],
                        second: &[]
                    },
                }
            )
            .is_err()
        );
        assert_eq!(channels, [[0; 9]; 4]);
    }
    #[test]
    fn local_lists_select_nearest_while_remote_ignores_them() {
        let channels = [[0; 9]; 4];
        let run = |observers| {
            prepare(
                Some(&channels),
                Input {
                    position: [0.; 3],
                    radius: 4.,
                    observers,
                },
            )
            .unwrap()
            .unwrap()
        };
        let local = run(Observers::Local {
            first: &[[3., 0., 0.]],
            second: &[[2., 0., 0.]],
        });
        assert_eq!(local.normalized_clock, 0.75);
        assert_eq!(
            run(Observers::Remote([3., 0., 0.])).normalized_clock,
            0.4375
        );
        assert_eq!(
            run(Observers::Local {
                first: &[],
                second: &[]
            })
            .normalized_clock,
            0.
        );
        assert_eq!(local.channels[0][8], 0x184);
        assert_eq!(local.channels[0][0], 1);
        assert_eq!(local.channels[1][0], 254);
    }
}
