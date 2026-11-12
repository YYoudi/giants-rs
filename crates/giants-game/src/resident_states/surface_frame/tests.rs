//! Source constructor/animation controls. Parent placement and its one common
//! movement command, clock and empty object/platform sets are supplied inputs.
use super::*;
use crate::{actor_fields::Fields, arithmetic::Precision, scene_resources::SceneResources};

#[test]
#[ignore = "Requires GIANTS_NPC_ROOT; full owned marker phases with supplied parent/clock/terrain, no ordinary awakening or graphics proof"]
fn source_surface_world_keeps_current_pose_and_latches_late_failure() {
    let root = std::path::PathBuf::from(std::env::var_os("GIANTS_NPC_ROOT").unwrap());
    let resources = Arc::new(SceneResources::load(&root, "w_Story2.bin", "English").unwrap());
    let globals =
        giants_formats::global_data::parse(&std::fs::read(root.join("Bin/GData.gbn")).unwrap())
            .unwrap();
    let flat = giants_formats::gti::Terrain {
        source_flags: 0,
        width: 5,
        height: 5,
        origin: [-2000.; 2],
        spacing: 1000.,
        height_bounds: [0.; 2],
        fallback_texture: String::new(),
        heights: vec![0.; 25],
        colors: vec![0; 25],
        tags: vec![5; 25],
        consumed_bytes: 0,
        opaque_tail: vec![],
    };
    let terrain =
        crate::terrain_queries::StaticTerrainQuery::new(&flat, 0.001, Precision::Double53).unwrap();
    let program = resources
        .classes
        .models
        .first_program(73)
        .unwrap()
        .program
        .as_ref()
        .unwrap();
    let seed =
        crate::state_storage::Seed::from_program_for(program, Profile::Debug, Precision::Double53)
            .unwrap();
    let bindings = resources
        .class_state_bindings_for(73, Profile::Debug)
        .unwrap();
    let specification =
        Arc::new(crate::specification::Prepared::load_registered(&resources, 73).unwrap());
    let states =
        Arc::new(crate::state_resources::Prepared::load(&resources, 73, Profile::Debug).unwrap());
    let selection = resources
        .classes
        .models
        .first_selection(219)
        .unwrap()
        .as_ref()
        .unwrap();
    let definition = resources
        .classes
        .models
        .specs
        .get(&selection.spec_id)
        .unwrap()
        .first()
        .unwrap();
    let image = Arc::new(
        resources
            .classes
            .models
            .specification_effects(definition)
            .unwrap()
            .clone(),
    );
    for (ticket, blending, fail) in [
        (9511, crate::animation_channels::Blending::Disabled, false),
        (9512, crate::animation_channels::Blending::Enabled, false),
        (9513, crate::animation_channels::Blending::Enabled, true),
    ] {
        let mut session = Session::new(ticket);
        let mut fields = Fields::load(&resources, 73, &specification)
            .unwrap()
            .placed(crate::collider_init::DeclaredPose {
                class: 73,
                position: [0., 0., 10.],
                yaw: 0.,
                pitch_roll: None,
            })
            .unwrap();
        fields.dispatch_context = 99;
        let source = session
            .construct(Construction {
                class: 73,
                seed: &seed,
                bindings: &bindings,
                flags: crate::object_lists::DirectFlags::project(73, program),
                specification: specification.clone(),
                state_resources: states.clone(),
                fields,
            })
            .unwrap();
        session.prepare_actor_affine(source).unwrap();
        session
            .prepare_actor_environment(source, &resources, &terrain)
            .unwrap();
        session.prepare_actor_finalization(source).unwrap();
        session.prepare_actor_motion(source).unwrap();
        session.begin_actor_frame_history(source).unwrap();
        session.begin_actor_motion_history(source, [0; 8]).unwrap();
        let mut factory = crate::primary_surface_marker::Factory::shared(resources.clone());
        let marker = factory.construct(&mut session, source).unwrap();
        session.prepare_surface_marker_world(marker).unwrap();
        assert!(session.prepare_surface_marker_world(marker).is_err());
        assert!(
            session
                .tick_surface_marker(marker, 0.1, &[], 1., blending, 0.1)
                .is_err()
        );
        let mut random = crate::audio_parameters::Random(1);
        let mut stamp = crate::animation_epoch::Stamp::default();
        let mut contexts = crate::attached_effect_context::Pool::new(ticket, Some(image.clone()));
        let mut modes = std::collections::BTreeSet::new();
        let mut marked = false;
        for step in 0..600 {
            if step == 48 {
                session
                    .advance_actor_common_motion(
                        source,
                        crate::resident_motion::Commands {
                            bytes: [0, 0, 32, 0, 0, 0, 0, 0],
                            context: 99,
                            target_heading: 0.,
                            target_position: [0.; 2],
                        },
                        crate::resident_motion::Time {
                            seconds: 1.,
                            areas: &[],
                            control_factor: 1.,
                            running_factor: 1.,
                            skip_heading: false,
                            controlled_actor: false,
                        },
                    )
                    .unwrap();
            }
            if step != 0 {
                session.begin_surface_marker_history(marker, step).unwrap();
                assert!(session.begin_surface_marker_history(marker, step).is_err());
            }
            let input = || Frame {
                number: step,
                global_seconds: 0.1,
                areas: &[],
                controlled: source,
                delay: globals.scalar(0x3d4).unwrap(),
                blending,
            };
            let mut environment = crate::actor_environment::FinalizationServices {
                actor: session.actor_collision_identity(marker).unwrap(),
                state: 48,
                slope_parameter: 100.,
                physics: None,
                skip_object_responses: false,
                objects: &[],
                ground: |point| {
                    crate::physical_ground::Ground::from_sample(terrain.sample(point)?, flat.width)
                },
                platform: |point| {
                    crate::platform_queries::sample_prepared(point, &[], Precision::Double53)
                },
                seconds: |_| 0.1,
                release: |footprint: Option<u32>| {
                    if footprint.is_none() {
                        Ok(())
                    } else {
                        Err("fixture has no footprints".into())
                    }
                },
                statistics: None,
            };
            struct LateFailure;
            impl crate::actor_finalization::Services for LateFailure {
                fn release_footprint(&mut self, _: Option<u32>) -> Result<(), String> {
                    Err("unexpected footprint".into())
                }
                fn environment(
                    &mut self,
                    _: &mut crate::actor_finalization::State,
                ) -> Result<(), String> {
                    Err("injected marker environment failure".into())
                }
            }
            let mut late_failure = LateFailure;
            let result = session.advance_surface_marker_world(
                marker,
                input(),
                &mut stamp,
                &mut contexts,
                &mut random,
                if fail {
                    &mut late_failure as &mut dyn crate::actor_finalization::Services
                } else {
                    &mut environment
                },
            );
            let after = session.get(marker).unwrap().channels;
            let saved_seed = random.0;
            assert!(
                session
                    .advance_surface_marker_world(
                        marker,
                        input(),
                        &mut stamp,
                        &mut contexts,
                        &mut random,
                        &mut environment
                    )
                    .is_err()
            );
            assert_eq!(session.get(marker).unwrap().channels, after);
            assert_eq!(random.0, saved_seed);
            if fail {
                assert!(
                    result
                        .unwrap_err()
                        .contains("injected marker environment failure")
                );
                assert!(
                    session
                        .surface_marker_frame(marker)
                        .unwrap()
                        .failure
                        .is_some()
                );
                assert!(session.begin_surface_marker_history(marker, 1).is_err());
                assert_eq!(
                    session.actor_world_tick(marker).unwrap().phase(),
                    crate::actor_world_tick::Phase::Failed
                );
                break;
            }
            let outcome = result.unwrap();
            let Some(SurfaceLink::Marker { mode, .. }) = session.surface_link(marker) else {
                panic!()
            };
            modes.insert(mode);
            if outcome == crate::actor_world_tick::Outcome::Inactive {
                assert_ne!(session.actor_world_flags(marker).unwrap() & 0x8000, 0);
                marked = true;
                break;
            }
            assert_eq!(outcome, crate::actor_world_tick::Outcome::Updated);
            let epoch = session
                .surface_marker_frame(marker)
                .unwrap()
                .animation_epoch
                .unwrap();
            session
                .prepare_actor_positions(marker, 0, epoch, false)
                .unwrap();
            assert!(
                !session
                    .actor_model_positions(marker, 0)
                    .unwrap()
                    .unwrap()
                    .is_empty()
            );
            session.actor_current_affine(marker).unwrap();
        }
        if !fail {
            assert!(marked);
            assert_eq!(modes, [0, 1, 2, 3].into_iter().collect());
        }
        assert!(
            session
                .incoming_actor_references(marker)
                .unwrap()
                .contains(&source)
        );
        assert!(
            session
                .incoming_actor_references(source)
                .unwrap()
                .contains(&marker)
        );
        assert!(
            Session::new(ticket + 100)
                .surface_marker_frame(marker)
                .is_err()
        );
        session.clear();
        assert!(session.surface_marker_frame(marker).is_err());
    }
}
