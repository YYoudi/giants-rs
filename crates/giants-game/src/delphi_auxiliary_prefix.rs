//! Acquired auxiliary-request selection before Delphi's physical/secondary phases.
//! The caller supplies a current, owned list12 snapshot and relationships. Selection
//! requires an action continuation; it does not execute or acknowledge that action.

#[derive(Clone, Copy, Debug)]
pub struct Input {
    /// The command-button word, separate from the actor's control flags.
    pub command_buttons: u32,
    pub response_flags: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Selection<A, S> {
    pub auxiliary: A,
    pub auxiliary_state: S,
    pub candidate: A,
    pub candidate_state: S,
    pub mode: u32,
}

/// All reads must refer to the same source-ordered phase snapshot. State handles
/// represent a loaded relationship, not initialized actors or ready geometry.
/// Providers retain responsibility for foreign/stale identities and missing state.
pub trait World {
    type Actor: Copy + Eq;
    type Node: Copy;
    type State: Copy;

    fn authority(&mut self, owner: Self::Actor) -> Result<bool, String>;
    fn auxiliary(&mut self, owner: Self::Actor) -> Result<Option<Self::Actor>, String>;
    fn list12_head(&mut self) -> Result<Option<Self::Node>, String>;
    fn state(&mut self, actor: Self::Actor) -> Result<Self::State, String>;
    fn actor(&mut self, node: Self::Node) -> Result<Self::Actor, String>;
    fn related_owner(&mut self, actor: Self::Actor) -> Result<Option<Self::Actor>, String>;
    fn class(&mut self, actor: Self::Actor) -> Result<i32, String>;
    fn mode(&mut self, state: Self::State) -> Result<u32, String>;
    fn next(&mut self, node: Self::Node) -> Result<Option<Self::Node>, String>;
}

pub type Request<W> = Selection<<W as World>::Actor, <W as World>::State>;

/// `node_budget` bounds the caller's owned snapshot, not native cyclic-list
/// behavior. Exhaustion refuses the incomplete traversal. No selected request is
/// cleared or completed here; required construction, placement and effects follow.
pub fn select<W: World>(
    owner: W::Actor,
    input: Input,
    world: &mut W,
    mut node_budget: usize,
) -> Result<Option<Request<W>>, String> {
    if !world.authority(owner)?
        || input.command_buttons & 0x0100_0000 == 0
        || input.response_flags & 0x0080_0000 == 0
    {
        return Ok(None);
    }
    let Some(auxiliary) = world.auxiliary(owner)? else {
        return Ok(None);
    };
    let mut node = world.list12_head()?;
    let auxiliary_state = world.state(auxiliary)?;
    while let Some(current) = node {
        if node_budget == 0 {
            return Err("Delphi auxiliary snapshot traversal budget exhausted".into());
        }
        node_budget -= 1;
        let candidate = world.actor(current)?;
        let related = world.related_owner(candidate)?;
        let candidate_state = world.state(candidate)?;
        if related == Some(owner) && world.class(candidate)? == 11 {
            let mode = world.mode(candidate_state)?;
            if mode != 0 {
                return Ok(Some(Selection {
                    auxiliary,
                    auxiliary_state,
                    candidate,
                    candidate_state,
                    mode,
                }));
            }
        }
        node = world.next(current)?;
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Provider {
        authorized: Result<bool, String>,
        related: bool,
        populated: bool,
        fail_state: Option<u8>,
        reads: Vec<&'static str>,
    }
    impl World for Provider {
        type Actor = u8;
        type Node = u8;
        type State = u8;
        fn authority(&mut self, _: u8) -> Result<bool, String> {
            self.reads.push("authority");
            self.authorized.clone()
        }
        fn auxiliary(&mut self, _: u8) -> Result<Option<u8>, String> {
            self.reads.push("auxiliary");
            Ok(Some(2))
        }
        fn list12_head(&mut self) -> Result<Option<u8>, String> {
            self.reads.push("list12");
            Ok(self.populated.then_some(0))
        }
        fn state(&mut self, actor: u8) -> Result<u8, String> {
            self.reads.push("state");
            if self.fail_state == Some(actor) {
                Err("state unavailable".into())
            } else {
                Ok(actor * 10)
            }
        }
        fn actor(&mut self, _: u8) -> Result<u8, String> {
            self.reads.push("actor");
            Ok(3)
        }
        fn related_owner(&mut self, _: u8) -> Result<Option<u8>, String> {
            self.reads.push("related");
            Ok(self.related.then_some(1))
        }
        fn class(&mut self, _: u8) -> Result<i32, String> {
            self.reads.push("class");
            Ok(11)
        }
        fn mode(&mut self, _: u8) -> Result<u32, String> {
            self.reads.push("mode");
            Ok(7)
        }
        fn next(&mut self, _: u8) -> Result<Option<u8>, String> {
            Err("later list node unavailable".into())
        }
    }
    fn provider() -> Provider {
        Provider {
            authorized: Ok(true),
            related: true,
            populated: true,
            fail_state: None,
            reads: Vec::new(),
        }
    }
    fn input() -> Input {
        Input {
            command_buttons: 0x0100_0000,
            response_flags: 0x0080_0000,
        }
    }
    #[test]
    fn required_reads_precede_candidate_filtering_and_empty_list_exit() {
        let mut p = provider();
        p.populated = false;
        p.fail_state = Some(2);
        assert!(select(1, input(), &mut p, 0).is_err());
        assert_eq!(p.reads, ["authority", "auxiliary", "list12", "state"]);

        let mut p = provider();
        p.related = false;
        p.fail_state = Some(3);
        assert!(select(1, input(), &mut p, 1).is_err());
        assert_eq!(
            p.reads,
            [
                "authority",
                "auxiliary",
                "list12",
                "state",
                "actor",
                "related",
                "state"
            ]
        );
    }
    #[test]
    fn authority_and_first_selection_bound_later_access() {
        let mut p = provider();
        p.authorized = Err("authority unavailable".into());
        let mut inactive = input();
        inactive.command_buttons = 0;
        assert!(select(1, inactive, &mut p, 0).is_err());
        assert_eq!(p.reads, ["authority"]);

        let mut p = provider();
        let selected = select(1, input(), &mut p, 1).unwrap().unwrap();
        assert_eq!(selected.candidate, 3);
        assert_eq!(selected.candidate_state, 30);
        assert_eq!(selected.mode, 7);
        assert_eq!(p.reads.last(), Some(&"mode"));

        let mut p = provider();
        assert!(
            select(1, input(), &mut p, 0)
                .unwrap_err()
                .contains("budget")
        );
        assert_eq!(p.reads, ["authority", "auxiliary", "list12", "state"]);
    }
}
