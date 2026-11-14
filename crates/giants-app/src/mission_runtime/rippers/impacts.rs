//! Ordinary missed-ground-shot reactions and their canonical mixed bodies.
//! Prototype PS2 `fx_logic`, `fx_processOpCodes__FP6FxDataiP6OBJECT` and
//! `ripper_move` remain unknown navigation relationships;
//! this explicit partial schedule does not establish full native interleaving.
use super::*;
use giants_game::{
    attached_effect_context::{
        Creation, Pool,
        mixed::population::{self, Population},
    },
    audio_parameters::Random,
    enemy_projectile::{
        EffectOutcome, GroundImpactServices, ImpactEffect, RadialRequest, SoundOutcome,
    },
};
pub(super) struct Effects {
    population: Population,
    world: giants_game::cinematic_timed::world::Prepared,
    platforms: giants_game::declared_platform_scene::Prepared,
    awaiting_geometry: bool,
    pub(super) last_retired: Option<Id>,
    pub(super) last_expired: Option<Id>,
    last_impact: Option<([f32; 3], u64)>,
    pub(super) births: usize,
}
/// Keep cached platform geometry exact while binding non-platform contacts to
/// their actual current ordinary owner. No empty scene or guessed footprint is
/// substituted, and source list order is retained among platform candidates.
fn same_static_platform_projection(
    initial: &[giants_game::actor_environment::Collider],
    current: &[giants_game::actor_environment::Collider],
) -> bool {
    let mut initial = initial
        .iter()
        .filter(|a| a.mesh.fields.collision_flags & 8 != 0);
    let mut current = current
        .iter()
        .filter(|a| a.mesh.fields.collision_flags & 8 != 0);
    loop {
        match (initial.next(), current.next()) {
            (None, None) => return true,
            (Some(a), Some(b)) => {
                let (x, y) = (a.mesh.fields, b.mesh.fields);
                if a.class != b.class
                    || a.state != b.state
                    || x.id != y.id
                    || x.parent != y.parent
                    || x.object_flags != y.object_flags
                    || x.collision_flags != y.collision_flags
                    || x.has_model != y.has_model
                    || x.bounds.min != y.bounds.min
                    || x.bounds.max != y.bounds.max
                    || a.mesh.triangles != b.mesh.triangles
                {
                    return false;
                }
            }
            _ => return false,
        }
    }
}

