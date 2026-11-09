use super::*;

impl Session {
    pub(crate) fn advance_kabuto_model_motion(
        &mut self,
        owner: Id,
        source: (&Resources, &giants_formats::global_data::GlobalData),
        number: u64,
        global_seconds: f32,
        areas: &[crate::object_time::SlowArea],
    ) -> Result<crate::mecc_model_motion::Output, String> {
        let (resources, globals) = source;
        let actor = self
            .get(owner)
            .ok_or("Kabuto model owner absent or foreign")?;
        let group = actor
            .kabuto_group
            .as_ref()
            .ok_or("Kabuto model group absent")?;
        if actor.class != 1
            || actor.seed.storage.state() != 1
            || group.cleaned_up
            || group.geometry_transition_failure.is_some()
            || group.motion_history != Some(number)
            || group.motion_prefix_completed != Some(number)
            || group.classification_completed != Some(number)
            || group.object_contacts_entered != Some(number)
            || group.object_contacts_completed != Some(number)
            || group.model_motion_completed.is_some()
            || !Arc::ptr_eq(&group.declared_class, &resources.specification)
            || self
                .specification(owner)
                .is_none_or(|s| !Arc::ptr_eq(s, &resources.specification))
        {
            return Err(
                "Kabuto model rates require their factory-owned unconsumed contact phase".into(),
            );
        }
        let payload = actor
            .seed
            .storage
            .data()
            .ok_or("Kabuto model payload absent")?;
        if payload.len() != 844
            || group.physics.is_none_or(|p| {
                p.words() != crate::kabuto_physics::Physics::from_owned_payload(payload).words()
            })
        {
            return Err("Kabuto model physical cache differs from payload".into());
        }
        if !global_seconds.is_finite()
            || global_seconds < 0.
            || areas.iter().any(|a| {
                !a.center
                    .into_iter()
                    .chain([a.radius_squared, a.factor])
                    .all(f32::is_finite)
                    || a.radius_squared < 0.
                    || a.factor < 0.
            })
        {
            return Err("Kabuto model clock outside finite nonnegative profile".into());
        }
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("Kabuto model fields absent")?;
        let cache = actor
            .affine_cache
            .as_ref()
            .ok_or("Kabuto model affine absent")?;
        let seconds = crate::object_time::seconds(
            global_seconds,
            Some(crate::object_time::Sample {
                owner: fields.dispatch_context,
                position: fields.position,
            }),
            None,
            areas,
        );
        let out = crate::mecc_model_motion::derive(
            crate::mecc_model_motion::Input {
                current: fields.position,
                previous: fields.current,
                velocity: group.motion.velocity,
                matrix: cache.value.matrix,
                scale: fields.scale,
                seconds,
                controls: [
                    group.motion.commands[2] as i8,
                    group.motion.commands[3] as i8,
                ],
                minimums: [
                    globals.scalar(0x6e0).map_err(|e| e.to_string())?,
                    globals.scalar(0x6e4).map_err(|e| e.to_string())?,
                ],
            },
            true,
            crate::arithmetic::Precision::Double53,
        )?;
        // Pure preparation above: an invalid clock/resource/cache has no effects.
        let actor = self.residents.get_mut(&owner.serial).unwrap();
        let payload = actor.seed.storage.data_mut().unwrap();
        let at = 0xe4 + 9 * 4;
        payload[at..at + 4].copy_from_slice(&out.world_speed.to_bits().to_le_bytes());
        let physics = crate::kabuto_physics::Physics::from_owned_payload(payload);
        let group = actor.kabuto_group.as_mut().unwrap();
        group.physics = Some(physics);
        group.motion.model[..3].copy_from_slice(&out.model);
        group.model_motion_completed = Some(number);
        // No history completion, initializer acknowledgment or ANM sample here.
        Ok(out)
    }
}
