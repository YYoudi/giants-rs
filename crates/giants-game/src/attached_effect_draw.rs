//! Render inputs from a real owned attached FX object and its current vertices.
//! Slot/vertex epoch and camera are explicit inputs; no live LOD or cadence implied.
use crate::{
    resident_states::{Id, Session},
    scene_resources::{ModelAsset, SceneResources},
};
pub struct Geometry {
    owner: std::sync::Arc<crate::specification::Prepared>,
    pub class: i32,
    pub slot: usize,
    pub model: ModelAsset,
    pub profiles: Vec<crate::unlit_material::Profile>,
}
pub struct Frame {
    pub positions: Vec<[f32; 3]>,
    pub matrix: [f32; 16],
    pub front: bool,
}
impl Geometry {
    pub fn load(
        resources: &SceneResources,
        session: &Session,
        id: Id,
        slot: usize,
    ) -> Result<Self, String> {
        let actor = session.get(id).ok_or("FX draw owner absent or foreign")?;
        session
            .attached_effect(id)
            .ok_or("FX draw requires real attached object")?;
        let spec = session
            .specification(id)
            .ok_or("FX draw specification absent")?;
        let declaration = resources
            .classes
            .models
            .first_program(actor.class)
            .ok_or("FX draw resource class absent")?;
        if declaration.catalog != spec.class_catalog {
            return Err("FX draw resource context differs from actor".into());
        }
        let source = spec
            .models
            .get(slot)
            .and_then(Option::as_ref)
            .ok_or("FX draw model slot absent")?;
        let model = resources
            .model_slot_for_body(
                actor.class,
                slot,
                crate::scene_resources::TextureEncoding::Rgb565OrArgb4444,
            )
            .map_err(|e| e.to_string())?
            .ok_or("FX draw geometry absent")?;
        if model.name != source.name
            || model.mesh.indices != source.prepared.mesh.indices
            || model.mesh.positions != source.prepared.mesh.positions
        {
            return Err("FX draw model differs from owned source geometry".into());
        }
        if model.mesh.normal_count != 0 {
            return Err("owned FX normal renderer consumer still required".into());
        }
        let profiles = model
            .mesh
            .subobjects
            .iter()
            .map(|part| {
                crate::unlit_material::prepare_with_texture_alpha(
                    &part.material,
                    model
                        .alpha_textures
                        .contains(&part.material.texture.to_ascii_lowercase()),
                )
                .map_err(str::to_owned)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            owner: spec.clone(),
            class: actor.class,
            slot,
            model,
            profiles,
        })
    }
    pub fn frame(
        &self,
        session: &mut Session,
        id: Id,
        epoch: u32,
        force: bool,
        view: [f32; 16],
    ) -> Result<Frame, String> {
        let actor = session.get(id).ok_or("FX draw actor absent or foreign")?;
        if actor.class != self.class
            || session
                .specification(id)
                .is_none_or(|s| !std::sync::Arc::ptr_eq(s, &self.owner))
        {
            return Err("FX draw geometry belongs to another class".into());
        }
        let world = session.current_attached_effect_world(id)?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("FX draw fields absent")?;
        let personality = fields.configuration[14];
        let matrix = crate::camera_facing::draw_matrix(
            world.values,
            personality,
            crate::camera_facing::Input {
                view,
                position: fields.position,
                scale: fields.scale,
                angle: fields.angles[0],
            },
            crate::arithmetic::Precision::Double53,
        )
        .map_err(str::to_owned)?;
        let front = personality & 4 == 0 && world.flags & 8 != 0;
        session.prepare_actor_positions(id, self.slot, epoch, force)?;
        let positions = session
            .actor_model_positions(id, self.slot)?
            .ok_or("FX current vertex buffer absent")?
            .to_vec();
        if positions.len() != self.model.mesh.positions.len()
            || !positions.iter().flatten().all(|v| v.is_finite())
        {
            return Err("FX vertex buffer invalid".into());
        }
        Ok(Frame {
            positions,
            matrix,
            front,
        })
    }
}
