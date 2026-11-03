//! Terrain texture transforms from an explicitly supplied affine camera view.
//! The native rigid-camera inverse transposes the basis, preserving its drift.

pub fn texture_from_eye(
    view: [f32; 16],
    class: u8,
    reciprocal: f32,
) -> Result<[f32; 16], &'static str> {
    if class > 8
        || !view.iter().all(|v| v.is_finite())
        || !reciprocal.is_finite()
        || view[3] != 0.
        || view[7] != 0.
        || view[11] != 0.
        || view[15] != 1.
    {
        return Err("terrain UV requires a finite affine view and class 0..8");
    }
    let mut inverse = [0f32; 16];
    inverse[15] = 1.;
    for row in 0..3 {
        for column in 0..3 {
            inverse[row * 4 + column] = view[column * 4 + row];
        }
    }
    for column in 0..3 {
        inverse[12 + column] = -((f64::from(inverse[8 + column]) * f64::from(view[14])
            + f64::from(inverse[column]) * f64::from(view[12]))
            + f64::from(inverse[4 + column]) * f64::from(view[13]))
            as f32;
    }
    let (u_axis, v_axis, u_scale) = match class {
        0 => (0, 1, reciprocal),
        1 | 5 => (0, 2, reciprocal),
        2 | 6 => (0, 2, -reciprocal),
        3 | 7 => (1, 2, reciprocal),
        _ => (1, 2, -reciprocal),
    };
    let mut plane = [0f32; 16];
    plane[u_axis * 4] = u_scale;
    plane[v_axis * 4 + 1] = reciprocal;
    plane[15] = 1.;
    let result = std::array::from_fn(|i| {
        let row = i / 4;
        let column = i % 4;
        if column == 3 {
            return if row == 3 { 1. } else { 0. };
        }
        let product = |k| f64::from(inverse[row * 4 + k]) * f64::from(plane[k * 4 + column]);
        if row == 3 {
            (((product(1) + product(0)) + product(2)) + 0.) as f32
        } else {
            ((product(0) + product(2)) + product(1)) as f32
        }
    });
    if result.iter().any(|v| !v.is_finite()) {
        return Err("terrain UV matrix overflow");
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn view() -> [f32; 16] {
        std::array::from_fn(|i| if i % 5 == 0 { 1. } else { 0. })
    }
    #[test]
    fn reject_projective_nonfinite_and_out_of_range_inputs() {
        let mut matrix = view();
        assert!(texture_from_eye(matrix, 9, 1.).is_err());
        assert!(texture_from_eye(matrix, 0, f32::INFINITY).is_err());
        matrix[3] = 0.25;
        assert!(texture_from_eye(matrix, 0, 1.).is_err());
        matrix = view();
        matrix[8] = f32::NAN;
        assert!(texture_from_eye(matrix, 0, 1.).is_err());
        matrix = view();
        matrix[0] = f32::MAX;
        assert!(texture_from_eye(matrix, 0, f32::MAX).is_err());
    }
}
