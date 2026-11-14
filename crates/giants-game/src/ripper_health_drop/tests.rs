//! Installed constructor/late-failure controls. Stop/blood are explicit supplied
//! callback boundaries; death FX deliberately refuses. No runtime/death proof.
use super::*;
use crate::{
    resident_states::State23Services,
    state23_damage::{self, Service},
};

struct Attach {
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
    fields.position = [1.25, -2., 3.5];
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
