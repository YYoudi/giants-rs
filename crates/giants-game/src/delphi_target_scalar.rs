//! Acquired Delphi production of the scalar consumed by target selection.
//! Its broader rendering/gameplay meaning and other actors' writers remain
//! unassigned. This does not bind the primary phase or publish a live header.
#[derive(Clone, Copy, Debug)]
pub struct Link {
    pub class: i32,
    pub context: u32,
}
#[derive(Clone, Copy, Debug)]
pub struct CurrentPlayer {
    pub context: u32,
    pub position: [f32; 3],
}
#[derive(Clone, Copy, Debug)]
pub struct Input {
    /// Actual shared mode word; no network/camera semantic is inferred here.
    pub shared_mode: i32,
    pub owner_context: u32,
    pub owner_position: [f32; 3],
    /// Actual caller relationship, not an arbitrary held or parent actor.
    pub linked: Option<Link>,
    pub current_player: Option<CurrentPlayer>,
}
#[derive(Clone, Copy, Debug)]
pub struct Parameters {
    pub near: f32,
    pub far: f32,
}
impl Parameters {
    pub fn from_globals(g: &giants_formats::global_data::GlobalData) -> Result<Self, String> {
        Ok(Self {
            near: g.scalar(0x1210).map_err(|e| e.to_string())?,
            far: g.scalar(0x1214).map_err(|e| e.to_string())?,
        })
    }
}
pub fn produce(input: Input, parameters: Parameters) -> Result<f32, String> {
    if input.shared_mode != 4
        || input
            .linked
            .is_none_or(|link| link.class != 0x4a5 || link.context != input.owner_context)
    {
        return Ok(0.);
    }
    let current = input
        .current_player
        .ok_or("Delphi target scalar current player absent")?;
    if current.context == input.owner_context {
        return Ok(0.);
    }
    if !input
        .owner_position
        .into_iter()
        .chain(current.position)
        .chain([parameters.near, parameters.far])
        .all(f32::is_finite)
        || parameters.near == parameters.far
    {
        return Err("Delphi target scalar outside acquired finite nonsingular profile".into());
    }
    let delta: [f64; 3] = std::array::from_fn(|i| {
        f64::from(input.owner_position[i]) - f64::from(current.position[i])
    });
    // Unlike target movement/readiness, this helper does not store squared
    // distance or the square root as f32 before using the result.
    let distance = ((delta[0] * delta[0] + delta[1] * delta[1]) + delta[2] * delta[2]).sqrt();
    let fraction = (distance - f64::from(parameters.near))
        / (f64::from(parameters.far) - f64::from(parameters.near));
    if !fraction.is_finite() {
        return Err("Delphi target scalar fraction nonfinite".into());
    }
    let mut stored = fraction as f32;
    if fraction > 1. {
        stored = 1.;
    }
    // The lower comparison reloads the stored scalar; retain a stored -0.
    if stored < 0. {
        stored = 0.;
    }
    Ok(stored)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn input(position: [f32; 3]) -> Input {
        Input {
            shared_mode: 4,
            owner_context: 1,
            owner_position: position,
            linked: Some(Link {
                class: 0x4a5,
                context: 1,
            }),
            current_player: Some(CurrentPlayer {
                context: 0,
                position: [0.; 3],
            }),
        }
    }
    #[test]
    fn distance_stays_wide_and_stored_negative_zero_survives() {
        let value = produce(
            input([1., 2f32.powi(-13), 0.]),
            Parameters { near: 1., far: 2. },
        )
        .unwrap();
        assert!(value > 0.);
        assert_eq!(
            produce(input([0.; 3]), Parameters { near: 0., far: -1. })
                .unwrap()
                .to_bits(),
            (-0f32).to_bits()
        );
        assert_eq!(
            produce(input([3., 0., 0.]), Parameters { near: 1., far: 2. }).unwrap(),
            1.
        );
    }
    #[test]
    fn inactive_route_does_not_demand_unused_current_player_or_parameters() {
        let mut i = input([f32::NAN; 3]);
        i.current_player = None;
        assert!(produce(i, Parameters { near: 1., far: 2. }).is_err());
        i.linked = None;
        assert_eq!(
            produce(
                i,
                Parameters {
                    near: f32::NAN,
                    far: f32::NAN
                }
            )
            .unwrap(),
            0.
        );
    }
}
