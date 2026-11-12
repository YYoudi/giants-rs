//! Real static-world services for canonical cinematic descendants. Unresolved
//! platform footprints remain errors at admitted points, never default surfaces.
use crate::{
    actor_environment,
    arithmetic::Precision,
    object_time::{self, SlowArea},
    platform_queries::{PreparedPlatform, UnpreparedPlatform},
    resident_states::{Id, Session, TimedEffectPreparationServices, TimedEffectServices},
    terrain_queries::StaticTerrainQuery,
};

/// Immutable declared collision inputs retained with the film's preparation.
/// Platform footprints are explicit unresolved dependencies, not fabricated hits.
pub struct Prepared {
    pub objects: Vec<actor_environment::Collider>,
    pub unresolved: Vec<UnpreparedPlatform>,
}
impl Prepared {
    pub fn load(
        plan: &crate::instances::Plan,
        terrain: &giants_formats::gti::Terrain,
    ) -> Result<Self, String> {
        let solids = crate::declared_mesh_scene::prepare(plan, terrain);
        let platforms = crate::declared_mesh_scene::prepare_platform_candidates(plan, terrain);
        if let Some(need) = platforms.needs.first() {
            return Err(format!("source platform {:?}: {}", need.key, need.reason));
        }
        for need in &solids.needs {
            let input = if need.key.0 {
                plan.world_entries
                    .get(need.key.1)
                    .ok_or("world collider key absent")?
                    .class_input()?
            } else {
                plan.entries
                    .get(need.key.1)
                    .ok_or("mission collider key absent")?
                    .source
                    .as_ref()
                    .map_err(Clone::clone)?
            };
            if input.direct_flags.personality & 8 == 0 {
                return Err(format!(
                    "source collision geometry {:?}: {}",
                    need.key, need.reason
                ));
            }
        }
        let unresolved = platforms
            .objects
            .iter()
            .map(|object| UnpreparedPlatform {
                id: object.fields.id,
                object_flags: object.fields.object_flags,
                collision_flags: object.fields.collision_flags,
                bounds_xy: [
                    [object.fields.bounds.min[0], object.fields.bounds.min[1]],
                    [object.fields.bounds.max[0], object.fields.bounds.max[1]],
                ],
            })
            .collect();
        let objects = solids
            .records
            .iter()
            .zip(solids.objects)
            .map(|(record, mesh)| actor_environment::Collider {
                class: record.class as u32,
                state: record.instance.state,
                volume: None,
                mesh,
            })
            .collect();
        Ok(Self {
            objects,
            unresolved,
        })
    }
}

