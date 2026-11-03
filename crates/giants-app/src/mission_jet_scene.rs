//! Renderer for the actual state0 Mecc children, fed only by completed owned
//! inspection frames. It neither advances clocks nor acknowledges readiness.
use crate::rigid_attachment_scene::{self, Material};
use bevy::{asset::RenderAssetUsages, prelude::*, render::storage::ShaderBuffer};
use std::collections::BTreeMap;

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
    model: giants_game::scene_resources::ModelAsset,
    world: Handle<ShaderBuffer>,
    parts: Vec<Part>,
}
struct Mounted {
    ticket: u64,
    objects: Vec<Object>,
    images: Vec<Handle<Image>>,
    visibility: Option<Vec<i32>>,
    number: u64,
}
#[derive(Resource, Default)]
struct State {
    mounted: Option<Mounted>,
    failed: Option<u64>,
}

#[allow(clippy::too_many_arguments)]
fn update(
    mut commands: Commands,
    runtime: Res<crate::mission_runtime::Runtime>,
    controllers: Query<&crate::mission_player::Controller>,
    scene: Option<Res<crate::world_scene::Scene>>,
    view: Res<crate::source_view::SourceView>,
    mut state: ResMut<State>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<Material>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
    mut visibility: Query<&mut Visibility>,
    mut exit: MessageWriter<AppExit>,
) {
    let selected = controllers
        .iter()
        .filter_map(|c| c.owned_jet_frames())
        .find(|v| Some(v.0) == runtime.active_ticket());
    let wanted = selected.map(|v| v.0);
    if state
        .mounted
        .as_ref()
        .is_some_and(|v| Some(v.ticket) != wanted)
    {
        let old = state.mounted.take().unwrap();
        for object in old.objects {
            for part in object.parts {
                commands.entity(part.entity).try_despawn();
                meshes.remove(part.mesh.id());
                materials.remove(part.material.id());
            }
            buffers.remove(object.world.id());
        }
        for image in old.images {
            images.remove(image.id());
        }
        println!(
            "mission_jet_draw_released ticket={} owned_assets_removed=true",
            old.ticket
        );
    }
    let (Some((ticket, number, frames)), Some(scene)) = (selected, scene) else {
        return;
    };
    let Some(camera) = &scene.validation_camera else {
        return;
    };
    if state.failed == Some(ticket) {
        return;
    }
    let result = (|| -> Result<(), String> {
        if frames.len() != 3 {
            return Err("jet draw requires three completed child frames".into());
        }
        if state.mounted.is_none() {
            // Retain partial ownership immediately so failure/retirement can clean up.
            state.mounted = Some(Mounted {
                ticket,
                objects: Vec::new(),
                images: Vec::new(),
                visibility: None,
                number: 0,
            });
            let models = runtime.owned_jet_models(ticket)?;
            for (index, model) in models.into_iter().enumerate() {
                if model.mesh.normal_count != 0 {
                    return Err("jet lit material consumer required".into());
                }
                let mounted = state.mounted.as_mut().unwrap();
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
                    let image = images.add(image);
                    mounted.images.push(image.clone());
                    textures.insert(source.name.to_ascii_lowercase(), image);
                }
                let world = buffers.add(crate::source_view::uniform_buffer(&Mat4::IDENTITY));
                mounted.objects.push(Object {
                    model,
                    world: world.clone(),
                    parts: Vec::new(),
                });
                let object = mounted.objects.last_mut().unwrap();
                for (part_index, part) in object.model.mesh.subobjects.iter().enumerate() {
                    let profile = giants_game::unlit_material::prepare_with_texture_alpha(
                        &part.material,
                        object
                            .model
                            .alpha_textures
                            .contains(&part.material.texture.to_ascii_lowercase()),
                    )?;
                    let ids = part
                        .triangles
                        .iter()
                        .flatten()
                        .map(|v| usize::from(*v))
                        .collect::<Vec<_>>();
                    let mesh = Mesh::new(
                        bevy::render::render_resource::PrimitiveTopology::TriangleList,
                        RenderAssetUsages::default(),
                    )
                    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0.; 3]; ids.len()])
                    .with_inserted_attribute(
                        Mesh::ATTRIBUTE_UV_0,
                        ids.iter()
                            .map(|i| object.model.mesh.uv[*i])
                            .collect::<Vec<_>>(),
                    )
                    .with_inserted_attribute(
                        Mesh::ATTRIBUTE_COLOR,
                        ids.iter()
                            .map(|i| {
                                if profile.vertex_rgb {
                                    let c = object.model.mesh.rgb[*i];
                                    [
                                        c[0] as f32 / 255.,
                                        c[1] as f32 / 255.,
                                        c[2] as f32 / 255.,
                                        1.,
                                    ]
                                } else {
                                    [1.; 4]
                                }
                            })
                            .collect::<Vec<_>>(),
                    );
                    let material = rigid_attachment_scene::owned_material(
                        textures
                            .get(&part.material.texture.to_ascii_lowercase())
                            .ok_or("jet texture absent")?
                            .clone(),
                        view.buffer.clone(),
                        world.clone(),
                        profile,
                        false,
                        Vec4::new(
                            1. / 1920.,
                            -1. / 1080.,
                            scene.asset.config.fog.start,
                            scene.asset.config.fog.end,
                        ),
                        Vec3::from_array(scene.asset.config.fog.color.map(|v| v as f32 / 255.)),
                        !scene.no_fog,
                    );
                    let mesh = meshes.add(mesh);
                    let material = materials.add(material);
                    let entity = commands
                        .spawn((
                            crate::mission_runtime::MissionOwned(ticket),
                            Mesh3d(mesh.clone()),
                            MeshMaterial3d(material.clone()),
                            Transform::default(),
                            Visibility::Hidden,
                            bevy::camera::visibility::NoFrustumCulling,
                            Name::new(format!(
                                "Owned Mecc jet {} / {part_index}",
                                [84, 85, 781][index]
                            )),
                        ))
                        .id();
                    object.parts.push(Part {
                        entity,
                        mesh,
                        material,
                        index: part_index,
                    });
                }
            }
        }
        let mounted = state.mounted.as_mut().unwrap();
        let basis = Mat4::from_cols(Vec4::X, -Vec4::Z, Vec4::Y, Vec4::W);
        let mut visible = Vec::new();
        for (index, (object, frame)) in mounted.objects.iter_mut().zip(frames).enumerate() {
            if frame.pose.class != [84, 85, 781][index]
                || frame.positions.len() != object.model.mesh.positions.len()
            {
                return Err("jet geometry/frame binding differs".into());
            }
            let matrix = giants_game::camera_facing::draw_matrix(
                frame.pose.world.values,
                frame.personality,
                giants_game::camera_facing::Input {
                    view: camera.view,
                    position: [
                        frame.pose.world.values[3],
                        frame.pose.world.values[7],
                        frame.pose.world.values[11],
                    ],
                    scale: frame.scale,
                    angle: frame.angle,
                },
                giants_game::arithmetic::Precision::Double53,
            )?;
            let matrix = basis * Mat4::from_cols_array(&matrix).transpose() * basis.inverse();
            let mut target = buffers
                .get_mut(&object.world)
                .ok_or("jet world buffer removed")?;
            crate::source_view::write_uniform(&matrix, &mut target);
            for part in &object.parts {
                let positions = object.model.mesh.subobjects[part.index]
                    .triangles
                    .iter()
                    .flatten()
                    .map(|i| {
                        let p =
                            frame.positions[object.model.mesh.indices[usize::from(*i)] as usize];
                        [p[0], p[2], -p[1]]
                    })
                    .collect::<Vec<_>>();
                meshes
                    .get_mut(&part.mesh)
                    .ok_or("jet mesh removed")?
                    .insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
                materials
                    .get_mut(&part.material)
                    .ok_or("jet material removed")?
                    .set_draw_view(
                        frame.personality & 4 == 0 && frame.pose.world.flags & 8 != 0,
                        camera.viewport.z as f32,
                        camera.viewport.w as f32,
                    );
                if let Ok(mut value) = visibility.get_mut(part.entity) {
                    *value = if frame.pose.hidden {
                        Visibility::Hidden
                    } else {
                        Visibility::Visible
                    };
                }
            }
            if !frame.pose.hidden {
                visible.push(frame.pose.class);
            }
        }
        if mounted.visibility.as_ref() != Some(&visible) {
            println!(
                "mission_jet_draw_visibility ticket={ticket} frame={number} classes={visible:?} actual_children=true owned_clocks=true world_ready=false"
            );
            mounted.visibility = Some(visible.clone());
        }
        if mounted.number != number {
            println!(
                "mission_jet_draw_frame ticket={ticket} frame={number} children=3 parts={} classes={visible:?} deformed_positions=true actual_bindings=true ready=false",
                mounted.objects.iter().map(|o| o.parts.len()).sum::<usize>()
            );
            mounted.number = number;
        }
        Ok(())
    })();
    if let Err(error) = result {
        state.failed = Some(ticket);
        error!("mission_jet_draw_failed ticket={ticket} error={error}");
        exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
    }
}
