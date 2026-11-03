//! Terrain material preview: file configuration, planar projections, inspection camera.
use crate::mesh_scene::FlyCamera;
use bevy::{
    asset::RenderAssetUsages,
    mesh::{MeshVertexAttribute, MeshVertexBufferLayoutRef},
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    reflect::TypePath,
    render::render_resource::{
        AsBindGroup, PrimitiveTopology, RenderPipelineDescriptor, SpecializedMeshPipelineError,
        VertexFormat,
    },
    shader::ShaderRef,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MaterialKey(bool);
impl From<&WorldBindings> for MaterialKey {
    fn from(material: &WorldBindings) -> Self {
        Self(material.reflection_profile)
    }
}
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
#[bind_group_data(MaterialKey)]
pub struct WorldBindings {
    #[uniform(0)]
    pub tint: Vec4,
    #[texture(1)]
    #[sampler(2)]
    pub texture: Handle<Image>,
    #[storage(3, read_only)]
    pub clip_from_bevy: Handle<bevy::render::storage::ShaderBuffer>,
    #[uniform(4)]
    pub validation: Vec4,
    #[storage(5, read_only)]
    pub uv_from_bevy: Handle<bevy::render::storage::ShaderBuffer>,
    #[uniform(6)]
    pub fog_parameters: Vec4,
    #[uniform(7)]
    pub fog_color: Vec4,
    #[storage(8, read_only)]
    pub additional_uv_from_bevy: Handle<bevy::render::storage::ShaderBuffer>,
    #[texture(9)]
    #[sampler(10)]
    pub additional_texture: Handle<Image>,
    #[uniform(11)]
    pub surface: Vec4,
    pub class: u8,
    pub reciprocal: f32,
    pub additional_reciprocal: f32,
    pub reflection_profile: bool,
}
// Keep both matrices in the shader's original uniform address space.
pub type WorldColorMaterial = crate::source_view::UniformMaterial<WorldBindings, 296>;
pub const ATTRIBUTE_SECONDARY_COLOR: MeshVertexAttribute = MeshVertexAttribute::new(
    "TerrainSecondaryColor",
    0x4743_4b02,
    VertexFormat::Float32x4,
);
impl Material for WorldColorMaterial {
    fn vertex_shader() -> ShaderRef {
        "embedded://giants_app/world.wesl".into()
    }
    fn fragment_shader() -> ShaderRef {
        "embedded://giants_app/world.wesl".into()
    }
    fn specialize(
        _: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.vertex.buffers = vec![layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_COLOR.at_shader_location(1),
            ATTRIBUTE_SECONDARY_COLOR.at_shader_location(2),
        ])?];
        // Source terrain winding is one-sided. Rendering the reverse faces
        // occludes low cinematic cameras located below the local ground.
        descriptor.primitive.cull_mode = Some(bevy::render::render_resource::Face::Back);
        if key.bind_group_data.0
            && let Some(depth) = &mut descriptor.depth_stencil
        {
            depth.depth_compare = Some(bevy::render::render_resource::CompareFunction::Always);
            depth.depth_write_enabled = Some(false);
        }
        Ok(())
    }
}
#[derive(Component)]
pub struct Rendered;
#[derive(Resource)]
pub struct Scene {
    pub asset: std::sync::Arc<giants_game::WorldAsset>,
    pub oblique: bool,
    pub validation_camera: Option<crate::validation_camera::ValidationCamera>,
    pub camera_source: &'static str,
    pub camera_origin: Option<[f32; 3]>,
    pub cinematic_capture_ready: bool,
    pub no_fog: bool,
    pub render_terrain: bool,
    pub render_sky: bool,
    pub render_underside: bool,
    pub reflection_profile: bool,
    pub water: Option<crate::water_scene::Scene>,
    pub render_water: bool,
}
fn convert(v: [f32; 3]) -> Vec3 {
    Vec3::new(v[0], v[2], -v[1])
}
#[derive(bevy::ecs::system::SystemParam)]
pub struct SceneMaterials<'w> {
    terrain: ResMut<'w, Assets<WorldColorMaterial>>,
    sky: ResMut<'w, Assets<crate::sky_scene::SkyMaterial>>,
    underside: ResMut<'w, Assets<crate::underside_scene::UndersideMaterial>>,
    presentation: ResMut<'w, Assets<crate::world_composition::Presentation>>,
    water: ResMut<'w, Assets<crate::water_scene::Material>>,
    patterns: ResMut<'w, Assets<crate::water_texture::Material>>,
    pub(crate) buffers: ResMut<'w, Assets<bevy::render::storage::ShaderBuffer>>,
}
pub fn apply_camera(scene: &Scene, materials: &mut SceneMaterials) {
    let Some(camera) = &scene.validation_camera else {
        return;
    };
    let clip = camera.clip_from_bevy();
    for (_, m) in materials.terrain.iter() {
        if let Some(mut buffer) = materials.buffers.get_mut(&m.clip_from_bevy) {
            crate::source_view::write_uniform(&clip, &mut buffer);
        }
        if let Some(mut buffer) = materials.buffers.get_mut(&m.uv_from_bevy) {
            crate::source_view::write_uniform(
                &camera.uv_from_bevy(m.class, m.reciprocal),
                &mut buffer,
            );
        }
        if let Some(mut buffer) = materials.buffers.get_mut(&m.additional_uv_from_bevy) {
            crate::source_view::write_uniform(
                &camera.uv_from_bevy(m.class, m.additional_reciprocal),
                &mut buffer,
            );
        }
    }
    for (_, m) in materials.sky.iter_mut() {
        m.clip_from_bevy = crate::sky_scene::clip_from_camera(camera, scene.reflection_profile);
        m.validation.y = 1. / camera.viewport.z as f32;
        m.validation.z = -1. / camera.viewport.w as f32;
    }
    for (_, m) in materials.underside.iter_mut() {
        m.clip_from_bevy = clip;
        m.validation.y = 1. / camera.viewport.z as f32;
        m.validation.z = -1. / camera.viewport.w as f32;
    }
    for (_, m) in materials.water.iter_mut() {
        m.clip_from_bevy = clip;
        m.validation.y = 1. / camera.viewport.z as f32;
        m.validation.z = -1. / camera.viewport.w as f32;
    }
}
pub fn setup(
    mut commands: Commands,
    scene: Res<Scene>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: SceneMaterials,
    cube: Option<ResMut<crate::reflection_cube::Build>>,
) {
    let world_target = crate::world_composition::spawn(
        &mut commands,
        &mut images,
        &mut materials.presentation,
        if scene.reflection_profile { 2 } else { 0 },
    );
    if let Some(mut cube) = cube.filter(|c| !c.complete) {
        cube.set_source(world_target.clone(), &mut images);
    }
    let a = &scene.asset;
    if scene.render_water
        && let Some(water) = &scene.water
    {
        crate::water_scene::spawn(
            &mut commands,
            &scene,
            water,
            &mut images,
            &mut meshes,
            &mut materials.water,
            &mut materials.patterns,
        );
    }
    let t = &a.terrain;
    let textures: Vec<_> = a
        .textures
        .iter()
        .map(|texture| {
            let first = &texture.levels[0];
            let mut image = Image::new(
                bevy::render::render_resource::Extent3d {
                    width: first.width,
                    height: first.height,
                    depth_or_array_layers: 1,
                },
                bevy::render::render_resource::TextureDimension::D2,
                first.rgba.clone(),
                bevy::render::render_resource::TextureFormat::Rgba8Unorm,
                RenderAssetUsages::default(),
            );
            image.texture_descriptor.mip_level_count = texture.levels.len() as u32;
            image.data = Some(
                texture
                    .levels
                    .iter()
                    .flat_map(|m| m.rgba.iter().copied())
                    .collect(),
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
            images.add(image)
        })
        .collect();
    let additional_textures: Vec<Option<Handle<Image>>> = a.additional.as_ref().map_or_else(
        || vec![None; 3],
        |prepared| {
            prepared
                .layers
                .iter()
                .map(|layer| {
                    layer.as_ref().map(|layer| {
                        let first = &layer.levels[0];
                        let mut image = Image::new(
                            bevy::render::render_resource::Extent3d {
                                width: first.width,
                                height: first.height,
                                depth_or_array_layers: 1,
                            },
                            bevy::render::render_resource::TextureDimension::D2,
                            first.rgba.clone(),
                            bevy::render::render_resource::TextureFormat::Rgba8Unorm,
                            RenderAssetUsages::default(),
                        );
                        image.texture_descriptor.mip_level_count = layer.levels.len() as u32;
                        image.data = Some(
                            layer
                                .levels
                                .iter()
                                .flat_map(|m| m.rgba.iter().copied())
                                .collect(),
                        );
                        image.sampler = bevy::image::ImageSampler::Descriptor(
                            bevy::image::ImageSamplerDescriptor {
                                address_mode_u: bevy::image::ImageAddressMode::Repeat,
                                address_mode_v: bevy::image::ImageAddressMode::Repeat,
                                mag_filter: bevy::image::ImageFilterMode::Linear,
                                min_filter: bevy::image::ImageFilterMode::Linear,
                                mipmap_filter: bevy::image::ImageFilterMode::Linear,
                                anisotropy_clamp: 16,
                                ..default()
                            },
                        );
                        images.add(image)
                    })
                })
                .collect()
        },
    );
    for class in 0..9 {
        if !scene.render_terrain {
            break;
        }
        let slot = if class == 0 {
            0
        } else if class <= 4 {
            1
        } else {
            2
        };
        let scale = a.config.prefix.materials[slot]
            .as_ref()
            .expect("world preparation validates base materials")
            .reciprocal;
        let mut positions = Vec::new();
        let mut colors = Vec::new();
        let mut secondary_colors = Vec::new();
        let additional_layer = a.additional.as_ref().and_then(|p| p.layers[slot].as_ref());
        let unpack_color = |color: u32| {
            [
                ((color >> 16) & 255) as f32 / 255.,
                ((color >> 8) & 255) as f32 / 255.,
                (color & 255) as f32 / 255.,
                1.,
            ]
        };
        for (triangle, face_class) in a.triangles.iter().zip(&a.face_classes) {
            if *face_class != class {
                continue;
            }
            let p = triangle.map(|i| convert(a.positions[i as usize]));
            for (index, position) in triangle.iter().zip(p) {
                positions.push(position.to_array());
                let color = t.colors[*index as usize];
                let diffuse = if additional_layer.is_some() {
                    a.additional.as_ref().unwrap().direction_colors[*index as usize]
                } else {
                    color
                };
                colors.push(unpack_color(diffuse));
                secondary_colors.push(unpack_color(color));
            }
        }
        if positions.is_empty() {
            continue;
        }
        let mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
        .with_inserted_attribute(ATTRIBUTE_SECONDARY_COLOR, secondary_colors);
        let validation = scene.validation_camera.as_ref();
        let clip_from_bevy = materials.buffers.add(crate::source_view::uniform_buffer(
            &validation.map_or(Mat4::IDENTITY, |v| v.clip_from_bevy()),
        ));
        let uv_from_bevy =
            materials
                .buffers
                .add(crate::source_view::uniform_buffer(&validation.map_or_else(
                    || {
                        crate::validation_camera::ValidationCamera::uv_for_view(
                            Mat4::IDENTITY.to_cols_array(),
                            class,
                            scale,
                        )
                    },
                    |v| v.uv_from_bevy(class, scale),
                )));
        let additional_reciprocal = additional_layer.map_or(scale, |layer| layer.reciprocal);
        let additional_uv_from_bevy = if additional_layer.is_some() {
            materials
                .buffers
                .add(crate::source_view::uniform_buffer(&validation.map_or_else(
                    || {
                        crate::validation_camera::ValidationCamera::uv_for_view(
                            Mat4::IDENTITY.to_cols_array(),
                            class,
                            additional_reciprocal,
                        )
                    },
                    |v| v.uv_from_bevy(class, additional_reciprocal),
                )))
        } else {
            uv_from_bevy.clone()
        };
        let mut entity = commands.spawn((
            Rendered,
            Name::new(format!("{} GTI terrain class {}", a.name, class)),
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(
                materials
                    .terrain
                    .add(WorldColorMaterial::from(WorldBindings {
                        reflection_profile: scene.reflection_profile,
                        tint: Vec4::ONE,
                        texture: textures[slot].clone(),
                        additional_texture: additional_textures[slot]
                            .clone()
                            .unwrap_or_else(|| textures[slot].clone()),
                        additional_uv_from_bevy,
                        additional_reciprocal,
                        surface: Vec4::new(
                            f32::from(additional_layer.is_some()),
                            f32::from(a.additional.as_ref().is_some_and(|p| p.secondary_color)),
                            0.,
                            0.,
                        ),
                        clip_from_bevy,
                        validation: validation.map_or(Vec4::ZERO, |v| {
                            Vec4::new(1., 1. / v.viewport.z as f32, -1. / v.viewport.w as f32, 0.)
                        }),
                        uv_from_bevy,
                        class,
                        reciprocal: scale,
                        fog_parameters: Vec4::new(
                            a.config.fog.start,
                            a.config.fog.end,
                            if scene.no_fog { 0. } else { 1. },
                            0.,
                        ),
                        fog_color: Vec4::new(
                            a.config.fog.color[0] as f32 / 255.,
                            a.config.fog.color[1] as f32 / 255.,
                            a.config.fog.color[2] as f32 / 255.,
                            1.,
                        ),
                    })),
            ),
        ));
        if validation.is_some() {
            entity.insert(bevy::camera::visibility::NoFrustumCulling);
        }
    }
    if scene.render_sky {
        crate::sky_scene::spawn(
            &mut commands,
            &scene,
            &mut meshes,
            &mut images,
            &mut materials.sky,
        );
    }
    let span = Vec2::new(
        (t.width - 1) as f32 * t.spacing,
        (t.height - 1) as f32 * t.spacing,
    );
    let center = Vec3::new(
        t.origin[0] + span.x / 2.,
        (t.height_bounds[0] + t.height_bounds[1]) / 2.,
        -t.origin[1] - span.y / 2.,
    );
    let extent = span.max_element();
    let offset = if scene.oblique {
        Vec3::new(1.2, 1.1, 1.2)
    } else {
        Vec3::new(0., 1.3, 1.3)
    } * extent;
    let pose = Transform::from_translation(center + offset).looking_at(center, Vec3::Y);
    let viewport = scene
        .validation_camera
        .as_ref()
        .map(|v| bevy::camera::Viewport {
            physical_position: UVec2::new(v.viewport.x, v.viewport.y),
            physical_size: UVec2::new(v.viewport.z, v.viewport.w),
            ..default()
        });
    let projection = Projection::Perspective(PerspectiveProjection {
        near: 1.,
        far: 100000.,
        ..default()
    });
    if scene.render_underside {
        crate::underside_scene::spawn(
            &mut commands,
            &scene,
            crate::underside_scene::BackgroundView {
                pose,
                projection: projection.clone(),
                viewport: viewport.clone(),
                target: world_target.clone(),
            },
            &mut meshes,
            &mut materials.underside,
            &mut images,
        );
    }
    if scene.reflection_profile {
        // The original cube has no depth attachment: preserve pass ordering explicitly.
        for (layer, order, enabled, name) in [
            (2, 0, scene.render_sky, "Cube sky pass"),
            (3, 1, scene.render_water, "Cube water pass"),
        ] {
            if enabled {
                commands.spawn((
                    Rendered,
                    Name::new(name),
                    bevy::camera::visibility::RenderLayers::layer(layer),
                    Camera3d::default(),
                    bevy::camera::RenderTarget::Image(world_target.clone().into()),
                    Camera {
                        order,
                        viewport: viewport.clone(),
                        clear_color: ClearColorConfig::None,
                        ..default()
                    },
                    projection.clone(),
                    pose,
                    Msaa::Off,
                    bevy::core_pipeline::tonemapping::Tonemapping::None,
                    bevy::core_pipeline::tonemapping::DebandDither::Disabled,
                ));
            }
        }
        info!(
            "reflection_pass_order=underside_terrain_sky_water depth_test=disabled depth_write=disabled native_depth_attachment=none"
        );
    }
    commands.spawn((
        Rendered,
        Camera3d::default(),
        bevy::camera::RenderTarget::Image(world_target.into()),
        Camera {
            order: -1,
            viewport,
            clear_color: if scene.render_underside
                && crate::underside_scene::background_pass(&scene)
            {
                ClearColorConfig::None
            } else if let Some(water) = &scene.water {
                let c = water.state.background;
                ClearColorConfig::Custom(Color::linear_rgb(
                    c[0] as f32 / 255.,
                    c[1] as f32 / 255.,
                    c[2] as f32 / 255.,
                ))
            } else {
                ClearColorConfig::Default
            },
            ..default()
        },
        projection,
        Msaa::Off,
        bevy::core_pipeline::tonemapping::Tonemapping::None,
        bevy::core_pipeline::tonemapping::DebandDither::Disabled,
        pose,
        FlyCamera {
            reset: pose,
            speed: 1000.,
        },
    ));
}
pub fn validation_exit(
    keys: Res<ButtonInput<KeyCode>>,
    mut exit: MessageWriter<bevy::app::AppExit>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        exit.write(bevy::app::AppExit::Success);
    }
}
pub fn update_uv(
    scene: Res<Scene>,
    cameras: Query<&Transform, With<FlyCamera>>,
    materials: Res<Assets<WorldColorMaterial>>,
    mut buffers: ResMut<Assets<bevy::render::storage::ShaderBuffer>>,
) {
    if scene.validation_camera.is_some() {
        return;
    }
    let Ok(camera) = cameras.single() else {
        return;
    };
    let source_to_bevy = Mat4::from_cols(Vec4::X, -Vec4::Z, Vec4::Y, Vec4::W);
    let view =
        (Mat4::from_scale(Vec3::new(1., 1., -1.)) * camera.to_matrix().inverse() * source_to_bevy)
            .to_cols_array();
    for (_, material) in materials.iter() {
        let matrix = crate::validation_camera::ValidationCamera::uv_for_view(
            view,
            material.class,
            material.reciprocal,
        );
        if let Some(mut buffer) = buffers.get_mut(&material.uv_from_bevy) {
            crate::source_view::write_uniform(&matrix, &mut buffer);
        }
        let additional_matrix = crate::validation_camera::ValidationCamera::uv_for_view(
            view,
            material.class,
            material.additional_reciprocal,
        );
        if let Some(mut buffer) = buffers.get_mut(&material.additional_uv_from_bevy) {
            crate::source_view::write_uniform(&additional_matrix, &mut buffer);
        }
    }
}
