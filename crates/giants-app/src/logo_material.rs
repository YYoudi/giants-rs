//! Measured first texture stage plus vertex lighting. Reflection environment remains absent.
use bevy::{
    image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    mesh::MeshVertexBufferLayoutRef,
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    reflect::TypePath,
    render::render_resource::{
        AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
    },
    shader::ShaderRef,
};

#[derive(Clone, Debug, ShaderType)]
pub struct DirectionalLight {
    pub direction: Vec4,
    pub diffuse: Vec4,
    pub specular: Vec4,
    pub ambient: Vec4,
}
#[derive(Clone, Debug, ShaderType)]
pub struct Parameters {
    pub emissive: Vec4,
    pub ambient: Vec4,
    pub diffuse: Vec4,
    pub specular: Vec4,
    pub camera: Vec4,
    pub power: Vec4,
    pub light0: DirectionalLight,
    pub light1: DirectionalLight,
}
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct LogoMaterial {
    #[uniform(0)]
    pub parameters: Parameters,
    #[texture(1)]
    #[sampler(2)]
    pub texture: Handle<Image>,
}
impl Material for LogoMaterial {
    fn vertex_shader() -> ShaderRef {
        "embedded://giants_app/logo.wesl".into()
    }
    fn fragment_shader() -> ShaderRef {
        "embedded://giants_app/logo.wesl".into()
    }
    fn specialize(
        _: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.vertex.buffers = vec![layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(2),
        ])?];
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}
pub fn parameters(colors: [u32; 4], power: f32) -> Parameters {
    fn rgb(word: u32) -> Vec4 {
        Vec4::new(
            (word & 255) as f32 / 255.,
            ((word >> 8) & 255) as f32 / 255.,
            ((word >> 16) & 255) as f32 / 255.,
            1.,
        )
    }
    // Inspection lights chosen by this project; these are NOT claimed as original menu lights.
    let light0 = DirectionalLight {
        direction: Vec4::new(0.3, -0.4, -1., 0.),
        diffuse: Vec4::splat(0.7),
        specular: Vec4::splat(0.7),
        ambient: Vec4::splat(0.15),
    };
    let light1 = DirectionalLight {
        direction: Vec4::new(-0.8, 0.3, 0.5, 0.),
        diffuse: Vec4::splat(0.3),
        specular: Vec4::splat(0.3),
        ambient: Vec4::ZERO,
    };
    Parameters {
        emissive: rgb(colors[0]),
        ambient: rgb(colors[1]),
        diffuse: rgb(colors[2]),
        specular: rgb(colors[3]),
        camera: Vec4::new(6.7, 0., 480., 1.),
        power: Vec4::new(power, 0., 0., 0.),
        light0,
        light1,
    }
}
pub fn sampler() -> ImageSampler {
    ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        ..default()
    })
}
pub fn update_camera(
    cameras: Query<&Transform, With<crate::mesh_scene::FlyCamera>>,
    mut materials: ResMut<Assets<LogoMaterial>>,
) {
    if let Ok(camera) = cameras.single() {
        for (_, material) in materials.iter_mut() {
            material.parameters.camera = camera.translation.extend(1.);
        }
    }
}
