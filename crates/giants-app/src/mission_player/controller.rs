//! Selected-mission movement and animation, gated by real gameplay ownership.
//! The explicit internal replay exercises the same controller while staging;
//! it does not advance the campaign flow or synthesize readiness messages.
use super::*;
use bevy::input::mouse::AccumulatedMouseMotion;
type PlayerDraw<'a> = (
    &'a Mesh3d,
    &'a mut Visibility,
    Option<&'a DrawnPlayer>,
    Option<&'a HeldPart>,
    Option<&'a super::DrawnArms>,
);

#[derive(Component)]
pub(crate) struct Controller {
    ticket: u64,
    audio_active: bool,
    audio_special: i32,
    state: crate::mecc_dev_scene::State,
    remainder: f64,
    flight_audio_dt: f32,
    boost_press: crate::mecc_input::ActionPress,
    fire_press: crate::mecc_input::ActionPress,
    steps: u32,
    replay: bool,
    staging_replay: bool,
    fire_replay: bool,
    finish_step: u32,
    pub(crate) replay_done: bool,
    was_active: bool,
    owned_motion_failed: bool,
    world_signals_failed: bool,
    npc_failed: bool,
    ripper_failed: bool,
    // Input-fixture memory only: release RMB long enough to recharge, as a
    // player would. Never changes fuel, actor pose or the ordinary jet service.
    probe_recharge: bool,
    probe_route: Option<route_probe::Route>,
    owned_backend: bool,
    owned_audio_backend: bool,
    owned_jet_probe: bool,
    normal_jet: Option<Result<super::jet::Presentation, String>>,
    normal_jet_frames: Vec<giants_game::mecc_jet_presentation::Frame>,
    owned_replay: bool,
    descent_replay: bool,
    descent_capture: bool,
    landing_failed: bool,
    landing_capture: bool,
    owned_draw_reported: bool,
    owned_input: giants_game::player_input_channels::State,
    pending_mouse: Vec2,
    initial: [f32; 3],
    minimum_fuel: f32,
    maximum_height: f32,
    animated: bool,
    initial_weapon: Option<[f32; 16]>,
    weapon_changed: bool,
    subjective: giants_game::subjective_player::Playback,
    subjective_error: bool,
    subjective_last_slot: Option<u8>,
    subjective_updates: u32,
    subjective_draw_visible: Option<bool>,
}
impl Controller {
    fn refresh_contacts(
        &mut self,
        runtime: &mut crate::mission_runtime::Runtime,
    ) -> Result<(), String> {
        let mut contacts = Vec::new();
        if !self.npc_failed {
            match runtime.npc_contact_snapshots(self.ticket) {
                Ok(vimps) => contacts.extend(vimps),
                Err(error) => {
                    self.npc_failed = true;
                    error!(
                        "mission_vimp_contacts_refused ticket={} error={error} fallback=false",
                        self.ticket
                    );
                }
            }
        }
        if !self.ripper_failed {
            match runtime.ripper_contact_snapshots(self.ticket) {
                Ok(rippers) => contacts.extend(rippers),
                Err(error) => {
                    self.ripper_failed = true;
                    error!(
                        "mission_ripper_contacts_refused ticket={} error={error} fallback=false",
                        self.ticket
                    );
                }
            }
        }
        // A quarantined family contributes no stale shapes. The other family's
        // real completed submissions remain independent of that failure.
        let contacts = runtime.order_npc_contacts(self.ticket, contacts)?;
        self.state.set_mission_contacts(self.ticket, contacts)
    }

