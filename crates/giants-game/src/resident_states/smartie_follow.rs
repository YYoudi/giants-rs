//! Live fields for the acquired fresh selector3 proximity call. The caller still
//! owns source primary scheduling, pickup, animation, audio and mission notices.
use super::{Id, Session};
use crate::smartie_follow::{self, Candidate, Child, State};
impl Session {
    pub fn select_declared_smartie_mecc(
        &mut self,
        child: Id,
        seconds: f32,
    ) -> Result<Option<Id>, String> {
        let actor = self
            .get(child)
            .ok_or("Smartie selector actor absent or foreign")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("Smartie selector fields absent")?;
        if actor.seed.storage.state() != 18
            || fields.action != 3
            || fields.control_flags & 8 != 0
            || actor.attachment.is_some()
            || !actor.acquired_construction
        {
            return Err("Smartie selector requires fresh acquired selector3 actor".into());
        }
        let data = actor
            .seed
            .storage
            .data()
            .ok_or("Smartie selector data absent")?;
        if data.len() != 200 {
            return Err("Smartie selector payload profile differs".into());
        }
        let mut state = State {
            cooldown: f32::from_le_bytes(data[0x38..0x3c].try_into().unwrap()),
            reset_reference: None,
        };
        let input = Child {
            selector: fields.action,
            flags: fields.control_flags | 0x20,
            position: fields.position,
            parent_team: None,
        };
        let mut players = Vec::new();
        for id in self.list(1) {
            let candidate = self.get(id).ok_or("Smartie player list identity absent")?;
            if !matches!(candidate.seed.storage.state(), 2 | 3) {
                continue;
            }
            if candidate.seed.storage.state() != 2 {
                return Err("Smartie selector requires acquired Mecc player fields".into());
            }
            let f = candidate
                .construction_fields
                .as_ref()
                .ok_or("Smartie player fields absent")?;
            let health = self.mecc_health(id)?;
            // Team is not read in this admitted flags&8==0 profile. An inventory
            // gate requiring an unavailable live item remains a service error.
            players.push(Candidate {
                id,
                state: 2,
                control_flags: f.control_flags,
                response_flags: f.response_flags,
                object_flags: f.object_flags,
                damage: health.damage,
                damage_limit: health.limit,
                team: 0,
                position: f.position,
                inventory_index: None,
            });
        }
        let result =
            smartie_follow::smartie_check_follow(&mut state, &input, seconds, 7., &players);
        // Native prefix/cooldown writes survive a later missing-service error.
        let actor = self.residents.get_mut(&child.serial).unwrap();
        actor.construction_fields.as_mut().unwrap().control_flags |= 0x20;
        actor.seed.storage.data_mut().unwrap()[0x38..0x3c]
            .copy_from_slice(&state.cooldown.to_le_bytes());
        result.map_err(str::to_owned)
    }
}
