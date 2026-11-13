//! First gun: native fixed-function lighting profile, source data only.
use bevy::{
    asset::RenderAssetUsages,
    mesh::MeshVertexBufferLayoutRef,
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    reflect::TypePath,
    render::render_resource::{
        AsBindGroup, PrimitiveTopology, RenderPipelineDescriptor, ShaderType,
        SpecializedMeshPipelineError,
    },
    shader::ShaderRef,
};

#[derive(Clone, Debug, ShaderType)]
pub struct Light {
    direction: Vec4,
    color: Vec4,
}
#[derive(Clone, Debug, ShaderType)]
pub struct Parameters {
    world: Mat4,
    view: Mat4,
    clip: Mat4,
    normal_eye: Mat4,
    emissive: Vec4,
    ambient: Vec4,
    diffuse: Vec4,
    specular: Vec4,
    lights: [Light; 3],
    fog: Vec4,
    fog_color: Vec4,
    pixel_power: Vec4,
    composition: Vec4,
}
impl Parameters {
    pub(crate) fn object_alpha(mut self, factor: f32) -> Self {
        self.composition.x = factor;
        self
    }
    pub(crate) fn additive_profile(self, profile: Option<(f32, bool)>) -> Self {
        match profile {
            Some((factor, true)) => self.additive_texture(factor),
            Some((factor, false)) => self.additive_diffuse(factor),
            None => self,
        }
    }
    fn additive_diffuse(mut self, factor: f32) -> Self {
        self.specular = Vec4::ZERO;
        self.fog_color = Vec4::ZERO;
        self.composition.x = factor;
        self.composition.y = 0.;
        self
    }
    pub(crate) fn additive_texture(mut self, factor: f32) -> Self {
        self.pixel_power.w = 1.;
        self.specular = Vec4::ZERO;
        self.fog_color = Vec4::ZERO;
        self.composition.x = factor;
        self
    }
    pub(crate) fn suppress_primary_diffuse(mut self, enabled: bool) -> Self {
        if enabled {
            self.lights[0].color.w = 0.;
        }
        self
    }
    pub(crate) fn buffer(&self) -> bevy::render::storage::ShaderBuffer {
        crate::source_view::uniform_buffer(self)
    }
    pub(crate) fn write_buffer(&self, buffer: &mut bevy::render::storage::ShaderBuffer) {
        crate::source_view::write_uniform(self, buffer);
    }
    pub(crate) fn white_texture_factor(mut self, enabled: bool) -> Self {
        self.pixel_power.w = if enabled { 1. } else { 0. };
        self
    }
    /// Opaque texture-times-white program, with the independently acquired
    /// no-specular draw state. A zero cube factor is a different program.
    pub(crate) fn opaque_texture_factor_only(mut self, enabled: bool) -> Self {
        if enabled {
            self.pixel_power.w = 2.;
            self.specular = Vec4::ZERO;
        }
        self
    }
    pub(crate) fn untextured(mut self, enabled: bool) -> Self {
        if enabled {
            self.pixel_power.w = -1.;
        }
        self
    }
}
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct Material {
    #[texture(0)]
    #[sampler(1)]
    texture: Handle<Image>,
    #[uniform(2)]
    parameters: Parameters,
}
impl bevy::pbr::Material for Material {
    fn vertex_shader() -> ShaderRef {
        "embedded://giants_app/gun.wesl".into()
    }
    fn fragment_shader() -> ShaderRef {
        "embedded://giants_app/gun.wesl".into()
    }
    fn specialize(
        _: &MaterialPipeline,
        d: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        d.vertex.buffers = vec![layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(2),
        ])?];
        d.primitive.cull_mode = Some(bevy::render::render_resource::Face::Back);
        Ok(())
    }
}
pub type Scene = crate::cinematic_scene::Scenes<Gun>;
pub struct Gun {
    gun: Option<giants_game::attachments::Attachment>,
    textures: Vec<giants_game::LogoTexture>,
    normals: Vec<[f32; 3]>,
    lights: Vec<giants_game::lighting::Directional>,
    materials: Vec<Handle<Material>>,
    fog: giants_game::lighting::Fog,
    tick: i32,
}
fn parameters(
    scene: &Gun,
    camera: &crate::validation_camera::ValidationCamera,
    animation: &giants_game::animation::Animation,
    tick: i32,
    sub: usize,
    no_fog: bool,
) -> Result<Parameters, Box<dyn std::error::Error>> {
    let gun = scene.gun.as_ref().ok_or("Gun missing")?;
    let (matrix, flags) = gun.matrix(animation, tick)?;
    if flags & 8 != 0 {
        return Err("Gun mirror outside measured profile".into());
    }
    let world = Mat4::from_cols_array(&matrix).transpose();
    Ok(source_parameters(
        &gun.mesh.subobjects[sub].material,
        world,
        camera,
        &scene.lights,
        scene.fog,
        no_fog,
    ))
}
pub(crate) fn source_parameters(
    material: &giants_formats::gbs::Material,
    world: Mat4,
    camera: &crate::validation_camera::ValidationCamera,
    source_lights: &[giants_game::lighting::Directional],
    fog: giants_game::lighting::Fog,
    no_fog: bool,
) -> Parameters {
    let view = Mat4::from_cols_array(&camera.view);
    let rgb = |word: u32| Vec4::from_array(giants_game::scenery::material_color(word));
    let lights = giants_game::lighting::shader_slots(source_lights).map(|source| match source {
        Some(l) => Light {
            direction: Vec3::from_array(l.direction).extend(0.),
            color: Vec3::from_array(l.color).extend(1.),
        },
        None => Light {
            direction: Vec4::ZERO,
            color: Vec4::ZERO,
        },
    });
    Parameters {
        world,
        view,
        clip: crate::source_view::source_clip(camera.clip_from_bevy()),
        normal_eye: (view * world).inverse().transpose(),
        emissive: rgb(material.colors[0]),
        ambient: rgb(material.colors[1]),
        diffuse: rgb(material.colors[2]),
        specular: rgb(material.colors[3]),
        lights,
        fog: Vec4::new(fog.start, fog.end, if no_fog { 0. } else { 1. }, 0.),
        fog_color: Vec4::new(
            fog.color[0] as f32 / 255.,
            fog.color[1] as f32 / 255.,
            fog.color[2] as f32 / 255.,
            1.,
        ),
        pixel_power: Vec4::new(
            1. / camera.viewport.z as f32,
            -1. / camera.viewport.w as f32,
            material.power,
            0.,
        ),
        // Neutral ordinary lit descriptor: absent quantized blend selects a
        // white texture factor, independently of texture alpha or specular.
        composition: Vec4::new(
            1.,
            if !material.texture.is_empty()
                && material.flags & 0x10000000 == 0
                && giants_game::material_descriptor::part_constant(material.blend)
                    .is_ok_and(|(constant, _)| constant == 0)
            {
                1.
            } else {
                0.
            },
            1.,
            1.,
        ),
    }
}
impl Scene {
    pub fn owned_slots(&self, director: usize) -> Vec<usize> {
        self.entry(director)
            .and_then(|s| s.gun.as_ref())
            .filter(|a| !a.mesh.subobjects.is_empty())
            .map(|a| vec![a.slot])
            .unwrap_or_default()
    }
    pub fn load(
        root: &std::path::Path,
        cinematic: &crate::cinematic_camera::Cinematic,
        slot: Option<usize>,
        config: &giants_formats::world::WorldConfig,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Self::prepare(cinematic, |segment| Gun::load(root, segment, slot, config))
    }
}
#[derive(Component)]
pub struct Rendered;
impl Gun {
    fn load(
        root: &std::path::Path,
        cinematic: &crate::cinematic_camera::Segment,
        slot: Option<usize>,
        config: &giants_formats::world::WorldConfig,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let source_lights = giants_game::lighting::first_world(config)?;
        let fog = config.fog;
        let gun = if slot.is_none_or(|s| s == 7) {
            (if cinematic.preview() {
                giants_game::attachments::load_sequence_preview
            } else {
                giants_game::attachments::load_sequence
            })(root, cinematic.sequence())?
            .into_iter()
            .find(|a| a.slot == 7 && a.model.eq_ignore_ascii_case("mc_gun_l0"))
        } else {
            None
        };
        let (textures, normals, lights) = if let Some(gun) = &gun {
            if gun
                .mesh
                .subobjects
                .iter()
                .any(|s| s.material.flags != 0xc0000010)
            {
                return Err("Gun material outside measured profile".into());
            }
            (
                gun.textures(root)?,
                gun.mesh.render_normals()?,
                source_lights,
            )
        } else {
            (Vec::new(), Vec::new(), Vec::new())
        };
        println!(
            "cinematic_gun={} gun_lighting_source=world_objects lighting_space=eye gun_visibility=source_projection flk_visibility_events=shared_actor_slot",
            gun.is_some()
        );
        Ok(Self {
            gun,
            textures,
            normals,
            lights,
            materials: Vec::new(),
            fog,
            tick: cinematic.tick(),
        })
    }
}
pub fn setup(
    mut commands: Commands,
    mut scene: ResMut<Scene>,
    world: Res<crate::world_scene::Scene>,
    cinematic: Res<crate::cinematic_camera::Cinematic>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<Material>>,
) {
    let Some(gun) = &scene.gun else {
        return;
    };
    let camera = world
        .validation_camera
        .as_ref()
        .expect("gun requires cinematic camera");
    let mut textures = std::collections::BTreeMap::new();
    for texture in &scene.textures {
        let mut image = Image::new(
            bevy::render::render_resource::Extent3d {
                width: texture.width,
                height: texture.height,
                depth_or_array_layers: 1,
            },
            bevy::render::render_resource::TextureDimension::D2,
            texture.rgba.clone(),
            bevy::render::render_resource::TextureFormat::Rgba8Unorm,
            RenderAssetUsages::default(),
        );
        image.sampler =
            bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
                address_mode_u: bevy::image::ImageAddressMode::Repeat,
                address_mode_v: bevy::image::ImageAddressMode::Repeat,
                mag_filter: bevy::image::ImageFilterMode::Linear,
                min_filter: bevy::image::ImageFilterMode::Linear,
                mipmap_filter: bevy::image::ImageFilterMode::Linear,
                anisotropy_clamp: 16,
                ..default()
            });
        textures.insert(texture.name.to_ascii_lowercase(), images.add(image));
    }
    let mut handles = Vec::new();
    for (index, sub) in gun.mesh.subobjects.iter().enumerate() {
        let indices: Vec<_> = sub
            .triangles
            .iter()
            .flatten()
            .map(|i| usize::from(*i))
            .collect();
        let positions: Vec<_> = indices
            .iter()
            .map(|i| gun.mesh.positions[gun.mesh.indices[*i] as usize])
            .collect();
        let normals: Vec<_> = indices.iter().map(|i| scene.normals[*i]).collect();
        let uv: Vec<_> = indices.iter().map(|i| gun.mesh.uv[*i]).collect();
        let mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv);
        let material = materials.add(Material {
            texture: textures[&sub.material.texture.to_ascii_lowercase()].clone(),
            parameters: parameters(
                &scene,
                camera,
                cinematic.animation(),
                cinematic.tick(),
                index,
                world.no_fog,
            )
            .expect("validated gun parameters"),
        });
        handles.push(material.clone());
        commands.spawn((
            Rendered,
            crate::cinematic_visibility::ActorSlot(7),
            Name::new(format!("FLK gun slot 7 / {index}")),
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(material),
            bevy::camera::visibility::NoFrustumCulling,
        ));
    }
    scene.materials = handles;
    info!(
        "cinematic_render_ready=gun_scene director={} meshes={}",
        cinematic.director_index(),
        scene.materials.len()
    );
}
pub fn update(
    cinematic: Res<crate::cinematic_camera::Cinematic>,
    world: Res<crate::world_scene::Scene>,
    mut scene: ResMut<Scene>,
    mut materials: ResMut<Assets<Material>>,
    mut exit: MessageWriter<AppExit>,
    rendered: Query<Entity, With<Rendered>>,
) {
    if rendered.iter().count() != scene.materials.len() {
        error!("Director render entity count differs: gun_scene");
        exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
        return;
    }

    if scene.tick == cinematic.tick() && !world.is_changed() || scene.gun.is_none() {
        return;
    }
    if let Some(camera) = &world.validation_camera {
        for (index, handle) in scene.materials.iter().enumerate() {
            match parameters(
                &scene,
                camera,
                cinematic.animation(),
                cinematic.tick(),
                index,
                world.no_fog,
            ) {
                Ok(p) => {
                    if let Some(mut m) = materials.get_mut(handle) {
                        m.parameters = p;
                    }
                }
                Err(e) => {
                    error!("Gun update failed: {}", e);
                    exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
                    return;
                }
            }
        }
    }
    scene.tick = cinematic.tick();
}

