//! Initial state18 pickup by a state2 parent. The caller owns both actors.
//! This transition does not construct an actor, execute its callbacks, schedule
//! animation41, retire a previous passenger or acknowledge mission readiness.
//! Returned service requests must be consumed by an actual actor backend.
use crate::object_reference::{self, Object};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Binding {
    pub parent: u64,
    /// Historical field name: this is the child reference index, not a lock mode.
    pub kind: u32,
    /// Parent reference index.
    pub reference: u32,
}
pub const MECC_CHILD_REFERENCE: u32 = 1;
pub const MECC_PARENT_REFERENCE: u32 = 10;

#[derive(Clone, Debug, PartialEq)]
pub struct Child {
    pub identity: u64,
    pub state: u32,
    pub object_flags: u32,
    pub collision_flags: u32,
    pub behavior: u32,
    pub scale: f32,
    pub position: [f32; 3],
    pub current: [f32; 3],
    pub previous: [f32; 3],
    /// Four adjacent opaque bytes; pickup preserves the second byte.
    pub channels: [u8; 4],
    pub timer: u32,
    pub parent: Option<u64>,
    pub binding: Option<Binding>,
    pub footprint: Option<u64>,
    /// A nonempty linked collection needs its real pool/ownership service.
    pub has_links: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Payload {
    /// Opaque state18 words, starting at its parameter18.
    pub parameters: [u32; 5],
    pub scale: f32,
    pub pickup_time: u32,
}

impl Payload {
    pub fn read(data: &[u8]) -> Result<Self, &'static str> {
        if data.len() != 200 {
            return Err("state18 pickup payload size differs");
        }
        let word = |at| u32::from_le_bytes(data[at..at + 4].try_into().unwrap());
        Ok(Self {
            parameters: std::array::from_fn(|i| word(0x18 + i * 4)),
            scale: f32::from_bits(word(0x50)),
            pickup_time: word(0x94),
        })
    }
    pub fn write(&self, data: &mut [u8]) -> Result<(), &'static str> {
        if data.len() != 200 {
            return Err("state18 pickup payload size differs");
        }
        for (at, value) in self
            .parameters
            .iter()
            .enumerate()
            .map(|(i, v)| (0x18 + i * 4, *v))
            .chain([(0x50, self.scale.to_bits()), (0x94, self.pickup_time)])
        {
            data[at..at + 4].copy_from_slice(&value.to_le_bytes());
        }
        Ok(())
    }
}

pub struct Parent<'a> {
    pub identity: u64,
    pub state: u32,
    pub response_flags: u32,
    pub pose: Object<'a>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Request {
    /// Position changed: invalidate the supplied footprint, even when absent.
    InvalidateFootprint(Option<u64>),
    /// Resolve the attachment root before the normal player/remote sound route.
    AttachmentSound { owner: u64, id: u32 },
    /// Pass this actor directly to the owner-aware catalogue wrapper.
    OwnerSound { owner: u64, id: u32 },
    /// Local mission-notice consumer; the native callee ignores the supplied
    /// child argument. This identity retains the caller's ownership context.
    /// Pending/gates/counter/localization and message display remain external.
    LocalPickup { child: u64 },
}

pub struct Input {
    pub local_player: Option<u64>,
    pub time_word: u32,
    /// Supplied result of the original random(100) request. Live RNG phase is
    /// not inferred from a cold seed or from the serialized mission.
    pub random_100: u32,
}

