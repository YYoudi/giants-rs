//! Measured unbound Euler object transform; ANM-bound objects remain separate.
pub fn sine_table() -> Vec<f32> {
    (0..0x5000)
        .map(|i| (i as f64 * f64::from(0.0003834952f32)).sin() as f32)
        .collect()
}
pub(crate) fn sincos(degrees: f32) -> (f64, f64) {
    let i = ((f64::from(degrees) * f64::from(45.511112f32) + 0.5) as i32 & 0x3fff) as usize;
    let sample = |i: usize| f64::from((i as f64 * f64::from(0.0003834952f32)).sin() as f32);
    (sample(i), sample(i + 4096))
}
pub fn euler(position: [f32; 3], angles: [f32; 3], scale: f32) -> Result<[f32; 16], &'static str> {
    if !position
        .into_iter()
        .chain(angles)
        .chain([scale])
        .all(f32::is_finite)
        || angles.into_iter().any(|v| v.abs() > 1_000_000.)
        || scale.abs() > 100_000.
    {
        return Err("unsupported Euler object transform");
    }
    let (sp, cp) = sincos(-angles[1]);
    let (sr, cr) = sincos(angles[2]);
    let (sy, cy) = sincos(angles[0]);
    let s = f64::from(scale);
    Ok([
        (cy * cr * s) as f32,
        ((-(sp * cy * sr) - sy * cp) * s) as f32,
        ((sy * sp - cy * sr * cp) * s) as f32,
        position[0],
        (sy * cr * s) as f32,
        ((cy * cp - sy * sr * sp) * s) as f32,
        ((-(sy * sr * cp) - cy * sp) * s) as f32,
        position[1],
        (sr * s) as f32,
        (cr * sp * s) as f32,
        (cr * cp * s) as f32,
        position[2],
        0.,
        0.,
        0.,
        1.,
    ])
}
pub fn sun(angles: [f32; 3], scale: f32) -> Result<[f32; 3], &'static str> {
    let m = euler([0.; 3], angles, scale)?;
    Ok([m[1] * 12000., m[5] * 12000., m[9] * 12000.])
}
#[cfg(test)]
mod tests {
    #[test]
    fn unsupported_transform_inputs_are_rejected() {
        assert!(super::euler([0.; 3], [f32::NAN, 0., 0.], 1.).is_err());
        assert!(super::euler([f32::INFINITY, 0., 0.], [0.; 3], 1.).is_err());
        assert!(super::euler([0.; 3], [1e8, 0., 0.], 1.).is_err());
        assert!(super::euler([0.; 3], [0.; 3], f32::INFINITY).is_err());
    }
}
