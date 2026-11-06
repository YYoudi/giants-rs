//! Acquired scalar/mode and target-reference phase. Notifications stop at the
//! caller's transport boundary; animation, effect transitions and world dispatch
//! remain separate. Identity parameters can use the common ticket-owned actor Id.

#[derive(Clone, Copy, Debug)]
pub struct State<K> {
    pub flags: u32,
    pub duration: f32,
    pub mode: u8,
    pub scalar: f32,
    pub target: Option<K>,
    pub target_status: u32,
}
#[derive(Clone, Copy)]
pub struct Target<K> {
    pub identity: K,
    pub owner_word: u32,
}
pub struct Input<K> {
    pub control_flags: u32,
    pub delta: f32,
    pub target: Option<Target<K>>,
}
/// Supplied immutable source context for the acquired notification guard.
/// This models eligibility only, not peers, messages or a network connection.
pub struct NotificationContext<'a> {
    pub session_mode: u32,
    pub peer_ids: &'a [u32],
    pub local_peer: u32,
    pub owner_word: u32,
}
impl NotificationContext<'_> {
    pub fn eligible(&self) -> Result<bool, &'static str> {
        if self.session_mode != 4 {
            return Ok(false);
        }
        let count = i32::try_from(self.peer_ids.len())
            .map_err(|_| "notification peer count outside acquired signed profile")?;
        let slot = self
            .peer_ids
            .iter()
            .position(|&peer| peer == self.local_peer)
            .map(|i| i as i32)
            .unwrap_or(count - 1);
        // The source tests only the low byte of the XOR, including fallback -1.
        Ok(self.owner_word as u8 == slot as u8)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Notification {
    Target(u32),
    Mode(u8),
}

