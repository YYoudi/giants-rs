//! Camera-facing matrices. Bounds and draw preparation have different flag
//! priority and translation arithmetic; their entry points remain distinct.
use crate::arithmetic::Precision;

#[derive(Clone, Copy, Debug)]
pub struct Input {
    pub view: [f32; 16],
    pub position: [f32; 3],
    pub scale: f32,
    pub angle: f32,
}

/// Source row-major affine matrix for bounds. The cached ordinary matrix is
/// preserved when neither facing bit is set; unused camera inputs are ignored.
pub fn bounds_matrix(
    cached: [f32; 16],
    personality: u32,
    input: Input,
    precision: Precision,
) -> Result<[f32; 16], &'static str> {
    matrix(cached, personality, input, precision, false)
}

/// Source row-major draw matrix after upstream animation/pose updates. This
/// models matrix construction only, not the draw wrapper's other conditions.
pub fn draw_matrix(
    cached: [f32; 16],
    personality: u32,
    input: Input,
    precision: Precision,
) -> Result<[f32; 16], &'static str> {
    matrix(cached, personality, input, precision, true)
}

fn matrix(
    cached: [f32; 16],
    personality: u32,
    input: Input,
    precision: Precision,
    draw: bool,
) -> Result<[f32; 16], &'static str> {
    if personality & 0x84 == 0 {
        return Ok(cached);
    }
    if !input
        .view
        .into_iter()
        .chain(input.position)
        .chain([input.scale])
        .all(f32::is_finite)
    {
        return Err("non-finite camera-facing matrix input");
    }
    let r = |value| precision.round(value);
    let v = input.view.map(f64::from);
    let scale = f64::from(input.scale);
    let mut matrix = [0.; 16];
    matrix[15] = 1.;
    if personality & 4 != 0 && !(draw && personality & 0x80 != 0) {
        if !input.angle.is_finite() || input.angle.abs() > 1_000_000. {
            return Err("unsupported camera-facing matrix angle");
        }
        static TABLE: std::sync::OnceLock<Vec<f32>> = std::sync::OnceLock::new();
        let table = TABLE.get_or_init(crate::object_transform::sine_table);
        let index =
            (r(r(f64::from(input.angle) * f64::from(45.511112f32)) + 0.5) as i32 & 0x3fff) as usize;
        let sine = r(f64::from(table[index]) * scale);
        let cosine = r(f64::from(table[index + 4096]) * scale);
        // The first basis column retains wide sine/cosine products. The third
        // reloads their stored float32 copies, including the negated sine.
        let stored_cosine = f64::from(cosine as f32);
        let stored_negative_sine = f64::from((-sine) as f32);
        for row in 0..3 {
            matrix[row * 4] = r(r(sine * v[row * 4 + 1]) + r(cosine * v[row * 4])) as f32;
            matrix[row * 4 + 1] = r(-scale * v[row * 4 + 2]) as f32;
            matrix[row * 4 + 2] =
                r(r(stored_cosine * v[row * 4 + 1]) + r(stored_negative_sine * v[row * 4])) as f32;
            matrix[row * 4 + 3] = input.position[row];
        }
    } else {
        for row in 0..3 {
            for (column, source_column) in [0, 2, 1].into_iter().enumerate() {
                matrix[row * 4 + column] = r(v[row * 4 + source_column] * scale) as f32;
            }
            let order = if draw {
                [[2, 1, 0], [0, 2, 1], [0, 2, 1]][row]
            } else {
                [[0, 2, 1], [2, 1, 0], [1, 0, 2]][row]
            };
            let product = |k| r(v[row * 4 + k] * v[12 + k]);
            let sum = r(r(product(order[0]) + product(order[1])) + product(order[2]));
            matrix[row * 4 + 3] = r(-sum + f64::from(input.position[row])) as f32;
        }
    }
    if !matrix.into_iter().all(f32::is_finite) {
        return Err("camera-facing matrix overflow");
    }
    Ok(matrix)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounds_bit_four_dominates_combined_flags_and_unused_camera_is_not_required() {
        let mut input = Input {
            view: [0.; 16],
            position: [2., 3., 4.],
            scale: 2.,
            angle: 0.,
        };
        for i in [0, 5, 10, 15] {
            input.view[i] = 1.;
        }
        let cached = [7.; 16];
        let four = bounds_matrix(cached, 4, input, Precision::Double53).unwrap();
        assert_eq!(
            four,
            bounds_matrix(cached, 0x84, input, Precision::Double53).unwrap()
        );
        assert_ne!(
            four,
            bounds_matrix(cached, 0x80, input, Precision::Double53).unwrap()
        );
        assert_eq!(
            draw_matrix(cached, 0x80, input, Precision::Double53).unwrap(),
            draw_matrix(cached, 0x84, input, Precision::Double53).unwrap()
        );
        assert_ne!(
            bounds_matrix(cached, 0x84, input, Precision::Double53).unwrap(),
            draw_matrix(cached, 0x84, input, Precision::Double53).unwrap()
        );
        input.angle = f32::NAN;
        assert!(bounds_matrix(cached, 4, input, Precision::Double53).is_err());
        assert!(bounds_matrix(cached, 0x80, input, Precision::Double53).is_ok());
        input.view[0] = f32::NAN;
        assert_eq!(
            cached,
            bounds_matrix(cached, 0, input, Precision::Double53).unwrap()
        );
    }
}
