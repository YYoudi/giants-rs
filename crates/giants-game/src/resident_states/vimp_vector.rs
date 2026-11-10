use super::*;
use crate::{
    vimp_response::Predicate,
    vimp_vector_response as vector,
    vimp_wander::{put, scalar, word},
};

#[derive(Clone, Copy, Debug)]
pub struct Request {
    pub number: u64,
    pub origin: [f32; 3],
    pub fallback: f32,
}

/// Required external services for an isolated owned vector invocation. Session
/// snapshots are current inputs, not supplied predicate decisions. This does not
/// produce the native session globals or admit an ordinary actor primary.
pub trait Services {
    fn session(
        &mut self,
        session: &Session,
        owner: Id,
        query: Predicate,
    ) -> Result<crate::actor_session_predicates::Snapshot<'_>, String>;
    /// Prefixes are already published. Writes and retirement survive errors.
    fn action(
        &mut self,
        session: &mut Session,
        owner: Id,
        parent: Id,
        action: vector::Action,
    ) -> Result<(), String>;
}
struct Adapter<'a, S> {
    session: &'a mut Session,
    services: &'a mut S,
    owner: Id,
    parent: Id,
    valid: bool,
}
impl<S: Services> Adapter<'_, S> {
    fn publish(&mut self, s: &vector::State<Id>, r: &vector::Route<Id>) -> Result<(), String> {
        self.session
            .publish_vimp_vector(self.owner, self.parent, s, r)
    }
    fn refresh(
        &mut self,
        s: &mut vector::State<Id>,
        r: &mut vector::Route<Id>,
    ) -> Result<(), String> {
        match self.session.vimp_vector_projection(self.owner, self.parent) {
            Ok((state, route)) => {
                *s = state;
                *r = route;
                Ok(())
            }
            Err(e) => {
                self.valid = false;
                Err(e)
            }
        }
    }
}
impl<S: Services> vector::Services<Id> for Adapter<'_, S> {
    fn session(
        &mut self,
        s: &vector::State<Id>,
        route: Option<&vector::Route<Id>>,
        q: Predicate,
    ) -> Result<crate::actor_session_predicates::Snapshot<'_>, String> {
        if let Some(route) = route {
            self.publish(s, route)?;
        } else {
            self.session
                .publish_vimp_vector_actor(self.owner, self.parent, s)?;
        }
        self.services.session(self.session, self.owner, q)
    }
    fn hint(
        &mut self,
        _: &mut vector::State<Id>,
        _: &mut vector::Route<Id>,
        _: Id,
    ) -> Result<(), String> {
        Err("Vimp vector hint identity and terrain/affine producer unacquired".into())
    }
    fn action(
        &mut self,
        s: &mut vector::State<Id>,
        r: &mut vector::Route<Id>,
        action: vector::Action,
    ) -> Result<(), String> {
        self.publish(s, r)?;
        let result = self
            .services
            .action(self.session, self.owner, self.parent, action);
        // Re-read even after an error. Never restore a pre-callback projection.
        self.refresh(s, r)?;
        result
    }
}
impl Session {
    pub fn vimp_vector_steps(&self, id: Id) -> Option<u64> {
        let a = self.get(id)?;
        a.vimp?;
        Some(a.vimp_vector_steps)
    }
    fn check_vimp_vector_owner(&self, id: Id, parent: Id) -> Result<(), String> {
        let a = self.get(id).ok_or("Vimp vector owner absent or foreign")?;
        let v = a.vimp.ok_or("Vimp vector source owner absent")?;
        if a.class != 8
            || a.seed.storage.profile() != Profile::Debug
            || a.seed.storage.state() != 21
            || !v.initialized
            || v.wander_pending
            || a.vimp_frame_stopped
            || v.born.map(|b| b.parent) != Some(parent)
            || a.attachment.is_some()
            || a.presentation_parent.is_some()
        {
            return Err(
                "Vimp vector requires its original unparented initialized herd child".into(),
            );
        }
        self.herd_vimp_inputs(parent)?;
        Ok(())
    }
    fn vimp_vector_projection(
        &self,
        id: Id,
        parent: Id,
    ) -> Result<(vector::State<Id>, vector::Route<Id>), String> {
        self.check_vimp_vector_owner(id, parent)?;
        let a = self.get(id).unwrap();
        let v = a.vimp.unwrap();
        let born = v.born.unwrap();
        let f = a
            .construction_fields
            .as_ref()
            .ok_or("Vimp vector fields absent")?;
        let b = a.seed.storage.data().ok_or("Vimp vector payload absent")?;
        if b.len() != 216 || word(b, 0xa8) != 0 {
            return Err(
                "Vimp vector requires an owned hint identity for nonzero hint state".into(),
            );
        }
        let affine = a
            .affine_cache
            .as_ref()
            .ok_or("Vimp vector affine absent")?
            .value
            .matrix;
        let state = vector::State {
            actor: vector::Actor {
                owner_word: born.uid,
                position: f.position,
                forward: [affine[0], affine[4], affine[8]],
                binding_flags: f.configuration[14],
                mode: v.mode,
                response_kind: word(b, 0x74),
                response_active: word(b, 0xb0),
                response_stage: word(b, 0xb8),
            },
            parent: Some(parent),
            heading: f.angles[0],
            selected_heading: scalar(b, 0xb4),
            marker: word(b, 0x98),
            visits: word(b, 0x9c),
            counted: word(b, 0xa0),
            offsets: born.offsets,
            wander_target: [scalar(b, 0x40), scalar(b, 0x44), scalar(b, 0x48)],
            hint: None,
        };
        let p = self.get(parent).unwrap();
        let pb = p.seed.storage.data().unwrap();
        let slots = self
            .herd_markers(parent)
            .ok_or("Vimp vector route not initialized")?;
        let markers = slots
            .iter()
            .map(|slot| {
                slot.map(|marker| {
                    let m = self
                        .herd_marker(marker)
                        .ok_or("Vimp vector marker retired or foreign")?;
                    let f = self
                        .get(marker)
                        .and_then(|a| a.construction_fields.as_ref())
                        .ok_or("Vimp vector marker fields absent")?;
                    Ok(vector::Marker {
                        position: f.position,
                        radius: f32::from_bits(m.values[1]),
                    })
                })
                .transpose()
            })
            .collect::<Result<Vec<_>, String>>()?;
        let route = vector::Route {
            owner: parent,
            selected: word(pb, 0x2c),
            position: p.construction_fields.as_ref().unwrap().position,
            markers,
        };
        Ok((state, route))
    }
    fn publish_vimp_vector_actor(
        &mut self,
        id: Id,
        parent: Id,
        s: &vector::State<Id>,
    ) -> Result<(), String> {
        self.check_vimp_vector_owner(id, parent)?;
        if s.parent != Some(parent) || s.hint.is_some() {
            return Err("Vimp vector projection owner changed".into());
        }
        let a = self.residents.get_mut(&id.serial).unwrap();
        let v = a.vimp.as_mut().unwrap();
        v.mode = s.actor.mode;
        v.born.as_mut().unwrap().offsets = s.offsets;
        let f = a.construction_fields.as_mut().unwrap();
        f.position = s.actor.position;
        f.configuration[14] = s.actor.binding_flags;
        let b = a.seed.storage.data_mut().unwrap();
        for (at, value) in [
            (0x74, s.actor.response_kind),
            (0xb0, s.actor.response_active),
            (0xb8, s.actor.response_stage),
            (0xb4, s.selected_heading.to_bits()),
            (0x98, s.marker),
            (0x9c, s.visits),
            (0xa0, s.counted),
        ] {
            put(b, at, value);
        }
        for (i, v) in s.wander_target.into_iter().enumerate() {
            put(b, 0x40 + 4 * i, v.to_bits());
        }
        Ok(())
    }
    fn publish_vimp_vector(
        &mut self,
        id: Id,
        parent: Id,
        s: &vector::State<Id>,
        r: &vector::Route<Id>,
    ) -> Result<(), String> {
        if r.owner != parent {
            return Err("Vimp vector route projection owner changed".into());
        }
        self.publish_vimp_vector_actor(id, parent, s)?;
        let p = self.residents.get_mut(&parent.serial).unwrap();
        p.construction_fields.as_mut().unwrap().position = r.position;
        put(p.seed.storage.data_mut().unwrap(), 0x2c, r.selected);
        Ok(())
    }
    /// Isolated vector helper on an actual owned herd route. Caller/session,
    /// global RNG phase and primary timing remain explicit prerequisites.
    /// Nonzero hint state refuses until its typed owner/terrain/affine service
    /// exists. This does not replace the ordinary primary's response refusal.
    pub fn advance_vimp_vector_offline(
        &mut self,
        id: Id,
        request: Request,
        globals: &giants_formats::global_data::GlobalData,
        random: &mut crate::audio_parameters::Random,
        services: &mut impl Services,
    ) -> Result<(), String> {
        let Request {
            number,
            origin,
            fallback,
        } = request;
        let a = self.get(id).ok_or("Vimp vector owner absent or foreign")?;
        let v = a.vimp.ok_or("Vimp vector source construction absent")?;
        if v.wander_pending
            || a.world_tick_running
            || a.vimp_prepared_history.is_some()
            || a.vimp_frame_stopped
            || a.vimp_vector_running
            || a.vimp_active_running
            || a.vimp_vector_steps != number
            || number == u64::MAX
            || a.vimp_physical.as_ref().is_some_and(|p| {
                !p.view.physics_consumed || !p.view.animation_consumed || p.view.failure.is_some()
            })
        {
            return Err(
                "Vimp vector requires a fresh isolated invocation without pending phases".into(),
            );
        }
        let parent = v
            .born
            .ok_or("Vimp vector requires actual herd birth")?
            .parent;
        self.actor_current_affine(id)?;
        let (mut state, mut route) = self.vimp_vector_projection(id, parent)?;
        let parameters = vector::Parameters {
            turn_low: globals.scalar(0x978).map_err(|e| e.to_string())?,
            turn_high: globals.scalar(0x97c).map_err(|e| e.to_string())?,
        };
        self.residents
            .get_mut(&id.serial)
            .unwrap()
            .vimp_vector_running = true;
        let mut adapter = Adapter {
            session: self,
            services,
            owner: id,
            parent,
            valid: true,
        };
        let result = vector::advance(
            &mut state,
            Some(&mut route),
            origin,
            fallback,
            parameters,
            random,
            &mut adapter,
        );
        let publication = if adapter.valid {
            adapter.publish(&state, &route)
        } else {
            Ok(())
        };
        let result = result.and(publication);
        if let Some(a) = self.residents.get_mut(&id.serial) {
            a.vimp_vector_running = false;
            a.vimp_vector_steps += 1;
            if result.is_err() {
                a.vimp_frame_stopped = true;
            }
        }
        result
    }
}
