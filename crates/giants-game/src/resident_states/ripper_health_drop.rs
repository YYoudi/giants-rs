//! Acquired class785 cold0 -> state37 suffix inside the real lethal callback.
//! No item update, player healing or effect readiness follows from construction.
use super::*;
use crate::{attached_effect_context::Creation, ripper_health_drop::Owned};

impl Session {
    pub(crate) fn ripper_health_geometry_source(
        &self,
        item: Id,
        body: &Arc<crate::specification::Prepared>,
    ) -> Result<u32, String> {
        let actor = self
            .get(item)
            .ok_or("Ripper health bounds owner absent or foreign")?;
        let owned = actor
            .ripper_health_drop
            .ok_or("Ripper health birth absent")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("Ripper health fields absent")?;
        if !owned.initialized
            || owned.failed
            || !actor.acquired_construction
            || actor.class != 785
            || actor.seed.storage.state() != 37
            || fields.configuration[20] != 37
            || fields.object_flags & 0x8000 != 0
            || !self
                .specification(item)
                .is_some_and(|s| Arc::ptr_eq(s, body))
        {
            return Err("Ripper health bounds require this factory's completed live birth".into());
        }
        Ok(actor
            .channels
            .ok_or("Ripper health animation channels absent")?[0][0])
    }
    /// Read current owned item/player eligibility after the caller's primary
    /// gates. Pause, ground and contact remain that primary's work. This
    /// produces no contact receipt, health response, sound or collected state.
    pub fn ripper_health_pickup_eligible(
        &self,
        item: Id,
        player: Id,
        step: u64,
        definition: crate::ripper_health_drop::PickupDefinition,
    ) -> Result<bool, String> {
        self.controlled_mecc_motion(player, step)?;
        let health = self.mecc_health(player)?;
        let actor = self
            .get(item)
            .ok_or("Ripper health item absent or foreign")?;
        let drop = actor
            .ripper_health_drop
            .ok_or("Ripper health source ownership absent")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("Ripper health fields absent")?;
        if actor.class != 785
            || actor.seed.storage.state() != 37
            || !drop.initialized
            || drop.failed
            || actor
                .seed
                .storage
                .data()
                .is_none_or(|p| p.len() != 24 || p[..4] != 36u32.to_le_bytes())
        {
            return Err("Ripper health eligibility requires its completed source birth".into());
        }
        if fields.object_flags & 0x8000 != 0 || fields.control_flags & 6 == 0 {
            return Ok(false);
        }
        let player_fields = self
            .get(player)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap();
        Ok(player_fields.object_flags & 0x4000c000 == 0
            && player_fields.configuration[14] & 1 != 0
            && (drop.owner != player || drop.owner_delay <= 0.)
            && definition.mecc_eligible(health)?)
    }
    pub fn ripper_health_drop(&self, id: Id) -> Option<Owned> {
        self.get(id)?.ripper_health_drop
    }
    pub(crate) fn construct_ripper_health_drop(
        &mut self,
        owner: Id,
        mut input: Construction<'_>,
    ) -> Result<Id, String> {
        let source = self.state23_snapshot(owner)?;
        if !self.ripper_damage_is_running(owner)
            || !matches!(source.class, 73 | 230)
            || source.damage < source.limit
            || source.limit <= 0.
            || !source.damage.is_finite()
            || !source.limit.is_finite()
            || !source.position.into_iter().all(f32::is_finite)
            || self
                .residents
                .values()
                .any(|a| a.ripper_health_drop.is_some_and(|d| d.owner == owner))
        {
            return Err("Ripper health birth requires its unique active lethal callback".into());
        }
        if input.class != 785
            || input.seed.storage.profile() != Profile::Debug
            || input.seed.storage.state() != 0
            || input.seed.storage.data().is_some()
            || !input.seed.diagnostics.is_empty()
            || !input.bindings.is_empty()
        {
            return Err("Ripper health birth requires acquired empty class785/cold0".into());
        }
        // Source A0 only: leave the constructor's AC/B8 history untouched.
        input.fields.position = source.position;
        let mut staged = Self::new(self.ticket);
        staged.next = self.next;
        // The callback passes constructor mask0, not automatic registration.
        // Keep the unconditional common list only, before the state switch.
        let id =
            staged.register_with_mask(input.class, input.seed, input.bindings, input.flags, 0)?;
        let channels = input.fields.channels;
        staged.bind_specification(id, input.specification)?;
        staged.bind_state_resources(id, input.state_resources)?;
        staged.bind_actor_fields(id, input.fields)?;
        let actor = staged.residents.get_mut(&id.serial).unwrap();
        actor.channels = Some(channels);
        let transition = actor.seed.storage.transition(37)?;
        if transition.allocated != 24 || transition.destructor_called {
            return Err("Ripper health state37 allocation outside acquired profile".into());
        }
        let fields = actor.construction_fields.as_mut().unwrap();
        fields.configuration[20] = 37;
        fields.object_flags |= 0x40000000;
        fields.control_flags |= 1;
        let payload = actor.seed.storage.data_mut().unwrap();
        for (i, word) in [36u32, 0, 0, 0, 1f32.to_bits(), 30f32.to_bits()]
            .into_iter()
            .enumerate()
        {
            payload[i * 4..i * 4 + 4].copy_from_slice(&word.to_le_bytes());
        }
        actor.ripper_health_drop = Some(Owned {
            owner,
            amount: 0,
            owner_delay: 1.,
            lifetime: 30.,
            context: None,
            initialized: false,
            failed: false,
        });
        actor.acquired_construction = true;
        self.publish_staged(staged, id)
    }
    pub(crate) fn finish_ripper_health_drop(
        &mut self,
        id: Id,
        owner: Id,
        amount: i32,
        large: bool,
        result: Result<Creation, String>,
    ) -> Result<(), String> {
        let running = self.ripper_damage_is_running(owner);
        let actor = self
            .residents
            .get_mut(&id.serial)
            .filter(|_| id.ticket == self.ticket)
            .ok_or("Ripper health effect removed its birth owner")?;
        let drop = actor
            .ripper_health_drop
            .as_mut()
            .ok_or("Ripper health metadata removed")?;
        if drop.owner != owner || drop.failed || drop.initialized || !running {
            drop.failed = true;
            return Err("Ripper health suffix lost its active birth scope".into());
        }
        match result {
            Ok(creation) => {
                drop.context = match creation {
                    Creation::Prepared(context) => Some(context),
                    Creation::Absent => None,
                };
                drop.amount = amount;
                actor.seed.storage.data_mut().unwrap()[4..8]
                    .copy_from_slice(&(amount as u32).to_le_bytes());
                actor.construction_fields.as_mut().unwrap().control_flags |=
                    if large { 4 } else { 2 };
                drop.initialized = true;
                Ok(())
            }
            Err(error) => {
                drop.failed = true;
                Err(error)
            }
        }
    }
}
