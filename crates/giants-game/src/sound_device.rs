//! Numeric debug software-audio conversion before DirectSound calls.
//! A returned native integer does not prove driver acceptance or audible mixing.
pub fn mix_gain(
    base: f32,
    attenuation: f32,
    global: f32,
    instance: f32,
) -> Result<f32, &'static str> {
    if [base, attenuation, global, instance]
        .iter()
        .any(|v| !v.is_finite() || *v < 0.)
    {
        return Err("invalid sound gain factor");
    }
    let global_attenuation = (f64::from(attenuation) * f64::from(global)) as f32;
    let source = (f64::from(global_attenuation) * f64::from(base)) as f32;
    let gain = (f64::from(source) * f64::from(instance)) as f32;
    if !gain.is_finite() {
        return Err("sound gain overflow");
    }
    Ok(gain)
}
pub fn volume_millibels(gain: f32) -> Result<i32, &'static str> {
    if !gain.is_finite() || gain < 0. {
        return Err("invalid sound device gain");
    }
    Ok(if gain >= 1. {
        0
    } else if gain == 0. {
        -10000
    } else {
        (-2000. * (1. / f64::from(gain)).log10()).trunc() as i32
    })
}
pub fn pan_units(pan: f32) -> Result<i32, &'static str> {
    if !pan.is_finite() || !(-1. ..=1.).contains(&pan) {
        return Err("sound pan outside checked unit profile");
    }
    Ok((f64::from(pan) * 6000.).trunc() as i32)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn silence_and_sub_floor_gains_are_distinct_native_outputs() {
        assert_eq!(volume_millibels(0.), Ok(-10000));
        assert!(volume_millibels(1e-8).unwrap() < -10000);
        assert_eq!(volume_millibels(2.), Ok(0));
        assert_eq!(pan_units(-1.), Ok(-6000));
        assert_eq!(pan_units(1.), Ok(6000));
        assert!(pan_units(1.1).is_err());
        assert!(mix_gain(f32::MAX, 1., 2., 1.).is_err());
        assert!(volume_millibels(f32::NAN).is_err());
    }
}
