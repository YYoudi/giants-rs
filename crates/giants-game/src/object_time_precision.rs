//! Object timestep selection from an ordered list of spatial time areas.
//! The caller owns area creation, expiry and native list ordering.
use crate::arithmetic::Precision;

#[derive(Clone, Copy, Debug)]
pub struct Actor {
    pub owner_id: u32,
    pub position: [f32; 2],
}

#[derive(Clone, Copy, Debug)]
pub struct Area {
    pub radius_squared: f32,
    pub multiplier: f32,
    pub owner_id: u32,
    pub center: [f32; 2],
}

/// The first containing area wins; areas belonging to the actor's owner are
/// skipped. An explicit actor takes precedence over the optional point.
/// Keep the wide return until the consumer's own stores, as in the native x87
/// return. Narrowing it to f32 before motion would change scaled timesteps.
pub fn seconds(
    frame_seconds: f32,
    actor: Option<Actor>,
    point: Option<[f32; 2]>,
    areas: &[Area],
    precision: Precision,
) -> Result<f64, &'static str> {
    if !frame_seconds.is_finite()
        || frame_seconds < 0.
        || actor
            .map(|a| a.position.into_iter().any(|v| !v.is_finite()))
            .unwrap_or(false)
        || point
            .map(|p| p.into_iter().any(|v| !v.is_finite()))
            .unwrap_or(false)
        || areas.iter().any(|a| {
            ![a.radius_squared, a.multiplier, a.center[0], a.center[1]]
                .into_iter()
                .all(f32::is_finite)
                || a.radius_squared < 0.
                || a.multiplier < 0.
        })
    {
        return Err("object time outside finite nonnegative profile");
    }
    let Some(position) = actor.map(|a| a.position).or(point) else {
        return Ok(f64::from(frame_seconds));
    };
    let r = |v| precision.round(v);
    for area in areas {
        if actor.is_some_and(|a| a.owner_id == area.owner_id) {
            continue;
        }
        let dx = r(f64::from(area.center[0]) - f64::from(position[0]));
        let dy = r(f64::from(area.center[1]) - f64::from(position[1]));
        let distance_squared = r(r(dx * dx) + r(dy * dy));
        if distance_squared <= f64::from(area.radius_squared) {
            return Ok(r(f64::from(frame_seconds) * f64::from(area.multiplier)));
        }
    }
    Ok(f64::from(frame_seconds))
}
