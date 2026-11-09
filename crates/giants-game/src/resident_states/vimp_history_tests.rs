//! Installed-resource phase controls, with supplied time/terrain failure.
use super::*;

fn source() -> (
    Session,
    crate::vimp_actor::Factory,
    giants_formats::global_data::GlobalData,
    Arc<giants_formats::effect_image::Image>,
    Vec<Id>,
) {
    let root =
        std::path::PathBuf::from(std::env::var_os("GIANTS_NPC_ROOT").expect("GIANTS_NPC_ROOT"));
    let resources =
        crate::scene_resources::SceneResources::load(&root, "w_Story2.bin", "English").unwrap();
    let common =
        crate::scene_resources::SceneResources::load(&root, "xx_permanentaliens.bin", "English")
            .unwrap();
    let herds = crate::herd_generator::Factory::load(&common).unwrap();
    let markers = crate::herd_markers::Factory::load(&common).unwrap();
    let effects = Arc::new(
        giants_formats::effect_image::parse(
            &std::fs::read(root.join("Bin/fx_world_dust.bin")).unwrap(),
        )
        .unwrap(),
    );
    let globals =
        giants_formats::global_data::parse(&std::fs::read(root.join("Bin/GData.gbn")).unwrap())
            .unwrap();
    let factory = crate::vimp_actor::Factory::load(&resources, &effects).unwrap();
    let world =
        giants_formats::world::parse_world(&std::fs::read(root.join("Bin/w_Story2.bin")).unwrap())
            .unwrap();
    let mut session = Session::new(660);
    let parent = world
        .placements
        .iter()
        .find(|p| p.object_type == 49)
        .unwrap()
        .source_offset;
    let mut owner = None;
    for p in &world.placements {
        if p.source_offset == parent {
            owner = Some(herds.place(&mut session, p).unwrap());
        } else if p.object_type == 679 {
            markers.place(&mut session, p).unwrap();
        }
    }
    let mut random = crate::audio_parameters::Random(1);
    struct Floor;
    impl crate::herd_generator::Ground for Floor {
        fn height(&mut self, _: [f32; 3]) -> Result<f32, String> {
            Ok(7.25)
        }
    }
    let children = session
        .advance_herd_offline(
            owner.unwrap(),
            crate::herd_generator::Tick {
                step: 0,
                global_delta: 0.25,
                slow_areas: &[],
            },
            &factory,
            &globals,
            &mut random,
            &mut Floor,
        )
        .unwrap();
    assert!(children.len() >= 2);
    for child in &children {
        session
            .initialize_vimp_offline(*child, &globals, &mut random)
            .unwrap();
    }
    (session, factory, globals, effects, children)
}

fn frame(number: u64, controlled_actor: Id) -> crate::vimp_frame::Frame<'static> {
    crate::vimp_frame::Frame {
        number,
        controlled_actor: Some(controlled_actor),
        tick: crate::vimp_wander::Tick {
            global_delta: 1. / 60.,
            control_factor: 1.,
            skip_heading: false,
            slow_areas: &[],
            now: 2.,
            noise_time: 0.,
            noise_position: [0.; 3],
        },
    }
}

