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
    pending_boost: bool,
    steps: u32,
    replay: bool,
    staging_replay: bool,
    fire_replay: bool,
    finish_step: u32,
    pub(crate) replay_done: bool,
    was_active: bool,
    owned_motion_failed: bool,
    owned_backend: bool,
    owned_audio_backend: bool,
    owned_jet_probe: bool,
    owned_replay: bool,
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
    pub(crate) fn owned_jet_frames(
        &self,
    ) -> Option<(u64, u64, &[giants_game::mecc_jet_presentation::Frame])> {
        self.state
            .owned_jet_frames()
            .map(|v| (self.ticket, u64::from(self.steps), v))
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
            owned_jet_probe: std::env::var("GIANTS_MISSION_OWNED_JET_PROBE").as_deref() == Ok("1"),
            owned_audio_backend: std::env::var("GIANTS_MISSION_MECC_AUDIO").as_deref()
                == Ok("reserved"),
            owned_replay: std::env::var("GIANTS_MISSION_REPLAY").as_deref()
                == Ok("owned-player-inspection"),
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
            pending_boost: false,
            steps: 0,
            replay: matches!(
                std::env::var("GIANTS_MISSION_REPLAY").ok().as_deref(),
                Some(
                    "player-motion"
                        | "owned-player-inspection"
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
                        | "showcase-mecc"
                )
            ),
            staging_replay: matches!(
                std::env::var("GIANTS_MISSION_REPLAY").ok().as_deref(),
                Some(
                    "player-motion"
                        | "player-motion-cycles"
                        | "player-motion-air"
                        | "owned-player-inspection"
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
            finish_step: if std::env::var("GIANTS_MISSION_REPLAY").as_deref()
                == Ok("owned-player-inspection")
            {
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
    time: Res<Time>,
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
        let focused = windows.iter().any(|(w, _)| w.focused);
        let look_ready = windows
            .iter()
            .any(|(w, c)| w.focused && c.grab_mode == bevy::window::CursorGrabMode::Locked);
        let controls = Some(frontend.mecc_controls());
        let held =
            |action| focused && mecc_input::pressed(controls, action, &keys, &buttons, false);
        let mut axes = if focused {
            mecc_input::movement_axes(controls, &keys, &buttons)
        } else {
            Vec2::ZERO
        };
        let mut thrust = held(Action::Thrust);
        controller.pending_boost |=
            focused && mecc_input::pressed(controls, Action::Boost, &keys, &buttons, true);
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
            let mut boost = std::mem::take(&mut controller.pending_boost);
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
                if controller.owned_replay && controller.owned_jet_probe {
                    axes = Vec2::ZERO;
                    thrust = (16..48).contains(&tick);
                    boost = tick == 32;
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
                        if controller.owned_replay && controller.steps == controller.finish_step {
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
            let fire = if controller.fire_replay {
                (60..300).contains(&tick)
            } else {
                !replay_running && held(Action::Fire)
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
            if replay_running && controller.steps == controller.finish_step {
                let distance = Vec3::from(position).distance(Vec3::from(controller.initial));
                if controller.fire_replay {
                    let pistol = controller.pistol().unwrap();
                    let expected = if controller.finish_step == 64 { 1 } else { 4 };
                    assert_eq!(pistol.fired, expected, "held pistol cadence failed");
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
            .weapon_matrix()
            .expect("selected mission weapon attachment failed");
        if let Some(initial) = controller.initial_weapon {
            controller.weapon_changed |= initial
                .iter()
                .zip(matrix)
                .any(|(a, b)| (*a - b).abs() > 0.00001);
        } else {
            controller.initial_weapon = Some(matrix);
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
        if controller.owned_replay && controller.replay_done && !controller.owned_draw_reported {
            assert_eq!(
                owned_body_parts,
                controller.state.body.mesh.subobjects.len()
            );
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
