//! One source vertical-fade segment, advanced only on visible state78 callbacks.
//! Mesh bounds and deformed local Z are explicit inputs; no raster is inferred.
//! Prototype PS2 `fx_move` / `fx_setupVerticleFade` remain unknown PC attributions.
#[derive(Clone, Copy, Debug)]
pub struct Configuration {
    pub start: [f32; 2],
    pub end: [f32; 2],
    pub duration: f32,
}
impl Configuration {
    pub fn validate(self) -> Result<Self, &'static str> {
        if !self.duration.is_finite()
            || self.duration <= 0.
            || self
                .start
                .into_iter()
                .chain(self.end)
                .any(|v| !v.is_finite() || !(0. ..=1.).contains(&v))
        {
            return Err("vertical fade outside bounded single-segment profile");
        }
        Ok(self)
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Playback {
    configuration: Configuration,
    pub elapsed: f32,
    pub values: [f32; 2],
    pub active: bool,
}
impl Playback {
    pub fn new(configuration: Configuration) -> Result<Self, &'static str> {
        Ok(Self {
            configuration: configuration.validate()?,
            elapsed: 0.,
            values: [0.; 2],
            active: true,
        })
    }
    pub fn advance(&mut self, dt: f32) -> Result<(), &'static str> {
        if !dt.is_finite() || dt < 0. {
            return Err("invalid vertical-fade delta");
        }
        if !self.active {
            return Ok(());
        }
        self.elapsed = (f64::from(self.elapsed) + f64::from(dt)) as f32;
        let c = self.configuration;
        if self.elapsed > c.duration {
            self.values = c.end;
            self.elapsed = 0.;
            self.active = false;
        } else {
            self.values = std::array::from_fn(|i| {
                ((f64::from(c.end[i]) - f64::from(c.start[i])) / f64::from(c.duration)
                    * f64::from(self.elapsed)
                    + f64::from(c.start[i])) as f32
            });
        }
        Ok(())
    }
    pub fn alpha(&self, z: f32, bounds: [f32; 2]) -> Result<u8, &'static str> {
        alpha(z, bounds, self.values)
    }
}
pub fn alpha(z: f32, bounds: [f32; 2], values: [f32; 2]) -> Result<u8, &'static str> {
    let height = (f64::from(bounds[1]) - f64::from(bounds[0])) as f32;
    if !z.is_finite()
        || !height.is_finite()
        || height <= 0.
        || bounds.into_iter().chain(values).any(|v| !v.is_finite())
    {
        return Err("vertical-fade vertex or bounds outside finite profile");
    }
    let a = (((f64::from(z) - f64::from(bounds[0])) * (1. / f64::from(height)))
        * (f64::from(values[0]) - f64::from(values[1]))
        + f64::from(values[1]))
        * 255.;
    if !a.is_finite() || a < f64::from(i32::MIN) || a > f64::from(i32::MAX) {
        return Err("vertical-fade integer conversion outside profile");
    }
    Ok(a as i32 as u8)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_gradient_and_segment_retirement() {
        let mut p = Playback::new(Configuration {
            start: [1., 0.],
            end: [0.; 2],
            duration: 0.7,
        })
        .unwrap();
        assert_eq!(p.values, [0.; 2]);
        p.advance(0.05).unwrap();
        assert_eq!(p.values[0].to_bits(), 1064154843);
        let mut exact = Playback::new(Configuration {
            start: [1., 0.25],
            end: [0., 0.5],
            duration: 0.5,
        })
        .unwrap();
        exact.advance(0.5).unwrap();
        assert_eq!(exact.values, [0., 0.5]);
        assert!(exact.active);
        assert_eq!(exact.elapsed, 0.5);
        exact.advance(0.00001).unwrap();
        assert!(!exact.active);
        assert_eq!(exact.elapsed, 0.);
        assert_eq!(exact.values, [0., 0.5]);
        p.advance(0.7).unwrap();
        assert!(!p.active);
        assert_eq!((p.elapsed, p.values), (0., [0.; 2]));
        for (z, expected) in [-2., -1., 0., 1., 2., 3.]
            .into_iter()
            .zip([51, 81, 112, 142, 173, 204])
        {
            assert_eq!(alpha(z, [-2., 3.], [0.8, 0.2]).unwrap(), expected);
        }
        assert!(alpha(0., [0., 0.], [1., 0.]).is_err());
    }
    #[test]
    fn exact_duration_and_zero_delta_keep_the_native_segment_active() {
        let mut fade = Playback::new(Configuration {
            start: [0.; 2],
            end: [0., 1.],
            duration: 0.1,
        })
        .unwrap();
        fade.advance(0.1).unwrap();
        assert!(fade.active);
        assert_eq!(fade.elapsed.to_bits(), 0.1f32.to_bits());
        assert_eq!(fade.values, [0., 1.]);
        fade.advance(0.).unwrap();
        assert!(fade.active);
        fade.advance(0.01).unwrap();
        assert!(!fade.active);
        assert_eq!(fade.elapsed, 0.);
        assert_eq!(fade.values, [0., 1.]);
    }
}
