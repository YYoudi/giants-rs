//! Ordinary mixed-effect ownership across real host frames. Prototype PS2
//! `fx_process_one`, `fx_processOpCodes` and `fx_logic` remain unknown search
//! routes; construction, this partial cohort schedule and GPU delivery differ.
use super::{Actor, Creation, Execution, Id, Pool};
use crate::{
    actor_finalization::{PhysicalProgress, Time},
    animation_epoch::Stamp,
    audio_parameters::Random,
    enemy_projectile::ImpactEffect,
    object_time::{self, SlowArea},
    resident_states::{Session, TimedEffectPreparationServices, TimedEffectSourcePreparation},
    scene_resources::SceneResources,
    timed_effect_object::{Destruction, Step},
};
use std::collections::{BTreeMap, BTreeSet};

struct Running {
    execution: Execution,
    observed: usize,
}
/// Real caller clocks/progress policy, never a newly seeded simulation or
/// fabricated PC argument. Context waits consume global time; bodies select
/// their current object-local time independently.
pub struct Frame<'a> {
    pub number: u64,
    pub global_seconds: f32,
    pub areas: &'a [SlowArea],
    pub progress: PhysicalProgress,
    pub pc_argument: Option<u32>,
}
/// Borrow the common owners for one phase; no independent clock, random stream,
/// world supplier or availability acknowledgment is created here.
pub struct Update<'a, S> {
    pub pool: &'a mut Pool<Actor>,
    pub resources: &'a SceneResources,
    pub random: &'a mut Random,
    pub stamp: &'a mut Stamp,
    pub world: &'a mut S,
}
#[derive(Default)]
pub struct Prepared {
    pub born: Vec<Actor>,
    /// Actual common-list order, not constructor/birth order.
    pub callbacks: Vec<(Actor, Step)>,
}
/// Own only admitted mixed effects. Other actor/context families retain their
/// real owners; this is not a complete native common-world traversal.
pub struct Population {
    ticket: u64,
    running: Vec<Running>,
    actors: BTreeMap<Actor, Id>,
    expired: BTreeSet<Actor>,
    last_frame: Option<u64>,
    awaiting_cleanup: bool,
    failure: Option<String>,
}
impl Population {
    pub fn new(ticket: u64) -> Self {
        Self {
            ticket,
            running: Vec::new(),
            actors: BTreeMap::new(),
            expired: BTreeSet::new(),
            last_frame: None,
            awaiting_cleanup: false,
            failure: None,
        }
    }
    pub fn failure(&self) -> Option<&str> {
        self.failure.as_deref()
    }
    pub fn actors(&self) -> impl Iterator<Item = Actor> + '_ {
        self.actors.keys().copied()
    }
    pub fn settled(&self) -> bool {
        self.failure.is_none()
            && !self.awaiting_cleanup
            && self.running.is_empty()
            && self.actors.is_empty()
    }
    fn check(&self, session: &Session, pool: &Pool<Actor>) -> Result<(), String> {
        if self.ticket != session.ticket() || self.ticket != pool.ticket {
            return Err("mixed population belongs to another Session".into());
        }
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        Ok(())
    }
    /// Real named mode8 preparation, retained for the next effect phase. No
    /// constructor ledger or completed context is reported as rendered output.
    pub fn prepare_impact(
        &mut self,
        session: &Session,
        pool: &mut Pool<Actor>,
        request: &ImpactEffect,
        random: &mut Random,
    ) -> Result<Creation, String> {
        self.check(session, pool)?;
        if self.awaiting_cleanup {
            return Err("mixed population cleanup phase still pending".into());
        }
        let result = pool.prepare_enemy_projectile_impact_invariant(request, random);
        match result {
            Ok((creation, _)) => {
                if let Creation::Prepared(context) = creation {
                    self.running.push(Running {
                        execution: Execution::new(context),
                        observed: 0,
                    });
                }
                Ok(creation)
            }
            Err(error) => {
                self.failure = Some(error.clone());
                Err(error)
            }
        }
    }
    /// Context phase precedes this cohort's ordinary body preparation. Bodies
    /// follow their current Session common-list order. Surrounding actor/context
    /// interleaving remains the composing application's explicit schedule.
    pub fn advance(
        &mut self,
        session: &mut Session,
        frame: Frame<'_>,
        update: Update<'_, impl TimedEffectPreparationServices>,
    ) -> Result<Prepared, String> {
        self.check(session, update.pool)?;
        if self.awaiting_cleanup || self.last_frame.is_some_and(|last| frame.number <= last) {
            return Err("mixed population frame already consumed or cleanup pending".into());
        }
        if !frame.global_seconds.is_finite()
            || frame.global_seconds < 0.
            || frame.areas.iter().any(|a| {
                !a.center
                    .into_iter()
                    .chain([a.radius_squared, a.factor])
                    .all(f32::is_finite)
            })
        {
            return Err("mixed population requires actual finite world clocks and areas".into());
        }
        self.last_frame = Some(frame.number);
        self.awaiting_cleanup = true;
        let result = self.advance_inner(session, &frame, update);
        if let Err(error) = &result {
            self.failure = Some(error.clone());
        }
        result
    }
    fn advance_inner(
        &mut self,
        session: &mut Session,
        frame: &Frame<'_>,
        update: Update<'_, impl TimedEffectPreparationServices>,
    ) -> Result<Prepared, String> {
        let mut prepared = Prepared::default();
        for running in &mut self.running {
            let result = if !running.execution.complete() {
                // The acquired positional wrapper queries global time with
                // neither object nor point; body slow-area time is not reused.
                running
                    .execution
                    .advance(
                        update.pool,
                        session,
                        update.resources,
                        frame.global_seconds,
                        update.random,
                    )
                    .map(|_| ())
            } else {
                Ok(())
            };
            for &actor in &running.execution.actors()[running.observed..] {
                if self
                    .actors
                    .insert(actor, running.execution.context())
                    .is_some()
                {
                    return Err("mixed actor already belongs to another context".into());
                }
                prepared.born.push(actor);
            }
            running.observed = running.execution.actors().len();
            // Preserve births from a failed constructor prefix before latching.
            result?;
        }
        for actor in self.actors.keys() {
            session
                .get(*actor)
                .ok_or("mixed body retired outside its owning population")?;
        }
        let ordered = session
            .list(0)
            .into_iter()
            .filter(|actor| self.actors.contains_key(actor))
            .collect::<Vec<_>>();
        if ordered.len() != self.actors.len() {
            return Err("mixed body common-list membership absent".into());
        }
        for actor in ordered {
            if self.expired.contains(&actor) {
                continue;
            }
            let fields = session
                .get(actor)
                .unwrap()
                .construction_fields
                .as_ref()
                .ok_or("mixed body fields absent")?;
            let context = fields.dispatch_context;
            let seconds = object_time::seconds(
                frame.global_seconds,
                Some(object_time::Sample {
                    owner: context,
                    position: fields.position,
                }),
                None,
                frame.areas,
            ) as f32;
            session.begin_timed_positioned_frame(actor)?;
            let step = session.prepare_timed_positioned_source(
                actor,
                TimedEffectSourcePreparation {
                    seconds,
                    finalization_time: Time {
                        global_seconds: frame.global_seconds,
                        context,
                        areas: frame.areas,
                    },
                    progress: frame.progress,
                    pc_argument: frame.pc_argument,
                },
                update.stamp,
                update.world,
            )?;
            prepared.callbacks.push((actor, step));
            if step == Step::Marked {
                self.expired.insert(actor);
            }
        }
        Ok(prepared)
    }
    /// Call after consuming the current prepared render inputs. No audio/device
    /// release is invented: the strict state78 destructor refuses owners that
    /// require unbound blocks, footprints or dependent actors.
    pub fn cleanup(
        &mut self,
        session: &mut Session,
        pool: &Pool<Actor>,
    ) -> Result<Vec<(Actor, Destruction)>, String> {
        self.check(session, pool)?;
        if !self.awaiting_cleanup {
            return Err("mixed population has no completed frame to clean".into());
        }
        let result: Result<Vec<(Actor, Destruction)>, String> = (|| {
            let mut retired = Vec::new();
            let ordered = session
                .list(0)
                .into_iter()
                .filter(|actor| self.expired.contains(actor))
                .collect::<Vec<_>>();
            if ordered.len() != self.expired.len() {
                return Err("expired mixed common-list membership absent".into());
            }
            for actor in ordered {
                let origin = *self
                    .actors
                    .get(&actor)
                    .ok_or("expired mixed origin absent")?;
                // An active origin is retained, not released to manufacture a
                // destructor precondition. Its real wait must finish first.
                if !pool.retired(origin) {
                    continue;
                }
                let destruction = session.destroy_expired_mixed_particle(actor, pool)?;
                self.actors.remove(&actor);
                self.expired.remove(&actor);
                retired.push((actor, destruction));
            }
            self.running.retain(|r| {
                !r.execution.complete()
                    || r.execution
                        .actors()
                        .iter()
                        .any(|id| self.actors.contains_key(id))
            });
            self.awaiting_cleanup = false;
            Ok(retired)
        })();
        if let Err(error) = &result {
            self.failure = Some(error.clone());
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn foreign_pool_cannot_prepare_an_origin_or_poison_the_real_owner() {
        let session = Session::new(17);
        let mut foreign = Pool::new(18, None);
        let mut owner = Population::new(session.ticket());
        let mut random = Random(29);
        let request = crate::enemy_projectile::impact_effect(0, [1., 2., 3.]).unwrap();
        assert!(
            owner
                .prepare_impact(&session, &mut foreign, &request, &mut random)
                .is_err()
        );
        assert_eq!(random.0, 29);
        assert!(foreign.active().is_empty());
        assert!(owner.actors().next().is_none());
        assert!(owner.failure().is_none());
        assert!(owner.settled());
    }
}
