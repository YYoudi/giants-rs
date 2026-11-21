//! Owned callback scope and partial publication for acquired Ripper damage.
//! Prototype PS2 `ripper_shot` remains an unknown relationship to PC debug-1.1
//! 0x004dafd0. The typed partial-pistol entry composes bounded hit routing;
//! canonical projectile allocation/launch and lethal services stay separate.
use super::*;
use crate::{audio_parameters::Random, state23_damage};
mod partial_pistol;
pub use partial_pistol::{
    Contact as PartialPistolContact, DeathServices as PartialPistolDeathServices,
    Services as PartialPistolServices,
};

#[derive(Debug, Default)]
pub(super) struct Owned {
    active: Option<Arc<()>>,
    failed: Option<String>,
}

struct Adapter<'a, S: ?Sized> {
    session: &'a mut Session,
    services: &'a mut S,
    owner: Id,
    attempt: Arc<()>,
}
impl<S: State23Services + ?Sized> state23_damage::Services<Id, crate::attached_effect_context::Id>
    for Adapter<'_, S>
{
    fn execute(
        &mut self,
        actor: &RipperDamageState,
        service: RipperDamageService,
        random: &mut Random,
    ) -> Result<(), String> {
        self.session
            .publish_ripper_damage_stage(self.owner, &self.attempt, actor)?;
        let removed = match &service {
            state23_damage::Service::RemoveEffect { effect } => Some(*effect),
            _ => None,
        };
        let result = self.services.execute(self.session, actor, service, random);
        if !self
            .session
            .ripper_damage_attempt(self.owner, &self.attempt)
        {
            return Err("damage service removed or replaced its callback scope".into());
        }
        if result.is_ok()
            && let Some(effect) = removed
        {
            let primary = self
                .session
                .residents
                .get_mut(&self.owner.serial)
                .unwrap()
                .ripper_primary
                .as_mut()
                .ok_or("removed Ripper effect lost its primary owner")?;
            let mut state = primary.state().clone();
            if state.effect != Some(effect) {
                return Err("removed Ripper effect differs from its canonical primary".into());
            }
            state.effect = None;
            primary.refresh_owned_frame(state)?;
        }
        result
    }
}

impl Session {
    /// Actual list3 family exclusion for the acquired Mecc-only corpse path.
    /// A possible mana recipient requires its genuine mana/creation owner.
    pub fn check_ripper_corpse_without_mana(&self) -> Result<(), String> {
        for id in self.list(3) {
            let actor = self.get(id).ok_or("corpse player-list owner absent")?;
            let fields = actor
                .construction_fields
                .as_ref()
                .ok_or("corpse player fields absent")?;
            if fields.configuration[20] == 3 || actor.class == 1080 {
                return Err("Ripper corpse has a potentially mana-eligible player family".into());
            }
        }
        Ok(())
    }
    pub fn update_ripper_corpse_without_mana(
        &mut self,
        id: Id,
        input: crate::ripper_primary_turn::FrameInput<'_, Id>,
        global_seconds: f32,
        parameters: crate::ripper_corpse::Parameters,
        random: &mut Random,
        services: &mut impl crate::ripper_primary_turn::Services<Id, crate::attached_effect_context::Id>,
    ) -> Result<crate::ripper_primary_turn::Outcome, String> {
        let mut corpse = self.state23_snapshot(id)?;
        let amount = if corpse.class == 230 {
            parameters.large_mana
        } else {
            parameters.medium_mana
        };
        if f64::from(corpse.timer) - f64::from(global_seconds) >= 0.
            && amount != 0.
            && corpse.control & 8 == 0
        {
            self.check_ripper_corpse_without_mana()?;
        }
        struct NoManaFamily;
        impl crate::ripper_corpse::Services<Id, crate::attached_effect_context::Id> for NoManaFamily {
            fn create_mana(
                &mut self,
                _: &RipperDamageState,
                _: f32,
                _: &mut Random,
            ) -> Result<bool, String> {
                // The actual source list3 was checked immediately above. This
                // branch creates nothing and consumes no RNG in the PC helper.
                Ok(false)
            }
        }
        let result = self.update_ripper_owned_primary(
            id,
            input,
            random,
            services,
            |_, machine, input, random, services| {
                machine.advance_corpse(
                    input.context,
                    &mut corpse,
                    global_seconds,
                    parameters,
                    random,
                    &mut NoManaFamily,
                    services,
                )
            },
        );
        if matches!(
            result,
            Ok(crate::ripper_primary_turn::Outcome::CompletedCorpse)
        ) && self.actor_world_flags(id)? & 0x8000 != 0
        {
            // Witness the source corpse callback, not caller-supplied flags.
            self.residents
                .get_mut(&id.serial)
                .unwrap()
                .ripper_retirement
                .marked = true;
        }
        result
    }
    pub(super) fn ripper_damage_is_running(&self, id: Id) -> bool {
        self.get(id).is_some_and(|actor| {
            actor.ripper_damage.active.is_some() && actor.ripper_damage.failed.is_none()
        })
    }
    pub fn state23_damage_failure(&self, id: Id) -> Option<&str> {
        self.get(id)?.ripper_damage.failed.as_deref()
    }
    pub(super) fn check_ripper_damage_available(&self, id: Id) -> Result<(), String> {
        let actor = self.get(id).ok_or("damage actor absent or foreign")?;
        if actor.ripper_damage.active.is_some() || actor.ripper_damage.failed.is_some() {
            return Err(
                "Ripper damage callback active or failed; fresh actor scope required".into(),
            );
        }
        Ok(())
    }
    fn ripper_damage_attempt(&self, id: Id, attempt: &Arc<()>) -> bool {
        self.get(id).is_some_and(|actor| {
            actor
                .ripper_damage
                .active
                .as_ref()
                .is_some_and(|current| Arc::ptr_eq(current, attempt))
        })
    }
    fn publish_ripper_damage_stage(
        &mut self,
        id: Id,
        attempt: &Arc<()>,
        state: &RipperDamageState,
    ) -> Result<(), String> {
        if state.id != id || !self.ripper_damage_attempt(id, attempt) {
            return Err("damage publication lost its actual callback scope".into());
        }
        let actor = self.residents.get_mut(&id.serial).unwrap();
        if actor.class != state.class || actor.seed.storage.state() != 23 {
            return Err("damage callback actor identity changed".into());
        }
        let fields = actor
            .construction_fields
            .as_mut()
            .ok_or("damage callback fields removed")?;
        fields.object_flags = state.flags;
        fields.configuration[14] = state.personality;
        fields.control_flags = state.control;
        fields.action = state.action;
        actor.state23_combat = Some(state.clone());
        Ok(())
    }

