//! Required world boundaries for the Session-owned post-control/resource phase.
//! Installing these providers does not admit a whole primary or live mission.
use crate::resident_states::{Id, Session};

pub struct Clock {
    pub global_seconds: f32,
    pub areas: Vec<crate::object_time_precision::Area>,
}
pub struct Network {
    pub session_mode: u32,
    pub peer_ids: Vec<u32>,
    pub local_peer: u32,
    /// Explicit world ownership context, not derived from a ticket or peer count.
    pub owner_word: u32,
}
impl Network {
    pub fn eligible(&self) -> Result<bool, String> {
        crate::kabuto_geometry_mode::NotificationContext {
            session_mode: self.session_mode,
            peer_ids: &self.peer_ids,
            local_peer: self.local_peer,
            owner_word: self.owner_word,
        }
        .eligible()
        .map_err(str::to_owned)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Request {
    StartSound(u32),
    NotifySound(u32),
    NotifyGrowth(u8),
    Reinitialize(u32),
}
/// No successful default services. Requests observe the published owner and may
/// change the real Session. Prefix and provider effects remain after refusal.
pub trait Services {
    fn local_owner(&mut self, session: &Session, owner: Id) -> Result<bool, String>;
    fn timer(&mut self, session: &Session) -> Result<f32, String>;
    fn global_seconds(&mut self, session: &Session) -> Result<f32, String>;
    fn store_timer(&mut self, session: &Session, value: f32) -> Result<(), String>;
    fn mode(&mut self, session: &Session) -> Result<u32, String>;
    fn clock(&mut self, session: &Session, owner: Id) -> Result<Clock, String>;
    fn network(&mut self, session: &Session, owner: Id) -> Result<Network, String>;
    fn execute(&mut self, session: &mut Session, owner: Id, request: Request)
    -> Result<(), String>;
}
#[derive(Clone, Copy, Debug)]
pub struct View {
    pub guard: crate::kabuto_post_control::View,
    pub resources: Option<crate::kabuto_resource_timers::View>,
}
