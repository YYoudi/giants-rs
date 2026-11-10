//! Acquired Vimp threat consideration. This accumulates signals; it does not
//! select an escape mode, play a sound or admit an actor to the live world.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct State {
    pub position: [f32; 3],
    pub priority: i32,
    pub flags: u32,
    pub remaining_signal: f32,
    pub linked_seconds: f32,
    /// Presence only; this helper never dereferences the linked actor.
    pub linked_present: bool,
    pub heard_at: f32,
    pub noise_level: f32,
}
#[derive(Clone, Copy)]
pub struct Signals {
    pub now: f32,
    pub noise_at: f32,
    pub noise_position: [f32; 3],
}
#[derive(Clone, Copy)]
pub struct Parameters {
    pub nearby_radius: f32,
    pub peer_radius: f32,
    pub noise_radius: f32,
    pub noise_threshold: f32,
}
#[derive(Clone, Copy)]
pub struct Peer {
    pub mode: u32,
    pub position: [f32; 3],
}
/// Ordered, lazily read list11 and list10 observations. Providers must resolve
/// actual ownership; end-of-list is distinct from a failed or stale lookup.
pub trait Sources {
    fn nearby(&mut self, index: usize) -> Result<Option<[f32; 3]>, String>;
    fn peer(&mut self, index: usize) -> Result<Option<Peer>, String>;
}
#[derive(Clone, Copy, Debug)]
pub struct Outcome {
    pub state: State,
    /// New evidence in this call, independent of already accumulated flag bits.
    pub considered: bool,
}
#[derive(Debug)]
pub struct Failure {
    pub state: State,
    pub message: String,
}
fn finite(value: f32) -> Result<(), String> {
    if value.is_finite() {
        Ok(())
    } else {
        Err("Vimp threat scalar outside acquired finite profile".into())
    }
}
fn near(a: [f32; 3], b: [f32; 3], radius: f32) -> Result<bool, String> {
    for v in [a[0], a[1], b[0], b[1], radius] {
        finite(v)?;
    }
    let x = f64::from(b[0]) - f64::from(a[0]);
    let y = f64::from(b[1]) - f64::from(a[1]);
    Ok(x * x + y * y < f64::from(radius) * f64::from(radius))
}
pub fn consider(
    mut state: State,
    signals: Signals,
    parameters: Parameters,
    sources: &mut impl Sources,
) -> Result<Outcome, Failure> {
    let result = (|| -> Result<bool, String> {
        let mut considered = false;
        if state.priority >= 5 {
            return Ok(false);
        }
        finite(state.remaining_signal)?;
        if state.remaining_signal > 0. {
            state.flags |= 0x20;
            considered = true;
        }
        if state.priority >= 4 {
            return Ok(considered);
        }
        let mut index = 0;
        while let Some(position) = sources.nearby(index)? {
            if near(state.position, position, parameters.nearby_radius)? {
                state.flags |= 0x10;
                considered = true;
                break;
            }
            index += 1;
        }
        if state.priority >= 3 {
            return Ok(considered);
        }
        let mut index = 0;
        while let Some(peer) = sources.peer(index)? {
            if matches!(peer.mode, 2..=4)
                && near(state.position, peer.position, parameters.peer_radius)?
            {
                state.flags |= 8;
                considered = true;
                break;
            }
            index += 1;
        }
        if state.priority >= 2 {
            return Ok(considered);
        }
        finite(state.linked_seconds)?;
        if state.linked_seconds > 0. && state.linked_present {
            state.flags |= 4;
            considered = true;
        }
        if state.priority >= 1 {
            return Ok(considered);
        }
        finite(signals.now)?;
        finite(signals.noise_at)?;
        if f64::from(signals.now) - f64::from(signals.noise_at) > 1. {
            return Ok(considered);
        }
        finite(state.heard_at)?;
        if state.heard_at == signals.noise_at {
            return Ok(considered);
        }
        state.heard_at = signals.noise_at;
        if near(
            state.position,
            signals.noise_position,
            parameters.noise_radius,
        )? {
            finite(state.noise_level)?;
            finite(parameters.noise_threshold)?;
            let level = f64::from(state.noise_level) + 1.;
            state.noise_level = level as f32;
            if level >= f64::from(parameters.noise_threshold) {
                state.flags |= 2;
                considered = true;
            }
        }
        Ok(considered)
    })();
    match result {
        Ok(considered) => Ok(Outcome { state, considered }),
        Err(message) => Err(Failure { state, message }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Lists {
        fail: bool,
    }
    impl Sources for Lists {
        fn nearby(&mut self, _: usize) -> Result<Option<[f32; 3]>, String> {
            if self.fail {
                Err("stale nearby owner".into())
            } else {
                Ok(None)
            }
        }
        fn peer(&mut self, _: usize) -> Result<Option<Peer>, String> {
            if self.fail {
                panic!("peer query after failed nearby query")
            } else {
                Ok(None)
            }
        }
    }
    fn state() -> State {
        State {
            position: [0.; 3],
            priority: 0,
            flags: 0x8000_0000,
            remaining_signal: 0.,
            linked_seconds: 0.,
            linked_present: false,
            heard_at: -1.,
            noise_level: 0.,
        }
    }
    fn signals() -> Signals {
        Signals {
            now: 1.,
            noise_at: 0.,
            noise_position: [0.; 3],
        }
    }
    fn parameters() -> Parameters {
        Parameters {
            nearby_radius: 1.,
            peer_radius: 1.,
            noise_radius: 1.,
            noise_threshold: 1.,
        }
    }
    #[test]
    fn stale_list_retains_earlier_signal_but_priority_can_skip_lookup() {
        let mut s = state();
        s.remaining_signal = 1.;
        let e = consider(s, signals(), parameters(), &mut Lists { fail: true }).unwrap_err();
        assert_eq!(e.state.flags, 0x8000_0020);
        assert_eq!(e.state.heard_at, -1.);
        s.priority = 4;
        assert!(
            consider(s, signals(), parameters(), &mut Lists { fail: true })
                .unwrap()
                .considered
        );
        s.priority = 5;
        s.flags |= 0x3e;
        let o = consider(s, signals(), parameters(), &mut Lists { fail: true }).unwrap();
        assert!(!o.considered);
        assert_eq!(o.state, s);
    }
    #[test]
    fn rounded_noise_store_does_not_replace_wide_threshold_comparison() {
        let mut s = state();
        s.noise_level = 3. * 2_f32.powi(-25);
        let threshold = f32::from_bits(1_f32.to_bits() + 1);
        let p = Parameters {
            noise_threshold: threshold,
            ..parameters()
        };
        let o = consider(s, signals(), p, &mut Lists { fail: false }).unwrap();
        assert_eq!(o.state.noise_level, threshold);
        assert_eq!(o.state.heard_at, 0.);
        assert!(!o.considered);
        let again = consider(o.state, signals(), p, &mut Lists { fail: false }).unwrap();
        assert_eq!(again.state, o.state);
        assert!(!again.considered);
    }
    #[test]
    fn unread_z_is_ignored_and_stamp_survives_late_coordinate_refusal() {
        let mut s = state();
        s.position[2] = f32::NAN;
        let o = consider(s, signals(), parameters(), &mut Lists { fail: false }).unwrap();
        assert!(o.considered);
        assert_eq!(o.state.flags, 0x8000_0002);
        s.position[0] = f32::NAN;
        let e = consider(s, signals(), parameters(), &mut Lists { fail: false }).unwrap_err();
        assert_eq!(e.state.heard_at, 0.);
        assert_eq!(e.state.noise_level, 0.);
    }
}
