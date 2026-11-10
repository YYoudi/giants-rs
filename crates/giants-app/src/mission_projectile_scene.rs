//! Source class38 geometry for the ordinary partial pistol controller. This
//! consumes actual live shots; it does not construct a native state77 actor/FX.
use crate::rigid_attachment_scene::{self, Material};
use bevy::{asset::RenderAssetUsages, prelude::*, render::storage::ShaderBuffer};
use std::collections::BTreeMap;

pub(crate) struct Visual {
    model: giants_game::scene_resources::ModelAsset,
    personality: u32,
    scale: f32,
    angle: f32,
    profiles: Vec<giants_game::unlit_material::Profile>,
}
impl Visual {
    pub(crate) fn load(
        source: &giants_game::scene_resources::SceneResources,
    ) -> Result<Self, String> {
        let spec = giants_game::specification::Prepared::load_registered(source, 38)?;
        let fields = giants_game::actor_fields::Fields::load(source, 38, &spec)?;
        let cpu = spec
            .models
            .first()
            .and_then(Option::as_ref)
            .ok_or("pistol projectile model absent")?;
        if !spec.needs.is_empty()
            || cpu.prepared.skin.is_some()
            || fields.channels[0][0] != 1
            || spec
                .animations
                .get(&1)
                .is_none_or(|a| a.template.flags != 1 || !a.template.needs.is_empty())
        {
            return Err("pistol projectile needs an unacquired animated geometry producer".into());
        }
        let model = source
            .model_slot_for_body(
                38,
                0,
                giants_game::scene_resources::TextureEncoding::Rgb565OrArgb4444,
            )
            .map_err(|e| e.to_string())?
            .ok_or("pistol projectile model missing")?;
        if model.mesh.normal_count != 0 {
            return Err("pistol projectile lit material required".into());
        }
        let profiles = model
            .mesh
            .subobjects
            .iter()
            .map(|part| {
                giants_game::unlit_material::prepare_with_texture_alpha(
                    &part.material,
                    model
                        .alpha_textures
                        .contains(&part.material.texture.to_ascii_lowercase()),
                )
                .map_err(str::to_owned)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            model,
            personality: fields.configuration[14],
            scale: fields.scale,
            angle: fields.angles[0],
            profiles,
        })
    }
}
struct Part {
    entity: Entity,
    mesh: Handle<Mesh>,
    material: Handle<Material>,
    index: usize,
}
struct Mounted {
    ticket: u64,
    visual: Visual,
    parts: Vec<Part>,
    images: Vec<Handle<Image>>,
    world: Handle<ShaderBuffer>,
    active: usize,
}
#[derive(Resource, Default)]
struct State {
    mounted: Option<Mounted>,
    failed: Option<u64>,
}
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
#[allow(clippy::too_many_arguments)]
fn update(
    mut commands: Commands,
    runtime: Res<crate::mission_runtime::Runtime>,
    frontend: Res<crate::frontend_scene::Scene>,
    showcase: Res<crate::mission_showcase::State>,
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
    let ticket = runtime
        .active_ticket()
        .filter(|t| *t == frontend.flow.ticket && showcase.controls(frontend.flow.phase));
    if state
        .mounted
        .as_ref()
        .is_some_and(|m| Some(m.ticket) != ticket)
    {
        let old = state.mounted.take().unwrap();
        for part in old.parts {
            commands.entity(part.entity).try_despawn();
            meshes.remove(part.mesh.id());
            materials.remove(part.material.id());
        }
        for image in old.images {
            images.remove(image.id());
        }
        buffers.remove(old.world.id());
        println!(
            "mission_projectile_draw_released ticket={} assets_removed=true",
            old.ticket
        );
    }
    let (Some(ticket), Some(scene)) = (ticket, scene) else {
        return;
    };
    if state.failed == Some(ticket) {
        return;
    }
    let Some(pistol) = controllers
        .iter()
        .find(|c| c.prepared(ticket))
        .and_then(|c| c.pistol())
    else {
        return;
    };
    let Some(camera) = &scene.validation_camera else {
        return;
    };
    let result = (|| -> Result<(), String> {
        if !matches!(pistol.definition.id, 17 | 42) || pistol.definition.class != 38 {
            return Err("projectile renderer outside ordinary base pistol profile".into());
        }
        if state.mounted.is_none() {
            if pistol.shots.is_empty() {
                return Ok(());
            }
            let visual = runtime.pistol_projectile_visual(ticket)?;
            let world = buffers.add(crate::source_view::uniform_buffer(&Mat4::IDENTITY));
            state.mounted = Some(Mounted {
                ticket,
                visual,
                parts: Vec::new(),
                images: Vec::new(),
                world: world.clone(),
                active: 0,
            });
            let mounted = state.mounted.as_mut().unwrap();
            let mut textures = BTreeMap::new();
            for source in &mounted.visual.model.textures {
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
            for (index, part) in mounted.visual.model.mesh.subobjects.iter().enumerate() {
                let profile = mounted.visual.profiles[index];
                let mesh = meshes.add(
                    Mesh::new(
                        bevy::render::render_resource::PrimitiveTopology::TriangleList,
                        RenderAssetUsages::default(),
                    )
                    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0.; 3]; 3])
                    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.; 2]; 3])
                    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, vec![[1.; 4]; 3]),
                );
                let material = materials.add(rigid_attachment_scene::owned_material(
                    textures
                        .get(&part.material.texture.to_ascii_lowercase())
                        .ok_or("projectile texture absent")?
                        .clone(),
                    view.buffer.clone(),
                    world.clone(),
                    profile,
                    false,
                    Vec4::new(
                        1. / camera.viewport.z as f32,
                        -1. / camera.viewport.w as f32,
                        scene.asset.config.fog.start,
                        scene.asset.config.fog.end,
                    ),
                    Vec3::from_array(scene.asset.config.fog.color.map(|c| c as f32 / 255.)),
                    !scene.no_fog,
                ));
                let entity = commands
                    .spawn((
                        crate::mission_runtime::MissionOwned(ticket),
                        Mesh3d(mesh.clone()),
                        MeshMaterial3d(material.clone()),
                        Transform::default(),
                        Visibility::Hidden,
                        bevy::camera::visibility::NoFrustumCulling,
                        Name::new("Mecc source projectile"),
                    ))
                    .id();
                mounted.parts.push(Part {
                    entity,
                    mesh,
                    material,
                    index,
                });
            }
            println!(
                "mission_projectile_draw_bound ticket={ticket} class=38 source_geometry=true source_material=true native_actor=false constructor_fx=false"
            );
        }
        let mounted = state.mounted.as_mut().unwrap();
        let visual = &mounted.visual;
        let matrices = pistol
            .shots
            .iter()
            .map(|shot| {
                giants_game::camera_facing::draw_matrix(
                    shot.world_matrix(),
                    visual.personality,
                    giants_game::camera_facing::Input {
                        view: camera.view,
                        position: shot.motion.position,
                        scale: visual.scale,
                        angle: visual.angle,
                    },
                    giants_game::arithmetic::Precision::Double53,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        for part in &mounted.parts {
            materials
                .get_mut(&part.material)
                .ok_or("projectile material removed")?
                .set_draw_view(false, camera.viewport.z as f32, camera.viewport.w as f32);
            if let Ok(mut visible) = visibility.get_mut(part.entity) {
                *visible = if matrices.is_empty() {
                    Visibility::Hidden
                } else {
                    Visibility::Visible
                };
            }
            if matrices.is_empty() {
                continue;
            }
            let indices = visual.model.mesh.subobjects[part.index]
                .triangles
                .iter()
                .flatten()
                .map(|i| usize::from(*i))
                .collect::<Vec<_>>();
            let mut positions = Vec::new();
            let mut uv = Vec::new();
            let mut colors = Vec::new();
            for matrix in &matrices {
                for &index in &indices {
                    let p = visual.model.mesh.positions[visual.model.mesh.indices[index] as usize];
                    let world: [f32; 3] = std::array::from_fn(|axis| {
                        (((f64::from(matrix[axis * 4]) * f64::from(p[0])
                            + f64::from(matrix[axis * 4 + 1]) * f64::from(p[1]))
                            + f64::from(matrix[axis * 4 + 2]) * f64::from(p[2]))
                            + f64::from(matrix[axis * 4 + 3])) as f32
                    });
                    positions.push([world[0], world[2], -world[1]]);
                    uv.push(visual.model.mesh.uv[index]);
                    let rgb = if visual.profiles[part.index].vertex_rgb {
                        visual.model.mesh.rgb[index].map(|c| c as f32 / 255.)
                    } else {
                        [1.; 3]
                    };
                    colors.push([rgb[0], rgb[1], rgb[2], 1.]);
                }
            }
            let mut mesh = meshes
                .get_mut(&part.mesh)
                .ok_or("projectile draw mesh removed")?;
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
            mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
            mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
        }
        if mounted.active != matrices.len() {
            println!(
                "mission_projectile_draw_active ticket={ticket} shots={} source_geometry=true",
                matrices.len()
            );
            mounted.active = matrices.len();
        }
        Ok(())
    })();
    if let Err(error) = result {
        state.failed = Some(ticket);
        error!("mission_projectile_draw_failed ticket={ticket} error={error}");
        exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
    }
}