pub use crate::animation_channels::Request as AnimationRequest;
pub const MECC_PICKUP_ANIMATION: AnimationRequest = AnimationRequest {
    slot: 41,
    flags: 0x21,
    speed_source: 0,
    reference_speed: 0.,
    blend_seconds: 0.5,
};
#[derive(Clone, Copy, Debug)]
pub struct FreshContext {
    pub action: u32,
    pub control_flags: u32,
    pub networked: bool,
}
pub struct FreshPickup {
    pub services: Vec<Request>,
    pub animation: AnimationRequest,
}
/// Fresh command12 / offline wrapper profile. Prior carrying, special actions
/// and network replication need their actual cleanup/event services.
/// The animation setter is requested, not simulated as a completed transition.
pub fn attach_fresh_mecc(
    child: &mut Child,
    payload: &mut Payload,
    parent: &mut Parent<'_>,
    input: Input,
    context: FreshContext,
) -> Result<FreshPickup, &'static str> {
    if context.action != 3 || context.control_flags & 8 != 0 || context.networked {
        return Err("pickup wrapper requires fresh offline command12 context");
    }
    let services = attach_mecc(child, payload, parent, input)?;
    Ok(FreshPickup {
        services,
        animation: MECC_PICKUP_ANIMATION,
    })
}

/// Validate first, then mutate the independently owned fields. Only the fresh
/// state18 / Mecc profile is admitted; replacement and other parent types need
/// their own verified service paths. All numeric pose inputs are source-space.
pub fn attach_mecc(
    child: &mut Child,
    payload: &mut Payload,
    parent: &mut Parent<'_>,
    input: Input,
) -> Result<Vec<Request>, &'static str> {
    let mut owner = Owner {
        identity: parent.identity,
        state: parent.state,
        response_flags: parent.response_flags,
    };
    let position = object_reference::point(&parent.pose, MECC_PARENT_REFERENCE as i32)?;
    let requests = attach_mecc_at_point(child, payload, &mut owner, input, position)?;
    parent.response_flags = owner.response_flags;
    Ok(requests)
}

