//! Ordinary lethal constructor and sound consumers. Actual prepared contexts
//! retain their own execution/failure; callback completion is not rendered FX.
use super::*;
use giants_game::{
    attached_effect_context::{Creation, Pool, mixed::Execution},
    audio_parameters::Random,
    resident_states::{
        PartialPistolDeathServices, RipperDamageService, RipperDamageState, State23Services,
    },
    ripper_health_drop,
};
pub(super) struct Effect {
    execution: Execution,
    failure: Option<String>,
}
pub(super) struct Services<'a> {
    pub contexts: &'a mut Pool<Id>,
    pub factory: Result<&'a ripper_health_drop::Factory, &'a String>,
    pub image: &'a giants_formats::effect_image::Image,
    pub effects: &'a mut Vec<Effect>,
    pub sounds: &'a mut Vec<crate::mission_audio::Event>,
    pub quality: f32,
    pub lite_distance: f32,
    pub camera: [f32; 3],
    pub random: Random,
}
impl Services<'_> {
    fn selection(
        &self,
        session: &Session,
        position: [f32; 3],
    ) -> Result<giants_game::effect_initialization::Selection, String> {
        Ok(giants_game::effect_initialization::Selection {
            active_objects: session
                .counts()
                .residents
                .try_into()
                .map_err(|_| "death FX object count overflow")?,
            quality: self.quality,
            lite_distance: self.lite_distance,
            camera: self.camera,
            recipient_position: position,
        })
    }
}
impl PartialPistolDeathServices for Services<'_> {
    fn check(&self, session: &Session, actor: &RipperDamageState) -> Result<(), String> {
        self.factory.map_err(Clone::clone)?;
        if actor.parent.is_some()
            || actor.owner.is_some()
            || actor.action == 100
            || actor
                .target
                .is_some_and(|t| matches!(t.state, 47 | 49) && t.flags & 0x8000 == 0)
        {
            return Err("lethal hierarchy/group/protected-target consumers remain required".into());
        }
        session.check_ripper_corpse_without_mana()?;
        let selection = self.selection(session, actor.position)?;
        let lite = selection.prefer_lite()?;
        for name in if actor.class == 230 {
            ["fx_ripper_death_large", "fx_ripper_large_health"]
        } else {
            ["fx_ripper_death_medium", "fx_ripper_medium_health"]
        } {
            let (_, entry) = self
                .image
                .adaptive_find(name, lite)
                .map_err(|e| e.to_string())?
                .ok_or("declared lethal effect absent")?;
            let program = self
                .image
                .program(entry.offset)
                .map_err(|e| e.to_string())?;
            let mut staged = self.random;
            giants_game::effect_initialization::skip(&program, &mut staged)?;
        }
        if let Some(effect) = actor.effect {
            let context = self
                .contexts
                .get(effect)
                .ok_or("attached Ripper context absent")?;
            if context.recipient() != Some(actor.id) {
                return Err("attached Ripper context owner differs".into());
            }
            self.contexts.check_prepared_release(effect)?;
        }
        Ok(())
    }
}
struct Attach<'a>(&'a mut Pool<Id>);
impl ripper_health_drop::Effects for Attach<'_> {
    fn attach(
        &mut self,
        session: &mut Session,
        item: Id,
        name: &'static str,
        _: &mut Random,
    ) -> Result<Creation, String> {
        let flags = session
            .get(item)
            .ok_or("new health owner absent")?
            .construction_fields
            .as_ref()
            .ok_or("new health fields absent")?
            .object_flags;
        self.0.prepare(name, item, flags, u32::MAX)
    }
}
impl State23Services for Services<'_> {
    fn execute(
        &mut self,
        session: &mut Session,
        _: &RipperDamageState,
        request: RipperDamageService,
        random: &mut Random,
    ) -> Result<(), String> {
        use crate::mission_audio::Event;
        use giants_game::state23_damage::Service;
        match request {
            Service::StopAttached { actor, sound } => self.sounds.push(Event::StopActorSound {
                owner: actor,
                sound,
            }),
            Service::PlayPosition { position, sound } => {
                self.sounds.push(Event::NpcPosition { position, sound })
            }
            Service::Drop { actor, .. } => {
                let item = self.factory.map_err(Clone::clone)?.spawn(
                    session,
                    actor,
                    random,
                    &mut Attach(self.contexts),
                )?;
                println!(
                    "mission_ripper_health_born owner={actor:?} item={item:?} source_constructor=true primary_complete=false rendered=false"
                );
            }
            Service::Effect { name, position } => {
                let selection = self.selection(session, position)?;
                let (creation, _) = self
                    .contexts
                    .prepare_positional(name, position, selection, random)?;
                if let Creation::Prepared(context) = creation {
                    self.effects.push(Effect {
                        execution: Execution::new(context),
                        failure: None,
                    });
                }
                println!(
                    "mission_ripper_death_context name={name} creation={creation:?} active_bodies_complete=false rendered=false"
                );
            }
            Service::RemoveEffect { effect } => {
                self.contexts.release_prepared(effect)?;
            }
            _ => return Err("ordinary lethal service outside preflighted owner profile".into()),
        }
        Ok(())
    }
}
impl State {
    pub(in crate::mission_runtime) fn corpse_retired(&self, id: Id) -> bool {
        self.driver.retired.contains(&id)
    }

