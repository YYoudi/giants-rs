//! Required external session and action producers for the common living primary.
//! This contract alone does not admit or schedule the mission population.
use crate::resident_states::{Id, Session, VimpResponseAction};
pub trait Services {
    fn session(
        &mut self,
        session: &Session,
        owner: Id,
        query: crate::vimp_response::Predicate,
    ) -> Result<crate::actor_session_predicates::Snapshot<'_>, String>;
    fn response(
        &mut self,
        session: &mut Session,
        owner: Id,
        parent: Id,
        action: VimpResponseAction,
    ) -> Result<(), String>;
    fn active(
        &mut self,
        session: &mut Session,
        owner: Id,
        action: crate::vimp_active_response::Action,
    ) -> Result<(), String>;
}
