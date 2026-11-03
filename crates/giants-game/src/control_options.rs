//! Independent physical input names, per character. Empty overrides mean unbound.
use std::collections::BTreeMap;
pub const ACTION_COUNTS: [usize; 3] = [28, 37, 24];
#[derive(Clone, Debug, PartialEq)]
pub struct ControlOptions(pub [BTreeMap<u8, String>; 3], pub f32, pub bool);
impl Default for ControlOptions {
    fn default() -> Self {
        Self(Default::default(), 0.5, false)
    }
}
impl ControlOptions {
    pub fn valid(&self) -> bool {
        self.1.is_finite()
            && (0.0..=1.0).contains(&self.1)
            && self.0.iter().enumerate().all(|(unit, bindings)| {
                bindings.iter().all(|(&row, value)| {
                    usize::from(row) < ACTION_COUNTS[unit]
                        && value.len() <= 32
                        && value.bytes().all(|b| b.is_ascii_alphanumeric())
                })
            })
    }
    pub fn adjust_mouse(&mut self, direction: i8) {
        self.1 = (self.1 + f32::from(direction.signum()) * 0.1).clamp(0.0, 1.0);
    }
    pub fn position_mouse(&mut self, fraction: f32) {
        if fraction.is_finite() {
            self.1 = fraction.clamp(0.0, 1.0);
        }
    }
    pub fn bind(&mut self, unit: usize, row: usize, physical: &str) -> bool {
        if unit >= 3
            || row >= ACTION_COUNTS[unit]
            || physical.len() > 32
            || !physical.bytes().all(|b| b.is_ascii_alphanumeric())
        {
            return false;
        }
        self.0[unit].insert(row as u8, physical.into());
        true
    }
    pub fn reset(&mut self, unit: usize) {
        if let Some(bindings) = self.0.get_mut(unit) {
            bindings.clear();
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn character_bindings_are_independent_and_invalid_requests_do_not_mutate() {
        let mut options = ControlOptions::default();
        assert!(options.bind(0, 1, "ArrowRight"));
        assert!(options.0[1].is_empty());
        let before = options.clone();
        assert!(!options.bind(3, 0, "KeyF"));
        assert!(!options.bind(0, 28, "KeyF"));
        assert!(!options.bind(0, 1, "invalid/name"));
        assert_eq!(options, before);
        assert!(options.bind(0, 1, ""));
        assert_eq!(options.0[0].get(&1).unwrap(), "");
        assert!(options.valid());
        options.adjust_mouse(-1);
        assert!((options.1 - 0.4).abs() < 1e-6);
        options.position_mouse(2.0);
        assert_eq!(options.1, 1.0);
        options.position_mouse(f32::NAN);
        assert_eq!(options.1, 1.0);
        options.1 = 0.5;
        options.reset(0);
        assert_eq!(options, ControlOptions::default());
    }
}
