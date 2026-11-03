//! Acquired no-held-effect reset and inactive auxiliary timer tail.
//! Inventory, healing and sound branches require their actual services.

pub fn reset_unheld(child_present: bool, scalar: f32) -> Result<[u32; 2], &'static str> {
    if child_present || !scalar.is_finite() {
        return Err("Mecc effect reset requires acquired finite unheld profile");
    }
    Ok([scalar.to_bits(), 0])
}

pub fn advance_timers(
    flags: u32,
    timers: [f32; 2],
    seconds: f64,
) -> Result<[f32; 2], &'static str> {
    if flags & 0xa != 0 {
        return Err("Mecc auxiliary timers require inventory/healing/audio services");
    }
    if !seconds.is_finite() || !timers.into_iter().all(f32::is_finite) {
        return Err("Mecc auxiliary timers outside acquired finite input profile");
    }
    Ok(timers.map(|old| (f64::from(old) + seconds) as f32))
}
