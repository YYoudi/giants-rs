//! Placement and alignment from an explicit ground-query result and initial matrix.
//! This does not perform the query, execute setter services or schedule live FX.

#[derive(Clone, Copy, Debug)]
pub struct Placement {
    pub position: [f32; 3],
    pub alignment_weight: f32,
}

/// Original surface-helper matrix arithmetic with an explicit placement/query.
/// No world lookup, object setter or live callback is executed by this function.
pub fn alignment_matrix(
    initial: crate::procedural_aim::Matrix,
    normal: [f32; 3],
    placement: Placement,
) -> Result<crate::procedural_aim::Matrix, &'static str> {
    use crate::animation::{Pose, cross, normalize, pose_matrix, simd_trig};
    if initial.flags & !31 != 0
        || initial.values[12..] != [0., 0., 0., 1.]
        || initial
            .values
            .into_iter()
            .chain(normal)
            .chain(placement.position)
            .chain([placement.alignment_weight])
            .any(|v| !v.is_finite())
        || !(0. ..=1.).contains(&placement.alignment_weight)
    {
        return Err("surface alignment requires finite affine inputs and bounded weight");
    }
    let mut basis = initial;
    basis.flags &= !16;
    for i in [3, 7, 11] {
        basis.values[i] = 0.;
    }
    let axis = normalize([basis.values[2], basis.values[6], basis.values[10]])
        .map_err(|_| "surface alignment has degenerate model axis")?;
    let wide_dot = (f64::from(axis[2]) * f64::from(normal[2])
        + f64::from(axis[1]) * f64::from(normal[1]))
        + f64::from(axis[0]) * f64::from(normal[0]);
    let dot = wide_dot.min(1.) as f32;
    let sine_squared = (1. - f64::from(dot) * f64::from(dot)) as f32;
    if sine_squared < 0. {
        return Err("surface normal outside checked quaternion domain");
    }
    let sine = f64::from(sine_squared).sqrt() as f32;
    let rotation = if sine == 0. {
        [0., 0., 0., 1.]
    } else {
        let half = (f64::from(dot).acos() * f64::from(placement.alignment_weight) * 0.5) as f32;
        let ratio = f64::from(simd_trig(half, false)) / f64::from(sine);
        let xyz = cross(axis, normal).map(|v| (f64::from(v) * ratio) as f32);
        [xyz[0], xyz[1], xyz[2], simd_trig(half, true)]
    };
    let q = crate::procedural_aim::Matrix {
        flags: 1,
        values: pose_matrix(&Pose {
            position: [0.; 3],
            rotation,
            scale: [1.; 3],
            flags: 1,
        }),
    };
    let mut translation = crate::procedural_aim::Matrix::identity();
    translation.flags = initial.flags & 16;
    for i in [3, 7, 11] {
        translation.values[i] = initial.values[i];
    }
    let mut result = translation.mul(q.mul(basis)?)?;
    for (i, v) in [3, 7, 11].into_iter().zip(placement.position) {
        result.values[i] = v;
    }
    Ok(result)
}

pub fn placement(
    point: [f32; 3],
    plane: [f32; 4],
    height: f32,
    offset: f32,
) -> Result<Placement, &'static str> {
    if point
        .into_iter()
        .chain(plane)
        .chain([height, offset])
        .any(|v| !v.is_finite())
    {
        return Err("surface placement requires finite query inputs");
    }
    // The helper uses the queried height, before adding the sampled offset.
    // Replacing it with the old object's Z changes the alignment weight.
    let distance = ((f64::from(plane[2]) * f64::from(height)
        + f64::from(plane[1]) * f64::from(point[1]))
        + f64::from(plane[0]) * f64::from(point[0]))
        + f64::from(plane[3]);
    let position = [
        point[0],
        point[1],
        (f64::from(height) + f64::from(offset)) as f32,
    ];
    if !position[2].is_finite() {
        return Err("surface placement height overflow");
    }
    Ok(Placement {
        position,
        alignment_weight: (1. - distance).clamp(0., 1.) as f32,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn opposite_vectors_keep_the_native_identity_branch_and_bad_matrices_fail() {
        let initial = crate::procedural_aim::Matrix::identity();
        let placement = super::Placement {
            position: [2., 3., 4.],
            alignment_weight: 1.,
        };
        let result = super::alignment_matrix(initial, [0., 0., -1.], placement).unwrap();
        let mut expected = initial.values;
        for (i, v) in [3, 7, 11].into_iter().zip(placement.position) {
            expected[i] = v;
        }
        assert_eq!(result.values, expected);
        let mut invalid = initial;
        invalid.flags = 32;
        assert!(super::alignment_matrix(invalid, [0., 0., 1.], placement).is_err());
        invalid = initial;
        invalid.values[2] = 0.;
        invalid.values[6] = 0.;
        invalid.values[10] = 0.;
        assert!(super::alignment_matrix(invalid, [0., 0., 1.], placement).is_err());
    }
    #[test]
    fn placement_uses_query_height_before_offset_and_keeps_weight_clamps() {
        for old_z in [-500., 0., 1000.] {
            for offset in [-0.25, 0., 0.3] {
                let p = super::placement([2., 3., old_z], [0., 0., 1., -4.], 4., offset).unwrap();
                assert_eq!(p.position, [2., 3., 4. + offset]);
                assert_eq!(p.alignment_weight, 1.);
            }
        }
        assert_eq!(
            super::placement([0.; 3], [0., 0., 1., 0.], 0.5, 7.)
                .unwrap()
                .alignment_weight,
            0.5
        );
        assert_eq!(
            super::placement([0.; 3], [0., 0., 1., 0.], 2., 0.)
                .unwrap()
                .alignment_weight,
            0.
        );
        assert!(super::placement([f32::NAN, 0., 0.], [0.; 4], 0., 0.).is_err());
        assert!(super::placement([0.; 3], [0.; 4], f32::MAX, f32::MAX).is_err());
    }
}
