use super::*;
use crate::herd_frame::{Frame, Projection, Services, View};

struct Adapter<'a, 'b> {
    frame: Frame<'a>,
    services: &'a mut Services<'b>,
    born: Vec<Id>,
}
impl ActorWorldServices for Adapter<'_, '_> {
    fn controlled_actor(&self) -> Option<Id> {
        self.frame.controlled
    }
    fn primary(&mut self, session: &mut Session, source: Id) -> Result<(), String> {
        let actor = session.herd_frame_actor(source)?;
        let fields = actor.construction_fields.as_ref().unwrap();
        let view = &actor.herd_frame.as_ref().unwrap().view;
        let entry = crate::actor_primary_gate::enter(
            fields.object_flags,
            fields.configuration[14],
            [0; 8], // Source constructors clear commands; this profile has no writer.
            view.rates.map(f32::to_bits),
            view.previous_rates.map(f32::to_bits),
        );
        let herd = actor.herd.is_some();
        let primary_step = actor.herd.as_ref().map_or(0, |h| h.clock.steps);
        let actor = session.residents.get_mut(&source.serial).unwrap();
        actor.construction_fields.as_mut().unwrap().configuration[14] = entry.binding_flags;
        actor.herd_frame.as_mut().unwrap().view.previous_rates =
            entry.previous_rates.map(f32::from_bits);
        if entry.invoke && herd {
            let result = session.advance_herd_offline(
                source,
                crate::herd_generator::Tick {
                    step: primary_step,
                    global_delta: self.frame.global_seconds,
                    slow_areas: self.frame.areas,
                },
                self.services.factory,
                self.services.globals,
                self.services.random,
                self.services.ground,
            );
            let steps = session
                .get(source)
                .unwrap()
                .herd
                .as_ref()
                .unwrap()
                .clock
                .steps;
            session
                .residents
                .get_mut(&source.serial)
                .unwrap()
                .herd_frame
                .as_mut()
                .unwrap()
                .primary_steps = steps;
            self.born = result?;
        }
        // The actual state0 primary dispatch entry is absent, not an unknown
        // behavior acknowledged by a default provider.
        Ok(())
    }
    fn secondary(&mut self, session: &mut Session, source: Id) -> Result<(), String> {
        session.herd_frame_actor(source)?;
        // Actual state0 and state17 secondary dispatch entries are absent.
        Ok(())
    }
    fn object_time(&mut self, session: &Session, source: Id) -> Result<f32, String> {
        let f = session
            .herd_frame_actor(source)?
            .construction_fields
            .as_ref()
            .unwrap();
        Ok(crate::object_time::seconds(
            self.frame.global_seconds,
            Some(crate::object_time::Sample {
                owner: f.dispatch_context,
                position: f.position,
            }),
            None,
            self.frame.areas,
        ) as f32)
    }
    fn animation(&mut self, session: &mut Session, source: Id, seconds: f32) -> Result<(), String> {
        session.herd_frame_actor(source)?;
        let actor = session.residents.get_mut(&source.serial).unwrap();
        let channels = actor.channels.as_mut().ok_or("herd ANM channels absent")?;
        actor.mecc_animation_stamp = crate::herd_frame::advance_empty(
            channels,
            seconds,
            self.services.stamp,
            actor.mecc_animation_stamp,
        )?;
        actor.construction_fields.as_mut().unwrap().channels = *channels;
        Ok(())
    }
    fn finalization(&mut self, session: &mut Session, source: Id) -> Result<(), String> {
        let actor = session.herd_frame_actor(source)?;
        let header = actor.construction_fields.as_ref().unwrap();
        let mut state = actor.herd_frame.as_ref().unwrap().view.finalization.clone();
        state.fields.current = header.position;
        state.fields.previous = header.current;
        state.fields.angles = header.angles;
        state.fields.object_flags = header.object_flags;
        state.fields.response_flags = header.response_flags;
        state.fields.collision_flags = header.configuration[14];
        state.fields.model_vertical_speed = actor.herd_frame.as_ref().unwrap().view.rates[0];
        state.affine = actor.affine_cache.as_ref().map(|c| c.value);
        let result = crate::actor_finalization::finalize(
            state,
            crate::actor_finalization::Time {
                global_seconds: self.frame.global_seconds,
                context: header.dispatch_context,
                areas: self.frame.areas,
            },
            self.services.finalization,
        );
        let (state, error) = match result {
            Ok(s) => (s, None),
            Err(e) => (*e.state, Some(e.message)),
        };
        let actor = session.residents.get_mut(&source.serial).unwrap();
        let header = actor.construction_fields.as_mut().unwrap();
        header.position = state.fields.current;
        header.current = state.fields.previous;
        header.angles = state.fields.angles;
        header.object_flags = state.fields.object_flags;
        header.response_flags = state.fields.response_flags;
        header.configuration[14] = state.fields.collision_flags;
        if error.is_none() {
            let value = state.affine.ok_or("herd finalization affine absent")?;
            actor.affine_cache = Some(OwnedAffine {
                value,
                pose: affine_pose_words(header),
                external_orientation: header.object_flags & 0x10000 != 0,
            });
        }
        let view = &mut actor.herd_frame.as_mut().unwrap().view;
        view.rates[0] = state.fields.model_vertical_speed;
        view.finalization = state;
        error.map_or(Ok(()), Err)
    }
}
impl Session {
    fn herd_frame_actor(&self, id: Id) -> Result<&Resident, String> {
        let actor = self.get(id).ok_or("herd frame owner absent or foreign")?;
        if !actor.acquired_construction
            || actor.seed.storage.profile() != Profile::Debug
            || actor.herd_frame.is_none()
            || !matches!(
                (actor.class, actor.seed.storage.state()),
                (49, 17) | (679, 0)
            )
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || actor.construction_fields.is_none()
        {
            return Err("herd frame requires acquired unparented herd/basic marker".into());
        }
        let fields = actor.construction_fields.as_ref().unwrap();
        if fields.object_flags & 0x10000 != 0
            || fields.configuration[20] != u32::from(actor.seed.storage.state())
            || actor.channels.is_none_or(|c| c.iter().any(|v| v[0] != 254))
        {
            return Err("herd frame animation/orientation profile changed without producer".into());
        }
        Ok(actor)
    }
    pub fn herd_world_frame(&self, id: Id) -> Result<&View, String> {
        Ok(&self.herd_frame_actor(id)?.herd_frame.as_ref().unwrap().view)
    }
    /// Separate active-node position-history pass. Caller owns surrounding
    /// world phases and must prepare the whole admitted population before callbacks.
    pub fn prepare_herd_world_history(&mut self, id: Id, number: u64) -> Result<(), String> {
        let actor = self.herd_frame_actor(id)?;
        let owned = actor.herd_frame.as_ref().unwrap();
        if actor.world_tick_running
            || owned.stopped
            || owned.view.failure.is_some()
            || owned.history.is_some()
            || owned.view.next_frame != number
            || actor.construction_fields.as_ref().unwrap().object_flags & 0xc000 != 0
            || actor
                .herd
                .as_ref()
                .is_some_and(|h| h.clock.steps != owned.primary_steps || h.clock.failure.is_some())
        {
            return Err(
                "herd history requires fresh active local frame without isolated primary updates"
                    .into(),
            );
        }
        let actor = self.residents.get_mut(&id.serial).unwrap();
        actor
            .construction_fields
            .as_mut()
            .unwrap()
            .advance_position_history();
        actor.herd_frame.as_mut().unwrap().history = Some(number);
        actor.world_tick = None;
        Ok(())
    }
    /// Consume exactly one prepared history with the common per-node policy.
    /// This does not establish world admission or supply environment services.
    pub fn advance_herd_world_after_history(
        &mut self,
        id: Id,
        frame: Frame<'_>,
        services: &mut Services<'_>,
    ) -> Result<Projection, String> {
        let actor = self.herd_frame_actor(id)?;
        let owned = actor.herd_frame.as_ref().unwrap();
        if actor.world_tick_running
            || actor.world_tick.is_some()
            || owned.stopped
            || owned.view.failure.is_some()
            || owned.history != Some(frame.number)
            || owned.view.next_frame != frame.number
            || actor
                .herd
                .as_ref()
                .is_some_and(|h| h.clock.steps != owned.primary_steps || h.clock.failure.is_some())
            || frame.number == u64::MAX
            || !frame.global_seconds.is_finite()
            || frame.global_seconds < 0.
            || frame.areas.iter().any(|a| {
                !a.factor.is_finite()
                    || a.factor < 0.
                    || !a.radius_squared.is_finite()
                    || a.radius_squared < 0.
                    || !a.center.into_iter().all(f32::is_finite)
            })
        {
            return Err(
                "herd world callback requires fresh prepared history and finite clock".into(),
            );
        }
        let actor = self.residents.get_mut(&id.serial).unwrap();
        actor.world_tick_running = true;
        actor.herd_frame.as_mut().unwrap().history = None;
        let mut tick = crate::actor_world_tick::Tick::new();
        let mut adapter = Adapter {
            frame,
            services,
            born: Vec::new(),
        };
        let result = tick.run(&mut ActorWorldAdapter {
            session: self,
            source: id,
            services: &mut adapter,
        });
        let actor = self
            .residents
            .get_mut(&id.serial)
            .ok_or("herd world owner removed during callback")?;
        actor.world_tick_running = false;
        actor.world_tick = Some(tick);
        let owned = actor.herd_frame.as_mut().unwrap();
        match result {
            Ok(outcome) => {
                owned.stopped = outcome == crate::actor_world_tick::Outcome::WorldAborted;
                owned.view.next_frame += 1;
                owned.view.finalization.fields.object_flags =
                    actor.construction_fields.as_ref().unwrap().object_flags;
                Ok(Projection {
                    outcome,
                    born: adapter.born,
                })
            }
            Err(e) => {
                owned.view.failure = Some(e.clone());
                Err(e)
            }
        }
    }
}
