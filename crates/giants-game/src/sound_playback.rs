//! Base gain/frequency of an explicitly supplied debug audio RNG context.
//! This does not implement spatial attenuation, instance policy or device mixing.
pub struct Parameters {
    pub gain: f32,
    pub frequency: u32,
}
fn random(seed: &mut u32, low: f32, high: f32) -> f64 {
    *seed = seed.wrapping_mul(214013).wrapping_add(2531011);
    let sample = (*seed >> 16) & 32767;
    let width = (f64::from(high) - f64::from(low)) as f32;
    f64::from(sample) * f64::from(width) * f64::from(f32::from_bits(0x38000000)) + f64::from(low)
}
pub fn one_shot(words: [u32; 6], rate: u32, seed: &mut u32) -> Result<Parameters, &'static str> {
    let pitch = f32::from_bits(words[1]);
    let percent = f32::from_bits(words[2]);
    let variation = f32::from_bits(words[3]);
    if words[0] != 0 {
        return Err("loop_or_instance_policy_pending");
    }
    if !pitch.is_finite()
        || !percent.is_finite()
        || !variation.is_finite()
        || !(0. ..=100.).contains(&percent)
        || pitch > 400.
        || variation > 200.
        || rate == 0
        || rate > 1_000_000
    {
        return Err("sound_parameters_outside_checked_preview");
    }
    let mut gain = (f64::from(percent) * f64::from(f32::from_bits(0x3c23d70a))) as f32;
    if variation > 0. {
        let low = (1. - f64::from(variation) * f64::from(f32::from_bits(0x3ba3d70a))) as f32;
        gain = (random(seed, low, 1.) * f64::from(gain)) as f32;
    }
    let base = rate as f32;
    let frequency = if pitch > 0. {
        let high = ((f64::from(pitch) * f64::from(f32::from_bits(0x3b23d70a)) + 1.)
            * f64::from(base)) as f32;
        let low = ((1. - f64::from(pitch) * f64::from(f32::from_bits(0x3aa3d70a)))
            * f64::from(base)) as f32;
        random(seed, low, high).trunc() as u32
    } else {
        base as u32
    };
    if frequency == 0 {
        return Err("zero_playback_frequency");
    }
    Ok(Parameters { gain, frequency })
}

#[cfg(test)]
mod tests {
    #[test]
    fn refused_modes_do_not_consume_the_rng_and_zero_variance_does_not_draw() {
        let mut seed = 1;
        let mut words = [0, 0, 100f32.to_bits(), 0, 0, 0];
        let parameters = super::one_shot(words, 22050, &mut seed).unwrap();
        assert_eq!(
            (parameters.gain, parameters.frequency, seed),
            (1., 22050, 1)
        );
        words[0] = 1;
        assert!(super::one_shot(words, 22050, &mut seed).is_err());
        assert_eq!(seed, 1);
        words[0] = 0;
        words[1] = 25f32.to_bits();
        super::one_shot(words, 22050, &mut seed).unwrap();
        assert_eq!(seed, 2745024);
    }
}
