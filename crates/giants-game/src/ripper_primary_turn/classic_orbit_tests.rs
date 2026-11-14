// Original-helper controls have supplied outer headers, TLS, globals and ground.
// They establish phase behavior, not an ordinary gameplay or damage receipt.
#[derive(Default)]
struct OrbitHost {
    cue: u32,
    marker_mode: u32,
}
impl Services<u32, u32> for OrbitHost {
    fn remove_effect(&mut self, _: &State<u32, u32>, _: u32, _: &mut Random) -> Result<(), String> {
        Err("unexpected opaque effect removal".into())
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
    fn stop_owner_sound(
        &mut self,
        _: &State<u32, u32>,
        cue: u32,
        _: &mut Random,
    ) -> Result<SoundOutcome, String> {
        self.cue = cue;
        Ok(SoundOutcome::ResourceUnavailable)
    }
    fn ground_normal_z(&mut self, _: &State<u32, u32>) -> Result<f32, String> {
        Ok(1.)
    }
    fn mark_surface_marker_retirement(&mut self, _: &State<u32, u32>) -> Result<(), String> {
        self.marker_mode = 2;
        Ok(())
    }
}
#[test]
#[ignore = "Requires GIANTS_RIPPER_CLASSIC_ORBIT_CONTROLS; private original helper, supplied headers/globals/flat ground and existing dirt identity, no live world or rendered acceptance"]
fn native_classic_orbit_controls() {
    let data =
        std::fs::read_to_string(std::env::var("GIANTS_RIPPER_CLASSIC_ORBIT_CONTROLS").unwrap())
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
        assert_eq!(w.len(), 28);
        let mut peer = candidate();
        peer.position = [
            f32::from_bits(w[1]),
            f32::from_bits(w[2]),
            f32::from_bits(w[3]),
        ];
        peer.active = w[8];
        peer.personality = 0;
        let actors = [peer];
        let frame = Frame::new(&actors, &[42]).unwrap();
        let mut s = classic_close_state(0.);
        s.entry.mode = w[0];
        s.primary[0] = w[0];
        s.primary[1] = w[10];
        s.primary[8] = w[9] * 2;
        s.timer = f32::from_bits(w[4]);
        s.orbit = Orbit {
            angle: f32::from_bits(w[5]),
            radius: f32::from_bits(w[6]),
        };
        s.effect = if w[0] == 45 { Some(7) } else { None };
        let mut machine = Machine::new(s);
        let mut random = Random(w[7]);
        let mut host = OrbitHost {
            marker_mode: 1,
            ..Default::default()
        };
        let p = ClassicOrbitParameters {
            orbit: Parameters {
                radius: [8., 20.],
                seconds: 4.,
            },
            decision_range: 10.,
            close_percent: 50.,
            fire_percent: 50.,
            buried_speed_percent: 50.,
        };
        assert_eq!(
            machine
                .advance_classic_orbit(
                    FrameInput {
                        context: context(),
                        frame: &frame,
                        range: 1000.
                    },
                    p,
                    &mut random,
                    &mut host
                )
                .unwrap(),
            Outcome::CompletedClassicOrbit
        );
        let s = machine.state();
        let mut actual = vec![
            s.entry.mode,
            s.entry.control,
            s.entry.flags,
            s.steering.personality,
            u32::from_le_bytes(s.steering.commands[..4].try_into().unwrap()),
            u32::from_le_bytes(s.steering.commands[4..].try_into().unwrap()),
            s.steering.target_heading.to_bits(),
        ];
        actual.extend(s.steering.target_position.map(f32::to_bits));
        actual.extend([
            s.parameter_184.to_bits(),
            s.timer.to_bits(),
            s.orbit.angle.to_bits(),
            s.orbit.radius.to_bits(),
            u32::from(s.retaliation == Some(42)),
            host.marker_mode,
            random.0,
        ]);
        assert_eq!(actual, &w[11..], "case{count}");
        assert!(machine.awaiting_motion());
        count += 1;
    }
    assert_eq!(count, 25);
}
