//! Vimp response dispatch with required session, target, random, vector and
//! action providers. This is not an escape-mode continuation or live admission.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct State<K> {
    /// Actual actor comparison word; it is not an owned actor identity.
    pub owner_word: u32,
    pub position: [f32; 3],
    pub forward: [f32; 3],
    pub physical_flags: u32,
    pub channel_flags: u32,
    pub priority: i32,
    pub flags: u32,
    pub binding_flags: u32,
    pub remaining_signal: f32,
    pub signal_origin: [f32; 3],
    pub linked_seconds: f32,
    pub linked: Option<K>,
    pub target: Option<K>,
    pub target_position: [f32; 3],
    pub backwards: u8,
    pub mode: u32,
    pub delay: f32,
    pub response_kind: u32,
    pub response_active: u32,
    pub response_stage: u32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    /// One float and one word, distinct from a byte-only target response.
    Nearby {
        value: f32,
        kind: u32,
    },
    Target {
        backwards: u8,
    },
}
#[derive(Clone, Copy, Debug)]
pub enum Predicate {
    First,
    Second,
}
pub trait Services<K> {
    /// Current producer-owned inputs for this exact query. The response service
    /// executes the acquired predicate; providers do not supply its decision.
    fn session(
        &mut self,
        state: &State<K>,
        query: Predicate,
    ) -> Result<crate::actor_session_predicates::Snapshot<'_>, String>;
    fn random_delay(&mut self, state: &State<K>) -> Result<f32, String>;
    fn target_position(&mut self, target: K) -> Result<[f32; 3], String>;
    /// Must execute the actual vector/route response, or fail. Its preceding
    /// priority, remaining-signal and flag writes have already occurred.
    fn vector_response(
        &mut self,
        state: &mut State<K>,
        origin: [f32; 3],
        fallback: f32,
    ) -> Result<(), String>;
    fn action(&mut self, state: &State<K>, action: Action) -> Result<(), String>;
}
fn predicate<K>(
    state: &State<K>,
    services: &mut impl Services<K>,
    query: Predicate,
) -> Result<bool, String> {
    let snapshot = services.session(state, query)?;
    match query {
        Predicate::First => crate::actor_session_predicates::first(&snapshot, state.owner_word),
        Predicate::Second => crate::actor_session_predicates::second(&snapshot, state.owner_word),
    }
}
#[derive(Debug)]
pub struct Outcome<K> {
    pub state: State<K>,
    pub handled: bool,
}
#[derive(Debug)]
pub struct Failure<K> {
    pub state: State<K>,
    pub message: String,
}

pub fn random_delay(
    random: &mut crate::audio_parameters::Random,
    low: f32,
    high: f32,
) -> Result<f32, String> {
    if !low.is_finite() || !high.is_finite() {
        return Err("Vimp response delay interval outside finite profile".into());
    }
    // This helper retains the interval subtraction wide through the random
    // scaling; it does not use the constructor's rounded range temporary.
    Ok(
        (f64::from(random.next_15()) * (f64::from(high) - f64::from(low)) / 32768. + f64::from(low))
            as f32,
    )
}

