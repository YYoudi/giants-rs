//! Owned resource-action prefix. Prototype PS2 `kabuto_move` is an unknown
//! parent navigation route, not an attributed name for this PC interval.
use super::*;
use crate::kabuto_action_charge as charge;

impl Session {
    pub(crate) fn captured_kabuto_action_charge(
        &self,
        owner: Id,
        resources: &Resources,
        number: u64,
    ) -> Result<charge::View, String> {
        self.captured_kabuto_reaction_admission(owner, resources, number)?;
        let p = &self.kabuto_group(owner).unwrap().charge;
        if p.running || p.completed != Some(number) {
            return Err("Kabuto action charge is not completed on this owner/frame".into());
        }
        p.view
            .ok_or_else(|| "Kabuto action charge view absent".into())
    }
    pub(crate) fn advance_kabuto_action_charge(
        &mut self,
        owner: Id,
        resources: &Resources,
        number: u64,
        world: &mut impl charge::Services,
    ) -> Result<charge::View, String> {
        let reaction = self.captured_kabuto_reaction_admission(owner, resources, number)?;
        if reaction.next != crate::kabuto_reaction_admission::Next::ResourceCharge {
            return Err("Kabuto reaction requires its directed selection/tail consumer".into());
        }
        let g = self.kabuto_group(owner).unwrap();
        if g.charge.entered.is_some()
            || g.action_select_entered.is_some()
            || g.held_reference_entered.is_some()
            || g.animation_commit_entered.is_some()
            || g.primary_tail_entered.is_some()
        {
            return Err("Kabuto action charge requires unconsumed ordered caller work".into());
        }
        let decision = g
            .resource_phase_view
            .and_then(|v| v.resources)
            .ok_or("Kabuto action charge retained resource decision absent")?;
        let input = charge::Input {
            earlier_behavior: decision.initial_mode,
            previous_definition: decision.previous_selection as i32,
            charge: decision.charge,
        };
        // Later sound/transport callbacks can change this alias. Returning to
        // the earlier resource/query snapshot would discard their effects.
        let mut caller = reaction.caller;
        let entry_flags = u32::from_le_bytes(
            self.get(owner).unwrap().seed.storage.data().unwrap()[..4]
                .try_into()
                .unwrap(),
        );
        let p = &mut self
            .residents
            .get_mut(&owner.serial)
            .unwrap()
            .kabuto_group
            .as_mut()
            .unwrap()
            .charge;
        p.entered = Some(number);
        p.running = true;
        p.caller = Some(caller);
        let result = charge::advance(
            &mut caller,
            input,
            &mut Adapter {
                session: self,
                owner,
                resources,
                number,
                world,
            },
        )
        .map(|next| charge::View {
            owner,
            number,
            input,
            entry_flags,
            caller,
            next,
        });
        if let Some(g) = self
            .residents
            .get_mut(&owner.serial)
            .and_then(|a| a.kabuto_group.as_mut())
            .filter(|g| {
                g.owner == owner
                    && g.charge.entered == Some(number)
                    && Arc::ptr_eq(&g.declared_class, &resources.specification)
            })
        {
            g.charge.running = false;
            g.charge.caller = Some(caller);
            match &result {
                Ok(view) => {
                    g.charge.completed = Some(number);
                    g.charge.view = Some(*view);
                }
                Err(error) => {
                    g.geometry_transition_failure
                        .get_or_insert_with(|| error.clone());
                }
            }
        }
        result?;
        self.captured_kabuto_action_charge(owner, resources, number)
    }
}

