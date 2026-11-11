//! Installed constructors with supplied flat terrain, eligibility, clocks and
//! captured/suppressed audio. No ordinary player/world admission is established.
use super::*;
use crate::{actor_fields::Fields, arithmetic::Precision, scene_resources::SceneResources};

#[derive(Default)]
struct Audio {
    calls: Vec<(bool, u32, u32)>,
    fail_play: Option<u32>,
    fail_stop: bool,
}
impl Services<Id, Effect> for Audio {
    fn owner_sound(
        &mut self,
        state: &State<Id, Effect>,
        cue: u32,
        _: &mut Random,
    ) -> Result<SoundOutcome, String> {
        self.calls.push((true, cue, state.entry.mode));
        if self.fail_play == Some(cue) {
            Err("supplied play failure".into())
        } else {
            Ok(SoundOutcome::Suppressed)
        }
    }
    fn stop_owner_sound(
        &mut self,
        state: &State<Id, Effect>,
        cue: u32,
        _: &mut Random,
    ) -> Result<SoundOutcome, String> {
        self.calls.push((false, cue, state.entry.mode));
        if self.fail_stop {
            Err("supplied stop failure".into())
        } else {
            Ok(SoundOutcome::Suppressed)
        }
    }
    fn remove_effect(
        &mut self,
        _: &State<Id, Effect>,
        _: Effect,
        _: &mut Random,
    ) -> Result<(), String> {
        Err("variant awakening must not retire dirt".into())
    }
}

fn source(resources: &SceneResources, ticket: u64, eligible: bool) -> (Session, Id, Id) {
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
    let mut session = Session::new(ticket);
    let mut ids = Vec::new();
    for (i, class) in [230, 73].into_iter().enumerate() {
        let program = resources
            .classes
            .models
            .first_program(class)
            .unwrap()
            .program
            .as_ref()
            .unwrap();
        let seed = Seed::from_program_for(program, Profile::Debug, Precision::Double53).unwrap();
        let bindings = resources
            .class_state_bindings_for(class, Profile::Debug)
            .unwrap();
        let flags = DirectFlags::project(class, program);
        let specification =
            Arc::new(crate::specification::Prepared::load_registered(resources, class).unwrap());
        let state_resources = Arc::new(
            crate::state_resources::Prepared::load(resources, class, Profile::Debug).unwrap(),
        );
        let fields = Fields::load(resources, class, &specification)
            .unwrap()
            .placed(crate::collider_init::DeclaredPose {
                class: class as u32,
                position: if i == 0 { [0.; 3] } else { [80., 0., 0.] },
                yaw: 0.,
                pitch_roll: None,
            })
            .unwrap();
        let id = session
            .construct(Construction {
                class,
                seed: &seed,
                bindings: &bindings,
                flags,
                specification,
                state_resources,
                fields,
            })
            .unwrap();
        session.prepare_actor_affine(id).unwrap();
        session
            .prepare_actor_environment(id, resources, &terrain)
            .unwrap();
        ids.push(id);
    }
    if eligible {
        // Explicit diagnostic input, matching the native peer control. This
        // does not acquire an ordinary writer of the eligibility flag.
        session
            .residents
            .get_mut(&ids[1].serial)
            .unwrap()
            .construction_fields
            .as_mut()
            .unwrap()
            .configuration[14] |= 1;
    }
    let id = ids[0];
    session.prepare_actor_finalization(id).unwrap();
    session.prepare_actor_motion(id).unwrap();
    session.begin_actor_frame_history(id).unwrap();
    assert!(session.begin_actor_motion_history(id, [0; 8]).unwrap());
    session.prepare_ripper_primary(id).unwrap();
    (session, id, ids[1])
}

