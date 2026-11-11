//! Owned ordinary Bevy mission objectives. This owner uses acquired PC
//! predicates and source records. It does not install snapshots for the native
//! world's still-unbound music/precheck, failure or impact-suppression services.
use giants_game::{
    mission_notice, mission_objectives,
    resident_states::{Id, Session},
};

pub(super) struct State {
    pub program: mission_objectives::Program,
    pub progress: mission_notice::State,
    pub completed: bool,
    pub queued: bool,
}
impl State {
    pub fn prepare(
        packet: &giants_game::mission_loading::PreparedMission,
        plan: &giants_game::instances::Plan,
    ) -> Result<Self, String> {
        Ok(Self {
            program: mission_objectives::Program::prepare(packet, plan)?,
            progress: Default::default(),
            completed: false,
            queued: false,
        })
    }
    pub fn advance(
        &mut self,
        session: &mut Session,
        player: Id,
        catalog: &giants_formats::gtext::Catalog,
        now: f32,
    ) -> Result<(), String> {
        if self.completed {
            return Ok(());
        }
        let actor = session.get(player).ok_or("objective player absent")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("objective player fields absent")?;
        let input = mission_objectives::Player {
            state: actor.seed.storage.state(),
            position: fields.position,
            resource: session.mecc_meat(player)?,
        };
        let context = mission_notice::Context {
            prefix: &self.program.prefix,
            gates: mission_notice::Gates {
                suppress: [0; 3],
                time: 0.,
            },
        };
        for (key, target) in &mut self.program.targets {
            if let Some(request) = mission_objectives::proximity(
                target,
                Some(input),
                self.program.quota.unwrap_or(0),
                &mut self.progress,
                context,
            )? {
                session.post_mission_lookup(&request, catalog, now)?;
                println!(
                    "mission_objective_proximity ticket={} placement={} counter={} required={} carried={} source_predicate=true",
                    player.ticket(),
                    key.index,
                    self.progress.counter,
                    self.program.required,
                    input.resource
                );
            }
        }
        if self.progress.counter as i32 >= self.program.required {
            self.completed = true;
            println!(
                "mission_objectives_completed ticket={} scenario={} counter={} required={} source_producers=true transition_pending=true",
                player.ticket(),
                self.program.scenario_index,
                self.progress.counter,
                self.program.required
            );
        }
        Ok(())
    }
}
