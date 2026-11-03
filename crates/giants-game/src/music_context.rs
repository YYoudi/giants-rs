//! Contextual music-request kernel. Counts and actor/backend fields are explicit
//! inputs; collecting a live world and actual audio playback are separate.
#[derive(Clone, Copy, Debug)]
pub struct State {
    pub mode: i32,
    pub poll_delay: f32,
    pub ambient_delay: f32,
    pub peak_count: i32,
    /// Original per-thread RNG state supplied by the caller.
    pub random: u32,
}
#[derive(Clone, Copy)]
pub struct Parameters {
    pub player_linked_high: i32,
    pub nearby_threshold: i32,
    pub player_linked_low: i32,
    pub player_linked_peak: i32,
    pub ratio: f32,
    pub ambient_min: f32,
    pub ambient_max: f32,
}
#[derive(Clone, Copy)]
pub struct Input {
    pub actor_present: bool,
    pub stream_disabled: bool,
    pub dt: f32,
    pub backend_jumping: bool,
    pub backend_playing: bool,
    /// Native actor kind word: 1 Kabuto, 2 Mecc, 3 Reaper.
    pub actor_kind: u32,
    pub actor_value_1c0: f32,
    pub actor_threshold_278: f32,
    pub actor_mode_flag: bool,
    pub player_linked: i32,
    pub nearby_other: i32,
}
#[derive(Debug, PartialEq, Eq)]
pub enum Request {
    None,
    Stop,
    Song(&'static str),
}
const MECC: [&str; 18] = [
    "MSU", "MSB", "MSBEnd", "MLB", "MLBEnd", "MWB", "MWBEnd", "MDH", "MVY", "MLS", "MLSEnd", "MA1",
    "MA2", "MST", "MFF", "MFFEnd", "", "",
];
const KABUTO: [&str; 18] = [
    "KSU", "KB1", "KB1End", "KLB", "KLBEnd", "KWB", "KWBEnd", "KDH", "KVY", "KLS", "KLSEnd", "KA1",
    "KA2", "", "KFF", "KFFEnd", "KB2", "KB2End",
];
const REAPER: [&str; 18] = [
    "RSU", "RSB", "RSBEnd", "RLB", "RLBEnd", "RWB", "RWBEnd", "RDH", "RVY", "RLS", "RLSEnd", "RA1",
    "RA2", "RST", "RFF", "RFFEnd", "", "",
];
fn random(state: &mut State) -> u32 {
    state.random = state.random.wrapping_mul(214013).wrapping_add(2531011);
    (state.random >> 16) & 32767
}
fn percent(state: &mut State) -> u32 {
    (random(state) * 100) >> 15
}
fn interval(state: &mut State, p: Parameters) -> f32 {
    let value = f64::from(random(state)) * (f64::from(p.ambient_max) - f64::from(p.ambient_min));
    ((value * (1. / 32768.)) + f64::from(p.ambient_min)) as f32
}
fn action_mode(state: &mut State, kind: u32) -> i32 {
    let value = percent(state);
    if kind == 1 {
        if value < 33 {
            14
        } else if value < 66 {
            1
        } else {
            16
        }
    } else if value < 50 {
        14
    } else {
        1
    }
}
pub fn advance(state: &mut State, input: Input, p: Parameters) -> Result<Request, &'static str> {
    if !(0..=100).contains(&state.mode)
        || !input.dt.is_finite()
        || !(0. ..=1.).contains(&input.dt)
        || ![
            state.poll_delay,
            state.ambient_delay,
            input.actor_value_1c0,
            input.actor_threshold_278,
            p.ratio,
            p.ambient_min,
            p.ambient_max,
        ]
        .iter()
        .all(|v| v.is_finite())
        || p.ratio <= 0.
        || p.ambient_min < 0.
        || p.ambient_max < p.ambient_min
        || p.ambient_max > 3600.
        || [
            input.player_linked,
            input.nearby_other,
            p.player_linked_high,
            p.nearby_threshold,
            p.player_linked_low,
            p.player_linked_peak,
        ]
        .iter()
        .any(|v| !(0..=65535).contains(v))
    {
        return Err("music context outside supported finite profile");
    }
    if !input.actor_present || input.stream_disabled {
        return Ok(Request::None);
    }
    state.poll_delay -= input.dt;
    if input.backend_jumping || state.poll_delay > 0. {
        if state.mode == 100 {
            state.ambient_delay -= input.dt;
        }
        return Ok(Request::None);
    }
    state.poll_delay = 0.1;
    let old = state.mode;
    let mut next = old;
    let healthy = input.actor_threshold_278 < input.actor_value_1c0;
    let active = input.actor_mode_flag && matches!(input.actor_kind, 2 | 3);
    let count = input.player_linked;
    match old {
        0 => {
            if count > p.player_linked_high {
                next = 3;
                state.peak_count = count;
            } else if count >= p.player_linked_low {
                next = action_mode(state, input.actor_kind);
            }
        }
        1 | 14 | 16 => {
            if !input.backend_playing {
                next = 100;
            }
            if count > p.player_linked_high {
                next = 3;
                state.peak_count = count;
            } else if count == 0 {
                next = match old {
                    1 => 2,
                    14 => 15,
                    _ => 17,
                };
            }
        }
        2 | 4 | 7 | 8 | 15 | 17 => {
            if !input.backend_playing {
                next = 100;
            }
        }
        3 => {
            if !input.backend_playing {
                next = 100;
            }
            if !healthy {
                next = 4;
            } else if f64::from(p.ratio) * f64::from(p.player_linked_high) > f64::from(count) {
                next = 5;
            } else if count > p.player_linked_peak {
                next = 9;
            }
        }
        5 => {
            if !healthy {
                next = 7;
            } else if f64::from(count) / f64::from(p.ratio) <= f64::from(p.player_linked_high) {
                next = if count <= 0 { 8 } else { 5 };
            } else {
                next = 3;
                state.peak_count = count;
            }
        }
        9 => {
            if !input.backend_playing {
                next = 100;
            }
            if !healthy {
                next = 7;
            } else if count < state.peak_count {
                next = 3;
                state.peak_count = count;
            }
        }
        11 | 12 => {
            if !input.backend_playing {
                next = 100;
            } else if healthy {
                if count > p.player_linked_high {
                    next = 3;
                    state.peak_count = count;
                } else if count >= p.player_linked_low {
                    next = action_mode(state, input.actor_kind);
                }
            }
        }
        13 => {
            if !input.backend_playing {
                next = 100;
            } else if !active && healthy && count > p.player_linked_high {
                next = 3;
                state.peak_count = count;
            }
        }
        100 => {
            if !healthy {
                return Ok(Request::None);
            }
            if count > p.player_linked_high {
                state.ambient_delay = interval(state, p);
                next = 3;
                state.peak_count = count;
            } else if count >= p.player_linked_low {
                state.ambient_delay = interval(state, p);
                next = action_mode(state, input.actor_kind);
            } else if input.nearby_other >= p.nearby_threshold {
                state.ambient_delay = interval(state, p);
                next = 0;
            } else if active {
                state.ambient_delay = interval(state, p);
                next = 13;
            } else {
                state.ambient_delay -= input.dt;
                if state.ambient_delay < 0. {
                    state.ambient_delay = interval(state, p);
                    next = 11 + i32::from(percent(state) > 49);
                }
            }
        }
        _ => {}
    }
    if next == old {
        return Ok(Request::None);
    }
    state.poll_delay = 5.;
    state.mode = next;
    if next == 100 {
        return Ok(Request::Stop);
    }
    Ok(match input.actor_kind {
        1 => Request::Song(KABUTO[next as usize]),
        2 => Request::Song(MECC[next as usize]),
        3 => Request::Song(REAPER[next as usize]),
        _ => Request::None,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn no_actor_retains_state_and_requests_are_not_playback_success() {
        let mut state = State {
            mode: 100,
            poll_delay: 0.,
            ambient_delay: -1.,
            peak_count: 0,
            random: 1,
        };
        let mut input = Input {
            actor_present: false,
            stream_disabled: false,
            dt: 0.1,
            backend_jumping: false,
            backend_playing: false,
            actor_kind: 2,
            actor_value_1c0: 100.,
            actor_threshold_278: 0.,
            actor_mode_flag: false,
            player_linked: 0,
            nearby_other: 0,
        };
        let p = Parameters {
            player_linked_high: 4,
            nearby_threshold: 3,
            player_linked_low: 1,
            player_linked_peak: 8,
            ratio: 1.5,
            ambient_min: 1.,
            ambient_max: 3.,
        };
        assert_eq!(advance(&mut state, input, p), Ok(Request::None));
        assert_eq!(state.random, 1);
        assert_eq!(state.poll_delay, 0.);
        input.actor_present = true;
        input.player_linked = 5;
        assert_eq!(advance(&mut state, input, p), Ok(Request::Song("MLB")));
        assert_eq!(state.mode, 3);
        assert_eq!(state.poll_delay, 5.);
        input.backend_playing = false;
        input.dt = 1.;
        state.poll_delay = 0.;
        state.mode = 2;
        assert_eq!(advance(&mut state, input, p), Ok(Request::Stop));
    }
}
