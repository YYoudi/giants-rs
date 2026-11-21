use super::*;

#[test]
#[ignore = "Requires GIANTS_DELPHI_ROOT; actual marker/shell/type owners, supplied camera transport, no ordinary gameplay"]
fn source_delphi_live_start_copies_stored_cache_and_keeps_player_identity() {
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
    let factory = player_shell::Factory::load(&resources).unwrap();
    let markers = crate::herd_markers::Factory::load(&resources).unwrap();
    let delphi = crate::delphi_special::Factory::load(&resources, &globals, &weapons).unwrap();
    struct Camera(usize);
    impl player_shell::Camera for Camera {
        fn change_view(&mut self, _: &mut Session, _: Id, request: i32) -> Result<(), String> {
            assert_eq!(request, 1);
            self.0 += 1;
            Ok(())
        }
        fn update_camera(&mut self, _: &mut Session, _: Id) -> Result<(), String> {
            self.0 += 1;
            Ok(())
        }
    }
    let mut session = Session::new(2150);
    let mut entry = factory.construct(&mut session).unwrap();
    assert!(entry.relocate_delphi_start(&mut session).is_err());
    let owner = entry.owner();
    let mut camera = Camera(0);
    entry
        .initialize(
            &mut session,
            player_shell::WorldPose {
                position: [-7., 11., 13.],
                heading: 17.,
            },
            &mut camera,
        )
        .unwrap();
    assert_eq!(camera.0, 2);
    let mut placement = giants_formats::mission::Placement {
        source_offset: 10,
        object_type: 679,
        position: [4., 5., 6.],
        yaw: 73.,
        pitch_roll: Some([5., 9.]),
        scale: None,
    };
    let skipped = markers
        .place_mission(&mut session, &placement, Some(1), Some(0), None)
        .unwrap();
    placement.source_offset += 10;
    let selected = markers
        .place_mission(&mut session, &placement, Some(2), Some(0), None)
        .unwrap();
    placement.source_offset += 10;
    let later = markers
        .place_mission(&mut session, &placement, Some(2), Some(0), None)
        .unwrap();
    // A source cache may legitimately be stale. Copy its raw stored words,
    // rather than regenerating an affine from the selected marker's new pose.
    session
        .residents
        .get_mut(&selected.serial)
        .unwrap()
        .construction_fields
        .as_mut()
        .unwrap()
        .position = [7., 8., 9.];
    assert!(session.actor_current_affine(selected).is_err());
    let cache = session
        .get(selected)
        .unwrap()
        .affine_cache
        .as_ref()
        .unwrap()
        .value;
    let words = |cache: crate::actor_affine::Cache| {
        let mut words = [0; 17];
        words[0] = cache.flags;
        words[1..].copy_from_slice(&cache.matrix.map(f32::to_bits));
        words
    };
    let before = session
        .get(owner)
        .unwrap()
        .construction_fields
        .as_ref()
        .unwrap()
        .clone();
    let lists = (0..15).map(|list| session.list(list)).collect::<Vec<_>>();
    let next = session.next;
    assert_eq!(
        entry.relocate_delphi_start(&mut session).unwrap(),
        Some(selected)
    );
    assert_eq!(entry.checkpoint(), player_shell::Checkpoint::StartApplied);
    assert!(entry.relocate_delphi_start(&mut session).is_err());
    assert_eq!(session.next, next);
    assert_eq!(
        (0..15).map(|list| session.list(list)).collect::<Vec<_>>(),
        lists
    );
    let after = session
        .get(owner)
        .unwrap()
        .construction_fields
        .as_ref()
        .unwrap();
    let mut expected = before.clone();
    expected.position = [7., 8., 9.];
    expected.angles = [73., 5., 9.];
    assert_eq!(after.words(), expected.words());
    assert_eq!(
        words(session.get(owner).unwrap().cold_player_start_cache.unwrap()),
        words(cache)
    );
    assert!(session.actor_current_affine(owner).is_err());
    assert!(
        session
            .publish_player_shell_pose(
                owner,
                player_shell::WorldPose {
                    position: [0.; 3],
                    heading: 0.
                }
            )
            .is_err()
    );
    let nodes = session.links.memberships(owner.serial);
    struct Projection<F>(F);
    impl<F: FnMut(&Session, Id) -> Result<(), String>> crate::delphi_player::ProjectionReset
        for Projection<F>
    {
        fn reset_projection(&mut self, s: &Session, id: Id) -> Result<(), String> {
            (self.0)(s, id)
        }
    }
    let mut projection = Projection(|s: &Session, id: Id| -> Result<(), String> {
        assert_eq!(id, owner);
        assert_eq!(s.next, next);
        assert_eq!(s.links.memberships(id.serial), nodes);
        assert_eq!(
            words(s.get(id).unwrap().cold_player_start_cache.unwrap()),
            words(cache)
        );
        assert!(s.actor_current_affine(id).is_err());
        let fields = s.get(id).unwrap().construction_fields.as_ref().unwrap();
        assert_eq!(fields.current, before.current);
        assert_eq!(fields.previous, before.previous);
        Ok(())
    });
    let result = delphi
        .retype_player_shell_with_projection(
            &mut session,
            owner,
            crate::animation_channels::Blending::Enabled,
            &mut projection,
        )
        .result
        .unwrap();
    assert_eq!(result.owner, owner);
    assert_eq!(
        words(session.get(owner).unwrap().cold_player_start_cache.unwrap()),
        words(cache)
    );
    assert!(session.actor_current_affine(owner).is_err());
    let fields = session
        .get(owner)
        .unwrap()
        .construction_fields
        .as_ref()
        .unwrap();
    assert_eq!(fields.position, [7., 8., 9.]);
    assert_eq!(fields.angles, [73., 0., 9.]);
    assert_eq!(fields.current, fields.position);
    assert_eq!(fields.previous, before.current);
    for id in session.list(0) {
        if ![owner, skipped, selected, later].contains(&id) {
            assert!(session.get(id).unwrap().cold_player_start_cache.is_none());
        }
    }
    session.clear();
    assert!(session.get(owner).is_none());
    // An absent marker is an observed no-copy result, not a manufactured start.
    let mut empty = factory.construct(&mut session).unwrap();
    empty
        .initialize(
            &mut session,
            player_shell::WorldPose {
                position: [2., 3., 4.],
                heading: 5.,
            },
            &mut camera,
        )
        .unwrap();
    assert_eq!(empty.relocate_delphi_start(&mut session).unwrap(), None);
    assert!(
        session
            .get(empty.owner())
            .unwrap()
            .cold_player_start_cache
            .is_none()
    );
    assert!(empty.relocate_delphi_start(&mut session).is_err());
    session.clear();
    let mut incomplete = factory.construct(&mut session).unwrap();
    incomplete
        .initialize(
            &mut session,
            player_shell::WorldPose {
                position: [2., 3., 4.],
                heading: 5.,
            },
            &mut camera,
        )
        .unwrap();
    let marker = markers
        .place_mission(&mut session, &placement, Some(2), Some(0), None)
        .unwrap();
    session
        .residents
        .get_mut(&marker.serial)
        .unwrap()
        .affine_cache = None;
    let owner = incomplete.owner();
    let before = session
        .get(owner)
        .unwrap()
        .construction_fields
        .as_ref()
        .unwrap()
        .words();
    assert!(
        incomplete
            .relocate_delphi_start(&mut session)
            .unwrap_err()
            .contains("raw cache unacquired")
    );
    assert_eq!(
        incomplete.checkpoint(),
        player_shell::Checkpoint::CameraApplied
    );
    assert_eq!(
        session
            .get(owner)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap()
            .words(),
        before
    );
    assert!(
        session
            .get(owner)
            .unwrap()
            .cold_player_start_cache
            .is_none()
    );
    let mut foreign = Session::new(2151);
    assert!(incomplete.relocate_delphi_start(&mut foreign).is_err());
    assert_eq!(foreign.counts().residents, 0);
}

