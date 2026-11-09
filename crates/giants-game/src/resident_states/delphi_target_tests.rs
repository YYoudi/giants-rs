use super::*;

#[test]
#[ignore = "Requires GIANTS_DELPHI_ROOT; silent owned target composition, no live controller"]
fn source_delphi_candidates_use_current_geometry_and_preserve_target_payload() {
    let root = std::path::PathBuf::from(std::env::var("GIANTS_DELPHI_ROOT").unwrap());
    let resources =
        crate::scene_resources::SceneResources::load(&root, "w_Rstory1.bin", "English").unwrap();
    let globals =
        giants_formats::global_data::parse(&std::fs::read(root.join("Bin/GData.gbn")).unwrap())
            .unwrap();
    let weapons = giants_formats::weapons::parse_with_profile(
        &std::fs::read(root.join("Bin/weapondefs.bin")).unwrap(),
        giants_formats::weapons::Profile::Debug,
    )
    .unwrap();
    let definitions = crate::projectile_definition::DebugDefinitions::parse(
        &std::fs::read(root.join("Bin/weapondefs.bin")).unwrap(),
    )
    .unwrap();
    let factory = crate::delphi_special::Factory::load(&resources, &globals, &weapons).unwrap();
    let mut session = Session::new(971);
    let owner = factory
        .construct_player_fields(
            &mut session,
            crate::delphi_player::ColdFrame {
                position: [0., 0., 10.],
                current: [0., 0., 10.],
                previous: [0., 0., 10.],
                angles: [0.; 3],
            },
            crate::animation_channels::Blending::Disabled,
        )
        .unwrap()
        .owner;
    session.initialize_delphi_physics(owner, &globals).unwrap();
    let executable = std::env::var_os("GIANTS_DELPHI_EXECUTABLE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| root.join("Giants.exe"));
    let prepared = crate::delphi_physical::Resources::load_locomotion(
        &resources,
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
    .unwrap();
    session.prepare_delphi_physical(owner, prepared).unwrap();
    let input = |number, epoch| crate::delphi_target_owned::Input {
        owner,
        number,
        normalization_threshold: 1e-4,
        range: 0.,
        vertex_epoch: epoch,
        force_vertices: true,
    };
    let view = crate::delphi_target_scan::View {
        selectors: [0; 3],
        camera: crate::first_person::Camera {
            position: [-10., -5., 10.],
            angles: [0., -45., 0.],
        },
        viewport_origin: [500., 500., 10.],
        viewport_forward: [1., 0., 0.],
    };
    let scan_input = |number, epoch, viewport| crate::delphi_target_scan::ScanInput {
        owner,
        number,
        normalization_threshold: 1e-4,
        vertex_epoch: epoch,
        force_vertices: true,
        view: crate::delphi_target_scan::View {
            selectors: if viewport { [0; 3] } else { [1, 0, 0] },
            ..view
        },
    };
    assert!(
        session
            .query_delphi_target(&resources, input(1, 1))
            .is_err()
    );
    let terrain = giants_formats::gti::Terrain {
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
    let terrain = crate::terrain_queries::StaticTerrainQuery::new(
        &terrain,
        0.001,
        crate::arithmetic::Precision::Double53,
    )
    .unwrap();
    let mut world = crate::delphi_post_physical::World::default();
    assert!(
        session
            .scan_delphi_target(&resources, &definitions, scan_input(1, 1, true), &terrain)
            .is_err()
    );
    let mut contexts = crate::attached_effect_context::Pool::from_active_resources(971, &resources);
    let mut random = crate::audio_parameters::Random(1);
    let mut stamp = crate::animation_epoch::Stamp::default();
    session
        .advance_delphi_tail_controlled(
            owner,
            crate::delphi_physical::Environment {
                number: 1,
                actor_key: 1,
                globals: &globals,
                global_seconds: 1. / 60.,
                areas: &[],
                commands: [0; 8],
                launch_blocked: false,
                objects: &[],
            },
            crate::delphi_physical::ManualRotation {
                sensitivity: 1.,
                local_owner: true,
                paused: false,
            },
            crate::delphi_items::Context {
                reserve: crate::delphi_reserve::Context {
                    post: crate::delphi_post_physical::Context {
                        world: &mut world,
                        view_mode: 0,
                        stop_sound: &mut |_| Ok(()),
                    },
                    recharge_inhibited: false,
                    services: &mut crate::delphi_reserve::EmptyContext,
                },
                session_mode: 0,
                peer_ids: &[],
                local_peer: 0,
                owner_word: None,
            },
            |p| crate::physical_ground::Ground::from_sample(terrain.sample(p)?, 5),
            &mut crate::actor_animation_phase::Services {
                contexts: &mut contexts,
                random: &mut random,
                stamp: &mut stamp,
                blending: crate::animation_channels::Blending::Disabled,
            },
        )
        .unwrap();
    assert!(
        session
            .query_delphi_target(&resources, input(1, 1))
            .is_err()
    );
    let camera_scan = session
        .scan_delphi_target(&resources, &definitions, scan_input(1, 1, true), &terrain)
        .unwrap();
    assert!(camera_scan.viewport);
    assert_eq!(camera_scan.range, 9000.);
    assert!(camera_scan.terrain.point.is_some());
    assert!(camera_scan.query.target.is_none());
    assert!(camera_scan.aim_point[2].abs() < 1e-5);
    assert_eq!(session.position_buffer_count(), 0);
    assert!(
        session
            .scan_delphi_target(&resources, &definitions, scan_input(1, 1, false), &terrain)
            .is_err()
    );
    session.advance_delphi_held_pose(owner, 1).unwrap();
    let ray = session.delphi_target_ray(owner, 1, 1e-4).unwrap();
    let mut candidates = Vec::new();
    for distance in [2., 22.] {
        let class = 73;
        let program = resources
            .classes
            .models
            .first_program(class)
            .unwrap()
            .program
            .as_ref()
            .unwrap();
        let specification =
            Arc::new(crate::specification::Prepared::load_registered(&resources, class).unwrap());
        let seed = Seed::from_program_for(
            program,
            Profile::Debug,
            crate::arithmetic::Precision::Double53,
        )
        .unwrap();
        let position = std::array::from_fn(|i| ray.origin[i] + ray.direction[i] * distance);
        let mut fields = crate::actor_fields::Fields::load(&resources, class, &specification)
            .unwrap()
            .placed(crate::collider_init::DeclaredPose {
                class: class as u32,
                position,
                yaw: 0.,
                pitch_roll: Some([0.; 2]),
            })
            .unwrap();
        fields.set_declared_scale(0.5).unwrap();
        let id = session
            .construct(Construction {
                class,
                seed: &seed,
                bindings: &resources
                    .class_state_bindings_for(class, Profile::Debug)
                    .unwrap(),
                flags: DirectFlags::project(class, program),
                specification,
                state_resources: Arc::new(
                    crate::state_resources::Prepared::load(&resources, class, Profile::Debug)
                        .unwrap(),
                ),
                fields,
            })
            .unwrap();
        session.prepare_actor_affine(id).unwrap();
        candidates.push(id);
    }
    assert!(
        session
            .query_delphi_target(&resources, input(1, 0))
            .is_err()
    );
    assert!(
        session
            .query_delphi_target(&resources, input(2, 1))
            .is_err()
    );
    let payload = session
        .get(owner)
        .unwrap()
        .seed
        .storage
        .data()
        .unwrap()
        .to_vec();
    let selected = session
        .query_delphi_target(&resources, input(1, 1))
        .unwrap();
    assert_eq!(selected.target, Some(candidates[1]));
    assert_eq!(selected.geometry_visits, candidates);
    assert_eq!(selected.mesh_visits, candidates);
    let held_scan = session
        .scan_delphi_target(&resources, &definitions, scan_input(1, 1, false), &terrain)
        .unwrap();
    assert!(!held_scan.viewport);
    assert_eq!(held_scan.query.target, selected.target);
    assert_eq!(
        held_scan.query.point.map(f32::to_bits),
        selected.point.map(f32::to_bits)
    );
    assert!(
        session
            .scan_delphi_target(&resources, &definitions, scan_input(2, 1, true), &terrain)
            .is_err()
    );
    assert!(
        session
            .scan_delphi_target(&resources, &definitions, scan_input(1, 0, true), &terrain)
            .is_err()
    );
    assert_eq!(
        session.get(owner).unwrap().seed.storage.data().unwrap(),
        payload
    );
    let target = selected.target.unwrap();
    {
        let body = session
            .residents
            .get_mut(&owner.serial)
            .unwrap()
            .seed
            .storage
            .data_mut()
            .unwrap();
        body[0xdb4..0xdb8].copy_from_slice(&1_i32.to_le_bytes());
        body[0xd78..0xd7c].copy_from_slice(&0x3f_i32.to_le_bytes());
    }
    assert!(
        session
            .scan_delphi_target(&resources, &definitions, scan_input(1, 1, false), &terrain)
            .err()
            .unwrap()
            .contains("default range branch")
    );
    {
        let body = session
            .residents
            .get_mut(&owner.serial)
            .unwrap()
            .seed
            .storage
            .data_mut()
            .unwrap();
        body[0xdb4..0xdb8].copy_from_slice(&payload[0xdb4..0xdb8]);
        body[0xd78..0xd7c].copy_from_slice(&payload[0xd78..0xd7c]);
    }
    let old = session
        .get(target)
        .unwrap()
        .construction_fields
        .as_ref()
        .unwrap()
        .position;
    session
        .residents
        .get_mut(&target.serial)
        .unwrap()
        .construction_fields
        .as_mut()
        .unwrap()
        .position[0] += 1.;
    assert!(
        session
            .query_delphi_target(&resources, input(1, 1))
            .err()
            .unwrap()
            .contains("stale")
    );
    session
        .residents
        .get_mut(&target.serial)
        .unwrap()
        .construction_fields
        .as_mut()
        .unwrap()
        .position = old;
    session
        .set_presentation_parent(target, Some(owner))
        .unwrap();
    assert!(
        session
            .query_delphi_target(&resources, input(1, 1))
            .is_err()
    );
    session.set_presentation_parent(target, None).unwrap();
    let restored = session
        .query_delphi_target(&resources, input(1, 1))
        .unwrap();
    assert_eq!(restored.target, selected.target);
    assert_eq!(
        restored.point.map(f32::to_bits),
        selected.point.map(f32::to_bits)
    );
    assert_eq!(
        session.get(owner).unwrap().seed.storage.data().unwrap(),
        payload
    );
    let mut foreign = Session::new(972);
    assert!(
        foreign
            .scan_delphi_target(&resources, &definitions, scan_input(1, 1, true), &terrain)
            .is_err()
    );
    assert!(
        foreign
            .query_delphi_target(&resources, input(1, 1))
            .is_err()
    );
    session.clear();
    assert_eq!(session.counts().residents, 0);
    assert_eq!(session.position_buffer_count(), 0);
    assert!(
        session
            .scan_delphi_target(&resources, &definitions, scan_input(1, 1, true), &terrain)
            .is_err()
    );
    assert!(
        session
            .query_delphi_target(&resources, input(1, 1))
            .is_err()
    );
}
