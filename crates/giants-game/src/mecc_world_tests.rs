//! Source-backed ownership and failure controls, CPU only.
use super::*;
use crate::{
    actor_world_tick::{Outcome, Phase},
    animation_channels::Blending,
};

#[test]
#[ignore = "Requires GIANTS_MECC_WORLD_ROOT; subjective ownership and failure lifecycle only, no window or sound"]
fn source_mecc_subjective_publishes_before_fire_and_retains_late_failure() {
    use crate::mecc_subjective::{self as subjective, History, Post, State};
    struct SubjectiveServices<'a> {
        session: &'a mut Session,
        phases: Vec<&'static str>,
        fail_references: bool,
        fail_fire: bool,
    }
    impl subjective::Services<Id> for SubjectiveServices<'_> {
        fn references(
            &mut self,
            id: Id,
            context: &mut crate::actor_references::Owned,
            channels: &crate::animation_channels::Channels,
            epoch: u32,
        ) -> Result<(), String> {
            self.phases.push("references");
            if self.fail_references {
                return Err("injected reference failure".into());
            }
            self.session
                .update_mecc_subjective_references(id, context, channels, epoch)?;
            Ok(())
        }
        fn fire(&mut self, id: Id, _: u64) -> Result<(), String> {
            self.phases.push("fire");
            // A completed source-state effect must survive a later service error.
            self.session
                .residents
                .get_mut(&id.serial)
                .unwrap()
                .seed
                .storage
                .data_mut()
                .unwrap()[0x140..0x144]
                .copy_from_slice(&123_f32.to_le_bytes());
            if self.fail_fire {
                Err("injected fire failure".into())
            } else {
                Ok(())
            }
        }
    }
    let root = std::path::PathBuf::from(std::env::var("GIANTS_MECC_WORLD_ROOT").unwrap());
    let r = crate::scene_resources::SceneResources::load(&root, "w_Story1.bin", "English").unwrap();
    let g = giants_formats::global_data::parse(&std::fs::read(root.join("Bin/GData.gbn")).unwrap())
        .unwrap();
    let c = giants_formats::weapons::parse_with_profile(
        &std::fs::read(root.join("Bin/weapondefs.bin")).unwrap(),
        giants_formats::weapons::Profile::Debug,
    )
    .unwrap();
    let mut session = Session::new(814);
    let source = actor(&mut session, &r, &g, &c, None, 0.);
    assert!(State::for_session(&Session::new(815), source, [[0; 9]; 4]).is_err());
    let foreign_spec = Arc::new(
        crate::specification::Prepared::load_state_registered(&r, 2, Profile::Debug, 0x70).unwrap(),
    );
    let mut foreign_context = crate::actor_references::Owned::new(foreign_spec).unwrap();
    assert!(
        session
            .update_mecc_subjective_references(source, &mut foreign_context, &[[0; 9]; 4], 1)
            .is_err()
    );
    let input = |flags, seconds| Post {
        controlled: source,
        camera_owner: Some(source),
        controlled_flags: flags,
        icon_index: 0,
        weapon: 0,
        inventory_present: false,
        previous_camera: crate::first_person::Camera {
            position: [0., 0., 10.],
            angles: [0.; 3],
        },
        current_camera: crate::first_person::Camera {
            position: [2., 1., 11.],
            angles: [0.; 3],
        },
        offset: [0.; 3],
        seconds,
        blending: Blending::Enabled,
    };
    for fail_references in [false, true] {
        session.update_current_references_at(source, 1).unwrap();
        let mut state = State::for_session(&session, source, [[0; 9]; 4]).unwrap();
        state.prepare_from_session(1, &session, source, 0).unwrap();
        let mut history = History {
            previous_weapon: -1,
        };
        let mut stamp = crate::animation_epoch::Stamp(9);
        let mut services = SubjectiveServices {
            session: &mut session,
            phases: vec![],
            fail_references,
            fail_fire: true,
        };
        // Invalid time is rejected without consuming this prepared frame.
        assert!(
            state
                .post(input(0, f32::NAN), &mut history, &mut stamp, &mut services)
                .is_err()
        );
        assert_eq!(state.phase(), subjective::Phase::Prepared);
        assert!(services.phases.is_empty());
        assert!(
            state
                .post(input(0, 1. / 60.), &mut history, &mut stamp, &mut services)
                .is_err()
        );
        assert_eq!(state.phase(), subjective::Phase::Failed);
        assert_eq!(state.channels()[0][0], 20);
        assert_eq!(state.channels()[0][1], (1_f32 / 60.).to_bits());
        assert_eq!(state.header().unwrap().animation_stamp, 10);
        assert_eq!(stamp.0, 10);
        let phases = services.phases.clone();
        assert_eq!(
            phases,
            if fail_references {
                vec!["references"]
            } else {
                vec!["references", "fire"]
            }
        );
        assert!(
            state
                .post(input(0, 1. / 60.), &mut history, &mut stamp, &mut services)
                .is_err()
        );
        assert!(state.inactive(2).is_err());
        assert_eq!(services.phases, phases);
        if !fail_references {
            let body = services.session.get(source).unwrap();
            assert_eq!(
                &body.seed.storage.data().unwrap()[0x140..0x144],
                &123_f32.to_le_bytes()
            );
            assert!(
                services.session.specifications[&source.serial]
                    .0
                    .current_matrices(body.channels.as_ref().unwrap())
                    .is_err()
            );
        }
    }
    let mut state = State::for_session(&session, source, [[0; 9]; 4]).unwrap();
    state.prepare_from_session(1, &session, source, 0).unwrap();
    let mut services = SubjectiveServices {
        session: &mut session,
        phases: vec![],
        fail_references: false,
        fail_fire: true,
    };
    let mut history = History {
        previous_weapon: -1,
    };
    let mut stamp = crate::animation_epoch::Stamp(19);
    let out = state
        .post(
            input(0x0200_0000, 1. / 60.),
            &mut history,
            &mut stamp,
            &mut services,
        )
        .unwrap()
        .unwrap();
    assert!(!out.fire_called);
    assert_eq!(out.tail.processed, 0);
    assert_eq!(stamp.0, 20);
    assert_eq!(services.phases, vec!["references"]);
    let retained = *state.channels();
    state.inactive(2).unwrap();
    assert!(
        state
            .post(input(0, 1. / 60.), &mut history, &mut stamp, &mut services)
            .unwrap()
            .is_none()
    );
    assert_eq!(*state.channels(), retained);
    assert!(!state.display(Some(source)).owns(&source));
    services.session.clear();
    assert!(
        state
            .prepare_from_session(3, services.session, source, 0)
            .is_err()
    );
}

