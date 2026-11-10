//! Optional private captured-entry control; no original input is embedded here.
use super::*;
use crate::{actor_common_motion as motion, vimp_response::Predicate, vimp_wander::put};
struct Caller {
    seconds: f32,
    control: f32,
    skip: bool,
}
impl VimpActiveServices for Caller {
    fn session(
        &mut self,
        _: &Session,
        _: Id,
        _: Predicate,
    ) -> Result<crate::actor_session_predicates::Snapshot<'_>, String> {
        Ok(crate::actor_session_predicates::Snapshot {
            mode: 0,
            declared_count: 0,
            current_key: 0,
            slots: &[],
        })
    }
    fn seconds(&mut self, _: &Session, _: Id) -> Result<f64, String> {
        Ok(f64::from(self.seconds))
    }
    fn orient(&mut self, s: &mut Session, id: Id) -> Result<(), String> {
        let a = s.get(id).ok_or("captured owner absent")?;
        let v = a.vimp.unwrap();
        let f = a.construction_fields.as_ref().unwrap();
        let c = |i| f32::from_bits(f.configuration[i]);
        let out = motion::orient(
            motion::State {
                position: f.position,
                heading: f.angles[0],
                look_pitch: 0.,
                rates: [0., 0., 0., v.angular_speed],
                previous_rates: [0., 0., 0., v.previous_angular_speed],
                active: f.object_flags,
                personality: f.configuration[14],
                state: 21,
                context: f.dispatch_context,
                parent_present: false,
                controls: std::array::from_fn(|i| v.commands[i] as i8),
                target_heading: 0.,
                target_position: [0.; 2],
            },
            &motion::Environment {
                parameters: motion::Parameters {
                    acceleration: c(0),
                    speed: c(1),
                    braking: c(4),
                    attenuation: c(3),
                    turn_acceleration: c(5),
                    turn_speed: c(6),
                    turn_braking: c(7),
                    gravity: c(10),
                },
                global_seconds: self.seconds,
                areas: &[],
                control_factor: self.control,
                running_factor: 1.,
                skip_heading: self.skip,
                controlled_actor: false,
            },
        )
        .map_err(str::to_owned)?;
        let a = s.residents.get_mut(&id.serial).unwrap();
        a.construction_fields.as_mut().unwrap().angles[0] = out.heading;
        let v = a.vimp.as_mut().unwrap();
        v.angular_speed = out.rates[3];
        v.previous_angular_speed = out.previous_rates[3];
        Ok(())
    }
    fn action(
        &mut self,
        _: &mut Session,
        _: Id,
        _: crate::vimp_active_response::Action,
    ) -> Result<(), String> {
        Err("captured non-network cycle unexpectedly requires an action".into())
    }
}
#[test]
#[ignore = "requires GIANTS_NPC_ROOT and GIANTS_NPC_ACTIVE_CONTROL; private captured caller motion/clock/external RNG schedule, not whole-primary"]
fn source_captured_vimp_active_cycle_retains_session_outputs_and_random() {
    let path = std::env::var_os("GIANTS_NPC_ACTIVE_CONTROL").expect("GIANTS_NPC_ACTIVE_CONTROL");
    let rows: Vec<Vec<u32>> = std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|line| {
            line.split_whitespace()
                .map(|v| u32::from_str_radix(v, 16).unwrap())
                .collect()
        })
        .collect();
    assert!(!rows.is_empty());
    assert!(rows.iter().all(|r| r.len() == 270));
    let (mut s, _, g, _, children, _) = super::vimp_history_tests::source_births(true);
    let id = *children
        .iter()
        .find(|&&id| s.vimp(id).unwrap().born.unwrap().uid == rows[0][9 + 0x188 / 4])
        .unwrap();
    let mut random = crate::audio_parameters::Random(rows[0][0]);
    let mut cursor = rows[0][256];
    let mut stages = std::collections::BTreeSet::new();
    for (number, w) in rows.iter().enumerate() {
        assert!(w[256] >= cursor);
        for _ in cursor..w[256] {
            random.next_15();
        }
        assert_eq!(random.0, w[0]);
        let h = &w[9..202];
        let b = &w[202..256];
        let f = |at: usize| f32::from_bits(h[at / 4]);
        let a = s.residents.get_mut(&id.serial).unwrap();
        let v = a.vimp.as_mut().unwrap();
        let fields = a.construction_fields.as_mut().unwrap();
        // Only the first active entry is supplied. Later active outputs and the
        // timer stay owned; caller motion, commands, history and flags remain inputs.
        if number == 0 {
            fields.angles[0] = f(0xc8);
            v.angular_speed = f(0x114);
            v.born.as_mut().unwrap().delay = f(0xec);
            assert!(v.response_turn_timer.is_none());
            let body = a.seed.storage.data_mut().unwrap();
            for at in [0xb4, 0xb8, 0xb0, 0x74, 0xd4] {
                put(body, at, b[at / 4]);
            }
        } else {
            assert_eq!(fields.angles[0].to_bits(), h[0xc8 / 4]);
            assert_eq!(v.angular_speed.to_bits(), h[0x114 / 4]);
            assert_eq!(v.response_turn_timer.unwrap().to_bits(), h[0xf0 / 4]);
            assert_eq!(v.born.unwrap().delay.to_bits(), h[0xec / 4]);
        }
        fields.position = [f(0xa0), f(0xa4), f(0xa8)];
        fields.object_flags = h[0xd8 / 4];
        fields.dispatch_context = h[0x18c / 4];
        fields.configuration[14] = h[0x1c8 / 4];
        for i in [0, 1, 3, 4, 5, 6, 7, 10] {
            assert_eq!(fields.configuration[i], h[0x190 / 4 + i]);
        }
        v.previous_angular_speed = f(0x124);
        if number != 0 {
            assert_eq!(
                v.previous_angular_speed.to_bits(),
                v.angular_speed.to_bits()
            );
        }
        v.commands = [h[0x208 / 4].to_le_bytes(), h[0x20c / 4].to_le_bytes()]
            .concat()
            .try_into()
            .unwrap();
        let body = a.seed.storage.data_mut().unwrap();
        for at in [0x40, 0x44, 0x48] {
            put(body, at, b[at / 4]);
        }
        for at in [0xb4, 0xb8, 0xb0, 0x74, 0xd4] {
            assert_eq!(crate::vimp_wander::word(body, at), b[at / 4]);
        }
        stages.insert(b[0xb8 / 4]);
        for (i, at) in [0x980, 0x984, 0x988, 0x92c, 0x930].into_iter().enumerate() {
            assert_eq!(g.words[at / 4], w[4 + i]);
        }
        let mut caller = Caller {
            seconds: f32::from_bits(w[1]),
            control: f32::from_bits(w[2]),
            skip: w[3] != 0,
        };
        let seed = random.0;
        s.advance_vimp_active_response_offline(id, number as u64, &g, &mut random, &mut caller)
            .unwrap();
        let mut one = crate::audio_parameters::Random(seed);
        one.next_15();
        cursor = w[256]
            + if random.0 == seed {
                0
            } else {
                assert_eq!(random.0, one.0);
                1
            };
        let a = s.get(id).unwrap();
        let v = a.vimp.unwrap();
        let f = a.construction_fields.as_ref().unwrap();
        let body = a.seed.storage.data().unwrap();
        let word = |at| crate::vimp_wander::word(body, at);
        let actual = [
            random.0,
            f.angles[0].to_bits(),
            v.angular_speed.to_bits(),
            v.previous_angular_speed.to_bits(),
            word(0xb4),
            v.response_turn_timer.unwrap().to_bits(),
            v.born.unwrap().delay.to_bits(),
            word(0xb8),
            word(0xb0),
            word(0x74),
            word(0xd4),
            u32::from_le_bytes(v.commands[..4].try_into().unwrap()),
            u32::from_le_bytes(v.commands[4..].try_into().unwrap()),
        ];
        assert_eq!(actual, &w[257..], "captured active entry {number}");
    }
    assert_eq!(stages.into_iter().collect::<Vec<_>>(), [0, 1, 2, 3]);
    assert_eq!(s.vimp_active_steps(id), Some(rows.len() as u64));
    assert_eq!(s.vimp_frames(id), Some(0));
}
