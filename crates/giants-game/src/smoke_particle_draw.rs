//! Shared unlit render inputs from canonical smoke, mixed and positional-impact
//! actors, without a second clock or RNG. Their callback owners stay distinct.
use crate::{
    resident_states::{Id, Session},
    scene_resources::{ModelAsset, SceneResources},
};
use std::sync::{Arc, Weak};
#[derive(Clone, Copy, Debug)]
pub struct State {
    pub class: i32,
    pub passes: u8,
    pub world: crate::procedural_aim::Matrix,
    pub personality: u32,
    pub position: [f32; 3],
    pub scale: f32,
    pub angle: f32,
    pub transparency: f32,
    pub epoch: u32,
}
pub struct Geometry {
    id: Id,
    owner: Weak<crate::specification::Prepared>,
    pub class: i32,
    pub slot: usize,
    pub model: ModelAsset,
}
pub struct Frame {
    pub passes: u8,
    pub positions: Vec<[f32; 3]>,
    pub matrix: [f32; 16],
    pub position: [f32; 3],
    pub front: bool,
    pub profiles: Vec<crate::unlit_material::Profile>,
    /// Source vertex-reference order, distinct from the indexed position buffer.
    pub uv: Vec<[f32; 2]>,
    pub colors: Vec<[f32; 4]>,
    pub vertex_alpha: bool,
}
impl Geometry {
    pub fn load(
        resources: &SceneResources,
        session: &Session,
        id: Id,
        slot: usize,
    ) -> Result<Self, String> {
        let state = session
            .particle_draw_state(id)?
            .ok_or("smoke draw owner is not admitted")?;
        let spec = session
            .specification(id)
            .ok_or("smoke draw specification absent")?;
        let class = resources
            .classes
            .models
            .first_program(state.class)
            .ok_or("smoke draw resource class absent")?;
        let source = spec
            .models
            .get(slot)
            .and_then(Option::as_ref)
            .ok_or("smoke draw model slot absent")?;
        let model = resources
            .model_slot_for_body(
                state.class,
                slot,
                crate::scene_resources::TextureEncoding::Rgb565OrArgb4444,
            )
            .map_err(|e| e.to_string())?
            .ok_or("smoke draw model absent")?;
        if class.catalog != spec.class_catalog
            || model.name != source.name
            || resources
                .archives
                .select_bin(&format!("{}.gbs", model.name), &resources.language)
                .map_err(|e| e.to_string())?
                != &source.geometry_source
            || model.mesh.positions != source.prepared.mesh.positions
            || model.mesh.indices != source.prepared.mesh.indices
            || model.mesh.normal_count != 0
            || source.prepared.skin.is_some()
        {
            return Err("smoke draw requires identical owned unlit static model".into());
        }
        for part in &model.mesh.subobjects {
            crate::unlit_material::prepare_with_object_transparency(
                &part.material,
                model
                    .alpha_textures
                    .contains(&part.material.texture.to_ascii_lowercase()),
                state.transparency,
            )?;
        }
        Ok(Self {
            id,
            owner: Arc::downgrade(spec),
            class: state.class,
            slot,
            model,
        })
    }
    pub fn frame(
        &self,
        session: &mut Session,
        id: Id,
        view: [f32; 16],
    ) -> Result<Option<Frame>, String> {
        if self.id != id {
            return Err("smoke draw geometry belongs to another actor".into());
        }
        let spec = session
            .specification(id)
            .ok_or("smoke draw specification lost")?;
        if !Weak::ptr_eq(&self.owner, &Arc::downgrade(spec)) {
            return Err("smoke draw geometry belongs to another owner".into());
        }
        let Some(state) = session.particle_draw_state(id)? else {
            return Ok(None);
        };
        if state.class != self.class {
            return Err("smoke draw class changed".into());
        }
        let matrix = crate::camera_facing::draw_matrix(
            state.world.values,
            state.personality,
            crate::camera_facing::Input {
                view,
                position: state.position,
                scale: state.scale,
                angle: state.angle,
            },
            crate::arithmetic::Precision::Double53,
        )?;
        session.prepare_actor_positions(id, self.slot, state.epoch, false)?;
        let positions = session
            .actor_model_positions(id, self.slot)?
            .ok_or("smoke draw position buffer absent")?
            .to_vec();
        if positions.len() != self.model.mesh.positions.len()
            || !positions.iter().flatten().all(|v| v.is_finite())
        {
            return Err("smoke draw position buffer invalid".into());
        }
        let response = session
            .get(id)
            .and_then(|a| a.construction_fields.as_ref())
            .ok_or("particle render fields absent")?
            .response_flags;
        let offset = if response & 0x80000 != 0 {
            Some(
                session
                    .timed_effect_uv_offset(id)
                    .ok_or("particle scroll owner absent")?,
            )
        } else {
            None
        }; // Actual absence of a scroll command, not a provider default.
        let vertical = if response & 0x40000 != 0 {
            Some(
                session
                    .timed_effect_vertical_fade(id)
                    .ok_or("particle vertical-fade owner absent")?,
            )
        } else {
            None
        };
        let bounds = self.model.mesh.bounds().map_err(|e| e.to_string())?;
        if self.model.mesh.uv.len() != self.model.mesh.indices.len()
            || self.model.mesh.rgb.len() != self.model.mesh.indices.len()
        {
            return Err("particle vertex-reference attributes inconsistent".into());
        }
        let uv = if let Some(offset) = offset {
            self.model
                .mesh
                .uv
                .iter()
                .map(|uv| std::array::from_fn(|i| (f64::from(uv[i]) + f64::from(offset[i])) as f32))
                .collect()
        } else {
            self.model.mesh.uv.clone()
        };
        let colors = self
            .model
            .mesh
            .rgb
            .iter()
            .enumerate()
            .map(|(i, rgb)| {
                let alpha = if let Some(values) = vertical {
                    let position = positions
                        .get(self.model.mesh.indices[i] as usize)
                        .ok_or("particle position reference absent")?;
                    crate::vertical_fade::alpha(position[2], [bounds[2], bounds[5]], values)?
                } else {
                    255
                };
                Ok([
                    rgb[0] as f32 / 255.,
                    rgb[1] as f32 / 255.,
                    rgb[2] as f32 / 255.,
                    alpha as f32 / 255.,
                ])
            })
            .collect::<Result<Vec<_>, &'static str>>()?;
        let profiles = self
            .model
            .mesh
            .subobjects
            .iter()
            .map(|part| {
                crate::unlit_material::prepare_with_object_transparency(
                    &part.material,
                    self.model
                        .alpha_textures
                        .contains(&part.material.texture.to_ascii_lowercase()),
                    state.transparency,
                )
                .map_err(str::to_owned)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Some(Frame {
            passes: state.passes,
            positions,
            matrix,
            position: state.position,
            front: state.personality & 4 == 0 && state.world.flags & 8 != 0,
            profiles,
            uv,
            colors,
            vertex_alpha: vertical.is_some(),
        }))
    }
}
