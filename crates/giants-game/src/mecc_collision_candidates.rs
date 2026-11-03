//! First-pass Mecc object candidates, not a narrow-phase response or live list builder.
use crate::{
    arithmetic::Precision,
    volume_queries::{self, Bounds},
};
pub struct Actor {
    pub id: u32,
    pub current: [f32; 3],
    pub previous: [f32; 3],
    pub radius: f32,
    pub scale: f32,
    pub height: f32,
    pub object_flags: u32,
    pub collision_flags: u32,
    pub has_model: bool,
}
#[derive(Clone, Copy)]
pub struct Object {
    pub id: u32,
    pub object_flags: u32,
    pub collision_flags: u32,
    pub has_model: bool,
    pub parent: u32,
    pub bounds: Bounds,
}
pub struct Pass {
    pub solid: Bounds,
    pub platform: Bounds,
    /// Supplied list order is retained. A candidate still needs geometric response.
    pub candidates: Vec<u32>,
}
pub fn select(
    actor: &Actor,
    objects: &[Object],
    precision: Precision,
) -> Result<Option<Pass>, &'static str> {
    validate(actor)?;
    if actor.object_flags & 0x1000_0000 == 0 && actor.current == actor.previous {
        return Ok(None);
    }
    select_pass(actor, objects, precision)
}
/// Caller has passed the dispatcher entry gate; retry passes do not repeat it.
pub(crate) fn select_pass(
    actor: &Actor,
    objects: &[Object],
    precision: Precision,
) -> Result<Option<Pass>, &'static str> {
    validate(actor)?;
    select_fields(actor, objects, precision)
}
fn validate(actor: &Actor) -> Result<(), &'static str> {
    if actor.id == 0
        || !actor
            .current
            .into_iter()
            .chain(actor.previous)
            .chain([actor.radius, actor.scale, actor.height])
            .all(f32::is_finite)
    {
        return Err("invalid Mecc candidate input");
    }
    Ok(())
}
fn select_fields(
    actor: &Actor,
    objects: &[Object],
    precision: Precision,
) -> Result<Option<Pass>, &'static str> {
    if !actor.has_model || actor.collision_flags & 0x600 != 0 {
        return Ok(None);
    }
    let r = |v| precision.round(v);
    let min: [f32; 3] = std::array::from_fn(|i| {
        if actor.previous[i] <= actor.current[i] {
            actor.previous[i]
        } else {
            actor.current[i]
        }
    });
    let max: [f32; 3] = std::array::from_fn(|i| {
        if actor.previous[i] <= actor.current[i] {
            actor.current[i]
        } else {
            actor.previous[i]
        }
    });
    let mut platform = Bounds {
        min: [
            r(f64::from(min[0]) - f64::from(actor.radius)) as f32,
            r(f64::from(min[1]) - f64::from(actor.radius)) as f32,
            min[2],
        ],
        max: [
            r(f64::from(max[0]) + f64::from(actor.radius)) as f32,
            r(f64::from(max[1]) + f64::from(actor.radius)) as f32,
            r(r(f64::from(actor.scale) * f64::from(actor.height)) + f64::from(max[2])) as f32,
        ],
    };
    let mut solid = platform;
    if actor.object_flags & 0x80000 != 0 {
        solid.min[2] = r(f64::from(solid.min[2]) + 2.) as f32;
        platform.min[2] = r(f64::from(platform.min[2]) - 1.) as f32;
    }
    if !platform
        .min
        .into_iter()
        .chain(platform.max)
        .chain(solid.min)
        .all(f32::is_finite)
    {
        return Err("Mecc candidate bounds overflow");
    }
    let mut candidates = Vec::new();
    for object in objects {
        if object.id == actor.id
            || object.object_flags & 0x4000_c000 != 0
            || object.collision_flags & 0x800 != 0
            || !object.has_model
            || object.parent == actor.id
        {
            continue;
        }
        let swept = if object.collision_flags & 8 != 0 {
            platform
        } else {
            solid
        };
        if volume_queries::overlaps(swept, object.bounds).ok_or("non-finite candidate bounds")? {
            candidates.push(object.id);
        }
    }
    Ok(Some(Pass {
        solid,
        platform,
        candidates,
    }))
}
