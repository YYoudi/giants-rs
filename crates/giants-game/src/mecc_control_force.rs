//! Velocity-target force production; gravity and terrain projection are separate.
//! Exact native control covers Double53. Single24 power-helper differences are
//! retained in the diagnostic evidence; Single24 is not certified here.
use crate::arithmetic::Precision;

#[derive(Clone, Copy, Debug)]
pub struct Input {
    pub desired_velocity: [f32; 3],
    pub velocity: [f32; 3],
    pub force: [f32; 3],
    pub gain: f32,
    pub slope_power: f32,
    pub mass: f32,
    pub force_time_limit: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct Output {
    pub force: [f32; 3],
    pub force_time_limit: f32,
}
/// Add the controller contribution to existing forces. Only positive vertical
/// velocity correction receives slope attenuation; falling correction does not.
pub fn apply(input: Input, precision: Precision) -> Result<Output, &'static str> {
    let i = input;
    if !i
        .desired_velocity
        .iter()
        .chain(&i.velocity)
        .chain(&i.force)
        .chain([&i.gain, &i.slope_power, &i.mass, &i.force_time_limit])
        .all(|v| v.is_finite())
        || i.gain < 0.
        || i.slope_power < 0.
        || i.mass <= 0.
        || i.force_time_limit < 0.
    {
        return Err("control force outside finite nonnegative profile");
    }
    let r = |v| precision.round(v);
    let [x, y, z] = i.desired_velocity.map(f64::from);
    let cap = r(r(r(r(r(x * x) + r(y * y)) + r(z * z)).sqrt()) * 0.75) as f32;
    let delta: [f32; 3] = std::array::from_fn(|a| {
        r(f64::from(i.desired_velocity[a]) - f64::from(i.velocity[a])) as f32
    });
    let [x, y, z] = delta.map(f64::from);
    let length = r(r(r(r(z * z) + r(y * y)) + r(x * x)).sqrt());
    let gain = if cap != 0. && f64::from(cap) < length {
        r(r(f64::from(cap) / length) * f64::from(i.gain)) as f32
    } else {
        i.gain
    };
    let mut out = Output {
        force: i.force,
        force_time_limit: i.force_time_limit,
    };
    if gain != 0. {
        let slope = if length > f64::from(0.0001f32) {
            r(z / length).max(0.)
        } else {
            0.
        };
        let base = r(r(1. - r(slope * slope)).sqrt()) as f32;
        let attenuation = r(f64::from(base).powf(f64::from(i.slope_power))).abs();
        let g = f64::from(gain);
        out.force[0] = r(r(g * x) + f64::from(i.force[0])) as f32;
        out.force[1] = r(r(g * y) + f64::from(i.force[1])) as f32;
        out.force[2] = r(r(r(attenuation * g) * z) + f64::from(i.force[2])) as f32;
        let limit = r(f64::from(i.mass) / g);
        if i.force_time_limit == 0. || limit < f64::from(i.force_time_limit) {
            out.force_time_limit = limit as f32;
        }
    }
    if !out
        .force
        .iter()
        .chain([&out.force_time_limit])
        .all(|v| v.is_finite())
    {
        return Err("control force outside finite output profile");
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn braking_does_not_attenuate_downward_correction() {
        let input = Input {
            desired_velocity: [0.; 3],
            velocity: [0., 0., 2.],
            force: [0.; 3],
            gain: 4.,
            slope_power: 400.,
            mass: 20.,
            force_time_limit: 0.1,
        };
        let out = apply(input, Precision::Double53).unwrap();
        assert_eq!(out.force, [0., 0., -8.]);
        assert_eq!(out.force_time_limit, 0.1);
        assert_eq!(
            apply(
                Input {
                    velocity: [0., 0., -2.],
                    ..input
                },
                Precision::Double53
            )
            .unwrap()
            .force,
            [0.; 3]
        );
    }
}
