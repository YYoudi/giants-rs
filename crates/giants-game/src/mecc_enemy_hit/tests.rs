use super::*;
#[test]
fn projectile_and_melee_identities_cannot_be_exchanged() {
    for melee_from_shot in [true, false] {
        let mut request = request();
        request.kind = if melee_from_shot { 3 } else { 0 };
        request.impact = None;
        if !melee_from_shot {
            request.source = mecc_hit_source::Source {
                actor: actor(3, 230, 23, 100),
                owner: None,
            };
        }
        let mut current = state();
        let mut random = Random(17);
        let mut sink = Sink {
            cues: vec![],
            fail_at: None,
        };
        let before = words(current, random, &sink);
        assert!(
            Machine::default()
                .run(&mut current, request, &mut random, &mut sink)
                .is_err()
        );
        assert_eq!(words(current, random, &sink), before);
    }
}
#[test]
#[ignore = "Requires GIANTS_MECC_KIND3_CONTROLS; private complete original melee dispatcher over supplied class73/230 state23 sources and guards; not an ordinary provider"]
fn native_mecc_kind3_receiver_controls() {
    let contents = std::fs::read_to_string(
        std::env::var("GIANTS_MECC_KIND3_CONTROLS").expect("private kind3 controls path"),
    )
    .unwrap();
    let mut count = 0;
    for line in contents
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
    {
        let row: Vec<u32> = line
            .split_whitespace()
            .map(|s| u32::from_str_radix(s, 16).unwrap())
            .collect();
        assert_eq!(row.len(), 20);
        let mut request = request();
        request.kind = 3;
        request.source = mecc_hit_source::Source {
            actor: actor(3, row[19] as i32, 23, 100),
            owner: None,
        };
        request.amount = 35.;
        request.impact = None;
        request.globally_disabled = row[2] != 0;
        request.suppressed = row[3] != 0;
        let mut current = state();
        current.pain_timer = 0.;
        current.shared_timer = 0.;
        let mut random = Random(17);
        let mut sink = Sink {
            cues: vec![],
            fail_at: None,
        };
        let mut machine = Machine::default();
        machine
            .run(&mut current, request, &mut random, &mut sink)
            .unwrap();
        let mut actual = vec![current.health.damage.to_bits(), current.health.flags];
        actual.extend(current.velocity.map(f32::to_bits));
        assert_eq!(
            actual,
            row[5..10],
            "original kind3 health/impulse row {count}"
        );
        assert_eq!((current.callback_reset_word, random.0), (row[15], row[16]));
        assert!(sink.cues.is_empty());
        assert_eq!(machine.phase(), Phase::Complete);
        count += 1;
    }
    assert_eq!(count, 8);
}
struct Sink {
    cues: Vec<u32>,
    fail_at: Option<usize>,
}
impl Services<u32> for Sink {
    fn retaliation(&mut self, _: u32, _: Option<u32>) -> Result<(), String> {
        panic!("cold target fixture has no associated player")
    }
    fn voice(&mut self, cue: u32, state: &State, _: &Random) -> Result<(), String> {
        assert!(state.velocity != [1., 2., 3.]);
        assert_eq!(state.callback_reset_word, u32::MAX);
        self.cues.push(cue);
        if self.fail_at == Some(self.cues.len()) {
            Err("actual cue transport failed".into())
        } else {
            Ok(())
        }
    }
}
fn actor(id: u32, class: i32, state: u32, context: u32) -> mecc_hit_source::Actor<u32> {
    mecc_hit_source::Actor {
        id,
        class,
        state,
        context,
    }
}
fn request() -> Request<u32> {
    Request {
        recipient: actor(1, 2, 2, 0),
        source: mecc_hit_source::Source {
            actor: actor(2, 232, 6, 100),
            owner: Some(actor(3, 230, 23, 100)),
        },
        associated_player: None,
        kind: 2,
        amount: 12.,
        globally_disabled: false,
        suppressed: false,
        gyro_present: false,
        critical_voice_present: false,
        recipient_motion: damage_impulse::Motion {
            current: [8., 10., 4.],
            previous: [7., 9., 3.],
        },
        source_motion: damage_impulse::Motion {
            current: [4., 6., 2.],
            previous: [1., 2., 1.],
        },
        impact: Some([4., 6., 2.]),
        radial_center: [8., 10., 4.],
    }
}
fn state() -> State {
    State {
        health: mecc_damage::State::cold(),
        velocity: [1., 2., 3.],
        pain_timer: -0.1,
        shared_timer: -0.1,
        callback_reset_word: 42,
    }
}
fn words(state: State, random: Random, sink: &Sink) -> Vec<u32> {
    let mut result = vec![state.health.damage.to_bits(), state.health.flags];
    result.extend(state.velocity.map(f32::to_bits));
    result.extend([
        state.pain_timer.to_bits(),
        state.shared_timer.to_bits(),
        random.0,
        state.callback_reset_word,
        sink.cues.len() as u32,
    ]);
    result.extend((0..2).map(|i| sink.cues.get(i).copied().unwrap_or(0)));
    result
}
#[test]
fn late_voice_failure_keeps_health_impulse_and_completed_timers_without_replay() {
    let mut state = state();
    let mut random = Random(17);
    let mut sink = Sink {
        cues: vec![],
        fail_at: Some(2),
    };
    let mut machine = Machine::default();
    assert!(
        machine
            .run(&mut state, request(), &mut random, &mut sink)
            .is_err()
    );
    assert_eq!(machine.phase(), Phase::Failed);
    assert_eq!(state.health.damage, 12.);
    assert!(state.pain_timer > 0.);
    assert_eq!(state.shared_timer, -0.1);
    let before = words(state, random, &sink);
    sink.fail_at = None;
    assert!(
        machine
            .run(&mut state, request(), &mut random, &mut sink)
            .is_err()
    );
    assert_eq!(words(state, random, &sink), before);
}
#[test]
fn failed_associated_player_consumer_precedes_health_and_cannot_replay() {
    struct Associated(Vec<(u32, Option<u32>)>);
    impl Services<u32> for Associated {
        fn retaliation(&mut self, player: u32, candidate: Option<u32>) -> Result<(), String> {
            self.0.push((player, candidate));
            Err("real associated-player consumer failed".into())
        }
        fn voice(&mut self, _: u32, _: &State, _: &Random) -> Result<(), String> {
            panic!("voice must not bypass failed associated-player consumer")
        }
    }
    let mut state = state();
    let mut random = Random(17);
    let empty = Sink {
        cues: vec![],
        fail_at: None,
    };
    let before = words(state, random, &empty);
    let mut input = request();
    input.associated_player = Some(9);
    let mut service = Associated(vec![]);
    let mut machine = Machine::default();
    assert!(
        machine
            .run(&mut state, input, &mut random, &mut service)
            .is_err()
    );
    assert_eq!(service.0, [(9, Some(3))]);
    assert_eq!(words(state, random, &empty), before);
    assert!(
        machine
            .run(&mut state, request(), &mut random, &mut service)
            .is_err()
    );
    assert_eq!(service.0.len(), 1);
}
#[test]
fn unsupported_lethal_and_equipped_profiles_do_not_publish_false_reactions() {
    for variant in 0..4 {
        let mut state = state();
        let mut request = request();
        match variant {
            0 => state.health.damage = state.health.limit - 1.,
            1 => state.health.flags |= 0x800000,
            2 => request.gyro_present = true,
            _ => request.critical_voice_present = true,
        }
        let mut random = Random(17);
        let mut sink = Sink {
            cues: vec![],
            fail_at: None,
        };
        let before = words(state, random, &sink);
        assert!(
            Machine::default()
                .run(&mut state, request, &mut random, &mut sink)
                .is_err()
        );
        assert_eq!(words(state, random, &sink), before);
    }
}
#[test]
#[ignore = "Requires GIANTS_MECC_ENEMY_HIT_CONTROLS; original PC nonlethal callback with supplied source/pose/timers/world gates/TLS and empty sound registry; no Session/runtime/GPU proof"]
fn source_bearing_mecc_hits_match_original_callback() {
    let text = std::fs::read_to_string(std::env::var_os("GIANTS_MECC_ENEMY_HIT_CONTROLS").unwrap())
        .unwrap();
    let mut cases = 0;
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let row = line
            .split_whitespace()
            .map(|v| u32::from_str_radix(v, 16).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(row.len(), 17);
        let mut request = request();
        request.kind = row[0];
        request.globally_disabled = row[1] != 0;
        request.suppressed = row[2] != 0;
        let mut state = state();
        state.pain_timer = f32::from_bits(row[3]);
        state.shared_timer = state.pain_timer;
        state.health.damage = f32::from_bits(row[4]);
        let mut random = Random(17);
        let mut sink = Sink {
            cues: vec![],
            fail_at: None,
        };
        Machine::default()
            .run(&mut state, request, &mut random, &mut sink)
            .unwrap();
        assert_eq!(words(state, random, &sink), row[5..], "case {cases}");
        cases += 1;
    }
    assert_eq!(cases, 32);
}