/// The observer sees the state at each original send boundary. Target changes
/// notify before publishing reference/status; mode changes publish the byte first.
/// Transport is infallible here, matching the ignored native return value.
pub fn advance<K: Copy + Eq>(
    state: &mut State<K>,
    input: Input<K>,
    context: &NotificationContext<'_>,
    mut notify: impl FnMut(Notification, &State<K>),
) -> Result<u8, &'static str> {
    let eligible = context.eligible()?;
    let mut mode = state.mode;
    let mut target = input.target;
    if input.control_flags & 0x2000 != 0 {
        mode ^= 1;
    }
    if state.flags & 0x80001 != 0x80001 || mode == 0 {
        state.duration = 0.;
        state.scalar = 0.;
        mode = 0;
    } else {
        if !state.duration.is_finite() || !input.delta.is_finite() {
            return Err("non-finite Kabuto scalar phase");
        }
        let duration = if input.control_flags & 4 != 0 {
            mode |= 2;
            let wide = f64::from(state.duration) + f64::from(input.delta);
            if wide > 2. { 2. } else { wide }
        } else {
            mode &= !2;
            let wide = f64::from(state.duration) - f64::from(input.delta);
            if wide < 0. { 0. } else { wide }
        } as f32;
        if !duration.is_finite() {
            return Err("Kabuto scalar phase overflow");
        }
        state.duration = duration;
        if duration <= 0. {
            target = None;
        }
        state.flags |= 0x100;
        state.scalar = (f64::from(duration) * 0.5) as f32;
    }
    let desired = if mode != 0 { target } else { None };
    if state.target != desired.as_ref().map(|t| t.identity) {
        if eligible {
            notify(
                Notification::Target(desired.as_ref().map_or(0, |t| t.owner_word)),
                state,
            );
        }
        state.target_status = 0;
        state.target = desired.map(|t| t.identity);
    }
    if state.mode != mode {
        state.mode = mode;
        if eligible {
            notify(Notification::Mode(mode), state);
        }
    }
    Ok(state.mode)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn state() -> State<u32> {
        State {
            flags: 0x80001,
            duration: 0.25,
            mode: 1,
            scalar: 0.,
            target: Some(1),
            target_status: 7,
        }
    }
    fn context() -> NotificationContext<'static> {
        NotificationContext {
            session_mode: 4,
            peer_ids: &[71],
            local_peer: 71,
            owner_word: 0x1200,
        }
    }
    #[test]
    fn notifications_observe_reference_before_publish_and_mode_after_publish() {
        let mut s = state();
        let mut events = Vec::new();
        advance(
            &mut s,
            Input {
                control_flags: 4,
                delta: 0.125,
                target: Some(Target {
                    identity: 2,
                    owner_word: 0x87654321,
                }),
            },
            &context(),
            |event, seen| events.push((event, *seen)),
        )
        .unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].0, Notification::Target(0x87654321));
        assert_eq!(
            (
                events[0].1.mode,
                events[0].1.target,
                events[0].1.target_status
            ),
            (1, Some(1), 7)
        );
        assert_eq!(events[0].1.scalar, 0.1875);
        assert_eq!(events[1].0, Notification::Mode(3));
        assert_eq!(
            (
                events[1].1.mode,
                events[1].1.target,
                events[1].1.target_status
            ),
            (3, Some(2), 0)
        );
    }
    #[test]
    fn release_can_zero_mode_while_retaining_positive_scalar() {
        let mut s = State {
            mode: 2,
            duration: 1.,
            ..state()
        };
        let mut events = Vec::new();
        let result = advance(
            &mut s,
            Input {
                control_flags: 0,
                delta: 0.25,
                target: Some(Target {
                    identity: 2,
                    owner_word: 99,
                }),
            },
            &context(),
            |event, _| events.push(event),
        )
        .unwrap();
        assert_eq!(result, 0);
        assert_eq!((s.duration, s.scalar), (0.75, 0.375));
        assert_eq!(s.target, None);
        assert_eq!(events, [Notification::Target(0), Notification::Mode(0)]);
    }
    #[test]
    fn inactive_reset_bypasses_invalid_time_but_active_refusal_is_atomic() {
        let mut s = State {
            flags: 0,
            duration: f32::NAN,
            ..state()
        };
        let input = || Input {
            control_flags: 4,
            delta: f32::NAN,
            target: None,
        };
        advance(&mut s, input(), &context(), |_, _| {}).unwrap();
        assert_eq!((s.mode, s.duration, s.scalar, s.target), (0, 0., 0., None));
        s = state();
        let before = s;
        assert!(
            advance(&mut s, input(), &context(), |_, _| panic!(
                "refusal must not notify"
            ))
            .is_err()
        );
        assert_eq!(
            (
                s.flags,
                s.mode,
                s.duration,
                s.scalar,
                s.target,
                s.target_status
            ),
            (
                before.flags,
                before.mode,
                before.duration,
                before.scalar,
                before.target,
                before.target_status
            )
        );
    }
    #[test]
    fn negative_zero_and_same_identity_preserve_source_details() {
        let mut s = State {
            duration: -0.,
            target: None,
            ..state()
        };
        advance(
            &mut s,
            Input {
                control_flags: 0,
                delta: 0.,
                target: Some(Target {
                    identity: 2,
                    owner_word: 99,
                }),
            },
            &context(),
            |_, _| panic!("no change"),
        )
        .unwrap();
        assert_eq!(s.duration.to_bits(), (-0.0f32).to_bits());
        assert_eq!(s.scalar.to_bits(), (-0.0f32).to_bits());
        s = state();
        advance(
            &mut s,
            Input {
                control_flags: 0,
                delta: 0.,
                target: Some(Target {
                    identity: 1,
                    owner_word: 99,
                }),
            },
            &context(),
            |_, _| panic!("identity unchanged"),
        )
        .unwrap();
        assert_eq!(s.target_status, 7);
    }
    #[test]
    fn guard_uses_first_match_fallback_and_only_low_byte() {
        assert!(
            NotificationContext {
                session_mode: 4,
                peer_ids: &[],
                local_peer: 4,
                owner_word: u32::MAX
            }
            .eligible()
            .unwrap()
        );
        let peers: Vec<u32> = (0..260).collect();
        assert!(
            NotificationContext {
                session_mode: 4,
                peer_ids: &peers,
                local_peer: 999,
                owner_word: 0x1203
            }
            .eligible()
            .unwrap()
        );
        for (owner_word, expected) in [(0x100, true), (2, false)] {
            assert_eq!(
                NotificationContext {
                    session_mode: 4,
                    peer_ids: &[7, 8, 7],
                    local_peer: 7,
                    owner_word
                }
                .eligible()
                .unwrap(),
                expected
            );
        }
        assert!(
            !NotificationContext {
                session_mode: 0,
                ..context()
            }
            .eligible()
            .unwrap()
        );
    }
}
