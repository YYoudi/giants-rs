use super::*;
use crate::{
    vimp_active_response as active,
    vimp_response::Predicate,
    vimp_wander::{put, scalar, word},
};

/// Required current caller services, not substitutes for world admission.
pub trait Services {
    fn session(
        &mut self,
        session: &Session,
        owner: Id,
        query: Predicate,
    ) -> Result<crate::actor_session_predicates::Snapshot<'_>, String>;
    fn seconds(&mut self, session: &Session, owner: Id) -> Result<f64, String>;
    fn orient(&mut self, session: &mut Session, owner: Id) -> Result<(), String>;
    fn action(
        &mut self,
        session: &mut Session,
        owner: Id,
        action: active::Action,
    ) -> Result<(), String>;
}
#[derive(Clone, Copy)]
pub struct Movement {
    /// Actual earlier caller decision; first-handled and later visits differ.
    pub produce_commands: bool,
    /// Required current header predicate, not inferred from an empty actor list.
    pub obstructed: bool,
}
pub trait MovementServices: Services {
    fn movement_seconds(
        &mut self,
        session: &Session,
        owner: Id,
        clock: crate::vimp_movement::Clock,
    ) -> Result<f64, String>;
}
struct MovementAdapter<'a, S> {
    session: &'a mut Session,
    services: &'a mut S,
    owner: Id,
    parent: Id,
}
impl<S: MovementServices> crate::vimp_movement::Services for MovementAdapter<'_, S> {
    fn seconds(
        &mut self,
        state: &crate::vimp_movement::State,
        clock: crate::vimp_movement::Clock,
    ) -> Result<f64, String> {
        self.session
            .publish_vimp_movement(self.owner, self.parent, state)?;
        self.services
            .movement_seconds(self.session, self.owner, clock)
    }
}
struct Adapter<'a, S> {
    session: &'a mut Session,
    services: &'a mut S,
    owner: Id,
    parent: Id,
    publish_final: bool,
}
impl<S: Services> Adapter<'_, S> {
    fn publish(&mut self, s: &active::State) -> Result<(), String> {
        self.session.publish_vimp_active(self.owner, self.parent, s)
    }
}
impl<S: Services> active::Services for Adapter<'_, S> {
    fn session(
        &mut self,
        s: &active::State,
        query: Predicate,
    ) -> Result<crate::actor_session_predicates::Snapshot<'_>, String> {
        self.publish(s)?;
        self.services.session(self.session, self.owner, query)
    }
    fn seconds(&mut self, s: &active::State) -> Result<f64, String> {
        self.publish(s)?;
        self.services.seconds(self.session, self.owner)
    }
    fn orient(&mut self, s: &mut active::State) -> Result<(), String> {
        self.publish(s)?;
        let result = self.services.orient(self.session, self.owner);
        // The heading callback precedes further predicates/stores. Reread even
        // after an error; a retired or changed owner must never be restored.
        match self.session.vimp_active_projection(self.owner, self.parent) {
            Ok(current) => *s = current,
            Err(error) => {
                self.publish_final = false;
                return Err(error);
            }
        }
        result
    }
    fn action(&mut self, s: &mut active::State, action: active::Action) -> Result<(), String> {
        self.publish(s)?;
        // Both actions are terminal in the acquired block. The real Session,
        // including cancellation or callback errors, owns the final result.
        self.publish_final = false;
        self.services.action(self.session, self.owner, action)
    }
}
impl Session {
    fn vimp_movement_projection(
        &self,
        id: Id,
        parent: Id,
        obstructed: bool,
    ) -> Result<crate::vimp_movement::State, String> {
        self.vimp_active_projection(id, parent)?;
        let a = self.get(id).unwrap();
        let v = a.vimp.unwrap();
        let f = a.construction_fields.as_ref().unwrap();
        let b = a.seed.storage.data().unwrap();
        Ok(crate::vimp_movement::State {
            uid: v.born.unwrap().uid,
            position: f.position,
            heading: f.angles[0],
            target: [scalar(b, 0x40), scalar(b, 0x44), scalar(b, 0x48)],
            damage: v.damage,
            scale: f.scale,
            gait: word(b, 0x74),
            speed: scalar(b, 0x68),
            parameter_6c: scalar(b, 0x6c),
            parameter_70: scalar(b, 0x70),
            commands: v.commands,
            obstructed,
            recovery_timer: scalar(b, 0x58),
            retry_timer: scalar(b, 0x5c),
            actor_delay: v.born.unwrap().delay,
        })
    }
    fn publish_vimp_movement(
        &mut self,
        id: Id,
        parent: Id,
        s: &crate::vimp_movement::State,
    ) -> Result<(), String> {
        self.vimp_active_projection(id, parent)?;
        let a = self.residents.get_mut(&id.serial).unwrap();
        let v = a.vimp.as_mut().unwrap();
        v.commands = s.commands;
        v.born.as_mut().unwrap().delay = s.actor_delay;
        let b = a.seed.storage.data_mut().unwrap();
        for (at, value) in [
            (0x68, s.speed),
            (0x6c, s.parameter_6c),
            (0x70, s.parameter_70),
            (0x58, s.recovery_timer),
            (0x5c, s.retry_timer),
        ] {
            put(b, at, value.to_bits());
        }
        Ok(())
    }
    /// One acquired movement/active lease, with shared RNG and owned actor state.
    /// Earlier growth/route decisions and the obstruction producer remain required.
    pub fn advance_vimp_movement_response_offline(
        &mut self,
        id: Id,
        number: u64,
        movement: Movement,
        globals: &giants_formats::global_data::GlobalData,
        random: &mut crate::audio_parameters::Random,
        services: &mut impl MovementServices,
    ) -> Result<(), String> {
        let (parent, _, parameters) = self.begin_vimp_active(id, number, globals)?;
        let result = (|| {
            let mut state = self.vimp_movement_projection(id, parent, movement.obstructed)?;
            let mut adapter = MovementAdapter {
                session: self,
                services,
                owner: id,
                parent,
            };
            let step = crate::vimp_movement::advance(
                &mut state,
                movement.produce_commands,
                globals,
                random,
                &mut adapter,
            );
            let publication = adapter.session.publish_vimp_movement(id, parent, &state);
            step.and(publication)?;
            let state = self.vimp_active_projection(id, parent)?;
            self.run_vimp_active(id, parent, state, parameters, random, services)
        })();
        self.end_vimp_active(id, result.is_err());
        result
    }
    pub fn vimp_active_steps(&self, id: Id) -> Option<u64> {
        let a = self.get(id)?;
        a.vimp?;
        Some(a.vimp_active_steps)
    }
    fn vimp_active_projection(&self, id: Id, parent: Id) -> Result<active::State, String> {
        let a = self.get(id).ok_or("Vimp active owner absent or foreign")?;
        let v = a.vimp.ok_or("Vimp active construction absent")?;
        let f = a
            .construction_fields
            .as_ref()
            .ok_or("Vimp active fields absent")?;
        let b = a.seed.storage.data().ok_or("Vimp active payload absent")?;
        if a.class != 8
            || a.seed.storage.profile() != Profile::Debug
            || a.seed.storage.state() != 21
            || !v.initialized
            || v.mode != 1
            || v.born.map(|b| b.parent) != Some(parent)
            || b.len() != 216
            || a.attachment.is_some()
            || a.presentation_parent.is_some()
            || f.object_flags & 0x80008000 != 0
        {
            return Err("Vimp active requires its original initialized mode1 herd child".into());
        }
        self.herd_vimp_inputs(parent)?;
        Ok(active::State {
            owner_word: v.born.unwrap().uid,
            position: f.position,
            wander_target: [scalar(b, 0x40), scalar(b, 0x44), scalar(b, 0x48)],
            heading: f.angles[0],
            angular_speed: v.angular_speed,
            previous_angular_speed: v.previous_angular_speed,
            commands: v.commands,
            selected_heading: scalar(b, 0xb4),
            turn_timer: v.response_turn_timer,
            delay: v.born.unwrap().delay,
            stage: word(b, 0xb8),
            active: word(b, 0xb0),
            kind: word(b, 0x74),
            priority: word(b, 0xd4) as i32,
        })
    }
    fn publish_vimp_active(&mut self, id: Id, parent: Id, s: &active::State) -> Result<(), String> {
        self.vimp_active_projection(id, parent)?;
        let a = self.residents.get_mut(&id.serial).unwrap();
        a.construction_fields.as_mut().unwrap().angles[0] = s.heading;
        let v = a.vimp.as_mut().unwrap();
        v.angular_speed = s.angular_speed;
        v.previous_angular_speed = s.previous_angular_speed;
        v.response_turn_timer = s.turn_timer;
        v.born.as_mut().unwrap().delay = s.delay;
        let b = a.seed.storage.data_mut().unwrap();
        for (at, value) in [
            (0xb4, s.selected_heading.to_bits()),
            (0xb8, s.stage),
            (0xb0, s.active),
            (0x74, s.kind),
            (0xd4, s.priority as u32),
        ] {
            put(b, at, value);
        }
        Ok(())
    }
    fn begin_vimp_active(
        &mut self,
        id: Id,
        number: u64,
        globals: &giants_formats::global_data::GlobalData,
    ) -> Result<(Id, active::State, active::Parameters), String> {
        let a = self.get(id).ok_or("Vimp active owner absent or foreign")?;
        let v = a.vimp.ok_or("Vimp active construction absent")?;
        if v.wander_pending
            || a.vimp_frame_stopped
            || a.world_tick_running
            || a.vimp_prepared_history.is_some()
            || a.vimp_vector_running
            || a.vimp_response_running
            || a.vimp_active_running
            || a.vimp_active_steps != number
            || number == u64::MAX
            || a.vimp_physical.as_ref().is_some_and(|p| {
                !p.view.physics_consumed || !p.view.animation_consumed || p.view.failure.is_some()
            })
        {
            return Err(
                "Vimp active requires a fresh isolated invocation without pending phases".into(),
            );
        }
        let parent = v
            .born
            .ok_or("Vimp active requires actual herd birth")?
            .parent;
        let state = self.vimp_active_projection(id, parent)?;
        if state.active == 0 {
            return Err("Vimp active response absent".into());
        }
        let p = |at| globals.scalar(at).map_err(|e| e.to_string());
        let parameters = active::Parameters {
            turn_rate: p(0x980)?,
            wait_low: p(0x984)?,
            wait_high: p(0x988)?,
            delay_low: p(0x92c)?,
            delay_high: p(0x930)?,
        };
        self.residents
            .get_mut(&id.serial)
            .unwrap()
            .vimp_active_running = true;
        Ok((parent, state, parameters))
    }
    fn run_vimp_active(
        &mut self,
        id: Id,
        parent: Id,
        mut state: active::State,
        parameters: active::Parameters,
        random: &mut crate::audio_parameters::Random,
        services: &mut impl Services,
    ) -> Result<(), String> {
        let mut adapter = Adapter {
            session: self,
            services,
            owner: id,
            parent,
            publish_final: true,
        };
        let result = active::advance(&mut state, parameters, random, &mut adapter);
        let publication = if adapter.publish_final {
            adapter.publish(&state)
        } else {
            Ok(())
        };
        result.and(publication)
    }
    fn end_vimp_active(&mut self, id: Id, failed: bool) {
        if let Some(a) = self.residents.get_mut(&id.serial) {
            a.vimp_active_running = false;
            a.vimp_active_steps += 1;
            if failed {
                a.vimp_frame_stopped = true;
            }
        }
    }
    /// Isolated acquired active block on an actual owned actor. Earlier growth,
    /// movement and later look/physics/ANM remain separate required caller phases.
    pub fn advance_vimp_active_response_offline(
        &mut self,
        id: Id,
        number: u64,
        globals: &giants_formats::global_data::GlobalData,
        random: &mut crate::audio_parameters::Random,
        services: &mut impl Services,
    ) -> Result<(), String> {
        let (parent, state, parameters) = self.begin_vimp_active(id, number, globals)?;
        let result = self.run_vimp_active(id, parent, state, parameters, random, services);
        self.end_vimp_active(id, result.is_err());
        result
    }
}
