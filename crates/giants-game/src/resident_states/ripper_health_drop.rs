//! Acquired class785 cold0 -> state37 suffix inside the real lethal callback.
//! No item update, player healing or effect readiness follows from construction.
use super::*;
use crate::{attached_effect_context::Creation, ripper_health_drop::Owned};
mod collection;
#[cfg(test)]
mod collection_tests;

impl Session {
    pub(crate) fn advance_ripper_health_prefix(
        &mut self,
        item: Id,
        body: &Arc<crate::specification::Prepared>,
        geometry: &crate::resident_environment::Resources,
        input: crate::ripper_health_drop::PrimaryInput<'_>,
        services: &mut impl crate::ripper_health_drop::PrimaryServices,
    ) -> Result<crate::ripper_health_drop::PrimaryPrefix, String> {
        use crate::ripper_health_drop::{PendingCollection, PrimaryPhase, PrimaryPrefix, primary};
        self.ripper_health_geometry_source(item, body)?;
        let actor = self.get(item).unwrap();
        let fields = actor.construction_fields.as_ref().unwrap();
        let owned = actor.ripper_health_drop.unwrap();
        if owned.primary_phase != PrimaryPhase::Idle
            || owned.primary_step.is_some_and(|old| input.step <= old)
            || !input.global_seconds.is_finite()
            || !(0. ..=60.).contains(&input.global_seconds)
            || !fields
                .position
                .into_iter()
                .chain(fields.current)
                .chain(fields.angles)
                .chain(owned.velocity)
                .chain([owned.floor, owned.owner_delay, owned.lifetime])
                .all(f32::is_finite)
            || fields.angles.into_iter().any(|a| a.abs() > 1_000_000.)
            || input.areas.iter().any(|a| {
                !a.center
                    .into_iter()
                    .chain([a.radius_squared, a.factor])
                    .all(f32::is_finite)
                    || a.radius_squared < 0.
                    || !(0. ..=1.).contains(&a.factor)
            })
            || fields.object_flags & 0xc000 != 0
            || fields.control_flags & 1 == 0
            || fields.control_flags & 6 == 0
        {
            return Err("health primary requires an unconsumed finite live source birth".into());
        }
        let actor = self.residents.get_mut(&item.serial).unwrap();
        let owned = actor.ripper_health_drop.as_mut().unwrap();
        owned.primary_step = Some(input.step);
        owned.primary_phase = PrimaryPhase::Running;
        actor.construction_fields.as_mut().unwrap().object_flags |= 0x40;
        let result = (|| {
            let actor = self.get(item).unwrap();
            let fields = actor.construction_fields.as_ref().unwrap();
            let seconds = crate::object_time::seconds(
                input.global_seconds,
                Some(crate::object_time::Sample {
                    owner: fields.dispatch_context,
                    position: fields.position,
                }),
                None,
                input.areas,
            ) as f32;
            let mut durations = vec![None; 256];
            for (&slot, descriptor) in &body.animations {
                durations[usize::from(slot)] = Some(descriptor.template.duration);
            }
            // These model rates are the health constructor's actual zeros;
            // this primary never writes them or runs common actor motion.
            let tick = self.advance_channels(
                item,
                &durations,
                [0.; 3],
                seconds,
                crate::arithmetic::Precision::Double53,
            )?;
            if tick.missing != 0 {
                return Err("health animation descriptor absent".into());
            }
            self.prepare_actor_affine(item)?;
            let slot = self.get(item).unwrap().channels.unwrap()[0][0];
            let selected = geometry.bounds(slot)?;
            let affine = self.actor_current_affine(item)?;
            let bounds = crate::collision_bounds::cached(
                Some(crate::collision_bounds::Models {
                    primary: selected.primary,
                    auxiliary: selected.current_auxiliary,
                    slot,
                }),
                affine.matrix[..12].try_into().unwrap(),
                crate::arithmetic::Precision::Double53,
            )?;
            let actor = self.get(item).unwrap();
            let mut state = primary::State {
                fields: actor.construction_fields.as_ref().unwrap().clone(),
                owned: actor.ripper_health_drop.unwrap(),
            };
            state.fields.channels = actor.channels.unwrap();
            let result = primary::advance(
                &mut state,
                &input,
                &mut HealthPrimaryAdapter {
                    session: self,
                    item,
                    step: input.step,
                    services,
                },
            );
            self.publish_health_primary(item, input.step, &state)?;
            result.map(|()| bounds)
        })();
        let owned = self
            .residents
            .get_mut(&item.serial)
            .unwrap()
            .ripper_health_drop
            .as_mut()
            .unwrap();
        match result {
            Err(error) => {
                owned.failed = true;
                owned.primary_phase = PrimaryPhase::Failed;
                Err(error)
            }
            Ok(_) if input.paused => {
                owned.primary_phase = PrimaryPhase::Idle;
                Ok(PrimaryPrefix::Paused)
            }
            Ok(bounds) => {
                owned.primary_phase = PrimaryPhase::CollectionPending;
                Ok(PrimaryPrefix::Collection(PendingCollection {
                    item,
                    step: input.step,
                    bounds,
                }))
            }
        }
    }
    fn publish_health_primary(
        &mut self,
        item: Id,
        step: u64,
        state: &crate::ripper_health_drop::primary::State,
    ) -> Result<(), String> {
        let actor = self
            .residents
            .get_mut(&item.serial)
            .filter(|_| item.ticket == self.ticket)
            .ok_or("health primary owner removed")?;
        let owned = actor
            .ripper_health_drop
            .as_ref()
            .ok_or("health primary ownership removed")?;
        if owned.failed
            || owned.primary_step != Some(step)
            || owned.primary_phase != crate::ripper_health_drop::PrimaryPhase::Running
        {
            return Err("health primary scope changed".into());
        }
        actor.construction_fields = Some(state.fields.clone());
        actor.ripper_health_drop = Some(state.owned);
        let data = actor
            .seed
            .storage
            .data_mut()
            .ok_or("health primary payload absent")?;
        data[16..20].copy_from_slice(&state.owned.owner_delay.to_le_bytes());
        data[20..24].copy_from_slice(&state.owned.lifetime.to_le_bytes());
        Ok(())
    }
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
        let current_collection = drop.primary_phase
            == crate::ripper_health_drop::PrimaryPhase::Collecting
            && drop.primary_step == Some(step)
            && drop.primary_recipient == Some(player);
        if (fields.object_flags & 0x8000 != 0 && !current_collection)
            || fields.control_flags & 6 == 0
        {
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
            && (drop.exclusion_owner != Some(player) || drop.owner_delay <= 0.)
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
            velocity: [0.; 3],
            floor: 0.,
            primary_phase: crate::ripper_health_drop::PrimaryPhase::Idle,
            primary_step: None,
            primary_recipient: None,
            exclusion_owner: None,
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

struct HealthPrimaryAdapter<'a, S> {
    session: &'a mut Session,
    item: Id,
    step: u64,
    services: &'a mut S,
}
impl<S: crate::ripper_health_drop::PrimaryServices> crate::ripper_health_drop::primary::Services
    for HealthPrimaryAdapter<'_, S>
{
    fn ground(&mut self, state: &crate::ripper_health_drop::primary::State) -> Result<f32, String> {
        self.session
            .publish_health_primary(self.item, self.step, state)?;
        self.services
            .ground(self.session, self.item, state.fields.position)
    }
    fn blocked(
        &mut self,
        state: &crate::ripper_health_drop::primary::State,
        from: [f32; 3],
    ) -> Result<bool, String> {
        self.session
            .publish_health_primary(self.item, self.step, state)?;
        self.services
            .blocked(self.session, self.item, from, state.fields.position)
    }
    fn expired(&mut self, state: &crate::ripper_health_drop::primary::State) -> Result<(), String> {
        self.session
            .publish_health_primary(self.item, self.step, state)?;
        self.services.expired(self.session, self.item)
    }
}
