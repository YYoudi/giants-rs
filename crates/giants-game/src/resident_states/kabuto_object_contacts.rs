use super::*;
use crate::kabuto_object_contacts::{self as contacts, Owner};

impl Session {
    fn kabuto_contacts_owner(
        &self,
        owner: Id,
        scene_key: u32,
        owner_word: u32,
        resources: &Resources,
        number: u64,
        entered: bool,
    ) -> Result<Owner<u32>, String> {
        let actor = self
            .get(owner)
            .ok_or("Kabuto contact owner absent or foreign")?;
        let group = actor
            .kabuto_group
            .as_ref()
            .ok_or("Kabuto contact group absent")?;
        if actor.class != 1
            || actor.seed.storage.state() != 1
            || group.cleaned_up
            || group.geometry_transition_failure.is_some()
            || group.motion_history != Some(number)
            || group.classification_completed != Some(number)
            || group.object_contacts_completed.is_some()
            || group.object_contacts_entered != entered.then_some(number)
            || scene_key == 0
            || !Arc::ptr_eq(&group.declared_class, &resources.specification)
            || self
                .specification(owner)
                .is_none_or(|s| !Arc::ptr_eq(s, &resources.specification))
        {
            return Err(
                "Kabuto contacts require their factory-owned unconsumed classification".into(),
            );
        }
        let payload = actor
            .seed
            .storage
            .data()
            .ok_or("Kabuto contact payload absent")?;
        if payload.len() != 844
            || group.physics.is_none_or(|p| {
                p.words() != crate::kabuto_physics::Physics::from_owned_payload(payload).words()
            })
        {
            return Err("Kabuto contact physical cache differs from payload".into());
        }
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("Kabuto contact fields absent")?;
        // This acquired consumer reads the preceding raw reference storage,
        // including constructor-cleared slots. It does not certify a current
        // ANM sample or turn those zeros into a current-pose query result.
        let storage = self.reference_cache_storage(owner)?;
        let references = storage
            .matrices
            .get(8)
            .zip(storage.matrices.get(9))
            .map(|(a, b)| [a, b].map(|m| [m.values[3], m.values[7], m.values[11]]));
        Ok(Owner {
            identity: scene_key,
            position: fields.position,
            scale: fields.scale,
            state: 1,
            factor: f32::from_le_bytes(payload[0x1d0..0x1d4].try_into().unwrap()),
            owner_word,
            affine: actor.affine_cache.as_ref().map(|c| c.value),
            references,
        })
    }

    pub(crate) fn advance_kabuto_object_contacts(
        &mut self,
        owner: Id,
        source: (&Resources, &giants_formats::global_data::GlobalData),
        number: u64,
        scene: contacts::Scene<'_>,
        mut execute: impl FnMut(
            &mut Session,
            crate::kabuto_contact::Request<u32>,
            &mut [contacts::Peer<u32>],
        ) -> Result<(), &'static str>,
        observe: impl FnMut(contacts::Event<u32>),
    ) -> Result<bool, String> {
        let (resources, globals) = source;
        let mut state = self.kabuto_contacts_owner(
            owner,
            scene.owner,
            scene.context.owner_word,
            resources,
            number,
            false,
        )?;
        if scene.peers.iter().any(|p| p.contact.identity == 0) {
            return Err("Kabuto contact scene uses a null identity".into());
        }
        self.residents
            .get_mut(&owner.serial)
            .unwrap()
            .kabuto_group
            .as_mut()
            .unwrap()
            .object_contacts_entered = Some(number);
        let amount = globals
            .words
            .get(0x119c / 4)
            .copied()
            .map(f32::from_bits)
            .ok_or("additional contact global scalar absent");
        let mut owner_failure = None;
        let result = contacts::traverse(
            &mut state,
            scene.peers,
            contacts::Frame {
                disabled: scene.disabled,
                base_amount: amount,
                session_mode: scene.context.session_mode,
                peer_ids: scene.context.peer_ids,
                local_peer: scene.context.local_peer,
            },
            |request, state, peers| {
                // The real Session remains authoritative at the action boundary.
                // Retain provider mutations and reread its fields before continuing.
                let result = execute(self, request, peers);
                if let Some(actor) = self.residents.get_mut(&owner.serial)
                    && actor.seed.storage.state() == 1
                    && let Some(payload) = actor.seed.storage.data()
                    && payload.len() == 844
                    && let Some(group) = actor.kabuto_group.as_mut()
                    && group.physics.is_some()
                {
                    group.physics =
                        Some(crate::kabuto_physics::Physics::from_owned_payload(payload));
                }
                match self.kabuto_contacts_owner(
                    owner,
                    scene.owner,
                    scene.context.owner_word,
                    resources,
                    number,
                    true,
                ) {
                    Ok(current) => *state = current,
                    Err(reason) => {
                        result?;
                        owner_failure = Some(reason);
                        return Err("Kabuto contact owner changed during action");
                    }
                }
                result
            },
            observe,
        );
        match result {
            Ok(value) => {
                let actor = self
                    .residents
                    .get_mut(&owner.serial)
                    .ok_or("Kabuto contact owner disappeared")?;
                actor
                    .kabuto_group
                    .as_mut()
                    .ok_or("Kabuto contact group disappeared")?
                    .object_contacts_completed = Some(number);
                // Neither the history nor any initializer prerequisite is cleared.
                Ok(value)
            }
            Err(reason) => {
                let reason = owner_failure.unwrap_or_else(|| reason.into());
                if let Some(actor) = self.residents.get_mut(&owner.serial)
                    && let Some(group) = actor.kabuto_group.as_mut()
                {
                    group.geometry_transition_failure.get_or_insert_with(|| {
                        format!("Kabuto additional contacts failed: {reason}")
                    });
                }
                Err(reason)
            }
        }
    }
}
