use super::*;

#[test]
fn phase_checks_frame_consumption_and_all_typed_reference_slots() {
    let id = |serial| Id {
        ticket: 1400,
        serial,
    };
    let mut view = View {
        number: 3,
        phase: Phase::Prepared,
        failure: None,
        state: target::State {
            action_available: 0,
            item_available: 0,
            movement_ready: 1,
            point_available: 0,
            highlighted: None,
            item_target: None,
            terrain_selected: 0,
            cooldown: 0.,
            controls: 0,
            proxy: None,
            candidate: None,
            pending: None,
            point: [0.; 3],
            terrain_point: [0.; 3],
            aim_point: [0.; 3],
        },
    };
    assert!(view.check_physical(3).is_ok());
    assert!(view.check_physical(4).is_err());
    view.phase = Phase::Consumed;
    assert!(view.check_physical(3).is_err());
    view.phase = Phase::Failed;
    view.state.candidate = Some(id(1));
    view.state.highlighted = Some(id(2));
    view.state.pending = Some(id(3));
    view.state.item_target = Some(id(4));
    view.state.proxy = Some(id(5));
    for serial in 1..=5 {
        assert!(view.references(id(serial)));
    }
    assert!(!view.references(Id {
        ticket: 1401,
        serial: 1
    }));
    view.phase = Phase::Prepared;
    assert!(
        view.check_physical(3)
            .unwrap_err()
            .contains("relationship consumers")
    );
}

#[test]
fn signed_selection_reads_preserve_adjacent_words_and_refuse_out_of_storage() {
    let words: Vec<u8> = [100_u32, 200, 300]
        .into_iter()
        .flat_map(u32::to_le_bytes)
        .collect();
    assert_eq!(indexed(&words, 8, -1, 4).unwrap(), 200);
    assert!(indexed(&words, 8, -3, 4).is_err());
    assert!(indexed(&words, 8, i32::MAX, 12).is_err());
}

