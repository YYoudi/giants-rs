// Supplied phase and marker/audio services, not ordinary world admission.
#[derive(Default)]
struct ClassicRetreatHost {
    linked: bool,
    marker_mode: u32,
    cue: u32,
    fail_mark: bool,
    fail_sound: bool,
    calls: Vec<&'static str>,
}
impl Services<u32, u32> for ClassicRetreatHost {
    fn remove_effect(&mut self, _: &State<u32, u32>, _: u32, _: &mut Random) -> Result<(), String> {
        Err("unexpected retreat effect removal".into())
    }
    fn mark_surface_marker_retirement(&mut self, state: &State<u32, u32>) -> Result<(), String> {
        self.calls.push("mark");
        assert_eq!(state.entry.mode, 43);
        assert_eq!(state.steering.personality & 0x40, 0);
        if self.linked {
            self.marker_mode = 2;
        }
        if self.fail_mark {
            Err("supplied late marker refusal".into())
        } else {
            Ok(())
        }
    }
    fn owner_sound(
        &mut self,
        state: &State<u32, u32>,
        cue: u32,
        _: &mut Random,
    ) -> Result<SoundOutcome, String> {
        self.calls.push("sound");
        assert_eq!((state.entry.mode, cue), (43, 1258));
        self.cue = cue;
        if self.fail_sound {
            Err("supplied retreat sound refusal".into())
        } else {
            Ok(SoundOutcome::Suppressed)
        }
    }
}
fn classic_retreat_state(time: u32, wrapped: bool) -> State<u32, u32> {
    let mut s = state();
    s.entry.mode = 43;
    s.entry.position = [4., 6., 2.];
    s.entry.saved_parameter = 100.;
    s.steering.position = s.entry.position;
    s.steering.personality = 0x800;
    s.primary = [43, time, 0, 0, 0, 0, 0, 0, u32::from(wrapped) * 2];
    s
}
#[test]
fn classic_retreat_keeps_partial_child_and_sound_effects_without_replay() {
    let actors = [candidate()];
    let frame = Frame::new(&actors, &[42]).unwrap();
    for fail_mark in [false, true] {
        let mut machine = Machine::new(classic_retreat_state(1f32.to_bits(), true));
        let mut random = Random(1);
        let mut host = ClassicRetreatHost {
            linked: true,
            marker_mode: 1,
            fail_mark,
            fail_sound: !fail_mark,
            ..Default::default()
        };
        let input = FrameInput {
            context: context(),
            frame: &frame,
            range: 1000.,
        };
        assert!(
            machine
                .advance_retreat(input, 50., &mut random, &mut host)
                .is_err()
        );
        assert_eq!(host.marker_mode, 2);
        assert_eq!(machine.state().entry.mode, 43);
        assert_eq!(
            machine.state().steering.personality,
            if fail_mark { 0x800 } else { 0x840 }
        );
        assert_eq!(
            host.calls,
            if fail_mark {
                vec!["mark"]
            } else {
                vec!["mark", "sound"]
            }
        );
        let calls = host.calls.len();
        assert!(
            machine
                .advance_retreat(input, 50., &mut random, &mut host)
                .is_err()
        );
        assert_eq!(host.calls.len(), calls);
        assert_eq!(random.0, 1);
    }
}
#[test]
#[ignore = "Requires GIANTS_RIPPER_RETREAT_CONTROLS; private original mode43 helper with supplied actor/marker/channels and absent sound registry, not full native world or graphics"]
fn native_classic_retreat_controls() {
    let text =
        std::fs::read_to_string(std::env::var("GIANTS_RIPPER_RETREAT_CONTROLS").unwrap()).unwrap();
    let actors = [candidate()];
    let frame = Frame::new(&actors, &[42]).unwrap();
    let mut count = 0;
    for line in text
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
    {
        let w: Vec<_> = line
            .split_whitespace()
            .map(|v| u32::from_str_radix(v, 16).unwrap())
            .collect();
        assert_eq!(w.len(), 12);
        let mut machine = Machine::new(classic_retreat_state(w[2], w[1] != 0));
        let mut random = Random(1);
        let mut host = ClassicRetreatHost {
            linked: w[0] != 0,
            marker_mode: 1,
            ..Default::default()
        };
        assert_eq!(
            machine
                .advance_retreat(
                    FrameInput {
                        context: context(),
                        frame: &frame,
                        range: 1000.
                    },
                    50.,
                    &mut random,
                    &mut host
                )
                .unwrap(),
            Outcome::CompletedRetreat
        );
        let s = machine.state();
        assert_eq!(
            [
                s.entry.mode,
                s.steering.personality,
                host.marker_mode,
                host.cue
            ],
            w[4..8]
        );
        assert_eq!(s.entry.position.map(f32::to_bits), w[8..11]);
        assert_eq!(u32::from(s.retaliation == Some(42)), w[11]);
        assert_eq!(s.entry.parameter, if w[1] != 0 { 50. } else { 100. });
        assert!(machine.awaiting_motion());
        assert_eq!(random.0, 1);
        count += 1;
    }
    assert_eq!(count, 16);
}
