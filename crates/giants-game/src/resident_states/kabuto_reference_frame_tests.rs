use super::*;
use crate::{
    kabuto_initial::Factory, kabuto_reference_frame as frame, kabuto_reference_target as target,
};
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Case {
    Success,
    Failure,
    Clear,
    Replace,
    Mutate,
    Feedback,
    FeedbackFailure,
}
struct World<'a> {
    factory: &'a Factory,
    target: Id,
    number: u64,
    case: Case,
    selected: Option<Id>,
    marker: f32,
    calls: usize,
    pose_reads: usize,
    moved_x: Option<f32>,
    replacement: Option<Id>,
}
impl frame::Services for World<'_> {
    fn bounds(&mut self, s: &Session, target: Id) -> Result<target::Bounds, String> {
        self.calls += 1;
        assert_eq!(target, self.target);
        let p = s
            .get(target)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap()
            .position;
        Ok(target::Bounds {
            lower: p.map(|v| v - 0.25),
            upper: p.map(|v| v + 0.25),
        })
    }
    fn clock(&mut self, s: &Session, owner: Id) -> Result<frame::Clock, String> {
        self.calls += 1;
        let state = s.kabuto_query_target_state(owner).unwrap();
        assert_eq!(
            (state.raw, state.accepted),
            (Some(self.target), Some(self.target))
        );
        assert_eq!(
            s.kabuto_group(owner)
                .unwrap()
                .primary_tail_links
                .interaction,
            Some(self.target)
        );
        Ok(frame::Clock {
            global_seconds: 0.25,
            areas: vec![],
        })
    }
    fn announce(&mut self, s: &mut Session, owner: Id) -> Result<(), String> {
        self.calls += 1;
        let state = s.kabuto_query_target_state(owner).unwrap();
        assert_eq!(state.accepted, Some(self.target));
        assert_eq!(state.remaining, 0.);
        assert_eq!(
            s.kabuto_group(owner).unwrap().reference_query.clock_word,
            Some(0.25f32.to_bits())
        );
        self.selected = state.accepted;
        Ok(())
    }
    fn reset_marker(&mut self, s: &mut Session, owner: Id) -> Result<(), String> {
        self.calls += 1;
        assert_eq!(self.selected, Some(self.target));
        assert_eq!(s.kabuto_query_target_state(owner).unwrap().remaining, 0.);
        self.marker = 1.;
        match self.case {
            Case::Failure => Err("declared marker entry failed after its effect".into()),
            Case::Clear => {
                s.clear();
                Ok(())
            }
            Case::Replace => {
                *s = Session::new(owner.ticket() + 1);
                self.replacement =
                    Some(self.factory.construct_declared(s, Blending::Disabled, 1)?);
                Ok(())
            }
            Case::Mutate => {
                self.factory
                    .publish_reference(s, owner, self.number, Some(self.target), 1.)?;
                let f = s
                    .residents
                    .get_mut(&owner.serial)
                    .unwrap()
                    .construction_fields
                    .as_mut()
                    .unwrap();
                f.position[0] += 1.;
                self.moved_x = Some(f.position[0]);
                Ok(())
            }
            Case::Success | Case::Feedback | Case::FeedbackFailure => Ok(()),
        }
    }
    fn parent(&mut self, s: &Session, target: Id) -> Result<Option<Id>, String> {
        self.calls += 1;
        assert!(self.case == Case::Mutate);
        Ok(s.get(target).unwrap().attachment.map(|a| a.parent))
    }
    fn geometry(&mut self, s: &Session, owner: Id) -> Result<frame::Geometry, String> {
        self.calls += 1;
        self.pose_reads += 1;
        assert_eq!(
            Some(
                s.get(owner)
                    .unwrap()
                    .construction_fields
                    .as_ref()
                    .unwrap()
                    .position[0]
            ),
            self.moved_x
        );
        // Declared analytic geometry, not a live current-axis producer.
        Ok(frame::Geometry {
            height: 0.,
            axis: [0.; 3],
        })
    }
}
pub(super) fn probe(
    factory: &Factory,
    foreign: &Factory,
    s: &mut Session,
    owner: Id,
    number: u64,
    case: Case,
) {
    let target = super::reference_query_tests::probe(factory, foreign, s, owner, number);
    let p = s.get(owner).unwrap().seed.storage.data().unwrap();
    let mask = (0..3).fold(0, |v, i| {
        v | u32::from_le_bytes(p[0x334 + i * 4..0x338 + i * 4].try_into().unwrap())
    });
    let mut saved = vec![];
    for id in s.list(0).into_iter().filter(|id| *id != owner) {
        let f = s
            .residents
            .get_mut(&id.serial)
            .unwrap()
            .construction_fields
            .as_mut()
            .unwrap();
        saved.push((id, f.object_flags, f.configuration[16]));
        if id == target {
            f.object_flags &= !0x4000c000;
            f.configuration[16] = mask;
        } else {
            f.object_flags |= 0x4000;
        }
    }
    let p = s
        .get(target)
        .unwrap()
        .construction_fields
        .as_ref()
        .unwrap()
        .position;
    let ray = || crate::kabuto_reference_query::Ray {
        origin: [p[0] - 10., p[1], p[2]],
        direction: [1., 0., 0.],
        forward_limit: 20.,
        score_limit: 1.,
    };
    let mut world = World {
        factory,
        target,
        number,
        case,
        selected: None,
        marker: 0.,
        calls: 0,
        pose_reads: 0,
        moved_x: None,
        replacement: None,
    };
    assert!(s.kabuto_query_target_state(owner).is_err());
    assert!(factory.captured_query_frame(s, owner, number).is_err());
    assert!(
        foreign
            .advance_reference_query(s, owner, number, ray(), &mut world)
            .is_err()
    );
    assert!(
        factory
            .advance_reference_query(s, owner, number + 1, ray(), &mut world)
            .is_err()
    );
    assert_eq!(world.calls, 0);
    let result = factory.advance_reference_query(s, owner, number, ray(), &mut world);
    assert_eq!(world.selected, Some(target));
    assert_eq!(world.marker, 1.);
    if case == Case::Replace {
        assert!(result.is_err());
        assert!(s.get(owner).is_none());
        let replacement = world.replacement.unwrap();
        assert_eq!(replacement.serial, owner.serial);
        assert_ne!(replacement.ticket(), owner.ticket());
        let g = s.kabuto_group(replacement).unwrap();
        assert!(g.geometry_transition_failure.is_none());
        assert!(g.reference_query.entered.is_none());
        assert!(!g.reference_query.running);
        assert!(g.reference_query.completed.is_none());
        return;
    }
    if case == Case::Clear {
        assert!(result.is_err());
        assert!(s.get(owner).is_none());
        assert_eq!(s.counts().residents, 0);
        assert!(factory.captured_query_frame(s, owner, number).is_err());
        return;
    }
    let state = s.kabuto_query_target_state(owner).unwrap();
    assert_eq!((state.raw, state.accepted), (Some(target), Some(target)));
    let g = s.kabuto_group(owner).unwrap();
    assert!(!g.reference_query.running);
    assert_eq!(g.motion_history, Some(number));
    assert_eq!(g.pending.len(), 4);
    assert_eq!(
        g.reference_query.caller.unwrap().retained_time,
        0.25f32.to_bits()
    );
    if case == Case::Failure {
        assert!(result.unwrap_err().contains("marker entry failed"));
        assert_eq!(g.reference_query.completed, None);
        assert_eq!(state.remaining, 0.);
        assert!(g.geometry_transition_failure.is_some());
        assert!(factory.captured_query_frame(s, owner, number).is_err());
    } else {
        let view = result.unwrap();
        assert_eq!((view.owner, view.number), (owner, number));
        assert_eq!(view.caller_scratch.retained_time, 0.25f32.to_bits());
        assert_eq!(state.remaining, 1.5);
        assert_eq!(state.retained, None);
        assert_eq!(
            factory
                .captured_query_frame(s, owner, number)
                .unwrap()
                .target
                .accepted,
            Some(target)
        );
        assert!(factory.captured_query_frame(s, owner, number + 1).is_err());
        if case == Case::Mutate {
            assert_eq!(world.pose_reads, 1);
        } else {
            assert_eq!(world.pose_reads, 0);
        }
    }
    let calls = world.calls;
    assert!(
        factory
            .advance_reference_query(s, owner, number, ray(), &mut world)
            .is_err()
    );
    assert_eq!(world.calls, calls);
    for (id, flags, mask) in saved {
        let f = s
            .residents
            .get_mut(&id.serial)
            .unwrap()
            .construction_fields
            .as_mut()
            .unwrap();
        f.object_flags = flags;
        f.configuration[16] = mask;
    }
    if matches!(case, Case::Feedback | Case::FeedbackFailure) {
        feedback_probe(
            factory,
            foreign,
            s,
            owner,
            target,
            number,
            case == Case::FeedbackFailure,
        );
    }
}

