//! Owned, static environment inspection of a prepared mission. No runtime ack.
use crate::{
    frontend_scene::{FrontendCamera, FrontendElement},
    mission_runtime::{MissionOwned, Runtime},
    world_scene,
};
use bevy::ecs::system::RunSystemOnce;
use bevy::{asset::AssetId, ecs::system::SystemParam, prelude::*};
use giants_game::mission_loading::Phase;
use std::collections::HashSet;

pub struct Plugin;
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EnvironmentUpdate;
fn load_preview_water(
    root: &std::path::Path,
    world: &giants_game::WorldAsset,
    eye: [f32; 3],
) -> Result<crate::water_scene::Scene, Box<dyn std::error::Error>> {
    let asset = giants_game::water::load_texture(root, &world.name)?;
    let state = crate::water_scene::ValidationState {
        eye,
        sun: world.sun,
        phases: [[0.; 2]; 5],
        background: [0; 3],
        sun_from_world: true,
    };
    let band = giants_game::water::band(
        [eye[0], eye[1]],
        [state.sun[0], state.sun[1]],
        world.config.sea_values[0],
        asset.water,
    )?;
    Ok(crate::water_scene::Scene {
        asset,
        state,
        band,
        autonomous: true,
    })
}
fn preview_water(
    scene: Res<world_scene::Scene>,
    runtime: Option<ResMut<crate::water_scene::RuntimeWater>>,
    time: Res<Time>,
    cine: Option<Res<crate::cinematic_camera::Cinematic>>,
    pipelines: Res<crate::PipelinesReady>,
    mut patterns: ResMut<Assets<crate::water_texture::Material>>,
) {
    if !pipelines.0 || scene.water.is_none() {
        return;
    }
    let Some(mut runtime) = runtime else { return };
    let dt = cine
        .as_ref()
        .map_or(time.delta_secs(), |c| c.animation_dt());
    if dt > 0.
        && let Err(error) = runtime.advance_preview(dt, &scene, &mut patterns)
    {
        warn!("Mission water preview: {error}");
    }
}
impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            bevy::pbr::MaterialPlugin::<world_scene::WorldColorMaterial>::default(),
            bevy::pbr::MaterialPlugin::<crate::sky_scene::SkyMaterial>::default(),
            bevy::pbr::MaterialPlugin::<crate::underside_scene::UndersideMaterial>::default(),
            bevy::pbr::MaterialPlugin::<crate::water_scene::Material>::default(),
            bevy::pbr::MaterialPlugin::<crate::prop_scene::Material>::default(),
            bevy::pbr::MaterialPlugin::<crate::scenery_scene::Material>::default(),
            bevy::pbr::MaterialPlugin::<crate::sword_scene::Material>::default(),
            UiMaterialPlugin::<crate::water_texture::Material>::default(),
            UiMaterialPlugin::<crate::world_composition::Presentation>::default(),
        ))
        .init_resource::<State>()
        .add_systems(
            Update,
            (
                sync,
                adopt,
                prepare_rigid,
                adopt_rigid,
                inspection,
                rigid_view,
            )
                .chain()
                .in_set(EnvironmentUpdate)
                .after(crate::reflection_cube::CubeUpdate)
                .run_if(crate::mecc_session::frontend_available)
                .after(crate::mission_runtime::audit)
                .before(crate::source_view::ViewUpdate),
        );
        app.add_systems(
            Update,
            (crate::water_scene::update_geometry, preview_water)
                .chain()
                .after(crate::mission_showcase::update)
                .before(crate::source_view::ViewUpdate)
                .run_if(resource_exists::<world_scene::Scene>)
                .run_if(crate::mecc_session::frontend_available),
        );
    }
}
#[derive(Component)]
pub struct EnvironmentEntity;
#[derive(Resource, Default)]
pub(crate) struct State {
    mounted: Option<Mounted>,
    failed: Option<u64>,
    inspecting: bool,
}
impl State {
    pub(crate) fn failed_ticket(&self) -> Option<u64> {
        self.failed
    }
    pub(crate) fn mounted_ticket(&self) -> Option<u64> {
        self.mounted.as_ref().map(|m| m.ticket)
    }
    pub(crate) fn end_inspection(&mut self, ticket: u64) {
        if self.mounted_ticket() == Some(ticket) {
            self.inspecting = false;
        }
    }
    pub(crate) fn inspect(&mut self, ticket: u64, eye: [f32; 3]) -> bool {
        let Some(mounted) = self.mounted.as_mut().filter(|m| m.ticket == ticket) else {
            return false;
        };
        mounted.eye = eye;
        mounted.lod_scale = 1.;
        self.inspecting = true;
        true
    }
    /// The same completed eye/LOD profile consumed by ordinary static draws.
    /// Gameplay must not depend on a retired cinematic-camera resource.
    pub(crate) fn source_detail(&self, ticket: u64) -> Option<([f32; 3], f32)> {
        self.mounted
            .as_ref()
            .filter(|m| m.ticket == ticket)
            .map(|m| (m.eye, m.lod_scale))
    }
    pub(crate) fn follow(&mut self, ticket: u64, eye: [f32; 3]) {
        if let Some(mounted) = self.mounted.as_mut().filter(|m| m.ticket == ticket) {
            mounted.eye = eye;
        }
    }
    pub(crate) fn idle(&self) -> bool {
        self.mounted.is_none()
    }
    pub(crate) fn presentation_ready(&self) -> bool {
        self.mounted
            .as_ref()
            .is_some_and(|m| m.pending_rigid.is_none() && m.rigid_adopted)
    }
}
struct Mounted {
    ticket: u64,
    entities: Vec<Entity>,
    gpu: OwnedGpu,
    eye: [f32; 3],
    lod_scale: f32,
    rigid_adopted: bool,
    pending_rigid: Option<crate::static_scene::Scene>,
    reflection: Option<Handle<Image>>,
}
#[derive(Default)]
struct OwnedGpu {
    meshes: HashSet<AssetId<Mesh>>,
    images: HashSet<AssetId<Image>>,
    terrain: HashSet<AssetId<world_scene::WorldColorMaterial>>,
    sky: HashSet<AssetId<crate::sky_scene::SkyMaterial>>,
    underside: HashSet<AssetId<crate::underside_scene::UndersideMaterial>>,
    presentation: HashSet<AssetId<crate::world_composition::Presentation>>,
    rgb: HashSet<AssetId<crate::prop_scene::Material>>,
    lit: HashSet<AssetId<crate::scenery_scene::Material>>,
    cube: HashSet<AssetId<crate::sword_scene::Material>>,
    water: HashSet<AssetId<crate::water_scene::Material>>,
    patterns: HashSet<AssetId<crate::water_texture::Material>>,
    buffers: HashSet<AssetId<bevy::render::storage::ShaderBuffer>>,
}
#[derive(SystemParam)]
struct AssetsForEnvironment<'w> {
    meshes: ResMut<'w, Assets<Mesh>>,
    images: ResMut<'w, Assets<Image>>,
    terrain: ResMut<'w, Assets<world_scene::WorldColorMaterial>>,
    sky: ResMut<'w, Assets<crate::sky_scene::SkyMaterial>>,
    underside: ResMut<'w, Assets<crate::underside_scene::UndersideMaterial>>,
    presentation: ResMut<'w, Assets<crate::world_composition::Presentation>>,
    rgb: ResMut<'w, Assets<crate::prop_scene::Material>>,
    lit: ResMut<'w, Assets<crate::scenery_scene::Material>>,
    cube: ResMut<'w, Assets<crate::sword_scene::Material>>,
    water: ResMut<'w, Assets<crate::water_scene::Material>>,
    patterns: ResMut<'w, Assets<crate::water_texture::Material>>,
    buffers: ResMut<'w, Assets<bevy::render::storage::ShaderBuffer>>,
}
impl OwnedGpu {
    fn count(&self) -> usize {
        self.meshes.len()
            + self.images.len()
            + self.terrain.len()
            + self.sky.len()
            + self.underside.len()
            + self.presentation.len()
            + self.rgb.len()
            + self.lit.len()
            + self.cube.len()
            + self.water.len()
            + self.patterns.len()
            + self.buffers.len()
    }
    fn release(&self, assets: &mut AssetsForEnvironment) {
        for id in &self.water {
            assets.water.remove(*id);
        }
        for id in &self.patterns {
            assets.patterns.remove(*id);
        }
        for id in &self.buffers {
            assets.buffers.remove(*id);
        }
        for id in &self.rgb {
            assets.rgb.remove(*id);
        }
        for id in &self.lit {
            assets.lit.remove(*id);
        }
        for id in &self.cube {
            assets.cube.remove(*id);
        }
        for id in &self.terrain {
            assets.terrain.remove(*id);
        }
        for id in &self.sky {
            assets.sky.remove(*id);
        }
        for id in &self.underside {
            assets.underside.remove(*id);
        }
        for id in &self.presentation {
            assets.presentation.remove(*id);
        }
        for id in &self.meshes {
            assets.meshes.remove(*id);
        }
        for id in &self.images {
            assets.images.remove(*id);
        }
    }
    fn remaining(&self, assets: &AssetsForEnvironment) -> usize {
        self.water
            .iter()
            .filter(|id| assets.water.contains(**id))
            .count()
            + self
                .patterns
                .iter()
                .filter(|id| assets.patterns.contains(**id))
                .count()
            + self
                .buffers
                .iter()
                .filter(|id| assets.buffers.contains(**id))
                .count()
            + self
                .rgb
                .iter()
                .filter(|id| assets.rgb.contains(**id))
                .count()
            + self
                .lit
                .iter()
                .filter(|id| assets.lit.contains(**id))
                .count()
            + self
                .cube
                .iter()
                .filter(|id| assets.cube.contains(**id))
                .count()
            + self
                .meshes
                .iter()
                .filter(|id| assets.meshes.contains(**id))
                .count()
            + self
                .images
                .iter()
                .filter(|id| assets.images.contains(**id))
                .count()
            + self
                .terrain
                .iter()
                .filter(|id| assets.terrain.contains(**id))
                .count()
            + self
                .sky
                .iter()
                .filter(|id| assets.sky.contains(**id))
                .count()
            + self
                .underside
                .iter()
                .filter(|id| assets.underside.contains(**id))
                .count()
            + self
                .presentation
                .iter()
                .filter(|id| assets.presentation.contains(**id))
                .count()
    }
}
fn sync(
    runtime: Res<Runtime>,
    frontend: Res<crate::frontend_scene::Scene>,
    mut state: ResMut<State>,
    mut commands: Commands,
    mut assets: AssetsForEnvironment,
    showcase: Res<crate::mission_showcase::State>,
    cube: Option<Res<crate::reflection_cube::Build>>,
) {
    let wanted = runtime
        .active_ticket()
        .filter(|t| crate::mission_runtime::live(*t, frontend.flow.ticket, frontend.flow.phase));
    if state
        .mounted
        .as_ref()
        .is_some_and(|m| Some(m.ticket) != wanted)
    {
        let mounted = state.mounted.take().unwrap();
        if mounted
            .reflection
            .as_ref()
            .is_some_and(|owned| cube.as_ref().and_then(|b| b.handle()).as_ref() == Some(owned))
        {
            // The generated cube is mission-owned, including when the opening
            // borrows it. Never leave its worker running after environment release.
            commands.remove_resource::<crate::reflection_cube::Build>();
        }
        for entity in &mounted.entities {
            commands.entity(*entity).try_despawn();
        }
        mounted.gpu.release(&mut assets);
        commands.remove_resource::<world_scene::Scene>();
        commands.remove_resource::<crate::static_scene::Scene>();
        commands.remove_resource::<crate::water_scene::RuntimeWater>();
        state.inspecting = false;
        println!(
            "mission_environment_released ticket={} entities_requested={} assets={} remaining_assets={}",
            mounted.ticket,
            mounted.entities.len(),
            mounted.gpu.count(),
            mounted.gpu.remaining(&assets)
        );
    }
    let Some(ticket) = wanted else {
        state.failed = None;
        state.inspecting = false;
        return;
    };
    if state.mounted.is_some() || state.failed == Some(ticket) {
        return;
    }
    match runtime.environment_candidate() {
        Ok(Some(candidate)) if candidate.ticket == ticket => {
            let water = if showcase.enabled() {
                match load_preview_water(frontend.data_root(), &candidate.asset, candidate.eye) {
                    Ok(water) => Some(water),
                    Err(error) => {
                        println!("mission_dressing_omission family=water reason={error}");
                        None
                    }
                }
            } else {
                None
            };
            let render_water = water.is_some();
            let rigid = crate::static_scene::Scene::from_prepared(
                std::sync::Arc::clone(&candidate.models),
                &candidate.asset.config,
            );
            println!(
                "mission_environment_prepare ticket={} world={} camera_source={} initial_frame_fraction=1 normal_first_visible_frame=unverified water=false actor_callbacks=false rigid_inspection=true ready=false",
                ticket, candidate.asset.name, candidate.camera_source
            );
            commands.insert_resource(world_scene::Scene {
                asset: candidate.asset,
                oblique: false,
                validation_camera: Some(candidate.camera),
                camera_source: candidate.camera_source,
                camera_origin: None,
                cinematic_capture_ready: false,
                no_fog: false,
                render_terrain: true,
                render_sky: true,
                render_underside: true,
                reflection_profile: false,
                water,
                render_water,
            });
            commands.run_system_cached(world_scene::setup);
            let pending_rigid = rigid
                .map_err(|error| println!("mission_rigid_blocked ticket={ticket} reason={error}"))
                .ok();
            state.mounted = Some(Mounted {
                ticket,
                entities: Vec::new(),
                gpu: OwnedGpu::default(),
                eye: candidate.eye,
                lod_scale: candidate.lod_scale,
                rigid_adopted: false,
                pending_rigid,
                reflection: None,
            });
        }
        Err(error) => {
            println!("mission_environment_blocked ticket={ticket} reason={error}");
            state.failed = Some(ticket);
        }
        _ => {}
    }
}
/// One generated cube belongs to the environment ticket. Openings borrow it;
/// their actor cleanup must not destroy the world's reflective materials.
pub(crate) fn prepare_reflection(world: &mut World, ticket: u64) -> Result<(), String> {
    let mounted = world
        .resource::<State>()
        .mounted
        .as_ref()
        .filter(|m| m.ticket == ticket)
        .ok_or("reflection environment ticket is not mounted")?;
    if world.resource::<crate::frontend_scene::Scene>().flow.ticket != ticket {
        return Err("reflection environment ticket is stale".into());
    }
    if let Some(owned) = &mounted.reflection {
        return if world
            .get_resource::<crate::reflection_cube::Build>()
            .and_then(|b| b.handle())
            .as_ref()
            == Some(owned)
        {
            Ok(())
        } else {
            Err("mission reflection worker ownership changed".into())
        };
    }
    if world.contains_resource::<crate::reflection_cube::Build>() {
        return Err("unrelated reflection worker is still mounted".into());
    }
    let root = world
        .resource::<crate::frontend_scene::Scene>()
        .data_root()
        .to_owned();
    crate::reflection_cube::prepare_cinematic_water_in(world, &root).map_err(|e| e.to_string())?;
    // Reuse the existing source-driven six-face builder. Dressing owns its
    // own draws; rebuilding it on cube completion would duplicate entities.
    crate::reflection_cube::begin_cinematic(world, true, false);
    let views: Vec<_> = world
        .query_filtered::<Entity, With<world_scene::Rendered>>()
        .iter(world)
        .collect();
    for entity in views {
        world.despawn(entity);
    }
    world
        .run_system_once(world_scene::setup)
        .map_err(|e| e.to_string())?;
    adopt_new(world);
    let cube = world
        .resource::<crate::reflection_cube::Build>()
        .handle()
        .ok_or("mission reflection cube allocation missing")?;
    let mounted = world
        .resource_mut::<State>()
        .into_inner()
        .mounted
        .as_mut()
        .unwrap();
    mounted.gpu.images.insert(cube.id());
    mounted.reflection = Some(cube);
    println!(
        "mission_reflection_owned ticket={ticket} source=selected_world faces=6 capture_input=false opening_borrowed=true ready=false"
    );
    Ok(())
}

