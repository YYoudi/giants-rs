//! Source-text frontend and real ticket-owned mission handoff.
#[path = "mission_replay.rs"]
mod replay;
use bevy::input::{ButtonState, keyboard::KeyboardInput};
use bevy::prelude::*;
use giants_game::{
    audio_options::AudioOptions,
    control_options::ControlOptions,
    display_options::DisplayOptions,
    frontend::{Activation, Entry, Frontend, Screen},
    graphics_options::GraphicsOptions,
    mission_loading::{self, Campaign, Flow, LaunchIntent, Phase, PreparedMission},
    mission_selection::{LoadRequest, Progress},
    profile_store::Profiles,
};
pub(crate) use replay::{
    capture_allowed as mission_capture_allowed, initial_body_inspection_present,
    initial_body_inspection_replay, install as install_mission_replay, owned_inspection_replay,
    vimp_interaction_probe,
};
impl Scene {
    pub(crate) fn begin_campaign_film(
        &mut self,
        completion: &crate::mission_runtime::CampaignCompletion,
    ) -> Result<(), String> {
        if self
            .prepared_mission
            .as_ref()
            .is_none_or(|p| p.request != completion.current)
            || completion.film.is_none()
            || !self.flow.begin_completion_film(completion.ticket)
        {
            return Err("completion film lost its admitted mission owner".into());
        }
        self.revision += 1;
        Ok(())
    }
    pub(crate) fn advance_campaign(
        &mut self,
        completion: &crate::mission_runtime::CampaignCompletion,
        images: &mut Assets<Image>,
        film: Option<crate::mission_opening::CompletionReceipt>,
    ) -> Result<u64, String> {
        let phase_admitted = if completion.film.is_some() {
            film.is_some_and(|f| f.ticket() == completion.ticket)
                && matches!(
                    self.flow.phase,
                    Phase::Cinematic | Phase::Objectives | Phase::Gameplay
                )
        } else {
            film.is_none() && self.flow.phase == Phase::Gameplay
        };
        if self.flow.ticket != completion.ticket
            || !phase_admitted
            || self
                .prepared_mission
                .as_ref()
                .is_none_or(|p| p.request != completion.current)
        {
            return Err("campaign transition is stale or not owned by ordinary gameplay".into());
        }
        begin_mission(
            self,
            Ok(completion.next.clone()),
            images,
            LaunchIntent::SavedMission,
        );
        if self.flow.ticket == completion.ticket {
            return Err("next campaign request did not start".into());
        }
        self.revision += 1;
        Ok(self.flow.ticket)
    }
    pub(crate) fn record_campaign_reached(
        &mut self,
        ticket: u64,
        request: &LoadRequest,
    ) -> Result<(), String> {
        if ticket != self.flow.ticket
            || !matches!(
                self.flow.phase,
                Phase::Cinematic | Phase::Objectives | Phase::Gameplay
            )
            || self
                .prepared_mission
                .as_ref()
                .is_none_or(|p| p.request != *request)
        {
            return Err("campaign entry has no matching runtime admission".into());
        }
        let profile = self
            .profiles
            .names
            .get(self.profiles.active)
            .ok_or("campaign profile absent")?;
        self.campaign.record_reached(profile, request)
    }
    pub(crate) fn browse_preview_missions(&mut self) -> Result<(), String> {
        let name = self.profiles.preview_profile().map_err(|e| e.to_string())?
            .ok_or("The five profile slots are occupied. Remove a profile in Player IDs to create Preview All.")?;
        let missions = self.campaign.unlock_profile(&name)?;
        self.completion_replay = false;
        self.completion_requests.clear();
        self.screen = Screen::LoadWorlds;
        self.selected = Some(0);
        self.notice = Some(format!(
            "{missions} missions unlocked for preview; playable support varies."
        ));
        self.revision += 1;
        Ok(())
    }
    pub(crate) fn browse_completion_replays(&mut self) -> Result<(), String> {
        self.browse_preview_missions()?;
        self.completion_replay = true;
        self.notice =
            Some("Replay a completion cinematic; no mission completion is recorded.".into());
        Ok(())
    }

    pub(crate) fn preview_notice(&mut self, message: String) {
        self.notice = Some(message);
        self.revision += 1;
    }
    pub(crate) fn message_palette(
        &self,
    ) -> Result<giants_game::message_presentation::Palette, String> {
        let catalog = giants_formats::gtext::Catalog::from_table(self.campaign.text.clone())
            .map_err(|e| e.to_string())?;
        giants_game::message_presentation::Palette::load(&catalog).map_err(str::to_owned)
    }
}

#[derive(Resource)]
pub struct Scene {
    campaign: Campaign,
    pub flow: Flow,
    load_request: Option<LoadRequest>,
    launch_intent: LaunchIntent,
    completion_replay: bool,
    completion_requests: Vec<LoadRequest>,
    splash: Option<Handle<Image>>,
    worker: Option<std::sync::Mutex<std::sync::mpsc::Receiver<Result<PreparedMission, String>>>>,
    pub prepared_mission: Option<std::sync::Arc<PreparedMission>>,
    data: Frontend,
    font: Vec<u8>,
    screen: Screen,
    selected: Option<usize>,
    notice: Option<String>,
    revision: u64,
    profiles: Profiles,
    draft: String,
    delete_target: Option<usize>,
    last_cursor: Option<Vec2>,
    controls: ControlOptions,
    binding_target: Option<usize>,
    dragging_mouse_speed: bool,
    display: DisplayOptions,
    display_adapter: String,
    graphics: GraphicsOptions,
    dragging_distance: bool,
    audio: AudioOptions,
    dragging_audio: Option<usize>,
    pub(crate) feedback_actions: Vec<crate::frontend_feedback::Cue>,
}
impl Scene {
    pub(crate) fn showcase_caption(&self) -> String {
        self.load_request.as_ref().map_or_else(String::new, |r| {
            let title = self.campaign.mission_title(&r.mission_name);
            if self.launch_intent == LaunchIntent::CompletionReplay {
                format!("Completion cinematic: {title}")
            } else {
                title
            }
        })
    }
    pub(crate) fn showcase_dismiss_objectives(&mut self) -> bool {
        if self.flow.phase == Phase::Objectives && self.flow.dismiss_objectives(self.flow.ticket) {
            self.revision += 1;
            true
        } else {
            false
        }
    }
    pub(crate) fn mecc_controls(&self) -> &ControlOptions {
        if matches!(self.screen, Screen::Controls(_)) {
            &self.controls
        } else {
            self.profiles
                .controls
                .get(self.profiles.active)
                .unwrap_or(&self.controls)
        }
    }

    pub(crate) fn feedback_selection(&self) -> (Screen, Option<usize>) {
        (
            self.screen,
            self.selected.filter(|i| *i < self.entries().len()),
        )
    }
    pub(crate) fn feedback_volume(&self) -> f32 {
        if self.screen == Screen::Sound {
            self.audio.effects
        } else {
            self.profiles
                .audio
                .get(self.profiles.active)
                .copied()
                .unwrap_or_default()
                .effects
        }
    }
    pub fn data_root(&self) -> &std::path::Path {
        self.campaign.root()
    }
    pub(crate) fn launch_intent(&self) -> LaunchIntent {
        self.launch_intent
    }
    pub(crate) fn music_volume(&self) -> f32 {
        if self.screen == Screen::Sound {
            self.audio.music
        } else {
            self.profiles
                .audio
                .get(self.profiles.active)
                .copied()
                .unwrap_or_default()
                .music
        }
    }

