//! Cached world/local point transforms used by D's mesh-contact wrapper.
//! The current actor cache and its flags remain explicit inputs. No pose is
//! inferred from a rendered entity or substituted for an absent native cache.
#[derive(Clone, Copy, Debug)]
pub struct Cached {
    world: [f32; 16],
    local: [f32; 16],
}

impl Cached {
    pub fn new(active: u32, flags: u32, matrix: [f32; 16]) -> Result<Self, String> {
        if active & 0x20 == 0 {
            return Err("contact pose requires the current affine-cache flag".into());
        }
        let local = crate::animation::inverse_affine(matrix, flags).map_err(|e| e.to_string())?;
        Ok(Self {
            world: matrix,
            local,
        })
    }

    pub fn to_world(self, point: [f32; 3]) -> Result<[f32; 3], &'static str> {
        crate::footprint_builder::transform_cached(
            self.world[..12].try_into().unwrap(),
            point,
            crate::arithmetic::Precision::Double53,
        )
    }

    pub fn to_local(self, point: [f32; 3]) -> Result<[f32; 3], &'static str> {
        // The inverse helper accumulates Z,Y,X; the forward cached helper
        // accumulates Z,X,Y. Reusing the forward helper changes rounding.
        crate::uncached_pose::transform(
            self.local[..12].try_into().unwrap(),
            point,
            crate::arithmetic::Precision::Double53,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_cache_and_singular_transform_are_not_replaced_with_identity() {
        let identity = std::array::from_fn(|i| f32::from(i / 4 == i % 4));
        assert!(Cached::new(0, 1, identity).is_err());
        let mut singular = identity;
        singular[0] = 0.;
        assert!(Cached::new(0x20, 2, singular).is_err());
    }
}