struct Adapter<'a, W> {
    session: &'a mut Session,
    owner: Id,
    resources: &'a Resources,
    number: u64,
    world: &'a mut W,
}
impl<W: charge::Services> Adapter<'_, W> {
    fn check(&self) -> Result<(), String> {
        self.session
            .check_kabuto_reference_owner(self.owner, self.resources, self.number)?;
        let g = self.session.kabuto_group(self.owner).unwrap();
        if !g.charge.running
            || g.charge.entered != Some(self.number)
            || g.reaction.completed != Some(self.number)
            || g.physics.is_none()
        {
            return Err("Kabuto action charge owner/phase changed during a provider".into());
        }
        Ok(())
    }
    fn word(&self, at: usize) -> Result<u32, String> {
        self.check()?;
        let p = self
            .session
            .get(self.owner)
            .unwrap()
            .seed
            .storage
            .data()
            .unwrap();
        Ok(u32::from_le_bytes(p[at..at + 4].try_into().unwrap()))
    }
    fn store(&mut self, at: usize, value: u32) -> Result<(), String> {
        self.check()?;
        let a = self.session.residents.get_mut(&self.owner.serial).unwrap();
        let p = a.seed.storage.data_mut().unwrap();
        p[at..at + 4].copy_from_slice(&value.to_le_bytes());
        a.kabuto_group.as_mut().unwrap().physics =
            Some(crate::kabuto_physics::Physics::from_owned_payload(p));
        Ok(())
    }
    fn definition(&self, key: u32) -> Result<&crate::kabuto_animation_cache::Definition, String> {
        self.check()?;
        let cache = self
            .session
            .kabuto_group(self.owner)
            .unwrap()
            .cold_animation_cache
            .as_ref()
            .ok_or("Kabuto action charge cold definitions absent")?;
        if !Arc::ptr_eq(cache.specification(), &self.resources.specification) {
            return Err("Kabuto action charge definition owner differs".into());
        }
        cache
            .entries()
            .get(key as usize)
            .map(|e| &e.definition)
            .ok_or_else(|| "Kabuto action charge definition outside cache".into())
    }
    fn position_of(&self, target: Id) -> Result<[f32; 3], String> {
        self.check()?;
        self.session
            .get(target)
            .filter(|_| target.ticket() == self.owner.ticket())
            .and_then(|a| a.construction_fields.as_ref())
            .map(|f| f.position)
            .ok_or_else(|| "Kabuto action charge target absent or foreign".into())
    }
    fn publish(&mut self, caller: crate::kabuto_action_select::Scratch) -> Result<(), String> {
        self.check()?;
        self.session
            .residents
            .get_mut(&self.owner.serial)
            .unwrap()
            .kabuto_group
            .as_mut()
            .unwrap()
            .charge
            .caller = Some(caller);
        Ok(())
    }
}
fn offset(field: charge::Scalar) -> usize {
    use charge::Scalar::*;
    match field {
        Remaining => 4,
        Maximum => 0x134,
        Meter => 0x140,
        RangeA => 0x180,
        RangeB => 0x184,
        RangeC => 0x188,
        RangeD => 0x18c,
        RangeE => 0x190,
        RangeF => 0x194,
    }
}
impl<W: charge::Services> charge::World<Id> for Adapter<'_, W> {
    fn flags(&self) -> Result<u32, String> {
        self.word(0)
    }
    fn set_flags(&mut self, v: u32) -> Result<(), String> {
        self.store(0, v)
    }
    fn scalar(&self, f: charge::Scalar) -> Result<f32, String> {
        self.word(offset(f)).map(f32::from_bits)
    }
    fn set_scalar(&mut self, f: charge::Scalar, v: f32) -> Result<(), String> {
        self.store(offset(f), v.to_bits())
    }
    fn definition_flags(&self, key: i32) -> Result<u32, String> {
        Ok(self.definition(key as u32)?.flags)
    }
    fn definition_slot(&self, key: u32) -> Result<u32, String> {
        Ok(self.definition(key)?.animation as u32)
    }
    fn interaction(&self) -> Result<Option<Id>, String> {
        self.check()?;
        Ok(self.session.kabuto_query_target_state(self.owner)?.accepted)
    }
    fn target_status(
        &mut self,
        target: Option<Id>,
        kind: u32,
        caller: crate::kabuto_action_select::Scratch,
    ) -> Result<bool, String> {
        self.publish(caller)?;
        if let Some(target) = target {
            self.position_of(target)?;
        }
        let result = self
            .world
            .target_status(self.session, self.owner, target, kind);
        self.check()?;
        result
    }
    fn position(&self) -> Result<[f32; 3], String> {
        self.position_of(self.owner)
    }
    fn related_position(&self) -> Result<[f32; 3], String> {
        self.check()?;
        let related = self
            .session
            .kabuto_query_target_state(self.owner)?
            .retained
            .ok_or("Kabuto action charge requires its actual retained target position")?;
        self.position_of(related)
    }
    fn random_two(&mut self, caller: crate::kabuto_action_select::Scratch) -> Result<u32, String> {
        self.publish(caller)?;
        let result = self.world.random_two(self.session, self.owner);
        self.check()?;
        result
    }
    fn mark_target(&mut self, target: Id) -> Result<(), String> {
        self.position_of(target)?;
        let result = self.world.mark_target(self.session, target);
        self.check()?;
        result
    }
    fn set_positions(&mut self, first: [f32; 3], second: [f32; 3]) -> Result<(), String> {
        for (i, v) in first.into_iter().chain(second).enumerate() {
            self.store(0x60 + i * 4, v.to_bits())?;
        }
        Ok(())
    }
    fn reset_counters(&mut self) -> Result<(), String> {
        for (at, v) in [(0x78, (-1f32).to_bits()), (0x7c, 0), (0x13c, 0), (0x138, 0)] {
            self.store(at, v)?;
        }
        Ok(())
    }
    fn set_target(&mut self, target: Option<Id>) -> Result<(), String> {
        self.check()?;
        if let Some(target) = target {
            self.position_of(target)?;
        }
        // Typed selected-target producer; never encode an original pointer.
        self.store(0x5c, 0)?;
        self.session
            .residents
            .get_mut(&self.owner.serial)
            .unwrap()
            .kabuto_group
            .as_mut()
            .unwrap()
            .charge
            .target = target;
        Ok(())
    }
}
