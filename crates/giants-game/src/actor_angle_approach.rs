//! Shared finite-profile angle approach used by acquired Delphi and Vimp callers.
fn normalize(value: f32) -> f32 {
    if value < 0. {
        ((f64::from(value) % 360.) + 360.) as f32
    } else if value >= 360. {
        (f64::from(value) % 360.) as f32
    } else {
        value
    }
}

// Both acquired callers use a maximum gap of 10,000. Inputs are validated by
// each caller; this is not a public arbitrary-angle admission API.
pub(crate) fn approach(current: f32, target: f32, step: f32) -> f32 {
    let target = normalize(target);
    let mut current = normalize(current);
    let gap = f64::from(target) - f64::from(current);
    if gap > 180. {
        current = (f64::from(current) + 360.) as f32;
    } else if gap < -180. {
        current = (f64::from(current) - 360.) as f32;
    }
    if current > target {
        let wide = f64::from(current) - f64::from(step);
        current = if wide < f64::from(target) {
            target
        } else {
            wide as f32
        };
        if f64::from(current) - f64::from(target) > 10000. {
            current = (f64::from(target) + 10000.) as f32;
        }
    } else if current < target {
        let wide = f64::from(current) + f64::from(step);
        current = if f64::from(target) < wide {
            target
        } else {
            wide as f32
        };
        if f64::from(target) - f64::from(current) > 10000. {
            current = (f64::from(target) - 10000.) as f32;
        }
    }
    normalize(current)
}
