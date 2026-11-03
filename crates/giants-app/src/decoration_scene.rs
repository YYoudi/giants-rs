//! Source-colored well and shrub in the measured mission.
use bevy::{asset::RenderAssetUsages, prelude::*, render::render_resource::PrimitiveTopology};
#[derive(Resource)]
pub struct Scene {
    source: Vec<giants_game::decorations::Decoration>,
}
impl Scene {
    pub fn source_keys(&self) -> impl Iterator<Item = (bool, usize)> + '_ {
        self.source.iter().map(|p| (false, p.source_offset))
    }
    pub fn load(
        root: &std::path::Path,
        plan: &giants_game::instances::Plan,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            source: giants_game::decorations::load_planned(root, plan)?,
        })
    }
}
pub fn setup(
    mut commands: Commands,
    view: Res<crate::source_view::SourceView>,
    scene: Res<Scene>,
    world: Res<crate::world_scene::Scene>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<crate::prop_scene::Material>>,
) {
    let camera = world.validation_camera.as_ref().expect("Decoration camera");
    let source_to_bevy = Mat4::from_cols(Vec4::X, -Vec4::Z, Vec4::Y, Vec4::W);
    for item in &scene.source {
        let matrix = source_to_bevy
            * Mat4::from_cols_array(&item.matrix).transpose()
            * source_to_bevy.inverse();
        let mut textures = std::collections::BTreeMap::new();
        for texture in &item.textures {
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
        for sub in &item.mesh.subobjects {
            let ids: Vec<_> = sub
                .triangles
                .iter()
                .flatten()
                .map(|i| usize::from(*i))
                .collect();
            let positions: Vec<_> = ids
                .iter()
                .map(|i| {
                    let p = item.mesh.positions[item.mesh.indices[*i] as usize];
                    [p[0], p[2], -p[1]]
                })
                .collect();
            let uv: Vec<_> = ids.iter().map(|i| item.mesh.uv[*i]).collect();
            let colors: Vec<_> = ids
                .iter()
                .map(|i| {
                    let rgb = item.mesh.rgb[*i];
                    [
                        rgb[0] as f32 / 255.,
                        rgb[1] as f32 / 255.,
                        rgb[2] as f32 / 255.,
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
            .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors);
            let alpha = item
                .alpha
                .contains(&sub.material.texture.to_ascii_lowercase());
            let material = materials.add(crate::prop_scene::Material {
                instanced: false,
                instance_matrices: view.transforms.clone(),
                multiply: false,
                alpha,
                texture: Some(textures[&sub.material.texture.to_ascii_lowercase()].clone()),
                view: view.buffer.clone(),
                world_from_bevy: matrix,
                pixel_offset: Vec4::new(
                    1. / camera.viewport.z as f32,
                    -1. / camera.viewport.w as f32,
                    if alpha { 1. } else { 0. },
                    0.,
                ),
                fog: Vec4::new(
                    world.asset.config.fog.start,
                    world.asset.config.fog.end,
                    if world.no_fog { 0. } else { 1. },
                    0.,
                ),
                fog_color: Vec4::new(
                    world.asset.config.fog.color[0] as f32 / 255.,
                    world.asset.config.fog.color[1] as f32 / 255.,
                    world.asset.config.fog.color[2] as f32 / 255.,
                    0.,
                ),
            });
            commands.spawn((
                Name::new(format!("Mission decoration{} / {}", item.class, sub.name)),
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(material),
                bevy::camera::visibility::NoFrustumCulling,
            ));
        }
    }
    info!(
        "cinematic_decorations_ready=516,992 meshes={}",
        scene
            .source
            .iter()
            .map(|item| item.mesh.subobjects.len())
            .sum::<usize>()
    );
}
