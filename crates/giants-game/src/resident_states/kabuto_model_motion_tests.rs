use super::*;

struct Empty;
impl crate::kabuto_classification::Peers for Empty {
    fn profile(&mut self, _: u32) -> Result<crate::kabuto_contact::Peer, &'static str> {
        Err("empty prepared scene")
    }
    fn bind_support(&mut self, _: u32, _: u32) -> Result<(), &'static str> {
        Err("empty prepared scene")
    }
}
fn words_line(w: impl IntoIterator<Item = u32>) -> String {
    w.into_iter()
        .map(|v| format!("{v:08x}"))
        .collect::<Vec<_>>()
        .join(" ")
        + "\n"
}

#[test]
#[ignore = "Requires GIANTS_KABUTO_ROOT; optional private stage-input/receipt exports; no window or sound"]
fn source_kabuto_model_motion_reuses_owned_classified_state_and_preserves_primary() {
    let (factory, _) = super::transition_tests::factories();
    let (foreign, _) = super::transition_tests::factories();
    let mut s = Session::new(991);
    let id = factory
        .construct(
            &mut s,
            ColdFrame {
                position: [0., 0., 10.],
                current: [0.; 3],
                previous: [0.; 3],
                angles: [0.; 3],
            },
            Blending::Disabled,
            1,
        )
        .unwrap();
    let baseline_fields = s.get(id).unwrap().construction_fields.clone().unwrap();
    let baseline_payload = s.get(id).unwrap().seed.storage.data().unwrap().to_vec();
    let radius = s.kabuto_group(id).unwrap().radius;
    let root = std::path::PathBuf::from(std::env::var("GIANTS_KABUTO_ROOT").unwrap());
    let globals =
        giants_formats::global_data::parse(&std::fs::read(root.join("Bin/GData.gbn")).unwrap())
            .unwrap();
    let mut inputs = String::new();
    let mut receipt = String::new();
    let mut moving = 0;
    for case in 0..512 {
        // Independent contexts replay the real preceding phases; resetting this
        // fixture is not acknowledgment of a completed native frame.
        let a = s.residents.get_mut(&id.serial).unwrap();
        a.construction_fields = Some(baseline_fields.clone());
        let f = a.construction_fields.as_mut().unwrap();
        f.position = [
            case as f32 * 0.125 - 32.,
            (case % 17) as f32 - 8.,
            10. + (case % 5) as f32,
        ];
        f.current = f.position;
        f.angles[0] = (case % 27) as f32 * 13.;
        a.affine_cache = None;
        a.seed
            .storage
            .data_mut()
            .unwrap()
            .copy_from_slice(&baseline_payload);
        let p = a.seed.storage.data_mut().unwrap();
        for (i, v) in [
            case as f32 - 200.,
            100. - case as f32,
            (case % 13) as f32 - 6.,
        ]
        .into_iter()
        .enumerate()
        {
            p[0xfc + i * 4..0x100 + i * 4].copy_from_slice(&v.to_bits().to_le_bytes());
        }
        let physics = crate::kabuto_physics::Physics::from_owned_payload(p);
        let g = a.kabuto_group.as_mut().unwrap();
        g.physics = Some(physics);
        g.radius = radius;
        g.motion = Default::default();
        g.motion.velocity = [case as f32 * 0.01, -2., (case % 3) as f32 - 1.];
        g.contact = Default::default();
        g.motion_history = None;
        g.motion_prefix_completed = None;
        g.classification_completed = None;
        g.object_contacts_entered = None;
        g.object_contacts_completed = None;
        g.model_motion_completed = None;
        let number = case as u64 + 1;
        let seconds = [0., 0.001, 1. / 60., 0.1][case % 4];
        let command = [0u8, 1, 32, 127, 255, 224, 128];
        let commands = [0, 0, command[case % 7], command[(case / 7) % 7], 0, 0, 0, 0];
        s.begin_kabuto_motion_history(id, number, commands).unwrap();
        factory
            .advance_motion_prefix(
                &mut s,
                id,
                number,
                crate::kabuto_motion_prefix::Frame {
                    global_seconds: seconds,
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
                &mut s,
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
        let context = crate::kabuto_geometry_mode::NotificationContext {
            session_mode: 0,
            peer_ids: &[],
            local_peer: 0,
            owner_word: 0,
        };
        assert!(
            factory
                .advance_model_motion(&mut s, id, number, seconds, &[])
                .is_err()
        );
        factory
            .advance_object_contacts(
                &mut s,
                id,
                number,
                crate::kabuto_object_contacts::Scene {
                    owner: 1,
                    disabled: false,
                    peers: &mut [],
                    context: &context,
                },
                |_, _, _| panic!("empty scene action"),
                |_| {},
            )
            .unwrap();
        let position = s
            .get(id)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap()
            .position;
        let owner_context = s
            .get(id)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap()
            .dispatch_context;
        let areas = [
            crate::object_time::SlowArea {
                source_owner: owner_context,
                center: position,
                radius_squared: 100.,
                factor: 0.,
            },
            crate::object_time::SlowArea {
                source_owner: owner_context.wrapping_add(1),
                center: position,
                radius_squared: 1.,
                factor: [0., 0.1, 0.7, 2.][case % 4],
            },
        ];
        let before = super::motion_tests::snapshot(&s, id);
        let counts = s.counts();
        let lists = (0..15).map(|i| s.list(i)).collect::<Vec<_>>();
        assert!(
            foreign
                .advance_model_motion(&mut s, id, number, seconds, &areas)
                .is_err()
        );
        assert!(
            factory
                .advance_model_motion(&mut s, id, number + 1, seconds, &areas)
                .is_err()
        );
        assert!(
            factory
                .advance_model_motion(&mut s, id, number, f32::NAN, &areas)
                .is_err()
        );
        assert_eq!(before, super::motion_tests::snapshot(&s, id));
        let cache = s.residents.get_mut(&id.serial).unwrap().affine_cache.take();
        assert!(
            factory
                .advance_model_motion(&mut s, id, number, seconds, &areas)
                .is_err()
        );
        s.residents.get_mut(&id.serial).unwrap().affine_cache = cache;
        let g = s.kabuto_group(id).unwrap();
        let fourth = g.motion.model[3].to_bits();
        let mut input = vec![1];
        input.extend(&before[..193]);
        input.extend(g.physics.unwrap().words());
        input.extend([
            seconds.to_bits(),
            globals.words[0x6e0 / 4],
            globals.words[0x6e4 / 4],
            areas.len() as u32,
        ]);
        for a in areas {
            input.extend([
                a.source_owner,
                a.center[0].to_bits(),
                a.center[1].to_bits(),
                a.center[2].to_bits(),
                a.radius_squared.to_bits(),
                a.factor.to_bits(),
            ]);
        }
        // Export the actual stage-entry state BEFORE running the Rust consumer.
        inputs.push_str(&words_line(input));
        let out = factory
            .advance_model_motion(&mut s, id, number, seconds, &areas)
            .unwrap();
        receipt.push_str(&words_line(
            out.model
                .map(f32::to_bits)
                .into_iter()
                .chain([out.world_speed.to_bits()]),
        ));
        let after = super::motion_tests::snapshot(&s, id);
        assert!(before.iter().zip(&after).enumerate().all(|(i, (a, b))| {
            [0x108 / 4, 0x10c / 4, 0x110 / 4, 193 + (0xe4 + 9 * 4) / 4].contains(&i) || a == b
        }));
        let g = s.kabuto_group(id).unwrap();
        assert_eq!(g.motion.model[3].to_bits(), fourth);
        assert_eq!(g.model_motion_completed, Some(number));
        assert_eq!(g.motion_history, Some(number));
        assert_eq!(g.object_contacts_completed, Some(number));
        assert_eq!(g.pending.len(), 4);
        assert_eq!(s.counts(), counts);
        assert_eq!((0..15).map(|i| s.list(i)).collect::<Vec<_>>(), lists);
        assert!(
            factory
                .advance_model_motion(&mut s, id, number, seconds, &areas)
                .is_err()
        );
        assert!(
            s.begin_kabuto_motion_history(id, number + 1, commands)
                .is_err()
        );
        if out.world_speed > 0. {
            moving += 1;
        }
    }
    assert!(moving > 300);
    if let Some(p) = std::env::var_os("GIANTS_KABUTO_MODEL_INPUTS") {
        std::fs::write(p, inputs).unwrap();
    }
    if let Some(p) = std::env::var_os("GIANTS_KABUTO_MODEL_RECEIPT") {
        std::fs::write(p, receipt).unwrap();
    }
}
