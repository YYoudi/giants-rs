//! Common mission consumer of canonical smoke actors; no simulation or RNG here.
use crate::rigid_attachment_scene::{self, Material};
use bevy::{
    asset::RenderAssetUsages, ecs::system::SystemParam, prelude::*, render::storage::ShaderBuffer,
};
use std::collections::{BTreeMap, BTreeSet};
type Id = giants_game::resident_states::Id;
pub struct Plugin;
impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<MaterialPlugin<Material>>() {
            app.add_plugins(MaterialPlugin::<Material>::default());
        }
        app.init_resource::<State>().add_systems(
            Update,
            update
                .after(crate::mission_player::PlayerUpdate)
                .before(crate::source_view::ViewUpdate),
        );
    }
}
struct Part {
    entity: Entity,
    mesh: Handle<Mesh>,
    material: Handle<Material>,
    index: usize,
}
struct Object {
    passes: u8,
    world: Handle<ShaderBuffer>,
    images: Vec<Handle<Image>>,
    parts: Vec<Part>,
}
#[derive(Resource, Default)]
struct State {
    ticket: Option<u64>,
    objects: BTreeMap<Id, Object>,
    failed: Option<u64>,
    reported: usize,
}
#[derive(SystemParam)]
struct Gpu<'w> {
    meshes: ResMut<'w, Assets<Mesh>>,
    images: ResMut<'w, Assets<Image>>,
    materials: ResMut<'w, Assets<Material>>,
    buffers: ResMut<'w, Assets<ShaderBuffer>>,
}
impl Gpu<'_> {
    fn remove(&mut self, commands: &mut Commands, object: Object) {
        for part in object.parts {
            commands.entity(part.entity).try_despawn();
            self.meshes.remove(part.mesh.id());
            self.materials.remove(part.material.id());
        }
        for image in object.images {
            self.images.remove(image.id());
        }
        self.buffers.remove(object.world.id());
    }
}
fn update(
    mut commands: Commands,
    mut runtime: ResMut<crate::mission_runtime::Runtime>,
    scene: Option<Res<crate::world_scene::Scene>>,
    view: Res<crate::source_view::SourceView>,
    mut state: ResMut<State>,
    mut gpu: Gpu,
    mut visuals: Query<(&mut Visibility, &mut Transform)>,
) {
    let ticket = runtime.active_ticket();
    if state.ticket != ticket {
        for (_, object) in std::mem::take(&mut state.objects) {
            gpu.remove(&mut commands, object);
        }
        state.ticket = ticket;
        state.failed = None;
        state.reported = 0;
    }
    let Some(ticket) = ticket else { return };
    if state.failed == Some(ticket) {
        return;
    }
    let Some(scene) = scene else { return };
    let Some(camera) = &scene.validation_camera else {
        return;
    };
    let result = (|| -> Result<(), String> {
        let Some((actual, draws)) = runtime.smoke_draws(camera.view)? else {
            return Ok(());
        };
        if actual != ticket {
            return Err("smoke draw mission identity changed".into());
        }
        let wanted = draws.iter().map(|d| d.id).collect::<BTreeSet<_>>();
        let removed = state
            .objects
            .keys()
            .filter(|id| !wanted.contains(id))
            .copied()
            .collect::<Vec<_>>();
        for id in removed {
            gpu.remove(&mut commands, state.objects.remove(&id).unwrap());
        }
        let basis = Mat4::from_cols(Vec4::X, -Vec4::Z, Vec4::Y, Vec4::W);
        for draw in draws {
            let model = &draw.geometry.model;
            if state
                .objects
                .get(&draw.id)
                .is_some_and(|o| o.passes != draw.frame.passes)
            {
                gpu.remove(&mut commands, state.objects.remove(&draw.id).unwrap());
            }
            if let std::collections::btree_map::Entry::Vacant(entry) = state.objects.entry(draw.id)
            {
                let world = gpu
                    .buffers
                    .add(crate::source_view::uniform_buffer(&Mat4::IDENTITY));
                let object = entry.insert(Object {
                    passes: draw.frame.passes,
                    world: world.clone(),
                    images: vec![],
                    parts: vec![],
                });
                let mut textures = BTreeMap::new();
                for source in &model.textures {
                    let mut image = Image::new(
                        bevy::render::render_resource::Extent3d {
                            width: source.width,
                            height: source.height,
                            depth_or_array_layers: 1,
                        },
                        bevy::render::render_resource::TextureDimension::D2,
                        source.rgba.clone(),
                        bevy::render::render_resource::TextureFormat::Rgba8Unorm,
                        RenderAssetUsages::default(),
                    );
                    image.sampler = crate::logo_material::sampler();
                    let image = gpu.images.add(image);
                    object.images.push(image.clone());
                    textures.insert(source.name.to_ascii_lowercase(), image);
                }
                for pass in 0..draw.frame.passes {
                    for (index, part) in model.mesh.subobjects.iter().enumerate() {
                        let ids = part
                            .triangles
                            .iter()
                            .flatten()
                            .map(|i| usize::from(*i))
                            .collect::<Vec<_>>();
                        let mesh = Mesh::new(
                            bevy::render::render_resource::PrimitiveTopology::TriangleList,
                            RenderAssetUsages::default(),
                        )
                        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0.; 3]; ids.len()])
                        .with_inserted_attribute(
                            Mesh::ATTRIBUTE_UV_0,
                            ids.iter().map(|i| model.mesh.uv[*i]).collect::<Vec<_>>(),
                        )
                        .with_inserted_attribute(
                            Mesh::ATTRIBUTE_COLOR,
                            ids.iter()
                                .map(|i| {
                                    let c = model.mesh.rgb[*i];
                                    [
                                        c[0] as f32 / 255.,
                                        c[1] as f32 / 255.,
                                        c[2] as f32 / 255.,
                                        1.,
                                    ]
                                })
                                .collect::<Vec<_>>(),
                        );
                        let profile = *draw
                            .frame
                            .profiles
                            .get(index)
                            .ok_or("smoke material profile absent")?;
                        let material = rigid_attachment_scene::owned_material(
                            textures
                                .get(&part.material.texture.to_ascii_lowercase())
                                .ok_or("smoke named texture absent")?
                                .clone(),
                            view.buffer.clone(),
                            world.clone(),
                            profile,
                            draw.frame.front,
                            Vec4::new(
                                1. / camera.viewport.z as f32,
                                -1. / camera.viewport.w as f32,
                                scene.asset.config.fog.start,
                                scene.asset.config.fog.end,
                            ),
                            Vec3::from_array(scene.asset.config.fog.color.map(|v| v as f32 / 255.)),
                            !scene.no_fog,
                        );
                        let mesh = gpu.meshes.add(mesh);
                        let material = gpu.materials.add(material);
                        let entity = commands
                            .spawn((
                                crate::mission_runtime::MissionOwned(ticket),
                                Mesh3d(mesh.clone()),
                                MeshMaterial3d(material.clone()),
                                Transform::from_translation(Vec3::new(
                                    draw.frame.position[0],
                                    draw.frame.position[2],
                                    -draw.frame.position[1],
                                )),
                                Visibility::Visible,
                                bevy::camera::visibility::NoFrustumCulling,
                                Name::new(format!(
                                    "Owned smoke {} part {index} pass {pass}",
                                    draw.geometry.class
                                )),
                            ))
                            .id();
                        object.parts.push(Part {
                            entity,
                            mesh,
                            material,
                            index,
                        });
                    }
                }
            }
            let object = state.objects.get_mut(&draw.id).unwrap();
            let matrix =
                basis * Mat4::from_cols_array(&draw.frame.matrix).transpose() * basis.inverse();
            let mut target = gpu
                .buffers
                .get_mut(&object.world)
                .ok_or("smoke world buffer removed")?;
            crate::source_view::write_uniform(&matrix, &mut target);
            for part in &object.parts {
                let source = &model.mesh.subobjects[part.index];
                let positions = source
                    .triangles
                    .iter()
                    .flatten()
                    .map(|i| {
                        let p = draw.frame.positions[model.mesh.indices[usize::from(*i)] as usize];
                        [p[0], p[2], -p[1]]
                    })
                    .collect::<Vec<_>>();
                gpu.meshes
                    .get_mut(&part.mesh)
                    .ok_or("smoke mesh removed")?
                    .insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
                let mut material = gpu
                    .materials
                    .get_mut(&part.material)
                    .ok_or("smoke material removed")?;
                material.set_owned_profile(draw.frame.profiles[part.index]);
                material.set_draw_view(
                    draw.frame.front,
                    camera.viewport.z as f32,
                    camera.viewport.w as f32,
                );
                if let Ok((mut visibility, mut transform)) = visuals.get_mut(part.entity) {
                    *visibility = Visibility::Visible;
                    // Shader uses its explicit world uniform. This translation
                    // supplies the engine's transparent-sort center only.
                    transform.translation = Vec3::new(
                        draw.frame.position[0],
                        draw.frame.position[2],
                        -draw.frame.position[1],
                    );
                }
            }
        }
        if state.reported != state.objects.len() {
            state.reported = state.objects.len();
            println!(
                "mission_smoke_draw ticket={ticket} objects={} canonical_actors=true simulation_in_renderer=false native_queue_order_verified=false native_pixels_verified=false ready=false",
                state.reported
            );
        }
        Ok(())
    })();
    if let Err(error) = result {
        for (_, object) in std::mem::take(&mut state.objects) {
            gpu.remove(&mut commands, object);
        }
        state.failed = Some(ticket);
        error!("mission_smoke_draw_refused ticket={ticket}: {error}");
    }
}

