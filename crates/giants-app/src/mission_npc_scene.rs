//! Consume completed canonical Vimp poses in the common mission renderer.
//! No actor birth, AI, animation clock or random draw occurs here.
use crate::actor_scene::{self, Material};
use crate::rigid_attachment_scene::Material as AuxiliaryMaterial;
use bevy::{asset::RenderAssetUsages, ecs::system::SystemParam, prelude::*};
use std::collections::{BTreeMap, BTreeSet};
mod rigid;

pub struct Plugin;
impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<MaterialPlugin<Material>>() {
            app.add_plugins(MaterialPlugin::<Material>::default());
        }
        if !app.is_plugin_added::<MaterialPlugin<AuxiliaryMaterial>>() {
            app.add_plugins(MaterialPlugin::<AuxiliaryMaterial>::default());
        }
        if !app.is_plugin_added::<MaterialPlugin<crate::scenery_scene::Material>>() {
            app.add_plugins(MaterialPlugin::<crate::scenery_scene::Material>::default());
        }
        if !app.is_plugin_added::<MaterialPlugin<crate::sword_scene::Material>>() {
            app.add_plugins(MaterialPlugin::<crate::sword_scene::Material>::default());
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
    material: PartMaterial,
    index: usize,
}
struct Object {
    rigid: bool,
    textures: BTreeMap<String, Handle<Image>>,
    world: Option<Handle<bevy::render::storage::ShaderBuffer>>,
    geometry: std::sync::Arc<giants_game::scene_resources::ModelAsset>,
    images: Vec<Handle<Image>>,
    parts: Vec<Part>,
    frame: u64,
}
#[derive(Resource, Default)]
struct State {
    ticket: Option<u64>,
    objects: BTreeMap<crate::mission_runtime::NpcDrawOwner, Object>,
    failed: Option<String>,
    reported: usize,
}
#[derive(SystemParam)]
struct Gpu<'w> {
    meshes: ResMut<'w, Assets<Mesh>>,
    images: ResMut<'w, Assets<Image>>,
    materials: ResMut<'w, Assets<Material>>,
    auxiliary_materials: ResMut<'w, Assets<AuxiliaryMaterial>>,
    rigid_lit: ResMut<'w, Assets<crate::scenery_scene::Material>>,
    rigid_cube: ResMut<'w, Assets<crate::sword_scene::Material>>,
    buffers: ResMut<'w, Assets<bevy::render::storage::ShaderBuffer>>,
}
enum PartMaterial {
    Body(Handle<Material>),
    Auxiliary(Handle<AuxiliaryMaterial>),
    Rigid(rigid::Material),
}
impl Gpu<'_> {
    fn remove(&mut self, commands: &mut Commands, object: Object) {
        for part in object.parts {
            commands.entity(part.entity).try_despawn();
            self.meshes.remove(part.mesh.id());
            match part.material {
                PartMaterial::Body(handle) => {
                    self.materials.remove(handle.id());
                }
                PartMaterial::Auxiliary(handle) => {
                    self.auxiliary_materials.remove(handle.id());
                }
                PartMaterial::Rigid(material) => rigid::remove(self, material),
            }
        }
        for image in object.images {
            self.images.remove(image.id());
        }
        if let Some(buffer) = object.world {
            self.buffers.remove(buffer.id());
        }
    }
}
fn vertices(draw: &crate::mission_runtime::NpcDraw, index: usize) -> Result<Vec<[f32; 3]>, String> {
    let mesh = &draw.geometry.mesh;
    mesh.subobjects
        .get(index)
        .ok_or("Vimp draw part absent")?
        .triangles
        .iter()
        .flatten()
        .map(|&i| {
            let vertex = *mesh
                .indices
                .get(usize::from(i))
                .ok_or("Vimp draw index outside geometry")?;
            draw.positions
                .get(vertex as usize)
                .copied()
                .ok_or_else(|| "Vimp pose vertex absent".into())
        })
        .collect()
}
#[allow(clippy::too_many_arguments)] // Independent ECS scene, assets and completed pose sources.
fn update(
    mut commands: Commands,
    mut runtime: ResMut<crate::mission_runtime::Runtime>,
    scene: Option<Res<crate::world_scene::Scene>>,
    cinematic: Option<Res<crate::cinematic_camera::Cinematic>>,
    view: Res<crate::source_view::SourceView>,
    mut state: ResMut<State>,
    mut gpu: Gpu,
    mut visuals: Query<(&mut Visibility, &mut Transform)>,
    controllers: Query<&crate::mission_player::Controller>,
    cube: Option<Res<crate::reflection_cube::Build>>,
) {
    let ticket = runtime.active_ticket();
    if state.ticket != ticket {
        let released_ticket = state.ticket;
        let released = state.objects.len();
        for (_, object) in std::mem::take(&mut state.objects) {
            gpu.remove(&mut commands, object);
        }
        if let Some(old) = released_ticket {
            println!(
                "mission_npc_scene_released ticket={old} objects={released} remaining_draw_owners={} owned_gpu_removal=true",
                state.objects.len()
            );
        }
        state.ticket = ticket;
        state.failed = None;
        state.reported = 0;
    }
    let Some(ticket) = ticket else { return };
    if state.failed.is_some() {
        return;
    }
    let Some((scene, camera)) = scene.as_deref().and_then(|scene| {
        scene
            .validation_camera
            .as_ref()
            .map(|camera| (scene, camera))
    }) else {
        for object in state.objects.values() {
            for part in &object.parts {
                if let Ok((mut visibility, _)) = visuals.get_mut(part.entity) {
                    *visibility = Visibility::Hidden;
                }
            }
        }
        return;
    };
    let result = (|| -> Result<(), String> {
        let mut draws = runtime.npc_draws(ticket)?;
        draws.extend(runtime.npc_effect_draws(ticket, camera.view)?);
        let detail = cinematic
            .as_ref()
            .and_then(|c| c.camera().ok())
            .map(|c| (c.origin, c.lod_scale));
        for controller in &controllers {
            draws.extend(controller.building_effect_draws(ticket, camera.view, detail)?);
        }
        let vimp_count = draws.iter().filter(|draw| draw.class == 8).count();
        let meat_count = draws
            .iter()
            .filter(|draw| (1117..=1119).contains(&draw.class))
            .count();
        let ghost_count = draws.iter().filter(|draw| draw.class == 859).count();
        let effect_count = draws
            .iter()
            .filter(|draw| {
                matches!(
                    draw.id,
                    crate::mission_runtime::NpcDrawOwner::Effect(_)
                        | crate::mission_runtime::NpcDrawOwner::BuildingEffect(_)
                )
            })
            .count();
        let present: BTreeSet<_> = draws.iter().map(|draw| draw.id).collect();
        let retired: Vec<_> = state
            .objects
            .keys()
            .filter(|id| !present.contains(id))
            .copied()
            .collect();
        for id in retired {
            gpu.remove(&mut commands, state.objects.remove(&id).unwrap());
        }
        let lights = if draws.iter().any(|d| d.rigid.is_some()) {
            giants_game::lighting::first_world(&scene.asset.config).map_err(|e| e.to_string())?
        } else {
            Vec::new()
        };
        for draw in draws {
            if let Some(old) = state.objects.get(&draw.id) {
                if old.frame > draw.frame || old.rigid != draw.rigid.is_some() {
                    return Err("mission draw profile changed or frame regressed".into());
                }
                if !std::sync::Arc::ptr_eq(&old.geometry, &draw.geometry) && old.rigid {
                    let old = state
                        .objects
                        .remove(&draw.id)
                        .expect("observed rigid LOD owner");
                    gpu.remove(&mut commands, old);
                }
            }
            if let std::collections::btree_map::Entry::Vacant(entry) = state.objects.entry(draw.id)
            {
                let object = entry.insert(Object {
                    rigid: draw.rigid.is_some(),
                    textures: BTreeMap::new(),
                    world: (draw.class != 8 && draw.rigid.is_none()).then(|| {
                        gpu.buffers
                            .add(crate::source_view::uniform_buffer(&Mat4::IDENTITY))
                    }),
                    geometry: draw.geometry.clone(),
                    images: vec![],
                    parts: vec![],
                    frame: draw.frame,
                });
                let mut textures = BTreeMap::new();
                for source in &draw.geometry.textures {
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
                    let handle = gpu.images.add(image);
                    textures.insert(source.name.to_ascii_lowercase(), handle.clone());
                    object.images.push(handle);
                }
                object.textures = textures.clone();
                for (index, part) in draw.geometry.mesh.subobjects.iter().enumerate() {
                    // The mission binding already validates the RGB body path.
                    // Keep this consumer opaque; special/alpha passes require
                    // their own acquired submission rather than a guessed blend.
                    if draw.class == 8
                        && (part.material.flags & 1 != 0
                            || draw
                                .geometry
                                .alpha_textures
                                .contains(&part.material.texture.to_ascii_lowercase()))
                    {
                        return Err(
                            "Vimp body requires an unsupported transparent/additive material"
                                .into(),
                        );
                    }
                    let profile = if draw.class == 8 || draw.rigid.is_some() {
                        None
                    } else {
                        Some(
                            giants_game::unlit_material::prepare_with_object_transparency(
                                &part.material,
                                draw.geometry
                                    .alpha_textures
                                    .contains(&part.material.texture.to_ascii_lowercase()),
                                draw.transparency,
                            )?,
                        )
                    };
                    let ids: Vec<_> = part
                        .triangles
                        .iter()
                        .flatten()
                        .map(|&i| usize::from(i))
                        .collect();
                    let colors: Vec<_> = ids
                        .iter()
                        .map(|&i| {
                            if profile.is_some_and(|p| p.vertex_rgb)
                                || part.material.flags & 0x20 != 0
                            {
                                let c = draw.geometry.mesh.rgb[i];
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
                        .collect();
                    let mut mesh = Mesh::new(
                        bevy::render::render_resource::PrimitiveTopology::TriangleList,
                        RenderAssetUsages::default(),
                    )
                    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices(&draw, index)?)
                    .with_inserted_attribute(
                        Mesh::ATTRIBUTE_UV_0,
                        ids.iter()
                            .map(|&i| draw.geometry.mesh.uv[i])
                            .collect::<Vec<_>>(),
                    )
                    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors);
                    let fog = scene.asset.config.fog;
                    let material = if draw.rigid.is_some() {
                        PartMaterial::Rigid(rigid::create(
                            &draw,
                            index,
                            &mut mesh,
                            &textures,
                            &mut gpu,
                            scene,
                            &lights,
                            cube.as_deref(),
                        )?)
                    } else if let Some(profile) = profile {
                        PartMaterial::Auxiliary(
                            gpu.auxiliary_materials.add(
                                crate::rigid_attachment_scene::owned_material(
                                    textures
                                        .get(&part.material.texture.to_ascii_lowercase())
                                        .ok_or("auxiliary named texture absent")?
                                        .clone(),
                                    view.buffer.clone(),
                                    object.world.as_ref().unwrap().clone(),
                                    profile,
                                    false,
                                    Vec4::new(
                                        1. / camera.viewport.z as f32,
                                        -1. / camera.viewport.w as f32,
                                        fog.start,
                                        fog.end,
                                    ),
                                    Vec3::from_array(fog.color.map(|v| v as f32 / 255.)),
                                    !scene.no_fog,
                                ),
                            ),
                        )
                    } else {
                        PartMaterial::Body(
                            gpu.materials
                                .add(actor_scene::Material::from(actor_scene::Bindings {
                                    texture: textures
                                        .get(&part.material.texture.to_ascii_lowercase())
                                        .ok_or("Vimp named texture absent")?
                                        .clone(),
                                    clip_from_bevy: view.buffer.clone(),
                                    validation: Vec4::new(
                                        1.,
                                        1. / camera.viewport.z as f32,
                                        -1. / camera.viewport.w as f32,
                                        1.,
                                    ),
                                    fog: Vec4::new(
                                        fog.start,
                                        fog.end,
                                        f32::from(!scene.no_fog),
                                        0.,
                                    ),
                                    fog_color: Vec4::new(
                                        fog.color[0] as f32 / 255.,
                                        fog.color[1] as f32 / 255.,
                                        fog.color[2] as f32 / 255.,
                                        0.,
                                    ),
                                    blend: false,
                                    force_additive: false,
                                })),
                        )
                    };
                    let mesh = gpu.meshes.add(mesh);
                    let entity = commands
                        .spawn((
                            crate::mission_runtime::MissionOwned(ticket),
                            Mesh3d(mesh.clone()),
                            Transform::from_translation(Vec3::new(
                                draw.center[0],
                                draw.center[2],
                                -draw.center[1],
                            )),
                            Visibility::Visible,
                            bevy::camera::visibility::NoFrustumCulling,
                            Name::new(format!("Owned NPC visual {} part {index}", draw.class)),
                        ))
                        .id();
                    match &material {
                        PartMaterial::Body(handle) => {
                            commands
                                .entity(entity)
                                .insert(MeshMaterial3d(handle.clone()));
                        }
                        PartMaterial::Auxiliary(handle) => {
                            commands
                                .entity(entity)
                                .insert(MeshMaterial3d(handle.clone()));
                        }
                        PartMaterial::Rigid(material) => {
                            rigid::attach(&mut commands, entity, material)
                        }
                    }
                    object.parts.push(Part {
                        entity,
                        mesh,
                        material,
                        index,
                    });
                }
            }
            let object = state.objects.get_mut(&draw.id).unwrap();
            if !std::sync::Arc::ptr_eq(&object.geometry, &draw.geometry)
                || object.frame > draw.frame
            {
                return Err("Vimp draw geometry changed or frame regressed".into());
            }
            object.frame = draw.frame;
            for part in &object.parts {
                gpu.meshes
                    .get_mut(&part.mesh)
                    .ok_or("Vimp mesh removed")?
                    .insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices(&draw, part.index)?);
                let fog = scene.asset.config.fog;
                match &part.material {
                    PartMaterial::Body(handle) => {
                        let mut material = gpu
                            .materials
                            .get_mut(handle)
                            .ok_or("Vimp material removed")?;
                        material.validation.y = 1. / camera.viewport.z as f32;
                        material.validation.z = -1. / camera.viewport.w as f32;
                        material.fog = Vec4::new(fog.start, fog.end, f32::from(!scene.no_fog), 0.);
                        material.fog_color =
                            Vec3::from_array(fog.color.map(|v| v as f32 / 255.)).extend(0.);
                    }
                    PartMaterial::Auxiliary(handle) => {
                        let source = &draw.geometry.mesh.subobjects[part.index].material;
                        let profile =
                            giants_game::unlit_material::prepare_with_object_transparency(
                                source,
                                draw.geometry
                                    .alpha_textures
                                    .contains(&source.texture.to_ascii_lowercase()),
                                draw.transparency,
                            )?;
                        let mut material = gpu
                            .auxiliary_materials
                            .get_mut(handle)
                            .ok_or("auxiliary material removed")?;
                        material.set_owned_profile(profile);
                        if let Some(name) = &draw.texture {
                            let texture = object
                                .textures
                                .get(&name.to_ascii_lowercase())
                                .ok_or("building animated texture was not prepared")?
                                .clone();
                            material.set_owned_texture(texture);
                        }
                        material.set_draw_view(
                            false,
                            camera.viewport.z as f32,
                            camera.viewport.w as f32,
                        );
                        material.set_owned_fog(
                            fog.start,
                            fog.end,
                            Vec3::from_array(fog.color.map(|v| v as f32 / 255.)),
                            !scene.no_fog,
                        );
                    }
                    PartMaterial::Rigid(material) => {
                        rigid::update(&draw, part.index, material, &mut gpu, scene, &lights)?
                    }
                }
                if let Ok((mut visibility, mut transform)) = visuals.get_mut(part.entity) {
                    *visibility = Visibility::Visible;
                    transform.translation =
                        Vec3::new(draw.center[0], draw.center[2], -draw.center[1]);
                }
            }
        }
        if state.reported != state.objects.len() {
            state.reported = state.objects.len();
            println!(
                "mission_npc_draw ticket={ticket} vimps={vimp_count} visuals={} effects={effect_count} meat={meat_count} ghosts={ghost_count} canonical_npc_poses=true fx_policy=partial_source_graph model_slot=0 simulation_in_renderer=false interaction_verified=false ready=false",
                state.reported
            );
        }
        Ok(())
    })();
    if let Err(error) = result {
        for (_, object) in std::mem::take(&mut state.objects) {
            gpu.remove(&mut commands, object);
        }
        state.failed = Some(error.clone());
        error!("mission_npc_draw_refused ticket={ticket}: {error}");
    }
}

