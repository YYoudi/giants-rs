//! Source health object required by the class73/230 lethal callback.
//! Prototype PS2 `ripper_drop_health__FP6OBJECT` (`ripper_drop_health`) is a
//! navigation candidate only; its
//! correspondence to the PC callback or state37 primary remains unknown.

//! This constructs the acquired callback suffix, not item physics/collection.
use crate::{
    actor_fields::Fields,
    arithmetic::Precision,
    attached_effect_context::Creation,
    audio_parameters::Random,
    model_registry::StateBinding,
    object_lists::DirectFlags,
    resident_states::{Construction, Id, Session},
    scene_resources::SceneResources,
    state_storage::{Profile, Seed},
};
use std::sync::Arc;

/// Bound to the actual source type36 declaration. An absent/unacquired field is
/// an error, separate from a declared family that does not admit the Mecc.
#[derive(Clone, Copy, Debug)]
pub struct PickupDefinition {
    player_type: i32,
}
impl PickupDefinition {
    pub fn load(catalog: &giants_formats::weapons::Catalog) -> Result<Self, String> {
        let icon = catalog
            .icons
            .get(&36)
            .filter(|icon| icon.id == 36)
            .ok_or("Ripper health item36 definition absent")?;
        Ok(Self {
            player_type: icon
                .player_type
                .ok_or("Ripper health player-family declaration absent or unacquired")?,
        })
    }
    /// Bounded PC type36 eligibility, not contact, healing or retirement.
    /// Other player families keep their separate native admission rules.
    pub fn mecc_eligible(self, health: crate::mecc_damage::State) -> Result<bool, String> {
        if ![health.damage, health.limit, health.shield]
            .into_iter()
            .all(f32::is_finite)
            || health.limit <= 0.
            || health.shield < 0.
        {
            return Err("Ripper health eligibility requires finite actual Mecc health".into());
        }
        Ok(matches!(self.player_type, 1 | 2) && health.damage > 0. && health.damage < health.limit)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Owned {
    pub owner: Id,
    pub amount: i32,
    pub owner_delay: f32,
    pub lifetime: f32,
    pub context: Option<crate::attached_effect_context::Id>,
    /// Constructor/amount/control suffix only, not a completed physical frame
    /// or active/running effect.
    pub initialized: bool,
    pub failed: bool,
}
/// Actual mode4 attachment; Prepared retains a context, not GPU readiness.
pub trait Effects {
    fn attach(
        &mut self,
        session: &mut Session,
        item: Id,
        name: &'static str,
        random: &mut Random,
    ) -> Result<Creation, String>;
}
pub struct Factory {
    seed: Seed,
    bindings: Vec<StateBinding>,
    fields: Fields,
    flags: DirectFlags,
    body: Arc<crate::specification::Prepared>,
    states: Arc<crate::state_resources::Prepared>,
    amount: [i32; 2],
}
impl Factory {
    pub fn load(
        resources: &SceneResources,
        globals: &giants_formats::global_data::GlobalData,
    ) -> Result<Self, String> {
        let program = resources
            .classes
            .models
            .first_program(785)
            .ok_or("Ripper health class785 absent")?
            .program
            .as_ref()
            .map_err(Clone::clone)?;
        let seed = Seed::from_program_for(program, Profile::Debug, Precision::Double53)?;
        let bindings = resources.class_state_bindings_for(785, Profile::Debug)?;
        if seed.storage.state() != 0
            || seed.storage.data().is_some()
            || !seed.diagnostics.is_empty()
            || !seed.unresolved.is_empty()
            || !bindings.is_empty()
        {
            return Err("Ripper health requires acquired empty cold0 construction".into());
        }
        let body = Arc::new(crate::specification::Prepared::load_registered(
            resources, 785,
        )?);
        let states = Arc::new(crate::state_resources::Prepared::load(
            resources,
            785,
            Profile::Debug,
        )?);
        if !body.needs.is_empty() || !states.needs().is_empty() {
            return Err("Ripper health source resources incomplete".into());
        }
        let fields = Fields::load(resources, 785, &body)?;
        let amount =
            [0x1078, 0x107c].map(|at| globals.words.get(at / 4).copied().map(|w| w as i32));
        let [Some(medium), Some(large)] = amount else {
            return Err("Ripper health quantities absent".into());
        };
        Ok(Self {
            seed,
            bindings,
            fields,
            flags: DirectFlags::project(785, program),
            body,
            states,
            amount: [medium, large],
        })
    }
    /// Only an active lethal callback can construct this child. A late effect
    /// error retains its published identity and marks its incomplete suffix.
    pub fn spawn(
        &self,
        session: &mut Session,
        owner: Id,
        random: &mut Random,
        effects: &mut impl Effects,
    ) -> Result<Id, String> {
        let state = session
            .state23_combat(owner)
            .ok_or("Ripper health source absent")?;
        let large = match state.class {
            73 => false,
            230 => true,
            _ => return Err("Ripper health source class invalid".into()),
        };
        let id = session.construct_ripper_health_drop(
            owner,
            Construction {
                class: 785,
                seed: &self.seed,
                bindings: &self.bindings,
                flags: self.flags,
                specification: self.body.clone(),
                state_resources: self.states.clone(),
                fields: self.fields.instantiate(),
            },
        )?;
        let result = effects.attach(
            session,
            id,
            if large {
                "fx_ripper_large_health"
            } else {
                "fx_ripper_medium_health"
            },
            random,
        );
        session.finish_ripper_health_drop(
            id,
            owner,
            self.amount[usize::from(large)],
            large,
            result,
        )?;
        Ok(id)
    }
}

#[cfg(test)]
mod tests;
