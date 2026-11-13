//! Real owned marker provider for the surfacing primary. FX/audio remain
//! mandatory dependencies; source reset requires an admitted owned frame.
use crate::{
    actor_fields::Fields,
    arithmetic::Precision,
    audio_parameters::Random,
    enemy_projectile::{EffectOutcome, SoundOutcome},
    resident_states::{Construction, Id, Session},
    ripper_primary_turn::{self, State},
    scene_resources::SceneResources,
    state_storage::{Profile, Seed},
};
use std::{marker::PhantomData, sync::Arc};
struct Template {
    seed: Seed,
    bindings: Vec<crate::model_registry::StateBinding>,
    flags: crate::object_lists::DirectFlags,
    body: Arc<crate::specification::Prepared>,
    states: Arc<crate::state_resources::Prepared>,
    fields: Fields,
}
impl Template {
    fn load(resources: &SceneResources) -> Result<Self, String> {
        let definition = resources
            .classes
            .models
            .first_program(219)
            .ok_or("surface marker class absent")?;
        let program = definition.program.as_ref().map_err(Clone::clone)?;
        let seed = Seed::from_program_for(program, Profile::Debug, Precision::Double53)?;
        let bindings = resources.class_state_bindings_for(219, Profile::Debug)?;
        let flags = crate::object_lists::DirectFlags::project(219, program);
        let body = Arc::new(crate::specification::Prepared::load_registered(
            resources, 219,
        )?);
        let states = Arc::new(crate::state_resources::Prepared::load(
            resources,
            219,
            Profile::Debug,
        )?);
        let fields = Fields::load(resources, 219, &body)?;
        if seed.storage.state() != 48
            || seed.storage.data().is_some()
            || !seed.diagnostics.is_empty()
            || !body.needs.is_empty()
            || !states.needs().is_empty()
        {
            return Err("surface marker resource preparation outside acquired profile".into());
        }
        Ok(Self {
            seed,
            bindings,
            flags,
            body,
            states,
            fields,
        })
    }
    fn input(&self) -> Construction<'_> {
        Construction {
            class: 219,
            seed: &self.seed,
            bindings: &self.bindings,
            flags: self.flags,
            specification: self.body.clone(),
            state_resources: self.states.clone(),
            fields: self.fields.instantiate(),
        }
    }
}
pub struct Factory<'a> {
    resources: ResourceOwner<'a>,
    template: Option<Result<Template, String>>,
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
    /// Preserve one immutable specification/event identity across ordinary
    /// births without borrowing the mission payload from inside itself.
    pub fn shared(resources: Arc<SceneResources>) -> Self {
        Self {
            resources: ResourceOwner::Shared(resources),
            template: None,
        }
    }
}
impl<'a> Factory<'a> {
    pub fn new(resources: &'a SceneResources) -> Self {
        Self {
            resources: ResourceOwner::Borrowed(resources),
            template: None,
        }
    }
    pub fn cached(&self) -> bool {
        self.template.is_some()
    }
    /// Actual registered namespace used while preparing this marker's events.
    /// Callers must not execute their offsets against another parent's image.
    pub fn captured_effect_image(&self) -> Result<&giants_formats::effect_image::Image, String> {
        let models = &self.resources.get().classes.models;
        let selected = models
            .first_selection(219)
            .ok_or("surface marker selection absent")?
            .as_ref()
            .map_err(Clone::clone)?;
        let definition = models
            .specs
            .get(&selected.spec_id)
            .and_then(|definitions| definitions.first())
            .ok_or("surface marker specification absent")?;
        models.specification_effects(definition)
    }
    pub fn construct(&mut self, session: &mut Session, owner: Id) -> Result<Id, String> {
        let template = self
            .template
            .get_or_insert_with(|| Template::load(self.resources.get()));
        match template {
            Ok(template) => session.construct_linked_surface_marker(owner, template.input()),
            Err(error) => Err(error.clone()),
        }
    }
}
pub trait Dependencies<E> {
    fn surface_popup(
        &mut self,
        session: &mut Session,
        owner: Id,
        state: &State<Id, E>,
        random: &mut Random,
    ) -> Result<EffectOutcome, String>;
    fn remove_effect(
        &mut self,
        session: &mut Session,
        owner: Id,
        effect: E,
        random: &mut Random,
    ) -> Result<(), String>;
    fn owner_sound(
        &mut self,
        session: &mut Session,
        owner: Id,
        state: &State<Id, E>,
        cue: u32,
        random: &mut Random,
    ) -> Result<SoundOutcome, String>;
}
pub struct Bridge<'a, 'r, E, S> {
    session: &'a mut Session,
    factory: &'a mut Factory<'r>,
    owner: Id,
    services: &'a mut S,
    reset: bool,
    marker: Option<Id>,
    finished: bool,
    entered: bool,
    effect: PhantomData<E>,
}
impl<'a, 'r, E, S> Bridge<'a, 'r, E, S> {
    pub fn new(
        session: &'a mut Session,
        factory: &'a mut Factory<'r>,
        owner: Id,
        services: &'a mut S,
    ) -> Self {
        Self {
            session,
            factory,
            owner,
            services,
            reset: false,
            marker: None,
            effect: PhantomData,
            finished: false,
            entered: false,
        }
    }
    pub fn session(&self) -> &Session {
        self.session
    }
    pub fn marker(&self) -> Option<Id> {
        self.marker
    }
    pub fn finish(&mut self, machine: &ripper_primary_turn::Machine<Id, E>) -> Result<(), String>
    where
        E: Copy,
    {
        if self.finished
            || !self.reset
            || !machine.awaiting_motion()
            || machine.surface_marker() != self.marker
        {
            return Err("surface publication requires this completed unconsumed callback".into());
        }
        self.session
            .publish_surface_primary_result(self.owner, machine.state(), self.marker)?;
        self.finished = true;
        Ok(())
    }
}
impl<E: Copy, S: Dependencies<E>> ripper_primary_turn::Services<Id, E> for Bridge<'_, '_, E, S> {
    fn surface_popup(
        &mut self,
        state: &State<Id, E>,
        random: &mut Random,
    ) -> Result<EffectOutcome, String> {
        if self.entered {
            return Err("surface effect entry already attempted".into());
        }
        self.session.validate_surface_entry(self.owner, state)?;
        self.entered = true;
        self.services
            .surface_popup(self.session, self.owner, state, random)
    }
    fn remove_effect(
        &mut self,
        _state: &State<Id, E>,
        effect: E,
        random: &mut Random,
    ) -> Result<(), String> {
        if !self.entered {
            return Err("surface retirement requires admitted callback entry".into());
        }
        self.services
            .remove_effect(self.session, self.owner, effect, random)
    }
    fn owner_sound(
        &mut self,
        state: &State<Id, E>,
        cue: u32,
        random: &mut Random,
    ) -> Result<SoundOutcome, String> {
        if !self.entered || self.session.get(self.owner).is_none() {
            return Err("surface audio requires its current admitted owner".into());
        }
        self.services
            .owner_sound(self.session, self.owner, state, cue, random)
    }
    fn reset_surface_motion(&mut self, state: &State<Id, E>) -> Result<(), String> {
        self.session.publish_surface_reset(self.owner, state)?;
        self.reset = true;
        Ok(())
    }
    fn construct_surface_marker(&mut self, state: &State<Id, E>) -> Result<Id, String> {
        if !self.reset || self.marker.is_some() || state.entry.position[2] <= 0. {
            return Err("surface marker requires an unconsumed positive owned reset".into());
        }
        let marker = self.factory.construct(self.session, self.owner)?;
        self.marker = Some(marker);
        Ok(marker)
    }
}
