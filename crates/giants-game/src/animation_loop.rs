//! One unblended looping channel. Clip selection, callbacks and pose blending
//! belong to separate stages; the caller supplies its playback-rate factor.
use crate::arithmetic::Precision;

/// Channel reference speed zero selects unit playback. Its model-speed source
/// is then unused, including the absent-source sentinel used by idle clips.
pub fn playback_rate<const N: usize>(
    reference_speed: f32,
    speed_source: i32,
    model: [f32; N],
    precision: Precision,
) -> Result<f64, &'static str> {
    if reference_speed == 0. {
        return Ok(1.);
    }
    let source = usize::try_from(speed_source).map_err(|_| "invalid animation speed source")?;
    let speed = *model
        .get(source)
        .filter(|_| source < 4)
        .ok_or("invalid animation speed source")?;
    if !speed.is_finite() || !reference_speed.is_finite() {
        return Err("nonfinite animation rate");
    }
    Ok(precision.round(f64::from(speed) / f64::from(reference_speed)))
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Clock {
    pub seconds: f32,
    pub previous: f32,
    pub overflow: f32,
    pub wrapped: bool,
}
impl Clock {
    pub fn advance(
        &mut self,
        duration: f32,
        dt: f32,
        rate: f32,
        precision: Precision,
    ) -> Result<(), &'static str> {
        self.advance_with_rate(duration, dt, f64::from(rate), precision)
    }
    /// Keep a native model-speed/reference-speed ratio wide until the clock store.
    pub fn advance_with_rate(
        &mut self,
        duration: f32,
        dt: f32,
        rate: f64,
        precision: Precision,
    ) -> Result<(), &'static str> {
        if ![duration, dt, self.seconds].into_iter().all(f32::is_finite)
            || !rate.is_finite()
            || duration <= 0.
            || dt < 0.
            || self.seconds < 0.
            || self.seconds > duration
            || (rate * f64::from(dt)).abs() / f64::from(duration) > 4096.
        {
            return Err("animation loop outside finite bounded channel profile");
        }
        let r = |v| precision.round(v);
        let time = r(r(rate * f64::from(dt)) + f64::from(self.seconds));
        if !(time as f32).is_finite() {
            return Err("animation clock store overflow");
        }
        self.previous = self.seconds;
        self.seconds = time as f32;
        self.overflow = 0.;
        self.wrapped = false;
        if rate >= 0. && f64::from(duration) <= time {
            self.wrapped = true;
            while self.seconds >= duration {
                self.seconds = r(f64::from(self.seconds) - f64::from(duration)) as f32;
            }
            self.overflow = self.seconds;
        } else if rate < 0. && time < 0. {
            self.wrapped = true;
            let final_wide = loop {
                let next = r(f64::from(duration) + f64::from(self.seconds));
                self.seconds = next as f32;
                if next >= 0. {
                    break next;
                }
            };
            self.overflow = r(f64::from(duration) - final_wide) as f32;
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn endpoint_and_reverse_wrap_keep_callback_interval() {
        let mut clock = Clock::default();
        clock.advance(1., 1., 1., Precision::Double53).unwrap();
        assert_eq!(clock.seconds, 0.);
        assert!(clock.wrapped);
        clock.advance(1., 0.25, -1., Precision::Double53).unwrap();
        assert_eq!(clock.seconds, 0.75);
        assert_eq!(clock.previous, 0.);
        assert_eq!(clock.overflow, 0.25);
        assert!(clock.wrapped);
    }
    #[test]
    fn finite_inputs_that_overflow_the_store_are_refused_before_mutation() {
        let mut clock = Clock {
            seconds: f32::MAX,
            ..Clock::default()
        };
        assert!(
            clock
                .advance(f32::MAX, f32::MAX, 1., Precision::Double53)
                .is_err()
        );
        assert_eq!(clock.seconds, f32::MAX);
        assert_eq!(clock.previous, 0.);
    }
}
