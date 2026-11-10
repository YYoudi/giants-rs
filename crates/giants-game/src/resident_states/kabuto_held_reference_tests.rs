use super::*;
use crate::kabuto_held_reference::{self as held, Input, Request, Response, State};

#[derive(Clone, Copy)]
pub(super) enum Case {
    Success,
    Failure,
    Clear,
    Captured,
    Commit,
    CommitFailure,
    CommitClear,
}

struct World<'a> {
    factory: &'a crate::kabuto_initial::Factory,
    number: u64,
    target: Id,
    case: Case,
    calls: usize,
    seen: Option<State<Id>>,
    prefix: Vec<u32>,
    binding: Option<crate::actor_attachment_bind::Input>,
}
impl held::World for World<'_> {
    fn session_mode(&self, _: &Session) -> Result<u32, String> {
        Ok(0)
    }
    fn network<'a>(
        &'a self,
        _: &Session,
        _: Id,
    ) -> Result<crate::kabuto_geometry_mode::NotificationContext<'a>, String> {
        Err("network producer outside declared direct-route control".into())
    }
    fn owner_word(&self, _: &Session, _: Id) -> Result<u32, String> {
        Err("owner-word producer outside declared direct-route control".into())
    }
    fn execute(
        &mut self,
        s: &mut Session,
        owner: Id,
        request: Request,
    ) -> Result<Response, String> {
        let Request::Bind { child, input } = request else {
            return Err("service outside declared binding-entry control".into());
        };
        assert_eq!(child, self.target);
        assert_eq!(input.scale_factor, 1.);
        assert_eq!(input.parent_reference, 0);
        assert!(input.notify);
        assert!(s.get(child).unwrap().construction_fields.is_some());
        let state = s.kabuto_reference_state(owner)?;
        assert_eq!(state.reference, None);
        assert_eq!(state.held, [Some(self.target), None]);
        assert_eq!(state.held_scales[0], 0.5);
        self.seen = Some(state);
        self.prefix = projected(s, owner, self.target);
        self.binding = Some(input);
        self.calls += 1;
        let factory = self.factory;
        assert!(
            factory
                .advance_held_references(s, owner, self.number, input_for_test(), self)
                .is_err()
        );
        // Declared entry-provider effects on the actual Session, not execution
        // of the original binding/animation/transport bodies.
        factory.publish_reference(s, owner, self.number, Some(owner), -0.)?;
        s.residents
            .get_mut(&owner.serial)
            .unwrap()
            .construction_fields
            .as_mut()
            .unwrap()
            .control_flags ^= 0x0100_0000;
        s.residents
            .get_mut(&owner.serial)
            .unwrap()
            .seed
            .storage
            .data_mut()
            .unwrap()[0x300..0x304]
            .copy_from_slice(&0x12345678u32.to_le_bytes());
        match self.case {
            Case::Clear => {
                s.clear();
                Ok(Response::Complete)
            }
            Case::Failure => Err("declared late binding failure".into()),
            Case::Success => Ok(Response::Complete),
            Case::Captured => panic!("captured no-target route must not bind"),
            Case::Commit | Case::CommitFailure | Case::CommitClear => unreachable!(),
        }
    }
}
fn input_for_test() -> Input {
    Input {
        decision: 11,
        previous: 0,
        current: 4800,
        finished: false,
    }
}

fn projected(s: &Session, owner: Id, target: Id) -> Vec<u32> {
    let state = s.kabuto_reference_state(owner).unwrap();
    let key = |id| match id {
        None => 0,
        Some(id) if id == owner => 1,
        Some(id) if id == target => 2,
        _ => panic!("reference outside this two-actor control"),
    };
    let a = s.get(owner).unwrap();
    let p = a.seed.storage.data().unwrap();
    vec![
        key(state.reference),
        key(state.held[0]),
        key(state.held[1]),
        state.reference_scale.to_bits(),
        state.held_scales[0].to_bits(),
        state.held_scales[1].to_bits(),
        a.construction_fields.as_ref().unwrap().control_flags,
        u32::from_le_bytes(p[0x300..0x304].try_into().unwrap()),
    ]
}

