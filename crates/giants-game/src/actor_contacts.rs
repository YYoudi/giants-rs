//! Copied current contact geometry. An actor identity, frame or mesh submission
//! does not acknowledge a hit callback, damage, or whole-world admission.
use crate::resident_states::Id;

pub struct Snapshot {
    pub actor: crate::projectile_contacts::Actor<Id>,
    pub frame: u64,
    pub response_flags: u32,
    pub bounds: crate::volume_queries::Bounds,
    pub mesh: crate::object_segment::Mesh,
    pub triangles: Vec<crate::player_mesh_contact::Triangle>,
}