fn setup(
    ticket: u64,
) -> (
    Session,
    Id,
    crate::scene_resources::SceneResources,
    giants_formats::global_data::GlobalData,
    crate::delphi_special::Factory,
) {
    let root = std::path::PathBuf::from(std::env::var("GIANTS_DELPHI_ROOT").unwrap());
    let source =
        crate::scene_resources::SceneResources::load(&root, "w_Rstory1.bin", "English").unwrap();
    let globals =
        giants_formats::global_data::parse(&std::fs::read(root.join("Bin/GData.gbn")).unwrap())
            .unwrap();
    let weapons = giants_formats::weapons::parse_with_profile(
        &std::fs::read(root.join("Bin/weapondefs.bin")).unwrap(),
        giants_formats::weapons::Profile::Debug,
    )
    .unwrap();
    let factory = crate::delphi_special::Factory::load(&source, &globals, &weapons).unwrap();
    let mut session = Session::new(ticket);
    let owner = make(&mut session, &factory);
    prepare(&mut session, owner, &source, &globals);
    (session, owner, source, globals, factory)
}
fn make(session: &mut Session, factory: &crate::delphi_special::Factory) -> Id {
    factory
        .construct_player_fields(
            session,
            crate::delphi_player::ColdFrame {
                position: [0., 0., 10.],
                current: [0., 0., 10.],
                previous: [0., 0., 10.],
                angles: [0.; 3],
            },
            crate::animation_channels::Blending::Disabled,
        )
        .unwrap()
        .owner
}
fn prepare(
    session: &mut Session,
    owner: Id,
    source: &crate::scene_resources::SceneResources,
    g: &giants_formats::global_data::GlobalData,
) {
    session.initialize_delphi_physics(owner, g).unwrap();
    let root = std::path::PathBuf::from(std::env::var("GIANTS_DELPHI_ROOT").unwrap());
    let exe = std::env::var_os("GIANTS_DELPHI_EXECUTABLE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| root.join("Giants.exe"));
    let resources = crate::delphi_physical::Resources::load_locomotion(
        source,
        session.specification(owner).unwrap().clone(),
        Arc::new(
            crate::mecc_contact_materials::Catalog::from_prepared(&[], Default::default()).unwrap(),
        ),
        &exe,
    )
    .unwrap()
    .with_manual_rotation()
    .unwrap()
    .with_post_physical()
    .unwrap()
    .with_reserve()
    .unwrap()
    .with_primary_tail()
    .unwrap()
    .with_local_target_phase()
    .unwrap();
    session.prepare_delphi_physical(owner, resources).unwrap();
}
fn context(owner: Id, number: u64, g: &giants_formats::global_data::GlobalData) -> Context<'_> {
    Context {
        number,
        current_player: Some(owner),
        suppressed: false,
        terrain_selected: 99,
        globals: g,
        exponent: 0.25,
    }
}
fn word(s: &Session, id: Id, at: usize) -> u32 {
    crate::delphi_physical::word(s.get(id).unwrap().seed.storage.data().unwrap(), at)
}
#[derive(Default)]
struct Plan {
    target: Option<Id>,
    fail: Option<&'static str>,
    clear: bool,
    binds: usize,
    events: Vec<&'static str>,
    proxy: Option<crate::delphi_target_proxy::Template>,
    geometry: Option<Arc<crate::scene_resources::SceneResources>>,
    ray_samples: Vec<crate::delphi_target_ray::Sample>,
    geometry_outputs: Vec<crate::delphi_target_owned::Output>,
    terrain: Option<Arc<giants_formats::gti::Terrain>>,
    terrain_points: Vec<[f32; 3]>,
    terrain_distances: Vec<f32>,
}
struct Bound<'a> {
    session: &'a mut Session,
    owner: Id,
    plan: &'a mut Plan,
}
impl Providers for Plan {
    type Bound<'a> = Bound<'a>;
    fn bind<'a>(
        &'a mut self,
        session: &'a mut Session,
        owner: Id,
        _: u64,
    ) -> Result<Self::Bound<'a>, String> {
        self.binds += 1;
        Ok(Bound {
            session,
            owner,
            plan: self,
        })
    }
}
impl Bound<'_> {
    fn event(&mut self, label: &'static str) -> Result<(), String> {
        self.plan.events.push(label);
        if self.plan.fail == Some(label) {
            Err(format!("required {label} failure"))
        } else {
            Ok(())
        }
    }
    fn position(&self) -> [f32; 3] {
        self.session
            .get(self.owner)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap()
            .position
    }
}
impl target::World<Id, Id> for Bound<'_> {
    fn seconds(&mut self) -> Result<f64, String> {
        self.event("seconds")?;
        Ok(1. / 60.)
    }
    fn hide_proxy(&mut self, id: Id) -> Result<(), String> {
        self.event("hide")?;
        self.session.hide_delphi_target_proxy(id)
    }
    fn projectile_range(&mut self, _: i32) -> Result<f32, String> {
        self.event("range")?;
        Ok(100.)
    }
    fn viewport_route(&mut self) -> Result<bool, String> {
        self.event("view")?;
        Ok(self.plan.terrain.is_none())
    }
    fn terrain(
        &mut self,
        _: bool,
        range: f32,
        point: &mut [f32; 3],
    ) -> Result<target::Terrain, String> {
        assert_eq!(
            self.session.delphi_local_target(self.owner).unwrap().phase,
            Phase::Running
        );
        if let Some(terrain) = self.plan.terrain.clone() {
            self.event("terrain")?;
            let query = crate::terrain_queries::StaticTerrainQuery::new(
                &terrain,
                1. / terrain.spacing,
                crate::arithmetic::Precision::Double53,
            )?;
            let number = self.session.delphi_local_target(self.owner).unwrap().number;
            let (_, result) = self.session.trace_delphi_prephysical_held_terrain(
                crate::delphi_target_scan::HeldTerrainInput {
                    owner: self.owner,
                    number,
                    normalization_threshold: 1e-4,
                    range,
                },
                &query,
                point,
            )?;
            self.plan.terrain_points.push(*point);
            self.plan.terrain_distances.push(result.distance);
            return Ok(result);
        }
        *point = self.position();
        point[2] += 1.;
        if self.plan.clear {
            self.session.clear();
        }
        self.event("terrain")?;
        Ok(target::Terrain {
            hit: true,
            distance: 100.5,
        })
    }
    fn query(
        &mut self,
        _: bool,
        distance: f32,
        point: &mut [f32; 3],
    ) -> Result<Option<Id>, String> {
        if let Some(source) = self.plan.geometry.clone() {
            self.event("query")?;
            let number = self.session.delphi_local_target(self.owner).unwrap().number;
            let sample = self
                .session
                .delphi_prephysical_target_ray(self.owner, number, 1e-4)?;
            self.plan.ray_samples.push(sample);
            let (provenance, output) = self.session.query_delphi_prephysical_held_target(
                &source,
                crate::delphi_target_owned::Input {
                    owner: self.owner,
                    number,
                    normalization_threshold: 1e-4,
                    range: distance,
                    vertex_epoch: number as u32,
                    force_vertices: false,
                },
            )?;
            assert_eq!(sample.source, provenance);
            *point = output.point;
            let target = output.target;
            self.plan.geometry_outputs.push(output);
            return Ok(target);
        }
        *point = self.position();
        self.event("query")?;
        Ok(self.plan.target)
    }
    fn eligible(&mut self, _: Id) -> Result<bool, String> {
        self.event("eligible")?;
        Ok(true)
    }
    fn personality(&mut self, id: Id) -> Result<u32, String> {
        self.event("personality")?;
        Ok(self
            .session
            .get(id)
            .ok_or("foreign candidate")?
            .construction_fields
            .as_ref()
            .unwrap()
            .configuration[14])
    }
    fn camera_fallback(&mut self) -> Result<[f32; 3], String> {
        self.event("camera")?;
        Ok(self.position())
    }
    fn pending(&mut self) -> Result<Option<Id>, String> {
        self.event("pending")?;
        Ok(None)
    }
    fn item_eligible(&mut self, _: i32) -> Result<bool, String> {
        self.event("item-eligible")?;
        Ok(false)
    }
    fn item_available(&mut self, _: i32, _: [f32; 3], _: &mut Option<Id>) -> Result<u32, String> {
        Err("unused scripted item body".into())
    }
    fn linked(
        &mut self,
        state: &mut crate::delphi_target_readiness::Linked<Id, Id>,
    ) -> Result<(), String> {
        self.plan.events.push("linked");
        state.prefix.fallback = self
            .session
            .get(state.target)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap()
            .position;
        state.prefix.readiness = 1;
        if let Some(template) = &self.plan.proxy {
            state.proxy = Some(template.construct(self.session)?);
        }
        if self.plan.fail == Some("linked") {
            Err("required linked failure".into())
        } else {
            Ok(())
        }
    }
    fn physics_blocks_action(&mut self) -> Result<bool, String> {
        self.event("physics")?;
        Ok(false)
    }
    fn busy(&mut self) -> Result<bool, String> {
        self.event("busy")?;
        Ok(false)
    }
    fn class(&mut self, id: Id) -> Result<i32, String> {
        self.event("class")?;
        Ok(self.session.get(id).unwrap().class)
    }
    fn flags(&mut self, id: Id) -> Result<u32, String> {
        self.event("flags")?;
        Ok(self
            .session
            .get(id)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap()
            .object_flags)
    }
    fn highlight_in_range(&mut self, _: Id, _: i32, _: i32) -> Result<bool, String> {
        self.event("highlight-range")?;
        Ok(false)
    }
}