#[test]
#[ignore = "Requires GIANTS_NPC_ROOT; source class230, flat terrain/peer eligibility and audio suppression supplied, no ordinary gameplay"]
fn source_variant_awakening_owns_marker_and_retains_late_failures() {
    let root = std::path::PathBuf::from(std::env::var_os("GIANTS_NPC_ROOT").unwrap());
    let resources = SceneResources::load(&root, "w_Story2.bin", "English").unwrap();
    let globals =
        giants_formats::global_data::parse(&std::fs::read(root.join("Bin/GData.gbn")).unwrap())
            .unwrap();
    for case in 0..6 {
        let (mut session, id, peer) = source(&resources, 690 + case, case != 0);
        let initial = session
            .get(id)
            .unwrap()
            .construction_fields
            .clone()
            .unwrap();
        let subset = session.perception_subset(&[id, peer]).unwrap();
        let frame = subset.frame().unwrap();
        let mut random = Random(314);
        let mut audio = Audio {
            fail_play: match case {
                2 => Some(1235),
                3 => Some(1246),
                _ => None,
            },
            fail_stop: case == 4,
            ..Default::default()
        };
        let mut factory = crate::primary_surface_marker::Factory::new(&resources);
        let areas = [crate::object_time::SlowArea {
            source_owner: 0,
            center: [0.; 3],
            radius_squared: 100.,
            factor: 0.,
        }];
        let context = crate::state23_entry::Context {
            paused: false,
            native_mode: 0,
            parent_state: None,
            player: None,
            delta: 0.1,
        };
        let parameters = VariantAwakeningContext {
            camera: [0.; 3],
            camera_range: globals.scalar(0x144).unwrap(),
            global_seconds: 0.1,
            areas: if case == 5 { &areas } else { &[] },
        };
        let result = session.update_ripper_variant_awakening(
            id,
            FrameInput {
                context,
                frame: &frame,
                range: globals.scalar(0x140).unwrap(),
            },
            parameters,
            &mut factory,
            &mut random,
            &mut audio,
        );
        assert_eq!(result.is_err(), matches!(case, 2..=4));
        let machine = session.ripper_primary(id).unwrap();
        let state = machine.state();
        let expected_mode = match case {
            0 | 3 | 5 => 61,
            2 => 0,
            _ => 62,
        };
        assert_eq!(state.entry.mode, expected_mode);
        assert_eq!(session.state23_combat(id).unwrap().mode, expected_mode);
        assert_eq!(
            session
                .get(id)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap()
                .dispatch_context,
            100
        );
        assert_eq!(state.entry.parameter.to_bits(), initial.configuration[1]);
        assert_eq!(random.0, 314);
        let marker = machine.surface_marker();
        assert_eq!(marker.is_some(), matches!(case, 1 | 4));
        if let Some(marker) = marker {
            let m = session.get(marker).unwrap();
            assert_eq!(
                (m.class, m.seed.storage.state(), m.constructor_mask),
                (219, 48, 0x11)
            );
            let fields = m.construction_fields.as_ref().unwrap();
            assert_eq!(fields.position, initial.position);
            assert_eq!(fields.current, initial.current);
            assert_eq!(fields.previous, initial.previous);
            assert_eq!(fields.angles, initial.angles);
            assert_eq!(fields.scale.to_bits(), initial.scale.to_bits());
            assert_eq!(
                session.state23_combat(id).unwrap().owner,
                Some((marker, 219))
            );
            assert!(
                matches!(session.surface_link(marker), Some(SurfaceLink::Marker { owner, .. }) if owner == id)
            );
            assert!(!session.actor_motion(id).unwrap().surface_reset());
            assert_eq!(
                audio.calls,
                [(true, 1235, 0), (true, 1246, 61), (false, 1235, 62)]
            );
        }
        if case == 5 {
            // Entry context100 makes the owner0 area apply to the second clock;
            // reusing the earlier supplied delta would incorrectly create a marker.
            assert_eq!(state.timer.to_bits(), 0);
        }
        let count = session.counts();
        let calls = audio.calls.len();
        assert!(
            session
                .update_ripper_variant_awakening(
                    id,
                    FrameInput {
                        context,
                        frame: &frame,
                        range: globals.scalar(0x140).unwrap()
                    },
                    parameters,
                    &mut factory,
                    &mut random,
                    &mut audio
                )
                .is_err()
        );
        assert_eq!(session.counts(), count);
        assert_eq!(audio.calls.len(), calls);
        session.clear();
        assert!(session.ripper_primary(id).is_err());
    }
}
