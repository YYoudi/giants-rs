//! Owner-side tracer callbacks with a supplied post-motion delta.
//! This does not schedule owners, advance global history, or release slots.
pub const ACTIVE: u32 = 0x4000;
pub const FADING_IN: u32 = 0x20000;
pub const INITIAL_SAMPLE: u32 = 0x2000;
#[derive(Clone, Debug)]
pub struct Owner {
    pub sampling_remaining: f32,
    pub fade_duration: f32,
    pub fade_remaining: f32,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Actions {
    pub opacity: Option<f32>,
    pub sample: bool,
}
pub fn consume_initial_sample(flags: &mut u32) -> bool {
    let requested = *flags & INITIAL_SAMPLE != 0;
    *flags &= !INITIAL_SAMPLE;
    requested
}
impl Owner {
    pub fn new(
        sampling_remaining: f32,
        fade_duration: f32,
        fade_remaining: f32,
    ) -> Result<Self, &'static str> {
        let value = Self {
            sampling_remaining,
            fade_duration,
            fade_remaining,
        };
        value.validate()?;
        Ok(value)
    }
    fn validate(&self) -> Result<(), &'static str> {
        if !self.sampling_remaining.is_finite()
            || !self.fade_duration.is_finite()
            || !self.fade_remaining.is_finite()
            || self.fade_duration < 0.
            || self.fade_remaining > self.fade_duration
        {
            return Err("tracer owner outside finite countdown profile");
        }
        Ok(())
    }
    pub fn post_motion(&mut self, flags: &mut u32, delta: f32) -> Result<Actions, &'static str> {
        self.validate()?;
        if !delta.is_finite() || delta < 0. {
            return Err("invalid tracer owner delta");
        }
        if *flags & ACTIVE == 0 {
            return Ok(Actions::default());
        }
        let sampling = (f64::from(self.sampling_remaining) - f64::from(delta)) as f32;
        if !sampling.is_finite() {
            return Err("tracer owner countdown overflow");
        }
        if sampling > 0. {
            self.sampling_remaining = sampling;
            return Ok(Actions::default());
        }
        let mut result = Actions {
            sample: true,
            opacity: None,
        };
        if *flags & FADING_IN != 0 {
            let remaining = (f64::from(self.fade_remaining) - f64::from(delta)) as f32;
            if !remaining.is_finite() {
                return Err("tracer owner fade overflow");
            }
            if remaining <= 0. {
                result.opacity = Some(1.);
                *flags &= !FADING_IN;
            } else {
                result.opacity =
                    Some((1. - f64::from(remaining) / f64::from(self.fade_duration)) as f32);
            }
            self.fade_remaining = remaining;
        }
        self.sampling_remaining = sampling;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_delta_preserves_prepared_owner_and_flags() {
        let mut owner = Owner::new(0., 1., 1.).unwrap();
        let mut flags = ACTIVE | FADING_IN;
        owner.post_motion(&mut flags, 0.25).unwrap();
        let before = format!("{owner:?}");
        let saved_flags = flags;
        assert!(owner.post_motion(&mut flags, f32::NAN).is_err());
        assert!(owner.post_motion(&mut flags, -0.5).is_err());
        assert_eq!(format!("{owner:?}"), before);
        assert_eq!(flags, saved_flags);
    }
}
