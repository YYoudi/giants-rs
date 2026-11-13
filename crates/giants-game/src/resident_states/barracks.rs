//! Canonical state98 initial branch. Prototype PS2 `barracks_logic` remains an
//! unknown-attribution route. Later spawning/AI and world phases remain separate.
use super::{Id, Session};
use crate::building_barracks::{self, InitialParameters, State};

#[derive(Debug)]
pub(super) struct Owned {
    pub(super) state: State<Id>,
    failure: Option<String>,
}
struct Sound<'a, F> {
    id: Id,
    request: &'a mut F,
}
impl<F: FnMut(Id, u32) -> Result<(), String>> building_barracks::InitialSound<Id> for Sound<'_, F> {
    fn object_sound(&mut self, _: &State<Id>, cue: u32) -> Result<(), String> {
        (self.request)(self.id, cue)
    }
}
impl Session {
    /// Execute the acquired initial branch once on its actual mission owner.
    /// `request_sound` must accept the real object-owned request; returning Ok
    /// does not acknowledge a backend voice. Neither this method nor a retained
    /// initial state acknowledges subsequent AI or an actor world tick.
    pub fn initialize_barracks(
        &mut self,
        id: Id,
        stock: InitialParameters,
        suppressed: bool,
        request_sound: &mut impl FnMut(Id, u32) -> Result<(), String>,
    ) -> Result<bool, String> {
        let actor = self.get(id).ok_or("barracks owner absent or foreign")?;
        if actor.barracks.is_some() {
            return Err("barracks initial attempt already consumed; do not replay".into());
        }
        if !actor.acquired_construction
            || actor.seed.storage.state() != 98
            || !matches!(actor.class, 1060 | 1226)
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || actor.world_tick_running
            || self.specification(id).is_none()
            || self.state_resources(id).is_none()
        {
            return Err("barracks initialization requires its acquired cold owner".into());
        }
        if suppressed {
            return Ok(false);
        }
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("barracks fields absent")?;
        let mut channels = actor.channels.ok_or("barracks channels absent")?;
        let mut state = State {
            position: fields.position,
            target: None,
            phase: 0,
            selector: fields.action as i32,
            damage: 0.,
            limit: f32::from_bits(fields.configuration[12]),
            mark_flags: fields.response_flags,
            remaining: 0,
        };
        let result = building_barracks::initialize(
            &mut state,
            actor.class,
            stock,
            false,
            &mut channels,
            &mut Sound {
                id,
                request: request_sound,
            },
        );
        let actor = self.residents.get_mut(&id.serial).unwrap();
        // Failures retain completed numeric writes and prevent a repeated sound
        // request. The actor identity, resources and registrations stay owned.
        actor.channels = Some(channels);
        actor.construction_fields.as_mut().unwrap().action = state.selector as u32;
        actor.barracks = Some(Owned {
            state,
            failure: result.as_ref().err().cloned(),
        });
        result
    }
    /// Initialized source state, independent of later primary/world readiness.
    /// A failed initial request is not exposed as a usable damage owner.
    pub fn barracks_state(&self, id: Id) -> Result<&State<Id>, String> {
        let owner = self
            .get(id)
            .and_then(|a| a.barracks.as_ref())
            .ok_or("barracks not initialized or foreign")?;
        if let Some(failure) = &owner.failure {
            return Err(format!("barracks initial failure: {failure}"));
        }
        Ok(&owner.state)
    }
}