    /// Acquired unparented callback only. General routing and launched projectile
    /// contacts require their own APIs. Publish the canonical prefix before each
    /// external service and retain the last prefix/RNG on error. A failed scope
    /// cannot retry the callback or continue ordinary primary/ANM work.
    pub fn damage_state23(
        &mut self,
        id: Id,
        amount: f32,
        source: Option<Id>,
        context: state23_damage::Context,
        random: &mut Random,
        services: &mut impl State23Services,
    ) -> Result<state23_damage::Outcome, String> {
        self.check_ripper_damage_available(id)?;
        let actor = self.get(id).unwrap();
        if actor.seed.storage.profile() != Profile::Debug
            || actor.seed.storage.state() != 23
            || !matches!(actor.class, 73 | 230)
            || !actor.acquired_construction
            || self.specification(id).is_none()
            || self.state_resources(id).is_none()
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
        {
            return Err("damage target lacks acquired unparented D state23 construction".into());
        }
        let source = source.map(|id| -> Result<state23_damage::Source<Id>, String> {
            let actor = self.get(id).ok_or("damage source is not owned by this ticket")?;
            if let Some(projectile) = &actor.basic_projectile {
                return Err(if projectile.launch.is_some() {
                    "launched projectile requires acquired contact/hit dispatch before damage"
                } else {
                    "constructed projectile requires acquired launch/owner initialization before damage"
                }.into());
            }
            if actor.enemy_projectile.is_some() {
                return Err("constructed enemy projectile requires its ordered launch and hit dispatch before damage".into());
            }
            if actor.seed.storage.profile() != Profile::Debug
                || !actor.acquired_construction
                || actor.construction_fields.is_none()
                || self.specification(id).is_none()
                || self.state_resources(id).is_none()
            {
                return Err("damage source requires a complete acquired D constructor".into());
            }
            Ok(state23_damage::Source {
                id,
                state: u32::from(actor.seed.storage.state()),
                owner: actor.state23_combat.as_ref().and_then(|s| s.owner.map(|(id, _)| id)),
            })
        }).transpose()?;
        let mut state = self.state23_snapshot(id)?;
        if state.parent.is_some() {
            return Err("damage callback requires the actual parent-detachment service".into());
        }
        if state23_damage::validate(&state, amount, context)?.is_none() {
            return Ok(state23_damage::Outcome::AlreadyDead);
        }
        let attempt = Arc::new(());
        self.residents
            .get_mut(&id.serial)
            .unwrap()
            .ripper_damage
            .active = Some(attempt.clone());
        let outcome = state23_damage::apply(
            &mut state,
            amount,
            source,
            context,
            random,
            &mut Adapter {
                session: self,
                services,
                owner: id,
                attempt: attempt.clone(),
            },
        );
        if !self.ripper_damage_attempt(id, &attempt) {
            return Err("damage callback ended outside its original actor scope".into());
        }
        let publication = self.publish_ripper_damage_stage(id, &attempt, &state);
        let result = match (outcome, publication) {
            (Err(error), _) | (Ok(_), Err(error)) => Err(error),
            (Ok(outcome), Ok(())) => Ok(outcome),
        };
        let owner = &mut self.residents.get_mut(&id.serial).unwrap().ripper_damage;
        owner.failed = result.as_ref().err().cloned();
        owner.active = None;
        result
    }
}
