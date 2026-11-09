use super::*;
use crate::{
    kabuto_contact::FollowupPeer,
    kabuto_object_contacts::{Event, Peer, Scene},
};

struct Empty;
impl crate::kabuto_classification::Peers for Empty {
    fn profile(&mut self, _: u32) -> Result<crate::kabuto_contact::Peer, &'static str> {
        Err("empty scene")
    }
    fn bind_support(&mut self, _: u32, _: u32) -> Result<(), &'static str> {
        Err("empty scene")
    }
}
fn context() -> crate::kabuto_geometry_mode::NotificationContext<'static> {
    crate::kabuto_geometry_mode::NotificationContext {
        session_mode: 0,
        peer_ids: &[],
        local_peer: 0,
        owner_word: 0,
    }
}
fn classify(factory: &crate::kabuto_initial::Factory, s: &mut Session, id: Id, number: u64) {
    assert!(s.begin_kabuto_motion_history(id, number, [0; 8]).unwrap());
    factory
        .advance_motion_prefix(
            s,
            id,
            number,
            crate::kabuto_motion_prefix::Frame {
                global_seconds: 0.,
                areas: &[],
                sensitivity: 1.,
                local_owner: true,
                skip_heading: false,
                parent_present: false,
            },
        )
        .unwrap();
    factory
        .advance_classification(
            s,
            id,
            number,
            crate::kabuto_classification::Scene {
                owner: 1,
                objects: &[],
                peers: &mut Empty,
            },
            |xy| {
                Ok(crate::physical_ground::Ground {
                    plane: [0., 0., 1., -0.],
                    height: -((f64::from(xy[0]) * 0. + f64::from(xy[1]) * 0.) + -0.) as f32,
                    key: [0, 0],
                })
            },
        )
        .unwrap();
}
fn construct(factory: &crate::kabuto_initial::Factory, s: &mut Session) -> Id {
    factory
        .construct(
            s,
            ColdFrame {
                position: [0., 0., 10.],
                current: [0.; 3],
                previous: [0.; 3],
                angles: [0.; 3],
            },
            Blending::Disabled,
            1,
        )
        .unwrap()
}
fn peer(id: u32) -> Peer<u32> {
    Peer {
        contact: FollowupPeer {
            identity: id,
            flags: 0x3c0_0000,
            reference: None,
        },
        object_flags: 0,
        collision_flags: 0,
        class_present: true,
        parent: None,
        upper_z: 11.,
        radius: 1.,
        pose: crate::uncached_pose::Pose {
            position: [0., 0., 9.],
            angles: [0.; 3],
            scale: 1.,
        },
        affine: None,
        bounds: None,
    }
}
fn factor(s: &Session, id: Id) -> f32 {
    f32::from_le_bytes(
        s.get(id).unwrap().seed.storage.data().unwrap()[0x1d0..0x1d4]
            .try_into()
            .unwrap(),
    )
}
fn put_factor(s: &mut Session, id: Id, v: f32) {
    s.residents
        .get_mut(&id.serial)
        .unwrap()
        .seed
        .storage
        .data_mut()
        .unwrap()[0x1d0..0x1d4]
        .copy_from_slice(&v.to_le_bytes());
}
fn emit(event: Event<u32>, out: &mut Vec<u32>) {
    match event {
        Event::Probes {
            peer,
            mask,
            probes,
            count,
        } => {
            out.extend([0, peer, mask, count as u32]);
            for p in &probes[..count] {
                out.extend(p.position.map(f32::to_bits));
                out.extend([p.height.to_bits(), p.radius.to_bits()]);
            }
        }
        Event::Contact {
            peer,
            mask,
            flags,
            reference,
        } => out.extend([1, peer, mask, flags, reference.unwrap_or(0)]),
        Event::Reference { peer, reference } => out.extend([3, peer, reference.unwrap_or(0)]),
        Event::BoxQuery {
            peer,
            point,
            extent,
            bounds,
        } => {
            out.extend([4, peer]);
            out.extend(point.map(f32::to_bits));
            out.push(extent.to_bits());
            out.extend(bounds.min.map(f32::to_bits));
            out.extend(bounds.max.map(f32::to_bits));
        }
        Event::Distance { peer, distance } => out.extend([5, peer, distance.to_bits()]),
    }
}
fn parse_peers(w: &[u32]) -> Vec<Peer<u32>> {
    w.as_chunks::<42>()
        .0
        .iter()
        .map(|p| {
            let f = |i| f32::from_bits(p[i]);
            Peer {
                contact: FollowupPeer {
                    identity: p[0],
                    flags: p[38],
                    reference: (p[39] != 0).then_some(p[39]),
                },
                object_flags: p[1] | if p[14] != 0 { 0x20 } else { 0 },
                collision_flags: p[2],
                class_present: p[3] != 0,
                parent: (p[4] != 0).then_some(p[4]),
                upper_z: f(5),
                radius: f(6),
                pose: crate::uncached_pose::Pose {
                    position: [f(7), f(8), f(9)],
                    scale: f(10),
                    angles: [f(11), f(12), f(13)],
                },
                affine: Some(crate::actor_affine::Cache {
                    flags: p[15],
                    matrix: std::array::from_fn(|i| f(16 + i)),
                }),
                bounds: Some(crate::volume_queries::Bounds {
                    min: [f(32), f(33), f(34)],
                    max: [f(35), f(36), f(37)],
                }),
            }
        })
        .collect()
}
#[test]
#[ignore = "Requires GIANTS_KABUTO_ROOT; optional private classified-boundary controls; no window or sound"]
fn source_kabuto_object_contacts_bind_real_storage_and_preserve_entered_history() {
    let (factory, _) = super::transition_tests::factories();
    let mut s = Session::new(981);
    let id = construct(&factory, &mut s);
    let controls = std::env::var_os("GIANTS_KABUTO_OBJECT_CONTROLS")
        .map(|p| std::fs::read_to_string(p).unwrap());
    let mut receipts = String::new();
    let mut native_inputs = String::new();
    if let Some(controls) = controls {
        let fields = s.get(id).unwrap().construction_fields.clone().unwrap();
        let payload = s.get(id).unwrap().seed.storage.data().unwrap().to_vec();
        let radius = s.kabuto_group(id).unwrap().radius;
        let root = std::path::PathBuf::from(std::env::var("GIANTS_KABUTO_ROOT").unwrap());
        let globals =
            giants_formats::global_data::parse(&std::fs::read(root.join("Bin/GData.gbn")).unwrap())
                .unwrap();
        for (row, line) in controls.lines().enumerate() {
            let w: Vec<u32> = line
                .split_whitespace()
                .map(|v| u32::from_str_radix(v, 16).unwrap())
                .collect();
            let n = w[34] as usize;
            let np = w[35] as usize;
            assert_eq!(w.len(), 38 + np + n * 42);
            let a = s.residents.get_mut(&id.serial).unwrap();
            a.construction_fields = Some(fields.clone());
            a.construction_fields.as_mut().unwrap().position =
                std::array::from_fn(|i| f32::from_bits(w[7 + i]));
            a.affine_cache = None;
            a.seed.storage.data_mut().unwrap().copy_from_slice(&payload);
            a.seed.storage.data_mut().unwrap()[0x1d0..0x1d4].copy_from_slice(&w[6].to_le_bytes());
            let physics =
                crate::kabuto_physics::Physics::from_owned_payload(a.seed.storage.data().unwrap());
            let g = a.kabuto_group.as_mut().unwrap();
            g.physics = Some(physics);
            g.radius = radius;
            g.motion = Default::default();
            g.contact = Default::default();
            g.motion_history = None;
            g.motion_prefix_completed = None;
            g.classification_completed = None;
            g.object_contacts_entered = None;
            g.object_contacts_completed = None;
            let number = row as u64 + 1;
            classify(&factory, &mut s, id, number);
            let mut peers = parse_peers(&w[38 + np..]);
            let raw: Vec<_> = w[38 + np..].as_chunks::<42>().0.iter().collect();
            let peer_ids = &w[38..38 + np];
            let ctx = crate::kabuto_geometry_mode::NotificationContext {
                session_mode: w[2],
                local_peer: w[3],
                owner_word: w[4],
                peer_ids,
            };
            let a = s.get(id).unwrap();
            let f = a.construction_fields.as_ref().unwrap();
            let cache = a.affine_cache.as_ref().unwrap().value;
            let storage = s.reference_cache_storage(id).unwrap();
            let refs: [f32; 6] =
                std::array::from_fn(|i| storage.matrices[8 + i / 3].values[[3, 7, 11][i % 3]]);
            let mut input = vec![
                w[0],
                globals.words[0x119c / 4],
                w[2],
                w[3],
                w[4],
                1,
                factor(&s, id).to_bits(),
            ];
            input.extend(f.position.map(f32::to_bits));
            input.push(f.scale.to_bits());
            input.push(cache.flags);
            input.extend(cache.matrix.map(f32::to_bits));
            input.extend(refs.map(f32::to_bits));
            input.extend([n as u32, np as u32, w[36], w[37]]);
            input.extend(peer_ids);
            input.extend(&w[38 + np..]);
            native_inputs.push_str(
                &input
                    .iter()
                    .map(|v| format!("{v:08x}"))
                    .collect::<Vec<_>>()
                    .join(" "),
            );
            native_inputs.push('\n');
            let before = super::motion_tests::snapshot(&s, id);
            let counts = s.counts();
            let lists = (0..15).map(|i| s.list(i)).collect::<Vec<_>>();
            let events = std::cell::RefCell::new(Vec::new());
            let result = factory
                .advance_object_contacts(
                    &mut s,
                    id,
                    number,
                    Scene {
                        owner: 1,
                        disabled: w[0] != 0,
                        peers: &mut peers,
                        context: &ctx,
                    },
                    |s, r, p| {
                        let i = (r.peer - 2) as usize;
                        events.borrow_mut().extend([
                            2,
                            r.peer,
                            r.action,
                            r.amount.to_bits(),
                            r.owner.unwrap_or(0),
                            r.peer,
                            r.trailing[0],
                            r.trailing[1],
                            p[i].contact.flags,
                            p[i].contact.reference.unwrap_or(0),
                        ]);
                        p[i].contact.flags = raw[i][40];
                        p[i].contact.reference = (raw[i][41] != 0).then_some(raw[i][41]);
                        if w[36] != 0 {
                            p[(i + 1) % n].contact.flags = 1;
                        }
                        if w[37] != 0 {
                            let next = (f64::from(factor(s, id)) + 0.125) as f32;
                            put_factor(s, id, next);
                        }
                        Ok(())
                    },
                    |e| emit(e, &mut events.borrow_mut()),
                )
                .unwrap();
            let after = super::motion_tests::snapshot(&s, id);
            assert!(
                before
                    .iter()
                    .zip(&after)
                    .enumerate()
                    .all(|(i, (a, b))| i == 193 + 0x1d0 / 4 || a == b)
            );
            assert_eq!(s.counts(), counts);
            assert_eq!((0..15).map(|i| s.list(i)).collect::<Vec<_>>(), lists);
            let mut out = vec![u32::from(result), factor(&s, id).to_bits()];
            for p in peers {
                out.extend([p.contact.flags, p.contact.reference.unwrap_or(0)]);
            }
            let events = events.into_inner();
            out.push(events.len() as u32);
            out.extend(events);
            receipts.push_str(
                &out.iter()
                    .map(|v| format!("{v:08x}"))
                    .collect::<Vec<_>>()
                    .join(" "),
            );
            receipts.push('\n');
            assert_eq!(
                s.kabuto_group(id).unwrap().object_contacts_completed,
                Some(number)
            );
            assert!(
                s.begin_kabuto_motion_history(id, number + 1, [0; 8])
                    .is_err()
            );
        }
    } else {
        put_factor(&mut s, id, 1.);
        classify(&factory, &mut s, id, 1);
        let ctx = context();
        let mut peers = [peer(2), peer(3)];
        let (foreign, _) = super::transition_tests::factories();
        assert!(
            foreign
                .advance_object_contacts(
                    &mut s,
                    id,
                    1,
                    Scene {
                        owner: 1,
                        disabled: false,
                        peers: &mut peers,
                        context: &ctx
                    },
                    |_, _, _| Err("foreign"),
                    |_| {}
                )
                .is_err()
        );
        assert!(
            factory
                .advance_object_contacts(
                    &mut s,
                    id,
                    2,
                    Scene {
                        owner: 1,
                        disabled: false,
                        peers: &mut peers,
                        context: &ctx
                    },
                    |_, _, _| Err("wrong number"),
                    |_| {}
                )
                .is_err()
        );
        let mut calls = 0;
        assert!(
            !factory
                .advance_object_contacts(
                    &mut s,
                    id,
                    1,
                    Scene {
                        owner: 1,
                        disabled: false,
                        peers: &mut peers,
                        context: &ctx
                    },
                    |s, r, p| {
                        calls += 1;
                        assert_eq!(r.action, 11);
                        let mut empty = [];
                        assert!(
                            factory
                                .advance_object_contacts(
                                    s,
                                    id,
                                    1,
                                    Scene {
                                        owner: 1,
                                        disabled: false,
                                        peers: &mut empty,
                                        context: &ctx
                                    },
                                    |_, _, _| Err("reentered"),
                                    |_| {}
                                )
                                .is_err()
                        );
                        let next = factor(s, id) + 0.5;
                        put_factor(s, id, next);
                        p[(r.peer - 2) as usize].contact.flags = 1;
                        Ok(())
                    },
                    |_| {}
                )
                .unwrap()
        );
        assert_eq!(calls, 2);
        assert_eq!(factor(&s, id), 2.);
        assert!(peers.iter().all(|p| p.contact.reference == Some(1)));
        assert!(
            factory
                .advance_object_contacts(
                    &mut s,
                    id,
                    1,
                    Scene {
                        owner: 1,
                        disabled: false,
                        peers: &mut peers,
                        context: &ctx
                    },
                    |_, _, _| Err("replay"),
                    |_| {}
                )
                .is_err()
        );
    }
    if let Some(p) = std::env::var_os("GIANTS_KABUTO_OBJECT_RECEIPT") {
        std::fs::write(p, receipts).unwrap();
    }
    if let Some(p) = std::env::var_os("GIANTS_KABUTO_OBJECT_NATIVE_INPUTS") {
        std::fs::write(p, native_inputs).unwrap();
    }
    assert_eq!(s.kabuto_group(id).unwrap().pending.len(), 4);
    assert!(s.begin_kabuto_motion_history(id, 99999, [0; 8]).is_err());
    s.clear();
    assert_eq!(s.counts().residents, 0);
}
#[test]
#[ignore = "Requires GIANTS_KABUTO_ROOT; explicit failing/cancelling action providers; no window or sound"]
fn source_kabuto_object_contacts_retain_action_failure_and_allow_owner_cancellation() {
    let (factory, _) = super::transition_tests::factories();
    let ctx = context();
    for clear in [false, true] {
        let mut s = Session::new(if clear { 983 } else { 982 });
        let id = construct(&factory, &mut s);
        put_factor(&mut s, id, 1.);
        classify(&factory, &mut s, id, 1);
        let mut peers = [peer(2)];
        let result = factory.advance_object_contacts(
            &mut s,
            id,
            1,
            Scene {
                owner: 1,
                disabled: false,
                peers: &mut peers,
                context: &ctx,
            },
            |s, _, p| {
                p[0].contact.flags = 1;
                p[0].contact.reference = Some(77);
                if clear {
                    s.clear();
                    Ok(())
                } else {
                    put_factor(s, id, 2.);
                    Err("injected additional action failure")
                }
            },
            |_| {},
        );
        assert!(result.is_err());
        assert_eq!(peers[0].contact.reference, Some(77));
        if clear {
            assert!(s.get(id).is_none());
            assert_eq!(s.counts().residents, 0);
        } else {
            assert_eq!(factor(&s, id), 2.);
            assert!(
                s.kabuto_group(id)
                    .unwrap()
                    .geometry_transition_failure()
                    .is_some()
            );
            assert!(
                factory
                    .advance_object_contacts(
                        &mut s,
                        id,
                        1,
                        Scene {
                            owner: 1,
                            disabled: false,
                            peers: &mut peers,
                            context: &ctx
                        },
                        |_, _, _| Err("quarantined replay"),
                        |_| {}
                    )
                    .is_err()
            );
            assert!(s.begin_kabuto_motion_history(id, 2, [0; 8]).is_err());
            s.clear();
        }
    }
}
