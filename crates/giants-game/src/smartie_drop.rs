//! Ordinary state18 detachment from a Mecc, with supplied service inputs.
//! This is the detach core, not backpack eligibility, actor construction,
//! subsequent physics, speech/audio playback or mission readiness.
#[derive(Clone, Debug, PartialEq)]
pub struct Actor {
    pub identity: u64,
    pub class: i32,
    pub state: u32,
    pub action: u32,
    pub object_flags: u32,
    pub control_flags: u32,
    pub collision_flags: u32,
    pub behavior: u32,
    pub scale: f32,
    pub timer_word: u32,
    pub horizontal_velocity: [f32; 2],
    pub position: [f32; 3],
    pub current: [f32; 3],
    pub previous: [f32; 3],
    pub parent: Option<u64>,
    pub binding: Option<u64>,
    /// Opaque field184. Its broader service identity is not inferred here.
    pub word_184: u32,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Payload {
    pub owned_child: Option<u64>,
    /// Opaque additive correction, not an assumed inverse pickup multiplier.
    pub scale_addend: f32,
    pub word_54: u32,
    pub chatter: [u8; 32],
}
#[derive(Clone, Debug, PartialEq)]
pub struct Parent {
    pub identity: u64,
    pub state: u32,
    pub response_flags: u32,
    pub yaw_degrees: f32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chatter {
    DropOff,
    DropBase,
}
impl Chatter {
    pub fn key(self) -> &'static str {
        match self {
            Self::DropOff => "SmChDropoff",
            Self::DropBase => "SmChDropBase",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Request {
    Animation(crate::animation_channels::Request),
    GroundHeight {
        position: [f32; 3],
    },
    AttachmentSound {
        owner: u64,
        id: u32,
    },
    /// Lookup parameters1,4, then display parameters4,0. Speaker bytes
    /// already exist in the payload; this service does not replace them.
    Chatter {
        kind: Chatter,
        speaker: [u8; 32],
    },
    OwnerSound {
        owner: u64,
        id: u32,
    },
}
pub struct Input {
    pub mode: u32,
    pub random_100: u32,
    pub offline: bool,
    /// Actual ground/collision service result, only used by action2.
    pub ground_height: Option<f32>,
}
pub fn detach_mecc(
    actor: &mut Actor,
    payload: &mut Payload,
    parent: &mut Parent,
    input: Input,
) -> Result<Vec<Request>, &'static str> {
    if actor.state != 18 || parent.state != 2 || !input.offline {
        return Err("detach requires offline state18 / Mecc parent profile");
    }
    if actor.identity == 0
        || parent.identity == 0
        || actor.identity == parent.identity
        || (actor.parent != Some(parent.identity) && actor.binding != Some(parent.identity))
    {
        return Err("detach requires distinct owned linked actors");
    }
    if payload.owned_child.is_some() || matches!(actor.action, 5 | 10) {
        return Err("detach requires owned-child or special-action services");
    }
    if input.random_100 >= 100
        || !parent.yaw_degrees.is_finite()
        || parent.yaw_degrees.abs() > 1_000_000.
        || !actor.scale.is_finite()
        || !payload.scale_addend.is_finite()
    {
        return Err("detach inputs outside supplied finite profile");
    }
    let scale = (f64::from(actor.scale) + f64::from(payload.scale_addend)) as f32;
    if !scale.is_finite() || scale <= 0. {
        return Err("detach scale result outside positive finite profile");
    }
    let chatter = if actor.class == 11 {
        match input.mode {
            1 => Some(Chatter::DropOff),
            5 => Some(Chatter::DropBase),
            _ => None,
        }
    } else {
        None
    };
    if chatter.is_some() && !payload.chatter.contains(&0) {
        return Err("detach speaker name is not terminated within owned buffer");
    }
    let ground = if actor.action == 2 {
        let height = input
            .ground_height
            .ok_or("detach ground service result not supplied")?;
        if !height.is_finite()
            || !actor
                .position
                .into_iter()
                .chain(actor.current)
                .chain(actor.previous)
                .all(f32::is_finite)
        {
            return Err("detach ground profile requires finite positions and height");
        }
        Some(height)
    } else {
        None
    };
    let mut requests = Vec::new();
    if let Some(height) = ground {
        requests.push(Request::Animation(crate::animation_channels::Request {
            slot: 1,
            flags: 0x21,
            speed_source: 0,
            reference_speed: 0.,
            blend_seconds: f32::from_bits(0x3e2a_aaab),
        }));
        requests.push(Request::GroundHeight {
            position: actor.position,
        });
        actor.control_flags |= 0x11;
        actor.position[2] = height;
        actor.previous = actor.current;
        actor.current = actor.position;
    }
    let angle = (f64::from(parent.yaw_degrees) + 180.) as f32;
    let (sine, cosine) = crate::object_transform::sincos(angle);
    actor.horizontal_velocity = [(sine * 30.) as f32, (cosine * -30.) as f32];
    actor.parent = None;
    actor.binding = None;
    actor.word_184 = 0;
    actor.object_flags &= !0x4008_0010;
    actor.control_flags &= !0x20;
    actor.collision_flags = (actor.collision_flags & !0x800) | 0x2_0000;
    parent.response_flags &= !0x80_0000;
    actor.timer_word = 1_f32.to_bits();
    actor.behavior = 4;
    actor.scale = scale;
    payload.word_54 = 0;
    requests.push(Request::AttachmentSound {
        owner: parent.identity,
        id: 1509,
    });
    if let Some(kind) = chatter {
        requests.push(Request::Chatter {
            kind,
            speaker: payload.chatter,
        });
    }
    requests.push(Request::OwnerSound {
        owner: actor.identity,
        id: 1707 + input.random_100 / 25,
    });
    Ok(requests)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unsupported_lifecycle_services_refuse_before_any_mutation() {
        let actor = Actor {
            identity: 1,
            class: 11,
            state: 18,
            action: 3,
            object_flags: 0,
            control_flags: 0,
            collision_flags: 0,
            behavior: 19,
            scale: 1.,
            timer_word: 0,
            horizontal_velocity: [0.; 2],
            position: [0.; 3],
            current: [0.; 3],
            previous: [0.; 3],
            parent: Some(2),
            binding: Some(2),
            word_184: 0,
        };
        let payload = Payload {
            owned_child: None,
            scale_addend: 0.2,
            word_54: 42,
            chatter: [0; 32],
        };
        let parent = Parent {
            identity: 2,
            state: 2,
            response_flags: 0x800000,
            yaw_degrees: 0.,
        };
        for case in 0..10 {
            let (mut a, mut p, mut o) = (actor.clone(), payload.clone(), parent.clone());
            let mut input = Input {
                mode: 1,
                random_100: 0,
                offline: true,
                ground_height: None,
            };
            match case {
                0 => a.action = 2,
                1 => a.action = 5,
                2 => a.action = 10,
                3 => p.owned_child = Some(3),
                4 => o.state = 3,
                5 => input.offline = false,
                6 => input.random_100 = 100,
                7 => p.chatter = [b'A'; 32],
                8 => a.identity = 0,
                _ => o.yaw_degrees = 2_000_000.,
            }
            let before = (a.clone(), p.clone(), o.clone());
            assert!(detach_mecc(&mut a, &mut p, &mut o, input).is_err());
            assert_eq!((a, p, o), before);
        }
    }
}
