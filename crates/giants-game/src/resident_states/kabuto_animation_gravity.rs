use super::*;

impl Session {
    pub(crate) fn advance_kabuto_animation_gravity(
        &mut self,
        owner: Id,
        resources: &Resources,
        number: u64,
        frame: crate::kabuto_animation_gravity::Frame<'_>,
        services: &mut crate::actor_animation_phase::Services<'_>,
    ) -> Result<crate::kabuto_animation_gravity::View, String> {
        let actor = self
            .get(owner)
            .ok_or("Kabuto ANM owner absent or foreign")?;
        let group = actor
            .kabuto_group
            .as_ref()
            .ok_or("Kabuto ANM group absent")?;
        if actor.class != 1
            || actor.seed.storage.state() != 1
            || group.cleaned_up
            || group.geometry_transition_failure.is_some()
            || group.motion_history != Some(number)
            || group.motion_prefix_completed != Some(number)
            || group.classification_completed != Some(number)
            || group.object_contacts_completed != Some(number)
            || group.model_motion_completed != Some(number)
            || group.animation_gravity_entered.is_some()
            || group.animation_gravity_completed.is_some()
            || !Arc::ptr_eq(&group.declared_class, &resources.specification)
            || self
                .specification(owner)
                .is_none_or(|s| !Arc::ptr_eq(s, &resources.specification))
        {
            return Err(
                "Kabuto ANM/gravity requires its factory-owned unconsumed model phase".into(),
            );
        }
        let payload = actor
            .seed
            .storage
            .data()
            .ok_or("Kabuto ANM payload absent")?;
        if payload.len() != 844
            || group.physics.is_none_or(|p| {
                p.words() != crate::kabuto_physics::Physics::from_owned_payload(payload).words()
            })
        {
            return Err("Kabuto ANM physical cache differs from payload".into());
        }
        let mut fields = actor
            .construction_fields
            .as_ref()
            .ok_or("Kabuto ANM fields absent")?
            .clone();
        if actor.channels != Some(fields.channels) {
            return Err("Kabuto ANM channel mirror differs".into());
        }
        let areas: Vec<_> = frame
            .areas
            .iter()
            .map(|a| crate::object_time_precision::Area {
                radius_squared: a.radius_squared,
                multiplier: a.factor,
                owner_id: a.source_owner,
                center: [a.center[0], a.center[1]],
            })
            .collect();
        let seconds = crate::object_time_precision::seconds(
            frame.global_seconds,
            Some(crate::object_time_precision::Actor {
                owner_id: fields.dispatch_context,
                position: fields.position[..2].try_into().unwrap(),
            }),
            None,
            &areas,
            crate::arithmetic::Precision::Double53,
        )
        .map_err(str::to_owned)? as f32;
        if !seconds.is_finite() || seconds < 0. {
            return Err("Kabuto ANM clock outside finite nonnegative profile".into());
        }
        let bank = self.kabuto_events.clone().unwrap_or_else(|| {
            crate::mecc_animation_events::Bank::new(resources.specification.clone())
        });
        if !bank.belongs_to(&resources.specification) {
            return Err("Kabuto ANM event bank specification differs".into());
        }
        let mut pipeline = crate::actor_animation_phase::Pipeline {
            specification: resources.specification.clone(),
            references: self
                .specifications
                .get(&owner.serial)
                .ok_or("Kabuto ANM references absent")?
                .0
                .clone(),
            bank,
            contexts: services.contexts.staged_copy(),
            random: *services.random,
            channels: fields.channels,
            owner,
            stamp: services.stamp,
            last_stamp: actor.mecc_animation_stamp,
            blending: services.blending,
            materials: frame.materials,
        };
        let state = group.motion;
        let contact = group.contact;
        let p = group.physics.unwrap().words();
        let pf = |i: usize| f32::from_bits(p[i]);
        let affine = actor.affine_cache.as_ref().map(|c| c.value);
        self.residents
            .get_mut(&owner.serial)
            .unwrap()
            .kabuto_group
            .as_mut()
            .unwrap()
            .animation_gravity_entered = Some(number);
        let result = (|| {
            let pass = pipeline.advance_sample(
                &fields,
                crate::actor_animation_phase::Sample {
                    model: state.model,
                    affine,
                    support: contact.support,
                    support_part: contact.support_part,
                },
                seconds,
            )?;
            fields.channels = pass.channels;
            let gravity = crate::mecc_gravity::apply(
                crate::mecc_gravity::Input {
                    force: [pf(6), pf(7), pf(8)],
                    velocity: state.velocity,
                    normal: contact.plane[..3].try_into().unwrap(),
                    gravity_mass: pf(1),
                    mass: pf(2),
                    friction_limit: pf(12),
                    damping_xy: pf(13),
                    damping_z: pf(14),
                    force_time_limit: pf(11),
                    object_flags: fields.object_flags,
                    physics_flags: p[0],
                    project_contact: state.commands[2..4] == [0, 0],
                },
                crate::arithmetic::Precision::Double53,
            )?;
            Ok::<_, String>((
                crate::kabuto_animation_gravity::View {
                    animation_processed: pass.processed,
                    event_visits: pass.visits,
                    stamp: pass.stamp,
                },
                gravity,
            ))
        })();
        // Keep the real completed ANM/channel/epoch/reference/event/context/RNG
        // effects even if a required event service or later gravity refuses.
        fields.channels = pipeline.channels;
        self.specifications.get_mut(&owner.serial).unwrap().0 = pipeline.references;
        self.kabuto_events = Some(pipeline.bank);
        *services.contexts = pipeline.contexts;
        *services.random = pipeline.random;
        let actor = self.residents.get_mut(&owner.serial).unwrap();
        actor.channels = Some(fields.channels);
        actor.construction_fields = Some(fields);
        actor.mecc_animation_stamp = pipeline.last_stamp;
        match result {
            Ok((view, gravity)) => {
                let payload = actor.seed.storage.data_mut().unwrap();
                for (i, v) in [6, 7, 8, 11]
                    .into_iter()
                    .zip(gravity.force.into_iter().chain([gravity.force_time_limit]))
                {
                    let at = 0xe4 + i * 4;
                    payload[at..at + 4].copy_from_slice(&v.to_bits().to_le_bytes());
                }
                let physics = crate::kabuto_physics::Physics::from_owned_payload(payload);
                let group = actor.kabuto_group.as_mut().unwrap();
                group.physics = Some(physics);
                group.animation_gravity_completed = Some(number);
                // All initializer prerequisites and entered history remain.
                Ok(view)
            }
            Err(error) => {
                actor
                    .kabuto_group
                    .as_mut()
                    .unwrap()
                    .geometry_transition_failure = Some(error.clone());
                Err(error)
            }
        }
    }
}
