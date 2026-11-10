//! Explicit internal frontend replay for preparation/inspection/cancellation.
//! Uses the real request and cancellation paths. Never sends readiness events,
//! injects operating-system input or claims normal-game interaction fidelity.
use super::*;
pub(crate) fn kabuto_body_inspection_replay() -> bool {
    matches!(
        std::env::var("GIANTS_MISSION_REPLAY").as_deref(),
        Ok("showcase-kabuto-body" | "showcase-kabuto-camera")
    )
}

pub(crate) fn initial_body_inspection_replay() -> bool {
    kabuto_body_inspection_replay()
        || std::env::var("GIANTS_MISSION_REPLAY").as_deref() == Ok("showcase-delphi-body")
}
pub(crate) fn initial_body_inspection_present(runtime: &crate::mission_runtime::Runtime) -> bool {
    if kabuto_body_inspection_replay() {
        runtime.kabuto_body().is_some()
    } else if std::env::var("GIANTS_MISSION_REPLAY").as_deref() == Ok("showcase-delphi-body") {
        runtime.delphi_body().is_some()
    } else {
        false
    }
}

pub(crate) fn owned_inspection_replay() -> bool {
    matches!(
        std::env::var("GIANTS_MISSION_REPLAY").as_deref(),
        Ok("owned-player-inspection" | "owned-player-inspection-cycles")
    )
}