fn normalize(v: [f32; 3]) -> Result<[f32; 3], String> {
    if !v.into_iter().all(f32::is_finite) {
        return Err("Vimp response vector outside finite profile".into());
    }
    let [x, y, z] = v.map(f64::from);
    let squared = (x * x + y * y) + z * z;
    if squared < f64::from(0.0001_f32) {
        return Ok([0.; 3]);
    }
    let reciprocal = 1. / squared.sqrt();
    Ok(v.map(|v| (f64::from(v) * reciprocal) as f32))
}
pub fn backwards(forward: [f32; 3], position: [f32; 3], target: [f32; 3]) -> Result<u8, String> {
    let forward = normalize(forward)?;
    let delta = std::array::from_fn(|i| (f64::from(position[i]) - f64::from(target[i])) as f32);
    let delta = normalize(delta)?;
    // The actual dot helper adds Z then Y then X; preserve cancellation order.
    let dot = (f64::from(forward[2]) * f64::from(delta[2])
        + f64::from(forward[1]) * f64::from(delta[1]))
        + f64::from(forward[0]) * f64::from(delta[0]);
    Ok(u8::from(dot < 0.))
}
pub fn respond<K: Copy>(
    mut state: State<K>,
    noise_position: [f32; 3],
    services: &mut impl Services<K>,
) -> Result<Outcome<K>, Failure<K>> {
    let result = (|| -> Result<bool, String> {
        if !predicate(&state, services, Predicate::First)?
            || state.physical_flags & 0x40 != 0
            || state.channel_flags & 2 == 0
        {
            return Ok(false);
        }
        if state.priority < 5 {
            if state.flags & 0x20 != 0 {
                state.priority = 5;
                state.remaining_signal = 0.;
                state.flags = 0;
                let origin = state.signal_origin;
                services.vector_response(&mut state, origin, -1.)?;
                return Ok(true);
            }
            if state.priority < 4 {
                let kind = if state.flags & 0x10 != 0 {
                    Some((4, 2))
                } else if state.priority < 3 && state.flags & 8 != 0 {
                    Some((3, 1))
                } else {
                    None
                };
                if let Some((priority, kind)) = kind {
                    state.priority = priority;
                    state.flags = 0;
                    state.binding_flags |= 0x400;
                    state.response_kind = kind;
                    state.response_active = 1;
                    state.response_stage = 3;
                    if !predicate(&state, services, Predicate::First)? {
                        state.delay = 0.;
                    } else {
                        state.delay = services.random_delay(&state)?;
                        if predicate(&state, services, Predicate::Second)? {
                            services.action(&state, Action::Nearby { value: 0., kind })?;
                        }
                    }
                    return Ok(true);
                }
                if state.priority < 2 {
                    let linked = state.flags & 4 != 0 && state.linked.is_some();
                    let noise = state.priority < 1 && state.flags & 2 != 0;
                    if linked || noise {
                        state.priority = if linked { 2 } else { 1 };
                        state.flags = 0;
                        state.linked_seconds = 0.;
                        state.target = if linked { state.linked } else { None };
                        if !predicate(&state, services, Predicate::First)? {
                            state.backwards = 0;
                        } else {
                            state.target_position = if let Some(target) = state.target {
                                services.target_position(target)?
                            } else {
                                noise_position
                            };
                            state.backwards =
                                backwards(state.forward, state.position, state.target_position)?;
                        }
                        state.mode = 7;
                        if predicate(&state, services, Predicate::Second)? {
                            services.action(
                                &state,
                                Action::Target {
                                    backwards: state.backwards,
                                },
                            )?;
                        }
                        return Ok(true);
                    }
                }
            }
        }
        state.flags = 0;
        Ok(false)
    })();
    match result {
        Ok(handled) => Ok(Outcome { state, handled }),
        Err(message) => Err(Failure { state, message }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Required {
        calls: Vec<(&'static str, i32)>,
        random: crate::audio_parameters::Random,
    }
    impl Services<u32> for Required {
        fn session(
            &mut self,
            s: &State<u32>,
            query: Predicate,
        ) -> Result<crate::actor_session_predicates::Snapshot<'_>, String> {
            self.calls.push((
                match query {
                    Predicate::First => "first",
                    Predicate::Second => "second",
                },
                s.priority,
            ));
            Ok(crate::actor_session_predicates::Snapshot {
                mode: 4,
                declared_count: 1,
                current_key: 17,
                slots: &[17],
            })
        }
        fn random_delay(&mut self, s: &State<u32>) -> Result<f32, String> {
            self.calls.push(("delay", s.priority));
            random_delay(&mut self.random, 1.5, 5.)
        }
        fn target_position(&mut self, _: u32) -> Result<[f32; 3], String> {
            Err("required target owner missing".into())
        }
        fn vector_response(
            &mut self,
            s: &mut State<u32>,
            _: [f32; 3],
            _: f32,
        ) -> Result<(), String> {
            self.calls.push(("vector", s.priority));
            Err("required vector service missing".into())
        }
        fn action(&mut self, s: &State<u32>, _: Action) -> Result<(), String> {
            self.calls.push(("action", s.priority));
            Err("required transport failed".into())
        }
    }
    fn state(flags: u32) -> State<u32> {
        State {
            owner_word: 0,
            position: [0.; 3],
            forward: [1., 0., 0.],
            physical_flags: 0,
            channel_flags: 2,
            priority: 0,
            flags,
            binding_flags: 0,
            remaining_signal: 3.,
            signal_origin: [0.; 3],
            linked_seconds: 2.,
            linked: Some(7),
            target: None,
            target_position: [8.; 3],
            backwards: 0xaa,
            mode: 1,
            delay: 0.75,
            response_kind: 0,
            response_active: 0,
            response_stage: 0,
        }
    }
    fn providers() -> Required {
        Required {
            calls: Vec::new(),
            random: crate::audio_parameters::Random(1),
        }
    }
    #[test]
    fn highest_priority_keeps_prefix_writes_when_vector_service_is_missing() {
        let mut p = providers();
        let e = respond(state(0x3e), [0.; 3], &mut p).unwrap_err();
        assert_eq!(e.state.priority, 5);
        assert_eq!(e.state.flags, 0);
        assert_eq!(e.state.remaining_signal, 0.);
        assert_eq!(e.state.mode, 1);
        assert_eq!(p.random.0, 1);
        assert_eq!(p.calls, [("first", 0), ("vector", 5)]);
    }
    #[test]
    fn action_failure_keeps_random_progress_and_predicates_observe_preceding_stores() {
        let mut p = providers();
        let e = respond(state(8), [0.; 3], &mut p).unwrap_err();
        assert_eq!(e.state.priority, 3);
        assert_eq!(e.state.response_kind, 1);
        assert_eq!(e.state.response_active, 1);
        assert_eq!(e.state.response_stage, 3);
        assert_eq!(e.state.flags, 0);
        assert_eq!(e.state.binding_flags, 0x400);
        assert_ne!(e.state.delay, 0.75);
        assert_ne!(p.random.0, 1);
        assert_eq!(
            p.calls,
            [
                ("first", 0),
                ("first", 3),
                ("delay", 3),
                ("second", 3),
                ("action", 3)
            ]
        );
    }
    #[test]
    fn failed_target_lookup_preserves_identity_before_mode_and_orientation_writes() {
        let mut p = providers();
        let e = respond(state(4), [0.; 3], &mut p).unwrap_err();
        assert_eq!(e.state.target, Some(7));
        assert_eq!(e.state.priority, 2);
        assert_eq!(e.state.linked_seconds, 0.);
        assert_eq!(e.state.mode, 1);
        assert_eq!(e.state.backwards, 0xaa);
        assert_eq!(e.state.target_position, [8.; 3]);
        assert_eq!(p.random.0, 1);
    }
    #[test]
    fn unready_physics_or_animation_keeps_pending_flags_and_does_not_clear_them() {
        for (physical_flags, channel_flags) in [(0x40, 2), (0, 0)] {
            let mut s = state(0x3e);
            s.physical_flags = physical_flags;
            s.channel_flags = channel_flags;
            let mut p = providers();
            let o = respond(s, [0.; 3], &mut p).unwrap();
            assert_eq!(o.state, s);
            assert!(!o.handled);
            assert_eq!(p.calls, [("first", 0)]);
        }
    }
    #[test]
    fn equal_bounds_still_consume_random_and_wide_intervals_remain_finite() {
        let mut random = crate::audio_parameters::Random(1);
        assert_eq!(random_delay(&mut random, 1.5, 1.5).unwrap(), 1.5);
        assert_ne!(random.0, 1);
        let previous = random.0;
        assert!(
            random_delay(&mut random, -f32::MAX, f32::MAX)
                .unwrap()
                .is_finite()
        );
        assert_ne!(random.0, previous);
        let previous = random.0;
        assert!(random_delay(&mut random, f32::NAN, 1.).is_err());
        assert_eq!(random.0, previous);
    }
}
