use super::*;
use bevy::ecs::world::CommandQueue;

#[test]
#[ignore = "requires GIANTS_BUILDING_ROOT; ordinary pub runtime with actual map neighbours/platform candidates and supplied clock; no GPU/Windows-input proof"]
fn installed_normal_pub_consumes_shared_clock_and_persistent_collision_prefixes() {
    let root = std::path::PathBuf::from(std::env::var_os("GIANTS_BUILDING_ROOT").unwrap());
    let menu = giants_game::mission_selection::MissionMenu::parse(
        &std::fs::read(root.join("Bin/w_Story1.bin")).unwrap(),
        0,
        true,
    )
    .unwrap();
    let packet = Arc::new(
        giants_game::mission_loading::prepare(&root, menu.request("Story1", 0).unwrap()).unwrap(),
    );
    let asset = Arc::new(packet.world_asset(&root).unwrap());
    let plan = packet.instance_plan(&root, "English").unwrap();
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
    let mut payload = super::tests::payload();
    payload.parameters = packet.slot_parameters(&root);
    payload.player = Ok(player);
    payload.world = Ok(asset.clone());
    payload.packet = packet;
    payload.models = Arc::new(
        giants_game::static_geometry::Scene::prepare_normal_initial_with_terrain(
            &plan,
            &Default::default(),
            &asset.terrain,
        ),
    );
    payload.plan = plan;
    let mut ecs = World::new();
    let mut queue = CommandQueue::default();
    let (scene_root, actors, blocked, residents, bindings) =
        spawn(&mut Commands::new(&mut queue, &ecs), 424, &mut payload);
    queue.apply(&mut ecs);
    let mut runtime = Runtime {
        active: Some(Active {
            ticket: 424,
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
    runtime.sample_world_clock(424, 1000).unwrap();
    runtime
        .initialize_normal_buildings(424, &mut viewer, &mut vec![], &mut vec![])
        .unwrap();
    let initial = runtime
        .advance_normal_building_bodies(424, &mut viewer)
        .unwrap();
    assert!(
        !initial.is_empty(),
        "ordinary map must exercise authored pub"
    );
    let owners = initial
        .iter()
        .map(|p| {
            let active = runtime.active.as_ref().unwrap();
            let instance = active
                .payload
                .models
                .instances
                .iter()
                .find(|i| i.key == p.key)
                .expect("published pose must address an actual rigid renderer instance");
            assert_eq!(instance.class, 53);
            let index = if p.key.0 {
                active
                    .payload
                    .plan
                    .world_entries
                    .iter()
                    .position(|e| e.declaration.source_offset == p.key.1)
            } else {
                active
                    .payload
                    .plan
                    .entries
                    .iter()
                    .position(|e| e.serialized.source_offset == p.key.1)
            }
            .expect("render offset must resolve to its authored declaration");
            assert_ne!(
                index, p.key.1,
                "fixture must distinguish index and file offset"
            );
            let key = giants_game::declared_residents::Key {
                scope: if p.key.0 {
                    giants_game::declared_residents::Scope::World
                } else {
                    giants_game::declared_residents::Scope::Mission
                },
                index,
            };
            let owner = runtime
                .active
                .as_ref()
                .unwrap()
                .resident_states
                .declaration(key)
                .unwrap();
            (owner, (p.key.0, index))
        })
        .collect::<Vec<_>>();
    for number in 1..=30 {
        runtime.sample_world_clock(424, 1000 + number * 16).unwrap();
        let poses = runtime
            .advance_normal_building_bodies(424, &mut viewer)
            .unwrap();
        let duplicate = runtime
            .advance_normal_building_bodies(424, &mut viewer)
            .unwrap();
        assert_eq!(poses.len(), initial.len());
        assert_eq!(duplicate.len(), poses.len());
        // Rebuild the ordinary transient suffix with supplied contact input;
        // persistent pub entries must survive the same truncation boundary.
        viewer.set_mission_contacts(424, vec![]).unwrap();
        for (pose, (owner, declaration)) in poses.iter().zip(&owners) {
            let session = &runtime.active.as_ref().unwrap().resident_states;
            let body = session.ordinary_building_body(*owner).unwrap();
            assert_eq!(body.steps(), u64::from(number) + 1);
            assert_eq!(
                pose.matrix.map(f32::to_bits),
                body.affine().unwrap().matrix.map(f32::to_bits)
            );
            assert_eq!(
                pose.position.map(f32::to_bits),
                body.fields().current.map(f32::to_bits)
            );
            let object = viewer
                .mission_colliders()
                .iter()
                .find(|o| o.fields.id == body.collision_identity())
                .unwrap();
            assert_eq!(object.fields.bounds.min, body.fields().bounds[..3]);
            assert_eq!(
                viewer
                    .normal_projectile_registry()
                    .declarations
                    .iter()
                    .filter(|d| {
                        d.object.id == body.collision_identity()
                            && d.target
                                == giants_game::enemy_projectile_scene::Target::Declaration(
                                    *declaration,
                                )
                    })
                    .count(),
                1
            );
            assert!(
                viewer
                    .buildings
                    .probe_requires_contact(body.collision_identity())
            );
        }
    }
    assert!(
        runtime
            .advance_normal_building_bodies(425, &mut viewer)
            .is_err()
    );
    runtime.active.as_mut().unwrap().resident_states.clear();
    for (owner, _) in owners {
        assert!(
            runtime
                .active
                .as_ref()
                .unwrap()
                .resident_states
                .ordinary_building_body(owner)
                .is_err()
        );
    }
}
