//! Scalar policies measured on the two identified sound backends. A seed is
//! supplied explicitly; this does not assert the live device thread's RNG phase.
use crate::arithmetic::Precision;

#[derive(Clone, Copy, Debug)]
pub struct Random(pub u32);
impl Random {
    pub fn next_15(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(214013).wrapping_add(2531011);
        (self.0 >> 16) & 32767
    }
    fn interval(&mut self, low: f32, high: f32, precision: Precision) -> f64 {
        let r = |v| precision.round(v);
        let range = r(f64::from(high) - f64::from(low)) as f32;
        let scaled = r(r(f64::from(self.next_15()) * f64::from(range)) / 32768.);
        r(scaled + f64::from(low))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Parameters {
    pub flags: u32,
    pub gain: f32,
    pub frequency: u32,
    pub minimum_distance: f32,
    pub maximum_distance: f32,
}

pub fn prepare(
    words: [u32; 6],
    sample_rate: u32,
    random: &mut Random,
    precision: Precision,
) -> Result<Parameters, String> {
    let [
        _,
        frequency_variation,
        volume_percent,
        volume_variation,
        minimum,
        maximum,
    ] = words.map(f32::from_bits);
    if ![
        frequency_variation,
        volume_percent,
        volume_variation,
        minimum,
        maximum,
    ]
    .into_iter()
    .all(f32::is_finite)
        || !(8000..=192000).contains(&sample_rate)
        || !(0.0..=100.).contains(&volume_percent)
        || volume_variation > 200.
        || frequency_variation > 800.
    {
        return Err("sound parameters outside bounded scalar profile".into());
    }
    let r = |v| precision.round(v);
    let minimum = if minimum <= 0. { 10. } else { minimum };
    let maximum = if maximum <= 0. {
        r(f64::from(minimum) * 15.) as f32
    } else {
        maximum
    };
    let mut gain = r(f64::from(volume_percent) * f64::from(0.01f32)) as f32;
    if volume_variation > 0. {
        let low = r(1. - r(f64::from(volume_variation) * f64::from(0.005f32))) as f32;
        gain = r(random.interval(low, 1., precision) * f64::from(gain)) as f32;
    }
    let mut frequency = sample_rate as f32;
    if frequency_variation > 0. {
        let high = r(
            r(1. + r(f64::from(frequency_variation) * f64::from(0.0025f32))) * f64::from(frequency),
        ) as f32;
        let low = r(
            r(1. - r(f64::from(frequency_variation) * f64::from(0.00125f32)))
                * f64::from(frequency),
        ) as f32;
        frequency = random.interval(low, high, precision) as f32;
    }
    Ok(Parameters {
        flags: words[0],
        gain,
        frequency: frequency as u32,
        minimum_distance: minimum,
        maximum_distance: maximum,
    })
}

/// Distance is supplied by the live spatial consumer. This function alone does
/// not establish position selection, pan, occlusion or Doppler behavior.
pub fn attenuation(
    distance: f32,
    minimum: f32,
    maximum: f32,
    precision: Precision,
) -> Result<f32, String> {
    if ![distance, minimum, maximum]
        .into_iter()
        .all(|v| v.is_finite() && v >= 0.)
    {
        return Err("sound distance outside finite nonnegative profile".into());
    }
    Ok(if distance <= minimum {
        1.
    } else if distance >= maximum {
        0.
    } else {
        precision.round(f64::from(minimum) / f64::from(distance)) as f32
    })
}

pub fn instance_gain(
    global: f32,
    sample: f32,
    spatial: f32,
    fraction: f32,
    precision: Precision,
) -> Result<f32, String> {
    if ![global, sample, spatial, fraction]
        .into_iter()
        .all(|v| v.is_finite() && (0.0..=1.).contains(&v))
    {
        return Err("sound gain outside measured linear profile".into());
    }
    let first = precision.round(f64::from(spatial) * f64::from(global)) as f32;
    let second = precision.round(f64::from(first) * f64::from(sample)) as f32;
    Ok(precision.round(f64::from(second) * f64::from(fraction)) as f32)
}
