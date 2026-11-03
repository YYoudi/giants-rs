//! Source-attached Reaper spears, in the measured unlit texture profile.
use bevy::{
    asset::RenderAssetUsages,
    mesh::MeshVertexBufferLayoutRef,
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    reflect::TypePath,
    render::render_resource::{
        AsBindGroup, PrimitiveTopology, RenderPipelineDescriptor, SpecializedMeshPipelineError,
    },
    shader::ShaderRef,
};
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct Material {
    #[texture(0)]
    #[sampler(1)]
    texture: Handle<Image>,
    #[uniform(2)]
    clip_world: Mat4,
    #[uniform(3)]
    pixel_fog: Vec4,
    #[uniform(4)]
    fog_color: Vec4,
}
impl bevy::pbr::Material for Material {
    fn vertex_shader() -> ShaderRef {
        "embedded://giants_app/spear.wesl".into()
    }
    fn fragment_shader() -> ShaderRef {
        Self::vertex_shader()
    }
    fn specialize(
        _: &MaterialPipeline,
        d: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        d.vertex.buffers = vec![layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(1),
        ])?];
        d.primitive.cull_mode = Some(bevy::render::render_resource::Face::Back);
        Ok(())
    }
}
struct Entry {
    attachment: giants_game::attachments::Attachment,
    textures: Vec<giants_game::LogoTexture>,
    materials: Vec<Handle<Material>>,
}
pub struct Spears {
    entries: Vec<Entry>,
    fog: giants_game::lighting::Fog,
    tick: i32,
}
pub type Scene = crate::cinematic_scene::Scenes<Spears>;
#[derive(Component)]
pub struct Rendered;
impl Scene {
    pub fn owned_slots(&self, director: usize) -> Vec<usize> {
        self.entry(director)
            .into_iter()
            .flat_map(|s| &s.entries)
            .filter(|e| !e.attachment.mesh.subobjects.is_empty())
            .map(|e| e.attachment.slot)
            .collect()
    }
    pub fn load(
        root: &std::path::Path,
        cinematic: &crate::cinematic_camera::Cinematic,
        slot: Option<usize>,
        config: &giants_formats::world::WorldConfig,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Self::prepare(cinematic, |segment| {
            let mut entries = Vec::new();
            for attachment in (if segment.preview() {
                giants_game::attachments::load_sequence_preview
            } else {
                giants_game::attachments::load_sequence
            })(root, segment.sequence())?
            {
                if !attachment.model.eq_ignore_ascii_case("rpman_spear")
                    || slot.is_some_and(|s| s != attachment.slot)
                {
                    continue;
                }
                if attachment
                    .mesh
                    .subobjects
                    .iter()
                    .any(|s| s.material.flags != 0x40000000)
                {
                    return Err("Spear material outside measured profile".into());
                }
                let textures = attachment.textures(root)?;
                entries.push(Entry {
                    attachment,
                    textures,
                    materials: Vec::new(),
                });
            }
            let fog = config.fog;
            println!(
                "cinematic_spears={} spear_material=source_texture spear_fog=source_linear",
                entries.len()
            );
            Ok(Spears {
                entries,
                fog,
                tick: i32::MIN,
            })
        })
    }
}
fn clip(
    entry: &Entry,
    cinematic: &crate::cinematic_camera::Cinematic,
    camera: &crate::validation_camera::ValidationCamera,
) -> Result<Mat4, Box<dyn std::error::Error>> {
    let (matrix, flags) = entry
        .attachment
        .matrix(cinematic.animation(), cinematic.tick())?;
    if flags & 8 != 0 {
        return Err("Spear mirror outside measured profile".into());
    }
    let source_to_bevy = Mat4::from_cols(Vec4::X, -Vec4::Z, Vec4::Y, Vec4::W);
    Ok(camera.clip_from_bevy() * source_to_bevy * Mat4::from_cols_array(&matrix).transpose())
}
pub fn setup(
    mut commands: Commands,
    mut scene: ResMut<Scene>,
    world: Res<crate::world_scene::Scene>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<Material>>,
    cinematic: Res<crate::cinematic_camera::Cinematic>,
) {
    if scene.entries.is_empty() {
        return;
    }
    let camera = world
        .validation_camera
        .as_ref()
        .expect("Spears require cinematic camera");
    let fog = Vec4::new(
        scene.fog.color[0] as f32 / 255.,
        scene.fog.color[1] as f32 / 255.,
        scene.fog.color[2] as f32 / 255.,
        if world.no_fog { 0. } else { 1. },
    );
    let pixel_fog = Vec4::new(
        1. / camera.viewport.z as f32,
        -1. / camera.viewport.w as f32,
        scene.fog.start,
        scene.fog.end,
    );
    let mut count = 0;
    for entry in &mut scene.entries {
        let clip_world = clip(entry, &cinematic, camera).expect("Measured spear attachment matrix");
        let mut textures = std::collections::BTreeMap::new();
        for texture in &entry.textures {
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
                    mipmap_filter: bevy::image::ImageFilterMode::Nearest,
                    anisotropy_clamp: 1,
                    ..default()
                });
            textures.insert(texture.name.to_ascii_lowercase(), images.add(image));
        }
        for sub in &entry.attachment.mesh.subobjects {
            let indices: Vec<_> = sub
                .triangles
                .iter()
                .flatten()
                .map(|i| usize::from(*i))
                .collect();
            let positions: Vec<_> = indices
                .iter()
                .map(|i| {
                    entry.attachment.mesh.positions[entry.attachment.mesh.indices[*i] as usize]
                })
                .collect();
            let uv: Vec<_> = indices
                .iter()
                .map(|i| entry.attachment.mesh.uv[*i])
                .collect();
            let mesh = Mesh::new(
                PrimitiveTopology::TriangleList,
                RenderAssetUsages::default(),
            )
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv);
            let material = materials.add(Material {
                texture: textures[&sub.material.texture.to_ascii_lowercase()].clone(),
                clip_world,
                pixel_fog,
                fog_color: fog,
            });
            entry.materials.push(material.clone());
            count += 1;
            commands.spawn((
                Rendered,
                crate::cinematic_visibility::ActorSlot(entry.attachment.slot),
                Name::new(format!(
                    "FLK spear slot {} / {}",
                    entry.attachment.slot, sub.name
                )),
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(material),
                bevy::camera::visibility::NoFrustumCulling,
            ));
        }
    }
    info!(
        "cinematic_render_ready=spear_scene director={} meshes={}",
        cinematic.director_index(),
        count
    );
}
pub fn update(
    cinematic: Res<crate::cinematic_camera::Cinematic>,
    world: Res<crate::world_scene::Scene>,
    mut scene: ResMut<Scene>,
    mut materials: ResMut<Assets<Material>>,
    rendered: Query<Entity, With<Rendered>>,
    mut exit: MessageWriter<AppExit>,
) {
    if rendered.iter().count()
        != scene
            .entries
            .iter()
            .map(|e| e.materials.len())
            .sum::<usize>()
    {
        error!("Director render entity count differs: spear_scene");
        exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
        return;
    }
    if scene.tick == cinematic.tick() && !world.is_changed() {
        return;
    }
    if let Some(camera) = &world.validation_camera {
        for entry in &scene.entries {
            let matrix = match clip(entry, &cinematic, camera) {
                Ok(m) => m,
                Err(e) => {
                    error!("Spear matrix failed: {}", e);
                    exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
                    return;
                }
            };
            for handle in &entry.materials {
                if let Some(mut m) = materials.get_mut(handle) {
                    m.clip_world = matrix;
                    m.pixel_fog.x = 1. / camera.viewport.z as f32;
                    m.pixel_fog.y = -1. / camera.viewport.w as f32;
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
        Err(e) => {
            error!("Spear selection failed: {}", e);
            exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
            return;
        }
    }
    for entity in &rendered {
        commands.entity(entity).despawn();
    }
    for entry in &mut scene.entries {
        entry.materials.clear();
    }
    scene.tick = i32::MIN;
    info!(
        "cinematic_scene_switch=spear_scene director={} sequence={}",
        cinematic.director_index(),
        cinematic.sequence()
    );
    commands.run_system_cached(setup);
}
