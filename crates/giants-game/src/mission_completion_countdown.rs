//! Acquired countdown phase of mission completion, including temporary damage
//! suppression ownership. Outer admission, mission records, clock production and
//! the message/progression continuation remain required caller responsibilities.
//! This state is a supplied snapshot, not a native global initializer.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct State {
    pub remaining: f32,
    pub elapsed: f32,
    pub onset: f32,
    pub damage_suppression: u32,
    pub owns_damage_suppression: u32,
    pub phase_active: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Pending,
    /// The caller must continue the acquired message/progression phase.
    Expired,
}

impl State {
    /// Advance an already admitted completion-countdown phase in Double53.
    /// Each clock call is independent. Completed stores survive a later clock
    /// failure; the enclosing owner must quarantine a failed turn against replay.
    pub fn advance(
        &mut self,
        mut seconds: impl FnMut() -> Result<f64, String>,
    ) -> Result<Phase, String> {
        if ![self.remaining, self.elapsed, self.onset]
            .into_iter()
            .all(f32::is_finite)
        {
            return Err("completion countdown requires finite owned timer state".into());
        }
        self.phase_active = 1;
        let mut clock = || -> Result<f64, String> {
            let value = seconds()?;
            if !value.is_finite() || value < 0. {
                return Err("completion countdown requires finite nonnegative clock output".into());
            }
            Ok(value)
        };
        let remaining = f64::from(self.remaining) - clock()?;
        self.remaining = remaining as f32;
        // The native branch tests the wide result, after storing the rounded
        // single. A tiny positive remainder can store zero and still be pending.
        if remaining <= 0. {
            if self.owns_damage_suppression != 0 {
                self.damage_suppression = 0;
                self.owns_damage_suppression = 0;
            }
            return Ok(Phase::Expired);
        }
        if self.elapsed > 0. {
            self.elapsed = (f64::from(self.elapsed) + clock()?) as f32;
        } else if self.onset > self.remaining {
            self.elapsed = (f64::from(self.onset) - f64::from(self.remaining)) as f32;
        }
        if self.damage_suppression == 0 {
            self.damage_suppression = 1;
            self.owns_damage_suppression = 1;
        }
        Ok(Phase::Pending)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn initial() -> State {
        State {
            remaining: 1.,
            elapsed: 0.,
            onset: 2.,
            damage_suppression: 0,
            owns_damage_suppression: 0,
            phase_active: 0,
        }
    }

    #[test]
    fn temporary_owner_releases_only_its_own_suppression() {
        for existing in [0, 1, 7] {
            let mut state = State {
                damage_suppression: existing,
                ..initial()
            };
            assert_eq!(state.advance(|| Ok(0.25)).unwrap(), Phase::Pending);
            assert_eq!(state.phase_active, 1);
            assert_eq!(state.damage_suppression, existing.max(1));
            assert_eq!(state.owns_damage_suppression, u32::from(existing == 0));
            assert_eq!(state.elapsed, 1.25);
            assert_eq!(state.advance(|| Ok(1.)).unwrap(), Phase::Expired);
            assert_eq!(state.damage_suppression, existing);
            assert_eq!(state.owns_damage_suppression, 0);
        }
    }

    #[test]
    fn wide_expiry_and_rounded_onset_tests_remain_distinct() {
        let mut state = State {
            remaining: f32::from_bits(1),
            onset: 0.,
            ..initial()
        };
        let mut calls = 0;
        assert_eq!(
            state
                .advance(|| {
                    calls += 1;
                    Ok(f64::from(f32::from_bits(1)) / 2.)
                })
                .unwrap(),
            Phase::Pending
        );
        assert_eq!(state.remaining.to_bits(), 0);
        assert_eq!(calls, 1);
        assert_eq!(state.damage_suppression, 1);
        assert_eq!(state.advance(|| Ok(0.)).unwrap(), Phase::Expired);
        assert_eq!(state.damage_suppression, 0);
    }

    #[test]
    fn independent_second_clock_failure_keeps_the_completed_prefix() {
        let mut state = State {
            elapsed: 2.,
            ..initial()
        };
        let mut calls = 0;
        assert!(
            state
                .advance(|| {
                    calls += 1;
                    if calls == 1 {
                        Ok(0.25)
                    } else {
                        Err("clock unavailable".into())
                    }
                })
                .is_err()
        );
        assert_eq!(calls, 2);
        assert_eq!(state.remaining, 0.75);
        assert_eq!(state.elapsed, 2.);
        assert_eq!(state.phase_active, 1);
        assert_eq!(state.damage_suppression, 0);
        assert_eq!(state.owns_damage_suppression, 0);
        state.remaining = f32::NAN;
        let before = state.phase_active;
        assert!(
            state
                .advance(|| panic!("invalid state reached clock"))
                .is_err()
        );
        assert_eq!(state.phase_active, before);
    }
}
