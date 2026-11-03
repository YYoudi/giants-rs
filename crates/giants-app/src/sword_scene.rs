//! Source-attached sword with native fixed-function lighting and a generated cube.
use bevy::ecs::system::SystemParam;
use bevy::{
    asset::RenderAssetUsages,
    mesh::MeshVertexBufferLayoutRef,
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    reflect::TypePath,
    render::render_resource::{
        AsBindGroup, PrimitiveTopology, RenderPipelineDescriptor, ShaderType,
        SpecializedMeshPipelineError,
    },
    shader::ShaderRef,
};

#[derive(Clone, Debug, ShaderType)]
pub struct Light {
    direction: Vec4,
    color: Vec4,
}
#[derive(Clone, Debug, ShaderType)]
pub struct Parameters {
    world: Mat4,
    view: Mat4,
    clip: Mat4,
    normal_eye: Mat4,
    emissive: Vec4,
    ambient: Vec4,
    diffuse: Vec4,
    specular: Vec4,
    lights: [Light; 3],
    fog: Vec4,
    fog_color: Vec4,
    pixel_power: Vec4,
}
impl Parameters {
    pub(crate) fn blended(mut self, enabled: bool) -> Self {
        self.fog.w = if enabled { 1. } else { 0. };
        self
    }
    pub(crate) fn buffer(&self) -> bevy::render::storage::ShaderBuffer {
        crate::source_view::uniform_buffer(self)
    }
    pub(crate) fn write_buffer(&self, buffer: &mut bevy::render::storage::ShaderBuffer) {
        crate::source_view::write_uniform(self, buffer);
    }
    pub(crate) fn cube_factor(mut self, weight: Option<f32>) -> Self {
        self.pixel_power.w = weight.unwrap_or(-1.);
        self
    }
}
pub type Material = crate::source_view::UniformMaterial<Bindings>;
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
#[bind_group_data(MaterialKey)]
pub struct Bindings {
    pub(crate) blend: u8,
    front: bool,
    #[texture(0)]
    #[sampler(1)]
    texture: Handle<Image>,
    #[storage(2, read_only)]
    pub(crate) parameters: Handle<bevy::render::storage::ShaderBuffer>,
    #[texture(3, dimension = "cube")]
    #[sampler(4)]
    cube: Handle<Image>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MaterialKey(u8, bool);
impl From<&Bindings> for MaterialKey {
    fn from(value: &Bindings) -> Self {
        Self(value.blend, value.front)
    }
}
impl Material {
    pub(crate) fn with_front(mut self, front: bool) -> Self {
        self.front = front;
        self
    }
    pub(crate) fn front(&self) -> bool {
        self.front
    }
    pub(crate) fn set_front(&mut self, front: bool) {
        self.front = front;
    }
    pub(crate) fn with_blend(mut self, mode: u8) -> Self {
        self.blend = mode;
        self
    }
    pub(crate) fn new(
        texture: Handle<Image>,
        cube: Handle<Image>,
        parameters: Handle<bevy::render::storage::ShaderBuffer>,
    ) -> Self {
        Self::from(Bindings {
            blend: 0,
            front: false,
            texture,
            cube,
            parameters,
        })
    }
}
impl bevy::pbr::Material for Material {
    fn alpha_mode(&self) -> AlphaMode {
        match self.blend {
            1 => AlphaMode::Add,
            2 => AlphaMode::Blend,
            _ => AlphaMode::Opaque,
        }
    }
    fn vertex_shader() -> ShaderRef {
        "embedded://giants_app/sword.wesl".into()
    }
    fn fragment_shader() -> ShaderRef {
        "embedded://giants_app/sword.wesl".into()
    }
    fn specialize(
        _: &MaterialPipeline,
        d: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        d.vertex.buffers = vec![layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(2),
        ])?];
        d.primitive.cull_mode = Some(crate::source_view::cull_face(key.bind_group_data.1));
        if key.bind_group_data.0 != 0 {
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
                            src_factor: BlendFactor::SrcAlpha,
                            dst_factor: if key.bind_group_data.0 == 2 {
                                BlendFactor::OneMinusSrcAlpha
                            } else {
                                BlendFactor::One
                            },
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
pub type Scene = crate::cinematic_scene::Scenes<Sword>;
pub fn install_materials(app: &mut App) {
    if !app.is_plugin_added::<bevy::pbr::MaterialPlugin<Material>>() {
        app.add_plugins(bevy::pbr::MaterialPlugin::<Material>::default());
    }
}
pub fn install(app: &mut App, scene: Scene) {
    install_materials(app);
    app.insert_resource(scene)
        .add_systems(
            Update,
            switch
                .after(crate::cinematic_camera::update)
                .before(update)
                .run_if(crate::reflection_cube::playback_ready),
        )
        .add_systems(
            Update,
            update
                .after(crate::cinematic_camera::update)
                .run_if(crate::reflection_cube::playback_ready),
        );
}
pub struct Sword {
    sword: Option<giants_game::attachments::Attachment>,
    textures: Vec<giants_game::LogoTexture>,
    normals: Vec<[f32; 3]>,
    lights: Vec<giants_game::lighting::Directional>,
    materials: Vec<Handle<Material>>,
    fog: giants_game::lighting::Fog,
    tick: i32,
}
fn parameters(
    scene: &Sword,
    camera: &crate::validation_camera::ValidationCamera,
    animation: &giants_game::animation::Animation,
    tick: i32,
    sub: usize,
    no_fog: bool,
) -> Result<Parameters, Box<dyn std::error::Error>> {
    let sword = scene.sword.as_ref().ok_or("Sword missing")?;
    let (matrix, flags) = sword.matrix(animation, tick)?;
    if flags & 8 != 0 {
        return Err("Sword mirror outside measured profile".into());
    }
    let world = Mat4::from_cols_array(&matrix).transpose();
    Ok(source_parameters(
        &sword.mesh.subobjects[sub].material,
        world,
        camera,
        &scene.lights,
        scene.fog,
        no_fog,
    )
    .cube_factor((sub == 1).then_some(76. / 255.)))
}
pub(crate) fn source_parameters(
    material: &giants_formats::gbs::Material,
    world: Mat4,
    camera: &crate::validation_camera::ValidationCamera,
    lights: &[giants_game::lighting::Directional],
    fog: giants_game::lighting::Fog,
    no_fog: bool,
) -> Parameters {
    let view = Mat4::from_cols_array(&camera.view);
    let source_to_bevy = Mat4::from_cols(Vec4::X, -Vec4::Z, Vec4::Y, Vec4::W);
    let rgb = |word: u32| {
        Vec4::new(
            (word & 255) as f32 * (1. / 255.),
            ((word >> 8) & 255) as f32 * (1. / 255.),
            ((word >> 16) & 255) as f32 * (1. / 255.),
            1.,
        )
    };
    let lights = giants_game::lighting::shader_slots(lights).map(|source| match source {
        Some(l) => Light {
            direction: Vec3::from_array(l.direction).extend(0.),
            color: Vec3::from_array(l.color).extend(1.),
        },
        None => Light {
            direction: Vec4::ZERO,
            color: Vec4::ZERO,
        },
    });
    Parameters {
        world,
        view,
        clip: camera.clip_from_bevy() * source_to_bevy,
        normal_eye: (view * world).inverse().transpose(),
        emissive: rgb(material.colors[0]),
        ambient: rgb(material.colors[1]),
        diffuse: rgb(material.colors[2]),
        specular: rgb(material.colors[3]),
        lights,
        fog: Vec4::new(fog.start, fog.end, if no_fog { 0. } else { 1. }, 0.),
        fog_color: Vec4::new(
            fog.color[0] as f32 / 255.,
            fog.color[1] as f32 / 255.,
            fog.color[2] as f32 / 255.,
            1.,
        ),
        pixel_power: Vec4::new(
            1. / camera.viewport.z as f32,
            -1. / camera.viewport.w as f32,
            material.power,
            -1.,
        ),
    }
}
impl Scene {
    pub fn owned_slots(&self, director: usize) -> Vec<usize> {
        self.entry(director)
            .and_then(|s| s.sword.as_ref())
            .filter(|a| !a.mesh.subobjects.is_empty())
            .map(|a| vec![a.slot])
            .unwrap_or_default()
    }
    pub fn has_geometry(&self) -> bool {
        self.any(|s| {
            s.sword
                .as_ref()
                .is_some_and(|a| !a.mesh.subobjects.is_empty())
        })
    }
    pub fn load(
        root: &std::path::Path,
        cinematic: &crate::cinematic_camera::Cinematic,
        slot: Option<usize>,
        config: &giants_formats::world::WorldConfig,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Self::prepare(cinematic, |segment| {
            Sword::load(root, segment, slot, config)
        })
    }
}
#[derive(Component)]
pub struct Rendered;
impl Sword {
    fn load(
        root: &std::path::Path,
        cinematic: &crate::cinematic_camera::Segment,
        slot: Option<usize>,
        config: &giants_formats::world::WorldConfig,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let source_lights = giants_game::lighting::first_world(config)?;
        let fog = config.fog;
        let mut swords = (if cinematic.preview() {
            giants_game::attachments::load_sequence_preview
        } else {
            giants_game::attachments::load_sequence
        })(root, cinematic.sequence())?
        .into_iter()
        .filter(|a| a.model.eq_ignore_ascii_case("rp_sword") && slot.is_none_or(|s| s == a.slot));
        let sword = swords.next();
        if swords.next().is_some() {
            return Err("Multiple sword attachments outside measured profile".into());
        }
        let (textures, normals, lights) = if let Some(sword) = &sword {
            if sword
                .mesh
                .subobjects
                .iter()
                .map(|s| s.material.flags)
                .collect::<Vec<_>>()
                != [0xc0000000, 0xd0000001, 0xc0000001]
            {
                return Err("Sword material outside measured profile".into());
            }
            (
                sword.textures(root)?,
                sword.mesh.render_normals()?,
                source_lights,
            )
        } else {
            (Vec::new(), Vec::new(), Vec::new())
        };
        println!(
            "cinematic_sword={} sword_lighting_source=world_objects lighting_space=eye sword_visibility=source_projection flk_visibility_events=shared_actor_slot",
            sword.is_some()
        );
        Ok(Self {
            sword,
            textures,
            normals,
            lights,
            materials: Vec::new(),
            fog,
            tick: cinematic.tick(),
        })
    }
}
#[derive(SystemParam)]
pub struct CubeAssets<'w> {
    build: Res<'w, crate::reflection_cube::Build>,
    images: ResMut<'w, Assets<Image>>,
    buffers: ResMut<'w, Assets<bevy::render::storage::ShaderBuffer>>,
}
pub fn setup(
    mut commands: Commands,
    mut scene: ResMut<Scene>,
    world: Res<crate::world_scene::Scene>,
    cinematic: Res<crate::cinematic_camera::Cinematic>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut cube: CubeAssets,
    mut materials: ResMut<Assets<Material>>,
) {
    if !cube.build.complete {
        return;
    }
    let Some(sword) = &scene.sword else {
        return;
    };
    let camera = world
        .validation_camera
        .as_ref()
        .expect("sword requires cinematic camera");
    let mut textures = std::collections::BTreeMap::new();
    for texture in &scene.textures {
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
        textures.insert(texture.name.to_ascii_lowercase(), cube.images.add(image));
    }
    let mut handles = Vec::new();
    for (index, sub) in sword.mesh.subobjects.iter().enumerate() {
        let indices: Vec<_> = sub
            .triangles
            .iter()
            .flatten()
            .map(|i| usize::from(*i))
            .collect();
        let positions: Vec<_> = indices
            .iter()
            .map(|i| sword.mesh.positions[sword.mesh.indices[*i] as usize])
            .collect();
        let normals: Vec<_> = indices.iter().map(|i| scene.normals[*i]).collect();
        let uv: Vec<_> = indices.iter().map(|i| sword.mesh.uv[*i]).collect();
        let mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv);
        let material = materials.add(Material::from(Bindings {
            front: false,
            blend: 0,
            cube: cube.build.handle().expect("Generated sword cube"),
            texture: textures[&sub.material.texture.to_ascii_lowercase()].clone(),
            parameters: cube.buffers.add(
                parameters(
                    &scene,
                    camera,
                    cinematic.animation(),
                    cinematic.tick(),
                    index,
                    world.no_fog,
                )
                .expect("validated sword parameters")
                .buffer(),
            ),
        }));
        handles.push(material.clone());
        commands.spawn((
            Rendered,
            crate::cinematic_visibility::ActorSlot(sword.slot),
            Name::new(format!("FLK sword slot {} / {index}", sword.slot)),
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(material),
            bevy::camera::visibility::NoFrustumCulling,
        ));
    }
    scene.materials = handles;
    info!(
        "cinematic_render_ready=sword_scene director={} meshes={}",
        cinematic.director_index(),
        scene.materials.len()
    );
}
pub fn update(
    cinematic: Res<crate::cinematic_camera::Cinematic>,
    world: Res<crate::world_scene::Scene>,
    mut scene: ResMut<Scene>,
    materials: Res<Assets<Material>>,
    mut buffers: ResMut<Assets<bevy::render::storage::ShaderBuffer>>,
    mut exit: MessageWriter<AppExit>,
    rendered: Query<Entity, With<Rendered>>,
) {
    if rendered.iter().count() != scene.materials.len() {
        error!("Director render entity count differs: sword_scene");
        exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
        return;
    }

    if scene.materials.is_empty()
        || scene.tick == cinematic.tick() && !world.is_changed()
        || scene.sword.is_none()
    {
        return;
    }
    if let Some(camera) = &world.validation_camera {
        for (index, handle) in scene.materials.iter().enumerate() {
            match parameters(
                &scene,
                camera,
                cinematic.animation(),
                cinematic.tick(),
                index,
                world.no_fog,
            ) {
                Ok(p) => {
                    if let Some(m) = materials.get(handle)
                        && let Some(mut buffer) = buffers.get_mut(&m.parameters)
                    {
                        p.write_buffer(&mut buffer);
                    }
                }
                Err(e) => {
                    error!("Sword update failed: {}", e);
                    exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
                    return;
                }
            }
        }
    }
    scene.tick = cinematic.tick();
}

pub fn switch(
    mut commands: Commands,
    cinematic: Res<crate::cinematic_camera::Cinematic>,
    mut scene: ResMut<Scene>,
    rendered: Query<Entity, With<Rendered>>,
    mut exit: MessageWriter<AppExit>,
) {
    match scene.select(cinematic.director_index()) {
        Ok(false) => return,
        Ok(true) => {}
        Err(error) => {
            error!("Sword scene selection failed: {}", error);
            exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
            return;
        }
    }
    for entity in &rendered {
        commands.entity(entity).despawn();
    }
    scene.materials.clear();
    scene.tick = i32::MIN;
    info!(
        "cinematic_scene_switch=sword_scene director={} sequence={}",
        cinematic.director_index(),
        cinematic.sequence()
    );
    commands.run_system_cached(setup);
}
