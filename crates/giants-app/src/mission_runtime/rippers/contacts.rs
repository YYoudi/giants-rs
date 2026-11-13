//! Actual completed Ripper collision submissions into the common Mecc scene.
use super::*;

impl State {
    pub(in crate::mission_runtime) fn contact_snapshots(
        &self,
        session: &mut Session,
        ticket: u64,
    ) -> Result<Vec<giants_game::actor_contacts::Snapshot>, String> {
        self.view(session, ticket)?;
        if self.driver.failed() {
            return Err("Ripper contact component quarantined".into());
        }
        let owners = &self.bound.as_ref().unwrap().1;
        session
            .list(0)
            .into_iter()
            .filter(|id| owners.iter().any(|owner| owner.id == *id))
            .filter_map(|id| self.driver.completed(id).map(|(number, _)| (id, number)))
            .map(|(id, number)| session.ripper_contact_snapshot(id, number))
            .collect()
    }
}