fn physical(
    session: &mut Session,
    owner: Id,
    number: u64,
    buttons: u32,
    g: &giants_formats::global_data::GlobalData,
    runtime: (
        &mut crate::attached_effect_context::Pool<Id>,
        &mut crate::audio_parameters::Random,
        &mut crate::animation_epoch::Stamp,
        &mut crate::delphi_post_physical::World,
    ),
) -> Result<crate::delphi_physical::View, String> {
    let (contexts, random, stamp, post) = runtime;
    let mut stop_requests = Vec::new();
    let mut stop = |cue| {
        stop_requests.push(cue);
        Ok(())
    }; // Explicit test request sink, not a voice backend.
    let mut reserve = crate::delphi_reserve::EmptyContext;
    let mut services = crate::actor_animation_phase::Services {
        contexts,
        random,
        stamp,
        blending: crate::animation_channels::Blending::Disabled,
    };
    let mut commands = [0; 8];
    commands[4..].copy_from_slice(&buttons.to_le_bytes());
    session.advance_delphi_tail_controlled(
        owner,
        crate::delphi_physical::Environment {
            number,
            actor_key: 1,
            globals: g,
            global_seconds: 1. / 60.,
            areas: &[],
            commands,
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
                    world: post,
                    view_mode: 0,
                    stop_sound: &mut stop,
                },
                recharge_inhibited: false,
                services: &mut reserve,
            },
            session_mode: 0,
            peer_ids: &[],
            local_peer: 0,
            owner_word: None,
        },
        |_| {
            Ok(crate::physical_ground::Ground {
                plane: [0., 0., 1., 0.],
                height: 0.,
                key: [0, 0],
            })
        },
        &mut services,
    )
}

