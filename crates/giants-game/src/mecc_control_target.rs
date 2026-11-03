//! Direct object-basis branch of signed byte controls to target velocity.
//! Includes the contact-normal quaternion branch and supplied eligibility gates.
use crate::arithmetic::Precision;

#[derive(Clone, Copy, Debug)]
pub struct Input {
    pub matrix: [f32; 16],
    pub scale: f32,
    pub controls: [i8; 2],
    pub speeds: [f32; 2],
    pub normal: [f32; 3],
    pub physics_flags: u32,
    /// The caller supplies the native object eligibility field; not inferred here.
    pub eligible: bool,
    pub override_contact: bool,
}

pub fn apply(i: Input, precision: Precision) -> Result<Option<[f32; 3]>, &'static str> {
    if !i.eligible && i.physics_flags & 0x68 == 0 && !i.override_contact {
        return Ok(None);
    }
    let matrix = if i.physics_flags & 0x28 != 0 && !i.override_contact {
        ground_basis(i.matrix, i.normal, precision)?
    } else {
        i.matrix
    };
    direct(matrix, i.scale, i.controls, i.speeds, precision).map(Some)
}

/// Rotate world up onto the supplied normal, then premultiply an affine basis.
/// This is the scalar dispatch profile, with a non-identity object basis.
pub fn ground_basis(
    matrix: [f32; 16],
    normal: [f32; 3],
    precision: Precision,
) -> Result<[f32; 16], &'static str> {
    if !matrix.into_iter().chain(normal).all(f32::is_finite) || normal[2] <= -1. {
        return Err("ground basis outside finite non-antipodal profile");
    }
    let r = |v| precision.round(v);
    let s = |v| f64::from(v as f32);
    let n = normal.map(f64::from);
    let up = [0., 0., 1.];
    let cross = [(1, 2), (2, 0), (0, 1)].map(|(a, b)| s(r(r(n[b] * up[a]) - r(n[a] * up[b]))));
    let dot = r(r(r(up[2] * n[2]) + r(up[1] * n[1])) + r(up[0] * n[0]));
    let w = r(s(r(r(dot + 1.) * 0.5)).sqrt());
    let inv = r(1. / r(w + w));
    let [x, y, z] = cross.map(|v| s(r(v * inv)));
    let w = s(w);
    let xx2 = r(x + x);
    let yy2 = s(r(y + y));
    let zz2 = s(r(z + z));
    let xw = s(r(xx2 * w));
    let yw = s(r(yy2 * w));
    let zw = s(r(zz2 * w));
    let xx = s(r(xx2 * x));
    let xy = s(r(yy2 * x));
    let xz = s(r(zz2 * x));
    let yy = s(r(yy2 * y));
    let yz = r(zz2 * y);
    let zz = r(zz2 * z);
    let rotation = [
        r(1. - r(yy + zz)),
        r(xy - zw),
        r(xz + yw),
        r(xy + zw),
        r(1. - r(zz + xx)),
        r(yz - xw),
        r(xz - yw),
        r(yz + xw),
        r(1. - r(yy + xx)),
    ]
    .map(s);
    let mut out = matrix;
    for row in 0..3 {
        for col in 0..3 {
            out[row * 4 + col] = r(r(r(f64::from(matrix[col]) * rotation[row * 3])
                + r(f64::from(matrix[8 + col]) * rotation[row * 3 + 2]))
                + r(f64::from(matrix[4 + col]) * rotation[row * 3 + 1]))
                as f32;
        }
    }
    // Translation does not contribute to movement target construction.
    if !out.into_iter().all(f32::is_finite) {
        return Err("ground basis overflow");
    }
    Ok(out)
}
pub fn direct(
    matrix: [f32; 16],
    scale: f32,
    controls: [i8; 2],
    speeds: [f32; 2],
    precision: Precision,
) -> Result<[f32; 3], &'static str> {
    if !matrix
        .into_iter()
        .chain([scale])
        .chain(speeds)
        .all(f32::is_finite)
        || scale <= 0.
    {
        return Err("control target outside finite positive-scale profile");
    }
    let r = |v| precision.round(v);
    let inv = r(1. / f64::from(scale));
    let amount: [f64; 2] = std::array::from_fn(|i| {
        r(r(r(f64::from(controls[i]) * 0.03125) * inv) * f64::from(speeds[i]))
    });
    Ok(std::array::from_fn(|row| {
        let forward = -r(f64::from(matrix[row * 4 + 1]) * amount[0]);
        let forward = if row < 2 {
            f64::from(forward as f32)
        } else {
            forward
        };
        r(r(f64::from(matrix[row * 4]) * f64::from(amount[1] as f32)) + forward) as f32
    }))
}
