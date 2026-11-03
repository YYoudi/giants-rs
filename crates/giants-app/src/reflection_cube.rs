//! Build the measured cube on the GPU; captured cube images are never inputs.
use bevy::{
    prelude::*,
    reflect::TypePath,
    render::{
        Extract, ExtractSchedule, Render, RenderApp, RenderSystems,
        gpu_readback::{ReadbackComplete, ReadbackOnce},
        render_asset::RenderAssets,
        render_resource::{
            AsBindGroup, CommandEncoderDescriptor, Extent3d, Origin3d, TexelCopyTextureInfo,
            TextureAspect, TextureDimension, TextureFormat, TextureUsages, TextureViewDescriptor,
            TextureViewDimension,
        },
        renderer::{RenderDevice, RenderQueue, render_system},
        texture::GpuImage,
    },
    shader::ShaderRef,
};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU8, Ordering},
    },
};

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CubeUpdate;

struct ResumeScene {
    layers: [bool; 4],
    water: Option<(crate::water_scene::ValidationState, bool)>,
    props: bool,
}
#[derive(Resource)]
pub struct Build {
    source: Option<Handle<Image>>,
    sword_camera: Option<crate::validation_camera::ValidationCamera>,
    sword_origin: Option<[f32; 3]>,
    sword_mode: bool,
    frontend_mode: bool,
    resume_scene: Option<ResumeScene>,
    continuous: bool,
    cube: Option<Handle<Image>>,
    face: u8,
    request: Option<u8>,
    copied: Arc<AtomicU8>,
    stable_frames: u32,
    pub complete: bool,
    readback_complete: bool,
    dump: Option<PathBuf>,
}
impl Build {
    pub fn new(dump: Option<PathBuf>) -> Self {
        Self {
            source: None,
            sword_camera: None,
            sword_origin: None,
            sword_mode: false,
            frontend_mode: false,
            resume_scene: None,
            continuous: false,
            cube: None,
            face: 0,
            request: None,
            copied: Arc::new(AtomicU8::new(255)),
            stable_frames: 0,
            complete: false,
            readback_complete: false,
            dump,
        }
    }
    pub fn playback_ready(&self) -> bool {
        self.complete && self.stable_frames >= 4
    }
    pub fn frontend() -> Self {
        Self {
            frontend_mode: true,
            ..Self::new(None)
        }
    }
    pub fn handle(&self) -> Option<Handle<Image>> {
        self.cube.clone()
    }
    pub fn set_source(&mut self, source: Handle<Image>, images: &mut Assets<Image>) {
        images
            .get_mut(&source)
            .expect("World target")
            .texture_descriptor
            .usage |= TextureUsages::COPY_SRC;
        let camera = giants_game::reflection::story4_camera(0).expect("Initial source face");
        info!(
            "reflection_cube_camera_face=0 view_bits={:?} projection_bits={:?}",
            camera.view.map(f32::to_bits),
            camera.projection.map(f32::to_bits)
        );
        let mut cube = Image::new_uninit(
            Extent3d {
                width: 256,
                height: 256,
                depth_or_array_layers: 6,
            },
            TextureDimension::D2,
            TextureFormat::Rgba8Unorm,
            bevy::asset::RenderAssetUsages::default(),
        );
        cube.texture_descriptor.usage =
            TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::COPY_SRC;
        cube.texture_view_descriptor = Some(TextureViewDescriptor {
            dimension: Some(TextureViewDimension::Cube),
            ..default()
        });
        cube.sampler = if self.sword_mode || self.frontend_mode {
            bevy::image::ImageSampler::linear()
        } else {
            bevy::image::ImageSampler::nearest()
        };
        self.source = Some(source);
        self.cube = Some(images.add(cube));
    }
}

