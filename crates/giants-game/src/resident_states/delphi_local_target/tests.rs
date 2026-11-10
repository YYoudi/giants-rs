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
        Ok(true)
    }
    fn terrain(
        &mut self,
        _: bool,
        _: f32,
        point: &mut [f32; 3],
    ) -> Result<target::Terrain, String> {
        assert_eq!(
            self.session.delphi_local_target(self.owner).unwrap().phase,
            Phase::Running
        );
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
    fn query(&mut self, _: bool, _: f32, point: &mut [f32; 3]) -> Result<Option<Id>, String> {
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
    let mut contexts = crate::attached_effect_context::Pool::from_active_resources(1401, &source);
    let mut random = crate::audio_parameters::Random(1);
    let mut stamp = crate::animation_epoch::Stamp::default();
    let mut post = crate::delphi_post_physical::World::default();
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