    pub fn load(
        root: &std::path::Path,
        user_root: &std::path::Path,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let user_root = if user_root.exists() {
            user_root.canonicalize()?
        } else {
            user_root
                .parent()
                .ok_or("User-data parent missing")?
                .canonicalize()?
                .join(user_root.file_name().ok_or("User-data folder missing")?)
        };
        if user_root.starts_with(root) || root.starts_with(&user_root) {
            return Err(
                "Frontend user data must be separate from game data and original installation"
                    .into(),
            );
        }
        let mut profiles = Profiles::open(&user_root)?;
        let mut campaign = Campaign::open(root, &user_root)?;
        let normal = std::env::var_os("GIANTS_MISSION_REPLAY").is_none()
            && std::env::var_os("GIANTS_MECC_DEV_REPLAY").is_none();
        let preview = if normal {
            profiles.preview_profile()?
        } else {
            None
        };
        if let Some(name) = &preview {
            let missions = campaign.unlock_profile(name)?;
            println!("preview_default_profile=true missions={missions} campaign_completed=false");
        }
        let font_path = std::env::var_os("GIANTS_INSPECTION_FONT")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(crate::inspection_font_path);
        let font = std::fs::read(&font_path)
            .map_err(|e| format!("Inspection font {}: {e}", font_path.display()))?;
        let data = Frontend::load(root)?;
        println!(
            "frontend_inspection=true source=GTextEnglish profiles=independent_store main_entries={} layout=measured_single_pose font=external_inspection background=owned_source_preview native_fidelity=unverified",
            data.main.len()
        );
        Ok(Self {
            campaign,
            flow: Flow::default(),
            load_request: None,
            launch_intent: LaunchIntent::SavedMission,
            completion_replay: false,
            completion_requests: Vec::new(),
            splash: None,
            worker: None,
            prepared_mission: None,
            data,
            font,
            screen: if preview.is_some() {
                Screen::Main
            } else {
                Screen::PlayerIds
            },
            selected: if preview.is_some() {
                Some(4)
            } else {
                Some(profiles.active)
            },
            notice: None,
            revision: 0,
            profiles,
            draft: String::new(),
            delete_target: None,
            last_cursor: None,
            controls: ControlOptions::default(),
            binding_target: None,
            dragging_mouse_speed: false,
            display: DisplayOptions::default(),
            display_adapter: String::new(),
            graphics: GraphicsOptions::default(),
            dragging_distance: false,
            audio: AudioOptions::default(),
            dragging_audio: None,
            feedback_actions: Vec::new(),
        })
    }
    fn leave_sound(&mut self) {
        match self.profiles.set_audio(self.audio) {
            Ok(()) => {
                self.screen = Screen::Options;
                self.selected = Some(1);
                self.notice = None;
                self.dragging_audio = None;
                println!(
                    "frontend_audio_saved=true effects={} music={}",
                    self.audio.effects, self.audio.music
                );
            }
            Err(error) => self.notice = Some(error.to_string()),
        }
        self.revision += 1;
    }
    fn leave_graphics(&mut self) {
        match self.profiles.set_graphics(self.graphics) {
            Ok(()) => {
                self.screen = Screen::Options;
                self.selected = Some(0);
                self.notice = None;
                self.dragging_distance = false;
                println!(
                    "frontend_graphics_saved={:?} renderer_applied=false",
                    self.graphics.0
                );
            }
            Err(error) => self.notice = Some(error.to_string()),
        }
        self.revision += 1;
    }
    fn leave_controls(&mut self, unit: usize) {
        match self.profiles.set_controls(self.controls.clone()) {
            Ok(()) => {
                self.screen = Screen::Options;
                self.selected = Some(unit + 2);
                self.notice = None;
                self.binding_target = None;
                self.dragging_mouse_speed = false;
                println!("frontend_controls_saved=true unit={unit}");
            }
            Err(error) => self.notice = Some(error.to_string()),
        }
        self.revision += 1;
    }
    fn entries(&self) -> Vec<Entry> {
        if matches!(self.screen, Screen::LoadWorlds | Screen::LoadMissions) {
            let names = if self.screen == Screen::LoadWorlds {
                self.campaign
                    .available_worlds()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|w| w.title.clone())
                    .collect::<Vec<_>>()
            } else if self.completion_replay {
                self.completion_requests
                    .iter()
                    .map(|r| self.campaign.mission_title(&r.mission_name))
                    .collect::<Vec<_>>()
            } else {
                self.campaign
                    .menu
                    .as_ref()
                    .map(|m| {
                        m.entries
                            .iter()
                            .map(|e| self.campaign.mission_title(&e.name))
                            .collect()
                    })
                    .unwrap_or_default()
            };
            return names
                .into_iter()
                .chain(std::iter::once("Back".into()))
                .enumerate()
                .map(|(i, text)| Entry {
                    text,
                    help: String::new(),
                    layout: [0.2, 0.15 + i as f32 * 0.037, 0.029],
                })
                .collect();
        }
        if self.screen == Screen::MissionLoading {
            return vec![];
        }
        if self.screen == Screen::Resolution {
            let mut entries: Vec<_> = self
                .display
                .visible()
                .iter()
                .enumerate()
                .map(|(i, mode)| Entry {
                    text: format!("{}x{}", mode[0], mode[1]),
                    help: "Select a display size to apply".into(),
                    layout: [0.626868, 0.3932773 + i as f32 * 0.0358, 0.0333333],
                })
                .collect();
            for (text, help, x, y) in [
                ("^", "Previous resolutions", 0.946, 0.409),
                ("v", "Next resolutions", 0.946, 0.601),
                (
                    self.data.resolution_text[5].as_str(),
                    "Apply the selected size to this window",
                    0.7282394,
                    0.7909456,
                ),
                (
                    self.data.resolution_text[6].as_str(),
                    "Restore the initial window size candidate",
                    0.7204269,
                    0.8269381,
                ),
                (
                    self.data.resolution_text[7].as_str(),
                    self.data.resolution_text[8].as_str(),
                    0.4167085,
                    0.9313075,
                ),
            ] {
                entries.push(Entry {
                    text: text.into(),
                    help: help.into(),
                    layout: [x, y, 0.0333333],
                });
            }
            return entries;
        }
        if let Screen::Controls(unit) = self.screen {
            let page = &self.data.control_pages[unit];
            let mut rows = page.entries.clone();
            rows[page.bindings.len() + 1].text =
                page.invert_labels[usize::from(self.controls.2)].clone();
            return rows;
        }
        if self.screen != Screen::PlayerIds {
            return self.data.entries(self.screen).to_vec();
        }
        let mut rows: Vec<Entry> = self
            .profiles
            .names
            .iter()
            .enumerate()
            .map(|(index, name)| {
                let mut entry = self.data.profile.clone();
                entry.text = name.clone();
                // Only the first profile slot is native-measured. Later slots reuse main-menu rows
                // as an explicit inspection layout until native multi-profile layout is established.
                if index > 0 {
                    entry.layout = self.data.main[index].layout;
                }
                entry.layout[0] = 0.3; // Wide inspection hit rectangle for variable-length names.
                entry
            })
            .collect();
        if rows.len() < 5 {
            rows.push(Entry {
                text: self.data.new_id.clone(),
                help: self.data.new_id_help.clone(),
                layout: [0.4422828, 0.9206434, 0.0425655],
            });
        }
        rows
    }
}
#[derive(Resource)]
pub struct InspectionFont(pub(crate) Handle<Font>);
#[derive(Component)]
pub struct FrontendElement;
#[derive(Component)]
pub struct FrontendCamera;