#[derive(Resource, Clone)]
struct Request {
    source: Handle<Image>,
    cube: Handle<Image>,
    face: Option<u8>,
    copied: Arc<AtomicU8>,
}
fn extract(mut commands: Commands, build: Extract<Option<Res<Build>>>) {
    if let Some(build) = build.as_ref()
        && let (Some(source), Some(cube)) = (&build.source, &build.cube)
    {
        commands.insert_resource(Request {
            source: source.clone(),
            cube: cube.clone(),
            face: build.request,
            copied: build.copied.clone(),
        });
    } else {
        commands.remove_resource::<Request>();
    }
}
fn copy_face(
    request: Option<Res<Request>>,
    images: Res<RenderAssets<GpuImage>>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    mut last: Local<Option<(bevy::asset::AssetId<Image>, u8)>>,
) {
    let Some(request) = request else { return };
    let Some(face) = request.face else { return };
    if *last == Some((request.cube.id(), face)) {
        return;
    }
    let (Some(source), Some(cube)) = (images.get(&request.source), images.get(&request.cube))
    else {
        return;
    };
    assert_eq!(
        cube.texture_descriptor.size,
        Extent3d {
            width: 256,
            height: 256,
            depth_or_array_layers: 6
        }
    );
    assert_eq!(cube.texture_descriptor.mip_level_count, 1);
    assert_eq!(cube.texture_descriptor.format, TextureFormat::Rgba8Unorm);
    assert_eq!(
        cube.texture_view_descriptor
            .as_ref()
            .and_then(|v| v.dimension),
        Some(TextureViewDimension::Cube)
    );
    if face == 0 {
        info!("reflection_cube_gpu_layout=256x256x6 levels=1 format=Rgba8Unorm view=Cube");
    }
    let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
        label: Some("Source reflection face to cube"),
    });
    encoder.copy_texture_to_texture(
        TexelCopyTextureInfo {
            texture: &source.texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: TextureAspect::All,
        },
        TexelCopyTextureInfo {
            texture: &cube.texture,
            mip_level: 0,
            origin: Origin3d {
                x: 0,
                y: 0,
                z: u32::from(face),
            },
            aspect: TextureAspect::All,
        },
        Extent3d {
            width: 256,
            height: 256,
            depth_or_array_layers: 1,
        },
    );
    queue.submit([encoder.finish()]);
    *last = Some((request.cube.id(), face));
    request.copied.store(face, Ordering::Release);
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct Preview {
    #[texture(0, dimension = "cube")]
    #[sampler(1)]
    pub cube: Handle<Image>,
}
impl UiMaterial for Preview {
    fn fragment_shader() -> ShaderRef {
        "embedded://giants_app/reflection_cube.wesl".into()
    }
}
pub fn install(app: &mut App, dump: Option<PathBuf>) {
    app.insert_resource(Build::new(dump));
    install_worker(app);
}
/// Register once; frontend mounts/removes its own Build while reusing one view.
pub fn install_worker(app: &mut App) {
    app.add_plugins(UiMaterialPlugin::<Preview>::default())
        .add_systems(
            Update,
            advance
                .in_set(CubeUpdate)
                .run_if(resource_exists::<Build>)
                .run_if(resource_exists::<crate::world_scene::Scene>)
                .before(crate::capture_frame)
                .before(crate::cinematic_camera::update),
        );
    app.sub_app_mut(RenderApp)
        .add_systems(ExtractSchedule, extract)
        .add_systems(
            Render,
            copy_face.after(render_system).in_set(RenderSystems::Render),
        );
}
pub fn install_sword(app: &mut App) {
    install(app, None);
    configure_sword(app.world_mut());
}
fn configure_sword(world: &mut World) {
    let original = {
        let mut scene = world.resource_mut::<crate::world_scene::Scene>();
        let original = scene
            .validation_camera
            .take()
            .expect("Frozen cinematic camera");
        scene.validation_camera = Some(crate::cinematic_camera::convert(
            &giants_game::reflection::story4_camera(0).unwrap(),
        ));
        let original_origin = scene.camera_origin.replace([0., 0., 800.]);
        scene.reflection_profile = true;
        scene.render_terrain = true;
        scene.render_sky = true;
        scene.render_underside = true;
        scene.render_water = true;
        scene.cinematic_capture_ready = false;
        (original, original_origin)
    };
    let mut build = world.resource_mut::<Build>();
    build.sword_camera = Some(original.0);
    build.sword_origin = original.1;
    build.sword_mode = true;
}
pub fn playback_ready(build: Option<Res<Build>>) -> bool {
    build.is_none_or(|build| build.playback_ready())
}
/// A late material dependency may request a cube in an actors-only view.
/// Prepare its source water data even when the final camera hides the sea.
/// The existing bounded initial-world cube policy remains unchanged.
pub fn prepare_cinematic_water(
    app: &mut App,
    root: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    prepare_cinematic_water_in(app.world_mut(), root)
}
pub(crate) fn prepare_cinematic_water_in(
    world: &mut World,
    root: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut scene = world.resource_mut::<crate::world_scene::Scene>();
    if scene.water.is_some() {
        return Ok(());
    }
    let asset = giants_game::water::load_texture(root, &scene.asset.name)?;
    let state = crate::water_scene::ValidationState {
        eye: [0.; 3],
        sun: scene.asset.sun,
        phases: [[0.; 2]; 5],
        background: [0; 3],
        sun_from_world: true,
    };
    let band = giants_game::water::band(
        [0., 0.],
        [state.sun[0], state.sun[1]],
        scene.asset.config.sea_values[0],
        asset.water,
    )?;
    scene.water = Some(crate::water_scene::Scene {
        asset,
        state,
        band,
        autonomous: true,
    });
    Ok(())
}
pub fn install_cinematic(app: &mut App, continuous: bool, props: bool) {
    install_worker(app);
    begin_cinematic(app.world_mut(), continuous, props);
}
/// Reuse an already registered GPU worker for a dynamically owned mission.
pub(crate) fn begin_cinematic(world: &mut World, continuous: bool, props: bool) {
    assert!(
        !world.contains_resource::<Build>(),
        "unrelated cube still mounted"
    );
    let resume = {
        let mut scene = world.resource_mut::<crate::world_scene::Scene>();
        let resume = ResumeScene {
            layers: [
                scene.render_terrain,
                scene.render_sky,
                scene.render_underside,
                scene.render_water,
            ],
            water: scene.water.as_ref().map(|w| (w.state, w.autonomous)),
            props,
        };
        let sea = scene.asset.config.sea_values[0];
        let water = scene.water.as_mut().expect("Cinematic cube water prepared");
        water.state.eye = [0., 0., 800.];
        water.state.phases =
            giants_game::reflection::story4_initial_water(water.asset.water.values[4])
                .expect("Measured initial water")
                .phases;
        water.autonomous = false;
        water.band = giants_game::water::band(
            [0., 0.],
            [water.state.sun[0], water.state.sun[1]],
            sea,
            water.asset.water,
        )
        .expect("Initial cube water band");
        resume
    };
    world.insert_resource(Build::new(None));
    configure_sword(world);
    let mut build = world.resource_mut::<Build>();
    build.resume_scene = Some(resume);
    build.continuous = continuous;
}
#[derive(bevy::ecs::system::SystemParam)]
struct Views<'w, 's> {
    cameras: Query<'w, 's, (&'static bevy::camera::RenderTarget, &'static mut Camera)>,
    world: Query<'w, 's, Entity, With<crate::world_scene::Rendered>>,
    prepared: PreparedScenes<'w>,
}

