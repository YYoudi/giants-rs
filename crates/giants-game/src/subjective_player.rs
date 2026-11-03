//! Owned subjective presentation resources and unblended playback. Input-flag
//! production and complete actor services remain outside this presentation.
use crate::{
    animation_channels as channels,
    arithmetic::Precision,
    first_person,
    scene_resources::{ModelAsset, SceneResources, TextureEncoding},
    specification, state_storage,
};

pub struct Resources {
    pub model: ModelAsset,
    pub offset: [f32; 3],
    pub icon_index: usize,
    pub initial_positions: Vec<[f32; 3]>,
    pub initial_normals: Vec<[f32; 3]>,
}
impl Resources {
    pub fn load(
        resources: &SceneResources,
        seed: &state_storage::Seed,
        spec: &specification::Prepared,
        catalog: &giants_formats::weapons::Catalog,
        icon: i32,
    ) -> Result<Self, String> {
        if seed.storage.profile() != state_storage::Profile::Debug || seed.storage.state() != 2 {
            return Err("subjective resources require the acquired debug Mecc state".into());
        }
        if !seed.unresolved.iter().any(|r| {
            r.offset == 0x70 && r.key == state_storage::ReferenceKey::Spec(spec.specification)
        }) {
            return Err("subjective specification does not match the state reference".into());
        }
        if !spec.needs.is_empty() {
            return Err("subjective specification has preparation needs".into());
        }
        let bytes = seed
            .storage
            .data()
            .ok_or("subjective offset storage absent")?;
        let offset = std::array::from_fn(|i| {
            f32::from_le_bytes(bytes[0x74 + i * 4..0x78 + i * 4].try_into().unwrap())
        });
        if !offset.into_iter().all(f32::is_finite) {
            return Err("nonfinite subjective source offset".into());
        }
        let icon = catalog
            .icons
            .get(&icon)
            .ok_or("subjective weapon icon absent")?;
        let mut index = None;
        for at in &icon.records {
            let record = catalog
                .records
                .iter()
                .find(|r| r.offset == *at)
                .ok_or("subjective icon record missing")?;
            if record.opcode == 0x26 {
                index = Some(
                    *record
                        .args
                        .first()
                        .ok_or("subjective icon index truncated")?,
                );
            }
        }
        let index = index.ok_or("subjective weapon animation index undeclared")?;
        let icon_index = usize::try_from(index).map_err(|_| "subjective icon index")?;
        if icon_index >= 4 {
            return Err("subjective icon outside acquired four-family profile".into());
        }
        let model = resources
            .selection_texture_profile(
                &spec.metadata.selection,
                0,
                TextureEncoding::Rgb565OrArgb4444,
                true,
                false,
            )
            .map_err(|e| e.to_string())?
            .ok_or("subjective model slot empty")?;
        if model.mesh.normal_count == 0
            || model
                .mesh
                .subobjects
                .iter()
                .any(|p| p.material.texture.is_empty())
        {
            return Err("subjective body outside acquired textured lit profile".into());
        }
        let initial = [20, 23, 22, 21][icon_index];
        let profile = crate::body_pipeline::Normals::select(false, true, false, true);
        for part in &model.mesh.subobjects {
            let surface = crate::body_pipeline::preview_surface(
                profile,
                &part.material,
                model
                    .alpha_textures
                    .contains(&part.material.texture.to_ascii_lowercase()),
            )
            .map_err(str::to_owned)?;
            if surface != crate::body_pipeline::Surface::Lit {
                return Err("subjective material outside ordinary lit profile".into());
            }
        }
        let initial_positions = positions(spec, initial, 0.)?;
        let initial_normals =
            crate::body_pipeline::prepare_normals(&model.mesh, &initial_positions, false)
                .map_err(|e| e.to_string())?;
        Ok(Self {
            model,
            offset,
            icon_index,
            initial_positions,
            initial_normals,
        })
    }
}

fn positions(
    spec: &specification::Prepared,
    slot: u8,
    seconds: f32,
) -> Result<Vec<[f32; 3]>, String> {
    let descriptor = spec.animations.get(&slot).ok_or("subjective clip absent")?;
    let skin = spec
        .models
        .first()
        .and_then(Option::as_ref)
        .and_then(|m| m.prepared.skin.as_ref())
        .ok_or("subjective skin absent")?;
    let binding = descriptor
        .template
        .bindings
        .first()
        .and_then(Option::as_ref)
        .ok_or("subjective animation model binding absent")?;
    let tick = crate::model_pose::seconds_tick(descriptor.animation.timeline, seconds)
        .map_err(str::to_owned)?;
    crate::skinning::positions_bound(skin, &descriptor.animation, binding, tick)
        .map_err(|e| e.to_string())
}

