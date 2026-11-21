//! Installed owned-prefix controls. Damage/pose/lifetime, notification and
//! cue/stat consumers are explicit test inputs, not ordinary gameplay proof.
use super::*;
use crate::{
    controlled_mecc::{Frame, HitPose, HitResponse, HitState},
    ripper_health_drop::*,
};

struct World;
impl PrimaryServices for World {
    fn ground(&mut self, _: &Session, _: Id, _: [f32; 3]) -> Result<f32, String> {
        panic!("grounded fixture")
    }
    fn blocked(&mut self, _: &Session, _: Id, _: [f32; 3], _: [f32; 3]) -> Result<bool, String> {
        panic!("grounded fixture")
    }
    fn expired(&mut self, _: &Session, _: Id) -> Result<(), String> {
        Ok(())
    } // Explicit no-network test boundary.
}
struct Consumer {
    item: Id,
    player: Id,
    controller: HitState,
    pose: HitPose,
    before: HitState,
    receipt: Option<HitResponse>,
    events: Vec<&'static str>,
    names: Vec<&'static str>,
    fail: Option<&'static str>,
}
impl CollectionServices for Consumer {
    fn local_cue(
        &mut self,
        session: &Session,
        item: Id,
        recipient: Id,
        cue: u32,
    ) -> Result<(), String> {
        assert_eq!((item, recipient, cue), (self.item, self.player, 1596));
        assert_eq!(
            session.mecc_health(recipient).unwrap().damage.to_bits(),
            self.before.health.damage.to_bits()
        );
        self.events.push("cue");
        if self.fail == Some("cue") {
            return Err("supplied cue failure".into());
        }
        Ok(())
    }
    fn copy_health(&mut self, session: &Session, receipt: HitResponse) -> Result<(), String> {
        self.events.push("copy");
        self.receipt = Some(receipt);
        if self.fail == Some("copy") {
            return Err("supplied controller-copy failure".into());
        }
        let after =
            receipt.validate_controller(session, self.player, self.pose, self.controller)?;
        assert_eq!(
            after.velocity.map(f32::to_bits),
            self.controller.velocity.map(f32::to_bits)
        );
        self.controller = after;
        Ok(())
    }
    fn statistic(
        &mut self,
        session: &Session,
        recipient: Id,
        kind: Statistic,
    ) -> Result<(), String> {
        assert_eq!(recipient, self.player);
        assert_ne!(
            session
                .get(self.item)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap()
                .object_flags
                & 0x8000,
            0
        );
        self.events.push("stat");
        if self.fail == Some("stat") {
            return Err("supplied named-stat failure".into());
        }
        self.names.push(kind.source_name());
        Ok(()) // Supplied counter consumer; native control stops before lookup.
    }
}
fn player_fixture(
    session: &mut Session,
    resources: &crate::scene_resources::SceneResources,
    damage: f32,
) -> (crate::controlled_mecc_contacts::Snapshot, Consumer) {
    let player = crate::ripper_health_drop::tests::source_at(resources, session, 2, [0.; 3]);
    crate::mecc_special::Factory::new(resources)
        .initialize_player_fields(
            session,
            player,
            crate::animation_channels::Blending::Enabled,
        )
        .unwrap();
    let f = session
        .get(player)
        .unwrap()
        .construction_fields
        .as_ref()
        .unwrap()
        .clone();
    session
        .publish_controlled_mecc_motion(
            player,
            1,
            Frame {
                before: f.position,
                position: f.position,
                contact_previous: f.position,
                angles: f.angles,
                scale: f.scale,
                velocity: [0.; 3],
                model_rates: [0.; 3],
            },
        )
        .unwrap();
    session
        .residents
        .get_mut(&player.serial)
        .unwrap()
        .mecc_damage = Some(damage); // Explicit native fixture input.
    let specification = session.specification(player).unwrap().clone();
    let contacts =
        crate::controlled_mecc_contacts::Resources::load(resources, specification.clone()).unwrap();
    let descriptor = &specification.animations[&1];
    let tree = descriptor
        .animation
        .trees
        .iter()
        .find(|t| t.offset == descriptor.animation.default_tree)
        .unwrap();
    let packet = session
        .controlled_mecc_contact_snapshot(
            player,
            1,
            &contacts,
            crate::controlled_mecc_contacts::Sample {
                slot: 1,
                name: &descriptor.name,
                tree: &tree.name,
                seconds: 0.,
            },
        )
        .unwrap();
    let current = HitState {
        health: session.mecc_health(player).unwrap(),
        velocity: session
            .controlled_mecc_motion(player, 1)
            .unwrap()
            .frame
            .velocity,
    };
    let f = session
        .get(player)
        .unwrap()
        .construction_fields
        .as_ref()
        .unwrap();
    let consumer = Consumer {
        item: player,
        player,
        controller: current,
        before: current,
        pose: HitPose {
            position: f.position,
            angles: f.angles,
            scale: f.scale,
        },
        receipt: None,
        events: vec![],
        names: vec![],
        fail: None,
    };
    (packet, consumer)
}

