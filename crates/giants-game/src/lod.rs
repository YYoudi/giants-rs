//! Numeric ten-slot LOD selection. Visibility and full spec decoding are separate.
const HALF_RADIANS: f32 = 0.008_726_646;
pub const QUALITY_FACTORS: [f32; 4] = [10., 3., 1., 0.5];

pub use crate::arithmetic::Precision;

/// Original coordinate-distance operation order, including the offset before
/// the final float32 store. No renderer origin or live precision is inferred.
pub fn distance_from_positions(
    object: [f32; 3],
    camera: [f32; 3],
    offset: f32,
    precision: Precision,
) -> Result<f32, &'static str> {
    if !object
        .into_iter()
        .chain(camera)
        .chain([offset])
        .all(f32::is_finite)
    {
        return Err("non-finite LOD position input");
    }
    let delta: [f64; 3] =
        std::array::from_fn(|i| precision.round(f64::from(camera[i]) - f64::from(object[i])));
    let square = delta.map(|v| precision.round(v * v));
    let sum = precision.round(precision.round(square[0] + square[1]) + square[2]);
    let distance = precision.round(sum.sqrt());
    let result = precision.round(distance + f64::from(offset)) as f32;
    result
        .is_finite()
        .then_some(result)
        .ok_or("LOD coordinate distance overflow")
}

/// The 0x51 record's angular extent and distance produce a selection threshold.
/// Zero angular extent uses the native default of sixty degrees.
pub fn threshold(angle: f32, distance: f32) -> Result<f32, &'static str> {
    threshold_with_precision(angle, distance, Precision::Double53)
}
pub fn threshold_with_precision(
    angle: f32,
    distance: f32,
    precision: Precision,
) -> Result<f32, &'static str> {
    if !angle.is_finite() || !distance.is_finite() || angle.abs() > 1_000_000. {
        return Err("unsupported LOD threshold input");
    }
    let angle = if angle == 0. { 60. } else { angle };
    let radians = precision.round(f64::from(angle) * f64::from(HALF_RADIANS));
    // FSIN supplies its wide result; precision control applies to the following multiply.
    let value = precision.round(radians.sin() * f64::from(distance)) as f32;
    value
        .is_finite()
        .then_some(value)
        .ok_or("LOD threshold overflow")
}

