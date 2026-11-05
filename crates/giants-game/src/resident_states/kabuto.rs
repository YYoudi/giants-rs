use super::{Id, Session};
use crate::{
    animation_channels::Blending,
    kabuto_player::{ColdFrame, Initialized},
    kabuto_resources::Resources,
    state_storage::Profile,
};
use std::sync::Arc;

impl Session {
    pub(crate) fn construct_kabuto_auxiliary(
        &mut self,
        input: super::Construction<'_>,
        mask: u32,
    ) -> Result<Id, String> {
        let effect = matches!(
            (input.class, input.seed.storage.state(), mask),
            (866 | 980, 78, 0)
        );
        if !(effect
            || matches!(
                (input.class, input.seed.storage.state(), mask),
                (749 | 1018, 0, 1)
            ))
            || input.seed.storage.profile() != Profile::Debug
            || !input.seed.diagnostics.is_empty()
            || !input.bindings.is_empty()
            || input.fields.object_flags != input.flags.active
            || input.fields.configuration[14] != input.flags.personality
            || (effect
                && input
                    .seed
                    .storage
                    .data()
                    .is_none_or(|p| p.len() != 268 || p.iter().any(|&b| b != 0)))
        {
            return Err(
                "Kabuto auxiliary requires its acquired class/state/mask and cold resources".into(),
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
        let channels = input.fields.channels;
        staged.bind_specification(id, input.specification)?;
        staged.bind_state_resources(id, input.state_resources)?;
        staged.bind_actor_fields(id, input.fields)?;
        let actor = staged.residents.get_mut(&id.serial).unwrap();
        actor.channels = Some(channels);
        if effect {
            // The actual declared state is allocated first; only its observed
            // all-zero destructor profile is admitted. Live emitters are refused.
            actor.seed.storage.transition_with(0, |state, body| {
                if state == 78 && body.len() == 268 && body.iter().all(|&b| b == 0) {
                    Ok(())
                } else {
                    Err("Kabuto cold effect cleanup requires empty state78".into())
                }
            })?;
            actor.construction_fields.as_mut().unwrap().configuration[20] = 0;
        }
        actor.acquired_construction = true;
        self.publish_staged(staged, id)
    }
    pub fn kabuto_group(&self, id: Id) -> Option<&crate::kabuto_initial::Group> {
        self.get(id)?.kabuto_group.as_ref()
    }
    /// The actual seeded external-orientation cache, not an evaluated world
    /// transform or contact-readiness acknowledgement.
    pub fn kabuto_initial_effect_cache(
        &self,
        owner: Id,
        id: Id,
    ) -> Option<crate::actor_affine::Cache> {
        let group = self.kabuto_group(owner)?;
        if !group.effects.contains(&id) && group.final_effect != id {
            return None;
        }
        let actor = self.get(id)?;
        let fields = actor.construction_fields.as_ref()?;
        let cache = actor.affine_cache.as_ref()?;
        if actor.world_tick.is_some()
            || actor.controlled_motion.is_some()
            || actor.finalization.is_some()
            || cache.pose != super::affine_pose_words(fields)
            || cache.value.flags != 0
        {
            return None;
        }
        Some(cache.value)
    }
    pub(crate) fn stage_kabuto_group(
        &mut self,
        build: impl FnOnce(&mut Self) -> Result<Id, String>,
    ) -> Result<Id, String> {
        if self.residents.values().any(|a| a.kabuto_group.is_some()) {
            return Err(
                "Kabuto group already published; live type replacement is unacquired".into(),
            );
        }
        let mut staged = Self::new(self.ticket);
        staged.next = self.next;
        let owner = build(&mut staged)?;
        if staged.get(owner).is_none_or(|a| a.kabuto_group.is_none())
            || staged.residents.len() != 12
        {
            return Err("Kabuto cold group is incomplete".into());
        }
        let mut links = self.links.clone();
        for (&serial, actor) in &staged.residents {
            links.construct(
                serial,
                actor.class,
                u32::from(actor.seed.storage.state()),
                actor.initial_flags,
                actor.constructor_mask,
            )?;
        }
        // No fallible operation follows publication of the fully checked group.
        self.links = links;
        self.next = staged.next;
        self.residents.append(&mut staged.residents);
        self.specifications.append(&mut staged.specifications);
        self.state_resources.append(&mut staged.state_resources);
        Ok(owner)
    }
    pub(crate) fn bind_kabuto_attachment(
        &mut self,
        owner: Id,
        child: Id,
        second: bool,
    ) -> Result<(), String> {
        let context = self
            .get(owner)
            .filter(|a| a.class == 1 && a.kabuto_player_fields)
            .and_then(|a| a.construction_fields.as_ref())
            .ok_or("Kabuto attachment owner absent")?
            .dispatch_context;
        let actor = self.get(child).ok_or("Kabuto attachment child absent")?;
        if actor.class != if second { 1018 } else { 749 }
            || actor.seed.storage.state() != 0
            || !actor.acquired_construction
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
        {
            return Err("Kabuto attachment requires its fresh acquired child".into());
        }
        let actor = self.residents.get_mut(&child.serial).unwrap();
        let fields = actor.construction_fields.as_mut().unwrap();
        fields.dispatch_context = context;
        fields.configuration[14] |= if second { 0x20000000 } else { 0x24000000 };
        fields.object_flags |= 0x400000;
        fields.response_flags |= 0x2000000;
        actor.attachment = Some(super::Attachment {
            parent: owner,
            child_reference: 0,
            parent_reference: 10,
            behavior: 0,
        });
        actor.presentation_parent = Some(owner);
        Ok(())
    }
    pub(crate) fn bind_kabuto_effect(&mut self, owner: Id, id: Id) -> Result<(), String> {
        let context = self
            .get(owner)
            .filter(|a| a.class == 1 && a.kabuto_player_fields)
            .and_then(|a| a.construction_fields.as_ref())
            .ok_or("Kabuto effect owner absent")?
            .dispatch_context;
        let actor = self.get(id).ok_or("Kabuto effect absent")?;
        if !matches!(actor.class, 866 | 980)
            || actor.seed.storage.state() != 0
            || !actor.acquired_construction
            || actor.affine_cache.is_some()
        {
            return Err("Kabuto effect requires its cold reset constructor".into());
        }
        let actor = self.residents.get_mut(&id.serial).unwrap();
        let fields = actor.construction_fields.as_mut().unwrap();
        fields.dispatch_context = context;
        fields.configuration[14] |= 0xe00;
        fields.object_flags |= 0x40010000;
        actor.affine_cache = Some(super::OwnedAffine {
            value: crate::actor_affine::Cache {
                flags: 0,
                matrix: crate::procedural_aim::Matrix::identity().values,
            },
            pose: super::affine_pose_words(fields),
            external_orientation: true,
        });
        Ok(())
    }
    pub(crate) fn finish_kabuto_group(
        &mut self,
        id: Id,
        fields: crate::actor_fields::Fields,
        payload: Vec<u8>,
        group: crate::kabuto_initial::Group,
    ) -> Result<(), String> {
        let actor = self.get(id).ok_or("Kabuto group owner absent")?;
        if actor.class != 1
            || !actor.kabuto_player_fields
            || actor.kabuto_group.is_some()
            || actor
                .seed
                .storage
                .data()
                .is_none_or(|p| p.len() != payload.len())
            || payload.len() != 844
        {
            return Err("Kabuto group owner is not a fresh cold player".into());
        }
        let actor = self.residents.get_mut(&id.serial).unwrap();
        actor
            .seed
            .storage
            .data_mut()
            .unwrap()
            .copy_from_slice(&payload);
        actor.channels = Some(fields.channels);
        actor.construction_fields = Some(fields);
        actor.kabuto_group = Some(group);
        Ok(())
    }
    pub fn has_cold_kabuto_player_fields(&self, id: Id) -> bool {
        self.get(id).is_some_and(|a| a.kabuto_player_fields)
    }
    pub(crate) fn initialize_cold_kabuto_player_fields(
        &mut self,
        owner: Id,
        resources: &Resources,
        frame: ColdFrame,
        blending: Blending,
    ) -> Result<Initialized, String> {
        let actor = self
            .get(owner)
            .ok_or("Kabuto player owner absent or foreign")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("Kabuto player fields absent")?;
        if actor.class != 1
            || !actor.acquired_construction
            || actor.kabuto_player_fields
            || actor.seed.storage.profile() != Profile::Debug
            || actor.seed.storage.state() != 1
            || actor.seed.storage.data() != resources.state_template().storage.data()
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || actor.controlled_motion.is_some()
            || actor.ordinary_motion.is_some()
            || actor.environment.is_some()
            || actor.finalization.is_some()
            || actor.world_tick.is_some()
            || actor.world_tick_running
            || actor.affine_cache.is_some()
            || actor.mecc_physics_present
            || actor.mecc_animation_stamp != 0
            || actor.channels != Some(fields.channels)
            || fields.channels != resources.class_fields().channels
            || !self
                .specification(owner)
                .is_some_and(|s| Arc::ptr_eq(s, &resources.specification))
            || !self
                .state_resources(owner)
                .is_some_and(|s| Arc::ptr_eq(s, &resources.states))
            || !resources.specification.animations.contains_key(&1)
            || !self.specifications.get(&owner.serial).is_some_and(|s| {
                !s.0.sampled()
                    && s.0
                        .matrices()
                        .iter()
                        .all(|m| m.flags == 0 && m.values.iter().all(|v| v.to_bits() == 0))
            })
        {
            return Err("Kabuto player fields require their fresh unparented cold constructor and resources".into());
        }
        let next =
            crate::kabuto_player::project(fields, frame, resources.class_fields().scale, blending)?;
        let actor = self.residents.get_mut(&owner.serial).unwrap();
        actor.channels = Some(next.channels);
        actor.construction_fields = Some(next);
        actor.kabuto_player_fields = true;
        Ok(Initialized {
            owner,
            camera_projection_reset_required: true,
        })
    }
}
