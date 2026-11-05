#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
use bevy::{
    app::AppExit,
    asset::RenderAssetUsages,
    image::ImageSampler,
    prelude::*,
    reflect::TypePath,
    render::{
        ExtractSchedule, MainWorld, RenderApp, RenderPlugin,
        render_resource::{AsBindGroup, Extent3d, TextureDimension, TextureFormat},
        settings::{Backends, RenderCreation, WgpuSettings},
        view::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk},
    },
    shader::ShaderRef,
    window::{PresentMode, WindowResolution},
};
use std::path::PathBuf;
mod actor_scene;
mod attachment_scene;
mod blood_scene;
mod body_context;
mod capture_readiness;
mod cinematic_audio;
mod cinematic_camera;
mod cinematic_effect_audio;
mod cinematic_prepare;
mod cinematic_scene;
mod cinematic_shadow_playback;
mod cinematic_text_scene;
mod cinematic_visibility;
mod decoration_scene;
mod desktop;
mod foliage_scene;
mod frontend_background;
mod frontend_feedback;
mod frontend_hover;
mod frontend_lens;
mod frontend_music;
mod frontend_scene;
mod frontend_sun;
mod gun_scene;
mod logo_material;
mod mecc_combat;
mod mecc_dev_scene;
mod mecc_incoming;
mod mecc_input;
mod mecc_session;
mod mecc_target;
mod mecc_weapon;
mod mesh_scene;
mod mission_ambient;
mod mission_audio;
mod mission_boot;
mod mission_carry;
mod mission_dressing;
mod mission_effects;
mod mission_environment;
mod mission_film;
mod mission_flight_audio;
mod mission_jet_scene;
mod mission_messages_hud;
mod mission_opening;
mod mission_player;
mod mission_runtime;
mod mission_showcase;
mod mission_weapon;
mod offscreen;
mod owned_effect_scene;
mod positional_pcm;
mod prop_scene;
mod reflection_cube;
mod render_profile;
mod rigid_attachment_scene;
mod scenery_scene;
mod shadow_receiver;
mod shadow_silhouette;
mod sky_scene;
mod source_instances;
mod source_view;
mod spear_scene;
mod static_scene;
mod sun_glare_scene;
mod sword_scene;
mod underside_scene;
mod validation_camera;
mod viewport_scene;
mod water_scene;
mod water_texture;
mod world_composition;
mod world_scene;
#[derive(Clone, Copy, PartialEq, Eq)]
enum Scenario {
    Loading,
    FrontendInspection,
    Mesh,
    Material,
    World,
    WaterTexture,
}

#[derive(Resource)]
struct LoadingPixels(Vec<u8>);
#[derive(Resource)]
struct Capture {
    effect_activation: bool,
    after_seconds: Option<f32>,
    hold_cinematic: bool,
    timed_effect: bool,
    projectile_effect: bool,
    burst_effect: bool,
    periodic_effect: bool,
    released_effect: bool,
    particle_effect: bool,
    attachment_loop: bool,
    timed_frames: u32,
    path: Option<PathBuf>,
    frame: u32,
    ready_frames: u32,
}
#[derive(Resource, Default)]
struct PipelinesReady(bool);
fn update_pipelines_ready(
    mut main_world: ResMut<MainWorld>,
    pipelines: Res<bevy::render::render_resource::PipelineCache>,
) {
    if let Some(mut ready) = main_world.get_resource_mut::<PipelinesReady>() {
        ready.0 = pipelines.pipelines().next().is_some()
            && pipelines.waiting_pipelines().next().is_none();
    }
}

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub(crate) struct LoadingMaterial {
    #[uniform(0)]
    uv_scale: Vec4,
    #[texture(1)]
    #[sampler(2)]
    image: Handle<Image>,
}
impl UiMaterial for LoadingMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://giants_app/loading.wesl".into()
    }
}

