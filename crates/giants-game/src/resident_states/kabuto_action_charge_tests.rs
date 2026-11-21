use super::reference_frame_tests::Case;
use super::*;
use crate::{kabuto_action_charge as charge, kabuto_initial::Factory};
struct World {
    fail: bool,
    calls: usize,
}
impl charge::Services for World {
    fn target_status(
        &mut self,
        s: &mut Session,
        owner: Id,
        target: Option<Id>,
        kind: u32,
    ) -> Result<bool, String> {
        self.calls += 1;
        assert_eq!(kind, 2);
        assert_eq!(target, s.kabuto_query_target_state(owner).unwrap().accepted);
        assert_eq!(
            s.kabuto_group(owner)
                .unwrap()
                .charge
                .caller
                .unwrap()
                .retained_time,
            0
        );
        // Declared mutation at the status entry. The following position read
        // must use this actual actor, including when the status call refuses.
        s.residents
            .get_mut(&owner.serial)
            .unwrap()
            .construction_fields
            .as_mut()
            .unwrap()
            .position[0] += 13.;
        if self.fail {
            Err("declared action status refused after mutation".into())
        } else {
            Ok(false)
        }
    }
    fn random_two(&mut self, _: &mut Session, _: Id) -> Result<u32, String> {
        panic!("partial-resource test must not request a full-resource draw")
    }
    fn mark_target(&mut self, _: &mut Session, _: Id) -> Result<(), String> {
        panic!("no-target test must not mark a target")
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
    let earlier = factory
        .captured_reaction_admission(s, owner, number)
        .unwrap();
    let decision = s
        .kabuto_group(owner)
        .unwrap()
        .resource_phase_view
        .unwrap()
        .resources
        .unwrap();
    let before = s
        .get(owner)
        .unwrap()
        .construction_fields
        .as_ref()
        .unwrap()
        .position;
    let payload = s.get(owner).unwrap().seed.storage.data().unwrap();
    let maximum = f32::from_le_bytes(payload[0x134..0x138].try_into().unwrap());
    let meter = f32::from_le_bytes(payload[0x140..0x144].try_into().unwrap());
    let mut world = World {
        fail: case == Case::ChargeFailure,
        calls: 0,
    };
    assert!(factory.captured_action_charge(s, owner, number).is_err());
    assert!(
        foreign
            .advance_action_charge(s, owner, number, &mut world)
            .is_err()
    );
    assert!(
        factory
            .advance_action_charge(s, owner, number + 1, &mut world)
            .is_err()
    );
    assert_eq!(world.calls, 0);
    let result = factory.advance_action_charge(s, owner, number, &mut world);
    let g = s.kabuto_group(owner).unwrap();
    assert!(!g.charge.running);
    assert_eq!(g.pending.len(), 4);
    assert_eq!(g.motion_history, Some(number));
    assert!(g.action_select_completed.is_none());
    assert!(g.animation_commit_completed.is_none());
    assert!(g.primary_tail_completed.is_none());
    let p = s.get(owner).unwrap().seed.storage.data().unwrap();
    if world.fail {
        assert!(result.unwrap_err().contains("status refused"));
        assert!(g.geometry_transition_failure.is_some());
        assert!(g.charge.completed.is_none());
        assert_eq!(world.calls, 1);
        assert_eq!(
            s.get(owner)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap()
                .position[0],
            before[0] + 13.
        );
        assert_eq!(f32::from_le_bytes(p[4..8].try_into().unwrap()), 10.);
        assert!(factory.captured_action_charge(s, owner, number).is_err());
    } else {
        let view = result.unwrap();
        assert_eq!(view.input.earlier_behavior, decision.initial_mode);
        assert_eq!(
            view.input.previous_definition,
            decision.previous_selection as i32
        );
        assert_eq!(view.input.charge.to_bits(), decision.charge.to_bits());
        assert_eq!(view.caller.packet_word, earlier.caller.packet_word);
        // This alias is deliberately overwritten by the acquired charge entry.
        assert_eq!(view.caller.retained_time, 0);
        assert_ne!(earlier.caller.retained_time, 0);
        if case == Case::Charge {
            assert_eq!(view.entry_flags & 0xa, 0);
            assert_eq!(view.next, charge::Next::FlaggedPolicy);
            assert_eq!(world.calls, 0);
        } else {
            assert_eq!(view.entry_flags & 6, 2);
            assert_eq!(view.next, charge::Next::CheckSelection);
            assert_eq!(world.calls, 1);
            assert!(decision.charge >= 0.33 && decision.charge < 1.);
            assert_eq!(view.caller.behavior, 6);
            assert_eq!(
                view.caller.slot,
                g.cold_animation_cache.as_ref().unwrap().entries()[view.caller.definition as usize]
                    .definition
                    .animation as u32
            );
            assert!(g.charge.target.is_none());
            assert_eq!(
                f32::from_le_bytes(p[0x60..0x64].try_into().unwrap()),
                before[0] + 13.
            );
            assert_eq!(f32::from_le_bytes(p[0x78..0x7c].try_into().unwrap()), -1.);
            assert_eq!(u32::from_le_bytes(p[0x138..0x13c].try_into().unwrap()), 0);
            assert_eq!(
                f32::from_le_bytes(p[0x134..0x138].try_into().unwrap()).to_bits(),
                ((f64::from(maximum) - f64::from(decision.charge)) as f32).to_bits()
            );
            assert_eq!(
                f32::from_le_bytes(p[0x140..0x144].try_into().unwrap()).to_bits(),
                ((f64::from(meter) + 1.5) as f32).to_bits()
            );
        }
        assert_eq!(
            g.physics.as_ref().unwrap().words(),
            crate::kabuto_physics::Physics::from_owned_payload(p).words()
        );
        assert!(factory.captured_action_charge(s, owner, number).is_ok());
    }
    let calls = world.calls;
    assert!(
        factory
            .advance_action_charge(s, owner, number, &mut world)
            .is_err()
    );
    assert_eq!(world.calls, calls);
}
