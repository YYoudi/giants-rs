//! Render inputs from a consumed positional burst birth. Free-flight snapshots
//! use an explicit fixed step; contacts and mission actor scheduling stay separate.
use crate::{
    arithmetic::Precision,
    effect_burst,
    scene_resources::{ModelAsset, SceneResources},
};
use giants_formats::effect_image::Image;

pub struct Geometry {
    pub class: i32,
    pub model: ModelAsset,
    pub profiles: Vec<crate::unlit_material::Profile>,
    playback: effect_burst::Playback,
    positions: Vec<[f32; 3]>,
    facing: Option<crate::attachment_pose::InitialFacing>,
}
impl Geometry {
    pub fn load(
        resources: &SceneResources,
        image: &Image,
        spawn: effect_burst::Spawn,
        birth: crate::procedural_aim::Matrix,
        slot: usize,
        frames: u32,
    ) -> Result<Self, String> {
        if slot != 0 || frames > 600 || spawn.hidden {
            return Err("burst draw requires visible slot0 and at most600 supplied steps".into());
        }
        let class = spawn.class;
        let program = resources
            .classes
            .models
            .first_program(class)
            .ok_or("burst draw class absent")?
            .program
            .as_ref()
            .map_err(Clone::clone)?;
        let facing =
            crate::released_effect::state78_burst_facing(class, program).map_err(str::to_owned)?;
        let mut animation =
            crate::effect_animation::Playback::new(crate::actor_context::Prepared::class_initial(
                std::sync::Arc::new(crate::actor_context::Class::load(resources, class, image)?),
            ))?;
        let model = resources
            .model_slot_for_body(
                class,
                slot,
                crate::scene_resources::TextureEncoding::Rgb565OrArgb4444,
            )
            .map_err(|e| e.to_string())?
            .ok_or("burst draw model absent")?;
        if model.mesh.normal_count != 0 {
            return Err("burst draw normal renderer still required".into());
        }
        let mut playback = effect_burst::Playback::new(spawn, birth).map_err(str::to_owned)?;
        for _ in 0..frames {
            let expired = playback.advance_free(1. / 60.).map_err(str::to_owned)?;
            if let Some(animation) = &mut animation
                && !expired
            {
                animation.advance(1. / 60.).map_err(str::to_owned)?;
            }
            if expired {
                break;
            }
        }
        let positions = match animation.as_ref().map(|a| a.positions()).transpose()? {
            Some(crate::actor_context::PositionUpdate::Replace(p)) => p,
            _ => model.mesh.positions.clone(),
        };
        if positions.len() != model.mesh.positions.len()
            || !positions.iter().flatten().all(|v| v.is_finite())
        {
            return Err("burst draw current vertices invalid".into());
        }
        let opacity = 1. - playback.visual.transparency;
        let profiles = model
            .mesh
            .subobjects
            .iter()
            .map(|part| {
                let mut profile = crate::unlit_material::prepare_with_texture_alpha(
                    &part.material,
                    model
                        .alpha_textures
                        .contains(&part.material.texture.to_ascii_lowercase()),
                )
                .map_err(str::to_owned)?;
                profile.alpha *= opacity;
                Ok(profile)
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(Self {
            class,
            model,
            profiles,
            playback,
            positions,
            facing,
        })
    }
    pub fn visible(&self) -> bool {
        self.playback.visible()
    }
    pub fn frame(&self, view: [f32; 16]) -> Result<crate::attached_effect_draw::Frame, String> {
        if !self.visible() {
            return Err("burst draw snapshot expired or hidden".into());
        }
        let world = self.playback.matrix().map_err(str::to_owned)?;
        let (matrix, front) = if let Some(facing) = self.facing {
            (
                crate::camera_facing::draw_matrix(
                    world.values,
                    facing.personality,
                    crate::camera_facing::Input {
                        view,
                        position: [world.values[3], world.values[7], world.values[11]],
                        scale: self.playback.visual.size,
                        angle: self.playback.angles.map_or(0., |a| a[2]),
                    },
                    Precision::Double53,
                )
                .map_err(str::to_owned)?,
                facing.front(world.flags),
            )
        } else {
            (world.values, world.flags & 8 != 0)
        };
        Ok(crate::attached_effect_draw::Frame {
            positions: self.positions.clone(),
            matrix,
            front,
        })
    }
}