fn feedback_probe(
    factory: &Factory,
    foreign: &Factory,
    s: &mut Session,
    owner: Id,
    target: Id,
    number: u64,
    fail: bool,
) {
    struct Transform {
        target: Id,
        calls: usize,
        fail: bool,
    }
    impl crate::kabuto_query_feedback::Services for Transform {
        fn target_transform(
            &mut self,
            s: &Session,
            target: Id,
            factor: f32,
        ) -> Result<crate::actor_affine::Cache, String> {
            assert_eq!(target, self.target);
            assert_eq!(factor, 2.);
            assert!(s.get(target).is_some());
            self.calls += 1;
            if self.fail {
                return Err("declared target transform unavailable".into());
            }
            // Analytic supplied service output, not a live transform producer.
            let mut matrix = crate::procedural_aim::Matrix::identity().values;
            for (i, value) in [13., 17., 19.].into_iter().enumerate() {
                matrix[i * 4 + 3] = value;
            }
            Ok(crate::actor_affine::Cache {
                flags: 0x12,
                matrix,
            })
        }
    }
    let effect = s.kabuto_group(owner).unwrap().final_effect;
    assert_eq!(s.get(effect).unwrap().kabuto_effect_scalar, Some(0.));
    let position = s
        .get(effect)
        .unwrap()
        .construction_fields
        .as_ref()
        .unwrap()
        .position;
    let mut world = Transform {
        target,
        calls: 0,
        fail,
    };
    assert!(factory.captured_query_feedback(s, owner, number).is_err());
    assert!(
        foreign
            .advance_query_feedback(s, owner, number, &mut world)
            .is_err()
    );
    assert!(
        factory
            .advance_query_feedback(s, owner, number + 1, &mut world)
            .is_err()
    );
    assert_eq!(world.calls, 0);
    let result = factory.advance_query_feedback(s, owner, number, &mut world);
    assert_eq!(world.calls, 1);
    if fail {
        assert!(result.unwrap_err().contains("transform unavailable"));
        assert_eq!(s.get(effect).unwrap().kabuto_effect_scalar, Some(0.));
        assert_eq!(
            s.get(effect)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap()
                .position,
            position
        );
        assert!(
            s.kabuto_group(owner)
                .unwrap()
                .geometry_transition_failure
                .is_some()
        );
        assert!(factory.captured_query_feedback(s, owner, number).is_err());
    } else {
        let view = result.unwrap();
        assert_eq!(
            (view.owner, view.number, view.effect),
            (owner, number, effect)
        );
        assert_eq!(view.state.scalar, 0.);
        assert_eq!(view.state.flags & 0x4000_0000, 0);
        assert_eq!(view.state.position, [13., 17., 19.]);
        let a = s.get(effect).unwrap();
        assert_eq!(
            a.construction_fields.as_ref().unwrap().position,
            view.state.position
        );
        assert_eq!(a.affine_cache.as_ref().unwrap().value.flags, 0x12);
        assert!(factory.captured_query_feedback(s, owner, number).is_ok());
        assert!(
            factory
                .captured_query_feedback(s, owner, number + 1)
                .is_err()
        );
    }
    assert_eq!(s.kabuto_group(owner).unwrap().pending.len(), 4);
    assert_eq!(s.kabuto_group(owner).unwrap().motion_history, Some(number));
    assert!(
        factory
            .advance_query_feedback(s, owner, number, &mut world)
            .is_err()
    );
    assert_eq!(world.calls, 1);
}
