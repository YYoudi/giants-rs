//! Installed constructor/late-failure controls. Stop/blood are explicit supplied
//! callback boundaries; death FX deliberately refuses. No runtime/death proof.
use super::*;
use crate::{
    resident_states::State23Services,
    state23_damage::{self, Service},
};

pub(crate) fn birth_fixture(
    resources: &SceneResources,
    factory: &Factory,
    position: [f32; 3],
) -> (Session, Id, Attach) {
    birth_fixture_for_class(resources, factory, 73, position)
}
pub(crate) fn birth_fixture_for_class(
    resources: &SceneResources,
    factory: &Factory,
    class: i32,
    position: [f32; 3],
) -> (Session, Id, Attach) {
    let mut session = Session::new(9840);
    let owner = source_at(resources, &mut session, class, position);
    let mut backend = Backend {
        factory,
        attach: Attach {
            pool: crate::attached_effect_context::Pool::from_active_resources(9840, resources),
            fail: false,
            observed: None,
        },
        born: None,
    };
    assert!(
        session
            .damage_state23(
                owner,
                10000.,
                None,
                state23_damage::Context {
                    player_is_state1: false,
                    drop_percent: 100,
                    death_timer: 10.,
                },
                &mut Random(17),
                &mut backend
            )
            .is_err()
    );
    let item = backend.born.expect("real drop callback reached");
    assert!(session.ripper_health_drop(item).unwrap().initialized);
    (session, item, backend.attach)
}

struct GroundedPrimaryFixture {
    expiry: usize,
}

fn primary_words(session: &Session, item: Id) -> Vec<u32> {
    let f = session
        .get(item)
        .unwrap()
        .construction_fields
        .as_ref()
        .unwrap();
    let owned = session.ripper_health_drop(item).unwrap();
    let mut actual = Vec::new();
    actual.extend(f.position.into_iter().chain(f.current).map(f32::to_bits));
    actual.extend([f.angles[0].to_bits(), f.object_flags, f.control_flags]);
    actual.extend(owned.velocity.map(f32::to_bits));
    actual.extend([owned.floor.to_bits(), f.configuration[17]]);
    actual.extend(
        session
            .get(item)
            .unwrap()
            .seed
            .storage
            .data()
            .unwrap()
            .as_chunks::<4>()
            .0
            .iter()
            .copied()
            .map(u32::from_le_bytes),
    );
    actual
}

