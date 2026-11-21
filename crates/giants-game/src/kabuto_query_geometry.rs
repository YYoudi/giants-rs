//! Owned mode/transition/geometry-header continuation after query feedback.
//! Prototype PS2 `kabuto_move` is an unknown parent navigation route. No
//! prototype signature/layout, complete primary or live admission is supplied.
use crate::{
    kabuto_geometry_mode as mode,
    kabuto_submission::State,
    resident_states::{Id, Session},
};
use std::sync::Arc;

pub struct Frame<'a> {
    /// Actual raw global frame interval; not the object-specific query clock.
    pub global_seconds: f32,
    pub areas: &'a [crate::object_time_precision::Area],
    /// Stable peer/session inputs; owner_word must be this actor's actual
    /// acquired header context, never a ticket/peer-count approximation.
    pub network: &'a mode::NotificationContext<'a>,
    pub materials: Option<Arc<crate::mecc_contact_materials::Catalog>>,
}
pub trait Services {
    /// Stable-context, ignored-return transport boundary of the acquired mode
    /// component. The observer sees real published Session state at each send.
    fn notify(&mut self, session: &Session, owner: Id, event: mode::Notification);
    fn sound(
        &mut self,
        session: &Session,
        event: crate::kabuto_geometry_transition::Sound,
    ) -> Result<(), String>;
    fn controlled(&mut self, session: &Session) -> Result<Option<Id>, String>;
    fn camera(
        &mut self,
        session: &Session,
        owner: Id,
    ) -> Result<crate::kabuto_geometry_state::Camera, String>;
    /// Current ANM attachment input; no bind-pose or first-node fallback.
    fn attachment(&mut self, session: &Session, owner: Id) -> Result<[f32; 3], String>;
    fn bounds(
        &mut self,
        session: &Session,
        target: Id,
    ) -> Result<crate::kabuto_reference_target::Bounds, String>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Next {
    ReactionAdmission,
    PrimaryTailRequired,
}
/// Snapshot of this completed component; caller lifetime and subsequent phases
/// still require their own guards. Neither continuation is a completed primary.
#[derive(Clone, Debug)]
pub struct View {
    pub owner: Id,
    pub number: u64,
    pub mode: u8,
    pub header: State,
    pub next: Next,
    pub transition: crate::kabuto_geometry_transition::View,
}
pub(crate) struct Phase {
    pub header: Option<State>,
    pub target: Option<Id>,
    pub entered: Option<u64>,
    pub running: bool,
    pub mode_completed: Option<u64>,
    pub completed: Option<u64>,
    pub view: Option<View>,
}
impl Phase {
    pub fn cold() -> Self {
        // The acquired cold geometry constructor explicitly clears its entire
        // header before filling topology/ownership. These are its state words,
        // not a default renderer or an initialization acknowledgment.
        Self {
            header: Some(State {
                enabled: false,
                position: [0.; 3],
                direction: [0.; 3],
                scale: [0.; 3],
                weight: 0.,
                flags: 0,
                state_word: 0,
            }),
            target: None,
            entered: None,
            running: false,
            mode_completed: None,
            completed: None,
            view: None,
        }
    }
}
