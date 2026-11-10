use super::held_reference_tests::Case;
use super::*;
use crate::{
    animation_epoch::Stamp, authored_camera_reference::Output, kabuto_action_select::Continuation,
    kabuto_primary_tail as tail,
};
#[derive(Default)]
pub(super) struct Controls {
    ranges: Option<tail::CameraRanges>,
    passed: usize,
    held: usize,
    poses: usize,
    positions: usize,
    failures: usize,
}
struct World<'a> {
    factory: &'a crate::kabuto_initial::Factory,
    ranges: &'a tail::CameraRanges,
    owner: Id,
    number: u64,
    kind: Case,
    widgets: std::collections::BTreeMap<u32, u32>,
    poses: usize,
    positions: usize,
    sounds: usize,
    camera_requests: Vec<i32>,
    camera_clears: usize,
    camera_poses: Vec<Output>,
}
impl tail::Services for World<'_> {
    fn controlled(&self, _: &Session) -> Result<Option<Id>, String> {
        Ok(Some(self.owner))
    }
    fn camera_ranges(&self) -> &tail::CameraRanges {
        self.ranges
    }
    fn camera_aspect(&self) -> Result<f32, String> {
        Ok(960. / 540.)
    }
    fn request_camera_reference(
        &mut self,
        _: &mut Session,
        owner: Id,
        reference: i32,
    ) -> Result<(), String> {
        assert_eq!(owner, self.owner);
        self.camera_requests.push(reference);
        Ok(())
    }
    fn set_camera_pose(&mut self, _: &mut Session, owner: Id, c: Output) -> Result<(), String> {
        assert_eq!(owner, self.owner);
        assert!(c.view.values.iter().all(|v| v.is_finite()));
        assert!(c.vertical_fov.is_none_or(f32::is_finite));
        self.poses += 1;
        self.camera_poses.push(c);
        Ok(())
    }
    fn clear_camera(&mut self, _: &mut Session, owner: Id) -> Result<(), String> {
        assert_eq!(owner, self.owner);
        self.camera_clears += 1;
        Ok(())
    }
    fn widget(&mut self, s: &mut Session, owner: Id, id: u32, value: u32) -> Result<(), String> {
        let prior = self.widgets.entry(id).or_insert(0xa5a5a5a5);
        *prior = (*prior & !2) | if value == 0 { 0 } else { 2 };
        let factory = self.factory;
        let mut stamp = Stamp(123);
        assert!(
            factory
                .advance_primary_tail(s, owner, self.number, &mut stamp, self)
                .is_err()
        );
        if id == 0x2d && matches!(self.kind, Case::TailFailure | Case::TailClear) {
            s.residents
                .get_mut(&owner.serial)
                .unwrap()
                .construction_fields
                .as_mut()
                .unwrap()
                .action ^= 0x1000;
            s.residents
                .get_mut(&owner.serial)
                .unwrap()
                .kabuto_group
                .as_mut()
                .unwrap()
                .motion
                .mode = 99;
            if matches!(self.kind, Case::TailClear) {
                s.clear();
                Ok(())
            } else {
                Err("declared late tail widget failure".into())
            }
        } else {
            Ok(())
        }
    }
    fn target_status(
        &mut self,
        _: &mut Session,
        _: Id,
        _: Option<Id>,
        kind: u32,
    ) -> Result<u32, String> {
        assert_eq!(kind, 2);
        Ok(0x100)
    }
    fn start_sound(&mut self, s: &mut Session, owner: Id, id: u32) -> Result<(), String> {
        assert_eq!(id, 0x45b);
        self.sounds += 1;
        let mut links = s.kabuto_primary_tail_links(owner)?;
        links.progress[1] = links.progress[0];
        let p = s.get(owner).unwrap().seed.storage.data().unwrap();
        let slot = u32::from_le_bytes(p[0x84..0x88].try_into().unwrap());
        self.factory
            .publish_primary_tail_links(s, owner, self.number, links, slot)?;
        s.residents
            .get_mut(&owner.serial)
            .unwrap()
            .construction_fields
            .as_mut()
            .unwrap()
            .position[0] += 0.5;
        Ok(())
    }
    fn network<'a>(
        &'a self,
        _: &Session,
        _: Id,
    ) -> Result<crate::kabuto_geometry_mode::NotificationContext<'a>, String> {
        Ok(crate::kabuto_geometry_mode::NotificationContext {
            session_mode: 4,
            peer_ids: &[11, 22],
            local_peer: 11,
            owner_word: 0,
        })
    }
    fn notify_sound(&mut self, _: &mut Session, _: Id, p: &mut u32) -> Result<(), String> {
        assert_eq!(*p, 0x45b);
        *p = 0x77889900;
        Ok(())
    }
    fn publish_position_allowed(&mut self, s: &mut Session, owner: Id) -> Result<bool, String> {
        s.residents
            .get_mut(&owner.serial)
            .unwrap()
            .construction_fields
            .as_mut()
            .unwrap()
            .position[1] += 0.25;
        Ok(self.number.is_multiple_of(2))
    }
    fn position_clock_word(&self) -> Result<u32, String> {
        Ok(0x12440000 | self.number as u32)
    }
    fn notify_position(
        &mut self,
        s: &mut Session,
        owner: Id,
        p: [u32; 5],
        length: u32,
    ) -> Result<(), String> {
        let f = s.get(owner).unwrap().construction_fields.as_ref().unwrap();
        assert_eq!(length, 16);
        assert_eq!(&p[..3], &f.position.map(f32::to_bits));
        assert_eq!(p[3], f.angles[0].to_bits());
        assert_eq!(p[4], 0x12440000 | self.number as u32);
        self.positions += 1;
        Ok(())
    }
}
impl Controls {
    pub(super) fn advance(
        &mut self,
        factories: (
            &crate::kabuto_initial::Factory,
            &crate::kabuto_initial::Factory,
        ),
        s: &mut Session,
        owner: Id,
        number: u64,
        kind: Case,
        stamp: &mut Stamp,
    ) {
        let (factory, foreign) = factories;
        if self.ranges.is_none() {
            self.ranges = Some(
                tail::CameraRanges::read(std::path::Path::new(
                    &std::env::var("GIANTS_KABUTO_EXECUTABLE").unwrap(),
                ))
                .unwrap(),
            );
        }
        let mut world = World {
            factory,
            ranges: self.ranges.as_ref().unwrap(),
            owner,
            number,
            kind,
            widgets: Default::default(),
            poses: 0,
            positions: 0,
            sounds: 0,
            camera_requests: vec![],
            camera_clears: 0,
            camera_poses: vec![],
        };
        if s.kabuto_group(owner).unwrap().action_gate_next != Some(Continuation::Commit) {
            assert!(
                factory
                    .advance_primary_tail(s, owner, number, stamp, &mut world)
                    .is_err()
            );
            assert!(
                s.kabuto_group(owner)
                    .unwrap()
                    .primary_tail_entered
                    .is_none()
            );
            self.held += 1;
            return;
        }
        let g = s.kabuto_group(owner).unwrap();
        let first = g.first;
        let current = s.get(owner).unwrap().channels.unwrap()[0][0];
        let target = s.residents.get_mut(&first.serial).unwrap();
        target.mecc_damage = Some(f32::from_bits(
            target.construction_fields.as_ref().unwrap().configuration[12],
        ));
        let links = tail::Links {
            transient: Some(first),
            interaction: Some(first),
            progress: [Some(first), Some(owner)],
        };
        assert!(
            foreign
                .publish_primary_tail_links(s, owner, number, links, current.wrapping_add(1))
                .is_err()
        );
        factory
            .publish_primary_tail_links(s, owner, number, links, current.wrapping_add(1))
            .unwrap();
        let before = super::motion_tests::snapshot(s, owner);
        let epoch = stamp.0;
        assert!(
            foreign
                .advance_primary_tail(s, owner, number, stamp, &mut world)
                .is_err()
        );
        assert!(
            factory
                .advance_primary_tail(s, owner, number + 1, stamp, &mut world)
                .is_err()
        );
        assert_eq!(before, super::motion_tests::snapshot(s, owner));
        assert_eq!(epoch, stamp.0);
        let result = factory.advance_primary_tail(s, owner, number, stamp, &mut world);
        if matches!(kind, Case::TailClear) {
            assert!(result.is_err());
            assert!(s.get(owner).is_none());
            assert_eq!(s.counts().residents, 0);
            self.failures += 1;
            return;
        }
        let g = s.kabuto_group(owner).unwrap();
        assert_eq!(g.primary_tail_entered, Some(number));
        assert!(!g.primary_tail_running);
        assert_eq!(g.motion_history, Some(number));
        assert_eq!(g.pending.len(), 4);
        assert!(g.primary_tail_links.transient.is_none());
        assert_ne!(
            s.get(first)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap()
                .object_flags
                & 0x8000,
            0
        );
        if matches!(kind, Case::TailFailure) {
            assert!(result.unwrap_err().contains("late tail widget"));
            assert_eq!(g.primary_tail_completed, None);
            assert!(g.geometry_transition_failure.is_some());
            assert_eq!(g.motion.mode, 99);
            self.failures += 1;
        } else {
            result.unwrap_or_else(|e| panic!("tail context{number}: {e}"));
            assert_eq!(g.primary_tail_completed, Some(number));
            assert_eq!(g.primary_tail_links.progress, [None; 2]);
            assert_eq!(world.sounds, 2);
            assert_eq!(world.widgets.len(), 21);
            assert_eq!(
                world.camera_requests.len() + world.camera_clears + world.camera_poses.len(),
                1
            );
            self.passed += 1;
            self.poses += world.poses;
            self.positions += world.positions;
        }
        assert!(
            factory
                .advance_primary_tail(s, owner, number, stamp, &mut world)
                .is_err()
        );
    }
    pub(super) fn finish(self) {
        if self.passed + self.held > 0 {
            assert_eq!(self.passed + self.held, 512);
            assert!(self.poses > 0);
            assert!(self.positions > 0);
            println!(
                "kabuto_primary_tail owned={} held_refused={} authored_poses={} position_entries={} primary_complete=false",
                self.passed, self.held, self.poses, self.positions
            );
        }
        if self.failures > 0 {
            assert_eq!(self.failures, 1);
        }
    }
}
