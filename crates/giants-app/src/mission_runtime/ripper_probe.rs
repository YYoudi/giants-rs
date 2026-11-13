//! Read-only input-diagnostic view of the same ordinary draw submissions.
use super::*;
pub(crate) struct RipperInputSnapshot {
    pub target: Option<[f32; 3]>,
    pub flight_revision: u64,
    pub projectile: Option<[f32; 3]>,
    pub impact: Option<([f32; 3], u64)>,
    /// Absent before the actual partial-controller owner is published.
    pub mecc_damage: Option<f32>,
    pub quarantine_step: Option<u64>,
}
impl Runtime {
    pub(crate) fn ripper_input_snapshot(
        &mut self,
        ticket: u64,
        position: [f32; 3],
    ) -> Result<RipperInputSnapshot, String> {
        let draws = self.npc_draws(ticket)?;
        let component = self
            .active
            .as_ref()
            .and_then(|a| a.payload.rippers.as_ref().ok()?.as_ref());
        let target = draws
            .iter()
            .filter(|d| matches!(d.class, 73 | 230))
            .filter(|d| match d.id {
                NpcDrawOwner::Actor(id) => component.is_some_and(|c| !c.actor_quarantined(id)),
                _ => false,
            })
            .min_by(|a, b| {
                let distance = |p: [f32; 3]| (p[0] - position[0]).hypot(p[1] - position[1]);
                distance(a.center).total_cmp(&distance(b.center))
            })
            .map(|d| d.center);
        let active = self
            .active
            .as_ref()
            .filter(|active| active.ticket == ticket)
            .ok_or("Ripper probe ticket absent")?;
        let session = &active.resident_states;
        let projectile = draws
            .iter()
            .filter_map(|d| {
                let NpcDrawOwner::Actor(id) = d.id else {
                    return None;
                };
                session.enemy_projectile(id).map(|_| (d.frame, d.center))
            })
            .max_by_key(|v| v.0);
        let flight_revision = projectile.map_or(0, |v| v.0);
        Ok(RipperInputSnapshot {
            quarantine_step: active
                .payload
                .rippers
                .as_ref()
                .ok()
                .and_then(|r| r.as_ref())
                .and_then(|r| r.quarantine_step()),
            target,
            flight_revision,
            projectile: projectile.map(|v| v.1),
            mecc_damage: active
                .controlled_mecc_owner
                .map(|owner| session.mecc_health(owner).map(|health| health.damage))
                .transpose()?,
            impact: self
                .active
                .as_ref()
                .and_then(|a| a.payload.rippers.as_ref().ok()?.as_ref()?.impact_snapshot()),
        })
    }
}