#[test]
#[ignore = "Requires GIANTS_DELPHI_ROOT and optional executable; actual owned phase with scripted target providers, no live controller"]
fn source_delphi_local_phase_publishes_then_reaches_postphysical_activation_boundary() {
    let (mut s, owner, source, g, _) = setup(1401);
    let mut plan = Plan::default();
    let mut contexts = crate::attached_effect_context::Pool::from_active_resources(1401, &source);
    let mut random = crate::audio_parameters::Random(1);
    let mut stamp = crate::animation_epoch::Stamp::default();
    let mut post = crate::delphi_post_physical::World::default();
    let missing = physical(
        &mut s,
        owner,
        1,
        0,
        &g,
        (&mut contexts, &mut random, &mut stamp, &mut post),
    )
    .unwrap_err();
    assert!(
        missing.contains("requires its owned local target phase"),
        "{missing}"
    );
    assert_eq!(s.delphi_physical(owner).unwrap().number, 0);
    assert!(s.delphi_local_target(owner).is_none());
    let prepared = s
        .advance_delphi_local_target(owner, context(owner, 1, &g), &mut plan)
        .unwrap();
    assert_eq!(prepared.phase, Phase::Prepared);
    assert_eq!(word(&s, owner, 0xf40), 1);
    assert_eq!(prepared.state.point, [0., 0., 10.]);
    assert_eq!(prepared.state.terrain_point, [0., 0., 11.]);
    assert_eq!(prepared.state.aim_point, [0., 0., 11.]);
    assert!(
        s.advance_delphi_local_target(owner, context(owner, 1, &g), &mut plan)
            .is_err()
    );
    assert_eq!(plan.binds, 1);
    assert!(
        physical(
            &mut s,
            owner,
            2,
            0,
            &g,
            (&mut contexts, &mut random, &mut stamp, &mut post)
        )
        .is_err()
    );
    assert_eq!(s.delphi_local_target(owner).unwrap().phase, Phase::Prepared);
    assert_eq!(s.delphi_physical(owner).unwrap().number, 0);
    physical(
        &mut s,
        owner,
        1,
        0,
        &g,
        (&mut contexts, &mut random, &mut stamp, &mut post),
    )
    .unwrap();
    assert_eq!(s.delphi_local_target(owner).unwrap().phase, Phase::Consumed);
    assert!(
        physical(
            &mut s,
            owner,
            2,
            0,
            &g,
            (&mut contexts, &mut random, &mut stamp, &mut post)
        )
        .is_err()
    );
    s.advance_delphi_local_target(owner, context(owner, 2, &g), &mut plan)
        .unwrap();
    let before = s
        .get(owner)
        .unwrap()
        .construction_fields
        .as_ref()
        .unwrap()
        .position;
    let error = physical(
        &mut s,
        owner,
        2,
        0x10,
        &g,
        (&mut contexts, &mut random, &mut stamp, &mut post),
    )
    .unwrap_err();
    assert!(error.contains("item action producer"), "{error}");
    let frame = s.delphi_physical(owner).unwrap();
    assert_eq!(frame.number, 2);
    assert!(frame.failure.is_some());
    assert_ne!(
        s.get(owner)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap()
            .position,
        before
    );
    assert_eq!(s.delphi_local_target(owner).unwrap().phase, Phase::Consumed);
    assert!(
        s.advance_delphi_local_target(owner, context(owner, 3, &g), &mut plan)
            .is_err()
    );
}