#[derive(Resource)]
pub(crate) struct Replay {
    cycles: u32,
    entry: bool,
    audio_cancel: bool,
    motion: bool,
    opening: bool,
    completed: u32,
    stage: u8,
    since: f32,
    done: bool,
    showcase: bool,
    film_finished: bool,
}
pub(crate) fn install(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(tick) = std::env::var("GIANTS_PERSISTENT_SAMPLE_MIN_TICK")
        && (std::env::var("GIANTS_MISSION_REPLAY").as_deref() != Ok("showcase-persistent")
            || std::env::var_os("GIANTS_PERSISTENT_SAMPLE_SEQUENCE").is_none()
            || !tick.parse::<i32>().is_ok_and(|tick| tick >= 0))
    {
        return Err(
            "persistent sample tick requires its explicit replay, sequence and a nonnegative i32"
                .into(),
        );
    }
    if std::env::var_os("GIANTS_PERSISTENT_SAMPLE_COMPLETION").is_some()
        && (std::env::var("GIANTS_PERSISTENT_SAMPLE_COMPLETION").as_deref() != Ok("1")
            || std::env::var("GIANTS_MISSION_REPLAY").as_deref() != Ok("showcase-persistent"))
    {
        return Err("completion sample requires the explicit persistent replay".into());
    }
    if let Ok(sequence) = std::env::var("GIANTS_PERSISTENT_SAMPLE_SEQUENCE")
        && (std::env::var("GIANTS_MISSION_REPLAY").as_deref() != Ok("showcase-persistent")
            || sequence.is_empty()
            || sequence.len() > 64
            || !sequence
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_'))
    {
        return Err(
            "persistent sample sequence requires its explicit replay and an ASCII identifier"
                .into(),
        );
    }
    if let Ok(step) = std::env::var("GIANTS_MISSION_OWNED_JET_CAPTURE_STEP")
        && (!matches!(step.as_str(), "30" | "36" | "49" | "50" | "128")
            || std::env::var("GIANTS_MISSION_OWNED_JET_PROBE").as_deref() != Ok("1"))
    {
        return Err(
            "jet capture step requires the explicit jet probe and a supported phase".into(),
        );
    }
    if std::env::var("GIANTS_MISSION_OWNED_JET_PROBE").as_deref() == Ok("1")
        && (std::env::var("GIANTS_MISSION_REPLAY").as_deref() != Ok("owned-player-inspection")
            || std::env::var("GIANTS_MISSION_MECC_AUDIO").as_deref() != Ok("reserved"))
    {
        return Err("owned jet probe requires owned-player-inspection and reserved audio".into());
    }
    if (owned_inspection_replay()
        || std::env::var("GIANTS_MISSION_REPLAY").as_deref()
            == Ok("owned-player-descent-inspection"))
        && std::env::var("GIANTS_MISSION_MECC_BACKEND").as_deref() != Ok("owned-inspection")
    {
        return Err("owned-player-inspection requires the explicit owned Mecc backend".into());
    }
    let cycles = match std::env::var("GIANTS_MISSION_REPLAY").ok().as_deref() {
        None => return Ok(()),
        Some(
            "initial-player"
            | "owned-player-inspection"
            | "owned-player-descent-inspection"
            | "player-motion"
            | "player-motion-air"
            | "opening"
            | "opening-sample"
            | "campaign-entry"
            | "campaign-fire"
            | "campaign-fire-pose"
            | "campaign-descent-pose"
            | "showcase"
            | "showcase-menu"
            | "showcase-opening"
            | "showcase-persistent"
            | "showcase-free"
            | "showcase-cancel"
            | "showcase-mecc"
            | "showcase-kabuto-body"
            | "showcase-kabuto-camera"
            | "showcase-delphi-body"
            | "showcase-audio-cancel",
        ) => 0,
        Some(
            "cancel-relaunch"
            | "showcase-opening-cycles"
            | "player-motion-cycles"
            | "owned-player-inspection-cycles"
            | "opening-cycles"
            | "campaign-entry-cycles"
            | "campaign-fire-cycles"
            | "campaign-audio-cancel-cycles"
            | "campaign-audio-impact-cycles"
            | "campaign-ambient-cycles"
            | "campaign-flight-cycles",
        ) => 3,
        Some("showcase-film-cycles") => {
            if !app
                .world()
                .contains_resource::<crate::cinematic_audio::MutedTransport>()
            {
                return Err(
                    "Film cycles require --offscreen --silent-capture --muted-transport".into(),
                );
            }
            2
        }
        Some(_) => return Err("unknown GIANTS_MISSION_REPLAY mode".into()),
    };
    install_skip_probe(app);
    let entry = matches!(
        std::env::var("GIANTS_MISSION_REPLAY").ok().as_deref(),
        Some(
            "campaign-entry"
                | "campaign-entry-cycles"
                | "campaign-fire"
                | "campaign-fire-cycles"
                | "campaign-audio-cancel-cycles"
                | "campaign-audio-impact-cycles"
                | "campaign-ambient-cycles"
                | "campaign-flight-cycles"
                | "campaign-fire-pose"
                | "campaign-descent-pose"
        )
    );
    let showcase = std::env::var("GIANTS_MISSION_REPLAY").is_ok_and(|s| s.starts_with("showcase"));
    if !showcase {
        app.insert_resource(crate::mission_boot::Automatic(entry));
    }
    app.insert_resource(Replay {
        cycles,
        entry,
        audio_cancel: std::env::var("GIANTS_MISSION_REPLAY")
            .ok()
            .is_some_and(|m| {
                matches!(
                    m.as_str(),
                    "campaign-audio-cancel-cycles"
                        | "campaign-audio-impact-cycles"
                        | "campaign-ambient-cycles"
                        | "campaign-flight-cycles"
                )
            }),
        motion: matches!(
            std::env::var("GIANTS_MISSION_REPLAY").ok().as_deref(),
            Some(
                "player-motion"
                    | "player-motion-cycles"
                    | "player-motion-air"
                    | "owned-player-inspection"
                    | "owned-player-inspection-cycles"
                    | "owned-player-descent-inspection"
            )
        ),
        completed: 0,
        opening: matches!(
            std::env::var("GIANTS_MISSION_REPLAY").ok().as_deref(),
            Some("opening" | "opening-cycles" | "opening-sample")
        ),
        stage: 0,
        since: 0.,
        done: false,
        showcase,
        film_finished: false,
    })
    .add_systems(
        Update,
        drive
            .after(crate::mission_player::PlayerUpdate)
            .before(crate::mission_showcase::update)
            .before(crate::source_view::ViewUpdate),
    );
    Ok(())
}
pub(crate) fn capture_allowed(replay: Option<Res<Replay>>) -> bool {
    replay.is_none_or(|r| r.done)
}
/// Explicit silent bench: exercise the real skip input consumer, not its acknowledgements.
pub(crate) fn install_skip_probe(app: &mut App) {
    if std::env::var("GIANTS_CINEMATIC_SKIP_PROBE").ok().as_deref() == Some("1") {
        assert!(std::env::var_os("GIANTS_MISSION_REPLAY").is_some());
        app.add_systems(
            Update,
            inject_skip_probe.before(crate::mission_opening::skip_input),
        );
    }
}
fn inject_skip_probe(
    scene: Res<crate::frontend_scene::Scene>,
    cinematic: Option<Res<crate::cinematic_camera::Cinematic>>,
    time: Res<Time<Real>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut sample: Local<Option<(u64, u8, f64)>>,
) {
    keys.release(KeyCode::Escape);
    mouse.release(MouseButton::Left);
    if !matches!(scene.flow.phase, Phase::Cinematic | Phase::AwaitingRuntime)
        || cinematic.is_none_or(|c| c.clock_steps() < 2)
    {
        *sample = None;
        return;
    }
    let ticket = scene.flow.ticket;
    let now = time.elapsed_secs_f64();
    let (current, stage, start) = sample.get_or_insert((ticket, 0, now));
    if *current != ticket {
        (*current, *stage, *start) = (ticket, 0, now);
    }
    if ticket % 4 == 3 {
        if *stage == 0 && now - *start > 0.15 {
            keys.press(KeyCode::Escape);
            *stage = 2;
            println!("cinematic_skip_probe ticket={ticket} input=escape simulated=true");
        }
    } else if *stage < 2 && now - *start > 0.1 + f64::from(*stage) * 0.15 {
        mouse.press(MouseButton::Left);
        *stage += 1;
        println!(
            "cinematic_skip_probe ticket={ticket} input=click click={} simulated=true",
            *stage
        );
    }
}
#[allow(clippy::too_many_arguments)]
#[derive(bevy::ecs::system::SystemParam)]
struct Presentation<'w> {
    effects: Option<Res<'w, crate::mission_effects::State>>,
    time: Res<'w, Time>,
    pipelines: Res<'w, crate::PipelinesReady>,
    audio: Res<'w, crate::mission_audio::ProbeStatus>,
    ambient: Res<'w, crate::mission_ambient::ProbeStatus>,
    flight: Res<'w, crate::mission_flight_audio::ProbeStatus>,
    showcase: Res<'w, crate::mission_showcase::State>,
    keys: ResMut<'w, ButtonInput<KeyCode>>,
}
#[allow(clippy::too_many_arguments)]
fn drive(
    mut replay: ResMut<Replay>,
    mut frontend: ResMut<Scene>,
    mut runtime: ResMut<crate::mission_runtime::Runtime>,
    mut environment: ResMut<crate::mission_environment::State>,
    mut opening: ResMut<crate::mission_opening::State>,
    mut cinematic: Option<ResMut<crate::cinematic_camera::Cinematic>>,
    voices: Query<Option<&AudioSink>, With<crate::cinematic_audio::PlayingVoice>>,
    mut world: Option<ResMut<crate::world_scene::Scene>>,
    players: Query<&crate::mission_player::DrawnPlayer>,
    owned: Query<&crate::mission_runtime::MissionOwned>,
    controllers: Query<&crate::mission_player::Controller>,
    mut images: ResMut<Assets<Image>>,
    mut cancelled: MessageWriter<MissionCancelled>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    presentation: Presentation,
) {
    let Presentation {
        effects,
        time,
        pipelines,
        audio,
        ambient,
        flight,
        showcase,
        mut keys,
    } = presentation;
    if replay.done {
        if initial_body_inspection_present(&runtime)
            && let Some(mut world) = world
        {
            // Scene adoption can replace this capture flag after the endpoint.
            // It remains an inspection flag, never a mission readiness event.
            if !world.cinematic_capture_ready {
                world.cinematic_capture_ready = true;
            }
        }
        return;
    }
    let mode = std::env::var("GIANTS_MISSION_REPLAY").unwrap();
    assert!(
        time.elapsed_secs()
            < if mode == "showcase-film-cycles" {
                1800.
            } else {
                300.
            },
        "mission replay timed out before its real endpoint"
    );
    if replay.showcase {
        if replay.stage == 0 {
            replay.film_finished = false;
            if replay.completed == 0 {
                frontend
                    .campaign
                    .unlock_profile("Showcase")
                    .expect("independent showcase progress");
            }
            if mode == "showcase-menu" {
                frontend.screen = Screen::LoadWorlds;
                frontend.selected = Some(0);
                frontend.revision += 1;
                replay.done = true;
                println!(
                    "showcase_replay_menu worlds={} all_unlocked=true",
                    frontend.campaign.available_worlds().unwrap().len()
                );
                return;
            }
            let request = if let Ok(wanted) = std::env::var("GIANTS_SHOWCASE_WORLD") {
                let index = frontend
                    .campaign
                    .available_worlds()
                    .unwrap()
                    .iter()
                    .position(|w| w.name.eq_ignore_ascii_case(&wanted))
                    .expect("selected showcase world");
                frontend
                    .campaign
                    .select_world(index)
                    .expect("showcase mission catalog");
                let index = std::env::var("GIANTS_SHOWCASE_MISSION")
                    .ok()
                    .map_or(0, |v| v.parse().expect("zero-based mission index"));
                frontend.campaign.request(index)
            } else {
                frontend.campaign.new_campaign()
            };
            begin_mission(
                &mut frontend,
                request,
                &mut images,
                if mode == "showcase-persistent"
                    && std::env::var("GIANTS_PERSISTENT_SAMPLE_COMPLETION").as_deref() == Ok("1")
                {
                    LaunchIntent::CompletionReplay
                } else if std::env::var_os("GIANTS_SHOWCASE_WORLD").is_some() {
                    LaunchIntent::SavedMission
                } else {
                    LaunchIntent::NewCampaign
                },
            );
            frontend.revision += 1;
            replay.stage = 1;
        } else {
            if replay.stage == 2 {
                if runtime.idle()
                    && environment.idle()
                    && opening.idle()
                    && owned.is_empty()
                    && players.is_empty()
                    && cinematic.is_none()
                {
                    if matches!(
                        mode.as_str(),
                        "showcase-opening-cycles" | "showcase-film-cycles"
                    ) {
                        replay.completed += 1;
                        println!(
                            "showcase_opening_cycle_released cycles={} remaining_entities=0 movie_released=true input_injected=false",
                            replay.completed
                        );
                        if replay.completed < replay.cycles {
                            replay.stage = 0;
                            return;
                        }
                    }
                    replay.done = true;
                    println!(
                        "showcase_replay_cancelled remaining_entities=0 movie_released=true normal_ready_simulated=false"
                    );
                }
                return;
            }
            let playing = showcase.controls(frontend.flow.phase);
            let opened = cinematic.as_ref().is_some_and(|c| c.clock_steps() > 180);
            if initial_body_inspection_present(&runtime)
                && players.iter().any(|p| p.ticket == frontend.flow.ticket)
                && environment.presentation_ready()
                && pipelines.0
            {
                if mode == "showcase-kabuto-camera" {
                    let result = std::env::var_os("GIANTS_KABUTO_EXECUTABLE")
                        .ok_or_else(|| "GIANTS_KABUTO_EXECUTABLE is required for the explicit camera inspection".to_string())
                        .and_then(|path| runtime.prepare_kabuto_camera_inspection(std::path::Path::new(&path)));
                    if let Err(error) = result {
                        println!("kabuto_camera_inspection_failed: {error}");
                        replay.done = true;
                        return;
                    }
                }
                if let Ok(Some(candidate)) = runtime.environment_candidate() {
                    if mode == "showcase-kabuto-camera" {
                        let Some(world) = world.as_mut() else { return };
                        if world.camera_source != "kabuto_profile_inspection" {
                            world.validation_camera = Some(candidate.camera);
                            world.camera_source = candidate.camera_source;
                            world.camera_origin = Some(candidate.eye);
                            world.cinematic_capture_ready = false;
                            println!(
                                "kabuto_profile_camera_published ticket={} mode_request=1 position_kernel_only=true floor_chase_mesh=false gameplay_ready=false",
                                frontend.flow.ticket
                            );
                        }
                    }
                    environment.inspect(frontend.flow.ticket, candidate.eye);
                }
                if replay.since == 0. {
                    replay.since = time.elapsed_secs();
                    return;
                }
                if time.elapsed_secs() - replay.since < 0.5 {
                    return;
                }
                if let Some(mut world) = world {
                    world.cinematic_capture_ready = true;
                }
                replay.done = true;
                let kind = if kabuto_body_inspection_replay() {
                    "kabuto"
                } else {
                    "delphi"
                };
                println!(
                    "{kind}_body_inspection_finished ticket={} actual_owned_body=true normal_input=false initializer_acknowledged=false gameplay_ready=false",
                    frontend.flow.ticket
                );
                return;
            }
            if mode == "showcase-film-cycles" {
                if !replay.film_finished
                    && let Some(terminal) = opening.source_terminal(frontend.flow.ticket)
                {
                    replay.film_finished = true;
                    println!(
                        "showcase_film_terminal ticket={} directors={} sequence={} tick={} source_terminal=true voices_drained=true input_injected=false fidelity_verified=false",
                        frontend.flow.ticket, terminal.directors, terminal.sequence, terminal.tick
                    );
                }
                if replay.film_finished
                    && cinematic.is_none()
                    && opening.idle()
                    && (playing || showcase.is_free())
                    && environment.presentation_ready()
                {
                    println!(
                        "showcase_film_endpoint ticket={} phase={:?} mecc_preview={} free_inspection={} campaign_ready_claimed=false",
                        frontend.flow.ticket,
                        frontend.flow.phase,
                        playing,
                        showcase.is_free()
                    );
                    cancel_mission(&mut frontend, &mut cancelled);
                    replay.stage = 2;
                }
                return;
            }
            let ready = if mode == "showcase-audio-cancel" {
                false
            } else if mode == "showcase-mecc" {
                playing && controllers.iter().any(|c| c.replay_done)
            } else if mode == "showcase-persistent" {
                opened
                    && std::env::var("GIANTS_PERSISTENT_SAMPLE_SEQUENCE").map_or(true, |wanted| {
                        cinematic
                            .as_ref()
                            .is_some_and(|c| c.sequence().eq_ignore_ascii_case(&wanted))
                    })
                    && std::env::var("GIANTS_PERSISTENT_SAMPLE_MIN_TICK").map_or(true, |tick| {
                        cinematic.as_ref().is_some_and(|c| {
                            tick.parse::<i32>()
                                .is_ok_and(|tick| c.tick() >= tick && c.event_tick() >= tick)
                        })
                    })
                    && effects.as_ref().is_some_and(|s| {
                        cinematic
                            .as_ref()
                            .is_some_and(|c| s.sampleable_persistent_objects(&runtime, c) > 0)
                    })
            } else if matches!(
                mode.as_str(),
                "showcase-opening" | "showcase-opening-cycles"
            ) {
                opened
            } else {
                playing
                    || showcase.is_free()
                    || showcase.unavailable()
                    || frontend.flow.phase == Phase::Failed
            };
            // Service-level cancellation after a real rendered opening sample.
            // No key/mouse injection, skip acknowledgement or gameplay admission.
            if mode == "showcase-opening-cycles"
                && (if std::env::var("GIANTS_CANCEL_PERSISTENT").ok().as_deref() == Some("1") {
                    effects
                        .as_ref()
                        .is_some_and(|s| s.advanced_persistent_objects(&runtime) > 0)
                } else {
                    opened && environment.presentation_ready() && pipelines.0
                })
            {
                if std::env::var("GIANTS_CANCEL_PERSISTENT").ok().as_deref() == Some("1") {
                    println!(
                        "showcase_persistent_cancel ticket={} objects={} input_injected=false observation_gate=two_owned_updates",
                        frontend.flow.ticket,
                        effects
                            .as_ref()
                            .unwrap()
                            .advanced_persistent_objects(&runtime)
                    );
                }
                if let Some(cinematic) = &cinematic {
                    println!(
                        "showcase_opening_cycle_sample ticket={} director={} steps={} director_seconds={} input_injected=false audio_clock_verified=false",
                        frontend.flow.ticket,
                        cinematic.director_index(),
                        cinematic.clock_steps(),
                        cinematic.director_seconds()
                    );
                }
                cancel_mission(&mut frontend, &mut cancelled);
                replay.stage = 2;
                return;
            }
            // A cyclic observer may still be waiting for the requested live
            // object. It must never fall through to one-shot completion below.
            if mode == "showcase-opening-cycles" {
                return;
            }
            if matches!(mode.as_str(), "showcase-free" | "showcase-cancel")
                && (playing || opened)
                && !showcase.is_free()
            {
                keys.press(KeyCode::F6);
                return;
            }
            if mode == "showcase-cancel" && showcase.is_free() {
                keys.reset(KeyCode::F6);
                cancel_mission(&mut frontend, &mut cancelled);
                replay.stage = 2;
                return;
            }
            if mode == "showcase-audio-cancel"
                && playing
                && audio.ticket == Some(frontend.flow.ticket)
                && audio.actor_started >= 2
                && audio.actor_active > 0
            {
                println!(
                    "mission_actor_audio_cancel_probe ticket={} started={} active={} actual_sink=true normal_ready_simulated=false",
                    frontend.flow.ticket, audio.actor_started, audio.actor_active
                );
                cancel_mission(&mut frontend, &mut cancelled);
                replay.stage = 2;
                return;
            }
            if mode == "showcase-mecc"
                && playing
                && std::env::var("GIANTS_SUBJECTIVE_PROBE").ok().as_deref() == Some("cycle-cancel")
                && controllers.iter().any(|c| c.replay_done)
            {
                println!(
                    "mission_subjective_cancel_probe ticket={} controller_finished=true normal_ready_simulated=false",
                    frontend.flow.ticket
                );
                cancel_mission(&mut frontend, &mut cancelled);
                replay.stage = 2;
                return;
            }
            if ready
                && environment.presentation_ready()
                && (mode != "showcase-free" || showcase.is_free())
                && pipelines.0
            {
                if matches!(mode.as_str(), "showcase-opening" | "showcase-persistent")
                    && let Some(cinematic) = &mut cinematic
                {
                    if mode == "showcase-persistent" {
                        println!(
                            "showcase_persistent_sample ticket={} director={} sequence={} tick={} objects={} input_injected=false native_fidelity_verified=false",
                            frontend.flow.ticket,
                            cinematic.director_index(),
                            cinematic.sequence(),
                            cinematic.tick(),
                            effects
                                .as_ref()
                                .unwrap()
                                .sampleable_persistent_objects(&runtime, cinematic)
                        );
                    }
                    cinematic.suspend(true);
                }
                if let Some(mut world) = world {
                    world.cinematic_capture_ready = true;
                }
                replay.done = true;
                println!(
                    "showcase_replay_finished phase={:?} free_camera={} normal_ready_simulated=false human_validation=false",
                    frontend.flow.phase,
                    showcase.is_free()
                );
            }
        }
        return;
    }
    match replay.stage {
        0 if frontend.flow.phase == Phase::Menu && runtime.idle() => {
            let request = frontend.campaign.new_campaign();
            begin_mission(
                &mut frontend,
                request,
                &mut images,
                LaunchIntent::NewCampaign,
            );
            frontend.revision += 1;
            assert_eq!(frontend.screen, Screen::MissionLoading);
            println!(
                "mission_replay_request ticket={} path=new_campaign ready_event=false",
                frontend.flow.ticket
            );
            replay.stage = 1;
        }
        1 | 5 if replay.entry => match frontend.flow.phase {
            Phase::Cinematic if replay.stage == 1 => {
                println!(
                    "mission_replay_entry ticket={} phase=Cinematic acknowledgements=backend_only",
                    frontend.flow.ticket
                );
                replay.stage = 5;
            }
            Phase::Objectives => {
                if !accept_objectives(&mut frontend, &mut mouse) {
                    return;
                }
                println!(
                    "mission_replay_entry ticket={} phase=Gameplay objectives_action=internal controller_ready=backend human_validation=false",
                    frontend.flow.ticket
                );
                replay.stage = 6;
            }
            _ => {}
        },
        6 if replay.entry && controllers.iter().any(|c| c.replay_done) => {
            if replay.audio_cancel {
                if audio.ticket != Some(frontend.flow.ticket) || !audio.active_loop {
                    return;
                }
                println!(
                    "mission_replay_audio_cancel ticket={} active_loop_observed=true synthetic_controller=true",
                    frontend.flow.ticket
                );
            }
            if matches!(
                std::env::var("GIANTS_MISSION_REPLAY").ok().as_deref(),
                Some("campaign-ambient-cycles" | "campaign-flight-cycles")
            ) {
                if ambient.ticket != Some(frontend.flow.ticket)
                    || !ambient.loop_started
                    || ambient.accents == 0
                {
                    return;
                }
                println!(
                    "mission_replay_ambient_cancel ticket={} loop_observed=true accents={} synthetic_controller=true",
                    frontend.flow.ticket, ambient.accents
                );
            }
            if std::env::var("GIANTS_MISSION_REPLAY").ok().as_deref()
                == Some("campaign-flight-cycles")
            {
                if flight.ticket != Some(frontend.flow.ticket) || !flight.active_loop {
                    return;
                }
                println!(
                    "mission_replay_flight_cancel ticket={} active_loop_observed=true controller_clock_supplied=true",
                    frontend.flow.ticket
                );
            }
            assert_eq!(frontend.flow.phase, Phase::Gameplay);
            assert!(opening.idle());
            if replay.cycles == 0 {
                if let Some(mut world) = world {
                    world.cinematic_capture_ready = true;
                }
                replay.done = true;
                println!(
                    "mission_replay_entry_finished ticket={} normal_admission=true human_validation=false",
                    frontend.flow.ticket
                );
            } else {
                cancel_mission(&mut frontend, &mut cancelled);
                replay.stage = 3;
            }
        }
        1 if frontend.flow.phase == Phase::AwaitingRuntime => {
            if replay.opening {
                let ticket = frontend.flow.ticket;
                if !opening.inspect(ticket) {
                    return;
                }
                println!(
                    "mission_replay_opening ticket={ticket} source=selected_mission normal_admission=false ready_event=false"
                );
                replay.since = time.elapsed_secs();
                replay.stage = 2;
                return;
            }
            let Some((ticket, player)) = runtime.player_candidate() else {
                return;
            };
            if ticket != frontend.flow.ticket {
                return;
            }
            // Staging waits for the initial body's resources. Alternate owned
            // draw models are also mounted, then selected by the live controller.
            let count = players
                .iter()
                .filter(|p| p.ticket == ticket && p.slot == 0)
                .count();
            if count != player.body.mesh.subobjects.len() {
                return;
            }
            let Some(mut world) = world else {
                return;
            };
            let pose = giants_game::mecc_camera::pose(
                player.start.position,
                player.start.angles[0],
                0.,
                false,
            )
            .expect("prepared player inspection pose");
            if !environment.inspect(ticket, pose.eye) {
                return;
            }
            let view = crate::mecc_dev_scene::camera_view(
                Vec3::from(pose.eye),
                Vec3::from(pose.forward),
                Vec3::from(pose.up),
            );
            world.validation_camera = Some(crate::validation_camera::ValidationCamera {
                view: view.to_cols_array(),
                projection: giants_game::mecc_camera::projection(1920. / 1080., 25000.)
                    .expect("fixed inspection lens"),
                viewport: UVec4::new(0, 0, 1920, 1080),
            });
            world.camera_source = "mission_player_inspection";
            world.camera_origin = Some(pose.eye);
            println!(
                "mission_replay_inspection ticket={ticket} meshes={count} camera=declared_start_mecc_profile normal_start_frame=false controller=false ready=false"
            );
            replay.since = time.elapsed_secs();
            replay.stage = 2;
        }
        2 if time.elapsed_secs() - replay.since >= 0.5 && pipelines.0 => {
            assert_eq!(frontend.flow.phase, Phase::AwaitingRuntime);
            if replay.opening {
                let Some(cinematic) = &mut cinematic else {
                    return;
                };
                if std::env::var("GIANTS_MISSION_REPLAY").ok().as_deref() == Some("opening-sample")
                {
                    let [start, end, _, _] = cinematic.animation().timeline;
                    if cinematic.director_index() != 1
                        || cinematic.tick() < start + (end - start) / 2
                    {
                        return;
                    }
                    cinematic.suspend(true);
                    if let Some(mut world) = world {
                        world.cinematic_capture_ready = true;
                    }
                    replay.done = true;
                    println!(
                        "mission_replay_opening_sample ticket={} director={} sequence={} tick={} paused=true campaign_ready=false",
                        frontend.flow.ticket,
                        cinematic.director_index(),
                        cinematic.sequence(),
                        cinematic.tick()
                    );
                    return;
                }
                let complete = cinematic.end_reached()
                    && cinematic.release_ready(false)
                    && voices.iter().all(|sink| sink.is_some_and(|s| s.empty()));
                if replay.cycles == 0 {
                    if !complete {
                        return;
                    }
                    if let Some(mut world) = world {
                        world.cinematic_capture_ready = true;
                    }
                    replay.done = true;
                    println!(
                        "mission_replay_opening_finished ticket={} directors={} sequence={} complete=true ready_events=0 normal_gameplay=false",
                        frontend.flow.ticket,
                        cinematic.segments().len(),
                        cinematic.sequence()
                    );
                    return;
                }
                // Cancel a running transport before its natural endpoint to
                // verify sink and resource retirement on a subsequent request.
                if cinematic.clock_steps() < 120
                    || !voices.iter().any(|sink| sink.is_some_and(|s| !s.empty()))
                {
                    return;
                }
                cancel_mission(&mut frontend, &mut cancelled);
                replay.stage = 3;
                return;
            }
            if replay.motion && !controllers.iter().any(|c| c.replay_done) {
                return;
            }
            if replay.cycles == 0 {
                if let Some(mut world) = world {
                    world.cinematic_capture_ready = true;
                }
                replay.done = true;
                println!(
                    "mission_replay_initial_player_finished passed=true real_request=true draw=true human_validation=false controller_simulated={} normal_gameplay=false campaign_ready=false",
                    replay.motion
                );
            } else {
                cancel_mission(&mut frontend, &mut cancelled);
                replay.stage = 3;
            }
        }
        3 if runtime.idle()
            && environment.idle()
            && opening.idle()
            && players.is_empty()
            && owned.is_empty() =>
        {
            assert_eq!(frontend.flow.phase, Phase::Menu);
            replay.completed += 1;
            println!(
                "mission_replay_cancelled cycles={} remaining_entities=0 normal_admission={}",
                replay.completed, replay.entry
            );
            if replay.completed == replay.cycles {
                replay.stage = 4;
                replay.since = time.elapsed_secs();
            } else {
                replay.stage = 0;
            }
        }
        4 if time.elapsed_secs() - replay.since >= 0.5 && pipelines.0 => {
            replay.done = true;
            println!(
                "mission_replay_cycles_finished passed=true requests={} cancellations={} remaining_entities=0 human_validation=false campaign_ready=false",
                replay.completed, replay.completed
            );
        }
        _ => {}
    }
}
