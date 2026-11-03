//! Source camera composition with explicit scalar precision. The output is the
//! row-vector view/projection product, stored in the source array convention.
use crate::arithmetic::Precision;

/// A disabled projection copies the view, including signed zero. Inputs and
/// outputs are separate values; overlapping native buffers are not modeled.
pub fn compose(view: [f32; 16], projection: Option<[f32; 16]>, precision: Precision) -> [f32; 16] {
    let Some(projection) = projection else {
        return view;
    };
    std::array::from_fn(|i| {
        let row = i / 4;
        let column = i % 4;
        // Addition order differs between the first row and following rows.
        // Keep products wide until each scalar output is stored.
        let order = if row == 0 {
            match column {
                0 | 2 => [0, 2, 1, 3],
                1 => [2, 0, 1, 3],
                _ => [3, 1, 0, 2],
            }
        } else {
            match column {
                0 | 2 => [1, 3, 2, 0],
                1 => [1, 0, 3, 2],
                _ => [0, 3, 2, 1],
            }
        };
        let product = |k| {
            precision.round(f64::from(view[row * 4 + k]) * f64::from(projection[k * 4 + column]))
        };
        let mut sum = product(order[0]);
        for k in &order[1..] {
            sum = precision.round(sum + product(*k));
        }
        sum as f32
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancellation_preserves_the_source_addition_order_and_disabled_copy() {
        let mut view = [0.; 16];
        view[..4].copy_from_slice(&[1e20, 1., -1e20, 3.]);
        for precision in [Precision::Single24, Precision::Double53] {
            assert_eq!(compose(view, Some([1.; 16]), precision)[0], 4.);
        }
        view[4] = -0.;
        assert_eq!(
            compose(view, None, Precision::Double53).map(f32::to_bits),
            view.map(f32::to_bits)
        );
    }
}
