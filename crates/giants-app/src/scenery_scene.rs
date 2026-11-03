//! Mission class1053 static geometry, source lighting and transparent gates.
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Key(bool);
impl From<&Bindings> for Key {
    fn from(value: &Bindings) -> Self {
        Self(value.alpha)
    }
}
pub type Material = crate::source_view::UniformMaterial<Bindings>;
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
#[bind_group_data(Key)]
pub struct Bindings {
    pub(crate) alpha: bool,
    #[texture(0)]
    #[sampler(1)]
    pub(crate) texture: Option<Handle<Image>>,
    #[storage(2, read_only)]
    pub(crate) parameters: Handle<bevy::render::storage::ShaderBuffer>,
}
impl bevy::pbr::Material for Material {
    fn vertex_shader() -> ShaderRef {
        "embedded://giants_app/gun.wesl".into()
    }
    fn fragment_shader() -> ShaderRef {
        "embedded://giants_app/gun.wesl".into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        if self.alpha {
            AlphaMode::Blend
        } else {
            AlphaMode::Opaque
        }
    }
    fn specialize(
        _: &MaterialPipeline,
        d: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        d.vertex.buffers = vec![layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(2),
        ])?];
        d.primitive.cull_mode = Some(bevy::render::render_resource::Face::Back);
        if key.bind_group_data.0
            && let Some(depth) = &mut d.depth_stencil
        {
            depth.depth_write_enabled = Some(false);
        }
        Ok(())
    }
}
#[derive(Resource)]
pub struct Scene {
    source: giants_game::scenery::Scenery,
    lights: Vec<giants_game::lighting::Directional>,
    fog: giants_game::lighting::Fog,
    materials: Vec<Handle<Material>>,
}
impl Scene {
    pub fn source_keys(&self) -> impl Iterator<Item = (bool, usize)> + '_ {
        std::iter::once((false, self.source.source_offset))
    }
    pub fn load(
        _root: &std::path::Path,
        plan: &giants_game::instances::Plan,
        config: &giants_formats::world::WorldConfig,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let source = giants_game::scenery::load_planned(plan)?;
        let lights = giants_game::lighting::first_world(config)?;
        let fog = config.fog;
        println!(
            "cinematic_scenery_class=1053 model={} parts={} placement=source_identity",
            source.model,
            source.mesh.subobjects.len()
        );
        Ok(Self {
            source,
            lights,
            fog,
            materials: Vec::new(),
        })
    }
}
#[derive(Component)]
pub struct Rendered;
pub fn setup(
    mut commands: Commands,
    mut scene: ResMut<Scene>,
    world: Res<crate::world_scene::Scene>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<Material>>,
    mut buffers: ResMut<Assets<bevy::render::storage::ShaderBuffer>>,
) {
    let camera = world
        .validation_camera
        .as_ref()
        .expect("Scenery source camera");
    let mut textures = std::collections::BTreeMap::new();
    for texture in &scene.source.textures {
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
        image.sampler = crate::logo_material::sampler();
        textures.insert(texture.name.to_ascii_lowercase(), images.add(image));
    }
    let mut handles = Vec::new();
    for (index, sub) in scene.source.mesh.subobjects.iter().enumerate() {
        let ids: Vec<_> = sub
            .triangles
            .iter()
            .flatten()
            .map(|v| usize::from(*v))
            .collect();
        let positions: Vec<_> = ids
            .iter()
            .map(|i| scene.source.mesh.positions[scene.source.mesh.indices[*i] as usize])
            .collect();
        let normals: Vec<_> = ids.iter().map(|i| scene.source.normals[*i]).collect();
        let uv: Vec<_> = ids.iter().map(|i| scene.source.mesh.uv[*i]).collect();
        let mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv);
        let material = materials.add(Material::from(Bindings {
            alpha: scene.source.material_passes[index]
                == giants_game::render_parts::Pass::TextureFormat,
            texture: Some(textures[&sub.material.texture.to_ascii_lowercase()].clone()),
            parameters: buffers.add(
                crate::gun_scene::source_parameters(
                    &sub.material,
                    Mat4::IDENTITY,
                    camera,
                    &scene.lights,
                    scene.fog,
                    world.no_fog,
                )
                .white_texture_factor(
                    scene.source.material_passes[index]
                        == giants_game::render_parts::Pass::TextureFormat,
                )
                .buffer(),
            ),
        }));
        handles.push(material.clone());
        commands.spawn((
            Rendered,
            Name::new(format!("Mission scenery1053 / {index}")),
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(material),
            bevy::camera::visibility::NoFrustumCulling,
        ));
    }
    scene.materials = handles;
    info!(
        "cinematic_scenery_ready=1053 meshes={}",
        scene.materials.len()
    );
}
pub fn update(
    world: Res<crate::world_scene::Scene>,
    scene: Res<Scene>,
    materials: Res<Assets<Material>>,
    mut buffers: ResMut<Assets<bevy::render::storage::ShaderBuffer>>,
) {
    let Some(camera) = &world.validation_camera else {
        return;
    };
    for ((sub, pass), handle) in scene
        .source
        .mesh
        .subobjects
        .iter()
        .zip(&scene.source.material_passes)
        .zip(&scene.materials)
    {
        if let Some(material) = materials.get(handle)
            && let Some(mut buffer) = buffers.get_mut(&material.parameters)
        {
            crate::gun_scene::source_parameters(
                &sub.material,
                Mat4::IDENTITY,
                camera,
                &scene.lights,
                scene.fog,
                world.no_fog,
            )
            .white_texture_factor(*pass == giants_game::render_parts::Pass::TextureFormat)
            .write_buffer(&mut buffer);
        }
    }
}
