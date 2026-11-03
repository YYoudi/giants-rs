//! Player physical-location dispatcher, with explicit contact/ground/event providers.
//! This is distinct from the generic object dispatcher. Providers must execute
//! their real behavior before an application can claim native player collisions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct State {
    pub current: [f32; 3],
    pub previous: [f32; 3],
    pub object_flags: u32,
    pub physics_flags: u32,
    pub support: u32,
    pub hit: u32,
}
pub struct Output {
    pub state: State,
    pub passes: u32,
    pub fallback: bool,
    pub ground_called: bool,
    pub notification_called: bool,
}
pub fn resolve(
    mut state: State,
    has_model: bool,
    collision_flags: u32,
    mut contacts: impl FnMut(&mut State) -> Result<bool, &'static str>,
    mut ground: impl FnMut(&mut State) -> Result<(), &'static str>,
    mut notify: impl FnMut(&mut State) -> Result<(), &'static str>,
) -> Result<Output, &'static str> {
    let finite = |s: &State| s.current.into_iter().chain(s.previous).all(f32::is_finite);
    if !finite(&state) {
        return Err("nonfinite player location input");
    }
    let mut out = Output {
        state,
        passes: 0,
        fallback: false,
        ground_called: false,
        notification_called: false,
    };
    if state.object_flags & 0x1000_0000 == 0 && state.current == state.previous {
        return Ok(out);
    }
    state.object_flags &= !0x1000_0000;
    // The native store clears a byte, preserving all upper physics bits.
    state.physics_flags &= 0xffff_ff00;
    state.support = 0;
    state.hit = 0;
    if has_model {
        if collision_flags & 0x400 == 0 {
            loop {
                let retry = contacts(&mut state)?;
                if !finite(&state) {
                    return Err("nonfinite player contact output");
                }
                out.passes += 1;
                if !retry {
                    break;
                }
                if state.current == state.previous || out.passes == 4 {
                    state.current = state.previous;
                    out.fallback = true;
                    break;
                }
            }
        }
        if collision_flags & 0x200 == 0 {
            ground(&mut state)?;
            out.ground_called = true;
        }
    }
    if state.physics_flags & 0x68 != 0 {
        state.physics_flags |= 0x80;
    }
    if state.physics_flags & 0x96 != 0 {
        notify(&mut state)?;
        out.notification_called = true;
    }
    if !finite(&state) {
        return Err("nonfinite player location output");
    }
    out.state = state;
    Ok(out)
}
