//! Owned inspection child ANM/pose/vertex pass. Global child world admission,
//! history/finalization and native frame order remain separate requirements.
use crate::{
    mecc_special::{self, ChildPose},
    resident_states::{Id, Session},
};

pub struct Frame {
    pub pose: ChildPose,
    pub positions: Vec<[f32; 3]>,
    pub personality: u32,
    pub scale: f32,
    pub angle: f32,
}
pub struct Children {
    parent: Id,
    ids: [Id; 3],
    number: u64,
    failed: bool,
}
impl Children {
    pub fn new(session: &Session, parent: Id) -> Result<Self, String> {
        let initialized = session
            .mecc_special(parent)
            .ok_or("jet child constructor absent")?;
        Ok(Self {
            parent,
            ids: initialized.children,
            number: 0,
            failed: false,
        })
    }
    pub fn advance(
        &mut self,
        session: &mut Session,
        number: u64,
        seconds: f32,
        epoch: u32,
    ) -> Result<Vec<Frame>, String> {
        if self.failed || self.number.checked_add(1) != Some(number) {
            return Err("jet presentation frame stale or quarantined".into());
        }
        let result = self.advance_inner(session, number, seconds, epoch);
        if result.is_err() {
            self.failed = true;
        }
        result
    }
    fn advance_inner(
        &mut self,
        session: &mut Session,
        number: u64,
        seconds: f32,
        epoch: u32,
    ) -> Result<Vec<Frame>, String> {
        if !seconds.is_finite()
            || seconds < 0.
            || epoch == 0
            || session
                .mecc_special(self.parent)
                .is_none_or(|v| v.children != self.ids)
            || session.mecc_physical_cycle(self.parent)?.number != number
            || session.actor_world_tick(self.parent)?.phase()
                != crate::actor_world_tick::Phase::Finished
            || session.actor_world_flags(self.parent)? & 0x40 == 0
            || session.mecc_physical_cycle_failure(self.parent).is_some()
        {
            return Err("jet presentation needs completed owned parent and constructor".into());
        }
        // Consume once before the first clock side effect. An error is never replayed.
        self.number = number;
        let mut frames = Vec::new();
        for (index, id) in self.ids.into_iter().enumerate() {
            let actor = session.get(id).ok_or("jet child retired")?;
            if actor.class != [84, 85, 781][index]
                || actor.seed.storage.state() != 0
                || session
                    .mecc_special_child(id)
                    .is_none_or(|v| v.parent != self.parent)
            {
                return Err("jet presentation child replaced or rebound".into());
            }
            let mut durations = vec![None; 256];
            for (&slot, descriptor) in &session
                .specification(id)
                .ok_or("jet child spec absent")?
                .animations
            {
                durations[slot as usize] = Some(descriptor.template.duration);
            }
            // These state0 children have null primary/secondary callbacks and
            // constructor-zero model rates; no missing behavior is synthesized.
            let tick = session.advance_channels(
                id,
                &durations,
                [0.; 3],
                seconds,
                crate::arithmetic::Precision::Double53,
            )?;
            if tick.missing != 0 {
                return Err("jet child ANM descriptor absent".into());
            }
            session.mirror_jet_child_channels(id)?;
            session.update_current_references(id)?;
            let pose = mecc_special::child_pose(session, id)?;
            session.prepare_actor_positions(id, 0, epoch, true)?;
            let positions = session
                .actor_model_positions(id, 0)?
                .ok_or("jet child vertices absent")?
                .to_vec();
            let fields = session
                .get(id)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap();
            frames.push(Frame {
                pose,
                positions,
                personality: fields.configuration[14],
                scale: fields.scale,
                angle: fields.angles[0],
            });
        }
        Ok(frames)
    }
}