#[test]
#[ignore = "Requires GIANTS_NPC_ROOT/GIANTS_RIPPER_HEALTH_COLLECTION_CONTROLS; actual birth, controller packet and canonical health copy, supplied cue/stat consumers, no GPU/gameplay acceptance"]
fn native_health_collection_matches_owned_contacts_and_keeps_late_failure_prefixes() {
    let root = std::path::PathBuf::from(std::env::var_os("GIANTS_NPC_ROOT").unwrap());
    let resources =
        crate::scene_resources::SceneResources::load(&root, "w_Story2.bin", "English").unwrap();
    let globals =
        giants_formats::global_data::parse(&std::fs::read(root.join("Bin/GData.gbn")).unwrap())
            .unwrap();
    let factory = Factory::load(&resources, &globals).unwrap();
    let weapons = giants_formats::weapons::parse_with_profile(
        &std::fs::read(root.join("Bin/weapondefs.bin")).unwrap(),
        giants_formats::weapons::Profile::Debug,
    )
    .unwrap();
    let definition = PickupDefinition::load(&weapons).unwrap();
    let text = std::fs::read_to_string(
        std::env::var_os("GIANTS_RIPPER_HEALTH_COLLECTION_CONTROLS").unwrap(),
    )
    .unwrap();
    let mut cases = 0;
    for line in text.lines() {
        let w: Vec<u32> = line
            .split_whitespace()
            .map(|s| u32::from_str_radix(s, 16).unwrap())
            .collect();
        assert_eq!(w.len(), 7);
        let large = w[0] != 0;
        let damage = f32::from_bits(w[1]);
        let far = w[2] != 0;
        let lifetime = f32::from_bits(w[3]);
        let failures: &[Option<&str>] = if damage == 10. && !large && !far {
            &[None, Some("cue"), Some("copy"), Some("stat")]
        } else {
            &[None]
        };
        for &fail in failures {
            let (mut session, item, _fx) =
                crate::ripper_health_drop::tests::birth_fixture_for_class(
                    &resources,
                    &factory,
                    if large { 230 } else { 73 },
                    [if far { 1000. } else { 1.25 }, -2., 2.],
                );
            let actor = session.residents.get_mut(&item.serial).unwrap();
            actor.ripper_health_drop.as_mut().unwrap().lifetime = lifetime;
            actor.seed.storage.data_mut().unwrap()[20..24].copy_from_slice(&lifetime.to_le_bytes());
            let (mut packet, mut consumer) = player_fixture(&mut session, &resources, damage);
            consumer.item = item;
            consumer.fail = fail;
            let PrimaryPrefix::Collection(pending) = factory
                .advance_primary_prefix(
                    &mut session,
                    item,
                    PrimaryInput {
                        step: 1,
                        global_seconds: 0.5,
                        areas: &[],
                        paused: false,
                        force_ground: false,
                    },
                    &mut World,
                )
                .unwrap()
            else {
                panic!("collection token required")
            };
            if damage == 10. && !large && !far && lifetime == 30. && fail.is_none() {
                packet.step = 2;
                assert!(
                    factory
                        .finish_collection(
                            &mut session,
                            &pending,
                            &packet,
                            definition,
                            &mut consumer
                        )
                        .is_err()
                );
                assert_eq!(
                    session.ripper_health_drop(item).unwrap().primary_phase,
                    PrimaryPhase::CollectionPending
                );
                assert!(consumer.events.is_empty());
                packet.step = 1;
            }
            let collected = damage > 0. && !far;
            let result = factory.finish_collection(
                &mut session,
                &pending,
                &packet,
                definition,
                &mut consumer,
            );
            let owned = session.ripper_health_drop(item).unwrap();
            if collected && fail.is_some() {
                assert!(result.is_err());
                assert!(owned.failed);
                assert_eq!(owned.primary_phase, PrimaryPhase::Failed);
                let expected = if fail == Some("cue") { w[1] } else { w[5] };
                assert_eq!(
                    session.mecc_health(packet.owner).unwrap().damage.to_bits(),
                    expected
                );
                assert!(consumer.names.is_empty());
                let marked = lifetime <= 0.5 || fail == Some("stat");
                let flags = session
                    .get(item)
                    .unwrap()
                    .construction_fields
                    .as_ref()
                    .unwrap()
                    .object_flags;
                assert_eq!(flags & 0x8000 != 0, marked);
            } else {
                assert_eq!(
                    result.unwrap(),
                    if collected {
                        CollectionOutcome::Collected
                    } else {
                        CollectionOutcome::NotCollected
                    }
                );
                assert!(!owned.failed);
                assert_eq!(owned.primary_phase, PrimaryPhase::Idle);
                assert_eq!(
                    session.mecc_health(packet.owner).unwrap().damage.to_bits(),
                    w[5]
                );
                assert_eq!(
                    session
                        .get(item)
                        .unwrap()
                        .construction_fields
                        .as_ref()
                        .unwrap()
                        .object_flags,
                    w[6]
                );
                if collected {
                    assert_eq!(consumer.events, ["cue", "copy", "stat"]);
                    assert_eq!(
                        consumer.names,
                        [if large {
                            "InstantHealthLarge"
                        } else {
                            "InstantHealthMed"
                        }]
                    );
                } else {
                    assert!(consumer.events.is_empty());
                }
            }
            let count = consumer.events.len();
            assert!(
                factory
                    .finish_collection(&mut session, &pending, &packet, definition, &mut consumer)
                    .is_err()
            );
            assert_eq!(
                consumer.events.len(),
                count,
                "closed/failed collection cannot repeat services"
            );
            if let Some(receipt) = consumer.receipt {
                assert!(
                    receipt
                        .validate_controller(&session, packet.owner, consumer.pose, consumer.before)
                        .is_err(),
                    "closed/failed collection must invalidate its copy receipt"
                );
            }
        }
        cases += 1;
    }
    assert_eq!(cases, 8);
    println!(
        "health_collection_native_match cases={cases} failure_variants=4 real_contact_packet=true native_stat_lookup_stopped=true runtime_acknowledged=false"
    );
}