fn prepare_rigid(world: &mut World) {
    let Some(mounted) = world.resource::<State>().mounted.as_ref() else {
        return;
    };
    let Some(pending) = &mounted.pending_rigid else {
        return;
    };
    let ticket = mounted.ticket;
    if pending.needs_cube() {
        if let Err(error) = prepare_reflection(world, ticket) {
            println!("mission_rigid_blocked ticket={ticket} reason={error}");
            world
                .resource_mut::<State>()
                .mounted
                .as_mut()
                .unwrap()
                .pending_rigid = None;
            return;
        }
        if !world
            .resource::<crate::reflection_cube::Build>()
            .playback_ready()
        {
            return;
        }
    }
    let scene = world
        .resource_mut::<State>()
        .mounted
        .as_mut()
        .unwrap()
        .pending_rigid
        .take()
        .unwrap();
    world.insert_resource(scene);
    world
        .run_system_once(crate::static_scene::setup)
        .expect("owned mission rigid setup");
}

type UnownedRigids<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static crate::static_scene::SourceRigid),
    (
        With<crate::static_scene::RenderedRigid>,
        Without<MissionOwned>,
    ),
>;
fn adopt_rigid(
    mut commands: Commands,
    mut state: ResMut<State>,
    scene: Option<Res<crate::static_scene::Scene>>,
    rendered: UnownedRigids,
) {
    let Some(mounted) = state.mounted.as_mut() else {
        return;
    };
    let Some(scene) = scene else {
        return;
    };
    if mounted.rigid_adopted {
        return;
    }
    let before = mounted.entities.len();
    for (entity, source) in &rendered {
        commands
            .entity(entity)
            .insert((MissionOwned(mounted.ticket), EnvironmentEntity));
        mounted.entities.push(entity);
        println!(
            "mission_rigid_entity ticket={} origin={:?} class={} entity={:?} visibility=inspection_only",
            mounted.ticket, source.key, source.class, entity
        );
    }
    let assets = scene.owned_assets();
    mounted.gpu.meshes.extend(assets.meshes.iter().copied());
    mounted.gpu.images.extend(assets.images.iter().copied());
    mounted.gpu.rgb.extend(assets.rgb.iter().copied());
    mounted.gpu.lit.extend(assets.lit.iter().copied());
    mounted.gpu.cube.extend(assets.cube.iter().copied());
    mounted.gpu.buffers.extend(assets.buffers.iter().copied());
    mounted.rigid_adopted = true;
    println!(
        "mission_rigid_staged ticket={} entities={} meshes={} images={} materials={} ready=false callbacks=false",
        mounted.ticket,
        mounted.entities.len() - before,
        assets.meshes.len(),
        assets.images.len(),
        assets.rgb.len() + assets.lit.len() + assets.cube.len()
    );
}
#[derive(SystemParam)]
struct RigidCinematic<'w, 's> {
    cinematic: Option<Res<'w, crate::cinematic_camera::Cinematic>>,
    last: Local<'s, Option<RigidViewKey>>,
    last_admission: Local<'s, Option<crate::static_scene::AdmissionSummary>>,
}
type RigidViewKey = (u64, bool, Option<usize>, Option<u32>);
fn rigid_view(
    state: Res<State>,
    scene: Option<Res<crate::static_scene::Scene>>,
    frontend: Res<crate::frontend_scene::Scene>,
    world: Option<Res<world_scene::Scene>>,
    mut visibility: Query<&mut Visibility>,
    mut materials: crate::static_scene::Materials,
    selection: RigidCinematic,
) {
    let RigidCinematic {
        cinematic,
        mut last,
        mut last_admission,
    } = selection;
    let Some(mounted) = &state.mounted else {
        *last = None;
        return;
    };
    let (Some(scene), Some(world)) = (scene, world) else {
        return;
    };
    let showing = state.inspecting
        || matches!(
            frontend.flow.phase,
            Phase::Cinematic | Phase::Objectives | Phase::Gameplay
        );
    let cinematic =
        cinematic.filter(|_| !state.inspecting && frontend.flow.phase == Phase::Cinematic);
    let key = (
        mounted.ticket,
        showing,
        cinematic.as_ref().map(|c| c.director_index()),
        cinematic.as_ref().map(|c| c.ordinary_range().to_bits()),
    );
    if *last == Some(key) && !world.is_changed() {
        return;
    }
    let admission = cinematic.as_ref().map(|c| {
        scene.ordinary_view(
            c.sequence(),
            mounted.eye,
            c.ordinary_range(),
            &mut last_admission,
        )
    });
    let visible = crate::static_scene::apply_view(
        &scene,
        &world,
        mounted.eye,
        mounted.lod_scale,
        crate::static_scene::ViewAdmission {
            show: showing,
            ordinary: admission.as_deref(),
        },
        &mut visibility,
        &mut materials,
    );
    // Camera changes still update visibility/materials every frame. The detailed
    // clip audit is a snapshot on admission, not a per-frame diagnostic stream.
    if *last != Some(key) {
        println!(
            "mission_rigid_view ticket={} inspection={} selected_render_entries={} camera_profile={} lod_quality=2 callbacks=false ready=false",
            mounted.ticket, state.inspecting, visible, world.camera_source
        );
    }
    if state.inspecting && *last != Some(key) {
        for (key, slot, part, plane) in scene.clip_audit(&world, mounted.eye, mounted.lod_scale) {
            println!(
                "mission_rigid_clip ticket={} origin={:?} slot={} part={} outside_plane={} scope=cpu_clip_only",
                mounted.ticket, key, slot, part, plane
            );
        }
    }
    *last = Some(key);
}
type RenderParts<'a> = (
    Entity,
    Option<&'a Mesh3d>,
    Option<&'a MeshMaterial3d<world_scene::WorldColorMaterial>>,
    Option<&'a MeshMaterial3d<crate::sky_scene::SkyMaterial>>,
    Option<&'a MeshMaterial3d<crate::underside_scene::UndersideMaterial>>,
    Option<&'a MaterialNode<crate::world_composition::Presentation>>,
    Option<&'a MeshMaterial3d<crate::water_scene::Material>>,
    Option<&'a MaterialNode<crate::water_texture::Material>>,
);
pub(crate) fn adopt_new(world: &mut World) {
    world
        .run_system_cached(adopt)
        .expect("new mission world view ownership");
}
pub(crate) fn restore_full_view(world: &mut World, ticket: u64) {
    if world
        .get_resource::<State>()
        .and_then(State::mounted_ticket)
        != Some(ticket)
    {
        return;
    }
    let entities = world
        .resource::<State>()
        .mounted
        .as_ref()
        .unwrap()
        .entities
        .clone();
    let mut presentations = Vec::new();
    for entity in entities {
        if world.get::<Camera3d>(entity).is_some()
            && let Some(mut camera) = world.get_mut::<Camera>(entity)
        {
            camera.viewport = None;
        }
        if let Some(node) =
            world.get::<MaterialNode<crate::world_composition::Presentation>>(entity)
        {
            presentations.push(node.0.clone());
        }
    }
    let mut assets = world.resource_mut::<Assets<crate::world_composition::Presentation>>();
    for handle in presentations {
        if let Some(mut p) = assets.get_mut(&handle) {
            p.frame_viewport = Vec4::ZERO;
            p.fade = Vec4::ZERO;
            p.initial_fade = Vec4::ZERO;
        }
    }
    println!("mission_viewport_restored ticket={ticket} full_screen=true owned_only=true");
}
fn adopt(
    mut commands: Commands,
    mut state: ResMut<State>,
    rendered: Query<RenderParts, (With<world_scene::Rendered>, Without<MissionOwned>)>,
    assets: AssetsForEnvironment,
) {
    let Some(mounted) = state.mounted.as_mut() else {
        return;
    };
    let before = mounted.entities.len();
    for (entity, mesh, terrain, sky, underside, presentation, water, pattern) in &rendered {
        // UI nodes remain roots: adding a non-UI parent would alter layout.
        commands
            .entity(entity)
            .insert((MissionOwned(mounted.ticket), EnvironmentEntity));
        mounted.entities.push(entity);
        if let Some(mesh) = mesh {
            mounted.gpu.meshes.insert(mesh.0.id());
        }
        if let Some(material) = water {
            mounted.gpu.water.insert(material.0.id());
            if let Some(m) = assets.water.get(&material.0) {
                mounted.gpu.images.insert(m.image.id());
            }
        }
        if let Some(material) = pattern {
            mounted.gpu.patterns.insert(material.0.id());
            if let Some(m) = assets.patterns.get(&material.0) {
                mounted.gpu.images.insert(m.source.id());
            }
        }
        if let Some(material) = terrain {
            mounted.gpu.terrain.insert(material.0.id());
            if let Some(m) = assets.terrain.get(&material.0) {
                mounted.gpu.images.insert(m.texture.id());
                mounted.gpu.images.insert(m.additional_texture.id());
                mounted.gpu.buffers.insert(m.additional_uv_from_bevy.id());
                mounted.gpu.buffers.insert(m.clip_from_bevy.id());
                mounted.gpu.buffers.insert(m.uv_from_bevy.id());
            }
        }
        if let Some(material) = sky {
            mounted.gpu.sky.insert(material.0.id());
            if let Some(m) = assets.sky.get(&material.0) {
                mounted.gpu.images.insert(m.texture.id());
            }
        }
        if let Some(material) = underside {
            mounted.gpu.underside.insert(material.0.id());
            if let Some(m) = assets.underside.get(&material.0) {
                mounted.gpu.images.insert(m.texture.id());
            }
        }
        if let Some(material) = presentation {
            mounted.gpu.presentation.insert(material.0.id());
            if let Some(m) = assets.presentation.get(&material.0) {
                mounted.gpu.images.insert(m.image.id());
            }
        }
    }
    if mounted.entities.len() != before {
        println!(
            "mission_environment_staged ticket={} entities={} meshes={} images={} materials={} inspection_key=F6 ready=false",
            mounted.ticket,
            mounted.entities.len(),
            mounted.gpu.meshes.len(),
            mounted.gpu.images.len(),
            mounted.gpu.terrain.len()
                + mounted.gpu.sky.len()
                + mounted.gpu.underside.len()
                + mounted.gpu.presentation.len()
        );
    }
}
#[derive(SystemParam)]
struct InspectionUi<'w, 's> {
    world_cameras:
        Query<'w, 's, &'static mut Camera, (With<EnvironmentEntity>, Without<FrontendCamera>)>,
    frontend_camera: FrontendCameras<'w, 's>,
    elements: FrontendElements<'w, 's>,
    presentation: Query<
        'w,
        's,
        &'static mut Node,
        (
            With<EnvironmentEntity>,
            With<MaterialNode<crate::world_composition::Presentation>>,
        ),
    >,
    windows: Query<'w, 's, &'static Window>,
}
type FrontendCameras<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static mut Camera),
    (With<FrontendCamera>, Without<EnvironmentEntity>),
