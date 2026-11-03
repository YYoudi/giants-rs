//! Cinematic lens with separate full-screen and current viewport aspects.
use giants_formats::Error;
#[derive(Clone, Copy, Debug)]
pub struct Context {
    pub extent: [u32; 2],
    pub fraction: f32,
}
pub struct Lens {
    pub viewport: [u32; 4],
    pub projection: [f32; 16],
    pub lod_scale: f32,
}
impl Context {
    pub const INSPECTION: Self = Self {
        extent: [1920, 1080],
        fraction: 0.75,
    };
    pub fn viewport(self) -> Result<[u32; 4], Error> {
        let [width, height] = self.extent;
        if !(1..=16384).contains(&width)
            || !(2..=16384).contains(&height)
            || !height.is_multiple_of(2)
        {
            return Err(Error("unsupported cinematic screen dimensions"));
        }
        let half = crate::cinematic_frame::Frame {
            fraction: self.fraction,
            target: self.fraction,
            speed: 0.,
        }
        .half_height(height, crate::arithmetic::Precision::Double53)
        .map_err(Error)?;
        if half == 0 {
            return Err(Error("cinematic viewport has no drawable height"));
        }
        Ok([0, height / 2 - half, width, half * 2])
    }
    pub fn lens(self, fov: f32, far: f32) -> Result<Lens, Error> {
        if !fov.is_finite()
            || !(0. ..std::f32::consts::PI).contains(&fov)
            || fov == 0.
            || !far.is_finite()
            || far > 25000.
            || far == 0.
            || far == 1.
        {
            return Err(Error("invalid cinematic lens input"));
        }
        let viewport = self.viewport()?;
        let full_aspect = self.extent[0] as f32 / self.extent[1] as f32;
        let viewport_aspect = viewport[2] as f32 / viewport[3] as f32;
        let vertical_degrees = ((f64::from(fov) * 0.5).tan() / f64::from(full_aspect)).atan()
            * f64::from(114.59156f32);
        let angle = f64::from(vertical_degrees as f32) * f64::from(0.008726646f32);
        let sine = angle.sin() as f32;
        let ratio = angle.cos() / f64::from(sine);
        let mut projection = [0.; 16];
        projection[0] = (ratio / f64::from(viewport_aspect)) as f32;
        projection[5] = ratio as f32;
        projection[10] = (f64::from(far) / (f64::from(far) - 1.)) as f32;
        projection[11] = 1.;
        projection[14] = -projection[10];
        Ok(Lens {
            viewport,
            projection,
            lod_scale: sine,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn signed_adaptive_far_keeps_the_native_projection_without_a_positive_fallback() {
        let lens = Context::INSPECTION
            .lens(std::f32::consts::FRAC_PI_4, -5000.)
            .unwrap();
        assert_eq!(lens.projection[10], (5000f64 / 5001.) as f32);
        assert_eq!(lens.projection[14], -lens.projection[10]);
        assert!(Context::INSPECTION.lens(1., 0.).is_err());
        assert!(Context::INSPECTION.lens(1., 1.).is_err());
    }
    #[test]
    fn shrinking_frame_changes_horizontal_projection_without_changing_full_screen_fov() {
        let full = Context {
            extent: [1920, 1080],
            fraction: 1.,
        }
        .lens(std::f32::consts::FRAC_PI_4, 12500.)
        .unwrap();
        let narrow = Context::INSPECTION
            .lens(std::f32::consts::FRAC_PI_4, 12500.)
            .unwrap();
        assert_eq!(narrow.viewport, [0, 135, 1920, 810]);
        assert_eq!(full.projection[5], narrow.projection[5]);
        assert_eq!(full.lod_scale, narrow.lod_scale);
        assert_ne!(full.projection[0], narrow.projection[0]);
        for context in [
            Context {
                extent: [1920, 1081],
                fraction: 0.75,
            },
            Context {
                extent: [1920, 1080],
                fraction: 0.,
            },
            Context {
                extent: [1920, 1080],
                fraction: f32::NAN,
            },
        ] {
            assert!(context.viewport().is_err());
        }
    }
}
