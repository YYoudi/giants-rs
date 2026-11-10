//! Acquired local, living pistol locomotion secondary with explicit blending.
//! Equipment and alternate-turn providers remain explicit refusals. This
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
    pub draw: Option<crate::mecc_pistol_draw::Binding>,
    pub commit_before_request: Option<Commit>,
}
pub(crate) struct Commit {
    pub fields: Fields,
    pub payload: Vec<u8>,
}

pub(crate) struct Services<'a> {
    pub fire: bool,
    pub blending: animation_channels::Blending,
    pub random: &'a mut crate::audio_parameters::Random,
    pub contact: Option<ContactServices<'a>>,
    /// Real current actor damage/authority provider; absent keeps reaction refusal.
    pub impact: Option<&'a mut dyn ImpactDamage>,
}

/// Damage executes before contact precedence and positional effect preparation.
/// The provider must retain its actual health/service effects on a late error.
pub(crate) trait ImpactDamage {
    fn apply(&mut self, fields: &mut Fields, payload: &mut [u8], amount: f32)
    -> Result<(), String>;

    fn apply_with_random(
        &mut self,
        fields: &mut Fields,
        payload: &mut [u8],
        amount: f32,
        _random: &mut crate::audio_parameters::Random,
    ) -> Result<(), String> {
        self.apply(fields, payload, amount)
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Contact {
    binding: Option<crate::attached_effect_context::Id>,
}
pub(crate) struct ContactServices<'a> {
    pub state: &'a mut Contact,
    pub pool: &'a mut crate::attached_effect_context::Pool<crate::resident_states::Id>,
    pub source: crate::resident_states::Id,
    pub checkpoint: &'a mut Option<(Fields, Vec<u8>)>,
}
impl ContactServices<'_> {
    fn advance(
        &mut self,
        previous: u32,
        marker: u32,
        flags: u32,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<u32, String> {
        if marker != u32::from(self.state.binding.is_some()) {
            return Err("contact effect marker differs from its owned binding".into());
        }
        if let Some(id) = self.state.binding {
            let context = self
                .pool
                .get(id)
                .ok_or("contact effect binding absent, foreign or stale")?;
            if context.recipient() != Some(self.source)
                || context.tag() != Some(u32::MAX)
                || context.flags() != 0x20004
            {
                return Err("contact effect binding belongs to another profile/source".into());
            }
        }
        if previous == 60 {
            if self.state.binding.is_none() {
                let (creation, _) = self.pool.prepare_extended_invariant(
                    "FX_mecc_slide",
                    self.source,
                    flags,
                    u32::MAX,
                    random,
                )?;
                if let crate::attached_effect_context::Creation::Prepared(id) = creation {
                    self.state.binding = Some(id);
                }
            }
        } else if let Some(id) = self.state.binding {
            self.pool.release_prepared(id)?;
            self.state.binding = None;
        }
        Ok(u32::from(self.state.binding.is_some()))
    }
}
pub(crate) const fn control_mask(fire: bool) -> u32 {
    0x1bc00 | if fire { 0x4000 } else { 0 }
}
fn word(data: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(data[at..at + 4].try_into().unwrap())
}
fn put(data: &mut [u8], at: usize, value: u32) {
    data[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

/// The fourth model rate is the source-owned angular rate, including stationary turns.
/// Model speeds, floor/support and local delta come from the owned physics.
// Retained entry for separately compiled acquisition controls without a draw provider.
#[allow(dead_code)]
pub(crate) fn ordinary(
    fields: &Fields,
    payload: &[u8],
    channels: Channels,
    view: &View,
    globals: &GlobalData,
    seconds: f64,
    services: Services<'_>,
) -> Result<Output, String> {
    ordinary_with_draw(
        fields, payload, channels, view, globals, seconds, services, None,
    )
}
#[allow(clippy::too_many_arguments)]
pub(crate) fn ordinary_with_draw(
    fields: &Fields,
    payload: &[u8],
    mut channels: Channels,
    view: &View,
    globals: &GlobalData,
    seconds: f64,
    services: Services<'_>,
    draw: Option<&mut crate::mecc_pistol_draw::Actions>,
) -> Result<Output, String> {
    let Services {
        blending,
        random,
        fire,
        mut contact,
        mut impact,
    } = services;
    if payload.len() != 524 {
        return Err(
            "ordinary secondary outside acquired source pistol profile: payload length".into(),
        );
    }
    let logical_state = word(payload, 0x30);
    let unsupported = [
        (fields.class != 2, "class"),
        (fields.action != 0, "action"),
        (
            fields.control_flags
                & !(control_mask(fire) & if draw.is_some() { u32::MAX } else { !0x10000 })
                != 0,
            "control flags",
        ),
        (
            fields.response_flags & !(0x80803000 | if fire { 0x08000020 } else { 0 }) != 0,
            "response flags",
        ),
        (fields.object_flags & 0x88000800 != 0, "object flags"),
        (
            word(payload, 0x1dc) != 0 || word(payload, 0x80) > u32::from(draw.is_some()),
            "active auxiliary",
        ),
        (word(payload, 0x64) != 0, "weapon transition"),
        (word(payload, 0x180) != 1, "weapon inventory"),
        (
            ![
                0, 1, 2, 3, 4, 5, 6, 7, 11, 13, 14, 15, 16, 18, 58, 59, 60, 61, 62,
            ]
            .contains(&logical_state)
                && !(23..=35).contains(&logical_state)
                && !(draw.is_some() && logical_state == 38 && word(payload, 0x80) == 1)
                && !(fire && [12, 17, 19, 20, 21].contains(&logical_state)),
            "logical state",
        ),
        (
            ![u32::MAX, 1].contains(&word(payload, 0x60)),
            "weapon cache",
        ),
    ]
    .into_iter()
    .find(|(failed, _)| *failed);
    if let Some((_, reason)) = unsupported {
        return Err(format!(
            "ordinary secondary outside acquired source pistol profile: {reason}; logical={logical_state} action={} height={}",
            fields.action, fields.position[2]
        ));
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
    let descent = mecc_locomotion::Descent::from_globals(globals)
        .map_err(|e| e.to_string())?
        .request(input, view.velocity[2])?;
    let descending = descent.is_some();
    if let Some(request) = descent {
        selection = request;
    }
    let reaction = if descending {
        None
    } else {
        Some(crate::mecc_impact::classify(
            view.velocity,
            view.previous_velocity,
            crate::mecc_impact::Parameters::from_globals(globals)?,
        )?)
    };
    if reaction.is_some_and(|r| r.severity != 0) && impact.is_none() {
        return Err("ordinary secondary requires acquired impact reaction provider".into());
    }
    let contact_selection = !descending
        && word(payload, 0x84) & 2 != 0
        && (view.velocity[2] < -scalar(0x664)?
            || word(payload, 0x30) == 2 && view.velocity[2] < -scalar(0x668)?);
    let contact_hold = !descending
        && !reaction.is_some_and(|r| r.severity != 0)
        && !contact_selection
        && word(payload, 0x30) == 60
        && view.velocity[2] < -scalar(0x668)?;
    if contact_selection || contact_hold {
        selection = mecc_locomotion::Request {
            clip: 181,
            flags: 0x21,
            speed_source: -1,
            reference_speed: 0.,
        };
    }
    if !contact_selection && let Some(reaction) = reaction.filter(|r| r.severity != 0) {
        selection = mecc_locomotion::Request {
            clip: if reaction.severity == 2 { 178 } else { 175 },
            flags: 0x20,
            speed_source: -1,
            reference_speed: 0.,
        };
    }
    let (mut logical, mut flag) = match selection.clip {
        1 => (1, 0x400),
        2 => (11, 0x1000),
        5 => (5, 0x800),
        6 => (6, 0),
        39 => (4, 0),
        40 => (3, 0),
        7 => (7, 0),
        172 => (58, 0),
        169 => (59, 0),
        50 => (18, 0x2000),
        38 => (16, 0x8000),
        41 => (2, 0),
        181 => (60, 0),
        175 => (61, 0x400),
        178 => (62, 0x400),
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
    if let Some(reaction) = reaction.filter(|r| r.severity != 0) {
        let service = contact
            .as_mut()
            .ok_or("impact requires the owned effect pool")?;
        f.channels = channels;
        *service.checkpoint = Some((f.clone(), body.clone()));
        let result = impact.as_mut().unwrap().apply_with_random(
            &mut f,
            &mut body,
            reaction.amount.unwrap(),
            random,
        );
        *service.checkpoint = Some((f.clone(), body.clone()));
        result?;
        // Damage precedes contact selection. Contact suppresses the positional
        // context, not the damage that has already executed.
        if !contact_selection {
            service
                .pool
                .prepare_positional_invariant("fx_mecc_hard_land", f.position, random)?;
            if reaction.severity == 2 && f.response_flags & 0x800000 != 0 {
                return Err(
                    "heavy impact requires the actual carried actor channel service".into(),
                );
            }
        }
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
    let drawing = logical == 1 && changed_weapon && context & 0x4000 != 0 && context & 0x2200 == 0;
    if drawing {
        draw.as_ref()
            .ok_or("normal pistol draw requires the owned animation callback/frame service")?;
        // Argument rebinding does not reset the registered class's event marks.
        // The acquired base icon has fixed left/right classes, checked at bank preparation.
        logical = 38;
        flag = 0x10000;
        selection.clip = 147;
        selection.flags = 0x20;
        selection.speed_source = -1;
        selection.reference_speed = 0.;
    }
    if logical == 1 && f.control_flags & 0x400 != 0 && ended && context & 0x8000 != 0 {
        if context & 0x6200 != 0x4000 {
            return Err("idle variant outside acquired stationary pistol profile".into());
        }
        // The draw is consumed even for the mandatory continuation of state28.
        let roll = random.next_15() * 800 / 32768;
        logical = if logical_state == 28 {
            29
        } else {
            match roll {
                0..10 => 23,
                10..20 => 24,
                20..30 => 25,
                30..40 => 26,
                40..50 => 27,
                50..90 => 28,
                90..110 => 30,
                110..120 => 31,
                120..130 => 32,
                130..140 => 33,
                140..150 => 35,
                _ => 1,
            }
        };
        if logical != 1 {
            selection.clip = match logical {
                23 => 156,
                24 => 157,
                25 | 27 => 150,
                26 => 151,
                28 | 29 => 146,
                30 => 148,
                31 => 149,
                32 => 158,
                33 => 152,
                35 => 155,
                _ => unreachable!(),
            };
            selection.flags = if matches!(logical, 28 | 29) {
                0x21
            } else {
                0x20
            };
            selection.speed_source = -1;
            selection.reference_speed = 0.;
        }
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
    // Native ownership uses the previous logical state, independently of this
    // frame's selection. Creation/cleanup precedes the primary channel setter.
    if word(payload, 0x30) == 60
        || word(payload, 0x1a8) != 0
        || contact.as_ref().is_some_and(|c| c.state.binding.is_some())
    {
        let service = contact
            .as_mut()
            .ok_or("ordinary contact requires the owned effect pool/binding")?;
        let mut checkpoint_fields = f.clone();
        checkpoint_fields.channels = channels;
        *service.checkpoint = Some((checkpoint_fields.clone(), body.clone()));
        let marker = service.advance(
            word(payload, 0x30),
            word(payload, 0x1a8),
            f.object_flags,
            random,
        )?;
        put(&mut body, 0x1a8, marker);
        *service.checkpoint = Some((checkpoint_fields, body.clone()));
    }
    let request = (!contact_hold
        && selecting
        && (force_request || channels[0][0] != u32::from(selection.clip)))
    .then_some(Request {
        slot: selection.clip,
        flags: selection.flags,
        speed_source: selection.speed_source,
        reference_speed: selection.reference_speed,
        blend_seconds,
    });
    let mut commit_before_request = None;
    if let Some(request) = request {
        animation_channels::select_primary(&mut channels, request, blending)?;
        if f.control_flags & 0x10000 != 0 {
            let mut fields = f.clone();
            fields.channels = channels;
            commit_before_request = Some(Commit {
                fields,
                payload: body.clone(),
            });
        }
        f.control_flags = (f.control_flags & 0xfffe03ff) | flag;
        put(&mut body, 0xd4, 0);
        put(&mut body, 0x80, u32::from(drawing));
        put(&mut body, 0x30, logical);
        let alternating = word(&body, 0xd8) ^ 1;
        put(&mut body, 0xd8, alternating);
    }
    f.channels = channels;
    let commit_weapon = changed_weapon && (f.control_flags & 0x10000 == 0 || ended);
    let draw = (word(&body, 0x80) != 0).then_some(crate::mecc_pistol_draw::Binding::Base);
    Ok(Output {
        fields: f,
        payload: body,
        channels,
        request,
        context,
        commit_weapon,
        draw,
        commit_before_request,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn idle_continuation_consumes_rng_without_reselecting_its_shared_clip() {
        for ended in [false, true] {
            let (mut fields, mut body, mut view, globals) = fixture();
            fields.response_flags = 0;
            fields.control_flags = 0x400;
            fields.channels[0][0] = 146;
            fields.channels[0][8] = if ended { 2 } else { 0 };
            view.model_rates = [0.; 3];
            put(&mut body, 0x30, 28);
            put(&mut body, 0x60, 1);
            put(&mut body, 0x6c, 0);
            let mut random = crate::audio_parameters::Random(42);
            let mut expected = random;
            if ended {
                expected.next_15();
            }
            let output = ordinary(
                &fields,
                &body,
                fields.channels,
                &view,
                &globals,
                1. / 60.,
                Services {
                    fire: false,
                    blending: animation_channels::Blending::Disabled,
                    random: &mut random,
                    contact: None,
                    impact: None,
                },
            )
            .unwrap();
            assert!(output.request.is_none());
            assert_eq!(output.channels, fields.channels);
            assert_eq!(word(&output.payload, 0x30), 28);
            assert_eq!(random.0, expected.0);
        }
    }
    #[test]
    fn movement_interrupts_an_unended_idle_variant_without_an_idle_rng_draw() {
        let (mut fields, mut body, mut view, globals) = fixture();
        fields.response_flags = 0;
        fields.control_flags = 0x400;
        fields.channels[0][0] = 156;
        fields.channels[0][8] = 0;
        view.model_rates = [0., 30., 0.];
        put(&mut body, 0x30, 23);
        put(&mut body, 0x60, 1);
        put(&mut body, 0x6c, 0);
        let mut random = crate::audio_parameters::Random(42);
        let output = ordinary(
            &fields,
            &body,
            fields.channels,
            &view,
            &globals,
            1. / 60.,
            Services {
                fire: false,
                blending: animation_channels::Blending::Disabled,
                random: &mut random,
                contact: None,
                impact: None,
            },
        )
        .unwrap();
        assert_eq!(output.request.unwrap().slot, 2);
        assert_eq!(word(&output.payload, 0x30), 11);
        assert_eq!(random.0, 42);
    }
    #[test]
    fn landing_hold_depends_on_channel_end_and_movement() {
        for (previous, clip) in [(61, 175), (62, 178)] {
            for ended in [false, true] {
                for moving in [false, true] {
                    let (mut fields, mut body, mut view, globals) = fixture();
                    fields.control_flags = 0x400;
                    fields.response_flags = 0;
                    fields.channels[0][0] = clip;
                    fields.channels[0][8] = if ended { 2 } else { 0 };
                    put(&mut body, 0x30, previous);
                    put(&mut body, 0x6c, 1f32.to_bits());
                    view.model_rates = [0., if moving { 30. } else { 0. }, 0.];
                    let mut random = crate::audio_parameters::Random(42);
                    let output = ordinary(
                        &fields,
                        &body,
                        fields.channels,
                        &view,
                        &globals,
                        1. / 60.,
                        Services {
                            fire: false,
                            blending: animation_channels::Blending::Disabled,
                            random: &mut random,
                            contact: None,
                            impact: None,
                        },
                    )
                    .unwrap();
                    if moving || ended {
                        assert_eq!(output.request.unwrap().slot, if moving { 2 } else { 1 });
                        assert_eq!(word(&output.payload, 0x30), if moving { 11 } else { 1 });
                    } else {
                        assert!(output.request.is_none());
                        assert_eq!(word(&output.payload, 0x30), previous);
                        assert_eq!(output.channels, fields.channels);
                    }
                    assert!(!output.commit_weapon);
                    assert_eq!(random.0, 42);
                    assert_eq!(word(&body, 0x30), previous);
                }
            }
        }
    }

    #[test]
    fn all_air_directions_can_continue_from_their_selected_state() {
        for (forward, side, clip, logical, flag) in [
            (0., 0., 5, 5, 0x800),
            (-75., 0., 39, 4, 0),
            (-150., 0., 40, 3, 0),
            (75., 0., 6, 6, 0),
            (150., 0., 7, 7, 0),
            (0., -150., 172, 58, 0),
            (0., 150., 169, 59, 0),
        ] {
            let (mut fields, mut body, mut view, mut globals) = fixture();
            fields.position[2] = 100.;
            fields.response_flags = 0;
            fields.control_flags = 0;
            globals.words[0xc18 / 4] = 50f32.to_bits();
            globals.words[0xc24 / 4] = 10f32.to_bits();
            put(&mut body, 0x6c, 1f32.to_bits());
            view.model_rates = [0., forward, side];
            let mut random = crate::audio_parameters::Random(42);
            let mut step = |f: &Fields, b: &[u8]| {
                ordinary(
                    f,
                    b,
                    f.channels,
                    &view,
                    &globals,
                    1. / 60.,
                    Services {
                        fire: false,
                        blending: animation_channels::Blending::Disabled,
                        random: &mut random,
                        contact: None,
                        impact: None,
                    },
                )
                .unwrap()
            };
            let selected = step(&fields, &body);
            assert_eq!(selected.request.unwrap().slot, clip);
            assert_eq!(word(&selected.payload, 0x30), logical);
            assert_eq!(selected.fields.control_flags, flag);
            let continued = step(&selected.fields, &selected.payload);
            assert!(continued.request.is_none());
            assert_eq!(word(&continued.payload, 0x30), logical);
            assert_eq!(continued.channels, selected.channels);
            assert_eq!(random.0, 42);
        }
    }

    #[test]
    fn idle_weapon_change_requires_real_draw_callback_before_commit() {
        let (mut fields, mut body, mut view, globals) = fixture();
        fields.response_flags = 0;
        fields.control_flags = 0x400;
        view.model_rates = [0.; 3];
        put(&mut body, 0x60, u32::MAX);
        put(&mut body, 0x6c, 1f32.to_bits());
        let before = body.clone();
        let mut random = crate::audio_parameters::Random(42);
        let error = ordinary(
            &fields,
            &body,
            fields.channels,
            &view,
            &globals,
            1. / 60.,
            Services {
                fire: false,
                blending: animation_channels::Blending::Disabled,
                random: &mut random,
                contact: None,
                impact: None,
            },
        )
        .err()
        .unwrap();
        assert!(error.contains("animation callback/frame service"));
        assert_eq!(body, before);
        assert_eq!(random.0, 42);
    }

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
    fn living_selection_uses_floor_relative_gap_across_absolute_zero() {
        let (mut fields, payload, mut view, mut globals) = fixture();
        globals.words[0xc10 / 4] = 10f32.to_bits();
        globals.words[0xc14 / 4] = 3f32.to_bits();
        for gap in [0., 4.] {
            let mut expected = None;
            for height in [-100., -1., -0.000001, -0., 0., 0.000001, 1., 100.] {
                fields.position[2] = height;
                fields.current[2] = height;
                fields.previous[2] = height;
                view.floor = height - gap;
                view.plane[3] = -view.floor;
                view.velocity[2] = -11.;
                view.previous_velocity = view.velocity;
                let mut random = crate::audio_parameters::Random(1);
                let output = ordinary(
                    &fields,
                    &payload,
                    fields.channels,
                    &view,
                    &globals,
                    1. / 60.,
                    Services {
                        impact: None,
                        contact: None,
                        fire: true,
                        blending: animation_channels::Blending::Enabled,
                        random: &mut random,
                    },
                )
                .unwrap();
                let request = output.request.map(|q| {
                    (
                        q.slot,
                        q.flags,
                        q.speed_source,
                        q.reference_speed.to_bits(),
                        q.blend_seconds.to_bits(),
                    )
                });
                let state = (
                    output.payload,
                    output.channels,
                    output.fields.control_flags,
                    request,
                    random.0,
                );
                assert_eq!(output.fields.position[2].to_bits(), height.to_bits());
                if let Some(previous) = &expected {
                    assert_eq!(&state, previous);
                } else {
                    expected = Some(state);
                }
            }
        }
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
                    impact: None,
                    contact: None,
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
                impact: None,
                contact: None,
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
                impact: None,
                contact: None,
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
        view.previous_velocity[2] = -20.;
        let contact = ordinary(
            &held.fields,
            &held.payload,
            held.channels,
            &view,
            &globals,
            1. / 60.,
            Services {
                impact: None,
                fire: true,
                blending: animation_channels::Blending::Enabled,
                random: &mut random,
                contact: None,
            },
        )
        .unwrap();
        assert_eq!(contact.request.unwrap().slot, 181);
        assert_eq!(word(&contact.payload, 0x30), 60);
        assert_eq!(word(&contact.payload, 0x1a8), 0);
        assert!(
            ordinary(
                &contact.fields,
                &contact.payload,
                contact.channels,
                &view,
                &globals,
                1. / 60.,
                Services {
                    impact: None,
                    fire: true,
                    blending: animation_channels::Blending::Enabled,
                    random: &mut random,
                    contact: None
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
                impact: None,
                contact: None,
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
                    impact: None,
                    contact: None,
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
                impact: None,
                contact: None,
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
                        impact: None,
                        contact: None,
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
                impact: None,
                contact: None,
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
