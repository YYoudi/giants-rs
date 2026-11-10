use super::*;
use crate::{
    actor_attachment_bind::Input as Binding,
    actor_fields::Fields,
    actor_reference_detach as detach,
    kabuto_held_reference::{self as held, Request, Response, State},
};

fn word(p: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(p[at..at + 4].try_into().unwrap())
}

impl Session {
    /// Current typed identities and the actual scalar words in the owned
    /// payload. A missing owner/producer is an error, not an empty target list.
    pub fn kabuto_reference_state(&self, owner: Id) -> Result<State<Id>, String> {
        let a = self
            .get(owner)
            .ok_or("Kabuto reference owner absent or foreign")?;
        let g = a
            .kabuto_group
            .as_ref()
            .ok_or("Kabuto reference group absent")?;
        let p = a
            .seed
            .storage
            .data()
            .filter(|p| p.len() == 844)
            .ok_or("Kabuto reference payload absent")?;
        if a.class != 1
            || a.seed.storage.profile() != Profile::Debug
            || a.seed.storage.state() != 1
            || g.cleaned_up
            || a.construction_fields
                .as_ref()
                .is_none_or(|f| f.class != 1 || f.configuration[20] != 1)
            || [0xc, 0x10, 0x14].into_iter().any(|at| word(p, at) != 0)
        {
            return Err("Kabuto reference owner outside acquired typed storage profile".into());
        }
        Ok(State {
            reference: g.reference_links[0],
            held: [g.reference_links[1], g.reference_links[2]],
            reference_scale: f32::from_bits(word(p, 0x2c)),
            held_scales: [f32::from_bits(word(p, 0x30)), f32::from_bits(word(p, 0x34))],
        })
    }

    fn store_kabuto_references(&mut self, owner: Id, state: &State<Id>) -> Result<(), String> {
        self.kabuto_reference_state(owner)?;
        let a = self.residents.get_mut(&owner.serial).unwrap();
        a.kabuto_group.as_mut().unwrap().reference_links =
            [state.reference, state.held[0], state.held[1]];
        let p = a.seed.storage.data_mut().unwrap();
        for (at, value) in [
            (0x2c, state.reference_scale),
            (0x30, state.held_scales[0]),
            (0x34, state.held_scales[1]),
        ] {
            p[at..at + 4].copy_from_slice(&value.to_bits().to_le_bytes());
        }
        Ok(())
    }

    fn check_kabuto_reference_owner(
        &self,
        owner: Id,
        resources: &Resources,
        number: u64,
    ) -> Result<(), String> {
        self.kabuto_reference_state(owner)?;
        let a = self.get(owner).unwrap();
        let g = a.kabuto_group.as_ref().unwrap();
        if g.geometry_transition_failure.is_some()
            || g.motion_history != Some(number)
            || g.resource_phase_completed != Some(number)
            || !Arc::ptr_eq(&g.declared_class, &resources.specification)
            || self
                .specification(owner)
                .is_none_or(|s| !Arc::ptr_eq(s, &resources.specification))
            || a.construction_fields
                .as_ref()
                .is_none_or(|f| f.configuration[20] != 1)
        {
            return Err(
                "Kabuto reference operation requires its completed owned resource checkpoint"
                    .into(),
            );
        }
        Ok(())
    }

    pub(crate) fn publish_kabuto_reference(
        &mut self,
        owner: Id,
        resources: &Resources,
        number: u64,
        target: Option<Id>,
        scale: f32,
    ) -> Result<(), String> {
        self.check_kabuto_reference_owner(owner, resources, number)?;
        if target.is_some_and(|id| self.get(id).is_none()) {
            return Err("Kabuto reference candidate absent or foreign".into());
        }
        let mut state = self.kabuto_reference_state(owner)?;
        state.reference = target;
        state.reference_scale = scale;
        self.store_kabuto_references(owner, &state)
    }

