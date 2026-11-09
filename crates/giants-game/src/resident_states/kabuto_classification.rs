use super::*;
use crate::{arithmetic::Precision, kabuto_classification::Contact, player_mesh_contact};

impl Session {
    pub(crate) fn advance_kabuto_classification(
        &mut self,
        owner: Id,
        resources: &Resources,
        bounds: &giants_formats::world_bounds::Resolved,
        number: u64,
        scene: crate::kabuto_classification::Scene<'_>,
        ground: impl FnMut([f32; 2]) -> Result<crate::physical_ground::Ground, &'static str>,
    ) -> Result<(), String> {
        let actor = self
            .get(owner)
            .ok_or("Kabuto classification owner absent or foreign")?;
        let group = actor
            .kabuto_group
            .as_ref()
            .ok_or("Kabuto classification group absent")?;
        if actor.class != 1
            || actor.seed.storage.state() != 1
            || group.cleaned_up
            || group.geometry_transition_failure.is_some()
            || group.motion_history != Some(number)
            || group.motion_prefix_completed != Some(number)
            || group.classification_completed.is_some()
            || !Arc::ptr_eq(&group.declared_class, &resources.specification)
            || self
                .specification(owner)
                .is_none_or(|s| !Arc::ptr_eq(s, &resources.specification))
            || scene.owner == 0
        {
            return Err(
                "Kabuto classification requires its factory-owned unconsumed motion prefix".into(),
            );
        }
        let body = actor
            .seed
            .storage
            .data()
            .ok_or("Kabuto classification payload absent")?;
        if body.len() != 844
            || group.physics.is_none_or(|p| {
                p.words() != crate::kabuto_physics::Physics::from_owned_payload(body).words()
            })
        {
            return Err("Kabuto classification physical cache differs from its payload".into());
        }
        let mut fields = actor
            .construction_fields
            .clone()
            .ok_or("Kabuto classification fields absent")?;
        let mut payload = body.to_vec();
        let mut motion = group.motion;
        let mut contact = group.contact;
        let mut radius = group.radius;
        let parent = actor.attachment.is_some() || actor.presentation_parent.is_some();
        let previous_affine = actor.affine_cache.as_ref().map(|c| c.value);
        let peers = std::cell::RefCell::new(scene.peers);
        let mut affine = None;
        // The owned group has no volume-query cache producer yet. Its absent
        // cache needs no allocation/free shim; a future producer must join here.
        fields.object_flags &= 0xf7e7_ffff;
        let result: Result<(), &'static str> = (|| {
            if parent {
                return Ok(());
            }
            let p = |at: usize| f32::from_le_bytes(payload[at..at + 4].try_into().unwrap());
            let physical_flags = u32::from_le_bytes(payload[0xe4..0xe8].try_into().unwrap());
            let out = crate::player_world_collision::resolve_for_actor_with_responses(
                scene.owner,
                crate::player_world_collision::Input {
                    state: player_mesh_contact::State {
                        current: fields.position,
                        previous: fields.current,
                        velocity: motion.velocity,
                        scale: fields.scale,
                        radius,
                        height: f32::from_bits(fields.configuration[23]),
                        floor: contact.floor,
                        plane: contact.plane,
                        physics_flags: physical_flags,
                        support: contact.support,
                        hit: contact.hit,
                        support_part: contact.support_part,
                        support_triangle: contact.support_triangle,
                        restitution: p(0xf0),
                        retention: p(0xf4),
                    },
                    object_flags: fields.object_flags,
                    collision_flags: fields.configuration[14],
                    ground_offset: f32::from_bits(fields.configuration[17]),
                    center_offset: f32::from_bits(fields.configuration[22]),
                    gravity_mass: p(0xe8),
                    force_z: p(0x104),
                },
                scene.objects,
                ground,
                |s| {
                    crate::player_collision_radius::Bounds {
                        primary: bounds.primary,
                        animation_one: bounds.animation,
                    }
                    .refresh(
                        s.radius,
                        s.scale,
                        s.physics_flags,
                        Precision::Double53,
                    )
                },
                |object, state| {
                    let mut peers = peers.borrow_mut();
                    let peer = peers.profile(object.fields.id)?;
                    let decision =
                        crate::kabuto_contact::decide(state.current[2], state.scale, Some(peer))?;
                    if decision.bind_peer_support {
                        peers.bind_support(object.fields.id, scene.owner)?;
                    }
                    Ok(decision.handled)
                },
                |object, response| {
                    if response.reciprocal_hit {
                        peers
                            .borrow_mut()
                            .bind_support(object.fields.id, scene.owner)?;
                    }
                    Ok(())
                },
            )?;
            fields.position = out.state.current;
            fields.current = out.state.previous;
            motion.velocity = out.state.velocity;
            radius = out.state.radius;
            contact = Contact {
                floor: out.state.floor,
                plane: out.state.plane,
                support: out.state.support,
                hit: out.state.hit,
                support_part: out.state.support_part,
                support_triangle: out.state.support_triangle,
            };
            payload[0xe4..0xe8].copy_from_slice(&out.state.physics_flags.to_le_bytes());
            payload[0x104..0x108].copy_from_slice(&out.force_z.to_le_bytes());
            let (flags, value) = crate::actor_affine::update(
                crate::uncached_pose::Pose {
                    position: fields.position,
                    angles: fields.angles,
                    scale: fields.scale,
                },
                out.object_flags,
                previous_affine,
                Precision::Double53,
            )?;
            fields.object_flags = flags;
            affine = Some(value);
            Ok(())
        })();
        let actor = self.residents.get_mut(&owner.serial).unwrap();
        if let Err(reason) = result {
            // External peer effects are already visible. Quarantine prevents
            // replay; no source exception/rollback behavior is inferred here.
            actor
                .kabuto_group
                .as_mut()
                .unwrap()
                .geometry_transition_failure =
                Some(format!("Kabuto classification failed: {reason}"));
            return Err(reason.into());
        }
        if let Some(value) = affine {
            actor.affine_cache = Some(super::super::OwnedAffine {
                value,
                pose: super::super::affine_pose_words(&fields),
                external_orientation: fields.object_flags & 0x10000 != 0,
            });
        }
        actor.construction_fields = Some(fields);
        actor
            .seed
            .storage
            .data_mut()
            .unwrap()
            .copy_from_slice(&payload);
        let group = actor.kabuto_group.as_mut().unwrap();
        group.motion = motion;
        group.contact = contact;
        group.radius = radius;
        group.physics = Some(crate::kabuto_physics::Physics::from_owned_payload(&payload));
        group.classification_completed = Some(number);
        // Keep the history/prefix tokens: world updates and new forces still
        // need actual consumers before another primary can start.
        Ok(())
    }
}
