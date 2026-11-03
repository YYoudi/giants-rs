//! Radius writer in the D physical player object walk, before list lookup.
//! This differs from late collider initialization: no override or class limit,
//! and the scale multiplication precedes the source-bounds factor.
use crate::arithmetic::Precision;

#[derive(Clone, Copy, Debug)]
pub struct Bounds {
    pub primary: [f32; 6],
    /// Bounds of animation slot1, not visual mesh slot1 or the current pose.
    pub animation_one: Option<[f32; 6]>,
}
impl Bounds {
    pub fn refresh(
        self,
        previous: f32,
        scale: f32,
        physics_flags: u32,
        precision: Precision,
    ) -> Result<f32, &'static str> {
        if !previous.is_finite() {
            return Err("nonfinite previous player radius");
        }
        if physics_flags & 0x100 != 0 {
            return Ok(previous);
        }
        let (bounds, factor) = self
            .animation_one
            .map_or((self.primary, 0.5f32), |b| (b, 0.85f32));
        if !scale.is_finite() || !bounds.into_iter().all(f32::is_finite) {
            return Err("nonfinite player radius source");
        }
        let x = (-bounds[0]).max(bounds[3]);
        let y = (-bounds[1]).max(bounds[4]);
        let r = |v| precision.round(v);
        let length = r(r(r(f64::from(x) * f64::from(x)) + r(f64::from(y) * f64::from(y))).sqrt());
        let radius = r(r(length * f64::from(scale)) * f64::from(factor)) as f32;
        if !radius.is_finite() {
            return Err("player radius overflow");
        }
        Ok(radius)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn locked_physics_retains_radius_without_reading_absent_geometry() {
        let bounds = Bounds {
            primary: [f32::NAN; 6],
            animation_one: None,
        };
        assert_eq!(
            bounds.refresh(7., f32::NAN, 0x100, Precision::Double53),
            Ok(7.)
        );
        assert!(bounds.refresh(7., 1., 0, Precision::Double53).is_err());
    }
}
