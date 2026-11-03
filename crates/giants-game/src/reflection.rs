//! Measured Story4 environment cube camera profile (six source axes).
use crate::animation::Camera;
use giants_formats::Error;

pub fn story4_camera(face: u8) -> Result<Camera, Error> {
    let (forward, up): ([f32; 3], [f32; 3]) = match face {
        0 => ([-1., 0., 0.], [0., 1., 0.]),
        1 => ([1., 0., 0.], [0., 1., 0.]),
        2 => ([0., 1., 0.], [0., 0., -1.]),
        3 => ([0., -1., 0.], [0., 0., 1.]),
        4 => ([0., 0., 1.], [0., 1., 0.]),
        5 => ([0., 0., -1.], [0., 1., 0.]),
        _ => return Err(Error("Reflection face outside 0..5")),
    };
    // Preserve the measured intermediate sign and operation order, including -0.
    let negative_right = [
        forward[2] * up[1] - forward[1] * up[2],
        forward[0] * up[2] - forward[2] * up[0],
        forward[1] * up[0] - forward[0] * up[1],
    ];
    let vertical = [
        forward[1] * negative_right[2] - forward[2] * negative_right[1],
        forward[2] * negative_right[0] - forward[0] * negative_right[2],
        forward[0] * negative_right[1] - forward[1] * negative_right[0],
    ];
    let origin = [0., 0., 800.];
    let mut view = [0.; 16];
    for row in 0..3 {
        view[row * 4] = -negative_right[row];
        view[row * 4 + 1] = vertical[row];
        view[row * 4 + 2] = forward[row];
    }
    let dot = |a: [f32; 3]| {
        ((f64::from(a[0]) * f64::from(origin[0]) + f64::from(a[1]) * f64::from(origin[1]))
            + f64::from(a[2]) * f64::from(origin[2])) as f32
    };
    view[12] = dot(negative_right);
    view[13] = -dot(vertical);
    view[14] = -dot(forward);
    view[15] = 1.;
    let far = (12500. - f64::from(dot(forward))) as f32;
    let projection = projection(far);
    Ok(Camera {
        view,
        projection,
        origin,
        viewport: [0, 0, 256, 256],
        lod_scale: (90. * f64::from(0.008726646f32)).sin() as f32,
    })
}

fn projection(far: f32) -> [f32; 16] {
    let angle = 90. * f64::from(0.008726646f32);
    let sine = angle.sin() as f32;
    let ratio = (angle.cos() / f64::from(sine)) as f32;
    let mut projection = [0.; 16];
    projection[0] = ratio;
    projection[5] = ratio;
    projection[10] = (f64::from(far) / (f64::from(far) - 1.)) as f32;
    projection[11] = 1.;
    projection[14] = -projection[10];
    projection
}

/// Initial Story4 benchmark cube: the menu has already consumed one producer call.
/// This profile is not a replacement for shared motion state in general gameplay.
pub fn story4_initial_water(speed: f32) -> Result<crate::water::Motion, &'static str> {
    let mut motion = crate::water::Motion::default();
    motion.advance(1. / 60., speed, 0.)?;
    Ok(motion)
}

/// The sky pass temporarily uses the measured 28,000-unit far clip.
pub fn story4_sky_projection() -> [f32; 16] {
    projection(28000.)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn face_basis_is_orthogonal_and_rejects_invalid_face() {
        for face in 0..6 {
            let c = story4_camera(face).unwrap();
            for a in 0..3 {
                for b in 0..3 {
                    let dot = (0..3)
                        .map(|r| c.view[r * 4 + a] * c.view[r * 4 + b])
                        .sum::<f32>();
                    assert_eq!(dot, if a == b { 1. } else { 0. });
                }
            }
            for col in 0..3 {
                let translation = (0..3)
                    .map(|r| c.origin[r] * c.view[r * 4 + col])
                    .sum::<f32>()
                    + c.view[12 + col];
                assert_eq!(translation, 0.);
            }
        }
        assert!(story4_camera(6).is_err());
        assert!(story4_camera(255).is_err());
    }
}
