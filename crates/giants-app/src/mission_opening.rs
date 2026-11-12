//! Mission-selected cinematic transport and audio ownership. Preparation is not
//! readiness: actor/effect dispatch and campaign admission remain separate gates.
use crate::{cinematic_audio, cinematic_camera, mission_runtime::Runtime};
use bevy::{asset::AssetId, ecs::system::RunSystemOnce, prelude::*};
use giants_game::mission_loading::Phase;
use std::collections::HashSet;

pub struct Prepared {
    cinematic: cinematic_camera::Cinematic,
    master: Option<cinematic_audio::Master>,
    voices: cinematic_audio::Voices,
    render: Result<crate::mission_film::Prepared, String>,
}
impl Prepared {
    pub(crate) fn prepare_timed_world(
        &mut self,
        plan: &giants_game::instances::Plan,
        terrain: &giants_formats::gti::Terrain,
    ) {
        if let Ok(render) = &mut self.render
            && let Err(error) = render.prepare_timed_world(plan, terrain)
        {
            println!("mission_timed_world_deferred reason={error:?} whole_film_ready=false");
        }
    }
    pub(crate) fn effects_bound(&self) -> bool {
        self.render.as_ref().is_ok_and(|r| r.effects_bound())
    }
    pub fn load(root: &std::path::Path, source: Option<&str>) -> Result<Option<Self>, String> {
        let Some(source) = source else {
            return Ok(None);
        };
        let result = (|| -> Result<Self, Box<dyn std::error::Error>> {
            let (mut cinematic, _) = cinematic_camera::Cinematic::load(
                root,
                cinematic_camera::Load {
                    tick: None,
                    benchmark: false,
                    directors: 1,
                    sequence: Some(source),
                    full_chain: true,
                    // Same partial transport profile as the explicit reader. This
                    // does not waive the strict director or campaign readiness gate.
                    preview: true,
                },
            )?;
            let master = cinematic_audio::Master::load(root, cinematic.programs())?;
            let voices = cinematic_audio::Voices::load(root, cinematic.programs())?;
            cinematic.suspend(true);
            Ok(Self {
                cinematic,
                master,
                voices,
                render: Err("render preparation not requested".into()),
            })
        })();
        result.map(Some).map_err(|e| e.to_string())
    }
    pub(crate) fn prepare_render(
        &mut self,
        root: &std::path::Path,
        resources: &giants_game::scene_resources::SceneResources,
        config: &giants_formats::world::WorldConfig,
    ) {
        self.render = crate::mission_film::Prepared::load(root, &self.cinematic, resources, config);
    }
}
#[derive(Resource)]
pub(crate) struct Owner(pub u64);
#[derive(Component)]
pub(crate) struct OwnedAudio(pub u64);
#[derive(Resource, Default)]
pub(crate) struct State {
    mounted: Option<Mounted>,
    attempted: Option<u64>,
    terminal: Option<SourceTerminal>,
    completion_ticket: Option<u64>,
    skipped_ticket: Option<u64>,
}
/// Source-end and drained owned voices observed before transport retirement.
/// This is not campaign readiness, native timing or complete film fidelity.
pub(crate) struct SourceTerminal {
    ticket: u64,
    pub(crate) directors: usize,
    pub(crate) sequence: String,
    pub(crate) tick: i32,
}
/// Only this transport owner constructs completion receipts. An explicit user
/// skip remains distinct from a naturally observed source terminal.
#[derive(Clone, Copy)]
pub(crate) struct CompletionReceipt {
    ticket: u64,
    pub source_end: bool,
}
impl CompletionReceipt {
    pub(crate) fn ticket(self) -> u64 {
        self.ticket
    }
}
#[derive(Resource, Default)]
struct SkipGesture {
    ticket: Option<u64>,
    last_click: Option<f64>,
}
impl SkipGesture {
    fn sample(&mut self, ticket: Option<u64>, now: f64, escape: bool, click: bool) -> bool {
        if self.ticket != ticket {
            self.ticket = ticket;
            self.last_click = None;
        }
        if ticket.is_none() {
            self.last_click = None;
            return false;
        }
        if escape {
            self.last_click = None;
            return true;
        }
        if !click {
            return false;
        }
        let double = self
            .last_click
            .is_some_and(|last| (0.0..=0.4).contains(&(now - last)));
        self.last_click = (!double).then_some(now);
        double
    }
}
struct Mounted {
    ticket: u64,
    cinematic: Option<cinematic_camera::Cinematic>,
    assets: HashSet<AssetId<AudioSource>>,
    inspecting: bool,
    was_running: bool,
    render_failed: bool,
}
impl State {
    pub(crate) fn arm_completion(&mut self, ticket: u64) -> Result<(), String> {
        if !self.idle() {
            return Err("completion cannot replace a mounted film".into());
        }
        self.attempted = None;
        self.terminal = None;
        self.skipped_ticket = None;
        self.completion_ticket = Some(ticket);
        Ok(())
    }
    pub(crate) fn completion_finished(&self, ticket: u64) -> bool {
        self.completion_ticket == Some(ticket)
            && self.idle()
            && (self.source_terminal(ticket).is_some() || self.skipped_ticket == Some(ticket))
    }
    pub(crate) fn completion_receipt(&self, ticket: u64) -> Option<CompletionReceipt> {
        self.completion_finished(ticket).then(|| CompletionReceipt {
            ticket,
            source_end: self.source_terminal(ticket).is_some(),
        })
    }
    pub(crate) fn source_terminal(&self, ticket: u64) -> Option<&SourceTerminal> {
        self.terminal.as_ref().filter(|end| end.ticket == ticket)
    }
    fn begin_attempt(&mut self, ticket: u64) {
        self.attempted = Some(ticket);
        self.terminal = None;
        self.skipped_ticket = None;
        if self.completion_ticket != Some(ticket) {
            self.completion_ticket = None;
        }
    }
    pub(crate) fn attempted(&self, ticket: u64) -> bool {
        self.attempted == Some(ticket)
    }
    pub(crate) fn render_failed(&self, ticket: u64) -> bool {
        self.mounted
            .as_ref()
            .is_some_and(|m| m.ticket == ticket && m.render_failed)
    }
    pub(crate) fn idle(&self) -> bool {
        self.mounted.is_none()
    }
    pub(crate) fn inspect(&mut self, ticket: u64) -> bool {
        let Some(mounted) = self.mounted.as_mut().filter(|m| m.ticket == ticket) else {
            return false;
        };
        mounted.inspecting = true;
        true
    }
}
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OpeningUpdate;
pub struct Plugin;
impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(crate::mission_film::Plugin)
            .init_resource::<State>()
            .init_resource::<SkipGesture>()
            .add_systems(
                Update,
                skip_input
                    .before(crate::frontend_scene::input)
                    .before(OpeningUpdate)
                    .before(crate::mission_player::PlayerUpdate)
                    .run_if(crate::mecc_session::frontend_available),
            )
            .add_systems(
                Update,
                sync.after(crate::mission_environment::EnvironmentUpdate)
                    .before(OpeningUpdate)
                    .run_if(crate::mecc_session::frontend_available),
            )
            .add_systems(
                Update,
                (
                    cinematic_audio::sample_master
                        .run_if(resource_exists::<cinematic_audio::Master>)
                        .run_if(cinematic_audio::playback_enabled),
                    cinematic_camera::update,
                    crate::viewport_scene::update,
                    crate::mission_effects::update
                        .run_if(resource_exists::<crate::mission_effects::State>),
                    cinematic_audio::update_master
                        .run_if(resource_exists::<cinematic_audio::Master>)
                        .run_if(cinematic_audio::playback_enabled),
                    cinematic_audio::update.run_if(cinematic_audio::playback_enabled),
                    follow,
                )
                    .chain()
                    .in_set(OpeningUpdate)
                    .run_if(running)
                    .before(crate::mission_player::PlayerUpdate)
                    .before(crate::source_view::ViewUpdate),
            );
    }
}
pub(crate) fn skip_input(world: &mut World) {
    let focused = world.query::<&Window>().iter(world).all(|w| w.focused);
    let ticket = if focused && running(world) {
        world.get_resource::<Owner>().map(|o| o.0)
    } else {
        None
    };
    let escape = world
        .resource::<ButtonInput<KeyCode>>()
        .just_pressed(KeyCode::Escape);
    let click = world
        .resource::<ButtonInput<MouseButton>>()
        .just_pressed(MouseButton::Left);
    let now = world.resource::<Time<Real>>().elapsed_secs_f64();
    if !world
        .resource_mut::<SkipGesture>()
        .sample(ticket, now, escape, click)
    {
        return;
    }
    let ticket = ticket.expect("skip gesture needs an active opening");
    // Consume the gesture before frontend cancellation, objective dismissal or firing.
    world
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset(KeyCode::Escape);
    world
        .resource_mut::<ButtonInput<MouseButton>>()
        .reset(MouseButton::Left);
    let phase = world.resource::<crate::frontend_scene::Scene>().flow.phase;
    if phase == Phase::Cinematic {
        world.resource_mut::<State>().skipped_ticket = Some(ticket);
        release(world);
        world
            .write_message(crate::frontend_scene::MissionRuntimeEvent::CinematicSkipped { ticket });
        println!(
            "mission_opening_skipped ticket={ticket} source_end=false input={} cleanup=owned_only",
            if escape { "escape" } else { "double_click" }
        );
    } else {
        crate::mission_showcase::skip_opening(world, ticket);
    }
}
fn running(world: &World) -> bool {
    requested(world)
        && (crate::mission_film::ready(world) || crate::mission_showcase::partial_opening(world))
}
fn requested(world: &World) -> bool {
    if !crate::mecc_session::frontend_available(world) {
        return false;
    }
    let Some(state) = world.get_resource::<State>() else {
        return false;
    };
    let Some(m) = &state.mounted else {
        return false;
    };
    let Some(frontend) = world.get_resource::<crate::frontend_scene::Scene>() else {
        return false;
    };
    admitted(
        m.ticket,
        frontend.flow.ticket,
        frontend.flow.phase,
        m.inspecting,
    ) && world
        .get_resource::<Runtime>()
        .is_some_and(|r| r.active_ticket() == Some(m.ticket))
}
fn admitted(ticket: u64, current: u64, phase: Phase, inspection: bool) -> bool {
    ticket == current
        && (phase == Phase::Cinematic || inspection && phase == Phase::AwaitingRuntime)
}
fn release(world: &mut World) {
    let Some(mounted) = world.resource_mut::<State>().mounted.take() else {
        return;
    };
    assert_eq!(world.resource::<Owner>().0, mounted.ticket);
    // Audio sinks advance on their own thread. A later consumer cannot safely
    // reproduce the terminal observation after this function removes the movie.
    let terminal = world
        .get_resource::<cinematic_camera::Cinematic>()
        .filter(|movie| movie.end_reached())
        .map(|movie| SourceTerminal {
            ticket: mounted.ticket,
            directors: movie.segments().len(),
            sequence: movie.sequence().to_owned(),
            tick: movie.tick(),
        });
    if let Some(terminal) = terminal {
        let drained = world
            .query_filtered::<(&OwnedAudio, Option<&AudioSink>), With<cinematic_audio::PlayingVoice>>()
            .iter(world)
            .filter(|(owner, _)| owner.0 == mounted.ticket)
            .all(|(_, sink)| sink.is_some_and(|sink| sink.empty()));
        if drained {
            println!(
                "mission_opening_source_terminal ticket={} directors={} sequence={} tick={} voices_drained=true readiness_claimed=false",
                terminal.ticket, terminal.directors, terminal.sequence, terminal.tick
            );
            world.resource_mut::<State>().terminal = Some(terminal);
        }
    }
    let playback_enabled = world
        .get_resource::<cinematic_audio::PlaybackEnabled>()
        .is_some_and(|p| p.0);
    let queued_players = world
        .query_filtered::<&OwnedAudio, Or<(
            With<cinematic_audio::PlayingMaster>,
            With<cinematic_audio::PlayingVoice>,
        )>>()
        .iter(world)
        .filter(|owner| owner.0 == mounted.ticket)
        .count();
    println!(
        "mission_opening_audio_policy ticket={} playback_enabled={playback_enabled} queued_players={queued_players}",
        mounted.ticket
    );
    crate::mission_film::release(world);
    crate::mission_environment::restore_full_view(world, mounted.ticket);
    for (owner, voice, sink) in world
        .query::<(
            &OwnedAudio,
            &cinematic_audio::PlayingVoice,
            Option<&AudioSink>,
        )>()
        .iter(world)
        .filter(|(owner, _, _)| owner.0 == mounted.ticket)
    {
        voice.report_release(owner.0, sink);
    }
    let entities: Vec<_> = world
        .query::<(Entity, &OwnedAudio)>()
        .iter(world)
        .filter(|(_, owner)| owner.0 == mounted.ticket)
        .map(|(e, _)| e)
        .collect();
    for entity in &entities {
        world.despawn(*entity);
    }
    world.remove_resource::<cinematic_camera::Cinematic>();
    world.remove_resource::<cinematic_audio::Master>();
    world.remove_resource::<cinematic_audio::Voices>();
    world.remove_resource::<Owner>();
    let mut assets = world.resource_mut::<Assets<AudioSource>>();
    for id in &mounted.assets {
        assets.remove(*id);
    }
    let remaining = mounted
        .assets
        .iter()
        .filter(|id| assets.contains(**id))
        .count();
    assert_eq!(remaining, 0);
    if let Some(mut environment) = world.get_resource_mut::<crate::mission_environment::State>() {
        environment.end_inspection(mounted.ticket);
    }
    println!(
        "mission_opening_released ticket={} audio_entities={} assets={} remaining_assets={remaining}",
        mounted.ticket,
        entities.len(),
        mounted.assets.len()
    );
}
pub(crate) fn stop_preview(world: &mut World) {
    release(world);
}
fn sync(world: &mut World) {
    let frontend = world.resource::<crate::frontend_scene::Scene>();
    let ticket = frontend.flow.ticket;
    let phase = frontend.flow.phase;
    let wanted = world.resource::<Runtime>().active_ticket().filter(|t| {
        *t == ticket
            && matches!(phase, Phase::AwaitingRuntime | Phase::Cinematic)
            && !crate::mission_showcase::free(world)
            && !crate::mission_showcase::mecc_preview(world)
            && !crate::frontend_scene::initial_body_inspection_replay()
    });
    if world
        .resource::<State>()
        .mounted
        .as_ref()
        .is_some_and(|m| Some(m.ticket) != wanted)
    {
        release(world);
    }
    let Some(ticket) = wanted else {
        world.resource_mut::<State>().attempted = None;
        return;
    };
    if world.resource::<State>().attempted != Some(ticket)
        && world
            .resource::<crate::mission_environment::State>()
            .mounted_ticket()
            == Some(ticket)
    {
        world.resource_mut::<State>().begin_attempt(ticket);
        let prepared = world.resource_mut::<Runtime>().take_opening(ticket);
        match prepared {
            Ok(Some(prepared)) => {
                // Refuse to adopt an unrelated explicit reader or its assets.
                assert!(!world.contains_resource::<cinematic_camera::Cinematic>());
                assert!(!world.contains_resource::<cinematic_audio::Master>());
                assert!(!world.contains_resource::<cinematic_audio::Voices>());
                let directors = prepared.cinematic.segments().len();
                let voice_commands = prepared
                    .cinematic
                    .programs()
                    .flat_map(|p| &p.commands)
                    .filter(|c| c.opcode == 44)
                    .count();
                let sync_commands = prepared
                    .cinematic
                    .programs()
                    .flat_map(|p| &p.commands)
                    .filter(|c| c.opcode == 52)
                    .count();
                println!(
                    "mission_opening_source ticket={ticket} directors={directors} voice_commands={voice_commands} sync_commands={sync_commands} source=selected_mission"
                );
                let master = prepared.master.is_some();
                let render_failed = prepared.render.is_err();
                match prepared.render {
                    Ok(render) => crate::mission_film::mount(world, ticket, render),
                    Err(error) => println!("mission_film_blocked ticket={ticket} reason={error}"),
                }
                let before: HashSet<_> = world.resource::<Assets<AudioSource>>().ids().collect();
                world.insert_resource(Owner(ticket));
                world.insert_resource(prepared.voices);
                world
                    .run_system_once(cinematic_audio::setup)
                    .expect("owned voice setup");
                if let Some(master) = prepared.master {
                    world.insert_resource(master);
                    world
                        .run_system_once(cinematic_audio::setup_master)
                        .expect("owned master setup");
                }
                let assets: HashSet<_> = world
                    .resource::<Assets<AudioSource>>()
                    .ids()
                    .filter(|id| !before.contains(id))
                    .collect();
                println!(
                    "mission_opening_mounted ticket={ticket} directors={directors} master={master} audio_assets={} started=false ready=false partial_rendering=true",
                    assets.len()
                );
                world.resource_mut::<State>().mounted = Some(Mounted {
                    ticket,
                    cinematic: Some(prepared.cinematic),
                    assets,
                    inspecting: false,
                    was_running: false,
                    render_failed,
                });
            }
            Ok(None) => println!(
                "mission_opening_absent ticket={ticket} default_movie_inferred=false ready=false"
            ),
            Err(error) => println!("mission_opening_blocked ticket={ticket} reason={error}"),
        }
    }
    let request = requested(world);
    let prepare =
        request || phase == Phase::AwaitingRuntime && crate::mission_boot::candidate(world);
    let mut state = world.resource_mut::<State>();
    let Some(mounted) = state.mounted.as_mut() else {
        return;
    };
    let inspecting = mounted.inspecting;
    let prepared = if prepare {
        mounted.cinematic.take()
    } else {
        None
    };
    if let Some(cinematic) = prepared {
        let camera = cinematic.camera().expect("prepared initial movie camera");
        let mut scene = world.resource_mut::<crate::world_scene::Scene>();
        scene.validation_camera = Some(crate::cinematic_camera::convert(&camera));
        scene.camera_origin = Some(camera.origin);
        world.insert_resource(cinematic);
    }
    if prepare && world.contains_resource::<cinematic_camera::Cinematic>() {
        crate::mission_film::prepare_gpu(world);
    }
    let run = running(world);
    let mut state = world.resource_mut::<State>();
    let mounted = state.mounted.as_mut().unwrap();
    let changed = mounted.was_running != run;
    mounted.was_running = run;
    if changed {
        world
            .resource_mut::<cinematic_camera::Cinematic>()
            .suspend(!run);
        println!(
            "mission_opening_transport ticket={ticket} running={run} inspection={inspecting} phase={phase:?} ready_event=false"
        );
    }
    if !run {
        for (owner, sink) in world.query::<(&OwnedAudio, &AudioSink)>().iter(world) {
            if owner.0 == ticket {
                sink.pause();
            }
        }
    }
}
fn follow(
    owner: Res<Owner>,
    cinematic: Res<cinematic_camera::Cinematic>,
    state: Res<State>,
    mut environment: ResMut<crate::mission_environment::State>,
    mut scene: ResMut<crate::world_scene::Scene>,
) {
    let Some(mounted) = &state.mounted else {
        return;
    };
    let Ok(camera) = cinematic.camera() else {
        return;
    };
    scene.camera_source = "mission_opening";
    if mounted.inspecting {
        environment.inspect(owner.0, camera.origin);
    } else {
        environment.follow(owner.0, camera.origin);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn skip_clicks_do_not_cross_tickets_focus_loss_or_expired_intervals() {
        let mut gesture = SkipGesture::default();
        assert!(!gesture.sample(None, 0., false, true));
        assert!(!gesture.sample(Some(7), 0.1, false, true));
        assert!(!gesture.sample(Some(8), 0.2, false, true));
        assert!(!gesture.sample(None, 0.3, false, false));
        assert!(!gesture.sample(Some(8), 0.4, false, true));
        assert!(!gesture.sample(Some(8), 1.0, false, true));
        assert!(gesture.sample(Some(8), 1.2, false, true));
        assert!(!gesture.sample(Some(8), 1.3, false, true));
        assert!(gesture.sample(Some(8), 1.4, true, false));
        assert!(!gesture.sample(Some(8), 1.5, false, true));
    }
    #[test]
    fn playback_requires_current_ticket_and_real_phase_or_explicit_inspection() {
        assert!(!admitted(7, 7, Phase::AwaitingRuntime, false));
        assert!(admitted(7, 7, Phase::AwaitingRuntime, true));
        assert!(admitted(7, 7, Phase::Cinematic, false));
        assert!(!admitted(6, 7, Phase::Cinematic, true));
        for phase in [
            Phase::Menu,
            Phase::Loading,
            Phase::Splash,
            Phase::Failed,
            Phase::Objectives,
            Phase::Gameplay,
        ] {
            assert!(!admitted(7, 7, phase, true));
        }
    }
    #[test]
    fn absent_declared_movie_does_not_load_a_default() {
        assert!(
            Prepared::load(std::path::Path::new("missing-corpus"), None)
                .unwrap()
                .is_none()
        );
    }
    #[test]
    fn completion_receipt_cannot_reuse_the_opening_terminal_or_a_bare_release() {
        let mut world = World::new();
        world.init_resource::<State>();
        world.init_resource::<Assets<AudioSource>>();
        world.resource_mut::<State>().terminal = Some(SourceTerminal {
            ticket: 7,
            directors: 1,
            sequence: "opening".into(),
            tick: 100,
        });
        assert!(world.resource::<State>().completion_receipt(7).is_none());
        world.resource_mut::<State>().arm_completion(7).unwrap();
        assert!(world.resource::<State>().source_terminal(7).is_none());
        world.insert_resource(Owner(7));
        world.resource_mut::<State>().mounted = Some(Mounted {
            ticket: 7,
            cinematic: None,
            assets: HashSet::new(),
            inspecting: false,
            was_running: false,
            render_failed: false,
        });
        assert!(world.resource_mut::<State>().arm_completion(7).is_err());
        release(&mut world);
        assert!(world.resource::<State>().completion_receipt(7).is_none());
        // A user-skip witness is a different outcome, never a source terminal.
        world.resource_mut::<State>().skipped_ticket = Some(7);
        let receipt = world.resource::<State>().completion_receipt(7).unwrap();
        assert!(!receipt.source_end);
        assert!(world.resource::<State>().source_terminal(7).is_none());
        assert!(world.resource::<State>().completion_receipt(8).is_none());
        world.resource_mut::<State>().begin_attempt(8);
        assert!(world.resource::<State>().completion_receipt(7).is_none());
    }
    #[test]
    #[ignore = "Requires GIANTS_MISSION_OWNED_ROOT; source completion CPU preparation only, not rendered mission completion"]
    fn installed_wife_completion_has_real_prepared_render_and_effect_owners() {
        let root = std::path::PathBuf::from(std::env::var("GIANTS_MISSION_OWNED_ROOT").unwrap());
        let bytes = std::fs::read(root.join("Bin/w_Story2.bin")).unwrap();
        let menu = giants_game::mission_selection::MissionMenu::parse(&bytes, 0, true).unwrap();
        let index = menu
            .entries
            .iter()
            .position(|m| m.name == "2b_1_savewife")
            .unwrap();
        let packet =
            giants_game::mission_loading::prepare(&root, menu.request("Story2", index).unwrap())
                .unwrap();
        let plan = packet.instance_plan(&root, "English").unwrap();
        let world = packet.world_asset(&root).unwrap();
        let selection = packet.completion(&root).unwrap();
        let mut prepared = Prepared::load(&root, selection.director.as_deref())
            .unwrap()
            .unwrap();
        prepared.prepare_render(&root, plan.resources(), &world.config);
        assert!(
            !prepared.effects_bound(),
            "actual world has not been prepared"
        );
        prepared.prepare_timed_world(&plan, &world.terrain);
        assert!(
            prepared.effects_bound(),
            "source completion render: {:?}",
            prepared.render.as_ref().err()
        );
        assert_eq!(prepared.cinematic.segments().len(), 6);
        println!(
            "wife_completion_source_prepared directors=6 effects_bound=true runtime_completion=false"
        );
    }
    #[test]
    fn release_without_source_end_does_not_create_a_terminal() {
        let mut world = World::new();
        world.init_resource::<State>();
        world.init_resource::<Assets<AudioSource>>();
        world.insert_resource(Owner(7));
        world.resource_mut::<State>().mounted = Some(Mounted {
            ticket: 7,
            cinematic: None,
            assets: HashSet::new(),
            inspecting: false,
            was_running: false,
            render_failed: false,
        });
        release(&mut world);
        assert!(world.resource::<State>().source_terminal(7).is_none());
    }
    #[test]
    fn release_preserves_unrelated_audio_and_is_idempotent() {
        let mut world = World::new();
        world.init_resource::<State>();
        world.init_resource::<Assets<AudioSource>>();
        let own = world
            .resource_mut::<Assets<AudioSource>>()
            .add(AudioSource {
                bytes: Vec::new().into(),
            });
        let unrelated = world
            .resource_mut::<Assets<AudioSource>>()
            .add(AudioSource {
                bytes: Vec::new().into(),
            });
        let ours = world.spawn(OwnedAudio(7)).id();
        let other = world.spawn(OwnedAudio(8)).id();
        world.insert_resource(Owner(7));
        world.resource_mut::<State>().mounted = Some(Mounted {
            ticket: 7,
            cinematic: None,
            assets: HashSet::from([own.id()]),
            inspecting: false,
            was_running: false,
            render_failed: false,
        });
        world.resource_mut::<State>().terminal = Some(SourceTerminal {
            ticket: 7,
            directors: 2,
            sequence: "synthetic".into(),
            tick: 300,
        });
        release(&mut world);
        release(&mut world);
        assert_eq!(
            world.resource::<State>().source_terminal(7).unwrap().tick,
            300
        );
        assert!(world.resource::<State>().source_terminal(8).is_none());
        world.resource_mut::<State>().begin_attempt(8);
        assert!(world.resource::<State>().source_terminal(7).is_none());
        assert!(world.resource::<State>().source_terminal(8).is_none());
        assert!(world.get_entity(ours).is_err());
        assert!(world.get_entity(other).is_ok());
        assert!(!world.resource::<Assets<AudioSource>>().contains(own.id()));
        assert!(
            world
                .resource::<Assets<AudioSource>>()
                .contains(unrelated.id())
        );
        assert!(!world.contains_resource::<Owner>());
    }
}
