//! Source-ordered mission command12 protocol, requiring real actor services.
//! A finished protocol is not mission readiness or a complete living actor.
//! Service errors retain earlier writes/effects, as the native path does;
//! callers must inspect the failure before retrying or cancelling the mission.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Actor<Id> {
    pub state: u32,
    /// Shared status/response word (including the carrying and dirty bits).
    pub response_flags: u32,
    pub object_flags: u32,
    pub control_flags: u32,
    pub behavior: u32,
    pub action: u32,
    pub parent: Option<Id>,
}

pub trait Backend {
    type Id: Copy + Eq;
    fn actor(&mut self, id: Self::Id) -> Result<&mut Actor<Self::Id>, String>;
    /// First entry in the actual ordered carrying collection, not a class lookup.
    fn carrying_head(&mut self) -> Result<Option<Self::Id>, String>;
    fn detach(&mut self, child: Self::Id, parent: Self::Id, mode: u32) -> Result<(), String>;
    fn retire(&mut self, actor: Self::Id) -> Result<(), String>;
    /// Construct a new, independently owned actor, never an alias of the player.
    fn construct(&mut self, class: u32, registration: u32) -> Result<Self::Id, String>;
    fn pickup(&mut self, child: Self::Id, parent: Self::Id, argument: u32) -> Result<(), String>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Slot,
    Player,
    CarryingHead,
    Previous,
    Detach,
    Retire,
    Construct,
    Initialize,
    Pickup,
}
#[derive(Debug, PartialEq, Eq)]
pub struct Failure<Id> {
    pub stage: Stage,
    pub message: String,
    /// An allocated child remains owned by the backend even if pickup fails.
    pub constructed: Option<Id>,
    pub retired: Option<Id>,
    pub slot_written: bool,
}
#[derive(Debug, PartialEq, Eq)]
pub struct Applied<Id> {
    pub child: Id,
    pub retired: Option<Id>,
}

