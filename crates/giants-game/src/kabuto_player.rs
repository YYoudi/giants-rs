//! Cold generic-to-Kabuto field projection. This is not live type changing,
//! camera completion, actor ticking or mission readiness.
use crate::{
    actor_fields::Fields,
    animation_channels::{self, Blending, Request},
    resident_states::Id,
};

#[derive(Clone, Copy, Debug)]
pub struct ColdFrame {
    pub position: [f32; 3],
    pub current: [f32; 3],
    pub previous: [f32; 3],
    pub angles: [f32; 3],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Initialized {
    pub owner: Id,
    /// The source requests a projection reset. No graphics service has
    /// acknowledged it at this CPU boundary.
    pub camera_projection_reset_required: bool,
}

pub(crate) fn project(
    fields: &Fields,
    frame: ColdFrame,
    class_scale: f32,
    blending: Blending,
) -> Result<Fields, String> {
    if fields.class != 1
        || fields.control_flags != 0
        || fields.action != 0
        || fields.configuration[14] & 1 != 0
        || !class_scale.is_finite()
        || class_scale <= 0.
        || !frame
            .position
            .into_iter()
            .chain(frame.current)
            .chain(frame.previous)
            .chain(frame.angles)
            .all(f32::is_finite)
    {
        return Err("Kabuto player fields require a fresh finite cold frame".into());
    }
    let mut next = fields.clone();
    next.position = frame.position;
    next.previous = frame.current;
    next.current = frame.position;
    next.angles = frame.angles;
    next.angles[1] = 0.;
    next.scale = class_scale;
    // The acquired generic shell supplies context zero; this is not an
    // arbitrary live actor's inherited context or a mission mutation producer.
    next.dispatch_context = 0;
    next.configuration[14] |= 1;
    animation_channels::select_primary(
        &mut next.channels,
        Request {
            slot: 1,
            flags: 0x21,
            speed_source: 0,
            reference_speed: 0.,
            blend_seconds: f32::from_bits(0x3e2a_aaab),
        },
        blending,
    )?;
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fields() -> Fields {
        Fields {
            class: 1,
            dispatch_context: 7,
            configuration: [0; 30],
            scale: 2.,
            object_flags: 0,
            response_flags: 0x80000000,
            control_flags: 0,
            action: 0,
            shadow_reciprocal: 0.,
            shadow: None,
            gunlist: Default::default(),
            channels: [[0; 9]; 4],
            position: [0.; 3],
            current: [0.; 3],
            previous: [0.; 3],
            angles: [0.; 3],
        }
    }
    fn frame() -> ColdFrame {
        ColdFrame {
            position: [1., 2., 3.],
            current: [4., 5., 6.],
            previous: [7., 8., 9.],
            angles: [10., 20., 30.],
        }
    }
    #[test]
    fn cold_history_keeps_old_current_and_requests_the_acquired_channel() {
        let before = fields();
        let after = project(&before, frame(), 1., Blending::Enabled).unwrap();
        assert_eq!(after.position, [1., 2., 3.]);
        assert_eq!(after.current, after.position);
        assert_eq!(after.previous, [4., 5., 6.]);
        assert_eq!(after.angles, [10., 0., 30.]);
        assert_eq!(after.scale, 1.);
        assert_eq!(after.dispatch_context, 0);
        assert_eq!(after.channels[0][0], 1);
        assert_eq!(after.channels[0][8], 0x121);
        assert_eq!(after.channels[0][6], 0x3e2aaaab);
        assert_eq!(before.configuration[14], 0);
        assert_eq!(before.channels[0][0], 0);
    }
    #[test]
    fn invalid_input_and_selected_actors_are_refused_without_changing_the_source() {
        let before = fields();
        let mut invalid = frame();
        invalid.previous[1] = f32::NAN;
        assert!(project(&before, invalid, 1., Blending::Disabled).is_err());
        let selected = project(&before, frame(), 1., Blending::Disabled).unwrap();
        assert_eq!(selected.channels[0][6], 0);
        assert!(project(&selected, frame(), 1., Blending::Disabled).is_err());
        assert_eq!(before.configuration[14], 0);
        assert_eq!(before.scale, 2.);
    }
}
