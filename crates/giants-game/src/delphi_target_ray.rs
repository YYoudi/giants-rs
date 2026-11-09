//! Acquired held-origin ray. This does not choose a world hit, candidate or
//! movement readiness. The zeroed constructor reference buffer is an explicit
//! acquired input, not an evaluated pose or a manufactured identity.
use crate::procedural_aim::Matrix;

#[derive(Clone, Copy, Debug)]
pub struct Ray {
    pub origin: [f32; 3],
    pub direction: [f32; 3],
}
/// Compose the held cache with its first reference. Direction uses the negative
/// second column of the owner's affine cache, with the acquired wide norm order.
/// The threshold is immutable metadata supplied from the selected source profile.
pub fn from_poses(
    owner: Matrix,
    held: Matrix,
    reference: Matrix,
    threshold: f32,
) -> Result<Ray, String> {
    for (index, pose) in [owner, held, reference].into_iter().enumerate() {
        if pose.flags & 0x20 != 0
            || !pose.values.into_iter().all(f32::is_finite)
            || (pose.values[12..] != [0., 0., 0., 1.]
                && !(index == 2 && pose.flags == 0 && pose.values.map(f32::to_bits) == [0; 16]))
        {
            return Err("Delphi target ray requires finite acquired affine poses".into());
        }
    }
    if !threshold.is_finite() || threshold <= 0. {
        return Err("Delphi target ray requires a positive finite normalization threshold".into());
    }
    let placed = held.mul(reference)?;
    let origin = [placed.values[3], placed.values[7], placed.values[11]];
    if !origin.into_iter().all(f32::is_finite) {
        return Err("Delphi target ray origin outside finite profile".into());
    }
    let input = [-owner.values[1], -owner.values[5], -owner.values[9]];
    let [x, y, z] = input.map(f64::from);
    let squared = (x * x + z * z) + y * y;
    let direction = if squared < f64::from(threshold) {
        [0.; 3]
    } else {
        let inverse = 1. / squared.sqrt();
        input.map(|v| (inverse * f64::from(v)) as f32)
    };
    Ok(Ray { origin, direction })
}
