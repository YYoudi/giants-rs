//! Affine blend consumed by D's multi-frame hierarchy evaluator. This uses
//! shortest-arc quaternion interpolation; the older track interpolation entry
//! points keep their independent arithmetic and sign conventions.
use crate::animation::{Pose, simd_trig};

pub fn affine(a: &Pose, b: &Pose, t: f32) -> Result<Pose, &'static str> {
    if !a
        .position
        .iter()
        .chain(&a.rotation)
        .chain(&a.scale)
        .chain(&b.position)
        .chain(&b.rotation)
        .chain(&b.scale)
        .chain([&t])
        .all(|v| v.is_finite())
        || !(0. ..=1.).contains(&t)
        || (a.flags ^ b.flags) & 8 != 0
    {
        return Err("affine blend outside finite interval/mirror profile");
    }
    let x = a.rotation.map(f64::from);
    let y = b.rotation.map(f64::from);
    let wide = ((x[3] * y[3] + x[2] * y[2]) + x[1] * y[1]) + x[0] * y[0];
    let negative = wide < 0.;
    let dot = f64::from(wide as f32).abs();
    let (wa, wb) = if 1. - dot > f64::from(1e-6_f32) {
        let angle = dot.acos() as f32;
        let inverse = (1. / f64::from(simd_trig(angle, false))) as f32;
        let first = (f64::from(simd_trig(
            ((1. - f64::from(t)) * f64::from(angle)) as f32,
            false,
        )) * f64::from(inverse)) as f32;
        let second = f64::from(simd_trig((f64::from(t) * f64::from(angle)) as f32, false))
            * f64::from(inverse);
        (f64::from(first), second)
    } else {
        (f64::from((1. - f64::from(t)) as f32), f64::from(t))
    };
    let rotation = std::array::from_fn(|i| {
        if negative {
            (wb * y[i] - wa * x[i]) as f32
        } else {
            (wa * x[i] + wb * y[i]) as f32
        }
    });
    let mix = |x: [f32; 3], y: [f32; 3]| {
        std::array::from_fn(|i| {
            ((1. - f64::from(t)) * f64::from(x[i]) + f64::from(t) * f64::from(y[i])) as f32
        })
    };
    Ok(Pose {
        position: mix(a.position, b.position),
        rotation,
        scale: mix(a.scale, b.scale),
        flags: a.flags | b.flags,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> Pose {
        Pose {
            position: [0.; 3],
            rotation: [0., 0., 0., 1.],
            scale: [1.; 3],
            flags: 0,
        }
    }

    #[test]
    fn antipodal_rotation_uses_native_negative_sign_without_collapsing() {
        let a = identity();
        let mut b = identity();
        b.rotation[3] = -1.;
        b.position = [10., 20., -10.];
        let result = affine(&a, &b, 0.5).unwrap();
        assert_eq!(result.rotation, [0., 0., 0., -1.]);
        assert_eq!(result.position, [5., 10., -5.]);
        assert_eq!(result.scale, [1.; 3]);
    }

    #[test]
    fn compatible_mirror_flags_survive_endpoints() {
        let mut a = identity();
        a.flags = 8 | 1;
        let mut b = identity();
        b.flags = 8 | 2;
        b.position = [3., 4., 5.];
        for (t, expected) in [(0., a.position), (1., b.position)] {
            let result = affine(&a, &b, t).unwrap();
            assert_eq!(result.position, expected);
            assert_eq!(result.flags, 11);
        }
        b.flags &= !8;
        assert!(affine(&a, &b, 0.5).is_err());
    }

    #[test]
    fn invalid_weight_or_pose_is_refused() {
        let a = identity();
        for t in [f32::NAN, f32::INFINITY, -0.01, 1.01] {
            assert!(affine(&a, &a, t).is_err());
        }
        let mut b = identity();
        b.scale[1] = f32::NAN;
        assert!(affine(&a, &b, 0.5).is_err());
    }
}
