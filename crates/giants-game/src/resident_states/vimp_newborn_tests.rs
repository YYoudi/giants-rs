//! Source-owned newborn entry; supplied world admission, clocks and terrain.
use super::*;

fn frame(number: u64, id: Id) -> crate::vimp_frame::Frame<'static> {
    crate::vimp_frame::Frame {
        number,
        controlled_actor: Some(id),
        tick: crate::vimp_wander::Tick {
            global_delta: [1. / 60., 0.1, 0.25][number as usize],
            control_factor: 0.5,
            skip_heading: false,
            slow_areas: &[],
            now: 2.,
            noise_time: 0.,
            noise_position: [50000., 50000., 0.],
        },
    }
}

fn flat_terrain() -> giants_formats::gti::Terrain {
    giants_formats::gti::Terrain {
        source_flags: 0,
        width: 5,
        height: 5,
        origin: [-200000.; 2],
        spacing: 100000.,
        height_bounds: [7.25; 2],
        fallback_texture: String::new(),
        heights: vec![7.25; 25],
        colors: vec![0; 25],
        tags: vec![5; 25],
        consumed_bytes: 0,
        opaque_tail: vec![],
    }
}

#[test]
#[ignore = "Requires GIANTS_NPC_ROOT; source newborn cohort and two prepared-history continuations; supplied admission/ground/time/control, no renderer"]
fn source_vimp_newborn_runs_first_body_and_keeps_constructor_history() {
    let (mut s, factory, globals, effects, children, mut random) =
        super::vimp_history_tests::source_births(false);
    let before = children
        .iter()
        .map(|id| {
            s.get(*id)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap()
                .previous
                .map(f32::to_bits)
        })
        .collect::<Vec<_>>();
    let mut stamp = crate::animation_epoch::Stamp::default();
    let mut contexts = crate::attached_effect_context::Pool::new(660, Some(effects));
    let terrain = flat_terrain();
    let query = crate::terrain_queries::StaticTerrainQuery::new(
        &terrain,
        0.00001,
        crate::arithmetic::Precision::Double53,
    )
    .unwrap();
    for number in 0..3 {
        if number != 0 {
            for id in &children {
                s.prepare_vimp_world_history(*id, number, &factory).unwrap();
            }
        }
        for (i, &id) in children.iter().enumerate() {
            let mut ground =
                |point| crate::physical_ground::Ground::from_sample(query.sample(point)?, 5);
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
            let result = if number == 0 {
                s.advance_vimp_newborn_world(
                    id,
                    frame(number, id),
                    &factory,
                    &globals,
                    &mut services,
                )
            } else {
                s.advance_vimp_world_after_history(
                    id,
                    frame(number, id),
                    &factory,
                    &globals,
                    &mut services,
                )
            };
            assert_eq!(
                result.unwrap(),
                crate::actor_world_tick::Outcome::UpdatedByPrimary
            );
            assert!(s.vimp(id).unwrap().initialized);
            assert!(s.vimp_physical(id).unwrap().animation_consumed);
            assert_eq!(s.vimp_frames(id), Some(number + 1));
            assert_eq!(s.get(id).unwrap().vimp_prepared_history, None);
            if number == 0 {
                assert_eq!(
                    s.get(id)
                        .unwrap()
                        .construction_fields
                        .as_ref()
                        .unwrap()
                        .previous
                        .map(f32::to_bits),
                    before[i]
                );
            }
            let rng = services.animation.random.0;
            let epoch = services.animation.stamp.0;
            assert!(
                s.advance_vimp_newborn_world(
                    id,
                    frame(number, id),
                    &factory,
                    &globals,
                    &mut services
                )
                .is_err()
            );
            assert_eq!(
                (services.animation.random.0, services.animation.stamp.0),
                (rng, epoch)
            );
        }
    }
    println!(
        "npc_newborn_source children={} callbacks={} supplied_world=true",
        children.len(),
        children.len() * 3
    );
    s.clear();
    assert_eq!(s.counts(), Counts::default());
}

