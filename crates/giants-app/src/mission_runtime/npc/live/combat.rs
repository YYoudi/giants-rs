//! Ordinary partial-pistol admission, actual sound/FX/blood owners and remains.
use super::*;
use giants_game::{actor_retirement, scene_resources::SceneResources, vimp_damage};
struct Effects<'a> {
    fx: &'a mut giants_game::vimp_death_fx::Runtime,
    source: &'a SceneResources,
    world: &'a giants_game::WorldAsset,
    selection: giants_game::effect_initialization::Selection,
}
impl vimp_damage::Effects for Effects<'_> {
    fn death(
        &mut self,
        _: &mut Session,
        effect: vimp_damage::DeathEffect,
        random: &mut Random,
    ) -> Result<(), String> {
        self.fx
            .death(self.source, effect, self.selection, random, self.world)
    }
}
struct Release<'a, 'b> {
    wounds: &'a mut giants_game::blood_wounds::Pool<Id, String>,
    device: &'b mut dyn actor_retirement::Services,
}
impl actor_retirement::Services for Release<'_, '_> {
    fn release(&mut self, session: &mut Session, id: Id) -> Result<(), String> {
        self.wounds.retire_owner(|owner| owner == id);
        self.device.release(session, id)
    }
}
pub(in crate::mission_runtime) struct Hit<'a> {
    pub source: &'a SceneResources,
    pub world: &'a giants_game::WorldAsset,
    pub shooter: Id,
    pub target: Id,
    pub projectile: giants_game::mecc_pistol::WorldHit,
    pub camera: [f32; 3],
}
impl State {
    pub(in crate::mission_runtime) fn hit(
        &mut self,
        session: &mut Session,
        hit: Hit<'_>,
        random: &mut Random,
        device: &mut impl actor_retirement::Services,
    ) -> Result<(bool, Vec<crate::mission_audio::Event>), String> {
        if self.driver.failure.is_some() {
            return Err("Vimp component already quarantined".into());
        }
        if !self.driver.children.contains(&hit.target) {
            return Err("Vimp hit owner outside ordinary component".into());
        }
        let mut sounds = super::super::sounds::Sounds::default();
        let selection = giants_game::effect_initialization::Selection {
            active_objects: u32::try_from(session.counts().residents + self.fx.active())
                .map_err(|_| "FX count overflow")?,
            quality: self.globals.scalar(0x13e0).map_err(|e| e.to_string())?,
            lite_distance: self.globals.scalar(0x1570).map_err(|e| e.to_string())?,
            camera: hit.camera,
            recipient_position: hit.projectile.point,
        };
        let mut chunks =
            giants_game::vimp_chunks::Factory::with_next_class(hit.source, self.chunk_cursor)?;
        let result = session.damage_vimp_from_partial_pistol(
            hit.target,
            vimp_damage::PartialPistolHit {
                shooter: hit.shooter,
                hit: hit.projectile,
            },
            random,
            &mut vimp_damage::Services {
                effects: &mut Effects {
                    fx: &mut self.fx,
                    source: hit.source,
                    world: hit.world,
                    selection,
                },
                sounds: &mut sounds,
                blood: &mut vimp_damage::AttachedBlood {
                    particles: &mut self.blood.particles,
                    wounds: &mut self.blood.wounds,
                },
                chunks: &mut chunks,
                proxy: &self.proxy,
                globals: &self.globals,
            },
        );
        self.chunk_cursor = chunks.next_class();
        let outcome = match result {
            Ok(v) => v,
            Err(e) => {
                self.driver.failure = Some(e.clone());
                return Err(e);
            }
        };
        let dead = matches!(outcome, vimp_damage::Outcome::Died { .. });
        if dead {
            self.driver.children.remove(&hit.target);
            session.cleanup_dead_vimp(
                hit.target,
                &mut Release {
                    wounds: &mut self.blood.wounds,
                    device,
                },
            )?;
        }
        println!(
            "mission_vimp_hit ticket={} owner={:?} died={dead} source=ordinary_partial_pistol damage_callback=true fx_births={} death_fx_policy=partial_source_graph native_fidelity=false",
            hit.shooter.ticket(),
            hit.target,
            self.fx.births
        );
        Ok((dead, sounds.0))
    }
    pub(super) fn remains(
        &mut self,
        session: &mut Session,
        source: &SceneResources,
        frame: &Frame<'_>,
        random: &mut Random,
        device: &mut impl actor_retirement::Services,
    ) -> Result<(), String> {
        let q = giants_game::terrain_queries::StaticTerrainQuery::new(
            &frame.world.terrain,
            1. / frame.world.terrain.spacing,
            giants_game::arithmetic::Precision::Double53,
        )?;
        for id in session.list(0) {
            if let Some(chunk) = session.vimp_chunk(id) {
                let f = session
                    .get(id)
                    .unwrap()
                    .construction_fields
                    .as_ref()
                    .unwrap();
                if f.object_flags & 0x8000 == 0 {
                    session.advance_vimp_chunk(
                        id,
                        giants_game::vimp_chunks::Environment {
                            number: chunk.number() + 1,
                            globals: &self.globals,
                            global_seconds: frame.tick.global_delta,
                            areas: frame.tick.slow_areas,
                            offline: true,
                        },
                        |p| Ok(q.sample([p[0], p[1]])?.height),
                    )?;
                }
                if session
                    .get(id)
                    .unwrap()
                    .construction_fields
                    .as_ref()
                    .unwrap()
                    .object_flags
                    & 0x8000
                    != 0
                {
                    let mut release = Release {
                        wounds: &mut self.blood.wounds,
                        device,
                    };
                    if chunk.collected() {
                        session.cleanup_collected_vimp_chunk(id, &mut release)?;
                        println!(
                            "mission_meat_retired ticket={} owner={id:?} reason=collected actual_external_release=true",
                            frame.ticket
                        );
                    } else {
                        session.cleanup_expired_vimp_chunk(id, &mut release)?;
                    }
                }
            } else if let Some(proxy) = session.vimp_death_proxy(id) {
                session.advance_vimp_death_proxy_without_mana(
                    id,
                    proxy.number,
                    &self.globals,
                    &mut self.bound.as_mut().unwrap().contexts,
                    &giants_game::vimp_ghost::Factory::new(source),
                    random,
                )?;
                if session
                    .get(id)
                    .unwrap()
                    .construction_fields
                    .as_ref()
                    .unwrap()
                    .object_flags
                    & 0x8000
                    != 0
                {
                    session.cleanup_vimp_death_proxy(
                        id,
                        &mut Release {
                            wounds: &mut self.blood.wounds,
                            device,
                        },
                    )?;
                }
            }
        }
        for id in session.list(0) {
            if let Some(number) = session.vimp_ghost_number(id) {
                let fields = session
                    .get(id)
                    .unwrap()
                    .construction_fields
                    .as_ref()
                    .unwrap();
                let dt = giants_game::object_time::seconds(
                    frame.tick.global_delta,
                    Some(giants_game::object_time::Sample {
                        owner: fields.dispatch_context,
                        position: fields.position,
                    }),
                    None,
                    frame.tick.slow_areas,
                ) as f32;
                if session.advance_vimp_ghost(id, number, dt, &mut self.driver.stamp)?
                    == giants_game::effect_visual::Step::Expired
                {
                    session.cleanup_vimp_ghost(
                        id,
                        &mut Release {
                            wounds: &mut self.blood.wounds,
                            device,
                        },
                    )?;
                }
            }
        }
        self.fx.advance(
            source,
            frame.tick.global_delta,
            frame.tick.slow_areas,
            frame.world,
            random,
        )?;
        Ok(())
    }
}
