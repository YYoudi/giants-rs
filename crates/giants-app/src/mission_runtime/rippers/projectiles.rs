//! Real owned launch admission. A created projectile and prepared FX do not
//! acknowledge current scene geometry, flight, impacts or damage consumers.
use super::*;

impl State {
    pub(super) fn require_projectile_flight(
        &mut self,
        session: &mut Session,
        frame: &live::Frame<'_>,
    ) -> Result<(), String> {
        let owners: std::collections::BTreeSet<_> = self
            .bound
            .as_ref()
            .unwrap()
            .1
            .iter()
            .map(|v| v.id)
            .collect();
        let shots: Vec<_> = session
            .list(0)
            .into_iter()
            .filter(|&id| {
                session
                    .enemy_projectile(id)
                    .and_then(|p| p.launch.as_ref())
                    .is_some_and(|launch| owners.contains(&launch.source))
            })
            .collect();
        let terrain = giants_game::terrain_queries::StaticTerrainQuery::new(
            &frame.world.terrain,
            1. / frame.world.terrain.spacing,
            giants_game::arithmetic::Precision::Double53,
        )?;
        for &id in &shots {
            if self.driver.projectiles.insert(id) {
                session.prepare_enemy_projectile_environment(id, frame.resources, &terrain)?;
                println!(
                    "mission_ripper_projectile_launched ticket={} owner={id:?} class={} source=ordinary_primary current_muzzle=true prepared_fx_only=true flight_complete=false",
                    frame.ticket,
                    session.get(id).unwrap().class
                );
            }
        }
        if !shots.is_empty() {
            return Err("Ripper projectile flight requires current ordinary Mecc/static/NPC contact geometry and impact services; no clear-world substitute".into());
        }
        Ok(())
    }
}
