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
        fn cameras(
            &mut self,
            _: Id,
            _: u64,
            previous: crate::first_person::Camera,
            current: crate::first_person::Camera,
        ) -> Result<(crate::first_person::Camera, crate::first_person::Camera), String> {
            self.phases.push("camera");
            Ok((previous, current))
        }
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
                vec!["camera", "references"]
            } else {
                vec!["camera", "references", "fire"]
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
    assert_eq!(services.phases, vec!["camera", "references"]);
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
#[ignore = "Requires GIANTS_MECC_WORLD_ROOT; deferred primary/post ownership and injected sound failure only"]
fn source_mecc_deferred_primary_requires_post_and_retains_real_fire_failure() {
    deferred_primary_controls(false);
}
#[test]
#[ignore = "Requires GIANTS_MECC_WORLD_ROOT; produced camera/post/fire lifecycle, no window or sound"]
fn source_mecc_produced_camera_orders_post_fire_and_outer_publication() {
    deferred_primary_controls(true);
}
fn deferred_primary_controls(produced: bool) {
    struct NumericBoundary;
    impl crate::mecc_subjective::Services<Id> for NumericBoundary {
        fn cameras(
            &mut self,
            _: Id,
            _: u64,
            previous: crate::first_person::Camera,
            current: crate::first_person::Camera,
        ) -> Result<(crate::first_person::Camera, crate::first_person::Camera), String> {
            Ok((previous, current))
        }
        fn references(
            &mut self,
            _: Id,
            _: &mut crate::actor_references::Owned,
            _: &crate::animation_channels::Channels,
            _: u32,
        ) -> Result<(), String> {
            Ok(())
        }
        fn fire(&mut self, _: Id, _: u64) -> Result<(), String> {
            Ok(())
        }
    }
    struct Sound {
        fail: bool,
    }
    impl crate::mecc_shot::Services for Sound {
        fn aim(&mut self, _: &mut Session, _: Id, _: [f32; 3], _: f32) -> Result<[f32; 3], String> {
            Err("post must execute owned aim".into())
        }
        fn muzzle(
            &mut self,
            _: &Session,
            _: crate::mecc_fire_lock::Binding<Id>,
        ) -> Result<[f32; 3], String> {
            Err("post must consume the current held cache".into())
        }
        fn effect(
            &mut self,
            _: &mut Session,
            _: Id,
            _: &str,
            _: &mut crate::audio_parameters::Random,
        ) -> Result<crate::attached_effect_context::Creation, String> {
            Err("post must prepare actual named effects".into())
        }
        fn sound(
            &mut self,
            _: &mut Session,
            _: Id,
            _: u32,
        ) -> Result<crate::enemy_projectile::SoundOutcome, String> {
            if self.fail {
                Err("injected post-shot sound failure".into())
            } else {
                Ok(crate::enemy_projectile::SoundOutcome::ResourceUnavailable)
            }
        }
    }
    let root = std::path::PathBuf::from(std::env::var("GIANTS_MECC_WORLD_ROOT").unwrap());
    let r = crate::scene_resources::SceneResources::load(&root, "w_Story1.bin", "English").unwrap();
    let g = giants_formats::global_data::parse(&std::fs::read(root.join("Bin/GData.gbn")).unwrap())
        .unwrap();
    let defs = crate::projectile_definition::DebugDefinitions::parse(
        &std::fs::read(root.join("Bin/weapondefs.bin")).unwrap(),
    )
    .unwrap();
    let factory = crate::mecc_shot::Factory::load(&r, &defs).unwrap();
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
    let query = crate::terrain_queries::StaticTerrainQuery::new(
        &terrain,
        0.001,
        crate::arithmetic::Precision::Double53,
    )
    .unwrap();
    let mut cases = vec![(0.25_f32, false), (0., false), (0., true)];
    if !produced {
        cases.push((0.25, false));
    }
    for (index, (clock, fail_sound)) in cases.into_iter().enumerate() {
        let skipped_camera_owner = index == 3;
        let mut s = Session::new(830 + index as u64);
        let owner = actor(&mut s, &r, &g, &defs.catalog, None, 0.);
        s.prepare_mecc_countdown_world(10.).unwrap();
        let body = s
            .residents
            .get_mut(&owner.serial)
            .unwrap()
            .seed
            .storage
            .data_mut()
            .unwrap();
        body[0x114..0x118].copy_from_slice(&25_f32.to_le_bytes());
        body[0x118..0x11c].copy_from_slice(&0x1234_u32.to_le_bytes());
        body[0x38..0x3c].copy_from_slice(&clock.to_le_bytes());
        // Generic math inputs cannot stand in for an owned Session pre-copy.
        let mut unbound =
            crate::mecc_subjective::State::for_session(&s, owner, [[0; 9]; 4]).unwrap();
        unbound
            .prepare(
                1,
                crate::mecc_subjective::Source {
                    source: owner,
                    fields: s.get(owner).unwrap().construction_fields.as_ref().unwrap(),
                    motion: [0.; 4],
                    secondary_flags: 1,
                    affine: None,
                    animation_stamp: 0,
                },
            )
            .unwrap();
        assert!(unbound.prepared_for_primary(&s, owner, 1).is_err());
        let mut state = crate::mecc_subjective::State::for_session(&s, owner, [[0; 9]; 4]).unwrap();
        let mut p = Providers::new(&r);
        s.begin_mecc_frame_history(owner, 1).unwrap();
        state.prepare_from_session(1, &s, owner, 1).unwrap();
        assert!(s.mecc_subjective_reference_matrices(&state, 1).is_err());
        let env = crate::mecc_physical_cycle::Environment {
            number: 1,
            generation: 1,
            globals: &g,
            catalog: &defs.catalog,
            global_seconds: 1. / 60.,
            areas: &[],
            controls: [0; 2],
            facing_pitch: 0.,
            orientation: Some(crate::mecc_physical_cycle::Orientation {
                controls: [0; 2],
                control_factor: 1.,
                skip_heading: false,
                controlled_actor: true,
            }),
            actor_key: 1,
            objects: &[],
        };
        let mut primary = p.borrowed();
        primary.jet.input.buttons = 1;
        let result = s
            .advance_mecc_subjective_world(&mut state, env, ground, primary)
            .unwrap();
        assert!(result.primary.unwrap().fire.is_none());
        let body = s.get(owner).unwrap().seed.storage.data().unwrap();
        assert_eq!(&body[0x114..0x118], &25_f32.to_le_bytes());
        assert_eq!(&body[0x118..0x11c], &0x1234_u32.to_le_bytes());
        assert_eq!(&body[0x38..0x3c], &clock.to_le_bytes());
        assert_eq!(
            state.phase(),
            crate::mecc_subjective::Phase::PrimaryComplete
        );
        assert!(s.begin_mecc_frame_history(owner, 2).is_err());
        let buffers_before_post = s.position_buffer_count();
        assert!(
            s.prepare_mecc_subjective_positions(&state, 1, 0, false)
                .is_err()
        );
        assert_eq!(s.position_buffer_count(), buffers_before_post);
        let mut held = crate::mecc_wield_pose::Producer::new(owner);
        let camera = crate::first_person::Camera {
            position: [0., 0., 20.],
            angles: [0.; 3],
        };
        let mut cameras = crate::mecc_world_camera::State::new(owner, Some(camera));
        let mut camera_world = crate::mecc_camera_wall::World::new(&query, 1, &[]);
        assert!(
            cameras
                .outer(&s, 1, true, 1. / 60., &mut camera_world)
                .is_err()
        );
        let unknown = crate::mecc_world_camera::State::new(owner, None);
        assert!(unknown.check_inner(&s, owner, 1).is_err());
        let view = crate::mecc_primary::FireView {
            source: owner,
            number: 1,
            camera,
            viewport_origin: camera.position,
            viewport_forward: [0., 1., 0.],
            first_person: true,
            display: state.display(Some(owner)),
            vertex_epoch: p.stamp.0,
            force_vertices: false,
            input_mode: -1,
            fire_disabled: false,
            counter: 1,
        };
        let mut sound = Sound { fail: fail_sound };
        let mut history = crate::mecc_subjective::History {
            previous_weapon: -1,
        };
        {
            let mut post = |s: &mut Session,
                            state: &mut crate::mecc_subjective::State<Id>,
                            held: &crate::mecc_wield_pose::Producer| {
                s.advance_mecc_subjective_post(
                    state,
                    crate::mecc_subjective::WorldPost {
                        number: 1,
                        controlled: owner,
                        camera_owner: Some(owner),
                        previous_camera: camera,
                        current_camera: camera,
                    },
                    &mut history,
                    crate::mecc_subjective::WorldServices {
                        animation: crate::mecc_animation_phase::Services {
                            contexts: &mut p.contexts,
                            random: &mut p.random,
                            stamp: &mut p.stamp,
                            blending: Blending::Enabled,
                        },
                        fire: crate::mecc_primary::FireServices {
                            factory: &factory,
                            resources: &r,
                            terrain: &query,
                            view,
                            other: &mut sound,
                        },
                        held,
                        globals: &g,
                        catalog: &defs.catalog,
                        global_seconds: 1. / 60.,
                        areas: &[],
                    },
                )
            };
            assert!(post(&mut s, &mut state, &held).is_err());
            assert_eq!(
                state.phase(),
                crate::mecc_subjective::Phase::PrimaryComplete
            );
        }
        p.stamp.0 = p.stamp.0.wrapping_add(1).max(1);
        assert!(
            held.advance(&mut s, 1, 1. / 60., p.stamp.0)
                .unwrap()
                .is_some()
        );
        let before = (p.stamp.0, p.random.0, s.counts().residents);
        let services = crate::mecc_subjective::WorldServices {
            animation: crate::mecc_animation_phase::Services {
                contexts: &mut p.contexts,
                random: &mut p.random,
                stamp: &mut p.stamp,
                blending: Blending::Enabled,
            },
            fire: crate::mecc_primary::FireServices {
                factory: &factory,
                resources: &r,
                terrain: &query,
                view,
                other: &mut sound,
            },
            held: &held,
            globals: &g,
            catalog: &defs.catalog,
            global_seconds: 1. / 60.,
            areas: &[],
        };
        let result = if produced {
            s.advance_mecc_subjective_post_produced(
                &mut state,
                crate::mecc_subjective::ProducedWorldPost {
                    number: 1,
                    controlled: owner,
                    camera_owner: owner,
                },
                &mut history,
                services,
                &mut cameras,
                &mut camera_world,
            )
        } else {
            s.advance_mecc_subjective_post(
                &mut state,
                crate::mecc_subjective::WorldPost {
                    number: 1,
                    controlled: owner,
                    camera_owner: if skipped_camera_owner {
                        None
                    } else {
                        Some(owner)
                    },
                    previous_camera: camera,
                    current_camera: camera,
                },
                &mut history,
                services,
            )
        };
        if skipped_camera_owner {
            let result = result.unwrap();
            assert!(result.animation.is_none() && result.fire.is_none());
            assert_eq!(state.phase(), crate::mecc_subjective::Phase::Finished);
            assert_eq!(p.stamp.0, before.0);
            let buffers = s.position_buffer_count();
            assert!(
                s.prepare_mecc_subjective_positions(&state, 1, 0, false)
                    .is_err()
            );
            assert_eq!(s.position_buffer_count(), buffers);
            assert!(s.mecc_subjective_reference_matrices(&state, 1).is_err());
            s.begin_mecc_frame_history(owner, 2).unwrap();
            s.clear();
            continue;
        }
        if produced {
            let inner = cameras.last().unwrap();
            assert_eq!(inner.pass(), crate::mecc_world_camera::Pass::Inner);
            assert_eq!(inner.output().number, 1);
            assert!(cameras.outer_output(1).is_err());
        }
        assert_eq!(p.stamp.0, before.0 + 1);
        assert!(s.current_reference_matrices(owner).is_err());
        if fail_sound {
            assert!(result.is_err());
            assert_eq!(state.phase(), crate::mecc_subjective::Phase::Failed);
            assert!(s.mecc_physical_cycle_failure(owner).is_some());
            assert!(s.mecc_shot_failure(owner).is_some());
            assert!(s.mecc_subjective_reference_matrices(&state, 1).is_err());
            let buffers_after_fire = s.position_buffer_count();
            assert!(
                s.prepare_mecc_subjective_positions(&state, 1, 0, false)
                    .is_err()
            );
            assert_eq!(s.position_buffer_count(), buffers_after_fire);
            assert_eq!(s.counts().residents, before.2 + 1);
            assert!(!p.contexts.active().is_empty());
            assert_ne!(p.random.0, before.1);
            assert!(s.begin_mecc_frame_history(owner, 2).is_err());
            if produced {
                assert!(
                    cameras
                        .outer(&s, 1, true, 1. / 60., &mut camera_world)
                        .is_err()
                );
            }
        } else {
            let result = result.unwrap();
            assert_eq!(result.fire.unwrap().shot.is_some(), clock == 0.);
            assert_eq!(state.phase(), crate::mecc_subjective::Phase::Finished);
            let child = s
                .wield_list()
                .unwrap()
                .into_iter()
                .find(|c| c.parent == Some(owner) && c.reference == 1 && c.flags & 0x8000 == 0)
                .unwrap()
                .id;
            let binding = s.wield_binding(child).unwrap();
            let child_before = s.get(child).unwrap().channels;
            let frozen = held
                .current_world_point(&s, 1, factory.definition())
                .unwrap()
                .1;
            let virtual_pose =
                crate::mecc_wield_pose::sample_subjective(&s, &state, 1, child).unwrap();
            assert!(virtual_pose.world.values.iter().all(|v| v.is_finite()));
            assert!(
                virtual_pose
                    .point(&s, 1)
                    .unwrap()
                    .iter()
                    .all(|v| v.is_finite())
            );
            assert_eq!(s.wield_binding(child), Some(binding));
            assert_eq!(s.get(child).unwrap().channels, child_before);
            assert_eq!(
                held.current_world_point(&s, 1, factory.definition())
                    .unwrap()
                    .1,
                frozen
            );
            assert!(s.current_reference_matrices(owner).is_err());
            assert!(crate::mecc_wield_pose::sample_subjective(&s, &state, 2, child).is_err());
            assert!(crate::mecc_wield_pose::sample_subjective(&s, &state, 1, owner).is_err());
            let saved = s.specifications[&owner.serial].0.clone();
            s.update_current_references_at(owner, p.stamp.0 + 1)
                .unwrap();
            assert!(s.mecc_subjective_reference_matrices(&state, 1).is_err());
            s.specifications.get_mut(&owner.serial).unwrap().0 = saved;
            assert!(s.mecc_subjective_reference_matrices(&state, 1).is_ok());
            s.prepare_mecc_subjective_positions(&state, 1, 0, false)
                .unwrap();
            let positions = s
                .mecc_subjective_model_positions(&state, 1, 0)
                .unwrap()
                .unwrap()
                .to_vec();
            assert_eq!(positions.len(), 270);
            assert!(positions.iter().flatten().all(|x| x.is_finite()));
            let cache = s
                .mecc_subjective_model_cache(&state, 1, 0)
                .unwrap()
                .unwrap();
            assert_eq!(cache.epoch, state.header().unwrap().animation_stamp);
            assert_eq!(
                s.prepare_mecc_subjective_positions(&state, 1, 0, false)
                    .unwrap(),
                crate::actor_position_buffers::Update::KeepEpoch
            );
            s.prepare_mecc_subjective_positions(&state, 1, 0, true)
                .unwrap();
            let cache = s
                .mecc_subjective_model_cache(&state, 1, 0)
                .unwrap()
                .unwrap();
            assert_eq!(cache.epoch, 0);
            assert_eq!(cache.seconds, -1.);
            assert_eq!(
                s.mecc_subjective_model_positions(&state, 1, 0)
                    .unwrap()
                    .unwrap(),
                positions
            );
            assert!(s.current_reference_matrices(owner).is_err());
            let buffers = s.position_buffer_count();
            assert!(
                s.prepare_mecc_subjective_positions(&state, 2, 0, false)
                    .is_err()
            );
            assert_eq!(s.position_buffer_count(), buffers);
            let foreign = Session::new(980 + index as u64);
            assert!(
                foreign
                    .mecc_subjective_reference_matrices(&state, 1)
                    .is_err()
            );
            assert!(
                foreign
                    .mecc_subjective_model_positions(&state, 1, 0)
                    .is_err()
            );
            // Generic numeric completion cannot certify native Session post
            // publication, even when it starts from an actual source pre-copy.
            let mut numeric =
                crate::mecc_subjective::State::for_session(&s, owner, [[0; 9]; 4]).unwrap();
            numeric.prepare_from_session(1, &s, owner, 0).unwrap();
            let mut numeric_history = crate::mecc_subjective::History {
                previous_weapon: -1,
            };
            let mut numeric_stamp = crate::animation_epoch::Stamp(100);
            assert!(
                numeric
                    .post(
                        crate::mecc_subjective::Post {
                            controlled: owner,
                            camera_owner: Some(owner),
                            controlled_flags: 0,
                            icon_index: 0,
                            weapon: 1,
                            inventory_present: false,
                            previous_camera: camera,
                            current_camera: camera,
                            offset: [0.; 3],
                            seconds: 0.,
                            blending: Blending::Enabled,
                        },
                        &mut numeric_history,
                        &mut numeric_stamp,
                        &mut NumericBoundary
                    )
                    .unwrap()
                    .is_some()
            );
            assert_eq!(numeric.phase(), crate::mecc_subjective::Phase::Finished);
            assert!(s.mecc_subjective_reference_matrices(&numeric, 1).is_err());
            assert!(
                s.prepare_mecc_subjective_positions(&numeric, 1, 0, false)
                    .is_err()
            );
            assert_eq!(s.position_buffer_count(), buffers);
            assert!(
                s.residents[&owner.serial]
                    .mecc_physical_cycle
                    .as_ref()
                    .unwrap()
                    .subjective_pending
                    .is_none()
            );
            if produced {
                let publication = result.camera.unwrap();
                assert_eq!(publication.frame(), 1);
                let outer = cameras
                    .outer(&s, 1, true, 1. / 60., &mut camera_world)
                    .unwrap();
                assert_eq!(outer.output().number, 2);
                assert_eq!(cameras.outer_output(1).unwrap().number, 2);
                let publication = cameras.last().unwrap().clone();
                let old_stamp = s.get(owner).unwrap().mecc_animation_stamp;
                let old_channels = s.get(owner).unwrap().channels;
                for slot in 0..10 {
                    let draw = crate::mecc_model_draw::prepare(
                        &mut s,
                        &publication,
                        crate::mecc_model_draw::Settings {
                            quality: 2,
                            forced: slot,
                        },
                    )
                    .unwrap();
                    assert_eq!(draw.source(), owner);
                    assert_eq!(draw.number(), 1);
                    assert_eq!(draw.slot(), slot as usize);
                    assert_eq!(
                        draw.positions().map(<[_]>::len),
                        draw.model().map(|m| m.prepared.mesh.positions.len())
                    );
                }
                assert_eq!(s.get(owner).unwrap().mecc_animation_stamp, old_stamp);
                assert_eq!(s.get(owner).unwrap().channels, old_channels);
                assert!(s.current_reference_matrices(owner).is_err());
                let before = s.position_buffer_count();
                assert!(
                    crate::mecc_model_draw::prepare(
                        &mut s,
                        &publication,
                        crate::mecc_model_draw::Settings {
                            quality: 4,
                            forced: -1
                        }
                    )
                    .is_err()
                );
                assert!(
                    crate::mecc_model_draw::prepare(
                        &mut s,
                        &publication,
                        crate::mecc_model_draw::Settings {
                            quality: 2,
                            forced: 10
                        }
                    )
                    .is_err()
                );
                assert_eq!(s.position_buffer_count(), before);
                assert!(
                    crate::mecc_model_draw::prepare(
                        &mut Session::new(999),
                        &publication,
                        crate::mecc_model_draw::Settings::default()
                    )
                    .is_err()
                );
                assert!(
                    cameras
                        .outer(&s, 1, true, 1. / 60., &mut camera_world)
                        .is_err()
                );
            }
            s.begin_mecc_frame_history(owner, 2).unwrap();
            if produced {
                assert!(
                    crate::mecc_model_draw::prepare(
                        &mut s,
                        cameras.last().unwrap(),
                        crate::mecc_model_draw::Settings::default()
                    )
                    .is_err()
                );
            }
        }
        let before_replay = (
            p.stamp.0,
            p.random.0,
            s.counts().residents,
            p.contexts.active().len(),
        );
        assert!(
            s.advance_mecc_subjective_post(
                &mut state,
                crate::mecc_subjective::WorldPost {
                    number: 1,
                    controlled: owner,
                    camera_owner: Some(owner),
                    previous_camera: camera,
                    current_camera: camera
                },
                &mut history,
                crate::mecc_subjective::WorldServices {
                    animation: crate::mecc_animation_phase::Services {
                        contexts: &mut p.contexts,
                        random: &mut p.random,
                        stamp: &mut p.stamp,
                        blending: Blending::Enabled
                    },
                    fire: crate::mecc_primary::FireServices {
                        factory: &factory,
                        resources: &r,
                        terrain: &query,
                        view,
                        other: &mut sound
                    },
                    held: &held,
                    globals: &g,
                    catalog: &defs.catalog,
                    global_seconds: 1. / 60.,
                    areas: &[]
                }
            )
            .is_err()
        );
        assert_eq!(
            (
                p.stamp.0,
                p.random.0,
                s.counts().residents,
                p.contexts.active().len()
            ),
            before_replay
        );
        s.clear();
        assert!(s.mecc_subjective_model_positions(&state, 1, 0).is_err());
        assert!(s.mecc_subjective_reference_matrices(&state, 1).is_err());
        assert_eq!(s.position_buffer_count(), 0);
        assert!(
            cameras
                .outer(&s, 2, true, 1. / 60., &mut camera_world)
                .is_err()
        );
        assert!(state.prepare_from_session(2, &s, owner, 0).is_err());
    }
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
