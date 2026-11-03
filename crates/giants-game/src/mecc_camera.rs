//! Normal Mecc steady camera profile from the exact reference binary.
//! No chase smoothing, wall avoidance, shake, carried objects or vehicle override.
#[derive(Clone, Copy, Debug)]
pub struct Pose {
    pub eye: [f32; 3],
    pub forward: [f32; 3],
    pub up: [f32; 3],
    pub vertical_fov: f32,
}
pub fn pose(position: [f32; 3], yaw: f32, pitch: f32, first: bool) -> Result<Pose, &'static str> {
    if !position.into_iter().chain([yaw, pitch]).all(f32::is_finite)
        || yaw.abs() > 1e6
        || pitch.abs() > 1e6
    {
        return Err("camera angle/position outside finite profile");
    }
    let (sy, cy) = crate::object_transform::sincos(yaw);
    let (sp, cp) = crate::object_transform::sincos(pitch);
    let distance = if first { 2. } else { 16. };
    let height = if first { 6. } else { 8. };
    let forward = [(cp * sy) as f32, (-(cp * cy)) as f32, sp as f32];
    let up = [(-(sp * sy)) as f32, (sp * cy) as f32, cp as f32];
    let eye = [
        (f64::from(position[0]) - f64::from(forward[0]) * distance) as f32,
        (f64::from(position[1]) - f64::from(forward[1]) * distance) as f32,
        (f64::from(position[2]) - (sp * distance - height)) as f32,
    ];
    Ok(Pose {
        eye,
        forward,
        up,
        vertical_fov: 62.,
    })
}

/// Original projection setter's vertical-angle profile, near=1.
pub fn projection(aspect: f32, far: f32) -> Result<[f32; 16], &'static str> {
    if !aspect.is_finite() || aspect <= 0. || !far.is_finite() || far <= 1. {
        return Err("invalid normal-camera lens");
    }
    let angle = 62_f64 * f64::from(0.008726646_f32);
    let ratio = angle.cos() / f64::from(angle.sin() as f32);
    let mut m = [0.; 16];
    m[0] = (ratio / f64::from(aspect)) as f32;
    m[5] = ratio as f32;
    m[10] = (f64::from(far) / (f64::from(far) - 1.)) as f32;
    m[11] = 1.;
    m[14] = -m[10];
    Ok(m)
}
