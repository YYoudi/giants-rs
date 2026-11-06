use super::*;
use crate::{actor_animation_phase::Services, kabuto_geometry_transition::*};

#[test]
#[ignore = "Requires GIANTS_KABUTO_ROOT; optional private controls and receipt; no window or sound"]
fn source_kabuto_transition_preserves_order_and_quarantines_partial_failure() {
    let root = std::path::PathBuf::from(std::env::var("GIANTS_KABUTO_ROOT").unwrap());
    let source = crate::world_source::resolve(&root, "Kstory1").unwrap();
    let menu = crate::mission_selection::MissionMenu::parse(
        &std::fs::read(root.join("Bin").join(source.catalog)).unwrap(),
        u32::MAX,
        true,
    )
    .unwrap();
    let packet =
        crate::mission_loading::prepare(&root, menu.request("Kstory1", 0).unwrap()).unwrap();
    let plan = packet.instance_plan(&root, "English").unwrap();
    let globals = Arc::new(
        giants_formats::global_data::parse(&std::fs::read(root.join("Bin/GData.gbn")).unwrap())
            .unwrap(),
    );
    let factory = crate::kabuto_initial::Factory::load(
        &plan,
        packet.world.as_ref().unwrap(),
        packet.mission.as_ref().unwrap(),
        globals,
    )
    .unwrap();
    let extra = crate::kabuto_auxiliary::ExtraFactory::load(plan.resources()).unwrap();
    let mut session = Session::new(950);
    let owner = factory
        .construct(
            &mut session,
            ColdFrame {
                position: [0., 0., 10.],
                current: [0.; 3],
                previous: [0.; 3],
                angles: [0.; 3],
            },
            Blending::Disabled,
            1,
        )
        .unwrap();
    let group = session.kabuto_group(owner).unwrap();
    let mut ids = std::iter::once(owner)
        .chain([group.first, group.second])
        .chain(group.effects)
        .chain([group.final_effect])
        .collect::<Vec<_>>();
    let mut contexts = crate::attached_effect_context::Pool::new(950, None);
    let mut random = crate::audio_parameters::Random(1);
    let mut stamp = crate::animation_epoch::Stamp(0);
    let controls = std::env::var_os("GIANTS_KABUTO_TRANSITION_CONTROLS")
        .map(|p| std::fs::read_to_string(p).unwrap());
    let mut receipt = String::new();
    if controls.is_none() {
        let mut sounds = Vec::new();
        let mut services = Services {
            contexts: &mut contexts,
            random: &mut random,
            stamp: &mut stamp,
            blending: Blending::Disabled,
        };
        for mode in [3, 0] {
            let view = session
                .advance_kabuto_geometry_transition(
                    owner,
                    &extra,
                    Frame {
                        mode,
                        global_seconds: 0.01,
                        areas: &[],
                        model: [0.; 4],
                        affine: None,
                        support: 0,
                        support_part: 0,
                        materials: None,
                    },
                    &mut services,
                    |s| {
                        sounds.push(s);
                        Ok(())
                    },
                )
                .unwrap();
            assert!(view.animation_processed);
            assert_eq!(view.creates, u32::from(mode == 3));
            assert_eq!(view.marks, u32::from(mode == 0));
        }
        assert_eq!(
            sounds
                .iter()
                .map(|s| (s.action, s.identifier))
                .collect::<Vec<_>>(),
            [
                (SoundAction::Start, 0x453),
                (SoundAction::Start, 0x452),
                (SoundAction::Stop, 0x453)
            ]
        );
        assert!(session.kabuto_group(owner).unwrap().extra.is_none());
        assert_eq!(session.counts().residents, 13);
        let owner_fields = session
            .get(owner)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap();
        assert_eq!(
            owner_fields.channels,
            session.get(owner).unwrap().channels.unwrap()
        );
        assert!(session.reference_cache_storage(owner).unwrap().sampled);
    }
    if let Some(controls) = controls {
        for line in controls.lines() {
            let words = line
                .split_whitespace()
                .map(|w| u32::from_str_radix(w, 16).unwrap())
                .collect::<Vec<_>>();
            assert!(matches!(words.len(), 47 | 48));
            let blending = if words.get(47) == Some(&1) {
                Blending::Enabled
            } else {
                Blending::Disabled
            };
            stamp.0 = words[2];
            let actor = session.residents.get_mut(&owner.serial).unwrap();
            actor.mecc_animation_stamp = words[3];
            actor.channels = Some(std::array::from_fn(|i| {
                std::array::from_fn(|j| words[11 + i * 9 + j])
            }));
            let payload = actor.seed.storage.data_mut().unwrap();
            for i in 0..3 {
                payload[4 + i * 4..8 + i * 4].copy_from_slice(&words[8 + i].to_le_bytes());
            }
            let mut sounds = Vec::new();
            let view = session
                .advance_kabuto_geometry_transition(
                    owner,
                    &extra,
                    Frame {
                        mode: words[0] as u8,
                        global_seconds: f32::from_bits(words[1]),
                        areas: &[],
                        model: std::array::from_fn(|i| f32::from_bits(words[4 + i])),
                        affine: None,
                        support: 0,
                        support_part: 0,
                        materials: None,
                    },
                    &mut Services {
                        contexts: &mut contexts,
                        random: &mut random,
                        stamp: &mut stamp,
                        blending,
                    },
                    |s| {
                        sounds.push(s);
                        Ok(())
                    },
                )
                .unwrap();
            if let Some(id) = session.kabuto_group(owner).unwrap().extra
                && !ids.contains(&id)
            {
                ids.push(id);
            }
            let ordinal = |id| ids.iter().position(|v| *v == id).unwrap() as u32 + 1;
            let actor = session.get(owner).unwrap();
            let mut output = vec![
                stamp.0,
                actor.mecc_animation_stamp,
                u32::from(view.animation_processed),
                view.selections,
                view.frame_queries,
                sounds.len() as u32,
            ];
            for s in sounds {
                output.extend([u32::from(s.action == SoundAction::Stop), s.identifier]);
            }
            output.extend(actor.channels.unwrap().into_iter().flatten());
            let matrices = session.reference_cache_storage(owner).unwrap();
            output.push(matrices.matrices.len() as u32);
            for m in matrices.matrices {
                output.push(m.flags);
                output.extend(m.values.map(f32::to_bits));
            }
            output.extend([
                session
                    .kabuto_group(owner)
                    .unwrap()
                    .extra
                    .map_or(0, ordinal),
                (ids.len() - 12) as u32,
            ]);
            for &id in &ids[12..] {
                let actor = session.get(id).unwrap();
                let fields = actor.construction_fields.as_ref().unwrap();
                output.extend([ordinal(id), fields.object_flags, fields.response_flags]);
                let attachment = actor.attachment.unwrap();
                output.extend([
                    ordinal(attachment.parent),
                    attachment.child_reference,
                    attachment.parent_reference,
                ]);
            }
            for list in 0..15 {
                let members = session.list(list);
                output.push(members.len() as u32);
                output.extend(members.into_iter().map(ordinal));
            }
            receipt.push_str(
                &output
                    .iter()
                    .map(|v| format!("{v:08x}"))
                    .collect::<Vec<_>>()
                    .join(" "),
            );
            receipt.push('\n');
        }
    }
    if let Some(path) = std::env::var_os("GIANTS_KABUTO_TRANSITION_RECEIPT") {
        std::fs::write(path, receipt).unwrap();
    }
    // The sound provider fails after the first externally visible request.
    // Earlier publication is retained, and no subsequent call can replay it.
    session
        .select_primary(
            owner,
            crate::animation_channels::Request {
                slot: 1,
                flags: 0x20,
                speed_source: 0,
                reference_speed: 0.,
                blend_seconds: 0.25,
            },
            Blending::Disabled,
        )
        .unwrap();
    let frame = || Frame {
        mode: 3,
        global_seconds: 0.01,
        areas: &[],
        model: [0.; 4],
        affine: None,
        support: 0,
        support_part: 0,
        materials: None,
    };
    let mut calls = 0;
    let mut services = Services {
        contexts: &mut contexts,
        random: &mut random,
        stamp: &mut stamp,
        blending: Blending::Disabled,
    };
    let failure = session
        .advance_kabuto_geometry_transition(owner, &extra, frame(), &mut services, |_| {
            calls += 1;
            Err("injected sound boundary failure".into())
        })
        .unwrap_err();
    assert_eq!(calls, 1);
    assert_eq!(
        session.get(owner).unwrap().seed.storage.data().unwrap()[0x228],
        3
    );
    assert_eq!(
        session
            .kabuto_group(owner)
            .unwrap()
            .geometry_transition_failure(),
        Some(failure.as_str())
    );
    assert!(
        session
            .advance_kabuto_geometry_transition(owner, &extra, frame(), &mut services, |_| {
                calls += 1;
                Ok(())
            })
            .is_err()
    );
    assert_eq!(calls, 1);
    assert!(session.mark_kabuto_extra(owner).is_err());
    assert!(extra.construct_for(&mut session, owner).is_err());
    assert!(session.kabuto_geometry_frame(owner).is_err());
    assert_eq!(session.kabuto_group(owner).unwrap().pending.len(), 4);
    session.clear();
    assert!(session.kabuto_events.is_none());
    assert_eq!(session.counts().residents, 0);
    assert!(
        session
            .advance_kabuto_geometry_transition(owner, &extra, frame(), &mut services, |_| Ok(()))
            .is_err()
    );
}
