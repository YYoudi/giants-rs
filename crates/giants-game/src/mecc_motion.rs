//! Controlled force-to-motion step. Collision and control-force production are
//! separate stages; caller supplies the independently resolved object timestep.
use crate::arithmetic::Precision;

/// D4a8bd0 position limiter, after integration and before placement. Velocity
/// remains untouched. The squared9000-unit bound and division/sqrt order are
/// independent of the force integrator's speed limit.
pub fn limit_position(position: [f32; 3], precision: Precision) -> Result<[f32; 3], &'static str> {
    if !position.into_iter().all(f32::is_finite) {
        return Err("nonfinite Mecc position limit input");
    }
    let r = |v| precision.round(v);
    let [x, y, z] = position.map(f64::from);
    let squared = r(r(r(x * x) + r(y * y)) + r(z * z));
    if squared <= 81_000_000. {
        return Ok(position);
    }
    let factor = r(r(81_000_000. / squared).sqrt());
    Ok(position.map(|v| r(f64::from(v) * factor) as f32))
}

#[derive(Clone, Copy, Debug)]
pub struct Motion {
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub previous_velocity: [f32; 3],
    pub force: [f32; 3],
    pub mass: f32,
    pub speed_limit: f32,
    pub force_time_limit: f32,
    pub stored_dt: f32,
}
/// Integrate supplied force, then clear it. A positive force-time limit bounds
/// acceleration only; position still advances over the full object timestep.
pub fn integrate(
    state: Motion,
    dt: f32,
    frozen: bool,
    preserve_frozen_velocity: bool,
    precision: Precision,
) -> Result<Motion, &'static str> {
    integrate_with_time(
        state,
        f64::from(dt),
        frozen,
        preserve_frozen_velocity,
        precision,
    )
}

/// Consume a resolved wide object timestep without an extra f32 narrowing.
pub fn integrate_with_time(
    mut state: Motion,
    dt: f64,
    frozen: bool,
    preserve_frozen_velocity: bool,
    precision: Precision,
) -> Result<Motion, &'static str> {
    if !state
        .position
        .iter()
        .chain(&state.velocity)
        .chain(&state.previous_velocity)
        .chain(&state.force)
        .chain([
            &state.mass,
            &state.speed_limit,
            &state.force_time_limit,
            &state.stored_dt,
        ])
        .all(|v| v.is_finite())
        || state.mass <= 0.
        || state.speed_limit < 0.
        || !dt.is_finite()
        || dt < 0.
        || dt > f64::from(f32::MAX)
    {
        return Err("motion input outside finite positive-mass profile");
    }
    if frozen {
        if !preserve_frozen_velocity {
            state.velocity = [0.; 3];
            state.previous_velocity = [0.; 3];
        }
        state.force = [0.; 3];
        return Ok(state);
    }
    let r = |v| precision.round(v);
    let force_dt = if state.force_time_limit > 0. && f64::from(state.force_time_limit) < dt {
        f64::from(state.force_time_limit)
    } else {
        dt
    };
    state.stored_dt = dt as f32;
    state.force_time_limit = 0.;
    state.previous_velocity = state.velocity;
    let inverse_mass = r(1. / f64::from(state.mass));
    let mut wide_z = 0.;
    for i in 0..3 {
        let wide = r(r(r(inverse_mass * f64::from(state.force[i])) * force_dt)
            + f64::from(state.velocity[i]));
        state.velocity[i] = wide as f32;
        if i == 2 {
            wide_z = wide;
        }
    }
    if state.speed_limit != 0. {
        let [x, y, _] = state.velocity.map(f64::from);
        let speed = r(r(r(r(wide_z * wide_z) + r(y * y)) + r(x * x)).sqrt()) as f32;
        if speed != 0. {
            let factor = if state.speed_limit < speed {
                r(f64::from(state.speed_limit) / f64::from(speed))
            } else {
                1.
            };
            for velocity in &mut state.velocity {
                *velocity = r(factor * f64::from(*velocity)) as f32;
            }
        }
    }
    for i in 0..3 {
        state.position[i] = r(r(r(r(
            f64::from(state.velocity[i]) + f64::from(state.previous_velocity[i])
        ) * dt)
            * 0.5)
            + f64::from(state.position[i])) as f32;
    }
    state.force = [0.; 3];
    if !state
        .position
        .iter()
        .chain(&state.velocity)
        .all(|v| v.is_finite())
    {
        return Err("motion arithmetic outside finite result profile");
    }
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn world_position_limit_keeps_boundary_and_direction() {
        assert_eq!(
            limit_position([9000., 0., 0.], Precision::Double53).unwrap(),
            [9000., 0., 0.]
        );
        assert_eq!(
            limit_position([-18000., 0., 0.], Precision::Double53).unwrap(),
            [-9000., 0., 0.]
        );
        assert!(limit_position([f32::NAN, 0., 0.], Precision::Double53).is_err());
    }
    #[test]
    fn force_limit_bounds_acceleration_without_shortening_motion_time() {
        let s = Motion {
            position: [0.; 3],
            velocity: [2., 0., 0.],
            previous_velocity: [9.; 3],
            force: [8., 0., 0.],
            mass: 2.,
            speed_limit: 0.,
            force_time_limit: 0.25,
            stored_dt: 0.,
        };
        let out = integrate(s, 1., false, false, Precision::Double53).unwrap();
        assert_eq!(out.velocity, [3., 0., 0.]);
        assert_eq!(out.position, [2.5, 0., 0.]);
        assert_eq!(out.force, [0.; 3]);
        assert_eq!(out.force_time_limit, 0.);
        assert_eq!(out.stored_dt, 1.);
    }
}
