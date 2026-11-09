//! Acquired physical classification after the motion prefix. Scene membership,
//! peer metadata/support publication and terrain queries are explicit services.
use crate::{kabuto_contact, player_world_collision};

#[derive(Clone, Copy, Debug, Default)]
pub struct Contact {
    pub floor: f32,
    pub plane: [f32; 4],
    pub support: u32,
    pub hit: u32,
    pub support_part: u32,
    pub support_triangle: u32,
}

/// Keys belong to the prepared geometry scene, not native pointers or Session
/// serials. Effects must be applied when called, including handled events.
pub trait Peers {
    fn profile(&mut self, key: u32) -> Result<kabuto_contact::Peer, &'static str>;
    /// Historical support-write name: this peer reference is distinct from
    /// the narrow phase's floor-support selection on the moving actor.
    fn bind_support(&mut self, peer: u32, owner: u32) -> Result<(), &'static str>;
}
pub struct Scene<'a> {
    pub owner: u32,
    pub objects: &'a [player_world_collision::Object],
    pub peers: &'a mut dyn Peers,
}
