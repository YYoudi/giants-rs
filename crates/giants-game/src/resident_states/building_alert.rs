//! Typed consumer of the acquired building-origin alerts. Prototype PS2
//! `rpman_alert_team__FP6OBJECTf` and `barracks_alert_rippers__FP6OBJECT` remain
//! unknown-attribution navigation candidates; see building_barracks.
use super::{Id, Session};
use crate::building_barracks::{self, Candidate, Data};

impl Session {
    fn building_alert_candidates(&self) -> Result<Vec<(Id, Candidate<Id>)>, String> {
        let mut result = Vec::new();
        // This is the actual constructor list, including runtime allocations,
        // not a class census or an invented empty-neighbour acknowledgement.
        for id in self.list(1) {
            let actor = self.get(id).ok_or("building alert list owner absent")?;
            let state = actor.seed.storage.state();
            if actor.class != 215 && state != 23 {
                continue;
            }
            if !actor.acquired_construction
                || actor.ripper_primary_running
                || actor.world_tick_running
                || actor
                    .ripper_primary
                    .as_ref()
                    .is_some_and(|m| m.failure().is_some())
            {
                return Err("building alert requires idle acquired list owners".into());
            }
            let fields = actor
                .construction_fields
                .as_ref()
                .ok_or("building alert fields absent")?;
            let data = if actor.class == 215 {
                let payload = actor
                    .seed
                    .storage
                    .data()
                    .ok_or("team alert payload absent")?;
                if state != 51 || payload.len() != 576 || payload[4..8] != [0; 4] {
                    return Err("team alert outside acquired typed class215 payload".into());
                }
                let word = |at| u32::from_le_bytes(payload[at..at + 4].try_into().unwrap());
                Data::Reaperman {
                    opaque_0: word(0),
                    target: actor.cold_alert_target,
                    flags_1b8: word(0x1b8),
                    opaque_1f0: word(0x1f0),
                    opaque_228: word(0x228),
                }
            } else {
                Data::Ripper {
                    target: self.state23_snapshot(id)?.retaliation,
                }
            };
            let target = match data {
                Data::Ripper { target } | Data::Reaperman { target, .. } => target,
                Data::Other => unreachable!(),
            };
            if target.is_some_and(|target| self.get(target).is_none()) {
                return Err("building alert retained target absent or foreign".into());
            }
            result.push((
                id,
                Candidate {
                    class: actor.class,
                    state: u32::from(state),
                    mode: fields.action,
                    control_flags: fields.control_flags,
                    position: fields.position,
                    data,
                },
            ));
        }
        Ok(result)
    }
    /// Current typed target, without creating a combat state or acknowledging
    /// an NPC callback. Foreign identities and unsupported payloads are errors.
    pub fn building_alert_target(&self, id: Id) -> Result<Option<Id>, String> {
        let actor = self
            .get(id)
            .ok_or("building alert actor absent or foreign")?;
        if actor.seed.storage.state() == 23 {
            return Ok(self.state23_snapshot(id)?.retaliation);
        }
        if actor.class == 215 && actor.seed.storage.state() == 51 && actor.acquired_construction {
            return Ok(actor.cold_alert_target);
        }
        Err("building alert target outside acquired actor profiles".into())
    }
    fn apply_building_alert(
        &mut self,
        building: Id,
        radius: f32,
        target: Id,
        team: bool,
    ) -> Result<(), String> {
        self.get(target)
            .ok_or("building alert target absent or foreign")?;
        let source = self
            .get(building)
            .ok_or("building alert source absent or foreign")?;
        if !source.acquired_construction
            || source.seed.storage.state() != 98
            || !matches!(source.class, 1060 | 1226)
        {
            return Err("building alert source outside acquired barracks profile".into());
        }
        let position = source
            .construction_fields
            .as_ref()
            .ok_or("building alert source fields absent")?
            .position;
        let owners = self.building_alert_candidates()?;
        let mut candidates: Vec<_> = owners.iter().map(|(_, candidate)| *candidate).collect();
        if team {
            building_barracks::alert_team_from_building(position, radius, target, &mut candidates)?;
        } else {
            building_barracks::alert_rippers(position, radius, target, &mut candidates)?;
        }
        // Every fallible owner/payload check precedes the first write. Retain
        // the source list order; a caller still owns complete mission admission.
        for ((id, _), candidate) in owners.into_iter().zip(candidates) {
            let actor = self.residents.get_mut(&id.serial).unwrap();
            actor.construction_fields.as_mut().unwrap().control_flags = candidate.control_flags;
            let selected = match candidate.data {
                Data::Ripper { target } | Data::Reaperman { target, .. } => target,
                Data::Other => unreachable!(),
            };
            if candidate.state == 23 {
                if let Some(combat) = &mut actor.state23_combat {
                    combat.retaliation = selected;
                } else {
                    actor.cold_alert_target = selected;
                }
                if let (Some(primary), Some(target)) = (&mut actor.ripper_primary, selected) {
                    primary.bind_alert_target(target);
                }
            } else {
                actor.cold_alert_target = selected;
            }
        }
        Ok(())
    }
    /// Building-origin team write over the current owned constructor list.
    /// The caller must establish complete admission of source-eligible actors;
    /// this operation does not execute their AI or admit an ordinary shot.
    pub fn alert_team_from_building(
        &mut self,
        building: Id,
        radius: f32,
        target: Id,
    ) -> Result<(), String> {
        self.apply_building_alert(building, radius, target, true)
    }
    /// Separate strict-radius, empty-target-only Ripper write. Call after the
    /// team helper in the acquired shot order, using the same actual target.
    pub fn alert_rippers_from_building(
        &mut self,
        building: Id,
        radius: f32,
        target: Id,
    ) -> Result<(), String> {
        self.apply_building_alert(building, radius, target, false)
    }
}
