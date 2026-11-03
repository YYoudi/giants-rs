//! Debug reference frame update after initialization.
//! Mode producers and the live OS clock are deliberately supplied by the caller.
use crate::arithmetic::Precision;

#[derive(Clone, Copy, Debug)]
pub struct State {
    pub warmup: i32,
    pub last_ms: u32,
    pub deadline_ms: u32,
    pub virtual_ms: u32,
    pub seconds: f32,
    pub smoothed_seconds: f32,
    pub elapsed_seconds: f32,
    pub blend: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Modes {
    pub synthetic: bool,
    pub slow: bool,
    pub fixed: bool,
    pub scale: f32,
}

/// Debug input block selects the global scale before frame timing.
/// These are native input words, not portable key codes or user bindings.
pub fn debug_input_scale(enabled: bool, held: u32, modifiers: u32) -> f32 {
    if !enabled || modifiers & ((1 << 22) | (1 << 10)) != 0 {
        1.
    } else if held & (1 << 12) != 0 {
        6.
    } else if held & (1 << 13) != 0 {
        16.
    } else {
        1.
    }
}

/// `elapsed_ms` is the wrapping OS counter minus its native origin.
/// The native last==0 initialization branch is outside this function's scope.
pub fn advance(
    mut s: State,
    m: Modes,
    elapsed_ms: u32,
    precision: Precision,
) -> Result<State, &'static str> {
    if s.last_ms == 0 {
        return Err("frame clock requires initialized nonzero timestamp");
    }
    if ![s.smoothed_seconds, s.blend, m.scale]
        .into_iter()
        .all(f32::is_finite)
        || !(0. ..=0.75).contains(&s.blend)
        || m.scale < 0.
    {
        return Err("frame clock outside finite measured profile");
    }
    let r = |v| precision.round(v);
    let ms = if m.synthetic {
        s.virtual_ms.wrapping_add(50)
    } else {
        elapsed_ms
    };
    let milliseconds = f64::from(0.001_f32);
    s.elapsed_seconds = r(f64::from(ms) * milliseconds) as f32;
    let previous = s.last_ms.min(ms);
    let mut delta = r(f64::from(ms - previous) * milliseconds);
    if s.warmup <= 5 {
        s.warmup = s.warmup.wrapping_add(1);
    } else {
        delta = r(r(r(1. - f64::from(s.blend)) * delta)
            + r(f64::from(s.smoothed_seconds) * f64::from(s.blend)));
        if s.blend < 0.75 {
            s.blend = r(f64::from(s.blend) + f64::from(0.15_f32)).min(0.75) as f32;
        }
    }
    // Native FST stores smooth but keeps the wide result for the final scale.
    s.smoothed_seconds = delta as f32;
    if m.fixed {
        delta = f64::from(0.025_f32);
    }
    if m.slow {
        delta = r(delta * f64::from(0.1_f32));
    }
    s.seconds = r(f64::from(m.scale) * delta) as f32;
    if !m.slow && !m.fixed && s.seconds > 0.25 {
        s.seconds = 0.25;
    }
    s.last_ms = ms;
    s.virtual_ms = ms;
    s.deadline_ms = ms.wrapping_add(25);
    Ok(s)
}
