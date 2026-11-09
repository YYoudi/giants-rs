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
    run_model(false, false);
}
#[test]
#[ignore = "Requires GIANTS_KABUTO_ROOT; optional private ANM/gravity inputs and receipts; no window or sound"]
fn source_kabuto_animation_gravity_uses_model_phase_and_retains_primary() {
    run_model(true, false);
}
#[test]
#[ignore = "Requires GIANTS_KABUTO_ROOT; optional private post-gravity control inputs/receipts; no window or sound"]
fn source_kabuto_control_force_uses_owned_animated_state_and_preserves_primary() {
    run_model(true, true);
}
fn run_model(animation: bool, control: bool) {
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
    let baseline_references = s.specifications.get(&id.serial).unwrap().0.clone();
    let mut contexts = crate::attached_effect_context::Pool::new(991, None);
    let mut random = crate::audio_parameters::Random(1);
    let mut stamp = crate::animation_epoch::Stamp(0);
    let mut animation_inputs = String::new();
    let mut animation_receipt = String::new();
    let mut control_inputs = String::new();
    let mut control_receipt = String::new();
    let mut targeted = 0;
    let radius = s.kabuto_group(id).unwrap().radius;
    let root = std::path::PathBuf::from(std::env::var("GIANTS_KABUTO_ROOT").unwrap());
    let globals =
        giants_formats::global_data::parse(&std::fs::read(root.join("Bin/GData.gbn")).unwrap())
            .unwrap();
    let mut inputs = String::new();
    let mut receipt = String::new();
    let mut moving = 0;
    for case in 0..if animation { 513 } else { 512 } {
        // Independent contexts replay the real preceding phases; resetting this
        // fixture is not acknowledgment of a completed native frame.
        s.specifications.get_mut(&id.serial).unwrap().0 = baseline_references.clone();
        let a = s.residents.get_mut(&id.serial).unwrap();
        a.construction_fields = Some(baseline_fields.clone());
        a.channels = Some(baseline_fields.channels);
        a.mecc_animation_stamp = 0;
        let f = a.construction_fields.as_mut().unwrap();
        f.position = [
            case as f32 * 0.125 - 32.,
            (case % 17) as f32 - 8.,
            10. + (case % 5) as f32,
        ];
        f.current = f.position;
        if control {
            f.position[2] = [-100., -10., -1., 1., 50., 100.][case % 6];
            f.current = f.position;
            if case % 7 == 0 {
                f.object_flags |= 0x10;
            }
            if case % 3 == 0 {
                f.object_flags |= 0x8000000;
            }
        }
        f.angles[0] = (case % 27) as f32 * 13.;
        a.affine_cache = None;
        a.seed
            .storage
            .data_mut()
            .unwrap()
            .copy_from_slice(&baseline_payload);
        let p = a.seed.storage.data_mut().unwrap();
        if control {
            // The preceding speed latch is a supplied input. Its later native
            // producer is not implemented by this post-gravity consumer.
            let at = [0x160, 0x164, 0x168, 0x16c][case % 4];
            let incoming = p[at..at + 4].to_vec();
            p[0x160..0x164].copy_from_slice(&incoming);
        }
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
        g.animation_gravity_entered = None;
        g.animation_gravity_completed = None;
        g.control_force_completed = None;
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
        let floor = if control && case % 5 == 0 {
            -200f32
        } else {
            0f32
        };
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
                        plane: [0., 0., 1., -floor],
                        height: -((f64::from(xy[0]) * 0. + f64::from(xy[1]) * 0.)
                            - f64::from(floor)) as f32,
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
        if animation {
            if case == 255 {
                stamp.0 = u32::MAX;
            }
            if case == 512 {
                // A separate real-resource context crosses an actual required
                // contact event. Missing services must retain ANM effects and
                // prevent gravity, not silently discard the event.
                let spec = s.specification(id).unwrap();
                let (slot, time) = spec
                    .metadata
                    .events
                    .iter()
                    .find_map(|t| {
                        let duration = spec.animations.get(&t.slot)?.template.duration;
                        t.events
                            .iter()
                            .find(|e| {
                                e.kind & 0x10008 != 0 && e.time > 0.02 && e.time < duration - 0.02
                            })
                            .map(|e| (t.slot, e.time))
                    })
                    .expect("Kabuto declared contact event");
                let a = s.residents.get_mut(&id.serial).unwrap();
                let mut channels = a.channels.unwrap();
                crate::animation_channels::select_primary(
                    &mut channels,
                    crate::animation_channels::Request {
                        slot,
                        flags: 1,
                        speed_source: 0,
                        reference_speed: 0.,
                        blend_seconds: 0.,
                    },
                    Blending::Disabled,
                )
                .unwrap();
                channels[0][1] = (time - 0.01).to_bits();
                channels[0][2] = channels[0][1];
                a.channels = Some(channels);
                let f = a.construction_fields.as_mut().unwrap();
                f.channels = channels;
                f.object_flags |= 0x100000;
                let payload = a.seed.storage.data().unwrap().to_vec();
                let epoch = stamp.0;
                let mut services = crate::actor_animation_phase::Services {
                    contexts: &mut contexts,
                    random: &mut random,
                    stamp: &mut stamp,
                    blending: Blending::Disabled,
                };
                let error = factory
                    .advance_animation_gravity(
                        &mut s,
                        id,
                        number,
                        crate::kabuto_animation_gravity::Frame {
                            global_seconds: 0.02,
                            areas: &[],
                            materials: None,
                        },
                        &mut services,
                    )
                    .unwrap_err();
                assert!(error.contains("unsupported service"), "{error}");
                assert_ne!(services.stamp.0, epoch);
                assert_eq!(s.get(id).unwrap().seed.storage.data().unwrap(), payload);
                assert_ne!(s.get(id).unwrap().channels.unwrap(), channels);
                assert!(s.reference_cache_storage(id).unwrap().sampled);
                let g = s.kabuto_group(id).unwrap();
                assert_eq!(g.animation_gravity_entered, Some(number));
                assert_eq!(g.animation_gravity_completed, None);
                assert!(g.geometry_transition_failure.is_some());
                assert_eq!(g.model_motion_completed, Some(number));
                assert_eq!(g.motion_history, Some(number));
                assert_eq!(g.pending.len(), 4);
                assert!(
                    factory
                        .advance_animation_gravity(
                            &mut s,
                            id,
                            number,
                            crate::kabuto_animation_gravity::Frame {
                                global_seconds: 0.02,
                                areas: &[],
                                materials: None,
                            },
                            &mut services
                        )
                        .is_err()
                );
                assert!(factory.advance_control_force(&mut s, id, number).is_err());
                continue;
            }
            let before = super::motion_tests::snapshot(&s, id);
            let refs = s.reference_cache_storage(id).unwrap();
            let refs_words: Vec<_> = refs
                .matrices
                .iter()
                .flat_map(|m| std::iter::once(m.flags).chain(m.values.map(f32::to_bits)))
                .collect();
            let mut input = vec![seconds.to_bits(), stamp.0, random.0, areas.len() as u32];
            input.extend(&before);
            input.push(refs.matrices.len() as u32);
            input.extend(refs_words);
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
            animation_inputs.push_str(&words_line(input));
            let mut services = crate::actor_animation_phase::Services {
                contexts: &mut contexts,
                random: &mut random,
                stamp: &mut stamp,
                blending: Blending::Disabled,
            };
            if control {
                assert!(factory.advance_control_force(&mut s, id, number).is_err());
            }
            assert!(
                foreign
                    .advance_animation_gravity(
                        &mut s,
                        id,
                        number,
                        crate::kabuto_animation_gravity::Frame {
                            global_seconds: seconds,
                            areas: &areas,
                            materials: None
                        },
                        &mut services
                    )
                    .is_err()
            );
            assert!(
                factory
                    .advance_animation_gravity(
                        &mut s,
                        id,
                        number + 1,
                        crate::kabuto_animation_gravity::Frame {
                            global_seconds: seconds,
                            areas: &areas,
                            materials: None
                        },
                        &mut services
                    )
                    .is_err()
            );
            assert!(
                factory
                    .advance_animation_gravity(
                        &mut s,
                        id,
                        number,
                        crate::kabuto_animation_gravity::Frame {
                            global_seconds: f32::NAN,
                            areas: &areas,
                            materials: None,
                        },
                        &mut services
                    )
                    .is_err()
            );
            assert_eq!(before, super::motion_tests::snapshot(&s, id));
            let view = factory
                .advance_animation_gravity(
                    &mut s,
                    id,
                    number,
                    crate::kabuto_animation_gravity::Frame {
                        global_seconds: seconds,
                        areas: &areas,
                        materials: None,
                    },
                    &mut services,
                )
                .unwrap();
            let after = super::motion_tests::snapshot(&s, id);
            assert!(before.iter().zip(&after).enumerate().all(|(i, (a, b))| {
                i == 3
                    || (4..40).contains(&i)
                    || [6, 7, 8, 11]
                        .into_iter()
                        .any(|n| i == 193 + (0xe4 + n * 4) / 4)
                    || a == b
            }));
            let mut output = after;
            output.extend([services.stamp.0, services.random.0]);
            output.extend(
                s.reference_cache_storage(id)
                    .unwrap()
                    .matrices
                    .iter()
                    .flat_map(|m| std::iter::once(m.flags).chain(m.values.map(f32::to_bits))),
            );
            animation_receipt.push_str(&words_line(output));
            let g = s.kabuto_group(id).unwrap();
            assert_eq!(g.animation_gravity_entered, Some(number));
            assert_eq!(g.animation_gravity_completed, Some(number));
            assert_eq!(g.model_motion_completed, Some(number));
            assert_eq!(g.motion_history, Some(number));
            assert_eq!(g.pending.len(), 4);
            assert_eq!(view.stamp, services.stamp.0);
            assert_eq!(s.counts(), counts);
            assert_eq!((0..15).map(|i| s.list(i)).collect::<Vec<_>>(), lists);
            assert!(
                factory
                    .advance_animation_gravity(
                        &mut s,
                        id,
                        number,
                        crate::kabuto_animation_gravity::Frame {
                            global_seconds: seconds,
                            areas: &areas,
                            materials: None
                        },
                        &mut services
                    )
                    .is_err()
            );
            assert!(
                s.begin_kabuto_motion_history(id, number + 1, commands)
                    .is_err()
            );
            if control {
                let before = super::motion_tests::snapshot(&s, id);
                let header = &before[..193];
                let input = [globals.words[0x5fc / 4], globals.words[0x600 / 4]]
                    .into_iter()
                    .chain(before.iter().copied());
                control_inputs.push_str(&words_line(input));
                assert!(foreign.advance_control_force(&mut s, id, number).is_err());
                assert!(
                    factory
                        .advance_control_force(&mut s, id, number + 1)
                        .is_err()
                );
                let cached = s.residents.get_mut(&id.serial).unwrap().affine_cache.take();
                assert!(factory.advance_control_force(&mut s, id, number).is_err());
                s.residents.get_mut(&id.serial).unwrap().affine_cache = cached;
                assert_eq!(before, super::motion_tests::snapshot(&s, id));
                let view = factory.advance_control_force(&mut s, id, number).unwrap();
                let after = super::motion_tests::snapshot(&s, id);
                assert!(before.iter().zip(&after).enumerate().all(|(i, (a, b))| {
                    [6, 7, 8, 11]
                        .into_iter()
                        .any(|n| i == 193 + (0xe4 + n * 4) / 4)
                        || a == b
                }));
                let mut output = after;
                output.extend([
                    view.speed.to_bits(),
                    view.speed.to_bits(),
                    globals.words[0x5fc / 4],
                    globals.words[0x600 / 4],
                    header[0xd8 / 4] & 0x10,
                ]);
                output.push(if view.desired_velocity.is_some() {
                    3
                } else {
                    0
                });
                if let Some(desired) = view.desired_velocity {
                    output.extend(desired.map(f32::to_bits));
                    targeted += 1;
                }
                control_receipt.push_str(&words_line(output));
                let g = s.kabuto_group(id).unwrap();
                assert_eq!(g.control_force_completed, Some(number));
                assert_eq!(g.animation_gravity_completed, Some(number));
                assert_eq!(g.motion_history, Some(number));
                assert_eq!(g.pending.len(), 4);
                assert_eq!(s.counts(), counts);
                assert_eq!((0..15).map(|i| s.list(i)).collect::<Vec<_>>(), lists);
                assert!(factory.advance_control_force(&mut s, id, number).is_err());
                assert!(
                    s.begin_kabuto_motion_history(id, number + 1, commands)
                        .is_err()
                );
            }
        }
    }
    assert!(moving > 300);
    if let Some(p) = std::env::var_os("GIANTS_KABUTO_MODEL_INPUTS") {
        std::fs::write(p, inputs).unwrap();
    }
    if let Some(p) = std::env::var_os("GIANTS_KABUTO_MODEL_RECEIPT") {
        std::fs::write(p, receipt).unwrap();
    }
    if animation {
        if let Some(p) = std::env::var_os("GIANTS_KABUTO_ANIMATION_INPUTS") {
            std::fs::write(p, animation_inputs).unwrap();
        }
        if let Some(p) = std::env::var_os("GIANTS_KABUTO_ANIMATION_RECEIPT") {
            std::fs::write(p, animation_receipt).unwrap();
        }
    }
    if control {
        assert!(targeted > 50 && targeted < 500, "{targeted}");
        if let Some(p) = std::env::var_os("GIANTS_KABUTO_CONTROL_INPUTS") {
            std::fs::write(p, control_inputs).unwrap();
        }
        if let Some(p) = std::env::var_os("GIANTS_KABUTO_CONTROL_RECEIPT") {
            std::fs::write(p, control_receipt).unwrap();
        }
    }
}
