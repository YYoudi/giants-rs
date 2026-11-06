//! Shared owned ordinary ANM phase with actor references and class event state.
//! Character selection, procedural consumers and scheduling remain separate.
use crate::{
    actor_fields::Fields,
    actor_references,
    animation_channels::{self, Channels},
    attached_effect_context::Pool,
    audio_parameters::Random,
    mecc_animation_events,
    procedural_aim::Matrix,
    resident_states::Id,
    specification::Prepared,
};
use std::sync::Arc;

/// World-owned dependencies shared across actors and both ANM passes.
pub struct Services<'a> {
    pub contexts: &'a mut Pool<Id>,
    pub random: &'a mut Random,
    pub stamp: &'a mut crate::animation_epoch::Stamp,
    pub blending: animation_channels::Blending,
}
pub(crate) struct Pipeline<'a> {
    pub materials: Option<Arc<crate::mecc_contact_materials::Catalog>>,
    pub specification: Arc<Prepared>,
    pub references: actor_references::Owned,
    pub bank: mecc_animation_events::Bank,
    pub contexts: Pool<Id>,
    pub random: Random,
    pub channels: Channels,
    pub owner: Id,
    pub stamp: &'a mut crate::animation_epoch::Stamp,
    pub last_stamp: u32,
    pub blending: animation_channels::Blending,
}
pub(crate) struct Pass {
    pub channels: Channels,
    pub references: Option<Vec<Matrix>>,
    pub processed: bool,
    pub visits: u32,
    pub stamp: u32,
}
pub(crate) struct Sample {
    pub model: [f32; 4],
    pub affine: Option<crate::actor_affine::Cache>,
    pub support: u32,
    pub support_part: u32,
}
impl Pipeline<'_> {
    pub fn advance_sample(
        &mut self,
        fields: &Fields,
        sample: Sample,
        seconds: f32,
    ) -> Result<Pass, String> {
        let mut durations = vec![None; 256];
        for (&slot, descriptor) in &self.specification.animations {
            durations[usize::from(slot)] = Some(descriptor.template.duration);
        }
        let tick = animation_channels::advance(
            &mut self.channels,
            &durations,
            [
                sample.model[0],
                sample.model[1],
                sample.model[2],
                sample.model[3],
            ],
            seconds,
            crate::arithmetic::Precision::Double53,
        )?;
        if tick.missing != 0 {
            return Err("interleaved ANM descriptor absent".into());
        }
        let mut references = None;
        let mut visits = 0;
        if tick.processed != 0 {
            self.last_stamp = self.stamp.advance();
            self.references
                .update_current(&self.channels, self.last_stamp)?;
            let matrices = self.references.matrices();
            visits = self.bank.dispatch(
                mecc_animation_events::Input {
                    owner: self.owner,
                    fields,
                    channels: &self.channels,
                    references: matrices,
                    affine: sample.affine.map(|cache| cache.matrix),
                    material: match &self.materials {
                        Some(catalog) => mecc_animation_events::Material::Acquired(
                            catalog.resolve(sample.support, sample.support_part)?,
                        ),
                        None => mecc_animation_events::Material::Unknown,
                    },
                },
                &mut self.contexts,
                &mut self.random,
            )?;
            references = Some(matrices.to_vec());
        }
        Ok(Pass {
            channels: self.channels,
            references,
            processed: tick.processed != 0,
            visits,
            stamp: self.last_stamp,
        })
    }
}
