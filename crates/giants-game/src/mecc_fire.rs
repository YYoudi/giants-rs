//! Checked fire-clock operations and straight projectile launch direction.
use crate::arithmetic::Precision;

/// Numeric common tail of D's normal fire wrapper, after the selected weapon
/// handling and any required external cleanup. This never acknowledges those
/// services or decides which path the wrapper takes.
pub struct Tail {
    pub cooldown: f32,
    pub response_flags: u32,
    pub control_flags: u32,
}
pub fn finish_tail(
    cooldown: f32,
    buttons: u32,
    mut response_flags: u32,
    mut control_flags: u32,
) -> Result<Tail, &'static str> {
    if !cooldown.is_finite() {
        return Err("fire tail requires a finite clock");
    }
    // The native comparison is strictly negative; preserve negative zero.
    let cooldown = if cooldown < 0. { 0. } else { cooldown };
    if buttons & 1 == 0 {
        response_flags &= !0x08000000;
        control_flags &= !0x00100000;
    } else if buttons & 0x1000 == 0 {
        response_flags |= 0x08000000;
    }
    Ok(Tail {
        cooldown,
        response_flags,
        control_flags,
    })
}
pub fn advance_clock(cooldown: f32, dt: f32, precision: Precision) -> Result<f32, &'static str> {
    if !cooldown.is_finite() || !dt.is_finite() || dt < 0. {
        return Err("invalid fire clock");
    }
    Ok(if cooldown > 0. {
        precision.round(f64::from(cooldown) - f64::from(dt)) as f32
    } else {
        cooldown
    })
}
pub fn request(
    cooldown: f32,
    interval: f32,
    flags: u32,
    definition_available: bool,
    target_present: bool,
    trigger: bool,
    precision: Precision,
) -> Result<(bool, f32), &'static str> {
    if !cooldown.is_finite() || !interval.is_finite() || interval < 0. {
        return Err("invalid fire request");
    }
    let ready =
        trigger && definition_available && (flags & 1 == 0 || target_present) && cooldown <= 0.;
    Ok((
        ready,
        if ready {
            precision.round(f64::from(interval) + f64::from(cooldown)) as f32
        } else {
            cooldown
        },
    ))
}
/// Straight profile with a supplied muzzle and aim point. Native matrix/trig
/// construction differs by small rounding terms; the control reports its error.
pub fn launch(
    origin: [f32; 3],
    target: [f32; 3],
    speed: f32,
    precision: Precision,
) -> Result<[f32; 3], &'static str> {
    if !origin
        .into_iter()
        .chain(target)
        .chain([speed])
        .all(f32::is_finite)
        || speed <= 0.
    {
        return Err("invalid projectile launch");
    }
    let r = |v| precision.round(v);
    let delta: [f32; 3] =
        std::array::from_fn(|n| r(f64::from(target[n]) - f64::from(origin[n])) as f32);
    let [x, y, z] = delta.map(f64::from);
    let length = r(r(r(r(x * x) + r(y * y)) + r(z * z)).sqrt());
    if length < 0.0001 {
        return Err("degenerate projectile direction");
    }
    let inv = r(1. / length);
    Ok(delta.map(|v| r(r(f64::from(v) * inv) * f64::from(speed)) as f32))
}
