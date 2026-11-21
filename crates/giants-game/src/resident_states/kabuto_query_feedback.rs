use super::*;
use crate::kabuto_query_feedback as feedback;

impl Session {
    fn kabuto_query_effect_state(&self, owner: Id) -> Result<(Id, feedback::State), String> {
        let g = self
            .kabuto_group(owner)
            .ok_or("Kabuto feedback group absent")?;
        let effect = g.final_effect;
        let a = self
            .get(effect)
            .ok_or("Kabuto query final effect absent or foreign")?;
        let f = a
            .construction_fields
            .as_ref()
            .ok_or("Kabuto effect fields absent")?;
        if effect.ticket() != owner.ticket()
            || a.class != 980
            || a.seed.storage.profile() != Profile::Debug
            || f.object_flags & 0x8000 != 0
        {
            return Err("Kabuto feedback requires its live final effect".into());
        }
        Ok((
            effect,
            feedback::State {
                scalar: a
                    .kabuto_effect_scalar
                    .ok_or("Kabuto effect scalar has no producer")?,
                flags: f.object_flags,
                position: f.position,
                affine: a
                    .affine_cache
                    .as_ref()
                    .ok_or("Kabuto effect affine absent")?
                    .value,
            },
        ))
    }

    pub(crate) fn captured_kabuto_query_feedback(
        &self,
        owner: Id,
        resources: &Resources,
        number: u64,
    ) -> Result<feedback::View, String> {
        self.captured_kabuto_query_frame(owner, resources, number)?;
        if self
            .kabuto_group(owner)
            .unwrap()
            .reference_query
            .feedback_completed
            != Some(number)
        {
            return Err("Kabuto query feedback is not completed on this owner/frame".into());
        }
        let (effect, state) = self.kabuto_query_effect_state(owner)?;
        Ok(feedback::View {
            owner,
            number,
            effect,
            state,
        })
    }

    pub(crate) fn advance_kabuto_query_feedback(
        &mut self,
        owner: Id,
        resources: &Resources,
        number: u64,
        services: &mut impl feedback::Services,
    ) -> Result<feedback::View, String> {
        let query = self.captured_kabuto_query_frame(owner, resources, number)?;
        let g = self.kabuto_group(owner).unwrap();
        if g.reference_query.feedback_entered.is_some()
            || g.action_select_entered.is_some()
            || g.held_reference_entered.is_some()
            || g.animation_commit_entered.is_some()
            || g.primary_tail_entered.is_some()
        {
            return Err("Kabuto query feedback requires unconsumed ordered work".into());
        }
        let (effect, mut state) = self.kabuto_query_effect_state(owner)?;
        self.residents
            .get_mut(&owner.serial)
            .unwrap()
            .kabuto_group
            .as_mut()
            .unwrap()
            .reference_query
            .feedback_entered = Some(number);
        let result = feedback::advance(
            &mut state,
            query.target.raw,
            f32::from_bits(query.caller_scratch.retained_time),
            |target, factor| services.target_transform(self, target, factor),
        );
        // The required transform provider sees an immutable Session. Publish
        // completed local writes even if a later finite-profile guard refused.
        let a = self.residents.get_mut(&effect.serial).unwrap();
        a.kabuto_effect_scalar = Some(state.scalar);
        let f = a.construction_fields.as_mut().unwrap();
        f.object_flags = state.flags;
        f.position = state.position;
        a.affine_cache = Some(super::super::OwnedAffine {
            value: state.affine,
            pose: super::super::affine_pose_words(f),
            external_orientation: true,
        });
        let g = self
            .residents
            .get_mut(&owner.serial)
            .unwrap()
            .kabuto_group
            .as_mut()
            .unwrap();
        if let Err(error) = result {
            g.geometry_transition_failure
                .get_or_insert_with(|| error.clone());
            return Err(error);
        }
        g.reference_query.feedback_completed = Some(number);
        self.captured_kabuto_query_feedback(owner, resources, number)
    }
}