#[test]
#[ignore = "Requires GIANTS_DELPHI_ROOT and optional executable; retained failures/cancellation with scripted geometry"]
fn source_delphi_local_phase_retains_prefix_and_cancels_without_restoring_owner() {
    let (mut s, owner, source, g, factory) = setup(1402);
    s.residents
        .get_mut(&owner.serial)
        .unwrap()
        .seed
        .storage
        .data_mut()
        .unwrap()[0xda8..0xdac]
        .copy_from_slice(&0.5_f32.to_le_bytes());
    let mut plan = Plan {
        fail: Some("terrain"),
        ..Default::default()
    };
    assert!(
        s.advance_delphi_local_target(owner, context(owner, 1, &g), &mut plan)
            .is_err()
    );
    assert_eq!(s.delphi_local_target(owner).unwrap().phase, Phase::Failed);
    assert!(f32::from_bits(word(&s, owner, 0xda8)) < 0.5);
    assert_eq!(f32::from_bits(word(&s, owner, 0xf54)), 11.);
    assert_eq!(word(&s, owner, 0xf40), 0);
    let binds = plan.binds;
    assert!(
        s.advance_delphi_local_target(owner, context(owner, 1, &g), &mut plan)
            .is_err()
    );
    assert_eq!(plan.binds, binds);
    s.clear();
    let next = make(&mut s, &factory);
    assert_ne!(next, owner);
    prepare(&mut s, next, &source, &g);
    let mut plan = Plan {
        clear: true,
        fail: Some("terrain"),
        ..Default::default()
    };
    assert!(
        s.advance_delphi_local_target(next, context(next, 1, &g), &mut plan)
            .unwrap_err()
            .contains("retired")
    );
    assert_eq!(s.counts(), Counts::default());
    assert!(s.get(next).is_none());
    let next = make(&mut s, &factory);
    prepare(&mut s, next, &source, &g);
    let mut plan = Plan::default();
    s.advance_delphi_local_target(next, context(next, 1, &g), &mut plan)
        .unwrap();
}