struct NoAudio;
impl crate::audio_sequence::Backend for NoAudio {
    type Handle = u32;
    fn play(&mut self, _: u32, _: u64, _: crate::audio_sequence::Route) -> Result<u32, String> {
        Err("unexpected test audio".into())
    }
    fn stop_sound(&mut self, _: u32, _: u64) -> Result<(), String> {
        Err("unexpected test sound stop".into())
    }
    fn duration(&mut self, _: u32) -> Result<f32, String> {
        Err("unexpected test duration".into())
    }
    fn volume(&mut self, _: u32, _: f32) -> Result<(), String> {
        Err("unexpected test volume".into())
    }
    fn stop(&mut self, _: u32) -> Result<(), String> {
        Err("unexpected test stop".into())
    }
}
struct Providers<'r> {
    contexts: crate::attached_effect_context::Pool<Id>,
    random: crate::audio_parameters::Random,
    stamp: crate::animation_epoch::Stamp,
    wield: crate::mecc_wield::Factory<'r>,
    sequences: crate::mecc_jet_owned::Pool,
    audio: NoAudio,
}
impl<'r> Providers<'r> {
    fn new(r: &'r crate::scene_resources::SceneResources) -> Self {
        Self {
            contexts: crate::attached_effect_context::Pool::from_active_resources(810, r),
            random: crate::audio_parameters::Random(1),
            stamp: crate::animation_epoch::Stamp(0),
            wield: crate::mecc_wield::Factory::new(r),
            sequences: crate::mecc_jet_owned::Pool::new(811),
            audio: NoAudio,
        }
    }
    fn borrowed(&mut self) -> crate::mecc_primary::Services<'_, 'r, '_> {
        crate::mecc_primary::Services {
            fire: None,
            animation: crate::mecc_animation_phase::Services {
                contexts: &mut self.contexts,
                random: &mut self.random,
                stamp: &mut self.stamp,
                blending: Blending::Disabled,
            },
            jet: crate::mecc_jet_owned::Services {
                wield: &mut self.wield,
                sequences: &mut self.sequences,
                backend: &mut self.audio,
                input: crate::mecc_jet_owned::Input {
                    audio_key: 1,
                    ..Default::default()
                },
            },
            collection: None,
        }
    }
}
fn actor(
    s: &mut Session,
    r: &crate::scene_resources::SceneResources,
    g: &giants_formats::global_data::GlobalData,
    c: &giants_formats::weapons::Catalog,
    shared: Option<Arc<crate::specification::Prepared>>,
    x: f32,
) -> Id {
    let body = shared.unwrap_or_else(|| {
        Arc::new(crate::specification::Prepared::load_registered(r, 2).unwrap())
    });
    let states = Arc::new(crate::state_resources::Prepared::load(r, 2, Profile::Debug).unwrap());
    let program = r
        .classes
        .models
        .first_program(2)
        .unwrap()
        .program
        .as_ref()
        .unwrap();
    let seed = Seed::from_program_for(
        program,
        Profile::Debug,
        crate::arithmetic::Precision::Double53,
    )
    .unwrap();
    let bindings = r.class_state_bindings_for(2, Profile::Debug).unwrap();
    let fields = crate::actor_fields::Fields::load(r, 2, &body)
        .unwrap()
        .placed(crate::collider_init::DeclaredPose {
            class: 2,
            position: [x, 0., 10.],
            yaw: 0.,
            pitch_roll: None,
        })
        .unwrap();
    let id = s
        .construct(Construction {
            class: 2,
            seed: &seed,
            bindings: &bindings,
            flags: DirectFlags::project(2, program),
            specification: body.clone(),
            state_resources: states,
            fields,
        })
        .unwrap();
    crate::mecc_special::Factory::new(r)
        .initialize_player_fields(s, id, Blending::Disabled)
        .unwrap();
    s.prepare_mecc_physics(id, 1, g, c).unwrap();
    s.prepare_mecc_physical_cycle(
        id,
        crate::mecc_physical_cycle::Resources::load(r, body).unwrap(),
    )
    .unwrap();
    id
}
fn ground(_: [f32; 2]) -> Result<crate::physical_ground::Ground, &'static str> {
    Ok(crate::physical_ground::Ground {
        plane: [0., 0., 1., -0.],
        height: -0.,
        key: [0, 0],
    })
}
#[test]
#[ignore = "Requires GIANTS_MECC_WORLD_ROOT pointing to the owned source clone; CPU-only, no window, sound or input"]
fn source_mecc_world_history_admission_and_failed_tokens() {
    let root = std::path::PathBuf::from(
        std::env::var("GIANTS_MECC_WORLD_ROOT").expect("owned source clone required"),
    );
    let r = crate::scene_resources::SceneResources::load(&root, "w_Story1.bin", "English").unwrap();
    let g = giants_formats::global_data::parse(&std::fs::read(root.join("Bin/GData.gbn")).unwrap())
        .unwrap();
    let c = giants_formats::weapons::parse_with_profile(
        &std::fs::read(root.join("Bin/weapondefs.bin")).unwrap(),
        giants_formats::weapons::Profile::Debug,
    )
    .unwrap();
    let env = |number| crate::mecc_physical_cycle::Environment {
        number,
        generation: 1,
        globals: &g,
        catalog: &c,
        global_seconds: 1. / 60.,
        areas: &[],
        controls: [0, 0],
        facing_pitch: 0.,
        orientation: Some(crate::mecc_physical_cycle::Orientation {
            controls: [0, 0],
            control_factor: 1.,
            skip_heading: false,
            controlled_actor: true,
        }),
        actor_key: 1,
        objects: &[],
    };
    let mut s = Session::new(812);
    let a = actor(&mut s, &r, &g, &c, None, 0.);
    let spec = s.specification(a).unwrap().clone();
    let b = actor(&mut s, &r, &g, &c, Some(spec), 100.);
    s.prepare_mecc_countdown_world(10.).unwrap();
    let mut p = Providers::new(&r);
    // Distinct history inputs detect a duplicate copy even when positions are static.
    for id in [a, b] {
        let f = s
            .residents
            .get_mut(&id.serial)
            .unwrap()
            .construction_fields
            .as_mut()
            .unwrap();
        f.current = [-25., -26., -27.];
        f.previous = [-50., -51., -52.];
        f.object_flags |= 0x40;
        s.begin_mecc_frame_history(id, 1).unwrap();
        assert!(s.begin_mecc_frame_history(id, 1).is_err());
    }
    for id in [a, b] {
        let step = s
            .advance_mecc_world_offline(id, env(1), Some(a), ground, p.borrowed())
            .unwrap();
        assert_eq!(step.outcome, Outcome::UpdatedByPrimary);
        assert!(step.primary.is_some());
        let f = s.get(id).unwrap().construction_fields.as_ref().unwrap();
        assert_eq!(f.previous, [-25., -26., -27.]);
        assert_ne!(f.object_flags & 0x40, 0);
        assert_eq!(f.object_flags & 0x800, 0);
        assert_eq!(s.actor_world_tick(id).unwrap().phase(), Phase::Finished);
    }
    let epoch = p.stamp.0;
    let world = s.mecc_countdown_world();
    assert!(
        s.advance_mecc_world_offline(
            a,
            env(1),
            Some(a),
            |_| panic!("replay reached ground"),
            p.borrowed()
        )
        .is_err()
    );
    assert_eq!(p.stamp.0, epoch);
    assert_eq!(s.mecc_countdown_world(), world);
    s.begin_mecc_frame_history(a, 2).unwrap();
    assert!(
        s.advance_mecc_normal_primary(
            a,
            env(2),
            |_| panic!("standalone bypass reached ground"),
            p.borrowed()
        )
        .is_err()
    );
    assert_eq!(s.mecc_countdown_world(), world);
    assert!(
        s.advance_mecc_world_offline(
            a,
            env(2),
            Some(a),
            |_| Err("declared ground failure"),
            p.borrowed()
        )
        .is_err()
    );
    assert_eq!(s.actor_world_tick(a).unwrap().phase(), Phase::Failed);
    assert_eq!(p.stamp.0, epoch);
    assert!(s.begin_mecc_frame_history(a, 2).is_err());
    assert!(s.begin_mecc_frame_history(a, 3).is_err());
    let retained = s.mecc_countdown_world();
    assert!(
        s.advance_mecc_world_offline(
            a,
            env(2),
            Some(a),
            |_| panic!("failed token replay"),
            p.borrowed()
        )
        .is_err()
    );
    assert_eq!(s.mecc_countdown_world(), retained);
    // Native common policy skips inactive actors before inspecting providers;
    // a missing current controller aborts only after the real primary returns.
    for (ticket, inactive) in [(813, true), (814, false)] {
        let mut s = Session::new(ticket);
        let id = actor(&mut s, &r, &g, &c, None, 0.);
        s.prepare_mecc_countdown_world(10.).unwrap();
        let mut p = Providers::new(&r);
        s.residents
            .get_mut(&id.serial)
            .unwrap()
            .construction_fields
            .as_mut()
            .unwrap()
            .object_flags |= if inactive { 0xc840 } else { 0 };
        s.begin_mecc_frame_history(id, 1).unwrap();
        let mut calls = 0;
        let mut supplied = env(1);
        if inactive {
            supplied.orientation = None;
        }
        let step = s
            .advance_mecc_world_offline(
                id,
                supplied,
                None,
                |at| {
                    calls += 1;
                    assert!(!inactive);
                    ground(at)
                },
                p.borrowed(),
            )
            .unwrap();
        assert_eq!(
            step.outcome,
            if inactive {
                Outcome::Inactive
            } else {
                Outcome::WorldAborted
            }
        );
        assert_eq!(step.primary.is_none(), inactive);
        assert_eq!(calls == 0, inactive);
        assert_eq!(s.actor_world_tick(id).unwrap().phase(), Phase::Finished);
        assert_ne!(s.actor_world_flags(id).unwrap() & 0x40, 0);
        if inactive {
            assert_ne!(s.actor_world_flags(id).unwrap() & 0x800, 0);
        }
        if inactive {
            assert_eq!(p.stamp.0, 0);
            assert_eq!(s.mecc_countdown_world(), Some(10.));
        }
        assert!(s.begin_mecc_frame_history(id, 2).is_err());
        assert!(
            s.advance_mecc_physical_cycle(id, env(1), |_| panic!("stopped standalone bypass"))
                .is_err()
        );
    }

    let mut s = Session::new(815);
    let id = actor(&mut s, &r, &g, &c, None, 0.);
    s.prepare_mecc_countdown_world(10.).unwrap();
    let mut p = Providers::new(&r);
    s.residents
        .get_mut(&id.serial)
        .unwrap()
        .construction_fields
        .as_mut()
        .unwrap()
        .object_flags |= 0x800;
    s.begin_mecc_frame_history(id, 1).unwrap();
    let error = s
        .advance_mecc_world_offline(id, env(1), Some(id), ground, p.borrowed())
        .err()
        .expect("active reaction must not silently succeed");
    assert!(error.contains("outside acquired source pistol profile"));
    assert_eq!(s.actor_world_tick(id).unwrap().phase(), Phase::Failed);
    assert_ne!(s.actor_world_flags(id).unwrap() & 0x800, 0);
    assert!(s.begin_mecc_frame_history(id, 2).is_err());

    let mut s = Session::new(816);
    let id = actor(&mut s, &r, &g, &c, None, 0.);
    s.prepare_mecc_countdown_world(10.).unwrap();
    let mut p = Providers::new(&r);
    s.begin_mecc_frame_history(id, 1).unwrap();
    let mut missing = env(1);
    missing.orientation = None;
    let error = s
        .advance_mecc_world_offline(
            id,
            missing,
            Some(id),
            |_| panic!("missing orientation reached ground"),
            p.borrowed(),
        )
        .err()
        .expect("missing orientation must not acknowledge a primary");
    assert!(error.contains("orientation input provider"));
    assert_eq!(s.mecc_countdown_world(), Some(10.));
    assert_eq!(p.stamp.0, 0);
    assert_eq!(s.actor_world_tick(id).unwrap().phase(), Phase::Failed);
}