struct AirFixture {
    ground: usize,
    sweep: usize,
    fail: Option<&'static str>,
}
impl PrimaryServices for AirFixture {
    fn ground(&mut self, _: &Session, _: Id, _: [f32; 3]) -> Result<f32, String> {
        self.ground += 1;
        if self.fail == Some("ground") {
            Err("supplied ground query failure".into())
        } else {
            Ok(0.)
        }
    }
    fn blocked(
        &mut self,
        _: &Session,
        _: Id,
        from: [f32; 3],
        to: [f32; 3],
    ) -> Result<bool, String> {
        self.sweep += 1;
        assert_eq!(
            from, to,
            "first fall step starts with constructor zero velocity"
        );
        if self.fail == Some("sweep") {
            Err("supplied obstruction query failure".into())
        } else {
            Ok(false)
        }
    }
    fn expired(&mut self, _: &Session, _: Id) -> Result<(), String> {
        panic!("air fixtures do not expire")
    }
}
#[test]
#[ignore = "Requires GIANTS_NPC_ROOT/GIANTS_RIPPER_HEALTH_AIR_CONTROLS; supplied plane-Z0/no-obstruction scalar controls, no ordinary terrain or collision proof"]
fn native_health_air_prefix_matches_and_preserves_failed_world_queries() {
    let root = std::path::PathBuf::from(std::env::var_os("GIANTS_NPC_ROOT").unwrap());
    let resources = SceneResources::load(&root, "w_Story2.bin", "English").unwrap();
    let globals =
        giants_formats::global_data::parse(&std::fs::read(root.join("Bin/GData.gbn")).unwrap())
            .unwrap();
    let factory = Factory::load(&resources, &globals).unwrap();
    let text =
        std::fs::read_to_string(std::env::var_os("GIANTS_RIPPER_HEALTH_AIR_CONTROLS").unwrap())
            .unwrap();
    let mut count = 0;
    for line in text.lines() {
        let words: Vec<u32> = line
            .split_whitespace()
            .map(|v| u32::from_str_radix(v, 16).unwrap())
            .collect();
        assert_eq!(words.len(), 22);
        let z = f32::from_bits(words[0]);
        let seconds = f32::from_bits(words[1]);
        for fail in [None, Some("ground"), Some("sweep")] {
            let (mut session, item, _fx) = birth_fixture(&resources, &factory, [1.25, -2., z]);
            let mut world = AirFixture {
                ground: 0,
                sweep: 0,
                fail,
            };
            let result = factory.advance_primary_prefix(
                &mut session,
                item,
                PrimaryInput {
                    step: 1,
                    global_seconds: seconds,
                    areas: &[],
                    paused: false,
                    force_ground: false,
                },
                &mut world,
            );
            assert_eq!(world.ground, 1);
            let owned = session.ripper_health_drop(item).unwrap();
            if let Some(fail) = fail {
                assert!(result.is_err());
                assert!(owned.failed);
                assert_eq!(owned.primary_phase, PrimaryPhase::Failed);
                assert_eq!(owned.lifetime, 30.);
                assert_eq!(owned.owner_delay, 1.);
                assert_eq!(
                    owned.velocity[2],
                    if fail == "ground" { 0. } else { -40. * seconds }
                );
                assert_eq!(world.sweep, usize::from(fail == "sweep"));
                assert!(
                    factory
                        .advance_primary_prefix(
                            &mut session,
                            item,
                            PrimaryInput {
                                step: 2,
                                global_seconds: seconds,
                                areas: &[],
                                paused: false,
                                force_ground: false
                            },
                            &mut world
                        )
                        .is_err()
                );
                assert_eq!(world.ground, 1, "failed query prefix cannot be replayed");
            } else {
                assert!(matches!(result.unwrap(), PrimaryPrefix::Collection(_)));
                assert_eq!(world.sweep, 1);
                assert_eq!(primary_words(&session, item), &words[2..]);
            }
        }
        count += 1;
    }
    assert_eq!(count, 4);
    println!(
        "health_air_native_match cases={count} ground_and_sweep_failures_retained=true world_geometry_acknowledged=false"
    );
}
impl PrimaryServices for GroundedPrimaryFixture {
    fn ground(&mut self, _: &Session, _: Id, _: [f32; 3]) -> Result<f32, String> {
        panic!("grounded control must not query ground")
    }
    fn blocked(&mut self, _: &Session, _: Id, _: [f32; 3], _: [f32; 3]) -> Result<bool, String> {
        panic!("grounded control must not sweep")
    }
    fn expired(&mut self, session: &Session, item: Id) -> Result<(), String> {
        assert!(session.ripper_health_drop(item).unwrap().lifetime <= 0.);
        assert_ne!(
            session
                .get(item)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap()
                .object_flags
                & 0x8000,
            0
        );
        self.expiry += 1;
        // Supplied notification boundary; original controls have no network
        // recipient. This is not an ordinary network/retirement provider.
        Ok(())
    }
}

#[test]
#[ignore = "Requires GIANTS_NPC_ROOT and GIANTS_RIPPER_HEALTH_PRIMARY_CONTROLS; real birth/canonical prefix compared to original grounded primary, no collection/rendering"]
fn native_health_primary_grounded_prefix_retains_collection_and_failure_boundaries() {
    let root = std::path::PathBuf::from(std::env::var_os("GIANTS_NPC_ROOT").unwrap());
    let resources = SceneResources::load(&root, "w_Story2.bin", "English").unwrap();
    let globals =
        giants_formats::global_data::parse(&std::fs::read(root.join("Bin/GData.gbn")).unwrap())
            .unwrap();
    let factory = Factory::load(&resources, &globals).unwrap();
    let text =
        std::fs::read_to_string(std::env::var_os("GIANTS_RIPPER_HEALTH_PRIMARY_CONTROLS").unwrap())
            .unwrap();
    let mut cases = 0;
    for line in text.lines() {
        let words: Vec<u32> = line
            .split_whitespace()
            .map(|s| u32::from_str_radix(s, 16).unwrap())
            .collect();
        assert_eq!(words.len(), 21);
        let seconds = f32::from_bits(words[0]);
        let (mut session, item, _attached_contexts) =
            birth_fixture(&resources, &factory, [1.25, -2., 2.]);
        let mut world = GroundedPrimaryFixture { expiry: 0 };
        let outcome = factory
            .advance_primary_prefix(
                &mut session,
                item,
                PrimaryInput {
                    step: 1,
                    global_seconds: seconds,
                    areas: &[],
                    paused: false,
                    force_ground: false,
                },
                &mut world,
            )
            .unwrap();
        let PrimaryPrefix::Collection(pending) = outcome else {
            panic!("collection continuation required")
        };
        assert_eq!((pending.item(), pending.step()), (item, 1));
        let owned = session.ripper_health_drop(item).unwrap();
        assert_eq!(owned.primary_phase, PrimaryPhase::CollectionPending);
        assert_eq!(
            primary_words(&session, item),
            &words[1..],
            "delta={seconds}"
        );
        assert_eq!(world.expiry, usize::from(seconds >= 30.));
        assert!(
            factory
                .advance_primary_prefix(
                    &mut session,
                    item,
                    PrimaryInput {
                        step: 2,
                        global_seconds: seconds,
                        areas: &[],
                        paused: false,
                        force_ground: false,
                    },
                    &mut world
                )
                .is_err(),
            "pending collection must not be acknowledged or replayed"
        );
        cases += 1;
    }
    assert_eq!(cases, 7);
    println!(
        "health_primary_native_match cases={cases} words_per_case=20 collection_acknowledged=false"
    );
}