fn main() {
    let mut arguments: Vec<_> = std::env::args_os().skip(1).collect();
    let desktop = arguments.is_empty();
    if desktop {
        match desktop::arguments() {
            Ok(Some(args)) => arguments = args,
            Ok(None) => return,
            Err(error) => {
                desktop::error(&error.to_string());
                return;
            }
        }
    }
    if let Err(error) = launch(arguments) {
        eprintln!("Giants: {error}");
        if desktop {
            desktop::error(&format!(
                "Unable to start Giants: {error}\nSee Giants.log in your preview user-data folder."
            ));
        }
        std::process::exit(1);
    }
}
fn launch(arguments: Vec<std::ffi::OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut root = std::env::var_os("GIANTS_DATA_ROOT").map(PathBuf::from);
    let mut user_data_root = std::env::var_os("GIANTS_USER_DATA_ROOT").map(PathBuf::from);
    let mut capture = None;
    let mut present_mode = None;
    let mut offscreen = false;
    let mut gpu_mesh_preprocessing = None;
    let mut dump = None;
    let mut dump_rgb = None;
    let mut verify = false;
    let mut audit_cases: Option<PathBuf> = None;
    let mut audit_output: Option<PathBuf> = None;
    let mut scenario = Scenario::Loading;
    let mut owned_effect_input: Option<PathBuf> = None;
    let mut oblique = false;
    let mut dump_mesh = None;
    let mut dump_textures = None;
    let mut dump_world = None;
    let mut selected_world = None;
    let mut validation_camera = None;
    let mut no_fog = false;
    let mut world_layer = None;
    let mut validation_water_phases = None;
    let mut validation_water_state = None;
    let mut dump_water_texture = None;
    let mut dump_water_motion = None;
    let mut water_steps: Option<u32> = None;
    let mut reflection_face: Option<u8> = None;
    let mut reflection_cube = false;
    let mut dump_reflection_cube: Option<PathBuf> = None;
    let mut dump_reflection_camera: Option<PathBuf> = None;
    let mut gamegauge = false;
    let mut mecc_dev = false;
    let mut mecc_menu = false;
    let mut cinematic_preview = false;
    let mut cinematic_camera = false;
    let mut cinematic_opening = false;
    let mut cinematic_chain = false;
    let mut cinematic_directors: usize = 1;
    let mut cinematic_sequence: Option<String> = None;
    let mut cinematic_mission_index: Option<usize> = None;
    let mut cinematic_tick: Option<i32> = None;
    let mut capture_timed_effect = false;
    let mut capture_projectile_effect = false;
    let mut capture_burst_effect = false;
    let mut capture_after_seconds = None;
    let mut hold_cinematic_at_capture = false;
    let mut capture_periodic_effect = false;
    let mut capture_released_effect = false;
    let mut capture_particle_effect = false;
    let mut capture_attachment_loop = false;
    let mut silent_capture = false;
    let mut muted_transport = false;
    let mut scenery_class: Option<u32> = None;
    let mut actor_slot: Option<usize> = None;
    let mut cinematic_effect_offset: Option<usize> = None;
    let mut capture_effect_activation = false;
    let mut cinematic_tracer_only = false;
    let mut dump_cinematic_camera = None;
    let mut args = arguments.into_iter();
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--audit-cinematic-consumers") => {
                audit_cases = Some(args.next().ok_or("Audit needs cases TSV")?.into())
            }
            Some("--audit-output") => {
                audit_output = Some(args.next().ok_or("Audit needs fresh output TSV")?.into())
            }
            Some("--data-root") => {
                root = Some(args.next().ok_or("--data-root needs a path")?.into())
            }
            Some("--capture-timed-effect") => capture_timed_effect = true,
            Some("--hold-cinematic-at-capture") => hold_cinematic_at_capture = true,
            Some("--capture-projectile-effect") => capture_projectile_effect = true,
            Some("--capture-burst-effect") => capture_burst_effect = true,
            Some("--capture-after-seconds") => {
                let value = args
                    .next()
                    .ok_or("Capture threshold needs seconds")?
                    .to_string_lossy()
                    .parse::<f32>()?;
                if !value.is_finite() || !(0. ..=3600.).contains(&value) {
                    return Err("Capture threshold outside 0..3600 seconds".into());
                }
                capture_after_seconds = Some(value);
            }
            Some("--capture-periodic-effect") => capture_periodic_effect = true,
            Some("--capture-released-effect") => capture_released_effect = true,
            Some("--capture-particle-effect") => capture_particle_effect = true,
            Some("--capture-attachment-loop") => capture_attachment_loop = true,
            Some("--silent-capture") => silent_capture = true,
            Some("--muted-transport") => muted_transport = true,
            Some("--capture") => {
                capture = Some(PathBuf::from(
                    args.next().ok_or("--capture needs a PNG path")?,
                ))
            }
            Some("--present-mode") => {
                present_mode = Some(
                    match args.next().and_then(|v| v.into_string().ok()).as_deref() {
                        Some("fifo") => PresentMode::Fifo,
                        Some("auto-no-vsync") => PresentMode::AutoNoVsync,
                        _ => return Err("--present-mode requires fifo or auto-no-vsync".into()),
                    },
                );
            }
            Some("--dump-member") => {
                dump = Some(PathBuf::from(
                    args.next().ok_or("--dump-member needs a path")?,
                ))
            }
            Some("--dump-rgb") => {
                dump_rgb = Some(PathBuf::from(args.next().ok_or("--dump-rgb needs a path")?))
            }
            Some("--verify-only") => verify = true,
            Some("--mesh-preprocessing") => {
                gpu_mesh_preprocessing = Some(
                    match args.next().and_then(|v| v.into_string().ok()).as_deref() {
                        Some("gpu") => true,
                        Some("cpu") => false,
                        _ => return Err("--mesh-preprocessing requires gpu or cpu".into()),
                    },
                );
            }
            Some("--offscreen") => offscreen = true,
            Some("--mecc-dev") => {
                mecc_dev = true;
                scenario = Scenario::World;
            }
            Some("--mecc-menu") => {
                mecc_dev = true;
                mecc_menu = true;
                scenario = Scenario::World;
            }
            Some("--no-fog") => no_fog = true,
            Some("--reflection-cube") => reflection_cube = true,
            Some("--dump-reflection-cube") => {
                dump_reflection_cube = Some(args.next().ok_or("Cube output required")?.into());
            }
            Some("--reflection-face") => {
                reflection_face = Some(
                    args.next()
                        .ok_or("--reflection-face needs 0..5")?
                        .into_string()
                        .map_err(|_| "Invalid reflection face")?
                        .parse()?,
                );
                if reflection_face.is_some_and(|f| f > 5) {
                    return Err("Reflection face outside 0..5".into());
                }
            }
            Some("--dump-reflection-camera") => {
                dump_reflection_camera = Some(
                    args.next()
                        .ok_or("Reflection camera output required")?
                        .into(),
                )
            }
            Some("--cinematic-camera") => cinematic_camera = true,
            Some("--cinematic-chain") => {
                cinematic_chain = true;
                cinematic_camera = true;
            }
            Some("--cinematic-opening") => {
                cinematic_camera = true;
                cinematic_opening = true;
            }
            Some("--cinematic-sequence") => {
                cinematic_sequence = Some(
                    args.next()
                        .ok_or("--cinematic-sequence needs an identifier")?
                        .into_string()
                        .map_err(|_| "Invalid sequence identifier")?,
                );
                cinematic_camera = true;
            }
            Some("--cinematic-directors") => {
                cinematic_directors = args
                    .next()
                    .ok_or("--cinematic-directors needs 1..128")?
                    .into_string()
                    .map_err(|_| "Invalid director count")?
                    .parse()?;
                cinematic_camera = true;
            }
            Some("--cinematic-tick") => {
                cinematic_camera = true;
                cinematic_tick = Some(
                    args.next()
                        .ok_or("--cinematic-tick needs an integer")?
                        .into_string()
                        .map_err(|_| "Invalid cinematic tick")?
                        .parse()?,
                );
            }
            Some("--scenery-class") => {
                let value: u32 = args
                    .next()
                    .ok_or("--scenery-class needs53 or1053")?
                    .into_string()
                    .map_err(|_| "Invalid scenery class")?
                    .parse()?;
                if ![53, 1053].contains(&value) {
                    return Err("Unsupported scenery inspection class".into());
                }
                scenery_class = Some(value);
            }
            Some("--cinematic-tracer-only") => cinematic_tracer_only = true,
            Some("--capture-effect-activation") => capture_effect_activation = true,
            Some("--cinematic-effect-offset") => {
                cinematic_effect_offset = Some(
                    args.next()
                        .ok_or("--cinematic-effect-offset needs an offset")?
                        .into_string()
                        .map_err(|_| "Invalid effect offset")?
                        .parse()?,
                );
            }
            Some("--actor-slot") => {
                let slot: usize = args
                    .next()
                    .ok_or("--actor-slot needs 0..31")?
                    .into_string()
                    .map_err(|_| "Invalid actor slot")?
                    .parse()?;
                if !(0..32).contains(&slot) {
                    return Err("--actor-slot needs 0..31".into());
                }
                actor_slot = Some(slot);
            }
            Some("--dump-cinematic-camera") => {
                dump_cinematic_camera = Some(PathBuf::from(
                    args.next().ok_or("--dump-cinematic-camera needs a path")?,
                ));
            }
            Some("--world-layer") => {
                let value = args
                    .next()
                    .ok_or(
                        "--world-layer needs all, terrain, sky, underside, water, actors, props or foliage",
                    )?
                    .into_string()
                    .map_err(|_| "Invalid world layer")?;
                if !matches!(
                    value.as_str(),
                    "all"
                        | "terrain"
                        | "sky"
                        | "underside"
                        | "water"
                        | "actors"
                        | "props"
                        | "foliage"
                ) {
                    return Err(
                        "--world-layer needs all, terrain, sky, underside, water, actors, props or foliage"
                            .into(),
                    );
                }
                world_layer = Some(value);
            }
            Some("--world") => {
                let requested = args
                    .next()
                    .ok_or("--world needs a world catalog identifier")?
                    .into_string()
                    .map_err(|_| "World name must be Unicode")?;
                selected_world = Some(giants_game::world_source::canonical_name(&requested)?);
            }
            Some("--cinematic-mission-index") => {
                cinematic_mission_index = Some(
                    args.next()
                        .ok_or("Cinematic mission menu index required")?
                        .into_string()
                        .map_err(|_| "Mission index must be Unicode")?
                        .parse()?,
                );
            }
            Some("--validation-camera") => {
                validation_camera = Some(PathBuf::from(
                    args.next().ok_or("--validation-camera needs a GCAM file")?,
                ));
            }
            Some("--validation-water-phases") => {
                validation_water_phases = Some(PathBuf::from(
                    args.next()
                        .ok_or("--validation-water-phases needs GWPH file")?,
                ));
            }
            Some("--validation-water-state") => {
                validation_water_state = Some(PathBuf::from(
                    args.next()
                        .ok_or("--validation-water-state needs GWST file")?,
                ));
            }
            Some("--dump-water-texture") => {
                dump_water_texture = Some(PathBuf::from(
                    args.next().ok_or("--dump-water-texture needs a path")?,
                ));
            }
            Some("--water-steps") => {
                let value: u32 = args
                    .next()
                    .ok_or("--water-steps needs an integer")?
                    .into_string()
                    .map_err(|_| "Invalid water steps")?
                    .parse()?;
                if value > 1_000_000 {
                    return Err("Water steps exceed control budget".into());
                }
                water_steps = Some(value);
            }
            Some("--dump-water-motion") => {
                dump_water_motion = Some(PathBuf::from(
                    args.next().ok_or("--dump-water-motion needs a path")?,
                ));
            }
            Some("--dump-world") => {
                dump_world = Some(PathBuf::from(
                    args.next().ok_or("--dump-world needs a path")?,
                ))
            }
            Some("--dump-mesh") => {
                dump_mesh = Some(PathBuf::from(
                    args.next().ok_or("--dump-mesh needs a path")?,
                ))
            }
            Some("--dump-textures") => {
                dump_textures = Some(PathBuf::from(
                    args.next().ok_or("--dump-textures needs a path")?,
                ))
            }
            Some("--mesh-view") => match args.next().as_deref().and_then(|s| s.to_str()) {
                Some("front") => oblique = false,
                Some("oblique") => oblique = true,
                _ => return Err("--mesh-view expects front or oblique".into()),
            },
            Some("--user-data-root") => {
                user_data_root = Some(PathBuf::from(
                    args.next().ok_or("--user-data-root needs a folder")?,
                ));
            }
            Some("--scenario") => match args.next().as_deref().and_then(|s| s.to_str()) {
                Some("loading") => scenario = Scenario::Loading,
                Some("frontend-inspect") => scenario = Scenario::FrontendInspection,
                Some("mesh") => scenario = Scenario::Mesh,
                Some("material") => scenario = Scenario::Material,
                Some("world") => scenario = Scenario::World,
                Some("water-texture") => scenario = Scenario::WaterTexture,
                _ => {
                    return Err(
                        "Integrated scenarios: loading, frontend-inspect, mesh, material, world, water-texture"
                            .into(),
                    );
                }
            },
            Some("--owned-effect-input") => {
                owned_effect_input = Some(
                    args.next()
                        .ok_or("--owned-effect-input needs an inspection request file")?
                        .into(),
                )
            }
            Some("-gamegauge" | "--gamegauge") => gamegauge = true,
            Some("--cinematic-preview") => cinematic_preview = true,
            Some("--help") => {
                println!(
                    "--cinematic-chain follows the source links from --cinematic-sequence with real frame delta and its synchronization track. This remains an explicit cinematic inspection, separate from -gamegauge and campaign readiness."
                );
                println!(
                    "--cinematic-preview explicitly permits partial rendering at a frozen or selected cinematic pose; strict diagnostics remain the default."
                );
                println!(
                    "Rendering diagnostics: --present-mode fifo|auto-no-vsync (gamegauge defaults to Immediate, other modes to Fifo); --mesh-preprocessing gpu|cpu (world defaults to CPU); --offscreen (capture required, loading/world only, no window or presentation surface)."
                );
                println!(
                    "Live cinematic snapshots: --capture-after-seconds <minimum> with one explicit source and capture path; no effect required. Actual sampled clock is logged; asynchronous image timing and native fidelity are unverified."
                );
                println!(
                    "CPU consumer audit: --data-root <data> --audit-cinematic-consumers <world-sequence.tsv> --audit-output <new.tsv>; no App, GPU, window or audio is initialized."
                );
                println!(
                    "giants-app --data-root <game folder> [--scenario loading|frontend-inspect|mesh|material|world|water-texture] [--user-data-root <independent folder> (frontend-inspect)] [--world <catalog identifier>; story4 and intro aliases supported] [--mesh-view front|oblique] [--capture <new.png>] [--capture-timed-effect (live cinematic)] [--capture-projectile-effect (partial free-flight snapshot)] [--capture-burst-effect (partial burst snapshot)] [--capture-periodic-effect (partial periodic snapshot)] [--verify-only] [--dump-member <new file>] [--dump-rgb <new file>] [--dump-mesh <new file>] [--dump-textures <new file>] [--dump-world <new file>] [--dump-water-texture <new file>] [--water-steps <0..1000000>] [--dump-water-motion <new file>] [--reflection-face <0..5>] [--reflection-cube] [--dump-reflection-cube <new file>] [--dump-reflection-camera <new file>] [--cinematic-camera] [--cinematic-opening] [--cinematic-directors <1..128>] [-gamegauge|--gamegauge] [--cinematic-sequence <identifier>] [--cinematic-tick <first flick tick>] [--dump-cinematic-camera <new file>] [--validation-camera <GCAM file>] [--owned-effect-input <static owned FX request file>; standalone world only] [--validation-water-phases <GWPH file>] [--validation-water-state <GWST file>] [--no-fog] [--world-layer all|terrain|sky|underside|water|actors] [--actor-slot <0..31>] [--scenery-class <53|1053> with props layer]\nViewers: physical WASD, Space/Ctrl, Shift faster, right mouse look, R reset, Esc exit. material: first texture stage, inspection lights, no reflection. world: textured GTI, measured mipmaps and file fog; file GB2 sky and underside background; animated sun band with sun from world light pose; full sea/objects/cinematic remain incomplete. Cinematic opening: first Story4 director with its synchronization track. --cinematic-directors follows a requested count of supported source-linked directors; -gamegauge follows the complete source chain with partial rendering; Space pause, R restart. Cinematic camera: later Story4 director camera and four rigid body meshes, measured body materials, additive visor, staff and gun with source world lights; FLK VM remains unimplemented; Space pause, R restart; capture waits until its end. --cinematic-tick freezes a source-evaluated tick. --no-fog: world inspection override. Validation camera: explicit oracle comparison input, fixed view, Esc exit; world only. water-texture: animated five-layer producer preview, 256x256, measured 60 Hz benchmark profile; --water-steps freezes an independently computed step from zero; explicit oracle phases freeze the legacy comparison; no autonomous sea/sun or cinematic."
                );
                return Ok(());
            }
            _ => return Err(format!("Unknown argument: {}", arg.to_string_lossy()).into()),
        }
    }
    // A bounded explicit source chain is normal playback. Preserve the legacy
    // opening selection only when no explicit initial source was requested.
    if cinematic_directors > 1 && cinematic_sequence.is_none() {
        cinematic_opening = true;
    }
    if cinematic_chain
        && (gamegauge
            || cinematic_opening
            || cinematic_tick.is_some()
            || cinematic_sequence.is_none()
            || cinematic_directors != 1)
    {
        return Err(
            "--cinematic-chain requires a live explicit source, separate from benchmark options"
                .into(),
        );
    }
    if silent_capture && (!offscreen || capture.is_none() || scenario != Scenario::World) {
        return Err("--silent-capture requires offscreen world capture".into());
    }
    if silent_capture
        && std::env::var("GIANTS_MISSION_MECC_AUDIO").as_deref() == Ok("reserved")
        && std::env::var("GIANTS_MISSION_MECC_BACKEND").as_deref() == Ok("owned-inspection")
    {
        return Err(
            "reserved owned audio needs real sinks; --silent-capture disables the audio device"
                .into(),
        );
    }
    if muted_transport
        && (!silent_capture
            || !(cinematic_camera || mecc_menu)
            || cinematic_tick.is_some()
            || gamegauge)
    {
        return Err(
            "--muted-transport requires silent offscreen live cinematic or menu capture".into(),
        );
    }
    if muted_transport && mecc_menu && !mission_audio::muted() {
        return Err(
            "Menu muted transport requires GIANTS_AUDIO_MUTE=1 for every mission audio service"
                .into(),
        );
    }
    if hold_cinematic_at_capture
        && (capture_after_seconds.is_none()
            || !silent_capture
            || muted_transport
            || capture_timed_effect
            || capture_projectile_effect
            || capture_burst_effect
            || capture_periodic_effect
            || capture_released_effect
            || capture_particle_effect
            || capture_attachment_loop)
    {
        return Err("Cinematic inspection hold requires a silent time capture without audio or effect selectors".into());
    }
    if capture_after_seconds.is_some()
        && (capture.is_none()
            || scenario != Scenario::World
            || cinematic_sequence.is_none()
            || cinematic_tick.is_some()
            || cinematic_chain
            || cinematic_opening
            || cinematic_directors != 1
            || gamegauge)
    {
        return Err(
            "Capture threshold requires one live explicit cinematic and capture path".into(),
        );
    }
    if (capture_timed_effect
        || capture_projectile_effect
        || capture_periodic_effect
        || capture_burst_effect
        || capture_released_effect
        || capture_particle_effect
        || capture_attachment_loop)
        && (capture.is_none()
            || scenario != Scenario::World
            || cinematic_sequence.is_none()
            || cinematic_tick.is_some())
    {
        return Err("effect capture requires a live source cinematic and capture path".into());
    }
    if gamegauge {
        if cinematic_tick.is_some() || cinematic_sequence.is_some() || cinematic_directors != 1 {
            return Err(
                "Gamegauge follows the complete chain; use cinematic options separately".into(),
            );
        }
        scenario = Scenario::World;
        cinematic_camera = true;
        cinematic_opening = true;
        println!(
            "gamegauge_preview=true full_chain=true visual_fidelity=partial normal_cadence_verified=false"
        );
    }
    if cinematic_preview && (!cinematic_camera || gamegauge) {
        return Err(
            "--cinematic-preview requires a cinematic camera and is separate from -gamegauge"
                .into(),
        );
    }
    if owned_effect_input.is_some()
        && (scenario != Scenario::World
            || gamegauge
            || cinematic_camera
            || cinematic_preview
            || mecc_dev)
    {
        return Err("--owned-effect-input requires a standalone world inspection, without cinematic or mission playback".into());
    }
    let mesh = matches!(scenario, Scenario::Mesh | Scenario::Material);
    let explicit_present_mode = present_mode.is_some();
    let present_mode = present_mode.unwrap_or(if gamegauge {
        PresentMode::Immediate
    } else {
        PresentMode::default()
    });
    let gpu_mesh_preprocessing = gpu_mesh_preprocessing.unwrap_or(!matches!(
        scenario,
        Scenario::World | Scenario::FrontendInspection
    ));
    if offscreen
        && (capture.is_none()
            || !matches!(
                scenario,
                Scenario::Loading | Scenario::World | Scenario::FrontendInspection
            )
            || reflection_face.is_some())
    {
        return Err("--offscreen requires a capture and loading/world/frontend-inspect; isolated reflection faces remain separate".into());
    }
    let textured = scenario == Scenario::Material;
    let world = scenario == Scenario::World;
    if capture_effect_activation
        && (cinematic_effect_offset.is_none()
            || capture_after_seconds.is_some()
            || hold_cinematic_at_capture
            || cinematic_tick.is_some()
            || cinematic_directors != 1
            || muted_transport
            || capture_timed_effect
            || capture_projectile_effect
            || capture_burst_effect
            || capture_periodic_effect
            || capture_released_effect
            || capture_particle_effect
            || capture_attachment_loop)
    {
        return Err("Effect activation capture requires FX isolation without a time threshold or frozen tick".into());
    }
    if cinematic_tracer_only && cinematic_effect_offset.is_none() {
        return Err("Tracer-only inspection requires an isolated effect root".into());
    }
    if cinematic_effect_offset.is_some()
        && (!world
            || !cinematic_camera
            || !cinematic_preview
            || cinematic_sequence.is_none()
            || cinematic_chain
            || gamegauge
            || actor_slot.is_some()
            || !matches!(world_layer.as_deref(), None | Some("all"))
            || capture.is_none()
            || !offscreen
            || !silent_capture)
    {
        return Err("FX isolation requires a single explicit preview, silent offscreen capture and no actor filter".into());
    }
    if cinematic_mission_index.is_some()
        && (!world
            || !cinematic_camera
            || cinematic_sequence.is_none()
            || mecc_dev
            || gamegauge
            || cinematic_opening
            || actor_slot.is_some())
    {
        return Err("Cinematic mission context requires world, explicit sequence and full-scene inspection; not Mecc, benchmark, opening or isolated actor".into());
    }
    if mecc_dev
        && (!world
            || cinematic_camera
            || reflection_cube
            || reflection_face.is_some()
            || validation_camera.is_some()
            || selected_world.as_deref().is_some_and(|s| s != "story4"))
    {
        return Err("--mecc-dev requires the independent Story4 world without cinematic/reflection overrides".into());
    }
    let water_preview = scenario == Scenario::WaterTexture;
    if user_data_root.is_some() && scenario != Scenario::FrontendInspection && !mecc_dev {
        return Err("User-data root requires frontend-inspect or --mecc-dev".into());
    }
    let root = root
        .ok_or("Use --data-root or GIANTS_DATA_ROOT to select external game data")?
        .canonicalize()?;
    if let Some(cases) = audit_cases {
        if capture.is_some()
            || offscreen
            || cinematic_camera
            || gamegauge
            || mecc_dev
            || scenario != Scenario::Loading
        {
            return Err("CPU consumer audit is separate from interactive/render modes".into());
        }
        return cinematic_prepare::audit(
            &root,
            &cases,
            &audit_output.ok_or("CPU audit needs --audit-output")?,
        );
    }
    if audit_output.is_some() {
        return Err("--audit-output requires --audit-cinematic-consumers".into());
    }
    if scenario != Scenario::Loading && dump_rgb.is_some()
        || !mesh && dump_mesh.is_some()
        || scenario == Scenario::Loading && oblique
        || !textured && dump_textures.is_some()
        || !world
            && (dump_world.is_some()
                || selected_world.is_some() && !water_preview
                || validation_camera.is_some()
                || no_fog
                || world_layer.is_some())
        || !water_preview && !reflection_cube && validation_water_phases.is_some()
        || !water_preview && dump_water_texture.is_some()
        || !water_preview && (water_steps.is_some() || dump_water_motion.is_some())
        || water_steps.is_some() && validation_water_phases.is_some()
        || water_preview && dump.is_some()
        || validation_water_state.is_some() && (!world || validation_camera.is_none())
        || cinematic_camera && (!world || validation_camera.is_some())
        || (reflection_face.is_some() || reflection_cube)
            && (!world
                || cinematic_camera
                || validation_camera.is_some()
                || actor_slot.is_some()
                || reflection_face.is_some()
                    && selected_world.as_deref().is_some_and(|s| s != "story4"))
        || reflection_cube && (reflection_face.is_some() || world_layer.is_some() || no_fog)
        || reflection_cube
            && selected_world
                .as_deref()
                .is_some_and(|s| !matches!(s, "story4" | "intro"))
        || dump_reflection_cube.is_some() && !reflection_cube
        || dump_reflection_camera.is_some() && reflection_face.is_none()
        || actor_slot.is_some()
            && !matches!(world_layer.as_deref().unwrap_or("all"), "all" | "actors")
        || dump_cinematic_camera.is_some() && !cinematic_camera
        || (actor_slot.is_some() || matches!(world_layer.as_deref(), Some("actors" | "props")))
            && !cinematic_camera
    {
        return Err("Dump/view options do not match the selected scenario".into());
    }
    if scenery_class.is_some() && (world_layer.as_deref() != Some("props") || actor_slot.is_some())
    {
        return Err("Scenery class inspection requires the props layer and no actor slot".into());
    }
    let sword_inspection = !cinematic_preview
        && actor_slot == Some(8)
        && cinematic_sequence.as_deref() == Some("story4_t_square_3");
    if sword_inspection
        && (cinematic_tick.is_none()
            || cinematic_sequence.as_deref() != Some("story4_t_square_3")
            || world_layer.as_deref() != Some("actors"))
    {
        return Err(
            "Sword currently requires a frozen story4_t_square_3 actor-only inspection".into(),
        );
    }
    let mut validation_camera = validation_camera
        .as_deref()
        .map(validation_camera::ValidationCamera::read)
        .transpose()?;
    for output in [
        &capture,
        &dump,
        &dump_rgb,
        &dump_mesh,
        &dump_textures,
        &dump_world,
        &dump_water_texture,
        &dump_water_motion,
        &dump_cinematic_camera,
        &dump_reflection_camera,
        &dump_reflection_cube,
    ]
    .into_iter()
    .flatten()
    {
        if output.exists() {
            return Err(format!("Refusing to overwrite {}", output.display()).into());
        }
        let parent = output
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(std::path::Path::new("."))
            .canonicalize()?;
        if parent.starts_with(&root) {
            return Err("Output must stay outside the read-only game data root".into());
        }
    }
    let logo = if mesh {
        Some(giants_game::load_logo(&root)?)
    } else {
        None
    };
    let textures = if textured {
        giants_game::load_logo_textures(&root)?
    } else {
        Vec::new()
    };
    if let Some(asset) = &logo
        && textured
        && asset.mesh.subobjects.iter().any(|s| {
            !textures
                .iter()
                .any(|t| t.name.eq_ignore_ascii_case(&s.material.texture))
        })
    {
        return Err("Unsupported logo texture reference".into());
    }
    if let Some(path) = dump_textures {
        let mut bytes = b"GTEX0001".to_vec();
        for texture in &textures {
            bytes.extend(texture.width.to_le_bytes());
            bytes.extend(texture.height.to_le_bytes());
            for word in &texture.words {
                bytes.extend(word.to_le_bytes());
            }
        }
        std::fs::write(path, bytes)?;
    }
    if textured {
        println!(
            "material_preview=true first_stage=modulate4x specular=vertex_blinn source_textures=2 inspection_lights=true reflection_cube=absent native_material_fidelity=unverified"
        );
    }
    if mecc_dev && user_data_root.is_none() {
        let path = std::env::current_dir()?.join(".local/mecc-user");
        std::fs::create_dir_all(&path)?;
        user_data_root = Some(path);
    }
    let frontend = if scenario == Scenario::FrontendInspection || mecc_dev {
        Some(frontend_scene::Scene::load(
            &root,
            user_data_root
                .as_deref()
                .ok_or("frontend-inspect requires --user-data-root or GIANTS_USER_DATA_ROOT")?,
        )?)
    } else {
        None
    };
    let loading = if scenario == Scenario::Loading {
        Some(giants_game::load_mecc(&root)?)
    } else {
        None
    };
    let terrain = if world {
        Some(giants_game::load_world(
            &root,
            selected_world.as_deref().unwrap_or("story4"),
        )?)
    } else {
        None
    };
    let mut camera_origin = None;
    let cinematic_mission = if let Some(index) = cinematic_mission_index {
        let asset = terrain.as_ref().ok_or("Cinematic mission world missing")?;
        let bytes = std::fs::read(root.join("Bin").join(&asset.catalog))?;
        let world_name = std::path::Path::new(&asset.catalog)
            .file_stem()
            .and_then(|s| s.to_str())
            .and_then(|s| s.strip_prefix("w_"))
            .ok_or("World catalog identifier missing")?;
        let menu = giants_game::mission_selection::MissionMenu::parse(&bytes, u32::MAX, true)?;
        let request = menu.request(world_name, index)?;
        let packet = giants_game::mission_loading::prepare(&root, request)?;
        if !packet.needs.is_empty() {
            return Err(format!(
                "Cinematic mission has unresolved reader needs: {:?}",
                packet.needs
            )
            .into());
        }
        println!(
            "cinematic_mission_context world={} menu_index={} source_id={} mission={} runtime_ready=false",
            packet.request.world, index, packet.request.source_id, packet.request.mission_name
        );
        let parameters = packet.slot_parameters(&root)?;
        if let Some((offset, value)) = parameters.parameter13 {
            println!(
                "cinematic_mission_parameter13 slot={} source_offset={} raw_word={} runtime_executed=false",
                parameters.index, offset, value
            );
        }
        for (offset, class_id) in &parameters.linked_actors12 {
            println!(
                "cinematic_mission_linked_actor12 slot={} source_offset={} requested_class={} backend=unbound runtime_executed=false",
                parameters.index, offset, class_id
            );
        }
        if let Some(offset) = parameters.player_proxy34 {
            println!(
                "cinematic_mission_proxy34 slot={} source_offset={} requested_class=1076 backend=unbound runtime_executed=false",
                parameters.index, offset
            );
        }
        if let Some(offset) = parameters.flag36 {
            println!(
                "cinematic_mission_flag36 slot={} source_offset={} declared_slot_word=1 actor_reset=requires_runtime_context runtime_executed=false",
                parameters.index, offset
            );
        }
        if let Some(offset) = parameters.flag37 {
            println!(
                "cinematic_mission_flag37 slot={} source_offset={} declared_slot_word=1 declared_global_word=1 runtime_executed=false",
                parameters.index, offset
            );
        }
        Some(packet)
    } else {
        None
    };
    let cinematic = if cinematic_camera {
        let (resource, camera) = cinematic_camera::Cinematic::load(
            &root,
            cinematic_camera::Load {
                tick: cinematic_tick,
                benchmark: cinematic_opening,
                directors: cinematic_directors,
                sequence: cinematic_sequence.as_deref(),
                full_chain: gamegauge || cinematic_chain,
                preview: gamegauge || cinematic_preview,
            },
        )?;
        let world = terrain.as_ref().ok_or("Cinematic world missing")?;
        for segment in resource.segments() {
            if !world
                .config
                .cinematics
                .iter()
                .any(|entry| entry.name.eq_ignore_ascii_case(segment.sequence()))
            {
                return Err(format!(
                    "Cinematic {} is not declared in selected world {}",
                    segment.sequence(),
                    world.catalog
                )
                .into());
            }
        }
        validation_camera = Some(cinematic_camera::convert(&camera));
        camera_origin = Some(camera.origin);
        if let Some(path) = dump_cinematic_camera {
            std::fs::write(path, cinematic_camera::control(&camera))?;
        }
        Some(resource)
    } else {
        None
    };
    if let Some(face) = reflection_face.or(if reflection_cube { Some(0) } else { None }) {
        let camera = giants_game::reflection::story4_camera(face)?;
        if let Some(path) = dump_reflection_camera {
            std::fs::write(path, cinematic_camera::control(&camera))?;
        }
        validation_camera = Some(cinematic_camera::convert(&camera));
        camera_origin = Some(camera.origin);
        println!(
            "reflection_face={} reflection_camera=source_cube_profile reflection_water=frozen_initial_producer_profile native_cube_fidelity=unverified",
            face
        );
    }
    let cinematic_cube = !sword_inspection
        && actor_slot.is_none()
        && matches!(world_layer.as_deref().unwrap_or("all"), "all" | "actors")
        && cinematic
            .as_ref()
            .is_some_and(|c| c.segments().iter().any(|s| s.has_actor_class(669)));
    let water_texture = if water_preview {
        let mut phases = validation_water_phases
            .as_deref()
            .map(water_texture::read_phases)
            .transpose()?
            .unwrap_or([[0.; 2]; 5]);
        let asset =
            giants_game::water::load_texture(&root, selected_world.as_deref().unwrap_or("story4"))?;
        if let Some(steps) = water_steps {
            let mut motion = giants_game::water::Motion::default();
            for _ in 0..steps {
                motion.advance(1. / 60., asset.water.values[4], 0.)?;
            }
            phases = motion.phases;
        }
        let animate = water_steps.is_none() && validation_water_phases.is_none();
        println!(
            "water_texture_preview=true phases={} animation={} water_steps={:?} surface=false",
            if validation_water_phases.is_some() {
                "explicit_oracle"
            } else {
                "independent_motion"
            },
            animate,
            water_steps
        );
        Some(water_texture::Scene {
            asset,
            phases,
            animate,
        })
    } else {
        None
    };
    let water_scene = if world
        && (sword_inspection
            || cinematic_cube
            || validation_water_state.is_some()
            || matches!(world_layer.as_deref().unwrap_or("all"), "all" | "water"))
    {
        let world_asset = terrain.as_ref().ok_or("Missing world")?;
        let mut state = if let Some(path) = validation_water_state.as_deref() {
            water_scene::ValidationState::read(path)?
        } else {
            water_scene::ValidationState {
                eye: if sword_inspection {
                    [0., 0., 800.]
                } else if reflection_face.is_some() || reflection_cube {
                    camera_origin.unwrap_or([0.; 3])
                } else {
                    [0.; 3]
                },
                sun: world_asset.sun,
                phases: [[0.; 2]; 5],
                background: [0; 3],
                sun_from_world: true,
            }
        };
        if state.sun_from_world {
            state.sun = world_asset.sun;
        }
        let autonomous = validation_water_state.is_none()
            && reflection_face.is_none()
            && !reflection_cube
            && !sword_inspection;
        let asset =
            giants_game::water::load_texture(&root, selected_world.as_deref().unwrap_or("story4"))?;
        if reflection_face.is_some() || reflection_cube || sword_inspection {
            state.phases = if reflection_cube && selected_world.as_deref() == Some("intro") {
                // The first menu producer renders the zero phases before advancing
                // them; Story4 inherits that prior call, as the retained trace shows.
                giants_game::water::Motion::default().phases
            } else {
                giants_game::reflection::story4_initial_water(asset.water.values[4])?.phases
            };
        }
        if reflection_cube && let Some(path) = validation_water_phases.as_deref() {
            state.phases = water_texture::read_phases(path)?;
            println!(
                "reflection_cube_water_phases=explicit_comparison_input runtime_profile=diagnostic"
            );
        }
        let band = giants_game::water::band(
            [state.eye[0], state.eye[1]],
            [state.sun[0], state.sun[1]],
            terrain.as_ref().ok_or("Missing world")?.config.sea_values[0],
            asset.water,
        )?;
        println!(
            "water_surface=true sun_source={} sun={:?} animation={} fixed_water_state={}",
            if state.sun_from_world {
                "world_light_variant1"
            } else {
                "explicit_oracle"
            },
            state.sun,
            autonomous,
            !autonomous
        );
        Some(water_scene::Scene {
            asset,
            state,
            band,
            autonomous,
        })
    } else {
        None
    };
    if let Some(path) = dump_water_texture {
        std::fs::write(
            path,
            giants_game::water::texture_control(
                &water_texture.as_ref().ok_or("Missing water asset")?.asset,
            ),
        )?;
    }
    if let Some(path) = dump_water_motion {
        let mut out = b"GWPH0001".to_vec();
        out.extend(
            water_texture
                .as_ref()
                .ok_or("Missing water motion")?
                .phases
                .into_iter()
                .flatten()
                .flat_map(f32::to_le_bytes),
        );
        std::fs::write(path, out)?;
    }
    if let Some(asset) = &terrain {
        println!(
            "data_root={} world={} grid={}x{} triangles={} spacing={} opaque_tail_bytes={} oracle_inputs={} diagnostic_camera=true material_preview={} planar_uv=measured mipmaps=measured_polynomial anisotropy=16 native_image_fidelity={} sky=file_gb2 underside=above_water_quality2 full_sea_and_objects=incomplete fog_start={} fog_end={} fog_enabled={}",
            root.display(),
            asset.name,
            asset.terrain.width,
            asset.terrain.height,
            asset.triangles.len(),
            asset.terrain.spacing,
            asset.terrain.opaque_tail.len(),
            if validation_camera.is_some()
                && !cinematic_camera
                && reflection_face.is_none()
                && !reflection_cube
            {
                "validation_camera"
            } else {
                "none"
            },
            if asset.additional.is_some() {
                "dot3_modulate2x_secondary"
            } else {
                "add_texture_color"
            },
            if asset.additional.is_some() {
                "unverified_two_texture"
            } else {
                "under_control"
            },
            asset.config.fog.start,
            asset.config.fog.end,
            !no_fog,
        );
        if let Some(path) = &dump {
            std::fs::write(path, &asset.member)?;
        }
        if let Some(path) = &dump_world {
            std::fs::write(path, giants_game::world_control(asset))?;
        }
    }
    if let Some(asset) = &loading {
        println!(
            "data_root={} member=mecc_loading.tga bytes={} image={}x{} tiles=20",
            root.display(),
            asset.member.len(),
            asset.image.width,
            asset.image.height
        );
        if let Some(path) = &dump {
            std::fs::write(path, &asset.member)?;
        }
        if let Some(path) = &dump_rgb {
            std::fs::write(path, &asset.image.pixels)?;
        }
    }
    if let Some(asset) = &logo {
        println!(
            "data_root={} member=Giants_logo_3D.gbs positions={} vertices={} triangles={} oracle_inputs=none diagnostic_camera=true diagnostic_material=true",
            root.display(),
            asset.mesh.positions.len(),
            asset.mesh.indices.len(),
            asset
                .mesh
                .subobjects
                .iter()
                .map(|s| s.triangles.len())
                .sum::<usize>()
        );
        if let Some(path) = &dump {
            std::fs::write(path, &asset.member)?;
        }
        if let Some(path) = &dump_mesh {
            std::fs::write(path, giants_game::mesh_control(asset))?;
        }
    }
    if verify {
        return Ok(());
    }
    let requested_capture = capture.clone();
    println!(
        "present_mode_requested={present_mode:?} gamegauge={gamegauge} explicit_override={explicit_present_mode} offscreen={offscreen}"
    );
    let profile_render = std::env::var_os("GIANTS_PROFILE_RENDER").is_some();
    let mut renderer = WgpuSettings::default();
    if profile_render {
        use bevy::render::settings::WgpuFeatures;
        renderer.features |= WgpuFeatures::TIMESTAMP_QUERY
            | WgpuFeatures::TIMESTAMP_QUERY_INSIDE_ENCODERS
            | WgpuFeatures::TIMESTAMP_QUERY_INSIDE_PASSES;
    }
    if cfg!(any(target_os = "windows", target_os = "linux")) && Backends::from_env().is_none() {
        renderer.backends = Some(Backends::VULKAN);
    }
    println!(
        "mesh_preprocessing={}",
        if gpu_mesh_preprocessing { "gpu" } else { "cpu" }
    );
    let plugins = DefaultPlugins
            .set(bevy::audio::AudioPlugin {
                global_volume: bevy::audio::GlobalVolume::new(bevy::audio::Volume::Linear(if mission_audio::muted() { 0. } else { 1. })),
                ..default()
            })
            .set(bevy::pbr::PbrPlugin {
                use_gpu_instance_buffer_builder: gpu_mesh_preprocessing,
                ..default()
            })
            .set(RenderPlugin {
                render_creation: RenderCreation::Automatic(Box::new(renderer)),
                ..default()
            })
            .set(WindowPlugin {
                primary_window: (!offscreen).then_some(Window {
                    title: if mecc_dev {"Giants"} else if scenario == Scenario::FrontendInspection {"Giants - Showcase / F8 hints / F10 quit"} else if water_preview {"Giants Â· water texture Â· diagnostic / static phases / Esc"} else if gamegauge {"Giants Â· gamegauge preview Â· PARTIAL RENDERING Â· Space pause / R restart / Esc"} else if cinematic_camera {"Giants Â· cinematic preview Â· partial rendering / Space pause / R restart / Esc"} else if world && validation_camera.is_some() {"Giants Â· world validation Â· recorded camera / Esc"} else if world {"Giants Â· textured terrain Â· inspection camera / WASD / RMB / R"} else if mesh {
                        if textured { "Giants Â· material preview Â· inspection lights / no reflection Â· WASD / RMB / R" } else { "Giants Â· mesh viewer Â· diagnostic normals Â· WASD / RMB / R" }
                    } else {
                        "Giants Â· Rust + Bevy Â· loading slice"
                    }
                    .into(),
                    resolution: if mecc_dev { WindowResolution::new(1280, 720) } else { WindowResolution::new(1920, 1080) }.with_scale_factor_override(1.0),
                    present_mode,
                    resizable: mecc_dev,
                    ..default()
                }),
                exit_condition: if offscreen { bevy::window::ExitCondition::DontExit } else { bevy::window::ExitCondition::OnAllClosed },
                ..default()
            });
    let plugins = if offscreen {
        plugins.disable::<bevy::winit::WinitPlugin>()
    } else {
        plugins
    };
    let plugins = if muted_transport {
        println!(
            "silent_capture=true audio_plugin=true audio_clock=true global_volume=0 muted_transport=true"
        );
        plugins.set(bevy::audio::AudioPlugin {
            global_volume: bevy::audio::GlobalVolume::new(bevy::audio::Volume::Linear(0.)),
            ..default()
        })
    } else if silent_capture {
        println!("silent_capture=true audio_plugin=false audio_clock=false");
        plugins.disable::<bevy::audio::AudioPlugin>()
    } else {
        plugins
    };
    let mut app = App::new();
    app.insert_resource(Capture {
        effect_activation: capture_effect_activation,
        after_seconds: capture_after_seconds,
        hold_cinematic: hold_cinematic_at_capture,
        timed_effect: capture_timed_effect,
        projectile_effect: capture_projectile_effect,
        burst_effect: capture_burst_effect,
        periodic_effect: capture_periodic_effect,
        released_effect: capture_released_effect,
        particle_effect: capture_particle_effect,
        attachment_loop: capture_attachment_loop,
        timed_frames: 0,
        path: capture,
        frame: 0,
        ready_frames: 0,
    })
    .init_resource::<PipelinesReady>()
    .insert_resource(ClearColor(Color::BLACK))
    .add_plugins(plugins)
    .add_systems(
        PostUpdate,
        capture_frame
            .after(capture_readiness::sample_expected)
            .run_if(mecc_session::capture_allowed)
            .run_if(frontend_scene::mission_capture_allowed),
    )
    .add_systems(Update, cinematic_camera::update_title);
    if silent_capture {
        // Frontend/Mecc systems still own and retire their audio assets in a
        // silent replay. Register storage without installing a device or sinks.
        app.init_asset::<AudioSource>();
    }
    if muted_transport {
        app.insert_resource(cinematic_audio::MutedTransport);
    }
    capture_readiness::install(&mut app);
    if let Some(offset) = cinematic_effect_offset {
        app.insert_resource(rigid_attachment_scene::capture::EffectIsolation {
            offset,
            hold_on_activation: capture_effect_activation,
            tracer_only: cinematic_tracer_only,
        })
        .add_systems(
            PostUpdate,
            rigid_attachment_scene::capture::isolate_effect_meshes
                .after(cinematic_visibility::update)
                .before(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate),
        );
    }
    cinematic_text_scene::install(&mut app, &root);
    cinematic_effect_audio::install(
        &mut app,
        &root,
        !silent_capture || muted_transport,
        cinematic
            .as_ref()
            .filter(|c| c.realtime() && c.preview())
            .and(terrain.as_ref())
            .map(|w| w.name.as_str()),
    );
    app.add_systems(
        Update,
        world_composition::update_policy
            .after(cinematic_camera::update)
            .run_if(resource_exists::<Assets<world_composition::Presentation>>),
    );
    if !gpu_mesh_preprocessing {
        use bevy::render::{RenderStartup, batching::gpu_preprocessing::GpuPreprocessingSupport};
        // The CPU buffer builder and render-phase preparation must agree.
        // PbrPlugin's builder option alone leaves the phase policy at GPU mode
        // on capable adapters, so opaque batches are silently skipped.
        app.sub_app_mut(RenderApp)
            .insert_resource(GpuPreprocessingSupport {
                max_supported_mode:
                    bevy::render::batching::gpu_preprocessing::GpuPreprocessingMode::None,
            })
            .add_systems(
                RenderStartup,
                cpu_mesh_phase_policy
                    .after(bevy::render::init_gpu_resource::<GpuPreprocessingSupport>),
            );
    }
    if offscreen {
        println!("offscreen_render=true presentation_surface=false output=1920x1080_srgb");
        offscreen::install(&mut app);
    }
    cinematic_shadow_playback::install(&mut app, &root)?;
    app.sub_app_mut(RenderApp)
        .add_systems(ExtractSchedule, update_pipelines_ready);
    if profile_render {
        render_profile::install(&mut app);
    }
    // MaterialPlugin can request its shaders while being added, before app.run().
    bevy::asset::embedded_asset!(app, "loading.wesl");
    bevy::asset::embedded_asset!(app, "logo.wesl");
    bevy::asset::embedded_asset!(app, "world.wesl");
    bevy::asset::embedded_asset!(app, "sky.wesl");
    bevy::asset::embedded_asset!(app, "actor.wesl");
    bevy::asset::embedded_asset!(app, "blood.wesl");
    bevy::asset::embedded_asset!(app, "body_context.wesl");
    bevy::asset::embedded_asset!(app, "rigid_attachment.wesl");
    bevy::asset::embedded_asset!(app, "attachment.wesl");
    bevy::asset::embedded_asset!(app, "prop.wesl");
    bevy::asset::embedded_asset!(app, "gun.wesl");
    bevy::asset::embedded_asset!(app, "spear.wesl");
    bevy::asset::embedded_asset!(app, "sword.wesl");
    bevy::asset::embedded_asset!(app, "reflection_cube.wesl");
    bevy::asset::embedded_asset!(app, "frontend_lens.wesl");
    bevy::asset::embedded_asset!(app, "frontend_hover.wesl");
    bevy::asset::embedded_asset!(app, "frontend_hover_composite.wesl");
    if reflection_cube {
        reflection_cube::install(&mut app, dump_reflection_cube);
    }
    bevy::asset::embedded_asset!(app, "underside.wesl");
    bevy::asset::embedded_asset!(app, "water_texture.wesl");
    bevy::asset::embedded_asset!(app, "world_composition.wesl");
    bevy::asset::embedded_asset!(app, "water.wesl");
    if let Some(scene) = water_texture {
        app.insert_resource(scene)
            .insert_resource(Time::<Fixed>::from_hz(60.))
            .add_plugins(UiMaterialPlugin::<water_texture::Material>::default())
            .add_systems(Startup, water_texture::setup)
            .add_systems(FixedUpdate, water_texture::animate)
            .add_systems(Update, water_texture::exit);
    }
    if let Some(asset) = terrain {
        let is_validation = validation_camera.is_some();
        // The combined development/frontend app gets these from mission_environment.
        if !mecc_dev {
            app.add_plugins(MaterialPlugin::<world_scene::WorldColorMaterial>::default())
                .add_plugins(UiMaterialPlugin::<world_composition::Presentation>::default())
                .add_plugins(UiMaterialPlugin::<water_texture::Material>::default())
                .add_plugins(MaterialPlugin::<water_scene::Material>::default())
                .add_plugins(MaterialPlugin::<sky_scene::SkyMaterial>::default())
                .add_plugins(MaterialPlugin::<underside_scene::UndersideMaterial>::default())
                .add_systems(Startup, world_scene::setup);
        }
        app.insert_resource(Time::<Fixed>::from_hz(60.))
            .add_systems(
                FixedUpdate,
                water_scene::animate
                    .run_if(resource_exists::<world_scene::Scene>)
                    .run_if(move |world: &World| !mecc_dev || mecc_session::playing(world)),
            )
            .insert_resource(world_scene::Scene {
                asset: std::sync::Arc::new(asset),
                oblique,
                validation_camera,
                camera_source: if reflection_face.is_some() || reflection_cube {
                    "reflection_profile"
                } else if cinematic_camera {
                    "cinematic_file"
                } else if is_validation {
                    "oracle"
                } else {
                    "free"
                },
                camera_origin,
                cinematic_capture_ready: (!cinematic_camera || cinematic_tick.is_some())
                    && !reflection_cube,
                no_fog,
                render_terrain: matches!(
                    world_layer.as_deref().unwrap_or("all"),
                    "all" | "terrain"
                ),
                render_sky: matches!(world_layer.as_deref().unwrap_or("all"), "all" | "sky"),
                render_underside: matches!(
                    world_layer.as_deref().unwrap_or("all"),
                    "all" | "underside"
                ),
                reflection_profile: reflection_face.is_some() || reflection_cube,
                water: water_scene,
                render_water: matches!(world_layer.as_deref().unwrap_or("all"), "all" | "water"),
            });
        if mecc_dev {
            mecc_dev_scene::install(&mut app, &root)?;
            app.add_systems(
                Update,
                water_scene::update_geometry
                    .after(mecc_dev_scene::update)
                    .run_if(mecc_session::playing),
            );
        } else if is_validation {
            app.add_systems(
                Update,
                (
                    cinematic_camera::update,
                    viewport_scene::update,
                    world_scene::validation_exit,
                    water_scene::update_geometry,
                )
                    .chain(),
            );
        } else {
            app.add_systems(
                Update,
                (
                    mesh_scene::navigate,
                    world_scene::update_uv,
                    underside_scene::sync_camera,
                    water_scene::update_geometry,
                )
                    .chain(),
            );
        }
    }
    if let Some(cinematic) = cinematic {
        let cinematic_world =
            std::sync::Arc::clone(&app.world().resource::<world_scene::Scene>().asset);
        let glare_supported = cinematic.segments().iter().all(|segment| {
            (1..=7).any(|i| {
                segment
                    .sequence()
                    .eq_ignore_ascii_case(&format!("story4_t_square_{i}"))
            })
        });
        if !glare_supported {
            eprintln!("cinematic_preview_omission lens_visibility_beyond_measured_profile=true");
        }
        if world_layer.as_deref().unwrap_or("all") == "all" && glare_supported {
            let glare = sun_glare_scene::State::load(
                &root,
                &app.world().resource::<world_scene::Scene>().asset,
                &cinematic,
            )?;
            app.insert_resource(glare).add_systems(
                Update,
                sun_glare_scene::update
                    .after(cinematic_camera::update)
                    .run_if(reflection_cube::playback_ready),
            );
        }
        if sword_inspection || cinematic_cube {
            if cinematic_cube {
                reflection_cube::install_cinematic(
                    &mut app,
                    cinematic.realtime(),
                    matches!(world_layer.as_deref().unwrap_or("all"), "all" | "props"),
                );
            } else {
                reflection_cube::install_sword(&mut app);
            }
            let sword =
                sword_scene::Scene::load(&root, &cinematic, actor_slot, &cinematic_world.config)?;
            sword_scene::install(&mut app, sword);
        }
        if cinematic.realtime() && (!silent_capture || muted_transport) {
            if let Some(master) = cinematic_audio::Master::load(&root, cinematic.programs())? {
                app.insert_resource(master)
                    .add_systems(Startup, cinematic_audio::setup_master)
                    .add_systems(
                        Update,
                        cinematic_audio::sample_master.before(cinematic_camera::update),
                    )
                    .add_systems(
                        Update,
                        cinematic_audio::update_master
                            .after(cinematic_camera::update)
                            .run_if(reflection_cube::playback_ready),
                    );
            }
            let voices = cinematic_audio::Voices::load(&root, cinematic.programs())?;
            app.insert_resource(voices)
                .add_systems(Startup, cinematic_audio::setup)
                .add_systems(
                    Update,
                    cinematic_audio::update
                        .after(cinematic_camera::update)
                        .run_if(reflection_cube::playback_ready),
                );
        }
        if actor_slot.is_none()
            && matches!(
                world_layer.as_deref().unwrap_or("all"),
                "all" | "props" | "foliage"
            )
        {
            let instances = source_instances::Scene::load(
                &root,
                &app.world().resource::<world_scene::Scene>().asset,
                cinematic_mission.as_ref(),
            )?;
            let mission_classes: std::collections::BTreeSet<_> = instances
                .plan
                .entries
                .iter()
                .map(|p| p.serialized.object_type)
                .collect();
            let declared_classes: std::collections::BTreeSet<_> = mission_classes
                .iter()
                .copied()
                .chain(
                    instances
                        .plan
                        .world_entries
                        .iter()
                        .map(|p| p.declaration.object_type),
                )
                .collect();
            app.add_plugins(MaterialPlugin::<scenery_scene::Material>::default())
                .init_resource::<Assets<sword_scene::Material>>();
            app.insert_resource(instances)
                .add_systems(Startup, source_instances::spawn_children)
                .add_systems(
                    Update,
                    (
                        source_instances::step_children,
                        source_instances::report_children,
                    )
                        .chain(),
                );
            if scenery_class != Some(1053) {
                app.add_plugins(MaterialPlugin::<prop_scene::Material>::default());
                if matches!(world_layer.as_deref().unwrap_or("all"), "all" | "foliage") {
                    if world_layer.as_deref().unwrap_or("all") == "all" {
                        let decorations = decoration_scene::Scene::load(
                            &root,
                            &app.world().resource::<source_instances::Scene>().plan,
                        )?;
                        app.insert_resource(decorations).add_systems(
                            Startup,
                            decoration_scene::setup
                                .after(world_scene::setup)
                                .run_if(reflection_cube::playback_ready),
                        );
                    }
                    if declared_classes.contains(&102) || declared_classes.contains(&1045) {
                        let foliage = foliage_scene::Scene::load(
                            &root,
                            &app.world().resource::<world_scene::Scene>().asset,
                            &app.world().resource::<source_instances::Scene>().plan,
                        )?;
                        app.insert_resource(foliage)
                            .add_systems(
                                Startup,
                                foliage_scene::setup
                                    .after(world_scene::setup)
                                    .run_if(reflection_cube::playback_ready),
                            )
                            .add_systems(
                                Update,
                                foliage_scene::update
                                    .after(cinematic_camera::update)
                                    .run_if(reflection_cube::playback_ready),
                            );
                    }
                }
                if (mission_classes.contains(&53) || scenery_class == Some(53))
                    && world_layer.as_deref() != Some("foliage")
                {
                    let props = prop_scene::Scene::load(
                        &app.world().resource::<world_scene::Scene>().asset,
                        &app.world().resource::<source_instances::Scene>().plan,
                    )?;
                    app.insert_resource(props).add_systems(
                        Startup,
                        prop_scene::setup
                            .after(world_scene::setup)
                            .run_if(reflection_cube::playback_ready),
                    );
                }
            }
            if (world_layer.as_deref().unwrap_or("all") == "all" && mission_classes.contains(&1053))
                || scenery_class == Some(1053)
            {
                let scenery = scenery_scene::Scene::load(
                    &root,
                    &app.world().resource::<source_instances::Scene>().plan,
                    &cinematic_world.config,
                )?;
                app.insert_resource(scenery)
                    .add_systems(
                        Startup,
                        scenery_scene::setup
                            .after(world_scene::setup)
                            .run_if(reflection_cube::playback_ready),
                    )
                    .add_systems(
                        Update,
                        scenery_scene::update.after(cinematic_camera::update),
                    );
            }
        }
        if actor_slot.is_none()
            && world_layer.as_deref().unwrap_or("all") == "all"
            && scenery_class.is_none()
        {
            let mut covered = std::collections::BTreeSet::new();
            if let Some(scene) = app.world().get_resource::<prop_scene::Scene>() {
                covered.extend(scene.source_keys());
            }
            if let Some(scene) = app.world().get_resource::<scenery_scene::Scene>() {
                covered.extend(scene.source_keys());
            }
            if let Some(scene) = app.world().get_resource::<foliage_scene::Scene>() {
                covered.extend(scene.source_keys());
            }
            if let Some(scene) = app.world().get_resource::<decoration_scene::Scene>() {
                covered.extend(scene.source_keys());
            }
            let scene = static_scene::Scene::load(
                &root,
                &app.world().resource::<source_instances::Scene>().plan,
                &covered,
                &cinematic_world,
            )?;
            if scene.needs_cube() {
                if !app.world().contains_resource::<reflection_cube::Build>() {
                    reflection_cube::install_cinematic(&mut app, cinematic.realtime(), true);
                }
                sword_scene::install_materials(&mut app);
            }
            app.insert_resource(scene)
                .add_systems(
                    Update,
                    static_scene::setup
                        .after(cinematic_camera::update)
                        .before(static_scene::update)
                        .run_if(static_scene::needs_setup)
                        .run_if(reflection_cube::playback_ready),
                )
                .add_systems(
                    Update,
                    static_scene::update
                        .after(cinematic_camera::update)
                        .run_if(reflection_cube::playback_ready),
                );
        }
        if matches!(world_layer.as_deref().unwrap_or("all"), "all" | "actors") {
            let spears =
                spear_scene::Scene::load(&root, &cinematic, actor_slot, &cinematic_world.config)?;
            app.add_plugins(MaterialPlugin::<spear_scene::Material>::default())
                .insert_resource(spears)
                .add_systems(
                    Startup,
                    spear_scene::setup
                        .after(world_scene::setup)
                        .run_if(reflection_cube::playback_ready),
                )
                .add_systems(
                    Update,
                    spear_scene::switch
                        .after(cinematic_camera::update)
                        .before(spear_scene::update),
                )
                .add_systems(Update, spear_scene::update.after(cinematic_camera::update));
            let gun =
                gun_scene::Scene::load(&root, &cinematic, actor_slot, &cinematic_world.config)?;
            app.add_plugins(MaterialPlugin::<gun_scene::Material>::default())
                .insert_resource(gun)
                .add_systems(
                    Startup,
                    gun_scene::setup
                        .after(world_scene::setup)
                        .run_if(reflection_cube::playback_ready),
                )
                .add_systems(
                    Update,
                    gun_scene::switch
                        .after(cinematic_camera::update)
                        .before(gun_scene::update),
                )
                .add_systems(Update, gun_scene::update.after(cinematic_camera::update));
            let staff = attachment_scene::Scene::load(&root, &cinematic, actor_slot)?;
            app.add_plugins(MaterialPlugin::<attachment_scene::Material>::default())
                .insert_resource(staff)
                .add_systems(
                    Startup,
                    attachment_scene::setup
                        .after(world_scene::setup)
                        .run_if(reflection_cube::playback_ready),
                )
                .add_systems(
                    Update,
                    attachment_scene::switch
                        .after(cinematic_camera::update)
                        .before(attachment_scene::update),
                )
                .add_systems(
                    Update,
                    attachment_scene::update.after(cinematic_camera::update),
                );
            let actor_resources =
                if let Some(instances) = app.world().get_resource::<source_instances::Scene>() {
                    instances.plan.resources().clone()
                } else {
                    std::sync::Arc::new(giants_game::scene_resources::SceneResources::load(
                        &root,
                        &app.world().resource::<world_scene::Scene>().asset.catalog,
                        "English",
                    )?)
                };
            let actors = actor_scene::Scene::load(
                &root,
                &cinematic,
                actor_slot,
                &actor_resources,
                &cinematic_world.config,
            )?;
            if !app.world().contains_resource::<sword_scene::Scene>() {
                match sword_scene::Scene::load(
                    &root,
                    &cinematic,
                    actor_slot,
                    &cinematic_world.config,
                ) {
                    Ok(sword) if sword.has_geometry() => {
                        if !app.world().contains_resource::<reflection_cube::Build>() {
                            reflection_cube::prepare_cinematic_water(&mut app, &root)?;
                            let props = app.world().contains_resource::<prop_scene::Scene>();
                            reflection_cube::install_cinematic(
                                &mut app,
                                cinematic.realtime(),
                                props,
                            );
                        }
                        sword_scene::install(&mut app, sword);
                    }
                    Ok(_) => {}
                    Err(error) => eprintln!(
                        "cinematic_specialized_consumer_deferred consumer=sword reason={error:?}"
                    ),
                }
            }
            let owners = cinematic_prepare::owners(
                &cinematic,
                app.world().resource::<gun_scene::Scene>(),
                app.world().resource::<attachment_scene::Scene>(),
                app.world().resource::<spear_scene::Scene>(),
                app.world().get_resource::<sword_scene::Scene>(),
            )?;
            let rigid = rigid_attachment_scene::Scene::load(
                &root,
                &cinematic,
                actor_slot,
                &actor_resources,
                &actors,
                &owners,
                &cinematic_world.config,
            )?;
            let attachment_cube = rigid.needs_cube();
            app.add_plugins(MaterialPlugin::<rigid_attachment_scene::Material>::default())
                .insert_resource(rigid)
                .add_systems(
                    Startup,
                    rigid_attachment_scene::setup
                        .after(world_scene::setup)
                        .run_if(reflection_cube::playback_ready),
                )
                .add_systems(
                    Update,
                    rigid_attachment_scene::switch
                        .after(cinematic_camera::update)
                        .before(rigid_attachment_scene::update),
                )
                .add_systems(
                    Update,
                    rigid_attachment_scene::update.after(cinematic_camera::update),
                );
            if actors.needs_cube() || attachment_cube {
                if !app.world().contains_resource::<reflection_cube::Build>() {
                    reflection_cube::prepare_cinematic_water(&mut app, &root)?;
                    let props = app.world().contains_resource::<prop_scene::Scene>();
                    reflection_cube::install_cinematic(&mut app, cinematic.realtime(), props);
                }
                sword_scene::install_materials(&mut app);
            }
            app.add_plugins(MaterialPlugin::<actor_scene::Material>::default())
                .add_plugins(MaterialPlugin::<actor_scene::LitMaterial>::default())
                .add_plugins(MaterialPlugin::<body_context::Material>::default())
                .insert_resource(actors)
                .add_systems(
                    Startup,
                    actor_scene::setup
                        .after(world_scene::setup)
                        .run_if(reflection_cube::playback_ready),
                )
                .add_systems(
                    Update,
                    actor_scene::switch
                        .after(cinematic_camera::update)
                        .before(actor_scene::update),
                )
                .add_systems(Update, actor_scene::update.after(cinematic_camera::update));
        }
        app.insert_resource(cinematic).add_systems(
            PostUpdate,
            cinematic_visibility::update
                .before(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate),
        );
    }
    if let Some(asset) = logo {
        app.add_plugins(MaterialPlugin::<logo_material::LogoMaterial>::default())
            .insert_resource(mesh_scene::Scene {
                asset,
                oblique,
                textures,
            })
            .add_systems(Startup, mesh_scene::setup)
            .add_systems(
                Update,
                (mesh_scene::navigate, logo_material::update_camera).chain(),
            );
    }
    if frontend.is_some() {
        frontend_scene::install_mission_replay(&mut app)?;
    }
    if let Some(scene) = frontend {
        app.add_message::<frontend_scene::MissionPrepared>()
            .add_plugins(mission_runtime::Plugin)
            .add_plugins(mission_environment::Plugin)
            .add_plugins(mission_opening::Plugin)
            .add_plugins(mission_player::Plugin)
            .add_plugins(mission_jet_scene::Plugin)
            .add_plugins(mission_carry::Plugin)
            .add_plugins(mission_boot::Plugin)
            .add_plugins(mission_showcase::Plugin)
            .add_plugins(mission_dressing::Plugin)
            .add_plugins(mission_weapon::Plugin)
            .add_plugins(blood_scene::Plugin)
            .add_plugins(mission_messages_hud::Plugin)
            .add_plugins(mission_audio::Plugin)
            .add_plugins(mission_ambient::Plugin)
            .add_plugins(mission_flight_audio::Plugin)
            .add_plugins(frontend_background::Plugin)
            .add_plugins(frontend_feedback::Plugin)
            .add_plugins(frontend_music::Plugin)
            .add_plugins(UiMaterialPlugin::<LoadingMaterial>::default())
            .add_message::<frontend_scene::MissionRuntimeEvent>()
            .add_message::<frontend_scene::MissionCancelled>()
            .insert_resource(scene)
            .add_systems(Startup, frontend_scene::setup)
            .add_systems(
                Update,
                (
                    frontend_scene::display_inventory,
                    frontend_scene::input,
                    frontend_scene::draw,
                    frontend_scene::advance_mission,
                    frontend_scene::runtime_events,
                )
                    .chain()
                    .run_if(mecc_session::frontend_available),
            );
    }
    if mecc_dev {
        mecc_session::install(&mut app, root.clone(), mecc_menu);
    }
    if let Some(asset) = loading {
        app.insert_resource(LoadingPixels(asset.image.pixels))
            .add_plugins(UiMaterialPlugin::<LoadingMaterial>::default())
            .add_systems(Startup, setup);
    }
    if let Some(path) = owned_effect_input {
        let world = app
            .world()
            .resource::<world_scene::Scene>()
            .asset
            .catalog
            .clone();
        let state = owned_effect_scene::State::load(&root, &world, &path)?;
        owned_effect_scene::install(&mut app, state);
    }
    app.init_resource::<source_view::SourceView>().add_systems(
        Update,
        source_view::update
            .after(cinematic_camera::update)
            .in_set(source_view::ViewUpdate),
    );
    let result = app.run();
    if result.is_error() {
        return Err(format!("Bevy runtime failed: {result:?}").into());
    }
    if let Some(path) = requested_capture
        && !path.is_file()
    {
        return Err("Requested capture did not complete".into());
    }
    Ok(())
}
fn cpu_mesh_phase_policy(
    mut support: ResMut<bevy::render::batching::gpu_preprocessing::GpuPreprocessingSupport>,
) {
    support.max_supported_mode =
        bevy::render::batching::gpu_preprocessing::GpuPreprocessingMode::None;
}

