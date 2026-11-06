//! Geometry transition prefix inputs. Mode production, geometry-header/render
//! consumers and whole-primary scheduling remain separate phases.
use crate::{actor_affine, mecc_contact_materials, object_time_precision, resident_states::Id};
use std::sync::Arc;

pub struct Frame<'a> {
    /// Output of the preceding acquired mode phase, not a keyboard mapping.
    pub mode: u8,
    pub global_seconds: f32,
    pub areas: &'a [object_time_precision::Area],
    pub model: [f32; 4],
    pub affine: Option<actor_affine::Cache>,
    pub support: u32,
    pub support_part: u32,
    pub materials: Option<Arc<mecc_contact_materials::Catalog>>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundAction {
    Start,
    Stop,
}
/// Invoke the existing owner-sound service. This is not a device-play request:
/// bank absence/suppression and attachment routing belong to that service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sound {
    pub owner: Id,
    pub identifier: u32,
    pub action: SoundAction,
}
#[derive(Default, Debug)]
pub struct View {
    pub selections: u32,
    pub sound_calls: u32,
    pub frame_queries: u32,
    pub creates: u32,
    pub marks: u32,
    pub animation_processed: bool,
    pub event_visits: u32,
    pub stamp: u32,
}
