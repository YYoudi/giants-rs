//! Initial owned player-body presentation. This samples actual current channels
//! without advancing a primary, acknowledging initializers or inventing history.
use crate::{
    resident_states::{Id, Session},
    scene_resources::{ModelAsset, SceneResources},
};

pub struct Prepared {
    pub model: ModelAsset,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub matrix: [f32; 16],
    pub owner: Id,
    class: i32,
    object_flags: u32,
    specification: std::sync::Arc<crate::specification::Prepared>,
    channels: crate::animation_channels::Channels,
    pose: ([u32; 3], [u32; 3], u32),
}
impl Prepared {
    pub fn from_session(
        resources: &SceneResources,
        session: &Session,
        owner: Id,
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
        // A fresh evaluation scope for this immutable presentation snapshot.
        // Its nonzero scratch epoch is never published into the actor or world
        // ANM caches, and acknowledges no live animation or primary history.
        let mut scratch = crate::actor_position_buffers::Bank::default();
        scratch.update(specification, 0, &channels, 1, false)?;
        let positions = scratch
            .positions(specification, 0)?
            .ok_or("initial body positions absent")?
            .to_vec();
        if positions.len() != model.mesh.positions.len() {
            return Err("initial body position count differs".into());
        }
        let normals = crate::body_pipeline::prepare_normals(&model.mesh, &positions, false)
            .map_err(|e| e.to_string())?;
        let matrix = if fields.object_flags & 0x10000 != 0 {
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
