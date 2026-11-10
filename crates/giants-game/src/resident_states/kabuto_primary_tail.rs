use super::*;
use crate::{
    animation_epoch::Stamp, kabuto_action_select::Continuation, kabuto_primary_tail as tail,
};
fn word(p: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(p[at..at + 4].try_into().unwrap())
}
struct Adapter<'a, W> {
    session: &'a mut Session,
    owner: Id,
    resources: &'a Resources,
    number: u64,
    stamp: &'a mut Stamp,
    world: &'a mut W,
}
impl<W: tail::Services> Adapter<'_, W> {
    fn check(&self) -> Result<(), String> {
        self.session
            .check_kabuto_reference_owner(self.owner, self.resources, self.number)
    }
    fn links(&mut self) -> Result<&mut tail::Links<Id>, String> {
        self.check()?;
        Ok(&mut self
            .session
            .residents
            .get_mut(&self.owner.serial)
            .unwrap()
            .kabuto_group
            .as_mut()
            .unwrap()
            .primary_tail_links)
    }
}
impl<W: tail::Services> tail::World<Id> for Adapter<'_, W> {
    fn state(&self) -> Result<tail::State<Id>, String> {
        self.check()?;
        let a = self.session.get(self.owner).unwrap();
        let g = a.kabuto_group.as_ref().unwrap();
        let f = a.construction_fields.as_ref().unwrap();
        let p = a.seed.storage.data().unwrap();
        if a.channels != Some(f.channels)
            || [0x50, 0x88, 0x320, 0x324]
                .iter()
                .any(|at| word(p, *at) != 0)
        {
            return Err(
                "Kabuto tail requires coherent channels and typed pointer-normalized links".into(),
            );
        }
        let l = g.primary_tail_links;
        Ok(tail::State {
            transient: l.transient,
            transient_slot: word(p, 0x84),
            animation_slot: f.channels[0][0],
            definition: word(p, 0x90) as i32,
            buttons: u32::from_le_bytes(g.motion.commands[4..8].try_into().unwrap()),
            flags: word(p, 0),
            size: f32::from_bits(word(p, 0x15c)),
            interaction: l.interaction,
            reference: g.reference_links[0],
            progress: l.progress,
            position: f.position.map(f32::to_bits),
            angle: f.angles[0].to_bits(),
        })
    }
    fn mark(&mut self, actor: Id) -> Result<(), String> {
        self.check()?;
        self.session.mark_wield_child(actor)
    }
    fn clear_transient(&mut self) -> Result<(), String> {
        self.links()?.transient = None;
        self.session
            .residents
            .get_mut(&self.owner.serial)
            .unwrap()
            .seed
            .storage
            .data_mut()
            .unwrap()[0x84..0x88]
            .copy_from_slice(&254u32.to_le_bytes());
        Ok(())
    }
    fn local_owner(&self) -> Result<bool, String> {
        self.check()?;
        Ok(self.world.controlled(self.session)? == Some(self.owner))
    }
    fn camera_table_present(&self, d: i32) -> Result<bool, String> {
        self.check()?;
        Ok(self.world.camera_ranges().sequence(d as usize)?.is_some())
    }
    fn camera_range(&self, d: i32, row: usize) -> Result<Option<tail::CameraRange>, String> {
        self.check()?;
        Ok(self
            .world
            .camera_ranges()
            .sequence(d as usize)?
            .and_then(|s| s.get(row))
            .copied())
    }
    fn request_camera_reference(&mut self, r: i32) -> Result<(), String> {
        self.check()?;
        let result = self
            .world
            .request_camera_reference(self.session, self.owner, r);
        self.check()?;
        result
    }
    fn sample_camera(&mut self) -> Result<crate::authored_camera_reference::Output, String> {
        self.check()?;
        let aspect = self.world.camera_aspect()?;
        self.session
            .sample_reference_camera(self.owner, 1, 7, aspect, self.stamp)?
            .ok_or_else(|| "Kabuto authored camera reference is absent".into())
    }
    fn set_camera_pose(
        &mut self,
        c: crate::authored_camera_reference::Output,
    ) -> Result<(), String> {
        self.check()?;
        let result = self.world.set_camera_pose(self.session, self.owner, c);
        self.check()?;
        result
    }
    fn clear_camera(&mut self) -> Result<(), String> {
        self.check()?;
        let result = self.world.clear_camera(self.session, self.owner);
        self.check()?;
        result
    }
    fn class(&self, actor: Id) -> Result<u32, String> {
        self.check()?;
        self.session
            .get(actor)
            .and_then(|a| a.construction_fields.as_ref())
            .map(|f| f.class as u32)
            .ok_or_else(|| "Kabuto tail interaction actor absent or foreign".into())
    }
    fn widget(&mut self, id: u32, value: u32) -> Result<(), String> {
        self.check()?;
        let result = self.world.widget(self.session, self.owner, id, value);
        self.check()?;
        result
    }
    fn target_status(&mut self, t: Option<Id>, kind: u32) -> Result<u32, String> {
        self.check()?;
        if t.is_some_and(|id| self.session.get(id).is_none()) {
            return Err("Kabuto tail target absent or foreign".into());
        }
        let result = self.world.target_status(self.session, self.owner, t, kind);
        self.check()?;
        result
    }
    fn progress_values(&self, actor: Id) -> Result<(f32, f32), String> {
        self.check()?;
        let a = self
            .session
            .get(actor)
            .ok_or("Kabuto tail progress actor absent or foreign")?;
        let f = a
            .construction_fields
            .as_ref()
            .ok_or("Kabuto tail progress header absent")?;
        Ok((
            a.mecc_damage
                .ok_or("Kabuto linked accumulated damage producer absent")?,
            f32::from_bits(f.configuration[12]),
        ))
    }
    fn start_sound(&mut self, id: u32) -> Result<(), String> {
        self.check()?;
        let result = self.world.start_sound(self.session, self.owner, id);
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
    fn notify_sound(&mut self, p: &mut u32) -> Result<(), String> {
        self.check()?;
        let result = self.world.notify_sound(self.session, self.owner, p);
        self.check()?;
        result
    }
    fn clear_progress(&mut self, index: usize) -> Result<(), String> {
        self.links()?.progress[index] = None;
        Ok(())
    }
    fn or_flags(&mut self, f: u32) -> Result<(), String> {
        self.check()?;
        let p = self
            .session
            .residents
            .get_mut(&self.owner.serial)
            .unwrap()
            .seed
            .storage
            .data_mut()
            .unwrap();
        let value = word(p, 0) | f;
        p[..4].copy_from_slice(&value.to_le_bytes());
        Ok(())
    }
    fn publish_position_allowed(&mut self) -> Result<bool, String> {
        self.check()?;
        let result = self
            .world
            .publish_position_allowed(self.session, self.owner);
        self.check()?;
        result
    }
    fn position_clock_word(&self) -> Result<u32, String> {
        self.check()?;
        self.world.position_clock_word()
    }
    fn notify_position(&mut self, p: [u32; 5], len: u32) -> Result<(), String> {
        self.check()?;
        let result = self.world.notify_position(self.session, self.owner, p, len);
        self.check()?;
        result
    }
}
impl Session {
    pub fn kabuto_primary_tail_links(&self, owner: Id) -> Result<tail::Links<Id>, String> {
        self.kabuto_reference_state(owner)?;
        Ok(self.kabuto_group(owner).unwrap().primary_tail_links)
    }

