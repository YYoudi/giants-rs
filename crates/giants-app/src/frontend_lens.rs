//! Two source lens meshes. Shared source view, owned buffers, additive RGB.
use bevy::{
    asset::RenderAssetUsages,
    mesh::MeshVertexBufferLayoutRef,
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    reflect::TypePath,
    render::{render_resource::*, storage::ShaderBuffer},
    shader::ShaderRef,
};

#[derive(Clone, Debug, ShaderType)]
struct Parameters {
    world: Mat4,
    pixel_alpha: Vec4,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Key(bool);
impl From<&Bindings> for Key {
    fn from(value: &Bindings) -> Self {
        Self(value.depth)
    }
}
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
#[bind_group_data(Key)]
pub struct Bindings {
    #[texture(0)]
    #[sampler(1)]
    texture: Handle<Image>,
    #[storage(2, read_only)]
    parameters: Handle<ShaderBuffer>,
    #[storage(3, read_only)]
    view: Handle<ShaderBuffer>,
    depth: bool,
}
pub type Material = crate::source_view::UniformMaterial<Bindings, 12>;
impl bevy::pbr::Material for Material {
    fn vertex_shader() -> ShaderRef {
        "embedded://giants_app/frontend_lens.wesl".into()
    }
    fn fragment_shader() -> ShaderRef {
        Self::vertex_shader()
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
            Mesh::ATTRIBUTE_UV_0.at_shader_location(1),
        ])?];
        d.primitive.cull_mode = Some(Face::Back);
        if let Some(depth) = &mut d.depth_stencil {
            depth.depth_write_enabled = Some(false);
            if !key.bind_group_data.0 {
                depth.depth_compare = Some(CompareFunction::Always);
            }
        }
        if let Some(fragment) = &mut d.fragment {
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
struct Lens {
    values: [f32; 10],
    cap: f32,
    channels: [f32; 2],
    rotation: f32,
    buffer: Handle<ShaderBuffer>,
}
#[derive(Resource)]
pub struct Runtime {
    lenses: Vec<Lens>,
    phase: f32,
    clip: Handle<ShaderBuffer>,
    previous_clip: Mat4,
}
pub fn spawn(
    world: &mut World,
    source: &giants_game::frontend_background::Background,
) -> Handle<ShaderBuffer> {
    let view = world
        .resource_mut::<Assets<ShaderBuffer>>()
        .add(crate::source_view::uniform_buffer(&Mat4::IDENTITY));
    let mut lenses = Vec::new();
    for lens in &source.lenses {
        let t = &lens.texture;
        let rgba = t
            .pixels
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|c| [c[0], c[1], c[2], 255])
            .collect();
        let mut image = Image::new(
            Extent3d {
                width: t.width,
                height: t.height,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            rgba,
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::default(),
        );
        image.sampler = crate::logo_material::sampler();
        let texture = world.resource_mut::<Assets<Image>>().add(image);
        let sub = &lens.mesh.subobjects[0];
        let indices: Vec<_> = sub
            .triangles
            .iter()
            .flatten()
            .map(|i| usize::from(*i))
            .collect();
        let positions: Vec<_> = indices
            .iter()
            .map(|i| {
                let p = lens.mesh.positions[usize::from(lens.mesh.indices[*i])];
                [p[0], p[2], -p[1]]
            })
            .collect();
        let uv: Vec<_> = indices.iter().map(|i| lens.mesh.uv[*i]).collect();
        let mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv);
        let mesh = world.resource_mut::<Assets<Mesh>>().add(mesh);
        let buffer =
            world
                .resource_mut::<Assets<ShaderBuffer>>()
                .add(crate::source_view::uniform_buffer(&Parameters {
                    world: Mat4::IDENTITY,
                    pixel_alpha: Vec4::ZERO,
                }));
        let material = world
            .resource_mut::<Assets<Material>>()
            .add(Material::from(Bindings {
                texture,
                parameters: buffer.clone(),
                view: view.clone(),
                depth: lens.class == 1102,
            }));
        world.spawn((
            super::frontend_background::Owned,
            Name::new(format!("Frontend lens {}", lens.class)),
            Mesh3d(mesh),
            MeshMaterial3d(material),
            bevy::camera::visibility::NoFrustumCulling,
        ));
        lenses.push(Lens {
            values: lens.values,
            cap: 1. - sub.material.falloff,
            channels: [0.; 2],
            rotation: 0.,
            buffer,
        });
    }
    world.insert_resource(Runtime {
        lenses,
        phase: 0.,
        clip: view.clone(),
        previous_clip: Mat4::IDENTITY,
    });
    view
}
pub fn update(
    time: Res<Time>,
    scene: Option<Res<crate::world_scene::Scene>>,
    runtime: Option<ResMut<Runtime>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
) {
    let (Some(scene), Some(mut runtime)) = (scene, runtime) else {
        return;
    };
    let (Some(camera), Some(eye)) = (scene.validation_camera.as_ref(), scene.camera_origin) else {
        return;
    };
    let result = (|| -> Result<(), giants_formats::Error> {
        let ray = giants_game::sun_glare::ray(
            scene.asset.sun,
            eye,
            [camera.view[2], camera.view[6], camera.view[10]],
        )?;
        let clear = giants_game::terrain_visibility::clear(&scene.asset.terrain, &ray)?;
        // Native solar passes temporarily use far=80000, retaining the same camera.
        let mut solar_camera = camera.clone();
        solar_camera.projection[10] = (80_000f64 / 79_999.) as f32;
        solar_camera.projection[14] = -solar_camera.projection[10];
        let clip = solar_camera.clip_from_bevy();
        if runtime.previous_clip != clip {
            if let Some(mut buffer) = buffers.get_mut(&runtime.clip) {
                crate::source_view::write_uniform(&clip, &mut buffer);
            }
            runtime.previous_clip = clip;
        }
        let convert = Mat4::from_cols(Vec4::X, -Vec4::Z, Vec4::Y, Vec4::W);
        let mut phase = runtime.phase;
        for lens in &mut runtime.lenses {
            let p = lens.values;
            for (i, (exponent, response)) in [(p[0], p[2]), (p[4], p[6])].into_iter().enumerate() {
                lens.channels[i] = giants_game::sun_glare::advance(
                    lens.channels[i],
                    time.delta_secs(),
                    ray.dot,
                    clear,
                    exponent,
                    response,
                )?
                .0;
            }
            lens.rotation += time.delta_secs() * p[9] * std::f32::consts::TAU;
            let scale = p[3] + p[5] * lens.channels[1] + phase.sin() * p[7];
            let mut basis = Mat4::IDENTITY;
            basis.x_axis = Vec4::new(camera.view[0], camera.view[4], camera.view[8], 0.);
            basis.y_axis = -Vec4::new(camera.view[2], camera.view[6], camera.view[10], 0.);
            basis.z_axis = Vec4::new(camera.view[1], camera.view[5], camera.view[9], 0.);
            basis.w_axis = Vec3::from_array(scene.asset.sun).extend(1.);
            let world = convert
                * basis
                * Mat4::from_rotation_y(lens.rotation)
                * Mat4::from_scale(Vec3::splat(scale))
                * convert.inverse();
            let alpha = (255. * (p[1] * lens.channels[0]).min(lens.cap)).trunc() / 255.;
            if let Some(mut buffer) = buffers.get_mut(&lens.buffer) {
                crate::source_view::write_uniform(
                    &Parameters {
                        world,
                        pixel_alpha: Vec4::new(
                            1. / camera.viewport.z as f32,
                            -1. / camera.viewport.w as f32,
                            alpha,
                            0.,
                        ),
                    },
                    &mut buffer,
                );
            }
            // Deterministic preview uses the mean native random multiplier (1).
            // It does not claim the original process's random/history stream.
            phase =
                (phase + p[8] * time.delta_secs() * std::f32::consts::PI) % std::f32::consts::TAU;
        }
        runtime.phase = phase;
        Ok(())
    })();
    if let Err(error) = result {
        error!("frontend_lens_failed={error}");
    }
}
