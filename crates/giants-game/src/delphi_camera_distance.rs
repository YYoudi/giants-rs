//! Acquired finite Delphi ordinary-camera distance policy.
//! The input flag is consumed as stored; its gameplay producer is separate.

#[derive(Clone, Copy, Debug)]
pub struct Output {
    pub distance: f32,
    pub chase: f32,
}

pub fn advance(distance: f32, chase: f32, flags: u32, seconds: f32) -> Result<Output, String> {
    if ![distance, chase, seconds].into_iter().all(f32::is_finite) || seconds < 0. {
        return Err("Delphi camera distance requires finite values and nonnegative time".into());
    }
    let extended = flags & 0x100 != 0;
    // This store precedes the comparison and the temporal-approach call.
    let mut distance = if extended {
        (f64::from(distance) + 5.) as f32
    } else {
        distance
    };
    if !distance.is_finite() {
        return Err("Delphi camera distance overflowed".into());
    }
    let mut chase = chase;
    if distance != chase {
        crate::mecc_camera_update::approach_with_time(
            &mut chase,
            distance,
            seconds,
            if extended { 1. } else { 0.1_f32 },
        );
        distance = chase;
    }
    Ok(Output { distance, chase })
}