pub(crate) struct Attach {
    pool: crate::attached_effect_context::Pool<Id>,
    fail: bool,
    observed: Option<Id>,
}
impl Effects for Attach {
    fn attach(
        &mut self,
        session: &mut Session,
        item: Id,
        name: &'static str,
        random: &mut Random,
    ) -> Result<Creation, String> {
        self.observed = Some(item);
        let owned = session.ripper_health_drop(item).unwrap();
        assert_eq!(
            (owned.amount, owned.owner_delay, owned.lifetime),
            (0, 1., 30.)
        );
        assert!(!owned.initialized && !owned.failed);
        let actor = session.get(item).unwrap();
        assert_eq!(actor.seed.storage.state(), 37);
        let fields = actor.construction_fields.as_ref().unwrap();
        assert_eq!(fields.control_flags & 7, 1);
        if self.fail {
            random.next_15();
            return Err("supplied health effect failure after RNG".into());
        }
        self.pool.prepare(name, item, fields.object_flags, u32::MAX)
    }
}
struct Backend<'a> {
    factory: &'a Factory,
    attach: Attach,
    born: Option<Id>,
}
impl State23Services for Backend<'_> {
    fn execute(
        &mut self,
        session: &mut Session,
        actor: &state23_damage::State<Id>,
        request: Service<Id>,
        random: &mut Random,
    ) -> Result<(), String> {
        match request {
            // Deliberate test boundaries, never production providers.
            Service::StopAttached { .. } | Service::Blood { .. } => Ok(()),
            Service::Drop { actor: owner, .. } => {
                assert_eq!(owner, actor.id);
                self.born = Some(
                    self.factory
                        .spawn(session, owner, random, &mut self.attach)?,
                );
                Ok(())
            }
            _ => Err("death effect consumer deliberately unavailable in birth control".into()),
        }
    }
}
fn source(resources: &SceneResources, session: &mut Session, class: i32) -> Id {
    source_at(resources, session, class, [1.25, -2., 3.5])
}
pub(crate) fn source_at(
    resources: &SceneResources,
    session: &mut Session,
    class: i32,
    position: [f32; 3],
) -> Id {
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
    let specification =
        Arc::new(crate::specification::Prepared::load_registered(resources, class).unwrap());
    let mut fields = Fields::load(resources, class, &specification)
        .unwrap()
        .instantiate();
    // Explicit constructor fixture, never an ordinary NPC pose override.
    fields.position = position;
    session
        .construct(Construction {
            class,
            seed: &seed,
            bindings: &bindings,
            flags: DirectFlags::project(class, program),
            specification,
            state_resources: Arc::new(
                crate::state_resources::Prepared::load(resources, class, Profile::Debug).unwrap(),
            ),
            fields,
        })
        .unwrap()
}
#[test]
#[ignore = "Requires GIANTS_NPC_ROOT; installed class785 birth and source health effect lookup under supplied stop/blood boundaries; death FX deliberately missing, no runtime/graphics proof"]
fn source_ripper_health_drop_birth_preserves_real_prefix_and_failed_scope() {
    let root = std::path::PathBuf::from(std::env::var_os("GIANTS_NPC_ROOT").unwrap());
    let resources = SceneResources::load(&root, "w_Story2.bin", "English").unwrap();
    let globals =
        giants_formats::global_data::parse(&std::fs::read(root.join("Bin/GData.gbn")).unwrap())
            .unwrap();
    let factory = Factory::load(&resources, &globals).unwrap();
    for class in [73, 230] {
        for fail in [false, true] {
            let mut session = Session::new(9837);
            let owner = source(&resources, &mut session, class);
            let mut random = Random(17);
            let mut backend = Backend {
                factory: &factory,
                attach: Attach {
                    pool: crate::attached_effect_context::Pool::from_active_resources(
                        9837, &resources,
                    ),
                    fail,
                    observed: None,
                },
                born: None,
            };
            let count = session.counts().residents;
            assert!(
                factory
                    .spawn(&mut session, owner, &mut random, &mut backend.attach)
                    .is_err()
            );
            assert_eq!(session.counts().residents, count);
            assert_eq!(random.0, 17);
            let result = session.damage_state23(
                owner,
                10000.,
                None,
                state23_damage::Context {
                    player_is_state1: false,
                    drop_percent: 100,
                    death_timer: 10.,
                },
                &mut random,
                &mut backend,
            );
            assert!(result.is_err());
            let item = backend
                .attach
                .observed
                .expect("actual lethal callback must reach health birth");
            assert_eq!(session.counts().residents, count + 1);
            let owned = session.ripper_health_drop(item).unwrap();
            let actor = session.get(item).unwrap();
            assert_eq!(
                actor.constructor_mask, 0,
                "source callback requests only common-list membership"
            );
            for list in 0..15 {
                assert_eq!(session.list(list).contains(&item), list == 0);
            }
            let fields = actor.construction_fields.as_ref().unwrap();
            assert_eq!(owned.owner, owner);
            assert_eq!(
                fields.position,
                session.state23_combat(owner).unwrap().position
            );
            assert_eq!(actor.seed.storage.data().unwrap().len(), 24);
            assert_eq!(fields.current, [0.; 3]);
            assert_eq!(fields.previous, [0.; 3]);
            assert_eq!(&actor.seed.storage.data().unwrap()[12..16], &[0; 4]);
            assert_eq!(
                u32::from_le_bytes(actor.seed.storage.data().unwrap()[..4].try_into().unwrap()),
                36
            );
            if fail {
                assert!(owned.failed && !owned.initialized);
                assert_eq!(owned.amount, 0);
                assert_eq!(fields.control_flags & 7, 1);
                assert!(factory.bounds(&session, item).is_err());
            } else {
                assert!(
                    owned.initialized && !owned.failed,
                    "birth failed: {result:?}"
                );
                assert_eq!(owned.amount, factory.amount[usize::from(class == 230)]);
                assert_eq!(fields.control_flags & 7, if class == 230 { 5 } else { 3 });
                let context = owned.context.expect("installed health FX must resolve");
                let context = backend.attach.pool.get(context).unwrap();
                assert_eq!(context.flags(), 4);
                assert_eq!(context.recipient(), Some(item));
                assert_eq!(context.tag(), Some(u32::MAX));
                assert_eq!(context.emitted_objects(), 0);
                assert!(
                    factory.bounds(&session, item).is_err(),
                    "no affine was prepared"
                );
                session.prepare_actor_affine(item).unwrap();
                let bounds = factory.bounds(&session, item).unwrap();
                assert!((0..3).all(|i| bounds[i] < bounds[i + 3]));
                // A distinct factory must not claim this birth's specification.
                assert!(
                    Factory::load(&resources, &globals)
                        .unwrap()
                        .bounds(&session, item)
                        .is_err()
                );
                if let Some(path) = std::env::var_os("GIANTS_RIPPER_HEALTH_BOUNDS_CONTROLS") {
                    let text = std::fs::read_to_string(path).unwrap();
                    let rows: Vec<_> = text
                        .lines()
                        .map(|line| {
                            let mut words = line.split_whitespace();
                            let source: i32 = words.next().unwrap().parse().unwrap();
                            let words: Vec<_> =
                                words.map(|w| u32::from_str_radix(w, 16).unwrap()).collect();
                            assert_eq!(words.len(), 6);
                            (source, words)
                        })
                        .collect();
                    assert_eq!(rows.iter().map(|r| r.0).collect::<Vec<_>>(), [73, 230]);
                    let expected = &rows.iter().find(|r| r.0 == class).unwrap().1;
                    assert_eq!(&bounds.map(f32::to_bits), expected.as_slice());
                    println!(
                        "health_bounds_native_match source_class={class} words=6 source_world_companion=true contact_acknowledged=false"
                    );
                }
            }
            assert!(session.state23_damage_failure(owner).is_some());
            let retained = random.0;
            assert!(
                factory
                    .spawn(&mut session, owner, &mut random, &mut backend.attach)
                    .is_err()
            );
            assert_eq!(session.counts().residents, count + 1);
            assert_eq!(random.0, retained);
        }
    }
}