#[derive(bevy::ecs::system::SystemParam)]
struct PreparedScenes<'w> {
    bodies: Option<Res<'w, crate::actor_scene::Scene>>,
    rigid: Option<Res<'w, crate::rigid_attachment_scene::Scene>>,
    staff: Option<Res<'w, crate::attachment_scene::Scene>>,
    gun: Option<Res<'w, crate::gun_scene::Scene>>,
    spear: Option<Res<'w, crate::spear_scene::Scene>>,
    sword: Option<Res<'w, crate::sword_scene::Scene>>,
    props: Option<Res<'w, crate::prop_scene::Scene>>,
    scenery: Option<Res<'w, crate::scenery_scene::Scene>>,
    decoration: Option<Res<'w, crate::decoration_scene::Scene>>,
    foliage: Option<Res<'w, crate::foliage_scene::Scene>>,
}

fn advance(
    mut commands: Commands,
    mut build: ResMut<Build>,
    pipelines: Res<crate::PipelinesReady>,
    mut scene: ResMut<crate::world_scene::Scene>,
    mut materials: crate::world_scene::SceneMaterials,
    mut previews: ResMut<Assets<Preview>>,
    mut views: Views,
) {
    if build.complete {
        build.stable_frames = if pipelines.0 {
            build.stable_frames + 1
        } else {
            0
        };
        if !build.continuous && !build.frontend_mode {
            scene.cinematic_capture_ready =
                build.playback_ready() && (build.dump.is_none() || build.readback_complete);
        }
        return;
    }
    if build.copied.load(Ordering::Acquire) == build.face {
        info!(
            "reflection_cube_copied_face={} layer={} size=256x256 format=Rgba8Unorm",
            build.face, build.face
        );
        build.request = None;
        build.stable_frames = 0;
        if build.face == 5 {
            build.complete = true;
            let cube = build.cube.as_ref().unwrap().clone();
            for (target, mut camera) in &mut views.cameras {
                if matches!(target, bevy::camera::RenderTarget::Image(image) if Some(&image.handle) == build.source.as_ref())
                {
                    camera.is_active = false;
                }
            }
            if build.frontend_mode {
                // The frontend owns the phase transition and both asset lifetimes.
                info!("reflection_cube_resume=frontend_owner faces=6 captured_cube_inputs=none");
            } else if let Some(camera) = build.sword_camera.take() {
                for entity in &views.world {
                    commands.entity(entity).despawn();
                }
                scene.validation_camera = Some(camera);
                scene.camera_origin = build.sword_origin.take();
                scene.reflection_profile = false;
                scene.cinematic_capture_ready = false;
                commands.remove_resource::<crate::water_scene::RuntimeWater>();
                if let Some(resume) = build.resume_scene.take() {
                    [
                        scene.render_terrain,
                        scene.render_sky,
                        scene.render_underside,
                        scene.render_water,
                    ] = resume.layers;
                    let sea = scene.asset.config.sea_values[0];
                    if let Some((state, autonomous)) = resume.water {
                        let water = scene.water.as_mut().expect("Restored cinematic water");
                        water.state = state;
                        water.autonomous = autonomous;
                        water.band = giants_game::water::band(
                            [state.eye[0], state.eye[1]],
                            [state.sun[0], state.sun[1]],
                            sea,
                            water.asset.water,
                        )
                        .expect("Restored water band");
                    }
                    commands.run_system_cached(crate::world_scene::setup);
                    let prepared = &views.prepared;
                    // A cinematic can require the cube without installing every
                    // specialized scene. Do not queue a system whose owner is
                    // absent; dependencies of an installed scene stay mandatory.
                    macro_rules! resume_prepared {
                        ($field:ident, $setup:path) => {
                            if prepared.$field.is_some() {
                                commands.run_system_cached($setup);
                                info!("reflection_cube_resume_consumer={} installed=true queued=true", stringify!($field));
                            } else {
                                info!("reflection_cube_resume_consumer={} installed=false queued=false", stringify!($field));
                            }
                        };
                    }
                    resume_prepared!(bodies, crate::actor_scene::setup);
                    resume_prepared!(rigid, crate::rigid_attachment_scene::setup);
                    resume_prepared!(staff, crate::attachment_scene::setup);
                    resume_prepared!(gun, crate::gun_scene::setup);
                    resume_prepared!(spear, crate::spear_scene::setup);
                    resume_prepared!(sword, crate::sword_scene::setup);
                    if resume.props {
                        resume_prepared!(props, crate::prop_scene::setup);
                        resume_prepared!(scenery, crate::scenery_scene::setup);
                        resume_prepared!(decoration, crate::decoration_scene::setup);
                        resume_prepared!(foliage, crate::foliage_scene::setup);
                    }
                    info!(
                        "reflection_cube_resume=cinematic captured_cube_inputs=none continuous={}",
                        build.continuous
                    );
                } else {
                    scene.render_terrain = false;
                    scene.render_sky = false;
                    scene.render_underside = false;
                    scene.render_water = false;
                    scene.water = None;
                    commands.run_system_cached(crate::world_scene::setup);
                    commands.run_system_cached(crate::sword_scene::setup);
                    info!("reflection_cube_resume=sword_inspection captured_cube_inputs=none");
                }
            } else {
                let camera = commands
                    .spawn((
                        Camera2d,
                        Camera {
                            order: 3,
                            ..default()
                        },
                        Msaa::Off,
                    ))
                    .id();
                commands.spawn((
                    UiTargetCamera(camera),
                    MaterialNode(previews.add(Preview { cube: cube.clone() })),
                    LayoutConfig {
                        use_rounding: false,
                    },
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(0.),
                        top: px(0.),
                        width: px(768.),
                        height: px(512.),
                        ..default()
                    },
                ));
            }
            if build.dump.is_some() {
                commands
                    .spawn(ReadbackOnce::texture(cube))
                    .observe(save_readback);
            }
            info!("reflection_cube_ready=true faces=6 levels=1 source_images=none");
        } else {
            build.face += 1;
            let camera =
                giants_game::reflection::story4_camera(build.face).expect("Bounded source face");
            info!(
                "reflection_cube_camera_face={} view_bits={:?} projection_bits={:?}",
                build.face,
                camera.view.map(f32::to_bits),
                camera.projection.map(f32::to_bits)
            );
            scene.validation_camera = Some(crate::cinematic_camera::convert(&camera));
            crate::world_scene::apply_camera(&scene, &mut materials);
        }
        return;
    }
    build.stable_frames = if pipelines.0 {
        build.stable_frames + 1
    } else {
        0
    };
    if build.stable_frames >= 4 {
        build.request = Some(build.face);
    }
}
fn save_readback(event: On<ReadbackComplete>, mut build: ResMut<Build>) {
    if build.readback_complete {
        return;
    }
    assert_eq!(
        event.data.len(),
        6 * 256 * 256 * 4,
        "Unexpected cube readback layout"
    );
    if let Some(path) = &build.dump {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .expect("Fresh cube output");
        file.write_all(b"GCUBRGBA").unwrap();
        for value in [256u32, 256, 6] {
            file.write_all(&value.to_le_bytes()).unwrap();
        }
        file.write_all(&event.data).unwrap();
    }
    build.readback_complete = true;
    info!("reflection_cube_readback=true bytes={}", event.data.len());
}
