//! Table-quantized direction from the independent native yaw/pitch inputs.
//! This does not produce either angle or infer camera/control updates.
use crate::arithmetic::Precision;

pub fn direction(yaw: f32, pitch: f32, precision: Precision) -> Result<[f32; 4], &'static str> {
    if [yaw, pitch]
        .into_iter()
        .any(|a| !a.is_finite() || a.abs() > 1_000_000.)
    {
        return Err("facing direction outside finite measured angle profile");
    }
    let r = |v| precision.round(v);
    let trig = |angle: f32| {
        let index =
            (r(r(f64::from(angle) * f64::from(45.511112_f32)) + 0.5) as i32 & 0x3fff) as usize;
        let sample = |i| (i as f64 * f64::from(0.0003834952_f32)).sin() as f32;
        (sample(index), sample(index + 4096))
    };
    let (sy, cy) = trig(yaw);
    let (sp, cp) = trig(pitch);
    Ok([
        r(f64::from(cp) * f64::from(sy)) as f32,
        r(f64::from(cp) * f64::from(-cy)) as f32,
        sp,
        0.,
    ])
}
