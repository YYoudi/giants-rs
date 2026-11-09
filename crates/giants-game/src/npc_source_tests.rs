//! Source-resource composition checks; no original executable, window or sound.
use crate::{
    audio_parameters::Random, herd_generator, herd_markers, resident_states::Session,
    scene_resources::SceneResources, vimp_actor,
};
use std::path::PathBuf;

struct Floor {
    fail: bool,
    queries: usize,
}
impl herd_generator::Ground for Floor {
    fn height(&mut self, _: [f32; 3]) -> Result<f32, String> {
        self.queries += 1;
        if self.fail {
            Err("injected NPC terrain failure".into())
        } else {
            Ok(7.25)
        }
    }
}

#[test]
#[ignore = "Requires GIANTS_NPC_ROOT; authored herd births and initialization, supplied flat floor/clock/RNG, no live AI or audio"]
fn source_npc_herd_births_keep_ownership_and_quarantine_failures() {
    let root = PathBuf::from(std::env::var_os("GIANTS_NPC_ROOT").expect("GIANTS_NPC_ROOT"));
    let resources = SceneResources::load(&root, "xx_permanentaliens.bin", "English").unwrap();
    let herds = herd_generator::Factory::load(&resources).unwrap();
    let markers = herd_markers::Factory::load(&resources).unwrap();
    let effects = giants_formats::effect_image::parse(
        &std::fs::read(root.join("Bin/fx_world_dust.bin")).unwrap(),
    )
    .unwrap();
    let globals =
        giants_formats::global_data::parse(&std::fs::read(root.join("Bin/GData.gbn")).unwrap())
            .unwrap();
    let mut worlds = std::fs::read_dir(root.join("Bin"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("w_") && n.ends_with(".bin"))
        })
        .collect::<Vec<_>>();
    worlds.sort();
    let (mut parents, mut births, mut failures, mut unsupported, mut inactive) = (0, 0, 0, 0, 0);
    let mut unparsed = 0;
    for path in worlds {
        let world = match giants_formats::world::parse_world(&std::fs::read(&path).unwrap()) {
            Ok(world) => world,
            Err(error) => {
                // A source parser refusal is an uncovered world, never an empty
                // population. Keep it visible separately from tested parents.
                unparsed += 1;
                eprintln!(
                    "npc_world_parse_refused file={} reason={error}",
                    path.file_name().unwrap().to_string_lossy()
                );
                continue;
            }
        };
        if !world.placements.iter().any(|p| p.object_type == 49) {
            continue;
        }
        let source = SceneResources::load(
            &root,
            path.file_name().unwrap().to_str().unwrap(),
            "English",
        )
        .unwrap();
        let vimps = vimp_actor::Factory::load(&source, &effects).unwrap();
        for parent in world.placements.iter().filter(|p| p.object_type == 49) {
            for fail in [false, true] {
                parents += usize::from(!fail);
                let mut session = Session::new(100 + parents as u64 * 2 + u64::from(fail));
                // Keep authored declaration order and all authored marker values.
                let mut owner = None;
                for placement in &world.placements {
                    if placement.source_offset == parent.source_offset {
                        owner = Some(herds.place(&mut session, placement).unwrap());
                    } else if placement.object_type == 679 {
                        markers.place(&mut session, placement).unwrap();
                    }
                }
                let owner = owner.unwrap();
                let mut random = Random(1);
                let mut floor = Floor { fail, queries: 0 };
                let mut observed = false;
                for _ in 0..40 {
                    let step = session.herd_clock(owner).unwrap().steps;
                    let before = session.counts();
                    let before_random = random.0;
                    let tick = || herd_generator::Tick {
                        step,
                        global_delta: 0.25,
                        slow_areas: &[],
                    };
                    let result = session.advance_herd_offline(
                        owner,
                        tick(),
                        &vimps,
                        &globals,
                        &mut random,
                        &mut floor,
                    );
                    match result {
                        Ok(children) => {
                            if session.actor_world_flags(owner).unwrap() & 0x8000 != 0 {
                                assert!(children.is_empty());
                                assert_eq!(floor.queries, 0);
                                inactive += usize::from(!fail);
                                observed = true;
                                break;
                            }
                            assert!(!fail || children.is_empty());
                            for child in children {
                                let born = session.vimp(child).unwrap().born.unwrap();
                                assert_eq!(born.parent, owner);
                                session
                                    .initialize_vimp_offline(child, &globals, &mut random)
                                    .unwrap();
                                assert!(session.vimp(child).unwrap().initialized);
                                assert_eq!(session.vimp(child).unwrap().mode, 1);
                                assert_eq!(
                                    session.vimp(child).unwrap().born.unwrap().parent,
                                    owner
                                );
                                let rng = random.0;
                                assert!(
                                    session
                                        .initialize_vimp_offline(child, &globals, &mut random)
                                        .is_err()
                                );
                                assert_eq!(random.0, rng);
                                births += 1;
                                observed = true;
                            }
                        }
                        Err(error) => {
                            if error
                                == "Vimp birth requires acquired configured herd parent for class8"
                            {
                                // Other authored herd profiles are real exclusions, not
                                // successful Vimp population or a reason to rewrite data.
                                assert_eq!(step, 0);
                                assert_eq!(floor.queries, 0);
                                assert_eq!(random.0, before_random);
                                assert_eq!(session.counts(), before);
                                assert!(session.herd_clock(owner).unwrap().failure.is_none());
                                unsupported += usize::from(!fail);
                                observed = true;
                                break;
                            }
                            assert!(fail, "unexpected herd failure: {error}");
                            assert!(error.contains("injected NPC terrain failure"), "{error}");
                            assert!(session.herd_clock(owner).unwrap().failure.is_some());
                            let rng = random.0;
                            let calls = floor.queries;
                            assert!(
                                session
                                    .advance_herd_offline(
                                        owner,
                                        tick(),
                                        &vimps,
                                        &globals,
                                        &mut random,
                                        &mut floor
                                    )
                                    .is_err()
                            );
                            assert_eq!((random.0, floor.queries), (rng, calls));
                            failures += 1;
                            observed = true;
                            break;
                        }
                    }
                    if observed {
                        break;
                    }
                }
                // No birth was requested during this supplied time window.
                if !observed {
                    assert_eq!(floor.queries, 0);
                }
                let foreign = Session::new(9_000_000);
                assert!(foreign.get(owner).is_none());
                session.clear();
                assert!(session.get(owner).is_none());
                assert!(session.list(10).is_empty());
                assert_eq!(session.counts(), crate::resident_states::Counts::default());
            }
        }
    }
    assert!(parents > 0 && births > 0 && failures > 0);
    println!(
        "npc_source_herds parents={parents} births={births} quarantined={failures} unsupported_authored_parents={unsupported} inactive_parents={inactive} unparsed_worlds={unparsed} supplied_floor=true live_ai=false"
    );
}
