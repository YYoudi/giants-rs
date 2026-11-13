//! Acquired ordinary Ripper turn/pursuit command producers. Target selection,
//! mode transitions, animation and effects remain upstream/downstream services.
//! Prototype PS2 search key: `ripper_chase_location__FP6OBJECTP3P3Df`.
//! Exact correspondence remains unknown; PC debug-1.1 0x004dbd10 is retained for
//! the acquired angle helper used by the direct class230 chase caller.
//! Its previous alias remains in docs/SYMBOL_NAVIGATION.md.
#[derive(Clone, Copy, Debug)]
pub struct State {
    pub position: [f32; 3],
    pub heading: f32,
    pub flags: u32,
    pub personality: u32,
    pub commands: [u8; 8],
    pub target_heading: f32,
    pub target_position: [f32; 3],
}
#[derive(Clone, Copy, Debug)]
pub struct Orbit {
    pub angle: f32,
    pub radius: f32,
}
fn validate(state: &State, target: [f32; 3]) -> Result<(), &'static str> {
    if !state
        .position
        .into_iter()
        .chain(target)
        .chain(state.target_position)
        .chain([state.heading, state.target_heading])
        .all(f32::is_finite)
        || state.heading.abs() > 1_000_000.
    {
        return Err("steering requires finite acquired current poses");
    }
    Ok(())
}
pub(crate) fn normalize(angle: f32) -> f32 {
    if angle < 0. {
        (f64::from(angle) % 360. + 360.) as f32
    } else if angle >= 360. {
        (f64::from(angle) % 360.) as f32
    } else {
        angle
    }
}
pub(crate) fn difference(a: f32, b: f32) -> f64 {
    let delta = f64::from(normalize(a)) - f64::from(normalize(b));
    if delta > 180. {
        delta - 360.
    } else if delta < -180. {
        delta + 360.
    } else {
        delta
    }
}
pub(crate) fn command(value: f32, minimum: f32) -> u8 {
    let mut value = value.clamp(-1., 1.);
    if value > 0. && value < minimum {
        value = minimum;
    }
    if value < 0. && value > -minimum {
        value = -minimum;
    }
    let mut result = (f64::from(value) * 32.) as i32;
    if result == 0 && value != 0. {
        result = if value < 0. { -1 } else { 1 };
    }
    result as i8 as u8
}
fn turn(state: &mut State, target: [f32; 3]) -> f32 {
    state.commands[0] = 0;
    let heading = crate::actor_volume::heading(state.position, target);
    let error = difference(heading, state.heading);
    if error != 0. {
        state.commands[0] = command((f64::from(error as f32) * f64::from(0.05_f32)) as f32, 0.25);
        state.target_heading = heading;
        state.flags |= 0x01000000;
    }
    error as f32
}
/// Mode32's turning block, ending before its animation/effect continuation.
pub fn turn_to_point(mut state: State, target: [f32; 3]) -> Result<State, &'static str> {
    validate(&state, target)?;
    turn(&mut state, target);
    state.personality &= !0x40;
    Ok(state)
}
/// Class230 mode64's direct destination producer. The angle helper returns a
/// stored f32 difference; no orbit or personality update occurs on this route.
pub fn chase_location(mut state: State, target: [f32; 3]) -> Result<State, &'static str> {
    validate(&state, target)?;
    let dx = f64::from(target[0]) - f64::from(state.position[0]);
    let dy = f64::from(target[1]) - f64::from(state.position[1]);
    let distance = (dx * dx + dy * dy).sqrt() as f32;
    if !distance.is_finite() {
        return Err("direct chase distance overflow");
    }
    let error = turn(&mut state, target);
    state.target_position = target;
    state.flags |= 0x02000000;
    state.commands[2] = if error.abs() < 10. {
        command((f64::from(distance) * f64::from(0.1_f32)) as f32, 0.2)
    } else {
        3
    };
    Ok(state)
}
/// Class230 mode67 keeps the previous forward command outside the angular
/// threshold. Within it the source writes full forward input, without an
/// orbit, distance-scaled speed or a forced position update.
pub fn close_chase(mut state: State, target: [f32; 3]) -> Result<State, &'static str> {
    validate(&state, target)?;
    let error = turn(&mut state, target);
    if error.abs() < 10. {
        state.commands[2] = 32;
    }
    state.target_position = target;
    state.flags |= 0x02000000;
    Ok(state)
}
/// Attack modes keep the angular difference wide until the command input
/// store, and preserve personality. Their producer differs from mode32.
pub fn aim_at_point(mut state: State, target: [f32; 3]) -> Result<State, &'static str> {
    validate(&state, target)?;
    state.commands[0] = 0;
    let heading = crate::actor_volume::heading(state.position, target);
    let error = difference(heading, state.heading);
    if error != 0. {
        state.commands[0] = command((error * f64::from(0.05_f32)) as f32, 0.25);
        state.target_heading = heading;
        state.flags |= 0x01000000;
    }
    Ok(state)
}
/// Shared modes33/34 pursuit block, ending before attack/visibility decisions.
/// Orbit parameters must come from their actual behavior producer, not guesses.
pub fn pursue(mut state: State, target: [f32; 3], orbit: Orbit) -> Result<State, &'static str> {
    validate(&state, target)?;
    if !orbit.angle.is_finite() || orbit.angle.abs() > 1_000_000. || !orbit.radius.is_finite() {
        return Err("steering orbit outside acquired finite profile");
    }
    let (sine, cosine) = crate::object_transform::sincos(orbit.angle);
    let point = [
        (f64::from(target[0]) + sine * f64::from(orbit.radius)) as f32,
        (f64::from(target[1]) - cosine * f64::from(orbit.radius)) as f32,
        target[2],
    ];
    let dx = f64::from(point[0]) - f64::from(state.position[0]);
    let dy = f64::from(point[1]) - f64::from(state.position[1]);
    let distance = (dx * dx + dy * dy).sqrt() as f32;
    if !point.into_iter().chain([distance]).all(f32::is_finite) {
        return Err("steering orbit position overflow");
    }
    let error = turn(&mut state, point);
    state.target_position = point;
    state.flags |= 0x02000000;
    state.commands[2] = if error.abs() < 10. {
        command((f64::from(distance) * f64::from(0.1_f32)) as f32, 0.2)
    } else {
        3
    };
    Ok(state)
}
