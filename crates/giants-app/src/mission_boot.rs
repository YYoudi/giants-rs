//! Admission reports actual resident capabilities, not campaign feature completion.
use bevy::prelude::*;
use giants_game::mission_loading::Phase;

#[derive(Resource)]
pub(crate) struct Automatic(pub bool);
impl Default for Automatic {
    fn default() -> Self {
        Self(true)
    }
}
#[derive(Resource, Default)]
struct State {
    ticket: Option<u64>,
    admitted: bool,
    controller: bool,
    finished: bool,
    refusal: Option<String>,
}
pub struct Plugin;
impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Automatic>()
            .init_resource::<State>()
            .add_systems(
                Update,
                update
                    .after(crate::mission_player::PlayerUpdate)
                    .before(crate::source_view::ViewUpdate)
                    .run_if(crate::mecc_session::frontend_available),
            );
    }
}
pub(crate) fn candidate(world: &World) -> bool {
    world.resource::<Automatic>().0
        && world
            .resource::<crate::mission_runtime::Runtime>()
            .startup()
            .is_ok_and(|s| s.is_some())
}
pub(crate) fn update(world: &mut World) {
    if !world.resource::<Automatic>().0 {
        return;
    }
    if crate::mission_showcase::free(world) {
        return;
    }
    let frontend = world.resource::<crate::frontend_scene::Scene>();
    let ticket = frontend.flow.ticket;
    let phase = frontend.flow.phase;
    if world.resource::<State>().ticket != Some(ticket) {
        *world.resource_mut::<State>() = State {
            ticket: Some(ticket),
            ..default()
        };
    }
    if !matches!(
        phase,
        Phase::AwaitingRuntime | Phase::Cinematic | Phase::Objectives
    ) {
        return;
    }
    let startup = match world
        .resource::<crate::mission_runtime::Runtime>()
        .startup()
    {
        Ok(Some(s)) if s.ticket == ticket => s,
        Ok(_) => return,
        Err(reason) => {
            let mut state = world.resource_mut::<State>();
            if state.refusal.as_ref() != Some(&reason) {
                println!("mission_admission_blocked ticket={ticket} reason={reason}");
                state.refusal = Some(reason);
            }
            return;
        }
    };
    if world
        .resource::<crate::mission_environment::State>()
        .mounted_ticket()
        != Some(ticket)
        || !world.resource::<crate::PipelinesReady>().0
        || !crate::mission_player::resident(world, ticket)
    {
        return;
    }
    use crate::frontend_scene::MissionRuntimeEvent as Event;
    if phase == Phase::AwaitingRuntime && !world.resource::<State>().admitted {
        if startup.cinematic
            && (!crate::mission_film::drawn(world)
                || !world
                    .get_resource::<crate::cinematic_camera::Cinematic>()
                    .is_some_and(|c| c.cameras_bound())
                || !world.contains_resource::<crate::mission_effects::State>())
        {
            return;
        }
        world.write_message(Event::Ready {
            ticket,
            cinematic: startup.cinematic,
            gameplay_ready: false,
        });
        world.resource_mut::<State>().admitted = true;
        println!(
            "mission_runtime_ready ticket={ticket} cinematic={} resident_controller=true requested_companions={} feature_completion=false",
            startup.cinematic, startup.companions
        );
    }
    if matches!(phase, Phase::Cinematic | Phase::Objectives)
        && !world.resource::<State>().controller
    {
        world.write_message(Event::GameplayReady { ticket });
        world.resource_mut::<State>().controller = true;
        println!(
            "mission_runtime_gameplay_ready ticket={ticket} capability=resident_selected_controller weapons_team_interactions_complete=false"
        );
    }
    if phase == Phase::Cinematic && !world.resource::<State>().finished {
        let end = world
            .get_resource::<crate::cinematic_camera::Cinematic>()
            .is_some_and(|c| c.end_reached() && c.release_ready(false));
        let effects = world
            .get_resource::<crate::mission_effects::State>()
            .is_some_and(|s| s.settled());
        let voices = world
            .query_filtered::<Option<&AudioSink>, With<crate::cinematic_audio::PlayingVoice>>()
            .iter(world)
            .all(|s| s.is_some_and(|s| s.empty()));
        if end && effects && voices {
            world.write_message(Event::CinematicFinished { ticket });
            world.resource_mut::<State>().finished = true;
            println!(
                "mission_runtime_cinematic_finished ticket={ticket} source_end=true voices_drained=true effects_settled=true"
            );
        }
    }
}