#[test]
#[ignore = "Requires GIANTS_NPC_ROOT and private GIANTS_RIPPER_HEALTH_EMISSION_CONTROLS; compares original active source56 registration arguments, not later callbacks/rendering"]
fn native_ripper_health_emission_registration_matches_scene_configuration() {
    let root = std::path::PathBuf::from(std::env::var_os("GIANTS_NPC_ROOT").unwrap());
    let image = giants_formats::effect_image::parse(
        &std::fs::read(root.join("Bin/fx_world_dust.bin")).unwrap(),
    )
    .unwrap();
    let controls = std::fs::read_to_string(
        std::env::var_os("GIANTS_RIPPER_HEALTH_EMISSION_CONTROLS").unwrap(),
    )
    .unwrap();
    let rows = controls
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>())
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 6);
    let mut at = 0;
    for name in ["fx_ripper_medium_health", "fx_ripper_large_health"] {
        let plan = crate::effect_scene::Plan::project(&image, name).unwrap();
        let mut seed = 17;
        let (objects, _) = plan.start(&mut seed).unwrap();
        assert_eq!(seed, 17, "fixed creation must consume no RNG");
        assert_eq!(objects.len(), 1);
        assert_eq!(objects[0].services.len(), 3);
        for service in &objects[0].services {
            let row = &rows[at];
            at += 1;
            assert_eq!(row.len(), 9);
            assert_eq!(row[0], name);
            assert_eq!(service.kind, crate::effect_scene::ServiceKind::Source56);
            assert!(service.random_sample.is_none());
            let config = service.emission_configuration().unwrap();
            assert_eq!(config.effect, row[1]);
            let expected = row[2..]
                .iter()
                .map(|s| u32::from_str_radix(s, 16).unwrap())
                .collect::<Vec<_>>();
            assert_eq!(
                vec![
                    config.interval.to_bits(),
                    config.selectors[0][0],
                    config.selectors[0][1],
                    config.selectors[1][0],
                    config.selectors[1][1],
                    u32::from(config.source_flag8),
                    u32::from(config.source_flag9)
                ],
                expected
            );
            assert_eq!(service.interval_bounds().unwrap(), [config.interval; 2]);
            assert!(matches!(
                crate::effect_reference_emission::Channel::new(&config)
                    .unwrap()
                    .mode,
                crate::effect_reference_emission::Mode::Paired(_)
            ));
        }
    }
}