/// The player and backend must belong to the caller's actual mission session.
/// This function deliberately supplies no fallback backend or successful ack.
pub fn apply<B: Backend>(
    slot: &mut [u8],
    class: u32,
    player: B::Id,
    backend: &mut B,
) -> Result<Applied<B::Id>, Failure<B::Id>> {
    let mut retired = None;
    let mut constructed = None;
    let mut slot_written = false;
    macro_rules! call {
        ($stage:ident, $operation:expr) => {
            $operation.map_err(|message| Failure {
                stage: Stage::$stage,
                message,
                constructed,
                retired,
                slot_written,
            })?
        };
    }
    // Only this destination is written by command12; the common slot contract
    // still requires complete storage before performing any external action.
    call!(
        Slot,
        (slot.len() >= crate::mission_slot::SLOT_SIZE)
            .then_some(())
            .ok_or_else(|| "mission child12 slot storage is incomplete".to_string())
    );
    slot[0x88..0x8c].copy_from_slice(&class.to_le_bytes());
    slot_written = true;
    let carrying = call!(Player, backend.actor(player)).response_flags & 0x80_0000 != 0;
    if carrying && let Some(previous) = call!(CarryingHead, backend.carrying_head()) {
        let old = *call!(Previous, backend.actor(previous));
        if old.state == 18 && old.parent == Some(player) {
            call!(Detach, backend.detach(previous, player, 3));
        }
        // The first entry is retired even if it was another state or parent.
        // Do not skip it or search for an arbitrary class18 replacement.
        call!(Retire, backend.retire(previous));
        retired = Some(previous);
    }
    let child = call!(Construct, backend.construct(class, 0x8000_0000));
    constructed = Some(child);
    call!(
        Initialize,
        (child != player)
            .then_some(())
            .ok_or_else(|| "child constructor returned the player identity".to_string())
    );
    let fields = call!(Initialize, backend.actor(child));
    fields.action = 3;
    fields.behavior = 3;
    fields.control_flags |= 0x20;
    fields.object_flags |= 0x400;
    call!(Pickup, backend.pickup(child, player, 0));
    Ok(Applied { child, retired })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    struct Owned {
        actors: BTreeMap<u64, Actor<u64>>,
        fail: Option<Stage>,
        calls: Vec<Stage>,
    }
    fn actor(parent: Option<u64>) -> Actor<u64> {
        Actor {
            state: 18,
            response_flags: 0x800000,
            object_flags: 0x11223344,
            control_flags: 0x99000000,
            behavior: 10,
            action: 12,
            parent,
        }
    }
    impl Owned {
        fn new(fail: Option<Stage>) -> Self {
            Self {
                actors: BTreeMap::from([(1, actor(None)), (2, actor(Some(1)))]),
                fail,
                calls: vec![],
            }
        }
        fn service(&mut self, stage: Stage) -> Result<(), String> {
            self.calls.push(stage);
            if self.fail == Some(stage) {
                Err("required service unavailable".into())
            } else {
                Ok(())
            }
        }
    }
    impl Backend for Owned {
        type Id = u64;
        fn actor(&mut self, id: u64) -> Result<&mut Actor<u64>, String> {
            self.actors.get_mut(&id).ok_or("actor absent".into())
        }
        fn carrying_head(&mut self) -> Result<Option<u64>, String> {
            Ok(Some(2))
        }
        fn detach(&mut self, c: u64, p: u64, m: u32) -> Result<(), String> {
            assert_eq!((c, p, m), (2, 1, 3));
            self.service(Stage::Detach)?;
            self.actor(c)?.parent = None;
            Ok(())
        }
        fn retire(&mut self, id: u64) -> Result<(), String> {
            self.service(Stage::Retire)?;
            self.actors.remove(&id).ok_or("actor absent")?;
            Ok(())
        }
        fn construct(&mut self, class: u32, mask: u32) -> Result<u64, String> {
            assert_eq!((class, mask), (11, 0x80000000));
            self.service(Stage::Construct)?;
            self.actors.insert(3, actor(None));
            Ok(3)
        }
        fn pickup(&mut self, c: u64, p: u64, a: u32) -> Result<(), String> {
            assert_eq!((c, p, a), (3, 1, 0));
            self.service(Stage::Pickup)?;
            self.actor(c)?.parent = Some(p);
            Ok(())
        }
    }
    #[test]
    fn missing_services_preserve_real_partial_progress_without_success_or_rollback() {
        for fail in [
            Stage::Detach,
            Stage::Retire,
            Stage::Construct,
            Stage::Pickup,
        ] {
            let mut backend = Owned::new(Some(fail));
            let mut slot = [0xa5; crate::mission_slot::SLOT_SIZE];
            let error = apply(&mut slot, 11, 1, &mut backend).unwrap_err();
            assert_eq!(error.stage, fail);
            assert!(error.slot_written);
            assert_eq!(&slot[0x88..0x8c], &11_u32.to_le_bytes());
            assert_eq!(backend.calls.last(), Some(&fail));
            assert_eq!(
                error.retired,
                if matches!(fail, Stage::Construct | Stage::Pickup) {
                    Some(2)
                } else {
                    None
                }
            );
            assert_eq!(
                error.constructed,
                if fail == Stage::Pickup { Some(3) } else { None }
            );
            assert_eq!(
                backend.actors.contains_key(&2),
                !matches!(fail, Stage::Construct | Stage::Pickup)
            );
            if fail == Stage::Retire {
                assert_eq!(backend.actors[&2].parent, None);
            }
            if fail == Stage::Pickup {
                let child = backend.actors[&3];
                assert_eq!(child.parent, None);
                assert_eq!((child.action, child.behavior), (3, 3));
                assert_eq!(child.control_flags, 0x99000020);
                assert_eq!(child.object_flags, 0x11223744);
            }
        }
    }
    #[test]
    fn destination_guard_runs_before_services_and_success_owns_only_the_new_child() {
        let mut backend = Owned::new(None);
        let mut short = [0xa5; 0x8c];
        assert_eq!(
            apply(&mut short, 11, 1, &mut backend).unwrap_err().stage,
            Stage::Slot
        );
        assert_eq!(short, [0xa5; 0x8c]);
        assert!(backend.calls.is_empty());
        let mut slot = [0xa5; crate::mission_slot::SLOT_SIZE];
        assert_eq!(
            apply(&mut slot, 11, 1, &mut backend).unwrap(),
            Applied {
                child: 3,
                retired: Some(2)
            }
        );
        assert_eq!(backend.actors[&3].parent, Some(1));
        assert!(!backend.actors.contains_key(&2));
        assert_eq!(
            backend.calls,
            [
                Stage::Detach,
                Stage::Retire,
                Stage::Construct,
                Stage::Pickup
            ]
        );
    }
}
