//! Opaque solar disk: source model/texture, measured camera-facing basis.
use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::{
        render_resource::{Extent3d, PrimitiveTopology, TextureDimension, TextureFormat},
        storage::ShaderBuffer,
    },
};
use giants_game::frontend_background::Background;

pub fn matrix(source: &Background, view: [f32; 16]) -> Mat4 {
    let source_to_bevy = Mat4::from_cols(Vec4::X, -Vec4::Z, Vec4::Y, Vec4::W);
    let bevy_to_source = source_to_bevy.inverse();
    source_to_bevy
        * Mat4::from_cols_array(&source.sun.matrix(view, source.world.sun))
        * bevy_to_source
}
pub fn spawn(
    world: &mut World,
    source: &Background,
    camera: &crate::validation_camera::ValidationCamera,
    view: Handle<ShaderBuffer>,
) -> Handle<ShaderBuffer> {
    let sun = &source.sun;
    let mut image = Image::new(
        Extent3d {
            width: sun.texture.width,
            height: sun.texture.height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        sun.texture
            .pixels
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|c| [c[0], c[1], c[2], 255])
            .collect(),
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::default(),
    );
    image.sampler = crate::logo_material::sampler();
    let texture = world.resource_mut::<Assets<Image>>().add(image);
    let indices: Vec<_> = sun.mesh.subobjects[0]
        .triangles
        .iter()
        .flatten()
        .map(|i| usize::from(*i))
        .collect();
    let positions: Vec<_> = indices
        .iter()
        .map(|i| {
            let p = sun.mesh.positions[usize::from(sun.mesh.indices[*i])];
            [p[0], p[2], -p[1]]
        })
        .collect();
    let uv: Vec<_> = indices.iter().map(|i| sun.mesh.uv[*i]).collect();
    // Native stage 0 selects TEXTURE; stored zero vertex RGB is not lighting.
    let geometry = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, vec![[0.; 4]; indices.len()]);
    let mesh = world.resource_mut::<Assets<Mesh>>().add(geometry);
    let matrices = world
        .resource_mut::<Assets<ShaderBuffer>>()
        .add(ShaderBuffer::from(vec![
            matrix(source, camera.view).to_cols_array(),
        ]));
    let material = world
        .resource_mut::<Assets<crate::prop_scene::Material>>()
        .add(crate::prop_scene::Material {
            instanced: true,
            instance_matrices: matrices.clone(),
            multiply: false,
            alpha: false,
            texture: Some(texture),
            view,
            world_from_bevy: Mat4::IDENTITY,
            pixel_offset: Vec4::new(
                1. / camera.viewport.z as f32,
                -1. / camera.viewport.w as f32,
                0.,
                -1.,
            ),
            fog: Vec4::ZERO,
            fog_color: Vec4::ZERO,
        });
    world.spawn((
        super::frontend_background::Owned,
        Name::new("Frontend opaque sun"),
        Mesh3d(mesh),
        MeshMaterial3d(material),
        bevy::mesh::MeshTag::new(0),
        bevy::camera::visibility::NoFrustumCulling,
    ));
    matrices
}
