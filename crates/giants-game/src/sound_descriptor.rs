//! Measured debug audio preparation, before device/sample creation.
//! Six catalog words stay distinct from a live handle or playback success.

/// Apply the debug audio component's distance defaults to catalog words.
/// Only zero (including negative zero) selects a default. Negative nonzero
/// distances remain unchanged; this is not a clamp or an attenuation model.
/// The finite-input profile uses the measured Double53 product and f32 store.
pub fn prepare(mut words: [u32; 6]) -> Result<[u32; 6], &'static str> {
    let mut minimum = f32::from_bits(words[4]);
    let mut maximum = f32::from_bits(words[5]);
    if !minimum.is_finite() || !maximum.is_finite() {
        return Err("nonfinite sound distance outside checked preparation profile");
    }
    if minimum == 0.0 {
        minimum = 10.0;
    }
    if maximum == 0.0 {
        maximum = (f64::from(minimum) * 15.0) as f32;
    }
    words[4] = minimum.to_bits();
    words[5] = maximum.to_bits();
    Ok(words)
}

#[cfg(test)]
mod tests {
    use super::prepare;

    #[test]
    fn zero_defaults_preserve_negative_distances_and_opaque_fields() {
        let raw = [0x12345678, 99, 100, 101, (-0.0f32).to_bits(), 0];
        let prepared = prepare(raw).unwrap();
        assert_eq!(&prepared[..4], &raw[..4]);
        assert_eq!(prepared[4], 10f32.to_bits());
        assert_eq!(prepared[5], 150f32.to_bits());
        let negative = [1, 2, 3, 4, (-2f32).to_bits(), (-3f32).to_bits()];
        assert_eq!(prepare(negative).unwrap(), negative);
        let mut inherited = negative;
        inherited[5] = 0;
        assert_eq!(prepare(inherited).unwrap()[5], (-30f32).to_bits());
        for nonfinite in [f32::INFINITY, f32::NEG_INFINITY, f32::NAN] {
            let mut unsupported = raw;
            unsupported[4] = nonfinite.to_bits();
            assert!(prepare(unsupported).is_err());
            unsupported[4] = 0;
            unsupported[5] = nonfinite.to_bits();
            assert!(prepare(unsupported).is_err());
        }
    }
}
