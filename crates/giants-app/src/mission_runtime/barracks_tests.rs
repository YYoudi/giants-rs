//! Installed ordinary-owner composition. Contacts/damage/time are supplied;
//! device sinks, GPU rendering and Windows input require separate acceptance.
use super::*;
use bevy::ecs::world::CommandQueue;

#[test]
#[ignore = "requires GIANTS_BUILDING_ROOT; canonical normal runtime, supplied hits/time, actual geometry/FX/audio resources; no device/GPU/OS proof"]
fn installed_normal_barracks_hits_update_owned_health_geometry_and_retire_effects() {
    let root = std::path::PathBuf::from(std::env::var_os("GIANTS_BUILDING_ROOT").unwrap());
    let menu = giants_game::mission_selection::MissionMenu::parse(
        &std::fs::read(root.join("Bin/w_Story2.bin")).unwrap(),
        0,
        true,
    )
    .unwrap();
    let packet = Arc::new(
        giants_game::mission_loading::prepare(&root, menu.request("Story2", 1).unwrap()).unwrap(),
    );
    let asset = Arc::new(packet.world_asset(&root).unwrap());
    let plan = packet.instance_plan(&root, "English").unwrap();
    let targets: Vec<_> = plan
        .entries
        .iter()
        .enumerate()
        .filter(|(_, p)| matches!(p.serialized.object_type, 1060 | 1226))
        .map(|(index, p)| {
            (
                giants_game::declared_residents::Key {
                    scope: giants_game::declared_residents::Scope::Mission,
                    index,
                },
                u32::try_from(p.index).unwrap() + 2,
                p.serialized.position,
            )
        })
        .collect();
    assert_eq!(targets.len(), 2);
    let player = Arc::new(
        giants_game::player_initial::Prepared::load(
            &root,
            &plan,
            packet.world.as_ref().unwrap(),
            packet.mission.as_ref().unwrap(),
        )
        .unwrap(),
    );
    let mut viewer =
        crate::mecc_dev_scene::State::load_mission(&root, &asset, &packet, &player).unwrap();
    for (_, object, _) in &targets {
        assert!(
            viewer.buildings.is_barracks(*object),
            "barracks damage/effect admission missing"
        );
        assert!(
            viewer.buildings.probe_requires_contact(*object),
            "actual player collision owner missing"
        );
    }
    let mut payload = super::tests::payload();
    payload.parameters = packet.slot_parameters(&root);
    payload.player = Ok(player);
    payload.world = Ok(asset.clone());
    payload.packet = packet;
    payload.plan = plan;
    let mut ecs = World::new();
    let unrelated = ecs.spawn_empty().id();
    let mut queue = CommandQueue::default();
    let (scene_root, actors, blocked, residents, bindings) =
        spawn(&mut Commands::new(&mut queue, &ecs), 414, &mut payload);
    queue.apply(&mut ecs);
    let mut runtime = Runtime {
        active: Some(Active {
            ticket: 414,
            root: scene_root,
            actors,
            resident_states: residents,
            blocked,
            payload,
            start: StartState {
                intent: LaunchIntent::SavedMission,
                progress: Progress::load(None, None).unwrap(),
            },
            reported: false,
            pickup: None,
            variant_random: giants_game::audio_parameters::Random(17),
            cinematic_epoch: giants_game::animation_epoch::Stamp::default(),
            audio_registry: None,
            bindings,
            controlled_mecc_owner: None,
            owned_mecc: None,
            landing: None,
            campaign: None,
        }),
        ..Default::default()
    };
    runtime.sample_world_clock(414, 1000).unwrap();
    let frame = viewer
        .step(1. / 120., Vec2::ZERO, &asset.terrain, false, false)
        .unwrap();
    runtime
        .publish_controlled_mecc_motion(414, 1, frame)
        .unwrap();
    let player = runtime.active.as_ref().unwrap().player_owner().unwrap();
    let mut events = Vec::new();
    let mut updates = Vec::new();
    runtime
        .initialize_normal_buildings(414, &mut viewer, &mut events, &mut updates)
        .unwrap();
    assert_eq!((events.len(), updates.len()), (2, 2));
    runtime
        .initialize_normal_buildings(414, &mut viewer, &mut events, &mut updates)
        .unwrap();
    assert_eq!((events.len(), updates.len()), (2, 2));
    for event in &events {
        let crate::mission_audio::Event::ActorSound { owner, sound, .. } = event else {
            panic!("initial sound lost its actual owner");
        };
        assert!(runtime.actor_audio_position(414, *owner).is_some());
        assert_eq!(
            runtime
                .actor_sound_route(414, *owner, giants_game::actor_sound::Kind::Owner, true)
                .unwrap(),
            Some(giants_game::actor_sound::Route::Attached(Some(*owner)))
        );
        assert!(runtime.resolve_actor_sound(414, *sound).unwrap().is_some());
    }
    let camera = viewer.subjective_inputs().0.position;
    let mut milliseconds = 1000;
    let mut absent = std::collections::BTreeSet::new();
    for (key, object, point) in targets {
        let id = runtime
            .active
            .as_ref()
            .unwrap()
            .resident_states
            .declaration(key)
            .unwrap();
        let limit = runtime
            .active
            .as_ref()
            .unwrap()
            .resident_states
            .barracks_state(id)
            .unwrap()
            .limit;
        let mut shots = 0;
        loop {
            let result = runtime
                .hit_normal_building(
                    414,
                    &mut viewer,
                    giants_game::mecc_pistol::WorldHit {
                        object,
                        serial: shots + 1,
                        older: point,
                        motion: giants_game::damage_impulse::Motion {
                            current: point,
                            previous: point,
                        },
                        point,
                        damage: 25.,
                    },
                    camera,
                )
                .unwrap()
                .unwrap();
            shots += 1;
            let actor = runtime
                .active
                .as_ref()
                .unwrap()
                .resident_states
                .get(id)
                .unwrap();
            let state = runtime
                .active
                .as_ref()
                .unwrap()
                .resident_states
                .barracks_state(id)
                .unwrap();
            assert_eq!(state.target, Some(player));
            assert_eq!(state.damage, shots as f32 * 25.);
            assert_eq!(state.phase, 2);
            let retired = actor.construction_fields.as_ref().unwrap().object_flags & 0x8000 != 0;
            for event in &result.sounds {
                match event {
                    crate::mission_audio::Event::ActorSound { owner, sound, .. } => {
                        assert_eq!(*owner, id);
                        let prepared = runtime.resolve_actor_sound(414, *sound).unwrap();
                        if *sound == 0x67b {
                            assert!(prepared.is_none());
                            absent.insert(*sound);
                        } else {
                            assert!(prepared.is_some());
                        }
                    }
                    crate::mission_audio::Event::NpcPosition { sound, .. } => {
                        let prepared = runtime.resolve_actor_sound(414, *sound).unwrap();
                        if *sound == 0x67c {
                            assert!(prepared.is_none());
                            absent.insert(*sound);
                        } else {
                            assert!(prepared.is_some());
                        }
                    }
                    _ => panic!("unexpected barracks audio route"),
                }
            }
            let projectile = viewer
                .projectile_objects
                .iter()
                .find(|p| p.id == object)
                .unwrap();
            assert_eq!(projectile.active & 0x8000 != 0, retired);
            if !retired {
                assert_eq!(
                    projectile.mesh.static_world_triangles().unwrap(),
                    result.triangles
                );
            }
            if result.physical {
                assert_eq!(
                    viewer
                        .mission_colliders()
                        .iter()
                        .find(|p| p.fields.id == object)
                        .unwrap()
                        .triangles
                        .is_empty(),
                    retired
                );
            }
            if retired {
                assert!(result.update.geometry.is_none());
                assert!(runtime.actor_audio_position(414, id).is_none());
                assert!(viewer.buildings.effects.active() > 0);
                break;
            }
            assert!(result.update.geometry.is_some());
            assert!(shots < 100);
        }
        assert_eq!(shots, (limit / 25.).ceil() as u32);
        for _ in 0..2000 {
            if viewer.buildings.effects.active() == 0 {
                break;
            }
            milliseconds += 17;
            runtime.sample_world_clock(414, milliseconds).unwrap();
            runtime
                .advance_normal_building_effects(414, &mut viewer.buildings.effects)
                .unwrap();
        }
        assert_eq!(viewer.buildings.effects.active(), 0);
        assert_eq!(
            viewer.buildings.effects.births,
            viewer.buildings.effects.retired
        );
        println!(
            "normal_barracks_owner_control object={object} shots={shots} births={} retired={} actual_source_identity=true supplied_contacts_damage_time=true gpu=false device=false windows_input=false",
            viewer.buildings.effects.births, viewer.buildings.effects.retired
        );
    }
    assert_eq!(absent, std::collections::BTreeSet::from([0x67b, 0x67c]));
    let active = runtime.active.take().unwrap();
    ecs.despawn(active.root);
    drop(active);
    assert!(ecs.get_entity(unrelated).is_ok());
}