#[test]
#[ignore = "Requires GIANTS_DELPHI_ROOT and optional executable; live typed links/proxy constructor, scripted targeting"]
fn source_delphi_local_phase_protects_typed_links_and_partial_proxy_construction() {
    let (mut s, owner, source, g, factory) = setup(1403);
    let other = make(&mut s, &factory);
    let mut plan = Plan {
        target: Some(other),
        proxy: Some(crate::delphi_target_proxy::Template::load(&source).unwrap()),
        fail: Some("linked"),
        ..Default::default()
    };
    assert!(
        s.advance_delphi_local_target(owner, context(owner, 1, &g), &mut plan)
            .is_err()
    );
    let v = s.delphi_local_target(owner).unwrap();
    assert_eq!(v.phase, Phase::Failed);
    assert_eq!(v.state.candidate, Some(other));
    let proxy = v.state.proxy.unwrap();
    assert!(s.incoming_actor_references(other).unwrap().contains(&owner));
    assert!(s.incoming_actor_references(proxy).unwrap().contains(&owner));
    assert_eq!(s.get(proxy).unwrap().class, 0x3d4);
    for at in [0xc, 0x50, 0xf44, 0xf48, 0xf8c] {
        assert_eq!(word(&s, owner, at), 0);
    }
    let flags = s
        .get(proxy)
        .unwrap()
        .construction_fields
        .as_ref()
        .unwrap()
        .object_flags;
    s.hide_delphi_target_proxy(proxy).unwrap();
    assert_eq!(
        s.get(proxy)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap()
            .object_flags,
        flags | 0x40000000
    );
    assert!(s.hide_delphi_target_proxy(owner).is_err());
    s.clear();
    assert!(s.get(proxy).is_none());
    let next = make(&mut s, &factory);
    assert_ne!(next, owner);
    prepare(&mut s, next, &source, &g);
    let mut no_providers = Plan::default();
    let mut c = context(next, 1, &g);
    c.current_player = None;
    s.advance_delphi_local_target(next, c, &mut no_providers)
        .unwrap();
    assert_eq!(no_providers.binds, 0);
}

#[test]
#[ignore = "Requires GIANTS_DELPHI_ROOT and executable; actual held-cache epochs and candidate geometry, scripted terrain/view"]
fn source_delphi_local_phase_reads_cold_then_previous_held_geometry() {
    use crate::delphi_target_ray::Source;
    let (mut s, owner, source, g, _) = setup(1405);
    let source = Arc::new(source);
    let mut plan = Plan {
        geometry: Some(source.clone()),
        ..Default::default()
    };
    assert!(s.delphi_prephysical_target_ray(owner, 1, 1e-4).is_err());
    s.advance_delphi_local_target(owner, context(owner, 1, &g), &mut plan)
        .unwrap();
    assert_eq!(plan.ray_samples.len(), 1);
    assert_eq!(plan.ray_samples[0].source, Source::ColdConstructor);
    assert_eq!(plan.ray_samples[0].ray.origin.map(f32::to_bits), [0; 3]);
    assert_eq!(plan.ray_samples[0].ray.direction.map(f32::to_bits), [0; 3]);
    assert!(s.actor_current_affine(owner).is_err());
    assert!(s.delphi_target_ray(owner, 0, 1e-4).is_err());
    let mut contexts = crate::attached_effect_context::Pool::from_active_resources(1405, &source);
    let mut random = crate::audio_parameters::Random(1);
    let mut stamp = crate::animation_epoch::Stamp::default();
    let mut post = crate::delphi_post_physical::World::default();
    for number in 1..=2 {
        physical(
            &mut s,
            owner,
            number,
            0,
            &g,
            (&mut contexts, &mut random, &mut stamp, &mut post),
        )
        .unwrap();
        let held = s.delphi_special(owner).unwrap().held;
        s.update_current_references_at(held, stamp.0 + 1).unwrap();
        s.advance_delphi_held_pose(owner, number).unwrap();
        let previous = s.delphi_target_ray(owner, number, 1e-4).unwrap();
        let candidates = if number == 2 {
            [2., 22.]
                .map(|distance| {
                    place_candidate(
                        &mut s,
                        &source,
                        std::array::from_fn(|i| {
                            previous.origin[i] + previous.direction[i] * distance
                        }),
                    )
                })
                .to_vec()
        } else {
            Vec::new()
        };
        assert!(
            s.delphi_prephysical_target_ray(owner, number + 1, 1e-4)
                .is_err()
        );
        s.advance_delphi_local_target(owner, context(owner, number + 1, &g), &mut plan)
            .unwrap();
        let sample = plan.ray_samples.last().unwrap();
        assert_eq!(sample.source, Source::PreviousFrame(number));
        assert_eq!(
            sample.ray.origin.map(f32::to_bits),
            previous.origin.map(f32::to_bits)
        );
        assert_eq!(
            sample.ray.direction.map(f32::to_bits),
            previous.direction.map(f32::to_bits)
        );
        if !candidates.is_empty() {
            let output = plan.geometry_outputs.last().unwrap();
            assert_eq!(output.target, Some(candidates[1]));
            assert_eq!(output.geometry_visits, candidates);
            assert_eq!(output.mesh_visits, candidates);
        }
    }
    let old = owner;
    s.clear();
    assert!(s.delphi_prephysical_target_ray(old, 4, 1e-4).is_err());
}