>;
type FrontendElements<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut Visibility,
        Option<&'static UiTargetCamera>,
    ),
    With<FrontendElement>,
>;
fn inspection(
    keys: Res<ButtonInput<KeyCode>>,
    frontend: Res<crate::frontend_scene::Scene>,
    mut state: ResMut<State>,
    mut ui: InspectionUi,
    mut commands: Commands,
    cube: Option<Res<crate::reflection_cube::Build>>,
    showcase: Res<crate::mission_showcase::State>,
) {
    if keys.just_pressed(KeyCode::F6)
        && !showcase.enabled()
        && let Some(mounted) = &state.mounted
    {
        let ticket = mounted.ticket;
        state.inspecting = !state.inspecting;
        println!(
            "mission_environment_inspection ticket={} visible={} camera_bound=false ready=false",
            ticket, state.inspecting
        );
    }
    let visible = state.mounted.is_some()
        && (state.inspecting
            || matches!(
                frontend.flow.phase,
                Phase::Cinematic | Phase::Objectives | Phase::Gameplay
            ));
    let hide_ui = showcase.presenting()
        || state.inspecting
        || matches!(frontend.flow.phase, Phase::Cinematic | Phase::Gameplay);
    for mut camera in &mut ui.world_cameras {
        if cube.as_ref().is_none_or(|b| b.complete) && camera.is_active != visible {
            camera.is_active = visible;
        }
    }
    let frontend_camera = ui.frontend_camera.iter().next().map(|(entity, _)| entity);
    let frontend_visible = !hide_ui;
    for (_, mut camera) in &mut ui.frontend_camera {
        if camera.is_active != frontend_visible {
            camera.is_active = frontend_visible;
        }
    }
    for (entity, mut v, target) in &mut ui.elements {
        if let Some(camera) = frontend_camera
            && target.is_none_or(|t| t.0 != camera)
        {
            commands.entity(entity).insert(UiTargetCamera(camera));
        }
        let value = if hide_ui {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        if *v != value {
            *v = value;
        }
    }
    if let Ok(window) = ui.windows.single() {
        let scale = (window.width() / 1920.).min(window.height() / 1080.);
        for mut node in &mut ui.presentation {
            let width = px(1920. * scale);
            let height = px(1080. * scale);
            let left = px((window.width() - 1920. * scale) * 0.5);
            let top = px((window.height() - 1080. * scale) * 0.5);
            if (node.width, node.height, node.left, node.top) != (width, height, left, top) {
                node.width = width;
                node.height = height;
                node.left = left;
                node.top = top;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::SystemState;
    #[test]
    fn release_is_idempotent_and_preserves_unrelated_and_later_assets() {
        let mut world = World::new();
        world.init_resource::<Assets<Mesh>>();
        world.init_resource::<Assets<Image>>();
        world.init_resource::<Assets<world_scene::WorldColorMaterial>>();
        world.init_resource::<Assets<crate::sky_scene::SkyMaterial>>();
        world.init_resource::<Assets<crate::underside_scene::UndersideMaterial>>();
        world.init_resource::<Assets<crate::world_composition::Presentation>>();
        world.init_resource::<Assets<crate::prop_scene::Material>>();
        world.init_resource::<Assets<crate::scenery_scene::Material>>();
        world.init_resource::<Assets<crate::sword_scene::Material>>();
        world.init_resource::<Assets<crate::water_scene::Material>>();
        world.init_resource::<Assets<crate::water_texture::Material>>();
        world.init_resource::<Assets<bevy::render::storage::ShaderBuffer>>();
        let buffer = world
            .resource_mut::<Assets<bevy::render::storage::ShaderBuffer>>()
            .add(bevy::render::storage::ShaderBuffer::from(vec![0u32; 4]));
        let external_buffer = world
            .resource_mut::<Assets<bevy::render::storage::ShaderBuffer>>()
            .add(bevy::render::storage::ShaderBuffer::from(vec![1u32; 4]));
        let owned = world.resource_mut::<Assets<Image>>().add(Image::default());
        let external = world.resource_mut::<Assets<Image>>().add(Image::default());
        let mut gpu = OwnedGpu::default();
        gpu.images.insert(owned.id());
        gpu.buffers.insert(buffer.id());
        let mut system = SystemState::<AssetsForEnvironment>::new(&mut world);
        {
            let mut assets = system.get_mut(&mut world).unwrap();
            gpu.release(&mut assets);
            assert_eq!(gpu.remaining(&assets), 0);
            assert!(assets.images.contains(external.id()));
            assert!(!assets.buffers.contains(buffer.id()));
            assert!(assets.buffers.contains(external_buffer.id()));
        }
        let later = world.resource_mut::<Assets<Image>>().add(Image::default());
        {
            let mut assets = system.get_mut(&mut world).unwrap();
            gpu.release(&mut assets);
            assert!(assets.images.contains(later.id()));
            assert!(assets.images.contains(external.id()));
        }
    }
}
