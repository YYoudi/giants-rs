//! Cached-matrix bounds, with a separate explicit camera-facing input path.
//! Clip matrices and depth limits come from the caller.
use crate::arithmetic::Precision;
#[derive(Clone, Copy, Debug)]
pub struct ResultFlags {
    pub flags: u32,
    pub outside_all: u8,
    pub outside_any: u8,
}
pub fn missing_spec(flags: u32) -> u32 {
    flags & !0x200080
}
#[derive(Clone, Copy, Debug)]
pub struct Input {
    pub bounds: [f32; 6],
    /// Row-major object matrix, matching the native cached transform.
    pub world: [f32; 16],
    /// Column-major source world-to-clip matrix; depth limits apply to W.
    pub clip: [f32; 16],
    pub near: f32,
    pub far: f32,
    pub flags: u32,
    pub personality: u32,
}
/// Evaluate the bounds prepass with explicit current camera and object pose.
/// This does not choose the draw-preparation matrix or mutate source placement.
pub fn evaluate_with_camera(
    mut input: Input,
    camera: crate::camera_facing::Input,
    precision: Precision,
) -> Result<ResultFlags, String> {
    input.world =
        crate::camera_facing::bounds_matrix(input.world, input.personality, camera, precision)?;
    input.personality &= !0x84;
    evaluate(input, precision)
}
pub fn evaluate(input: Input, precision: Precision) -> Result<ResultFlags, String> {
    let Input {
        bounds,
        world,
        clip,
        near,
        far,
        flags,
        personality,
    } = input;
    if personality & 0x84 != 0 {
        return Err("camera-facing culling matrix not implemented".into());
    }
    if !bounds
        .into_iter()
        .chain(world)
        .chain(clip)
        .chain([near, far])
        .all(f32::is_finite)
        || (0..3).any(|i| bounds[i] > bounds[i + 3])
        || near > far
    {
        return Err("invalid object culling inputs".into());
    }
    let r = |v: f64| precision.round(v);
    let mut all = 0x3f;
    let mut any = 0;
    for corner in 0..8 {
        let point: [f64; 3] = std::array::from_fn(|i| {
            f64::from(bounds[i + if corner & (1 << i) != 0 { 3 } else { 0 }])
        });
        let transformed: [f64; 3] = std::array::from_fn(|i| {
            let m: [f64; 4] = std::array::from_fn(|j| f64::from(world[i * 4 + j]));
            let stored = |v: f64| f64::from(v as f32);
            let [high_x, high_y, high_z] = [0, 1, 2].map(|axis| corner & (1 << axis) != 0);
            // Cached products and partial sums are float32 stores in the
            // original corner expansion, except these retained X/Y/Z terms.
            let x = r(point[0] * m[0]);
            let x = if i == 0 && high_x { x } else { stored(x) };
            let y = r(point[1] * m[1]);
            let y = if i == 0 && high_y { y } else { stored(y) };
            let z = r(point[2] * m[2]);
            let z = if i < 2 && !high_z { z } else { stored(z) };
            let zt = r(z + m[3]);
            let zt = if high_y && (!high_z || i > 0) {
                stored(zt)
            } else {
                zt
            };
            let yz = r(zt + y);
            let yz = if high_x && !(high_y && high_z && i < 2) {
                stored(yz)
            } else {
                yz
            };
            stored(r(yz + x))
        });
        // The original dot product adds X+Z, then Y. X stays wide for
        // side comparisons; Y is stored, and W is stored after its near test.
        let projected: [f64; 3] = [0, 1, 3].map(|i| {
            r(r(r(
                r(transformed[0] * f64::from(clip[i])) + r(transformed[2] * f64::from(clip[8 + i]))
            ) + r(transformed[1] * f64::from(clip[4 + i])))
                + f64::from(clip[12 + i]))
        });
        if !projected.iter().all(|v| v.is_finite()) {
            return Err("object culling transform overflow".into());
        }
        let [x, y, wide_w] = projected;
        let y = f64::from(y as f32);
        let w = f64::from(wide_w as f32);
        if !y.is_finite() || !w.is_finite() {
            return Err("object culling stored coordinate overflow".into());
        }
        let code = u8::from(wide_w < f64::from(near))
            | (u8::from(w > f64::from(far)) << 1)
            | (u8::from(x < -w) << 2)
            | (u8::from(x > w) << 3)
            | (u8::from(y < -w) << 4)
            | (u8::from(y > w) << 5);
        all &= code;
        any |= code;
    }
    if flags & 2 != 0 {
        all &= !2;
        any &= !2;
    }
    let flags = if all == 0 {
        flags & !0x80
    } else {
        flags | 0x80
    };
    let flags = if any == 0 {
        flags | 0x200000
    } else {
        flags & !0x200000
    };
    Ok(ResultFlags {
        flags,
        outside_all: all,
        outside_any: any,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_spec_and_camera_facing_modes_do_not_invent_a_matrix() {
        assert_eq!(missing_spec(u32::MAX), !0x200080);
        assert!(
            evaluate(
                Input {
                    bounds: [0.; 6],
                    world: [0.; 16],
                    clip: [0.; 16],
                    near: 1.,
                    far: 10.,
                    flags: 0,
                    personality: 4
                },
                Precision::Double53
            )
            .is_err()
        );
    }
}
