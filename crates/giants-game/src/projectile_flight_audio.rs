//! Measured delayed owner-sound prefix of a source effect.
//! This emits a request only: creating/updating the attached actor and admitting
//! an audio sink remain the integration layer's responsibilities.
use crate::arithmetic::Precision;
use giants_formats::effect_program::Program;

pub struct Recipe {
    pub delay: f32,
    /// Retain every source operand; only operand1's class role is measured here.
    pub class_context: [u32; 3],
    pub sound: u32,
}
impl Recipe {
    pub fn prepare(program: &Program) -> Result<Self, String> {
        let ops: Vec<_> = program.records.iter().map(|r| r.opcode).collect();
        if ops != [0x0b, 0x24, 0x06, 0x21, 0x2c, 0x07, 0x02] {
            return Err("flight effect outside measured delayed owner-sound profile".into());
        }
        let word = |record: usize, at: usize| -> Result<u32, String> {
            let bytes = program.records[record]
                .bytes
                .get(at..at + 4)
                .ok_or("truncated flight operand")?;
            Ok(u32::from_le_bytes(bytes.try_into().unwrap()))
        };
        let delay = f32::from_bits(word(0, 1)?);
        let class_context = [word(1, 1)?, word(1, 5)?, word(1, 9)?];
        let sound = word(4, 1)?;
        if !delay.is_finite() || delay <= 0. || delay > 1. || sound >= 2000 {
            return Err("flight delay or sound outside measured bounded profile".into());
        }
        Ok(Self {
            delay,
            class_context,
            sound,
        })
    }
}
#[derive(Default)]
pub struct Driver {
    initialized: bool,
    requested: bool,
    pub timer: f32,
}
impl Driver {
    /// Native context wait-state indicator, not a claim of actor/audio readiness.
    pub fn waiting(&self) -> bool {
        self.initialized && !self.requested
    }
    /// Returns true once at the measured construction/sound-request boundary.
    /// It does not consume an audio RNG draw or invent a successful backend ID.
    pub fn update(
        &mut self,
        recipe: &Recipe,
        dt: f32,
        precision: Precision,
    ) -> Result<bool, String> {
        if !dt.is_finite() || !(0.0..=1.).contains(&dt) {
            return Err("flight delta outside finite bounded profile".into());
        }
        if self.requested {
            return Ok(false);
        }
        if !self.initialized {
            self.initialized = true;
            self.timer = recipe.delay;
            return Ok(false);
        }
        let remaining = precision.round(f64::from(self.timer) - f64::from(dt));
        self.timer = remaining as f32;
        if remaining > 0. {
            return Ok(false);
        }
        self.requested = true;
        Ok(true)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn program(delay: f32) -> Program {
        let mut bytes = vec![0x0b];
        bytes.extend(delay.to_le_bytes());
        bytes.push(0x24);
        for word in [7u32, 91, 0] {
            bytes.extend(word.to_le_bytes());
        }
        bytes.extend([6, 0x21, 0x2c]);
        bytes.extend(123u32.to_le_bytes());
        bytes.extend([7, 2]);
        giants_formats::effect_program::parse(&bytes).unwrap()
    }
    #[test]
    fn unsupported_control_flow_and_nonfinite_delay_never_become_audio_recipes() {
        assert!(Recipe::prepare(&program(f32::NAN)).is_err());
        assert!(Recipe::prepare(&program(0.)).is_err());
        let mut p = program(0.04);
        p.records[3].opcode = 0x55;
        assert!(Recipe::prepare(&p).is_err());
        assert!(Recipe::prepare(&program(0.04)).is_ok());
    }
}
