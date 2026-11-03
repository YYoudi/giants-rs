//! Object bounds writer with an explicit affine cache or uncached Euler pose.
//! Model table/animation selection and live object state remain upstream.
use crate::{arithmetic::Precision, footprint_builder::transform_cached};
pub struct Models {
    pub primary: [f32; 6],
    /// Caller-provided lookup of the current auxiliary slot, when it exists.
    pub auxiliary: Option<[f32; 6]>,
    pub slot: u32,
}
pub fn cached(
    models: Option<Models>,
    matrix: [f32; 12],
    precision: Precision,
) -> Result<[f32; 6], &'static str> {
    with_transform(models, precision, |p| {
        transform_cached(matrix, p, precision)
    })
}

/// Reconstruct the native uncached pose, without manufacturing a cache flag.
pub fn uncached(
    models: Option<Models>,
    pose: crate::uncached_pose::Pose,
    precision: Precision,
) -> Result<[f32; 6], &'static str> {
    let matrix = crate::uncached_pose::matrix(pose, precision)?;
    with_transform(models, precision, |p| {
        crate::uncached_pose::transform(matrix, p, precision)
    })
}

fn with_transform(
    models: Option<Models>,
    precision: Precision,
    mut transform: impl FnMut([f32; 3]) -> Result<[f32; 3], &'static str>,
) -> Result<[f32; 6], &'static str> {
    let Some(models) = models else {
        let origin = transform([0.; 3])?;
        let upper = origin.map(|v| precision.round(f64::from(v) + 1.) as f32);
        return Ok([
            origin[0], origin[1], origin[2], upper[0], upper[1], upper[2],
        ]);
    };
    let bounds = if models.slot == 254 {
        models.primary
    } else {
        models.auxiliary.unwrap_or(models.primary)
    };
    if !bounds.iter().all(|v| v.is_finite()) {
        return Err("non-finite selected collision bounds");
    }
    let mut result = [0.; 6];
    for corner in 0..8 {
        let point = std::array::from_fn(|axis| {
            bounds[axis + if corner & (1 << axis) == 0 { 3 } else { 0 }]
        });
        let point = transform(point)?;
        for i in 0..3 {
            if corner == 0 {
                result[i] = point[i];
                result[i + 3] = point[i];
            } else {
                if point[i] < result[i] {
                    result[i] = point[i];
                }
                if point[i] > result[i + 3] {
                    result[i + 3] = point[i];
                }
            }
        }
    }
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn forced_primary_does_not_use_auxiliary_and_no_class_has_unit_extent() {
        let matrix = [0., -2., 0., 10., 1., 0., 0., 20., 0., 0., 0.5, 30.];
        let models = Models {
            primary: [-1., -2., -3., 4., 5., 6.],
            auxiliary: Some([100.; 6]),
            slot: 254,
        };
        assert_eq!(
            cached(Some(models), matrix, Precision::Double53).unwrap(),
            [0., 19., 28.5, 14., 24., 33.]
        );
        assert_eq!(
            cached(None, matrix, Precision::Double53).unwrap(),
            [10., 20., 30., 11., 21., 31.]
        );
    }
}
