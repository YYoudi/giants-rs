//! GB2 sky with encoded RGB sampling; no captured geometry is loaded here.
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
pub struct MaterialKey(bool);
impl From<&SkyMaterial> for MaterialKey {
    fn from(material: &SkyMaterial) -> Self {
        Self(material.validation.w > 0.)
    }
}
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
#[bind_group_data(MaterialKey)]
pub struct SkyMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub texture: Handle<Image>,
    #[uniform(2)]
    pub clip_from_bevy: Mat4,
    #[uniform(3)]
    pub validation: Vec4,
}
impl Material for SkyMaterial {
    fn vertex_shader() -> ShaderRef {
        "embedded://giants_app/sky.wesl".into()
    }
    fn fragment_shader() -> ShaderRef {
        "embedded://giants_app/sky.wesl".into()
    }
    fn specialize(
        _: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.vertex.buffers = vec![layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(1),
        ])?];
        descriptor.primitive.cull_mode = if key.bind_group_data.0 {
            Some(bevy::render::render_resource::Face::Back)
        } else {
            None
        };
        if let Some(depth) = &mut descriptor.depth_stencil {
            depth.depth_write_enabled = Some(false);
            if key.bind_group_data.0 {
                depth.depth_compare = Some(bevy::render::render_resource::CompareFunction::Always);
            }
        }
        Ok(())
    }
}
pub fn clip_from_camera(
    camera: &crate::validation_camera::ValidationCamera,
    reflection: bool,
) -> Mat4 {
    if reflection {
        crate::validation_camera::ValidationCamera {
            view: camera.view,
            projection: giants_game::reflection::story4_sky_projection(),
            viewport: camera.viewport,
        }
        .clip_from_bevy()
    } else {
        camera.clip_from_bevy()
    }
}
pub fn spawn(
    commands: &mut Commands,
    scene: &crate::world_scene::Scene,
    meshes: &mut Assets<Mesh>,
    images: &mut Assets<Image>,
    materials: &mut Assets<SkyMaterial>,
) {
    let sky = &scene.asset.sky;
    let mut positions = Vec::new();
    let mut uv = Vec::new();
    for triangle in &sky.mesh.triangles {
        for i in triangle {
            let p = sky.mesh.positions[*i as usize];
            positions.push([p[0], p[2], -p[1]]);
            uv.push(sky.mesh.uv[*i as usize]);
        }
    }
    let mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    let mut image = Image::new(
        bevy::render::render_resource::Extent3d {
            width: sky.image.width,
            height: sky.image.height,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        sky.image
            .pixels
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        bevy::render::render_resource::TextureFormat::Rgba8Unorm,
        RenderAssetUsages::default(),
    );
    image.sampler = bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
        address_mode_u: bevy::image::ImageAddressMode::Repeat,
        address_mode_v: bevy::image::ImageAddressMode::Repeat,
        mag_filter: bevy::image::ImageFilterMode::Linear,
        min_filter: bevy::image::ImageFilterMode::Linear,
        ..default()
    });
    let validation = scene.validation_camera.as_ref();
    let mut entity = commands.spawn((
        crate::world_scene::Rendered,
        Name::new("GB2 sky"),
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(materials.add(SkyMaterial {
            texture: images.add(image),
            clip_from_bevy: validation.map_or(Mat4::IDENTITY, |v| {
                clip_from_camera(v, scene.reflection_profile)
            }),
            validation: validation.map_or(Vec4::ZERO, |v| {
                Vec4::new(
                    1.,
                    1. / v.viewport.z as f32,
                    -1. / v.viewport.w as f32,
                    if scene.reflection_profile { 1. } else { 0. },
                )
            }),
        })),
    ));
    if scene.reflection_profile {
        entity.insert(bevy::camera::visibility::RenderLayers::layer(2));
    }
    if validation.is_some() {
        entity.insert(bevy::camera::visibility::NoFrustumCulling);
    }
}