fn place_candidate(
    s: &mut Session,
    source: &crate::scene_resources::SceneResources,
    position: [f32; 3],
) -> Id {
    let class = 73;
    let program = source
        .classes
        .models
        .first_program(class)
        .unwrap()
        .program
        .as_ref()
        .unwrap();
    let specification =
        Arc::new(crate::specification::Prepared::load_registered(source, class).unwrap());
    let seed = Seed::from_program_for(
        program,
        Profile::Debug,
        crate::arithmetic::Precision::Double53,
    )
    .unwrap();
    let mut fields = crate::actor_fields::Fields::load(source, class, &specification)
        .unwrap()
        .placed(crate::collider_init::DeclaredPose {
            class: class as u32,
            position,
            yaw: 0.,
            pitch_roll: Some([0.; 2]),
        })
        .unwrap();
    fields.set_declared_scale(0.5).unwrap();
    let id = s
        .construct(Construction {
            class,
            seed: &seed,
            bindings: &source
                .class_state_bindings_for(class, Profile::Debug)
                .unwrap(),
            flags: DirectFlags::project(class, program),
            specification,
            state_resources: Arc::new(
                crate::state_resources::Prepared::load(source, class, Profile::Debug).unwrap(),
            ),
            fields,
        })
        .unwrap();
    s.prepare_actor_affine(id).unwrap();
    id
}

#[test]
#[ignore = "Requires GIANTS_DELPHI_ROOT and executable; acquired cache provenance refusals, no controller"]
fn source_delphi_local_phase_refuses_unowned_cache_epochs_before_geometry() {
    for mode in 0..3 {
        let ticket = 1410 + mode;
        let (mut s, owner, source, g, _) = setup(ticket);
        let source = Arc::new(source);
        let mut plan = Plan {
            geometry: Some(source.clone()),
            ..Default::default()
        };
        let number = if mode == 0 {
            s.prepare_actor_affine(owner).unwrap();
            1
        } else if mode == 1 {
            let held = s.delphi_special(owner).unwrap().held;
            s.update_current_references_at(held, 1).unwrap();
            1
        } else {
            s.advance_delphi_local_target(owner, context(owner, 1, &g), &mut plan)
                .unwrap();
            let mut contexts =
                crate::attached_effect_context::Pool::from_active_resources(ticket, &source);
            let mut random = crate::audio_parameters::Random(1);
            let mut stamp = crate::animation_epoch::Stamp::default();
            let mut post = crate::delphi_post_physical::World::default();
            physical(
                &mut s,
                owner,
                1,
                0,
                &g,
                (&mut contexts, &mut random, &mut stamp, &mut post),
            )
            .unwrap();
            2
        };
        let error = s
            .advance_delphi_local_target(owner, context(owner, number, &g), &mut plan)
            .unwrap_err();
        assert!(
            error.contains(if mode < 2 {
                "constructor caches"
            } else {
                "held cache not produced"
            }),
            "{error}"
        );
        assert_eq!(s.delphi_local_target(owner).unwrap().phase, Phase::Failed);
        assert_eq!(s.delphi_physical(owner).unwrap().number, number - 1);
        assert_eq!(plan.geometry_outputs.len(), if mode == 2 { 1 } else { 0 });
        assert!(
            s.advance_delphi_local_target(owner, context(owner, number, &g), &mut plan)
                .is_err()
        );
    }
}

