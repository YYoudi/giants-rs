//! Dedicated collision geometry from the ordinary controller's actual clip and
//! clock. This composes the partial controller; it is not a native state2 frame,
//! a global vertex epoch, a health provider or an acknowledgement of damage.
use std::sync::Arc;

pub struct Resources {
    pub(crate) specification: Arc<crate::specification::Prepared>,
    pub(crate) bounds: crate::resident_environment::Resources,
}
impl Resources {
    pub fn load(
        source: &crate::scene_resources::SceneResources,
        specification: Arc<crate::specification::Prepared>,
    ) -> Result<Self, String> {
        if specification.class != 2 {
            return Err(
                "controlled Mecc contacts require the canonical class2 specification".into(),
            );
        }
        let bounds = crate::resident_environment::Resources::load(source, &specification)?;
        Ok(Self {
            specification,
            bounds,
        })
    }
}

/// Borrowed directly from the normal controller's selected descriptor/clock.
/// The producer must not advance either while composing this contact packet.
#[derive(Clone, Copy)]
pub struct Sample<'a> {
    pub slot: u8,
    pub name: &'a str,
    pub tree: &'a str,
    pub seconds: f32,
}

pub struct Snapshot {
    pub owner: crate::resident_states::Id,
    pub step: u64,
    pub slot: u8,
    pub tick: i32,
    pub active: u32,
    pub personality: u32,
    pub bounds: crate::volume_queries::Bounds,
    pub mesh: crate::object_segment::Mesh,
}
