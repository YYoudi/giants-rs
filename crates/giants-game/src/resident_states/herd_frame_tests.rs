//! Installed CPU composition with supplied world admission, time and flat ground.
use super::*;
use crate::{
    herd_frame::{Frame, Services},
    scene_resources::SceneResources,
};

struct Source {
    session: Session,
    factory: crate::vimp_actor::Factory,
    globals: giants_formats::global_data::GlobalData,
    effects: Arc<giants_formats::effect_image::Image>,
    owner: Id,
}
fn source() -> Source {
    let root =
        std::path::PathBuf::from(std::env::var_os("GIANTS_NPC_ROOT").expect("GIANTS_NPC_ROOT"));
    let common = SceneResources::load(&root, "xx_permanentaliens.bin", "English").unwrap();
    let resources = SceneResources::load(&root, "w_Story2.bin", "English").unwrap();
    let herds = crate::herd_generator::Factory::load(&common).unwrap();
    let markers = crate::herd_markers::Factory::load(&common).unwrap();
    let effects = Arc::new(
        giants_formats::effect_image::parse(
            &std::fs::read(root.join("Bin/fx_world_dust.bin")).unwrap(),
        )
        .unwrap(),
    );
    let factory = crate::vimp_actor::Factory::load(&resources, &effects).unwrap();
    let globals =
        giants_formats::global_data::parse(&std::fs::read(root.join("Bin/GData.gbn")).unwrap())
            .unwrap();
    let world =
        giants_formats::world::parse_world(&std::fs::read(root.join("Bin/w_Story2.bin")).unwrap())
            .unwrap();
    let offset = world
        .placements
        .iter()
        .find(|p| p.object_type == 49)
        .unwrap()
        .source_offset;
    let mut session = Session::new(673);
    let mut owner = None;
    for p in &world.placements {
        if p.source_offset == offset {
            owner = Some(herds.place(&mut session, p).unwrap());
        } else if p.object_type == 679 {
            markers.place(&mut session, p).unwrap();
        }
    }
    Source {
        session,
        factory,
        globals,
        effects,
        owner: owner.unwrap(),
    }
}
struct Floor;
impl crate::herd_generator::Ground for Floor {
    fn height(&mut self, _: [f32; 3]) -> Result<f32, String> {
        Ok(7.25)
    }
}
fn ground(_: [f32; 2]) -> Result<crate::physical_ground::Ground, &'static str> {
    Ok(crate::physical_ground::Ground {
        height: 7.25,
        plane: [0., 0., 1., -7.25],
        key: [0, 5],
    })
}

