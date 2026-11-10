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
    /// Isolated acquired active block on an actual owned actor. Earlier growth,
    /// movement and later look/physics/ANM remain distinct required caller phases.
    /// The arrival timer persists only after its real producer has written it.
    pub fn advance_vimp_active_response_offline(
        &mut self,
        id: Id,
        number: u64,
        globals: &giants_formats::global_data::GlobalData,
        random: &mut crate::audio_parameters::Random,
        services: &mut impl Services,
    ) -> Result<(), String> {
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
        let mut state = self.vimp_active_projection(id, parent)?;
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
        let result = result.and(publication);
        if let Some(a) = self.residents.get_mut(&id.serial) {
            a.vimp_active_running = false;
            a.vimp_active_steps += 1;
            if result.is_err() {
                a.vimp_frame_stopped = true;
            }
        }
        result
    }
}
