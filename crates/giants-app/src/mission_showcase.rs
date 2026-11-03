//! Presentation fallback, deliberately separate from campaign acknowledgements.
//! An unsupported character can show its movie/world without becoming playable.
use bevy::{input::mouse::AccumulatedMouseMotion, prelude::*};
use giants_game::mission_loading::Phase;

pub(crate) struct Plugin;
impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<State>().add_systems(
            Update,
            update
                .after(crate::mission_boot::update)
                .before(crate::source_view::ViewUpdate)
                .run_if(crate::mecc_session::frontend_available),
        );
    }
}
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
enum Mode {
    #[default]
    Loading,
    Opening,
    Mecc,
    Free,
    Unavailable,
}
#[derive(Resource)]
pub(crate) struct State {
    enabled: bool,
    ticket: u64,
    mode: Mode,
    camera: Option<FreeCamera>,
    overlay: Option<(Entity, Entity)>,
    hidden: bool,
    failure: Option<String>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            enabled: std::env::var("GIANTS_SHOWCASE").ok().as_deref() != Some("0")
                && std::env::var("GIANTS_MISSION_REPLAY")
                    .ok()
                    .is_none_or(|s| s.starts_with("showcase")),
            ticket: 0,
            mode: Mode::Loading,
            camera: None,
            overlay: None,
            hidden: false,
            failure: None,
        }
    }
}
impl State {
    pub(crate) fn is_mecc_preview(&self) -> bool {
        self.enabled && self.mode == Mode::Mecc
    }
    pub(crate) fn controls(&self, phase: Phase) -> bool {
        phase == Phase::Gameplay && !self.is_free()
            || phase == Phase::AwaitingRuntime && self.is_mecc_preview()
    }
    pub(crate) fn unavailable(&self) -> bool {
        self.enabled && self.mode == Mode::Unavailable
    }
    pub(crate) fn playback_failed(&mut self, reason: &str) -> bool {
        if !self.enabled {
            return false;
        }
        self.failure = Some(reason.to_owned());
        true
    }
    pub(crate) fn enabled(&self) -> bool {
        self.enabled
    }
    pub(crate) fn is_free(&self) -> bool {
        self.enabled && self.mode == Mode::Free
    }
    pub(crate) fn presenting(&self) -> bool {
        self.enabled && self.mode != Mode::Loading
    }
}
pub(crate) fn free(world: &World) -> bool {
    world.get_resource::<State>().is_some_and(State::is_free)
}
pub(crate) fn mecc_preview(world: &World) -> bool {
    world
        .get_resource::<State>()
        .is_some_and(State::is_mecc_preview)
}
fn preview_opening(world: &World) -> bool {
    world
        .get_resource::<State>()
        .is_some_and(|s| s.enabled && s.mode == Mode::Opening)
}
pub(crate) fn partial_opening(world: &World) -> bool {
    preview_opening(world)
        && world
            .get_resource::<crate::mission_opening::State>()
            .is_some_and(|opening| opening.render_failed(world.resource::<State>().ticket))
}

