//! Acquired local, living pistol locomotion secondary with explicit blending.
//! Reaction/equipment/idle-variant providers remain explicit refusals. This
//! stage owns its actual payload/control writes, not a complete primary frame.
use crate::{
    actor_fields::Fields,
    animation_channels::{self, Channels, Request},
    arithmetic::Precision,
    mecc_locomotion,
    mecc_physical_cycle::View,
};
use giants_formats::global_data::GlobalData;

pub(crate) struct Output {
    pub fields: Fields,
    pub payload: Vec<u8>,
    pub channels: Channels,
    pub request: Option<Request>,
    pub context: u32,
    pub commit_weapon: bool,
}
pub(crate) struct Services<'a> {
    pub blending: animation_channels::Blending,
    pub random: &'a mut crate::audio_parameters::Random,
}
fn word(data: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(data[at..at + 4].try_into().unwrap())
}
fn put(data: &mut [u8], at: usize, value: u32) {
    data[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

/// The fourth model rate is the source-owned angular rate, including stationary turns.
/// Model speeds, floor/support and local delta come from the owned physics.
pub(crate) fn ordinary(
    fields: &Fields,
    payload: &[u8],
    mut channels: Channels,
    view: &View,
    globals: &GlobalData,
    seconds: f64,
    services: Services<'_>,
) -> Result<Output, String> {
    let Services { blending, random } = services;
    if payload.len() != 524
        || fields.class != 2
        || fields.action != 0
        || fields.control_flags & !0xbc00 != 0
        || fields.response_flags & !0x80803000 != 0
        || fields.object_flags & 0x88000800 != 0
        || fields.position[2] < 0.
        || [0x80, 0x1a8, 0x1dc]
            .into_iter()
            .any(|at| word(payload, at) != 0)
        || word(payload, 0x64) != 0
        || word(payload, 0x180) != 1
        || ![0, 1, 5, 6, 11, 13, 14, 15, 16, 18].contains(&word(payload, 0x30))
        || ![u32::MAX, 1].contains(&word(payload, 0x60))
    {
        return Err("ordinary secondary outside acquired source pistol profile".into());
    }
    let parameters =
        mecc_locomotion::Parameters::from_globals(globals).map_err(|e| e.to_string())?;
    let input = mecc_locomotion::Input {
        model: view.model_rates,
        position_z: fields.position[2],
        floor: view.floor,
        center_offset: f32::from_bits(fields.configuration[17]),
        scale: fields.scale,
        supported: view.support != 0,
    };
    let mut selection = mecc_locomotion::select(input, parameters, Precision::Double53)?;
    if selection.clip == 1 && view.angular_rate != 0. {
        selection = mecc_locomotion::Request {
            clip: 38,
            flags: 0x21,
            speed_source: 3,
            reference_speed: (-321f64 * f64::from(input.scale)) as f32,
        };
    }
    let gap = (f64::from(input.position_z) - f64::from(input.floor))
        - f64::from(input.center_offset) * f64::from(input.scale);
    let scalar = |at| globals.scalar(at).map_err(|e| e.to_string());
    if view.velocity[2] < -scalar(0xc10)? && gap < f64::from(scalar(0xc14)?) {
        return Err("ordinary secondary requires acquired landing reaction provider".into());
    }
    let norm = |v: [f32; 3]| {
        let [x, y, z] = v.map(f64::from);
        (x * x + y * y + z * z).sqrt()
    };
    let difference = std::array::from_fn(|i| {
        (f64::from(view.velocity[i]) - f64::from(view.previous_velocity[i])) as f32
    });
    if norm(difference) >= f64::from(scalar(0x66c)?)
        && norm(view.velocity) < norm(view.previous_velocity)
    {
        return Err("ordinary secondary requires acquired impact reaction provider".into());
    }
    if word(payload, 0x84) & 2 != 0
        && (view.velocity[2] < -scalar(0x664)?
            || word(payload, 0x30) == 2 && view.velocity[2] < -scalar(0x668)?)
    {
        return Err("ordinary secondary contact landing effect provider pending".into());
    }
    let (mut logical, flag) = match selection.clip {
        1 => (1, 0x400),
        2 => (11, 0x1000),
        5 => (5, 0x800),
        6 => (6, 0),
        50 => (18, 0x2000),
        38 => (16, 0x8000),
        _ => return Err("ordinary secondary directional-air variant pending".into()),
    };
    let mut f = fields.clone();
    let mut body = payload.to_vec();
    let changed_weapon = word(&body, 0x60) != 1;
    let ended = channels[0][8] & 2 != 0;
    let airborne = !input.supported && gap > f64::from(parameters.air_height);
    let mut context = if changed_weapon { 0x20000 } else { 0 };
    if ended {
        context |= 0x80000;
    }
    context |= (f.object_flags >> 10) & 4;
    if airborne {
        context |= 2;
    }
    context |= (f.control_flags & 0x100) << 2;
    context |= (f.response_flags & 0x20) << 4;
    context |= (f.response_flags >> 16) & 0x800;
    if airborne && view.model_rates[0] != 0. {
        context |= 0x40;
    }
    if view.model_rates[1] != 0. {
        context |= 0x10;
    }
    if view.model_rates[2] != 0. {
        context |= 0x20;
    }
    if view.angular_rate != 0. {
        context |= 0x80;
    }
    if context & 0xf0 != 0 {
        context |= 0x100;
    }
    if view.model_rates[0] < -60. {
        context |= 8;
    }
    for at in [0x68, 0x6c] {
        let value = f32::from_bits(word(&body, at));
        if !value.is_finite() || !seconds.is_finite() || seconds < 0. {
            return Err("ordinary secondary timer outside finite profile".into());
        }
        let next = (f64::from(value) - seconds).max(0.) as f32;
        put(&mut body, at, next.to_bits());
        if at == 0x68 {
            if context & 0x800 != 0 {
                put(&mut body, at, 3f32.to_bits());
            }
            if word(&body, at) != 0 {
                context |= 0x2000;
            }
        }
    }
    if context & 0x1900 == 0 {
        context |= 0x4000;
    }
    if context & 0x4000 == 0 || context & 0x2000 != 0 {
        put(&mut body, 0x6c, 2f32.to_bits());
    }
    if f32::from_bits(word(&body, 0x6c)) == 0. && context & 0x10000 == 0 {
        context |= 0x8000;
    }
    if logical == 16 && context & 0x2000 != 0 {
        return Err("turn secondary alternate idle provider pending".into());
    }
    if logical == 1 && f.control_flags & 0x400 != 0 && ended && context & 0x8000 != 0 {
        return Err(
            "ordinary secondary idle variant requires actual RNG/selection provider".into(),
        );
    }
    if changed_weapon {
        f.control_flags &= 0xffff03ff;
    }
    if logical == 11 && f.control_flags & 0x1000 != 0 && ended {
        let roll = random.next_15() * 100 / 32768;
        logical = if roll < 8 {
            13
        } else if roll < 15 {
            14
        } else if roll < 24 {
            15
        } else {
            11
        };
        if logical != 11 {
            selection.clip = if logical == 14 { 72 } else { 71 };
            selection.flags = 0x20;
        }
    }
    let selecting = f.control_flags & flag == 0 || ended || logical == 5 && context & 0x200 != 0;
    let request = (selecting && channels[0][0] != u32::from(selection.clip)).then_some(Request {
        slot: selection.clip,
        flags: selection.flags,
        speed_source: selection.speed_source,
        reference_speed: selection.reference_speed,
        blend_seconds: f32::from_bits(0x3e2aaaab),
    });
    if let Some(request) = request {
        animation_channels::select_primary(&mut channels, request, blending)?;
        f.control_flags = (f.control_flags & 0xfffe03ff) | flag;
        put(&mut body, 0xd4, 0);
        put(&mut body, 0x80, 0);
        put(&mut body, 0x30, logical);
        let alternating = word(&body, 0xd8) ^ 1;
        put(&mut body, 0xd8, alternating);
    }
    f.channels = channels;
    Ok(Output {
        fields: f,
        payload: body,
        channels,
        request,
        context,
        commit_weapon: changed_weapon,
    })
}
