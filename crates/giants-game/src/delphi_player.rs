//! Acquired cold local-player fields around the Delphi special phase.
//! Projection submission, live type changing and player scheduling are separate.
use crate::{
    actor_fields::Fields,
    animation_channels::{self, Blending, Channels, Request},
    delphi_special,
    resident_states::Id,
};

#[derive(Clone, Copy, Debug)]
pub struct ColdFrame {
    pub position: [f32; 3],
    /// Previous frame's current position becomes the new history entry.
    pub current: [f32; 3],
    /// Supplied shell history is validated, then replaced by `current`.
    pub previous: [f32; 3],
    pub angles: [f32; 3],
}

#[derive(Clone, Debug)]
pub struct Initialized {
    pub owner: Id,
    pub special: delphi_special::Initialized,
    /// Required by the acquired caller, not acknowledged by a renderer here.
    pub camera_projection_reset_required: bool,
}
/// Retain the actual published constructor identity after a later failure.
/// A missing owner means construction itself never published an actor.
#[derive(Debug)]
pub struct Attempt {
    pub owner: Option<Id>,
    pub result: Result<Initialized, String>,
}

pub(crate) fn prepare(template: &Fields, frame: ColdFrame) -> Result<Fields, String> {
    if template.class != 3
        || template.configuration[14] & 1 != 0
        || template.control_flags != 0
        || template.action != 0
        || !template.scale.is_finite()
        || template.scale <= 0.
        || !frame
            .position
            .into_iter()
            .chain(frame.current)
            .chain(frame.previous)
            .chain(frame.angles)
            .all(f32::is_finite)
    {
        return Err("Delphi player requires a fresh template and finite cold frame".into());
    }
    let mut fields = template.instantiate();
    fields.position = frame.position;
    fields.current = frame.position;
    fields.previous = frame.current;
    fields.angles = frame.angles;
    fields.angles[1] = 0.;
    // The cold generic shell supplies context zero. Arbitrary live actor
    // inheritance, inventory teardown and dynamic type changes are not admitted.
    fields.dispatch_context = 0;
    Ok(fields)
}

pub(crate) fn selected(channels: Channels, blending: Blending) -> Result<Channels, String> {
    let mut next = channels;
    animation_channels::select_primary(
        &mut next,
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
    #[test]
    fn cold_frame_replaces_history_but_refuses_nonfinite_discarded_inputs() {
        let template = Fields {
            class: 3,
            dispatch_context: 7,
            configuration: [0; 30],
            scale: 1.,
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
        };
        let mut frame = ColdFrame {
            position: [1., 2., 3.],
            current: [4., 5., 6.],
            previous: [7., 8., 9.],
            angles: [10., 20., 30.],
        };
        let fields = prepare(&template, frame).unwrap();
        assert_eq!(fields.position, frame.position);
        assert_eq!(fields.current, frame.position);
        assert_eq!(fields.previous, frame.current);
        assert_eq!(fields.angles, [10., 0., 30.]);
        assert_eq!(fields.dispatch_context, 0);
        assert_eq!(fields.configuration[14] & 1, 0);
        frame.previous[0] = f32::NAN;
        assert!(prepare(&template, frame).is_err());
        assert_eq!(template.dispatch_context, 7);
        assert_eq!(template.position, [0.; 3]);
    }
}