#[derive(Clone)]
struct FreeCamera {
    eye: Vec3,
    yaw: f32,
    pitch: f32,
    reset: (Vec3, f32, f32),
}
impl FreeCamera {
    fn from_view(view: [f32; 16]) -> Option<Self> {
        let inverse = Mat4::from_cols_array(&view).inverse();
        if !inverse.is_finite() {
            return None;
        }
        let eye = inverse.transform_point3(Vec3::ZERO);
        let forward = inverse.transform_vector3(Vec3::Z).normalize_or_zero();
        if forward.length_squared() < 0.5 || !eye.is_finite() {
            return None;
        }
        let yaw = forward.x.atan2(-forward.y);
        let pitch = forward.z.clamp(-1., 1.).asin();
        Some(Self {
            eye,
            yaw,
            pitch,
            reset: (eye, yaw, pitch),
        })
    }
    fn step(&mut self, axis: Vec3, look: Vec2, dt: f32, fast: bool) {
        self.yaw -= look.x * 0.003;
        self.pitch = (self.pitch - look.y * 0.003).clamp(-1.5, 1.5);
        let (forward, right) = self.axes();
        self.eye += (right * axis.x + forward * axis.y + Vec3::Z * axis.z).normalize_or_zero()
            * if fast { 1000. } else { 250. }
            * dt.clamp(0., 0.1);
    }
    fn axes(&self) -> (Vec3, Vec3) {
        let forward = Vec3::new(
            self.pitch.cos() * self.yaw.sin(),
            -self.pitch.cos() * self.yaw.cos(),
            self.pitch.sin(),
        );
        (forward, forward.cross(Vec3::Z).normalize())
    }
    fn view(&self) -> [f32; 16] {
        let (forward, _) = self.axes();
        crate::mecc_dev_scene::camera_view(self.eye, forward, Vec3::Z).to_cols_array()
    }
}
fn enter_free(world: &mut World, ticket: u64, reason: &str) {
    let camera = world
        .get_resource::<crate::world_scene::Scene>()
        .and_then(|s| s.validation_camera.as_ref())
        .and_then(|c| FreeCamera::from_view(c.view));
    let Some(camera) = camera else {
        world.resource_mut::<State>().mode = Mode::Unavailable;
        println!(
            "showcase_unavailable ticket={ticket} reason={reason:?} camera=false readiness_waived=false"
        );
        return;
    };
    let eye = camera.eye.to_array();
    {
        let mut state = world.resource_mut::<State>();
        state.mode = Mode::Free;
        state.camera = Some(camera);
    }
    // Cancelling a preview does not synthesize CinematicFinished or Ready.
    crate::mission_opening::stop_preview(world);
    world
        .resource_mut::<crate::mission_environment::State>()
        .inspect(ticket, eye);
    println!(
        "showcase_free ticket={ticket} reason={reason:?} gameplay_ready=false actor_services_simulated=false"
    );
}
fn finish_preview(world: &mut World, ticket: u64, reason: &str) {
    let carried_resident = crate::mission_carry::resident(world, ticket);
    if world
        .resource::<crate::mission_runtime::Runtime>()
        .mecc_preview_candidate(carried_resident)
        && crate::mission_player::resident(world, ticket)
    {
        world.resource_mut::<State>().mode = Mode::Mecc;
        crate::mission_opening::stop_preview(world);
        // The campaign flow stays AwaitingRuntime; keep preview world rendering
        // active while the real controller supplies subsequent camera positions.
        if let Some(eye) = world.resource::<crate::world_scene::Scene>().camera_origin {
            world
                .resource_mut::<crate::mission_environment::State>()
                .inspect(ticket, eye);
        }
        println!(
            "showcase_mecc_control ticket={ticket} reason={reason:?} resident_controller=true campaign_ready=false mission_rules_complete=false"
        );
    } else {
        enter_free(world, ticket, reason);
    }
}
pub(crate) fn skip_opening(world: &mut World, ticket: u64) {
    if world.resource::<State>().ticket == ticket && preview_opening(world) {
        finish_preview(world, ticket, "opening skipped by user");
        println!("showcase_opening_skipped ticket={ticket} source_end=false gameplay_ready=false");
    }
}
pub(crate) fn update(world: &mut World) {
    if !world.resource::<State>().enabled {
        return;
    }
    let frontend = world.resource::<crate::frontend_scene::Scene>();
    let (ticket, phase) = (frontend.flow.ticket, frontend.flow.phase);
    if world.resource::<State>().ticket != ticket {
        let mut state = world.resource_mut::<State>();
        state.ticket = ticket;
        state.mode = Mode::Loading;
        state.camera = None;
        state.failure = None;
    }
    let keys = world.resource::<ButtonInput<KeyCode>>();
    let (inspect, hide) = (
        keys.just_pressed(KeyCode::F6),
        keys.just_pressed(KeyCode::F8),
    );
    if hide {
        let mut state = world.resource_mut::<State>();
        state.hidden = !state.hidden;
    }
    let failure = world.resource_mut::<State>().failure.take();
    if let Some(reason) = failure {
        finish_preview(world, ticket, &reason);
    }
    if phase == Phase::Failed {
        world.resource_mut::<State>().mode = Mode::Unavailable;
    } else if phase == Phase::AwaitingRuntime && !free(world) && !mecc_preview(world) {
        let runtime = world.resource::<crate::mission_runtime::Runtime>();
        let staging_failed = runtime.failed_ticket() == Some(ticket);
        let needs = world
            .resource::<crate::frontend_scene::Scene>()
            .prepared_mission
            .as_ref()
            .is_some_and(|p| !p.needs.is_empty());
        if staging_failed
            || needs
            || world
                .resource::<crate::mission_environment::State>()
                .failed_ticket()
                == Some(ticket)
        {
            world.resource_mut::<State>().mode = Mode::Unavailable;
        } else if runtime.active_ticket() == Some(ticket) {
            let reason = runtime.startup().err();
            let render_failed = world
                .resource::<crate::mission_opening::State>()
                .render_failed(ticket);
            if reason.is_some() || render_failed {
                if world.resource::<State>().mode == Mode::Opening {
                    let finished = world
                        .get_resource::<crate::cinematic_camera::Cinematic>()
                        .is_some_and(|c| c.end_reached());
                    let voices = world.query_filtered::<Option<&AudioSink>, With<crate::cinematic_audio::PlayingVoice>>()
                        .iter(world).all(|s| s.is_some_and(|s| s.empty()));
                    if finished && voices {
                        finish_preview(world, ticket, "preview opening completed");
                    }
                } else if world
                    .resource_mut::<crate::mission_opening::State>()
                    .inspect(ticket)
                {
                    world.resource_mut::<State>().mode = Mode::Opening;
                    println!(
                        "showcase_opening ticket={ticket} backend_refusal={reason:?} partial_rendering=true readiness_waived=false"
                    );
                } else if world
                    .resource::<crate::mission_opening::State>()
                    .attempted(ticket)
                {
                    finish_preview(world, ticket, "character or opening backend unavailable");
                }
            }
        }
    }
    if phase == Phase::Objectives
        && !free(world)
        && world
            .resource_mut::<crate::frontend_scene::Scene>()
            .showcase_dismiss_objectives()
    {
        world
            .resource_mut::<ButtonInput<MouseButton>>()
            .reset(MouseButton::Left);
        println!("showcase_objectives_dismissed ticket={ticket} controller_ack=real");
    }
    if inspect
        && world
            .resource::<crate::mission_environment::State>()
            .mounted_ticket()
            == Some(ticket)
    {
        if free(world) && phase == Phase::AwaitingRuntime {
            finish_preview(world, ticket, "debug controller requested");
        } else if free(world) && phase == Phase::Gameplay {
            world.resource_mut::<State>().mode = Mode::Loading;
            world
                .resource_mut::<crate::mission_environment::State>()
                .end_inspection(ticket);
            println!("showcase_resume_controller ticket={ticket}");
        } else if !free(world) {
            enter_free(world, ticket, "debug free camera requested");
        }
    }
    if free(world) {
        navigate(world, ticket);
    }
    overlay(world);
}
fn navigate(world: &mut World, ticket: u64) {
    let focused = world.query::<&Window>().iter(world).all(|w| w.focused);
    let keys = world.resource::<ButtonInput<KeyCode>>();
    let active = |k| focused && (keys.pressed(k) || keys.just_pressed(k));
    let axis = Vec3::new(
        (active(KeyCode::KeyD) || active(KeyCode::ArrowRight)) as u8 as f32
            - (active(KeyCode::KeyA) || active(KeyCode::ArrowLeft)) as u8 as f32,
        (active(KeyCode::KeyW) || active(KeyCode::ArrowUp)) as u8 as f32
            - (active(KeyCode::KeyS) || active(KeyCode::ArrowDown)) as u8 as f32,
        active(KeyCode::Space) as u8 as f32 - active(KeyCode::ControlLeft) as u8 as f32,
    );
    let fast = active(KeyCode::ShiftLeft);
    let reset = focused && keys.just_pressed(KeyCode::KeyR);
    let look = if focused
        && world
            .resource::<ButtonInput<MouseButton>>()
            .pressed(MouseButton::Right)
    {
        world.resource::<AccumulatedMouseMotion>().delta
    } else {
        Vec2::ZERO
    };
    let dt = world.resource::<Time>().delta_secs();
    let (view, eye) = {
        let mut state = world.resource_mut::<State>();
        let Some(camera) = &mut state.camera else {
            return;
        };
        if reset {
            (camera.eye, camera.yaw, camera.pitch) = camera.reset;
        }
        camera.step(axis, look, dt, fast);
        (camera.view(), camera.eye.to_array())
    };
    let mut scene = world.resource_mut::<crate::world_scene::Scene>();
    scene.validation_camera = Some(crate::validation_camera::ValidationCamera {
        view,
        projection: giants_game::mecc_camera::projection(1920. / 1080., 100000.)
            .expect("debug camera lens"),
        viewport: UVec4::new(0, 0, 1920, 1080),
    });
    scene.camera_origin = Some(eye);
    scene.camera_source = "showcase_free";
    world
        .resource_mut::<crate::mission_environment::State>()
        .inspect(ticket, eye);
    world
        .run_system_cached(apply_camera)
        .expect("showcase camera materials");
}
fn apply_camera(
    scene: Res<crate::world_scene::Scene>,
    mut materials: crate::world_scene::SceneMaterials,
) {
    crate::world_scene::apply_camera(&scene, &mut materials);
}
fn overlay(world: &mut World) {
    if world.resource::<State>().overlay.is_none() {
        let Some(font) = world
            .get_resource::<crate::frontend_scene::InspectionFont>()
            .map(|f| f.0.clone())
        else {
            return;
        };
        let camera = world
            .spawn((
                Camera2d,
                Camera {
                    order: 110,
                    clear_color: ClearColorConfig::None,
                    ..default()
                },
                Msaa::Off,
            ))
            .id();
        let label = world
            .spawn((
                UiTargetCamera(camera),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(12.),
                    top: px(10.),
                    max_width: percent(96.),
                    padding: UiRect::all(px(6.)),
                    ..default()
                },
                Text::new(""),
                TextFont::from_font_size(FontSize::Px(18.)).with_font(font),
                TextColor(Color::WHITE),
                BackgroundColor(Color::srgba(0., 0., 0., 0.65)),
            ))
            .id();
        world.resource_mut::<State>().overlay = Some((camera, label));
    }
    let state = world.resource::<State>();
    let (camera, label) = state.overlay.unwrap();
    let mode = state.mode;
    let frontend = world.resource::<crate::frontend_scene::Scene>();
    let showing = frontend.flow.phase != Phase::Menu && !state.hidden;
    let text = if !showing {
        String::new()
    } else {
        let title = frontend.showcase_caption();
        let status = match mode {
            Mode::Free => "FREE CAMERA - scene preview",
            Mode::Opening => "CINEMATIC PREVIEW - mission admission incomplete",
            Mode::Mecc => "MECC PREVIEW - mission rules incomplete",
            Mode::Unavailable => "PREVIEW UNAVAILABLE - this mission still needs implementation",
            Mode::Loading => match frontend.flow.phase {
                Phase::Cinematic => "CINEMATIC",
                Phase::Gameplay => "MECC - movement / jet / diagnostic pistol",
                _ => "LOADING",
            },
        };
        format!(
            "{title} | {status}\nPartial scene / native fidelity incomplete. F6: free camera | F8: hide hints | {} | F10: quit{}",
            if mode == Mode::Opening || frontend.flow.phase == Phase::Cinematic {
                "Esc / double-click: skip cinematic"
            } else {
                "Esc: menu"
            },
            if mode == Mode::Free {
                "\nZQSD / WASD or arrows: move | RMB: look | Space / Ctrl: up / down | Shift: faster | R: reset"
            } else {
                ""
            }
        )
    };
    if world.get::<Text>(label).is_some_and(|t| t.0 != text) {
        world.get_mut::<Text>(label).unwrap().0 = text;
    }
    *world.get_mut::<Visibility>(label).unwrap() = if showing {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    let mut camera = world.get_mut::<Camera>(camera).unwrap();
    camera.is_active = showing;
    camera.clear_color = if mode == Mode::Unavailable {
        ClearColorConfig::Custom(Color::BLACK)
    } else {
        ClearColorConfig::None
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mecc_preview_control_is_separate_from_loading_and_campaign_readiness() {
        let mut state = State::default();
        assert!(!state.controls(Phase::AwaitingRuntime));
        state.enabled = true;
        state.mode = Mode::Mecc;
        assert!(state.controls(Phase::AwaitingRuntime));
        assert!(!state.controls(Phase::Cinematic));
        assert!(!state.controls(Phase::Menu));
        state.mode = Mode::Free;
        assert!(!state.controls(Phase::AwaitingRuntime));
        assert!(!state.controls(Phase::Gameplay));
        state.mode = Mode::Loading;
        assert!(state.controls(Phase::Gameplay));
    }
    #[test]
    fn free_camera_inherits_source_pose_and_moves_in_its_view_axes() {
        let eye = Vec3::new(10., 20., 30.);
        let view = crate::mecc_dev_scene::camera_view(eye, Vec3::NEG_Y, Vec3::Z).to_cols_array();
        let mut camera = FreeCamera::from_view(view).unwrap();
        assert!(camera.eye.distance(eye) < 0.001);
        camera.step(Vec3::Y, Vec2::ZERO, 0.1, false);
        assert!(camera.eye.distance(eye + Vec3::NEG_Y * 25.) < 0.001);
        camera.step(Vec3::Z, Vec2::new(100., 200.), 0.1, true);
        assert!(camera.eye.z > eye.z + 90. && camera.view().iter().all(|x| x.is_finite()));
        assert!(FreeCamera::from_view([0.; 16]).is_none());
    }
}
