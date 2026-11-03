//! First positional sound of a bounded, straight-line source effect.
//! Visual activation after that first request and the live RNG phase are separate.
use crate::audio_parameters::Random;
use giants_formats::{Error, effect_program::Program};

/// The actual positional wrapper/backend boundary. A loaded bank may consume
/// further shared RNG while preparing device parameters. Errors cannot undo
/// requests or draws that the service already performed.
pub trait Service {
    fn request(
        &mut self,
        selector: u32,
        position: [f32; 3],
        random: &mut Random,
    ) -> Result<(), String>;
}
/// Explicit counterpart of the acquired native null bank, not missing wiring.
pub struct AbsentBank;
impl Service for AbsentBank {
    fn request(&mut self, _: u32, _: [f32; 3], _: &mut Random) -> Result<(), String> {
        Ok(())
    }
}

pub struct Recipe {
    pub range: [u32; 2],
    pub preparation_draws: usize,
}
impl Recipe {
    /// The sound must precede every visual operation. Other commands are a
    /// bounded preparation profile; control-flow traversal is refused.
    pub fn prepare(program: &Program) -> Result<Self, Error> {
        if program.records.first().map(|r| r.opcode) != Some(0x2b)
            || program.records.last().map(|r| r.opcode) != Some(2)
        {
            return Err(Error("effect outside first-positional-sound profile"));
        }
        let first = &program.records[0];
        let word = |offset: usize| -> Result<u32, Error> {
            let bytes = first
                .bytes
                .get(offset..offset + 4)
                .ok_or(Error("truncated positional sound range"))?;
            Ok(u32::from_le_bytes(bytes.try_into().unwrap()))
        };
        let range = [word(1)?, word(5)?];
        if range[0] > range[1] || range[1] >= 2000 || range[1] - range[0] > 32 {
            return Err(Error("positional sound range outside checked profile"));
        }
        let mut preparation_draws = 1;
        for record in program.records.iter().skip(1) {
            if matches!(record.opcode, 0x05 | 0x37) {
                let pairs = if record.opcode == 0x05 { 3 } else { 2 };
                for pair in 0..pairs {
                    let at = 1 + pair * 8;
                    let values = record
                        .bytes
                        .get(at..at + 8)
                        .ok_or(Error("truncated preparation random range"))?;
                    let low = f32::from_le_bytes(values[..4].try_into().unwrap());
                    let high = f32::from_le_bytes(values[4..].try_into().unwrap());
                    if !low.is_finite() || !high.is_finite() || low > high {
                        return Err(Error(
                            "preparation random range outside finite ordered profile",
                        ));
                    }
                }
            }
            preparation_draws += match record.opcode {
                0x37 => 2,
                0x05 => 3,
                0x02 | 0x03 | 0x04 | 0x06 | 0x07 | 0x08 | 0x10 | 0x1b | 0x21 | 0x46 => 0,
                _ => return Err(Error("unmeasured positional sound preparation command")),
            };
        }
        Ok(Self {
            range,
            preparation_draws,
        })
    }

    /// Advances the supplied CRT state through preparation, then activation's
    /// first sound command. This is not a model of the visual activation tail.
    pub fn first_sound(&self, random: &mut Random) -> u32 {
        for _ in 0..self.preparation_draws {
            random.next_15();
        }
        self.range[0] + ((random.next_15() * (self.range[1] - self.range[0] + 1)) >> 15)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unsupported_traversal_is_refused_and_preparation_draws_are_retained() {
        let mut data = vec![0x2b];
        data.extend(40u32.to_le_bytes());
        data.extend(41u32.to_le_bytes());
        data.push(0x37);
        data.extend([0; 16]);
        data.push(5);
        data.extend([0; 24]);
        data.push(2);
        let program = giants_formats::effect_program::parse(&data).unwrap();
        let recipe = Recipe::prepare(&program).unwrap();
        assert_eq!(recipe.preparation_draws, 6);
        data.insert(data.len() - 1, 0x55);
        assert!(Recipe::prepare(&giants_formats::effect_program::parse(&data).unwrap()).is_err());
    }
}