pub struct Frame {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub matrix: [f32; 16],
    pub weapon_matrix: [f32; 16],
    pub slot: u8,
    pub seconds: f32,
    pub tick: i32,
}
impl Frame {
    pub fn vertices(&self, mesh: &giants_formats::gbs::Mesh, part: usize) -> Vec<[f32; 3]> {
        mesh.subobjects[part]
            .triangles
            .iter()
            .flatten()
            .map(|id| {
                let p = self.positions[usize::from(mesh.indices[usize::from(*id)])];
                let m = self.matrix;
                let v = std::array::from_fn::<_, 3, _>(|r| {
                    m[r * 4] * p[0] + m[r * 4 + 1] * p[1] + m[r * 4 + 2] * p[2] + m[r * 4 + 3]
                });
                [v[0], v[2], -v[1]]
            })
            .collect()
    }
}
pub struct Playback {
    channels: channels::Channels,
    previous_camera: Option<first_person::Camera>,
    previous_weapon: i32,
    pub frame: Option<Frame>,
}
impl Default for Playback {
    fn default() -> Self {
        let mut channels = [[0; 9]; 4];
        for channel in &mut channels {
            channel[0] = 254;
        }
        Self {
            channels,
            previous_camera: None,
            previous_weapon: -1,
            frame: None,
        }
    }
}
impl Playback {
    pub fn observe(&mut self, camera: first_person::Camera) {
        self.previous_camera = Some(camera);
        self.frame = None;
    }
    #[allow(clippy::too_many_arguments)]
    pub fn step(
        &mut self,
        resources: &Resources,
        spec: &specification::Prepared,
        weapon: &crate::held_weapon::Weapon,
        icon: i32,
        camera: first_person::Camera,
        scale: f32,
        motion: [f32; 3],
        held: bool,
        dt: f32,
    ) -> Result<(), String> {
        let update = first_person::animation(first_person::AnimationInput {
            primary: self.channels[0][0],
            channel_flags: self.channels[0][8],
            // Explicit presentation mapping; native input flag phase is not claimed.
            response: if held { 0x4000 } else { 0 },
            secondary_flags: 0,
            motion,
            icon_index: resources.icon_index,
            weapon: icon,
            previous_weapon: self.previous_weapon,
            inventory_present: false,
        })
        .map_err(str::to_owned)?;
        if let Some(command) = update.command {
            let slot = u8::try_from(command.slot).map_err(|_| "subjective slot overflow")?;
            if !spec.animations.contains_key(&slot) {
                return Err(format!("subjective clip {slot} absent"));
            }
            if update.invalidate_primary || self.channels[0][0] != command.slot {
                let flags = match command.method {
                    first_person::Method::Select => 0x20,
                    first_person::Method::User => 0x21,
                    first_person::Method::Restart => 0x30,
                };
                channels::select_primary(
                    &mut self.channels,
                    channels::Request {
                        slot,
                        flags,
                        speed_source: 0,
                        reference_speed: 0.,
                        blend_seconds: f32::from_bits(0x3e2aaaab),
                    },
                    channels::Blending::Disabled,
                )
                .map_err(str::to_owned)?;
            }
        }
        self.previous_weapon = update.previous_weapon;
        let mut durations = vec![None; 254];
        for (slot, descriptor) in &spec.animations {
            durations[usize::from(*slot)] = Some(descriptor.template.duration);
        }
        channels::advance(
            &mut self.channels,
            &durations,
            motion,
            dt,
            Precision::Double53,
        )
        .map_err(str::to_owned)?;
        let slot = u8::try_from(self.channels[0][0]).map_err(|_| "subjective primary slot")?;
        let seconds = f32::from_bits(self.channels[0][1]);
        let pose = first_person::place(
            self.previous_camera.unwrap_or(camera),
            camera,
            resources.offset,
        )
        .map_err(str::to_owned)?;
        let matrix = crate::object_transform::euler(pose.position, pose.angles, scale)
            .map_err(str::to_owned)?;
        let descriptor = spec
            .animations
            .get(&slot)
            .ok_or("subjective selected clip absent")?;
        let tick = crate::model_pose::seconds_tick(descriptor.animation.timeline, seconds)
            .map_err(str::to_owned)?;
        let reference = spec
            .metadata
            .references
            .bind(&descriptor.animation)
            .map_err(|e| e.to_string())?
            .evaluate(1, tick)
            .map_err(|e| e.to_string())?
            .matrix
            .ok_or("subjective right hand reference absent")?;
        let reference = crate::animation_links::apply(1, reference, &spec.corrections)
            .map_err(|e| e.to_string())?;
        let weapon_matrix = weapon
            .matrix_from_reference(reference.values, matrix, scale)
            .map_err(|e| e.to_string())?;
        let positions = positions(spec, slot, seconds)?;
        let normals =
            crate::body_pipeline::prepare_normals(&resources.model.mesh, &positions, false)
                .map_err(|e| e.to_string())?;
        self.frame = Some(Frame {
            positions,
            normals,
            matrix,
            weapon_matrix,
            slot,
            seconds,
            tick,
        });
        self.previous_camera = Some(camera);
        Ok(())
    }
}
