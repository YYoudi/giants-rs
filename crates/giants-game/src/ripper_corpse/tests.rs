use super::*;

fn corpse() -> State<u32> {
    State {
        id: 1,
        class: 230,
        limit: 100.,
        damage: 100.,
        mode: 94,
        action: 0,
        flags: 0x4000_0010,
        personality: 0xe00,
        control: 0,
        timer: 1.,
        position: [0.; 3],
        parent: None,
        owner: None,
        retaliation: None,
        target: None,
        group: None,
        group_index: -1,
        effect: None,
    }
}

struct Mana {
    calls: u32,
    accepted: bool,
    fail: bool,
}
impl Services<u32> for Mana {
    fn create_mana(&mut self, _: &State<u32>, _: f32, random: &mut Random) -> Result<bool, String> {
        self.calls += 1;
        if self.fail {
            random.next_15();
            Err("actual factory failed after an external draw".into())
        } else {
            Ok(self.accepted)
        }
    }
}
const PARAMETERS: Parameters = Parameters {
    large_mana: 3.,
    medium_mana: 2.,
};

#[test]
fn missing_creation_retains_consumed_clock_and_external_rng_without_marking() {
    let mut state = corpse();
    let mut random = Random(17);
    let mut expected = random;
    expected.next_15();
    let mut factory = Mana {
        calls: 0,
        accepted: false,
        fail: true,
    };
    assert!(ripper_death_logic(&mut state, 0.25, PARAMETERS, &mut random, &mut factory).is_err());
    assert_eq!(state.timer, 0.75);
    assert_eq!(state.flags, 0x4000_0010);
    assert_eq!(random.0, expected.0);
    assert_eq!(factory.calls, 1);
}

#[test]
fn living_invalid_or_uncompleted_death_cannot_consume_clock_or_request_creation() {
    for change in 0..5 {
        let mut state = corpse();
        match change {
            0 => state.damage = 99.,
            1 => state.mode = 68,
            2 => state.flags &= !0x10,
            3 => state.class = 21,
            _ => state.limit = f32::NAN,
        }
        let mut random = Random(17);
        let mut factory = Mana {
            calls: 0,
            accepted: true,
            fail: false,
        };
        assert!(
            ripper_death_logic(&mut state, 0.25, PARAMETERS, &mut random, &mut factory).is_err()
        );
        assert_eq!(state.timer, 1.);
        assert_eq!(factory.calls, 0);
        assert_eq!(random.0, 17);
    }
}

#[test]
#[ignore = "requires GIANTS_RIPPER_CORPSE_CONTROL: private original whole-helper observations; supplied corpse/global clock and original empty lookup or supplied creation return, no positive native creation, primary, GPU or lifecycle proof"]
fn original_corpse_helper_observations() {
    let path = std::env::var("GIANTS_RIPPER_CORPSE_CONTROL").unwrap();
    let source = std::fs::read_to_string(path).unwrap();
    let mut cases = 0;
    for line in source.lines().filter(|line| !line.trim().is_empty()) {
        let words: Vec<u32> = line
            .split_whitespace()
            .map(|w| u32::from_str_radix(w, 16).unwrap())
            .collect();
        assert_eq!(words.len(), 10);
        let mut state = corpse();
        state.class = words[0] as i32;
        state.timer = f32::from_bits(words[1]);
        state.personality = words[3];
        let amount = f32::from_bits(words[4]);
        let parameters = Parameters {
            large_mana: if state.class == 230 { amount } else { 42. },
            medium_mana: if state.class == 73 { amount } else { 43. },
        };
        let mut factory = Mana {
            calls: 0,
            accepted: words[5] != 0,
            fail: false,
        };
        let result = ripper_death_logic(
            &mut state,
            f32::from_bits(words[2]),
            parameters,
            &mut Random(17),
            &mut factory,
        )
        .unwrap();
        assert_eq!(
            [
                state.timer.to_bits(),
                state.flags,
                factory.calls,
                u32::from(result == Outcome::Marked),
            ],
            words[6..],
            "case {cases}"
        );
        cases += 1;
    }
    assert_eq!(cases, 540);
}
