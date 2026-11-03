//! Current actor affine-cache writer, separate from callback/list scheduling.
//! The external-orientation flag retains the acquired linear cache and only
//! replaces translation. It never supplies an identity for a missing cache.
use crate::{arithmetic::Precision, uncached_pose::Pose};

#[derive(Clone, Copy, Debug)]
pub struct Cache {
    pub flags: u32,
    pub matrix: [f32; 16],
}

pub fn update(
    pose: Pose,
    active: u32,
    previous: Option<Cache>,
    precision: Precision,
) -> Result<(u32, Cache), &'static str> {
    if !pose.position.into_iter().all(f32::is_finite) {
        return Err("non-finite actor affine translation");
    }
    let mut cache = if active & 0x10000 != 0 {
        previous.ok_or("external actor orientation cache absent")?
    } else {
        let rows = crate::uncached_pose::matrix(pose, precision)?;
        let mut matrix = [0.; 16];
        matrix[..12].copy_from_slice(&rows);
        Cache {
            flags: 1
                | if pose.scale.to_bits() != 1f32.to_bits() {
                    2
                } else {
                    0
                }
                | if pose.scale < 0. { 8 } else { 0 },
            matrix,
        }
    };
    cache.flags |= 0x10;
    for (i, value) in pose.position.into_iter().enumerate() {
        cache.matrix[i * 4 + 3] = value;
    }
    cache.matrix[12..].copy_from_slice(&[0., 0., 0., 1.]);
    Ok((active | 0x20, cache))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn external_orientation_requires_a_real_cache_and_preserves_its_linear_part() {
        let pose = Pose {
            position: [1., 2., 3.],
            angles: [f32::NAN; 3],
            scale: f32::NAN,
        };
        assert!(update(pose, 0x10000, None, Precision::Double53).is_err());
        let previous = Cache {
            flags: 0x82,
            matrix: std::array::from_fn(|i| i as f32),
        };
        let (active, cache) = update(pose, 0x10040, Some(previous), Precision::Double53).unwrap();
        assert_eq!(active, 0x10060);
        assert_eq!(cache.flags, 0x92);
        assert_eq!(
            [cache.matrix[0], cache.matrix[5], cache.matrix[10]],
            [0., 5., 10.]
        );
        assert_eq!(
            [cache.matrix[3], cache.matrix[7], cache.matrix[11]],
            pose.position
        );
        assert_eq!(&cache.matrix[12..], &[0., 0., 0., 1.]);
    }
    #[test]
    fn negative_and_zero_scale_keep_native_flags() {
        for (scale, flags) in [(1., 0x11), (0., 0x13), (-0., 0x13), (-1., 0x1b)] {
            let (_, cache) = update(
                Pose {
                    position: [0.; 3],
                    angles: [0.; 3],
                    scale,
                },
                0,
                None,
                Precision::Double53,
            )
            .unwrap();
            assert_eq!(cache.flags, flags);
        }
    }
}