#[test]
#[ignore = "Requires GIANTS_NPC_ROOT; newborn admission/replay and retained initialization after required FX-resource failure; no window or sound"]
fn source_vimp_newborn_refuses_wrong_entry_and_retains_late_failure() {
    let (mut s, factory, globals, _effects, children, mut random) =
        super::vimp_history_tests::source_births(false);
    let id = children[0];
    let history = s
        .get(id)
        .unwrap()
        .construction_fields
        .as_ref()
        .unwrap()
        .previous
        .map(f32::to_bits);
    let mut stamp = crate::animation_epoch::Stamp::default();
    let mut contexts = crate::attached_effect_context::Pool::new(660, None);
    let terrain = flat_terrain();
    let query = crate::terrain_queries::StaticTerrainQuery::new(
        &terrain,
        0.00001,
        crate::arithmetic::Precision::Double53,
    )
    .unwrap();
    let calls = std::cell::Cell::new(0);
    let mut ground = |point| {
        calls.set(calls.get() + 1);
        crate::physical_ground::Ground::from_sample(query.sample(point)?, 5)
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
    let rng = services.animation.random.0;
    assert!(
        Session::new(661)
            .advance_vimp_newborn_world(id, frame(0, id), &factory, &globals, &mut services)
            .is_err()
    );
    assert!(
        s.advance_vimp_newborn_world(id, frame(1, id), &factory, &globals, &mut services)
            .is_err()
    );
    assert!(s.prepare_vimp_world_history(id, 0, &factory).is_err());
    assert!(
        s.advance_vimp_world_after_history(id, frame(0, id), &factory, &globals, &mut services)
            .is_err()
    );
    assert!(
        s.advance_vimp_world_offline(id, frame(0, id), &factory, &globals, &mut services)
            .is_err()
    );
    let flags = s.actor_world_flags(id).unwrap();
    s.residents
        .get_mut(&id.serial)
        .unwrap()
        .construction_fields
        .as_mut()
        .unwrap()
        .object_flags |= 0xc000;
    assert!(
        s.advance_vimp_newborn_world(id, frame(0, id), &factory, &globals, &mut services)
            .is_err()
    );
    s.residents
        .get_mut(&id.serial)
        .unwrap()
        .construction_fields
        .as_mut()
        .unwrap()
        .object_flags = flags;
    assert_eq!(services.animation.random.0, rng);
    let error = s
        .advance_vimp_newborn_world(id, frame(0, id), &factory, &globals, &mut services)
        .unwrap_err();
    assert!(error.contains("effect namespace"), "{error}");
    assert!(s.vimp_physical(id).unwrap().physics_consumed);
    assert!(!s.vimp_physical(id).unwrap().animation_consumed);
    let completed_queries = calls.get();
    assert!(s.vimp(id).unwrap().initialized);
    assert_eq!(s.vimp(id).unwrap().mode, 1);
    assert_eq!(
        s.get(id)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap()
            .previous
            .map(f32::to_bits),
        history
    );
    assert_eq!(
        s.actor_world_tick(id).unwrap().phase(),
        crate::actor_world_tick::Phase::Failed
    );
    let advanced = services.animation.random.0;
    assert_ne!(advanced, rng);
    assert!(
        s.advance_vimp_newborn_world(id, frame(0, id), &factory, &globals, &mut services)
            .is_err()
    );
    assert!(
        s.advance_vimp_world_offline(id, frame(0, id), &factory, &globals, &mut services)
            .is_err()
    );
    assert!(s.prepare_vimp_world_history(id, 0, &factory).is_err());
    assert_eq!(services.animation.random.0, advanced);
    let orphan = factory.birth(&mut s).unwrap();
    assert!(
        s.advance_vimp_newborn_world(orphan, frame(0, orphan), &factory, &globals, &mut services)
            .is_err()
    );
    assert_eq!(services.animation.random.0, advanced);
    assert_eq!(calls.get(), completed_queries);
    s.clear();
    assert_eq!(s.counts(), Counts::default());
}