#[test]
#[ignore = "Requires GIANTS_NPC_ROOT; source-owned Vimp split history and injected ground failure; no live world or audio"]
fn source_vimp_world_history_is_separate_and_retains_late_failure() {
    let (mut s, factory, globals, effects, children) = source();
    let (a, b) = (children[0], children[1]);
    let original = s
        .get(a)
        .unwrap()
        .construction_fields
        .as_ref()
        .unwrap()
        .position;
    s.residents
        .get_mut(&a.serial)
        .unwrap()
        .construction_fields
        .as_mut()
        .unwrap()
        .current = [11., 12., 13.];
    let mut random = crate::audio_parameters::Random(1);
    let mut stamp = crate::animation_epoch::Stamp::default();
    let mut contexts = crate::attached_effect_context::Pool::new(660, Some(effects));
    let mut queries = 0;
    let mut ground = |_: [f32; 2]| {
        queries += 1;
        Err("injected split-history terrain failure")
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
    assert!(
        s.advance_vimp_world_after_history(a, frame(0, a), &factory, &globals, &mut services)
            .is_err()
    );
    assert!(s.prepare_vimp_world_history(a, 1, &factory).is_err());
    s.prepare_vimp_world_history(a, 0, &factory).unwrap();
    s.prepare_vimp_world_history(b, 0, &factory).unwrap();
    let header = s.get(a).unwrap().construction_fields.as_ref().unwrap();
    assert_eq!(header.previous, [11., 12., 13.]);
    assert_eq!(header.current, original);
    assert_eq!(services.animation.random.0, 1);
    assert_eq!(services.animation.stamp.0, 0);
    assert!(s.prepare_vimp_world_history(a, 0, &factory).is_err());
    assert!(
        s.advance_vimp_world_offline(a, frame(0, a), &factory, &globals, &mut services)
            .is_err()
    );
    // Model another callback changing the live position after the history pass.
    s.residents
        .get_mut(&a.serial)
        .unwrap()
        .construction_fields
        .as_mut()
        .unwrap()
        .position[0] += 0.125;
    let error = s
        .advance_vimp_world_after_history(a, frame(0, a), &factory, &globals, &mut services)
        .unwrap_err();
    assert!(
        error.contains("injected split-history terrain failure"),
        "{error}"
    );
    let header = s.get(a).unwrap().construction_fields.as_ref().unwrap();
    assert_eq!(header.previous, [11., 12., 13.]);
    assert_eq!(header.current, original);
    assert_eq!(
        s.actor_world_tick(a).unwrap().phase(),
        crate::actor_world_tick::Phase::Failed
    );
    assert_eq!(s.get(a).unwrap().vimp_prepared_history, None);
    assert_eq!(s.get(b).unwrap().vimp_prepared_history, Some(0));
    let rng = services.animation.random.0;
    assert!(
        s.advance_vimp_world_after_history(a, frame(0, a), &factory, &globals, &mut services)
            .is_err()
    );
    assert!(s.prepare_vimp_world_history(a, 0, &factory).is_err());
    assert_eq!(services.animation.random.0, rng);
    assert_eq!(queries, 1);
    s.clear();
    assert_eq!(s.counts(), Counts::default());
}

#[test]
#[ignore = "Requires GIANTS_NPC_ROOT; source-owned Vimp history preflight and inactive refusal; no window or audio"]
fn source_vimp_world_history_keeps_preflight_and_foreign_refusals() {
    let (mut s, factory, globals, effects, children) = source();
    let a = children[0];
    assert!(
        Session::new(661)
            .prepare_vimp_world_history(a, 0, &factory)
            .is_err()
    );
    let flags = s.actor_world_flags(a).unwrap();
    s.residents
        .get_mut(&a.serial)
        .unwrap()
        .construction_fields
        .as_mut()
        .unwrap()
        .object_flags |= 0xc000;
    let current = s
        .get(a)
        .unwrap()
        .construction_fields
        .as_ref()
        .unwrap()
        .current;
    assert!(s.prepare_vimp_world_history(a, 0, &factory).is_err());
    assert_eq!(
        s.get(a)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap()
            .current,
        current
    );
    s.residents
        .get_mut(&a.serial)
        .unwrap()
        .construction_fields
        .as_mut()
        .unwrap()
        .object_flags = flags;
    // Explicit supplied hurt entry: a real sound-stop provider is absent.
    s.residents
        .get_mut(&a.serial)
        .unwrap()
        .vimp
        .as_mut()
        .unwrap()
        .mode = 2;
    s.prepare_vimp_world_history(a, 0, &factory).unwrap();
    let mut random = crate::audio_parameters::Random(1);
    let mut stamp = crate::animation_epoch::Stamp::default();
    let mut contexts = crate::attached_effect_context::Pool::new(660, Some(effects));
    let mut ground = |_| panic!("preflight must not query terrain");
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
    assert!(
        s.advance_vimp_world_after_history(a, frame(0, a), &factory, &globals, &mut services)
            .unwrap_err()
            .contains("sound-stop")
    );
    assert_eq!(s.get(a).unwrap().vimp_prepared_history, Some(0));
    assert_eq!(services.animation.random.0, 1);
    assert_eq!(services.animation.stamp.0, 0);
    assert!(s.prepare_vimp_world_history(a, 0, &factory).is_err());
}

#[test]
#[ignore = "Requires GIANTS_NPC_ROOT; Vimp split/combined CPU prefix with supplied flat ground/time/RNG; no full world or renderer"]
fn source_vimp_world_history_matches_existing_completed_prefix() {
    let (mut a, fa, ga, ea, ca) = source();
    let (mut b, fb, gb, eb, cb) = source();
    let mut ra = crate::audio_parameters::Random(1);
    let mut rb = crate::audio_parameters::Random(1);
    let mut sa = crate::animation_epoch::Stamp::default();
    let mut sb = crate::animation_epoch::Stamp::default();
    let mut pa = crate::attached_effect_context::Pool::new(660, Some(ea));
    let mut pb = crate::attached_effect_context::Pool::new(660, Some(eb));
    let terrain = giants_formats::gti::Terrain {
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
    };
    let query = crate::terrain_queries::StaticTerrainQuery::new(
        &terrain,
        0.00001,
        crate::arithmetic::Precision::Double53,
    )
    .unwrap();
    for number in 0..8 {
        b.prepare_vimp_world_history(cb[0], number, &fb).unwrap();
        for (session, factory, globals, contexts, random, stamp, id, prepared) in [
            (&mut a, &fa, &ga, &mut pa, &mut ra, &mut sa, ca[0], false),
            (&mut b, &fb, &gb, &mut pb, &mut rb, &mut sb, cb[0], true),
        ] {
            let mut ground =
                |point| crate::physical_ground::Ground::from_sample(query.sample(point)?, 5);
            let mut services = crate::vimp_frame::Services {
                animation: crate::vimp_animation::Services {
                    blending: crate::animation_channels::Blending::Disabled,
                    contexts,
                    random,
                    stamp,
                },
                ground: &mut ground,
                recovery_sounds: None,
            };
            let result = if prepared {
                session.advance_vimp_world_after_history(
                    id,
                    frame(number, id),
                    factory,
                    globals,
                    &mut services,
                )
            } else {
                session.advance_vimp_world_offline(
                    id,
                    frame(number, id),
                    factory,
                    globals,
                    &mut services,
                )
            };
            assert_eq!(
                result.unwrap(),
                crate::actor_world_tick::Outcome::UpdatedByPrimary
            );
        }
        let ha = a.get(ca[0]).unwrap().construction_fields.as_ref().unwrap();
        let hb = b.get(cb[0]).unwrap().construction_fields.as_ref().unwrap();
        assert_eq!(ha.position.map(f32::to_bits), hb.position.map(f32::to_bits));
        assert_eq!(ha.current.map(f32::to_bits), hb.current.map(f32::to_bits));
        assert_eq!(ha.previous.map(f32::to_bits), hb.previous.map(f32::to_bits));
        assert_eq!(ha.configuration, hb.configuration);
        assert_eq!(ha.channels, hb.channels);
        assert_eq!(
            a.get(ca[0]).unwrap().seed.storage.data(),
            b.get(cb[0]).unwrap().seed.storage.data()
        );
        assert_eq!((ra.0, sa.0), (rb.0, sb.0));
        assert_eq!(
            (a.vimp_frames(ca[0]), b.vimp_frames(cb[0])),
            (Some(number + 1), Some(number + 1))
        );
    }
}
