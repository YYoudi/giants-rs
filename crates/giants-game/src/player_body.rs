//! Owned player-body presentation: immutable initial inspection or a completed
//! Delphi phase's shared deformation cache. Neither path advances a primary.
use crate::{
    resident_states::{Id, Session},
    scene_resources::{ModelAsset, SceneResources},
};

pub struct Prepared {
    pub model: ModelAsset,
    pub positions: Vec<[f32; 3]>,
    /// Known absence follows the acquired no-normal draw route. Preparation
    /// errors on a model declaring normals remain errors.
    pub normals: Option<Vec<[f32; 3]>>,
    pub matrix: [f32; 16],
    pub owner: Id,
    class: i32,
    object_flags: u32,
    specification: std::sync::Arc<crate::specification::Prepared>,
    channels: crate::animation_channels::Channels,
    pose: ([u32; 3], [u32; 3], u32),
    evaluated: Option<Evaluated>,
}
struct Evaluated {
    number: u64,
    epoch: u32,
    cache: [u32; 3],
}
fn cache_words(cache: crate::vertex_cache::Cache) -> [u32; 3] {
    [cache.epoch, cache.seconds.to_bits(), cache.animation]
}
fn completed_delphi_epoch(session: &Session, owner: Id, number: u64) -> Result<u32, String> {
    let actor = session.get(owner).ok_or("body owner absent or foreign")?;
    let frame = session
        .delphi_physical(owner)
        .ok_or("body Delphi phase absent")?;
    let fields = actor
        .construction_fields
        .as_ref()
        .ok_or("body fields absent")?;
    if actor.class != 3
        || number == 0
        || frame.number != number
        || frame.failure.is_some()
        || actor.mecc_animation_stamp == 0
        || actor.channels != Some(fields.channels)
        || fields.object_flags & 0x40008000 != 0
    {
        return Err("body requires its completed unfailed Delphi phase".into());
    }
    session.current_reference_matrices(owner)?;
    session.actor_current_affine(owner)?;
    Ok(actor.mecc_animation_stamp)
}
impl Prepared {
    /// Consume the actual phase/ANM owners, then run the existing model-buffer
    /// producer at that epoch. No scratch epoch, history or initializer is added.
    /// Completion of this acquired phase is not full-primary/world admission.
    pub fn from_delphi_frame(
        resources: &SceneResources,
        session: &mut Session,
        owner: Id,
        number: u64,
    ) -> Result<Self, String> {
        let epoch = completed_delphi_epoch(session, owner, number)?;
        session.prepare_actor_positions(owner, 0, epoch, false)?;
        let cache = session
            .actor_model_cache(owner, 0)?
            .ok_or("evaluated body model cache absent")?;
        if cache.epoch != epoch {
            return Err("evaluated body model epoch differs from its ANM owner".into());
        }
        let positions = session
            .actor_model_positions(owner, 0)?
            .ok_or("evaluated body positions absent")?
            .to_vec();
        Self::snapshot(
            resources,
            session,
            owner,
            Some((
                positions,
                Evaluated {
                    number,
                    epoch,
                    cache: cache_words(cache),
                },
            )),
        )
    }
    pub fn from_session(
        resources: &SceneResources,
        session: &Session,
        owner: Id,
    ) -> Result<Self, String> {
        Self::snapshot(resources, session, owner, None)
    }
    fn snapshot(
        resources: &SceneResources,
        session: &Session,
        owner: Id,
        evaluated: Option<(Vec<[f32; 3]>, Evaluated)>,
    ) -> Result<Self, String> {
        let actor = session
            .get(owner)
            .ok_or("initial body owner absent or foreign")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("initial body fields absent")?;
        let channels = actor.channels.ok_or("initial body channels absent")?;
        if fields.channels != channels {
            return Err("initial body channel owners disagree".into());
        }
        if fields.object_flags & 0x40008000 != 0 {
            return Err("initial body owner is hidden or marked".into());
        }
        let specification = session
            .specification(owner)
            .ok_or("initial body specification absent")?;
        let source = specification
            .models
            .first()
            .and_then(Option::as_ref)
            .ok_or("initial body model absent")?;
        let model = resources
            .model_slot_for_body(
                actor.class,
                0,
                crate::scene_resources::TextureEncoding::Rgb565OrArgb4444,
            )
            .map_err(|e| e.to_string())?
            .ok_or("initial body asset absent")?;
        if model.name != source.name {
            return Err("initial body asset differs from owned specification".into());
        }
        let (positions, evaluated) = if let Some((positions, provenance)) = evaluated {
            (positions, Some(provenance))
        } else {
            // Initial inspection has a private epoch, never published into
            // actor/world ANM caches or acknowledged as live phase history.
            let mut scratch = crate::actor_position_buffers::Bank::default();
            scratch.update(specification, 0, &channels, 1, false)?;
            (
                scratch
                    .positions(specification, 0)?
                    .ok_or("initial body positions absent")?
                    .to_vec(),
                None,
            )
        };
        if positions.len() != model.mesh.positions.len() {
            return Err("initial body position count differs".into());
        }
        let normals = if model.mesh.normal_count == 0 {
            None
        } else {
            Some(
                crate::body_pipeline::prepare_normals(&model.mesh, &positions, false)
                    .map_err(|e| e.to_string())?,
            )
        };
        let matrix = if evaluated.is_some() || fields.object_flags & 0x10000 != 0 {
            session.actor_current_affine(owner)?.matrix
        } else {
            crate::object_transform::euler(fields.position, fields.angles, fields.scale)
                .map_err(|e| e.to_string())?
        };
        Ok(Self {
            model,
            positions,
            normals,
            matrix,
            owner,
            class: actor.class,
            object_flags: fields.object_flags,
            specification: specification.clone(),
            channels,
            evaluated,
            pose: (
                fields.position.map(f32::to_bits),
                fields.angles.map(f32::to_bits),
                fields.scale.to_bits(),
            ),
        })
    }
    pub fn current(&self, session: &Session) -> bool {
        session.get(self.owner).is_some_and(|a| {
            a.class == self.class
                && session
                    .specification(self.owner)
                    .is_some_and(|s| std::sync::Arc::ptr_eq(s, &self.specification))
                && a.channels == Some(self.channels)
                && self.evaluated.as_ref().is_none_or(|e| {
                    completed_delphi_epoch(session, self.owner, e.number) == Ok(e.epoch)
                        && session.actor_model_cache(self.owner, 0).is_ok_and(|cache| {
                            cache.is_some_and(|cache| cache_words(cache) == e.cache)
                        })
                })
                && ((self.evaluated.is_none() && self.object_flags & 0x10000 == 0)
                    || session
                        .actor_current_affine(self.owner)
                        .is_ok_and(|c| c.matrix.map(f32::to_bits) == self.matrix.map(f32::to_bits)))
                && a.construction_fields.as_ref().is_some_and(|f| {
                    f.object_flags == self.object_flags
                        && f.object_flags & 0x40008000 == 0
                        && f.channels == self.channels
                        && (
                            f.position.map(f32::to_bits),
                            f.angles.map(f32::to_bits),
                            f.scale.to_bits(),
                        ) == self.pose
                })
        })
    }
    pub fn vertices(&self, part: usize) -> Vec<[f32; 3]> {
        self.model.mesh.subobjects[part]
            .triangles
            .iter()
            .flatten()
            .map(|id| {
                let q = self.positions[usize::from(self.model.mesh.indices[usize::from(*id)])];
                let p: [f32; 3] = std::array::from_fn(|r| {
                    self.matrix[r * 4] * q[0]
                        + self.matrix[r * 4 + 1] * q[1]
                        + self.matrix[r * 4 + 2] * q[2]
                        + self.matrix[r * 4 + 3]
                });
                [p[0], p[2], -p[1]]
            })
            .collect()
    }
}
