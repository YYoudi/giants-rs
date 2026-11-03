//! Real owned ordinary ANM tail at a caller-selected physical boundary.
//! Selection is still the acquired normal profile, not the whole secondary.
use crate::{
    actor_fields::Fields,
    actor_references,
    animation_channels::{self, Channels},
    attached_effect_context::Pool,
    audio_parameters::Random,
    mecc_animation_events,
    mecc_physical_cycle::View,
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
impl Pipeline<'_> {
    pub fn advance(&mut self, fields: &Fields, view: &View, seconds: f32) -> Result<Pass, String> {
        let mut durations = vec![None; 256];
        for (&slot, descriptor) in &self.specification.animations {
            durations[usize::from(slot)] = Some(descriptor.template.duration);
        }
        let tick = animation_channels::advance(
            &mut self.channels,
            &durations,
            [
                view.model_rates[0],
                view.model_rates[1],
                view.model_rates[2],
                view.angular_rate,
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
                    affine: view.affine.ok_or("interleaved ANM affine absent")?.matrix,
                    material: match &self.materials {
                        Some(catalog) => mecc_animation_events::Material::Acquired(
                            catalog.resolve(view.support, view.support_part)?,
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
    pub fn select_normal(
        &mut self,
        fields: &Fields,
        view: &View,
        globals: &giants_formats::global_data::GlobalData,
    ) -> Result<Option<animation_channels::Request>, String> {
        let selection = crate::mecc_locomotion::select(
            crate::mecc_locomotion::Input {
                model: view.model_rates,
                position_z: fields.position[2],
                floor: view.floor,
                center_offset: f32::from_bits(fields.configuration[17]),
                scale: fields.scale,
                supported: view.support != 0,
            },
            crate::mecc_locomotion::Parameters::from_globals(globals).map_err(|e| e.to_string())?,
            crate::arithmetic::Precision::Double53,
        )?;
        let request = (self.channels[0][0] != u32::from(selection.clip)).then_some(
            animation_channels::Request {
                slot: selection.clip,
                flags: selection.flags,
                speed_source: selection.speed_source,
                reference_speed: selection.reference_speed,
                blend_seconds: f32::from_bits(0x3e2aaaab),
            },
        );
        if let Some(request) = request {
            animation_channels::select_primary(&mut self.channels, request, self.blending)?;
        }
        Ok(request)
    }
}
