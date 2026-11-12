//! Real owned launch admission. A created projectile and prepared FX do not
//! acknowledge current scene geometry, flight, impacts or damage consumers.
use super::*;

impl State {
    pub(in crate::mission_runtime) fn publish_controlled_contact(
        &mut self,
        session: &mut Session,
        owner: Id,
        step: u64,
        resources: &giants_game::scene_resources::SceneResources,
        sample: giants_game::controlled_mecc_contacts::Sample<'_>,
    ) -> Result<(), String> {
        self.controlled_contact = None;
        if self.controlled_contact_resources.is_none() {
            let spec = session
                .specification(owner)
                .ok_or("Mecc canonical contact specification absent")?
                .clone();
            self.controlled_contact_resources = Some(
                giants_game::controlled_mecc_contacts::Resources::load(resources, spec)?,
            );
        }
        self.controlled_contact = Some(session.controlled_mecc_contact_snapshot(
            owner,
            step,
            self.controlled_contact_resources.as_ref().unwrap(),
            sample,
        )?);
        Ok(())
    }
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
            let contact = self
                .controlled_contact
                .as_ref()
                .filter(|c| c.owner == frame.controlled && c.step == frame.step)
                .ok_or(
                    "Ripper projectile requires the current normal Mecc dedicated contact packet",
                )?;
            session.controlled_mecc_motion(contact.owner, contact.step)?;
            return Err(format!(
                "Ripper projectile flight requires the ordered static/NPC scene and impact services; current normal Mecc dedicated contact is available (step={} slot={} tick={} center={:?}); no clear-world substitute",
                contact.step,
                contact.slot,
                contact.tick,
                contact.mesh.center()
            ));
        }
        Ok(())
    }
}
