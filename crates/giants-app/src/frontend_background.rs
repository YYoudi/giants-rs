//! Owned intro environment for frontend pages. No mission acknowledgements.
use bevy::{asset::AssetId, ecs::system::RunSystemOnce, prelude::*};
use giants_game::{frontend_background::Background, mission_loading::Phase};
use std::collections::HashSet;

pub struct Plugin;
impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        crate::reflection_cube::install_worker(app);
        app.add_plugins(bevy::pbr::MaterialPlugin::<crate::frontend_lens::Material>::default());
        app.add_plugins(UiMaterialPlugin::<crate::frontend_hover::Material>::default());
        app.add_plugins(UiMaterialPlugin::<crate::frontend_hover::Composite>::default());
        app.init_resource::<State>()
            .add_systems(
                Update,
                release
                    .before(crate::mission_runtime::audit)
                    .after(crate::frontend_scene::runtime_events),
            )
            .add_systems(
                Update,
                (
                    finish_cube,
                    mount,
                    animate,
                    lenses,
                    hover,
                    veil,
                    water_geometry,
                    water_animation,
                    present,
                )
                    .chain()
                    .after(crate::reflection_cube::CubeUpdate)
                    .after(crate::mission_environment::EnvironmentUpdate)
                    .before(crate::source_view::ViewUpdate),
            );
    }
}
#[derive(Component)]
pub(crate) struct Owned;
#[derive(Resource, Default)]
struct State {
    source: Option<Background>,
    mounted: Option<AssetsOwned>,
    failed: bool,
    seconds: f64,
    segment: usize,
    buffers: Vec<Handle<bevy::render::storage::ShaderBuffer>>,
    fixed_tick: Option<i32>,
    sun_matrices: Option<Handle<bevy::render::storage::ShaderBuffer>>,
    glare_intensity: f32,
    building_cube: bool,
    cube: Option<Handle<Image>>,
}
macro_rules! owned_assets {
    ($($field:ident : $ty:ty),* $(,)?) => {
        #[derive(Default)]
        struct AssetsOwned { $( $field: HashSet<AssetId<$ty>>, )* }
        impl AssetsOwned {
            fn snapshot(world: &World) -> Self {
                Self { $( $field: world.resource::<Assets<$ty>>().ids().collect(), )* }
            }
            fn added_since(self, before: &Self) -> Self {
                Self { $( $field: self.$field.difference(&before.$field).copied().collect(), )* }
            }
            fn release(self, world: &mut World) -> usize {
                let mut count = 0;
                $( for id in self.$field { count += usize::from(world.resource_mut::<Assets<$ty>>().remove(id).is_some()); } )*
                count
            }
        }
    }
}
owned_assets! {
    meshes: Mesh, images: Image,
    terrain: crate::world_scene::WorldColorMaterial,
    sky: crate::sky_scene::SkyMaterial,
    underside: crate::underside_scene::UndersideMaterial,
    presentation: crate::world_composition::Presentation,
    lit: crate::scenery_scene::Material,
    reflective: crate::sword_scene::Material,
    lenses: crate::frontend_lens::Material,
    hover: crate::frontend_hover::Material,
    hover_composite: crate::frontend_hover::Composite,
    water: crate::water_scene::Material,
    patterns: crate::water_texture::Material,
    prelit: crate::prop_scene::Material,
    buffers: bevy::render::storage::ShaderBuffer,
}
fn menu(world: &World) -> bool {
    crate::mecc_session::frontend_available(world)
        && world.resource::<crate::frontend_scene::Scene>().flow.phase == Phase::Menu
}
fn release(world: &mut World) {
    if menu(world) {
        return;
    }
    release_for_development(world);
}
pub(crate) fn release_for_development(world: &mut World) {
    let Some(assets) = world.resource_mut::<State>().mounted.take() else {
        return;
    };
    let entities: Vec<_> = world
        .query_filtered::<Entity, With<Owned>>()
        .iter(world)
        .collect();
    for entity in &entities {
        world.despawn(*entity);
    }
    let count = assets.release(world);
    world.remove_resource::<crate::water_scene::RuntimeWater>();
    world.remove_resource::<crate::world_scene::Scene>();
    world.remove_resource::<crate::reflection_cube::Build>();
    world.remove_resource::<crate::frontend_lens::Runtime>();
    world.remove_resource::<crate::frontend_hover::Runtime>();
    let mut state = world.resource_mut::<State>();
    state.buffers.clear();
    state.cube = None;
    state.building_cube = false;
    println!(
        "frontend_background_released entities={} assets={} source_view=available_for_mission",
        entities.len(),
        count
    );
}
fn finish_cube(world: &mut World) {
    if !menu(world) || !world.resource::<State>().building_cube {
        return;
    }
    let Some(cube) = world
        .get_resource::<crate::reflection_cube::Build>()
        .filter(|build| build.complete)
        .and_then(|build| build.handle())
    else {
        return;
    };
    let mut assets = world
        .resource_mut::<State>()
        .mounted
        .take()
        .expect("Cube phase ownership");
    // Transfer only the generated cube; temporary views and producer assets end here.
    assert!(assets.images.remove(&cube.id()));
    let entities: Vec<_> = world
        .query_filtered::<Entity, With<Owned>>()
        .iter(world)
        .collect();
    for entity in entities {
        world.despawn(entity);
    }
    let released = assets.release(world);
    world.remove_resource::<crate::reflection_cube::Build>();
    world.remove_resource::<crate::water_scene::RuntimeWater>();
    world.remove_resource::<crate::world_scene::Scene>();
    let mut state = world.resource_mut::<State>();
    state.cube = Some(cube);
    state.building_cube = false;
    println!(
        "frontend_cube_phase_released assets={released} transfer_images=1 source_view=sequential"
    );
}
fn camera(
    source: &Background,
    index: usize,
    tick: i32,
) -> Result<crate::validation_camera::ValidationCamera, Box<dyn std::error::Error>> {
    let camera = source.segments[index].camera(tick)?;
    Ok(crate::validation_camera::ValidationCamera {
        view: camera.view,
        projection: camera.projection,
        viewport: UVec4::from_array(camera.viewport),
    })
}
fn logo_matrix(
    source: &Background,
    index: usize,
    tick: i32,
) -> Result<Mat4, Box<dyn std::error::Error>> {
    let segment = &source.segments[index];
    let anchor = Mat4::from_cols_array(&segment.anchor(tick)?).transpose();
    let socket = source.logo_animation.default_node(&segment.socket)?;
    let socket = giants_game::animation::Evaluator {
        animation: &source.logo_animation,
    }
    .matrix(socket.offset, source.logo_animation.timeline[0])?;
    Ok(anchor
        * Mat4::from_scale(Vec3::splat(source.scale))
        * Mat4::from_cols_array(&socket).transpose().inverse())
}
fn intro_lights(
    config: &giants_formats::world::WorldConfig,
) -> Result<Vec<giants_game::lighting::Directional>, Box<dyn std::error::Error>> {
    let mut result = Vec::new();
    let mut variants = HashSet::new();
    for light in &config.lights {
        let variant = light
            .variant
            .filter(|v| (1..=3).contains(v))
            .ok_or("intro directional variant")?;
        if !variants.insert(variant) {
            return Err("duplicate intro directional variant".into());
        }
        let color = light.color.ok_or("intro directional color")?;
        let matrix =
            giants_game::object_transform::euler(light.position, light.angles, light.scale)?;
        result.push(giants_game::lighting::Directional {
            variant,
            position: light.position,
            direction: [-matrix[1], -matrix[5], -matrix[9]],
            color,
        });
    }
    result.sort_by_key(|l| l.variant);
    while result.len() < 3 {
        result.push(giants_game::lighting::Directional {
            variant: 0,
            position: [0.; 3],
            direction: [0.; 3],
            color: [0.; 3],
        });
    }
    Ok(result)
}
fn mount(world: &mut World) {
    if !menu(world)
        || world.resource::<State>().mounted.is_some()
        || world.resource::<State>().failed
    {
        return;
    }
    // Mission teardown precedes this system. Never overwrite another active view.
    if world.contains_resource::<crate::world_scene::Scene>() {
        return;
    }
    let before = AssetsOwned::snapshot(world);
    let entities_before: HashSet<_> = world
        .query_filtered::<Entity, With<crate::world_scene::Rendered>>()
        .iter(world)
        .collect();
    let result = world.resource_scope(|world, mut state: Mut<State>| -> Result<(), Box<dyn std::error::Error>> {
        if state.source.is_none() {
            let root = world.resource::<crate::frontend_scene::Scene>().data_root();
            state.source = Some(Background::load(root)?);
        }
        let source = state.source.as_ref().unwrap();
        let tick = source.segments[0].animation.timeline[0];
        let building_cube = state.cube.is_none();
        let (camera, origin) = if building_cube {
            let view = giants_game::reflection::story4_camera(0)?;
            world.insert_resource(crate::reflection_cube::Build::frontend());
            (crate::cinematic_camera::convert(&view), view.origin)
        } else {
            (camera(source, 0, tick)?, source.segments[0].camera(tick)?.origin)
        };
        let water_asset = giants_game::water::load_texture(world.resource::<crate::frontend_scene::Scene>().data_root(), "intro")?;
        let band = giants_game::water::band([origin[0], origin[1]], [source.world.sun[0], source.world.sun[1]], source.world.config.sea_values[0], water_asset.water)?;
        let water = crate::water_scene::Scene {
            asset: water_asset, band, autonomous: !building_cube,
            state: crate::water_scene::ValidationState {
                eye: origin, sun: source.world.sun, phases: [[0.; 2]; 5], background: [0; 3], sun_from_world: true,
            },
        };
        world.insert_resource(crate::world_scene::Scene {
            asset: source.world.clone(), oblique: false, validation_camera: Some(camera),
            camera_source: "frontend_intro_source_preview", camera_origin: Some(origin),
            cinematic_capture_ready: !building_cube, no_fog: false, render_terrain: true,
            render_sky: true, render_underside: true, reflection_profile: building_cube,
            water: Some(water), render_water: true,
        });
        world.run_system_once(crate::world_scene::setup).map_err(|e| format!("intro world setup: {e}"))?;
        if building_cube {
            let nodes: Vec<_> = world.query_filtered::<Entity, With<MaterialNode<crate::world_composition::Presentation>>>().iter(world).collect();
            for node in nodes { world.entity_mut(node).insert(Visibility::Hidden); }
            state.building_cube = true;
            println!("frontend_cube_phase_started profile=intro_initial phases=zero captured_cube_inputs=none pixel_fidelity=unverified");
            return Ok(());
        }
        let buffers = spawn_logo(world, source, tick, state.cube.as_ref().unwrap().clone())?;
        let disk_camera = world.resource::<crate::world_scene::Scene>().validation_camera.clone().ok_or("intro disk camera")?;
        let solar_view=crate::frontend_lens::spawn(world, source);
        let sun_matrices = crate::frontend_sun::spawn(world, source, &disk_camera, solar_view);
        let root=world.resource::<crate::frontend_scene::Scene>().data_root().to_path_buf();
        crate::frontend_hover::spawn(world, &root)?;
        println!("frontend_background_mounted world=intro logo_class={} segments={} logo_scale={} camera=source full_viewport=true clock=serialized_preview native_timing=unverified logo_reflection=generated_initial_cube_unverified water=sun_band_source", source.segments[0].class, source.segments.len(), source.scale);
        let fixed_tick = std::env::var("GIANTS_FRONTEND_PREVIEW_TICK").ok().map(|value| value.parse::<i32>()).transpose()?;
        let segment = std::env::var("GIANTS_FRONTEND_PREVIEW_SEGMENT").ok().map(|value| value.parse::<usize>()).transpose()?.unwrap_or(0);
        let selected = source.segments.get(segment).ok_or("frontend preview segment outside source chain")?;
        if fixed_tick.is_some_and(|tick| !(selected.animation.timeline[0]..=selected.animation.timeline[1]).contains(&tick)) {
            return Err("frontend preview tick outside selected source timeline".into());
        }
        state.buffers = buffers;
        state.sun_matrices = Some(sun_matrices);
        state.fixed_tick = fixed_tick;
        state.seconds = 0.; state.segment = segment;
        state.glare_intensity = 0.;
        Ok(())
    });
    let entities: Vec<_> = world
        .query_filtered::<Entity, With<crate::world_scene::Rendered>>()
        .iter(world)
        .filter(|e| !entities_before.contains(e))
        .collect();
    for entity in entities {
        world.entity_mut(entity).insert(Owned);
    }
    let mut assets = AssetsOwned::snapshot(world).added_since(&before);
    if let Some(cube) = &world.resource::<State>().cube {
        assets.images.insert(cube.id());
    }
    world.resource_mut::<State>().mounted = Some(assets);
    if let Err(error) = result {
        eprintln!("frontend_background_blocked reason={error}");
        world.resource_mut::<State>().failed = true;
        // Roll back a partial setup as well as fully mounted environments.
        let assets = world.resource_mut::<State>().mounted.take().unwrap();
        let entities: Vec<_> = world
            .query_filtered::<Entity, With<Owned>>()
            .iter(world)
            .collect();
        for entity in entities {
            world.despawn(entity);
        }
        assets.release(world);
        world.remove_resource::<crate::water_scene::RuntimeWater>();
        world.remove_resource::<crate::world_scene::Scene>();
        world.remove_resource::<crate::reflection_cube::Build>();
        world.remove_resource::<crate::frontend_lens::Runtime>();
        world.remove_resource::<crate::frontend_hover::Runtime>();
        world.resource_mut::<State>().cube = None;
        world.resource_mut::<State>().building_cube = false;
    }
}
fn spawn_logo(
    world: &mut World,
    source: &Background,
    tick: i32,
    cube: Handle<Image>,
) -> Result<Vec<Handle<bevy::render::storage::ShaderBuffer>>, Box<dyn std::error::Error>> {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{
        Extent3d, PrimitiveTopology, TextureDimension, TextureFormat,
    };
    let mut textures = std::collections::BTreeMap::new();
    for texture in &source.textures {
        let mut image = Image::new(
            Extent3d {
                width: texture.width,
                height: texture.height,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            texture.rgba.clone(),
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::default(),
        );
        image.sampler = crate::logo_material::sampler();
        textures.insert(
            texture.name.to_ascii_lowercase(),
            world.resource_mut::<Assets<Image>>().add(image),
        );
    }
    let camera = camera(source, 0, tick)?;
    let matrix = logo_matrix(source, 0, tick)?;
    let lights = intro_lights(&source.world.config)?;
    let mesh = &source.logo.mesh;
    let positions: Vec<_> = mesh
        .indices
        .iter()
        .map(|i| mesh.positions[*i as usize])
        .collect();
    let mut buffers = Vec::new();
    for sub in &mesh.subobjects {
        let geometry = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions.clone())
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, source.logo.normals.clone())
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, mesh.uv.clone())
        .with_inserted_indices(bevy::mesh::Indices::U16(
            sub.triangles.iter().flatten().copied().collect(),
        ));
        let parameters = crate::sword_scene::source_parameters(
            &sub.material,
            matrix,
            &camera,
            &lights,
            source.world.config.fog,
            true,
        )
        .cube_factor(Some(76. / 255.));
        let buffer = world
            .resource_mut::<Assets<bevy::render::storage::ShaderBuffer>>()
            .add(parameters.buffer());
        let texture = textures
            .get(&sub.material.texture.to_ascii_lowercase())
            .ok_or("intro logo texture")?
            .clone();
        let material = world
            .resource_mut::<Assets<crate::sword_scene::Material>>()
            .add(crate::sword_scene::Material::new(
                texture,
                cube.clone(),
                buffer.clone(),
            ));
        let geometry = world.resource_mut::<Assets<Mesh>>().add(geometry);
        world.spawn((
            Owned,
            Name::new(format!("Frontend logo / {}", sub.name)),
            Mesh3d(geometry),
            MeshMaterial3d(material),
            bevy::camera::visibility::NoFrustumCulling,
        ));
        buffers.push(buffer);
    }
    Ok(buffers)
}
fn animate(
    time: Res<Time>,
    mut state: ResMut<State>,
    mut scene: Option<ResMut<crate::world_scene::Scene>>,
    mut materials: crate::world_scene::SceneMaterials,
) {
    if state.mounted.is_none() || state.building_cube {
        return;
    }
    if state.fixed_tick.is_none() {
        state.seconds += time.delta_secs_f64();
    }
    let source = state.source.as_ref().unwrap();
    let mut segment = state.segment;
    let mut seconds = state.seconds;
    loop {
        let [start, end, ticks, fps] = source.segments[segment].animation.timeline;
        let duration = f64::from(end - start) / (f64::from(ticks) * f64::from(fps));
        if seconds < duration {
            break;
        }
        if segment + 1 == source.segments.len() && source.segments[segment].next.is_none() {
            seconds = duration;
            break;
        }
        seconds -= duration;
        segment = (segment + 1).min(source.segments.len() - 1);
    }
    let [start, end, ticks, fps] = source.segments[segment].animation.timeline;
    let tick = state
        .fixed_tick
        .unwrap_or((start + (seconds * f64::from(ticks) * f64::from(fps)) as i32).min(end));
    if let (Some(scene), Ok(camera)) = (scene.as_mut(), camera(source, segment, tick)) {
        scene.camera_origin = source.segments[segment].camera(tick).ok().map(|c| c.origin);
        scene.validation_camera = Some(camera);
        if let Some(handle) = &state.sun_matrices
            && let Some(mut buffer) = materials.buffers.get_mut(handle)
        {
            buffer.clear();
            buffer.extend_from_slice(&[crate::frontend_sun::matrix(
                source,
                scene.validation_camera.as_ref().unwrap().view,
            )
            .to_cols_array()]);
        }
        crate::world_scene::apply_camera(scene, &mut materials);
        if let (Ok(matrix), Ok(lights)) = (
            logo_matrix(source, segment, tick),
            intro_lights(&source.world.config),
        ) {
            for (sub, handle) in source.logo.mesh.subobjects.iter().zip(&state.buffers) {
                if let Some(mut buffer) = materials.buffers.get_mut(handle) {
                    crate::sword_scene::source_parameters(
                        &sub.material,
                        matrix,
                        scene.validation_camera.as_ref().unwrap(),
                        &lights,
                        source.world.config.fog,
                        true,
                    )
                    .cube_factor(Some(76. / 255.))
                    .write_buffer(&mut buffer);
                }
            }
        }
    }
    state.segment = segment;
    state.seconds = seconds;
}
/// The measured additive screen pass. The two textured lens objects are separate.
/// Accumulate actual preview deltas; a frozen pose does not invent native history.
fn veil(
    time: Res<Time>,
    mut state: ResMut<State>,
    scene: Option<Res<crate::world_scene::Scene>>,
    mut presentations: ResMut<Assets<crate::world_composition::Presentation>>,
) {
    if state.mounted.is_none() || state.building_cube {
        return;
    }
    let Some(scene) = scene else { return };
    let (Some(camera), Some(eye), Some(parameters)) = (
        scene.validation_camera.as_ref(),
        scene.camera_origin,
        scene.asset.config.sun_glare.as_ref(),
    ) else {
        return;
    };
    let result = (|| -> Result<_, giants_formats::Error> {
        let ray = giants_game::sun_glare::ray(
            scene.asset.sun,
            eye,
            [camera.view[2], camera.view[6], camera.view[10]],
        )?;
        let clear = giants_game::terrain_visibility::clear(&scene.asset.terrain, &ray)?;
        let (intensity, _) = giants_game::sun_glare::advance(
            state.glare_intensity,
            time.delta_secs(),
            ray.dot,
            clear,
            parameters.exponent,
            2.,
        )?;
        Ok((
            intensity,
            giants_game::sun_glare::screen_rgb(parameters, intensity, 0.)?.unwrap_or([0; 3]),
        ))
    })();
    match result {
        Ok((intensity, rgb)) => {
            state.glare_intensity = intensity;
            let glare = Vec4::new(
                rgb[0] as f32 / 255.,
                rgb[1] as f32 / 255.,
                rgb[2] as f32 / 255.,
                0.,
            );
            for id in &state.mounted.as_ref().unwrap().presentation {
                if let Some(mut material) = presentations.get_mut(*id)
                    && (material.glare != glare
                        || material.glare_viewport != camera.viewport.as_vec4())
                {
                    material.glare = glare;
                    material.glare_viewport = camera.viewport.as_vec4();
                }
            }
        }
        Err(error) => error!("frontend_sun_veil_failed={error}"),
    }
}
fn lenses(world: &mut World) {
    if let Err(error) = world.run_system_once(crate::frontend_lens::update) {
        error!("frontend_lens_update_failed={error}");
    }
}
fn hover(world: &mut World) {
    if let Err(error) = world.run_system_once(crate::frontend_hover::update) {
        error!("frontend_hover_update_failed={error}");
    }
}
fn water_geometry(world: &mut World) {
    if world.resource::<State>().mounted.is_some()
        && !world.resource::<State>().building_cube
        && let Err(error) = world.run_system_once(crate::water_scene::update_geometry)
    {
        error!("frontend_water_geometry_failed={error}");
    }
}
fn water_animation(
    state: Res<State>,
    time: Res<Time>,
    scene: Option<Res<crate::world_scene::Scene>>,
    runtime: Option<ResMut<crate::water_scene::RuntimeWater>>,
    mut patterns: ResMut<Assets<crate::water_texture::Material>>,
) {
    if state.mounted.is_none() || state.building_cube {
        return;
    }
    if let (Some(scene), Some(mut runtime)) = (scene, runtime)
        && state.fixed_tick.is_none()
        && let Err(error) = runtime.advance_preview(time.delta_secs(), &scene, &mut patterns)
    {
        error!("frontend_water_preview_failed={error}");
    }
}
fn present(
    state: Res<State>,
    windows: Query<&Window>,
    mut nodes: Query<
        &mut Node,
        (
            With<Owned>,
            With<MaterialNode<crate::world_composition::Presentation>>,
        ),
    >,
) {
    if state.mounted.is_none() || state.building_cube {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    for mut node in &mut nodes {
        node.width = px(window.width());
        node.height = px(window.height());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn teardown_keeps_preexisting_and_later_foreign_assets() {
        let mut world = World::new();
        world.init_resource::<Assets<Mesh>>();
        world.init_resource::<Assets<Image>>();
        world.init_resource::<Assets<crate::world_scene::WorldColorMaterial>>();
        world.init_resource::<Assets<crate::sky_scene::SkyMaterial>>();
        world.init_resource::<Assets<crate::underside_scene::UndersideMaterial>>();
        world.init_resource::<Assets<crate::world_composition::Presentation>>();
        world.init_resource::<Assets<crate::scenery_scene::Material>>();
        world.init_resource::<Assets<crate::sword_scene::Material>>();
        world.init_resource::<Assets<crate::frontend_lens::Material>>();
        world.init_resource::<Assets<crate::frontend_hover::Material>>();
        world.init_resource::<Assets<crate::frontend_hover::Composite>>();
        world.init_resource::<Assets<crate::water_scene::Material>>();
        world.init_resource::<Assets<crate::water_texture::Material>>();
        world.init_resource::<Assets<crate::prop_scene::Material>>();
        world.init_resource::<Assets<bevy::render::storage::ShaderBuffer>>();
        let shared = world.resource_mut::<Assets<Image>>().add(Image::default());
        let before = AssetsOwned::snapshot(&world);
        let owned = world.resource_mut::<Assets<Image>>().add(Image::default());
        let assets = AssetsOwned::snapshot(&world).added_since(&before);
        let foreign = world.resource_mut::<Assets<Image>>().add(Image::default());
        assert_eq!(assets.release(&mut world), 1);
        let images = world.resource::<Assets<Image>>();
        assert!(images.contains(&shared));
        assert!(images.contains(&foreign));
        assert!(!images.contains(&owned));
    }
}
