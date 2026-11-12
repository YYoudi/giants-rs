//! Actual FLK39 -> canonical body -> world-preparation lane. Whole-film effects
//! admission remains with the parent owner; this family does not waive it.
use giants_game::{
    cinematic_timed::{self, Frame},
    resident_states::CinematicScope,
};
use std::sync::Arc;

pub(super) struct Timed {
    bindings: Vec<Option<cinematic_timed::Bindings>>,
    world: Option<Arc<cinematic_timed::world::Prepared>>,
    active: Option<Active>,
}
struct Active {
    director: usize,
    generation: u64,
    queue: cinematic_timed::Queue,
    revision: u64,
    number: u64,
}
impl Timed {
    pub(super) fn new(
        bindings: Vec<Option<cinematic_timed::Bindings>>,
        world: Option<Arc<cinematic_timed::world::Prepared>>,
    ) -> Self {
        Self {
            bindings,
            world,
            active: None,
        }
    }
    pub(super) fn release(
        &mut self,
        ticket: u64,
        runtime: &mut crate::mission_runtime::Runtime,
    ) -> Result<(), String> {
        if let Some(mut active) = self.active.take() {
            if let Some(session) = runtime.cinematic_session_mut(ticket) {
                let count = active.queue.release(session)?;
                println!(
                    "mission_timed_released ticket={ticket} director={} objects={count} remaining_objects={} remaining_contexts={} source_children_suppressed=true",
                    active.director,
                    active.queue.objects(),
                    active.queue.contexts()
                );
            } else {
                println!(
                    "mission_timed_released ticket={ticket} director={} session_already_retired=true",
                    active.director
                );
            }
        }
        Ok(())
    }
    pub(super) fn update(
        &mut self,
        ticket: u64,
        cinematic: &crate::cinematic_camera::Cinematic,
        terrain: &giants_formats::gti::Terrain,
        runtime: &mut crate::mission_runtime::Runtime,
    ) -> Result<(), String> {
        if cinematic.paused() || self.world.is_none() {
            return Ok(());
        }
        let director = cinematic.director_index();
        let generation = cinematic.generation();
        if self
            .active
            .as_ref()
            .is_some_and(|a| a.director != director || a.generation != generation)
        {
            self.release(ticket, runtime)?;
        }
        let Some(binding) = self.bindings.get(director).and_then(Option::as_ref) else {
            return Ok(());
        };
        if self.active.is_none() {
            self.active = Some(Active {
                director,
                generation,
                queue: binding.queue(CinematicScope {
                    ticket,
                    director,
                    generation,
                }),
                revision: 0,
                number: 0,
            });
        }
        let active = self.active.as_mut().unwrap();
        let world = self.world.as_ref().unwrap();
        runtime.with_cinematic_services(ticket,|session,random,stamp,resources,areas| {
            active.number+=1;
            let delta=cinematic.animation_dt();
            let mut services=cinematic_timed::world::World::new(terrain,&world.objects,&[],&world.unresolved,delta,areas)?;
            // The actual existing children walk precedes this frame's new births.
            active.queue.prepare_children(cinematic_timed::WorldStep {
                number:active.number,global_seconds:delta,areas,
                progress:giants_game::actor_finalization::PhysicalProgress::PrototypePs2Zero,
                pc_argument:None,
            },session,stamp,&mut services)?;
            if active.revision!=cinematic.event_revision() {
                for event in cinematic.events().iter().filter(|event|event.kind==giants_game::flick::EventKind::Effect) {
                    active.queue.dispatch(binding,event.offset,resources,random,session)?;
                    println!("mission_timed_dispatched ticket={ticket} director={director} offset={} objects={} source_event=true native_rng_phase_verified=false",event.offset,active.queue.objects());
                }
                active.revision=cinematic.event_revision();
            }
            active.queue.update_attached(binding,cinematic.pose_seconds(),delta,resources,random,session,stamp)?;
            if services.calls.motion>0 {
                println!("mission_timed_world ticket={ticket} director={director} frame={} children={} physical_calls={} environment_calls={} platform_queries={} source_progress=prototype_zero",active.number,active.queue.children().len(),services.calls.motion,services.calls.environment,services.calls.platforms);
            }
            Ok(())
        })
    }
    pub(super) fn frame(
        &self,
        ticket: u64,
        runtime: &crate::mission_runtime::Runtime,
        director: usize,
        offset: usize,
        class: i32,
    ) -> Option<Option<Frame>> {
        self.world.as_ref()?;
        let parent = self.bindings.get(director)?.as_ref()?.role(offset, class)?;
        Some((|| {
            let active = self.active.as_ref().filter(|a| a.director == director)?;
            let session = runtime.cinematic_session(ticket)?;
            if parent {
                active.queue.frame(offset, session)
            } else {
                let child = *active.queue.children().iter().find(|&&child| {
                    active.queue.child_source(child) == Some(offset)
                        && session.get(child).is_some_and(|a| a.class == class)
                })?;
                active.queue.child_frame(child, session)
            }
        })())
    }
}
