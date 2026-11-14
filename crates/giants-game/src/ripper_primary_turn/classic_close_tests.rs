// Explicit phase controls; these hosts do not provide ordinary world readiness.
#[derive(Default)]
struct ClassicApproachHost {
    cue: u32,
    fail: bool,
    calls: u32,
}
impl Services<u32, u32> for ClassicApproachHost {
    fn remove_effect(&mut self, _: &State<u32, u32>, _: u32, _: &mut Random) -> Result<(), String> {
        Err("unexpected close effect retirement".into())
    }
    fn owner_sound(
        &mut self,
        s: &State<u32, u32>,
        cue: u32,
        _: &mut Random,
    ) -> Result<SoundOutcome, String> {
        assert_eq!(s.entry.mode, 41);
        self.cue = cue;
        self.calls += 1;
        if self.fail {
            Err("supplied close sound failure".into())
        } else {
            Ok(SoundOutcome::ResourceUnavailable)
        }
    }
}
fn classic_close_state(heading: f32) -> State<u32, u32> {
    let mut s = state();
    s.entry.mode = 41;
    s.entry.flags = 0;
    s.entry.position = [4., 6., 2.];
    s.entry.retained_timer = 0.;
    s.steering.position = s.entry.position;
    s.steering.heading = heading;
    s.steering.flags = 0;
    s.steering.personality = 0;
    s.primary = [2, 0, 0, 0, 0, 0, 0, 0, 0];
    s.effect = None;
    s
}
#[test]
fn classic_close_sound_failure_retains_real_steering_without_replay() {
    let mut peer = candidate();
    peer.position = [4., 5., 2.];
    peer.limit = 100.;
    let actors = [peer];
    let frame = Frame::new(&actors, &[42]).unwrap();
    let mut machine = Machine::new(classic_close_state(0.));
    let mut random = Random(7);
    let mut host = ClassicApproachHost {
        fail: true,
        ..Default::default()
    };
    let input = FrameInput {
        context: context(),
        frame: &frame,
        range: 1000.,
    };
    assert!(
        machine
            .advance_classic_close(input, 10., &mut random, &mut host)
            .is_err()
    );
    assert_eq!(machine.state().entry.mode, 41);
    assert_eq!(machine.state().steering.target_position, peer.position);
    assert_ne!(machine.state().steering.flags & 0x02000000, 0);
    assert_eq!(host.cue, 1254);
    assert!(
        machine
            .advance_classic_close(input, 10., &mut random, &mut host)
            .is_err()
    );
    assert_eq!(host.calls, 1);
    assert_eq!(random.0, 7);
}
#[test]
#[ignore = "Requires GIANTS_RIPPER_CLASSIC_CLOSE_CONTROLS; private original mode41 helper with supplied headers/globals, not ordinary world or rendered combat"]
fn native_classic_close_controls() {
    let data =
        std::fs::read_to_string(std::env::var("GIANTS_RIPPER_CLASSIC_CLOSE_CONTROLS").unwrap())
            .unwrap();
    let mut count = 0;
    for line in data
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
    {
        let w: Vec<u32> = line
            .split_whitespace()
            .map(|v| u32::from_str_radix(v, 16).unwrap())
            .collect();
        assert_eq!(w.len(), 18);
        let mut peer = candidate();
        peer.position = [
            f32::from_bits(w[1]),
            f32::from_bits(w[2]),
            f32::from_bits(w[3]),
        ];
        peer.active = w[4];
        peer.control = w[5];
        peer.damage = f32::from_bits(w[6]);
        peer.limit = 100.;
        let actors = [peer];
        let frame = Frame::new(&actors, &[42]).unwrap();
        let mut machine = Machine::new(classic_close_state(f32::from_bits(w[0])));
        let mut random = Random(1);
        let mut host = ClassicApproachHost::default();
        assert_eq!(
            machine
                .advance_classic_close(
                    FrameInput {
                        context: context(),
                        frame: &frame,
                        range: 1000.
                    },
                    10.,
                    &mut random,
                    &mut host
                )
                .unwrap(),
            Outcome::CompletedClassicClose
        );
        let s = machine.state();
        let mut actual = vec![
            s.entry.mode,
            s.entry.control,
            s.entry.flags,
            u32::from(s.retaliation == Some(42)),
            u32::from_le_bytes(s.steering.commands[..4].try_into().unwrap()),
            u32::from_le_bytes(s.steering.commands[4..].try_into().unwrap()),
            s.steering.target_heading.to_bits(),
        ];
        actual.extend(s.steering.target_position.map(f32::to_bits));
        actual.push(host.cue);
        assert_eq!(actual, &w[7..], "case{count}");
        assert_eq!(random.0, 1);
        assert!(machine.awaiting_motion());
        count += 1;
    }
    assert_eq!(count, 18);
}
