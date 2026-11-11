//! Actual ticket-to-ticket continuation after owned objective completion.
//! No CPU success, preview replay, timer or visual inspection grants completion.
use bevy::prelude::*;
use giants_game::{mission_loading::Phase, mission_selection::LoadRequest};
#[derive(Resource, Default)]
struct State {
    pending: Option<(u64, LoadRequest)>,
    failed: Option<u64>,
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
    if phase != Phase::Gameplay {
        return;
    }
    match world
        .resource_mut::<crate::mission_runtime::Runtime>()
        .take_campaign_completion(ticket)
    {
        Ok(Some(completion)) => {
            let result = world.resource_scope(|world, mut images: Mut<Assets<Image>>| {
                world
                    .resource_mut::<crate::frontend_scene::Scene>()
                    .advance_campaign(&completion, &mut images)
            });
            match result {
                Ok(next) => {
                    // Every old owner receives the same ticket release used by
                    // menu cancellation; a new request alone is not its ack.
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
                Err(e) => {
                    println!("mission_campaign_transition_failed ticket={ticket} reason={e}");
                    world.resource_mut::<State>().failed = Some(ticket);
                }
            }
        }
        Ok(None) => {}
        Err(e) => {
            println!("mission_campaign_transition_blocked ticket={ticket} reason={e}");
            world.resource_mut::<State>().failed = Some(ticket);
        }
    }
}