#[test]
#[ignore = "requires GIANTS_NPC_ROOT original installation"]
fn source_herd_frame_walk_visits_births_and_preserves_marker_fallback() {
    let Source {
        mut session,
        factory,
        globals,
        effects,
        owner,
    } = source();
    assert_eq!(session.list(0).len(), 16);
    let mut random = crate::audio_parameters::Random(1);
    let mut stamp = crate::animation_epoch::Stamp::default();
    let mut contexts = crate::attached_effect_context::Pool::new(673, Some(effects));
    let mut births = Vec::new();
    for number in 0..3 {
        let dt = [1. / 60., 0.1, 0.25][number as usize];
        let population = session.list(0);
        assert_eq!(population.len(), if number == 0 { 16 } else { 30 });
        for id in population {
            if session.vimp(id).is_some() {
                session
                    .prepare_vimp_world_history(id, number, &factory)
                    .unwrap();
            } else {
                session.prepare_herd_world_history(id, number).unwrap();
            }
        }
        let mut walk = session.new_common_walk(64).unwrap();
        let mut visited = Vec::new();
        session
            .walk_common(&mut walk, &mut |session, id| {
                visited.push(id);
                if session.vimp(id).is_none() {
                    let state = u32::from(session.get(id).unwrap().seed.storage.state());
                    let mut finalization = crate::actor_environment::FinalizationServices {
                        actor: 1,
                        state,
                        slope_parameter: 0.,
                        physics: None,
                        skip_object_responses: false,
                        objects: &[],
                        ground,
                        platform: |p| {
                            crate::platform_queries::sample_prepared(
                                p,
                                &[],
                                crate::arithmetic::Precision::Double53,
                            )
                        },
                        seconds: |_| f64::from(dt),
                        release: |id: Option<u32>| {
                            if id.is_none() {
                                Ok(())
                            } else {
                                Err("unexpected footprint owner".into())
                            }
                        },
                        statistics: None,
                    };
                    let result = session.advance_herd_world_after_history(
                        id,
                        Frame {
                            number,
                            global_seconds: dt,
                            areas: &[],
                            controlled: Some(owner),
                        },
                        &mut Services {
                            factory: &factory,
                            globals: &globals,
                            random: &mut random,
                            ground: &mut Floor,
                            stamp: &mut stamp,
                            finalization: &mut finalization,
                        },
                    )?;
                    assert_eq!(result.outcome, crate::actor_world_tick::Outcome::Updated);
                    births.extend(result.born);
                    assert_eq!(session.herd_world_frame(id)?.next_frame, number + 1);
                    let affine = session.actor_current_affine(id)?;
                    assert_eq!(
                        [affine.matrix[3], affine.matrix[7], affine.matrix[11]],
                        session
                            .get(id)
                            .unwrap()
                            .construction_fields
                            .as_ref()
                            .unwrap()
                            .position
                    );
                } else {
                    let cold = !session.vimp(id).unwrap().initialized;
                    let old_history = session
                        .get(id)
                        .unwrap()
                        .construction_fields
                        .as_ref()
                        .unwrap()
                        .previous
                        .map(f32::to_bits);
                    let frame = crate::vimp_frame::Frame {
                        number,
                        controlled_actor: Some(owner),
                        tick: crate::vimp_wander::Tick {
                            global_delta: dt,
                            control_factor: 0.5,
                            skip_heading: false,
                            slow_areas: &[],
                            now: 2.,
                            noise_time: 0.,
                            noise_position: [50000., 50000., 0.],
                        },
                    };
                    let mut services = crate::vimp_frame::Services {
                        animation: crate::vimp_animation::Services {
                            blending: crate::animation_channels::Blending::Disabled,
                            contexts: &mut contexts,
                            random: &mut random,
                            stamp: &mut stamp,
                        },
                        ground: &mut ground,
                        recovery_sounds: None,
                    };
                    let result = if cold {
                        session.advance_vimp_newborn_world(
                            id,
                            frame,
                            &factory,
                            &globals,
                            &mut services,
                        )
                    } else {
                        session.advance_vimp_world_after_history(
                            id,
                            frame,
                            &factory,
                            &globals,
                            &mut services,
                        )
                    }?;
                    assert_eq!(result, crate::actor_world_tick::Outcome::UpdatedByPrimary);
                    if cold {
                        assert_eq!(number, 0);
                        assert_eq!(
                            session
                                .get(id)
                                .unwrap()
                                .construction_fields
                                .as_ref()
                                .unwrap()
                                .previous
                                .map(f32::to_bits),
                            old_history
                        );
                    }
                }
                Ok(crate::object_list_walk::Flow::Continue)
            })
            .unwrap();
        assert_eq!(visited.len(), 30);
        assert_eq!(visited, session.list(0));
        assert_eq!(births.len(), 14);
        assert_eq!(stamp.0, 30 * (number as u32 + 1));
    }
    session.clear();
    assert!(session.list(0).is_empty());
    assert!(session.get(owner).is_none());
    assert!(births.iter().all(|id| session.get(*id).is_none()));
}

struct LateFailure {
    calls: usize,
}

