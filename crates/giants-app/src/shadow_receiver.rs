//! Flick silhouette on prepared static terrain. Default class blobs are separate.
use bevy::{
    asset::RenderAssetUsages,
    mesh::MeshVertexBufferLayoutRef,
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    reflect::TypePath,
    render::{render_resource::*, storage::ShaderBuffer},
    shader::ShaderRef,
};

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct Bindings {
    #[texture(0)]
    #[sampler(1)]
    pub texture: Handle<Image>,
    #[storage(2, read_only)]
    pub clip: Handle<ShaderBuffer>,
    #[uniform(3)]
    pub pixel_strength: Vec4,
    #[uniform(4)]
    pub fog: Vec4,
    #[uniform(5)]
    pub fog_color: Vec4,
}
pub type Material = crate::source_view::UniformMaterial<Bindings>;
impl bevy::pbr::Material for Material {
    fn vertex_shader() -> ShaderRef {
        "embedded://giants_app/shadow_receiver.wesl".into()
    }
    fn fragment_shader() -> ShaderRef {
        Self::vertex_shader()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
    fn specialize(
        _: &MaterialPipeline,
        d: &mut RenderPipelineDescriptor,
        l: &MeshVertexBufferLayoutRef,
        _: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        d.vertex.buffers = vec![l.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(1),
        ])?];
        d.primitive.cull_mode = Some(Face::Back);
        if let Some(depth) = &mut d.depth_stencil {
            depth.depth_write_enabled = Some(false);
            depth.depth_compare = Some(CompareFunction::GreaterEqual);
            depth.bias = DepthBiasState {
                constant: 2,
                slope_scale: 0.,
                clamp: 0.,
            };
        }
        if let Some(fragment) = &mut d.fragment {
            for target in fragment.targets.iter_mut().flatten() {
                target.blend = Some(BlendState {
                    color: BlendComponent {
                        src_factor: BlendFactor::Zero,
                        dst_factor: BlendFactor::OneMinusSrc,
                        operation: BlendOperation::Add,
                    },
                    alpha: BlendComponent {
                        src_factor: BlendFactor::Zero,
                        dst_factor: BlendFactor::One,
                        operation: BlendOperation::Add,
                    },
                });
            }
        }
        Ok(())
    }
}
pub struct Pass {
    pub entity: Entity,
    pub mesh: Handle<Mesh>,
    pub material: Handle<Material>,
}
pub fn install(app: &mut App) {
    bevy::asset::embedded_asset!(app, "shadow_receiver.wesl");
    app.add_plugins(MaterialPlugin::<Material>::default());
}
fn attributes(mesh: &mut Mesh, geometry: &giants_game::cinematic_shadow_terrain::Geometry) {
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        geometry
            .vertices
            .iter()
            .map(|v| [v.position[0], v.position[2], -v.position[1]])
            .collect::<Vec<_>>(),
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_0,
        geometry.vertices.iter().map(|v| v.uv).collect::<Vec<_>>(),
    );
}
pub fn spawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<Material>,
    geometry: &giants_game::cinematic_shadow_terrain::Geometry,
    bindings: Bindings,
) -> Pass {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    attributes(&mut mesh, geometry);
    let mesh = meshes.add(mesh);
    let material = materials.add(Material::from(bindings));
    let entity = commands
        .spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::default(),
            bevy::camera::visibility::NoFrustumCulling,
        ))
        .id();
    Pass {
        entity,
        mesh,
        material,
    }
}
pub fn update(
    pass: &Pass,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<Material>,
    geometry: &giants_game::cinematic_shadow_terrain::Geometry,
    bindings: Bindings,
) -> Result<(), &'static str> {
    let mut mesh = meshes
        .get_mut(&pass.mesh)
        .ok_or("shadow receiver mesh retired")?;
    let mut material = materials
        .get_mut(&pass.material)
        .ok_or("shadow receiver material retired")?;
    attributes(&mut mesh, geometry);
    **material = bindings;
    Ok(())
}
