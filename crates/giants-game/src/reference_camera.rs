//! Authored camera-reference selection over owned poses and prepared resources.
//! Publication, target selection and ordinary camera updates remain separate.
use crate::{
    actor_references::Storage, animation, animation_epoch::Stamp, animation_references,
    authored_camera_reference, procedural_aim::Matrix, specification::Prepared,
};

pub struct Source<'a> {
    pub specification: &'a Prepared,
    pub current_clip: u8,
    pub current_seconds: f32,
    pub world: Option<Matrix>,
    pub references: Storage<'a>,
}

/// Resolve one declared reference. A missing node returns absence so the actual
/// caller can select its fallback; no identity matrix is supplied for absence.
pub fn sample(
    source: &Source<'_>,
    clip: u8,
    reference: usize,
    aspect: f32,
    stamp: &mut Stamp,
) -> Result<Option<authored_camera_reference::Output>, String> {
    if reference >= source.specification.metadata.references.names.len() {
        return Ok(None);
    }
    if !source.specification.needs.is_empty() {
        return Err("camera specification has unresolved resources".into());
    }
    let descriptor = source
        .specification
        .animations
        .get(&clip)
        .ok_or("camera animation descriptor absent")?;
    let mut binding = source
        .specification
        .metadata
        .references
        .bind(&descriptor.animation)
        .map_err(|e| e.to_string())?;
    let resolution = binding.resolve(reference).map_err(|e| e.to_string())?;
    let node = match resolution {
        animation_references::Resolution::Found(node)
        | animation_references::Resolution::Cached(node) => node,
        animation_references::Resolution::Missing | animation_references::Resolution::Outside => {
            return Ok(None);
        }
        animation_references::Resolution::Undeclared => {
            return Err("camera reference name is undeclared".into());
        }
    };
    let local = if clip == source.current_clip {
        if !source.references.sampled {
            return Err("current camera reference has not been sampled".into());
        }
        *source
            .references
            .matrices
            .get(reference)
            .ok_or("current camera reference buffer absent")?
    } else {
        // A non-current descriptor advances the same epoch as ordinary ANM,
        // and evaluates its hierarchy at the declared start frame.
        stamp.advance();
        animation_references::node_matrix(
            &descriptor.animation,
            node,
            descriptor.animation.timeline[0],
        )
        .map_err(|e| e.to_string())?
    };
    let horizontal_fov = if descriptor
        .animation
        .node(node)
        .map_err(|e| e.to_string())?
        .camera
        .is_some()
    {
        let tick = crate::animation_frames::frame_at_time(
            crate::animation_frames::Descriptor {
                timeline: descriptor.animation.timeline,
                flags: descriptor.template.flags,
            },
            source.current_seconds,
        )?;
        Some(
            animation::Evaluator {
                animation: &descriptor.animation,
            }
            .camera_fov(node, tick)
            .map_err(|e| e.to_string())?,
        )
    } else {
        None
    };
    authored_camera_reference::compose(
        source.world.ok_or("camera world cache absent")?,
        local,
        horizontal_fov,
        aspect,
    )
    .map(Some)
    .map_err(Into::into)
}
