//! Installed eventless-state0 callback coverage; caller/world admission is supplied.
use super::*;

#[test]
#[ignore = "Requires GIANTS_NPC_ROOT; actual Story2 neutral declarations with supplied callback admission/control/time and unused object/platform providers"]
fn source_passive_world_updates_neutral_declarations_without_default_callbacks() {
    let root = std::path::PathBuf::from(std::env::var_os("GIANTS_NPC_ROOT").unwrap());
    let source = crate::world_source::resolve(&root, "Story2").unwrap();
    let menu = crate::mission_selection::MissionMenu::parse(
        &std::fs::read(root.join("Bin").join(source.catalog)).unwrap(),
        u32::MAX,
        true,
    )
    .unwrap();
    let packet =
        crate::mission_loading::prepare(&root, menu.request("Story2", 0).unwrap()).unwrap();
    let plan = packet.instance_plan(&root, "English").unwrap();
    let mut session = Session::new(680);
    let report = crate::declared_residents::construct_acquired(&plan, &mut session);
    assert_eq!(report.constructed(), 162);
    let neutral: Vec<_> = session
        .list(0)
        .into_iter()
        .filter(|&id| {
            let a = session.get(id).unwrap();
            a.seed.storage.state() == 0 && a.class != 679 && session.passive_world_frame(id).is_ok()
        })
        .collect();
    let unsupported: Vec<_> = session
        .list(0)
        .into_iter()
        .filter(|&id| {
            let a = session.get(id).unwrap();
            a.seed.storage.state() == 0
                && a.class != 679
                && session.passive_world_frame(id).is_err()
        })
        .collect();
    assert_eq!(neutral.len(), 134);
    assert_eq!(unsupported.len(), 0);
    let reference_owners: Vec<_> = neutral
        .iter()
        .copied()
        .filter(|&id| {
            !session
                .reference_cache_storage(id)
                .unwrap()
                .matrices
                .is_empty()
        })
        .collect();
    assert_eq!(reference_owners.len(), 5);
    let animated = neutral
        .iter()
        .copied()
        .find(|&id| session.get(id).unwrap().class == 1276)
        .unwrap();
    let factory = crate::vimp_actor::Factory::load_registered(plan.resources()).unwrap();
    let globals =
        giants_formats::global_data::parse(&std::fs::read(root.join("Bin/GData.gbn")).unwrap())
            .unwrap();
    let mut random = crate::audio_parameters::Random(314);
    let mut stamp = crate::animation_epoch::Stamp(0);
    // This fixture supplies registered controlled presence. It does not select
    // a playable class or claim that the other mission families were updated.
    let controlled = neutral[0];
    struct Ground;
    impl crate::herd_generator::Ground for Ground {
        fn height(&mut self, _: [f32; 3]) -> Result<f32, String> {
            Err("neutral callback must not generate a herd".into())
        }
    }
    let positions: Vec<_> = neutral
        .iter()
        .map(|&id| {
            session
                .get(id)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap()
                .position
                .map(f32::to_bits)
        })
        .collect();
    for (number, seconds) in [0., 1. / 60., 0.].into_iter().enumerate() {
        if number == 2 {
            // Explicit force-entry input, matching the zero-time PC guard
            // control. This is not acquisition of an ordinary flag producer.
            for id in &reference_owners {
                let actor = session.residents.get_mut(&id.serial).unwrap();
                actor.channels.as_mut().unwrap()[3][8] |= 0x100;
                actor.construction_fields.as_mut().unwrap().channels = actor.channels.unwrap();
            }
        }
        for &id in &neutral {
            session
                .prepare_passive_world_history(id, number as u64)
                .unwrap();
        }
        for (i, &id) in neutral.iter().enumerate() {
            let mut finalization = crate::actor_environment::FinalizationServices {
                actor: u32::try_from(id.serial).unwrap(),
                state: 0,
                slope_parameter: 0.,
                physics: None,
                skip_object_responses: false,
                objects: &[],
                ground: |_| Err("stationary neutral must not query terrain"),
                platform: |_| Err("stationary neutral must not query platforms"),
                seconds: |_| f64::from(seconds),
                release: |_| Err("stationary neutral must not release a footprint".into()),
                statistics: None,
            };
            let result = session
                .advance_passive_world_after_history(
                    id,
                    crate::passive_frame::Frame {
                        number: number as u64,
                        global_seconds: seconds,
                        areas: &[],
                        controlled: Some(controlled),
                    },
                    &mut crate::passive_frame::Services {
                        factory: &factory,
                        globals: &globals,
                        random: &mut random,
                        ground: &mut Ground,
                        stamp: &mut stamp,
                        finalization: &mut finalization,
                    },
                )
                .unwrap();
            assert_eq!(result.outcome, crate::actor_world_tick::Outcome::Updated);
            assert!(result.born.is_empty());
            assert_eq!(
                session.passive_world_frame(id).unwrap().next_frame,
                number as u64 + 1
            );
            assert_eq!(
                session
                    .get(id)
                    .unwrap()
                    .construction_fields
                    .as_ref()
                    .unwrap()
                    .position
                    .map(f32::to_bits),
                positions[i]
            );
            session.actor_current_affine(id).unwrap();
            if reference_owners.contains(&id) {
                let storage = session.reference_cache_storage(id).unwrap();
                if id == animated {
                    assert!(storage.sampled);
                    assert_eq!(session.current_reference_matrices(id).unwrap().len(), 15);
                    let epoch = session.get(id).unwrap().mecc_animation_stamp;
                    session
                        .prepare_actor_positions(id, 0, epoch, false)
                        .unwrap();
                    assert!(
                        !session
                            .actor_model_positions(id, 0)
                            .unwrap()
                            .unwrap()
                            .is_empty()
                    );
                } else {
                    assert!(!storage.sampled);
                    assert!(
                        storage
                            .matrices
                            .iter()
                            .all(|m| m.flags == 0 && m.values.iter().all(|v| v.to_bits() == 0))
                    );
                    assert!(session.current_reference_matrices(id).is_err());
                }
                assert_eq!(session.get(id).unwrap().channels.unwrap()[3][8] & 0x100, 0);
            }
            assert!(
                session
                    .advance_passive_world_after_history(
                        id,
                        crate::passive_frame::Frame {
                            number: number as u64,
                            global_seconds: seconds,
                            areas: &[],
                            controlled: Some(controlled)
                        },
                        &mut crate::passive_frame::Services {
                            factory: &factory,
                            globals: &globals,
                            random: &mut random,
                            ground: &mut Ground,
                            stamp: &mut stamp,
                            finalization: &mut finalization
                        },
                    )
                    .is_err()
            );
        }
    }
    assert_eq!(random.0, 314);
    // A failure after ANM must retain the consumed clock, references and epoch,
    // and quarantine the callback rather than replaying it as a fresh frame.
    struct FailedEnvironment;
    impl crate::actor_finalization::Services for FailedEnvironment {
        fn release_footprint(&mut self, _: Option<u32>) -> Result<(), String> {
            Err("unexpected footprint".into())
        }
        fn environment(&mut self, _: &mut crate::actor_finalization::State) -> Result<(), String> {
            Err("supplied late environment failure".into())
        }
    }
    let before = session.get(animated).unwrap().channels.unwrap()[0][1];
    let epoch = session.get(animated).unwrap().mecc_animation_stamp;
    session.prepare_passive_world_history(animated, 3).unwrap();
    let channels = session.get(animated).unwrap().channels.unwrap();
    session
        .advance_passive_world_after_history(
            animated,
            crate::passive_frame::Frame {
                number: 3,
                global_seconds: 0.,
                areas: &[],
                controlled: Some(controlled),
            },
            &mut crate::passive_frame::Services {
                factory: &factory,
                globals: &globals,
                random: &mut random,
                ground: &mut Ground,
                stamp: &mut stamp,
                finalization: &mut FailedEnvironment,
            },
        )
        .unwrap();
    assert_eq!(session.get(animated).unwrap().channels.unwrap(), channels);
    assert_eq!(session.get(animated).unwrap().mecc_animation_stamp, epoch);
    assert_eq!(
        session.current_reference_matrices(animated).unwrap().len(),
        15
    );
    session.prepare_passive_world_history(animated, 4).unwrap();
    // Explicit fault admission: the source stationary profile normally bypasses
    // environment. Clear that gate here to reach the failing service, without
    // claiming an ordinary writer of this flag.
    session
        .residents
        .get_mut(&animated.serial)
        .unwrap()
        .construction_fields
        .as_mut()
        .unwrap()
        .object_flags &= !0x10;
    let error = session
        .advance_passive_world_after_history(
            animated,
            crate::passive_frame::Frame {
                number: 4,
                global_seconds: 0.25,
                areas: &[],
                controlled: Some(controlled),
            },
            &mut crate::passive_frame::Services {
                factory: &factory,
                globals: &globals,
                random: &mut random,
                ground: &mut Ground,
                stamp: &mut stamp,
                finalization: &mut FailedEnvironment,
            },
        )
        .unwrap_err();
    assert_eq!(error, "supplied late environment failure");
    assert_ne!(
        session.get(animated).unwrap().channels.unwrap()[0][1],
        before
    );
    assert_ne!(session.get(animated).unwrap().mecc_animation_stamp, epoch);
    assert_eq!(
        session.current_reference_matrices(animated).unwrap().len(),
        15
    );
    assert!(session.prepare_passive_world_history(animated, 4).is_err());
    assert!(session.prepare_passive_world_history(animated, 5).is_err());
    for id in unsupported {
        assert!(session.prepare_passive_world_history(id, 0).is_err());
    }
    session.clear();
    assert!(session.passive_world_frame(controlled).is_err());
    println!(
        "passive_source actors={} callbacks={} unsupported_state0=0 supplied_admission=true whole_world=false",
        neutral.len(),
        neutral.len() * 3
    );
}
