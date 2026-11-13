//! Common source rigid meshes. Current callbacks and global queue ordering are
//! still pending; unsupported capabilities remain visible in loader diagnostics.
use bevy::ecs::system::SystemParam;
use bevy::{
    asset::{AssetId, RenderAssetUsages},
    prelude::*,
    render::render_resource::PrimitiveTopology,
};
use giants_game::static_geometry::{Asset, Instance};
use std::collections::{BTreeMap, BTreeSet};

enum MaterialHandle {
    Rgb,
    Lit(Handle<crate::scenery_scene::Material>),
    Cube(Handle<crate::sword_scene::Material>),
}
#[derive(SystemParam)]
pub struct Materials<'w> {
    rgb: ResMut<'w, Assets<crate::prop_scene::Material>>,
    lit: ResMut<'w, Assets<crate::scenery_scene::Material>>,
    cube: ResMut<'w, Assets<crate::sword_scene::Material>>,
    buffers: ResMut<'w, Assets<bevy::render::storage::ShaderBuffer>>,
}
struct Rendered {
    mesh: Handle<Mesh>,
    entity: Entity,
    instance: usize,
    slot: usize,
    part: usize,
    material: MaterialHandle,
}
#[derive(Component)]
pub struct RenderedRigid;
#[derive(Component)]
pub struct SourceRigid {
    pub key: giants_game::static_geometry::SourceKey,
    pub class: i32,
}
#[derive(Default)]
pub struct OwnedAssets {
    pub meshes: Vec<AssetId<Mesh>>,
    pub images: Vec<AssetId<Image>>,
    pub rgb: Vec<AssetId<crate::prop_scene::Material>>,
    pub lit: Vec<AssetId<crate::scenery_scene::Material>>,
    pub cube: Vec<AssetId<crate::sword_scene::Material>>,
    pub buffers: Vec<AssetId<bevy::render::storage::ShaderBuffer>>,
}
pub fn needs_setup(scene: Res<Scene>, existing: Query<(), With<RenderedRigid>>) -> bool {
    !scene.source.instances.is_empty() && existing.is_empty()
}
#[derive(Resource)]
pub struct Scene {
    retired_houses: BTreeSet<giants_game::static_geometry::SourceKey>,
    source: std::sync::Arc<giants_game::static_geometry::Scene>,
    lights: Vec<giants_game::lighting::Directional>,
    fog: giants_game::lighting::Fog,
    rendered: Vec<Rendered>,
    owned: OwnedAssets,
    ordinary: Ordinary,
}
struct Ordinary {
    slots: Vec<String>,
    masks: Vec<Result<[u32; 3], &'static str>>,
}
#[derive(PartialEq, Eq)]
pub struct AdmissionSummary {
    sequence: String,
    range_bits: u32,
    accepted: usize,
    rejected: usize,
    unresolved: usize,
}
impl Ordinary {
    fn new(
        source: &giants_game::static_geometry::Scene,
        config: &giants_formats::world::WorldConfig,
    ) -> Self {
        let masks = source.instances.iter().map(|instance| {
            let result = giants_game::cinematic_ordinary::exclusion_words(&instance.cinematic_exclusions, &config.cinematics);
            if let Err(reason) = result {
                println!("cinematic_ordinary_exclusion_unresolved origin={} offset={} class={} reason={reason}", instance.key.0, instance.key.1, instance.class);
            }
            result
        }).collect();
        Self {
            slots: config
                .cinematics
                .iter()
                .map(|entry| entry.name.clone())
                .collect(),
            masks,
        }
    }
}
impl Scene {
    pub fn ordinary_view(
        &self,
        sequence: &str,
        eye: [f32; 3],
        range: f32,
        last: &mut Option<AdmissionSummary>,
    ) -> Vec<Option<bool>> {
        let slot = self
            .ordinary
            .slots
            .iter()
            .position(|name| name.eq_ignore_ascii_case(sequence))
            .map(|slot| slot as i32);
        let choices: Vec<_> = self
            .source
            .instances
            .iter()
            .zip(&self.ordinary.masks)
            .map(|(instance, masks)| {
                let masks = (*masks).ok()?;
                // A zero mask is independent of the slot. An unresolved nonzero
                // mask stays an explicit inspection limitation, never slot zero.
                if slot.is_none() && masks != [0; 3] {
                    return None;
                }
                giants_game::cinematic_ordinary::admit(
                    &giants_game::cinematic_ordinary::Input {
                        class: instance.class,
                        state: instance.initial_state,
                        flick_slot: slot.unwrap_or(-1),
                        exclusion_words: masks,
                        position: instance.position,
                        eye,
                        range,
                    },
                    giants_game::arithmetic::Precision::Double53,
                )
                .ok()
            })
            .collect();
        let accepted = choices.iter().filter(|value| **value == Some(true)).count();
        let rejected = choices
            .iter()
            .filter(|value| **value == Some(false))
            .count();
        let unresolved = choices.len() - accepted - rejected;
        let state = AdmissionSummary {
            sequence: sequence.to_owned(),
            range_bits: range.to_bits(),
            accepted,
            rejected,
            unresolved,
        };
        if last.as_ref() != Some(&state) {
            println!(
                "cinematic_ordinary_admission sequence={sequence} source_slot={slot:?} accepted={accepted} rejected={rejected} unresolved_inspection_visible={unresolved} range={range} live_callbacks_verified=false"
            );
            *last = Some(state);
        }
        choices
    }
    pub fn owned_assets(&self) -> &OwnedAssets {
        &self.owned
    }
    /// CPU clip-volume diagnostic, not a GPU visibility or occlusion result.
    /// Uses source positions before the RGB/lit backend coordinate adapter.
    pub fn clip_audit(
        &self,
        world: &crate::world_scene::Scene,
        eye: [f32; 3],
        lod_scale: f32,
    ) -> Vec<(
        giants_game::static_geometry::SourceKey,
        usize,
        usize,
        &'static str,
    )> {
        let Some(camera) = &world.validation_camera else {
            return Vec::new();
        };
        let source_to_bevy = Mat4::from_cols(Vec4::X, -Vec4::Z, Vec4::Y, Vec4::W);
        self.rendered
            .iter()
            .filter_map(|rendered| {
                let instance = &self.source.instances[rendered.instance];
                if instance.detail(eye, lod_scale) != Ok(rendered.slot) {
                    return None;
                }
                let asset = &self.source.assets[&(instance.class, rendered.slot)];
                let clip = camera.clip_from_bevy()
                    * source_to_bevy
                    * Mat4::from_cols_array(&instance.matrix).transpose();
                let points: Vec<_> = asset
                    .vertices(rendered.part)
                    .positions
                    .iter()
                    .map(|p| clip * Vec3::from_array(*p).extend(1.))
                    .collect();
                let plane = if points.iter().any(|p| !p.is_finite()) {
                    "nonfinite"
                } else {
                    [
                        ("behind", points.iter().all(|p| p.w <= 0.)),
                        ("left", points.iter().all(|p| p.x < -p.w)),
                        ("right", points.iter().all(|p| p.x > p.w)),
                        ("bottom", points.iter().all(|p| p.y < -p.w)),
                        ("top", points.iter().all(|p| p.y > p.w)),
                        ("near", points.iter().all(|p| p.z < 0.)),
                        ("far", points.iter().all(|p| p.z > p.w)),
                    ]
                    .into_iter()
                    .find(|(_, outside)| *outside)
                    .map_or("intersecting", |(name, _)| name)
                };
                Some((instance.key, rendered.slot, rendered.part, plane))
            })
            .collect()
    }
    /// Consume exactly the prepared mission geometry and selected environment.
    /// No benchmark animation warmup or first-world file selection is performed.
    pub fn from_prepared(
        source: std::sync::Arc<giants_game::static_geometry::Scene>,
        config: &giants_formats::world::WorldConfig,
    ) -> Result<Self, String> {
        if source.assets.values().any(|asset| {
            asset.initial_pose.as_ref().is_some_and(|p| {
                !matches!(
                    p.profile,
                    "debug_normal_intact"
                        | "debug_normal_declared_start"
                        | "debug_initial_idle_tree"
                        | "debug_constructed_actor_start"
                        | "source_stationary_smartie_selected_start"
                )
            })
        }) {
            return Err("mission rigid inspection rejects benchmark animation poses".into());
        }
        Ok(Self {
            retired_houses: BTreeSet::new(),
            ordinary: Ordinary::new(&source, config),
            source,
            lights: giants_game::lighting::first_world(config).map_err(|e| e.to_string())?,
            fog: config.fog,
            rendered: Vec::new(),
            owned: OwnedAssets::default(),
        })
    }
    pub fn needs_cube(&self) -> bool {
        self.source.assets.values().any(|asset| {
            (0..asset.model.mesh.subobjects.len())
                .any(|part| asset.reflection_descriptor(part).is_some())
        })
    }
    pub fn load(
        _root: &std::path::Path,
        plan: &giants_game::instances::Plan,
        covered: &BTreeSet<(bool, usize)>,
        world: &giants_game::WorldAsset,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let config = &world.config;
        let source = giants_game::static_geometry::Scene::prepare_cinematic_ground_stage(
            plan,
            covered,
            &world.terrain,
        );
        for d in &source.diagnostics {
            println!(
                "source_geometry origin={} offset={} class={} result={}",
                if d.key.0 { "world" } else { "mission" },
                d.key.1,
                d.class,
                d.result
            );
        }
        println!(
            "source_geometry_instances={} models={} callbacks=false visibility=source_projection lod_quality=2",
            source.instances.len(),
            source.assets.len()
        );
        let lights = giants_game::lighting::first_world(config)?;
        let fog = config.fog;
        Ok(Self {
            retired_houses: BTreeSet::new(),
            ordinary: Ordinary::new(&source, config),
            source: std::sync::Arc::new(source),
            lights,
            fog,
            rendered: Vec::new(),
            owned: OwnedAssets::default(),
        })
    }
}
fn pass(asset: &Asset, part: usize) -> giants_game::render_parts::Pass {
    let p = &asset.model.mesh.subobjects[part];
    giants_game::render_parts::material_pass(
        p.material.falloff,
        asset
            .model
            .alpha_textures
            .contains(&p.material.texture.to_ascii_lowercase()),
    )
}
fn rgb_material(
    asset: &Asset,
    part: usize,
    instance: &Instance,
    world: &crate::world_scene::Scene,
    texture: Option<Handle<Image>>,
    view: &crate::source_view::SourceView,
) -> crate::prop_scene::Material {
    let camera = world
        .validation_camera
        .as_ref()
        .expect("Static source camera");
    let source_to_bevy = Mat4::from_cols(Vec4::X, -Vec4::Z, Vec4::Y, Vec4::W);
    let sub = &asset.model.mesh.subobjects[part];
    let category = pass(asset, part);
    let color_profile = asset.texture_color_profile(part);
    crate::prop_scene::Material {
        instanced: false,
        instance_matrices: view.transforms.clone(),
        multiply: category == giants_game::render_parts::Pass::ObjectOrMaterial,
        alpha: category == giants_game::render_parts::Pass::TextureFormat,
        texture,
        view: view.buffer.clone(),
        world_from_bevy: source_to_bevy
            * Mat4::from_cols_array(&instance.matrix).transpose()
            * source_to_bevy.inverse(),
        pixel_offset: Vec4::new(
            1. / camera.viewport.z as f32,
            -1. / camera.viewport.w as f32,
            if category == giants_game::render_parts::Pass::TextureFormat {
                1.
            } else {
                0.
            },
            if giants_game::body_pipeline::untextured_diffuse(&sub.material) {
                -2.
            } else if color_profile.is_some_and(|p| p.add_vertex_rgb) {
                7.
            } else if color_profile.is_some_and(|p| p.vertex_rgb) || sub.material.flags & 0x20 != 0
            {
                0.
            } else {
                -1.
            },
        ),
        fog: Vec4::new(
            world.asset.config.fog.start,
            world.asset.config.fog.end,
            if world.no_fog { 0. } else { 1. },
            0.,
        ),
        fog_color: if sub.material.flags == 0x40000002 {
            Vec4::ZERO
        } else {
            Vec3::from_array(world.asset.config.fog.color.map(|v| v as f32 / 255.)).extend(0.)
        },
    }
}
fn lit_parameters(
    scene: &Scene,
    asset: &Asset,
    part: usize,
    instance: &Instance,
    world: &crate::world_scene::Scene,
) -> crate::gun_scene::Parameters {
    crate::gun_scene::source_parameters(
        &asset.model.mesh.subobjects[part].material,
        Mat4::from_cols_array(&instance.matrix).transpose(),
        world
            .validation_camera
            .as_ref()
            .expect("Static source camera"),
        &scene.lights,
        scene.fog,
        world.no_fog,
    )
    .white_texture_factor(pass(asset, part) == giants_game::render_parts::Pass::TextureFormat)
    .opaque_texture_factor_only(asset.texture_factor_only[part])
    .untextured(giants_game::body_pipeline::untextured_diffuse(
        &asset.model.mesh.subobjects[part].material,
    ))
}
fn sort_transform(asset: &Asset, part: usize, instance: &Instance) -> Transform {
    let p = giants_game::render_sort::centroid(
        &asset.model.mesh,
        part,
        giants_game::arithmetic::Precision::Double53,
    )
    .expect("Nonempty rigid part centroid");
    let point = giants_game::render_sort::world_point(
        p,
        instance.position,
        giants_game::render_sort::Transform::Cached(instance.matrix),
        giants_game::arithmetic::Precision::Double53,
    )
    .expect("Rigid sort point");
    Transform::from_translation(Vec3::new(point[0], point[2], -point[1]))
}
fn cube_parameters(
    scene: &Scene,
    asset: &Asset,
    part: usize,
    instance: &Instance,
    world: &crate::world_scene::Scene,
) -> crate::sword_scene::Parameters {
    let (factor, blend) = asset
        .reflection_descriptor(part)
        .expect("Source auxiliary descriptor")
        .cube_blend_profile()
        .expect("Measured lit cube program");
    crate::sword_scene::source_parameters(
        &asset.model.mesh.subobjects[part].material,
        Mat4::from_cols_array(&instance.matrix).transpose(),
        world.validation_camera.as_ref().expect("Source camera"),
        &scene.lights,
        scene.fog,
        world.no_fog,
    )
    .cube_factor(Some(factor))
    .blended(blend != 0)
}
#[allow(clippy::too_many_arguments)] // Independent ECS resources, including the shared source view.
pub fn setup(
    mut commands: Commands,
    view: Res<crate::source_view::SourceView>,
    mut scene: ResMut<Scene>,
    world: Res<crate::world_scene::Scene>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: Materials,
    cube: Option<Res<crate::reflection_cube::Build>>,
) {
    let mut textures = BTreeMap::new();
    let mut mesh_handles = BTreeMap::new();
    let mut owned = OwnedAssets::default();
    for (key, asset) in &scene.source.assets {
        for texture in &asset.model.textures {
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
            let handle = images.add(image);
            owned.images.push(handle.id());
            textures.insert((*key, texture.name.to_ascii_lowercase()), handle);
        }
        for (part, sub) in asset.model.mesh.subobjects.iter().enumerate() {
            if sub.triangles.is_empty() {
                continue;
            }
            let vertices = asset.vertices(part);
            let positions: Vec<_> = vertices
                .positions
                .iter()
                .map(|p| {
                    if asset.normals.is_some() {
                        *p
                    } else {
                        [p[0], p[2], -p[1]]
                    }
                })
                .collect();
            let mut mesh = Mesh::new(
                PrimitiveTopology::TriangleList,
                RenderAssetUsages::default(),
            )
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vertices.uv);
            if let Some(normals) = vertices.normals {
                mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
            } else {
                mesh.insert_attribute(
                    Mesh::ATTRIBUTE_COLOR,
                    vertices
                        .rgb
                        .iter()
                        .map(|p| {
                            [
                                p[0] as f32 / 255.,
                                p[1] as f32 / 255.,
                                p[2] as f32 / 255.,
                                1.,
                            ]
                        })
                        .collect::<Vec<_>>(),
                );
            }
            let handle = meshes.add(mesh);
            owned.meshes.push(handle.id());
            mesh_handles.insert((*key, part), handle);
        }
    }
    let mut rendered = Vec::new();
    for (index, instance) in scene.source.instances.iter().enumerate() {
        for slot in &instance.slots {
            let key = (instance.class, *slot);
            let asset = &scene.source.assets[&key];
            for (part, sub) in asset.model.mesh.subobjects.iter().enumerate() {
                let Some(mesh) = mesh_handles.get(&(key, part)) else {
                    continue;
                };
                // Health deformation belongs to an instance, including every
                // loaded LOD. Allocate before mission asset adoption so exit
                // releases these buffers through the ordinary environment owner.
                let mesh = if matches!(instance.initial_state, 61 | 101) {
                    let copy = meshes.get(mesh).expect("prepared house mesh").clone();
                    let handle = meshes.add(copy);
                    owned.meshes.push(handle.id());
                    handle
                } else {
                    mesh.clone()
                };
                let texture = textures
                    .get(&(key, sub.material.texture.to_ascii_lowercase()))
                    .cloned();
                let entity = commands
                    .spawn((
                        RenderedRigid,
                        SourceRigid {
                            key: instance.key,
                            class: instance.class,
                        },
                        Name::new(format!(
                            "Source geometry {}:{} / {} / {}",
                            instance.key.0, instance.key.1, asset.model.name, part
                        )),
                        Mesh3d(mesh.clone()),
                        sort_transform(asset, part, instance),
                        Visibility::Hidden,
                        bevy::camera::visibility::NoFrustumCulling,
                    ))
                    .id();
                let material = if asset.reflection_descriptor(part).is_some() {
                    let parameters = materials
                        .buffers
                        .add(cube_parameters(&scene, asset, part, instance, &world).buffer());
                    owned.buffers.push(parameters.id());
                    let blend = asset
                        .reflection_descriptor(part)
                        .expect("Source cube descriptor")
                        .cube_blend_profile()
                        .expect("Source cube blend")
                        .1;
                    let handle = materials.cube.add(
                        crate::sword_scene::Material::new(
                            texture.expect("Validated source cube texture"),
                            cube.as_ref()
                                .and_then(|b| b.handle())
                                .expect("Prepared source reflection cube"),
                            parameters,
                        )
                        .with_blend(blend),
                    );
                    commands
                        .entity(entity)
                        .insert(MeshMaterial3d(handle.clone()));
                    MaterialHandle::Cube(handle)
                } else if asset.normals.is_some() {
                    let parameters = materials
                        .buffers
                        .add(lit_parameters(&scene, asset, part, instance, &world).buffer());
                    owned.buffers.push(parameters.id());
                    let handle = materials.lit.add(crate::scenery_scene::Material::from(
                        crate::scenery_scene::Bindings {
                            alpha: pass(asset, part)
                                == giants_game::render_parts::Pass::TextureFormat,
                            texture,
                            parameters,
                        },
                    ));
                    commands
                        .entity(entity)
                        .insert(MeshMaterial3d(handle.clone()));
                    MaterialHandle::Lit(handle)
                } else {
                    let handle = materials
                        .rgb
                        .add(rgb_material(asset, part, instance, &world, texture, &view));
                    commands
                        .entity(entity)
                        .insert(MeshMaterial3d(handle.clone()));
                    owned.rgb.push(handle.id());
                    MaterialHandle::Rgb
                };
                match &material {
                    MaterialHandle::Rgb => {}
                    MaterialHandle::Lit(handle) => owned.lit.push(handle.id()),
                    MaterialHandle::Cube(handle) => owned.cube.push(handle.id()),
                }
                rendered.push(Rendered {
                    mesh: mesh.clone(),
                    entity,
                    instance: index,
                    slot: *slot,
                    part,
                    material,
                });
            }
        }
    }
    info!(
        "source_geometry_ready instances={} cached_meshes={} render_entries={}",
        scene.source.instances.len(),
        mesh_handles.len(),
        rendered.len()
    );
    scene.rendered = rendered;
    scene.owned = owned;
}
pub fn update(
    scene: Res<Scene>,
    cinematic: Res<crate::cinematic_camera::Cinematic>,
    world: Res<crate::world_scene::Scene>,
    mut visibility: Query<&mut Visibility>,
    mut materials: Materials,
    mut last_admission: Local<Option<AdmissionSummary>>,
) {
    let profile_started = std::time::Instant::now();
    let camera = cinematic.camera().expect("Static geometry source camera");
    let admission = scene.ordinary_view(
        cinematic.sequence(),
        camera.origin,
        cinematic.ordinary_range(),
        &mut last_admission,
    );
    apply_view(
        &scene,
        &world,
        camera.origin,
        camera.lod_scale,
        ViewAdmission {
            show: true,
            ordinary: Some(&admission),
        },
        &mut visibility,
        &mut materials,
    );
    if std::env::var_os("GIANTS_PROFILE_FRAME").is_some() {
        info!(
            "frame_cpu_system=static_scene micros={}",
            profile_started.elapsed().as_micros()
        );
    }
}
/// Shared selection/material path for a real cinematic or an explicit static
/// mission inspection camera. The caller owns the decision to expose inspection.
pub struct ViewAdmission<'a> {
    pub show: bool,
    pub ordinary: Option<&'a [Option<bool>]>,
}
pub fn apply_view(
    scene: &Scene,
    world: &crate::world_scene::Scene,
    eye: [f32; 3],
    lod_scale: f32,
    admission: ViewAdmission<'_>,
    visibility: &mut Query<&mut Visibility>,
    materials: &mut Materials,
) -> usize {
    let details: Vec<_> = scene
        .source
        .instances
        .iter()
        .map(|p| p.detail(eye, lod_scale))
        .collect();
    let mut visible = 0;
    for rendered in &scene.rendered {
        let instance = &scene.source.instances[rendered.instance];
        let asset = &scene.source.assets[&(instance.class, rendered.slot)];
        let selected = admission.show
            && !scene.retired_houses.contains(&instance.key)
            && details[rendered.instance] == Ok(rendered.slot)
            && admission
                .ordinary
                .is_none_or(|values| values[rendered.instance] != Some(false));
        if let Ok(mut value) = visibility.get_mut(rendered.entity) {
            if selected {
                visible += 1;
            }
            value.set_if_neq(if selected {
                Visibility::Visible
            } else {
                Visibility::Hidden
            });
        }
        // A hidden source LOD is still loaded, but needs no new view-dependent
        // parameters until selected. The selected slot updates before extraction.
        if !selected {
            continue;
        }
        match &rendered.material {
            MaterialHandle::Rgb => {}
            MaterialHandle::Lit(handle) => {
                if let Some(m) = materials.lit.get(handle) {
                    let parameters = m.parameters.clone();
                    if let Some(mut buffer) = materials.buffers.get_mut(&parameters) {
                        lit_parameters(scene, asset, rendered.part, instance, world)
                            .write_buffer(&mut buffer);
                    }
                }
            }
            MaterialHandle::Cube(handle) => {
                if let Some(material) = materials.cube.get(handle) {
                    let parameters = material.parameters.clone();
                    if let Some(mut buffer) = materials.buffers.get_mut(&parameters) {
                        cube_parameters(scene, asset, rendered.part, instance, world)
                            .write_buffer(&mut buffer);
                    }
                }
            }
        }
    }
    visible
}