#[derive(Message)]
pub struct MissionPrepared {
    pub ticket: u64,
    pub request: LoadRequest,
    pub intent: LaunchIntent,
    pub progress: Progress,
}

/// Only an actual runtime consumer may send these acknowledgements.
#[derive(Message)]
#[allow(dead_code)] // Runtime integration port; no fabricated producer in the inspection frontend.
pub enum MissionRuntimeEvent {
    Ready {
        ticket: u64,
        cinematic: bool,
        gameplay_ready: bool,
    },
    CinematicFinished {
        ticket: u64,
    },
    /// Explicit user skip after the opening owner has released its resources.
    CinematicSkipped {
        ticket: u64,
    },
    GameplayReady {
        ticket: u64,
    },
}
#[derive(Message)]
pub struct MissionCancelled(pub u64);

pub fn runtime_events(
    mut scene: ResMut<Scene>,
    mut events: MessageReader<MissionRuntimeEvent>,
    mut cameras: Query<&mut Camera, With<FrontendCamera>>,
) {
    for event in events.read() {
        let applied = match *event {
            MissionRuntimeEvent::Ready {
                ticket,
                cinematic,
                gameplay_ready,
            } => {
                if scene
                    .prepared_mission
                    .as_ref()
                    .is_none_or(|p| !p.needs.is_empty())
                {
                    eprintln!("mission_runtime_ack_rejected ticket={ticket} unresolved_data=true");
                    false
                } else {
                    scene.flow.runtime_ready(ticket, cinematic, gameplay_ready)
                }
            }
            MissionRuntimeEvent::CinematicFinished { ticket } => {
                scene.flow.cinematic_finished(ticket)
            }
            MissionRuntimeEvent::CinematicSkipped { ticket } => {
                scene.flow.cinematic_skipped(ticket)
            }
            MissionRuntimeEvent::GameplayReady { ticket } => scene.flow.gameplay_ready(ticket),
        };
        if applied {
            scene.revision += 1;
        }
    }
    for mut camera in &mut cameras {
        camera.is_active = !matches!(scene.flow.phase, Phase::Cinematic | Phase::Gameplay);
        camera.clear_color = if scene.flow.phase == Phase::Menu {
            ClearColorConfig::None
        } else {
            ClearColorConfig::Default
        };
    }
}

fn accept_objectives(scene: &mut Scene, mouse: &mut ButtonInput<MouseButton>) -> bool {
    if !scene.flow.dismiss_objectives(scene.flow.ticket) {
        return false;
    }
    // The click belongs to the dialog, not the first gameplay shot.
    mouse.reset(MouseButton::Left);
    scene.revision += 1;
    true
}

fn begin_mission(
    scene: &mut Scene,
    request: Result<LoadRequest, String>,
    images: &mut Assets<Image>,
    intent: LaunchIntent,
) {
    let result = (|| {
        let request = request?;
        let image = mission_loading::splash(scene.campaign.root(), &request)?;
        let rgba = image
            .pixels
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect();
        let mut texture = Image::new(
            bevy::render::render_resource::Extent3d {
                width: image.width,
                height: image.height,
                depth_or_array_layers: 1,
            },
            bevy::render::render_resource::TextureDimension::D2,
            rgba,
            bevy::render::render_resource::TextureFormat::Rgba8Unorm,
            bevy::asset::RenderAssetUsages::default(),
        );
        texture.sampler = bevy::image::ImageSampler::linear();
        scene.splash = Some(images.add(texture));
        let ticket = scene.flow.begin();
        println!(
            "mission_request ticket={ticket} world={} menu_index={} source_id={} name={} splash={} runtime_started=false",
            request.world,
            request.menu_index,
            request.source_id,
            request.mission_name,
            request.splash.asset_stem()
        );
        scene.load_request = Some(request);
        scene.launch_intent = intent;
        scene.completion_replay = intent == LaunchIntent::CompletionReplay;
        scene.prepared_mission = None;
        scene.worker = None;
        scene.screen = Screen::MissionLoading;
        scene.notice = None;
        Ok::<_, String>(())
    })();
    if let Err(error) = result {
        eprintln!("mission_request_error={error}");
        scene.notice = Some("This mission could not be loaded.".into());
    }
}

fn cancel_mission(scene: &mut Scene, cancelled: &mut MessageWriter<MissionCancelled>) {
    let event = MissionCancelled(scene.flow.ticket);
    scene
        .feedback_actions
        .push(crate::frontend_feedback::Cue::Back);
    println!("mission_cancelled ticket={}", event.0);
    cancelled.write(event);
    scene.flow.cancel();
    scene.worker = None;
    scene.prepared_mission = None;
    scene.splash = None;
    scene.screen = if scene.campaign.menu.is_some() {
        Screen::LoadMissions
    } else {
        Screen::Main
    };
    scene.selected = None;
    scene.notice = None;
    scene.revision += 1;
}

/// CPU work begins after three prepared UI frames with render pipelines ready.
/// This is not a measurement of three native Present calls or native load timing.
pub fn advance_mission(
    mut scene: ResMut<Scene>,
    pipelines: Res<crate::PipelinesReady>,
    mut ready: MessageWriter<MissionPrepared>,
) {
    if scene.screen != Screen::MissionLoading {
        return;
    }
    if pipelines.0 && scene.flow.presented() {
        let root = scene.campaign.root().to_owned();
        let request = scene
            .load_request
            .clone()
            .expect("Loading requires a request");
        let (tx, rx) = std::sync::mpsc::channel();
        scene.worker = Some(std::sync::Mutex::new(rx));
        std::thread::spawn(move || {
            let _ = tx.send(mission_loading::prepare(&root, request));
        });
        scene.revision += 1;
    }
    let result = scene.worker.as_ref().and_then(|rx| {
        match rx.lock().expect("Owned loading receiver").try_recv() {
            Ok(result) => Some(result),
            Err(std::sync::mpsc::TryRecvError::Empty) => None,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                Some(Err("Loading worker disconnected".into()))
            }
        }
    });
    if let Some(result) = result {
        let ticket = scene.flow.ticket;
        scene.worker = None;
        match result {
            Ok(prepared) => {
                println!(
                    "mission_prepared ticket={ticket} scenario_index={:?} placements={:?} runtime_started=false needs={:?}",
                    prepared.scenario_index,
                    prepared.mission.as_ref().map(|m| m.placements.len()),
                    prepared.needs
                );
                let event = MissionPrepared {
                    ticket,
                    request: prepared.request.clone(),
                    intent: scene.launch_intent,
                    progress: if scene.launch_intent == LaunchIntent::NewCampaign {
                        Progress::load(None, None).unwrap()
                    } else {
                        scene.campaign.progress.clone()
                    },
                };
                println!(
                    "mission_handoff ticket={} world={} intent={:?} progress_world={} cinematic_ack=false gameplay_ack=false",
                    event.ticket,
                    event.request.world,
                    event.intent,
                    event.progress.world_word()
                );
                ready.write(event);
                scene.prepared_mission = Some(std::sync::Arc::new(prepared));
                scene.flow.prepared(ticket);
            }
            Err(error) => {
                eprintln!("mission_prepare_error ticket={ticket} error={error}");
                scene.flow.fail(ticket, error);
            }
        }
        scene.revision += 1;
    }
}