fn setup(
    mut commands: Commands,
    pixels: Res<LoadingPixels>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<LoadingMaterial>>,
) {
    commands.spawn(Camera2d);
    for tile in giants_game::loading_tiles() {
        let [source_x, source_y, width, height] = tile.source;
        let mut rgba = vec![0; 128 * 128 * 4];
        for y in 0..height as usize {
            for x in 0..width as usize {
                let from = ((source_y as usize + y) * 640 + source_x as usize + x) * 3;
                let to = (y * 128 + x) * 4;
                rgba[to..to + 3].copy_from_slice(&pixels.0[from..from + 3]);
                rgba[to + 3] = 255;
            }
        }
        // Native padding is black; retain opaque alpha so the final partial tile is sampled consistently.
        for alpha in rgba.iter_mut().skip(3).step_by(4) {
            *alpha = 255;
        }
        let mut image = Image::new(
            Extent3d {
                width: 128,
                height: 128,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            rgba,
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::default(),
        );
        image.sampler = ImageSampler::linear();
        let handle = images.add(image);
        let [left, top, screen_width, screen_height] = tile.screen;
        commands.spawn((
            MaterialNode(materials.add(LoadingMaterial {
                image: handle,
                uv_scale: Vec4::new(width as f32 / 128.0, height as f32 / 128.0, 0., 0.),
            })),
            LayoutConfig {
                use_rounding: false,
            },
            Node {
                position_type: PositionType::Absolute,
                left: px(left + 0.5),
                top: px(top + 0.5),
                width: px(screen_width),
                height: px(screen_height),
                ..default()
            },
        ));
    }
}
#[derive(bevy::ecs::system::SystemParam)]
struct CaptureExtras<'w, 's> {
    desired: Res<'w, capture_readiness::Desired>,
    prepared: Res<'w, capture_readiness::Prepared>,
    offscreen: Option<Res<'w, offscreen::Output>>,
    attachments: Option<Res<'w, rigid_attachment_scene::Scene>>,
    cinematic: Option<Res<'w, cinematic_camera::Cinematic>>,
    blood: Option<Res<'w, blood_scene::State>>,
    actor_meshes: Query<
        'w,
        's,
        (
            Entity,
            &'static cinematic_visibility::ActorSlot,
            &'static Mesh3d,
            &'static Visibility,
        ),
    >,
    all_meshes: Query<'w, 's, (&'static Mesh3d, &'static Visibility)>,
    meshes: Option<Res<'w, Assets<Mesh>>>,
    rigid_materials: Option<Res<'w, Assets<rigid_attachment_scene::Material>>>,
    fx_isolation: Option<Res<'w, rigid_attachment_scene::capture::EffectIsolation>>,
}
fn capture_frame(
    mut commands: Commands,
    mut capture: ResMut<Capture>,
    time: Res<Time>,
    pipelines: Res<PipelinesReady>,
    cameras: Query<(&Transform, &Projection), With<mesh_scene::FlyCamera>>,
    scene: Option<Res<world_scene::Scene>>,
    extras: CaptureExtras,
) {
    capture.frame += 1;
    if extras
        .blood
        .as_ref()
        .is_some_and(|blood| blood.capture_pending())
    {
        return;
    }
    capture.ready_frames = if pipelines.0 && extras.prepared.ready_for(&extras.desired) {
        capture.ready_frames + 1
    } else {
        0
    };
    let effect_capture = capture.timed_effect
        || capture.projectile_effect
        || capture.periodic_effect
        || capture.burst_effect
        || capture.released_effect
        || capture.particle_effect
        || capture.attachment_loop;
    // A source-clock observation must also work for directors with no FX.
    // The threshold is a minimum: record the actual clock once pipelines are
    // ready rather than claim that asynchronous readback is source-frame exact.
    let time_capture =
        (capture.after_seconds.is_some() || capture.effect_activation) && !effect_capture;
    let active_effect = extras.attachments.as_ref().is_some_and(|s| {
        if capture.attachment_loop {
            extras
                .cinematic
                .as_ref()
                .is_some_and(|c| s.attachment_loop_capture_ready(c))
        } else if capture.periodic_effect {
            s.periodic_effect_capture_ready()
        } else if capture.burst_effect {
            s.burst_effect_capture_ready()
        } else if capture.projectile_effect {
            s.projectile_effect_capture_ready()
        } else if capture.particle_effect {
            s.source_particle_capture_ready()
        } else if capture.released_effect {
            s.released_effect_capture_ready()
        } else {
            s.has_visible_timed_effect()
        }
    }) && extras
        .cinematic
        .as_ref()
        .is_some_and(|c| c.animation_dt() > 0.);
    capture.timed_frames = if active_effect {
        capture.timed_frames + 1
    } else {
        0
    };
    if capture.frame >= 30
        && (!capture.effect_activation || extras.cinematic.as_ref().is_some_and(|c| c.paused()))
        && capture.after_seconds.is_none_or(|s| {
            extras.cinematic.as_ref().is_some_and(|c| {
                let seconds = if time_capture {
                    c.director_seconds()
                } else {
                    c.world_seconds()
                };
                seconds >= s
                    && if capture.hold_cinematic {
                        c.paused()
                    } else {
                        c.animation_dt() > 0. || c.end_reached()
                    }
            })
        })
        && capture.ready_frames >= 3
        && (effect_capture || time_capture || time.elapsed_secs() >= 2.0)
        && if effect_capture {
            capture.timed_frames >= 2
        } else {
            time_capture || scene.as_ref().is_none_or(|s| s.cinematic_capture_ready)
        }
        && let Some(path) = capture.path.take()
    {
        if let Some(cinematic) = extras.cinematic.as_ref() {
            if let Some(attachments) = extras.attachments.as_ref() {
                if let Some(meshes) = extras.meshes.as_ref() {
                    attachments.capture_vertical_buffers(cinematic, meshes);
                }
                let ready = extras.prepared.ready_for(&extras.desired);
                attachments.capture_effect_entities(cinematic, |entity, expected_mesh| {
                    extras
                        .all_meshes
                        .get(entity)
                        .ok()
                        .filter(|(mesh, _)| &mesh.0 == expected_mesh)
                        .map(|(_, visibility)| {
                            (
                                *visibility != Visibility::Hidden,
                                ready && extras.desired.contains_mesh_entity(entity),
                            )
                        })
                });
                if let (Some(selected), Some(meshes), Some(materials), Some(camera)) = (
                    extras.fx_isolation.as_ref(),
                    extras.meshes.as_ref(),
                    extras.rigid_materials.as_ref(),
                    scene.as_ref().and_then(|s| s.validation_camera.as_ref()),
                ) {
                    attachments.capture_effect_projection(
                        cinematic,
                        camera,
                        meshes,
                        materials,
                        selected.offset,
                    );
                }
            }
            let declared = cinematic_visibility::declared_slots(cinematic.program());
            let ready = extras.prepared.ready_for(&extras.desired);
            let rows = cinematic_visibility::capture_counts(
                declared,
                extras
                    .actor_meshes
                    .iter()
                    .map(|(entity, slot, _, visibility)| {
                        (
                            slot.0,
                            *visibility != Visibility::Hidden,
                            ready && extras.desired.contains_mesh_entity(entity),
                        )
                    }),
            );
            for (slot, row) in rows {
                println!(
                    "cinematic_actor_entity_snapshot sequence={} director={} tick={} actor_slot={} mesh_entities={} local_render_enabled={} gpu_prepared_entities={} declaration_reached={} tagged_ownership_only=true pixels_verified=false",
                    cinematic.sequence(),
                    cinematic.director_index(),
                    cinematic.tick(),
                    slot,
                    row.mesh_entities,
                    row.local_render_enabled,
                    row.gpu_prepared_entities,
                    cinematic.actor_requested(slot) == Some(true)
                );
            }
        }
        if time_capture && let Some(cinematic) = extras.cinematic.as_ref() {
            println!(
                "cinematic_time_capture sequence={} tick={} seconds={} minimum_seconds={} exact_image_clock_verified=false",
                cinematic.sequence(),
                cinematic.tick(),
                cinematic.director_seconds(),
                capture.after_seconds.unwrap_or(0.)
            );
        }
        if capture.periodic_effect
            && let Some(cinematic) = extras.cinematic.as_ref()
        {
            println!(
                "cinematic_periodic_effect_capture sequence={} tick={} native_raster_verified=false",
                cinematic.sequence(),
                cinematic.tick()
            );
        }
        if capture.burst_effect
            && let Some(cinematic) = extras.cinematic.as_ref()
        {
            println!(
                "cinematic_burst_effect_capture sequence={} tick={} motion_profile=contact_free native_raster_verified=false",
                cinematic.sequence(),
                cinematic.tick()
            );
        }
        if capture.projectile_effect
            && let Some(cinematic) = extras.cinematic.as_ref()
        {
            println!(
                "cinematic_projectile_effect_capture sequence={} tick={} motion_profile=contact_free native_raster_verified=false",
                cinematic.sequence(),
                cinematic.tick()
            );
        }
        if capture.attachment_loop
            && let Some(cinematic) = extras.cinematic.as_ref()
        {
            println!(
                "cinematic_attachment_loop_capture sequence={} tick={} native_pixels_verified=false",
                cinematic.sequence(),
                cinematic.tick()
            );
        }
        if capture.particle_effect
            && let Some(cinematic) = extras.cinematic.as_ref()
        {
            println!(
                "cinematic_particle_effect_capture sequence={} tick={} native_raster_verified=false",
                cinematic.sequence(),
                cinematic.tick()
            );
        }
        if (capture.timed_effect || capture.released_effect)
            && let Some(cinematic) = extras.cinematic.as_ref()
        {
            println!(
                "cinematic_timed_effect_capture sequence={} tick={} released={} native_raster_verified=false",
                cinematic.sequence(),
                cinematic.tick(),
                capture.released_effect
            );
        }
        if let Some(world_scene) = scene.as_ref()
            && let Some(camera) = world_scene.validation_camera.as_ref()
        {
            println!(
                "render_control={{\"oracle_camera\":{},\"camera_source\":\"{}\",\"view\":{:?},\"projection\":{:?},\"reversed_depth_adapter\":true}}",
                world_scene.camera_source == "oracle",
                world_scene.camera_source,
                camera.view,
                camera.projection
            );
        } else if let Ok((transform, projection)) = cameras.single() {
            println!(
                "render_control={{\"view\":{:?},\"projection\":{:?},\"camera\":{:?}}}",
                transform.to_matrix().inverse().to_cols_array(),
                projection.get_clip_from_view().to_cols_array(),
                transform.translation.to_array()
            );
        }
        println!(
            "capture_ready frame={} elapsed_seconds={} pipelines_ready_frames={}",
            capture.frame,
            time.elapsed_secs(),
            capture.ready_frames
        );
        commands
            .spawn(
                extras
                    .offscreen
                    .map_or_else(Screenshot::primary_window, |output| {
                        Screenshot::image(output.0.clone())
                    }),
            )
            .observe(save_to_disk(path))
            .observe(
                |_: On<ScreenshotCaptured>, mut exit: MessageWriter<AppExit>| {
                    exit.write(AppExit::Success);
                },
            );
    }
}

fn inspection_font_path() -> std::path::PathBuf {
    std::env::var_os("WINDIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("C:/Windows"))
        .join("Fonts/arial.ttf")
}
