use super::*;
use crate::kabuto_reaction_admission as reaction;

impl Session {
    pub(crate) fn captured_kabuto_reaction_admission(
        &self,
        owner: Id,
        resources: &Resources,
        number: u64,
    ) -> Result<reaction::View, String> {
        self.captured_kabuto_query_geometry(owner, resources, number)?;
        let p = &self.kabuto_group(owner).unwrap().reaction;
        if p.running || p.completed != Some(number) {
            return Err("Kabuto reaction admission is not completed on this owner/frame".into());
        }
        p.view.ok_or_else(|| "Kabuto reaction view absent".into())
    }
    pub(crate) fn advance_kabuto_reaction_admission(
        &mut self,
        owner: Id,
        resources: &Resources,
        globals: &giants_formats::global_data::GlobalData,
        number: u64,
        world: &mut impl reaction::Services,
    ) -> Result<reaction::View, String> {
        let geometry = self.captured_kabuto_query_geometry(owner, resources, number)?;
        if geometry.next != crate::kabuto_query_geometry::Next::ReactionAdmission {
            return Err("Kabuto query geometry requires the primary tail before reactions".into());
        }
        let query = self.captured_kabuto_query_frame(owner, resources, number)?;
        let g = self.kabuto_group(owner).unwrap();
        if g.reaction.entered.is_some()
            || g.reaction.running
            || g.action_select_entered.is_some()
            || g.held_reference_entered.is_some()
            || g.animation_commit_entered.is_some()
            || g.primary_tail_entered.is_some()
        {
            return Err("Kabuto reaction entry requires unconsumed ordered caller work".into());
        }
        let decision = g
            .resource_phase_view
            .and_then(|v| v.resources)
            .ok_or("Kabuto reaction requires its retained resource decision")?;
        // The owned resource adapter keeps these early mode/channel fields
        // across callbacks; only the native reread fields are refreshed there.
        let input = reaction::Input {
            earlier_behavior: decision.initial_mode,
            previous_clip: decision.slot,
            previous_selection: decision.previous_selection as i32,
            wrapped: decision.wrapped,
        };
        let mut caller = query.caller_scratch;
        let p = &mut self
            .residents
            .get_mut(&owner.serial)
            .unwrap()
            .kabuto_group
            .as_mut()
            .unwrap()
            .reaction;
        p.entered = Some(number);
        p.running = true;
        p.caller = Some(caller);
        let result = reaction::advance(
            &mut caller,
            input,
            &mut Adapter {
                session: self,
                owner,
                resources,
                globals,
                number,
                world,
            },
        )
        .and_then(|next| {
            let view = reaction::View {
                owner,
                number,
                input,
                caller,
                next,
            };
            if let reaction::Next::Required(required) = next {
                Err(format!(
                    "Kabuto reaction requires acquired {required:?} continuation"
                ))
            } else {
                Ok(view)
            }
        });
        if let Some(g) = self
            .residents
            .get_mut(&owner.serial)
            .and_then(|a| a.kabuto_group.as_mut())
            .filter(|g| {
                g.owner == owner
                    && g.reaction.entered == Some(number)
                    && Arc::ptr_eq(&g.declared_class, &resources.specification)
            })
        {
            g.reaction.running = false;
            g.reaction.caller = Some(caller);
            match &result {
                Ok(view) => {
                    g.reaction.completed = Some(number);
                    g.reaction.view = Some(*view);
                }
                Err(error) => {
                    g.geometry_transition_failure
                        .get_or_insert_with(|| error.clone());
                }
            }
        }
        result?;
        self.captured_kabuto_reaction_admission(owner, resources, number)
    }
}
struct Adapter<'a, W> {
    session: &'a mut Session,
    owner: Id,
    resources: &'a Resources,
    globals: &'a giants_formats::global_data::GlobalData,
    number: u64,
    world: &'a mut W,
}
impl<W: reaction::Services> Adapter<'_, W> {
    fn check(&self) -> Result<(), String> {
        self.session
            .check_kabuto_reference_owner(self.owner, self.resources, self.number)?;
        let g = self.session.kabuto_group(self.owner).unwrap();
        if !g.reaction.running
            || g.reaction.entered != Some(self.number)
            || g.query_geometry.completed != Some(self.number)
            || g.physics.is_none()
        {
            return Err("Kabuto reaction owner/phase changed during a provider".into());
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
}
impl<W: reaction::Services> reaction::World<Id> for Adapter<'_, W> {
    fn flags(&self) -> Result<u32, String> {
        self.word(0)
    }
    fn set_flags(&mut self, value: u32) -> Result<(), String> {
        self.store(0, value)
    }
    fn buttons(&self) -> Result<u32, String> {
        self.check()?;
        Ok(u32::from_le_bytes(
            self.session
                .kabuto_group(self.owner)
                .unwrap()
                .motion
                .commands[4..8]
                .try_into()
                .unwrap(),
        ))
    }
    fn interaction(&self) -> Result<Option<Id>, String> {
        self.check()?;
        Ok(self.session.kabuto_query_target_state(self.owner)?.accepted)
    }
    fn class(&self, target: Id) -> Result<u32, String> {
        self.check()?;
        self.session
            .get(target)
            .filter(|_| target.ticket() == self.owner.ticket())
            .map(|a| a.class as u32)
            .ok_or_else(|| "Kabuto reaction target absent or foreign".into())
    }
    fn held_present(&self) -> Result<bool, String> {
        self.check()?;
        Ok(self
            .session
            .kabuto_reference_state(self.owner)?
            .held
            .into_iter()
            .any(|id| id.is_some()))
    }
    fn global_request(&mut self) -> Result<bool, String> {
        self.check()?;
        self.world.global_request(self.session)
    }
    fn tracked(&self) -> Result<(i32, u32), String> {
        Ok((self.word(0x98)? as i32, self.word(0x9c)?))
    }
    fn clear_tracked(&mut self) -> Result<(), String> {
        self.store(0x98, u32::MAX)?;
        self.store(0x9c, u32::MAX)
    }
    fn checkpoint(&mut self, caller: crate::kabuto_action_select::Scratch) -> Result<(), String> {
        self.check()?;
        self.session
            .residents
            .get_mut(&self.owner.serial)
            .unwrap()
            .kabuto_group
            .as_mut()
            .unwrap()
            .reaction
            .caller = Some(caller);
        Ok(())
    }
    fn stop_sound(&mut self, id: u32) -> Result<(), String> {
        self.check()?;
        let result = self.world.stop_sound(self.session, self.owner, id);
        result.and(self.check())
    }
    fn notifications(&mut self) -> Result<bool, String> {
        self.check()?;
        let n = self.world.network(self.session, self.owner)?;
        let f = self
            .session
            .get(self.owner)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap();
        if n.owner_word != f.dispatch_context {
            return Err("Kabuto reaction notification owner word differs".into());
        }
        n.eligible()
    }
    fn notify_stop_sound(&mut self, packet: &mut u32) -> Result<(), String> {
        self.check()?;
        let result = self
            .world
            .notify_stop_sound(self.session, self.owner, packet);
        result.and(self.check())
    }
    fn header_flags(&self) -> Result<u32, String> {
        self.check()?;
        Ok(self
            .session
            .get(self.owner)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap()
            .object_flags)
    }
    fn physics_flags(&self) -> Result<u32, String> {
        self.word(0xe4)
    }
    fn seconds(&mut self) -> Result<f64, String> {
        self.check()?;
        let clock = self.world.clock(self.session, self.owner)?;
        let f = self
            .session
            .get(self.owner)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap();
        crate::object_time_precision::seconds(
            clock.global_seconds,
            Some(crate::object_time_precision::Actor {
                owner_id: f.dispatch_context,
                position: f.position[..2].try_into().unwrap(),
            }),
            None,
            &clock.areas,
            crate::arithmetic::Precision::Double53,
        )
        .map_err(str::to_owned)
    }
    fn cooldown(&self) -> Result<f32, String> {
        self.word(0x344).map(f32::from_bits)
    }
    fn set_cooldown(&mut self, v: f32) -> Result<(), String> {
        self.store(0x344, v.to_bits())
    }
    fn configured_rate(&self) -> Result<f32, String> {
        self.globals
            .words
            .get(0x6b4 / 4)
            .copied()
            .map(f32::from_bits)
            .ok_or_else(|| "Kabuto reaction configured rate absent".into())
    }
    fn set_rate(&mut self, v: f32) -> Result<(), String> {
        self.store(0xe8, v.to_bits())
    }
    fn definition_flags(&self, key: u32) -> Result<u32, String> {
        self.check()?;
        let cache = self
            .session
            .kabuto_group(self.owner)
            .unwrap()
            .cold_animation_cache
            .as_ref()
            .ok_or("Kabuto reaction cold definitions absent")?;
        if !Arc::ptr_eq(cache.specification(), &self.resources.specification) {
            return Err("Kabuto reaction definition owner differs".into());
        }
        cache
            .entries()
            .get(key as usize)
            .map(|e| e.definition.flags)
            .ok_or_else(|| "Kabuto reaction definition outside cache".into())
    }
    fn related_class(&self) -> Result<Option<u32>, String> {
        if self.word(0x5c)? != 0 {
            return Err(
                "Kabuto reaction requires an acquired typed related-target producer".into(),
            );
        }
        Ok(None)
    }
}
