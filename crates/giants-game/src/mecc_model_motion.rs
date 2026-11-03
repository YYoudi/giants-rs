//! Actual displacement projected into the model basis after collision.
//! World velocity magnitude is distinct from displacement-derived model speeds.
use crate::arithmetic::Precision;

#[derive(Clone, Copy, Debug)]
pub struct Input {
    pub current: [f32; 3],
    pub previous: [f32; 3],
    pub velocity: [f32; 3],
    pub matrix: [f32; 16],
    pub scale: f32,
    pub seconds: f64,
    pub controls: [i8; 2],
    /// Directional minimum speeds, not maximum physical velocity limits.
    pub minimums: [f32; 2],
}

#[derive(Clone, Copy, Debug)]
pub struct Output {
    pub world_speed: f32,
    /// Vertical, forward, side, in native field order +108/+10c/+110.
    pub model: [f32; 3],
}

pub fn derive(i: Input, controlled: bool, precision: Precision) -> Result<Output, &'static str> {
    if !i
        .current
        .into_iter()
        .chain(i.previous)
        .chain(i.velocity)
        .chain(i.matrix)
        .chain([i.scale])
        .chain(i.minimums)
        .all(f32::is_finite)
        || !i.seconds.is_finite()
        || i.seconds < 0.
    {
        return Err("model motion outside finite nonnegative-time profile");
    }
    let r = |v| precision.round(v);
    let [x, y, z] = i.velocity.map(f64::from);
    let world_speed = r(r(r(r(x * x) + r(y * y)) + r(z * z)).sqrt()) as f32;
    let mut model = [0.; 3];
    if i.scale != 0. {
        let inv = r(1. / f64::from(i.scale));
        let side: [f32; 3] = std::array::from_fn(|n| r(inv * f64::from(i.matrix[n * 4])) as f32);
        let forward: [f32; 3] =
            std::array::from_fn(|n| -r(inv * f64::from(i.matrix[n * 4 + 1])) as f32);
        let up: [f32; 3] = std::array::from_fn(|n| r(inv * f64::from(i.matrix[n * 4 + 2])) as f32);
        let mut displacement = [0.; 3];
        if i.seconds != 0. {
            let inv_dt = r(1. / i.seconds);
            for (n, v) in displacement.iter_mut().enumerate() {
                *v = r(r(f64::from(i.current[n]) - f64::from(i.previous[n])) * inv_dt);
                if n < 2 {
                    *v = f64::from(*v as f32);
                }
            }
        }
        let dot =
            |basis: [f32; 3]| {
                r(r(r(f64::from(basis[2]) * displacement[2])
                    + r(displacement[1] * f64::from(basis[1])))
                    + r(displacement[0] * f64::from(basis[0]))) as f32
            };
        model = [dot(up), dot(forward), dot(side)];
        for n in 0..2 {
            let value = &mut model[n + 1];
            if i.controls[n] < 0 && *value > -i.minimums[n] {
                *value = -i.minimums[n];
            }
            if i.controls[n] > 0 && *value < i.minimums[n] {
                *value = i.minimums[n];
            }
        }
    }
    if controlled {
        for n in 0..2 {
            if i.controls[n] == 0 {
                model[n + 1] = 0.;
            }
        }
    }
    if !world_speed.is_finite() || !model.into_iter().all(f32::is_finite) {
        return Err("model motion arithmetic overflow");
    }
    Ok(Output { world_speed, model })
}
