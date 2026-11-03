//! Director frame-height envelope. Advances at the render boundary, not from ticks.
use crate::arithmetic::Precision;

#[derive(Clone, Copy, Debug)]
pub struct Frame {
    pub fraction: f32,
    pub target: f32,
    pub speed: f32,
}
impl Default for Frame {
    fn default() -> Self {
        Self {
            fraction: 1.,
            target: 1.,
            speed: 0.,
        }
    }
}
impl Frame {
    pub fn set(
        &mut self,
        target: f32,
        duration: f32,
        precision: Precision,
    ) -> Result<(), &'static str> {
        if !self.fraction.is_finite()
            || !(0. ..=1.).contains(&target)
            || !duration.is_finite()
            || !(0. ..=3600.).contains(&duration)
        {
            return Err("cinematic frame outside supported envelope profile");
        }
        let speed = if duration == 0. {
            0.
        } else {
            precision.round(
                precision.round(f64::from(target) - f64::from(self.fraction)) / f64::from(duration),
            ) as f32
        };
        if !speed.is_finite() {
            return Err("cinematic frame speed overflow");
        }
        self.target = target;
        self.speed = speed;
        if duration == 0. {
            self.fraction = target;
        }
        Ok(())
    }
    pub fn advance(&mut self, dt: f32, precision: Precision) -> Result<(), &'static str> {
        if !(0. ..=1.).contains(&dt)
            || !self.fraction.is_finite()
            || !self.target.is_finite()
            || !self.speed.is_finite()
        {
            return Err("invalid cinematic frame update");
        }
        if self.speed != 0. {
            let value = precision.round(
                precision.round(f64::from(self.speed) * f64::from(dt)) + f64::from(self.fraction),
            );
            self.fraction = value as f32;
            // Compare the retained arithmetic result, before its f32 store.
            if self.speed < 0. && value < f64::from(self.target)
                || self.speed >= 0. && value > f64::from(self.target)
            {
                self.fraction = self.target;
                self.speed = 0.;
            }
        }
        Ok(())
    }
    pub fn half_height(self, height: u32, precision: Precision) -> Result<u32, &'static str> {
        if !(1..=16384).contains(&height) || !(0. ..=1.).contains(&self.fraction) {
            return Err("cinematic viewport outside supported dimensions");
        }
        Ok(precision.round(f64::from(height) * 0.5 * f64::from(self.fraction)) as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_target_keeps_speed_until_strict_crossing_and_zero_dt_does_not_progress() {
        let mut frame = Frame::default();
        frame.set(0.75, 1., Precision::Double53).unwrap();
        frame.advance(0., Precision::Double53).unwrap();
        assert_eq!(frame.fraction, 1.);
        frame.advance(1., Precision::Double53).unwrap();
        assert_eq!((frame.fraction, frame.speed), (0.75, -0.25));
        frame.advance(0.1, Precision::Double53).unwrap();
        assert_eq!((frame.fraction, frame.speed), (0.75, 0.));
        assert_eq!(frame.half_height(1080, Precision::Double53).unwrap(), 405);
        frame.set(1., 0., Precision::Double53).unwrap();
        assert_eq!((frame.fraction, frame.target, frame.speed), (1., 1., 0.));
        assert!(frame.set(0.5, -1., Precision::Double53).is_err());
        assert!(frame.advance(f32::NAN, Precision::Double53).is_err());
    }
}