#[cfg(test)]
pub(crate) fn check_render_assets(
    runtime: crate::mission_runtime::Runtime,
    asset: std::sync::Arc<giants_game::WorldAsset>,
    advance: impl FnOnce(&mut crate::mission_runtime::Runtime),
    remove_mesh: bool,
) -> crate::mission_runtime::Runtime {
    let mut app = App::new();
    let matrix = giants_game::procedural_aim::Matrix::identity().values;
    app.insert_resource(runtime)
        .insert_resource(crate::world_scene::Scene {
            asset,
            oblique: false,
            validation_camera: Some(crate::validation_camera::ValidationCamera {
                view: matrix,
                projection: matrix,
                viewport: UVec4::new(0, 0, 640, 360),
            }),
            camera_source: "NPC CPU asset control",
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
        .init_resource::<Assets<AuxiliaryMaterial>>()
        .init_resource::<Assets<crate::scenery_scene::Material>>()
        .init_resource::<Assets<crate::sword_scene::Material>>()
        .init_resource::<Assets<bevy::render::storage::ShaderBuffer>>()
        .init_resource::<State>()
        .init_resource::<crate::source_view::SourceView>()
        .add_systems(Update, update);
    let base_buffers = app
        .world()
        .resource::<Assets<bevy::render::storage::ShaderBuffer>>()
        .len();
    let external = app
        .world_mut()
        .spawn(Name::new("unrelated draw owner"))
        .id();
    app.update();
    assert!(
        app.world().resource::<State>().failed.is_none(),
        "{:?}",
        app.world().resource::<State>().failed
    );
    let count = app.world().resource::<State>().objects.len();
    assert!(count > 1);
    let meshes = app.world().resource::<Assets<Mesh>>().len();
    let images = app.world().resource::<Assets<Image>>().len();
    let materials = app.world().resource::<Assets<Material>>().len();
    let auxiliary_materials = app.world().resource::<Assets<AuxiliaryMaterial>>().len();
    assert!(meshes >= count && images >= count && materials + auxiliary_materials == meshes);
    let entity = app
        .world()
        .resource::<State>()
        .objects
        .values()
        .next()
        .unwrap()
        .parts[0]
        .entity;
    let before: Vec<_> = app
        .world()
        .resource::<State>()
        .objects
        .values()
        .flat_map(|o| o.parts.iter())
        .map(|part| {
            (
                part.mesh.clone(),
                app.world()
                    .resource::<Assets<Mesh>>()
                    .get(&part.mesh)
                    .unwrap()
                    .attribute(Mesh::ATTRIBUTE_POSITION)
                    .unwrap()
                    .as_float3()
                    .unwrap()
                    .to_vec(),
            )
        })
        .collect();
    app.update();
    assert_eq!(app.world().resource::<Assets<Mesh>>().len(), meshes);
    assert_eq!(app.world().resource::<Assets<Image>>().len(), images);
    advance(
        &mut app
            .world_mut()
            .resource_mut::<crate::mission_runtime::Runtime>(),
    );
    app.update();
    assert!(
        app.world().resource::<State>().failed.is_none(),
        "{:?}",
        app.world().resource::<State>().failed
    );
    assert!(before.iter().any(|(handle, old)| {
        app.world()
            .resource::<Assets<Mesh>>()
            .get(handle)
            .unwrap()
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .unwrap()
            .as_float3()
            .unwrap()
            != old.as_slice()
    }));
    let camera = app
        .world_mut()
        .resource_mut::<crate::world_scene::Scene>()
        .validation_camera
        .take();
    app.update();
    assert_eq!(
        app.world().get::<Visibility>(entity),
        Some(&Visibility::Hidden)
    );
    app.world_mut()
        .resource_mut::<crate::world_scene::Scene>()
        .validation_camera = camera;
    app.update();
    assert_eq!(
        app.world().get::<Visibility>(entity),
        Some(&Visibility::Visible)
    );
    assert_eq!(app.world().resource::<Assets<Material>>().len(), materials);
    if remove_mesh {
        let handle = before[0].0.clone();
        app.world_mut()
            .resource_mut::<Assets<Mesh>>()
            .remove(handle.id());
        app.update();
        assert!(
            app.world()
                .resource::<State>()
                .failed
                .as_deref()
                .unwrap()
                .contains("mesh removed")
        );
        assert!(app.world().resource::<Assets<Mesh>>().is_empty());
        assert!(app.world().resource::<Assets<Image>>().is_empty());
        assert!(app.world().resource::<Assets<Material>>().is_empty());
        assert!(
            app.world()
                .resource::<Assets<AuxiliaryMaterial>>()
                .is_empty()
        );
        assert_eq!(
            app.world()
                .resource::<Assets<bevy::render::storage::ShaderBuffer>>()
                .len(),
            base_buffers
        );
        app.update();
        assert!(app.world().resource::<State>().objects.is_empty());
    }
    let runtime = app
        .world_mut()
        .remove_resource::<crate::mission_runtime::Runtime>()
        .unwrap();
    app.init_resource::<crate::mission_runtime::Runtime>();
    app.update();
    assert!(app.world().resource::<Assets<Mesh>>().is_empty());
    assert!(app.world().resource::<Assets<Image>>().is_empty());
    assert!(app.world().resource::<Assets<Material>>().is_empty());
    assert!(
        app.world()
            .resource::<Assets<AuxiliaryMaterial>>()
            .is_empty()
    );
    assert_eq!(
        app.world()
            .resource::<Assets<bevy::render::storage::ShaderBuffer>>()
            .len(),
        base_buffers
    );
    assert!(app.world().resource::<State>().objects.is_empty());
    assert!(app.world().get_entity(external).is_ok());
    runtime
}
