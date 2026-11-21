//! Full acquired callback over an actual partial-pistol contact. Actor, shot
//! and attached-context identities remain separate through every service.
use super::*;
type Effect = crate::attached_effect_context::Id;

pub trait DeathServices: State23Services {
    /// Read-only validation of actual consumers and current relationships.
    /// A refusal happens before sounds, damage, RNG or the callback latch.
    fn check(&self, session: &Session, actor: &RipperDamageState) -> Result<(), String>;
}
struct Running<'a> {
    session: &'a mut Session,
    services: &'a mut dyn DeathServices,
    target: Id,
    attempt: Arc<()>,
}
impl state23_damage::Services<Reference, Effect> for Running<'_> {
    fn execute(
        &mut self,
        actor: &state23_damage::State<Reference, Effect>,
        service: state23_damage::Service<Reference, Effect>,
        random: &mut Random,
    ) -> Result<(), String> {
        use state23_damage::Service;
        let actor = map_state(actor.clone(), Reference::actor)?;
        let service = match service {
            Service::StopAttached { actor, sound } => Service::StopAttached {
                actor: actor.actor()?,
                sound,
            },
            Service::PlayAttached { actor, sound } => Service::PlayAttached {
                actor: actor.actor()?,
                sound,
            },
            Service::Blood { actor } => Service::Blood {
                actor: actor.actor()?,
            },
            Service::PlayPosition { position, sound } => Service::PlayPosition { position, sound },
            Service::Effect { name, position } => Service::Effect { name, position },
            Service::Drop {
                actor,
                position,
                large,
                discarded_angle,
            } => Service::Drop {
                actor: actor.actor()?,
                position,
                large,
                discarded_angle,
            },
            Service::DecrementTarget { target } => Service::DecrementTarget {
                target: target.actor()?,
            },
            Service::DestroyActor { actor } => Service::DestroyActor {
                actor: actor.actor()?,
            },
            Service::ReorderGroup {
                group,
                index,
                except,
            } => Service::ReorderGroup {
                group: group.actor()?,
                index,
                except: except.actor()?,
            },
            Service::GroupMemberDied { group, decrement } => Service::GroupMemberDied {
                group: group.actor()?,
                decrement,
            },
            Service::RemoveEffect { effect } => Service::RemoveEffect { effect },
            // The lethal branch never propagates source. In particular, do not
            // manufacture a canonical state77 actor for the partial shot.
            Service::PropagateSource { .. } => {
                return Err(
                    "lethal contact unexpectedly requested nonlethal source propagation".into(),
                );
            }
        };
        state23_damage::Services::execute(
            &mut super::super::Adapter {
                session: self.session,
                services: self.services,
                owner: self.target,
                attempt: self.attempt.clone(),
            },
            &actor,
            service,
            random,
        )
    }
}
impl Session {
    pub(super) fn partial_pistol_death(
        &mut self,
        initial: RipperDamageState,
        hit: mecc_pistol::OwnedHit,
        context: state23_damage::Context,
        random: &mut Random,
        services: &mut dyn DeathServices,
    ) -> Result<Contact, String> {
        if initial.parent.is_some() {
            return Ok(Contact::UnavailableLethal(
                "attached Ripper requires actual detachment".into(),
            ));
        }
        state23_damage::validate(&initial, hit.damage(), context)?;
        let target = initial.id;
        let mut state = map_state(initial, |id| Ok(Reference::Actor(id)))?;
        let attempt = Arc::new(());
        self.residents
            .get_mut(&target.serial)
            .unwrap()
            .ripper_damage
            .active = Some(attempt.clone());
        let outcome = state23_damage::apply(
            &mut state,
            hit.damage(),
            Some(state23_damage::Source {
                id: Reference::PartialShot {
                    owner: hit.owner(),
                    serial: hit.serial(),
                },
                state: 77,
                owner: Some(Reference::Actor(hit.owner())),
            }),
            context,
            random,
            &mut Running {
                session: self,
                services,
                target,
                attempt: attempt.clone(),
            },
        );
        if !self.ripper_damage_attempt(target, &attempt) {
            return Err("lethal partial-pistol callback lost its original scope".into());
        }
        let publication = map_state(state, Reference::actor)
            .and_then(|state| self.publish_ripper_damage_stage(target, &attempt, &state));
        let result = match (outcome, publication) {
            (Err(error), _) | (_, Err(error)) => Err(error),
            (Ok(state23_damage::Outcome::Died), Ok(())) => Ok(Contact::Died),
            (Ok(_), Ok(())) => {
                Err("lethal contact did not complete its acquired death branch".into())
            }
        };
        let operation = &mut self
            .residents
            .get_mut(&target.serial)
            .unwrap()
            .ripper_damage;
        operation.active = None;
        operation.failed = result.as_ref().err().cloned();
        result
    }
}

#[cfg(test)]
mod tests;
