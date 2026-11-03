//! Menu preferences only; renderer capability/application is a separate interface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GraphicsOptions(pub [u8; 10]);
impl Default for GraphicsOptions {
    fn default() -> Self {
        // S07 defaults on the observed hardware. Lower renderer capabilities remain open.
        Self([2, 1, 1, 2, 2, 1, 1, 1, 1, 0])
    }
}
impl GraphicsOptions {
    pub fn valid(self) -> bool {
        self.0
            .iter()
            .enumerate()
            .all(|(row, &value)| value <= Self::maximum(row))
    }
    fn maximum(row: usize) -> u8 {
        match row {
            0 | 3 | 4 => 2,
            9 => 13,
            _ => 1,
        }
    }
    pub fn cycle(&mut self, row: usize, direction: i8) {
        if row >= 9 || direction == 0 {
            return;
        }
        let modulus = i16::from(Self::maximum(row)) + 1;
        self.0[row] =
            (i16::from(self.0[row]) + i16::from(direction.signum())).rem_euclid(modulus) as u8;
    }
    pub fn adjust_distance(&mut self, direction: i8) {
        // Native normalized +/-0.1 rounds to one of thirteen inverse distance steps.
        self.0[9] = (i16::from(self.0[9]) - i16::from(direction.signum())).clamp(0, 13) as u8;
    }
    pub fn position_distance(&mut self, fraction: f32) {
        if fraction.is_finite() {
            self.0[9] = ((1.0 - fraction.clamp(0.0, 1.0)) * 13.0 + 0.5).floor() as u8;
        }
    }
    pub fn distance_fraction(self) -> f32 {
        1.0 - f32::from(self.0[9]) / 13.0
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn observed_cycles_and_quantized_distance() {
        let mut options = GraphicsOptions::default();
        options.cycle(0, 1);
        assert_eq!(options.0[0], 0);
        options.cycle(3, 1);
        assert_eq!(options.0[3], 0);
        options.cycle(3, -1);
        assert_eq!(options.0[3], 2);
        options.cycle(1, 1);
        assert_eq!(options.0[1], 0);
        options.adjust_distance(-1);
        assert_eq!(options.0[9], 1);
        options.position_distance(0.5);
        assert_eq!(options.0[9], 7);
        options.position_distance(f32::NAN);
        assert_eq!(options.0[9], 7);
        for _ in 0..30 {
            options.adjust_distance(1);
        }
        assert_eq!(options.distance_fraction(), 1.0);
        assert!(options.valid());
        options.0[5] = 2;
        assert!(!options.valid());
    }
}
