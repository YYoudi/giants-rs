//! Declared selector3 pickup, using the ordinary mission's source identities,
//! shared clock/random stream and current rendered Mecc reference service.
use super::{PickupPacket, Runtime};
use giants_game::{
    carried_actor,
    resident_states::{Id, Session},
    smartie_declared::Prepared,
};
use std::sync::Arc;

#[derive(Default)]
pub(super) struct State {
    entries: Vec<Prepared>,
    picked: Option<(usize, Id)>,
    last_frame: Option<u64>,
    catalog: Option<giants_formats::gtext::Catalog>,
}
impl State {
    pub fn prepare(
        root: &std::path::Path,
        plan: &giants_game::instances::Plan,
    ) -> Result<Self, String> {
        let entries = Prepared::mission(plan)?;
        let catalog = if entries.is_empty() {
            None
        } else {
            Some(
                giants_formats::gtext::Catalog::parse(
                    &std::fs::read(
                        root.join("Bin")
                            .join(format!("GText{}.bin", plan.resources().language)),
                    )
                    .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?,
            )
        };
        Ok(Self {
            entries,
            catalog,
            ..Default::default()
        })
    }
    pub fn construct(&self, session: &mut Session) -> Result<(), String> {
        for entry in &self.entries {
            let id = entry.construct(session)?;
            println!(
                "mission_smartie_declared ticket={} placement={} source_offset={} class={} owner={id:?} attached=false source_placement=true",
                session.ticket(),
                entry.key.index,
                entry.source_offset,
                entry.carry.class
            );
        }
        Ok(())
    }
    pub fn carried(&self, session: &Session) -> Result<Option<Id>, String> {
        let Some((index, id)) = self.picked else {
            return Ok(None);
        };
        let actor = session
            .get(id)
            .ok_or("picked Smartie owner absent or foreign")?;
        if actor.class != self.entries[index].carry.class
            || actor.attachment_words().is_none_or(|a| a.behavior != 19)
        {
            return Err("picked Smartie lost its actual carrying relationship".into());
        }
        Ok(Some(id))
    }
    pub fn carried_asset(&self) -> Option<(usize, Arc<carried_actor::Prepared>)> {
        let (index, _) = self.picked?;
        let entry = &self.entries[index];
        Some((entry.source_offset, entry.carry.clone()))
    }
}
impl Runtime {
    pub(crate) fn declared_pickup_ready(&self, ticket: u64) -> bool {
        self.active
            .as_ref()
            .filter(|a| a.ticket == ticket)
            .is_some_and(|active| {
                active.payload.smarties.as_ref().is_ok_and(|state| {
                    state.entries.is_empty()
                        || state
                            .carried(&active.resident_states)
                            .is_ok_and(|id| id.is_some())
                })
            })
    }
    pub(crate) fn deliver_declared_pickup_notice(
        &mut self,
        ticket: u64,
        child: Id,
    ) -> Result<bool, String> {
        let active = self
            .active
            .as_mut()
            .filter(|a| a.ticket == ticket)
            .ok_or("pickup notice ticket absent")?;
        let state = active.payload.smarties.as_ref().map_err(Clone::clone)?;
        if state.carried(&active.resident_states)? != Some(child) {
            return Ok(false);
        }
        let catalog = state
            .catalog
            .as_ref()
            .ok_or("declared pickup language catalog absent")?;
        let (_, clock) = active
            .payload
            .signals
            .current()
            .ok_or("pickup notice clock absent")?;
        let campaign = active
            .campaign
            .as_mut()
            .and_then(|c| c.as_mut().ok())
            .ok_or("declared pickup mission notice owner absent")?;
        active.resident_states.deliver_pickup_notice_to_queue(
            child,
            &mut campaign.progress,
            Some(giants_game::mission_notice::Context {
                prefix: &campaign.program.prefix,
                gates: giants_game::mission_notice::Gates {
                    suppress: [0, 0, u32::from(campaign.completed)],
                    time: 0.,
                },
            }),
            catalog,
            clock.elapsed_seconds,
        )?;
        println!(
            "mission_smartie_pickup_notice ticket={ticket} child={child:?} actual_mission_state=true counter={} pending={} queue_delivery=true",
            campaign.progress.counter,
            active
                .resident_states
                .get(child)
                .unwrap()
                .pickup_notice_pending
        );
        Ok(true)
    }
    pub(crate) fn advance_declared_smartie_pickup(
        &mut self,
        ticket: u64,
        mut point: impl FnMut(&carried_actor::Prepared) -> Result<[f32; 3], String>,
    ) -> Result<(), String> {
        let Some(active) = self.active.as_mut().filter(|a| a.ticket == ticket) else {
            return Ok(());
        };
        let Some((number, clock)) = active.payload.signals.current() else {
            return Ok(());
        };
        let player = active.player_owner()?;
        if active.owned_mecc.is_some() || active.controlled_mecc_owner != Some(player) {
            return Ok(());
        }
        let state = active.payload.smarties.as_mut().map_err(|e| e.clone())?;
        if state.picked.is_some() || state.entries.is_empty() || state.last_frame == Some(number) {
            return Ok(());
        }
        state.last_frame = Some(number);
        if active.pickup.is_some() {
            return Err("declared pickup cannot replace unconsumed services".into());
        }
        for (index, entry) in state.entries.iter().enumerate() {
            let child = active
                .resident_states
                .declaration(entry.key)
                .ok_or("declared Smartie constructor absent")?;
            let f = active
                .resident_states
                .get(child)
                .and_then(|a| a.construction_fields.as_ref())
                .ok_or("Smartie live fields absent")?;
            let areas = active.payload.signals.areas.snapshot(|id| {
                active
                    .resident_states
                    .get(id)
                    .and_then(|a| a.construction_fields.as_ref())
                    .map(|f| f.dispatch_context)
                    .ok_or_else(|| "Smartie slow-area owner absent".into())
            })?;
            let seconds = giants_game::object_time::seconds(
                clock.seconds,
                Some(giants_game::object_time::Sample {
                    owner: f.dispatch_context,
                    position: f.position,
                }),
                None,
                &areas,
            ) as f32;
            let Some(parent) = active
                .resident_states
                .select_declared_smartie_mecc(child, seconds)?
            else {
                continue;
            };
            if parent != player {
                return Err(
                    "declared pickup requires current ordinary controller reference".into(),
                );
            }
            let actual_point = point(&entry.carry)?;
            let before = active.variant_random.0;
            let choice = active.variant_random.next_15() * 100 / 32768;
            // Real shared mission clock boundary, not the command12 start word.
            // Equivalence of native global clock origin/interleaving is unverified.
            let services = active.resident_states.pickup_declared_mecc(
                child,
                parent,
                actual_point,
                clock.elapsed_seconds.to_bits(),
                choice,
            )?;
            state.picked = Some((index, child));
            active.pickup = Some(PickupPacket {
                parent,
                child,
                services,
                random: [before, choice, active.variant_random.0],
            });
            println!(
                "mission_smartie_proximity_pickup ticket={ticket} placement={} class={} parent={parent:?} child={child:?} frame={number} source_radius=7 current_parent_reference=true time_bits={} forced_pose=false source_selector=3 notice_pending=true",
                entry.key.index,
                entry.carry.class,
                clock.elapsed_seconds.to_bits()
            );
            break;
        }
        Ok(())
    }
}
