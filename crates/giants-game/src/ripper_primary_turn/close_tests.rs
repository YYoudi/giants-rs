// Supplied controls below are not ordinary world, motion or sound providers.
#[test]
#[ignore = "Requires GIANTS_RIPPER_CLOSE_SECONDARY_CONTROLS; private original secondary after mode67, first two channels with disabled blending"]
fn native_variant_close_secondary_controls() {
    struct NoFire;
    impl crate::state23_animation::Services for NoFire {
        fn fire(
            &mut self,
            _: &crate::state23_animation::State,
            _: [f32; 3],
            _: f32,
            _: &mut Random,
        ) -> Result<crate::state23_animation::FireOutcome, String> {
            Err("unexpected close selection projectile request".into())
        }
    }
    let path = std::env::var("GIANTS_RIPPER_CLOSE_SECONDARY_CONTROLS")
        .expect("private secondary controls path");
    let contents = std::fs::read_to_string(path).unwrap();
    let mut count = 0;
    for line in contents
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
    {
        let w: Vec<u32> = line
            .split_whitespace()
            .map(|s| u32::from_str_radix(s, 16).unwrap())
            .collect();
        assert_eq!(w.len(), 41);
        let mut channels = [[0; 9]; 4];
        channels[0].copy_from_slice(&w[5..14]);
        channels[1].copy_from_slice(&w[14..23]);
        let mut random = Random(w[2]);
        let next = crate::state23_animation::update(
            crate::state23_animation::State {
                class: 230,
                mode: w[0],
                timer: f32::from_bits(w[1]),
                counter: 0,
                forward_speed: 0.,
                channels,
            },
            crate::state23_animation::Context {
                parent_state: None,
                target_position: None,
                leap_reference_speed: f32::from_bits(w[4]),
                leap_model_speed: 0.,
                vertical_spread_percent: 0.,
                cooldown: [0.; 2],
                repeat_timer: 0.,
                blending: crate::animation_channels::Blending::Disabled,
            },
            &mut random,
            &mut NoFire,
        )
        .unwrap();
        assert_eq!(
            next.channels[..2].concat(),
            w[23..],
            "original secondary row {count}"
        );
        assert_eq!(next.channels[2..], channels[2..]);
        assert_eq!(random.0, w[3]);
        assert_eq!((next.mode, next.timer.to_bits()), (w[0], w[1]));
        count += 1;
    }
    assert_eq!(count, 6);
}
#[derive(Default)]
struct CloseSounds {
    calls: Vec<u32>,
    fail: bool,
}
impl Services<u32, u32> for CloseSounds {
    fn remove_effect(&mut self, _: &State<u32, u32>, _: u32, _: &mut Random) -> Result<(), String> {
        Err("unexpected close-branch effect removal".into())
    }
    fn owner_sound(
        &mut self,
        s: &State<u32, u32>,
        cue: u32,
        _: &mut Random,
    ) -> Result<SoundOutcome, String> {
        assert_eq!(s.entry.mode, 67);
        assert!(matches!(cue, 1231 | 1248));
        self.calls.push(cue);
        if self.fail {
            Err("supplied close cue failure".into())
        } else {
            Ok(SoundOutcome::Suppressed)
        }
    }
}
fn close_state() -> State<u32, u32> {
    let mut s = state();
    s.entry.class = 230;
    s.entry.mode = 67;
    s.steering.heading = 90.;
    s.effect = None;
    s
}
fn close_parameters() -> VariantCloseContext {
    VariantCloseContext {
        attack_range: 20.,
        alternate_threshold: 50.,
    }
}
#[test]
fn variant_close_selects_both_attacks_at_strict_range_without_forcing_motion() {
    for (distance, seed, mode, cue) in [
        (19., 1, 69, None),
        (19., 10000, 68, Some(1231)),
        (20., 1, 67, None),
        (21., 1, 67, None),
    ] {
        let mut peer = candidate();
        peer.position = [distance, 0., 0.];
        let peers = [peer];
        let frame = Frame::new(&peers, &[42]).unwrap();
        let input = FrameInput {
            context: context(),
            frame: &frame,
            range: 500.,
        };
        let mut machine = Machine::new(close_state());
        let mut random = Random(seed);
        let mut sounds = CloseSounds::default();
        assert_eq!(
            machine
                .advance_variant_close(input, close_parameters(), &mut random, &mut sounds)
                .unwrap(),
            Outcome::CompletedVariantClose
        );
        assert_eq!(machine.state().entry.mode, mode);
        assert_eq!(machine.state().entry.position, [0.; 3]);
        assert_eq!(machine.state().steering.position, [0.; 3]);
        assert_eq!(machine.state().steering.target_position, peer.position);
        assert_eq!(machine.state().steering.commands[2], 32);
        assert_eq!(sounds.calls, cue.into_iter().collect::<Vec<_>>());
        let mut expected = Random(seed);
        if distance < 20. {
            expected.next_15();
        }
        assert_eq!(random.0, expected.0);
        assert!(machine.awaiting_motion());
        assert!(
            machine
                .advance_variant_close(input, close_parameters(), &mut random, &mut sounds)
                .is_err()
        );
    }
}
#[test]
fn variant_close_failed_cue_keeps_steering_and_rng_without_selecting_or_replaying_attack() {
    let peers = [candidate()];
    let frame = Frame::new(&peers, &[42]).unwrap();
    let input = FrameInput {
        context: context(),
        frame: &frame,
        range: 500.,
    };
    let mut machine = Machine::new(close_state());
    let mut random = Random(10000);
    let mut sounds = CloseSounds {
        fail: true,
        ..Default::default()
    };
    assert!(
        machine
            .advance_variant_close(input, close_parameters(), &mut random, &mut sounds)
            .is_err()
    );
    assert_eq!(machine.state().entry.mode, 67);
    assert_eq!(machine.state().steering.commands[2], 32);
    assert_eq!(machine.state().steering.target_position, peers[0].position);
    assert_eq!(random.0, 2142661011);
    assert!(machine.failure().is_some());
    assert!(
        machine
            .advance_variant_close(input, close_parameters(), &mut random, &mut sounds)
            .is_err()
    );
    assert_eq!(sounds.calls, [1231]);
}
#[test]
#[ignore = "Requires GIANTS_RIPPER_CLOSE_CONTROLS; private original complete mode67 primary observations"]
fn native_variant_close_primary_controls() {
    let path = std::env::var("GIANTS_RIPPER_CLOSE_CONTROLS").expect("private close controls path");
    let contents = std::fs::read_to_string(path).unwrap();
    let mut count = 0;
    for line in contents
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
    {
        let w: Vec<u32> = line
            .split_whitespace()
            .map(|s| u32::from_str_radix(s, 16).unwrap())
            .collect();
        assert_eq!(w.len(), 56);
        let f = |i| f32::from_bits(w[i]);
        let mut s = close_state();
        s.entry.flags = w[0];
        s.entry.control = w[1];
        s.steering.personality = w[2];
        s.entry.position = [f(3), f(4), f(5)];
        s.steering.position = s.entry.position;
        s.steering.flags = s.entry.flags;
        s.steering.heading = f(6);
        s.steering.commands = [w[7].to_le_bytes(), w[8].to_le_bytes()]
            .concat()
            .try_into()
            .unwrap();
        s.entry.packed = w[7];
        s.steering.target_heading = f(9);
        s.steering.target_position = [f(10), f(11), f(12)];
        s.entry.retained_timer = f(13);
        s.timer = f(14);
        s.entry.parameter = f(15);
        s.entry.active_limit = f(16);
        s.entry.saved_parameter = f(17);
        s.entry.context = w[18];
        s.primary.copy_from_slice(&w[19..28]);
        s.retaliation = (w[29] != 0).then_some(42);
        let peer = Candidate {
            id: 42,
            class: w[48] as i32,
            state: w[49],
            active: w[50],
            personality: w[51],
            control: w[52],
            position: [f(30), f(31), f(32)],
            damage: f(53),
            limit: f(54),
            parent_present: w[55] != 0,
        };
        let peers = [peer];
        let frame = Frame::new(&peers, if w[29] != 0 { &[42] } else { &[] }).unwrap();
        let input = FrameInput {
            context: context(),
            frame: &frame,
            range: 500.,
        };
        let mut random = Random(w[28]);
        let mut sounds = CloseSounds::default();
        let mut machine = Machine::new(s);
        machine
            .advance_variant_close(
                input,
                VariantCloseContext {
                    attack_range: f(33),
                    alternate_threshold: f(34),
                },
                &mut random,
                &mut sounds,
            )
            .unwrap();
        let s = machine.state();
        let mut actual = vec![
            s.entry.mode,
            s.entry.flags,
            s.entry.control,
            s.steering.personality,
            u32::from_le_bytes(s.steering.commands[..4].try_into().unwrap()),
            u32::from_le_bytes(s.steering.commands[4..].try_into().unwrap()),
            s.steering.target_heading.to_bits(),
        ];
        actual.extend(s.steering.target_position.map(f32::to_bits));
        actual.extend([
            u32::from(s.retaliation.is_some()),
            random.0,
            sounds.calls.first().copied().unwrap_or(0),
        ]);
        assert_eq!(actual, w[35..48], "original mode67 row {count}");
        assert!(sounds.calls.len() <= 1);
        count += 1;
    }
    assert_eq!(count, 6);
}
