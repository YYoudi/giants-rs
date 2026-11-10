use super::*;
use crate::{
    vimp_response as response, vimp_vector_response as vector,
    vimp_wander::{put, scalar, word},
};

#[derive(Clone, Copy, Debug)]
pub struct Request {
    pub number: u64,
    pub noise_position: [f32; 3],
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    Dispatch(response::Action),
    Vector(vector::Action),
}
/// Current session inputs and actual action execution are required dependencies.
/// This interface does not establish the native globals or primary caller phase.
pub trait Services {
    fn session(
        &mut self,
        session: &Session,
        owner: Id,
        query: response::Predicate,
    ) -> Result<crate::actor_session_predicates::Snapshot<'_>, String>;
    fn action(
        &mut self,
        session: &mut Session,
        owner: Id,
        parent: Id,
        action: Action,
    ) -> Result<(), String>;
}
struct VectorServices<'a, S>(&'a mut S);
impl<S: Services> super::VimpVectorServices for VectorServices<'_, S> {
    fn session(
        &mut self,
        s: &Session,
        id: Id,
        q: response::Predicate,
    ) -> Result<crate::actor_session_predicates::Snapshot<'_>, String> {
        self.0.session(s, id, q)
    }
    fn action(
        &mut self,
        s: &mut Session,
        id: Id,
        parent: Id,
        a: vector::Action,
    ) -> Result<(), String> {
        self.0.action(s, id, parent, Action::Vector(a))
    }
}
struct Adapter<'a, S> {
    session: &'a mut Session,
    services: &'a mut S,
    owner: Id,
    parent: Id,
    globals: &'a giants_formats::global_data::GlobalData,
    random: &'a mut crate::audio_parameters::Random,
    external: bool,
}
impl<S: Services> Adapter<'_, S> {
    fn publish(&mut self, s: &response::State<Id>) -> Result<(), String> {
        self.session
            .publish_vimp_response(self.owner, self.parent, s)
    }
}
impl<S: Services> response::Services<Id> for Adapter<'_, S> {
    fn session(
        &mut self,
        s: &response::State<Id>,
        q: response::Predicate,
    ) -> Result<crate::actor_session_predicates::Snapshot<'_>, String> {
        self.publish(s)?;
        self.services.session(self.session, self.owner, q)
    }
    fn random_delay(&mut self, s: &response::State<Id>) -> Result<f32, String> {
        self.publish(s)?;
        response::random_delay(
            self.random,
            self.globals.scalar(0x98c).map_err(|e| e.to_string())?,
            self.globals.scalar(0x9a8).map_err(|e| e.to_string())?,
        )
    }
    fn target_position(&mut self, _: Id) -> Result<[f32; 3], String> {
        Err("Vimp response linked target producer unacquired".into())
    }
    fn vector_response(
        &mut self,
        s: &mut response::State<Id>,
        origin: [f32; 3],
        fallback: f32,
    ) -> Result<(), String> {
        self.publish(s)?;
        self.external = true;
        let number = self
            .session
            .vimp_vector_steps(self.owner)
            .ok_or("Vimp response vector owner absent")?;
        let result = self.session.advance_vimp_vector_offline(
            self.owner,
            super::VimpVectorRequest {
                number,
                origin,
                fallback,
            },
            self.globals,
            self.random,
            &mut VectorServices(self.services),
        );
        // Vector callbacks may change fields outside the vector projection.
        // Re-read the complete acquired response projection even after failure.
        *s = self
            .session
            .vimp_response_projection(self.owner, self.parent)?;
        result
    }
    fn action(&mut self, s: &response::State<Id>, action: response::Action) -> Result<(), String> {
        self.publish(s)?;
        self.external = true;
        let result = self.services.action(
            self.session,
            self.owner,
            self.parent,
            Action::Dispatch(action),
        );
        self.session
            .check_vimp_response_owner(self.owner, self.parent)?;
        result
    }
}
impl Session {
    pub fn vimp_response_steps(&self, id: Id) -> Option<u64> {
        let a = self.get(id)?;
        a.vimp?;
        Some(a.vimp_response_steps)
    }
    fn check_vimp_response_owner(&self, id: Id, parent: Id) -> Result<(), String> {
        let a = self
            .get(id)
            .ok_or("Vimp response owner absent or foreign")?;
        let v = a.vimp.ok_or("Vimp response source construction absent")?;
        if a.class != 8
            || a.seed.storage.profile() != Profile::Debug
            || a.seed.storage.state() != 21
            || !v.initialized
            || v.born.map(|b| b.parent) != Some(parent)
            || a.attachment.is_some()
            || a.presentation_parent.is_some()
        {
            return Err(
                "Vimp response requires its original initialized unparented herd child".into(),
            );
        }
        self.herd_vimp_inputs(parent)?;
        Ok(())
    }
    fn vimp_response_projection(&self, id: Id, parent: Id) -> Result<response::State<Id>, String> {
        self.check_vimp_response_owner(id, parent)?;
        let a = self.get(id).unwrap();
        let v = a.vimp.unwrap();
        let born = v.born.unwrap();
        let f = a
            .construction_fields
            .as_ref()
            .ok_or("Vimp response fields absent")?;
        if a.channels != Some(f.channels) {
            return Err("Vimp response channel mirror differs".into());
        }
        let b = a
            .seed
            .storage
            .data()
            .ok_or("Vimp response payload absent")?;
        if b.len() != 216 || word(b, 0x8c) != 0 || word(b, 0x78) != 0 {
            return Err("Vimp response nonzero link/target requires its typed producer".into());
        }
        let m = a
            .affine_cache
            .as_ref()
            .ok_or("Vimp response affine absent")?
            .value
            .matrix;
        Ok(response::State {
            owner_word: born.uid,
            position: f.position,
            forward: [m[0], m[4], m[8]],
            physical_flags: word(b, 0),
            channel_flags: f.channels[0][8],
            priority: word(b, 0xd4) as i32,
            flags: f.control_flags,
            binding_flags: f.configuration[14],
            remaining_signal: scalar(b, 0xc4),
            signal_origin: [scalar(b, 0xc8), scalar(b, 0xcc), scalar(b, 0xd0)],
            linked_seconds: scalar(b, 0x90),
            linked: None,
            target: None,
            target_position: [scalar(b, 0x7c), scalar(b, 0x80), scalar(b, 0x84)],
            backwards: b[0x88],
            mode: v.mode,
            delay: born.delay,
            response_kind: word(b, 0x74),
            response_active: word(b, 0xb0),
            response_stage: word(b, 0xb8),
        })
    }
    fn publish_vimp_response(
        &mut self,
        id: Id,
        parent: Id,
        s: &response::State<Id>,
    ) -> Result<(), String> {
        self.check_vimp_response_owner(id, parent)?;
        if s.linked.is_some() || s.target.is_some() {
            return Err("Vimp response unowned projected link/target".into());
        }
        let a = self.residents.get_mut(&id.serial).unwrap();
        let f = a.construction_fields.as_mut().unwrap();
        f.control_flags = s.flags;
        f.configuration[14] = s.binding_flags;
        let v = a.vimp.as_mut().unwrap();
        v.mode = s.mode;
        v.born.as_mut().unwrap().delay = s.delay;
        let b = a.seed.storage.data_mut().unwrap();
        for (at, value) in [
            (0xd4, s.priority as u32),
            (0xc4, s.remaining_signal.to_bits()),
            (0x90, s.linked_seconds.to_bits()),
            (0x78, 0),
            (0x74, s.response_kind),
            (0xb0, s.response_active),
            (0xb8, s.response_stage),
        ] {
            put(b, at, value);
        }
        for (i, value) in s.target_position.into_iter().enumerate() {
            put(b, 0x7c + 4 * i, value.to_bits());
        }
        // The native response writes only byte88; opaque upper bytes survive.
        b[0x88] = s.backwards;
        Ok(())
    }
    /// Isolated response dispatch with actual inline flags/channels and nested
    /// owned vector execution. Supplied session/noise/RNG/affine entry is not a
    /// native caller or world-admission producer. Nonzero link/target state
    /// requires its typed owner; no pointer is silently cleared on admission.
    pub fn respond_vimp_offline(
        &mut self,
        id: Id,
        request: Request,
        globals: &giants_formats::global_data::GlobalData,
        random: &mut crate::audio_parameters::Random,
        services: &mut impl Services,
    ) -> Result<bool, String> {
        let a = self
            .get(id)
            .ok_or("Vimp response owner absent or foreign")?;
        let v = a.vimp.ok_or("Vimp response source construction absent")?;
        if v.wander_pending
            || a.vimp_frame_stopped
            || a.world_tick_running
            || a.vimp_prepared_history.is_some()
            || a.vimp_vector_running
            || a.vimp_response_running
            || a.vimp_active_running
            || a.vimp_response_steps != request.number
            || request.number == u64::MAX
            || a.vimp_physical.as_ref().is_some_and(|p| {
                !p.view.physics_consumed || !p.view.animation_consumed || p.view.failure.is_some()
            })
        {
            return Err(
                "Vimp response requires fresh isolated invocation without pending phases".into(),
            );
        }
        let parent = v
            .born
            .ok_or("Vimp response requires actual herd birth")?
            .parent;
        self.actor_current_affine(id)?;
        let state = self.vimp_response_projection(id, parent)?;
        self.residents
            .get_mut(&id.serial)
            .unwrap()
            .vimp_response_running = true;
        let mut adapter = Adapter {
            session: self,
            services,
            owner: id,
            parent,
            globals,
            random,
            external: false,
        };
        let result = response::respond(state, request.noise_position, &mut adapter);
        let (state, handled, error) = match result {
            Ok(o) => (o.state, o.handled, None),
            Err(e) => (e.state, false, Some(e.message)),
        };
        // Action/vector services are terminal in this helper. Their callbacks
        // already own the published state; never overwrite it with Outcome.
        let publication = if adapter.external {
            Ok(())
        } else {
            adapter.publish(&state)
        };
        let result = error
            .map_or(Ok(handled), Err)
            .and(publication.map(|_| handled));
        if let Some(a) = self.residents.get_mut(&id.serial) {
            a.vimp_response_running = false;
            a.vimp_response_steps += 1;
            if result.is_err() {
                a.vimp_frame_stopped = true;
            }
        }
        result
    }
}