pub fn setup(
    mut commands: Commands,
    scene: Res<Scene>,
    mut fonts: ResMut<Assets<Font>>,
    actor: Option<Res<crate::mecc_dev_scene::State>>,
) {
    commands.spawn((
        Camera2d,
        FrontendCamera,
        IsDefaultUiCamera,
        Msaa::Off,
        Camera {
            order: 10,
            is_active: actor.is_none(),
            ..default()
        },
    ));
    commands.insert_resource(InspectionFont(
        fonts.add(Font::from_bytes(scene.font.clone())),
    ));
}
#[allow(clippy::too_many_arguments)] // Bevy system parameters; images belong to this input transaction.
pub fn input(
    mut scene: ResMut<Scene>,
    mut windows: Query<&mut Window>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut keyboard: MessageReader<KeyboardInput>,
    mut exit: MessageWriter<AppExit>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut cancelled: MessageWriter<MissionCancelled>,
) {
    if keys.just_pressed(KeyCode::F9)
        && let Some(path) = std::env::var_os("GIANTS_FRONTEND_CAPTURE")
        && !std::path::Path::new(&path).exists()
    {
        commands
            .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
            .observe(bevy::render::view::screenshot::save_to_disk(
                std::path::PathBuf::from(path),
            ));
    }
    if keys.just_pressed(KeyCode::F10) {
        exit.write(AppExit::Success);
    }
    if scene.screen == Screen::MissionLoading {
        if scene.flow.phase == Phase::Objectives && mouse.just_pressed(MouseButton::Left) {
            accept_objectives(&mut scene, &mut mouse);
        }
        if keys.just_pressed(KeyCode::Escape) {
            cancel_mission(&mut scene, &mut cancelled);
        }
        keyboard.clear();
        return;
    }
    if let Screen::Controls(unit) = scene.screen
        && let Some(row) = scene.binding_target
    {
        if keys.just_pressed(KeyCode::Escape) {
            scene.binding_target = None;
            scene
                .feedback_actions
                .push(crate::frontend_feedback::Cue::Back);
            scene.notice = None;
            scene.revision += 1;
            keyboard.clear();
            return;
        }
        let key = keyboard
            .read()
            .find(|event| {
                event.state == ButtonState::Pressed
                    && !matches!(event.key_code, KeyCode::F9 | KeyCode::F10)
            })
            .map(|event| format!("{:?}", event.key_code));
        let physical = key.or_else(|| {
            [
                (MouseButton::Left, "MouseLeft"),
                (MouseButton::Right, "MouseRight"),
                (MouseButton::Middle, "MouseMiddle"),
            ]
            .into_iter()
            .find(|(button, _)| mouse.just_pressed(*button))
            .map(|(_, name)| name.into())
        });
        if let Some(physical) = physical {
            if scene.controls.bind(unit, row, &physical) {
                scene.binding_target = None;
                scene.notice = None;
                println!("frontend_binding_changed unit={unit} row={row} physical={physical}");
            } else {
                scene.notice =
                    Some("This input is not supported by the inspection runtime.".into());
            }
            scene.revision += 1;
        }
        return;
    }
    if scene.screen == Screen::NewPlayer {
        for event in keyboard.read().filter(|e| e.state == ButtonState::Pressed) {
            if let Some(text) = &event.text {
                for character in text
                    .chars()
                    .filter(|c| c.is_ascii() && !c.is_ascii_control())
                {
                    if scene.draft.len() < 31 {
                        scene.draft.push(character);
                        scene.revision += 1;
                    }
                }
            }
        }
        if keys.just_pressed(KeyCode::Backspace) {
            scene.draft.pop();
            scene.revision += 1;
        }
        if keys.just_pressed(KeyCode::Escape) {
            scene.screen = Screen::PlayerIds;
            scene
                .feedback_actions
                .push(crate::frontend_feedback::Cue::Back);
            scene.draft.clear();
            scene.notice = None;
            scene.revision += 1;
            println!("frontend_profile_edit=cancelled");
        } else if keys.just_pressed(KeyCode::Enter) {
            let draft = scene.draft.clone();
            match scene.profiles.create(&draft) {
                Ok(()) => {
                    scene.screen = Screen::Main;
                    scene
                        .feedback_actions
                        .push(crate::frontend_feedback::Cue::Activate);
                    scene.selected = None;
                    scene.draft.clear();
                    scene.notice = None;
                    println!(
                        "frontend_profile_created=true count={}",
                        scene.profiles.names.len()
                    );
                }
                Err(error) => scene.notice = Some(error.to_string()),
            }
            scene.revision += 1;
        }
        return;
    }
    keyboard.clear();
    if keys.just_pressed(KeyCode::Escape) {
        if matches!(
            scene.screen,
            Screen::LoadWorlds
                | Screen::LoadMissions
                | Screen::ExitConfirmation
                | Screen::DeletePlayer
                | Screen::Controls(_)
                | Screen::Resolution
                | Screen::Graphics
                | Screen::Sound
                | Screen::Options
        ) {
            scene
                .feedback_actions
                .push(crate::frontend_feedback::Cue::Back);
        }
        match scene.screen {
            Screen::LoadWorlds | Screen::LoadMissions => {
                scene.screen = if scene.screen == Screen::LoadMissions {
                    Screen::LoadWorlds
                } else {
                    Screen::Main
                };
                scene.selected = None;
                scene.notice = None;
                scene.revision += 1;
                return;
            }
            Screen::ExitConfirmation => {
                scene.screen = Screen::Main;
                scene.selected = Some(6);
                scene.notice = None;
                scene.revision += 1;
                println!("frontend_exit=cancelled");
                return;
            }
            Screen::DeletePlayer => {
                scene.screen = Screen::PlayerIds;
                scene.selected = scene.delete_target.take();
                scene.notice = None;
                scene.revision += 1;
                println!("frontend_profile_delete=cancelled");
                return;
            }
            Screen::Controls(unit) => {
                scene.leave_controls(unit);
                return;
            }
            Screen::Resolution => {
                scene.screen = Screen::Graphics;
                scene.selected = Some(11);
                scene.notice = None;
                scene.revision += 1;
                return;
            }
            Screen::Graphics => {
                scene.leave_graphics();
                return;
            }
            Screen::Sound => {
                scene.leave_sound();
                return;
            }
            Screen::Options => {
                scene.screen = Screen::Main;
                scene.selected = Some(3);
                scene.notice = None;
                scene.revision += 1;
                return;
            }
            _ => {}
        }
    }
    if scene.screen == Screen::PlayerIds
        && keys.just_pressed(KeyCode::Delete)
        && let Some(index) = scene.selected.filter(|&i| i < scene.profiles.names.len())
    {
        scene.delete_target = Some(index);
        scene.screen = Screen::DeletePlayer;
        scene.selected = Some(1); // S04 native confirmation opens with No selected.
        scene.notice = None;
        scene.revision += 1;
        println!("frontend_profile_delete=confirmation");
        return;
    }

    if let Screen::Controls(unit) = scene.screen
        && keys.just_pressed(KeyCode::Delete)
        && let Some(row) = scene
            .selected
            .filter(|&row| row < scene.data.control_pages[unit].bindings.len())
    {
        scene.controls.bind(unit, row, "");
        scene.revision += 1;
        return;
    }
    // Keyboard navigation does not depend on having a cursor inside the window.
    let entries = scene.entries();
    let point = windows
        .single()
        .ok()
        .and_then(|window| window.cursor_position())
        .and_then(|cursor| {
            windows
                .single()
                .ok()
                .map(|window| cursor / Vec2::new(window.width(), window.height()))
        });
    if scene.screen == Screen::Sound {
        if !mouse.pressed(MouseButton::Left) {
            scene.dragging_audio = None;
        }
        let direction = i8::from(keys.just_pressed(KeyCode::ArrowRight))
            - i8::from(keys.just_pressed(KeyCode::ArrowLeft));
        if direction != 0
            && let Some(row @ 0..=1) = scene.selected
        {
            scene.audio.adjust(row == 1, direction);
            scene.revision += 1;
            return;
        }
        if let Some(point) = point {
            let row = scene.dragging_audio.or_else(|| {
                [0.43970147, 0.4965251]
                    .iter()
                    .position(|&y| point.y >= y - 0.0426218 * 15.0 / 24.0 && point.y <= y + 0.018)
            });
            if let Some(row) = row {
                let before = scene.audio;
                if scene.dragging_audio.is_some() && mouse.pressed(MouseButton::Left) {
                    scene
                        .audio
                        .position(row == 1, (point.x - 0.5833) / (0.75 - 0.5833));
                } else if mouse.just_pressed(MouseButton::Left) {
                    if (0.55..0.5833).contains(&point.x) {
                        scene.audio.adjust(row == 1, -1);
                    } else if (0.75..=0.7833).contains(&point.x) {
                        scene.audio.adjust(row == 1, 1);
                    } else if (0.5833..0.75).contains(&point.x) {
                        scene.dragging_audio = Some(row);
                        scene
                            .audio
                            .position(row == 1, (point.x - 0.5833) / (0.75 - 0.5833));
                    }
                }
                if scene.audio != before {
                    scene.selected = Some(row);
                    scene.notice = None;
                    scene.revision += 1;
                    return;
                }
            }
        }
    }
    if let Screen::Controls(unit) = scene.screen {
        let count = scene.data.control_pages[unit].bindings.len();
        let direction = i8::from(keys.just_pressed(KeyCode::ArrowRight))
            - i8::from(keys.just_pressed(KeyCode::ArrowLeft));
        if direction != 0 && scene.selected == Some(count) {
            scene.controls.adjust_mouse(direction);
            scene.revision += 1;
            return;
        }
        if !mouse.pressed(MouseButton::Left) {
            scene.dragging_mouse_speed = false;
        }
        if let Some(point) = point
            && (scene.dragging_mouse_speed
                || (point.y - scene.data.control_pages[unit].slider_y).abs() < 0.022)
        {
            let before = scene.controls.1;
            if scene.dragging_mouse_speed && mouse.pressed(MouseButton::Left) {
                scene.controls.position_mouse((point.x - 0.095) / 0.168);
            } else if mouse.just_pressed(MouseButton::Left) {
                if (0.063..0.095).contains(&point.x) {
                    scene.controls.adjust_mouse(-1);
                } else if (0.263..0.295).contains(&point.x) {
                    scene.controls.adjust_mouse(1);
                } else if (0.095..0.263).contains(&point.x) {
                    scene.dragging_mouse_speed = true;
                    scene.controls.position_mouse((point.x - 0.095) / 0.168);
                }
            }
            if before != scene.controls.1 {
                scene.selected = Some(count);
                scene.revision += 1;
                return;
            }
        }
    }
    if scene.screen == Screen::Resolution {
        let direction = i8::from(keys.just_pressed(KeyCode::ArrowRight))
            - i8::from(keys.just_pressed(KeyCode::ArrowLeft));
        if direction != 0 {
            scene.display.cycle(direction);
            scene.selected = scene
                .display
                .visible()
                .iter()
                .position(|mode| *mode == scene.display.pending);
            scene.revision += 1;
            return;
        }
        let page = i8::from(keys.just_pressed(KeyCode::PageDown))
            - i8::from(keys.just_pressed(KeyCode::PageUp));
        if page != 0 {
            scene.display.page(page);
            scene.revision += 1;
            return;
        }
    }
    if scene.screen == Screen::Graphics {
        if !mouse.pressed(MouseButton::Left) {
            scene.dragging_distance = false;
        }
        let direction = i8::from(keys.just_pressed(KeyCode::ArrowRight))
            - i8::from(keys.just_pressed(KeyCode::ArrowLeft));
        if direction != 0
            && let Some(row @ 0..=9) = scene.selected
        {
            if row == 9 {
                scene.graphics.adjust_distance(direction);
            } else {
                scene.graphics.cycle(row, direction);
            }
            scene.revision += 1;
            return;
        }
        if let Some(point) = point
            && (scene.dragging_distance || (point.y >= 0.707 && point.y <= 0.752))
        {
            let before = scene.graphics;
            if scene.dragging_distance && mouse.pressed(MouseButton::Left) {
                scene.graphics.position_distance((point.x - 0.6) / 0.2333);
            } else if mouse.just_pressed(MouseButton::Left) {
                if (0.5667..0.6).contains(&point.x) {
                    scene.graphics.adjust_distance(-1);
                } else if (0.8333..=0.8667).contains(&point.x) {
                    scene.graphics.adjust_distance(1);
                } else if (0.6..0.8333).contains(&point.x) {
                    scene.dragging_distance = true;
                    scene.graphics.position_distance((point.x - 0.6) / 0.2333);
                }
            }
            if scene.graphics != before {
                scene.selected = Some(9);
                scene.notice = None;
                scene.revision += 1;
                return;
            }
        }
    }
    let moved = scene.last_cursor != point;
    scene.last_cursor = point;
    if !entries.is_empty() {
        let last = entries.len() - 1;
        let next = if keys.just_pressed(KeyCode::ArrowUp) {
            Some(
                scene
                    .selected
                    .map_or(last, |index| index.min(last).saturating_sub(1)),
            )
        } else if keys.just_pressed(KeyCode::ArrowDown) {
            Some(scene.selected.map_or(0, |index| (index + 1).min(last)))
        } else {
            None
        };
        if let Some(next) = next {
            scene.selected = Some(next);
            scene.notice = None;
            scene.revision += 1;
            println!(
                "frontend_keyboard_selection={:?} index={next}",
                scene.screen
            );
            return;
        }
    }
    let hovered = point.and_then(|point| {
        entries.iter().position(|entry| {
            let [left, baseline, size] = entry.layout;
            // Sampled text rectangles, not the original menu's polygon hit tests.
            point.x >= left
                && point.x
                    <= if matches!(scene.screen, Screen::Controls(_)) {
                        if left < 0.1 { 0.52 } else { 0.98 }
                    } else if matches!(
                        scene.screen,
                        Screen::Options | Screen::Sound | Screen::Graphics
                    ) {
                        0.985
                    } else {
                        1.0 - left
                    }
                && point.y >= baseline - size * 15.0 / 24.0
                && point.y <= baseline + size * 9.0 / 24.0
        })
    });
    if let Some(index) = hovered
        && moved
        && scene.selected != Some(index)
    {
        scene.selected = Some(index);
        scene.notice = None;
        scene.revision += 1;
    }
    let activation = if keys.just_pressed(KeyCode::Enter) {
        scene.selected.filter(|&index| index < entries.len())
    } else {
        mouse
            .just_pressed(MouseButton::Left)
            .then_some(hovered)
            .flatten()
    };
    if let Some(index) = activation {
        scene
            .feedback_actions
            .push(crate::frontend_feedback::Cue::Activate);
        if let Screen::Controls(unit) = scene.screen {
            let count = scene.data.control_pages[unit].bindings.len();
            if index < count {
                scene.binding_target = Some(index);
                scene.selected = Some(index);
                scene.notice = None;
            } else if index == count { /* slider handled above */
            } else if index == count + 1 {
                scene.controls.2 = !scene.controls.2;
                scene.notice = None;
            } else if index == count + 2 {
                scene.controls.reset(unit);
                scene.notice = None;
            } else {
                scene.leave_controls(unit);
                return;
            }
            scene.revision += 1;
            return;
        }
        if scene.screen == Screen::Resolution {
            let count = scene.display.visible().len();
            if index < count {
                let mode = scene.display.visible()[index];
                scene.display.choose(mode);
            } else {
                match index - count {
                    0 => scene.display.page(-1),
                    1 => scene.display.page(1),
                    2 => {
                        if let Ok(mut window) = windows.single_mut() {
                            let [width, height] = scene.display.pending;
                            window.resolution.set_physical_resolution(width, height);
                            scene.display.current = [width, height];
                            println!(
                                "frontend_window_size_requested={width}x{height} exclusive_fullscreen=false"
                            );
                        }
                    }
                    3 => scene.display.pending = scene.display.initial,
                    4 => {
                        scene.screen = Screen::Graphics;
                        scene.selected = Some(11);
                    }
                    _ => {}
                }
            }
            scene.notice = None;
            scene.revision += 1;
            return;
        }
        if scene.screen == Screen::Graphics {
            match index {
                0..=8 => scene.graphics.cycle(index, 1),
                10 => scene.graphics = GraphicsOptions::default(),
                11 => {
                    scene.screen = Screen::Resolution;
                    scene.display.pending = scene.display.current;
                    scene.selected = Some(scene.display.visible().len() + 4);
                    scene.notice = None;
                }
                12 => {
                    scene.leave_graphics();
                    return;
                }
                _ => {}
            }
            scene.revision += 1;
            return;
        }
        if scene.screen == Screen::Sound {
            match index {
                2 => {
                    scene.audio = AudioOptions::default();
                    scene.notice = None;
                    scene.revision += 1;
                }
                3 => scene.leave_sound(),
                _ => {}
            }
            return;
        }
        if scene.screen == Screen::PlayerIds {
            if index == scene.profiles.names.len() {
                scene.screen = Screen::NewPlayer;
                scene.draft.clear();
                scene.notice = None;
                scene.selected = None;
                println!("frontend_profile_edit=opened");
            } else {
                match scene.profiles.select(index) {
                    Ok(()) => {
                        scene.screen = Screen::Main;
                        scene.selected = None;
                        scene.notice = None;
                        println!("frontend_profile_selected={index}");
                    }
                    Err(error) => scene.notice = Some(error.to_string()),
                }
            }
            scene.revision += 1;
            return;
        }
        if scene.screen == Screen::DeletePlayer {
            if index == 0 {
                let target = scene
                    .delete_target
                    .expect("Deletion screen requires a target");
                match scene.profiles.delete(target) {
                    Ok(()) => {
                        scene.screen = Screen::PlayerIds;
                        scene.selected = Some(scene.profiles.active);
                        scene.delete_target = None;
                        scene.notice = None;
                        println!(
                            "frontend_profile_deleted=true count={}",
                            scene.profiles.names.len()
                        );
                    }
                    Err(error) => scene.notice = Some(error.to_string()),
                }
            } else {
                scene.screen = Screen::PlayerIds;
                scene.selected = scene.delete_target.take();
                scene.notice = None;
                println!("frontend_profile_delete=cancelled");
            }
            scene.revision += 1;
            return;
        }
        if scene.screen == Screen::Main && matches!(index, 0 | 4) {
            let profile = scene
                .profiles
                .names
                .get(scene.profiles.active)
                .cloned()
                .unwrap_or_default();
            if let Err(e) = scene.campaign.select_profile(&profile) {
                scene.notice = Some(e);
                scene.revision += 1;
                return;
            }
            if index == 4 {
                scene.screen = Screen::LoadWorlds;
                scene.selected = Some(0);
                scene.notice = if scene
                    .campaign
                    .available_worlds()
                    .is_ok_and(|w| w.is_empty())
                {
                    Some("No saved games. Start a new single player game.".into())
                } else {
                    None
                };
            } else {
                let request = scene.campaign.new_campaign();
                begin_mission(&mut scene, request, &mut images, LaunchIntent::NewCampaign);
            }
            scene.revision += 1;
            return;
        }
        if scene.screen == Screen::LoadWorlds {
            let count = scene
                .campaign
                .available_worlds()
                .map(|w| w.len())
                .unwrap_or(0);
            if index == count {
                scene.screen = Screen::Main;
                scene.selected = Some(4);
            } else {
                match scene.campaign.select_world(index) {
                    Ok(()) => {
                        let completion = if scene.completion_replay {
                            scene.campaign.completion_replays()
                        } else {
                            Ok(Vec::new())
                        };
                        match completion {
                            Ok(requests) => {
                                scene.completion_requests = requests;
                                scene.screen = Screen::LoadMissions;
                                scene.selected = Some(0);
                                scene.notice = if scene.completion_replay {
                                    Some(if scene.completion_requests.is_empty() {
                                        "No completion cinematic is declared for this world."
                                    } else {
                                        "Completion replay preview; no mission completion is recorded."
                                    }.into())
                                } else {
                                    None
                                };
                            }
                            Err(error) => scene.notice = Some(error),
                        }
                    }
                    Err(e) => scene.notice = Some(e),
                }
            }
            scene.revision += 1;
            return;
        }
        if scene.screen == Screen::LoadMissions {
            let count = if scene.completion_replay {
                scene.completion_requests.len()
            } else {
                scene.campaign.menu.as_ref().map_or(0, |m| m.entries.len())
            };
            if index == count {
                scene.screen = Screen::LoadWorlds;
                scene.selected = Some(0);
            } else {
                let intent = if scene.completion_replay {
                    LaunchIntent::CompletionReplay
                } else {
                    LaunchIntent::SavedMission
                };
                let request = if scene.completion_replay {
                    scene
                        .completion_requests
                        .get(index)
                        .cloned()
                        .ok_or("No completion replay at this index".into())
                } else {
                    scene.campaign.request(index)
                };
                begin_mission(&mut scene, request, &mut images, intent);
            }
            scene.revision += 1;
            return;
        }
        match Frontend::activate(scene.screen, index) {
            Activation::Transition(next) => {
                println!(
                    "frontend_transition={:?}->{next:?} index={index}",
                    scene.screen
                );
                let previous = scene.screen;
                scene.screen = next;
                if next == Screen::LoadWorlds {
                    scene.completion_replay = false;
                    scene.completion_requests.clear();
                }
                if let Screen::Controls(_) = next {
                    scene.controls = scene
                        .profiles
                        .controls
                        .get(scene.profiles.active)
                        .cloned()
                        .unwrap_or_default();
                    scene.binding_target = None;
                }
                if next == Screen::Graphics {
                    scene.graphics = scene
                        .profiles
                        .graphics
                        .get(scene.profiles.active)
                        .copied()
                        .unwrap_or_default();
                }
                if next == Screen::Sound {
                    scene.audio = scene
                        .profiles
                        .audio
                        .get(scene.profiles.active)
                        .copied()
                        .unwrap_or_default();
                }
                scene.selected = if next == Screen::ExitConfirmation {
                    Some(1) // Native page1e selects No (native row2) on entry.
                } else if next == Screen::Main && previous == Screen::ExitConfirmation {
                    Some(6)
                } else if next == Screen::PlayerIds {
                    Some(scene.profiles.active)
                } else if let Screen::Controls(unit) = next {
                    Some(scene.data.control_pages[unit].entries.len() - 1)
                } else if next == Screen::Graphics {
                    Some(12)
                } else if next == Screen::Sound {
                    Some(3)
                } else if next == Screen::Options {
                    Some(5)
                } else {
                    None
                };
                scene.last_cursor = point;
                scene.notice = None;
            }
            Activation::Quit => {
                println!("frontend_exit=confirmed");
                exit.write(AppExit::Success);
            }
            Activation::Unimplemented(_) => {
                scene.notice = Some("This action is outside the current inspection slice.".into());
                println!("frontend_request_unimplemented={index}");
            }
        }
        scene.revision += 1;
    }
}
#[allow(clippy::too_many_arguments)] // Bevy UI and material asset system parameters.
pub fn draw(
    mut commands: Commands,
    mut loading_materials: ResMut<Assets<crate::LoadingMaterial>>,
    scene: Res<Scene>,
    font: Res<InspectionFont>,
    fonts: Res<Assets<Font>>,
    old: Query<Entity, With<FrontendElement>>,
    windows: Query<&Window>,
    mut rendered: Local<Option<(u64, [u32; 2])>>,
) {
    // Wait for Parley registration before laying out centered children. Building
    // their parents while the asset has no alias can leave zero-width layout.
    if fonts.get(&font.0).is_none_or(|font| font.alias.is_empty()) {
        return;
    }
    let window = windows.single().ok();
    let viewport = window.map_or([1920, 1080], |window| {
        [
            window.resolution.physical_width(),
            window.resolution.physical_height(),
        ]
    });
    if *rendered == Some((scene.revision, viewport)) {
        return;
    }
    *rendered = Some((scene.revision, viewport));
    let width = window.map_or(1920., Window::width);
    let height = window.map_or(1080., Window::height);
    for entity in &old {
        commands.entity(entity).despawn();
    }
    // Inspection captures the staged body while native admission stays pending.
    // This hides only this diagnostic overlay; it emits no readiness event.
    if replay::initial_body_inspection_replay() && scene.flow.phase == Phase::AwaitingRuntime {
        return;
    }
    if scene.screen == Screen::MissionLoading
        && matches!(
            scene.flow.phase,
            Phase::Splash | Phase::Loading | Phase::AwaitingRuntime | Phase::Failed
        )
        && let Some(image) = &scene.splash
    {
        commands.spawn((
            FrontendElement,
            MaterialNode(loading_materials.add(crate::LoadingMaterial {
                image: image.clone(),
                uv_scale: Vec4::new(1., 1., 0., 0.),
            })),
            Node {
                position_type: PositionType::Absolute,
                left: percent(25.),
                top: percent(25.),
                width: percent(50.),
                height: percent(50.),
                ..default()
            },
        ));
    }
    let selected_label = scene
        .selected
        .and_then(|index| scene.entries().get(index).cloned());
    let mut label = |text: &str, baseline: f32, size: f32, color: Color, left: Option<f32>| {
        // Fixed text bounds avoid relying on intrinsic flex-child measurements
        // when the same string is removed and recreated during a menu transition.
        let mut entity = commands.spawn((
            FrontendElement,
            Node {
                position_type: PositionType::Absolute,
                left: px(left.unwrap_or(0.0) * width),
                top: px(height * (baseline - size * 15.0 / 24.0)),
                width: px(width * (1.0 - left.unwrap_or(0.0))),
                height: px(height * size * 2.0),
                ..default()
            },
            Text::new(text),
            TextFont::from_font_size(FontSize::Px(height * size)).with_font(font.0.clone()),
            TextColor(color),
            TextLayout::new(
                if left.is_none() {
                    Justify::Center
                } else {
                    Justify::Left
                },
                LineBreak::WordBoundary,
            ),
            LayoutConfig {
                use_rounding: false,
            },
        ));
        if selected_label
            .as_ref()
            .is_some_and(|entry| entry.text == text && entry.layout[1] == baseline)
        {
            entity.insert(crate::frontend_hover::SelectedLabel);
        }
    };
    if matches!(scene.screen, Screen::LoadWorlds | Screen::LoadMissions) {
        let key = if scene.screen == Screen::LoadWorlds {
            b"LoadWorld".as_slice()
        } else {
            b"LoadMission".as_slice()
        };
        let source_title = scene
            .campaign
            .text
            .get(key)
            .and_then(|v| std::str::from_utf8(v).ok())
            .unwrap_or("Select mission");
        let title = if scene.completion_replay {
            if scene.screen == Screen::LoadWorlds {
                "Select world for cinematic replay"
            } else {
                "Select completion cinematic"
            }
        } else {
            source_title
        };
        label(title, 0.07, 0.04, Color::WHITE, None);
    }
    if scene.screen == Screen::MissionLoading {
        if let Some(request) = &scene.load_request
            && matches!(
                scene.flow.phase,
                Phase::Splash | Phase::Loading | Phase::AwaitingRuntime | Phase::Failed
            )
        {
            let caption = request
                .loading_caption(&scene.campaign.text)
                .ok()
                .and_then(|v| String::from_utf8(v).ok())
                .unwrap_or_default();
            label(&caption, 0.79, 0.028, Color::WHITE, None);
        }
        let status = match scene.flow.phase {
            Phase::Failed => "This mission could not be loaded. Press Esc to return.",
            Phase::AwaitingRuntime => "Loading...",
            _ => "",
        };
        label(status, 0.88, 0.024, Color::WHITE, None);
        if scene.flow.phase == Phase::Objectives
            && let Some(request) = &scene.load_request
        {
            let title = scene.campaign.mission_title(&request.mission_name);
            label(&title, 0.2, 0.035, Color::WHITE, None);
            let key = format!("MO_{}", request.mission_name);
            let objective = scene
                .campaign
                .text
                .get(key.as_bytes())
                .and_then(|v| std::str::from_utf8(v).ok())
                .unwrap_or("");
            let mut chars = objective.chars();
            let mut plain = String::new();
            while let Some(c) = chars.next() {
                match c {
                    '^' => plain.push('\n'),
                    '~' => {
                        chars.next();
                    }
                    _ => plain.push(c),
                }
            }
            label(&plain, 0.3, 0.025, Color::WHITE, Some(0.18));
        }
    }
    if scene.screen == Screen::PlayerIds {
        label(&scene.data.title, 0.0541848, 0.0425656, Color::WHITE, None);
    }
    if scene.screen == Screen::Options {
        label(
            &scene.data.options_title,
            0.09659953,
            0.0426218,
            Color::WHITE,
            None,
        );
    }
    if scene.screen == Screen::ExitConfirmation {
        label(
            &scene.data.exit_title,
            0.17884363,
            0.04262192,
            Color::WHITE,
            Some(0.42425716),
        );
    }
    if scene.screen == Screen::DeletePlayer {
        label(
            &scene.data.delete_title,
            0.17884365,
            0.0426219,
            Color::WHITE,
            None,
        );
        if let Some(name) = scene
            .delete_target
            .and_then(|i| scene.profiles.names.get(i))
        {
            label(name, 0.1235324, 0.0425653, Color::WHITE, None);
        }
    }
    if let Screen::Controls(unit) = scene.screen {
        let page = &scene.data.control_pages[unit];
        label(&page.title, 0.04, 0.0333333, Color::WHITE, None);
        for (row, (initial, [x, y, size])) in page.bindings.iter().enumerate() {
            let physical = scene.controls.0[unit].get(&(row as u8)).unwrap_or(initial);
            label(&binding_label(physical), *y, *size, Color::WHITE, Some(*x));
        }
        label("<", page.slider_y, 0.0333333, Color::WHITE, Some(0.068));
        label(">", page.slider_y, 0.0333333, Color::WHITE, Some(0.27));
    }
    if scene.screen == Screen::Resolution {
        for (index, x, y) in [
            (0, 0.4369603, 0.0475836),
            (1, 0.4199212, 0.1493016),
            (2, 0.4218744, 0.744836),
            (3, 0.1984767, 0.3505196),
            (4, 0.7048073, 0.349995),
        ] {
            label(
                &scene.data.resolution_text[index],
                y,
                0.0333333,
                Color::WHITE,
                Some(x),
            );
        }
        for (mode, y) in [
            (scene.display.current, 0.2314038),
            (scene.display.pending, 0.8269381),
        ] {
            label(
                &format!("{}x{}", mode[0], mode[1]),
                y,
                0.0333333,
                Color::WHITE,
                Some(0.0881161),
            );
        }
        for (x, y) in [
            (0.0881161, 0.1954114),
            (0.0881161, 0.7909456),
            (0.0194052, 0.4671156),
        ] {
            label(&scene.display_adapter, y, 0.0333333, Color::WHITE, Some(x));
        }
    }
    if scene.screen == Screen::Graphics {
        label(
            &scene.data.graphics_title,
            0.09659953,
            0.0426218,
            Color::WHITE,
            None,
        );
        for (row, &value) in scene.graphics.0[..9].iter().enumerate() {
            let values = &scene.data.graphics_values;
            let text = match row {
                0 => &values[[2, 3, 4][usize::from(value)]],
                3 => &values[[0, 2, 4][usize::from(value)]],
                4 => &values[[2, 5, 4][usize::from(value)]],
                _ => &values[usize::from(value)],
            };
            label(
                text,
                scene.data.graphics[row].layout[1],
                0.0426218,
                Color::srgb(0.6, 0.6, 0.6),
                Some(0.5704051),
            );
        }
        label("<", 0.7340321, 0.0426218, Color::WHITE, Some(0.572));
        label(">", 0.7340321, 0.0426218, Color::WHITE, Some(0.840));
    }
    if scene.screen == Screen::Sound {
        label(
            &scene.data.sound_title,
            0.09659953,
            0.0426218,
            Color::WHITE,
            None,
        );
        for entry in &scene.data.sound_unavailable {
            label(
                &entry.text,
                entry.layout[1],
                entry.layout[2],
                Color::srgb(0.6, 0.6, 0.6),
                Some(entry.layout[0]),
            );
            label(
                &scene.data.unavailable,
                entry.layout[1] - 0.00546,
                entry.layout[2],
                Color::srgb(0.2, 0.2, 0.2),
                Some(0.60213214),
            );
        }
        for y in [0.43970147, 0.4965251] {
            label("<", y, 0.0426218, Color::WHITE, Some(0.557));
            label(">", y, 0.0426218, Color::WHITE, Some(0.756));
        }
    }
    if scene.screen == Screen::NewPlayer {
        label(
            &scene.data.new_title,
            0.0541848,
            0.0425656,
            Color::WHITE,
            None,
        );
        label(
            &format!("{}|", scene.draft),
            0.11328125,
            0.03125,
            Color::srgb(1.0, 1.0, 0.0),
            Some(0.34765625),
        );
    }
    let entries = scene.entries();
    for (index, entry) in entries.iter().enumerate() {
        label(
            &entry.text,
            entry.layout[1],
            entry.layout[2],
            if scene.selected == Some(index) {
                Color::WHITE
            } else {
                Color::srgb(0.6, 0.6, 0.6)
            },
            if matches!(
                scene.screen,
                Screen::Options
                    | Screen::ExitConfirmation
                    | Screen::DeletePlayer
                    | Screen::Sound
                    | Screen::Graphics
                    | Screen::Resolution
                    | Screen::Controls(_)
            ) {
                Some(entry.layout[0])
            } else {
                None
            },
        );
    }
    let help = scene
        .notice
        .as_deref()
        .or_else(|| {
            if scene.binding_target.is_some() {
                Some(scene.data.mapping_help.as_str())
            } else if scene.screen == Screen::NewPlayer {
                Some(scene.data.new_help.as_str())
            } else {
                scene
                    .selected
                    .and_then(|i| entries.get(i))
                    .map(|e| e.help.as_str())
            }
        })
        .unwrap_or("");
    label(help, 0.9808333, 0.0333333, Color::WHITE, Some(0.01));
    if matches!(
        scene.screen,
        Screen::Sound | Screen::Graphics | Screen::Controls(_)
    ) {
        // Native screenshot measurements; texture/quad geometry parity remains open.
        let sliders = if scene.screen == Screen::Sound {
            vec![
                (scene.audio.effects, 0.43970147, 0.5833, 0.1667),
                (scene.audio.music, 0.4965251, 0.5833, 0.1667),
            ]
        } else if let Screen::Controls(unit) = scene.screen {
            vec![(
                scene.controls.1,
                scene.data.control_pages[unit].slider_y,
                0.095,
                0.168,
            )]
        } else {
            vec![(scene.graphics.distance_fraction(), 0.7340321, 0.6, 0.2333)]
        };
        for (gain, y, left, length) in sliders {
            for (x, bar_width, bar_height, color) in [
                (left, length, 0.003, Color::srgb(0.35, 0.45, 0.4)),
                (
                    left + gain * length - 0.007,
                    0.014,
                    0.022,
                    Color::srgb(0.7, 0.9, 0.8),
                ),
            ] {
                commands.spawn((
                    FrontendElement,
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(x * width),
                        top: px((y - 0.009 - bar_height * 0.5) * height),
                        width: px(bar_width * width),
                        height: px(bar_height * height),
                        ..default()
                    },
                    BackgroundColor(color),
                ));
            }
        }
    }
}

