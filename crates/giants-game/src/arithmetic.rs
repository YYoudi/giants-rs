//! Explicit x87 significand profiles for measured scalar components.
/// Explicit arithmetic policy. This does not infer a live thread control word.
/// Single24 retains a wide exponent between stores, as x87 precision control does.
#[derive(Clone, Copy, Debug)]
pub enum Precision {
    Single24,
    Double53,
}
impl Precision {
    pub(crate) fn round(self, value: f64) -> f64 {
        match self {
            Self::Double53 => value,
            Self::Single24 => {
                // Round the significand to 24 bits without narrowing the exponent.
                // All finite f32 operand products/ratios here fit normal f64 range.
                let bits = value.to_bits();
                let mask = (1u64 << 29) - 1;
                let base = bits & !mask;
                let tail = bits & mask;
                let half = 1u64 << 28;
                let up = tail > half || (tail == half && base & (1u64 << 29) != 0);
                f64::from_bits(base + if up { 1u64 << 29 } else { 0 })
            }
        }
    }
}
