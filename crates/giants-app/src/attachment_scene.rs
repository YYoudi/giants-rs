//! The measured first staff attachment; the gun uses its own lighting profile.
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

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct Material {
    #[texture(0)]
    #[sampler(1)]
    pub texture: Handle<Image>,
    #[uniform(2)]
    pub clip_from_bevy: Mat4,
    #[uniform(3)]
    pub world_from_bevy: Mat4,
    #[uniform(4)]
    pub pixel_offset: Vec4,
}
impl bevy::pbr::Material for Material {
    fn vertex_shader() -> ShaderRef {
        "embedded://giants_app/attachment.wesl".into()
    }
    fn fragment_shader() -> ShaderRef {
        "embedded://giants_app/attachment.wesl".into()
    }
    fn specialize(
        _: &MaterialPipeline,
        d: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        d.vertex.buffers = vec![layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(1),
            Mesh::ATTRIBUTE_COLOR.at_shader_location(2),
        ])?];
        d.primitive.cull_mode = Some(bevy::render::render_resource::Face::Front);
        Ok(())
    }
}
pub type Scene = crate::cinematic_scene::Scenes<Staff>;
pub struct Staff {
    staff: Option<giants_game::attachments::Attachment>,
    textures: Vec<giants_game::LogoTexture>,
    materials: Vec<Handle<Material>>,
    tick: i32,
    matrix: Mat4,
}
fn world_matrix(matrix: [f32; 16]) -> Mat4 {
    let to_source = Mat4::from_cols(Vec4::X, Vec4::Z, -Vec4::Y, Vec4::W);
    let to_bevy = Mat4::from_cols(Vec4::X, -Vec4::Z, Vec4::Y, Vec4::W);
    to_bevy * Mat4::from_cols_array(&matrix).transpose() * to_source
}
impl Scene {
    pub fn owned_slots(&self, director: usize) -> Vec<usize> {
        self.entry(director)
            .and_then(|s| s.staff.as_ref())
            .filter(|a| !a.mesh.subobjects.is_empty())
            .map(|a| vec![a.slot])
            .unwrap_or_default()
    }
    pub fn load(
        root: &std::path::Path,
        cinematic: &crate::cinematic_camera::Cinematic,
        slot: Option<usize>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Self::prepare(cinematic, |segment| Staff::load(root, segment, slot))
    }
}
#[derive(Component)]
pub struct Rendered;
impl Staff {
    fn load(
        root: &std::path::Path,
        cinematic: &crate::cinematic_camera::Segment,
        slot: Option<usize>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let staff = if slot.is_none_or(|s| s == 6) {
            (if cinematic.preview() {
                giants_game::attachments::load_sequence_preview
            } else {
                giants_game::attachments::load_sequence
            })(root, cinematic.sequence())?
            .into_iter()
            .find(|a| a.slot == 6 && a.model.eq_ignore_ascii_case("sm_staff"))
        } else {
            None
        };
        let (textures, matrix) = if let Some(staff) = &staff {
            if staff
                .mesh
                .subobjects
                .iter()
                .any(|s| s.material.flags != 0x40000020)
            {
                return Err("Staff material outside measured profile".into());
            }
            let (matrix, flags) = staff.matrix(cinematic.animation(), cinematic.tick())?;
            if flags & 8 == 0 {
                return Err("Staff mirror outside measured profile".into());
            }
            (staff.textures(root)?, world_matrix(matrix))
        } else {
            (Vec::new(), Mat4::IDENTITY)
        };
        println!(
            "cinematic_staff={} staff_material=source_rgb attachment_source=flk_catalog_anm",
            staff.is_some()
        );
        Ok(Self {
            staff,
            textures,
            materials: Vec::new(),
            tick: cinematic.tick(),
            matrix,
        })
    }
}
pub fn setup(
    mut commands: Commands,
    mut scene: ResMut<Scene>,
    world: Res<crate::world_scene::Scene>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<Material>>,
    cinematic: Res<crate::cinematic_camera::Cinematic>,
) {
    let Some(staff) = &scene.staff else {
        return;
    };
    let camera = world
        .validation_camera
        .as_ref()
        .expect("staff requires cinematic camera");
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
        image.sampler = crate::logo_material::sampler();
        textures.insert(texture.name.to_ascii_lowercase(), images.add(image));
    }
    let mut handles = Vec::new();
    for sub in &staff.mesh.subobjects {
        let indices: Vec<_> = sub
            .triangles
            .iter()
            .flatten()
            .map(|i| usize::from(*i))
            .collect();
        let positions: Vec<_> = indices
            .iter()
            .map(|i| {
                let p = staff.mesh.positions[staff.mesh.indices[*i] as usize];
                [p[0], p[2], -p[1]]
            })
            .collect();
        let uv: Vec<_> = indices.iter().map(|i| staff.mesh.uv[*i]).collect();
        let rgb: Vec<_> = indices
            .iter()
            .map(|i| {
                let c = staff.mesh.rgb[*i];
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
            texture: textures[&sub.material.texture.to_ascii_lowercase()].clone(),
            clip_from_bevy: camera.clip_from_bevy(),
            world_from_bevy: scene.matrix,
            pixel_offset: Vec4::new(
                1. / camera.viewport.z as f32,
                -1. / camera.viewport.w as f32,
                0.,
                0.,
            ),
        });
        handles.push(material.clone());
        commands.spawn((
            Rendered,
            crate::cinematic_visibility::ActorSlot(6),
            Name::new(format!("FLK staff slot 6 / {}", sub.name)),
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(material),
            bevy::camera::visibility::NoFrustumCulling,
        ));
    }
    scene.materials = handles;
    info!(
        "cinematic_render_ready=attachment_scene director={} meshes={}",
        cinematic.director_index(),
        scene.materials.len()
    );
}
pub fn update(
    cinematic: Res<crate::cinematic_camera::Cinematic>,
    world: Res<crate::world_scene::Scene>,
    mut scene: ResMut<Scene>,
    mut materials: ResMut<Assets<Material>>,
    mut exit: MessageWriter<AppExit>,
    rendered: Query<Entity, With<Rendered>>,
) {
    if rendered.iter().count() != scene.materials.len() {
        error!("Director render entity count differs: attachment_scene");
        exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
        return;
    }

    if scene.tick == cinematic.tick() && !world.is_changed() {
        return;
    }
    let Some(staff) = &scene.staff else {
        return;
    };
    let matrix = match staff.matrix(cinematic.animation(), cinematic.tick()) {
        Ok((matrix, flags)) if flags & 8 != 0 => world_matrix(matrix),
        result => {
            error!("Staff attachment outside measured profile: {:?}", result);
            exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
            return;
        }
    };
    if let Some(camera) = &world.validation_camera {
        for handle in &scene.materials {
            if let Some(mut m) = materials.get_mut(handle) {
                m.world_from_bevy = matrix;
                m.clip_from_bevy = camera.clip_from_bevy();
                m.pixel_offset.x = 1. / camera.viewport.z as f32;
                m.pixel_offset.y = -1. / camera.viewport.w as f32;
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
            error!("Director scene selection failed: {}", error);
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
        "cinematic_scene_switch=attachment_scene director={} sequence={}",
        cinematic.director_index(),
        cinematic.sequence()
    );
    commands.run_system_cached(setup);
}