#[derive(Default)]
pub struct Calls {
    pub motion: usize,
    pub gravity: usize,
    pub environment: usize,
    pub platforms: usize,
    pub footprint_checks: usize,
    pub callback_ground_queries: usize,
}
/// Concrete source-child and retirement owners. No default successful factory
/// is installed; immutable terrain sampling remains this world's own service.
pub trait GroundEffects {
    fn emit(
        &mut self,
        session: &mut Session,
        actor: Id,
        image: &giants_formats::effect_image::Image,
        source: &crate::effect_ground_emission::Source,
        request: crate::effect_ground_emission::Emission,
        terrain: &giants_formats::gti::Terrain,
    ) -> Result<Id, String>;
    fn retire(
        &mut self,
        session: &mut Session,
        actor: Id,
        request: crate::effect_ground_emission::Retirement,
    ) -> Result<(), String>;
}
pub struct World<'a> {
    ground_effects: Option<&'a mut dyn GroundEffects>,
    terrain: &'a giants_formats::gti::Terrain,
    query: StaticTerrainQuery<'a>,
    objects: &'a [actor_environment::Collider],
    platforms: &'a [PreparedPlatform<'a>],
    unresolved: &'a [UnpreparedPlatform],
    global_seconds: f32,
    areas: &'a [SlowArea],
    pub calls: Calls,
}
impl<'a> World<'a> {
    pub fn new(
        terrain: &'a giants_formats::gti::Terrain,
        objects: &'a [actor_environment::Collider],
        platforms: &'a [PreparedPlatform<'a>],
        unresolved: &'a [UnpreparedPlatform],
        global_seconds: f32,
        areas: &'a [SlowArea],
    ) -> Result<Self, &'static str> {
        if !global_seconds.is_finite()
            || global_seconds < 0.
            || areas.iter().any(|a| {
                !a.center
                    .into_iter()
                    .chain([a.radius_squared, a.factor])
                    .all(f32::is_finite)
            })
        {
            return Err("cinematic world requires finite actual clock/areas");
        }
        Ok(Self {
            ground_effects: None,
            terrain,
            query: StaticTerrainQuery::new(terrain, 1. / terrain.spacing, Precision::Double53)?,
            objects,
            platforms,
            unresolved,
            global_seconds,
            areas,
            calls: Calls::default(),
        })
    }
    pub fn with_ground_effects(mut self, effects: &'a mut dyn GroundEffects) -> Self {
        self.ground_effects = Some(effects);
        self
    }
}
impl TimedEffectServices for World<'_> {
    fn physical(&mut self, session: &mut Session, actor: Id, _: f32) -> Result<(), String> {
        let fields = session
            .get(actor)
            .and_then(|a| a.construction_fields.as_ref())
            .ok_or("cinematic motion fields absent")?;
        // An independent physical-stage query uses current owned fields and the
        // composing world's actual clock, not the callback argument as a stub.
        let seconds = object_time::seconds(
            self.global_seconds,
            Some(object_time::Sample {
                owner: fields.dispatch_context,
                position: fields.position,
            }),
            None,
            self.areas,
        );
        self.calls.motion += 1;
        session.advance_timed_effect_motion(actor, seconds)?;
        if session.timed_effect_needs_ground(actor) {
            let result = session.resolve_timed_effect_ground(actor, |xy| {
                self.calls.callback_ground_queries += 1;
                crate::physical_ground::Ground::from_sample(
                    self.query.sample(xy)?,
                    self.terrain.width,
                )
            })?;
            debug_assert!(result.queries > 0);
        }
        Ok(())
    }
    fn gravity(&mut self, session: &mut Session, actor: Id) -> Result<(), String> {
        self.calls.gravity += 1;
        session.apply_timed_effect_gravity(actor)
    }
    fn particle_contact(&mut self, session: &mut Session, actor: Id) -> Result<(), String> {
        session.apply_timed_particle_contact(actor)
    }
    fn ground_contact(
        &mut self,
        session: &mut Session,
        actor: Id,
        image: &giants_formats::effect_image::Image,
        source: &crate::effect_ground_emission::Source,
    ) -> Result<(), String> {
        let decision = {
            let (seed, fields) = session.timed_effect_contact_fields(actor)?;
            source.decision(fields, seed)?
        };
        if let Some(request) = decision.emission {
            let child = self
                .ground_effects
                .as_mut()
                .ok_or("ground effect requires its real surface-child factory")?
                .emit(session, actor, image, source, request, self.terrain)?;
            if child == actor
                || session.get(child).is_none()
                || session.cinematic_timed_scope(child) != session.cinematic_timed_scope(actor)
            {
                return Err("ground child factory returned a foreign or absent owner".into());
            }
            let (seed, _) = session.timed_effect_contact_fields(actor)?;
            source.emitted(seed, request)?;
        }
        if let Some(request) = decision.retirement {
            self.ground_effects
                .as_mut()
                .ok_or("ground effect requires its contact-retirement owner")?
                .retire(session, actor, request)?;
        }
        Ok(())
    }
    fn retire(
        &mut self,
        _: &mut Session,
        _: Id,
        _: &giants_formats::effect_image::Image,
        binding: Option<&crate::effect_scene::RetirementBinding>,
    ) -> Result<(), String> {
        if binding.is_some() {
            return Err(
                "positioned cinematic descendant requires its nested retirement owner".into(),
            );
        }
        Ok(()) // Actual source absence of a child request.
    }
}
impl TimedEffectPreparationServices for World<'_> {
    fn release_footprint(
        &mut self,
        _: &Session,
        _: Id,
        footprint: Option<u32>,
    ) -> Result<(), String> {
        self.calls.footprint_checks += 1;
        if footprint.is_some() {
            return Err("cinematic descendant footprint release owner absent".into());
        }
        Ok(()) // The real fresh child has no footprint allocation.
    }
    fn environment(
        &mut self,
        session: &Session,
        actor: Id,
        input: actor_environment::Input,
    ) -> Result<actor_environment::Output, actor_environment::Failure> {
        self.calls.environment += 1;
        let Some(context) = session
            .get(actor)
            .and_then(|a| a.construction_fields.as_ref())
            .map(|fields| fields.dispatch_context)
        else {
            return Err(actor_environment::Failure {
                output: Box::new(actor_environment::Output {
                    fields: input.fields,
                    physics: input.physics,
                    statistics: actor_environment::Statistics::default(),
                }),
                message: "cinematic environment owner absent",
            });
        };
        actor_environment::resolve_retaining(
            input,
            self.objects,
            |xy| {
                crate::physical_ground::Ground::from_sample(
                    self.query.sample(xy)?,
                    self.terrain.width,
                )
            },
            |point| {
                self.calls.platforms += 1;
                crate::platform_queries::sample_with_unprepared(
                    point,
                    self.platforms,
                    self.unresolved,
                    Precision::Double53,
                )
            },
            |position| {
                object_time::seconds(
                    self.global_seconds,
                    Some(object_time::Sample {
                        owner: context,
                        position,
                    }),
                    None,
                    self.areas,
                )
            },
        )
    }
}
