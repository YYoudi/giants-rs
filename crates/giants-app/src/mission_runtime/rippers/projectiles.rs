//! Ordinary launch/current-scene flight and acquired missed-ground reactions.
//! Active damage and other contact branches remain required consumers. Flights
//! consume the real source ANM clock and current affine before body submission.
use super::*;

impl State {
    pub(in crate::mission_runtime) fn publish_projectile_scene(
        &mut self,
        step: u64,
        registry: giants_game::enemy_projectile_scene::Registry,
        contacts: Result<Vec<giants_game::actor_contacts::Snapshot>, String>,
    ) {
        self.projectile_scene = Some((step, registry, contacts));
    }
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
    pub(super) fn advance_projectile_flight(
        &mut self,
        session: &mut Session,
        frame: &live::Frame<'_>,
        random: &mut giants_game::audio_parameters::Random,
        sounds: &mut Vec<crate::mission_audio::Event>,
        device: &mut impl super::combat::Services,
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
            let (_, registry, peers) = self
                .projectile_scene
                .as_ref()
                .filter(|(step, _, _)| *step == frame.step)
                .ok_or("Ripper projectile requires the current ordinary scene registry")?;
            use giants_game::enemy_projectile_scene::{Entry, Scene, Target};
            let mut entries = registry.declarations.clone();
            // Key1 is the partial controller's query-local player slot, not a
            // Session serial. Prepared declaration keys start at2.
            entries.push(Entry {
                target: Target::Actor(contact.owner),
                object: giants_game::projectile_world::Object {
                    id: 1,
                    parent: 0,
                    active: contact.active,
                    personality: contact.personality,
                    bounds: contact.bounds,
                    mesh: contact.mesh.clone(),
                },
            });
            let peers = peers.as_ref().map_err(Clone::clone)?;
            let snapshots = self.contact_snapshots(session, frame.ticket)?;
            // Vimp packets come from the completed preceding component in this
            // actual partial schedule; Ripper packets are copied after this turn.
            let mut ordered = peers.iter().chain(snapshots.iter()).collect::<Vec<_>>();
            let list = session.list(0);
            ordered.sort_by_key(|s| list.iter().position(|id| *id == s.actor.id));
            for snapshot in ordered {
                let actor = snapshot.actor;
                let key = *registry
                    .actors
                    .get(&actor.id)
                    .ok_or("projectile NPC missing from common controller registry")?;
                if actor.parent.is_some() {
                    return Err("ordinary projectile parent binding not acquired".into());
                }
                entries.push(Entry {
                    target: Target::Actor(actor.id),
                    object: giants_game::projectile_world::Object {
                        id: key,
                        parent: 0,
                        active: actor.flags.active,
                        personality: actor.flags.personality,
                        bounds: snapshot.bounds,
                        mesh: snapshot.mesh.clone(),
                    },
                });
            }
            let query = giants_game::segment_queries::StaticCellQuery::new(
                &frame.world.terrain,
                giants_game::arithmetic::Precision::Double53,
            )?;
            for id in shots {
                let outcome = session.advance_enemy_projectile_singleplayer_scene(
                    id,
                    giants_game::enemy_projectile::SceneFlightWorld {
                        terrain: &query,
                        inverse_spacing: 1. / frame.world.terrain.spacing,
                        seconds: frame.seconds,
                        areas: frame.areas,
                    },
                    Scene::new(entries.clone())?,
                )?;
                if matches!(
                    outcome,
                    giants_game::enemy_projectile::FlightOutcome::GroundContact { .. }
                ) {
                    self.ground_impact(session, frame, id, random, sounds, device)?;
                    continue;
                }
                if matches!(
                    outcome,
                    giants_game::enemy_projectile::FlightOutcome::Expired
                ) {
                    self.expire_projectile(session, frame, id, random, sounds, device)?;
                    continue;
                }
                if let giants_game::enemy_projectile::FlightOutcome::ObjectContact {
                    target, ..
                } = outcome
                {
                    let result = session.complete_enemy_projectile_object_contact(
                        id,
                        random,
                        &mut super::combat::Object {
                            device,
                            step: frame.step,
                            source: id,
                        },
                    ).map_err(|error| format!("Ripper object contact source={id:?} target={target:?} step={}: {error}", frame.step))?;
                    if result == giants_game::enemy_projectile::ObjectContactOutcome::Marked {
                        session.cleanup_enemy_projectile_object_contact(
                            id,
                            &mut super::impacts::Retirement {
                                contexts: self
                                    .contexts
                                    .as_mut()
                                    .ok_or("Ripper object retirement contexts absent")?,
                                device,
                            },
                        )?;
                        self.driver.projectiles.remove(&id);
                        self.driver.flights.remove(&id);
                        continue;
                    }
                }
                if !matches!(
                    outcome,
                    giants_game::enemy_projectile::FlightOutcome::Flying
                        | giants_game::enemy_projectile::FlightOutcome::ObjectContact { .. }
                ) {
                    return Err(format!(
                        "ordinary Ripper projectile has pending {outcome:?}; impact/damage/retirement consumers not connected"
                    ));
                }
                session.advance_enemy_projectile_presentation(id, &mut self.driver.stamp)?;
                let count = self.driver.flights.entry(id).or_default();
                *count += 1;
                if *count == 1 {
                    println!(
                        "mission_ripper_projectile_flight ticket={} owner={id:?} step={} objects={} current_scene=true animated_body_prepared=true damage=false",
                        frame.ticket,
                        frame.step,
                        entries.len()
                    );
                }
            }
        }
        Ok(())
    }
}
