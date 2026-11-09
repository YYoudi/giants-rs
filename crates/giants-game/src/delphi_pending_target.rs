//! Acquired pending-target search over the ordered common population. This uses
//! current bounds centres and the readiness policy, not the triangle-hit query.
//! Target/viewport/bounds producers, lifetime and primary admission are separate.
use crate::{delphi_target_readiness as readiness, volume_queries::Bounds};

#[derive(Clone, Copy, Debug)]
pub struct Parameters {
    pub near: f32,
    pub far: f32,
    pub width_squared: f32,
}
impl Parameters {
    pub fn from_globals(g: &giants_formats::global_data::GlobalData) -> Result<Self, String> {
        let width = g.scalar(0x908).map_err(|e| e.to_string())?;
        Ok(Self {
            near: g.scalar(0x900).map_err(|e| e.to_string())?,
            far: g.scalar(0x904).map_err(|e| e.to_string())?,
            width_squared: (f64::from(width) * f64::from(width)) as f32,
        })
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Input {
    pub owner_position: [f32; 3],
    pub owner_second_column: [f32; 3],
    pub viewport_origin: [f32; 3],
    /// Actual viewport basis. Do not substitute an independently normalized ray.
    pub viewport_forward: [f32; 3],
    pub parameters: Parameters,
    /// Immutable external source constants, not inferred actor metadata.
    pub half: f32,
    pub threshold: f32,
}
pub trait Services<K> {
    fn candidate(&mut self, key: K) -> Result<readiness::Candidate, String>;
    /// Current scalar tested after eligibility/personality. The Delphi writer
    /// is acquired in delphi_target_scalar; owned publication and other actors'
    /// writers remain separate. No implicit cold/default value is supplied.
    fn threshold_value(&mut self, key: K) -> Result<f32, String>;
    fn current_bounds(&mut self, key: K) -> Result<Bounds, String>;
}
/// Original ordered selection. Equal perpendicular distances keep the earlier
/// candidate; along-segment distance does not rank candidates. The near endpoint
/// is inclusive, the far endpoint and current perpendicular limit are strict.
pub fn query<K: Copy>(
    input: Input,
    order: &[K],
    services: &mut impl Services<K>,
) -> Result<Option<K>, String> {
    let p = input.parameters;
    if !input
        .owner_position
        .into_iter()
        .chain(input.owner_second_column)
        .chain(input.viewport_origin)
        .chain(input.viewport_forward)
        .chain([p.near, p.far, p.width_squared, input.half, input.threshold])
        .all(f32::is_finite)
    {
        return Err("Delphi pending-target search outside acquired finite profile".into());
    }
    let origin: [f32; 3] = std::array::from_fn(|i| {
        (f64::from(input.viewport_forward[i]) * f64::from(p.near)
            + f64::from(input.viewport_origin[i])) as f32
    });
    let span = (f64::from(p.far) - f64::from(p.near)) as f32;
    if !origin.into_iter().chain([span]).all(f32::is_finite) {
        return Err("Delphi pending-target interval overflow".into());
    }
    let mut limit = p.width_squared;
    let mut result = None;
    for &key in order {
        let candidate = services.candidate(key)?;
        let flags = candidate.flags;
        let personality = candidate.personality;
        if !readiness::eligible(candidate, input.owner_position, input.owner_second_column)?
            || flags & 0x4000c000 != 0
            || personality & 0x400000 == 0
        {
            continue;
        }
        let threshold = services.threshold_value(key)?;
        if !threshold.is_finite() {
            return Err("Delphi pending-target threshold outside acquired finite profile".into());
        }
        if threshold >= input.threshold {
            continue;
        }
        let bounds = services.current_bounds(key)?;
        if !bounds.min.into_iter().chain(bounds.max).all(f32::is_finite) {
            return Err("Delphi pending-target current bounds nonfinite".into());
        }
        let delta_wide: [f64; 3] = std::array::from_fn(|i| {
            ((f64::from(bounds.max[i]) + f64::from(bounds.min[i])) * f64::from(input.half))
                - f64::from(origin[i])
        });
        let delta = delta_wide.map(|v| v as f32);
        if !delta.into_iter().all(f32::is_finite) {
            return Err("Delphi pending-target centre overflow".into());
        }
        // Z remains wide for the first projection. X/Y and the subsequent
        // squared-distance computation use the original single-precision stores.
        let projection = (delta_wide[2] * f64::from(input.viewport_forward[2])
            + f64::from(delta[1]) * f64::from(input.viewport_forward[1]))
            + f64::from(delta[0]) * f64::from(input.viewport_forward[0]);
        let stored_projection = projection as f32;
        if !projection.is_finite() || !stored_projection.is_finite() {
            return Err("Delphi pending-target projection overflow".into());
        }
        if projection < 0. || stored_projection >= span {
            continue;
        }
        let [x, y, z] = delta.map(f64::from);
        let along = f64::from(stored_projection);
        let perpendicular = ((z * z + y * y) + x * x) - along * along;
        if !perpendicular.is_finite() {
            return Err("Delphi pending-target distance overflow".into());
        }
        if perpendicular < f64::from(limit) {
            limit = perpendicular as f32;
            if !limit.is_finite() {
                return Err("Delphi pending-target stored distance overflow".into());
            }
            result = Some(key);
        }
    }
    Ok(result)
}

/// Acquired signed guard used before the caller's target-range check. This does
/// not decide readiness or movement activation; the other gates remain separate.
pub fn mode_skips_range_check(mode: i32) -> bool {
    (0x31..=0x37).contains(&mode) || mode == 0x4c
}

#[cfg(test)]
mod tests {
    use super::*;
    struct World(Vec<Bounds>);
    impl Services<usize> for World {
        fn candidate(&mut self, _: usize) -> Result<readiness::Candidate, String> {
            Ok(readiness::Candidate {
                state: 23,
                class: 11,
                flags: 0,
                personality: 0x400000,
                limit: 100.,
                accumulated: 0.,
                exclusion: 0,
                shares_current_player_context: false,
                position: [0., -1., 0.],
            })
        }
        fn threshold_value(&mut self, _: usize) -> Result<f32, String> {
            Ok(0.)
        }
        fn current_bounds(&mut self, key: usize) -> Result<Bounds, String> {
            Ok(self.0[key])
        }
    }
    fn input() -> Input {
        Input {
            owner_position: [0.; 3],
            owner_second_column: [0., 1., 0.],
            viewport_origin: [0.; 3],
            viewport_forward: [1., 0., 0.],
            parameters: Parameters {
                near: 0.,
                far: 10.,
                width_squared: 1.,
            },
            half: 0.5,
            threshold: 1.,
        }
    }
    #[test]
    fn perpendicular_rank_keeps_first_tie_and_strict_far_and_width_limits() {
        let a = Bounds {
            min: [8., 0.5, 0.],
            max: [8., 0.5, 0.],
        };
        let b = Bounds {
            min: [5., 0.5, 0.],
            max: [5., 0.5, 0.],
        };
        let mut world = World(vec![a, b]);
        assert_eq!(query(input(), &[0, 1], &mut world).unwrap(), Some(0));
        assert_eq!(query(input(), &[1, 0], &mut world).unwrap(), Some(1));
        world.0[0] = Bounds {
            min: [10., 0., 0.],
            max: [10., 0., 0.],
        };
        assert_eq!(query(input(), &[0], &mut world).unwrap(), None);
        world.0[0] = Bounds {
            min: [5., 1., 0.],
            max: [5., 1., 0.],
        };
        assert_eq!(query(input(), &[0], &mut world).unwrap(), None);
        world.0[0] = Bounds {
            min: [0.; 3],
            max: [0.; 3],
        };
        assert_eq!(query(input(), &[0], &mut world).unwrap(), Some(0));
    }
    #[test]
    fn negative_wide_projection_is_not_replaced_by_stored_zero() {
        let mut i = input();
        i.viewport_forward = [0., 0., 1.];
        let mut world = World(vec![Bounds {
            min: [0., 0., -f32::from_bits(1)],
            max: [0.; 3],
        }]);
        assert_eq!(query(i, &[0], &mut world).unwrap(), None);
    }
}
