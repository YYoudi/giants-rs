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
    pub fire: bool,
    pub blending: animation_channels::Blending,
    pub random: &'a mut crate::audio_parameters::Random,
}
pub(crate) const fn control_mask(fire: bool) -> u32 {
    0xbc00 | if fire { 0x4000 } else { 0 }
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
    let Services {
        blending,
        random,
        fire,
    } = services;
    if payload.len() != 524
        || fields.class != 2
        || fields.action != 0
        || fields.control_flags & !control_mask(fire) != 0
        || fields.response_flags & !(0x80803000 | if fire { 0x08000020 } else { 0 }) != 0
        || fields.object_flags & 0x88000800 != 0
        || fields.position[2] < 0.
        || [0x80, 0x1a8, 0x1dc]
            .into_iter()
            .any(|at| word(payload, at) != 0)
        || word(payload, 0x64) != 0
        || word(payload, 0x180) != 1
        || ![0, 1, 2, 4, 5, 6, 11, 13, 14, 15, 16, 18].contains(&word(payload, 0x30))
            && !(fire && [12, 17, 19, 20, 21].contains(&word(payload, 0x30)))
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
    let descending = view.velocity[2] < -scalar(0xc10)? && f64::from(scalar(0xc14)?) < gap;
    if descending {
        selection = mecc_locomotion::Request {
            clip: 41,
            flags: 0x21,
            speed_source: 0,
            reference_speed: (-160f64 * f64::from(input.scale)) as f32,
        };
    }
    let norm = |v: [f32; 3]| {
        let [x, y, z] = v.map(f64::from);
        (x * x + y * y + z * z).sqrt()
    };
    let difference = std::array::from_fn(|i| {
        (f64::from(view.velocity[i]) - f64::from(view.previous_velocity[i])) as f32
    });
    // The high-descent branch goes directly to the shared animation tail,
    // before the independent impact and contact-landing branches.
    if !descending
        && norm(difference) >= f64::from(scalar(0x66c)?)
        && norm(view.velocity) < norm(view.previous_velocity)
    {
        return Err("ordinary secondary requires acquired impact reaction provider".into());
    }
    if !descending
        && word(payload, 0x84) & 2 != 0
        && (view.velocity[2] < -scalar(0x664)?
            || word(payload, 0x30) == 2 && view.velocity[2] < -scalar(0x668)?)
    {
        return Err("ordinary secondary contact landing effect provider pending".into());
    }
    let (mut logical, mut flag) = match selection.clip {
        1 => (1, 0x400),
        2 => (11, 0x1000),
        5 => (5, 0x800),
        6 => (6, 0),
        39 => (4, 0),
        50 => (18, 0x2000),
        38 => (16, 0x8000),
        41 => (2, 0),
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
    if !fire && logical == 16 && context & 0x2000 != 0 {
        return Err("turn secondary alternate idle provider pending".into());
    }
    let mut force_request = false;
    let mut selecting_override = None;
    let mut blend_seconds = f32::from_bits(0x3e2aaaab);
    if fire && logical == 1 && context & 0x2200 != 0 {
        if context & 0x200 != 0 {
            logical = 21;
            flag = 0x4000;
            selection.clip = 10;
            selection.flags = 0x28;
            blend_seconds = 0.;
            force_request = true;
            selecting_override = Some(true);
        } else {
            logical = 20;
            selection.clip = 43;
            flag = 0;
            selecting_override = Some(f.control_flags & 0x4000 == 0 || ended);
        }
    } else if fire && context & 0x2000 != 0 {
        match logical {
            11 => {
                // The held-trigger timer selects the normal pistol's armed
                // walk even while its ordinary walk channel has not ended.
                logical = 12;
                selection.clip = 44;
            }
            18 => {
                logical = 19;
                selection.clip = 51;
            }
            16 => {
                logical = 17;
                selection.clip = 45;
            }
            _ => {}
        }
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
    let selecting = selecting_override.unwrap_or(
        f.control_flags & flag == 0
            || ended
            || matches!(logical, 12 | 17 | 19)
            || logical == 5 && context & 0x200 != 0,
    );
    let request = (selecting && (force_request || channels[0][0] != u32::from(selection.clip)))
        .then_some(Request {
            slot: selection.clip,
            flags: selection.flags,
            speed_source: selection.speed_source,
            reference_speed: selection.reference_speed,
            blend_seconds,
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

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Fields, Vec<u8>, View, GlobalData) {
        let mut channels = [[0; 9]; 4];
        channels[0][0] = 2;
        let fields = Fields {
            class: 2,
            dispatch_context: 0,
            configuration: [0; 30],
            scale: 1.,
            object_flags: 0,
            response_flags: 0x08000020,
            control_flags: 0x1000,
            action: 0,
            shadow_reciprocal: 0.,
            shadow: None,
            gunlist: Default::default(),
            channels,
            position: [0., 0., 10.],
            current: [0., 0., 10.],
            previous: [0., 0., 10.],
            angles: [0.; 3],
        };
        let mut payload = vec![0; 524];
        for (at, value) in [(0x30, 11), (0x60, 1), (0x180, 1)] {
            put(&mut payload, at, value);
        }
        let view = View {
            number: 1,
            local_seconds: 1. / 60.,
            velocity: [0.; 3],
            previous_velocity: [0.; 3],
            model_rates: [0., 15., 0.],
            previous_model_rates: [0.; 4],
            angular_rate: 0.,
            look_pitch: 0.,
            commands: [0; 8],
            floor: 10.,
            plane: [0., 0., 1., -10.],
            radius: 1.,
            hit: 0,
            support: 0,
            support_part: 0,
            support_triangle: 0,
            affine: None,
        };
        let mut globals = GlobalData {
            words: vec![0; 0xc28 / 4],
            array_words: vec![],
        };
        for at in [0x66c, 0xc10, 0xc14, 0xc18, 0xc1c, 0xc20, 0xc24] {
            globals.words[at / 4] = 100f32.to_bits();
        }
        (fields, payload, view, globals)
    }
    #[test]
    fn descent_transition_requires_both_strict_velocity_and_height_thresholds() {
        let (mut fields, payload, mut view, mut globals) = fixture();
        globals.words[0xc10 / 4] = 10f32.to_bits();
        globals.words[0xc14 / 4] = 3f32.to_bits();
        for (velocity, height, descending) in [
            (-11., 12.999, false),
            (-11., 13., false),
            (-11., 13.001, true),
            (-10., 13.001, false),
            (-9., 13.001, false),
        ] {
            fields.position[2] = height;
            view.velocity[2] = velocity;
            view.previous_velocity[2] = velocity;
            let mut random = crate::audio_parameters::Random(1);
            let result = ordinary(
                &fields,
                &payload,
                fields.channels,
                &view,
                &globals,
                1. / 60.,
                Services {
                    fire: true,
                    blending: animation_channels::Blending::Enabled,
                    random: &mut random,
                },
            );
            let output = result.unwrap();
            if descending {
                let request = output.request.unwrap();
                assert_eq!((request.slot, request.speed_source), (41, 0));
                assert_eq!(request.reference_speed, -160.);
                assert_eq!(word(&output.payload, 0x30), 2);
                assert_eq!(output.fields.control_flags, 0);
            } else {
                assert_eq!(output.request.unwrap().slot, 44);
                assert_eq!(word(&output.payload, 0x30), 12);
            }
            assert_eq!(random.0, 1);
            assert_eq!(word(&payload, 0x30), 11);
        }
    }
    #[test]
    fn high_descent_bypasses_later_reactions_and_retains_an_already_selected_channel() {
        let (mut fields, mut payload, mut view, mut globals) = fixture();
        fields.position[2] = 500.;
        fields.scale = 2.;
        view.velocity[2] = -100.;
        view.previous_velocity[2] = -200.;
        globals.words[0xc10 / 4] = 80f32.to_bits();
        globals.words[0xc14 / 4] = 10f32.to_bits();
        globals.words[0x66c / 4] = 85f32.to_bits();
        put(&mut payload, 0x84, 2);
        put(&mut payload, 0xd4, 7);
        let mut random = crate::audio_parameters::Random(1);
        let first = ordinary(
            &fields,
            &payload,
            fields.channels,
            &view,
            &globals,
            1. / 60.,
            Services {
                fire: true,
                blending: animation_channels::Blending::Enabled,
                random: &mut random,
            },
        )
        .unwrap();
        assert_eq!(first.request.unwrap().reference_speed, -320.);
        assert_eq!(word(&first.payload, 0x30), 2);
        assert_eq!(word(&first.payload, 0xd4), 0);
        assert_eq!(word(&first.payload, 0xd8), 1);
        let held = ordinary(
            &first.fields,
            &first.payload,
            first.channels,
            &view,
            &globals,
            1. / 60.,
            Services {
                fire: true,
                blending: animation_channels::Blending::Enabled,
                random: &mut random,
            },
        )
        .unwrap();
        assert!(held.request.is_none());
        assert_eq!(held.channels, first.channels);
        assert_eq!(word(&held.payload, 0xd8), 1);
        assert_eq!(random.0, 1);
        view.velocity[2] = -20.;
        assert!(
            ordinary(
                &held.fields,
                &held.payload,
                held.channels,
                &view,
                &globals,
                1. / 60.,
                Services {
                    fire: true,
                    blending: animation_channels::Blending::Enabled,
                    random: &mut random
                },
            )
            .is_err()
        );
    }
    #[test]
    fn trigger_selects_armed_walk_before_the_existing_channel_ends() {
        let (fields, payload, view, globals) = fixture();
        let mut random = crate::audio_parameters::Random(1);
        let output = ordinary(
            &fields,
            &payload,
            fields.channels,
            &view,
            &globals,
            1. / 60.,
            Services {
                fire: true,
                blending: animation_channels::Blending::Enabled,
                random: &mut random,
            },
        )
        .unwrap();
        assert_eq!(output.request.unwrap().slot, 44);
        assert_eq!(word(&output.payload, 0x30), 12);
        assert_eq!(f32::from_bits(word(&output.payload, 0x68)), 3.);
        assert_eq!(output.fields.control_flags, 0x1000);
        assert_eq!(random.0, 1);
        assert_eq!(word(&payload, 0x30), 11);
        assert!(
            ordinary(
                &fields,
                &payload,
                fields.channels,
                &view,
                &globals,
                1. / 60.,
                Services {
                    fire: false,
                    blending: animation_channels::Blending::Enabled,
                    random: &mut random
                }
            )
            .is_err()
        );
    }
    #[test]
    fn trigger_selects_armed_turn_with_angular_playback_rate() {
        let (fields, payload, mut view, globals) = fixture();
        view.model_rates = [0.; 3];
        view.angular_rate = 5.;
        let mut random = crate::audio_parameters::Random(1);
        let output = ordinary(
            &fields,
            &payload,
            fields.channels,
            &view,
            &globals,
            1. / 60.,
            Services {
                fire: true,
                blending: animation_channels::Blending::Enabled,
                random: &mut random,
            },
        )
        .unwrap();
        let request = output.request.unwrap();
        assert_eq!((request.slot, request.speed_source), (45, 3));
        assert_eq!(request.reference_speed, -321.);
        assert_eq!(output.fields.control_flags, 0x8000);
        assert_eq!(word(&output.payload, 0x30), 17);
        assert_eq!(random.0, 1);
    }
    #[test]
    fn standing_shot_restarts_without_blending_and_holds_until_channel_end() {
        let (mut fields, payload, mut view, globals) = fixture();
        view.model_rates = [0.; 3];
        // A fresh shot must restart even an already selected firing clip.
        fields.channels[0][0] = 10;
        let mut random = crate::audio_parameters::Random(1);
        let step =
            |fields: &Fields, payload: &[u8], random: &mut crate::audio_parameters::Random| {
                ordinary(
                    fields,
                    payload,
                    fields.channels,
                    &view,
                    &globals,
                    1. / 60.,
                    Services {
                        fire: true,
                        blending: animation_channels::Blending::Enabled,
                        random,
                    },
                )
                .unwrap()
            };
        let shot = step(&fields, &payload, &mut random);
        let request = shot.request.unwrap();
        assert_eq!((request.slot, request.flags), (10, 0x28));
        assert_eq!(request.blend_seconds, 0.);
        assert_eq!(shot.fields.control_flags, 0x4000);
        assert_eq!(word(&shot.payload, 0x30), 21);
        let mut released = shot.fields;
        released.response_flags = 0;
        let held = step(&released, &shot.payload, &mut random);
        assert!(held.request.is_none());
        assert_eq!(word(&held.payload, 0x30), 21);
        let mut ended = held.fields;
        ended.channels[0][8] |= 2;
        let aiming = step(&ended, &held.payload, &mut random);
        assert_eq!(aiming.request.unwrap().slot, 43);
        assert_eq!(aiming.fields.control_flags, 0);
        assert_eq!(word(&aiming.payload, 0x30), 20);
        assert_eq!(random.0, 1);
    }
    #[test]
    fn trigger_selects_armed_side_before_channel_end() {
        let (fields, payload, mut view, globals) = fixture();
        view.model_rates = [0., 0., 15.];
        let mut random = crate::audio_parameters::Random(1);
        let output = ordinary(
            &fields,
            &payload,
            fields.channels,
            &view,
            &globals,
            1. / 60.,
            Services {
                fire: true,
                blending: animation_channels::Blending::Enabled,
                random: &mut random,
            },
        )
        .unwrap();
        let request = output.request.unwrap();
        assert_eq!((request.slot, request.speed_source), (51, 2));
        assert_eq!(request.reference_speed, -40.);
        assert_eq!(output.fields.control_flags, 0x2000);
        assert_eq!(word(&output.payload, 0x30), 19);
        assert_eq!(random.0, 1);
    }
}