fn probe_captured(
    factory: &crate::kabuto_initial::Factory,
    foreign: &crate::kabuto_initial::Factory,
    s: &mut Session,
    owner: Id,
    number: u64,
) {
    let snapshot = s.kabuto_group(owner).unwrap().resource_phase_view.unwrap();
    let r = snapshot.resources.unwrap();
    assert!(!matches!(r.initial_mode, 10 | 11));
    let words = |i: Input| (i.decision, i.previous, i.current, i.finished);
    let expected = (r.initial_mode, r.previous_frame, r.current_frame, r.wrapped);
    assert_eq!(
        words(
            factory
                .captured_held_reference_input(s, owner, number)
                .unwrap()
        ),
        expected
    );
    assert!(
        foreign
            .captured_held_reference_input(s, owner, number)
            .is_err()
    );
    assert!(
        factory
            .captured_held_reference_input(s, owner, number + 1)
            .is_err()
    );
    factory
        .publish_reference(s, owner, number, Some(owner), 0.5)
        .unwrap();
    let before = projected(s, owner, owner);
    let a = s.residents.get_mut(&owner.serial).unwrap();
    // Declared later writes to the actual owner. They must not replace the
    // earlier caller snapshot. This does not implement the intervening phases.
    let g = a.kabuto_group.as_mut().unwrap();
    g.motion.mode = 11;
    g.resource_phase_view = None;
    let f = a.construction_fields.as_mut().unwrap();
    f.action = 10;
    f.channels[0][1] = 100f32.to_bits();
    f.channels[0][2] = (-100f32).to_bits();
    f.channels[0][8] ^= 2;
    a.channels = Some(f.channels);
    let mut world = World {
        factory,
        number,
        target: owner,
        case: Case::Captured,
        calls: 0,
        seen: None,
        prefix: vec![],
        binding: None,
    };
    assert!(
        factory
            .advance_captured_held_references(s, owner, number, &mut world)
            .is_err()
    );
    assert!(
        s.kabuto_group(owner)
            .unwrap()
            .held_reference_entered
            .is_none()
    );
    s.residents
        .get_mut(&owner.serial)
        .unwrap()
        .kabuto_group
        .as_mut()
        .unwrap()
        .resource_phase_view = Some(snapshot);
    assert_eq!(
        words(
            factory
                .captured_held_reference_input(s, owner, number)
                .unwrap()
        ),
        expected
    );
    assert_eq!(
        factory
            .advance_captured_held_references(s, owner, number, &mut world)
            .unwrap(),
        held::Next::ContinueCaller
    );
    assert_eq!(world.calls, 0);
    assert_eq!(projected(s, owner, owner), before);
    let g = s.kabuto_group(owner).unwrap();
    assert_eq!(g.held_reference_completed, Some(number));
    assert_eq!(g.motion_history, Some(number));
    assert_eq!(g.pending.len(), 4);
    assert!(
        factory
            .advance_captured_held_references(s, owner, number, &mut world)
            .is_err()
    );
}

