//! Height-relative control parameters before target construction.
//! Inputs are native object fields; this module does not infer their producers.
use crate::arithmetic::Precision;
#[derive(Clone, Copy, Debug)]
pub struct Input {
    pub position_z: f32,
    pub reference_height: f32,
    pub range: f32,
    pub scale: f32,
    pub attenuation: f32,
    pub gain: f32,
    pub speed: f32,
    pub slope_power: f32,
    pub flags: u32,
}
#[derive(Clone, Copy, Debug)]
pub struct Output {
    pub speeds: [f32; 2],
    pub gain: f32,
    pub slope_power: f32,
    pub override_contact: bool,
}
pub fn resolve(i: Input, precision: Precision) -> Result<Output, &'static str> {
    if ![
        i.position_z,
        i.reference_height,
        i.range,
        i.scale,
        i.attenuation,
        i.gain,
        i.speed,
        i.slope_power,
    ]
    .into_iter()
    .all(f32::is_finite)
        || i.scale <= 0.
    {
        return Err("control state outside finite positive-scale profile");
    }
    let r = |v| precision.round(v);
    let height = r(f64::from(i.position_z) - r(f64::from(i.reference_height) * f64::from(i.scale)));
    let denominator = r(f64::from(i.range) * f64::from(i.scale));
    let ratio = -r(height / denominator);
    let (mut gain, mut speed) = (i.gain, i.speed);
    if ratio >= 0. {
        let factor = r(1. - r(ratio.min(1.) * f64::from(i.attenuation)));
        gain = r(f64::from(gain) * factor) as f32;
        speed = r(f64::from(speed) * factor) as f32;
    }
    if i.flags & 0x08000000 != 0 {
        gain = r(f64::from(gain) * 0.5) as f32;
        speed = r(f64::from(speed) * 0.5) as f32;
    }
    if !gain.is_finite() || !speed.is_finite() {
        return Err("control state arithmetic overflow");
    }
    Ok(Output {
        speeds: [speed; 2],
        gain,
        slope_power: i.slope_power,
        override_contact: i.flags & 0x10 != 0,
    })
}
