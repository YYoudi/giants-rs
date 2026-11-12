//! Actual ticket-to-ticket continuation after owned objective completion.
//! No CPU success, preview replay, timer or visual inspection grants completion.
use bevy::prelude::*;
use giants_game::{mission_loading::Phase, mission_selection::LoadRequest};
#[derive(Resource, Default)]
struct State {
    pending: Option<(u64, LoadRequest)>,
    failed: Option<u64>,
    film: Option<Film>,
}
pub struct Plugin;
impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<State>().add_systems(
            Update,
            update
                .after(crate::mission_player::PlayerUpdate)
                .after(crate::mission_boot::update)
                .run_if(crate::mecc_session::frontend_available),
        );
    }
}
fn update(world: &mut World) {
    let scene = world.resource::<crate::frontend_scene::Scene>();
    let ticket = scene.flow.ticket;
    let phase = scene.flow.phase;
    if world.resource::<State>().failed == Some(ticket) {
        return;
    }
    if let Some((pending, request)) = world.resource::<State>().pending.clone() {
        if pending != ticket {
            world.resource_mut::<State>().pending = None;
        } else if matches!(
            phase,
            Phase::Cinematic | Phase::Objectives | Phase::Gameplay
        ) {
            let result = world
                .resource_mut::<crate::frontend_scene::Scene>()
                .record_campaign_reached(ticket, &request);
            match result {
                Ok(()) => {
                    println!(
                        "mission_campaign_reached ticket={ticket} name={} sidecar_saved=true actor_census_written=false",
                        request.mission_name
                    );
                    world.resource_mut::<State>().pending = None;
                }
                Err(e) => {
                    println!("mission_campaign_progress_failed ticket={ticket} reason={e}");
                    world.resource_mut::<State>().failed = Some(ticket);
                }
            }
        }
    }
    if drive_film(world) || phase != Phase::Gameplay {
        return;
    }
    match world
        .resource_mut::<crate::mission_runtime::Runtime>()
        .take_campaign_completion(ticket)
    {
        Ok(Some(completion)) if completion.film.is_some() => {
            let root = world
                .resource::<crate::frontend_scene::Scene>()
                .data_root()
                .to_owned();
            match world
                .resource::<crate::mission_runtime::Runtime>()
                .prepare_campaign_film(ticket, &root)
            {
                Ok(rx) => {
                    println!(
                        "mission_campaign_completion_preparing ticket={ticket} film={} source_objectives=true",
                        completion.film.as_deref().unwrap()
                    );
                    world.resource_mut::<State>().film = Some(Film {
                        completion,
                        stage: Stage::Preparing(std::sync::Mutex::new(rx)),
                    });
                }
                Err(e) => fail(world, ticket, e),
            }
        }
        Ok(Some(completion)) => transition(world, completion, None),
        Ok(None) => {}
        Err(e) => fail(world, ticket, e),
    }
}
struct Film {
    completion: crate::mission_runtime::CampaignCompletion,
    stage: Stage,
}
enum Stage {
    Preparing(
        std::sync::Mutex<
            std::sync::mpsc::Receiver<Result<crate::mission_opening::Prepared, String>>,
        >,
    ),
    Playing,
}
fn fail(world: &mut World, ticket: u64, error: String) {
    println!("mission_campaign_transition_failed ticket={ticket} reason={error}");
    world.resource_mut::<State>().failed = Some(ticket);
}
fn drive_film(world: &mut World) -> bool {
    let Some(mut film) = world.resource_mut::<State>().film.take() else {
        return false;
    };
    let ticket = film.completion.ticket;
    let scene = world.resource::<crate::frontend_scene::Scene>();
    if scene.flow.ticket != ticket || matches!(scene.flow.phase, Phase::Menu | Phase::Failed) {
        println!("mission_campaign_completion_cancelled ticket={ticket} cpu_result_discarded=true");
        return true;
    }
    match &film.stage {
        Stage::Preparing(rx) => {
            let ready = match rx.lock().expect("owned completion channel").try_recv() {
                Ok(result) => result.map(Some),
                Err(std::sync::mpsc::TryRecvError::Empty) => Ok(None),
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    Err("completion preparation worker disconnected".into())
                }
            };
            match ready {
                Ok(Some(prepared)) => {
                    let result = (|| {
                        world
                            .resource_mut::<crate::mission_runtime::Runtime>()
                            .install_campaign_film(ticket, prepared)?;
                        world
                            .resource_mut::<crate::mission_opening::State>()
                            .arm_completion(ticket)?;
                        crate::mission_boot::begin_completion(world, ticket)?;
                        world
                            .resource_mut::<crate::frontend_scene::Scene>()
                            .begin_campaign_film(&film.completion)
                    })();
                    if let Err(e) = result {
                        fail(world, ticket, e);
                        return true;
                    }
                    println!(
                        "mission_campaign_completion_started ticket={ticket} film={} previous_readiness_preserved=true source_end_simulated=false",
                        film.completion.film.as_deref().unwrap()
                    );
                    film.stage = Stage::Playing;
                }
                Ok(None) => {}
                Err(e) => {
                    fail(world, ticket, e);
                    return true;
                }
            }
        }
        Stage::Playing => {
            if let Some(receipt) = world
                .resource::<crate::mission_opening::State>()
                .completion_receipt(ticket)
            {
                println!(
                    "mission_campaign_completion_finished ticket={ticket} source_end={} explicit_user_skip={}",
                    receipt.source_end, !receipt.source_end
                );
                transition(world, film.completion, Some(receipt));
                return true;
            }
        }
    }
    world.resource_mut::<State>().film = Some(film);
    true
}
fn transition(
    world: &mut World,
    completion: crate::mission_runtime::CampaignCompletion,
    receipt: Option<crate::mission_opening::CompletionReceipt>,
) {
    let ticket = completion.ticket;
    let result = world.resource_scope(|world, mut images: Mut<Assets<Image>>| {
        world
            .resource_mut::<crate::frontend_scene::Scene>()
            .advance_campaign(&completion, &mut images, receipt)
    });
    match result {
        Ok(next) => {
            world.write_message(crate::frontend_scene::MissionCancelled(ticket));
            println!(
                "mission_campaign_transition previous_ticket={ticket} next_ticket={next} from={} to={} counter={} required={} carried_before={} meat_reset_on_next_source_load=true source_record=true",
                completion.current.mission_name,
                completion.next.mission_name,
                completion.counter,
                completion.required,
                completion.carried
            );
            world.resource_mut::<State>().pending = Some((next, completion.next));
        }
        Err(e) => fail(world, ticket, e),
    }
}