#[derive(Clone, Debug)]
pub struct Table {
    pub present: [bool; 10],
    pub thresholds: [f32; 10],
}
#[derive(Clone, Copy, Debug)]
pub struct Input {
    /// Euclidean distance plus offset, already rounded to float32 by the caller.
    pub distance_with_offset: f32,
    /// Native camera half-angle sine, already stored as float32.
    pub camera_factor: f32,
    pub object_scale: f32,
    pub quality: usize,
    /// A nonnegative value bypasses the automatic selector, including range checks.
    pub forced: i32,
    pub allow_missing: bool,
}
/// Selection inputs before the native coordinate-distance calculation.
#[derive(Clone, Copy, Debug)]
pub struct PositionInput {
    pub object_position: [f32; 3],
    pub camera_position: [f32; 3],
    pub offset: f32,
    pub camera_factor: f32,
    pub object_scale: f32,
    pub quality: usize,
    pub forced: i32,
    pub allow_missing: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct Distances {
    pub unscaled: f32,
    pub quality_scaled: f32,
}
pub fn distances(input: Input) -> Result<Distances, &'static str> {
    distances_with_precision(input, Precision::Double53)
}
pub fn distances_with_precision(
    input: Input,
    precision: Precision,
) -> Result<Distances, &'static str> {
    let quality = *QUALITY_FACTORS
        .get(input.quality)
        .ok_or("unsupported model quality")?;
    if ![
        input.distance_with_offset,
        input.camera_factor,
        input.object_scale,
    ]
    .into_iter()
    .all(f32::is_finite)
        || input.object_scale == 0.
    {
        return Err("unsupported LOD distance input");
    }
    // Quality multiplies the wide intermediate, not its already-rounded copy.
    let factor = precision.round(f64::from(input.camera_factor) / f64::from(input.object_scale));
    let wide = precision.round(factor * f64::from(input.distance_with_offset));
    let unscaled = wide as f32;
    let quality_scaled = precision.round(wide * f64::from(quality)) as f32;
    if !unscaled.is_finite() || !quality_scaled.is_finite() {
        return Err("LOD distance overflow");
    }
    Ok(Distances {
        unscaled,
        quality_scaled,
    })
}
impl Table {
    /// Geometry slot used by collision preparation. This is independent of
    /// camera distance, quality and the current render slot. A -1 threshold
    /// returns its own index, even when that slot is absent.
    pub fn collision_slot(&self) -> Result<usize, &'static str> {
        if !self.thresholds.into_iter().all(f32::is_finite) {
            return Err("non-finite collision model threshold");
        }
        let mut last = 0;
        let mut previous = 0;
        for index in 0..10 {
            let threshold = self.thresholds[index];
            if threshold == -1. {
                return Ok(index);
            }
            if self.present[index] {
                if index > 0 && threshold <= 0. {
                    return Ok(previous);
                }
                previous = last;
                last = index;
            }
        }
        Ok(previous)
    }
    fn early_return_zero(&self) -> bool {
        self.thresholds[1] <= 0. && (self.thresholds[1] != -1. || self.thresholds[2] <= 0.)
    }
    pub fn select_positioned(
        &self,
        input: PositionInput,
        precision: Precision,
    ) -> Result<i32, &'static str> {
        let mut numeric = Input {
            distance_with_offset: 0.,
            camera_factor: input.camera_factor,
            object_scale: input.object_scale,
            quality: input.quality,
            forced: input.forced,
            allow_missing: input.allow_missing,
        };
        // Native forced and fixed-table paths do not evaluate coordinates.
        if input.forced < 0
            && !self.early_return_zero()
            && self.thresholds.into_iter().all(f32::is_finite)
        {
            numeric.distance_with_offset = distance_from_positions(
                input.object_position,
                input.camera_position,
                input.offset,
                precision,
            )?;
        }
        self.select_with_precision(numeric, precision)
    }
    /// Array inputs must describe the actual spec table, including negative
    /// sentinels and absent entries. A consecutive prefix alone may be incomplete.
    pub fn select(&self, input: Input) -> Result<i32, &'static str> {
        self.select_with_precision(input, Precision::Double53)
    }
    /// Thresholds must already reflect the spec initialization policy. Selection
    /// precision is explicit and may differ from initialization precision.
    pub fn select_with_precision(
        &self,
        input: Input,
        precision: Precision,
    ) -> Result<i32, &'static str> {
        if input.forced >= 0 {
            return Ok(input.forced);
        }
        if !self.thresholds.into_iter().all(f32::is_finite) {
            return Err("non-finite LOD table");
        }
        if self.early_return_zero() {
            return Ok(0);
        }
        let distances = distances_with_precision(input, precision)?;
        let mut slot = 0;
        while slot < 9 {
            let next = slot + 1;
            let threshold = self.thresholds[next];
            if threshold != -1. {
                if !self.present[next] && !input.allow_missing {
                    break;
                }
                let distance = if self.present[next] {
                    distances.quality_scaled
                } else {
                    distances.unscaled
                };
                if threshold > distance || threshold <= 0. {
                    break;
                }
            }
            slot += 1;
        }
        Ok(slot as i32 - i32::from(self.thresholds[slot] == -1.))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn input(distance: f32) -> Input {
        Input {
            distance_with_offset: distance,
            camera_factor: 1.,
            object_scale: 1.,
            quality: 2,
            forced: -1,
            allow_missing: false,
        }
    }
    #[test]
    fn equality_advances_sentinels_and_absent_models_remain_distinct() {
        let mut table = Table {
            present: [true; 10],
            thresholds: [0., 100., 200., 300., 400., 500., 600., 700., 800., 900.],
        };
        assert_eq!(table.select(input(99.)).unwrap(), 0);
        assert_eq!(table.select(input(100.)).unwrap(), 1);
        assert_eq!(table.select(input(1000.)).unwrap(), 9);
        table.thresholds[1] = -1.;
        assert_eq!(table.select(input(0.)).unwrap(), 0);
        table.thresholds[0] = -1.;
        table.thresholds[1] = 100.;
        assert_eq!(table.select(input(0.)).unwrap(), -1);
        table.thresholds[0] = 0.;
        table.present[1] = false;
        assert_eq!(table.select(input(1000.)).unwrap(), 0);
        assert_eq!(
            table
                .select(Input {
                    allow_missing: true,
                    ..input(1000.)
                })
                .unwrap(),
            9
        );
        assert_eq!(
            table
                .select(Input {
                    forced: 42,
                    object_scale: 0.,
                    ..input(0.)
                })
                .unwrap(),
            42
        );
    }
    #[test]
    fn coordinates_offset_and_wide_exponent_are_explicit() {
        for precision in [Precision::Single24, Precision::Double53] {
            assert_eq!(
                distance_from_positions([0.; 3], [3., 4., 0.], -2., precision).unwrap(),
                3.
            );
            assert_eq!(
                distance_from_positions([0.; 3], [1e30, 0., 0.], 0., precision).unwrap(),
                1e30
            );
            assert!(distance_from_positions([f32::NAN, 0., 0.], [0.; 3], 0., precision).is_err());
        }
        // A float32 sqrt before adding the offset erases this residual.
        let result = distance_from_positions(
            [0.; 3],
            [1., 1., 0.],
            -std::f32::consts::SQRT_2,
            Precision::Double53,
        )
        .unwrap();
        assert_ne!(result, 0.);
        let mut table = Table {
            present: [true; 10],
            thresholds: [0.; 10],
        };
        let positioned = PositionInput {
            object_position: [f32::NAN; 3],
            camera_position: [0.; 3],
            offset: 0.,
            camera_factor: 0.,
            object_scale: 0.,
            quality: 99,
            forced: -1,
            allow_missing: false,
        };
        assert_eq!(
            table
                .select_positioned(positioned, Precision::Single24)
                .unwrap(),
            0
        );
        table.thresholds[1] = 1.;
        assert_eq!(
            table
                .select_positioned(
                    PositionInput {
                        forced: 42,
                        ..positioned
                    },
                    Precision::Double53
                )
                .unwrap(),
            42
        );
        assert!(
            table
                .select_positioned(positioned, Precision::Double53)
                .is_err()
        );
        assert_eq!(
            distance_from_positions(
                [0.; 3],
                [1., 1., 0.],
                -std::f32::consts::SQRT_2,
                Precision::Single24
            )
            .unwrap(),
            0.
        );
    }
    #[test]
    fn threshold_default_and_invalid_numeric_inputs() {
        assert_eq!(
            threshold(0., 700.).unwrap().to_bits(),
            threshold(60., 700.).unwrap().to_bits()
        );
        assert_eq!(threshold(60., 700.).unwrap(), 350.);
        assert!(threshold(f32::NAN, 1.).is_err());
        assert!(
            distances(Input {
                quality: 4,
                ..input(1.)
            })
            .is_err()
        );
        assert!(
            distances(Input {
                object_scale: 0.,
                ..input(1.)
            })
            .is_err()
        );
    }
}
