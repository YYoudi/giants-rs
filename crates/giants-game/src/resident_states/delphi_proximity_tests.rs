//! Source-backed ownership of the explicitly selected proximity sequence.
use super::*;

#[test]
#[ignore = "Requires GIANTS_DELPHI_ROOT and optional GIANTS_DELPHI_EXECUTABLE; CPU component sequence with supplied clock/ground and no state102 actor, not live world admission"]
fn source_delphi_proximity_retains_counter_failure_and_refuses_profile_replay() {
    let root = std::path::PathBuf::from(std::env::var("GIANTS_DELPHI_ROOT").unwrap());
    let source =
        crate::scene_resources::SceneResources::load(&root, "w_Rstory1.bin", "English").unwrap();
    let globals =
        giants_formats::global_data::parse(&std::fs::read(root.join("Bin/GData.gbn")).unwrap())
            .unwrap();
    let weapons = giants_formats::weapons::parse_with_profile(
        &std::fs::read(root.join("Bin/weapondefs.bin")).unwrap(),
        giants_formats::weapons::Profile::Debug,
    )
    .unwrap();
    let factory = crate::delphi_special::Factory::load(&source, &globals, &weapons).unwrap();
    let mut session = Session::new(1328);
    let frame = crate::delphi_player::ColdFrame {
        position: [0., 0., 10.],
        current: [0., 0., 10.],
        previous: [0., 0., 10.],
        angles: [0.; 3],
    };
    let make = |s: &mut Session| {
        factory
            .construct_player_fields(s, frame, crate::animation_channels::Blending::Disabled)
            .unwrap()
            .owner
    };
    let owner = make(&mut session);
    session.initialize_delphi_physics(owner, &globals).unwrap();
    let executable = std::env::var_os("GIANTS_DELPHI_EXECUTABLE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| root.join("Giants.exe"));
    let resources = crate::delphi_physical::Resources::load_locomotion(
        &source,
        session.specification(owner).unwrap().clone(),
        Arc::new(
            crate::mecc_contact_materials::Catalog::from_prepared(&[], Default::default()).unwrap(),
        ),
        &executable,
    )
    .unwrap()
    .with_manual_rotation()
    .unwrap()
    .with_post_physical()
    .unwrap()
    .with_reserve()
    .unwrap()
    .with_primary_tail()
    .unwrap()
    .with_proximity_continuation()
    .unwrap();
    session
        .prepare_delphi_physical(owner, resources.clone())
        .unwrap();
    assert!(
        session
            .list(0)
            .into_iter()
            .all(|id| { session.get(id).unwrap().seed.storage.state() != 102 })
    );
    let mut contexts = crate::attached_effect_context::Pool::from_active_resources(1328, &source);
    let mut random = crate::audio_parameters::Random(1);
    let mut stamp = crate::animation_epoch::Stamp::default();
    let mut post = crate::delphi_post_physical::World::default();
    let environment = |number| crate::delphi_physical::Environment {
        number,
        actor_key: 1,
        globals: &globals,
        global_seconds: 1. / 60.,
        areas: &[],
        commands: [0; 8],
        launch_blocked: false,
        objects: &[],
    };
    let manual = crate::delphi_physical::ManualRotation {
        sensitivity: 1.,
        local_owner: true,
        paused: false,
    };
    let mut remaining = 0_f32;
    let mut stop_requests = Vec::new();
    let mut failed_snapshot = None;
    let mut failed_epoch = 0;
    for number in 1..=42 {
        // Explicit test sink: records requests, without claiming voice execution.
        let mut stop = |cue| {
            stop_requests.push(cue);
            Ok(())
        };
        let mut reserve = crate::delphi_reserve::EmptyContext;
        let items = crate::delphi_items::Context {
            reserve: crate::delphi_reserve::Context {
                post: crate::delphi_post_physical::Context {
                    world: &mut post,
                    view_mode: 0,
                    stop_sound: &mut stop,
                },
                recharge_inhibited: false,
                services: &mut reserve,
            },
            session_mode: 0,
            peer_ids: &[],
            local_peer: 0,
            owner_word: None,
        };
        let mut services = crate::actor_animation_phase::Services {
            contexts: &mut contexts,
            random: &mut random,
            stamp: &mut stamp,
            blending: crate::animation_channels::Blending::Disabled,
        };
        if number == 1 {
            assert!(
                session
                    .advance_delphi_tail_controlled(
                        owner,
                        environment(number),
                        manual,
                        items,
                        |_| Ok(crate::physical_ground::Ground {
                            plane: [0., 0., 1., 0.],
                            height: 0.,
                            key: [0, 0]
                        }),
                        &mut services,
                    )
                    .unwrap_err()
                    .contains("profile")
            );
            assert_eq!(session.delphi_physical(owner).unwrap().number, 0);
            // The mismatched entry ran no component; retry with the actual profile.
            continue;
        }
        let result = session.advance_delphi_proximity_controlled(
            owner,
            environment(number - 1),
            manual,
            crate::delphi_proximity::Context {
                items,
                // Supplied threshold; this fixture has no state102 candidate.
                squared_distance_limit: 0.,
            },
            |_| {
                Ok(crate::physical_ground::Ground {
                    plane: [0., 0., 1., 0.],
                    height: 0.,
                    key: [0, 0],
                })
            },
            &mut services,
        );
        if number <= 40 {
            let wide = f64::from(remaining) - f64::from(1_f32 / 60.);
            remaining = if wide < 0. { 0.5 } else { wide as f32 };
            let view = result.unwrap();
            assert_eq!(view.post_commands, Some([0; 8]));
            assert_eq!(
                crate::delphi_physical::word(
                    session.get(owner).unwrap().seed.storage.data().unwrap(),
                    0xfb0,
                ),
                remaining.to_bits()
            );
            if number == 40 {
                session
                    .residents
                    .get_mut(&owner.serial)
                    .unwrap()
                    .seed
                    .storage
                    .data_mut()
                    .unwrap()[0xfb0..0xfb4]
                    .copy_from_slice(&f32::NAN.to_bits().to_le_bytes());
            }
        } else {
            assert!(result.is_err());
            assert!(session.delphi_physical(owner).unwrap().failure.is_some());
            assert_eq!(session.delphi_physical(owner).unwrap().number, 40);
            let bytes = session.get(owner).unwrap().seed.storage.data().unwrap();
            if number == 41 {
                failed_snapshot = Some(bytes.to_vec());
                failed_epoch = stamp.0;
            } else {
                assert_eq!(Some(bytes), failed_snapshot.as_deref());
                assert_eq!(stamp.0, failed_epoch);
            }
        }
    }
    assert!(!stop_requests.is_empty());
    session.clear();
    assert!(session.get(owner).is_none());
    assert_eq!(session.counts().residents, 0);
    let second = make(&mut session);
    assert_ne!(second, owner);
    assert!(session.get(second).is_some());
    session.initialize_delphi_physics(second, &globals).unwrap();
    session.prepare_delphi_physical(second, resources).unwrap();
    let mut contexts = crate::attached_effect_context::Pool::from_active_resources(1328, &source);
    let mut stamp = crate::animation_epoch::Stamp::default();
    let mut post = crate::delphi_post_physical::World::default();
    let mut stops = Vec::new();
    let mut stop = |cue| {
        stops.push(cue);
        Ok(())
    };
    let mut reserve = crate::delphi_reserve::EmptyContext;
    let view = session
        .advance_delphi_proximity_controlled(
            second,
            environment(1),
            manual,
            crate::delphi_proximity::Context {
                items: crate::delphi_items::Context {
                    reserve: crate::delphi_reserve::Context {
                        post: crate::delphi_post_physical::Context {
                            world: &mut post,
                            view_mode: 0,
                            stop_sound: &mut stop,
                        },
                        recharge_inhibited: false,
                        services: &mut reserve,
                    },
                    session_mode: 0,
                    peer_ids: &[],
                    local_peer: 0,
                    owner_word: None,
                },
                squared_distance_limit: 0.,
            },
            |_| {
                Ok(crate::physical_ground::Ground {
                    plane: [0., 0., 1., 0.],
                    height: 0.,
                    key: [0, 0],
                })
            },
            &mut crate::actor_animation_phase::Services {
                contexts: &mut contexts,
                random: &mut random,
                stamp: &mut stamp,
                blending: crate::animation_channels::Blending::Disabled,
            },
        )
        .unwrap();
    assert_eq!(view.number, 1);
    assert_eq!(view.post_commands, Some([0; 8]));
    assert_eq!(
        crate::delphi_physical::word(
            session.get(second).unwrap().seed.storage.data().unwrap(),
            0xfb0,
        ),
        0.5_f32.to_bits()
    );
}
