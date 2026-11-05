//! Delphi-specific owned mutations; ordinary Mecc admission stays unchanged.
use super::*;
use crate::delphi_special::{CopySource, Initialized, State};
impl Session {
    pub fn delphi_special(&self, id: Id) -> Option<&Initialized> {
        match self.get(id)?.delphi_special.as_ref()? {
            State::Complete(value) => Some(value),
            _ => None,
        }
    }
    pub fn delphi_special_failure(&self, id: Id) -> Option<&str> {
        match self.get(id)?.delphi_special.as_ref()? {
            State::Failed(value) => Some(value),
            _ => None,
        }
    }
    pub fn delphi_copy_source(&self, id: Id) -> Option<CopySource> {
        self.get(id)?.delphi_copy_source
    }
    /// Acquired header parameter; its later rendering semantics are unverified.
    pub fn delphi_copy_parameter(&self, id: Id) -> Option<f32> {
        self.get(id)?.delphi_copy_parameter
    }
    pub(crate) fn construct_delphi_cold(
        &mut self,
        input: Construction<'_>,
        mask: u32,
    ) -> Result<Id, String> {
        if !matches!(
            (input.class, input.seed.storage.state(), mask),
            (3, 3, 0x80000000) | (311, 62, 4) | (669, 0, 0x20)
        ) || input.seed.storage.profile() != Profile::Debug
            || !input.seed.diagnostics.is_empty()
            || input.fields.object_flags != input.flags.active
            || input.fields.configuration[14] != input.flags.personality
        {
            return Err(
                "Delphi construction requires the acquired class/state/mask template".into(),
            );
        }
        let mut staged = Self::new(self.ticket);
        staged.next = self.next;
        let id = staged.register_with_mask(
            input.class,
            input.seed,
            input.bindings,
            input.flags,
            mask,
        )?;
        staged.bind_specification(id, input.specification)?;
        staged.bind_state_resources(id, input.state_resources)?;
        let channels = input.fields.channels;
        staged.bind_actor_fields(id, input.fields)?;
        let actor = staged.residents.get_mut(&id.serial).unwrap();
        actor.channels = Some(channels);
        actor.acquired_construction = true;
        self.publish_staged(staged, id)
    }
    pub(crate) fn begin_delphi_special(
        &mut self,
        id: Id,
        cold: &Seed,
        template: &crate::actor_fields::Fields,
    ) -> Result<(), String> {
        let actor = self.get(id).ok_or("Delphi owner absent or foreign")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("Delphi fields absent")?;
        if actor.class != 3
            || !actor.acquired_construction
            || actor.seed.storage.profile() != Profile::Debug
            || actor.seed.storage.state() != 3
            || actor.seed.storage.data() != cold.storage.data()
            || cold.storage.data().is_none_or(|p| p.len() != 4096)
            || actor.delphi_special.is_some()
            || actor.channels != Some(template.channels)
            || fields.channels != template.channels
            || fields.object_flags != template.object_flags
            || fields.control_flags != template.control_flags
            || fields.action != template.action
            || fields.configuration != template.configuration
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || fields.dispatch_context != 0
            || fields.object_flags & 0x8000 != 0
            || !fields
                .position
                .into_iter()
                .chain(fields.angles)
                .chain([fields.scale])
                .all(f32::is_finite)
        {
            return Err(
                "Delphi special phase requires a fresh acquired normal class3 owner".into(),
            );
        }
        let actor = self.residents.get_mut(&id.serial).unwrap();
        actor.delphi_special = Some(State::Initializing);
        actor.seed.storage.data_mut().unwrap()[0x68..0x6c].copy_from_slice(&u32::MAX.to_le_bytes());
        Ok(())
    }
    pub(crate) fn bind_delphi_copy(
        &mut self,
        copy: Id,
        owner: Id,
        index: u32,
        parameter: f32,
    ) -> Result<(), String> {
        if !matches!(
            self.get(owner).and_then(|a| a.delphi_special.as_ref()),
            Some(State::Initializing)
        ) {
            return Err("Delphi copy source is not initializing".into());
        }
        let source = self
            .get(owner)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap();
        let position = source.position;
        let scale = source.scale;
        let context = source.dispatch_context;
        let actor = self.get(copy).ok_or("Delphi copy absent")?;
        if actor.class != 311
            || actor.seed.storage.state() != 62
            || actor.constructor_mask != 4
            || actor.delphi_copy_source.is_some()
            || !parameter.is_finite()
        {
            return Err("Delphi copy outside fresh source profile".into());
        }
        let actor = self.residents.get_mut(&copy.serial).unwrap();
        let fields = actor.construction_fields.as_mut().unwrap();
        fields.position = position;
        fields.current = position;
        fields.scale = scale;
        fields.dispatch_context = context;
        fields.configuration[14] |= 0xe00;
        fields.object_flags |= 0x40400010;
        actor.delphi_copy_source = Some(CopySource { owner, index });
        actor.delphi_copy_parameter = Some(parameter);
        Ok(())
    }
    pub(crate) fn bind_delphi_held(
        &mut self,
        child: Id,
        parent: Id,
        reference: i32,
    ) -> Result<(), String> {
        let actor = self.get(parent).ok_or("Delphi held parent absent")?;
        let allowed = matches!(actor.delphi_special, Some(State::Initializing))
            || actor.delphi_copy_source.is_some_and(|c| {
                matches!(
                    self.get(c.owner).and_then(|a| a.delphi_special.as_ref()),
                    Some(State::Initializing)
                )
            });
        if !allowed || reference != 1 || actor.wield_failure.is_some() {
            return Err("Delphi held requires its initializing owner or copy".into());
        }
        let source = actor
            .construction_fields
            .as_ref()
            .ok_or("Delphi held parent fields absent")?;
        let position = source.position;
        let yaw = source.angles[0];
        let actor = self.get(child).ok_or("Delphi held child absent")?;
        if actor.class != 669
            || actor.constructor_mask != 0x20
            || actor.wield_binding.is_some()
            || actor.attachment.is_some()
        {
            return Err("Delphi held requires its fresh acquired constructor".into());
        }
        let actor = self.residents.get_mut(&child.serial).unwrap();
        let fields = actor.construction_fields.as_mut().unwrap();
        fields.position = position;
        fields.angles[0] = yaw;
        fields.object_flags |= 0x20400000;
        fields.dispatch_context = 0;
        actor.attachment = Some(Attachment {
            parent,
            child_reference: 0,
            parent_reference: 1,
            behavior: 0,
        });
        actor.presentation_parent = Some(parent);
        actor.wield_binding = Some(crate::mecc_wield::Binding {
            parent,
            reference: 1,
            context: 0,
        });
        Ok(())
    }
    pub(crate) fn finish_delphi_copy_held(&mut self, id: Id, parameter: f32) -> Result<(), String> {
        let actor = self
            .residents
            .get_mut(&id.serial)
            .ok_or("Delphi copy held absent")?;
        let fields = actor.construction_fields.as_mut().unwrap();
        fields.object_flags |= 0x40000010;
        fields.configuration[14] |= 0xe00;
        actor.delphi_copy_parameter = Some(parameter);
        Ok(())
    }
    pub(crate) fn initialize_delphi_equipment(
        &mut self,
        id: Id,
        charges: i32,
    ) -> Result<(), String> {
        let actor = self
            .residents
            .get_mut(&id.serial)
            .ok_or("Delphi equipment owner absent")?;
        if actor.class != 3 || !matches!(actor.delphi_special, Some(State::Initializing)) {
            return Err("Delphi equipment requires its initializing cold owner".into());
        }
        let body = actor
            .seed
            .storage
            .data_mut()
            .ok_or("Delphi payload absent")?;
        for at in [
            0xd3c, 0xd48, 0xd54, 0xd60, 0xd6c, 0xd78, 0xd80, 0xd88, 0xd90, 0xd98, 0xda0, 0xdb0,
            0xdb4,
        ] {
            body[at..at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        }
        for at in [0xd7c, 0xd84, 0xd8c, 0xd94, 0xd9c, 0xda4] {
            body[at..at + 4].fill(0);
        }
        body[0xd78..0xd7c].copy_from_slice(&44u32.to_le_bytes());
        body[0xd7c..0xd80].copy_from_slice(&charges.to_le_bytes());
        Ok(())
    }
    pub(crate) fn finish_delphi_special(&mut self, id: Id) -> Result<(), String> {
        let actor = self
            .residents
            .get_mut(&id.serial)
            .ok_or("Delphi owner absent at completion")?;
        actor.seed.storage.data_mut().unwrap()[0xdb4..0xdb8].fill(0);
        Ok(())
    }
    pub(crate) fn end_delphi_special(&mut self, id: Id, result: &Result<Initialized, String>) {
        if let Some(actor) = self.residents.get_mut(&id.serial) {
            actor.delphi_special = Some(match result {
                Ok(value) => State::Complete(value.clone()),
                Err(error) => State::Failed(error.clone()),
            });
        }
    }
}
