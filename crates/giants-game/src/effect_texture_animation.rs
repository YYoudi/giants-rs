//! Numeric texture-frame clock from the debug effect callback.
//! Admission, numbered resource loading and material binding belong upstream.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Configuration {
    pub frames: u32,
    pub rate: f32,
}
impl Configuration {
    /// Numbered texture stems from the first material's texture name.
    /// A missing digit is outside the checked native constructor profile.
    pub fn numbered_names(self, base: &str) -> Result<Vec<String>, &'static str> {
        self.validate()?;
        if self.frames > 4096 || base.len() > 127 || !base.is_ascii() || base.contains('\0') {
            return Err("texture animation naming outside bounded profile");
        }
        let end = base
            .find(|c: char| c.is_ascii_digit())
            .ok_or("texture animation base has no numeric suffix")?;
        let prefix = &base[..end];
        Ok((0..self.frames)
            .map(|i| format!("{prefix}{i:02}"))
            .collect())
    }
    pub fn validate(self) -> Result<Self, &'static str> {
        if self.frames == 0
            || self.frames > i32::MAX as u32
            || !self.rate.is_finite()
            || self.rate <= 0.
        {
            return Err("texture animation outside positive finite profile");
        }
        Ok(self)
    }
}

#[derive(Clone, Debug)]
pub struct State {
    configuration: Configuration,
    frame: u32,
    elapsed: f32,
}
impl State {
    pub fn new(configuration: Configuration) -> Result<Self, &'static str> {
        Ok(Self {
            configuration: configuration.validate()?,
            frame: 0,
            elapsed: 0.,
        })
    }
    pub fn frame(&self) -> u32 {
        self.frame
    }
    pub fn elapsed(&self) -> f32 {
        self.elapsed
    }
    /// One admitted visual callback. The native x87 clock retains the sum at
    /// double precision while storing its fractional residual as a float.
    pub fn advance(&mut self, delta: f32) -> Result<(), &'static str> {
        if !delta.is_finite() || delta < 0. {
            return Err("invalid texture animation delta");
        }
        let product =
            (f64::from(delta) + f64::from(self.elapsed)) * f64::from(self.configuration.rate);
        let steps = product.trunc();
        if !product.is_finite() || steps > f64::from(i32::MAX - self.frame as i32) {
            return Err("texture animation signed frame overflow");
        }
        self.elapsed = ((product - steps) / f64::from(self.configuration.rate)) as f32;
        self.frame = (self.frame + steps as u32) % self.configuration.frames;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejected_step_preserves_clock() {
        let mut state = State::new(Configuration {
            frames: 4,
            rate: 2.,
        })
        .unwrap();
        state.advance(0.25).unwrap();
        for delta in [f32::NAN, f32::INFINITY, -1., f32::MAX] {
            assert!(state.advance(delta).is_err());
            assert_eq!((state.frame(), state.elapsed()), (0, 0.25));
        }
    }
}