#[test]
#[ignore = "requires GIANTS_NPC_ROOT; real environment composition with a late required-platform failure"]
fn source_herd_frame_retains_real_environment_prefix_after_platform_failure() {
    let Source {
        mut session,
        factory,
        globals,
        owner,
        ..
    } = source();
    let mut random = crate::audio_parameters::Random(1);
    let mut stamp = crate::animation_epoch::Stamp::default();
    session.prepare_herd_world_history(owner, 0).unwrap();
    let mut finalization = crate::actor_environment::FinalizationServices {
        actor: 1,
        state: 17,
        slope_parameter: 0.,
        physics: None,
        skip_object_responses: false,
        objects: &[],
        ground,
        platform: |_| Err("required platform failed after terrain"),
        seconds: |_| 0.1,
        release: |id: Option<u32>| {
            if id.is_none() {
                Ok(())
            } else {
                Err("unexpected footprint".into())
            }
        },
        statistics: None,
    };
    let error = session
        .advance_herd_world_after_history(
            owner,
            Frame {
                number: 0,
                global_seconds: 0.1,
                areas: &[],
                controlled: Some(owner),
            },
            &mut Services {
                factory: &factory,
                globals: &globals,
                random: &mut random,
                ground: &mut Floor,
                stamp: &mut stamp,
                finalization: &mut finalization,
            },
        )
        .unwrap_err();
    assert!(error.contains("required platform failed"));
    let view = session.herd_world_frame(owner).unwrap();
    assert_eq!(view.finalization.fields.floor, 7.25);
    assert_eq!(view.finalization.fields.plane, [0., 0., 1., -7.25]);
    assert!(view.failure.is_some());
    assert_eq!(session.list(10).len(), 14);
    assert_eq!(session.herd_clock(owner).unwrap().steps, 1);
    assert_eq!(stamp.0, 1);
    assert_ne!(random.0, 1);
    assert!(finalization.statistics.unwrap().ground_queries > 0);
    assert!(session.prepare_herd_world_history(owner, 0).is_err());
}

