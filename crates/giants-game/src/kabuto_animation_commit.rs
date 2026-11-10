//! Selected-animation commit, not the policy selecting an action or a complete
//! primary. Uses the shared channel setter and zero-second ANM consumer.
use crate::{
    animation_channels::Request,
    kabuto_geometry_mode::NotificationContext,
    mecc_contact_materials,
    resident_states::{Id, Session},
};
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
pub struct Selection {
    pub slot: u8,
    pub flags: u32,
    pub clear_flags: u32,
    pub speed_source: i32,
    /// Scaled by the current actor scale before calling the common setter.
    pub reference_speed: f32,
    pub blend_seconds: f32,
    pub behavior: u32,
    pub definition: u32,
    /// Raw earlier clock word restored when the selected flags request it.
    pub retained_time: u32,
}
pub struct Frame {
    pub selection: Selection,
    pub materials: Option<Arc<mecc_contact_materials::Catalog>>,
}
#[derive(Clone, Copy, Debug)]
pub struct View {
    /// Converted using the earlier resource descriptor, not the newly selected
    /// slot. The preceding caller's previous-frame local is not overwritten.
    pub current_frame: i32,
    pub animation_processed: bool,
    pub event_visits: u32,
    pub stamp: u32,
}
pub trait World {
    fn network<'a>(
        &'a self,
        session: &Session,
        owner: Id,
    ) -> Result<NotificationContext<'a>, String>;
    /// The acquired notification carries the five words of this request. This
    /// interface does not infer packet padding, transport encoding or delivery.
    fn notify(&mut self, session: &mut Session, owner: Id, request: Request) -> Result<(), String>;
}