#[test]
#[ignore = "Requires GIANTS_DELPHI_ROOT and executable; actual terrain/held geometry, supplied camera/items/time/physical floor"]
fn source_delphi_local_phase_traces_installed_terrain_from_cold_and_previous_caches() {
    for (case, world) in ["w_Rstory1.bin", "w_Rstory2.bin", "w_Rstory4.bin"]
        .into_iter()
        .enumerate()
    {
        let ticket = 1420 + case as u64;
        let (mut s, owner, source, g, _) = setup(ticket);
        let root = std::path::PathBuf::from(std::env::var("GIANTS_DELPHI_ROOT").unwrap());
        let bytes = std::fs::read(root.join("Bin").join(world)).unwrap();
        let prefix = giants_formats::world::terrain_prefix(&bytes).unwrap();
        let bytes = source
            .archives
            .read_bin(&prefix.terrain, "English")
            .unwrap();
        let terrain = Arc::new(giants_formats::gti::parse(&bytes).unwrap());
        let source = Arc::new(source);
        let mut plan = Plan {
            geometry: Some(source.clone()),
            terrain: Some(terrain.clone()),
            ..Default::default()
        };
        let mut contexts =
            crate::attached_effect_context::Pool::from_active_resources(ticket, &source);
        let mut random = crate::audio_parameters::Random(1);
        let mut stamp = crate::animation_epoch::Stamp::default();
        let mut post = crate::delphi_post_physical::World::default();
        for number in 1..=3 {
            let result = s
                .advance_delphi_local_target(owner, context(owner, number, &g), &mut plan)
                .unwrap();
            let point = plan.terrain_points.last().unwrap();
            assert_eq!(
                result.state.terrain_point.map(f32::to_bits),
                point.map(f32::to_bits)
            );
            let ray = plan.ray_samples.last().unwrap();
            assert_eq!(
                ray.source,
                if number == 1 {
                    crate::delphi_target_ray::Source::ColdConstructor
                } else {
                    crate::delphi_target_ray::Source::PreviousFrame(number - 1)
                }
            );
            assert!(plan.terrain_distances.last().unwrap().is_finite());
            physical(
                &mut s,
                owner,
                number,
                0,
                &g,
                (&mut contexts, &mut random, &mut stamp, &mut post),
            )
            .unwrap();
            let held = s.delphi_special(owner).unwrap().held;
            s.update_current_references_at(held, stamp.0 + 1).unwrap();
            s.advance_delphi_held_pose(owner, number).unwrap();
        }
        assert_eq!(plan.terrain_points.len(), 3);
        plan.fail = Some("query");
        let error = s
            .advance_delphi_local_target(owner, context(owner, 4, &g), &mut plan)
            .unwrap_err();
        assert!(error.contains("required query failure"));
        let failed = s.delphi_local_target(owner).unwrap();
        assert_eq!(failed.phase, Phase::Failed);
        assert_eq!(
            failed.state.terrain_point.map(f32::to_bits),
            plan.terrain_points.last().unwrap().map(f32::to_bits)
        );
        assert_eq!(s.delphi_physical(owner).unwrap().number, 3);
        assert!(
            s.advance_delphi_local_target(owner, context(owner, 4, &g), &mut plan)
                .is_err()
        );
        let mut untouched = [7., 8., 9.];
        let query = crate::terrain_queries::StaticTerrainQuery::new(
            &terrain,
            1. / terrain.spacing,
            crate::arithmetic::Precision::Double53,
        )
        .unwrap();
        assert!(
            s.trace_delphi_prephysical_held_terrain(
                crate::delphi_target_scan::HeldTerrainInput {
                    owner,
                    number: 4,
                    normalization_threshold: 1e-4,
                    range: 9000.,
                },
                &query,
                &mut untouched
            )
            .is_err()
        );
        assert_eq!(untouched, [7., 8., 9.]);
        s.clear();
        assert!(s.delphi_prephysical_target_ray(owner, 4, 1e-4).is_err());
    }
}
