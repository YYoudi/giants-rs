use super::reference_frame_tests::Case;
use super::*;
use crate::{kabuto_initial::Factory, kabuto_reaction_admission as reaction};

struct World {
    owner: Id,
    fail: bool,
    deferred: bool,
    stops: usize,
    notices: usize,
    clocks: usize,
}
impl reaction::Services for World {
    fn global_request(&mut self, s: &Session) -> Result<bool, String> {
        assert!(s.kabuto_group(self.owner).unwrap().reaction.running);
        Ok(self.deferred)
    }
    fn stop_sound(&mut self, s: &mut Session, owner: Id, id: u32) -> Result<(), String> {
        assert_eq!((owner, id), (self.owner, 0x486));
        self.stops += 1;
        s.residents
            .get_mut(&owner.serial)
            .unwrap()
            .construction_fields
            .as_mut()
            .unwrap()
            .object_flags |= 0x400;
        if self.fail {
            return Err("declared sound stop refused after effect".into());
        }
        Ok(())
    }
    fn network(
        &mut self,
        s: &Session,
        owner: Id,
    ) -> Result<crate::kabuto_resource_phase::Network, String> {
        Ok(crate::kabuto_resource_phase::Network {
            session_mode: 4,
            peer_ids: vec![19],
            local_peer: 19,
            owner_word: s
                .get(owner)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap()
                .dispatch_context,
        })
    }
    fn notify_stop_sound(
        &mut self,
        s: &mut Session,
        owner: Id,
        packet: &mut u32,
    ) -> Result<(), String> {
        self.notices += 1;
        assert_eq!(*packet, 0x486);
        assert_eq!(
            s.kabuto_group(owner)
                .unwrap()
                .reaction
                .caller
                .unwrap()
                .retained_time,
            0x486
        );
        // Supplied transport mutation, not actual network/body equivalence.
        *packet ^= 0x12340000;
        Ok(())
    }
    fn clock(
        &mut self,
        s: &Session,
        owner: Id,
    ) -> Result<crate::kabuto_resource_phase::Clock, String> {
        self.clocks += 1;
        assert_eq!(
            s.kabuto_group(owner)
                .unwrap()
                .reaction
                .caller
                .unwrap()
                .retained_time,
            0x12340486
        );
        Ok(crate::kabuto_resource_phase::Clock {
            global_seconds: 0.125,
            areas: vec![],
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
    let decision = s
        .kabuto_group(owner)
        .unwrap()
        .resource_phase_view
        .unwrap()
        .resources
        .unwrap();
    let previous = factory
        .captured_query_frame(s, owner, number)
        .unwrap()
        .caller_scratch;
    let a = s.residents.get_mut(&owner.serial).unwrap();
    // Deliberately differ from the earlier retained mode/clip. Native admission
    // uses its resource-entry locals even after intervening actor changes.
    a.kabuto_group.as_mut().unwrap().motion.mode = 14;
    a.construction_fields.as_mut().unwrap().action = 14;
    a.channels.as_mut().unwrap()[0][0] = 166;
    a.construction_fields.as_mut().unwrap().channels[0][0] = 166;
    a.construction_fields.as_mut().unwrap().object_flags |= 0x80000;
    let p = a.seed.storage.data_mut().unwrap();
    // Explicit common reaction context; retained decision/clip/caller are NOT
    // replaced. This is an independent CPU context, not live input acceptance.
    p[..4].copy_from_slice(&0x30001u32.to_le_bytes());
    p[0x344..0x348].copy_from_slice(&0.5f32.to_le_bytes());
    let buttons = if matches!(case, Case::ChargeFailure | Case::ChargeSelect) {
        8u32
    } else {
        0
    };
    a.kabuto_group.as_mut().unwrap().motion.commands[4..8].copy_from_slice(&buttons.to_le_bytes());
    let mut world = World {
        owner,
        fail: case == Case::ReactionFailure,
        deferred: case == Case::ReactionDeferred,
        stops: 0,
        notices: 0,
        clocks: 0,
    };
    assert!(
        factory
            .captured_reaction_admission(s, owner, number)
            .is_err()
    );
    assert!(
        foreign
            .advance_reaction_admission(s, owner, number, &mut world)
            .is_err()
    );
    assert!(
        factory
            .advance_reaction_admission(s, owner, number + 1, &mut world)
            .is_err()
    );
    assert_eq!(world.stops + world.notices + world.clocks, 0);
    let result = factory.advance_reaction_admission(s, owner, number, &mut world);
    let g = s.kabuto_group(owner).unwrap();
    assert!(!g.reaction.running);
    assert_eq!(g.pending.len(), 4);
    assert_eq!(g.motion_history, Some(number));
    assert!(g.action_select_completed.is_none());
    assert!(g.animation_commit_completed.is_none());
    assert!(g.primary_tail_completed.is_none());
    if world.fail || world.deferred {
        let error = result.unwrap_err();
        assert!(error.contains(if world.fail {
            "sound stop refused"
        } else {
            "GlobalRequest"
        }));
        assert!(g.geometry_transition_failure.is_some());
        assert!(g.reaction.completed.is_none());
        assert_eq!(
            g.reaction.caller.unwrap().retained_time,
            previous.retained_time
        );
        assert_eq!(world.stops, usize::from(world.fail));
        assert_eq!(world.notices + world.clocks, 0);
        if world.fail {
            assert_ne!(
                s.get(owner)
                    .unwrap()
                    .construction_fields
                    .as_ref()
                    .unwrap()
                    .object_flags
                    & 0x400,
                0
            );
        }
        assert!(
            factory
                .captured_reaction_admission(s, owner, number)
                .is_err()
        );
    } else {
        let view = result.unwrap();
        assert_eq!((view.owner, view.number), (owner, number));
        assert_eq!(view.next, reaction::Next::ResourceCharge);
        assert_eq!(view.input.earlier_behavior, decision.initial_mode);
        assert_eq!(view.input.previous_clip, decision.slot);
        assert_eq!(
            view.input.previous_selection,
            decision.previous_selection as i32
        );
        assert_eq!(view.input.wrapped, decision.wrapped);
        assert_eq!(view.caller.retained_time, 0x12340486);
        assert_eq!(view.caller.packet_word, previous.packet_word);
        assert_eq!((world.stops, world.notices, world.clocks), (1, 1, 1));
        let a = s.get(owner).unwrap();
        let p = a.seed.storage.data().unwrap();
        assert_eq!(u32::from_le_bytes(p[..4].try_into().unwrap()) & 0x30000, 0);
        assert_eq!(
            f32::from_le_bytes(p[0x344..0x348].try_into().unwrap()),
            0.375
        );
        assert_eq!(
            g.physics.as_ref().unwrap().words(),
            crate::kabuto_physics::Physics::from_owned_payload(p).words()
        );
        assert!(
            factory
                .captured_reaction_admission(s, owner, number)
                .is_ok()
        );
        assert!(
            factory
                .captured_reaction_admission(s, owner, number + 1)
                .is_err()
        );
    }
    let calls = world.stops + world.notices + world.clocks;
    assert!(
        factory
            .advance_reaction_admission(s, owner, number, &mut world)
            .is_err()
    );
    assert_eq!(world.stops + world.notices + world.clocks, calls);
    if matches!(
        case,
        Case::Charge | Case::ChargeFailure | Case::ChargeSelect
    ) {
        super::action_charge_tests::probe(factory, foreign, s, owner, number, case);
    }
}
