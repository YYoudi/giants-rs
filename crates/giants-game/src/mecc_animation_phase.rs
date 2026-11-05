//! Mecc selection and compatibility adapter over the shared owned ANM core.
pub use crate::actor_animation_phase::Services;
pub(crate) use crate::actor_animation_phase::{Pass, Pipeline};
use crate::{actor_fields::Fields, animation_channels, mecc_physical_cycle::View};
impl Pipeline<'_> {
    pub fn advance(&mut self, fields: &Fields, view: &View, seconds: f32) -> Result<Pass, String> {
        self.advance_sample(
            fields,
            crate::actor_animation_phase::Sample {
                model: [
                    view.model_rates[0],
                    view.model_rates[1],
                    view.model_rates[2],
                    view.angular_rate,
                ],
                affine: view.affine,
                support: view.support,
                support_part: view.support_part,
            },
            seconds,
        )
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
