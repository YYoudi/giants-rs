//! Required-service bridge from the acquired primary to real owned state6
//! construction and launch. Initialization and mission scheduling stay separate.
//! Prototype PS2 `ripper_move` names related caller work; the exact relationship
//! of PC debug-1.1 helper 0x004dbaf0 to prototype missile names stays unassigned.
use crate::{
    actor_fields::Fields,
    arithmetic::Precision,
    audio_parameters::Random,
    enemy_projectile::{self, EffectOutcome, SoundOutcome},
    resident_states::{Construction, Id, Session},
    ripper_primary_turn,
    scene_resources::SceneResources,
    state_storage::{Profile, Seed},
    state23_animation::FireOutcome,
};
use std::{collections::BTreeMap, marker::PhantomData, sync::Arc};
struct Template {
    class: i32,
    seed: Seed,
    bindings: Vec<crate::model_registry::StateBinding>,
    flags: crate::object_lists::DirectFlags,
    specification: Arc<crate::specification::Prepared>,
    states: Arc<crate::state_resources::Prepared>,
    fields: Fields,
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resource_absence_and_invalid_program_are_cached_without_constructing_or_reclassifying() {
        let mut models = crate::model_registry::ModelRegistry::default();
        models.programs.insert(
            90,
            vec![crate::model_registry::ClassProgram {
                catalog: "fixture.bin".into(),
                program: Err("malformed class fixture".into()),
                bindings: Ok(vec![]),
            }],
        );
        let resources = SceneResources {
            world_bounds: Default::default(),
            classes: crate::catalog_registry::WorldClassRegistry {
                visits: vec![],
                classes: Default::default(),
                textures: Default::default(),
                models,
            },
            archives: Default::default(),
            language: "English".into(),
        };
        let mut factory = Factory::new(&resources);
        let mut session = Session::new(149);
        for _ in 0..2 {
            assert!(factory.construct(&mut session, 232).unwrap().is_none());
            assert_eq!(
                factory.construct(&mut session, 90).unwrap_err(),
                "malformed class fixture"
            );
        }
        assert_eq!(factory.cached_classes(), 2);
        assert_eq!(session.counts().residents, 0);
    }
}
impl Template {
    fn load(resources: &SceneResources, class: i32) -> Result<Option<Self>, String> {
        let Some(definition) = resources.classes.models.first_program(class) else {
            return Ok(None);
        };
        let program = definition.program.as_ref().map_err(Clone::clone)?;
        let seed = Seed::from_program_for(program, Profile::Debug, Precision::Double53)?;
        let bindings = resources.class_state_bindings_for(class, Profile::Debug)?;
        let flags = crate::object_lists::DirectFlags::project(class, program);
        let specification = Arc::new(crate::specification::Prepared::load_registered(
            resources, class,
        )?);
        let states = Arc::new(crate::state_resources::Prepared::load(
            resources,
            class,
            Profile::Debug,
        )?);
        let fields = Fields::load(resources, class, &specification)?;
        // The same constructor validation also applies to cached templates.
        enemy_projectile::prepare(&seed, &fields)?;
        if !specification.needs.is_empty() || !states.needs().is_empty() {
            return Err("primary projectile resource preparation incomplete".into());
        }
        Ok(Some(Self {
            class,
            seed,
            bindings,
            flags,
            specification,
            states,
            fields,
        }))
    }
    fn input(&self) -> Construction<'_> {
        Construction {
            class: self.class,
            seed: &self.seed,
            bindings: &self.bindings,
            flags: self.flags,
            specification: self.specification.clone(),
            state_resources: self.states.clone(),
            fields: self.fields.instantiate(),
        }
    }
}
/// Scoped to one immutable world-resource owner. Positive, negative and failed
/// lookups are cached; none of them creates a projectile implicitly.
pub struct Factory<'a> {
    resources: ResourceOwner<'a>,
    templates: BTreeMap<i32, Result<Option<Template>, String>>,
}
enum ResourceOwner<'a> {
    Borrowed(&'a SceneResources),
    Shared(Arc<SceneResources>),
}
impl ResourceOwner<'_> {
    fn get(&self) -> &SceneResources {
        match self {
            Self::Borrowed(value) => value,
            Self::Shared(value) => value,
        }
    }
}
impl Factory<'static> {
    /// Retain the mission's actual immutable resources and per-class templates
    /// across ordinary births. This acknowledges no projectile flight service.
    pub fn shared(resources: Arc<SceneResources>) -> Self {
        Self {
            resources: ResourceOwner::Shared(resources),
            templates: BTreeMap::new(),
        }
    }
}
impl<'a> Factory<'a> {
    pub fn new(resources: &'a SceneResources) -> Self {
        Self {
            resources: ResourceOwner::Borrowed(resources),
            templates: BTreeMap::new(),
        }
    }
    pub fn cached_classes(&self) -> usize {
        self.templates.len()
    }
    fn construct(&mut self, session: &mut Session, class: i32) -> Result<Option<Id>, String> {
        let value = self
            .templates
            .entry(class)
            .or_insert_with(|| Template::load(self.resources.get(), class));
        match value {
            Ok(Some(template)) => session
                .construct_enemy_projectile(template.input())
                .map(Some),
            Ok(None) => Ok(None),
            Err(error) => Err(error.clone()),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    OwnedPrimary,
    /// Explicit differential fixture: constructed pose/channels, no owned
    /// motion/finalization has begun. This does not certify a full actor tick.
    IsolatedCallback,
}
pub trait Dependencies<E> {
    fn prepare_effect(
        &mut self,
        session: &mut Session,
        request: enemy_projectile::EffectRequest<'_>,
        random: &mut Random,
    ) -> Result<EffectOutcome, String>;
    fn positional_sound(
        &mut self,
        session: &mut Session,
        cue: u32,
        position: [f32; 3],
    ) -> Result<SoundOutcome, String>;
    fn owner_sound(
        &mut self,
        _session: &mut Session,
        _source: Id,
        _state: &ripper_primary_turn::State<Id, E>,
        _cue: u32,
        _random: &mut Random,
    ) -> Result<SoundOutcome, String> {
        Err("actual primary owner audio unavailable".into())
    }
    fn remove_effect(
        &mut self,
        _session: &mut Session,
        _source: Id,
        _effect: E,
        _random: &mut Random,
    ) -> Result<(), String> {
        Err("actual primary effect retirement unavailable".into())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Started,
    ResourceUnavailable,
    Launched,
    Failed,
}
#[derive(Clone, Debug)]
pub struct Attempt {
    pub phase: Phase,
    pub projectile: Option<Id>,
    pub failure: Option<String>,
}
pub struct Bridge<'a, 'r, E, S> {
    session: &'a mut Session,
    factory: &'a mut Factory<'r>,
    source: Id,
    scope: Scope,
    services: &'a mut S,
    attempt: Option<Attempt>,
    effect: PhantomData<E>,
}
impl<'a, 'r, E, S> Bridge<'a, 'r, E, S> {
    pub fn new(
        session: &'a mut Session,
        factory: &'a mut Factory<'r>,
        source: Id,
        scope: Scope,
        services: &'a mut S,
    ) -> Self {
        Self {
            session,
            factory,
            source,
            scope,
            services,
            attempt: None,
            effect: PhantomData,
        }
    }
    pub fn attempt(&self) -> Option<&Attempt> {
        self.attempt.as_ref()
    }
    pub fn session(&self) -> &Session {
        self.session
    }
}
struct ShotDependencies<'a, E, S> {
    services: &'a mut S,
    scope: Scope,
    effect: PhantomData<E>,
}
impl<E, S: Dependencies<E>> enemy_projectile::Services for ShotDependencies<'_, E, S> {
    fn prepare_effect(
        &mut self,
        session: &mut Session,
        request: enemy_projectile::EffectRequest<'_>,
        random: &mut Random,
    ) -> Result<EffectOutcome, String> {
        self.services.prepare_effect(session, request, random)
    }
    fn sound(
        &mut self,
        session: &mut Session,
        cue: u32,
        position: [f32; 3],
    ) -> Result<SoundOutcome, String> {
        self.services.positional_sound(session, cue, position)
    }
    fn muzzle(
        &mut self,
        session: &mut Session,
        source: Id,
        index: i32,
        _random: &mut Random,
    ) -> Result<[f32; 3], String> {
        session.update_current_references(source)?;
        match self.scope {
            Scope::OwnedPrimary => session.current_primary_reference_point(source, index),
            Scope::IsolatedCallback => {
                session.prepare_actor_affine(source)?;
                session.current_reference_point(source, index)
            }
        }
    }
}
impl<E: Copy, S: Dependencies<E>> ripper_primary_turn::Services<Id, E> for Bridge<'_, '_, E, S> {
    fn remove_effect(
        &mut self,
        _state: &ripper_primary_turn::State<Id, E>,
        effect: E,
        random: &mut Random,
    ) -> Result<(), String> {
        self.services
            .remove_effect(self.session, self.source, effect, random)
    }
    fn owner_sound(
        &mut self,
        state: &ripper_primary_turn::State<Id, E>,
        cue: u32,
        random: &mut Random,
    ) -> Result<SoundOutcome, String> {
        self.services
            .owner_sound(self.session, self.source, state, cue, random)
    }
    fn fire(
        &mut self,
        state: &ripper_primary_turn::State<Id, E>,
        target: [f32; 3],
        spread: f32,
        random: &mut Random,
    ) -> Result<FireOutcome, String> {
        if self.attempt.is_some() {
            return Err("primary shot attempt already consumed".into());
        }
        if !spread.is_finite() {
            return Err("primary shot spread is nonfinite".into());
        }
        self.session.publish_primary_shot_source(
            self.source,
            state,
            target,
            self.scope == Scope::OwnedPrimary,
        )?;
        self.attempt = Some(Attempt {
            phase: Phase::Started,
            projectile: None,
            failure: None,
        });
        let result: Result<FireOutcome, String> = (|| {
            let class = crate::enemy_ballistic::resource_class(state.entry.class)?;
            let Some(id) = self.factory.construct(self.session, class)? else {
                self.attempt.as_mut().unwrap().phase = Phase::ResourceUnavailable;
                return Ok(FireOutcome::ResourceUnavailable);
            };
            self.attempt.as_mut().unwrap().projectile = Some(id);
            self.session.finish_enemy_projectile_fire(
                id,
                enemy_projectile::LaunchInput {
                    source: self.source,
                    target,
                    vertical_spread_percent: spread,
                    dispatch_context: state.entry.context,
                },
                random,
                &mut ShotDependencies {
                    services: self.services,
                    scope: self.scope,
                    effect: PhantomData::<E>,
                },
            )?;
            if self
                .session
                .enemy_projectile(id)
                .is_none_or(|p| p.phase != enemy_projectile::Phase::Launched)
            {
                return Err("primary projectile did not reach launched ownership".into());
            }
            self.attempt.as_mut().unwrap().phase = Phase::Launched;
            Ok(FireOutcome::Spawned)
        })();
        if let Err(error) = &result {
            let attempt = self.attempt.as_mut().unwrap();
            attempt.phase = Phase::Failed;
            attempt.failure = Some(error.clone());
        }
        result
    }
}