    pub(crate) fn advance_kabuto_held_references(
        &mut self,
        owner: Id,
        resources: &Resources,
        number: u64,
        input: held::Input,
        world: &mut impl held::World,
    ) -> Result<held::Next, String> {
        self.check_kabuto_reference_owner(owner, resources, number)?;
        let g = self.get(owner).unwrap().kabuto_group.as_ref().unwrap();
        if g.held_reference_running || g.held_reference_entered == Some(number) {
            return Err("Kabuto reference handoff reentry or replay".into());
        }
        let mut state = self.kabuto_reference_state(owner)?;
        let g = self
            .residents
            .get_mut(&owner.serial)
            .unwrap()
            .kabuto_group
            .as_mut()
            .unwrap();
        g.held_reference_running = true;
        g.held_reference_entered = Some(number);
        let mut adapter = Adapter {
            session: self,
            world,
            owner,
            resources,
            number,
            valid: true,
        };
        let mut result = held::advance(&mut state, owner, input, &mut adapter);
        if adapter.valid
            && let Err(error) = adapter.publish(&state)
        {
            adapter.valid = false;
            if result.is_ok() {
                result = Err(error);
            }
        }
        if !adapter.valid && result.is_ok() {
            result = Err("Kabuto reference owner invalidated by service".into());
        }
        if let Some(g) = adapter
            .session
            .residents
            .get_mut(&owner.serial)
            .and_then(|a| a.kabuto_group.as_mut())
            .filter(|g| {
                g.held_reference_entered == Some(number)
                    && Arc::ptr_eq(&g.declared_class, &resources.specification)
            })
        {
            g.held_reference_running = false;
            match &result {
                Ok(_) => g.held_reference_completed = Some(number),
                Err(error) => g.geometry_transition_failure = Some(error.clone()),
            }
        }
        result
    }
}

