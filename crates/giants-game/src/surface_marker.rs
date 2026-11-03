//! Acquired D state48 primary and secondary. Retirement marks D8 bit15; it
//! does not unlink the reciprocal owner or acknowledge resource destruction.
use crate::animation_channels::{self, Blending, Channels, Request};
#[derive(Clone, Copy, Debug)]
pub struct State {
    pub mode: u32,
    pub timer: f32,
    pub flags: u32,
    pub position: [f32; 3],
}
#[derive(Clone, Copy, Debug)]
pub struct Owner {
    pub state: u32,
    pub position: [f32; 3],
}
pub fn primary(
    mut s: State,
    channel: [u32; 9],
    owner: Option<Owner>,
    delay: f32,
    seconds: f64,
) -> Result<State, String> {
    if s.mode > 3
        || !s.timer.is_finite()
        || !delay.is_finite()
        || !seconds.is_finite()
        || !s
            .position
            .into_iter()
            .chain(owner.into_iter().flat_map(|o| o.position))
            .all(f32::is_finite)
    {
        return Err("surface marker outside acquired finite mode0..3 profile".into());
    }
    match s.mode {
        0 if channel[0] == 11 && channel[8] & 2 != 0 => s.mode = 1,
        1 => {
            if let Some(o) = owner.filter(|o| o.state == 23)
                && (s.position[0] != o.position[0] || s.position[1] != o.position[1])
            {
                s.timer = delay;
                s.mode = 3;
            }
        }
        2 if channel[0] == 12 && channel[8] & 2 != 0 => s.flags |= 0x8000,
        3 => {
            let wide = f64::from(s.timer) - seconds;
            if !(wide as f32).is_finite() {
                return Err("surface timer overflow".into());
            }
            s.timer = wide as f32;
            // Original comparison precedes rounding: equality stays in3.
            if wide < 0. {
                s.mode = 2;
            }
        }
        _ => {}
    }
    Ok(s)
}
pub fn secondary(
    mode: u32,
    mut channels: Channels,
    blending: Blending,
) -> Result<Channels, String> {
    let (slot, flags) = match mode {
        0 => (11, 0x20),
        1 | 3 => (1, 0x21),
        2 => (12, 0x20),
        _ => return Err("surface animation outside acquired mode0..3".into()),
    };
    if channels[0][0] != u32::from(slot) {
        animation_channels::select_primary(
            &mut channels,
            Request {
                slot,
                flags,
                speed_source: 0,
                reference_speed: 0.,
                blend_seconds: f32::from_bits(0x3e2aaaab),
            },
            blending,
        )?;
    }
    Ok(channels)
}
