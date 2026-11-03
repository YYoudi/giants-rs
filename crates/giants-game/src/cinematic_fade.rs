//! Director screen fade. Zero/negative source color channels preserve old bytes.
//! The render-boundary envelope and alpha byte retain explicit x87 precision.
use crate::{arithmetic::Precision, cinematic_frame::Frame};
#[derive(Clone, Copy, Debug, Default)]
pub struct Fade {
    pub envelope: Frame,
    pub color: [u8; 3],
}
impl Fade {
    pub fn set(
        &mut self,
        color: [f32; 3],
        opacity: f32,
        duration: f32,
        precision: Precision,
    ) -> Result<(), &'static str> {
        if !color.iter().all(|v| v.is_finite() && *v <= 1.) || !(0. ..=1.).contains(&opacity) {
            return Err("cinematic fade outside supported color/opacity profile");
        }
        let mut next = *self;
        let transparency = precision.round(1. - f64::from(opacity)) as f32;
        next.envelope.set(transparency, duration, precision)?;
        for (channel, value) in next.color.iter_mut().zip(color) {
            if value > 0. {
                *channel = precision.round(f64::from(value) * 255.) as u8;
            }
        }
        *self = next;
        Ok(())
    }
    pub fn advance(&mut self, dt: f32, precision: Precision) -> Result<(), &'static str> {
        self.envelope.advance(dt, precision)
    }
    pub fn alpha_byte(self, precision: Precision) -> Result<u8, &'static str> {
        if !(0. ..=1.).contains(&self.envelope.fraction) {
            return Err("cinematic fade transparency outside unit interval");
        }
        Ok(precision.round(precision.round(1. - f64::from(self.envelope.fraction)) * 255.) as u8)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn color_zero_preserves_bytes_and_opacity_is_complemented() {
        let mut f = Fade {
            color: [91, 92, 93],
            ..Default::default()
        };
        f.set([0., -1., 0.5], 1., 0.25, Precision::Double53)
            .unwrap();
        assert_eq!(f.color, [91, 92, 127]);
        assert_eq!(f.envelope.target, 0.);
        f.advance(0.25, Precision::Double53).unwrap();
        assert_eq!(f.envelope.speed, -4.);
        assert_eq!(f.alpha_byte(Precision::Double53), Ok(255));
        f.advance(0.01, Precision::Double53).unwrap();
        assert_eq!(f.envelope.speed, 0.);
        assert!(
            f.set([f32::NAN, 0., 0.], 1., 0., Precision::Double53)
                .is_err()
        );
    }
}
