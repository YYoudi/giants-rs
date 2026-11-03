//! Screen veil after lens intensity evolution; not a camera visibility solver.
use giants_formats::{Error, world::SunGlare};

#[derive(Clone, Copy, Debug)]
pub struct Ray {
    pub direction: [f32; 3],
    pub start: [f32; 3],
    pub end: [f32; 3],
    pub dot: f32,
}

/// Native lens normalization keeps its reciprocal length in extended precision.
/// This differs from the animation LOOKAT normalization's float32 stores.
pub fn normalize(vector: [f32; 3]) -> [f32; 3] {
    let [x, y, z] = vector.map(f64::from);
    let sum = (x * x + z * z) + y * y;
    if sum < f64::from(0.0001_f32) {
        return [0.; 3];
    }
    let inverse = 1. / sum.sqrt();
    vector.map(|v| (f64::from(v) * inverse) as f32)
}

pub fn ray(sun: [f32; 3], eye: [f32; 3], axis: [f32; 3]) -> Result<Ray, Error> {
    if sun
        .into_iter()
        .chain(eye)
        .chain(axis)
        .any(|v| !v.is_finite())
    {
        return Err(Error("nonfinite sun ray input"));
    }
    let direction = normalize(std::array::from_fn(|i| sun[i] - eye[i]));
    let start = std::array::from_fn(|i| (f64::from(direction[i]) * 10. + f64::from(eye[i])) as f32);
    let end = std::array::from_fn(|i| (f64::from(sun[i]) - f64::from(direction[i]) * 1000.) as f32);
    let dot = (f64::from(direction[2]) * f64::from(axis[2])
        + f64::from(direction[1]) * f64::from(axis[1])
        + f64::from(direction[0]) * f64::from(axis[0])) as f32;
    Ok(Ray {
        direction,
        start,
        end,
        dot,
    })
}

/// Persistent first lens channel. Visibility is supplied by scene geometry.
/// The measured world initializes both response rates to two.
pub fn advance(
    intensity: f32,
    dt: f32,
    dot: f32,
    clear: bool,
    exponent: f32,
    response: f32,
) -> Result<(f32, f32), Error> {
    if [intensity, dt, exponent, response]
        .iter()
        .any(|v| !v.is_finite() || *v < 0.)
        || !dot.is_finite()
        || exponent == 0.
    {
        return Err(Error("unsupported lens evolution input"));
    }
    let target = if clear {
        f64::from(dot.max(0.)).powf(f64::from(exponent))
    } else {
        0.
    };
    let rate = (target - f64::from(intensity)) * f64::from(response);
    let value = rate * f64::from(dt) + f64::from(intensity);
    let value = if rate >= 0. {
        value.min(target)
    } else {
        value.max(target)
    };
    if !value.is_finite() || !(rate as f32).is_finite() {
        return Err(Error("lens evolution overflow"));
    }
    Ok((value as f32, rate as f32))
}

/// One native update when the target lens intensity is zero (hidden sun).
/// Returns the stored intensity and rate. The unrounded rate advances the state.
/// Selecting visibility and computing a nonzero target remain separate work.
pub fn decay(intensity: f32, dt: f32, fall_rate: f32) -> Result<(f32, f32), Error> {
    if [intensity, dt, fall_rate]
        .iter()
        .any(|v| !v.is_finite() || *v < 0.)
    {
        return Err(Error("unsupported sun glare decay input"));
    }
    let rate = -f64::from(intensity) * f64::from(fall_rate);
    let value = (rate * f64::from(dt) + f64::from(intensity)).max(0.);
    if !(rate as f32).is_finite() {
        return Err(Error("sun glare decay overflow"));
    }
    Ok((value as f32, rate as f32))
}

/// Native byte-space additive color, with integer intensity before RGB tint.
/// `intensity` is persistent lens state, not the instantaneous camera/sun dot.
/// The measured world initializes the separate base intensity to zero.
/// Higher intensities/colors outside this bounded byte profile remain unsupported.
pub fn screen_rgb(
    parameters: &SunGlare,
    intensity: f32,
    base: f32,
) -> Result<Option<[u8; 3]>, Error> {
    if !intensity.is_finite()
        || intensity < 0.
        || !base.is_finite()
        || base < 0.
        || !parameters.scale.is_finite()
        || parameters.scale < 0.
        || parameters
            .color
            .iter()
            .any(|v| !v.is_finite() || !(0. ..=1.).contains(v))
    {
        return Err(Error("unsupported sun glare input"));
    }
    // Keep extended intermediates through both native truncations.
    let amount = f64::from(intensity) * f64::from(parameters.scale) + f64::from(base);
    if amount <= f64::from(0.003_921_569_f32) {
        return Ok(None);
    }
    let level = (amount * 255.).trunc();
    if level > 255. {
        return Err(Error("sun glare byte intensity outside measured profile"));
    }
    Ok(Some(parameters.color.map(|v| (level * f64::from(v)) as u8)))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normalized_lens_ray_keeps_extended_reciprocal() {
        let ray = ray([10., 20., 30.], [1., 2., 3.], [0., 0., 1.]).unwrap();
        assert_eq!(normalize([0.001, 0., 0.]), [0.; 3]);
        let length = (9f64 * 9. + 18. * 18. + 27. * 27.).sqrt();
        assert_eq!(
            ray.direction,
            [
                (9. / length) as f32,
                (18. / length) as f32,
                (27. / length) as f32
            ]
        );
        assert_eq!(ray.dot, ray.direction[2]);
        assert!(super::ray([f32::NAN, 0., 0.], [0.; 3], [0.; 3]).is_err());
    }
    #[test]
    fn positive_target_accumulates_and_survives_hidden_sun() {
        let (bright, rate) = advance(0., 0.1, 0.5, true, 6., 2.).unwrap();
        assert!(bright > 0. && rate > 0.);
        assert_eq!(
            advance(bright, 1. / 60., -0.5, true, 6., 2.).unwrap(),
            decay(bright, 1. / 60., 2.).unwrap()
        );
        assert_eq!(advance(0., 10., 0.5, true, 6., 2.).unwrap().0, 0.015625);
        assert!(advance(0., 0.1, 0., false, 0., 2.).is_err());
    }
    #[test]
    fn hidden_sun_keeps_history_and_clamps_overshoot() {
        let (remaining, rate) = decay(0.5, 1. / 60., 2.).unwrap();
        assert!(remaining > 0. && remaining < 0.5);
        assert_eq!(rate, -1.);
        assert_eq!(decay(0.5, 1., 2.).unwrap(), (0., -1.));
        assert!(decay(0.5, -1., 2.).is_err());
    }
    #[test]
    fn threshold_and_integer_before_tint_are_preserved() {
        let parameters = SunGlare {
            effect: "synthetic".into(),
            color: [1., 0.7, 0.1],
            exponent: 6.,
            scale: 1.,
        };
        assert_eq!(screen_rgb(&parameters, 0., 0.).unwrap(), None);
        assert_eq!(screen_rgb(&parameters, 1. / 255., 0.).unwrap(), None);
        // Directly tinting the untruncated 15.9 would incorrectly give G=11.
        assert_eq!(
            screen_rgb(&parameters, 15.9 / 255., 0.).unwrap(),
            Some([15, 10, 1])
        );
        assert!(screen_rgb(&parameters, f32::NAN, 0.).is_err());
        assert!(screen_rgb(&parameters, 2., 0.).is_err());
    }
}