pub fn switch(
    mut commands: Commands,
    cinematic: Res<crate::cinematic_camera::Cinematic>,
    mut scene: ResMut<Scene>,
    rendered: Query<Entity, With<Rendered>>,
    mut exit: MessageWriter<AppExit>,
) {
    match scene.select(cinematic.director_index()) {
        Ok(false) => return,
        Ok(true) => {}
        Err(error) => {
            error!("Director scene selection failed: {}", error);
            exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
            return;
        }
    }
    for entity in &rendered {
        commands.entity(entity).despawn();
    }

    scene.materials.clear();

    scene.tick = i32::MIN;
    info!(
        "cinematic_scene_switch=gun_scene director={} sequence={}",
        cinematic.director_index(),
        cinematic.sequence()
    );
    commands.run_system_cached(setup);
}

#[cfg(test)]
mod uniform_tests {
    use super::*;
    #[test]
    fn source_lighting_buffer_preserves_all_uniform_words_and_updates_in_place() {
        let matrix =
            |start: u32| Mat4::from_cols_array(&std::array::from_fn(|i| (start + i as u32) as f32));
        let vector =
            |start: u32| Vec4::from_array(std::array::from_fn(|i| (start + i as u32) as f32));
        let parameters = Parameters {
            world: matrix(1),
            view: matrix(17),
            clip: matrix(33),
            normal_eye: matrix(49),
            emissive: vector(65),
            ambient: vector(69),
            diffuse: vector(73),
            specular: vector(77),
            lights: std::array::from_fn(|i| Light {
                direction: vector(81 + 8 * i as u32),
                color: vector(85 + 8 * i as u32),
            }),
            fog: vector(105),
            fog_color: vector(109),
            pixel_power: vector(113),
            composition: vector(117),
        };
        // Independently specified field order: four matrices, four material
        // vectors, three directional/color pairs and three final vectors.
        let expected: Vec<_> = (1..=120).flat_map(|i| (i as f32).to_le_bytes()).collect();
        let mut buffer = parameters.buffer();
        assert_eq!(buffer.cast_slice::<u8>().unwrap(), expected);
        assert_eq!(buffer.buffer_size(), 480);
        let suppressed = parameters.clone().suppress_primary_diffuse(true);
        assert_eq!(
            suppressed.lights[0].color.truncate(),
            parameters.lights[0].color.truncate()
        );
        assert_eq!(suppressed.lights[0].color.w, 0.);
        assert_eq!(suppressed.lights[1].color, parameters.lights[1].color);
        assert_eq!(suppressed.lights[2].color, parameters.lights[2].color);
        assert!(
            buffer
                .buffer_usage
                .contains(bevy::render::render_resource::BufferUsages::UNIFORM)
        );
        buffer.clear();
        buffer.extend_from_slice(&[0_u8; 480]);
        parameters.write_buffer(&mut buffer);
        assert_eq!(buffer.cast_slice::<u8>().unwrap(), expected);
        assert_eq!(buffer.buffer_size(), 480);
    }
}
