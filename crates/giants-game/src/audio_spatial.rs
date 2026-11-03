//! Software spatial-update scalars with an explicitly supplied listener matrix.
//! The live matrix producer and device's pan response are separate boundaries.
use crate::{arithmetic::Precision, audio_parameters};

pub struct Spatial {
    pub distance: f32,
    pub pan: f32,
    pub gain: f32,
}
pub struct Inputs {
    pub matrix: [f32; 16],
    pub position: [f32; 3],
    pub flags: u32,
    pub previous_pan: f32,
    pub previous_gain: f32,
    pub minimum: f32,
    pub maximum: f32,
}
/// Integer passed to the original software pan adapter. Device response is not
/// established by this conversion. The measured multiplier is 6000.
pub fn pan_units(pan: f32, precision: Precision) -> Result<i32, String> {
    if !pan.is_finite() || pan.abs() > 1. {
        return Err("pan outside finite unit interval".into());
    }
    Ok(precision.round(f64::from(pan) * 6000.) as i32)
}
pub fn update(input: &Inputs, precision: Precision) -> Result<Spatial, String> {
    if input.flags & !0x1000e != 0
        || input.flags & 0x10000 == 0
        || input.flags & 6 == 0
        || !input
            .matrix
            .into_iter()
            .chain(input.position)
            .chain([
                input.previous_pan,
                input.previous_gain,
                input.minimum,
                input.maximum,
            ])
            .all(f32::is_finite)
    {
        return Err("spatial update outside finite fixed-position software profile".into());
    }
    let r = |v| precision.round(v);
    let m = input.matrix.map(f64::from);
    let p = input.position.map(f64::from);
    let mut transformed = [0f32; 3];
    for j in 0..3 {
        let first = r(r(m[j] * p[0]) + r(m[4 + j] * p[1]));
        transformed[j] = r(r(first + r(m[8 + j] * p[2])) + m[12 + j]) as f32;
    }
    let t = transformed.map(f64::from);
    let squared = r(r(r(t[0] * t[0]) + r(t[1] * t[1])) + r(t[2] * t[2])) as f32;
    let distance = if squared > 0. {
        r(f64::from(squared).sqrt()) as f32
    } else {
        1.
    };
    if !distance.is_finite() || !transformed.into_iter().all(f32::is_finite) {
        return Err("spatial transform overflow".into());
    }
    let pan = if input.flags & 4 != 0 {
        let inverse = r(1. / f64::from(distance)) as f32;
        let x = r(t[0] * f64::from(inverse)) as f32;
        let z = r(t[2] * f64::from(inverse)) as f32;
        r(r(f64::from(x) * m[9]) - r(f64::from(z) * m[8])) as f32
    } else {
        input.previous_pan
    };
    let gain = if input.flags & 2 != 0 {
        audio_parameters::attenuation(distance, input.minimum, input.maximum, precision)?
    } else {
        input.previous_gain
    };
    if !pan.is_finite() || !gain.is_finite() {
        return Err("spatial output overflow".into());
    }
    Ok(Spatial {
        distance,
        pan,
        gain,
    })
}