#[cfg(test)]
pub(crate) fn check_render_assets(
    runtime: crate::mission_runtime::Runtime,
    asset: std::sync::Arc<giants_game::WorldAsset>,
    view: [f32; 16],
) -> crate::mission_runtime::Runtime {
    let mut app = App::new();
    app.insert_resource(runtime)
        .insert_resource(crate::world_scene::Scene {
            asset,
            oblique: false,
            validation_camera: Some(crate::validation_camera::ValidationCamera {
                view,
                projection: giants_game::procedural_aim::Matrix::identity().values,
                viewport: UVec4::new(0, 0, 640, 360),
            }),
            camera_source: "owned smoke CPU asset control",
            camera_origin: None,
            cinematic_capture_ready: false,
            no_fog: false,
            render_terrain: false,
            render_sky: false,
            render_underside: false,
            reflection_profile: false,
            water: None,
            render_water: false,
        })
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<Image>>()
        .init_resource::<Assets<Material>>()
        .init_resource::<Assets<ShaderBuffer>>()
        .init_resource::<State>()
        .init_resource::<crate::source_view::SourceView>()
        .add_systems(Update, update);
    let base = app.world().resource::<Assets<ShaderBuffer>>().len();
    app.update();
    app.update();
    assert_eq!(app.world().resource::<State>().objects.len(), 1);
    assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 1);
    assert_eq!(app.world().resource::<Assets<Material>>().len(), 1);
    assert_eq!(app.world().resource::<Assets<Image>>().len(), 1);
    assert_eq!(
        app.world().resource::<Assets<ShaderBuffer>>().len(),
        base + 1
    );
    let runtime = app
        .world_mut()
        .remove_resource::<crate::mission_runtime::Runtime>()
        .unwrap();
    app.init_resource::<crate::mission_runtime::Runtime>();
    app.update();
    assert!(app.world().resource::<State>().objects.is_empty());
    assert!(app.world().resource::<Assets<Mesh>>().is_empty());
    assert!(app.world().resource::<Assets<Material>>().is_empty());
    assert!(app.world().resource::<Assets<Image>>().is_empty());
    assert_eq!(app.world().resource::<Assets<ShaderBuffer>>().len(), base);
    assert_eq!(
        app.world_mut()
            .query::<&crate::mission_runtime::MissionOwned>()
            .iter(app.world())
            .count(),
        0
    );
    runtime
}
