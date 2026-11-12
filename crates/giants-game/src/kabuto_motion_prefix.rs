//! Acquired input/orientation and force-to-position prefix. Force production,
//! common contacts, history dispatch and complete primary/world ticking are separate.
use crate::{actor_common_motion, actor_fields::Fields, arithmetic::Precision, mecc_motion};

#[derive(Clone, Copy, Debug, Default)]
pub struct State {
    pub commands: [u8; 8],
    /// Header mode word; later reaction/animation consumers remain separate.
    pub mode: u32,
    pub model: [f32; 4],
    pub previous_model: [f32; 4],
    pub look: f32,
    pub velocity: [f32; 3],
    pub previous_velocity: [f32; 3],
    pub target_heading: f32,
}
pub struct Frame<'a> {
    pub global_seconds: f32,
    pub areas: &'a [crate::object_time::SlowArea],
    pub sensitivity: f32,
    pub local_owner: bool,
    pub skip_heading: bool,
    pub parent_present: bool,
}
pub struct Prepared {
    pub fields: Fields,
    pub payload: Vec<u8>,
    pub state: State,
}
pub fn prepare(
    fields: &Fields,
    payload: &[u8],
    mut state: State,
    frame: Frame<'_>,
) -> Result<Prepared, String> {
    if fields.class != 1
        || payload.len() != 844
        || !frame.sensitivity.is_finite()
        || frame.sensitivity < 0.
    {
        return Err("Kabuto motion prefix outside acquired owner profile".into());
    }
    let word = |at| u32::from_le_bytes(payload[at..at + 4].try_into().unwrap());
    let scalar = |at| f32::from_bits(word(at));
    let mut flags = word(0);
    let mut buttons = u32::from_le_bytes(state.commands[4..].try_into().unwrap());
    if flags & 0x80 != 0 {
        state.commands[2] = 32;
        flags &= !0x80;
    }
    if flags & 0x300 != 0 {
        buttons &= !2;
    }
    if flags & 0x100 != 0 {
        state.commands[2] = 0;
        state.commands[3] = 0;
        flags &= !0x300;
    }
    if flags & 0x200 != 0 {
        state.commands[0] = 0;
        flags &= !0x300;
    }
    if flags & 0x400 != 0 {
        state.commands[1] = 0;
        flags &= !0x300;
    }
    state.commands[4..].copy_from_slice(&buttons.to_le_bytes());
    if state.mode != 12
        && f64::from(scalar(0x1d0)) * -30. >= f64::from(fields.position[2])
        && word(0xe4) & 0x40 != 0
    {
        state.mode = 14;
    }
    let time = crate::object_time::seconds(
        frame.global_seconds,
        Some(crate::object_time::Sample {
            owner: fields.dispatch_context,
            position: fields.position,
        }),
        None,
        frame.areas,
    );
    let rotation = actor_common_motion::orient(
        actor_common_motion::State {
            position: fields.position,
            heading: fields.angles[0],
            look_pitch: state.look,
            rates: state.model,
            previous_rates: state.previous_model,
            active: fields.object_flags,
            personality: 0,
            state: 1,
            context: fields.dispatch_context,
            parent_present: frame.parent_present,
            controls: std::array::from_fn(|i| state.commands[i] as i8),
            target_heading: state.target_heading,
            target_position: [0.; 2],
        },
        &actor_common_motion::Environment {
            parameters: actor_common_motion::Parameters {
                acceleration: 0.,
                speed: 0.,
                braking: 0.,
                attenuation: 0.,
                gravity: 0.,
                turn_acceleration: f32::from_bits(fields.configuration[5]),
                turn_speed: f32::from_bits(fields.configuration[6]),
                turn_braking: f32::from_bits(fields.configuration[7]),
            },
            global_seconds: frame.global_seconds,
            areas: frame.areas,
            control_factor: frame.sensitivity,
            running_factor: 0.,
            skip_heading: frame.skip_heading,
            controlled_actor: frame.local_owner,
        },
    )
    .map_err(str::to_owned)?;
    state.model = rotation.rates;
    state.previous_model = rotation.previous_rates;
    state.look = rotation.look_pitch;
    let integrated = mecc_motion::integrate_with_time(
        mecc_motion::Motion {
            position: fields.position,
            velocity: state.velocity,
            previous_velocity: state.previous_velocity,
            force: [scalar(0xfc), scalar(0x100), scalar(0x104)],
            mass: scalar(0xec),
            speed_limit: scalar(0x10c),
            force_time_limit: scalar(0x110),
            stored_dt: scalar(0xf8),
        },
        time,
        frame.parent_present,
        fields.response_flags & 0x1000 != 0,
        Precision::Double53,
    )
    .map_err(str::to_owned)?;
    let mut next = fields.clone();
    next.angles[0] = rotation.heading;
    next.position = mecc_motion::limit_position(integrated.position, Precision::Double53)
        .map_err(str::to_owned)?;
    state.velocity = integrated.velocity;
    state.previous_velocity = integrated.previous_velocity;
    let mut body = payload.to_vec();
    body[..4].copy_from_slice(&flags.to_le_bytes());
    for (at, value) in [
        (0xf8, integrated.stored_dt),
        (0xfc, 0.),
        (0x100, 0.),
        (0x104, 0.),
        (0x110, integrated.force_time_limit),
    ] {
        body[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }
    Ok(Prepared {
        fields: next,
        payload: body,
        state,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fields() -> Fields {
        Fields {
            class: 1,
            dispatch_context: 0,
            configuration: [0; 30],
            scale: 1.,
            object_flags: 0,
            response_flags: 0,
            control_flags: 0,
            action: 0,
            shadow_reciprocal: 0.,
            shadow: None,
            shadow_texture: None,
            gunlist: std::sync::Arc::from([]),
            channels: [[0; 9]; 4],
            position: [0., 0., 10.],
            current: [0.; 3],
            previous: [0.; 3],
            angles: [0.; 3],
        }
    }
    fn body() -> Vec<u8> {
        let mut body = vec![0; 844];
        body[0xec..0xf0].copy_from_slice(&1f32.to_le_bytes());
        body
    }
    fn frame(parent_present: bool) -> Frame<'static> {
        Frame {
            global_seconds: 0.1,
            areas: &[],
            sensitivity: 1.,
            local_owner: true,
            skip_heading: true,
            parent_present,
        }
    }
    #[test]
    fn ordered_masks_do_not_revisit_cleared_branches_or_clear_bit400() {
        let mut body = body();
        body[..4].copy_from_slice(&0x781u32.to_le_bytes());
        let state = State {
            commands: [17, 19, 21, 23, 255, 255, 255, 255],
            mode: 12,
            ..State::default()
        };
        let out = prepare(&fields(), &body, state, frame(false)).unwrap();
        assert_eq!(out.state.commands, [17, 0, 0, 0, 253, 255, 255, 255]);
        assert_eq!(
            u32::from_le_bytes(out.payload[..4].try_into().unwrap()),
            0x401
        );
    }
    #[test]
    fn parented_force_clear_preserves_time_and_conditionally_velocity() {
        let mut body = body();
        body[0xf8..0xfc].copy_from_slice(&0.7f32.to_le_bytes());
        body[0x110..0x114].copy_from_slice(&0.3f32.to_le_bytes());
        for at in [0xfc, 0x100, 0x104] {
            body[at..at + 4].copy_from_slice(&20f32.to_le_bytes());
        }
        let state = State {
            velocity: [5., 6., 7.],
            previous_velocity: [-1., -2., -3.],
            ..State::default()
        };
        for preserve in [false, true] {
            let mut fields = fields();
            if preserve {
                fields.response_flags |= 0x1000;
            }
            let out = prepare(&fields, &body, state, frame(true)).unwrap();
            assert_eq!(
                out.state.velocity,
                if preserve { state.velocity } else { [0.; 3] }
            );
            assert_eq!(
                out.state.previous_velocity,
                if preserve {
                    state.previous_velocity
                } else {
                    [0.; 3]
                }
            );
            assert_eq!(&out.payload[0xf8..0xfc], &body[0xf8..0xfc]);
            assert_eq!(&out.payload[0x110..0x114], &body[0x110..0x114]);
            assert_eq!(&out.payload[0xfc..0x108], &[0; 12]);
        }
    }
}
