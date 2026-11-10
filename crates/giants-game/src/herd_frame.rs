//! Bounded herd/marker common-node composition. World admission and population
//! traversal remain upstream; environment and footprint services are required.
use crate::{actor_fields::Fields, actor_finalization, collider_copy, resident_states::Id};

pub struct Frame<'a> {
    pub number: u64,
    pub global_seconds: f32,
    pub areas: &'a [crate::object_time::SlowArea],
    pub controlled: Option<Id>,
}

pub struct Services<'a> {
    pub factory: &'a crate::vimp_actor::Factory,
    pub globals: &'a giants_formats::global_data::GlobalData,
    pub random: &'a mut crate::audio_parameters::Random,
    pub ground: &'a mut dyn crate::herd_generator::Ground,
    pub stamp: &'a mut crate::animation_epoch::Stamp,
    pub finalization: &'a mut dyn actor_finalization::Services,
}

#[derive(Debug)]
pub struct Projection {
    pub outcome: crate::actor_world_tick::Outcome,
    pub born: Vec<Id>,
}

#[derive(Clone, Debug)]
pub struct View {
    /// Next local callback number, independent of a supplied world clock.
    pub next_frame: u64,
    pub finalization: actor_finalization::State,
    pub rates: [f32; 4],
    pub previous_rates: [f32; 4],
    pub failure: Option<String>,
}

#[derive(Debug)]
pub(crate) struct Owned {
    pub view: View,
    pub history: Option<u64>,
    pub stopped: bool,
    pub primary_steps: u64,
}
impl Owned {
    /// Actual source constructors clear the nonphysical motion, contact and
    /// affine fields. Placements provide the declared header, not physics.
    pub fn fresh(
        fields: &Fields,
        specification: &crate::specification::Prepared,
    ) -> Result<Self, String> {
        if !matches!(
            (fields.class, fields.configuration[20]),
            (49, 17) | (679, 0)
        ) || fields.channels.iter().any(|c| c[0] != 254)
            || !specification.metadata.references.names.is_empty()
            || fields.object_flags & 0x10000 != 0
        {
            return Err("herd frame requires source nonphysical herd/basic marker with empty channels and references".into());
        }
        let c = fields.configuration;
        let mut f = collider_copy::Fields::zeroed();
        f.class = fields.class as u32;
        f.model = specification.specification;
        f.current = fields.position;
        f.previous = fields.current;
        f.scale = fields.scale;
        f.angles = fields.angles;
        f.object_flags = fields.object_flags;
        f.response_flags = fields.response_flags;
        f.collision_flags = c[14];
        f.ground_offset = f32::from_bits(c[17]);
        f.center_offset = f32::from_bits(c[22]);
        f.height = f32::from_bits(c[23]);
        f.packed_parameters = c[25];
        f.radius_limit = f32::from_bits(c[26]);
        f.radius_override = f32::from_bits(c[27]);
        f.animation_slot = 254;
        Ok(Self {
            view: View {
                next_frame: 0,
                finalization: actor_finalization::State {
                    fields: f,
                    previous_physical_velocity: [0.; 3],
                    friction: f32::from_bits(c[28]),
                    parent_present: false,
                    footprint: None,
                    affine: None,
                    physical_argument_word: None,
                },
                rates: [0.; 4],
                previous_rates: [0.; 4],
                failure: None,
            },
            history: None,
            stopped: false,
            primary_steps: 0,
        })
    }
}

/// Empty descriptor slots still enter ANM's channel loop. An entered channel
/// clears its force flag and triggers the shared epoch even without a pose.
/// The owning adapter must separately establish the empty reference set.
pub fn advance_empty(
    channels: &mut crate::animation_channels::Channels,
    seconds: f32,
    stamp: &mut crate::animation_epoch::Stamp,
    previous_stamp: u32,
) -> Result<u32, String> {
    if !seconds.is_finite() || seconds < 0. || channels.iter().any(|c| c[0] != 254) {
        return Err("herd ANM requires finite nonnegative time and empty source channels".into());
    }
    let mut entered = false;
    for channel in channels {
        if seconds != 0. || channel[8] & 0x100 != 0 {
            channel[8] &= !0x100;
            entered = true;
        }
    }
    Ok(if entered {
        stamp.advance()
    } else {
        previous_stamp
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_actor_epoch_requires_an_entered_channel_and_preserves_refusals() {
        let mut channels = [[0; 9]; 4];
        for channel in &mut channels {
            channel[0] = 254;
        }
        let mut stamp = crate::animation_epoch::Stamp(u32::MAX);
        assert_eq!(
            advance_empty(&mut channels, -0., &mut stamp, 0x1234).unwrap(),
            0x1234
        );
        assert_eq!(stamp.0, u32::MAX);
        channels[3][8] = 0x2184;
        assert_eq!(
            advance_empty(&mut channels, 0., &mut stamp, 0x1234).unwrap(),
            1
        );
        assert_eq!(channels[3][8], 0x2084);
        assert_eq!(
            advance_empty(&mut channels, 0.25, &mut stamp, 1).unwrap(),
            2
        );
        channels[0][0] = 0;
        let previous = channels;
        assert!(advance_empty(&mut channels, 0.25, &mut stamp, 2).is_err());
        assert_eq!(channels, previous);
        assert_eq!(stamp.0, 2);
        channels[0][0] = 254;
        assert!(advance_empty(&mut channels, f32::NAN, &mut stamp, 2).is_err());
        assert!(advance_empty(&mut channels, -1., &mut stamp, 2).is_err());
        assert_eq!(stamp.0, 2);
    }
}
