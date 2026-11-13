// Explicit supplied controls, not ordinary damage or sound owners.
#[derive(Default)]
struct MeleeControls {
    sounds: Vec<u32>,
    requests: Vec<(u32, u32)>,
}
impl Services<u32, u32> for MeleeControls {
    fn remove_effect(&mut self, _: &State<u32, u32>, _: u32, _: &mut Random) -> Result<(), String> {
        Err("unexpected melee effect retirement".into())
    }
    fn owner_sound(
        &mut self,
        state: &State<u32, u32>,
        cue: u32,
        _: &mut Random,
    ) -> Result<SoundOutcome, String> {
        assert_eq!((state.entry.mode, cue), (68, 1248));
        self.sounds.push(cue);
        Ok(SoundOutcome::Suppressed)
    }
    fn melee(
        &mut self,
        state: &State<u32, u32>,
        target: u32,
        amount: f32,
        _: &mut Random,
    ) -> Result<(), String> {
        assert_eq!(state.entry.mode, 66);
        self.requests.push((target, amount.to_bits()));
        Err("supplied observation stops before kind3 hurt; no damage backend".into())
    }
}
fn melee_state(completed: bool) -> State<u32, u32> {
    let mut state = close_state();
    state.entry.mode = 68;
    state.primary[0] = 10;
    state.primary[8] = u32::from(completed) * 2;
    state
}
#[test]
fn variant_melee_waits_for_animation_and_uses_full_distance_without_forward_motion() {
    for (completed, position, expected) in [
        (false, [19., 0., 0.], 68),
        (true, [20., 0., 0.], 66),
        (true, [19., 0., 20.], 66),
    ] {
        let mut peer = candidate();
        peer.position = position;
        let peers = [peer];
        let frame = Frame::new(&peers, &[42]).unwrap();
        let input = FrameInput {
            context: context(),
            frame: &frame,
            range: 500.,
        };
        let mut machine = Machine::new(melee_state(completed));
        let mut random = Random(17);
        let mut services = MeleeControls::default();
        machine
            .advance_variant_melee(
                input,
                VariantMeleeContext {
                    attack_range: 20.,
                    damage: 35.,
                },
                &mut random,
                &mut services,
            )
            .unwrap();
        assert_eq!(machine.state().entry.mode, expected);
        assert_eq!(machine.state().entry.position, [0.; 3]);
        assert_eq!(machine.state().steering.commands[2], 0);
        assert_eq!(machine.state().steering.flags & 0x02000000, 0);
        assert!(services.requests.is_empty());
        assert_eq!(random.0, 17);
        assert!(machine.awaiting_motion());
    }
}
#[test]
fn variant_melee_requires_kind3_consumer_after_mode_change_and_refuses_replay() {
    let peers = [candidate()];
    let frame = Frame::new(&peers, &[42]).unwrap();
    let input = FrameInput {
        context: context(),
        frame: &frame,
        range: 500.,
    };
    let mut machine = Machine::new(melee_state(true));
    let mut random = Random(17);
    let mut services = MeleeControls::default();
    let parameters = VariantMeleeContext {
        attack_range: 20.,
        damage: 35.,
    };
    assert!(
        machine
            .advance_variant_melee(input, parameters, &mut random, &mut services)
            .is_err()
    );
    assert_eq!(machine.state().entry.mode, 66);
    assert!(machine.failure().is_some());
    assert_eq!(services.requests, [(42, 35f32.to_bits())]);
    assert_eq!(random.0, 17);
    assert!(
        machine
            .advance_variant_melee(input, parameters, &mut random, &mut services)
            .is_err()
    );
    assert_eq!(services.requests.len(), 1);
}
#[test]
#[ignore = "Requires GIANTS_RIPPER_MELEE_CONTROLS; six complete original primaries and one actual pre-hurt service boundary, not damage or ordinary encounter proof"]
fn native_variant_melee_primary_controls() {
    let contents = std::fs::read_to_string(
        std::env::var("GIANTS_RIPPER_MELEE_CONTROLS").expect("private melee controls path"),
    )
    .unwrap();
    let mut count = 0;
    let mut boundaries = 0;
    for line in contents
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
    {
        let w: Vec<u32> = line
            .split_whitespace()
            .map(|s| u32::from_str_radix(s, 16).unwrap())
            .collect();
        assert_eq!(w.len(), 57);
        let f = |i| f32::from_bits(w[i]);
        let mut state = melee_state(false);
        state.entry.flags = w[0];
        state.entry.control = w[1];
        state.steering.personality = w[2];
        state.entry.position = [f(3), f(4), f(5)];
        state.steering.position = state.entry.position;
        state.steering.flags = state.entry.flags;
        state.steering.heading = f(6);
        state.steering.commands = [w[7].to_le_bytes(), w[8].to_le_bytes()]
            .concat()
            .try_into()
            .unwrap();
        state.entry.packed = w[7];
        state.steering.target_heading = f(9);
        state.steering.target_position = [f(10), f(11), f(12)];
        state.entry.retained_timer = f(13);
        state.timer = f(14);
        state.entry.parameter = f(15);
        state.entry.active_limit = f(16);
        state.entry.saved_parameter = f(17);
        state.entry.context = w[18];
        state.primary.copy_from_slice(&w[19..28]);
        state.retaliation = (w[29] != 0).then_some(42);
        let peers = [Candidate {
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
        }];
        let frame = Frame::new(&peers, if w[29] != 0 { &[42] } else { &[] }).unwrap();
        let input = FrameInput {
            context: context(),
            frame: &frame,
            range: 500.,
        };
        let mut random = Random(w[28]);
        let mut services = MeleeControls::default();
        let mut machine = Machine::new(state);
        let result = machine.advance_variant_melee(
            input,
            VariantMeleeContext {
                attack_range: f(33),
                damage: f(34),
            },
            &mut random,
            &mut services,
        );
        assert_eq!(
            result.is_err(),
            w[56] != 0,
            "original pre-hurt boundary row {count}"
        );
        assert_eq!(machine.failure().is_some(), w[56] != 0);
        assert_eq!(
            services.requests,
            if w[56] != 0 {
                vec![(42, w[34])]
            } else {
                vec![]
            }
        );
        boundaries += w[56];
        let state = machine.state();
        let mut actual = vec![
            state.entry.mode,
            state.entry.flags,
            state.entry.control,
            state.steering.personality,
            u32::from_le_bytes(state.steering.commands[..4].try_into().unwrap()),
            u32::from_le_bytes(state.steering.commands[4..].try_into().unwrap()),
            state.steering.target_heading.to_bits(),
        ];
        actual.extend(state.steering.target_position.map(f32::to_bits));
        actual.extend([
            u32::from(state.retaliation.is_some()),
            random.0,
            services.sounds.first().copied().unwrap_or(0),
        ]);
        assert_eq!(actual, w[35..48], "original melee primary row {count}");
        count += 1;
    }
    assert_eq!((count, boundaries), (7, 1));
}
