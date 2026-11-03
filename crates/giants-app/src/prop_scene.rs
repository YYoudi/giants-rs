//! First pub geometry and measured opaque/multiplicative material profiles.
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
pub struct MaterialKey(u8);
impl From<&Material> for MaterialKey {
    fn from(value: &Material) -> Self {
        let pass = if value.multiply {
            1
        } else if value.alpha {
            2
        } else {
            0
        };
        Self(pass | if value.instanced { 8 } else { 0 })
    }
}
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
#[bind_group_data(MaterialKey)]
pub struct Material {
    pub instanced: bool,
    #[storage(7, read_only, visibility(vertex))]
    pub instance_matrices: Handle<bevy::render::storage::ShaderBuffer>,
    pub multiply: bool,
    pub alpha: bool,
    #[texture(0)]
    #[sampler(1)]
    pub texture: Option<Handle<Image>>,
    #[storage(2, read_only)]
    pub view: Handle<bevy::render::storage::ShaderBuffer>,
    #[uniform(3)]
    pub world_from_bevy: Mat4,
    #[uniform(4)]
    pub pixel_offset: Vec4,
    #[uniform(5)]
    pub fog: Vec4,
    #[uniform(6)]
    pub fog_color: Vec4,
}
impl bevy::pbr::Material for Material {
    fn vertex_shader() -> ShaderRef {
        "embedded://giants_app/prop.wesl".into()
    }
    fn fragment_shader() -> ShaderRef {
        "embedded://giants_app/prop.wesl".into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        if self.multiply || self.alpha {
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
            Mesh::ATTRIBUTE_UV_0.at_shader_location(1),
            Mesh::ATTRIBUTE_COLOR.at_shader_location(2),
        ])?];
        d.primitive.cull_mode = Some(bevy::render::render_resource::Face::Back);
        if key.bind_group_data.0 & 8 != 0 {
            d.vertex.shader_defs.push("SOURCE_FOLIAGE_INSTANCES".into());
        }
        if key.bind_group_data.0 & 7 == 1 {
            use bevy::render::render_resource::{
                BlendComponent, BlendFactor, BlendOperation, BlendState,
            };
            if let Some(depth) = &mut d.depth_stencil {
                depth.depth_write_enabled = Some(false);
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
        }
        Ok(())
    }
}

#[derive(Resource)]
pub struct Scene {
    prop: giants_game::props::Pub,
}
impl Scene {
    pub fn source_keys(&self) -> impl Iterator<Item = (bool, usize)> + '_ {
        std::iter::once((false, self.prop.source_offset))
    }
    pub fn load(
        world: &giants_game::WorldAsset,
        plan: &giants_game::instances::Plan,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let prop = giants_game::props::load_planned(plan, world)?;
        for (sub, pass) in prop.mesh.subobjects.iter().zip(&prop.material_passes) {
            if !matches!(sub.material.flags, 0x40000000 | 0x40000002 | 0x40000020)
                || ![0., 1., 0.001f32].contains(&sub.material.blend)
            {
                return Err("Pub material outside first-scene profile".into());
            }
            if *pass == giants_game::render_parts::Pass::TextureFormat
                || (*pass == giants_game::render_parts::Pass::ObjectOrMaterial)
                    != (sub.material.flags == 0x40000002)
            {
                return Err("Pub outside measured material routing profile".into());
            }
        }
        println!(
            "cinematic_pub=true pub_parts={} pub_material=source_texture_rgb_multiply pub_animation=false",
            prop.mesh.subobjects.len()
        );
        Ok(Self { prop })
    }
}
pub fn setup(
    mut commands: Commands,
    view: Res<crate::source_view::SourceView>,
    scene: Res<Scene>,
    world: Res<crate::world_scene::Scene>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<Material>>,
) {
    let camera = world
        .validation_camera
        .as_ref()
        .expect("Pub requires cinematic camera");
    let source_to_bevy = Mat4::from_cols(Vec4::X, -Vec4::Z, Vec4::Y, Vec4::W);
    let bevy_to_source = Mat4::from_cols(Vec4::X, Vec4::Z, -Vec4::Y, Vec4::W);
    let matrix =
        source_to_bevy * Mat4::from_cols_array(&scene.prop.matrix).transpose() * bevy_to_source;
    let mut textures = std::collections::BTreeMap::new();
    for texture in &scene.prop.textures {
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
    for (sub, pass) in scene
        .prop
        .mesh
        .subobjects
        .iter()
        .zip(&scene.prop.material_passes)
    {
        let indices: Vec<usize> = sub
            .triangles
            .iter()
            .flatten()
            .map(|i| usize::from(*i))
            .collect();
        let positions: Vec<_> = indices
            .iter()
            .map(|i| {
                let p = scene.prop.mesh.positions[scene.prop.mesh.indices[*i] as usize];
                [p[0], p[2], -p[1]]
            })
            .collect();
        let uv: Vec<_> = indices.iter().map(|i| scene.prop.mesh.uv[*i]).collect();
        let rgb: Vec<_> = indices
            .iter()
            .map(|i| {
                let c = if sub.material.flags == 0x40000020 && sub.material.blend == 1. {
                    scene.prop.mesh.rgb[*i]
                } else {
                    [255; 3]
                };
                [
                    c[0] as f32 / 255.,
                    c[1] as f32 / 255.,
                    c[2] as f32 / 255.,
                    1.,
                ]
            })
            .collect();
        let mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, rgb);
        let material = materials.add(Material {
            instanced: false,
            instance_matrices: view.transforms.clone(),
            multiply: *pass == giants_game::render_parts::Pass::ObjectOrMaterial,
            alpha: false,
            fog: Vec4::new(
                world.asset.config.fog.start,
                world.asset.config.fog.end,
                if world.no_fog { 0. } else { 1. },
                0.,
            ),
            fog_color: if sub.material.flags == 0x40000002 {
                Vec4::ZERO
            } else {
                Vec4::new(
                    world.asset.config.fog.color[0] as f32 / 255.,
                    world.asset.config.fog.color[1] as f32 / 255.,
                    world.asset.config.fog.color[2] as f32 / 255.,
                    0.,
                )
            },
            texture: Some(textures[&sub.material.texture.to_ascii_lowercase()].clone()),
            view: view.buffer.clone(),
            world_from_bevy: matrix,
            pixel_offset: Vec4::new(
                1. / camera.viewport.z as f32,
                -1. / camera.viewport.w as f32,
                0.,
                0.,
            ),
        });
        commands.spawn((
            Name::new(format!("Mission pub / {}", sub.name)),
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(material),
            bevy::camera::visibility::NoFrustumCulling,
        ));
    }
}