#[test]
#[ignore = "Requires GIANTS_DELPHI_ROOT; source world pose -> actual shell fields, stops at required camera provider; not full startup"]
fn source_delphi_world_declaration_reaches_shell_before_required_camera_boundary() {
    let root = std::path::PathBuf::from(std::env::var("GIANTS_DELPHI_ROOT").unwrap());
    let resources =
        crate::scene_resources::SceneResources::load(&root, "w_Rstory1.bin", "English").unwrap();
    let mut world =
        giants_formats::world::parse_world(&std::fs::read(root.join("Bin/w_Rstory1.bin")).unwrap())
            .unwrap();
    let declaration = *world
        .entry_poses
        .iter()
        .rev()
        .find(|pose| pose.index == 0)
        .unwrap();
    let factory = player_shell::Factory::load(&resources).unwrap();
    struct Camera {
        words: [u32; 4],
        calls: Vec<&'static str>,
    }
    impl player_shell::Camera for Camera {
        fn change_view(&mut self, s: &mut Session, owner: Id, request: i32) -> Result<(), String> {
            assert_eq!(request, 1);
            self.calls.push("view");
            let fields = s.get(owner).unwrap().construction_fields.as_ref().unwrap();
            assert_eq!(fields.dispatch_context, 0);
            assert_eq!(fields.position, [0.; 3]);
            Ok(())
        }
        fn update_camera(&mut self, s: &mut Session, owner: Id) -> Result<(), String> {
            self.calls.push("update");
            let fields = s.get(owner).unwrap().construction_fields.as_ref().unwrap();
            assert_eq!(fields.dispatch_context, 1);
            assert_eq!(fields.position.map(f32::to_bits), self.words[..3]);
            assert_eq!(fields.current, fields.position);
            assert_eq!(fields.previous, fields.position);
            assert_eq!(fields.angles[0].to_bits(), self.words[3]);
            Err("required actual camera world/device provider".into())
        }
    }
    let mut session = Session::new(2160);
    let mut entry = factory.construct(&mut session).unwrap();
    let owner = entry.owner();
    let mut camera = Camera {
        words: declaration.words,
        calls: Vec::new(),
    };
    assert!(
        entry
            .initialize_declared_world_pose(&mut session, &world, &mut camera)
            .unwrap_err()
            .contains("required actual camera")
    );
    assert_eq!(entry.checkpoint(), player_shell::Checkpoint::PoseApplied);
    assert_eq!(camera.calls, ["view", "update"]);
    assert!(entry.failure().is_some());
    assert!(
        session
            .get(owner)
            .unwrap()
            .cold_player_start_cache
            .is_none()
    );
    assert!(session.actor_current_affine(owner).is_err());
    assert!(entry.relocate_delphi_start(&mut session).is_err());
    assert!(
        entry
            .initialize_declared_world_pose(&mut session, &world, &mut camera)
            .is_err()
    );
    assert_eq!(camera.calls.len(), 2);
    for absent in [false, true] {
        let mut fresh = factory.construct(&mut session).unwrap();
        if absent {
            world.entry_poses.clear();
        } else {
            world
                .entry_poses
                .iter_mut()
                .rev()
                .find(|pose| pose.index == 0)
                .unwrap()
                .words[0] = f32::NAN.to_bits();
        }
        assert!(
            fresh
                .initialize_declared_world_pose(&mut session, &world, &mut camera)
                .is_err()
        );
        assert_eq!(fresh.checkpoint(), player_shell::Checkpoint::Constructed);
        assert_eq!(camera.calls.len(), 2);
    }
    session.clear();
    assert!(session.get(owner).is_none());
}
