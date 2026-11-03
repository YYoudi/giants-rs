//! Object-local delta selection and the measured paired-effect countdown.
//! The global delta and ordered slow areas are inputs, not a clock policy.

#[derive(Clone, Copy, Debug)]
pub struct Sample {
    pub owner: u32,
    pub position: [f32; 3],
}

#[derive(Clone, Copy, Debug)]
pub struct SlowArea {
    pub source_owner: u32,
    pub center: [f32; 3],
    pub radius_squared: f32,
    pub factor: f32,
}

// The measured x87 profile selects the larger quiet-NaN significand when both
// operands are NaNs, preferring positive on a tie. SSE's default can differ.
fn paired_nan(left: f64, right: f64) -> Option<f64> {
    if !left.is_nan() || !right.is_nan() {
        return None;
    }
    let l = left.to_bits();
    let r = right.to_bits();
    let payload = 0x000fffffffffffff;
    Some(f64::from_bits(if (l & payload) > (r & payload) {
        l
    } else if (l & payload) < (r & payload) {
        r
    } else {
        l.min(r)
    }))
}

/// Measured53-bit arithmetic profile. Object queries skip areas with the same
/// opaque owner word; position-only queries do not. First qualifying area wins.
/// Distance uses XY, including the boundary. Z is deliberately not consulted.
pub fn seconds(
    global: f32,
    object: Option<Sample>,
    position: Option<[f32; 3]>,
    areas: &[SlowArea],
) -> f64 {
    let position = object.map(|o| o.position).or(position);
    if let Some(position) = position {
        for area in areas {
            if object.is_some_and(|o| o.owner == area.source_owner) {
                continue;
            }
            let dx = f64::from(area.center[0]) - f64::from(position[0]);
            let dy = f64::from(area.center[1]) - f64::from(position[1]);
            if dx * dx + dy * dy <= f64::from(area.radius_squared) {
                let (global, factor) = (f64::from(global), f64::from(area.factor));
                return paired_nan(global, factor).unwrap_or(global * factor);
            }
        }
    }
    f64::from(global)
}

/// Visibility is decided from the old timer, before decrement. Crossing zero
/// therefore does not hide a previously visible pair until the next callback.
pub fn step_pair(timer: &mut f32, first: &mut u32, second: &mut u32, delta: f64) {
    const HIDDEN: u32 = 0x40000000;
    if *timer > 0. {
        *first &= !HIDDEN;
        *second &= !HIDDEN;
    } else {
        *first |= HIDDEN;
        *second |= HIDDEN;
    }
    let next = paired_nan(f64::from(*timer), delta).unwrap_or(f64::from(*timer) - delta);
    *timer = if next <= 0. { 0. } else { next as f32 };
}
