//! Canonical retained smoke construction and explicit stop marking only.
//! Object callbacks, descendants and eventual dead-object destruction are separate.
use super::*;
use crate::{attached_effect_context::Id as ContextId, effect_actor::Placement};

#[derive(Debug)]
pub(super) struct Metadata {
    pub origin: ContextId,
    pub parent: Id,
    pub reference: u32,
    pub tag: u32,
    pub render_flags: u32,
    pub matrix: crate::procedural_aim::Matrix,
    pub emission: crate::mecc_smoke_effect::Emission,
    pub stopped: bool,
}
impl Session {
    pub(crate) fn adopt_smoke(
        &mut self,
        staged: crate::mecc_smoke_effect::Staged<Id>,
    ) -> Result<Id, String> {
        let Placement::Attached {
            recipient,
            reference,
            tag,
            ..
        } = staged.initial.placement
        else {
            return Err("retained smoke requires actual attached placement".into());
        };
        if self
            .get(recipient)
            .and_then(|a| a.construction_fields.as_ref())
            .is_none()
        {
            return Err("retained smoke parent absent, foreign or unconstructed".into());
        }
        let initial = staged.initial;
        let template = staged.template;
        let mut pending = Self::new(self.ticket);
        pending.next = self.next;
        let id = pending.register_with_mask(
            initial.fields.class,
            &initial.seed,
            &template.bindings,
            DirectFlags {
                active: initial.fields.object_flags,
                personality: initial.fields.configuration[14],
            },
            0,
        )?;
        pending.bind_specification(id, template.specification)?;
        pending.bind_state_resources(id, template.states)?;
        let channels = initial.fields.channels;
        pending.bind_actor_fields(id, initial.fields)?;
        let actor = pending.residents.get_mut(&id.serial).unwrap();
        actor.seed = initial.seed;
        actor.channels = Some(channels);
        actor.presentation_parent = Some(recipient);
        actor.acquired_construction = true;
        actor.smoke_effect = Some(Metadata {
            origin: initial.origin,
            parent: recipient,
            reference,
            tag,
            render_flags: initial.render_flags,
            matrix: initial.matrix,
            emission: staged.emission,
            stopped: false,
        });
        let id = self.publish_staged(pending, id)?;
        // The VM adds this head registration after the constructor's common list.
        self.links
            .insert(14, id.serial, crate::object_links::Position::Head);
        Ok(id)
    }
    pub fn smoke_effect_origin(&self, id: Id) -> Option<ContextId> {
        Some(self.get(id)?.smoke_effect.as_ref()?.origin)
    }
    pub fn smoke_effect_emission(&self, id: Id) -> Option<&crate::mecc_smoke_effect::Emission> {
        Some(&self.get(id)?.smoke_effect.as_ref()?.emission)
    }
    pub fn smoke_effect_stopped(&self, id: Id) -> Option<bool> {
        Some(self.get(id)?.smoke_effect.as_ref()?.stopped)
    }
    pub fn smoke_effect_attachment(&self, id: Id) -> Option<(Id, u32, u32)> {
        let metadata = self.get(id)?.smoke_effect.as_ref()?;
        Some((metadata.parent, metadata.reference, metadata.tag))
    }
    /// Constructor-local cache only; no current parent reference was resolved.
    pub fn smoke_effect_initial_matrix(&self, id: Id) -> Option<crate::procedural_aim::Matrix> {
        Some(self.get(id)?.smoke_effect.as_ref()?.matrix)
    }
    pub(crate) fn validate_smoke(
        &self,
        id: Id,
        origin: ContextId,
        parent: Id,
    ) -> Result<(), String> {
        let actor = self
            .get(id)
            .ok_or("retained smoke actor absent or foreign")?;
        let metadata = actor
            .smoke_effect
            .as_ref()
            .ok_or("retained smoke ownership absent")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("retained smoke fields absent")?;
        let payload = actor
            .seed
            .storage
            .data()
            .ok_or("retained smoke payload absent")?;
        if metadata.origin != origin
            || metadata.parent != parent
            || metadata.stopped
            || actor.seed.storage.state() != 78
            || payload.len() != 268
            || actor.presentation_parent != Some(parent)
            || actor.attachment.is_some()
            || fields.control_flags & 1 != 0
            || metadata.render_flags & 6 != 0
            || fields.action != 0
            || payload[0x108..0x10c] != [0; 4]
            || !self.list(14).contains(&id)
        {
            return Err("retained smoke stop outside acquired unadvanced ownership profile".into());
        }
        Ok(())
    }
    pub(crate) fn stop_smoke(
        &mut self,
        id: Id,
        origin: ContextId,
        parent: Id,
    ) -> Result<(), String> {
        self.validate_smoke(id, origin, parent)?;
        let actor = self.residents.get_mut(&id.serial).unwrap();
        // The acquired bounded stop marks dead. It neither frees this actor nor
        // removes its registrations; the later dead-object pass owns those.
        actor.construction_fields.as_mut().unwrap().object_flags |= 0x8000;
        actor.smoke_effect.as_mut().unwrap().stopped = true;
        Ok(())
    }
}
