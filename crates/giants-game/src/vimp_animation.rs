//! Acquired mode1 locomotion and hurt mode2..4 animation selection.
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
    if fields.class != 8 || vimp.mode != 1 || fields.object_flags & 0x8000_0000 != 0 {
        return Err("Vimp animation requires its owned mode1 locomotion profile".into());
    }
    mode1_request_with_locomotion(
        Mode1 {
            forward_rate: physical.model_rates[1],
            angular_speed: vimp.angular_speed,
            damage: vimp.damage,
            scale: fields.scale,
        },
        Locomotion {
            gait: crate::vimp_wander::word(body, 0x74),
            physical_flags: crate::vimp_wander::word(body, 0),
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
#[derive(Clone, Copy)]
pub struct Locomotion {
    pub gait: u32,
    pub physical_flags: u32,
}
/// Existing dry gait0 entry, including the visit returning from mode3.
/// The primary and resource consumer own requested descriptor failures.
pub fn mode1_request(
    state: Mode1,
    channels: &Channels,
    has_clip: impl Fn(u8) -> bool,
    globals: &giants_formats::global_data::GlobalData,
) -> Result<Option<Request>, String> {
    mode1_request_with_locomotion(
        state,
        Locomotion {
            gait: 0,
            physical_flags: 0,
        },
        channels,
        has_clip,
        globals,
    )
}
/// Numeric locomotion selection after source mode/ownership admission. This
/// does not supply movement, water contact, animation clocks or world readiness.
pub fn mode1_request_with_locomotion(
    state: Mode1,
    locomotion: Locomotion,
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
    let (slot, reference_offset) = if locomotion.physical_flags & 0x40 != 0 {
        (72, 0x970)
    } else if state.damage > 0. {
        (71, 0x9ac)
    } else {
        match locomotion.gait {
            0 => (2, 0x9b0),
            1 => (3, 0x9b4),
            _ => (4, 0x9b8),
        }
    };
    if channels[0][0] == 64 && channels[0][8] & 2 == 0 || channels[0][0] == u32::from(slot) {
        return Ok(None);
    }
    let reference_speed = (f64::from(
        globals
            .scalar(reference_offset)
            .map_err(|e| e.to_string())?,
    ) * f64::from(state.scale)) as f32;
    if !reference_speed.is_finite() {
        return Err("Vimp animation reference speed overflow".into());
    }
    Ok(Some(Request {
        slot,
        flags: 0x21,
        speed_source: 1,
        reference_speed,
        blend_seconds: 0.5,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn globals() -> giants_formats::global_data::GlobalData {
        let mut g = giants_formats::global_data::GlobalData {
            words: vec![0; 0xa00 / 4],
            array_words: Vec::new(),
        };
        for (at, v) in [
            (0x9b0, 12f32),
            (0x9b4, 23.),
            (0x9b8, 34.),
            (0x9ac, 56.),
            (0x970, 78.),
        ] {
            g.words[at / 4] = v.to_bits();
        }
        g
    }
    fn channels(slot: u32) -> Channels {
        let mut c = [[0; 9]; 4];
        for row in &mut c {
            row[0] = 254;
        }
        c[0][0] = slot;
        c
    }
    fn state() -> Mode1 {
        Mode1 {
            forward_rate: 2.,
            angular_speed: 0.,
            damage: 0.,
            scale: 0.5,
        }
    }
    #[test]
    fn locomotion_priority_and_native_fallback_choose_distinct_reference_sources() {
        let g = globals();
        let c = channels(254);
        for (gait, water, damage, slot, speed) in [
            (0, 0, 0., 2, 6.),
            (1, 0, 0., 3, 11.5),
            (2, 0, 0., 4, 17.),
            (u32::MAX, 0, 0., 4, 17.),
            (2, 0, 0.1, 71, 28.),
            (1, 0x40, 0.1, 72, 39.),
        ] {
            let request = mode1_request_with_locomotion(
                Mode1 { damage, ..state() },
                Locomotion {
                    gait,
                    physical_flags: water,
                },
                &c,
                |_| false,
                &g,
            )
            .unwrap()
            .unwrap();
            assert_eq!(
                (
                    request.slot,
                    request.reference_speed,
                    request.flags,
                    request.speed_source
                ),
                (slot, speed, 0x21, 1)
            );
            assert_eq!(request.blend_seconds, 0.5);
        }
    }
    #[test]
    fn direct_moving_selection_retains_request_when_descriptor_is_absent() {
        let g = globals();
        let c = channels(254);
        let request = mode1_request_with_locomotion(
            state(),
            Locomotion {
                gait: 1,
                physical_flags: 0,
            },
            &c,
            |_| false,
            &g,
        )
        .unwrap()
        .unwrap();
        assert_eq!(request.slot, 3);
        let still = Mode1 {
            forward_rate: 0.,
            angular_speed: 2.,
            ..state()
        };
        assert!(
            mode1_request_with_locomotion(
                still,
                Locomotion {
                    gait: 2,
                    physical_flags: 0x40
                },
                &c,
                |_| false,
                &g
            )
            .unwrap()
            .is_none()
        );
        assert_eq!(
            mode1_request_with_locomotion(
                still,
                Locomotion {
                    gait: 2,
                    physical_flags: 0x40
                },
                &c,
                |slot| slot == 38,
                &g
            )
            .unwrap()
            .unwrap()
            .slot,
            38
        );
    }
    #[test]
    fn same_clip_and_unready_64_preserve_channels_before_reference_work() {
        let g = globals();
        for (slot, flags) in [(3, 0), (64, 0)] {
            let mut c = channels(slot);
            c[0][8] = flags;
            assert!(
                mode1_request_with_locomotion(
                    state(),
                    Locomotion {
                        gait: 1,
                        physical_flags: 0
                    },
                    &c,
                    |_| true,
                    &g
                )
                .unwrap()
                .is_none()
            );
        }
        let mut c = channels(64);
        c[0][8] = 2;
        assert_eq!(
            mode1_request_with_locomotion(
                state(),
                Locomotion {
                    gait: 1,
                    physical_flags: 0
                },
                &c,
                |_| true,
                &g
            )
            .unwrap()
            .unwrap()
            .slot,
            3
        );
    }
}
