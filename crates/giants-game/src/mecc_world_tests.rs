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
            impact: None,
            impact_voice: None,
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

#[test]
#[ignore = "Requires GIANTS_MECC_WORLD_ROOT; ordinary contact queue ownership only, no active FX or GPU"]
fn source_mecc_contact_effect_preserves_prior_state_delay_and_stale_binding_refusals() {
    let root = std::path::PathBuf::from(std::env::var("GIANTS_MECC_WORLD_ROOT").unwrap());
    let r = crate::scene_resources::SceneResources::load(&root, "w_Story1.bin", "English").unwrap();
    let g = giants_formats::global_data::parse(&std::fs::read(root.join("Bin/GData.gbn")).unwrap())
        .unwrap();
    let c = giants_formats::weapons::parse_with_profile(
        &std::fs::read(root.join("Bin/weapondefs.bin")).unwrap(),
        giants_formats::weapons::Profile::Debug,
    )
    .unwrap();
    let mut s = Session::new(913);
    let source = actor(&mut s, &r, &g, &c, None, 0.);
    let other = actor(&mut s, &r, &g, &c, None, 10.);
    let mut fields = s
        .get(source)
        .unwrap()
        .construction_fields
        .as_ref()
        .unwrap()
        .clone();
    fields.object_flags = 0;
    fields.response_flags = 0;
    fields.control_flags = 0;
    fields.configuration[17] = 0;
    fields.scale = 1.;
    fields.position[2] = 15.;
    fields.channels = [[0; 9]; 4];
    fields.channels[0][0] = 41;
    let mut payload = vec![0; 524];
    for (at, value) in [(0x30, 2u32), (0x60, 1), (0x180, 1), (0x84, 2)] {
        payload[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }
    let mut view = crate::mecc_physical_cycle::View {
        number: 1,
        local_seconds: 1. / 60.,
        velocity: [0., 0., -15.],
        previous_velocity: [0., 0., -15.],
        model_rates: [-15., 15., 0.],
        previous_model_rates: [0.; 4],
        angular_rate: 0.,
        look_pitch: 0.,
        commands: [0; 8],
        floor: 10.,
        plane: [0., 0., 1., -10.],
        radius: 1.,
        hit: 0,
        support: 0,
        support_part: 0,
        support_triangle: 0,
        affine: None,
    };
    let mut pool = crate::attached_effect_context::Pool::from_active_resources(913, &r);
    let mut contact = crate::mecc_secondary::Contact::default();
    let mut random = crate::audio_parameters::Random(1);
    let run = |fields: &crate::actor_fields::Fields,
               payload: &[u8],
               view: &crate::mecc_physical_cycle::View,
               source,
               contact: &mut crate::mecc_secondary::Contact,
               pool: &mut crate::attached_effect_context::Pool<Id>,
               random: &mut crate::audio_parameters::Random| {
        let mut checkpoint = None;
        crate::mecc_secondary::ordinary(
            fields,
            payload,
            fields.channels,
            view,
            &g,
            1. / 60.,
            crate::mecc_secondary::Services {
                impact: None,

                fire: false,
                blending: Blending::Disabled,
                random,
                contact: Some(crate::mecc_secondary::ContactServices {
                    state: contact,
                    pool,
                    source,
                    checkpoint: &mut checkpoint,
                }),
            },
        )
    };
    let entered = run(
        &fields,
        &payload,
        &view,
        source,
        &mut contact,
        &mut pool,
        &mut random,
    )
    .unwrap();
    assert_eq!(entered.request.unwrap().slot, 181);
    assert!(pool.active().is_empty());
    assert_eq!(random.0, 1);
    let prepared = run(
        &entered.fields,
        &entered.payload,
        &view,
        source,
        &mut contact,
        &mut pool,
        &mut random,
    )
    .unwrap();
    assert!(prepared.request.is_none());
    assert_eq!(pool.active().len(), 1);
    let handle = pool.active()[0];
    let mut stale_contact = contact.clone();
    assert_eq!(pool.get(handle).unwrap().recipient(), Some(source));
    assert!(pool.get(handle).unwrap().require_first_update().is_err());
    let mut expected = crate::audio_parameters::Random(1);
    for _ in 0..3 {
        expected.next_15();
    }
    assert_eq!(random.0, expected.0);
    assert!(
        run(
            &prepared.fields,
            &prepared.payload,
            &view,
            other,
            &mut contact,
            &mut pool,
            &mut random
        )
        .is_err()
    );
    assert_eq!(pool.active(), [handle]);
    assert_eq!(random.0, expected.0);
    view.velocity[2] = 0.;
    view.previous_velocity[2] = 0.;
    let mut body = prepared.payload.clone();
    body[0x84..0x88].fill(0);
    let exited = run(
        &prepared.fields,
        &body,
        &view,
        source,
        &mut contact,
        &mut pool,
        &mut random,
    )
    .unwrap();
    assert!(pool.get(handle).is_some()); // previous logical60 retains it for this call
    let cleaned = run(
        &exited.fields,
        &exited.payload,
        &view,
        source,
        &mut contact,
        &mut pool,
        &mut random,
    )
    .unwrap();
    assert!(pool.active().is_empty());
    assert_eq!(&cleaned.payload[0x1a8..0x1ac], &[0; 4]);
    assert!(
        run(
            &prepared.fields,
            &prepared.payload,
            &view,
            source,
            &mut stale_contact,
            &mut pool,
            &mut random,
        )
        .is_err()
    );
    assert!(
        run(
            &prepared.fields,
            &prepared.payload,
            &view,
            source,
            &mut contact,
            &mut pool,
            &mut random
        )
        .is_err()
    );
    assert_eq!(random.0, expected.0);
}

#[test]
#[ignore = "Requires GIANTS_MECC_WORLD_ROOT; actual owned impact health/context boundary, no window or audio"]
fn source_mecc_impact_secondary_orders_owned_health_context_and_channel() {
    let root = std::path::PathBuf::from(std::env::var("GIANTS_MECC_WORLD_ROOT").unwrap());
    let r = crate::scene_resources::SceneResources::load(&root, "w_Story1.bin", "English").unwrap();
    let g = giants_formats::global_data::parse(&std::fs::read(root.join("Bin/GData.gbn")).unwrap())
        .unwrap();
    let c = giants_formats::weapons::parse_with_profile(
        &std::fs::read(root.join("Bin/weapondefs.bin")).unwrap(),
        giants_formats::weapons::Profile::Debug,
    )
    .unwrap();
    for (contact, carrying, current, previous, expected) in [
        (false, false, 0., -100., 61),
        (false, false, 0., -145., 62),
        (true, false, -25., -125., 60),
        (false, true, 0., -145., 62),
    ] {
        let mut session = Session::new(810);
        let source = actor(&mut session, &r, &g, &c, None, 0.);
        let mut fields = session
            .get(source)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap()
            .clone();
        fields.position = [0., 0., 15.];
        fields.current = [-17., 9., 11.]; // Acquired kind16 excludes directional response.
        fields.object_flags = 0;
        fields.response_flags = if carrying { 0x800000 } else { 0 };
        fields.control_flags = 0x1000;
        fields.channels[0][0] = 41;
        let channels = fields.channels;
        let mut body = session
            .get(source)
            .unwrap()
            .seed
            .storage
            .data()
            .unwrap()
            .to_vec();
        for (at, value) in [
            (0x30, 11u32),
            (0x60, 1),
            (0x180, 1),
            (0x80, 0),
            (0x1a8, 0),
            (0x1dc, 0),
            (0x84, if contact { 2 } else { 0 }),
        ] {
            body[at..at + 4].copy_from_slice(&value.to_le_bytes());
        }
        let mut view = session.mecc_physical_cycle(source).unwrap().clone();
        view.velocity = [0., 0., current];
        view.previous_velocity = [0., 0., previous];
        view.floor = 10.;
        view.model_rates = [0.; 3];
        let mut pool = crate::attached_effect_context::Pool::from_active_resources(810, &r);
        let mut random = crate::audio_parameters::Random(1);
        let mut state = crate::mecc_secondary::Contact::default();
        let mut checkpoint = None;
        let counts = session.counts();
        let mut damage = super::mecc_impact::Damage {
            voice: None,
            session: &mut session,
            source,
            context: crate::mecc_impact::Context {
                offline: true,
                damage: MeccDamageContext {
                    globally_disabled: false,
                    cutscene: false,
                    shield_percent: 90.,
                    ambient_voice_timer: 0.,
                },
            },
        };
        let result = crate::mecc_secondary::ordinary(
            &fields,
            &body,
            channels,
            &view,
            &g,
            1. / 60.,
            crate::mecc_secondary::Services {
                fire: false,
                blending: Blending::Disabled,
                random: &mut random,
                impact: Some(&mut damage),
                contact: Some(crate::mecc_secondary::ContactServices {
                    state: &mut state,
                    pool: &mut pool,
                    source,
                    checkpoint: &mut checkpoint,
                }),
            },
        );
        let health = session.mecc_health(source).unwrap();
        let classification = crate::mecc_impact::classify(
            view.velocity,
            view.previous_velocity,
            crate::mecc_impact::Parameters::from_globals(&g).unwrap(),
        )
        .unwrap();
        assert_eq!(
            health.damage.to_bits(),
            classification.amount.unwrap().to_bits()
        );
        assert_eq!(session.counts(), counts); // Preparation emits no effect actor.
        assert_eq!(session.get(source).unwrap().channels.unwrap()[0][0], 41); // Health before returned channel publication.
        assert_eq!(pool.active().len(), usize::from(!contact));
        let mut expected_random = crate::audio_parameters::Random(1);
        if !contact {
            for _ in 0..3 {
                expected_random.0 = expected_random.0.wrapping_mul(214013).wrapping_add(2531011);
            }
        }
        assert_eq!(random.0, expected_random.0);
        if let Some(id) = pool.active().first() {
            let context = pool.get(*id).unwrap();
            assert_eq!(context.position(), Some(fields.position));
            assert_eq!(context.flags(), 0);
        }
        if carrying {
            assert!(result.err().unwrap().contains("carried actor"));
            assert!(checkpoint.is_some());
        } else {
            let output = result.unwrap();
            assert_eq!(
                u32::from_le_bytes(output.payload[0x30..0x34].try_into().unwrap()),
                expected
            );
            assert_eq!(
                output.request.unwrap().slot,
                match expected {
                    60 => 181,
                    61 => 175,
                    _ => 178,
                }
            );
            assert_eq!(output.payload[0xf4..0xf8], u32::MAX.to_le_bytes());
        }
        session.clear();
    }
}

#[test]
#[ignore = "Requires GIANTS_MECC_WORLD_ROOT; owned falling-entry impact health prefix and retained voice-service refusal"]
fn source_mecc_impact_primary_retains_health_before_required_voice_and_quarantines_replay() {
    let root = std::path::PathBuf::from(std::env::var("GIANTS_MECC_WORLD_ROOT").unwrap());
    let r = crate::scene_resources::SceneResources::load(&root, "w_Story1.bin", "English").unwrap();
    let g = giants_formats::global_data::parse(&std::fs::read(root.join("Bin/GData.gbn")).unwrap())
        .unwrap();
    let c = giants_formats::weapons::parse_with_profile(
        &std::fs::read(root.join("Bin/weapondefs.bin")).unwrap(),
        giants_formats::weapons::Profile::Debug,
    )
    .unwrap();
    let mut s = Session::new(810);
    let id = actor(&mut s, &r, &g, &c, None, 0.);
    s.prepare_mecc_countdown_world(10.).unwrap();
    s.bind_mecc_contact_materials(
        crate::mecc_contact_materials::Catalog::from_prepared(&[], Default::default()).unwrap(),
    )
    .unwrap();
    let resident = s.residents.get_mut(&id.serial).unwrap();
    let f = resident.construction_fields.as_mut().unwrap();
    f.position = [0., 0., 5.];
    f.current = f.position;
    f.previous = f.position;
    resident.mecc_physical_cycle.as_mut().unwrap().view.velocity = [0., 0., -145.];
    let mut p = Providers::new(&r);
    let context = crate::mecc_impact::Context {
        offline: true,
        damage: MeccDamageContext {
            globally_disabled: false,
            cutscene: false,
            shield_percent: 0.,
            ambient_voice_timer: 0.,
        },
    };
    let env = |number| crate::mecc_physical_cycle::Environment {
        number,
        generation: 1,
        globals: &g,
        catalog: &c,
        global_seconds: 0.1,
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
    s.begin_mecc_frame_history(id, 1).unwrap();
    let mut services = p.borrowed();
    services.impact = Some(context);
    let result = s.advance_mecc_world_offline(id, env(1), Some(id), ground, services);
    eprintln!(
        "owned_impact_primary_entry result={:?} health={:?} contexts={} rng={}",
        result.as_ref().map(|_| ()),
        s.mecc_health(id).unwrap(),
        p.contexts.active().len(),
        p.random.0
    );
    let error = result
        .err()
        .expect("actual pain voice remains a required service");
    assert!(error.contains("health committed; actual pain voice"));
    assert!(s.mecc_health(id).unwrap().damage > 0.);
    assert!(p.contexts.active().is_empty());
    assert_eq!(p.random.0, 1); // Voice must precede its timer draw and hard-land preparation.
    let health = s.mecc_health(id).unwrap().damage;
    assert_eq!(s.actor_world_tick(id).unwrap().phase(), Phase::Failed);
    assert!(s.begin_mecc_frame_history(id, 2).is_err());
    assert_eq!(s.mecc_health(id).unwrap().damage, health);
    assert_eq!(p.random.0, 1);
    assert!(p.contexts.active().is_empty());
}

#[test]
#[ignore = "Requires GIANTS_MECC_WORLD_ROOT; owned damage/voice publication with explicit cue provider, no window or device"]
fn source_mecc_impact_voice_publishes_health_and_retains_late_timer_failure() {
    struct Voice {
        calls: Vec<(u32, f32, f32)>,
        fail_at: usize,
    }
    impl MeccImpactVoice for Voice {
        fn play(&mut self, session: &Session, source: Id, sound: u32) -> Result<(), String> {
            let health = session.mecc_health(source)?;
            let payload = session.get(source).unwrap().seed.storage.data().unwrap();
            let timer = f32::from_le_bytes(payload[0x1fc..0x200].try_into().unwrap());
            self.calls.push((sound, health.damage, timer));
            if self.calls.len() == self.fail_at {
                Err("required cue failed after observing owned state".into())
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
    for (fail_at, start_damage) in [(0, 55.), (1, 0.), (2, 0.)] {
        let mut session = Session::new(912);
        let source = actor(&mut session, &r, &g, &c, None, 0.);
        let resident = session.residents.get_mut(&source.serial).unwrap();
        resident.mecc_damage = Some(start_damage);
        resident.seed.storage.data_mut().unwrap()[0x1fc..0x200]
            .copy_from_slice(&(-0.1f32).to_le_bytes());
        let context = crate::mecc_impact::Context {
            offline: true,
            damage: MeccDamageContext {
                globally_disabled: false,
                cutscene: false,
                shield_percent: 0.,
                ambient_voice_timer: -1.,
            },
        };
        session.prepare_mecc_countdown_world(-1.).unwrap();
        let mut random = crate::audio_parameters::Random(42);
        let mut voice = Voice {
            calls: Vec::new(),
            fail_at,
        };
        let counts = session.counts();
        let outcome =
            session.damage_mecc_impact_nonlethal(source, 1., context, &mut random, &mut voice);
        assert_eq!(outcome.is_err(), fail_at != 0);
        assert_eq!(
            session.mecc_health(source).unwrap().damage,
            start_damage + 1.
        );
        assert_eq!(session.counts(), counts);
        assert_eq!(
            voice.calls[0],
            (
                if start_damage == 0. { 1504 } else { 1505 },
                start_damage + 1.,
                -0.1
            )
        );
        let mut expected = crate::audio_parameters::Random(42);
        let timer = if fail_at == 1 {
            -0.1
        } else {
            let timer = (f64::from(expected.next_15()) / 32768. + 1.) as f32;
            let cue = 1683 + ((expected.next_15() * 100) >> 15) / 25;
            assert_eq!(voice.calls[1], (cue, start_damage + 1., timer));
            timer
        };
        let payload = session.get(source).unwrap().seed.storage.data().unwrap();
        assert_eq!(
            f32::from_le_bytes(payload[0x1fc..0x200].try_into().unwrap()),
            timer
        );
        assert_eq!(
            session.mecc_countdown_world().unwrap(),
            if fail_at == 0 { 10. } else { -1. }
        );
        assert_eq!(random.0, expected.0);
        let damage = session.mecc_health(source).unwrap().damage;
        session.mecc_countdown_world = Some(99.);
        assert!(
            session
                .damage_mecc_impact_nonlethal(source, 1., context, &mut random, &mut voice,)
                .is_err()
        );
        assert_eq!(session.mecc_health(source).unwrap().damage, damage);
        let mut foreign = Session::new(913);
        assert!(
            foreign
                .damage_mecc_impact_nonlethal(source, 1., context, &mut random, &mut voice,)
                .is_err()
        );
        session.clear();
        assert!(session.get(source).is_none());
    }
}

#[test]
#[ignore = "Requires GIANTS_MECC_WORLD_ROOT; real primary/impact voice composition with supplied cues, no device"]
fn source_mecc_impact_primary_voice_uses_countdown_owner_and_retains_failed_frame() {
    struct Voice {
        count: usize,
        fail_at: usize,
    }
    impl MeccImpactVoice for Voice {
        fn play(&mut self, session: &Session, source: Id, sound: u32) -> Result<(), String> {
            self.count += 1;
            assert_eq!(session.mecc_health(source)?.damage, 15.);
            assert!(session.mecc_countdown_world().unwrap() < 0.);
            assert_eq!(sound == 1504, self.count == 1);
            if self.count == self.fail_at {
                Err("controlled actual cue-entry failure".into())
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
    for fail_at in [1, 2, 0] {
        let mut session = Session::new(810);
        let source = actor(&mut session, &r, &g, &c, None, 0.);
        session.prepare_mecc_countdown_world(-1.).unwrap();
        session
            .bind_mecc_contact_materials(
                crate::mecc_contact_materials::Catalog::from_prepared(&[], Default::default())
                    .unwrap(),
            )
            .unwrap();
        let actor = session.residents.get_mut(&source.serial).unwrap();
        let fields = actor.construction_fields.as_mut().unwrap();
        fields.position = [0., 0., 5.];
        fields.current = fields.position;
        fields.previous = fields.position;
        actor.mecc_physical_cycle.as_mut().unwrap().view.velocity = [0., 0., -145.];
        let mut providers = Providers::new(&r);
        let mut voice = Voice { count: 0, fail_at };
        let context = crate::mecc_impact::Context {
            offline: true,
            damage: MeccDamageContext {
                globally_disabled: false,
                cutscene: false,
                shield_percent: 0.,
                ambient_voice_timer: 999.,
            }, // Caller snapshot is not the live owner.
        };
        let env = |number| crate::mecc_physical_cycle::Environment {
            number,
            generation: 1,
            globals: &g,
            catalog: &c,
            global_seconds: 0.1,
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
        session.begin_mecc_frame_history(source, 1).unwrap();
        let mut services = providers.borrowed();
        services.impact = Some(context);
        services.impact_voice = Some(&mut voice);
        let outcome =
            session.advance_mecc_world_offline(source, env(1), Some(source), ground, services);
        eprintln!(
            "impact_voice_primary fail_at={fail_at} outcome={:?} cue_entries={} contexts={}",
            outcome.as_ref().map(|_| ()),
            voice.count,
            providers.contexts.active().len()
        );
        if fail_at != 0 {
            assert!(
                outcome
                    .err()
                    .expect("controlled cue must fail")
                    .contains("controlled actual cue-entry failure")
            );
            assert_eq!(
                session.actor_world_tick(source).unwrap().phase(),
                Phase::Failed
            );
            assert!(session.begin_mecc_frame_history(source, 2).is_err());
            assert_eq!(session.mecc_health(source).unwrap().damage, 15.);
            assert_eq!(voice.count, fail_at);
            assert!(providers.contexts.active().is_empty());
            assert!(session.mecc_countdown_world().unwrap() < 0.);
        } else {
            outcome.unwrap();
            assert_eq!(voice.count, 2);
            assert_eq!(session.mecc_countdown_world(), Some(10.));
            assert_eq!(providers.contexts.active().len(), 1);
        }
        session.clear();
    }
}

#[test]
#[ignore = "Requires GIANTS_MECC_WORLD_ROOT; prepared damage smoke ownership, no active effect/window/audio"]
fn source_mecc_damage_smoke_owns_reuse_release_and_late_failure() {
    use crate::mecc_damage_effect::{Input, Prepared, State};
    let root = std::path::PathBuf::from(std::env::var("GIANTS_MECC_WORLD_ROOT").unwrap());
    let r = crate::scene_resources::SceneResources::load(&root, "w_Story1.bin", "English").unwrap();
    let g = giants_formats::global_data::parse(&std::fs::read(root.join("Bin/GData.gbn")).unwrap())
        .unwrap();
    let c = giants_formats::weapons::parse_with_profile(
        &std::fs::read(root.join("Bin/weapondefs.bin")).unwrap(),
        giants_formats::weapons::Profile::Debug,
    )
    .unwrap();
    let mut session = Session::new(851);
    let source = actor(&mut session, &r, &g, &c, None, 0.);
    let mut pool = crate::attached_effect_context::Pool::from_active_resources(851, &r);
    let mut random = crate::audio_parameters::Random(1);
    let mut state = State::default();
    let input = |damage| Input::from_globals(damage, 100., &g).unwrap();
    for (damage, level) in [(55., 1), (75., 2), (95., 3)] {
        let before = state.binding;
        state
            .advance(
                input(damage),
                &mut Prepared {
                    reset: false,
                    pool: &mut pool,
                    source,
                    source_flags: 0,
                    random: &mut random,
                },
            )
            .unwrap();
        assert_eq!(state.level, level);
        let id = state.binding.unwrap();
        assert_eq!(pool.active(), [id]);
        let context = pool.get(id).unwrap();
        assert_eq!(context.recipient(), Some(source));
        assert_eq!(context.tag(), Some(3));
        assert_eq!(context.flags(), 0x20004);
        if let Some(old) = before {
            assert!(pool.get(old).is_none());
        }
        let seed = random.0;
        state
            .advance(
                input(damage),
                &mut Prepared {
                    reset: false,
                    pool: &mut pool,
                    source,
                    source_flags: 0,
                    random: &mut random,
                },
            )
            .unwrap();
        assert_eq!(state.binding, Some(id));
        assert_eq!(random.0, seed);
        if level == 1 {
            assert_eq!(seed, 2745024);
        }
    }
    let old = state.binding.unwrap();
    state
        .advance(
            input(95.),
            &mut Prepared {
                reset: true,
                pool: &mut pool,
                source,
                source_flags: 0,
                random: &mut random,
            },
        )
        .unwrap();
    assert!(pool.get(old).is_none());
    assert_eq!(pool.active(), [state.binding.unwrap()]);
    let current = state.binding.unwrap();
    let seed = random.0;
    let mut foreign_session = Session::new(852);
    let foreign = actor(&mut foreign_session, &r, &g, &c, None, 0.);
    assert!(
        state
            .advance(
                input(95.),
                &mut Prepared {
                    reset: false,
                    pool: &mut pool,
                    source: foreign,
                    source_flags: 0,
                    random: &mut random
                }
            )
            .is_err()
    );
    assert_eq!(state.binding, Some(current));
    assert_eq!(random.0, seed);
    // A required inherited-parameter producer is absent after the real old release.
    assert!(
        state
            .advance(
                input(55.),
                &mut Prepared {
                    reset: false,
                    pool: &mut pool,
                    source,
                    source_flags: 0x400,
                    random: &mut random
                }
            )
            .is_err()
    );
    assert_eq!(state.level, 1);
    assert!(state.binding.is_none());
    assert!(pool.active().is_empty());
    assert_eq!(random.0, seed);
    state
        .advance(
            input(55.),
            &mut Prepared {
                reset: false,
                pool: &mut pool,
                source,
                source_flags: 0,
                random: &mut random,
            },
        )
        .unwrap();
    let current = state.binding.unwrap();
    pool.release_prepared(current).unwrap();
    let seed = random.0;
    assert!(
        state
            .advance(
                input(55.),
                &mut Prepared {
                    reset: false,
                    pool: &mut pool,
                    source,
                    source_flags: 0,
                    random: &mut random
                }
            )
            .is_err()
    );
    assert_eq!(random.0, seed);
    assert_eq!(state.binding, Some(current));
    // Context cancellation belongs to the enclosing owner; this test has no actor birth.
    assert_eq!(session.mecc_health(source).unwrap().damage, 0.);
    session.clear();
}
