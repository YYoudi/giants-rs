//! State78 matrix-to-angle extraction and angular stepping, kept distinct from
//! the older render-cache Euler convention. Inputs and callback time are explicit.
use crate::procedural_aim::Matrix;
fn angle(y: f32, x: f32) -> f32 {
    let mut value = if x == 0. && y == 0. {
        0.
    } else {
        f64::from(y).atan2(f64::from(x)) * f64::from(57.295776f32)
    };
    if value < 0. {
        value += 360.;
    }
    if value >= f64::from(359.99f32) {
        value = 0.;
    }
    value as f32
}
pub fn from_matrix(matrix: Matrix) -> Result<[f32; 3], &'static str> {
    if matrix.flags & 0x20 != 0 || !matrix.values.iter().all(|v| v.is_finite()) {
        return Err("effect orientation requires finite affine input");
    }
    let m = matrix.values;
    let squared = (f64::from(m[0]) * f64::from(m[0]) + f64::from(m[1]) * f64::from(m[1])) as f32;
    let wide = f64::from(squared).sqrt();
    if !wide.is_finite() {
        return Err("effect orientation length overflow");
    }
    let length = wide as f32;

    let (yaw, pitch, roll) = if wide > f64::from(0.00001f32) {
        (angle(m[4], m[0]), angle(m[9], m[10]), angle(m[8], length))
    } else {
        (0., angle(-m[6], m[5]), angle(m[2], length))
    };
    Ok([yaw, pitch, roll])
}
pub fn advance(angles: [f32; 3], rates: [f32; 3], dt: f32) -> Result<[f32; 3], &'static str> {
    if !angles
        .into_iter()
        .chain(rates)
        .chain([dt])
        .all(f32::is_finite)
        || dt < 0.
    {
        return Err("invalid angular callback input");
    }
    let result = std::array::from_fn(|i| {
        (f64::from(angles[i]) + f64::from(dt) * f64::from(rates[i])) as f32
    });
    if !result.into_iter().all(f32::is_finite) {
        return Err("angular callback overflow");
    }
    Ok(result)
}
pub fn matrix(position: [f32; 3], angles: [f32; 3], scale: f32) -> Result<Matrix, &'static str> {
    if scale <= 0. {
        return Err("effect orientation requires positive scale");
    }
    if !position
        .into_iter()
        .chain(angles)
        .chain([scale])
        .all(f32::is_finite)
        || angles.into_iter().any(|v| v.abs() > 1000000.)
    {
        return Err("unsupported effect Euler input");
    }
    let trig = |degrees: f32, cosine| {
        let radians = (f64::from(degrees) * f64::from(0.017453292f32)) as f32;
        f64::from(crate::animation::simd_trig(radians, cosine))
    };
    let (sy, cy) = (trig(angles[0], false), trig(angles[0], true));
    let (sp, cp) = (trig(angles[1], false), trig(angles[1], true));
    let (sr, cr) = (trig(angles[2], false), trig(angles[2], true));
    let s = f64::from(scale);
    let values = [
        (cr * cy * s) as f32,
        ((-((sr * cy) * sp) - sy * cp) * s) as f32,
        ((sy * sp - (sr * cy) * cp) * s) as f32,
        position[0],
        (sy * cr * s) as f32,
        ((cy * cp - (sy * sr) * sp) * s) as f32,
        ((-(sy * sr) * cp - cy * sp) * s) as f32,
        position[1],
        (sr * s) as f32,
        (cr * sp * s) as f32,
        (cp * cr * s) as f32,
        position[2],
        0.,
        0.,
        0.,
        1.,
    ];
    if !values.into_iter().all(f32::is_finite) {
        return Err("effect Euler overflow");
    }
    Ok(Matrix { flags: 19, values })
}
