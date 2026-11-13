//! Acquired offline, nonlethal positive-hit voice tail (kinds0/2/16).
//! Health is committed by
//! the caller before entry. A required cue is an external service, not playback.
use crate::audio_parameters::Random;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct State {
    pub damage: f32,
    pub limit: f32,
    pub pain_timer: f32,
    pub shared_timer: f32,
}

/// Execute only after positive effective damage. The callback observes the
/// actual pre-cue timers. Its failure preserves preceding VM draws and writes;
/// the caller must retain this state and quarantine its enclosing transaction.
/// Audio variation has its own RNG and must not consume this VM stream.
pub fn after_damage(
    state: &mut State,
    random: &mut Random,
    mut play: impl FnMut(u32, &State, &Random) -> Result<(), String>,
) -> Result<(), String> {
    if ![
        state.damage,
        state.limit,
        state.pain_timer,
        state.shared_timer,
    ]
    .into_iter()
    .all(f32::is_finite)
        || state.limit <= 0.
        || state.damage < 0.
        || state.damage >= state.limit
    {
        return Err("impact voice requires acquired finite nonlethal health and timers".into());
    }
    if state.pain_timer < 0. {
        let sound = if f64::from(state.damage) / f64::from(state.limit) > 0.5 {
            1505
        } else {
            1504
        };
        play(sound, state, random)?;
        state.pain_timer = (f64::from(random.next_15()) / 32768. + 1.) as f32;
    }
    if state.shared_timer < 0. {
        let choice = (random.next_15() * 100) >> 15;
        let sound = 1683 + choice / 25;
        play(sound, state, random)?;
        state.shared_timer = 10.;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn state() -> State {
        State {
            damage: 50.,
            limit: 100.,
            pain_timer: -0.1,
            shared_timer: -0.1,
        }
    }
    #[test]
    fn strict_timer_and_health_boundaries_preserve_service_order() {
        for (damage, pain) in [(50., 1504), (f32::from_bits(50f32.to_bits() + 1), 1505)] {
            let mut state = State { damage, ..state() };
            let mut random = Random(1);
            let mut expected = random;
            let timer = (f64::from(expected.next_15()) / 32768. + 1.) as f32;
            let ambient = 1683 + ((expected.next_15() * 100) >> 15) / 25;
            let mut seen = Vec::new();
            after_damage(&mut state, &mut random, |cue, observed, _| {
                seen.push((cue, observed.pain_timer, observed.shared_timer));
                Ok(())
            })
            .unwrap();
            assert_eq!(seen, [(pain, -0.1, -0.1), (ambient, timer, -0.1)]);
            assert_eq!(state.shared_timer, 10.);
            assert_eq!(random.0, expected.0);
        }
        let mut state = State {
            pain_timer: -0.,
            shared_timer: 0.,
            ..state()
        };
        let mut random = Random(42);
        after_damage(&mut state, &mut random, |_, _, _| {
            panic!("zero timer is not expired")
        })
        .unwrap();
        assert_eq!(random.0, 42);
        assert_eq!(state.pain_timer.to_bits(), (-0f32).to_bits());
    }
    #[test]
    fn late_cue_failure_keeps_only_completed_writes_and_draws() {
        let mut state = state();
        let before = state;
        let mut random = Random(1);
        assert!(after_damage(&mut state, &mut random, |_, _, _| Err("no device".into())).is_err());
        assert_eq!(state, before);
        assert_eq!(random.0, 1);
        let mut count = 0;
        assert!(
            after_damage(&mut state, &mut random, |_, _, _| {
                count += 1;
                if count == 2 {
                    Err("ambient failure".into())
                } else {
                    Ok(())
                }
            })
            .is_err()
        );
        let mut expected = Random(1);
        let timer = (f64::from(expected.next_15()) / 32768. + 1.) as f32;
        expected.next_15();
        assert_eq!(state.pain_timer, timer);
        assert_eq!(state.shared_timer, -0.1);
        assert_eq!(random.0, expected.0);
    }
}
