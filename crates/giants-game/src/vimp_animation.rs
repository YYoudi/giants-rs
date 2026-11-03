//! Source ordinary mode1 and hurt mode2..4 animation selection.
//! Clocks, references and event effects are separate consumers of this choice.
use crate::{
    actor_fields::Fields,
    animation_channels::{Channels, Request},
};

pub use crate::animation_epoch::Stamp;
pub struct Services<'a> {
    pub blending: crate::animation_channels::Blending,
    pub contexts: &'a mut crate::attached_effect_context::Pool<crate::resident_states::Id>,
    pub random: &'a mut crate::audio_parameters::Random,
    pub stamp: &'a mut Stamp,
}

/// Hurt selector and its default/user wrappers. Descriptor absence and an
/// already selected slot are native no-ops; a failed mode4 choice still draws.
/// Applying the request selects numeric channels, without evaluating poses.
pub fn hurt_request(
    mode: u32,
    channels: &Channels,
    has_clip: impl Fn(u8) -> bool,
    random: &mut crate::audio_parameters::Random,
) -> Result<Option<Request>, String> {
    let (slot, flags) = match mode {
        4 => {
            if matches!(channels[0][0], 32 | 69 | 70) {
                return Ok(None);
            }
            let choice = (random.next_15() * 100) >> 15;
            (
                if choice < 33 {
                    32
                } else if choice < 66 {
                    69
                } else {
                    70
                },
                0x20,
            )
        }
        2 => (36, 0x21),
        3 => (34, 0x20),
        _ => return Err("Vimp hurt animation requires mode2..4".into()),
    };
    if channels[0][0] == u32::from(slot) || !has_clip(slot) {
        return Ok(None);
    }
    Ok(Some(Request {
        slot,
        flags,
        speed_source: 0,
        reference_speed: 0.,
        blend_seconds: f32::from_bits(0x3e2a_aaab),
    }))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn select(
    fields: &Fields,
    vimp: crate::vimp_actor::Cold,
    physical: &crate::vimp_physical::View,
    body: &[u8],
    channels: &Channels,
    specification: &crate::specification::Prepared,
    globals: &giants_formats::global_data::GlobalData,
    random: &mut crate::audio_parameters::Random,
) -> Result<Option<Request>, String> {
    if fields.class == 8 && fields.object_flags & 0x8000_0000 == 0 && matches!(vimp.mode, 2..=4) {
        return hurt_request(
            vimp.mode,
            channels,
            |slot| specification.animations.contains_key(&slot),
            random,
        );
    }
    if fields.class != 8
        || vimp.mode != 1
        || fields.object_flags & 0x8000_0000 != 0
        || crate::vimp_wander::word(body, 0) & 0x40 != 0
        || crate::vimp_wander::word(body, 0x74) != 0
    {
        return Err("Vimp animation requires its ordinary dry mode1 profile".into());
    }
    mode1_request(
        Mode1 {
            forward_rate: physical.model_rates[1],
            angular_speed: vimp.angular_speed,
            damage: vimp.damage,
            scale: fields.scale,
        },
        channels,
        |slot| specification.animations.contains_key(&slot),
        globals,
    )
}

#[derive(Clone, Copy)]
pub struct Mode1 {
    pub forward_rate: f32,
    pub angular_speed: f32,
    pub damage: f32,
    pub scale: f32,
}
/// Acquired dry gait0 requests, also used on the visit returning from mode3.
/// The primary and resource consumer own any requested descriptor failures.
pub fn mode1_request(
    state: Mode1,
    channels: &Channels,
    has_clip: impl Fn(u8) -> bool,
    globals: &giants_formats::global_data::GlobalData,
) -> Result<Option<Request>, String> {
    if ![
        state.forward_rate,
        state.angular_speed,
        state.damage,
        state.scale,
    ]
    .into_iter()
    .all(f32::is_finite)
    {
        return Err("Vimp mode1 ANM inputs are nonfinite".into());
    }
    if state.forward_rate == 0. {
        if state.angular_speed != 0. && channels[0][0] != 38 && has_clip(38) {
            return Ok(Some(Request {
                slot: 38,
                flags: 0x21,
                speed_source: 0,
                reference_speed: 0.,
                blend_seconds: f32::from_bits(0x3e2a_aaab),
            }));
        }
        return Ok(None);
    }
    let slot = if state.damage > 0. { 71 } else { 2 };
    if channels[0][0] == 64 && channels[0][8] & 2 == 0 || channels[0][0] == slot {
        return Ok(None);
    }
    let reference_speed = (f64::from(
        globals
            .scalar(if slot == 71 { 0x9ac } else { 0x9b0 })
            .map_err(|e| e.to_string())?,
    ) * f64::from(state.scale)) as f32;
    if !reference_speed.is_finite() {
        return Err("Vimp animation reference speed overflow".into());
    }
    Ok(Some(Request {
        slot: slot as u8,
        flags: 0x21,
        speed_source: 1,
        reference_speed,
        blend_seconds: 0.5,
    }))
}
