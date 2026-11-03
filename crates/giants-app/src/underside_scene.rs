//! Native textured seabed, with separate retained background controls.
use bevy::{
    asset::RenderAssetUsages,
    camera::visibility::RenderLayers,
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
pub struct MaterialKey(bool, bool);
impl From<&UndersideMaterial> for MaterialKey {
    fn from(material: &UndersideMaterial) -> Self {
        Self(material.validation.w > 0., material.appearance.y > 0.)
    }
}
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
#[bind_group_data(MaterialKey)]
pub struct UndersideMaterial {
    #[uniform(0)]
    pub clip_from_bevy: Mat4,
    #[uniform(1)]
    pub validation: Vec4,
    #[uniform(2)]
    pub fog: Vec4,
    #[uniform(3)]
    pub fog_color: Vec4,
    #[texture(4)]
    #[sampler(5)]
    pub texture: Handle<Image>,
    #[uniform(6)]
    pub appearance: Vec4,
}
impl Material for UndersideMaterial {
    fn vertex_shader() -> ShaderRef {
        "embedded://giants_app/underside.wesl".into()
    }
    fn fragment_shader() -> ShaderRef {
        "embedded://giants_app/underside.wesl".into()
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
        descriptor.primitive.cull_mode = if key.bind_group_data.0 || key.bind_group_data.1 {
            Some(bevy::render::render_resource::Face::Back)
        } else {
            None
        };
        if let Some(depth) = &mut descriptor.depth_stencil {
            depth.depth_write_enabled = Some(key.bind_group_data.1);
            if !key.bind_group_data.1 {
                depth.depth_compare = Some(bevy::render::render_resource::CompareFunction::Always);
            }
        }
        Ok(())
    }
}
#[derive(Component)]
pub struct UndersideCamera;
pub struct BackgroundView {
    pub pose: Transform,
    pub projection: Projection,
    pub viewport: Option<bevy::camera::Viewport>,
    pub target: Handle<Image>,
}
pub fn spawn(
    commands: &mut Commands,
    scene: &crate::world_scene::Scene,
    view: BackgroundView,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<UndersideMaterial>,
    images: &mut Assets<Image>,
) {
    let BackgroundView {
        pose,
        projection,
        viewport,
        target: world_target,
    } = view;
    let asset = &scene.asset;
    // The explicit oracle-camera path retains the old untextured quality-2
    // control. Ordinary worlds and source cinematics use the observed debug
    // textured/depth-writing surface. Reflection cubes keep their own pass.
    let background = background_pass(scene);
    let textured = scene.reflection_profile || !background;
    let mut positions = Vec::new();
    let mut uv = Vec::new();
    for triangle in &asset.underside.mesh.triangles {
        for i in triangle {
            let p = asset.underside.mesh.positions[*i as usize];
            positions.push([p[0], p[2], -p[1]]);
            uv.push(asset.underside.mesh.uv[*i as usize]);
        }
    }
    let mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    let words = giants_formats::pixels::pack_rgb565(&asset.underside.image.pixels)
        .expect("Validated underside image");
    let mut image = Image::new(
        bevy::render::render_resource::Extent3d {
            width: asset.underside.image.width,
            height: asset.underside.image.height,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        giants_formats::pixels::rgba8(&words),
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
    let texture = images.add(image);
    let v = scene.validation_camera.as_ref();
    let fog = asset.config.water_fog;
    let mut entity = commands.spawn((
        crate::world_scene::Rendered,
        Name::new("GB2 underside"),
        RenderLayers::layer(if background { 1 } else { 0 }),
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(materials.add(UndersideMaterial {
            texture,
            appearance: Vec4::new(f32::from(textured), f32::from(!background), 0., 0.),
            clip_from_bevy: v.map_or(Mat4::IDENTITY, |v| v.clip_from_bevy()),
            validation: v.map_or(Vec4::ZERO, |v| {
                Vec4::new(
                    1.,
                    1. / v.viewport.z as f32,
                    -1. / v.viewport.w as f32,
                    if scene.reflection_profile { 1. } else { 0. },
                )
            }),
            fog: Vec4::new(fog.start, fog.end, if scene.no_fog { 0. } else { 1. }, 0.),
            fog_color: Vec4::new(
                fog.color[0] as f32 / 255.,
                fog.color[1] as f32 / 255.,
                fog.color[2] as f32 / 255.,
                1.,
            ),
        })),
    ));
    if v.is_some() {
        entity.insert(bevy::camera::visibility::NoFrustumCulling);
    }
    info!(
        "underside_profile={} depth_write={} separate_background_pass={}",
        if textured {
            "textured"
        } else {
            "legacy_zero_factor_control"
        },
        !background,
        background
    );
    if !background {
        return;
    }
    commands.spawn((
        crate::world_scene::Rendered,
        Name::new("Underside background camera"),
        UndersideCamera,
        RenderLayers::layer(1),
        Camera3d::default(),
        bevy::camera::RenderTarget::Image(world_target.into()),
        Camera {
            order: -2,
            viewport,
            ..default()
        },
        projection,
        pose,
        Msaa::Off,
        bevy::core_pipeline::tonemapping::Tonemapping::None,
        bevy::core_pipeline::tonemapping::DebandDither::Disabled,
    ));
}
pub fn background_pass(scene: &crate::world_scene::Scene) -> bool {
    scene.reflection_profile || scene.camera_source == "oracle"
}
pub fn sync_camera(
    source: Query<&Transform, (With<crate::mesh_scene::FlyCamera>, Without<UndersideCamera>)>,
    mut background: Query<&mut Transform, With<UndersideCamera>>,
) {
    if let Ok(pose) = source.single() {
        for mut transform in &mut background {
            *transform = *pose;
        }
    }
}
