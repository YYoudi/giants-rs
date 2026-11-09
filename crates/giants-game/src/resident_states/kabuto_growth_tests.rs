use super::*;

pub(super) fn header(session: &Session, id: Id) -> Vec<u32> {
    let actor = session.get(id).unwrap();
    let f = actor.construction_fields.as_ref().unwrap();
    let mut words = vec![0; 193];
    words[0x190 / 4..0x208 / 4].copy_from_slice(&f.configuration);
    for (offset, value) in [
        (4, f.class as u32),
        (0xc4, f.scale.to_bits()),
        (0xd8, f.object_flags),
        (0xdc, f.response_flags),
        (0xe0, f.control_flags),
        (0xe8, f.action),
        (0x18c, f.dispatch_context),
        (
            0x2d8,
            session
                .specification(id)
                .unwrap()
                .metadata
                .references
                .names
                .len() as u32,
        ),
        (0x2e4, f.shadow_reciprocal.to_bits()),
        (0xc, actor.mecc_animation_stamp),
    ] {
        words[offset / 4] = value;
    }
    words[4..40].copy_from_slice(
        &actor
            .channels
            .unwrap()
            .into_iter()
            .flatten()
            .collect::<Vec<_>>(),
    );
    for (offset, values) in [
        (0xa0, f.position),
        (0xac, f.current),
        (0xb8, f.previous),
        (0xc8, f.angles),
    ] {
        for (i, value) in values.into_iter().enumerate() {
            words[offset / 4 + i] = value.to_bits();
        }
    }
    if let Some(group) = session.kabuto_group(id) {
        words[0x104 / 4] = u32::from(group.physics.is_some());
        words[0x154 / 4] = group.radius.to_bits();
    }
    if let Some(a) = actor.attachment {
        words[0x210 / 4] = 1;
        words[0x214 / 4] = a.child_reference;
        words[0x218 / 4] = a.parent_reference;
    }
    words
}