    fn campaign_probe() -> bool {
        Self::meat_probe() && std::env::var("GIANTS_MISSION_CAMPAIGN_PROBE").as_deref() == Ok("1")
    }
    fn meat_probe() -> bool {
        crate::frontend_scene::vimp_interaction_probe()
            && std::env::var("GIANTS_MISSION_MEAT_PROBE").as_deref() == Ok("1")
    }
    fn vimp_contact_probe() -> bool {
        crate::frontend_scene::vimp_interaction_probe()
            && std::env::var("GIANTS_MISSION_VIMP_CONTACT_PROBE").as_deref() == Ok("1")
    }
    pub(crate) fn vimp_contact_capture_done(&self) -> bool {
        Self::vimp_contact_probe()
            && self.replay_done
            && self.state.mission_npc_contact_responses() > 0
    }
    pub(crate) fn vimp_death_capture_done(&self) -> bool {
        crate::frontend_scene::vimp_interaction_probe()
            && std::env::var("GIANTS_MISSION_VIMP_DEATH_CAPTURE").as_deref() == Ok("1")
            && self.replay_done
            && self.finish_step < 6000
    }
    pub(crate) fn jet_clock_source(&self) -> &'static str {
        if self.owned_backend {
            "owned_world"
        } else {
            "ordinary_presentation"
        }
    }
    pub(crate) fn jet_frames(
        &self,
    ) -> Option<(u64, u64, &[giants_game::mecc_jet_presentation::Frame])> {
        if !self.was_active {
            return None;
        }
        let frames = if self.owned_backend {
            self.state.owned_jet_frames()?
        } else {
            self.normal_jet.as_ref()?.as_ref().ok()?;
            self.normal_jet_frames.as_slice()
        };
        (!frames.is_empty()).then_some((self.ticket, u64::from(self.steps), frames))
    }
    fn update_normal_jet(&mut self, runtime: &mut crate::mission_runtime::Runtime) {
        if self.owned_motion_failed {
            self.normal_jet_frames.clear();
            return;
        }
        if matches!(self.normal_jet, Some(Err(_))) {
            return;
        }
        let result = (|| {
            let (session, owner) = runtime.controlled_jet_session_mut(self.ticket)?;
            if self.normal_jet.is_none() {
                self.normal_jet = Some(super::jet::Presentation::new(session, owner));
            }
            let producer = self
                .normal_jet
                .as_mut()
                .unwrap()
                .as_mut()
                .map_err(|error| error.clone())?;
            producer.advance(
                session,
                u64::from(self.steps) + 1,
                1. / 120.,
                self.state.normal_jet_parent(),
            )
        })();
        match result {
            Ok(frames) => self.normal_jet_frames = frames,
            Err(error) => {
                eprintln!(
                    "mission_normal_jet_refused ticket={} reason={error:?}",
                    self.ticket
                );
                self.normal_jet = Some(Err(error));
                self.normal_jet_frames.clear();
            }
        }
    }
    fn update_subjective(
        &mut self,
        player: &giants_game::player_initial::Prepared,
        runtime: &crate::mission_runtime::Runtime,
        dt: f32,
        held: bool,
    ) {
        let (camera, scale, motion) = self.state.subjective_inputs();
        if !self.state.first_person_view() || !self.state.alive() {
            self.subjective.observe(camera);
            self.state.set_subjective_weapon(None);
            return;
        }
        let result = (|| -> Result<(), String> {
            let resources = player.subjective.as_ref().map_err(Clone::clone)?;
            let spec = runtime.player_state_specification(self.ticket, 0x70)?;
            self.subjective.step(
                resources,
                spec,
                &player.weapon,
                player.equipment.weapons[0],
                camera,
                scale,
                motion,
                held,
                dt,
            )
        })();
        match result {
            Ok(()) => {
                self.subjective_updates += 1;
                let frame = self.subjective.frame.as_ref().unwrap();
                self.state.set_subjective_weapon(Some(frame.weapon_matrix));
                if self.subjective_last_slot != Some(frame.slot) {
                    println!(
                        "mission_subjective_pose ticket={} slot={} tick={} seconds={} updates={} weapon_reference=1 corrections_applied=true unblended=true input_flag_phase_verified=false actor_backend_ready=false",
                        self.ticket, frame.slot, frame.tick, frame.seconds, self.subjective_updates
                    );
                    self.subjective_last_slot = Some(frame.slot);
                }
            }
            Err(error) => {
                self.subjective.frame = None;
                self.state.set_subjective_weapon(None);
                if !self.subjective_error {
                    println!(
                        "mission_subjective_refused ticket={} reason={error:?} actor_backend_ready=false",
                        self.ticket
                    );
                    self.subjective_error = true;
                }
            }
        }
    }
    pub(crate) fn actor_audio_position(&self) -> [f32; 3] {
        self.state.mission_snapshot().0
    }
    pub(crate) fn set_carried_load(&mut self, carried: bool) -> Result<(), &'static str> {
        if let Some([mass, gravity]) = self.state.set_carried_load(carried)? {
            println!(
                "mission_carried_load ticket={} enabled={carried} mass_bits={} gravity_bits={} source=owned_pickup_state actor_backend_ready=false",
                self.ticket,
                mass.to_bits(),
                gravity.to_bits()
            );
        }
        Ok(())
    }
    pub(crate) fn carried_matrix(
        &self,
        actor: &giants_game::carried_actor::Prepared,
        child_seconds: f32,
    ) -> Result<[f32; 16], Box<dyn std::error::Error>> {
        self.state.carried_matrix(actor, child_seconds)
    }
    pub(crate) fn carried_matrix_from_reference(
        &self,
        actor: &giants_game::carried_actor::Prepared,
        child: giants_game::procedural_aim::Matrix,
    ) -> Result<[f32; 16], Box<dyn std::error::Error>> {
        self.state.carried_matrix_from_reference(actor, child)
    }
    pub(crate) fn flight_audio_replay_delta(&self) -> Option<f32> {
        self.replay.then_some(self.flight_audio_dt)
    }
    pub(crate) fn audio_listener(&self) -> [f32; 3] {
        self.state.camera().1
    }
    pub(crate) fn audio_view(&self) -> [f32; 16] {
        self.state.camera().0.view
    }
    pub(crate) fn pistol(&self) -> Option<&giants_game::mecc_pistol::Pistol> {
        self.state.mission_pistol.as_ref()
    }
    pub(crate) fn hud(&self) -> (f32, f32, f32) {
        self.state.mission_hud()
    }
    pub(crate) fn message_time(&self) -> f32 {
        self.state.mission_snapshot().3
    }
    pub(crate) fn prepared(&self, ticket: u64) -> bool {
        let (position, fuel, _, seconds, _) = self.state.mission_snapshot();
        self.ticket == ticket
            && position.into_iter().all(f32::is_finite)
            && fuel.is_finite()
            && seconds.is_finite()
    }
    pub(crate) fn diagnostic_motion_complete(&self) -> bool {
        self.replay_done
    }
    pub(crate) fn new(ticket: u64, state: crate::mecc_dev_scene::State) -> Self {
        let initial = state.mission_snapshot().0;
        Self {
            normal_jet: None,
            normal_jet_frames: Vec::new(),
            owned_jet_probe: std::env::var("GIANTS_MISSION_OWNED_JET_PROBE").as_deref() == Ok("1"),
            owned_audio_backend: std::env::var("GIANTS_MISSION_MECC_AUDIO").as_deref()
                == Ok("reserved"),
            owned_replay: crate::frontend_scene::owned_inspection_replay(),
            descent_replay: std::env::var("GIANTS_MISSION_REPLAY").as_deref()
                == Ok("owned-player-descent-inspection"),
            owned_draw_reported: false,
            owned_backend: std::env::var("GIANTS_MISSION_MECC_BACKEND").as_deref()
                == Ok("owned-inspection"),
            owned_input: Default::default(),
            pending_mouse: Vec2::ZERO,
            ticket,
            audio_active: false,
            audio_special: -1,
            state,
            remainder: 0.,
            flight_audio_dt: 0.,
            boost_press: Default::default(),
            fire_press: Default::default(),
            steps: 0,
            replay: matches!(
                std::env::var("GIANTS_MISSION_REPLAY").ok().as_deref(),
                Some(
                    "player-motion"
                        | "owned-player-descent-inspection"
                        | "owned-player-inspection"
                        | "owned-player-inspection-cycles"
                        | "player-motion-cycles"
                        | "player-motion-air"
                        | "campaign-entry"
                        | "campaign-entry-cycles"
                        | "campaign-fire"
                        | "campaign-fire-cycles"
                        | "campaign-audio-cancel-cycles"
                        | "campaign-audio-impact-cycles"
                        | "campaign-flight-cycles"
                        | "campaign-ambient-cycles"
                        | "campaign-fire-pose"
                        | "campaign-descent-pose"
                        | "campaign-landing-pose"
                        | "showcase-mecc"
                )
            ),
            staging_replay: matches!(
                std::env::var("GIANTS_MISSION_REPLAY").ok().as_deref(),
                Some(
                    "player-motion"
                        | "owned-player-descent-inspection"
                        | "player-motion-cycles"
                        | "player-motion-air"
                        | "owned-player-inspection"
                        | "owned-player-inspection-cycles"
                )
            ),
            fire_replay: matches!(
                std::env::var("GIANTS_MISSION_REPLAY").ok().as_deref(),
                Some(
                    "campaign-fire"
                        | "campaign-fire-cycles"
                        | "campaign-fire-pose"
                        | "campaign-audio-cancel-cycles"
                        | "campaign-audio-impact-cycles"
                        | "campaign-flight-cycles"
                        | "campaign-ambient-cycles"
                        | "showcase-mecc"
                )
            ),
            descent_capture: std::env::var("GIANTS_MISSION_REPLAY").as_deref()
                == Ok("campaign-descent-pose"),
            landing_failed: false,
            landing_capture: std::env::var("GIANTS_MISSION_REPLAY").as_deref()
                == Ok("campaign-landing-pose"),
            finish_step: if Self::campaign_probe() {
                120000
            } else if Self::vimp_contact_probe() {
                12000
            } else if crate::frontend_scene::vimp_interaction_probe() {
                6000
            } else if matches!(
                std::env::var("GIANTS_MISSION_REPLAY").as_deref(),
                Ok("campaign-descent-pose" | "campaign-landing-pose")
            ) {
                1600
            } else if std::env::var("GIANTS_MISSION_REPLAY").as_deref()
                == Ok("owned-player-descent-inspection")
            {
                145
            } else if crate::frontend_scene::owned_inspection_replay() {
                std::env::var("GIANTS_MISSION_OWNED_JET_CAPTURE_STEP")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(128)
            } else if std::env::var("GIANTS_MISSION_REPLAY").ok().as_deref()
                == Some("campaign-fire-pose")
            {
                64
            } else if std::env::var("GIANTS_MISSION_REPLAY")
                .ok()
                .is_some_and(|m| {
                    matches!(
                        m.as_str(),
                        "campaign-audio-cancel-cycles"
                            | "showcase-mecc"
                            | "campaign-audio-impact-cycles"
                            | "campaign-flight-cycles"
                            | "campaign-ambient-cycles"
                    )
                })
            {
                275
            } else if std::env::var("GIANTS_MISSION_REPLAY").ok().as_deref()
                == Some("player-motion-air")
            {
                480
            } else {
                720
            },
            replay_done: false,
            was_active: false,
            owned_motion_failed: false,
            world_signals_failed: false,
            npc_failed: false,
            ripper_failed: false,
            probe_recharge: false,
            probe_route: None,
            initial,
            minimum_fuel: 100.,
            maximum_height: initial[2],
            animated: false,
            initial_weapon: None,
            weapon_changed: false,
            subjective: Default::default(),
            subjective_error: false,
            subjective_last_slot: None,
            subjective_updates: 0,
            subjective_draw_visible: None,
        }
    }
}
#[derive(Component)]
pub(super) struct HeldPart {
    pub ticket: u64,
    pub index: usize,
    pub buffer: Handle<bevy::render::storage::ShaderBuffer>,
}
#[derive(bevy::ecs::system::SystemParam)]
pub(super) struct HostTime<'w> {
    simulation: Res<'w, Time>,
    real: Res<'w, Time<Real>>,
}
#[derive(bevy::ecs::system::SystemParam)]
pub(super) struct AudioRequests<'w, 's> {
    local: MessageWriter<'w, crate::mission_audio::Request>,
    flight: MessageWriter<'w, crate::mission_flight_audio::Born>,
    owned: crate::mission_audio::OwnedAudio<'w, 's>,
}
#[allow(clippy::too_many_arguments)]
pub(super) fn update(
    mut controllers: Query<(&PlayerInitial, &mut Controller)>,
    mut runtime: ResMut<Runtime>,
    frontend: Res<crate::frontend_scene::Scene>,
    host_time: HostTime,
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    mouse: Res<AccumulatedMouseMotion>,
    mut windows: Query<(&Window, &mut bevy::window::CursorOptions)>,
    scene: Option<ResMut<crate::world_scene::Scene>>,
    mut environment: ResMut<crate::mission_environment::State>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: crate::world_scene::SceneMaterials,
    mut cursor_owner: Local<Option<u64>>,
    mut audio: AudioRequests,
    showcase: Res<crate::mission_showcase::State>,
    mut draws: Query<PlayerDraw<'_>>,
) {
    use crate::mecc_input::{self, Action};
    let HostTime {
        simulation: time,
        real: real_time,
    } = host_time;
    let cursor_ticket = runtime.active_ticket().filter(|ticket| {
        *ticket == frontend.flow.ticket
            && showcase.controls(frontend.flow.phase)
            && scene.is_some()
            && !showcase.is_free()
    });
    if cursor_owner.is_some() && *cursor_owner != cursor_ticket {
        for (_, mut cursor) in &mut windows {
            cursor.visible = true;
            cursor.grab_mode = bevy::window::CursorGrabMode::None;
        }
        *cursor_owner = None;
    }
    let Some(mut scene) = scene else {
        return;
    };
    for (prepared, mut controller) in &mut controllers {
        if Some(controller.ticket) != runtime.active_ticket()
            || controller.ticket != frontend.flow.ticket
        {
            continue;
        }
        controller.flight_audio_dt = 0.;
        let preview =
            controller.staging_replay && scene.camera_source == "mission_player_inspection";
        let replay_running = controller.replay
            && (preview || controller.was_active || showcase.controls(frontend.flow.phase));
        let active = showcase.controls(frontend.flow.phase) || replay_running;
        if !active {
            controller.fire_press.clear();
            controller.boost_press.clear();
            controller.pending_mouse = Vec2::ZERO;
            // Subjective geometry belongs to the controlled view, including
            // the held weapon. Do not leave it floating during free inspection.
            for (_, mut visibility, _, gun, arms) in &mut draws {
                if arms.is_some_and(|p| p.ticket == controller.ticket)
                    || controller.state.first_person_view()
                        && gun.is_some_and(|p| p.ticket == controller.ticket)
                {
                    *visibility = Visibility::Hidden;
                }
            }
            controller.state.set_subjective_weapon(None);
            controller.subjective_draw_visible = None;
            if controller.was_active {
                for (_, mut cursor) in &mut windows {
                    cursor.visible = true;
                    cursor.grab_mode = bevy::window::CursorGrabMode::None;
                }
                controller.was_active = false;
            }
            continue;
        }
        controller.was_active = true;
        if !controller.owned_backend
            && let Err(error) = controller.refresh_contacts(&mut runtime)
        {
            controller.npc_failed = true;
            controller.ripper_failed = true;
            error!(
                "mission_npc_contact_registry_refused ticket={} error={error} fallback=false",
                controller.ticket
            );
        }
        if !controller.world_signals_failed
            && let Err(error) = runtime
                .sample_world_clock(controller.ticket, real_time.elapsed().as_millis() as u32)
        {
            controller.world_signals_failed = true;
            error!(
                "mission_world_clock_refused ticket={} error={error}",
                controller.ticket
            );
        }
        let focused = windows.iter().any(|(w, _)| w.focused);
        let look_ready = windows
            .iter()
            .any(|(w, c)| w.focused && c.grab_mode == bevy::window::CursorGrabMode::Locked);
        let controls = Some(frontend.mecc_controls());
        let ordinary_input = focused && !replay_running && !controller.owned_backend;
        controller
            .fire_press
            .sample(controls, Action::Fire, &keys, &buttons, ordinary_input);
        let held =
            |action| focused && mecc_input::pressed(controls, action, &keys, &buttons, false);
        let mut axes = if focused {
            mecc_input::movement_axes(controls, &keys, &buttons)
        } else {
            Vec2::ZERO
        };
        let mut thrust = held(Action::Thrust);
        controller.boost_press.sample(
            controls,
            Action::Boost,
            &keys,
            &buttons,
            focused && !replay_running,
        );
        if !replay_running {
            *cursor_owner = Some(controller.ticket);
            let vertical = if controls.is_some_and(|o| o.2) {
                -1.
            } else {
                1.
            };
            let gain = mecc_input::mouse_gain(controls);
            let development_look_ready = look_ready && !controller.owned_backend;
            controller.state.mission_look(
                if development_look_ready {
                    Vec2::new(mouse.delta.x * gain, mouse.delta.y * gain * vertical)
                } else {
                    Vec2::ZERO
                },
                focused && mecc_input::pressed(controls, Action::View, &keys, &buttons, true),
            );
            for (window, mut cursor) in &mut windows {
                cursor.visible = !window.focused;
                cursor.grab_mode = if window.focused {
                    bevy::window::CursorGrabMode::Locked
                } else {
                    bevy::window::CursorGrabMode::None
                };
            }
        }
        if controller.owned_backend {
            if focused && look_ready {
                controller.pending_mouse += mouse.delta;
            } else {
                controller.pending_mouse = Vec2::ZERO;
            }
        }
        if controller.owned_backend
            && controller.owned_audio_backend
            && !audio.owned.ready(controller.ticket)
        {
            if !controller.owned_motion_failed
                && let Some(error) = audio.owned.failure(controller.ticket)
            {
                controller.owned_motion_failed = true;
                audio.owned.quarantine(controller.ticket);
                eprintln!(
                    "mission_owned_primary_refused ticket={} step={} reason={error:?} fallback=false ready=false",
                    controller.ticket,
                    controller.steps + 1
                );
            }
            // Waiting admits no simulation time or input edge and acknowledges
            // no voice. Raw mouse accumulation remains owned by this controller.
            continue;
        }
        controller.remainder += if replay_running {
            if controller.replay_done {
                0.
            } else if controller.owned_replay {
                1. / 60.
            } else if crate::frontend_scene::vimp_interaction_probe() {
                // Keep a many-actor/audio probe to one fixed step per render
                // host frame. Accelerating four steps ahead of asynchronous
                // sink creation stresses a different transport than normal input.
                1. / 120.
            } else {
                4. / 120.
            }
        } else {
            f64::from(time.delta_secs().min(0.1))
        };
        let flight_before = controller.steps;
        let mut owned_advanced_seconds = 0.;
        while (if controller.owned_backend {
            controller.remainder > 0.
        } else {
            controller.remainder >= 1. / 120.
        }) && !controller.replay_done
            && !(controller.owned_backend && controller.owned_motion_failed)
        {
            let tick = controller.steps;
            if replay_running {
                let probe = std::env::var("GIANTS_SUBJECTIVE_PROBE").ok();
                let toggle = match probe.as_deref() {
                    Some("first") => tick == 0,
                    Some("cycle" | "cycle-cancel") => [0, 80, 140, 210, 240].contains(&tick),
                    _ => false,
                };
                if toggle {
                    controller.state.mission_look(Vec2::ZERO, true);
                    println!(
                        "mission_subjective_view ticket={} tick={tick} first={} internal_probe=true os_input=false",
                        controller.ticket,
                        controller.state.first_person_view()
                    );
                }
            }
            if replay_running
                && !controller.owned_backend
                && tick == 230
                && std::env::var("GIANTS_MISSION_REPLAY").ok().as_deref()
                    == Some("campaign-flight-cycles")
            {
                controller.state.mission_look(Vec2::new(0., -1.6), false);
                println!(
                    "mission_flight_probe ticket={} aim=upward_for_final_shot supplied_controller=true",
                    controller.ticket
                );
            }
            if replay_running
                && !controller.owned_backend
                && tick == 40
                && matches!(
                    std::env::var("GIANTS_MISSION_REPLAY").ok().as_deref(),
                    Some(
                        "campaign-audio-impact-cycles"
                            | "campaign-flight-cycles"
                            | "campaign-ambient-cycles"
                    )
                )
            {
                controller.state.mission_look(Vec2::new(0., 0.8), false);
                println!(
                    "mission_impact_probe ticket={} aim=downward supplied_controller=true",
                    controller.ticket
                );
            }
            let mut boost = controller.boost_press.take(false);
            if replay_running {
                axes = if controller.owned_replay {
                    if (15..95).contains(&tick) {
                        Vec2::Y
                    } else {
                        Vec2::ZERO
                    }
                } else {
                    match tick {
                        60..=119 => Vec2::Y,
                        120..=179 => Vec2::ONE,
                        _ => Vec2::ZERO,
                    }
                };
                thrust = !controller.owned_replay && (240..360).contains(&tick);
                boost = !controller.owned_replay && tick == 270;
                if controller.descent_capture || controller.landing_capture {
                    // Exercise the same sustained flight/release as the installed
                    // body/jet control. The real selector must request clip41;
                    // this probe never writes an animation or native readiness.
                    axes = if (64..96).contains(&tick) {
                        Vec2::Y
                    } else {
                        Vec2::ZERO
                    };
                    thrust = (16..420).contains(&tick);
                    boost = tick == 32;
                }
                if controller.owned_replay && controller.owned_jet_probe {
                    axes = Vec2::ZERO;
                    thrust = (16..48).contains(&tick);
                    boost = tick == 32;
                }
            }
            if replay_running && crate::frontend_scene::vimp_interaction_probe() {
                axes = Vec2::ZERO;
                thrust = false;
                boost = false;
                let return_target = Controller::campaign_probe()
                    .then(|| runtime.campaign_return_target(controller.ticket))
                    .flatten();
                let meat_target = (Controller::meat_probe() && return_target.is_none())
                    .then(|| runtime.meat_target(controller.ticket))
                    .flatten();
                let route_target = return_target.map(|goal| {
                    let position = controller.state.mission_snapshot().0;
                    if controller.probe_route.is_none() {
                        controller.probe_route = Some(
                            route_probe::Route::prepare(
                                &scene.asset.terrain,
                                position,
                                goal,
                                scene.asset.config.slope_thresholds[0],
                            )
                            .or_else(|_| {
                                route_probe::Route::prepare(
                                    &scene.asset.terrain,
                                    position,
                                    goal,
                                    scene.asset.config.slope_thresholds[1],
                                )
                            })
                            .or_else(|_| {
                                route_probe::Route::prepare(
                                    &scene.asset.terrain,
                                    position,
                                    goal,
                                    0.35,
                                )
                            })
                            .expect("sampled terrain input route with jet-assisted slopes"),
                        );
                    }
                    controller
                        .probe_route
                        .as_mut()
                        .unwrap()
                        .next(position)
                        .unwrap_or(goal)
                });
                if let Some(target) = route_target
                    .or(meat_target)
                    .or_else(|| runtime.npc_target(controller.ticket))
                {
                    let position = controller.state.mission_snapshot().0;
                    let dx = target[0] - position[0];
                    let dy = target[1] - position[1];
                    let distance = dx.hypot(dy);
                    let camera = controller.state.subjective_inputs().0;
                    let yaw = camera.angles[0].to_radians();
                    let desired = dx.atan2(-dy);
                    let horizontal =
                        (target[0] - camera.position[0]).hypot(target[1] - camera.position[1]);
                    let pitch = (camera.position[2] - target[2] - 8.).atan2(horizontal.max(1.));
                    controller.state.mission_look(
                        Vec2::new(
                            (yaw - desired + std::f32::consts::PI)
                                .rem_euclid(std::f32::consts::TAU)
                                - std::f32::consts::PI,
                            pitch + camera.angles[1].to_radians(),
                        ),
                        false,
                    );
                    if (return_target.is_some() && target[2] - position[2] > 5.)
                        || distance
                            > if meat_target.is_some() || return_target.is_some() {
                                2.
                            } else if Controller::vimp_contact_probe() {
                                0.
                            } else {
                                60.
                            }
                    {
                        axes = Vec2::Y;
                        thrust = position[2]
                            < target[2]
                                + if Controller::campaign_probe() {
                                    32.
                                } else {
                                    80.
                                }
                            && controller.state.mission_snapshot().1 > 0.3
                            && (!Controller::vimp_contact_probe() || distance > 120.)
                            && meat_target.is_none();
                        boost = distance > 300. && tick % 360 == 0;
                        if Controller::campaign_probe() {
                            let fuel = controller.state.mission_snapshot().1;
                            if return_target.is_some() {
                                let commands = route_probe::commands(
                                    position,
                                    target,
                                    fuel,
                                    &mut controller.probe_recharge,
                                );
                                axes = Vec2::from_array(commands.0);
                                thrust = commands.1;
                                boost = commands.2;
                            } else {
                                if fuel < 10. {
                                    controller.probe_recharge = true;
                                }
                                if fuel >= 95. {
                                    controller.probe_recharge = false;
                                }
                                if controller.probe_recharge {
                                    thrust = false;
                                    boost = false;
                                }
                            }
                            if tick % 600 == 0 {
                                println!(
                                    "mission_campaign_navigation ticket={} step={tick} position={position:?} target={target:?} fuel={fuel} recharge={} axes={axes:?} thrust={thrust} boost={boost} return_target={} physical={:?} supplied_input_only=true",
                                    controller.ticket,
                                    controller.probe_recharge,
                                    return_target.is_some(),
                                    controller.state.mission_physical_snapshot()
                                );
                            }
                        }
                    }
                }
            }
            if controller.owned_backend {
                let seconds = controller.remainder.min(0.1) as f32;
                let delta = if controller.owned_replay && controller.owned_jet_probe {
                    Vec2::ZERO
                } else if controller.owned_replay && (15..95).contains(&tick) {
                    Vec2::new(2., -1.)
                } else {
                    std::mem::take(&mut controller.pending_mouse)
                };
                let result = (|| -> Result<(), String> {
                    if held(Action::Fire) {
                        return Err("owned inspection weapon callbacks are not bound".into());
                    }
                    let sample_mouse = controller.owned_replay || focused && look_ready;
                    let commands = mecc_input::owned_commands(
                        controls,
                        &mut controller.owned_input,
                        delta,
                        sample_mouse,
                        axes,
                        thrust,
                        boost,
                    )?;
                    let input = crate::mission_runtime::OwnedMeccInput {
                        root: frontend.data_root(),
                        number: u64::from(controller.steps) + 1,
                        commands,
                        seconds,
                        terrain: &scene.asset.terrain,
                        objects: controller.state.mission_colliders(),
                        materials: controller.state.mission_contact_materials(),
                        first_person: controller.state.first_person_view(),
                    };
                    let frame = if controller.owned_audio_backend {
                        audio.owned.with_backend(
                            controller.ticket,
                            frontend.feedback_volume(),
                            |backend| runtime.advance_owned_mecc(controller.ticket, input, backend),
                        )?
                    } else {
                        runtime.advance_owned_mecc(
                            controller.ticket,
                            input,
                            &mut crate::mission_audio::UnavailableDevice,
                        )?
                    };
                    controller.state.accept_owned_pose(frame)
                })();
                controller.remainder = 0.;
                match result {
                    Ok(()) => {
                        controller.steps += 1;
                        let (position, fuel, _, _, _) = controller.state.mission_snapshot();
                        controller.minimum_fuel = controller.minimum_fuel.min(fuel);
                        controller.maximum_height = controller.maximum_height.max(position[2]);
                        owned_advanced_seconds += seconds;
                        controller.update_subjective(&prepared.prepared, &runtime, seconds, false);
                        if (controller.owned_replay || controller.descent_replay)
                            && controller.steps == controller.finish_step
                        {
                            let position = controller.state.mission_snapshot().0;
                            let distance =
                                Vec3::from(position).distance(Vec3::from(controller.initial));
                            if controller.owned_jet_probe {
                                if controller.finish_step == 128 {
                                    assert!(
                                        controller.maximum_height > controller.initial[2] + 1.
                                            && controller.minimum_fuel < 99.,
                                        "owned jet inspection did not exercise lift and fuel use"
                                    );
                                } else {
                                    assert_eq!(
                                        controller.state.mission_audio_jet().0,
                                        controller.finish_step < 49,
                                        "jet snapshot command phase differs from owned thrust flag"
                                    );
                                }
                            } else {
                                assert!(
                                    distance > 1.,
                                    "owned inspection did not exercise movement"
                                );
                            }
                            if controller.descent_replay {
                                let clip = controller.state.mission_snapshot().2;
                                assert_eq!(
                                    clip, 41,
                                    "descent endpoint must retain the owned descent clip"
                                );
                                println!(
                                    "mission_owned_descent_snapshot ticket={} steps={} clip={clip} supplied_commands=true primary_owned=true contact_landing_validated=false ready=false",
                                    controller.ticket, controller.steps
                                );
                            }
                            controller.replay_done = true;
                            println!(
                                "mission_owned_inspection_finished ticket={} steps={} distance={distance} position_bits={:?} supplied_counters=true primary_owned=true ready=false",
                                controller.ticket,
                                controller.steps,
                                position.map(f32::to_bits)
                            );
                            if controller.owned_jet_probe {
                                println!(
                                    "mission_owned_jet_controller_{} ticket={} steps={} initial_height={} maximum_height={} minimum_fuel={} actual_controller=true supplied_commands=true ready=false",
                                    if controller.finish_step == 128 {
                                        "finished"
                                    } else {
                                        "snapshot"
                                    },
                                    controller.ticket,
                                    controller.steps,
                                    controller.initial[2],
                                    controller.maximum_height,
                                    controller.minimum_fuel
                                );
                            }
                        }
                    }
                    Err(error) => {
                        controller.owned_motion_failed = true;
                        if controller.owned_audio_backend {
                            audio.owned.quarantine(controller.ticket);
                        }
                        eprintln!(
                            "mission_owned_primary_refused ticket={} step={} reason={error:?} fallback=false ready=false",
                            controller.ticket,
                            controller.steps + 1
                        );
                    }
                }
                continue;
            }
            let motion = controller
                .state
                .step(1. / 120., axes, &scene.asset.terrain, thrust, boost)
                .expect("selected mission controller step failed");
            if !controller.owned_motion_failed
                && let Err(error) = runtime.publish_controlled_mecc_motion(
                    controller.ticket,
                    u64::from(controller.steps) + 1,
                    motion,
                )
            {
                controller.owned_motion_failed = true;
                eprintln!(
                    "mission_controlled_actor_motion_refused ticket={} step={} reason={error:?} actor_backend_ready=false",
                    controller.ticket,
                    controller.steps + 1
                );
            }
            controller.update_normal_jet(&mut runtime);
            if !controller.owned_motion_failed && !controller.npc_failed {
                match runtime.collect_normal_meat(
                    controller.ticket,
                    u64::from(controller.steps) + 1,
                    1. / 120.,
                    frontend.flow.phase == giants_game::mission_loading::Phase::Gameplay,
                ) {
                    Ok(events) => {
                        for event in events {
                            audio.local.write(crate::mission_audio::Request {
                                ticket: controller.ticket,
                                event,
                            });
                        }
                    }
                    Err(error) => {
                        controller.npc_failed = true;
                        error!(
                            "mission_meat_collection_refused ticket={} error={error}",
                            controller.ticket
                        );
                    }
                }
            }
            let landing = controller.state.take_landing_impact();
            if !controller.landing_failed
                && (landing.is_some() || runtime.normal_landing_has_work(controller.ticket))
            {
                let result = (|| -> Result<(), String> {
                    let local = runtime
                        .normal_landing_owner(controller.ticket)
                        .ok_or("landing source owner absent")?;
                    let cues = runtime.owned_impact_cues(controller.ticket)?;
                    if !audio.owned.impact_ready(controller.ticket, &cues) {
                        return Err("landing requires actual reserved impact sinks".into());
                    }
                    audio.owned.with_impact(
                        controller.ticket,
                        frontend.feedback_volume(),
                        controller.state.camera().0.view,
                        local,
                        |device| {
                            runtime.advance_normal_landing(
                                controller.ticket,
                                1. / 120.,
                                landing,
                                device,
                            )
                        },
                    )
                })();
                if let Err(error) = result {
                    controller.landing_failed = true;
                    error!(
                        "mission_landing_refused ticket={} step={} error={error} replay=false",
                        controller.ticket,
                        controller.steps + 1
                    );
                }
            }
            let fire = if replay_running && crate::frontend_scene::vimp_interaction_probe() {
                !Controller::vimp_contact_probe()
                    && !(Controller::campaign_probe()
                        && runtime.campaign_return_target(controller.ticket).is_some())
                    && !(Controller::meat_probe()
                        && runtime.meat_target(controller.ticket).is_some())
            } else if controller.fire_replay {
                (60..300).contains(&tick)
            } else {
                !replay_running && controller.fire_press.take(held(Action::Fire))
            };
            controller.update_subjective(&prepared.prepared, &runtime, 1. / 120., fire);
            let footsteps = controller.state.mission_audio_steps();
            if showcase.controls(frontend.flow.phase) {
                for cue in footsteps {
                    audio.local.write(crate::mission_audio::Request {
                        ticket: controller.ticket,
                        event: crate::mission_audio::Event::Footstep(cue),
                    });
                }
                let (active, boost_started, special) = controller.state.mission_audio_jet();
                if boost_started {
                    audio.local.write(crate::mission_audio::Request {
                        ticket: controller.ticket,
                        event: crate::mission_audio::Event::Boost,
                    });
                }
                if active != controller.audio_active || special != controller.audio_special {
                    audio.local.write(crate::mission_audio::Request {
                        ticket: controller.ticket,
                        event: crate::mission_audio::Event::Jet { active, special },
                    });
                    controller.audio_active = active;
                    controller.audio_special = special;
                }
                let before = controller.pistol().unwrap().cooldown;
                let result = controller
                    .state
                    .mission_fire_step(1. / 120., fire, &scene.asset.terrain)
                    .expect("selected mission fire failed");
                if result.fired {
                    if !controller.world_signals_failed
                        && let Err(error) = runtime.publish_player_noise(controller.ticket)
                    {
                        controller.world_signals_failed = true;
                        error!(
                            "mission_player_noise_refused ticket={} error={error}",
                            controller.ticket
                        );
                    }
                    audio.local.write(crate::mission_audio::Request {
                        ticket: controller.ticket,
                        event: crate::mission_audio::Event::Fire(
                            controller.pistol().unwrap().definition.id,
                        ),
                    });
                    let origin = controller
                        .state
                        .combat_pose()
                        .expect("selected mission muzzle")
                        .0;
                    let pistol = controller.pistol().unwrap();
                    audio.flight.write(crate::mission_flight_audio::Born {
                        ticket: controller.ticket,
                        serial: pistol.fired,
                        origin,
                    });
                    println!(
                        "mission_player_fire ticket={} tick={tick} shot={} interval_bits={} before_bits={} cooldown_bits={} held={fire} alive={} origin={:?} aim_profile=camera_center_500 effects_native=false",
                        controller.ticket,
                        pistol.fired,
                        pistol.definition.interval.to_bits(),
                        before.to_bits(),
                        pistol.cooldown.to_bits(),
                        controller.state.alive(),
                        origin
                    );
                }
                for hit in result.world_hits {
                    if let Some(owner) = controller.state.mission_npc_hit_owner(hit.object) {
                        if matches!(
                            controller.state.mission_npc_hit_class(hit.object),
                            Some(73 | 230)
                        ) {
                            error!(
                                "mission_ripper_hit_unavailable ticket={} owner={owner:?} projectile_serial={} reason=damage_consumer_not_connected damage_callback=false",
                                controller.ticket, hit.serial
                            );
                            continue;
                        }
                        let local = runtime
                            .normal_landing_owner(controller.ticket)
                            .ok_or_else(|| "NPC shooter absent".to_owned());
                        let response = local.and_then(|local| {
                            audio.owned.with_impact(
                                controller.ticket,
                                frontend.feedback_volume(),
                                controller.state.camera().0.view,
                                local,
                                |device| {
                                    runtime.hit_normal_vimp(
                                        controller.ticket,
                                        owner,
                                        hit,
                                        controller.state.subjective_inputs().0.position,
                                        device,
                                    )
                                },
                            )
                        });
                        match response {
                            Ok((dead, events)) => {
                                for event in events {
                                    audio.local.write(crate::mission_audio::Request {
                                        ticket: controller.ticket,
                                        event,
                                    });
                                }
                                if dead {
                                    if crate::frontend_scene::vimp_interaction_probe()
                                        && std::env::var("GIANTS_MISSION_VIMP_DEATH_CAPTURE")
                                            .as_deref()
                                            == Ok("1")
                                        && controller.finish_step == 6000
                                    {
                                        controller.finish_step = tick + 16;
                                        println!(
                                            "mission_vimp_death_capture_requested ticket={} finish_step={} screenshot_only=true normal_feature_enablement=false",
                                            controller.ticket, controller.finish_step
                                        );
                                    }
                                    match controller.refresh_contacts(&mut runtime) {
                                        Ok(()) => {}
                                        Err(e) => {
                                            controller.npc_failed = true;
                                            controller.ripper_failed = true;
                                            error!("NPC contact registry after Vimp death: {e}");
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                controller.npc_failed = true;
                                error!("Vimp hit refused: {e}");
                            }
                        }
                    }
                }
                for impact in result.impacts {
                    let (code, point) = match &impact {
                        giants_game::mecc_pistol::Impact::World(q) => (2, q.point),
                        giants_game::mecc_pistol::Impact::Ground(p) => (3, *p),
                    };
                    if let Some(cue) = controller.state.impact_sound(code, point) {
                        audio.local.write(crate::mission_audio::Request {
                            ticket: controller.ticket,
                            event: crate::mission_audio::Event::Impact(cue),
                        });
                    }
                    match impact {
                        giants_game::mecc_pistol::Impact::World(q) => println!(
                            "mission_player_projectile_hit ticket={} object={:?} point={:?} tag={} damage_callback=false",
                            controller.ticket, q.selected, q.point, q.tag
                        ),
                        giants_game::mecc_pistol::Impact::Ground(point) => println!(
                            "mission_player_projectile_ground ticket={} point={point:?} native_broad_phase=false",
                            controller.ticket
                        ),
                    }
                }
            }
            controller.steps += 1;
            controller.remainder -= 1. / 120.;
            let (position, fuel, clip, seconds, responses) = controller.state.mission_snapshot();
            assert!(position.into_iter().all(f32::is_finite) && fuel.is_finite());
            controller.minimum_fuel = controller.minimum_fuel.min(fuel);
            controller.maximum_height = controller.maximum_height.max(position[2]);
            controller.animated |= clip != 1 && seconds != 0.;
            if replay_running && [60, 240, 360, 480, 720].contains(&controller.steps) {
                let (carried, [mass, gravity]) = controller.state.carried_load();
                println!(
                    "mission_player_load_consumed ticket={} steps={} carried={carried} mass_bits={} gravity_bits={} actor_backend_ready=false",
                    controller.ticket,
                    controller.steps,
                    mass.to_bits(),
                    gravity.to_bits()
                );
                println!(
                    "mission_player_motion ticket={} steps={} position={position:?} fuel={fuel} clip={clip} animation_seconds={seconds} mesh_responses={responses} ready=false",
                    controller.ticket, controller.steps
                );
            }
            if replay_running && controller.descent_capture && controller.steps > 420 && clip == 41
            {
                println!(
                    "mission_player_descent_observed ticket={} step={} position={position:?} fuel={fuel} clip={clip} animation_seconds={seconds} supplied_commands=true forced_pose=false os_input=false",
                    controller.ticket, controller.steps
                );
                controller.finish_step = controller.steps;
            }
            if replay_running
                && controller.landing_capture
                && controller.steps > 420
                && runtime
                    .normal_landing_visible(controller.ticket)
                    .expect("landing diagnostic draw state refused")
            {
                println!(
                    "mission_player_landing_observed ticket={} step={} position={position:?} canonical_draw=true scripted_birth=false os_input=false",
                    controller.ticket, controller.steps
                );
                controller.finish_step = controller.steps;
            }
            if replay_running && controller.steps == controller.finish_step {
                assert!(
                    !controller.landing_capture
                        || runtime
                            .normal_landing_visible(controller.ticket)
                            .expect("landing diagnostic draw state refused"),
                    "ordinary sustained flight/release did not produce a visible canonical landing effect"
                );
                assert!(
                    !controller.descent_capture || clip == 41,
                    "ordinary sustained flight/release did not reach the acquired descent pose"
                );
                let distance = Vec3::from(position).distance(Vec3::from(controller.initial));
                if controller.fire_replay {
                    let pistol = controller.pistol().unwrap();
                    let expected = if controller.finish_step == 64 { 1 } else { 4 };
                    if Controller::vimp_contact_probe() {
                        assert_eq!(
                            pistol.fired, 0,
                            "contact probe must retain living collision owners"
                        );
                        let responses = controller.state.mission_npc_contact_responses();
                        assert!(
                            responses > 0,
                            "ordinary approach never contacted a Vimp collision model"
                        );
                        println!(
                            "mission_vimp_contact_input_probe_finished ticket={} responses={responses} supplied_controller=true forced_actor_pose=false os_input=false",
                            controller.ticket
                        );
                    } else if crate::frontend_scene::vimp_interaction_probe() {
                        if Controller::meat_probe() {
                            let (meat, chunks) = runtime
                                .meat_status(controller.ticket)
                                .expect("meat source state absent");
                            assert!(
                                chunks > 0 && meat > 0.,
                                "ordinary navigation did not collect source meat"
                            );
                            println!(
                                "mission_meat_input_probe_finished ticket={} carried={meat} chunks={chunks} supplied_controller=true forced_actor_pose=false os_input=false",
                                controller.ticket
                            );
                        }
                        assert!(
                            pistol.fired >= 5,
                            "Vimp input probe produced too few actual pistol shots"
                        );
                        println!(
                            "mission_vimp_input_probe_finished ticket={} supplied_controller=true forced_actor_pose=false os_input=false reactions={:?} player_contacts={}",
                            controller.ticket,
                            runtime.npc_reactions(controller.ticket),
                            controller.state.mission_npc_contact_responses()
                        );
                    } else {
                        assert_eq!(pistol.fired, expected, "held pistol cadence failed");
                    }
                    assert_eq!(pistol.fired, pistol.retired + pistol.shots.len() as u32);
                    println!(
                        "mission_player_fire_finished ticket={} shots={} retired={} active={} world_hits={} ground_hits={} initial_ammo_unchanged=true human_validation=false",
                        controller.ticket,
                        pistol.fired,
                        pistol.retired,
                        pistol.shots.len(),
                        pistol.world_hits,
                        pistol.ground_hits
                    );
                } else {
                    assert!(
                        distance > 1.
                            && controller.animated
                            && controller.minimum_fuel < 99.
                            && controller.maximum_height > controller.initial[2] + 2.
                            && controller.weapon_changed,
                        "mission player replay did not exercise movement, animation, jet and attachment"
                    );
                }
                controller.replay_done = true;
                controller.remainder = 0.;
                println!(
                    "mission_player_motion_finished ticket={} passed=true steps={} distance={distance} animation_changed={} minimum_fuel={} weapon_attachment_changed={} campaign_ready=false human_validation=false",
                    controller.ticket,
                    controller.steps,
                    controller.animated,
                    controller.minimum_fuel,
                    controller.weapon_changed
                );
            }
        }
        controller.flight_audio_dt = if controller.owned_backend {
            owned_advanced_seconds
        } else {
            (controller.steps - flight_before) as f32 / 120.
        };
        controller
            .state
            .sample_pose()
            .expect("selected mission player skinning failed");
        let matrix = controller
            .state
            .render_weapon_matrix()
            .expect("selected mission weapon attachment failed");
        if let Some(matrix) = matrix {
            if let Some(initial) = controller.initial_weapon {
                controller.weapon_changed |= initial
                    .iter()
                    .zip(matrix)
                    .any(|(a, b)| (*a - b).abs() > 0.00001);
            } else {
                controller.initial_weapon = Some(matrix);
            }
        }
        let (camera, eye) = controller.state.camera();
        scene.validation_camera = Some(camera.clone());
        scene.camera_origin = Some(eye);
        scene.camera_source = "mission_player_live";
        environment.follow(controller.ticket, eye);
        crate::world_scene::apply_camera(&scene, &mut materials);
        let mut visible_arms = 0usize;
        let mut owned_body_parts = 0usize;
        for (handle, mut visibility, body, gun, arms) in &mut draws {
            if let Some(body) = body.filter(|p| p.ticket == controller.ticket) {
                if let Some(positions) = controller.state.expanded_draw_model(body.slot, body.part)
                    && let Some(mut mesh) = meshes.get_mut(&handle.0)
                {
                    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
                    owned_body_parts += 1;
                }
                *visibility = if controller.state.first_person_view()
                    || controller.state.body_draw_slot() != Some(body.slot)
                {
                    Visibility::Hidden
                } else {
                    Visibility::Visible
                };
            }
            if let Some(gun) = gun.filter(|p| p.ticket == controller.ticket) {
                let Some(matrix) = matrix else {
                    *visibility = Visibility::Hidden;
                    continue;
                };
                if let Some(positions) = controller.state.world_weapon_positions()
                    && let Some(mut mesh) = meshes.get_mut(&handle.0)
                {
                    let source = &controller.state.weapon.model.mesh;
                    let positions = if controller.state.first_person_view() {
                        source.positions.as_slice()
                    } else {
                        positions
                    };
                    let normals =
                        giants_game::body_pipeline::prepare_normals(source, positions, false)
                            .expect("owned pistol normals");
                    let ids = source.subobjects[gun.index]
                        .triangles
                        .iter()
                        .flatten()
                        .map(|id| usize::from(*id))
                        .collect::<Vec<_>>();
                    mesh.insert_attribute(
                        Mesh::ATTRIBUTE_POSITION,
                        ids.iter()
                            .map(|id| positions[usize::from(source.indices[*id])])
                            .collect::<Vec<_>>(),
                    );
                    mesh.insert_attribute(
                        Mesh::ATTRIBUTE_NORMAL,
                        ids.iter().map(|id| normals[*id]).collect::<Vec<_>>(),
                    );
                }
                let subjective = controller
                    .state
                    .first_person_view()
                    .then_some(controller.subjective.frame.as_ref())
                    .flatten();
                let matrix = subjective.map_or(matrix, |frame| frame.weapon_matrix);
                *visibility = if controller.state.alive()
                    && (!controller.state.first_person_view() || subjective.is_some())
                {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                };
                if let Some(mut buffer) = materials.buffers.get_mut(&gun.buffer) {
                    crate::gun_scene::source_parameters(
                        &controller.state.weapon.model.mesh.subobjects[gun.index].material,
                        Mat4::from_cols_array(&matrix).transpose(),
                        &camera,
                        &prepared.prepared.lights,
                        scene.asset.config.fog,
                        scene.no_fog,
                    )
                    .write_buffer(&mut buffer);
                }
            }
            if let Some(arms) = arms.filter(|p| p.ticket == controller.ticket) {
                let frame = controller
                    .state
                    .first_person_view()
                    .then_some(controller.subjective.frame.as_ref())
                    .flatten();
                *visibility = if frame.is_some() {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                };
                if let (Some(frame), Ok(resources)) = (frame, &prepared.prepared.subjective) {
                    let model = &resources.model.mesh;
                    let part = &model.subobjects[arms.part];
                    let ids = part
                        .triangles
                        .iter()
                        .flatten()
                        .map(|v| usize::from(*v))
                        .collect::<Vec<_>>();
                    let mut mesh_updated = false;
                    if let Some(mut mesh) = meshes.get_mut(&handle.0) {
                        mesh.insert_attribute(
                            Mesh::ATTRIBUTE_POSITION,
                            ids.iter()
                                .map(|id| frame.positions[usize::from(model.indices[*id])])
                                .collect::<Vec<_>>(),
                        );
                        mesh.insert_attribute(
                            Mesh::ATTRIBUTE_NORMAL,
                            ids.iter().map(|id| frame.normals[*id]).collect::<Vec<_>>(),
                        );
                        mesh_updated = true;
                    }
                    if let Some(mut buffer) = materials.buffers.get_mut(&arms.buffer) {
                        crate::gun_scene::source_parameters(
                            &part.material,
                            Mat4::from_cols_array(&frame.matrix).transpose(),
                            &camera,
                            &prepared.prepared.lights,
                            scene.asset.config.fog,
                            scene.no_fog,
                        )
                        .write_buffer(&mut buffer);
                        if mesh_updated {
                            visible_arms += 1;
                        }
                    }
                }
            }
        }
        if !controller.owned_backend
            && controller.steps > 0
            && !controller.owned_motion_failed
            && !controller.world_signals_failed
            && !controller.npc_failed
        {
            let advanced = runtime
                .normal_landing_owner(controller.ticket)
                .ok_or("NPC controlled owner absent".to_owned())
                .and_then(|local| {
                    audio.owned.with_impact(
                        controller.ticket,
                        frontend.feedback_volume(),
                        controller.state.camera().0.view,
                        local,
                        |device| {
                            runtime.advance_normal_npcs_with_retirement(
                                controller.ticket,
                                controller.state.mission_environment_colliders(),
                                device,
                            )
                        },
                    )
                });
            match advanced {
                Ok(sounds) => {
                    for event in sounds {
                        audio.local.write(crate::mission_audio::Request {
                            ticket: controller.ticket,
                            event,
                        });
                    }
                }
                Err(error) => {
                    controller.npc_failed = true;
                    error!(
                        "mission_vimp_component_refused ticket={} error={error}",
                        controller.ticket
                    );
                }
            }
        }
        if !controller.owned_backend
            && controller.steps > 0
            && !controller.owned_motion_failed
            && !controller.world_signals_failed
            && !controller.ripper_failed
        {
            let advanced = runtime
                .normal_landing_owner(controller.ticket)
                .ok_or("Ripper controlled owner absent".to_owned())
                .and_then(|local| {
                    runtime.publish_normal_mecc_contact(
                        controller.ticket,
                        u64::from(controller.steps),
                        controller.state.normal_contact_sample()?,
                    )?;
                    runtime.publish_normal_projectile_scene(
                        controller.ticket,
                        u64::from(controller.steps),
                        controller.state.normal_projectile_registry(),
                    )?;
                    audio.owned.with_impact(
                        controller.ticket,
                        frontend.feedback_volume(),
                        controller.state.camera().0.view,
                        local,
                        |device| {
                            runtime.advance_normal_rippers_with_retirement(
                                controller.ticket,
                                u64::from(controller.steps),
                                controller.state.camera().1,
                                controller.state.mission_environment_colliders(),
                                device,
                            )
                        },
                    )
                });
            match advanced {
                Ok(sounds) => {
                    for event in sounds {
                        audio.local.write(crate::mission_audio::Request {
                            ticket: controller.ticket,
                            event,
                        });
                    }
                }
                Err(error) => {
                    controller.ripper_failed = true;
                    // Preserve transport submissions preceding a late callback
                    // failure, without attempting that callback again.
                    for event in runtime.take_normal_ripper_sounds(controller.ticket) {
                        audio.local.write(crate::mission_audio::Request {
                            ticket: controller.ticket,
                            event,
                        });
                    }
                    error!(
                        "mission_ripper_component_refused ticket={} error={error}",
                        controller.ticket
                    );
                }
            }
        }
        if (controller.owned_replay || controller.descent_replay)
            && controller.replay_done
            && !controller.owned_draw_reported
        {
            assert_eq!(owned_body_parts, controller.state.body_draw_part_count());
            println!(
                "mission_owned_inspection_upload ticket={} steps={} body_parts={owned_body_parts} camera_eye_bits={:?} mesh_from_owned_pose=true anm_repeated=false ready=false",
                controller.ticket,
                controller.steps,
                eye.map(f32::to_bits)
            );
            controller.owned_draw_reported = true;
        }
        if controller.subjective_draw_visible != Some(visible_arms != 0) {
            println!(
                "mission_subjective_draw ticket={} visible_parts={visible_arms} mesh_and_parameters_updated=true weapon_source={} actor_backend_ready=false",
                controller.ticket,
                if visible_arms != 0 {
                    "subjective_corrected_reference"
                } else {
                    "world_body"
                }
            );
            controller.subjective_draw_visible = Some(visible_arms != 0);
        }
    }
}
