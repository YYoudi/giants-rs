//! Bounded terrain-landing presentation for the ordinary partial controller.
//! Canonical effect ownership is real; this does not complete Mecc damage or
//! acknowledge the experimental player's primary/world schedule.
use super::*;
use giants_game::{animation_epoch::Stamp, attached_effect_context::Pool, positional_impact};

pub(super) struct Driver {
    contexts: Pool<ActorId>,
    actors: Vec<ActorId>,
    stamp: Stamp,
    failure: Option<String>,
}
impl Runtime {
    pub(crate) fn normal_landing_supported(&self, ticket: u64) -> bool {
        self.active.as_ref().is_some_and(|a| {
            a.ticket == ticket
                && a.payload.controller.as_ref().is_ok_and(|c| c.is_some())
                && a.owned_mecc.is_none()
        })
    }
    pub(crate) fn normal_landing_owner(&self, ticket: u64) -> Option<ActorId> {
        let a = self
            .active
            .as_ref()
            .filter(|a| a.ticket == ticket && a.owned_mecc.is_none())?;
        let owner = a.player_owner().ok()?;
        (a.controlled_mecc_owner == Some(owner)).then_some(owner)
    }
    pub(crate) fn normal_landing_has_work(&self, ticket: u64) -> bool {
        self.active.as_ref().is_some_and(|a| {
            a.ticket == ticket && a.landing.as_ref().is_some_and(|d| !d.actors.is_empty())
        })
    }
    pub(crate) fn normal_landing_visible(&self, ticket: u64) -> Result<bool, String> {
        let Some(a) = self.active.as_ref().filter(|a| a.ticket == ticket) else {
            return Ok(false);
        };
        let Some(d) = &a.landing else {
            return Ok(false);
        };
        for &id in &d.actors {
            if a.resident_states
                .positional_impact_draw_state(id)?
                .is_some_and(|s| s.transparency < 0.5)
            {
                return Ok(true);
            }
        }
        Ok(false)
    }
    pub(crate) fn advance_normal_landing(
        &mut self,
        ticket: u64,
        seconds: f32,
        position: Option<[f32; 3]>,
        backend: &mut (
                 impl positional_impact::Cue
                 + positional_impact::Stop
                 + giants_game::actor_retirement::Services
             ),
    ) -> Result<(), String> {
        self.normal_landing_owner(ticket)
            .ok_or("landing needs the actual ordinary motion owner")?;
        if !seconds.is_finite() || seconds < 0. {
            return Err("landing requires an actual nonnegative controller delta".into());
        }
        let a = self.active.as_mut().unwrap();
        let source = Arc::clone(a.payload.plan.resources());
        let mut driver = a.landing.take().unwrap_or_else(|| Driver {
            contexts: Pool::from_active_resources(ticket, &source),
            actors: vec![],
            stamp: Stamp::default(),
            failure: None,
        });
        let result = (|| -> Result<(), String> {
            if let Some(error) = &driver.failure {
                return Err(error.clone());
            }
            if let Some(position) = position {
                let (creation, _) = driver.contexts.prepare_positional_invariant(
                    "fx_mecc_hard_land",
                    position,
                    &mut a.variant_random,
                )?;
                let giants_game::attached_effect_context::Creation::Prepared(context) = creation
                else {
                    return Err("landing source context absent".into());
                };
                let id = positional_impact::Factory::new(&source).activate(
                    &mut a.resident_states,
                    &mut driver.contexts,
                    context,
                    &mut a.variant_random,
                    backend,
                )?;
                driver.actors.push(id);
                println!(
                    "mission_landing_born ticket={ticket} actor={id:?} position={position:?} source=actual_terrain_impact native_damage=false world_schedule_verified=false"
                );
            }
            let mut live = Vec::new();
            for &actor in &driver.actors {
                let step = a.resident_states.advance_positional_impact(
                    actor,
                    seconds,
                    &mut driver.stamp,
                    backend,
                )?;
                if step == positional_impact::Step::Marked {
                    a.resident_states
                        .cleanup_positional_impact(actor, backend)?;
                    println!(
                        "mission_landing_retired ticket={ticket} actor={actor:?} canonical_cleanup=true"
                    );
                } else {
                    live.push(actor);
                }
            }
            driver.actors = live;
            Ok(())
        })();
        if let Err(error) = &result {
            driver.failure = Some(error.clone());
        }
        a.landing = Some(driver);
        result
    }
}
