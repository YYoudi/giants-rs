//! Mission foliage with source deformation; native random phase remains unaligned.
use bevy::{asset::RenderAssetUsages, prelude::*, render::render_resource::PrimitiveTopology};
#[derive(Resource)]
pub struct Scene {
    source: giants_game::foliage::Foliage,
    rendered: Vec<Part>,
    last_tick: i32,
    last_camera: Option<([f32; 16], [f32; 16])>,
    mesh_cache: std::collections::BTreeMap<(usize, usize), Handle<Mesh>>,
    preview_seconds: f32,
}
struct Part {
    entity: Entity,
    instance: usize,
    part: usize,
}
fn local_center(model: &giants_game::foliage::Model, part: usize, positions: &[[f32; 3]]) -> Vec3 {
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for i in model.mesh.subobjects[part].triangles.iter().flatten() {
        let p = positions[usize::from(model.mesh.indices[usize::from(*i)])];
        let p = Vec3::new(p[0], p[2], -p[1]);
        min = min.min(p);
        max = max.max(p);
    }
    (min + max) * 0.5
}
fn sort_transform(
    source: &giants_game::foliage::Foliage,
    instance: &giants_game::foliage::Instance,
    part: usize,
    camera: &giants_game::animation::Camera,
    center: Vec3,
) -> Transform {
    let p = source.sort_point(instance, part, camera);
    Transform::from_translation(Vec3::new(p[0], p[2], -p[1]) - center)
}
impl Scene {
    pub(crate) fn mesh_ids(&self) -> impl Iterator<Item = bevy::asset::AssetId<Mesh>> + '_ {
        self.mesh_cache.values().map(Handle::id)
    }
    pub fn source_keys(&self) -> impl Iterator<Item = (bool, usize)> + '_ {
        self.source
            .instances
            .iter()
            .map(|p| (p.from_world, p.source_offset))
    }
    pub fn load(
        root: &std::path::Path,
        world: &giants_game::WorldAsset,
        plan: &giants_game::instances::Plan,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let source = giants_game::foliage::Foliage::load_planned(root, world, plan)?;
        println!(
            "cinematic_foliage=true palm_instances={} canopy_instances={} world_instances={} phase_policy=zero-start-unmatched",
            source.instances.iter().filter(|p| p.class == 102).count(),
            source.instances.iter().filter(|p| p.class == 1045).count(),
            source.instances.iter().filter(|p| p.from_world).count()
        );
        Ok(Self {
            source,
            rendered: Vec::new(),
            last_tick: i32::MIN,
            last_camera: None,
            mesh_cache: Default::default(),
            preview_seconds: 0.,
        })
    }
}
fn geometry(model: &giants_game::foliage::Model, part: usize, positions: &[[f32; 3]]) -> Mesh {
    let sub = &model.mesh.subobjects[part];
    let ids: Vec<_> = sub
        .triangles
        .iter()
        .flatten()
        .map(|i| usize::from(*i))
        .collect();
    let p: Vec<_> = ids
        .iter()
        .map(|i| {
            let p = positions[model.mesh.indices[*i] as usize];
            [p[0], p[2], -p[1]]
        })
        .collect();
    let uv: Vec<_> = ids.iter().map(|i| model.mesh.uv[*i]).collect();
    let colors: Vec<_> = ids
        .iter()
        .map(|i| {
            let c = model.mesh.rgb[*i];
            [
                c[0] as f32 / 255.,
                c[1] as f32 / 255.,
                c[2] as f32 / 255.,
                1.,
            ]
        })
        .collect();
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, p)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
}
#[allow(clippy::too_many_arguments)] // Independent ECS resources, including the shared source view.
pub fn setup(
    mut commands: Commands,
    view: Res<crate::source_view::SourceView>,
    mut scene: ResMut<Scene>,
    world: Res<crate::world_scene::Scene>,
    cine: Option<Res<crate::cinematic_camera::Cinematic>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<crate::prop_scene::Material>>,
    mut buffers: ResMut<Assets<bevy::render::storage::ShaderBuffer>>,
) {
    let camera = world
        .validation_camera
        .as_ref()
        .expect("Foliage source camera");
    let source_camera = source_camera(&world, cine.as_deref()).expect("Foliage sorting camera");
    let source_to_bevy = Mat4::from_cols(Vec4::X, -Vec4::Z, Vec4::Y, Vec4::W);
    let mut textures = std::collections::BTreeMap::new();
    for t in &scene.source.textures {
        let mut image = Image::new(
            bevy::render::render_resource::Extent3d {
                width: t.width,
                height: t.height,
                depth_or_array_layers: 1,
            },
            bevy::render::render_resource::TextureDimension::D2,
            t.rgba.clone(),
            bevy::render::render_resource::TextureFormat::Rgba8Unorm,
            RenderAssetUsages::default(),
        );
        image.sampler = crate::logo_material::sampler();
        textures.insert(t.name.to_ascii_lowercase(), images.add(image));
    }
    let positions = scene
        .source
        .positions(0, scene.source.animation.timeline[0])
        .expect("Source palm pose");
    let mut rendered = Vec::new();
    let mut mesh_cache = std::collections::BTreeMap::new();
    let mut material_cache = std::collections::BTreeMap::new();
    let instancing = std::env::var("GIANTS_DISABLE_FOLIAGE_INSTANCING").as_deref() != Ok("1");
    let matrices: Vec<_> = scene
        .source
        .instances
        .iter()
        .map(|instance| {
            (source_to_bevy
                * Mat4::from_cols_array(&instance.matrix).transpose()
                * source_to_bevy.inverse())
            .to_cols_array()
        })
        .collect();
    let mut transforms = buffers
        .get_mut(&view.transforms)
        .expect("Foliage instance transforms");
    if !matrices.is_empty() {
        transforms.clear();
        transforms.extend_from_slice(&matrices);
    }

    for (index, instance) in scene.source.instances.iter().enumerate() {
        let (model, positions) = if instance.class == 102 {
            (&scene.source.palms[0], positions.as_slice())
        } else {
            (
                &scene.source.canopy,
                scene.source.canopy.mesh.positions.as_slice(),
            )
        };
        let matrix = source_to_bevy
            * Mat4::from_cols_array(&instance.matrix).transpose()
            * source_to_bevy.inverse();
        for (part, sub) in model.mesh.subobjects.iter().enumerate() {
            let alpha = sub.material.flags & 0x10 == 0;
            let mode: f32 = if sub.material.flags & 0x10 != 0 {
                7.
            } else if sub.material.flags & 0x20 != 0 {
                1.
            } else {
                -1.
            };
            let material_key = (
                sub.material.texture.to_ascii_lowercase(),
                alpha,
                mode.to_bits(),
                if instancing {
                    None
                } else {
                    Some((index, part))
                },
            );
            let material = material_cache
                .entry(material_key)
                .or_insert_with(|| {
                    materials.add(crate::prop_scene::Material {
                        instanced: instancing,
                        instance_matrices: view.transforms.clone(),
                        multiply: false,
                        alpha,
                        texture: Some(textures[&sub.material.texture.to_ascii_lowercase()].clone()),
                        view: view.buffer.clone(),
                        world_from_bevy: if instancing { Mat4::IDENTITY } else { matrix },
                        pixel_offset: Vec4::new(
                            1. / camera.viewport.z as f32,
                            -1. / camera.viewport.w as f32,
                            if alpha { 1. } else { 0. },
                            mode,
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
                    })
                })
                .clone();
            let model_key = if instance.class == 102 { 0 } else { usize::MAX };
            let mesh = mesh_cache
                .entry((model_key, part))
                .or_insert_with(|| meshes.add(geometry(model, part, positions)))
                .clone();
            let transform = if alpha {
                sort_transform(
                    &scene.source,
                    instance,
                    part,
                    &source_camera,
                    local_center(model, part, positions),
                )
            } else {
                Transform::from_matrix(matrix)
            };
            let entity = commands
                .spawn((
                    Name::new(format!(
                        "Foliage{} / {}:{} / {}",
                        instance.class,
                        if instance.from_world {
                            "world"
                        } else {
                            "mission"
                        },
                        instance.source_offset,
                        sub.name
                    )),
                    bevy::mesh::MeshTag::new(index as u32),
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(material.clone()),
                    transform,
                    bevy::camera::visibility::NoFrustumCulling,
                ))
                .id();
            rendered.push(Part {
                entity,
                instance: index,
                part,
            });
        }
    }
    info!(
        "cinematic_foliage_ready=102,1045 instances={} meshes={}",
        scene.source.instances.len(),
        rendered.len()
    );
    info!("foliage_material_instancing={instancing}");
    info!(
        "foliage_shared_materials={} instance_matrices={} source_sort_transforms_unchanged=true",
        material_cache.len(),
        matrices.len()
    );
    scene.rendered = rendered;
    scene.mesh_cache = mesh_cache;
}
fn source_camera(
    world: &crate::world_scene::Scene,
    cine: Option<&crate::cinematic_camera::Cinematic>,
) -> Option<giants_game::animation::Camera> {
    if let Some(cine) = cine {
        return cine.camera().ok();
    }
    let camera = world.validation_camera.as_ref()?;
    Some(giants_game::animation::Camera {
        view: camera.view,
        projection: camera.projection,
        origin: world.camera_origin.unwrap_or_else(|| {
            Mat4::from_cols_array(&camera.view)
                .inverse()
                .w_axis
                .truncate()
                .to_array()
        }),
        viewport: camera.viewport.to_array(),
        lod_scale: 1.,
    })
}
pub fn update(
    mut scene: ResMut<Scene>,
    cine: Option<Res<crate::cinematic_camera::Cinematic>>,
    world: Res<crate::world_scene::Scene>,
    time: Res<Time>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut transforms: Query<&mut Transform>,
    mut mesh_components: Query<&mut Mesh3d>,
) {
    let profile_started = std::time::Instant::now();
    if scene.rendered.is_empty() {
        return;
    }
    let Some(source_camera) = source_camera(&world, cine.as_deref()) else {
        return;
    };
    scene.preview_seconds = cine.as_ref().map_or_else(
        || scene.preview_seconds + time.delta_secs().min(0.1),
        |c| c.world_seconds(),
    );
    let [start, end, tpf, fps] = scene.source.animation.timeline;
    let tick =
        start + ((scene.preview_seconds * tpf as f32 * fps as f32) as i32).rem_euclid(end - start);
    let camera_key = (source_camera.view, source_camera.projection);
    if scene.last_tick == tick && scene.last_camera == Some(camera_key) {
        return;
    }
    let positions = scene
        .source
        .all_positions(tick)
        .expect("Prepared foliage poses");
    let mut changed = 0;
    let mut updated = std::collections::BTreeSet::new();
    let mut centers = std::collections::BTreeMap::new();
    let scene = &mut *scene;
    for rendered in &scene.rendered {
        let instance = &scene.source.instances[rendered.instance];
        if instance.class != 102 {
            if let Ok(mut transform) = transforms.get_mut(rendered.entity) {
                *transform = sort_transform(
                    &scene.source,
                    instance,
                    rendered.part,
                    &source_camera,
                    local_center(
                        &scene.source.canopy,
                        rendered.part,
                        &scene.source.canopy.mesh.positions,
                    ),
                );
            }
            continue;
        }
        let detail = if rendered.part >= 10 {
            scene
                .source
                .detail(instance, source_camera.origin, source_camera.lod_scale)
        } else {
            0
        };
        let model = &scene.source.palms[detail];
        let part = if rendered.part >= 10 {
            model.mesh.subobjects.len() - 2 + rendered.part - 10
        } else {
            rendered.part
        };
        let p = &positions[detail];
        if rendered.part < 10
            && let Ok(mut transform) = transforms.get_mut(rendered.entity)
        {
            *transform = sort_transform(
                &scene.source,
                instance,
                rendered.part,
                &source_camera,
                *centers
                    .entry((detail, part))
                    .or_insert_with(|| local_center(model, part, p)),
            );
        }
        let key = (detail, part);
        let handle = scene
            .mesh_cache
            .entry(key)
            .or_insert_with(|| meshes.add(geometry(model, part, p)));
        if updated.insert(key)
            && let Some(mut mesh) = meshes.get_mut(&*handle)
        {
            // UV and vertex colors are immutable; deform each shared part once.
            mesh.insert_attribute(
                Mesh::ATTRIBUTE_POSITION,
                model.mesh.subobjects[part]
                    .triangles
                    .iter()
                    .flatten()
                    .map(|i| {
                        let p = p[model.mesh.indices[usize::from(*i)] as usize];
                        [p[0], p[2], -p[1]]
                    })
                    .collect::<Vec<_>>(),
            );
            changed += 1;
        }
        if let Ok(mut component) = mesh_components.get_mut(rendered.entity)
            && component.0 != *handle
        {
            component.0 = handle.clone();
        }
    }
    if scene.last_tick != tick {
        info!(
            "cinematic_foliage_tick={} updated_meshes={} shared_geometry=true native_phase_aligned=false",
            tick, changed
        );
        scene.last_tick = tick;
    }
    scene.last_camera = Some(camera_key);
    if std::env::var_os("GIANTS_PROFILE_FRAME").is_some() {
        info!(
            "frame_cpu_system=foliage_scene micros={}",
            profile_started.elapsed().as_micros()
        );
    }
}
