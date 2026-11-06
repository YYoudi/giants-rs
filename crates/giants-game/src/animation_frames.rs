//! D channel-to-frame selection. Pose blending and the native shared ANM-root
//! context writes remain downstream services, not fabricated matrix results.
use crate::{animation_channels::Channels, arithmetic::Precision};

#[derive(Clone, Copy, Debug)]
pub struct Descriptor {
    pub timeline: [i32; 4],
    pub flags: u32,
}
/// Standalone template clock in the acquired Double53 profile. It adds the
/// signed base before truncation, unlike the separate channel converter below.
/// End bounds and flags are not consulted by this source routine.
pub fn frame_at_time(descriptor: Descriptor, seconds: f32) -> Result<i32, &'static str> {
    if !seconds.is_finite() {
        return Err("template frame clock is nonfinite");
    }
    let rate = f64::from(descriptor.timeline[3]) * f64::from(descriptor.timeline[2]);
    let value = rate * f64::from(seconds) + f64::from(descriptor.timeline[0]);
    Ok(crate::arithmetic::integer_low_word(value) as i32)
}
#[derive(Clone, Copy, Debug)]
pub struct Frame {
    pub channel: usize,
    pub descriptor: u8,
    pub tick: i32,
    pub uses_model_binding: bool,
    pub backup: bool,
    /// Backup frame weights are not written by this native converter.
    pub transition: Option<f32>,
}

pub fn select(
    channels: &Channels,
    descriptors: &[Option<Descriptor>],
    precision: Precision,
) -> Result<Vec<Frame>, &'static str> {
    let mut result = Vec::new();
    let mut limit = 2;
    for (index, channel) in channels.iter().enumerate() {
        if index >= limit {
            break;
        }
        let slot = channel[0];
        if slot == 254 {
            continue;
        }
        if slot > 253 {
            return Err("animation frame slot outside descriptor table");
        }
        let Some(descriptor) = descriptors.get(slot as usize).copied().flatten() else {
            continue;
        };
        let seconds = f32::from_bits(channel[1]);
        if !seconds.is_finite() {
            return Err("animation frame clock is nonfinite");
        }
        let rate =
            precision.round(f64::from(descriptor.timeline[2]) * f64::from(descriptor.timeline[3]));
        let offset = precision.round(rate * f64::from(seconds)).trunc();
        if offset < f64::from(i32::MIN) || offset > f64::from(i32::MAX) {
            return Err("animation frame tick conversion overflow");
        }
        let transition = if index < 2 {
            let weight = f32::from_bits(channel[7]);
            if !weight.is_finite() || weight.abs() > 1_000_000. {
                return Err("animation frame transition outside finite profile");
            }
            let angle = precision.round(f64::from(weight) * f64::from(f32::from_bits(0x4049_0fdb)));
            let value = precision.round(precision.round(1. - angle.cos()) * 0.5);
            if value != 0. {
                limit = 4;
            }
            Some(value as f32)
        } else {
            None
        };
        result.push(Frame {
            channel: index,
            descriptor: slot as u8,
            tick: (offset as i32).wrapping_add(descriptor.timeline[0]),
            uses_model_binding: descriptor.flags & 1 == 0,
            backup: index >= 2,
            transition,
        });
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn standalone_clock_adds_base_before_conversion_and_keeps_low_word_profile() {
        let descriptor = Descriptor {
            timeline: [10, -100, 1, -1],
            flags: u32::MAX,
        };
        assert_eq!(frame_at_time(descriptor, 0.25).unwrap(), 9);
        assert_eq!(frame_at_time(descriptor, -0.25).unwrap(), 10);
        assert_eq!(frame_at_time(descriptor, -20.).unwrap(), 30);
        assert!(frame_at_time(descriptor, f32::NAN).is_err());
        let huge = Descriptor {
            timeline: [0, 0, i32::MAX, i32::MAX],
            flags: 0,
        };
        assert_eq!(frame_at_time(huge, 1e30).unwrap(), 0);
        let wrap = Descriptor {
            timeline: [0, 0, 65536, 65536],
            flags: 0,
        };
        assert_eq!(frame_at_time(wrap, 1.).unwrap(), 0);
    }
    #[test]
    fn secondary_transition_reaches_backup_and_ignores_unwritten_backup_weight() {
        let mut channels = crate::actor_context::initial_channels(None, false);
        channels[1][0] = 0;
        channels[1][7] = 0.5_f32.to_bits();
        channels[2][0] = 1;
        channels[2][7] = f32::NAN.to_bits();
        let descriptors = [Some(Descriptor {
            timeline: [0, 160, 160, 30],
            flags: 0,
        }); 2];
        let frames = select(&channels, &descriptors, Precision::Double53).unwrap();
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].channel, 1);
        assert!(frames[1].backup);
        assert!(frames[1].transition.is_none());
        channels[1][1] = f32::MAX.to_bits();
        assert!(select(&channels, &descriptors, Precision::Double53).is_err());
    }
}
