//! Flat source-geometry silhouette pass. Allocation and actor admission are caller-owned.
use bevy::{
    asset::RenderAssetUsages,
    camera::{RenderTarget, Viewport},
    mesh::MeshVertexBufferLayoutRef,
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    reflect::TypePath,
    render::render_resource::{
        AsBindGroup, Face, PrimitiveTopology, RenderPipelineDescriptor,
        SpecializedMeshPipelineError, TextureFormat,
    },
    shader::ShaderRef,
};

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct Material {
    #[uniform(0)]
    pub view: Mat4,
    #[uniform(1)]
    pub projection: Mat4,
    #[uniform(2)]
    pub pixel: Vec4,
    #[uniform(3)]
    pub world: Mat4,
}
impl bevy::pbr::Material for Material {
    fn vertex_shader() -> ShaderRef {
        concat!(
            "embedded://",
            env!("CARGO_CRATE_NAME"),
            "/shadow_silhouette.wesl"
        )
        .into()
    }
    fn fragment_shader() -> ShaderRef {
        Self::vertex_shader()
    }
    fn specialize(
        _: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.vertex.buffers = vec![
            layout
                .0
                .get_layout(&[Mesh::ATTRIBUTE_POSITION.at_shader_location(0)])?,
        ];
        descriptor.primitive.cull_mode = Some(Face::Back);
        if let Some(depth) = &mut descriptor.depth_stencil {
            depth.depth_write_enabled = Some(false);
            depth.depth_compare = Some(bevy::render::render_resource::CompareFunction::Always);
        }
        Ok(())
    }
}

pub fn install(app: &mut App) {
    bevy::asset::embedded_asset!(app, "shadow_silhouette.wesl");
    app.add_plugins(MaterialPlugin::<Material>::default());
}

#[derive(Clone)]
pub struct Request {
    pub camera: giants_game::cinematic_shadow::SilhouetteCamera,
    pub extent: [u32; 2],
    pub positions: Vec<[f32; 3]>,
    pub layer: usize,
    pub source_world: Mat4,
}
pub struct Pass {
    pub image: Handle<Image>,
    pub entities: [Entity; 2],
    pub mesh: Handle<Mesh>,
    pub material: Handle<Material>,
    extent: [u32; 2],
}

pub fn update(
    pass: &Pass,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<Material>,
    sample: &Request,
) -> Result<(), &'static str> {
    if pass.extent != sample.extent {
        return Err("shadow extent changed; replace the owned pass");
    }
    if sample.positions.is_empty()
        || !sample.positions.len().is_multiple_of(3)
        || sample.positions.len() > 900_000
        || sample.positions.iter().flatten().any(|v| !v.is_finite())
        || !sample.source_world.is_finite()
    {
        return Err("invalid updated silhouette geometry");
    }
    let mut material = materials
        .get_mut(&pass.material)
        .ok_or("shadow material retired")?;
    let mut mesh = meshes.get_mut(&pass.mesh).ok_or("shadow mesh retired")?;
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, sample.positions.clone());
    material.world = sample.source_world;
    material.view = Mat4::from_cols_array(&sample.camera.renderer_view);
    material.projection = Mat4::from_cols_array(&sample.camera.projection);
    Ok(())
}

/// Input vertices are posed native world coordinates, not Bevy-axis positions.
pub fn spawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<Material>,
    images: &mut Assets<Image>,
    request: Request,
) -> Result<Pass, &'static str> {
    let Request {
        camera,
        extent,
        positions,
        layer,
        source_world,
    } = request;
    if !(1..=64).contains(&layer) {
        return Err("silhouette requires an isolated render layer");
    }
    if positions.is_empty()
        || !source_world.is_finite()
        || extent.iter().any(|&v| v <= 2 || v > 4096)
        || !positions.len().is_multiple_of(3)
        || positions.len() > 900_000
        || positions.iter().flatten().any(|v| !v.is_finite())
        || camera.viewport
            != [
                1,
                1,
                extent[0].saturating_sub(2),
                extent[1].saturating_sub(2),
            ]
    {
        return Err("invalid silhouette geometry or surface extent");
    }
    let target = images.add(Image::new_target_texture(
        extent[0],
        extent[1],
        TextureFormat::Rgba8UnormSrgb,
        None,
    ));
    let mesh = meshes.add(
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions),
    );
    let material = materials.add(Material {
        world: source_world,
        view: Mat4::from_cols_array(&camera.renderer_view),
        projection: Mat4::from_cols_array(&camera.projection),
        pixel: Vec4::new(
            1. / camera.viewport[2] as f32,
            -1. / camera.viewport[3] as f32,
            0.,
            0.,
        ),
    });
    let layers = bevy::camera::visibility::RenderLayers::layer(layer);
    let body = commands
        .spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            bevy::camera::visibility::NoFrustumCulling,
            layers.clone(),
        ))
        .id();
    let view = commands
        .spawn((
            Camera3d::default(),
            Camera {
                order: -200 + layer as isize,
                clear_color: ClearColorConfig::Custom(Color::BLACK),
                viewport: Some(Viewport {
                    physical_position: UVec2::ONE,
                    physical_size: UVec2::new(camera.viewport[2], camera.viewport[3]),
                    ..default()
                }),
                ..default()
            },
            RenderTarget::Image(target.clone().into()),
            Msaa::Off,
            bevy::core_pipeline::tonemapping::Tonemapping::None,
            bevy::core_pipeline::tonemapping::DebandDither::Disabled,
            layers,
        ))
        .id();
    Ok(Pass {
        image: target,
        entities: [body, view],
        mesh,
        material,
        extent,
    })
}
