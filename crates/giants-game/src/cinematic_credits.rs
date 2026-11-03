//! Credits command projection before localization, font drawing and scrolling.
//! Text identities are opaque tokens, never addresses to dereference in Rust.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Registers(pub [u32; 8]);
impl Registers {
    /// Slots: text identity, reverse, pause, hold, scroll, metric, speed, and
    /// director line counter. Localization must supply a nonzero identity.
    pub fn apply(
        &mut self,
        opcode: u32,
        operand: Option<u32>,
        resolved_text: Option<u32>,
    ) -> Result<(), &'static str> {
        match (opcode, operand, resolved_text) {
            (47, None, Some(text)) if text != 0 => {
                self.0[0] = text;
                for index in [1, 2, 3, 4, 7] {
                    self.0[index] = 0;
                }
            }
            (49, None, None) => {
                self.0[1] = u32::from(self.0[1] == 0);
                self.0[2] = 0;
            }
            (51, Some(word), None) => self.0[6] = word,
            (53, None, None) => self.0[3] = u32::from(self.0[3] == 0),
            _ => {
                return Err("credits command needs its exact operands and localized text identity");
            }
        }
        Ok(())
    }
}

/// A preview request is retained until a real text/scroll renderer consumes it.
/// Framing and a scalar projection do not acknowledge display completion.
#[derive(Clone, Debug)]
pub struct Request {
    pub command_offset: usize,
    pub opcode: u32,
    pub text_key: Option<String>,
    pub word: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reset_preserves_metric_and_speed_and_needs_actual_text_resolution() {
        let mut state = Registers([7, 8, 9, 10, 11, 12, 13, 14]);
        let old = state.clone();
        assert!(state.apply(47, None, None).is_err());
        assert!(state.apply(47, None, Some(0)).is_err());
        assert_eq!(state, old);
        state.apply(47, None, Some(99)).unwrap();
        assert_eq!(state.0, [99, 0, 0, 0, 0, 12, 13, 0]);
        state.apply(49, None, None).unwrap();
        assert_eq!(state.0[1], 1);
        state.apply(49, None, None).unwrap();
        assert_eq!(state.0[1], 0);
        state.apply(51, Some(8f32.to_bits()), None).unwrap();
        assert_eq!(state.0[6], 8f32.to_bits());
    }
}
