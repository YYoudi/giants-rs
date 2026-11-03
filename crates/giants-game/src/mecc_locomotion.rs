//! Normal living Mecc locomotion with animation style zero (development pistol).
//! Weapon actions, damage reactions, death, swimming and procedural poses are separate.
use crate::arithmetic::Precision;
use giants_formats::{Error, global_data::GlobalData};

pub const AIR_CLIPS: [u8; 7] = [5, 6, 7, 39, 40, 169, 172];

#[derive(Clone, Copy, Debug)]
pub struct Parameters {
    pub forward_low: f32,
    pub forward_high: f32,
    pub side: f32,
    pub air_height: f32,
}
impl Parameters {
    pub fn from_globals(g: &GlobalData) -> Result<Self, Error> {
        Ok(Self {
            forward_low: g.scalar(0xc18)?,
            forward_high: g.scalar(0xc1c)?,
            side: g.scalar(0xc20)?,
            air_height: g.scalar(0xc24)?,
        })
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Input {
    pub model: [f32; 3],
    pub position_z: f32,
    pub floor: f32,
    pub center_offset: f32,
    pub scale: f32,
    pub supported: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct Request {
    pub clip: u8,
    pub flags: u32,
    pub speed_source: i32,
    pub reference_speed: f32,
}
impl Request {
    pub fn rate(self, model: [f32; 3], precision: Precision) -> Result<f64, &'static str> {
        crate::animation_loop::playback_rate(
            self.reference_speed,
            self.speed_source,
            model,
            precision,
        )
    }
}
pub fn select(i: Input, p: Parameters, precision: Precision) -> Result<Request, &'static str> {
    if !i
        .model
        .into_iter()
        .chain([
            i.position_z,
            i.floor,
            i.center_offset,
            i.scale,
            p.forward_low,
            p.forward_high,
            p.side,
            p.air_height,
        ])
        .all(f32::is_finite)
        || i.scale <= 0.
        || p.forward_low < 0.
        || p.forward_high < p.forward_low
        || p.side < 0.
    {
        return Err("locomotion outside normal finite profile");
    }
    let r = |v| precision.round(v);
    let gap = r(r(f64::from(i.position_z) - f64::from(i.floor))
        - r(f64::from(i.center_offset) * f64::from(i.scale)));
    let forward = i.model[1];
    let side = i.model[2];
    let (clip, source, reference) = if !i.supported && gap > f64::from(p.air_height) {
        let band = if forward < -p.forward_high {
            -2
        } else if forward < -p.forward_low {
            -1
        } else if forward <= p.forward_low {
            0
        } else if forward <= p.forward_high {
            1
        } else {
            2
        };
        let clip = match band {
            -2 => 40,
            -1 => 39,
            1 => 6,
            2 => 7,
            _ if side < -p.side => 172,
            _ if side > p.side => 169,
            _ => 5,
        };
        (clip, -1, 0.)
    } else if forward != 0. {
        (2, 1, r(30. * f64::from(i.scale)) as f32)
    } else if side != 0. {
        (50, 2, r(-40. * f64::from(i.scale)) as f32)
    } else {
        (1, -1, 0.)
    };
    Ok(Request {
        clip,
        flags: 0x21,
        speed_source: source,
        reference_speed: reference,
    })
}
