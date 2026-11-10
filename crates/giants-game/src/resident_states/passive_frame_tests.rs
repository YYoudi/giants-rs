//! Installed empty-state0 callback coverage; caller/world admission is supplied.
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
    assert_eq!(neutral.len(), 129);
    assert_eq!(unsupported.len(), 5);
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
    for (number, seconds) in [1. / 60., 0., 0.1].into_iter().enumerate() {
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
    for id in unsupported {
        assert!(session.prepare_passive_world_history(id, 0).is_err());
    }
    session.clear();
    assert!(session.passive_world_frame(controlled).is_err());
    println!(
        "passive_source actors={} callbacks={} unsupported_state0=5 supplied_admission=true whole_world=false",
        neutral.len(),
        neutral.len() * 3
    );
}