#[test]
#[ignore = "Requires GIANTS_NPC_ROOT and private GIANTS_RIPPER_HEALTH_ELIGIBILITY_CONTROLS; actual declaration plus original PC type36 eligibility, not item contact/healing/gameplay"]
fn native_ripper_health_definition_and_mecc_eligibility_match_loaded_source() {
    let root = std::path::PathBuf::from(std::env::var_os("GIANTS_NPC_ROOT").unwrap());
    let bytes = std::fs::read(root.join("Bin/weapondefs.bin")).unwrap();
    let catalog = giants_formats::weapons::parse_with_profile(
        &bytes,
        giants_formats::weapons::Profile::Debug,
    )
    .unwrap();
    assert_eq!(catalog.icons[&36].player_type, Some(1));
    let definition = PickupDefinition::load(&catalog).unwrap();
    let text = std::fs::read_to_string(
        std::env::var_os("GIANTS_RIPPER_HEALTH_ELIGIBILITY_CONTROLS").unwrap(),
    )
    .unwrap();
    let mut count = 0;
    for line in text.lines() {
        let row = line.split_whitespace().collect::<Vec<_>>();
        assert_eq!(row.len(), 3);
        let state = crate::mecc_damage::State {
            damage: row[0].parse().unwrap(),
            limit: row[1].parse().unwrap(),
            shield: 0.,
            flags: 0,
        };
        assert_eq!(definition.mecc_eligible(state).unwrap(), row[2] == "1");
        count += 1;
    }
    assert_eq!(count, 6);
    let other =
        giants_formats::weapons::parse_with_profile(&bytes, giants_formats::weapons::Profile::Gog)
            .unwrap();
    assert!(
        PickupDefinition::load(&other).is_err(),
        "unacquired profile must not become a player-family default"
    );
}
