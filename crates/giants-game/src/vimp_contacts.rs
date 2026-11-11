//! Collision submissions from completed owned Vimp frames. These do not create
//! list membership, advance animation or dispatch a hit.
use crate::{resident_environment, resident_states::Id, specification};
use std::sync::Arc;

pub struct Resources {
    pub(crate) specification: Arc<specification::Prepared>,
    pub(crate) bounds: resident_environment::Resources,
}
impl Resources {
    pub fn load(
        source: &crate::scene_resources::SceneResources,
        specification: Arc<specification::Prepared>,
    ) -> Result<Self, String> {
        if specification.class != 8 {
            return Err("Vimp contact resources require the actual class8 specification".into());
        }
        let bounds = resident_environment::Resources::load(source, &specification)?;
        Ok(Self {
            specification,
            bounds,
        })
    }
}

pub struct Snapshot {
    pub actor: crate::projectile_contacts::Actor<Id>,
    pub frame: u64,
    pub response_flags: u32,
    pub bounds: crate::volume_queries::Bounds,
    pub mesh: crate::object_segment::Mesh,
    pub triangles: Vec<crate::player_mesh_contact::Triangle>,
}