struct Adapter<'a, W> {
    session: &'a mut Session,
    world: &'a mut W,
    owner: Id,
    resources: &'a Resources,
    number: u64,
    valid: bool,
}
impl<W: held::World> Adapter<'_, W> {
    fn check(&self) -> Result<(), String> {
        self.session
            .check_kabuto_reference_owner(self.owner, self.resources, self.number)?;
        let g = self
            .session
            .get(self.owner)
            .unwrap()
            .kabuto_group
            .as_ref()
            .unwrap();
        if !g.held_reference_running || g.held_reference_entered != Some(self.number) {
            return Err("Kabuto reference handoff owner changed during service".into());
        }
        Ok(())
    }
    fn publish(&mut self, state: &State<Id>) -> Result<(), String> {
        self.check()?;
        self.session.store_kabuto_references(self.owner, state)
    }
    fn send(&mut self, state: &mut State<Id>, request: Request) -> Result<Response, String> {
        self.publish(state)?;
        let result = self.world.execute(self.session, self.owner, request);
        // Pull actual stores even on failure; never restore a stale snapshot
        // after the callback has removed or changed the owning actor.
        match self
            .check()
            .and_then(|()| self.session.kabuto_reference_state(self.owner))
        {
            Ok(current) => *state = current,
            Err(error) => {
                self.valid = false;
                return result.and(Err(error));
            }
        }
        result
    }
    fn complete(&mut self, state: &mut State<Id>, request: Request) -> Result<(), String> {
        match self.send(state, request)? {
            Response::Complete => Ok(()),
            Response::Actor(_) => Err("unexpected actor response to reference service".into()),
        }
    }
}
impl<W: held::World> detach::Services<Id, State<Id>> for Adapter<'_, W> {
    fn fields(&self, actor: Id) -> Result<&Fields, String> {
        self.session
            .get(actor)
            .and_then(|a| a.construction_fields.as_ref())
            .ok_or("reference actor fields absent or foreign".into())
    }
    fn fields_mut(&mut self, actor: Id) -> Result<&mut Fields, String> {
        self.session
            .get(actor)
            .ok_or("reference actor absent or foreign")?;
        self.session
            .residents
            .get_mut(&actor.serial)
            .unwrap()
            .construction_fields
            .as_mut()
            .ok_or("reference actor fields absent".into())
    }
    fn state(&self, actor: Id) -> Result<u32, String> {
        Ok(self.fields(actor)?.configuration[20])
    }
    fn parent_reference(&self, actor: Id) -> Result<i32, String> {
        self.session
            .get(actor)
            .and_then(|a| a.attachment_words())
            .map(|a| a.parent_reference as i32)
            .ok_or("reference parent word has no acquired producer".into())
    }
    fn clear_parent(&mut self, actor: Id) -> Result<(), String> {
        self.session
            .clear_actor_parent(actor)
            .map_err(str::to_owned)
    }
    fn set_state89_parameter(&mut self, actor: Id, value: f32) -> Result<(), String> {
        if self.state(actor)? != 89 {
            return Ok(());
        }
        self.session
            .get(actor)
            .filter(|a| a.seed.storage.state() == 89)
            .ok_or("reference state89 payload tag differs".to_owned())?;
        let p = self
            .session
            .residents
            .get_mut(&actor.serial)
            .unwrap()
            .seed
            .storage
            .data_mut()
            .filter(|p| p.len() == 4)
            .ok_or("reference state89 storage absent".to_owned())?;
        p.copy_from_slice(&value.to_bits().to_le_bytes());
        Ok(())
    }
    fn reset_state18(&mut self, state: &mut State<Id>, actor: Id) -> Result<(), String> {
        self.complete(state, Request::ResetState18(actor))
    }
    fn state3_scale(&self, actor: Id) -> Result<f32, String> {
        let p = self
            .session
            .get(actor)
            .filter(|a| a.seed.storage.state() == 3)
            .and_then(|a| a.seed.storage.data())
            .filter(|p| p.len() == 4096)
            .ok_or("reference state3 stored scale absent")?;
        Ok(f32::from_bits(word(p, 0xfac)))
    }
}
impl<W: held::World> held::Services<Id> for Adapter<'_, W> {
    fn session_mode(&self) -> Result<u32, String> {
        self.world.session_mode(self.session)
    }
    fn notification_context(
        &self,
        source: Id,
    ) -> Result<crate::kabuto_geometry_mode::NotificationContext<'_>, String> {
        self.world.network(self.session, source)
    }
    fn physical_flags(&self, actor: Id) -> Result<u32, String> {
        Ok(detach::Services::fields(self, actor)?.configuration[14])
    }
    fn type_id(&self, actor: Id) -> Result<u32, String> {
        Ok(detach::Services::fields(self, actor)?.class as u32)
    }
    fn owner_word(&self, actor: Id) -> Result<u32, String> {
        self.session
            .get(actor)
            .ok_or("reference owner-word actor absent or foreign")?;
        self.world.owner_word(self.session, actor)
    }
    fn set_owner_word(
        &mut self,
        state: &mut State<Id>,
        actor: Id,
        value: u32,
    ) -> Result<(), String> {
        self.session
            .get(actor)
            .ok_or("reference owner-word actor absent")?;
        self.complete(state, Request::SetOwner { actor, word: value })
    }
    fn construct(
        &mut self,
        state: &mut State<Id>,
        _: Id,
        parameter: u32,
        type_id: u32,
    ) -> Result<Id, String> {
        match self.send(state, Request::Construct { parameter, type_id })? {
            Response::Actor(id) if self.session.get(id).is_some() => Ok(id),
            _ => Err("reference construction returned no Session-owned actor".into()),
        }
    }
    fn notify_created(
        &mut self,
        state: &mut State<Id>,
        _: Id,
        notice: held::CreatedNotice,
    ) -> Result<(), String> {
        self.complete(state, Request::Created(notice))
    }
    fn notify_release(
        &mut self,
        state: &mut State<Id>,
        _: Id,
        notice: crate::kabuto_reference_release::Notice<Id>,
    ) -> Result<(), String> {
        self.complete(state, Request::Released(notice))
    }
    fn bind(
        &mut self,
        state: &mut State<Id>,
        _: Id,
        child: Id,
        input: Binding,
    ) -> Result<(), String> {
        self.complete(state, Request::Bind { child, input })
    }
    fn action(
        &mut self,
        state: &mut State<Id>,
        request: crate::kabuto_contact::Request<Id>,
    ) -> Result<(), String> {
        self.complete(state, Request::Action(request))
    }
}