    pub(super) fn retire_corpses(
        &mut self,
        session: &mut Session,
        frame: &live::Frame<'_>,
        device: &mut impl giants_game::actor_retirement::Services,
    ) -> Result<(), String> {
        struct Release<'a, D> {
            contexts: &'a mut Pool<Id>,
            device: &'a mut D,
        }
        impl<D: giants_game::actor_retirement::Services> giants_game::actor_retirement::Services
            for Release<'_, D>
        {
            fn release(&mut self, session: &mut Session, id: Id) -> Result<(), String> {
                self.contexts.release_recipient_prepared(id)?;
                self.device.release(session, id)
            }
        }
        // Preserve common-list order. A held reference grants no release; a
        // failed actual release is quarantined individually without replay.
        let owners = self
            .bound
            .as_ref()
            .ok_or("Ripper owners absent at retirement")?
            .1
            .clone();
        for id in session.list(0) {
            let Some(owner) = owners.iter().find(|owner| owner.id == id) else {
                continue;
            };
            if self.driver.actor_failures.contains_key(&id)
                || session.actor_world_flags(id)? & 0x8000 == 0
            {
                continue;
            }
            if let Err(error) = session.check_dead_ripper_retirement(id, owner.declaration.key) {
                if self.driver.retirement_holds.get(&id) != Some(&error) {
                    eprintln!(
                        "mission_ripper_corpse_retirement_held ticket={} owner={id:?} reason={error} release_started=false",
                        frame.ticket
                    );
                    self.driver.retirement_holds.insert(id, error);
                }
                continue;
            }
            let result = session.cleanup_dead_ripper(
                id,
                owner.declaration.key,
                &mut Release {
                    contexts: self
                        .contexts
                        .as_mut()
                        .ok_or("Ripper contexts absent at corpse retirement")?,
                    device,
                },
            );
            match result {
                Ok(counts) => {
                    self.driver.remove_completed(id);
                    self.driver.retired.insert(id);
                    self.driver.retirement_holds.remove(&id);
                    self.geometry.1.remove(&id);
                    println!(
                        "mission_ripper_corpse_retired ticket={} owner={id:?} residents={} payload_bytes={} registrations={} source=ordinary_completed_corpse external_owners_released=true declaration_history_retained=true",
                        frame.ticket, counts.residents, counts.payload_bytes, counts.registrations
                    );
                }
                Err(error) => {
                    eprintln!(
                        "mission_ripper_corpse_retirement_failed ticket={} owner={id:?} error={error} independent_ai_continues=true",
                        frame.ticket
                    );
                    self.driver.quarantine_actor(id, frame.step, error);
                }
            }
        }
        Ok(())
    }

    pub(super) fn advance_death_effects(
        &mut self,
        session: &mut Session,
        frame: &live::Frame<'_>,
        random: &mut Random,
    ) {
        let Some(contexts) = self.contexts.as_mut() else {
            return;
        };
        for effect in &mut self.death_effects {
            if effect.failure.is_some() || effect.execution.complete() {
                continue;
            }
            if let Err(error) =
                effect
                    .execution
                    .advance(contexts, session, frame.resources, frame.seconds, random)
            {
                eprintln!(
                    "mission_ripper_death_effect_held ticket={} error={error} actual_bodies={} full_effect_acknowledged=false",
                    frame.ticket,
                    effect.execution.actors().len()
                );
                effect.failure = Some(error);
            }
        }
    }
}
