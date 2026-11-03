//! Ordinary actor model-speed/heading step. This is not the Mecc force
//! integrator, terrain collision or a complete behavior callback.
use crate::object_time::{Sample, SlowArea};

#[derive(Clone, Copy, Debug)]
pub struct Parameters {
    pub acceleration: f32,
    pub speed: f32,
    pub braking: f32,
    pub attenuation: f32,
    pub turn_acceleration: f32,
    pub turn_speed: f32,
    pub turn_braking: f32,
    pub gravity: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct State {
    pub position: [f32; 3],
    pub heading: f32,
    /// Native look pitch is separate from the body's Euler pitch/roll.
    pub look_pitch: f32,
    /// Vertical, forward, side and heading speed.
    pub rates: [f32; 4],
    pub previous_rates: [f32; 4],
    pub active: u32,
    pub personality: u32,
    pub state: u32,
    pub context: u32,
    pub parent_present: bool,
    /// Heading, look pitch, forward and side signed commands.
    pub controls: [i8; 4],
    pub target_heading: f32,
    pub target_position: [f32; 2],
}
pub struct Environment<'a> {
    pub parameters: Parameters,
    pub global_seconds: f32,
    pub areas: &'a [SlowArea],
    pub control_factor: f32,
    pub running_factor: f32,
    /// Supplied global heading gate; not a general simulation-pause policy.
    pub skip_heading: bool,
    pub controlled_actor: bool,
}
impl Environment<'_> {
    fn seconds(&self, s: &State) -> f64 {
        crate::object_time::seconds(
            self.global_seconds,
            Some(Sample {
                owner: s.context,
                position: s.position,
            }),
            None,
            self.areas,
        )
    }
}
fn ramp(old: f32, desired: f32, increment: f32, braking: f64) -> f32 {
    let mut old = old;
    if desired > 0. {
        if old < 0. {
            old = 0.;
        }
        if old < desired {
            let next = f64::from(old) + f64::from(increment);
            if next > f64::from(desired) {
                desired
            } else {
                next as f32
            }
        } else if old > desired {
            let next = f64::from(old) - braking;
            if next < f64::from(desired) {
                desired
            } else {
                next as f32
            }
        } else {
            old
        }
    } else if desired < 0. {
        if old > 0. {
            old = 0.;
        }
        if old > desired {
            let next = f64::from(old) + f64::from(increment);
            if next < f64::from(desired) {
                desired
            } else {
                next as f32
            }
        } else if old < desired {
            let next = f64::from(old) + braking;
            if next > f64::from(desired) {
                desired
            } else {
                next as f32
            }
        } else {
            old
        }
    } else if old > 0. {
        let next = f64::from(old) - braking;
        if next < 0. { 0. } else { next as f32 }
    } else if old < 0. {
        let next = f64::from(old) + braking;
        if next > 0. { 0. } else { next as f32 }
    } else {
        old
    }
}
fn normalized(a: f32) -> f32 {
    if a < 0. {
        (f64::from(a) % 360. + 360.) as f32
    } else if a >= 360. {
        (f64::from(a) % 360.) as f32
    } else {
        a
    }
}
fn angle_difference(target: f32, current: f32) -> f64 {
    let mut delta = f64::from(normalized(target)) - f64::from(normalized(current));
    if delta > 180. {
        delta -= 360.;
    }
    if delta < -180. {
        delta += 360.;
    }
    delta
}
/// Native heading and look helpers only. Physical actors use this phase before
/// their force integrator; position and linear rates/history are preserved.
pub fn orient(mut s: State, e: &Environment<'_>) -> Result<State, &'static str> {
    let p = e.parameters;
    if !s
        .position
        .into_iter()
        .chain([s.heading, s.look_pitch, s.target_heading])
        .chain(s.target_position)
        .chain(s.rates)
        .chain(s.previous_rates)
        .chain([
            p.acceleration,
            p.speed,
            p.braking,
            p.attenuation,
            p.turn_acceleration,
            p.turn_speed,
            p.turn_braking,
            p.gravity,
            e.global_seconds,
            e.control_factor,
            e.running_factor,
        ])
        .all(f32::is_finite)
        || e.areas.iter().any(|a| {
            !a.center
                .into_iter()
                .chain([a.radius_squared, a.factor])
                .all(f32::is_finite)
        })
        || s.heading.abs() > 1_000_000.
        || s.target_heading.abs() > 1_000_000.
    {
        return Err("ordinary actor motion outside finite heading profile");
    }
    if !e.skip_heading {
        let command = (f64::from(s.controls[0]) * 0.03125 * f64::from(e.control_factor)) as f32;
        let dt = e.seconds(&s);
        let increment = (dt * f64::from(p.turn_acceleration) * f64::from(command)) as f32;
        let desired = (f64::from(command) * f64::from(p.turn_speed)) as f32;
        s.rates[3] = ramp(
            s.rates[3],
            desired,
            increment,
            dt * f64::from(p.turn_braking),
        );
        let mut delta =
            ((f64::from(s.previous_rates[3]) + f64::from(s.rates[3])) * dt * 0.5) as f32;
        if s.active & 0x0100_0000 != 0 {
            let remaining = angle_difference(s.target_heading, s.heading);
            if (remaining > 0. && f64::from(delta) > remaining)
                || (remaining < 0. && f64::from(delta) < remaining)
            {
                delta = remaining as f32;
                s.previous_rates[3] = 0.;
                s.rates[3] = 0.;
            }
        }
        let wide = f64::from(s.heading) + f64::from(delta);
        s.heading = if wide >= 360. {
            (wide - 360.) as f32
        } else {
            wide as f32
        };
        if s.heading < 0. {
            s.heading = (f64::from(s.heading) + 360.) as f32;
        }
    }
    if s.controls[1] != 0 {
        // The pitch path narrows object time before applying sensitivity.
        let dt = e.seconds(&s) as f32;
        let wide = (((f64::from(e.control_factor) * f64::from(dt))
            * (f64::from(s.controls[1]) * 0.03125))
            * 90.)
            + f64::from(s.look_pitch);
        s.look_pitch = if e.controlled_actor && wide > 80. {
            80.
        } else {
            wide as f32
        };
        if s.look_pitch < -90. {
            s.look_pitch = -90.;
        }
    }
    Ok(s)
}

/// Complete nonphysical movement, composed after the shared heading/look phase.
pub fn step(s: State, e: &Environment<'_>) -> Result<State, &'static str> {
    let mut s = orient(s, e)?;
    let p = e.parameters;
    let dt = e.seconds(&s);
    let (sin, cos) = crate::object_transform::sincos(s.heading);
    for axis in 0..2 {
        let command = (f64::from(s.controls[axis + 2]) * 0.03125) as f32;
        let increment = (dt * f64::from(command) * f64::from(p.acceleration)) as f32;
        let mut desired = f64::from((f64::from(command) * f64::from(p.speed)) as f32);
        if s.state == 2 && s.active & 0x0800_0000 != 0 {
            desired *= f64::from(e.running_factor);
        } else if s.active & 0x0010_0000 != 0 {
            desired *= f64::from(p.attenuation);
        }
        if axis == 1 && s.personality & 0x0004_0000 == 0 {
            desired *= f64::from(0.2f32);
        }
        s.rates[axis + 1] = ramp(
            s.rates[axis + 1],
            desired as f32,
            increment,
            dt * f64::from(p.braking),
        );
    }
    let forward = ((f64::from(s.previous_rates[1]) + f64::from(s.rates[1])) * dt * 0.5) as f32;
    let side = (f64::from(s.previous_rates[2]) + f64::from(s.rates[2])) * dt * 0.5;
    let mut dx = (f64::from(forward) * sin + side * cos) as f32;
    let mut dy = (side * sin - f64::from(forward) * cos) as f32;
    if s.active & 0x0200_0000 != 0 {
        let x = f64::from(s.target_position[0]) - f64::from(s.position[0]);
        let y = (f64::from(s.target_position[1]) - f64::from(s.position[1])) as f32;
        if (x >= 0. && f64::from(dx) > x) || (x <= 0. && f64::from(dx) < x) {
            dx = x as f32;
            if x * f64::from(0.2f32) > f64::from(y).abs() {
                dy = y;
            }
        }
        if (y >= 0. && dy > y) || (y <= 0. && dy < y) {
            dy = y;
            if f64::from(y) * f64::from(0.2f32) > x.abs() {
                dx = x as f32;
            }
        }
        if x == f64::from(dx) && y == dy {
            s.rates[1] = 0.;
            s.rates[2] = 0.;
        }
    }
    s.position[0] = (f64::from(s.position[0]) + f64::from(dx)) as f32;
    s.position[1] = (f64::from(s.position[1]) + f64::from(dy)) as f32;
    // Native helper recomputes object time at the new XY position.
    let dt = e.seconds(&s);
    if s.active & 0x10 == 0 {
        if s.active & 0x0008_0000 == 0 && !s.parent_present {
            s.rates[0] = (f64::from(s.rates[0]) - dt * f64::from(p.gravity)) as f32;
        } else if s.rates[0] < 0. {
            s.rates[0] = 0.;
        }
    }
    let dz = ((f64::from(s.previous_rates[0]) + f64::from(s.rates[0])) * dt * 0.5) as f32;
    s.position[2] = (f64::from(s.position[2]) + f64::from(dz)) as f32;
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (State, Environment<'static>) {
        (
            State {
                position: [-1., 0., 10.],
                heading: 90.,
                look_pitch: 0.,
                rates: [0., 100., 0., 0.],
                previous_rates: [0., 100., 0., 0.],
                active: 0,
                personality: 0x40000,
                state: 23,
                context: 1,
                parent_present: false,
                controls: [0, 0, 32, 0],
                target_heading: 0.,
                target_position: [0.; 2],
            },
            Environment {
                parameters: Parameters {
                    acceleration: 0.,
                    speed: 100.,
                    braking: 0.,
                    attenuation: 1.,
                    turn_acceleration: 0.,
                    turn_speed: 0.,
                    turn_braking: 0.,
                    gravity: 10.,
                },
                global_seconds: 0.1,
                areas: &[],
                control_factor: 1.,
                running_factor: 1.,
                skip_heading: false,
                controlled_actor: false,
            },
        )
    }

    #[test]
    fn vertical_time_is_requeried_after_crossing_area_boundary() {
        let (s, mut e) = fixture();
        let areas = [SlowArea {
            source_owner: 2,
            center: [9., 0., 0.],
            radius_squared: 4.,
            factor: 0.25,
        }];
        e.areas = &areas;
        let result = step(s, &e).unwrap();
        // Exact output words of the complete native callback, including trig
        // table residual and the new local time used by gravity.
        let words = result.position.map(f32::to_bits);
        assert_eq!(words, [1091567616, 896183418, 1092612915]);
        assert_eq!(result.rates[0], -0.25);
    }

    #[test]
    fn strict_heading_overshoot_clears_both_rate_slots() {
        let (mut s, mut e) = fixture();
        e.global_seconds = 1.;
        s.active = 0x0100_0020;
        s.rates[3] = 10.;
        s.previous_rates[3] = 10.;
        s.target_heading = 95.;
        let result = step(s, &e).unwrap();
        assert_eq!(result.heading, 95.);
        assert_eq!([result.rates[3], result.previous_rates[3]], [0.; 2]);
        assert_eq!(result.active, s.active);
        // Equality is not an overshoot and does not stop rotation.
        s.target_heading = 100.;
        let exact = step(s, &e).unwrap();
        assert_eq!(exact.heading, 100.);
        assert_eq!([exact.rates[3], exact.previous_rates[3]], [10.; 2]);
    }

    #[test]
    fn pitch_clamp_requires_input_and_upper_clamp_requires_controlled_actor() {
        let (mut s, mut e) = fixture();
        s.look_pitch = 100.;
        e.controlled_actor = true;
        assert_eq!(step(s, &e).unwrap().look_pitch, 100.);
        s.controls[1] = 32;
        assert_eq!(step(s, &e).unwrap().look_pitch, 80.);
        e.controlled_actor = false;
        assert_eq!(step(s, &e).unwrap().look_pitch, 109.);
        s.look_pitch = -100.;
        s.controls[1] = -32;
        assert_eq!(step(s, &e).unwrap().look_pitch, -90.);
    }

    #[test]
    fn parent_or_support_clamps_negative_speed_but_gravity_bypass_keeps_it() {
        let (mut s, e) = fixture();
        s.rates[0] = -3.;
        s.parent_present = true;
        assert_eq!(step(s, &e).unwrap().rates[0], 0.);
        s.parent_present = false;
        s.active = 0x80000;
        assert_eq!(step(s, &e).unwrap().rates[0], 0.);
        s.active |= 0x10;
        assert_eq!(step(s, &e).unwrap().rates[0], -3.);
    }

    #[test]
    fn nonfinite_inputs_are_refused() {
        let (mut s, e) = fixture();
        s.position[0] = f32::NAN;
        assert!(step(s, &e).is_err());
    }
}
