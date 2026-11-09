//! Owned post-model ANM and gravity sequence. Remaining primary consumers and
//! live frame/world completion are separate; unsupported event services refuse.
pub struct Frame<'a> {
    pub global_seconds: f32,
    pub areas: &'a [crate::object_time::SlowArea],
    pub materials: Option<std::sync::Arc<crate::mecc_contact_materials::Catalog>>,
}
#[derive(Debug, Clone, Copy)]
pub struct View {
    pub animation_processed: bool,
    pub event_visits: u32,
    pub stamp: u32,
}
