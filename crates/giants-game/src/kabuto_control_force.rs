//! Acquired post-gravity control-target/force phase. The incoming speed latch
//! and later primary consumers remain separate; no frame completion is implied.
use crate::{actor_affine::Cache, actor_fields::Fields, arithmetic::Precision};

#[derive(Clone, Copy, Debug)]
pub struct View {
    pub speed: f32,
    pub desired_velocity: Option<[f32; 3]>,
}
pub(crate) struct Prepared {
    pub payload: Vec<u8>,
    pub view: View,
}
pub(crate) fn prepare(
    fields: &Fields,
    payload: &[u8],
    state: crate::kabuto_motion_prefix::State,
    contact: crate::kabuto_classification::Contact,
    affine: Cache,
    globals: &giants_formats::global_data::GlobalData,
) -> Result<Prepared, String> {
    if fields.class != 1 || payload.len() != 844 {
        return Err("Kabuto control force requires its acquired state payload".into());
    }
    let word = |at: usize| u32::from_le_bytes(payload[at..at + 4].try_into().unwrap());
    let scalar = |at| f32::from_bits(word(at));
    let gain = globals.scalar(0x5fc).map_err(|e| e.to_string())?;
    let slope = globals.scalar(0x600).map_err(|e| e.to_string())?;
    let mut speed = scalar(0x160);
    let offset = f32::from_bits(fields.configuration[17]);
    let span = f32::from_bits(fields.configuration[22]);
    if ![fields.position[2], fields.scale, speed, offset, span]
        .into_iter()
        .all(f32::is_finite)
    {
        return Err("Kabuto speed attenuation outside finite input profile".into());
    }
    // Keep the original wide ratio until the speed store. A zero span follows
    // the acquired ordered comparisons: NaN does not enter the positive arm.
    let ratio = -((f64::from(fields.position[2]) - f64::from(offset) * f64::from(fields.scale))
        / (f64::from(span) * f64::from(fields.scale)));
    if ratio > 0. {
        speed = ((1. - ratio.min(1.) * 0.5) * f64::from(speed)) as f32;
    }
    if fields.object_flags & 0x8000000 != 0 {
        speed = (f64::from(speed) * f64::from(0.8f32)) as f32;
    }
    if !speed.is_finite() {
        return Err("Kabuto speed attenuation overflow".into());
    }
    let desired_velocity = crate::mecc_control_target::apply(
        crate::mecc_control_target::Input {
            matrix: affine.matrix,
            scale: fields.scale,
            controls: [state.commands[2] as i8, state.commands[3] as i8],
            speeds: [speed; 2],
            normal: contact.plane[..3].try_into().unwrap(),
            physics_flags: word(0xe4),
            eligible: contact.support != 0,
            override_contact: fields.object_flags & 0x10 != 0,
        },
        Precision::Double53,
    )?;
    let mut body = payload.to_vec();
    if let Some(desired_velocity) = desired_velocity {
        let out = crate::mecc_control_force::apply(
            crate::mecc_control_force::Input {
                desired_velocity,
                velocity: state.velocity,
                force: [scalar(0xfc), scalar(0x100), scalar(0x104)],
                gain,
                slope_power: slope,
                mass: scalar(0xec),
                force_time_limit: scalar(0x110),
            },
            Precision::Double53,
        )?;
        for (at, v) in [0xfc, 0x100, 0x104, 0x110]
            .into_iter()
            .zip(out.force.into_iter().chain([out.force_time_limit]))
        {
            body[at..at + 4].copy_from_slice(&v.to_bits().to_le_bytes());
        }
    }
    Ok(Prepared {
        payload: body,
        view: View {
            speed,
            desired_velocity,
        },
    })
}