/// Update the owned per-instance meshes, retaining material and LOD routing.
/// The same source health sample has already replaced the physical inputs.
pub(crate) fn house_updates(
    mut updates: MessageReader<crate::mission_buildings::Update>,
    runtime: Res<crate::mission_runtime::Runtime>,
    scene: Option<ResMut<Scene>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut visibility: Query<&mut Visibility>,
) {
    let Some(mut scene) = scene else {
        updates.clear();
        return;
    };
    for update in updates.read() {
        if runtime.active_ticket() != Some(update.ticket) {
            continue;
        }
        let mut count = 0;
        for rendered in &scene.rendered {
            let instance = &scene.source.instances[rendered.instance];
            if instance.key != update.key || instance.class != update.class {
                continue;
            }
            if let Some(stage) = &update.geometry {
                let Some(positions) = stage.positions.get(&rendered.slot) else {
                    error!(
                        "mission_building_draw_refused ticket={} missing_slot={}",
                        update.ticket, rendered.slot
                    );
                    continue;
                };
                let asset = &scene.source.assets[&(instance.class, rendered.slot)];
                let part = &asset.model.mesh.subobjects[rendered.part];
                let posed = part
                    .triangles
                    .iter()
                    .flatten()
                    .map(|i| {
                        let p = positions[usize::from(asset.model.mesh.indices[usize::from(*i)])];
                        if asset.normals.is_some() {
                            p
                        } else {
                            [p[0], p[2], -p[1]]
                        }
                    })
                    .collect::<Vec<_>>();
                if let Some(mut mesh) = meshes.get_mut(&rendered.mesh) {
                    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, posed);
                    count += 1;
                }
            } else if let Ok(mut visible) = visibility.get_mut(rendered.entity) {
                *visible = Visibility::Hidden;
                count += 1;
            }
        }
        if update.geometry.is_none() {
            scene.retired_houses.insert(update.key);
        }
        println!(
            "mission_building_draw ticket={} key={:?} class={} parts={count} retired={}",
            update.ticket,
            update.key,
            update.class,
            update.geometry.is_none()
        );
    }
}