#[test]
#[ignore = "requires GIANTS_NPC_ROOT original installation"]
fn source_herd_frame_skipped_primary_does_not_advance_birth_clock() {
    let Source {
        mut session,
        factory,
        globals,
        owner,
        ..
    } = source();
    let mut random = crate::audio_parameters::Random(1);
    let mut stamp = crate::animation_epoch::Stamp::default();
    for number in 0..2 {
        // Supplied wrapper-gate transition, not an acquired live sleep/wake producer.
        let fields = session
            .residents
            .get_mut(&owner.serial)
            .unwrap()
            .construction_fields
            .as_mut()
            .unwrap();
        if number == 0 {
            fields.configuration[14] |= 0x10;
        } else {
            fields.configuration[14] &= !0x10;
        }
        session.prepare_herd_world_history(owner, number).unwrap();
        let mut finalization = crate::actor_environment::FinalizationServices {
            actor: 1,
            state: 17,
            slope_parameter: 0.,
            physics: None,
            skip_object_responses: false,
            objects: &[],
            ground,
            platform: |p| {
                crate::platform_queries::sample_prepared(
                    p,
                    &[],
                    crate::arithmetic::Precision::Double53,
                )
            },
            seconds: |_| 0.1,
            release: |id: Option<u32>| {
                if id.is_none() {
                    Ok(())
                } else {
                    Err("unexpected footprint".into())
                }
            },
            statistics: None,
        };
        let result = session
            .advance_herd_world_after_history(
                owner,
                Frame {
                    number,
                    global_seconds: 0.1,
                    areas: &[],
                    controlled: Some(owner),
                },
                &mut Services {
                    factory: &factory,
                    globals: &globals,
                    random: &mut random,
                    ground: &mut Floor,
                    stamp: &mut stamp,
                    finalization: &mut finalization,
                },
            )
            .unwrap();
        assert_eq!(result.outcome, crate::actor_world_tick::Outcome::Updated);
        assert_eq!(
            session.herd_world_frame(owner).unwrap().next_frame,
            number + 1
        );
        assert_eq!(session.herd_clock(owner).unwrap().steps, number);
        assert_eq!(stamp.0, number as u32 + 1);
        if number == 0 {
            assert!(result.born.is_empty());
            assert_eq!(random.0, 1);
        } else {
            assert_eq!(result.born.len(), 14);
            assert_ne!(random.0, 1);
        }
    }
}
impl crate::actor_finalization::Services for LateFailure {
    fn release_footprint(&mut self, id: Option<u32>) -> Result<(), String> {
        assert!(id.is_none());
        Ok(())
    }
    fn environment(&mut self, state: &mut crate::actor_finalization::State) -> Result<(), String> {
        self.calls += 1;
        state.fields.floor = 123.;
        Err("required environment failed after write".into())
    }
}
#[test]
#[ignore = "requires GIANTS_NPC_ROOT original installation"]
fn source_herd_frame_late_failure_keeps_births_epoch_and_partial_environment() {
    let Source {
        mut session,
        factory,
        globals,
        owner,
        ..
    } = source();
    let mut random = crate::audio_parameters::Random(1);
    let mut stamp = crate::animation_epoch::Stamp::default();
    let mut failure = LateFailure { calls: 0 };
    assert!(
        session
            .prepare_herd_world_history(
                Id {
                    ticket: 999,
                    ..owner
                },
                0
            )
            .is_err()
    );
    session.prepare_herd_world_history(owner, 0).unwrap();
    let mut services = Services {
        factory: &factory,
        globals: &globals,
        random: &mut random,
        ground: &mut Floor,
        stamp: &mut stamp,
        finalization: &mut failure,
    };
    assert!(
        session
            .advance_herd_world_after_history(
                owner,
                Frame {
                    number: 1,
                    global_seconds: 0.1,
                    areas: &[],
                    controlled: Some(owner)
                },
                &mut services
            )
            .is_err()
    );
    assert!(
        session
            .advance_herd_world_after_history(
                owner,
                Frame {
                    number: 0,
                    global_seconds: f32::NAN,
                    areas: &[],
                    controlled: Some(owner)
                },
                &mut services
            )
            .is_err()
    );
    assert_eq!(services.random.0, 1);
    assert_eq!(services.stamp.0, 0);
    let mut walk = session.new_common_walk(64).unwrap();
    let error = session
        .walk_common(&mut walk, &mut |s, id| {
            s.advance_herd_world_after_history(
                id,
                Frame {
                    number: 0,
                    global_seconds: 0.1,
                    areas: &[],
                    controlled: Some(owner),
                },
                &mut services,
            )?;
            Ok(crate::object_list_walk::Flow::Continue)
        })
        .unwrap_err();
    assert!(error.contains("environment failed"));
    assert_eq!(
        session
            .herd_world_frame(owner)
            .unwrap()
            .finalization
            .fields
            .floor,
        123.
    );
    assert!(session.herd_world_frame(owner).unwrap().failure.is_some());
    assert_eq!(session.herd_clock(owner).unwrap().steps, 1);
    assert_eq!(session.list(10).len(), 14);
    let rng = services.random.0;
    assert_ne!(rng, 1);
    assert_eq!(services.stamp.0, 1);
    assert!(
        session
            .advance_herd_world_after_history(
                owner,
                Frame {
                    number: 0,
                    global_seconds: 0.1,
                    areas: &[],
                    controlled: Some(owner)
                },
                &mut services
            )
            .is_err()
    );
    assert_eq!(services.random.0, rng);
    assert!(session.prepare_herd_world_history(owner, 0).is_err());
    assert!(session.new_common_walk(64).is_err());
    assert_eq!(failure.calls, 1);
}

#[test]
#[ignore = "requires GIANTS_NPC_ROOT original installation"]
fn source_herd_frame_lost_control_stops_after_birth_without_fallback() {
    let Source {
        mut session,
        factory,
        globals,
        owner,
        ..
    } = source();
    let mut random = crate::audio_parameters::Random(1);
    let mut stamp = crate::animation_epoch::Stamp::default();
    let mut failure = LateFailure { calls: 0 };
    session.prepare_herd_world_history(owner, 0).unwrap();
    let result = session
        .advance_herd_world_after_history(
            owner,
            Frame {
                number: 0,
                global_seconds: 0.1,
                areas: &[],
                controlled: None,
            },
            &mut Services {
                factory: &factory,
                globals: &globals,
                random: &mut random,
                ground: &mut Floor,
                stamp: &mut stamp,
                finalization: &mut failure,
            },
        )
        .unwrap();
    assert_eq!(
        result.outcome,
        crate::actor_world_tick::Outcome::WorldAborted
    );
    assert_eq!(result.born.len(), 14);
    assert_eq!(stamp.0, 0);
    assert_eq!(failure.calls, 0);
    assert!(session.prepare_herd_world_history(owner, 1).is_err());
}
