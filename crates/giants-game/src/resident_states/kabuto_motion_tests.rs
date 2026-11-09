use super::*;
use crate::kabuto_motion_prefix::{Frame, State};
struct EmptyPeers;
impl crate::kabuto_classification::Peers for EmptyPeers {
    fn profile(&mut self, _: u32) -> Result<crate::kabuto_contact::Peer, &'static str> {
        Err("no peer metadata supplied")
    }
    fn bind_support(&mut self, _: u32, _: u32) -> Result<(), &'static str> {
        Err("no peer support backend supplied")
    }
}
fn classify(
    factory: &crate::kabuto_initial::Factory,
    s: &mut Session,
    owner: Id,
    number: u64,
) -> Result<(), String> {
    factory.advance_classification(
        s,
        owner,
        number,
        crate::kabuto_classification::Scene {
            owner: 17,
            objects: &[],
            peers: &mut EmptyPeers,
        },
        |xy| {
            Ok(crate::physical_ground::Ground {
                plane: [0., 0., 1., -0.],
                height: -((f64::from(xy[0]) * 0. + f64::from(xy[1]) * 0.) + -0.) as f32,
                key: [0, 0],
            })
        },
    )
}
fn header(s: &Session, id: Id) -> Vec<u32> {
    let mut h = super::growth_tests::header(s, id);
    let m = s.kabuto_group(id).unwrap().motion_prefix();
    h[0x1e0 / 4] = u32::from(s.get(id).unwrap().seed.storage.state());
    h[0x278 / 4] = s.kabuto_damage_state(id).unwrap().0.to_bits();
    h[0xd4 / 4] = m.look.to_bits();
    h[0xe4 / 4] = m.mode;
    h[0x158 / 4] = m.target_heading.to_bits();
    for (at, values) in [
        (0x108, m.model.to_vec()),
        (0x118, m.previous_model.to_vec()),
        (0x128, m.velocity.to_vec()),
        (0x134, m.previous_velocity.to_vec()),
    ] {
        for (i, v) in values.into_iter().enumerate() {
            h[at / 4 + i] = v.to_bits();
        }
    }
    for (i, v) in m.commands.as_chunks::<4>().0.iter().enumerate() {
        h[0x208 / 4 + i] = u32::from_le_bytes(*v);
    }
    let c = s.kabuto_group(id).unwrap().contact();
    h[0x140 / 4] = c.floor.to_bits();
    for (i, v) in c.plane.iter().enumerate() {
        h[0x144 / 4 + i] = v.to_bits();
    }
    h[0x224 / 4] = c.support;
    h[0x220 / 4] = c.hit;
    h[0x228 / 4] = c.support_part;
    h[0x22c / 4] = c.support_triangle;
    if let Some(a) = &s.get(id).unwrap().affine_cache {
        h[0x294 / 4] = a.value.flags;
        for (i, v) in a.value.matrix.iter().enumerate() {
            h[0x298 / 4 + i] = v.to_bits();
        }
    }
    h
}
pub(super) fn snapshot(s: &Session, id: Id) -> Vec<u32> {
    let mut w = header(s, id);
    let mut p: Vec<_> = s
        .get(id)
        .unwrap()
        .seed
        .storage
        .data()
        .unwrap()
        .as_chunks::<4>()
        .0
        .iter()
        .map(|v| u32::from_le_bytes(*v))
        .collect();
    p[0x150 / 4] = 3;
    p[0x154 / 4] = 1;
    p[0x220 / 4] = 1;
    for i in 0..8 {
        p[(0x22c + i * 28) / 4] = 4 + i as u32;
    }
    p[0x314 / 4] = 12;
    w.extend(p);
    w
}
#[test]
#[ignore = "Requires GIANTS_KABUTO_ROOT; optional private motion-prefix controls; no window or audio"]
fn source_kabuto_motion_prefix_owns_fields_and_retains_pending_primary() {
    run_motion(false);
}
#[test]
#[ignore = "Requires GIANTS_KABUTO_ROOT; optional private composed classification controls; no window or sound"]
fn source_kabuto_classification_owns_contacts_radius_and_affine_without_finishing_primary() {
    run_motion(true);
}
#[test]
#[ignore = "Requires GIANTS_KABUTO_ROOT; prepared mesh/peer services and late provider failure; no window or sound"]
fn source_kabuto_classification_keeps_peer_effects_and_quarantines_late_failure() {
    struct Peers {
        support: u32,
        calls: u32,
    }
    impl crate::kabuto_classification::Peers for Peers {
        fn profile(&mut self, key: u32) -> Result<crate::kabuto_contact::Peer, &'static str> {
            assert_eq!(key, 77);
            self.calls += 1;
            Ok(crate::kabuto_contact::Peer {
                class: 2,
                upper_z: 1.,
                radius: 1.,
                collision_mask: 0x3ffc,
            })
        }
        fn bind_support(&mut self, peer: u32, owner: u32) -> Result<(), &'static str> {
            assert_eq!(peer, 77);
            self.support = owner;
            Ok(())
        }
    }
    let (factory, _) = super::transition_tests::factories();
    for success in [true, false] {
        let mut s = Session::new(if success { 978 } else { 979 });
        let owner = factory
            .construct(
                &mut s,
                ColdFrame {
                    position: [0.; 3],
                    current: [0., 0., 1.],
                    previous: [0.; 3],
                    angles: [0.; 3],
                },
                Blending::Disabled,
                1,
            )
            .unwrap();
        // Supply a preceding physical pose after cold construction. The cold
        // projection itself deliberately initializes current == position.
        s.residents
            .get_mut(&owner.serial)
            .unwrap()
            .construction_fields
            .as_mut()
            .unwrap()
            .current[2] = 1.;
        s.begin_kabuto_motion_history(owner, 1, [0; 8]).unwrap();
        factory
            .advance_motion_prefix(
                &mut s,
                owner,
                1,
                Frame {
                    global_seconds: 0.,
                    areas: &[],
                    sensitivity: 1.,
                    local_owner: true,
                    skip_heading: false,
                    parent_present: false,
                },
            )
            .unwrap();
        let before = snapshot(&s, owner);
        let objects = [crate::player_world_collision::Object {
            fields: crate::mecc_collision_candidates::Object {
                id: 77,
                parent: 0,
                has_model: true,
                object_flags: 0,
                collision_flags: 0,
                bounds: crate::volume_queries::Bounds {
                    min: [-100.; 3],
                    max: [100.; 3],
                },
            },
            response_flags: 0,
            triangles: vec![crate::player_mesh_contact::Triangle {
                vertices: [[-100., -100., 1.], [100., -100., 1.], [0., 100., 1.]],
                part: 3,
                ordinal: 4,
            }],
        }];
        let mut peers = Peers {
            support: 0,
            calls: 0,
        };
        let mut queries = 0;
        let result = factory.advance_classification(
            &mut s,
            owner,
            1,
            crate::kabuto_classification::Scene {
                owner: 17,
                objects: &objects,
                peers: &mut peers,
            },
            |_| {
                queries += 1;
                if success {
                    Ok(crate::physical_ground::Ground {
                        plane: [0., 0., 1., -0.],
                        height: -0.,
                        key: [0, 0],
                    })
                } else {
                    Err("late classification terrain failure")
                }
            },
        );
        assert!(peers.calls > 0);
        assert_eq!(peers.support, 17);
        assert!(queries > 0);
        assert_eq!(result.is_ok(), success);
        if success {
            assert_eq!(s.kabuto_group(owner).unwrap().contact().support, 0);
            assert!(s.actor_current_affine(owner).is_ok());
        } else {
            assert_eq!(snapshot(&s, owner), before);
            assert!(
                s.kabuto_group(owner)
                    .unwrap()
                    .geometry_transition_failure()
                    .is_some()
            );
        }
        assert!(classify(&factory, &mut s, owner, 1).is_err());
        assert!(s.begin_kabuto_motion_history(owner, 2, [0; 8]).is_err());
        s.clear();
        assert_eq!(s.counts().residents, 0);
    }
}
fn run_motion(with_classification: bool) {
    let (factory, _) = super::transition_tests::factories();
    let controls = std::env::var_os("GIANTS_KABUTO_MOTION_CONTROLS")
        .map(|p| std::fs::read_to_string(p).unwrap());
    let parse = |s: &str| {
        s.split_whitespace()
            .map(|v| u32::from_str_radix(v, 16).unwrap())
            .collect::<Vec<_>>()
    };
    let mut lines = controls.as_deref().unwrap_or("").lines();
    let config = lines.next().map(parse);
    let cold = config.map_or(
        ColdFrame {
            position: [0., 0., 10.],
            current: [0.; 3],
            previous: [0.; 3],
            angles: [0.; 3],
        },
        |w| {
            assert_eq!(w.len(), 12);
            ColdFrame {
                position: std::array::from_fn(|i| f32::from_bits(w[i])),
                current: std::array::from_fn(|i| f32::from_bits(w[3 + i])),
                previous: std::array::from_fn(|i| f32::from_bits(w[6 + i])),
                angles: std::array::from_fn(|i| f32::from_bits(w[9 + i])),
            }
        },
    );
    let mut s = Session::new(977);
    let owner = factory
        .construct(&mut s, cold, Blending::Disabled, 1)
        .unwrap();
    let baseline_fields = s.get(owner).unwrap().construction_fields.clone().unwrap();
    let baseline_payload = s.get(owner).unwrap().seed.storage.data().unwrap().to_vec();
    let lists = (0..15).map(|i| s.list(i)).collect::<Vec<_>>();
    let counts = s.counts();
    let mut receipt = String::new();
    if controls.is_some() {
        for line in lines {
            let w = parse(line);
            let n = w[5] as usize;
            assert_eq!(w.len(), 410 + 5 * n);
            let h = &w[6..199];
            let p = &w[199..410];
            let f = |at: usize| f32::from_bits(h[at / 4]);
            let actor = s.residents.get_mut(&owner.serial).unwrap();
            actor.affine_cache = None;
            let mut fields = baseline_fields.clone();
            fields.position = std::array::from_fn(|i| f(0xa0 + i * 4));
            fields.angles = std::array::from_fn(|i| f(0xc8 + i * 4));
            fields.object_flags = h[0xd8 / 4];
            fields.response_flags = h[0xdc / 4];
            for i in 5..8 {
                fields.configuration[i] = h[0x190 / 4 + i];
            }
            actor.construction_fields = Some(fields);
            let body = actor.seed.storage.data_mut().unwrap();
            body.copy_from_slice(&baseline_payload);
            for at in [0, 0x1d0] {
                body[at..at + 4].copy_from_slice(&p[at / 4].to_le_bytes());
            }
            for i in 0..15 {
                body[0xe4 + i * 4..0xe8 + i * 4].copy_from_slice(&p[0xe4 / 4 + i].to_le_bytes());
            }
            let physics = crate::kabuto_physics::Physics::from_owned_payload(body);
            let group = actor.kabuto_group.as_mut().unwrap();
            group.physics = Some(physics);
            group.motion_history = None;
            group.motion_prefix_completed = None;
            group.classification_completed = None;
            group.contact = Default::default();
            group.motion = State {
                commands: [0; 8],
                mode: h[0xe4 / 4],
                model: std::array::from_fn(|i| f(0x108 + i * 4)),
                previous_model: [f32::NAN; 4],
                look: f(0xd4),
                velocity: std::array::from_fn(|i| f(0x128 + i * 4)),
                previous_velocity: std::array::from_fn(|i| f(0x134 + i * 4)),
                target_heading: f(0x158),
            };
            let commands: [u8; 8] = [h[0x208 / 4].to_le_bytes(), h[0x20c / 4].to_le_bytes()]
                .concat()
                .try_into()
                .unwrap();
            assert!(
                s.begin_kabuto_motion_history(owner, u64::from(w[0]), commands)
                    .unwrap()
            );
            let areas: Vec<_> = w[410..]
                .as_chunks::<5>()
                .0
                .iter()
                .map(|a| crate::object_time::SlowArea {
                    radius_squared: f32::from_bits(a[0]),
                    factor: f32::from_bits(a[1]),
                    source_owner: a[2],
                    center: [f32::from_bits(a[3]), f32::from_bits(a[4]), 0.],
                })
                .collect();
            factory
                .advance_motion_prefix(
                    &mut s,
                    owner,
                    u64::from(w[0]),
                    Frame {
                        global_seconds: f32::from_bits(w[1]),
                        sensitivity: f32::from_bits(w[2]),
                        skip_heading: w[3] != 0,
                        local_owner: w[4] != 0,
                        parent_present: false,
                        areas: &areas,
                    },
                )
                .unwrap();
            if with_classification {
                // These supplied controls enter an unlocked object walk. Poison
                // only the Rust prior radius to prove the real resource writer
                // runs; retaining the initialized radius must not pass parity.
                let g = s
                    .residents
                    .get_mut(&owner.serial)
                    .unwrap()
                    .kabuto_group
                    .as_mut()
                    .unwrap();
                assert_eq!(g.physics.unwrap().words()[0] & 0x100, 0);
                g.radius = 123.;
                classify(&factory, &mut s, owner, u64::from(w[0])).unwrap();
            }
            receipt.push_str(
                &snapshot(&s, owner)
                    .iter()
                    .map(|v| format!("{v:08x}"))
                    .collect::<Vec<_>>()
                    .join(" "),
            );
            receipt.push('\n');
            assert_eq!(s.counts(), counts);
            assert_eq!((0..15).map(|i| s.list(i)).collect::<Vec<_>>(), lists);
            assert_eq!(s.kabuto_group(owner).unwrap().pending.len(), 4);
        }
    } else {
        let actor = s.residents.get_mut(&owner.serial).unwrap();
        let body = actor.seed.storage.data_mut().unwrap();
        body[0xfc..0x100].copy_from_slice(&300f32.to_le_bytes());
        actor.kabuto_group.as_mut().unwrap().physics =
            Some(crate::kabuto_physics::Physics::from_owned_payload(body));
        assert!(
            s.begin_kabuto_motion_history(owner, 1, [16, 8, 32, 0, 0, 0, 0, 0])
                .unwrap()
        );
        let before = snapshot(&s, owner);
        if with_classification {
            assert!(classify(&factory, &mut s, owner, 1).is_err());
        }
        let frame = |seconds| Frame {
            global_seconds: seconds,
            areas: &[],
            sensitivity: 1.,
            local_owner: true,
            skip_heading: false,
            parent_present: false,
        };
        let (foreign, _) = super::transition_tests::factories();
        assert!(
            foreign
                .advance_motion_prefix(&mut s, owner, 1, frame(0.1))
                .is_err()
        );
        assert!(
            factory
                .advance_motion_prefix(&mut s, owner, 2, frame(0.1))
                .is_err()
        );
        assert!(
            factory
                .advance_motion_prefix(&mut s, owner, 1, frame(f32::NAN))
                .is_err()
        );
        assert_eq!(snapshot(&s, owner), before);
        factory
            .advance_motion_prefix(&mut s, owner, 1, frame(0.1))
            .unwrap();
        assert!(
            s.get(owner)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap()
                .position[0]
                > 0.
        );
        assert_ne!(snapshot(&s, owner), before);
        if with_classification {
            let (foreign, _) = super::transition_tests::factories();
            let entered = snapshot(&s, owner);
            assert!(classify(&foreign, &mut s, owner, 1).is_err());
            assert!(classify(&factory, &mut s, owner, 2).is_err());
            assert_eq!(snapshot(&s, owner), entered);
            classify(&factory, &mut s, owner, 1).unwrap();
            assert!(s.actor_current_affine(owner).is_ok());
            assert_eq!(
                s.kabuto_group(owner).unwrap().classification_completed,
                Some(1)
            );
            assert!(classify(&factory, &mut s, owner, 1).is_err());
        }
        assert!(
            factory
                .advance_motion_prefix(&mut s, owner, 1, frame(0.1))
                .is_err()
        );
        assert!(s.begin_kabuto_motion_history(owner, 2, [0; 8]).is_err());
    }
    if let Some(path) = std::env::var_os("GIANTS_KABUTO_MOTION_RECEIPT") {
        std::fs::write(path, receipt).unwrap();
    }
    assert_eq!(s.kabuto_group(owner).unwrap().pending.len(), 4);
    assert!(s.kabuto_group(owner).unwrap().primary_started);
    assert!(s.begin_kabuto_motion_history(owner, 999, [0; 8]).is_err());
    s.clear();
    assert!(s.begin_kabuto_motion_history(owner, 1000, [0; 8]).is_err());
    assert_eq!(s.counts().residents, 0);
}
