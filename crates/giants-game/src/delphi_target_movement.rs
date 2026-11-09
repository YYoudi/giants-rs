//! Acquired PC movement toward a linked actor or fallback position. Target
//! identity production, activation and world/physical scheduling stay separate.
#[derive(Clone, Copy, Debug)]
pub struct State {
    pub cached_target: [f32; 3],
    pub force: [f32; 3],
    pub velocity: [f32; 3],
}
#[derive(Clone, Copy, Debug)]
pub struct Input {
    pub position: [f32; 3],
    pub fallback: [f32; 3],
    /// Resolve an actual owned actor before supplying its current position.
    pub linked_position: Option<[f32; 3]>,
    pub gravity_mass: f32,
    pub mass: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct Constants {
    /// Immutable external source scalars, not a bundled metadata table.
    pub exponent: f64,
    pub acceleration_scale: f32,
    pub half: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct Parameters {
    pub fallback_rate: f32,
    pub linked_rate: f32,
    pub constants: Constants,
}
impl Parameters {
    pub fn from_globals(
        g: &giants_formats::global_data::GlobalData,
        constants: Constants,
    ) -> Result<Self, String> {
        Ok(Self {
            fallback_rate: g.scalar(0xfe0).map_err(|e| e.to_string())?,
            linked_rate: g.scalar(0x748).map_err(|e| e.to_string())?,
            constants,
        })
    }
}
/// Source-order prefix writes remain on refusal. The caller must quarantine
/// replay. Finite, positive-rate/mass, noncoincident inputs are acquired; error
/// behavior, coincident targets and the original FPU return ABI are not claimed.
pub fn advance(state: &mut State, i: Input, p: Parameters) -> Result<f32, String> {
    state.cached_target = i.linked_position.unwrap_or(i.fallback);
    state.force = [0.; 3];
    let rate = if i.linked_position.is_some() {
        p.linked_rate
    } else {
        p.fallback_rate
    };
    if !i.position.into_iter().all(f32::is_finite)
        || !state.cached_target.into_iter().all(f32::is_finite)
        || ![
            i.gravity_mass,
            i.mass,
            rate,
            p.constants.acceleration_scale,
            p.constants.half,
        ]
        .into_iter()
        .all(f32::is_finite)
        || i.mass <= 0.
        || rate <= 0.
        || !p.constants.exponent.is_finite()
    {
        return Err("Delphi target movement outside acquired finite profile".into());
    }
    let delta: [f64; 3] =
        std::array::from_fn(|at| f64::from(state.cached_target[at]) - f64::from(i.position[at]));
    let squared = ((delta[0] * delta[0] + delta[1] * delta[1]) + delta[2] * delta[2]) as f32;
    let duration = (f64::from(squared).powf(p.constants.exponent) * f64::from(rate)) as f32;
    if !squared.is_finite() || squared <= 0. || !duration.is_finite() || duration <= 0. {
        return Err("Delphi target movement duration outside acquired positive profile".into());
    }
    let acceleration = ((f64::from(i.gravity_mass) / f64::from(i.mass))
        * f64::from(p.constants.acceleration_scale)) as f32;
    let reciprocal = 1. / f64::from(duration);
    let stored_reciprocal = reciprocal as f32;
    let velocity = [
        (reciprocal * delta[0]) as f32,
        (f64::from(stored_reciprocal) * delta[1]) as f32,
        (f64::from(stored_reciprocal) * f64::from(delta[2] as f32)
            - (f64::from(duration) * f64::from(acceleration)) * f64::from(p.constants.half))
            as f32,
    ];
    if !velocity.into_iter().all(f32::is_finite) {
        return Err("Delphi target movement velocity outside acquired finite profile".into());
    }
    state.velocity = velocity;
    Ok(duration)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn refusal_retains_target_and_force_prefix_but_does_not_invent_velocity() {
        let mut state = State {
            cached_target: [0.; 3],
            force: [5.; 3],
            velocity: [7.; 3],
        };
        assert!(
            advance(
                &mut state,
                Input {
                    position: [1.; 3],
                    fallback: [2.; 3],
                    linked_position: Some([3.; 3]),
                    gravity_mass: 1.,
                    mass: 0.,
                },
                Parameters {
                    fallback_rate: 1.,
                    linked_rate: 2.,
                    constants: Constants {
                        exponent: 0.25,
                        acceleration_scale: -1.,
                        half: 0.5
                    }
                }
            )
            .is_err()
        );
        assert_eq!(state.cached_target, [3.; 3]);
        assert_eq!(state.force, [0.; 3]);
        assert_eq!(state.velocity, [7.; 3]);
    }
}