/// Enumerate the actual Bevy monitor inventory once; never copy the oracle's hardware list.
pub fn display_inventory(
    mut scene: ResMut<Scene>,
    windows: Query<&Window>,
    monitors: Query<(&bevy::window::Monitor, Has<bevy::window::PrimaryMonitor>)>,
    adapter: Option<Res<bevy::render::renderer::RenderAdapterInfo>>,
) {
    if !scene.display.modes.is_empty() {
        return;
    }
    let Some((monitor, _)) = monitors
        .iter()
        .find(|(_, primary)| *primary)
        .or_else(|| monitors.iter().next())
    else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };
    let modes = monitor
        .video_modes
        .iter()
        .map(|mode| mode.physical_size.to_array())
        .filter(|[width, height]| {
            *width >= 640
                && *height >= 480
                && *width <= monitor.physical_width
                && *height <= monitor.physical_height
        });
    scene.display = DisplayOptions::new(
        modes,
        [
            window.resolution.physical_width(),
            window.resolution.physical_height(),
        ],
    );
    scene.display_adapter = adapter
        .map(|info| format!("{} ({:?})", info.name, info.backend))
        .unwrap_or_else(|| "Bevy display".into());
    scene.revision += 1;
}

fn binding_label(physical: &str) -> String {
    if let Some(key) = physical
        .strip_prefix("Key")
        .or_else(|| physical.strip_prefix("Digit"))
    {
        return key.into();
    }
    match physical {
        "" => "---",
        "MouseLeft" => "Left Mouse Button",
        "MouseRight" => "Right Mouse Button",
        "MouseMiddle" => "Middle Mouse Button",
        "ShiftLeft" => "Left Shift",
        "ShiftRight" => "Right Shift",
        "ControlLeft" => "Left Control",
        "ControlRight" => "Right Control",
        "AltLeft" => "Left Alt",
        "AltRight" => "Right Alt",
        "Space" => "Space Bar",
        "Backspace" => "Back Space",
        "PageUp" => "Page Up",
        "PageDown" => "Page Down",
        other => other.strip_prefix("Arrow").unwrap_or(other),
    }
    .into()
}
