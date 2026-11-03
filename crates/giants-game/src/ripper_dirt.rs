//! Actual attached dirt context provider for the owned ordinary Ripper primary.
//! Preparation is not a first object update, emitted particles or visible FX.
//! All other services are forwarded with their actual results and failures.
use crate::{
    attached_effect_context::{Creation, Id as EffectId, Pool},
    audio_parameters::Random,
    effect_initialization::Selection,
    enemy_projectile::{EffectOutcome, SoundOutcome},
    resident_states::{Id, Session},
    ripper_primary_turn::{Services, State},
    state23_animation::FireOutcome,
};
pub struct Dirt<'a, S> {
    owner: Id,
    effects: &'a mut Pool<Id>,
    selection: Selection,
    services: &'a mut S,
}
impl<'a, S> Dirt<'a, S> {
    pub fn new(
        session: &Session,
        owner: Id,
        effects: &'a mut Pool<Id>,
        selection: Selection,
        services: &'a mut S,
    ) -> Result<Self, String> {
        session.ripper_primary(owner)?;
        Ok(Self {
            owner,
            effects,
            selection,
            services,
        })
    }
}
impl<S: Services<Id, EffectId>> Services<Id, EffectId> for Dirt<'_, S> {
    fn ground_normal_z(&mut self, state: &State<Id, EffectId>) -> Result<f32, String> {
        self.services.ground_normal_z(state)
    }
    fn observe_ground(&mut self, state: &State<Id, EffectId>) {
        self.services.observe_ground(state);
    }
    fn prepare_awakening_dirt(
        &mut self,
        state: &State<Id, EffectId>,
        random: &mut Random,
    ) -> Result<Option<EffectId>, String> {
        let (created, _) = self.effects.prepare_extended(
            "fx_ripper_dirt",
            self.owner,
            state.entry.flags,
            u32::MAX,
            Selection {
                recipient_position: state.entry.position,
                ..self.selection
            },
            random,
        )?;
        Ok(match created {
            Creation::Absent => None,
            Creation::Prepared(id) => Some(id),
        })
    }
    fn surface_popup(
        &mut self,
        state: &State<Id, EffectId>,
        random: &mut Random,
    ) -> Result<EffectOutcome, String> {
        self.services.surface_popup(state, random)
    }
    fn reset_surface_motion(&mut self, state: &State<Id, EffectId>) -> Result<(), String> {
        self.services.reset_surface_motion(state)
    }
    fn construct_surface_marker(&mut self, state: &State<Id, EffectId>) -> Result<Id, String> {
        self.services.construct_surface_marker(state)
    }
    fn stop_owner_sound(
        &mut self,
        state: &State<Id, EffectId>,
        cue: u32,
        random: &mut Random,
    ) -> Result<SoundOutcome, String> {
        self.services.stop_owner_sound(state, cue, random)
    }
    fn remove_effect(
        &mut self,
        state: &State<Id, EffectId>,
        effect: EffectId,
        random: &mut Random,
    ) -> Result<(), String> {
        self.services.remove_effect(state, effect, random)
    }
    fn owner_sound(
        &mut self,
        state: &State<Id, EffectId>,
        cue: u32,
        random: &mut Random,
    ) -> Result<SoundOutcome, String> {
        self.services.owner_sound(state, cue, random)
    }
    fn fire(
        &mut self,
        state: &State<Id, EffectId>,
        target: [f32; 3],
        vertical_spread_percent: f32,
        random: &mut Random,
    ) -> Result<FireOutcome, String> {
        self.services
            .fire(state, target, vertical_spread_percent, random)
    }
}