    pub(crate) fn publish_kabuto_tail_links(
        &mut self,
        owner: Id,
        resources: &Resources,
        number: u64,
        links: tail::Links<Id>,
        slot: u32,
    ) -> Result<(), String> {
        self.check_kabuto_reference_owner(owner, resources, number)?;
        for id in [
            links.transient,
            links.interaction,
            links.progress[0],
            links.progress[1],
        ]
        .into_iter()
        .flatten()
        {
            self.get(id)
                .ok_or("Kabuto tail relationship absent or foreign")?;
        }
        let a = self.residents.get_mut(&owner.serial).unwrap();
        let p = a.seed.storage.data_mut().unwrap();
        if [0x50, 0x88, 0x320, 0x324]
            .iter()
            .any(|at| word(p, *at) != 0)
        {
            return Err("Kabuto tail pointer cell is not normalized".into());
        }
        p[0x84..0x88].copy_from_slice(&slot.to_le_bytes());
        a.kabuto_group.as_mut().unwrap().primary_tail_links = links;
        Ok(())
    }
    pub(crate) fn advance_kabuto_primary_tail(
        &mut self,
        owner: Id,
        resources: &Resources,
        number: u64,
        stamp: &mut Stamp,
        world: &mut impl tail::Services,
    ) -> Result<(), String> {
        self.check_kabuto_reference_owner(owner, resources, number)?;
        let g = self.kabuto_group(owner).unwrap();
        if g.primary_tail_running || g.primary_tail_entered == Some(number) {
            return Err("Kabuto primary tail reentry or replay".into());
        }
        let captured = g
            .resource_phase_view
            .and_then(|v| v.resources)
            .ok_or("Kabuto primary tail resource sample absent")?;
        let current_frame=match g.action_gate_next {
   Some(Continuation::Commit) if g.action_gate_completed==Some(number)&&g.animation_commit_completed==Some(number)=>g.animation_commit_current_frame.ok_or("Kabuto tail committed counter absent")?,
   Some(Continuation::SkipCommit) if g.action_gate_completed==Some(number)&&g.animation_commit_entered.is_none()=>captured.current_frame,
   _=>return Err("Kabuto primary tail requires completed selected/skipped animation; held route remains unacquired".into()),
  };
        let input = tail::Input {
            current_frame,
            earlier_behavior: captured.initial_mode,
        };
        let g = self
            .residents
            .get_mut(&owner.serial)
            .unwrap()
            .kabuto_group
            .as_mut()
            .unwrap();
        g.primary_tail_entered = Some(number);
        g.primary_tail_running = true;
        let mut adapter = Adapter {
            session: self,
            owner,
            resources,
            number,
            stamp,
            world,
        };
        let result = tail::advance(input, &mut adapter).and_then(|()| adapter.check());
        if let Some(g) = adapter
            .session
            .residents
            .get_mut(&owner.serial)
            .and_then(|a| a.kabuto_group.as_mut())
            .filter(|g| {
                g.primary_tail_entered == Some(number)
                    && Arc::ptr_eq(&g.declared_class, &resources.specification)
            })
        {
            g.primary_tail_running = false;
            match &result {
                Ok(()) => g.primary_tail_completed = Some(number),
                Err(e) => g.geometry_transition_failure = Some(e.clone()),
            }
        }
        result
    }
}
