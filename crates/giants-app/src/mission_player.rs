//! Ticket-owned body/weapon drawing and selected-mission controller.
//! Opening handoff, continuity and team remain incomplete; no readiness event.
use crate::{
    actor_scene,
    mission_runtime::{PlayerInitial, Runtime},
};
use bevy::{asset::RenderAssetUsages, prelude::*};
use std::collections::BTreeMap;
mod controller;
pub(crate) use controller::Controller;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlayerUpdate;
pub struct Plugin;
impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<MaterialPlugin<actor_scene::Material>>() {
            app.add_plugins(MaterialPlugin::<actor_scene::Material>::default());
        }
        if !app.is_plugin_added::<MaterialPlugin<actor_scene::LitMaterial>>() {
            app.add_plugins(MaterialPlugin::<actor_scene::LitMaterial>::default());
        }
        app.init_resource::<State>()
            .add_systems(
                Update,
                (sync, controller::update)
                    .chain()
                    .in_set(PlayerUpdate)
                    .after(crate::mission_environment::EnvironmentUpdate)
                    .before(crate::source_view::ViewUpdate)
                    .run_if(crate::mecc_session::frontend_available),
            )
            .add_systems(
                Update,
                opening_visibility
                    .after(PlayerUpdate)
                    .after(crate::mission_opening::OpeningUpdate),
            );
    }
}
#[derive(Component)]
pub struct DrawnPlayer {
    pub ticket: u64,
    pub slot: usize,
    pub part: usize,
}
#[derive(Component)]
pub(super) struct DrawnArms {
    ticket: u64,
    part: usize,
    buffer: Handle<bevy::render::storage::ShaderBuffer>,
}
type OpeningDraw<'a> = (
    &'a mut Visibility,
    Option<&'a DrawnPlayer>,
    Option<&'a controller::HeldPart>,
    Option<&'a DrawnArms>,
);
fn opening_visibility(
    owner: Option<Res<crate::mission_opening::Owner>>,
    cinematic: Option<Res<crate::cinematic_camera::Cinematic>>,
    runtime: Res<Runtime>,
    mut draws: Query<OpeningDraw<'_>>,
) {
    if let Some((ticket, _)) = runtime.kabuto_body() {
        let hidden = cinematic.is_some() && owner.as_ref().is_some_and(|owner| owner.0 == ticket);
        let wanted = if hidden {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
        for (mut visibility, player, _, _) in &mut draws {
            if player.is_some_and(|p| p.ticket == ticket) && *visibility != wanted {
                *visibility = wanted;
            }
        }
    }
    if cinematic.is_none() {
        return;
    }
    let Some(owner) = owner else { return };
    for (mut visibility, player, held, arms) in &mut draws {
        if player.is_some_and(|p| p.ticket == owner.0)
            || held.is_some_and(|p| p.ticket == owner.0)
            || arms.is_some_and(|p| p.ticket == owner.0)
        {
            *visibility = Visibility::Hidden;
        }
    }
}
struct Mounted {
    ticket: u64,
    entities: Vec<Entity>,
    meshes: Vec<Handle<Mesh>>,
    images: Vec<Handle<Image>>,
    materials: Vec<Handle<actor_scene::Material>>,
    lit: Vec<Handle<actor_scene::LitMaterial>>,
    buffers: Vec<Handle<bevy::render::storage::ShaderBuffer>>,
}
#[derive(Resource, Default)]
struct State {
    mounted: Option<Mounted>,
}
pub(crate) fn resident(world: &mut World, ticket: u64) -> bool {
    let Some(state) = world.get_resource::<State>() else {
        return false;
    };
    let Some(mounted) = state.mounted.as_ref().filter(|m| m.ticket == ticket) else {
        return false;
    };
    if mounted.entities.is_empty()
        || !mounted
            .entities
            .iter()
            .all(|e| world.get::<Mesh3d>(*e).is_some())
    {
        return false;
    }
    world
        .query::<(&PlayerInitial, &Controller)>()
        .iter(world)
        .any(|(p, c)| p.ticket == ticket && c.prepared(ticket))
}

#[allow(clippy::too_many_arguments)]
fn sync(
    mut commands: Commands,
    runtime: Res<Runtime>,
    players: Query<&PlayerInitial>,
    scene: Option<Res<crate::world_scene::Scene>>,
    view: Res<crate::source_view::SourceView>,
    mut state: ResMut<State>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<actor_scene::Material>>,
    mut lit: ResMut<Assets<actor_scene::LitMaterial>>,
    mut buffers: ResMut<Assets<bevy::render::storage::ShaderBuffer>>,
) {
    let selected = players
        .iter()
        .find(|p| Some(p.ticket) == runtime.active_ticket());
    let kabuto = runtime.kabuto_body();
    let wanted = selected
        .map(|p| p.ticket)
        .or_else(|| kabuto.map(|(ticket, _)| ticket));
    if state
        .mounted
        .as_ref()
        .is_some_and(|m| Some(m.ticket) != wanted)
    {
        let old = state.mounted.take().unwrap();
        for entity in old.entities {
            commands.entity(entity).try_despawn();
        }
        for handle in &old.materials {
            materials.remove(handle.id());
        }
        for handle in &old.lit {
            lit.remove(handle.id());
        }
        for handle in &old.buffers {
            buffers.remove(handle.id());
        }
        for handle in &old.meshes {
            meshes.remove(handle.id());
        }
        for handle in &old.images {
            images.remove(handle.id());
        }
        assert!(old.materials.iter().all(|h| !materials.contains(h.id())));
        assert!(old.lit.iter().all(|h| !lit.contains(h.id())));
        assert!(old.buffers.iter().all(|h| !buffers.contains(h.id())));
        assert!(old.meshes.iter().all(|h| !meshes.contains(h.id())));
        assert!(old.images.iter().all(|h| !images.contains(h.id())));
        println!(
            "mission_player_draw_released ticket={} meshes={} images={} materials={} buffers={} remaining_assets=0",
            old.ticket,
            old.meshes.len(),
            old.images.len(),
            old.materials.len() + old.lit.len(),
            old.buffers.len()
        );
    }
    if let (Some((_, body)), Some(scene), Some(mounted)) =
        (kabuto, scene.as_ref(), state.mounted.as_ref())
        && scene.is_changed()
        && let Some(camera) = scene.validation_camera.as_ref()
        && let Ok(lights) = giants_game::lighting::first_world(&scene.asset.config)
    {
        for (part, handle) in body.model.mesh.subobjects.iter().zip(&mounted.buffers) {
            if let Some(mut buffer) = buffers.get_mut(handle) {
                *buffer = crate::gun_scene::source_parameters(
                    &part.material,
                    Mat4::from_cols_array(&body.matrix).transpose(),
                    camera,
                    &lights,
                    scene.asset.config.fog,
                    scene.no_fog,
                )
                .buffer();
            }
        }
    }
    if state.mounted.is_some() {
        return;
    }
    let (Some(ticket), Some(scene)) = (wanted, scene) else {
        return;
    };
    if !matches!(
        scene.camera_source,
        "mission_candidate" | "mission_player_inspection" | "declared_start_debug_camera"
    ) {
        return;
    }
    let kabuto_lights = if kabuto.is_some() {
        if scene.validation_camera.is_none() {
            return;
        }
        match giants_game::lighting::first_world(&scene.asset.config) {
            Ok(lights) => lights,
            Err(error) => {
                error!("Kabuto body lighting refused: {error}");
                return;
            }
        }
    } else {
        Vec::new()
    };
    let p = selected.map(|player| &player.prepared);
    let mut mounted = Mounted {
        ticket,
        entities: Vec::new(),
        meshes: Vec::new(),
        images: Vec::new(),
        materials: Vec::new(),
        lit: Vec::new(),
        buffers: Vec::new(),
    };
    let owned_models = if p.is_some()
        && std::env::var("GIANTS_MISSION_MECC_BACKEND").as_deref() == Ok("owned-inspection")
    {
        match runtime.owned_body_models(ticket) {
            Ok(models) => models,
            Err(error) => {
                error!("Owned draw models refused: {error}");
                return;
            }
        }
    } else {
        Vec::new()
    };
    let variants = if let Some((_, body)) = kabuto {
        vec![(
            0,
            &body.model.mesh,
            &body.model.textures,
            &body.model.alpha_textures,
        )]
    } else if owned_models.is_empty() {
        let p = p.expect("selected Mecc body");
        vec![(0, &p.body.mesh, &p.body.textures, &p.body.alpha_textures)]
    } else {
        owned_models
            .iter()
            .map(|(slot, m)| (*slot, &m.mesh, &m.textures, &m.alpha_textures))
            .collect()
    };
    for (body_slot, body_mesh, body_textures, body_alpha) in variants {
        let mut textures = BTreeMap::new();
        for source in body_textures {
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
            let handle = images.add(image);
            textures.insert(source.name.to_ascii_lowercase(), handle.clone());
            mounted.images.push(handle);
        }
        for (part_index, part) in body_mesh.subobjects.iter().enumerate() {
            let ids: Vec<_> = part
                .triangles
                .iter()
                .flatten()
                .map(|i| usize::from(*i))
                .collect();
            let colors: Vec<_> = ids
                .iter()
                .map(|i| {
                    if part.material.flags & 0x20 != 0 {
                        let rgb = body_mesh.rgb[*i];
                        [
                            rgb[0] as f32 / 255.,
                            rgb[1] as f32 / 255.,
                            rgb[2] as f32 / 255.,
                            1.,
                        ]
                    } else {
                        [1.; 4]
                    }
                })
                .collect();
            let mesh = Mesh::new(
                bevy::render::render_resource::PrimitiveTopology::TriangleList,
                RenderAssetUsages::default(),
            )
            .with_inserted_attribute(
                Mesh::ATTRIBUTE_POSITION,
                if let Some((_, body)) = kabuto {
                    ids.iter()
                        .map(|i| body.positions[usize::from(body_mesh.indices[*i])])
                        .collect()
                } else if body_slot == 0 {
                    p.unwrap().vertices(part_index)
                } else {
                    ids.iter()
                        .map(|i| {
                            let q = body_mesh.positions[usize::from(body_mesh.indices[*i])];
                            let m = p.unwrap().matrix;
                            let q = std::array::from_fn::<_, 3, _>(|r| {
                                m[r * 4] * q[0]
                                    + m[r * 4 + 1] * q[1]
                                    + m[r * 4 + 2] * q[2]
                                    + m[r * 4 + 3]
                            });
                            [q[0], q[2], -q[1]]
                        })
                        .collect::<Vec<_>>()
                },
            )
            .with_inserted_attribute(
                Mesh::ATTRIBUTE_UV_0,
                ids.iter().map(|i| body_mesh.uv[*i]).collect::<Vec<_>>(),
            )
            .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors);
            if let Some((_, body)) = kabuto {
                let mesh = meshes.add(mesh.with_inserted_attribute(
                    Mesh::ATTRIBUTE_NORMAL,
                    ids.iter().map(|i| body.normals[*i]).collect::<Vec<_>>(),
                ));
                let camera = scene.validation_camera.as_ref().unwrap();
                let parameters = crate::gun_scene::source_parameters(
                    &part.material,
                    Mat4::from_cols_array(&body.matrix).transpose(),
                    camera,
                    &kabuto_lights,
                    scene.asset.config.fog,
                    scene.no_fog,
                );
                let buffer = buffers.add(parameters.buffer());
                let material = lit.add(actor_scene::shared_lit_material(
                    textures[&part.material.texture.to_ascii_lowercase()].clone(),
                    buffer.clone(),
                    body_alpha.contains(&part.material.texture.to_ascii_lowercase()),
                ));
                let entity = commands
                    .spawn((
                        DrawnPlayer {
                            ticket,
                            part: part_index,
                            slot: body_slot,
                        },
                        crate::mission_runtime::MissionOwned(ticket),
                        Name::new(format!("Mission initial Kabuto / {}", part.name)),
                        Mesh3d(mesh.clone()),
                        MeshMaterial3d(material.clone()),
                        bevy::camera::visibility::NoFrustumCulling,
                    ))
                    .id();
                mounted.entities.push(entity);
                mounted.meshes.push(mesh);
                mounted.lit.push(material);
                mounted.buffers.push(buffer);
                continue;
            }
            let mesh = meshes.add(mesh);
            let alpha = if part.material.flags & 1 != 0 {
                ((1. - part.material.falloff) * 255.) as u8 as f32 / 255.
            } else {
                1.
            };
            let fog = scene.asset.config.fog;
            let material = materials.add(actor_scene::Material::from(actor_scene::Bindings {
                texture: textures[&part.material.texture.to_ascii_lowercase()].clone(),
                clip_from_bevy: view.buffer.clone(),
                validation: Vec4::new(1., 1. / 1920., -1. / 1080., alpha),
                fog: Vec4::new(fog.start, fog.end, 1., 0.),
                fog_color: if part.material.flags & 1 != 0 {
                    Vec4::ZERO
                } else {
                    Vec4::new(
                        fog.color[0] as f32 / 255.,
                        fog.color[1] as f32 / 255.,
                        fog.color[2] as f32 / 255.,
                        0.,
                    )
                },
                force_additive: false,
                blend: body_alpha.contains(&part.material.texture.to_ascii_lowercase()),
            }));
            let entity = commands
                .spawn((
                    DrawnPlayer {
                        ticket,
                        part: part_index,
                        slot: body_slot,
                    },
                    crate::mission_runtime::MissionOwned(ticket),
                    Name::new(format!("Mission initial Mecc / {}", part.name)),
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(material.clone()),
                    if body_slot == 0 {
                        Visibility::Visible
                    } else {
                        Visibility::Hidden
                    },
                    bevy::camera::visibility::NoFrustumCulling,
                ))
                .id();
            mounted.entities.push(entity);
            mounted.meshes.push(mesh);
            mounted.materials.push(material);
        }
    }
    let (Some(player), Some(p)) = (selected, p) else {
        println!(
            "mission_kabuto_body_draw_staged ticket={ticket} meshes={} materials={} current_channel_snapshot=true live_animation=false controller=false ready=false",
            mounted.meshes.len(),
            mounted.lit.len()
        );
        state.mounted = Some(mounted);
        return;
    };
    // The selected cold equipment resolves through the common icon/class and
    // animated-hand consumers. This is a fixed initial attachment, not firing.
    let weapon = &p.weapon;
    let mut weapon_textures = BTreeMap::new();
    for source in &weapon.model.textures {
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
        let handle = images.add(image);
        weapon_textures.insert(source.name.to_ascii_lowercase(), handle.clone());
        mounted.images.push(handle);
    }
    let lights = &p.lights;
    let camera = scene
        .validation_camera
        .as_ref()
        .expect("mission inspection camera");
    match &p.subjective {
        Ok(arms) => {
            let mut arm_textures = BTreeMap::new();
            for source in &arms.model.textures {
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
                let handle = images.add(image);
                arm_textures.insert(source.name.to_ascii_lowercase(), handle.clone());
                mounted.images.push(handle);
            }
            for (index, part) in arms.model.mesh.subobjects.iter().enumerate() {
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
                .with_inserted_attribute(
                    Mesh::ATTRIBUTE_POSITION,
                    ids.iter()
                        .map(|id| arms.initial_positions[usize::from(arms.model.mesh.indices[*id])])
                        .collect::<Vec<_>>(),
                )
                .with_inserted_attribute(
                    Mesh::ATTRIBUTE_NORMAL,
                    ids.iter()
                        .map(|id| arms.initial_normals[*id])
                        .collect::<Vec<_>>(),
                )
                .with_inserted_attribute(
                    Mesh::ATTRIBUTE_UV_0,
                    ids.iter()
                        .map(|id| arms.model.mesh.uv[*id])
                        .collect::<Vec<_>>(),
                );
                let mesh = meshes.add(mesh);
                let buffer = buffers.add(
                    crate::gun_scene::source_parameters(
                        &part.material,
                        Mat4::IDENTITY,
                        camera,
                        lights,
                        scene.asset.config.fog,
                        scene.no_fog,
                    )
                    .buffer(),
                );
                let material = lit.add(actor_scene::shared_lit_material(
                    arm_textures[&part.material.texture.to_ascii_lowercase()].clone(),
                    buffer.clone(),
                    arms.model
                        .alpha_textures
                        .contains(&part.material.texture.to_ascii_lowercase()),
                ));
                let entity = commands
                    .spawn((
                        crate::mission_runtime::MissionOwned(player.ticket),
                        DrawnArms {
                            ticket: player.ticket,
                            part: index,
                            buffer: buffer.clone(),
                        },
                        Name::new(format!("Mission subjective arms / {}", part.name)),
                        Mesh3d(mesh.clone()),
                        MeshMaterial3d(material.clone()),
                        Visibility::Hidden,
                        bevy::camera::visibility::NoFrustumCulling,
                    ))
                    .id();
                mounted.entities.push(entity);
                mounted.meshes.push(mesh);
                mounted.lit.push(material);
                mounted.buffers.push(buffer);
            }
            println!(
                "mission_subjective_bound ticket={} parts={} textures={} offset={:?} icon_index={} pass=shared_lit source_placement=true actor_backend_ready=false",
                player.ticket,
                arms.model.mesh.subobjects.len(),
                arms.model.textures.len(),
                arms.offset,
                arms.icon_index
            );
        }
        Err(error) => println!(
            "mission_subjective_resources_refused ticket={} reason={error:?} actor_backend_ready=false",
            player.ticket
        ),
    }
    for (index, part) in weapon.model.mesh.subobjects.iter().enumerate() {
        let ids: Vec<_> = part
            .triangles
            .iter()
            .flatten()
            .map(|i| usize::from(*i))
            .collect();
        let mesh = Mesh::new(
            bevy::render::render_resource::PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_POSITION,
            ids.iter()
                .map(|i| weapon.model.mesh.positions[usize::from(weapon.model.mesh.indices[*i])])
                .collect::<Vec<_>>(),
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_NORMAL,
            ids.iter().map(|i| p.weapon_normals[*i]).collect::<Vec<_>>(),
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_UV_0,
            ids.iter()
                .map(|i| weapon.model.mesh.uv[*i])
                .collect::<Vec<_>>(),
        );
        let mesh = meshes.add(mesh);
        let parameters = crate::gun_scene::source_parameters(
            &part.material,
            Mat4::from_cols_array(&p.weapon_matrix).transpose(),
            camera,
            lights,
            scene.asset.config.fog,
            scene.no_fog,
        );
        let buffer = buffers.add(parameters.buffer());
        let material = lit.add(actor_scene::shared_lit_material(
            weapon_textures[&part.material.texture.to_ascii_lowercase()].clone(),
            buffer.clone(),
            false,
        ));
        let entity = commands
            .spawn((
                crate::mission_runtime::MissionOwned(player.ticket),
                controller::HeldPart {
                    ticket: player.ticket,
                    index,
                    buffer: buffer.clone(),
                },
                Name::new(format!("Mission initial held weapon / {}", part.name)),
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material.clone()),
                bevy::camera::visibility::NoFrustumCulling,
            ))
            .id();
        mounted.entities.push(entity);
        mounted.meshes.push(mesh);
        mounted.lit.push(material);
        mounted.buffers.push(buffer);
    }
    println!(
        "mission_player_draw_staged ticket={} meshes={} images={} materials={} pose=initial_slot1 lod=explicit_zero equipment={:?} requested_companions={} weapon_drawing=initial_attachment animation_clock=false controller=false ready=false",
        player.ticket,
        mounted.meshes.len(),
        mounted.images.len(),
        mounted.materials.len() + mounted.lit.len(),
        p.equipment.weapons,
        p.requested_companions
    );
    state.mounted = Some(mounted);
}
