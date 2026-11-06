//! Authored reference camera arithmetic, with explicit current world/local poses.
//! Resource selection, ANM epochs and camera publication are separate producers.
use crate::{animation, procedural_aim::Matrix};

#[derive(Clone, Copy, Debug)]
pub struct Output {
    pub view: Matrix,
    /// A non-camera reference does not supply a lens value.
    pub vertical_fov: Option<f32>,
}

/// Compose the acquired finite affine reference profile at Double53 precision.
/// `horizontal_fov` is the authored camera angle in radians, when present.
pub fn compose(
    world: Matrix,
    reference: Matrix,
    horizontal_fov: Option<f32>,
    aspect: f32,
) -> Result<Output, &'static str> {
    if !world
        .values
        .iter()
        .chain(&reference.values)
        .all(|v| v.is_finite())
        || world.values[12..] != [0., 0., 0., 1.]
        || reference.values[12..] != [0., 0., 0., 1.]
    {
        return Err("authored camera requires finite affine input poses");
    }
    let placed = world.mul(reference)?;
    if !placed.values.iter().all(|v| v.is_finite()) {
        return Err("authored camera placement overflow");
    }
    let original_forward = [placed.values[2], placed.values[6], placed.values[10]];
    let forward =
        animation::normalize(original_forward).map_err(|_| "degenerate authored camera forward")?;
    let right = animation::normalize(animation::cross(
        [placed.values[1], placed.values[5], placed.values[9]],
        // Cross against the original direction, before its normalization.
        // Scaling the first operand away earlier changes float-store rounding.
        original_forward,
    ))
    .map_err(|_| "degenerate authored camera right")?;
    let up = animation::cross(forward, right);
    let mut basis = Matrix::identity();
    basis.flags = 0x11;
    for row in 0..3 {
        basis.values[row * 4..row * 4 + 4].copy_from_slice(&[
            right[row],
            up[row],
            forward[row],
            placed.values[row * 4 + 3],
        ]);
    }
    let inverse = basis.inverse()?;
    if !inverse.values.iter().all(|v| v.is_finite()) {
        return Err("authored camera inverse overflow");
    }
    let view = Matrix {
        flags: (inverse.flags >> 1 & 0x10)
            | ((inverse.flags & 0x10) << 1)
            | (inverse.flags & !0x30),
        values: std::array::from_fn(|i| inverse.values[(i % 4) * 4 + i / 4]),
    };
    let vertical_fov = horizontal_fov
        .map(|angle| {
            if !angle.is_finite()
                || angle <= 0.
                || f64::from(angle) >= std::f64::consts::PI
                || !aspect.is_finite()
                || aspect <= 0.
            {
                return Err("authored camera lens outside acquired finite profile");
            }
            // The source stores tan to float before division by its supplied
            // aspect scalar, then uses a promoted float degrees conversion.
            let tangent = (f64::from(angle) * 0.5).tan() as f32;
            Ok(
                ((f64::from(tangent) / f64::from(aspect)).atan() * f64::from(114.591_56_f32))
                    as f32,
            )
        })
        .transpose()?;
    Ok(Output { view, vertical_fov })
}
