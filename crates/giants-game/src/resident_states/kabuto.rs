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