#[test]
#[ignore = "Requires GIANTS_KABUTO_ROOT; optional private growth controls/receipt; no window or sound"]
fn source_kabuto_growth_composes_size_and_auxiliary_without_admitting_gameplay() {
    let (factory, _) = super::transition_tests::factories();
    let decision_controls = std::env::var_os("GIANTS_KABUTO_GROWTH_DECISION_CONTROLS");
    let with_decision = decision_controls.is_some();
    let controls = decision_controls
        .or_else(|| std::env::var_os("GIANTS_KABUTO_GROWTH_CONTROLS"))
        .map(|path| std::fs::read_to_string(path).unwrap());
    let parse = |line: &str| {
        line.split_whitespace()
            .map(|v| u32::from_str_radix(v, 16).unwrap())
            .collect::<Vec<_>>()
    };
    let mut lines = controls.as_deref().unwrap_or("").lines();
    let config = lines.next().map(parse);
    let cold = config.as_ref().map_or(
        ColdFrame {
            position: [0., 0., 10.],
            current: [0.; 3],
            previous: [0.; 3],
            angles: [0.; 3],
        },
        |w| {
            assert_eq!(w.len(), 13);
            ColdFrame {
                position: std::array::from_fn(|i| f32::from_bits(w[1 + i])),
                current: std::array::from_fn(|i| f32::from_bits(w[4 + i])),
                previous: std::array::from_fn(|i| f32::from_bits(w[7 + i])),
                angles: std::array::from_fn(|i| f32::from_bits(w[10 + i])),
            }
        },
    );
    let sequential = config.as_ref().is_some_and(|w| w[0] != 0);
    let mut session = Session::new(953);
    let owner = factory
        .construct(&mut session, cold, Blending::Disabled, 1)
        .unwrap();
    let group = session.kabuto_group(owner).unwrap();
    let first = group.first;
    let second = group.second;
    let initial_radius = group.radius;
    let initial_fields = session
        .get(owner)
        .unwrap()
        .construction_fields
        .clone()
        .unwrap();
    let initial_payload = session
        .get(owner)
        .unwrap()
        .seed
        .storage
        .data()
        .unwrap()
        .to_vec();
    let child_fields = session
        .get(second)
        .unwrap()
        .construction_fields
        .clone()
        .unwrap();
    let lists = (0..15).map(|i| session.list(i)).collect::<Vec<_>>();
    let counts = session.counts();
    let mut receipt = String::new();
    if controls.is_some() {
        for line in lines {
            let w = parse(line);
            let area_count = w[14] as usize;
            assert_eq!(w.len(), 15 + 5 * area_count + usize::from(with_decision));
            if !sequential {
                let actor = session.residents.get_mut(&owner.serial).unwrap();
                actor.construction_fields = Some(initial_fields.clone());
                actor
                    .seed
                    .storage
                    .data_mut()
                    .unwrap()
                    .copy_from_slice(&initial_payload);
                actor.kabuto_group.as_mut().unwrap().radius = initial_radius;
                actor.seed.storage.data_mut().unwrap()[0x15c..0x160]
                    .copy_from_slice(&w[0].to_le_bytes());
                let child = session.residents.get_mut(&second.serial).unwrap();
                child.construction_fields = Some(child_fields.clone());
                child.channels = Some(child_fields.channels);
            } else {
                assert_eq!(
                    session.get(owner).unwrap().seed.storage.data().unwrap()[0x15c..0x160],
                    w[0].to_le_bytes()
                );
            }
            let fields = session
                .residents
                .get_mut(&owner.serial)
                .unwrap()
                .construction_fields
                .as_mut()
                .unwrap();
            fields.object_flags = w[3];
            fields.position = std::array::from_fn(|i| f32::from_bits(w[4 + i]));
            for (id, start) in [(second, 7), (first, 10)] {
                session
                    .residents
                    .get_mut(&id.serial)
                    .unwrap()
                    .construction_fields
                    .as_mut()
                    .unwrap()
                    .position = std::array::from_fn(|i| f32::from_bits(w[start + i]));
            }
            let areas = w[15..15 + 5 * area_count]
                .as_chunks::<5>()
                .0
                .iter()
                .map(|a| crate::object_time_precision::Area {
                    radius_squared: f32::from_bits(a[0]),
                    multiplier: f32::from_bits(a[1]),
                    owner_id: a[2],
                    center: [f32::from_bits(a[3]), f32::from_bits(a[4])],
                })
                .collect::<Vec<_>>();
            let mut target = f32::from_bits(w[1]);
            let mut decision_events = Vec::new();
            if with_decision {
                let actor = session.residents.get_mut(&owner.serial).unwrap();
                let payload = actor.seed.storage.data_mut().unwrap();
                payload[0x94..0x98].copy_from_slice(&w[w.len() - 1].to_le_bytes());
                payload[0x158..0x15c].copy_from_slice(&w[1].to_le_bytes());
                factory
                    .decide_growth(
                        &mut session,
                        owner,
                        &crate::kabuto_geometry_mode::NotificationContext {
                            session_mode: 0,
                            peer_ids: &[],
                            local_peer: 0,
                            owner_word: 0,
                        },
                        |event, state| decision_events.push((event, state)),
                    )
                    .unwrap();
                target = f32::from_le_bytes(
                    session.get(owner).unwrap().seed.storage.data().unwrap()[0x158..0x15c]
                        .try_into()
                        .unwrap(),
                );
            }
            let view = factory
                .advance_growth(
                    &mut session,
                    owner,
                    crate::kabuto_growth::Frame {
                        target,
                        global_seconds: f32::from_bits(w[2]),
                        areas: &areas,
                    },
                )
                .unwrap();
            session
                .select_kabuto_auxiliary_animation(owner, if w[13] != 0 { owner } else { first })
                .unwrap();
            let group = session.kabuto_group(owner).unwrap();
            let mut output = header(&session, owner);
            let mut payload = session
                .get(owner)
                .unwrap()
                .seed
                .storage
                .data()
                .unwrap()
                .as_chunks::<4>()
                .0
                .iter()
                .map(|v| u32::from_le_bytes(*v))
                .collect::<Vec<_>>();
            payload[0x150 / 4] = 3;
            payload[0x154 / 4] = 1;
            payload[0x220 / 4] = 1;
            for i in 0..8 {
                payload[(0x22c + i * 28) / 4] = 4 + i as u32;
            }
            payload[0x314 / 4] = 12;
            output.extend(payload);
            output.extend(group.physics.unwrap().words());
            output.extend(header(&session, second));
            output.push(u32::from(view.refreshed));
            if with_decision {
                output.push(decision_events.len() as u32);
                for (event, state) in decision_events {
                    match event {
                        crate::kabuto_growth::DecisionEvent::Sound(id) => output.extend([0, id]),
                        _ => panic!("single-player decision unexpectedly notified"),
                    }
                    output.extend([state.trigger.to_bits(), state.target.to_bits()]);
                }
            }
            receipt.push_str(
                &output
                    .iter()
                    .map(|v| format!("{v:08x}"))
                    .collect::<Vec<_>>()
                    .join(" "),
            );
            receipt.push('\n');
            assert_eq!(session.counts(), counts);
            assert_eq!((0..15).map(|i| session.list(i)).collect::<Vec<_>>(), lists);
            assert_eq!(group.pending.len(), 4);
        }
    } else {
        for target in [3., 1.] {
            for _ in 0..20 {
                let view = factory
                    .advance_growth(
                        &mut session,
                        owner,
                        crate::kabuto_growth::Frame {
                            target,
                            global_seconds: 0.5,
                            areas: &[],
                        },
                    )
                    .unwrap();
                assert!(view.current >= 0. && view.current <= 3.);
                session
                    .select_kabuto_auxiliary_animation(owner, owner)
                    .unwrap();
            }
            assert_eq!(
                f32::from_le_bytes(
                    session.get(owner).unwrap().seed.storage.data().unwrap()[0x15c..0x160]
                        .try_into()
                        .unwrap()
                ),
                target
            );
        }
    }
    if controls.is_none() {
        let context = crate::kabuto_geometry_mode::NotificationContext {
            session_mode: 0,
            peer_ids: &[],
            local_peer: 0,
            owner_word: 0,
        };
        let actor = session.residents.get_mut(&owner.serial).unwrap();
        actor.seed.storage.data_mut().unwrap()[0x94..0x98].copy_from_slice(&100f32.to_le_bytes());
        let before_current = actor.seed.storage.data().unwrap()[0x15c..0x160].to_vec();
        let mut events = Vec::new();
        assert!(
            factory
                .decide_growth(&mut session, owner, &context, |e, s| events.push((e, s)))
                .unwrap()
        );
        let payload = session.get(owner).unwrap().seed.storage.data().unwrap();
        assert_eq!(&payload[0x15c..0x160], before_current);
        assert_eq!(
            f32::from_le_bytes(payload[0x94..0x98].try_into().unwrap()),
            0.
        );
        assert_eq!(
            f32::from_le_bytes(payload[0x158..0x15c].try_into().unwrap()),
            2.
        );
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].0,
            crate::kabuto_growth::DecisionEvent::Sound(0x6e4)
        );
        let (foreign, _) = super::transition_tests::factories();
        assert!(
            foreign
                .decide_growth(&mut session, owner, &context, |_, _| panic!(
                    "foreign decision emitted"
                ))
                .is_err()
        );
        let before = session
            .get(owner)
            .unwrap()
            .seed
            .storage
            .data()
            .unwrap()
            .to_vec();
        let actor = session.residents.get_mut(&owner.serial).unwrap();
        actor.seed.storage.data_mut().unwrap()[0x94..0x98].copy_from_slice(&100f32.to_le_bytes());
        actor.seed.storage.data_mut().unwrap()[0x158..0x15c]
            .copy_from_slice(&f32::NAN.to_le_bytes());
        let invalid = actor.seed.storage.data().unwrap().to_vec();
        assert!(
            factory
                .decide_growth(&mut session, owner, &context, |_, _| panic!(
                    "invalid decision emitted"
                ))
                .is_err()
        );
        assert_eq!(
            session.get(owner).unwrap().seed.storage.data().unwrap(),
            invalid
        );
        session
            .residents
            .get_mut(&owner.serial)
            .unwrap()
            .seed
            .storage
            .data_mut()
            .unwrap()
            .copy_from_slice(&before);
    }
    if let Some(path) = std::env::var_os("GIANTS_KABUTO_GROWTH_RECEIPT") {
        std::fs::write(path, receipt).unwrap();
    }
    let before = header(&session, owner);
    let payload = session
        .get(owner)
        .unwrap()
        .seed
        .storage
        .data()
        .unwrap()
        .to_vec();
    let current = f32::from_le_bytes(payload[0x15c..0x160].try_into().unwrap());
    assert!(
        factory
            .advance_growth(
                &mut session,
                owner,
                crate::kabuto_growth::Frame {
                    target: current + 1.,
                    global_seconds: f32::NAN,
                    areas: &[],
                }
            )
            .is_err()
    );
    assert_eq!(header(&session, owner), before);
    assert_eq!(
        session.get(owner).unwrap().seed.storage.data().unwrap(),
        payload
    );
    let (foreign_factory, _) = super::transition_tests::factories();
    assert!(
        foreign_factory
            .advance_growth(
                &mut session,
                owner,
                crate::kabuto_growth::Frame {
                    target: current,
                    global_seconds: 1.,
                    areas: &[],
                }
            )
            .is_err()
    );
    assert_eq!(header(&session, owner), before);
    assert_eq!(session.kabuto_group(owner).unwrap().pending.len(), 4);
    assert!(session.kabuto_group(owner).unwrap().primary_started);
    assert_eq!(session.counts(), counts);
    session.clear();
    assert!(
        factory
            .advance_growth(
                &mut session,
                owner,
                crate::kabuto_growth::Frame {
                    target: 1.,
                    global_seconds: 1.,
                    areas: &[],
                }
            )
            .is_err()
    );
    assert_eq!(session.counts().residents, 0);
    assert!(
        factory
            .decide_growth(
                &mut session,
                owner,
                &crate::kabuto_geometry_mode::NotificationContext {
                    session_mode: 0,
                    peer_ids: &[],
                    local_peer: 0,
                    owner_word: 0,
                },
                |_, _| panic!("cleared owner emitted"),
            )
            .is_err()
    );
}