pub(super) fn probe(
    factory: &crate::kabuto_initial::Factory,
    foreign: &crate::kabuto_initial::Factory,
    s: &mut Session,
    owner: Id,
    number: u64,
    case: Case,
) {
    if matches!(case, Case::Commit | Case::CommitFailure | Case::CommitClear) {
        // The common source fixture invokes that continuation with its existing
        // ANM context/RNG/epoch services after this resource probe returns.
        return;
    }
    if matches!(case, Case::Captured) {
        probe_captured(factory, foreign, s, owner, number);
        return;
    }
    // This preceding source-backed query prepares a real ordinary actor and
    // exercises actual ownership. Capture policy/scale below remain supplied.
    let target = super::reference_query_tests::probe(factory, foreign, s, owner, number);
    let cold = s.kabuto_reference_state(owner).unwrap();
    assert!(cold.reference.is_none() && cold.held.iter().all(Option::is_none));
    assert!(
        foreign
            .publish_reference(s, owner, number, Some(target), 0.5)
            .is_err()
    );
    assert!(
        factory
            .publish_reference(s, owner, number + 1, Some(target), 0.5)
            .is_err()
    );
    let mut other = Session::new(998);
    let other_owner = factory
        .construct(
            &mut other,
            ColdFrame {
                position: [0.; 3],
                current: [0.; 3],
                previous: [0.; 3],
                angles: [0.; 3],
            },
            Blending::Disabled,
            1,
        )
        .unwrap();
    assert!(
        factory
            .publish_reference(s, owner, number, Some(other_owner), 0.5)
            .is_err()
    );
    other.clear();
    factory
        .publish_reference(s, owner, number, Some(target), 0.5)
        .unwrap();
    assert_eq!(
        s.kabuto_reference_state(owner).unwrap().reference,
        Some(target)
    );
    let control = s
        .get(owner)
        .unwrap()
        .construction_fields
        .as_ref()
        .unwrap()
        .control_flags;
    let before = projected(s, owner, target);
    let mut world = World {
        factory,
        number,
        target,
        case,
        calls: 0,
        seen: None,
        prefix: vec![],
        binding: None,
    };
    assert!(
        foreign
            .advance_held_references(s, owner, number, input_for_test(), &mut world)
            .is_err()
    );
    assert_eq!(world.calls, 0);
    let result = factory.advance_held_references(s, owner, number, input_for_test(), &mut world);
    assert_eq!(world.calls, 1);
    assert!(world.seen.is_some());
    if matches!(case, Case::Clear) {
        assert!(result.is_err());
        assert!(s.get(owner).is_none());
        assert_eq!(s.counts().residents, 0);
        assert!(s.kabuto_reference_state(owner).is_err());
    } else {
        assert_eq!(result.is_ok(), matches!(case, Case::Success));
        let state = s.kabuto_reference_state(owner).unwrap();
        assert_eq!(state.reference, Some(owner));
        assert_eq!(state.reference_scale.to_bits(), (-0f32).to_bits());
        assert_eq!(state.held, [Some(target), None]);
        assert_eq!(state.held_scales[0], 0.5);
        let a = s.get(owner).unwrap();
        let p = a.seed.storage.data().unwrap();
        for at in [0xc, 0x10, 0x14] {
            assert_eq!(u32::from_le_bytes(p[at..at + 4].try_into().unwrap()), 0);
        }
        assert_eq!(
            u32::from_le_bytes(p[0x300..0x304].try_into().unwrap()),
            0x12345678
        );
        assert_eq!(
            a.construction_fields.as_ref().unwrap().control_flags,
            control ^ 0x0100_0000
        );
        let g = a.kabuto_group.as_ref().unwrap();
        assert_eq!(g.motion_history, Some(number));
        assert_eq!(g.resource_phase_completed, Some(number));
        assert_eq!(g.pending.len(), 4);
        assert!(!g.held_reference_running);
        assert_eq!(
            g.held_reference_completed,
            matches!(case, Case::Success).then_some(number)
        );
        assert_eq!(
            g.geometry_transition_failure.is_some(),
            matches!(case, Case::Failure)
        );
        assert!(
            factory
                .advance_held_references(s, owner, number, input_for_test(), &mut world)
                .is_err()
        );
        assert_eq!(world.calls, 1);
        if matches!(case, Case::Success)
            && let Some(path) = std::env::var_os("GIANTS_KABUTO_HELD_BANK_RECEIPT")
        {
            let binding = world.binding.unwrap();
            let rows = [
                before,
                world.prefix.clone(),
                projected(s, owner, target),
                vec![
                    1,
                    2,
                    binding.scale_factor.to_bits(),
                    binding.parent_reference,
                    u32::from(binding.notify),
                ],
                vec![u32::from(result.is_ok_and(|n| n == held::Next::Processed))],
            ];
            let text = rows
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|v| format!("{v:08x}"))
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .collect::<Vec<_>>()
                .join("\n");
            std::fs::write(path, text + "\n").unwrap();
        }
        s.clear();
    }
    let next = factory
        .construct(
            s,
            ColdFrame {
                position: [0.; 3],
                current: [0.; 3],
                previous: [0.; 3],
                angles: [0.; 3],
            },
            Blending::Disabled,
            1,
        )
        .unwrap();
    assert_ne!(next, owner);
    let fresh = s.kabuto_reference_state(next).unwrap();
    assert!(fresh.reference.is_none() && fresh.held.iter().all(Option::is_none));
    assert!(s.kabuto_reference_state(owner).is_err());
    s.clear();
}
