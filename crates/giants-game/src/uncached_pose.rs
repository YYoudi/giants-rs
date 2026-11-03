//! Euler pose consumed by the uncached collision/reference helper.
//! Object words C8/CC/D0 are kept distinct from reversed native call arguments.
use crate::arithmetic::Precision;

#[derive(Clone, Copy, Debug)]
pub struct Pose {
    pub position: [f32; 3],
    /// Native object words C8, CC, D0, without renaming their axes.
    pub angles: [f32; 3],
    pub scale: f32,
}

pub fn matrix(pose: Pose, precision: Precision) -> Result<[f32; 12], &'static str> {
    if !pose
        .position
        .into_iter()
        .chain(pose.angles)
        .chain([pose.scale])
        .all(f32::is_finite)
        || pose.angles.into_iter().any(|a| a.abs() > 1_000_000.)
        || pose.scale.abs() > 100_000.
    {
        return Err("uncached pose outside measured finite profile");
    }
    let r = |v| precision.round(v);
    let trig = |angle: f32| {
        let i = (r(r(f64::from(angle) * f64::from(45.511112_f32)) + 0.5) as i32 & 0x3fff) as usize;
        let sample = |j| f64::from((j as f64 * f64::from(0.0003834952_f32)).sin() as f32);
        (sample(i), sample(i + 4096))
    };
    let (sp, cp) = trig(-pose.angles[1]);
    let (sr, cr) = trig(pose.angles[2]);
    let (sy, cy) = trig(pose.angles[0]);
    let s = f64::from(pose.scale);
    let a = r(cy * sr);
    let b = r(sy * sr);
    Ok([
        r(r(cy * cr) * s) as f32,
        r(r(-r(sp * a) - r(sy * cp)) * s) as f32,
        r(r(r(sy * sp) - r(a * cp)) * s) as f32,
        pose.position[0],
        r(r(sy * cr) * s) as f32,
        r(r(r(cy * cp) - r(b * sp)) * s) as f32,
        r(r(-r(b * cp) - r(cy * sp)) * s) as f32,
        pose.position[1],
        r(sr * s) as f32,
        r(r(cr * sp) * s) as f32,
        r(r(cr * cp) * s) as f32,
        pose.position[2],
    ])
}

pub fn transform(
    matrix: [f32; 12],
    point: [f32; 3],
    precision: Precision,
) -> Result<[f32; 3], &'static str> {
    if !matrix.iter().chain(&point).all(|v| v.is_finite()) {
        return Err("non-finite uncached transform");
    }
    let r = |v| precision.round(v);
    let out = std::array::from_fn(|axis| {
        let m = &matrix[axis * 4..axis * 4 + 4];
        r(r(
            r(r(f64::from(m[2]) * f64::from(point[2])) + r(f64::from(m[1]) * f64::from(point[1])))
                + r(f64::from(m[0]) * f64::from(point[0])),
        ) + f64::from(m[3])) as f32
    });
    if out.iter().any(|v| !v.is_finite()) {
        return Err("uncached transform overflow");
    }
    Ok(out)
}
