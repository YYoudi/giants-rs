//! Pointer-free channel selection and bounded clock update. Shared resource
//! ownership, callbacks and blending actual poses remain separate consumers.
use crate::{animation_loop::Clock, arithmetic::Precision};
pub type Channels = [[u32; 9]; 4];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Request {
    pub slot: u8,
    pub flags: u32,
    pub speed_source: i32,
    pub reference_speed: f32,
    pub blend_seconds: f32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Blending {
    Disabled,
    Enabled,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tick {
    pub processed: u8,
    pub wrapped: u8,
    pub missing: u8,
}

pub fn clock(channel: &[u32; 9]) -> Clock {
    Clock {
        seconds: f32::from_bits(channel[1]),
        previous: f32::from_bits(channel[2]),
        overflow: f32::from_bits(channel[3]),
        wrapped: channel[8] & 2 != 0,
    }
}

/// Update all four channels in source order. Reset/pause flags zero the shared
/// delta for later channels too. Descriptors are supplied, never fabricated.
/// Unsupported arithmetic is refused atomically; no pose/callback is evaluated.
pub fn advance<const N: usize>(
    channels: &mut Channels,
    durations: &[Option<f32>],
    model: [f32; N],
    dt: f32,
    precision: Precision,
) -> Result<Tick, &'static str> {
    if !dt.is_finite() || dt < 0. {
        return Err("channel delta outside finite nonnegative profile");
    }
    let mut next = *channels;
    let mut step = dt;
    let mut tick = Tick::default();
    let r = |v| precision.round(v);
    for (index, channel) in next.iter_mut().enumerate() {
        if step == 0. && channel[8] & 0x100 == 0 {
            continue;
        }
        channel[8] &= !0x100;
        if channel[0] == 254 {
            continue;
        }
        if channel[0] > 254 {
            return Err("channel slot outside descriptor table");
        }
        let Some(duration) = durations.get(channel[0] as usize).copied().flatten() else {
            tick.missing |= 1 << index;
            continue;
        };
        if !duration.is_finite() || duration <= 0. {
            return Err("channel duration outside finite positive profile");
        }
        let mut rate = crate::animation_loop::playback_rate(
            f32::from_bits(channel[5]),
            channel[4] as i32,
            model,
            precision,
        )?;
        if channel[8] & 0x10 != 0 {
            rate = -rate;
        }
        if channel[8] & 0xc != 0 {
            channel[8] &= !8;
            step = 0.;
            channel[3] = 0;
        }
        if channel[8] & 0x80 != 0 {
            channel[1] = if rate < 0. {
                duration.to_bits()
            } else {
                (r(f64::from(f32::from_bits(channel[1])) * f64::from(duration)) as f32).to_bits()
            };
            channel[8] &= !0x80;
            step = 0.;
            channel[3] = 0;
        }
        for (flag, subtract) in [(0x20, rate < 0.), (0x40, rate > 0.)] {
            if channel[8] & flag != 0 {
                channel[1] = if subtract {
                    (r(f64::from(duration) - f64::from(f32::from_bits(channel[3]))) as f32)
                        .to_bits()
                } else {
                    channel[3]
                };
                channel[8] &= !flag;
            }
        }
        let mut value = clock(channel);
        value.advance_with_rate(duration, step, rate, precision)?;
        if value.wrapped && channel[8] & 1 == 0 {
            value.seconds = if rate < 0. { 0. } else { duration };
        }
        channel[1] = value.seconds.to_bits();
        channel[2] = value.previous.to_bits();
        channel[3] = value.overflow.to_bits();
        channel[8] = (channel[8] & !2) | if value.wrapped { 2 } else { 0 };
        let weight = f32::from_bits(channel[7]);
        let transition = f32::from_bits(channel[6]);
        if weight > 0. && transition != 0. {
            if !weight.is_finite() || !transition.is_finite() {
                return Err("channel transition outside finite profile");
            }
            let weight = r(f64::from(weight) - r(f64::from(step) / f64::from(transition)));
            if !weight.is_finite() || !(weight as f32).is_finite() {
                return Err("channel transition overflow");
            }
            channel[7] = (if weight < 0. { 0. } else { weight as f32 }).to_bits();
        }
        tick.processed |= 1 << index;
        if value.wrapped {
            tick.wrapped |= 1 << index;
        }
    }
    *channels = next;
    Ok(tick)
}
/// Apply the measured primary/secondary setter on finite transition input.
/// Old channel words are retained verbatim, including clocks and opaque bits.
/// Sentinel slots and nonfinite transitions are refused before any mutation.
pub fn select_primary(
    channels: &mut Channels,
    request: Request,
    blending: Blending,
) -> Result<(), &'static str> {
    if request.slot >= 254 || !request.blend_seconds.is_finite() {
        return Err("animation selection outside supported slot/finite-transition profile");
    }
    let transition = match blending {
        Blending::Disabled => 0.,
        Blending::Enabled => request.blend_seconds,
    };
    for index in 0..2 {
        if transition != 0. {
            if f32::from_bits(channels[index][7]) < f32::from_bits(0x3f66_6666) {
                channels[index + 2] = channels[index];
                channels[index + 2][8] |= 0x100;
            }
            channels[index][6] = transition.to_bits();
            channels[index][7] = 1f32.to_bits();
        } else {
            channels[index][6] = 0;
            channels[index][7] = 0;
        }
        channels[index][0] = if index == 0 {
            u32::from(request.slot)
        } else {
            254
        };
        channels[index][8] = if index == 0 {
            request.flags | 0x100
        } else {
            0x100
        };
        channels[index][4] = request.speed_source as u32;
        channels[index][5] = request.reference_speed.to_bits();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pause_zeroes_later_delta_and_later_failure_is_atomic() {
        let mut channels = [[0; 9]; 4];
        channels[0][8] = 0x109;
        channels[1][8] = 1;
        channels[1][1] = 0.25f32.to_bits();
        channels[2][0] = 254;
        channels[3][0] = 254;
        let old_second = channels[1];
        let tick = advance(
            &mut channels,
            &[Some(1.)],
            [0.; 3],
            0.5,
            Precision::Double53,
        )
        .unwrap();
        assert_eq!(tick.processed, 1);
        assert_eq!(channels[1], old_second);
        channels[0][8] = 1;
        channels[1][0] = 1;
        let original = channels;
        assert!(
            advance(
                &mut channels,
                &[Some(1.), Some(0.)],
                [0.; 3],
                0.5,
                Precision::Double53
            )
            .is_err()
        );
        assert_eq!(channels, original);
    }
    #[test]
    fn refusal_preserves_all_channels_and_transition_copy_keeps_old_clocks() {
        let mut channels = std::array::from_fn(|i| std::array::from_fn(|j| (i * 9 + j) as u32));
        channels[0][7] = 0f32.to_bits();
        channels[1][7] = 1f32.to_bits();
        let original = channels;
        let request = Request {
            slot: 41,
            flags: 0x21,
            speed_source: 0,
            reference_speed: 0.,
            blend_seconds: 0.5,
        };
        for bad in [
            Request {
                slot: 254,
                ..request
            },
            Request {
                blend_seconds: f32::NAN,
                ..request
            },
        ] {
            assert!(select_primary(&mut channels, bad, Blending::Enabled).is_err());
            assert_eq!(channels, original);
        }
        select_primary(&mut channels, request, Blending::Enabled).unwrap();
        let mut old = original[0];
        old[8] |= 0x100;
        assert_eq!(channels[2], old);
        assert_eq!(channels[3], original[3]);
        assert_eq!(&channels[0][1..4], &original[0][1..4]);
        assert_eq!(channels[0][8], 0x121);
        assert_eq!(channels[1][0], 254);
    }
}
