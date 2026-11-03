//! Native menu gain range/default/step. Mixer routing is a separate integration.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AudioOptions {
    pub effects: f32,
    pub music: f32,
}
impl Default for AudioOptions {
    fn default() -> Self {
        Self {
            effects: 0.75,
            music: 0.75,
        }
    }
}
impl AudioOptions {
    pub fn valid(self) -> bool {
        [self.effects, self.music]
            .into_iter()
            .all(|gain| gain.is_finite() && (0.0..=1.0).contains(&gain))
    }
    pub fn adjust(&mut self, music: bool, direction: i8) {
        let gain = if music {
            &mut self.music
        } else {
            &mut self.effects
        };
        *gain = (*gain + f32::from(direction.signum()) * 0.1).clamp(0.0, 1.0);
    }
    pub fn position(&mut self, music: bool, fraction: f32) {
        if !fraction.is_finite() {
            return;
        }
        let gain = if music {
            &mut self.music
        } else {
            &mut self.effects
        };
        *gain = fraction.clamp(0.0, 1.0);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn slider_endpoints_steps_and_channels() {
        let mut gains = AudioOptions::default();
        gains.adjust(false, -1);
        assert!((gains.effects - 0.65).abs() < 1e-6);
        assert_eq!(gains.music, 0.75);
        for _ in 0..20 {
            gains.adjust(false, -1);
            gains.adjust(true, 1);
        }
        assert_eq!(gains.effects, 0.0);
        assert_eq!(gains.music, 1.0);
        gains.position(true, 0.317);
        assert_eq!(gains.music, 0.317);
        gains.position(true, f32::NAN);
        assert_eq!(gains.music, 0.317);
        gains.position(false, -2.0);
        assert_eq!(gains.effects, 0.0);
        assert!(gains.valid());
    }
}
