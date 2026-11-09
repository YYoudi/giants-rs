use super::*;
use crate::resident_states::affine_pose_words;
impl Session {
    pub(crate) fn advance_kabuto_control_force(
        &mut self,
        owner: Id,
        source: (&Resources, &giants_formats::global_data::GlobalData),
        number: u64,
    ) -> Result<crate::kabuto_control_force::View, String> {
        let (resources, globals) = source;
        let actor = self
            .get(owner)
            .ok_or("Kabuto control owner absent or foreign")?;
        let group = actor
            .kabuto_group
            .as_ref()
            .ok_or("Kabuto control group absent")?;
        if actor.class != 1
            || actor.seed.storage.state() != 1
            || group.cleaned_up
            || group.geometry_transition_failure.is_some()
            || group.motion_history != Some(number)
            || group.motion_prefix_completed != Some(number)
            || group.classification_completed != Some(number)
            || group.object_contacts_completed != Some(number)
            || group.model_motion_completed != Some(number)
            || group.animation_gravity_entered != Some(number)
            || group.animation_gravity_completed != Some(number)
            || group.control_force_completed.is_some()
            || !Arc::ptr_eq(&group.declared_class, &resources.specification)
            || self
                .specification(owner)
                .is_none_or(|s| !Arc::ptr_eq(s, &resources.specification))
        {
            return Err(
                "Kabuto control force requires its factory-owned unconsumed ANM/gravity phase"
                    .into(),
            );
        }
        let payload = actor
            .seed
            .storage
            .data()
            .ok_or("Kabuto control payload absent")?;
        if payload.len() != 844
            || group.physics.is_none_or(|p| {
                p.words() != crate::kabuto_physics::Physics::from_owned_payload(payload).words()
            })
        {
            return Err("Kabuto control physical cache differs from payload".into());
        }
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("Kabuto control fields absent")?;
        let affine = actor
            .affine_cache
            .as_ref()
            .filter(|c| c.pose == affine_pose_words(fields))
            .ok_or("Kabuto control affine absent or stale")?;
        let prepared = crate::kabuto_control_force::prepare(
            fields,
            payload,
            group.motion,
            group.contact,
            affine.value,
            globals,
        )?;
        let actor = self.residents.get_mut(&owner.serial).unwrap();
        actor
            .seed
            .storage
            .data_mut()
            .unwrap()
            .copy_from_slice(&prepared.payload);
        let physics = crate::kabuto_physics::Physics::from_owned_payload(&prepared.payload);
        let group = actor.kabuto_group.as_mut().unwrap();
        group.physics = Some(physics);
        group.control_force_completed = Some(number);
        // The remaining native primary, history and all initializer prerequisites
        // remain entered. This phase does not perform the next frame's integration.
        Ok(prepared.view)
    }
}