/// Ownership fields for a caller that supplies the actual reference10 result.
pub struct Owner {
    pub identity: u64,
    pub state: u32,
    pub response_flags: u32,
}
/// The supplied point is a service result, never an invented actor placement.
/// Geometry, RNG phase, audio and mission-notice execution remain external.
pub fn attach_mecc_at_point(
    child: &mut Child,
    payload: &mut Payload,
    parent: &mut Owner,
    input: Input,
    position: [f32; 3],
) -> Result<Vec<Request>, &'static str> {
    if child.state != 18 || parent.state != 2 {
        return Err("pickup outside state18 child / Mecc parent profile");
    }
    if child.identity == 0 || parent.identity == 0 || child.identity == parent.identity {
        return Err("pickup requires distinct owned actor identities");
    }
    if child.parent.is_some() || child.binding.is_some() || child.has_links {
        return Err("pickup requires previous attachment and link services");
    }
    if input.random_100 >= 100 {
        return Err("pickup requires a random(100) result");
    }
    if !child.scale.is_finite()
        || child.scale <= 0.
        || !child
            .position
            .into_iter()
            .chain(child.current)
            .chain(child.previous)
            .all(f32::is_finite)
    {
        return Err("pickup outside finite positive-scale actor profile");
    }
    if !position.into_iter().all(f32::is_finite) {
        return Err("pickup requires a finite parent reference position");
    }
    let scale = (f64::from(child.scale) * f64::from(0.8_f32)) as f32;
    let mut requests = Vec::new();
    if position != child.position {
        requests.push(Request::InvalidateFootprint(child.footprint));
        child.footprint = None;
    }
    child.object_flags = (child.object_flags & !0x0b00_0000) | 0x10;
    child.collision_flags = (child.collision_flags & !0x2_0000) | 0x800;
    child.behavior = 19;
    child.parent = Some(parent.identity);
    child.binding = Some(Binding {
        parent: parent.identity,
        kind: MECC_CHILD_REFERENCE,
        reference: MECC_PARENT_REFERENCE,
    });
    child.channels[0] = 0;
    child.channels[2] = 0;
    child.channels[3] = 0;
    child.timer = 0;
    child.scale = scale;
    child.previous = child.current;
    child.current = position;
    child.position = position;
    payload.parameters = [0, u32::MAX, u32::MAX, u32::MAX, u32::MAX];
    payload.scale = scale;
    payload.pickup_time = input.time_word;
    parent.response_flags |= 0x80_0000;
    requests.push(Request::AttachmentSound {
        owner: parent.identity,
        id: 1510,
    });
    if input.local_player == Some(parent.identity) {
        requests.push(Request::LocalPickup {
            child: child.identity,
        });
    }
    requests.push(Request::OwnerSound {
        owner: child.identity,
        id: if input.random_100 < 50 { 1705 } else { 1706 },
    });
    Ok(requests)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn owned_payload_codec_keeps_opaque_bytes_and_refuses_partial_destinations() {
        let mut data = [0xa5; 200];
        let before = data;
        let payload = Payload {
            parameters: [0, 1, 2, 3, u32::MAX],
            scale: 0.8,
            pickup_time: 123,
        };
        payload.write(&mut data).unwrap();
        assert_eq!(Payload::read(&data).unwrap(), payload);
        for i in 0..200 {
            if !(0x18..0x2c).contains(&i)
                && !(0x50..0x54).contains(&i)
                && !(0x94..0x98).contains(&i)
            {
                assert_eq!(data[i], before[i]);
            }
        }
        let snapshot = data;
        assert!(payload.write(&mut data[..199]).is_err());
        assert_eq!(data, snapshot);
        assert!(Payload::read(&data[..199]).is_err());
    }

    #[test]
    fn refused_services_and_invalid_inputs_leave_both_actors_unchanged() {
        for invalid in 0..10 {
            let mut child = Child {
                identity: 1,
                state: 18,
                object_flags: u32::MAX,
                collision_flags: u32::MAX,
                behavior: 3,
                scale: 0.95,
                position: [1., 2., 3.],
                current: [4., 5., 6.],
                previous: [7., 8., 9.],
                channels: [1, 2, 3, 4],
                timer: 123,
                parent: None,
                binding: None,
                footprint: Some(3),
                has_links: false,
            };
            let mut payload = Payload {
                parameters: [1, 2, 3, 4, 5],
                scale: 1.,
                pickup_time: 456,
            };
            let mut parent = Parent {
                identity: 2,
                state: 2,
                response_flags: 123,
                pose: Object {
                    position: [10., 20., 30.],
                    angles: [0.; 3],
                    scale: 1.,
                    flags: 0x20,
                    cached: [0.; 16],
                    references: &[],
                },
            };
            let mut input = Input {
                local_player: Some(2),
                time_word: 789,
                random_100: 49,
            };
            if invalid == 0 {
                for context in [
                    FreshContext {
                        action: 5,
                        control_flags: 0,
                        networked: false,
                    },
                    FreshContext {
                        action: 3,
                        control_flags: 8,
                        networked: false,
                    },
                    FreshContext {
                        action: 3,
                        control_flags: 0,
                        networked: true,
                    },
                ] {
                    let original = (child.clone(), payload.clone(), parent.response_flags);
                    assert!(
                        attach_fresh_mecc(
                            &mut child,
                            &mut payload,
                            &mut parent,
                            Input {
                                local_player: Some(2),
                                time_word: 789,
                                random_100: 49
                            },
                            context
                        )
                        .is_err()
                    );
                    assert_eq!(
                        (&child, &payload, parent.response_flags),
                        (&original.0, &original.1, original.2)
                    );
                }
            }
            match invalid {
                0 => child.has_links = true,
                1 => child.parent = Some(2),
                2 => {
                    child.binding = Some(Binding {
                        parent: 2,
                        kind: 1,
                        reference: 10,
                    })
                }
                3 => child.state = 44,
                4 => parent.state = 3,
                5 => child.identity = 0,
                6 => parent.identity = child.identity,
                7 => input.random_100 = 100,
                8 => child.scale = 0.,
                9 => parent.pose.position[0] = f32::NAN,
                _ => unreachable!(),
            }
            let before_child = child.clone();
            let before_payload = payload.clone();
            let before_flags = parent.response_flags;
            assert!(attach_mecc(&mut child, &mut payload, &mut parent, input).is_err());
            assert_eq!(child, before_child);
            assert_eq!(payload, before_payload);
            assert_eq!(parent.response_flags, before_flags);
        }
    }
}
