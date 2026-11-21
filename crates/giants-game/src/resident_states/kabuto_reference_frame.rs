use super::*;
use crate::{kabuto_reference_frame as frame, kabuto_reference_target as target};

fn word(p: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(p[at..at + 4].try_into().unwrap())
}
fn same(a: target::State<Id>, b: target::State<Id>) -> bool {
    a.raw == b.raw
        && a.accepted == b.accepted
        && a.retained == b.retained
        && a.remaining.to_bits() == b.remaining.to_bits()
        && a.score.to_bits() == b.score.to_bits()
}
impl Session {
    /// Current typed target state, including completed prefix effects on failure.
    /// A completed frame/primary is not established by this diagnostic read.
    pub fn kabuto_query_target_state(&self, owner: Id) -> Result<target::State<Id>, String> {
        self.kabuto_reference_state(owner)?;
        let a = self.get(owner).unwrap();
        let g = a.kabuto_group.as_ref().unwrap();
        let targets = g
            .reference_query
            .targets
            .ok_or("Kabuto query state has no producer")?;
        let p = a.seed.storage.data().unwrap();
        // The accepted identity has one existing owner: the tail's p50 role.
        // Scalar words likewise remain canonical in the actual actor payload.
        let state = target::State {
            raw: targets.raw,
            retained: targets.retained,
            accepted: g.primary_tail_links.interaction,
            remaining: f32::from_bits(word(p, 0x48)),
            score: f32::from_bits(word(p, 0x58)),
        };
        if [0x4c, 0x50, 0x54].into_iter().any(|at| word(p, at) != 0)
            || [state.raw, state.accepted, state.retained]
                .into_iter()
                .flatten()
                .any(|id| id.ticket() != owner.ticket() || self.get(id).is_none())
        {
            return Err(
                "Kabuto query typed state, scalar storage or target lifetime differs".into(),
            );
        }
        Ok(state)
    }
    pub(crate) fn captured_kabuto_query_frame(
        &self,
        owner: Id,
        resources: &Resources,
        number: u64,
    ) -> Result<frame::View, String> {
        self.check_kabuto_reference_owner(owner, resources, number)?;
        let a = self.get(owner).unwrap();
        let g = a.kabuto_group.as_ref().unwrap();
        let p = &g.reference_query;
        if p.running
            || p.completed != Some(number)
            || a.construction_fields.as_ref().unwrap().object_flags & 0x8000 != 0
        {
            return Err("Kabuto query component is not completed on this live owner/frame".into());
        }
        let caller = p.caller.ok_or("Kabuto query caller absent")?;
        if p.clock_word != Some(caller.retained_time) {
            return Err("Kabuto query caller clock differs from its producer".into());
        }
        Ok(frame::View {
            owner,
            number,
            target: self.kabuto_query_target_state(owner)?,
            caller_scratch: caller,
        })
    }
    pub(crate) fn advance_kabuto_reference_query(
        &mut self,
        owner: Id,
        resources: &Resources,
        number: u64,
        ray: crate::kabuto_reference_query::Ray,
        world: &mut impl frame::Services,
    ) -> Result<frame::View, String> {
        self.check_kabuto_reference_owner(owner, resources, number)?;
        let a = self.get(owner).unwrap();
        let g = a.kabuto_group.as_ref().unwrap();
        if g.reference_query.running
            || g.reference_query.entered.is_some()
            || g.action_select_entered.is_some()
            || g.held_reference_entered.is_some()
            || g.animation_commit_entered.is_some()
            || g.primary_tail_entered.is_some()
            || a.construction_fields.as_ref().unwrap().object_flags & 0x8000 != 0
        {
            return Err("Kabuto query entry requires unconsumed ordered caller work".into());
        }
        let caller = self.captured_kabuto_resource_scratch(owner, resources, number)?;
        let phase = &mut self
            .residents
            .get_mut(&owner.serial)
            .unwrap()
            .kabuto_group
            .as_mut()
            .unwrap()
            .reference_query;
        phase.entered = Some(number);
        phase.running = true;
        phase.caller = Some(caller);
        let result: Result<(), String> = (|| {
            let selection =
                self.query_kabuto_reference(owner, resources, number, ray, |session, target| {
                    world.bounds(session, target).map(|b| (b.lower, b.upper))
                })?;
            let a = self.get(owner).ok_or("Kabuto query owner removed")?;
            let g = a.kabuto_group.as_ref().unwrap();
            if g.reference_query.targets.is_none() {
                let p = a.seed.storage.data().unwrap();
                if [0x4c, 0x50, 0x54].into_iter().any(|at| word(p, at) != 0) {
                    return Err(
                        "Kabuto query requires typed identities or acquired null cells".into(),
                    );
                }
                self.residents
                    .get_mut(&owner.serial)
                    .unwrap()
                    .kabuto_group
                    .as_mut()
                    .unwrap()
                    .reference_query
                    .targets = Some(frame::Targets {
                    raw: None,
                    retained: None,
                });
            }
            let mut adapter = Adapter {
                session: self,
                owner,
                resources,
                number,
                world,
            };
            let sample = target::advance_world(owner, selection, &mut adapter)?;
            adapter.check()?;
            let p = &mut adapter
                .session
                .residents
                .get_mut(&owner.serial)
                .unwrap()
                .kabuto_group
                .as_mut()
                .unwrap()
                .reference_query;
            p.caller
                .as_mut()
                .unwrap()
                .retain_reference_clock(owner, &sample)?;
            if p.clock_word != Some(sample.clock_word()) {
                return Err("Kabuto query clock producer changed".into());
            }
            Ok(())
        })();
        if let Some(g) = self
            .residents
            .get_mut(&owner.serial)
            .and_then(|a| a.kabuto_group.as_mut())
            .filter(|g| {
                g.owner == owner
                    && g.reference_query.entered == Some(number)
                    && Arc::ptr_eq(&g.declared_class, &resources.specification)
            })
        {
            g.reference_query.running = false;
            match &result {
                Ok(()) => g.reference_query.completed = Some(number),
                Err(error) => {
                    g.geometry_transition_failure
                        .get_or_insert_with(|| error.clone());
                }
            }
        }
        result?;
        self.captured_kabuto_query_frame(owner, resources, number)
    }
}
struct Adapter<'a, W> {
    session: &'a mut Session,
    owner: Id,
    resources: &'a Resources,
    number: u64,
    world: &'a mut W,
}
impl<W: frame::Services> Adapter<'_, W> {
    fn check(&self) -> Result<(), String> {
        self.session
            .check_kabuto_reference_owner(self.owner, self.resources, self.number)?;
        let a = self.session.get(self.owner).unwrap();
        let p = &a.kabuto_group.as_ref().unwrap().reference_query;
        if !p.running
            || p.entered != Some(self.number)
            || a.construction_fields.as_ref().unwrap().object_flags & 0x8000 != 0
        {
            return Err("Kabuto query owner/frame changed during a service".into());
        }
        self.session.kabuto_query_target_state(self.owner)?;
        Ok(())
    }
    fn target(&self, id: Id) -> Result<(), String> {
        if id.ticket() != self.owner.ticket() || self.session.get(id).is_none() {
            return Err("Kabuto query target absent or foreign".into());
        }
        Ok(())
    }
}
impl<W: frame::Services> target::World<Id> for Adapter<'_, W> {
    fn state(&self) -> Result<target::State<Id>, String> {
        self.check()?;
        self.session.kabuto_query_target_state(self.owner)
    }
    fn source_reference_present(&self) -> Result<bool, String> {
        self.check()?;
        Ok(self
            .session
            .kabuto_reference_state(self.owner)?
            .reference
            .is_some())
    }
    fn publish(&mut self, state: target::State<Id>) -> Result<(), String> {
        self.check()?;
        for id in [state.raw, state.accepted, state.retained]
            .into_iter()
            .flatten()
        {
            self.target(id)?;
        }
        let a = self.session.residents.get_mut(&self.owner.serial).unwrap();
        let p = a.seed.storage.data_mut().unwrap();
        p[0x48..0x4c].copy_from_slice(&state.remaining.to_bits().to_le_bytes());
        p[0x58..0x5c].copy_from_slice(&state.score.to_bits().to_le_bytes());
        let g = a.kabuto_group.as_mut().unwrap();
        g.reference_query.targets = Some(frame::Targets {
            raw: state.raw,
            retained: state.retained,
        });
        g.primary_tail_links.interaction = state.accepted;
        Ok(())
    }
}
impl<W: frame::Services> target::Services<Id> for Adapter<'_, W> {
    fn bounds(&mut self, target: Id) -> Result<target::Bounds, String> {
        self.check()?;
        self.target(target)?;
        self.world.bounds(self.session, target)
    }
    fn seconds(&mut self, state: &target::State<Id>) -> Result<f64, String> {
        self.check()?;
        if !same(*state, self.session.kabuto_query_target_state(self.owner)?) {
            return Err("Kabuto clock did not observe the published query prefix".into());
        }
        let clock = self.world.clock(self.session, self.owner)?;
        let f = self
            .session
            .get(self.owner)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap();
        let seconds = crate::object_time_precision::seconds(
            clock.global_seconds,
            Some(crate::object_time_precision::Actor {
                owner_id: f.dispatch_context,
                position: f.position[..2].try_into().unwrap(),
            }),
            None,
            &clock.areas,
            crate::arithmetic::Precision::Double53,
        )
        .map_err(str::to_owned)?;
        if !seconds.is_finite() || seconds < 0. || !(seconds as f32).is_finite() {
            return Err("Kabuto query clock outside finite profile".into());
        }
        let p = &mut self
            .session
            .residents
            .get_mut(&self.owner.serial)
            .unwrap()
            .kabuto_group
            .as_mut()
            .unwrap()
            .reference_query;
        p.clock_word = Some((seconds as f32).to_bits());
        p.caller.as_mut().unwrap().retained_time = (seconds as f32).to_bits();
        Ok(seconds)
    }
    fn announce(&mut self, _: &target::State<Id>) -> Result<(), String> {
        self.check()?;
        let result = self.world.announce(self.session, self.owner);
        result.and(self.check())
    }
    fn reset_marker(&mut self, _: &target::State<Id>) -> Result<(), String> {
        self.check()?;
        let result = self.world.reset_marker(self.session, self.owner);
        result.and(self.check())
    }
    fn parent(&mut self, target: Id) -> Result<Option<Id>, String> {
        self.check()?;
        self.target(target)?;
        let parent = self.world.parent(self.session, target)?;
        if let Some(id) = parent {
            self.target(id)?;
        }
        Ok(parent)
    }
    fn pose(&mut self) -> Result<target::Pose, String> {
        self.check()?;
        let geometry = self.world.geometry(self.session, self.owner)?;
        let f = self
            .session
            .get(self.owner)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap();
        Ok(target::Pose {
            position: f.position,
            scale: f.scale,
            height: geometry.height,
            axis: geometry.axis,
        })
    }
}
