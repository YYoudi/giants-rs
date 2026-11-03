//! Measured sun band with explicit fixed water/camera comparison state.
use bevy::{
    asset::RenderAssetUsages,
    mesh::MeshVertexBufferLayoutRef,
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    reflect::TypePath,
    render::render_resource::{
        AsBindGroup, PrimitiveTopology, RenderPipelineDescriptor, SpecializedMeshPipelineError,
        TextureFormat,
    },
    shader::ShaderRef,
};
#[derive(Clone, Copy)]
pub struct ValidationState {
    pub eye: [f32; 3],
    pub sun: [f32; 3],
    pub phases: [[f32; 2]; 5],
    pub background: [u8; 3],
    pub sun_from_world: bool,
}
impl ValidationState {
    pub fn read(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        Self::parse(&std::fs::read(path)?)
    }
    fn parse(data: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        if data.len() == 64 && &data[..8] == b"GWST0002" {
            let mut expanded = b"GWST0001".to_vec();
            expanded.extend(&data[8..20]);
            expanded.extend([1f32, 0., 0.].into_iter().flat_map(f32::to_le_bytes));
            expanded.extend(&data[20..]);
            let mut state = Self::parse(&expanded)?;
            state.sun_from_world = true;
            return Ok(state);
        }
        if data.len() != 76 || &data[..8] != b"GWST0001" || data[75] != 0 {
            return Err("Invalid water state header/size".into());
        }
        let values: Vec<f32> = data[8..72]
            .as_chunks::<4>()
            .0
            .iter()
            .map(|v| f32::from_le_bytes(*v))
            .collect();
        if !values.iter().all(|v| v.is_finite())
            || values[6..].iter().any(|v| v.abs() > 32.)
            || f64::from(values[3]).hypot(f64::from(values[4])) < 0.01
        {
            return Err("Invalid water state scalars".into());
        }
        Ok(Self {
            eye: values[..3].try_into()?,
            sun: values[3..6].try_into()?,
            phases: std::array::from_fn(|i| [values[6 + i * 2], values[7 + i * 2]]),
            background: data[72..75].try_into()?,
            sun_from_world: false,
        })
    }
}
pub struct Scene {
    pub asset: giants_game::water::TextureAsset,
    pub state: ValidationState,
    pub band: giants_game::water::Band,
    pub autonomous: bool,
}
#[derive(Resource)]
pub struct RuntimeWater {
    mesh: Handle<Mesh>,
    pattern: Handle<crate::water_texture::Material>,
    motion: giants_game::water::Motion,
    last_eye: Option<[f32; 2]>,
    steps: u64,
}
impl RuntimeWater {
    /// Presentation preview using elapsed time; no normal native timing claim.
    pub fn advance_preview(
        &mut self,
        dt: f32,
        scene: &crate::world_scene::Scene,
        patterns: &mut Assets<crate::water_texture::Material>,
    ) -> Result<(), &'static str> {
        let water = scene.water.as_ref().ok_or("water preview scene missing")?;
        if let Some(mut pattern) = patterns.get_mut(&self.pattern) {
            pattern.parameters =
                crate::water_texture::parameters(&water.asset, self.motion.phases, true);
        }
        self.motion.advance(dt, water.asset.water.values[4], 0.)?;
        self.steps += 1;
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MaterialKey(bool);
impl From<&Material> for MaterialKey {
    fn from(material: &Material) -> Self {
        Self(material.validation.w > 0.)
    }
}
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
#[bind_group_data(MaterialKey)]
pub struct Material {
    #[texture(0)]
    #[sampler(1)]
    pub image: Handle<Image>,
    #[uniform(2)]
    pub clip_from_bevy: Mat4,
    #[uniform(3)]
    pub validation: Vec4,
    #[uniform(4)]
    pub fog: Vec4,
    #[uniform(5)]
    pub tint: Vec4,
}
impl bevy::pbr::Material for Material {
    fn vertex_shader() -> ShaderRef {
        "embedded://giants_app/water.wesl".into()
    }
    fn fragment_shader() -> ShaderRef {
        "embedded://giants_app/water.wesl".into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Add
    }
    fn specialize(
        _: &MaterialPipeline,
        d: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        d.vertex.buffers = vec![layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_COLOR.at_shader_location(1),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(2),
        ])?];
        if let Some(depth) = &mut d.depth_stencil {
            depth.depth_write_enabled = Some(false);
            if key.bind_group_data.0 {
                depth.depth_compare = Some(bevy::render::render_resource::CompareFunction::Always);
            }
        }
        if let Some(fragment) = &mut d.fragment {
            use bevy::render::render_resource::{
                BlendComponent, BlendFactor, BlendOperation, BlendState,
            };
            for target in fragment.targets.iter_mut().flatten() {
                target.blend = Some(BlendState {
                    color: BlendComponent {
                        src_factor: BlendFactor::SrcAlpha,
                        dst_factor: BlendFactor::One,
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
pub fn spawn(
    commands: &mut Commands,
    scene: &crate::world_scene::Scene,
    water: &Scene,
    images: &mut Assets<Image>,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<Material>,
    patterns: &mut Assets<crate::water_texture::Material>,
) {
    let source = images.add(crate::water_texture::source_image(&water.asset));
    let mut tile = Image::new_target_texture(256, 256, TextureFormat::Rgba8Unorm, None);
    tile.sampler = bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
        address_mode_u: bevy::image::ImageAddressMode::Repeat,
        address_mode_v: bevy::image::ImageAddressMode::Repeat,
        min_filter: bevy::image::ImageFilterMode::Linear,
        mag_filter: bevy::image::ImageFilterMode::Linear,
        ..default()
    });
    let target = images.add(tile);
    let camera = commands
        .spawn((
            crate::world_scene::Rendered,
            Name::new("Water pattern producer"),
            Camera2d,
            Camera {
                order: -3,
                ..default()
            },
            bevy::camera::RenderTarget::Image(target.clone().into()),
            Msaa::Off,
        ))
        .id();
    let pattern = patterns.add(crate::water_texture::Material {
        source,
        parameters: crate::water_texture::parameters(&water.asset, water.state.phases, true),
    });
    commands.spawn((
        crate::world_scene::Rendered,
        UiTargetCamera(camera),
        MaterialNode(pattern.clone()),
        LayoutConfig {
            use_rounding: false,
        },
        Node {
            position_type: PositionType::Absolute,
            left: px(0.),
            top: px(0.),
            width: px(256.),
            height: px(256.),
            ..default()
        },
    ));
    let mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION,
        water
            .band
            .vertices
            .iter()
            .map(|v| [v.position[0], v.position[2], -v.position[1]])
            .collect::<Vec<_>>(),
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_UV_0,
        water.band.vertices.iter().map(|v| v.uv).collect::<Vec<_>>(),
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_COLOR,
        water
            .band
            .vertices
            .iter()
            .map(|v| [1., 1., 1., (v.color >> 24) as f32 / 255.])
            .collect::<Vec<_>>(),
    )
    .with_inserted_indices(bevy::mesh::Indices::U32(water.band.indices.clone()));
    let v = scene.validation_camera.as_ref();
    let fog = scene.asset.config.water_fog;
    let c = water.asset.water.colors[0];
    let mesh = meshes.add(mesh);
    if water.autonomous {
        commands.insert_resource(RuntimeWater {
            mesh: mesh.clone(),
            pattern,
            motion: giants_game::water::Motion::default(),
            last_eye: None,
            steps: 0,
        });
    }
    let mut entity = commands.spawn((
        crate::world_scene::Rendered,
        Name::new("Water sun band"),
        Mesh3d(mesh),
        MeshMaterial3d(materials.add(Material {
            image: target,
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
            tint: Vec4::new(
                c[0] as f32 / 255.,
                c[1] as f32 / 255.,
                c[2] as f32 / 255.,
                1.,
            ),
        })),
        bevy::camera::visibility::NoFrustumCulling,
    ));
    if scene.reflection_profile {
        entity.insert(bevy::camera::visibility::RenderLayers::layer(3));
    }
}

pub fn update_geometry(
    scene: Res<crate::world_scene::Scene>,
    runtime: Option<ResMut<RuntimeWater>>,
    cameras: Query<&Transform, With<crate::mesh_scene::FlyCamera>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let Some(mut runtime) = runtime else {
        return;
    };
    let Some(water) = &scene.water else {
        return;
    };
    let eye = if let Some(origin) = scene.camera_origin {
        Vec3::from_array(origin)
    } else if let Some(v) = &scene.validation_camera {
        Mat4::from_cols_array(&v.view).inverse().w_axis.truncate()
    } else {
        let Ok(camera) = cameras.single() else {
            return;
        };
        Vec3::new(
            camera.translation.x,
            -camera.translation.z,
            camera.translation.y,
        )
    };
    if runtime.last_eye == Some([eye.x, eye.y]) {
        return;
    }
    let Ok(band) = giants_game::water::band(
        [eye.x, eye.y],
        [water.state.sun[0], water.state.sun[1]],
        scene.asset.config.sea_values[0],
        water.asset.water,
    ) else {
        return;
    };
    if let Some(mut mesh) = meshes.get_mut(&runtime.mesh) {
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            band.vertices
                .iter()
                .map(|v| [v.position[0], v.position[2], -v.position[1]])
                .collect::<Vec<_>>(),
        );
        if runtime.last_eye.is_none() {
            info!(
                "water_geometry_eye_source={:?} vertices={} indices={}",
                [eye.x, eye.y, eye.z],
                band.vertices.len(),
                band.indices.len()
            );
        }
        runtime.last_eye = Some([eye.x, eye.y]);
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_UV_0,
            band.vertices.iter().map(|v| v.uv).collect::<Vec<_>>(),
        );
    }
}
pub fn animate(
    scene: Res<crate::world_scene::Scene>,
    runtime: Option<ResMut<RuntimeWater>>,
    pipelines: Res<crate::PipelinesReady>,
    mut patterns: ResMut<Assets<crate::water_texture::Material>>,
) {
    if !pipelines.0 {
        return;
    }
    let Some(mut runtime) = runtime else {
        return;
    };
    let Some(water) = &scene.water else {
        return;
    };
    if let Some(mut pattern) = patterns.get_mut(&runtime.pattern) {
        pattern.parameters =
            crate::water_texture::parameters(&water.asset, runtime.motion.phases, true);
        runtime
            .motion
            .advance(1. / 60., water.asset.water.values[4], 0.)
            .expect("validated water motion");
        runtime.steps += 1;
        if runtime.steps.is_multiple_of(60) {
            info!("world_water_animation_steps={}", runtime.steps);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ValidationState;
    #[test]
    fn water_state_rejects_truncated_nonfinite_and_degenerate_sun() {
        let mut data = b"GWST0001".to_vec();
        for value in [
            0f32, 0., 100., 1., 0., 1., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0.,
        ] {
            data.extend(value.to_le_bytes());
        }
        data.extend([32, 64, 96, 0]);
        assert_eq!(
            ValidationState::parse(&data).unwrap().background,
            [32, 64, 96]
        );
        for end in 0..data.len() {
            assert!(ValidationState::parse(&data[..end]).is_err());
        }
        for (at, value) in [(8, f32::NAN), (20, 0.), (32, 33.)] {
            let mut invalid = data.clone();
            invalid[at..at + 4].copy_from_slice(&value.to_le_bytes());
            assert!(ValidationState::parse(&invalid).is_err());
        }
        data.push(0);
        assert!(ValidationState::parse(&data).is_err());
    }
    #[test]
    fn version_two_has_no_serialized_sun() {
        let mut data = b"GWST0002".to_vec();
        data.extend([2f32, 3., 4.].into_iter().flat_map(f32::to_le_bytes));
        data.extend([0; 44]);
        let state = ValidationState::parse(&data).unwrap();
        assert!(state.sun_from_world);
        assert_eq!(state.eye, [2., 3., 4.]);
        for end in 0..data.len() {
            assert!(ValidationState::parse(&data[..end]).is_err());
        }
    }
}