impl Effects {
    pub(super) fn load(
        ticket: u64,
        plan: &Plan,
        world: &giants_game::WorldAsset,
    ) -> Result<Self, String> {
        Ok(Self {
            population: Population::new(ticket),
            world: giants_game::cinematic_timed::world::Prepared::load(plan, &world.terrain)?,
            platforms: giants_game::declared_platform_scene::Prepared::load(plan, &world.terrain)?,
            awaiting_geometry: false,
            last_retired: None,
            last_expired: None,
            last_impact: None,
            births: 0,
        })
    }
}
struct Impact<'a> {
    population: &'a mut Population,
    contexts: &'a mut Pool<Id>,
    globals: &'a giants_formats::global_data::GlobalData,
    sounds: &'a mut Vec<crate::mission_audio::Event>,
}
impl GroundImpactServices for Impact<'_> {
    fn effect(
        &mut self,
        session: &mut Session,
        request: ImpactEffect,
        random: &mut Random,
    ) -> Result<EffectOutcome, String> {
        Ok(
            match self
                .population
                .prepare_impact(session, self.contexts, &request, random)?
            {
                Creation::Prepared(_) => EffectOutcome::Prepared,
                Creation::Absent => EffectOutcome::Rejected,
            },
        )
    }
    fn radial(
        &mut self,
        session: &mut Session,
        request: RadialRequest,
        _: &mut Random,
    ) -> Result<(), String> {
        let range = self.globals.scalar(0x9c0).map_err(|e| e.to_string())?;
        if let Some(obstacle) =
            session.enemy_projectile_radial_no_effect_obstacle(&request, Some(range))?
        {
            return Err(format!(
                "ordinary Ripper blast requires an actual active-hit or Vimp-alert consumer; no-effect invariant does not hold; obstacle={obstacle:?}; eligibility=not_evaluated"
            ));
        }
        Ok(())
    }
    fn sound(
        &mut self,
        _: &mut Session,
        sound: u32,
        position: [f32; 3],
    ) -> Result<SoundOutcome, String> {
        self.sounds
            .push(crate::mission_audio::Event::NpcPosition { sound, position });
        Ok(SoundOutcome::Submitted)
    }
}
pub(super) struct Retirement<'a, D> {
    pub(super) contexts: &'a mut Pool<Id>,
    pub(super) device: &'a mut D,
}
impl<D: giants_game::actor_retirement::Services> giants_game::actor_retirement::Services
    for Retirement<'_, D>
{
    fn release(&mut self, session: &mut Session, id: Id) -> Result<(), String> {
        self.contexts.release_recipient_prepared(id)?;
        self.device.release(session, id)
    }
}
impl State {
    pub(in crate::mission_runtime) fn impact_geometry_pending(&self) -> bool {
        self.effects
            .as_ref()
            .is_some_and(|effects| effects.awaiting_geometry)
    }
    pub(in crate::mission_runtime) fn impact_snapshot(&self) -> Option<([f32; 3], u64)> {
        self.effects.as_ref()?.last_impact
    }
    pub(super) fn ground_impact(
        &mut self,
        session: &mut Session,
        frame: &live::Frame<'_>,
        id: Id,
        random: &mut Random,
        sounds: &mut Vec<crate::mission_audio::Event>,
        device: &mut impl giants_game::actor_retirement::Services,
    ) -> Result<(), String> {
        let effects = self
            .effects
            .as_mut()
            .ok_or("ordinary Ripper effect owner absent")?;
        let contexts = self
            .contexts
            .as_mut()
            .ok_or("ordinary Ripper context pool absent")?;
        let outcome = session.complete_enemy_projectile_ground_flight(
            id,
            random,
            &mut Impact {
                population: &mut effects.population,
                contexts,
                globals: &self.globals,
                sounds,
            },
        )?;
        if matches!(
            outcome,
            giants_game::enemy_projectile::FlightOutcome::Expired
        ) {
            session.complete_enemy_projectile_expiry(
                id,
                random,
                &mut Impact {
                    population: &mut effects.population,
                    contexts,
                    globals: &self.globals,
                    sounds,
                },
            )?;
        }
        let position = session
            .get(id)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap()
            .position;
        if matches!(
            outcome,
            giants_game::enemy_projectile::FlightOutcome::Expired
        ) {
            session.cleanup_enemy_projectile_expiry(id, &mut Retirement { contexts, device })?;
            effects.last_expired = Some(id);
        } else {
            session.cleanup_enemy_projectile(id, &mut Retirement { contexts, device })?;
        }
        effects.last_retired = Some(id);
        effects.last_impact = Some((position, frame.step));
        self.driver.projectiles.remove(&id);
        self.driver.flights.remove(&id);
        println!(
            "mission_ripper_ground_impact ticket={} step={} owner={id:?} actual_radial_list_no_effect=true effect_context_prepared=true positional_sound_submitted=true source_retired=true bodies_pending=true",
            frame.ticket, frame.step
        );
        Ok(())
    }
    pub(super) fn expire_projectile(
        &mut self,
        session: &mut Session,
        frame: &live::Frame<'_>,
        id: Id,
        random: &mut Random,
        sounds: &mut Vec<crate::mission_audio::Event>,
        device: &mut impl giants_game::actor_retirement::Services,
    ) -> Result<(), String> {
        let effects = self
            .effects
            .as_mut()
            .ok_or("Ripper expiry effect owner absent")?;
        let contexts = self
            .contexts
            .as_mut()
            .ok_or("Ripper expiry contexts absent")?;
        session.complete_enemy_projectile_expiry(
            id,
            random,
            &mut Impact {
                population: &mut effects.population,
                contexts,
                globals: &self.globals,
                sounds,
            },
        )?;
        session.cleanup_enemy_projectile_expiry(id, &mut Retirement { contexts, device })?;
        effects.last_expired = Some(id);
        self.driver.projectiles.remove(&id);
        self.driver.flights.remove(&id);
        println!(
            "mission_ripper_projectile_expired ticket={} step={} owner={id:?} actual_expiry=true sound_before_fx=true required_radial_completed_or_skipped=true source_retired=true ground_contact_invented=false",
            frame.ticket, frame.step
        );
        Ok(())
    }
    pub(super) fn advance_impact_effects(
        &mut self,
        session: &mut Session,
        frame: &live::Frame<'_>,
        random: &mut Random,
    ) -> Result<(), String> {
        let effects = self
            .effects
            .as_mut()
            .ok_or("ordinary Ripper effect owner absent")?;
        if effects.population.settled() {
            return Ok(());
        }
        // Only the cached platform footprint/geometry requires the static
        // projection. Ordinary solids below use the actual current controller
        // meshes, including canonical barracks initialization and damage.
        // Retain exact platform guards and all unresolved source candidates.
        if !same_static_platform_projection(&effects.world.objects, frame.objects) {
            return Err(
                "Ripper effect declared platform changed; current footprint owner required".into(),
            );
        }
        let mut world = giants_game::cinematic_timed::world::World::new(
            &frame.world.terrain,
            frame.objects,
            &[],
            &effects.world.unresolved,
            frame.seconds,
            frame.areas,
        )?
        .with_declared_platforms(&effects.platforms)?;
        let prepared = effects.population.advance(
            session,
            population::Frame {
                // This is the common-world cohort frame, not the controlled
                // Mecc's fixed physics step. A world sample can advance while
                // that step is unchanged; seconds still use that world sample.
                number: frame.sample,
                global_seconds: frame.seconds,
                areas: frame.areas,
                progress: giants_game::actor_finalization::PhysicalProgress::PrototypePs2Zero,
                pc_argument: None,
            },
            population::Update {
                pool: self.contexts.as_mut().ok_or("Ripper contexts absent")?,
                resources: frame.resources,
                random,
                stamp: &mut self.driver.stamp,
                world: &mut world,
            },
        )?;
        effects.births += prepared.born.len();
        effects.awaiting_geometry = true;
        Ok(())
    }
    /// Called by the common particle geometry consumer after real current-frame
    /// buffers have been prepared. This is not a GPU/device availability ack.
    pub(in crate::mission_runtime) fn finish_impact_geometry(
        &mut self,
        session: &mut Session,
    ) -> Result<(), String> {
        let Some(effects) = self.effects.as_mut() else {
            return Ok(());
        };
        if !effects.awaiting_geometry {
            return Ok(());
        }
        effects.population.cleanup(
            session,
            self.contexts
                .as_ref()
                .ok_or("Ripper contexts absent at effect cleanup")?,
        )?;
        effects.awaiting_geometry = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn collider(id: u32, collision_flags: u32) -> giants_game::actor_environment::Collider {
        giants_game::actor_environment::Collider {
            class: 1060,
            state: 98,
            volume: None,
            mesh: giants_game::mecc_mesh_dispatch::MeshObject {
                fields: giants_game::mecc_collision_candidates::Object {
                    id,
                    parent: 0,
                    has_model: true,
                    object_flags: 16,
                    collision_flags,
                    bounds: giants_game::volume_queries::Bounds {
                        min: [0.; 3],
                        max: [1.; 3],
                    },
                },
                triangles: vec![[[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]]],
            },
        }
    }
    #[test]
    fn current_solid_changes_do_not_stale_platform_footprints_but_platform_changes_do() {
        let initial = [collider(1, 0x21000), collider(2, 8)];
        let mut current = [collider(1, 0x21000), collider(2, 8)];
        current[0].mesh.fields.object_flags = 1072;
        current[0].mesh.triangles[0][1][2] = 0.25;
        assert!(same_static_platform_projection(&initial, &current));
        assert!(same_static_platform_projection(&initial, &current[1..]));
        current[1].mesh.triangles[0][1][2] = 0.25;
        assert!(!same_static_platform_projection(&initial, &current));
        current[1] = collider(2, 8);
        current[1].mesh.fields.object_flags |= 0x8000;
        assert!(!same_static_platform_projection(&initial, &current));
        assert!(!same_static_platform_projection(&initial, &current[..1]));
        current[1] = collider(2, 8);
        current[0].mesh.fields.collision_flags |= 8;
        assert!(!same_static_platform_projection(&initial, &current));
    }
}
