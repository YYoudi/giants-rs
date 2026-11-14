// The native contact cases stop at the actual injury call, without executing
// or acknowledging its receiver. This host preserves that explicit boundary.
#[derive(Default)]
struct ClassicMeleeHost {
    cue: u32,
    hurt: bool,
}
impl Services<u32, u32> for ClassicMeleeHost {
    fn remove_effect(&mut self, _: &State<u32, u32>, _: u32, _: &mut Random) -> Result<(), String> {
        Err("unexpected dirt removal".into())
    }
    fn owner_sound(
        &mut self,
        _: &State<u32, u32>,
        cue: u32,
        _: &mut Random,
    ) -> Result<SoundOutcome, String> {
        self.cue = cue;
        Ok(SoundOutcome::ResourceUnavailable)
    }
    fn melee(
        &mut self,
        s: &State<u32, u32>,
        target: u32,
        amount: f32,
        _: &mut Random,
    ) -> Result<(), String> {
        assert_eq!((s.entry.mode, target, amount), (35, 42, 7.));
        self.hurt = true;
        Err("stop before required kind3 consumer".into())
    }
}
#[test]
#[ignore = "Requires GIANTS_RIPPER_CLASSIC_MELEE_CONTROLS; original helper with supplied globals/headers/TLS; contact cases stop BEFORE actual kind3 consumer, no injury/readiness acceptance"]
fn native_classic_melee_controls() {
    let data =
        std::fs::read_to_string(std::env::var("GIANTS_RIPPER_CLASSIC_MELEE_CONTROLS").unwrap())
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
        assert_eq!(w.len(), 21);
        let mut peer = candidate();
        peer.position = [
            f32::from_bits(w[1]),
            f32::from_bits(w[2]),
            f32::from_bits(w[3]),
        ];
        peer.active = w[5];
        peer.control = w[6];
        peer.personality = 0;
        let actors = [peer];
        let frame = Frame::new(&actors, &[42]).unwrap();
        let mut s = classic_close_state(f32::from_bits(w[0]));
        s.entry.mode = 42;
        s.primary[0] = 42;
        s.primary[8] = w[4] * 2;
        let mut machine = Machine::new(s);
        let mut random = Random(w[7]);
        let mut host = ClassicMeleeHost::default();
        let input = FrameInput {
            context: context(),
            frame: &frame,
            range: 1000.,
        };
        let p = ClassicMeleeContext {
            attack_range: 10.,
            damage: 7.,
            orbit: Parameters {
                radius: [8., 20.],
                seconds: 4.,
            },
        };
        let result = machine.advance_classic_melee(input, p, &mut random, &mut host);
        assert_eq!(result.is_err(), w[20] != 0, "case{count} {result:?}");
        let s = machine.state();
        let actual = [
            s.entry.mode,
            s.entry.control,
            s.entry.flags,
            u32::from_le_bytes(s.steering.commands[..4].try_into().unwrap()),
            u32::from_le_bytes(s.steering.commands[4..].try_into().unwrap()),
            s.steering.target_heading.to_bits(),
            s.timer.to_bits(),
            s.orbit.angle.to_bits(),
            s.orbit.radius.to_bits(),
            u32::from(s.retaliation == Some(42)),
            random.0,
            host.cue,
            u32::from(host.hurt),
        ];
        assert_eq!(actual, &w[8..], "case{count}");
        if host.hurt {
            assert!(
                machine
                    .advance_classic_melee(input, p, &mut random, &mut host)
                    .is_err()
            );
        }
        count += 1;
    }
    assert_eq!(count, 14);
}
