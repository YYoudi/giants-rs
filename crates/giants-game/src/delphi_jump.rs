//! Acquired Delphi jump/hold/release service, not a complete player turn.
//! Contact, rates, orientation and context producers remain separate phases.
use crate::object_time::{self, Sample, SlowArea};
use giants_formats::global_data::GlobalData;

#[derive(Clone, Copy, Debug)]
pub struct Parameters {
    pub impulse: f32,
    pub held_acceleration: f32,
    pub hold_seconds: f32,
}
impl Parameters {
    pub fn from_globals(g: &GlobalData) -> Result<Self, String> {
        Ok(Self {
            impulse: g.scalar(0x7fc).map_err(|e| e.to_string())?,
            held_acceleration: g.scalar(0x800).map_err(|e| e.to_string())?,
            hold_seconds: g.scalar(0x804).map_err(|e| e.to_string())?,
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct State {
    pub mode: i32,
    pub active: u32,
    pub response: u32,
    pub controls: u32,
    pub horizontal_rates: [f32; 2],
    pub pitch_roll: [f32; 2],
    pub position: [f32; 3],
    pub vertical_speed: f32,
    pub hold_seconds: f32,
    pub stored_height: f32,
    /// Two acquired status words; their live producer is not supplied here.
    pub context: [u32; 2],
    pub launch_reset: u32,
    pub physical_request: f32,
    pub physical_acceleration_z: f32,
}
pub struct Tick<'a> {
    pub held: bool,
    /// The acquired outer gate blocks launch, not an already active hold.
    pub launch_blocked: bool,
    pub global_seconds: f32,
    pub owner: u32,
    pub areas: &'a [SlowArea],
}

/// Advances only this service. No gravity, displacement or landing is invented.
/// All mutations are returned together; refused profiles leave the input intact.
pub fn step(mut state: State, parameters: Parameters, tick: Tick<'_>) -> Result<State, String> {
    let finite = state
        .horizontal_rates
        .into_iter()
        .chain(state.pitch_roll)
        .chain(state.position)
        .chain([
            state.vertical_speed,
            state.hold_seconds,
            state.stored_height,
            state.physical_request,
            state.physical_acceleration_z,
            parameters.impulse,
            parameters.held_acceleration,
            parameters.hold_seconds,
            tick.global_seconds,
        ])
        .all(f32::is_finite);
    if !finite
        || tick.global_seconds < 0.
        || tick.areas.iter().any(|a| {
            !a.center
                .into_iter()
                .chain([a.radius_squared, a.factor])
                .all(f32::is_finite)
                || a.radius_squared < 0.
                || a.factor < 0.
        })
    {
        return Err("Delphi jump requires the acquired finite nonnegative-time profile".into());
    }
    if !matches!(state.mode, 0x21 | 0x22) {
        if state.active & 0x100 == 0 {
            if state.active & 0x80000 != 0 {
                if state.response & 0x100000 == 0 {
                    if tick.held && !tick.launch_blocked {
                        if (state.context[0] == 0
                            || state.context[1] != 0
                            || state.horizontal_rates[0] < 0.
                            || (state.horizontal_rates[0] == 0. && state.horizontal_rates[1] == 0.))
                            && state.pitch_roll == [0.; 2]
                        {
                            state.vertical_speed = (f64::from(parameters.impulse)
                                + f64::from(state.vertical_speed))
                                as f32;
                        }
                        state.active |= 0x100;
                        state.hold_seconds = parameters.hold_seconds;
                        state.launch_reset = 0;
                        state.response |= 0x100000;
                        state.controls |= 4;
                    }
                } else if !tick.held {
                    state.response &= !0x100000;
                }
            }
        } else if tick.held && state.hold_seconds > 0. {
            if state.context[0] == 0 || state.context[1] != 0 {
                state.physical_request =
                    crate::physical_request::reduce(state.physical_request, state.hold_seconds)?;
                state.physical_acceleration_z = (f64::from(parameters.held_acceleration)
                    + f64::from(state.physical_acceleration_z))
                    as f32;
            }
            let seconds = object_time::seconds(
                tick.global_seconds,
                Some(Sample {
                    owner: tick.owner,
                    position: state.position,
                }),
                None,
                tick.areas,
            );
            state.hold_seconds = (f64::from(state.hold_seconds) - seconds) as f32;
        } else if state.controls & 4 != 0 && state.vertical_speed <= 0. {
            state.stored_height = state.position[2];
            state.active &= !0x100;
            state.controls = state.controls & !4 | 0x800;
        }
    }
    if ![
        state.vertical_speed,
        state.hold_seconds,
        state.physical_acceleration_z,
    ]
    .into_iter()
    .all(f32::is_finite)
    {
        return Err("Delphi jump arithmetic overflow".into());
    }
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> State {
        State {
            mode: 0,
            active: 0x80000,
            response: 0,
            controls: 0,
            horizontal_rates: [1., 0.],
            pitch_roll: [0.; 2],
            position: [0., 0., 7.],
            vertical_speed: 0.,
            hold_seconds: 0.,
            stored_height: -1.,
            context: [1, 0],
            launch_reset: 99,
            physical_request: 1.,
            physical_acceleration_z: 0.,
        }
    }
    fn parameters() -> Parameters {
        Parameters {
            impulse: 10.,
            held_acceleration: 2.,
            hold_seconds: 0.5,
        }
    }
    fn tick(held: bool) -> Tick<'static> {
        Tick {
            held,
            launch_blocked: false,
            global_seconds: 0.125,
            owner: 0,
            areas: &[],
        }
    }
    #[test]
    fn impulse_gate_does_not_cancel_latch_hold_or_release_transitions() {
        let mut state = step(fixture(), parameters(), tick(true)).unwrap();
        assert_eq!(state.vertical_speed, 0.);
        assert_eq!(state.active & 0x100, 0x100);
        assert_eq!(state.response & 0x100000, 0x100000);
        assert_eq!(state.launch_reset, 0);
        let mut held = tick(true);
        held.launch_blocked = true;
        state = step(state, parameters(), held).unwrap();
        assert_eq!(state.hold_seconds, 0.375);
        assert_eq!(state.physical_acceleration_z, 0.);
        state.context = [0; 2];
        state = step(state, parameters(), tick(true)).unwrap();
        assert_eq!(state.hold_seconds, 0.25);
        assert_eq!(state.physical_request, 0.375);
        assert_eq!(state.physical_acceleration_z, 2.);
        state = step(state, parameters(), tick(false)).unwrap();
        assert_eq!(state.active & 0x100, 0);
        assert_eq!(state.controls & 0x804, 0x800);
        assert_eq!(state.stored_height, 7.);
        assert_eq!(state.response & 0x100000, 0x100000);
        state = step(state, parameters(), tick(false)).unwrap();
        assert_eq!(state.response & 0x100000, 0);
    }
    #[test]
    fn refused_clock_or_state_is_atomic() {
        let state = fixture();
        let mut time = tick(true);
        time.global_seconds = -1.;
        assert!(step(state, parameters(), time).is_err());
        assert_eq!(state, fixture());
        let mut bad = state;
        bad.vertical_speed = f32::NAN;
        assert!(step(bad, parameters(), tick(false)).is_err());
        let mut blocked = state;
        blocked.mode = 0x21;
        assert_eq!(step(blocked, parameters(), tick(true)).unwrap(), blocked);
    }
}
