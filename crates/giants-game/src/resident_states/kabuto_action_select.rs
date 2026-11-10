use super::*;
use crate::kabuto_action_select::{
    self as select, Input, Next, PacketWord, Scalar, Scratch, Sound,
};

struct Adapter<'a, W> {
    session: &'a mut Session,
    world: &'a mut W,
    owner: Id,
    resources: &'a Resources,
    number: u64,
}
impl<W: select::Services> Adapter<'_, W> {
    fn check(&self) -> Result<(), String> {
        self.session
            .check_kabuto_reference_owner(self.owner, self.resources, self.number)?;
        let g = self.session.kabuto_group(self.owner).unwrap();
        if !g.action_select_running || g.action_select_entered != Some(self.number) {
            return Err("Kabuto action selector changed during a service".into());
        }
        Ok(())
    }
    fn publish(&mut self, scratch: Scratch) -> Result<(), String> {
        self.check()?;
        self.session
            .residents
            .get_mut(&self.owner.serial)
            .unwrap()
            .kabuto_group
            .as_mut()
            .unwrap()
            .action_select_scratch = Some(scratch);
        Ok(())
    }
}
impl<W: select::Services> select::World<Id> for Adapter<'_, W> {
    fn scalar(&self, field: Scalar) -> Result<f32, String> {
        self.check()?;
        let a = self.session.get(self.owner).unwrap();
        let g = a.kabuto_group.as_ref().unwrap();
        match field {
            Scalar::LongitudinalRate => Ok(g.motion.model[1]),
            Scalar::LateralRate => Ok(g.motion.model[2]),
            _ => {
                let at = match field {
                    Scalar::CurrentSize => 0x15c,
                    Scalar::TargetSize => 0x158,
                    Scalar::SlowThreshold => 0x168,
                    Scalar::FastThreshold => 0x16c,
                    Scalar::SpeedA => 0x174,
                    Scalar::SpeedB => 0x178,
                    Scalar::SpeedC => 0x17c,
                    _ => unreachable!(),
                };
                let p = a.seed.storage.data().unwrap();
                Ok(f32::from_bits(u32::from_le_bytes(
                    p[at..at + 4].try_into().unwrap(),
                )))
            }
        }
    }
    fn buttons(&self) -> Result<u32, String> {
        self.check()?;
        Ok(u32::from_le_bytes(
            self.session
                .kabuto_group(self.owner)
                .unwrap()
                .motion
                .commands[4..]
                .try_into()
                .unwrap(),
        ))
    }
    fn reference(&self) -> Result<Option<Id>, String> {
        self.check()?;
        Ok(self.session.kabuto_reference_state(self.owner)?.reference)
    }
    fn other_reference_present(&self) -> Result<bool, String> {
        self.check()?;
        self.world.other_reference_present(self.session, self.owner)
    }
    fn held(&self) -> Result<[Option<Id>; 2], String> {
        self.check()?;
        Ok(self.session.kabuto_reference_state(self.owner)?.held)
    }
    fn class(&self, actor: Id) -> Result<u32, String> {
        self.check()?;
        self.session
            .get(actor)
            .and_then(|a| a.construction_fields.as_ref())
            .map(|f| f.class as u32)
            .ok_or_else(|| "Kabuto selection target absent or foreign".into())
    }
    fn definition_slot(&self, key: u32) -> Result<u32, String> {
        self.check()?;
        self.world.definition_slot(self.session, self.owner, key)
    }
    fn random(&mut self, upper: u32, scratch: Scratch) -> Result<u32, String> {
        self.publish(scratch)?;
        self.world.random(self.session, self.owner, upper)
    }
    fn sound(&mut self, kind: Sound, id: u32, scratch: Scratch) -> Result<(), String> {
        self.publish(scratch)?;
        let result = self.world.sound(self.session, self.owner, kind, id);
        self.check()?;
        result
    }
    fn notifying(&self) -> Result<bool, String> {
        self.check()?;
        self.world
            .network(self.session, self.owner)?
            .eligible()
            .map_err(str::to_owned)
    }
    fn notify(
        &mut self,
        kind: Sound,
        scratch: Scratch,
        alias: PacketWord,
        packet: &mut u32,
    ) -> Result<(), String> {
        self.publish(scratch)?;
        let result = self
            .world
            .notify(self.session, self.owner, kind, alias, packet);
        let mut completed = scratch;
        match alias {
            PacketWord::RetainedTime => completed.retained_time = *packet,
            PacketWord::Temporary => completed.packet_word = *packet,
        }
        self.publish(completed)?;
        result
    }
}
impl Session {
    pub(crate) fn select_kabuto_actions(
        &mut self,
        owner: Id,
        resources: &Resources,
        number: u64,
        scratch: &mut Scratch,
        world: &mut impl select::Services,
    ) -> Result<Next, String> {
        self.check_kabuto_reference_owner(owner, resources, number)?;
        let g = self.kabuto_group(owner).unwrap();
        if g.action_select_running || g.action_select_entered == Some(number) {
            return Err("Kabuto action selection reentry or replay".into());
        }
        let captured = g
            .resource_phase_view
            .and_then(|v| v.resources)
            .ok_or("Kabuto action selection resource sample absent")?;
        let input = Input {
            earlier_behavior: captured.initial_mode,
            activity: captured.activity,
            wrapped: captured.wrapped,
        };
        let g = self
            .residents
            .get_mut(&owner.serial)
            .unwrap()
            .kabuto_group
            .as_mut()
            .unwrap();
        g.action_select_entered = Some(number);
        g.action_select_running = true;
        g.action_select_scratch = Some(*scratch);
        let mut adapter = Adapter {
            session: self,
            world,
            owner,
            resources,
            number,
        };
        let result = select::advance(scratch, input, &mut adapter);
        // Caller scratch remains owned by the caller, including on errors. A
        // removed owner cannot be restored from this diagnostic snapshot.
        let published = adapter.publish(*scratch);
        let result = result.and_then(|next| published.map(|()| next));
        if let Some(g) = adapter
            .session
            .residents
            .get_mut(&owner.serial)
            .and_then(|a| a.kabuto_group.as_mut())
            .filter(|g| {
                g.action_select_entered == Some(number)
                    && Arc::ptr_eq(&g.declared_class, &resources.specification)
            })
        {
            g.action_select_running = false;
            match &result {
                Ok(next) => {
                    g.action_select_completed = Some(number);
                    g.action_select_next = Some(*next);
                }
                Err(error) => g.geometry_transition_failure = Some(error.clone()),
            }
        }
        result
    }
}
