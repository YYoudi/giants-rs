//! Scalar overlap predicates; these do not select or resolve a player's volume.
use crate::arithmetic::Precision;

#[derive(Clone, Copy, Debug)]
pub struct Bounds {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl Bounds {
    fn finite(self) -> bool {
        self.min.into_iter().chain(self.max).all(f32::is_finite)
    }
}

/// Inclusive overlap, except that zero X width in either operand is rejected.
/// Inverted bounds are passed through unchanged. Non-finite inputs are outside
/// the verified domain and return None rather than inventing native semantics.
pub fn overlaps(a: Bounds, b: Bounds) -> Option<bool> {
    if !a.finite() || !b.finite() {
        return None;
    }
    Some(
        a.min[0] != a.max[0]
            && b.min[0] != b.max[0]
            && (0..3).all(|axis| b.max[axis] >= a.min[axis] && a.max[axis] >= b.min[axis]),
    )
}

/// Overlap of per-axis point +/- extent with bounds. The native predicate has
/// no zero-X-width guard here. A negative extent is preserved, not normalized.
pub fn expanded_point(
    point: [f32; 3],
    extent: f32,
    bounds: Bounds,
    precision: Precision,
) -> Option<bool> {
    if !bounds.finite() || !extent.is_finite() || !point.into_iter().all(f32::is_finite) {
        return None;
    }
    Some((0..3).all(|axis| {
        let lower = precision.round(point[axis] as f64 - extent as f64);
        let upper = precision.round(extent as f64 + point[axis] as f64);
        bounds.max[axis] as f64 >= lower && upper >= bounds.min[axis] as f64
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unsupported_nonfinite_inputs_remain_explicit() {
        let bounds = Bounds {
            min: [-1.; 3],
            max: [1.; 3],
        };
        assert_eq!(
            expanded_point([f32::NAN, 0., 0.], 0., bounds, Precision::Double53),
            None
        );
        assert_eq!(
            expanded_point([0.; 3], f32::INFINITY, bounds, Precision::Single24),
            None
        );
        assert_eq!(
            overlaps(
                bounds,
                Bounds {
                    min: [f32::NEG_INFINITY; 3],
                    ..bounds
                }
            ),
            None
        );
    }
}
