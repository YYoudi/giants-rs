//! Post-primary dependency order. This neither runs attachments nor prepares
//! reference caches; the caller still owns the surrounding world phases.
use super::*;

impl Session {
    /// Order actual list0 registrations from typed parent/copy relationships.
    /// Attachment ownership precedes the state-62 copy source, as acquired.
    /// A late error preserves earlier moves and quarantines common traversal
    /// until clear(), using the same failure boundary as a common walk.
    pub fn order_common_dependencies(&mut self, visit_budget: usize) -> Result<usize, String> {
        if self.common_walk_running || self.common_walk_failure.is_some() {
            return Err("common walk running or failed; cancellation required".into());
        }
        let result = (|| {
            let mut parents = BTreeMap::new();
            for id in self.list(0) {
                let actor = self.get(id).ok_or("dependency actor absent or foreign")?;
                if actor.seed.storage.profile() != Profile::Debug {
                    return Err("dependency ordering outside acquired debug profile".into());
                }
                let parent = actor.attachment.map(|binding| binding.parent).or_else(|| {
                    (actor.seed.storage.state() == 62)
                        .then_some(actor.delphi_copy_source)
                        .flatten()
                        .map(|copy| copy.owner)
                });
                if let Some(parent) = parent {
                    self.get(parent)
                        .ok_or("dependency parent absent or foreign")?;
                }
                parents.insert(id.serial, parent.map(|id| id.serial));
            }
            self.links.order_dependencies(0, visit_budget, |object| {
                parents
                    .get(&object)
                    .copied()
                    .ok_or_else(|| "dependency actor not captured".into())
            })
        })();
        if let Err(error) = &result {
            self.common_walk_failure = Some(error.clone());
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn actor(session: &mut Session, state: u8) -> Id {
        let program = giants_formats::class_program::parse(&[0x1e, state, 0xff]).unwrap();
        let seed = Seed::from_program_for(
            &program,
            Profile::Debug,
            crate::arithmetic::Precision::Double53,
        )
        .unwrap();
        session
            .register(311, &seed, &[], DirectFlags::default())
            .unwrap()
    }
    fn attach(session: &mut Session, child: Id, parent: Id) {
        session.residents.get_mut(&child.serial).unwrap().attachment = Some(Attachment {
            parent,
            child_reference: 0,
            parent_reference: 1,
            behavior: 0,
        });
    }

    #[test]
    fn typed_dependencies_prefer_parent_and_keep_copy_fallback_and_other_lists() {
        let mut session = Session::new(1003);
        let child = actor(&mut session, 62);
        let copy = actor(&mut session, 62);
        let ignored = actor(&mut session, 18);
        let parent = actor(&mut session, 18);
        let alternate = actor(&mut session, 18);
        attach(&mut session, child, parent);
        for id in [child, copy, ignored] {
            session
                .residents
                .get_mut(&id.serial)
                .unwrap()
                .delphi_copy_source = Some(crate::delphi_special::CopySource {
                owner: alternate,
                index: 0,
            });
        }
        let other = session.list(1);
        let reverse = session.links.memberships(child.serial);
        session.order_common_dependencies(40).unwrap();
        assert_eq!(session.list(0), [ignored, parent, child, alternate, copy]);
        assert_eq!(session.links.memberships(child.serial), reverse);
        assert_eq!(session.list(1), other);
        assert_eq!(session.order_common_dependencies(40).unwrap(), 5);
        session.clear();
        assert_eq!(session.order_common_dependencies(1).unwrap(), 0);
        let fresh = actor(&mut session, 18);
        assert_ne!(fresh, child);
        assert_eq!(session.list(0), [fresh]);
    }

    #[test]
    fn failed_order_retains_moves_and_quarantines_walk_until_clear() {
        let mut session = Session::new(1004);
        let child = actor(&mut session, 18);
        let middle = actor(&mut session, 18);
        let parent = actor(&mut session, 18);
        attach(&mut session, child, parent);
        assert!(
            session
                .order_common_dependencies(1)
                .unwrap_err()
                .contains("budget exhausted")
        );
        assert_eq!(session.list(0), [middle, parent, child]);
        assert!(session.new_common_walk(10).is_err());
        assert!(session.order_common_dependencies(10).is_err());
        session.clear();
        let child = actor(&mut session, 18);
        let mut foreign = Session::new(1005);
        let foreign_parent = actor(&mut foreign, 18);
        attach(&mut session, child, foreign_parent);
        assert!(
            session
                .order_common_dependencies(10)
                .unwrap_err()
                .contains("foreign")
        );
        assert_eq!(session.list(0), [child]);
    }

    #[test]
    #[ignore = "Requires GIANTS_DELPHI_ROOT; actual cold group with deliberately permuted registrations, not a scheduled world tick"]
    fn source_delphi_dependency_order_keeps_group_identities_and_cleanup() {
        let root = std::path::PathBuf::from(std::env::var("GIANTS_DELPHI_ROOT").unwrap());
        let source =
            crate::scene_resources::SceneResources::load(&root, "w_Rstory1.bin", "English")
                .unwrap();
        let globals =
            giants_formats::global_data::parse(&std::fs::read(root.join("Bin/GData.gbn")).unwrap())
                .unwrap();
        let weapons = giants_formats::weapons::parse_with_profile(
            &std::fs::read(root.join("Bin/weapondefs.bin")).unwrap(),
            giants_formats::weapons::Profile::Debug,
        )
        .unwrap();
        let factory = crate::delphi_special::Factory::load(&source, &globals, &weapons).unwrap();
        let mut session = Session::new(1006);
        let make = |session: &mut Session| {
            factory
                .construct_player_fields(
                    session,
                    crate::delphi_player::ColdFrame {
                        position: [0., 0., 10.],
                        current: [0., 0., 10.],
                        previous: [0., 0., 10.],
                        angles: [0.; 3],
                    },
                    crate::animation_channels::Blending::Disabled,
                )
                .unwrap()
        };
        let initialized = make(&mut session);
        let original = session.list(0);
        assert!(!initialized.special.copies.is_empty());
        let other: Vec<_> = (1..15).map(|list| session.list(list)).collect();
        // Supplied permutation tests the acquired pass on actual constructor
        // relationships; it is not a claim about the normal constructor order.
        for id in &original {
            assert!(session.links.remove_membership(0, id.serial));
        }
        for id in original.iter().rev() {
            session
                .links
                .insert(0, id.serial, crate::object_links::Position::Tail);
        }
        let reverse: Vec<_> = original
            .iter()
            .map(|id| session.links.memberships(id.serial))
            .collect();
        session.order_common_dependencies(2000).unwrap();
        let ordered = session.list(0);
        let index = |id| {
            ordered
                .iter()
                .position(|candidate| *candidate == id)
                .unwrap()
        };
        assert!(index(initialized.owner) < index(initialized.special.held));
        for copy in &initialized.special.copies {
            assert!(index(initialized.owner) < index(copy.actor));
            assert!(index(copy.actor) < index(copy.held));
        }
        assert_eq!(
            (1..15).map(|list| session.list(list)).collect::<Vec<_>>(),
            other
        );
        assert_eq!(
            original
                .iter()
                .map(|id| session.links.memberships(id.serial))
                .collect::<Vec<_>>(),
            reverse
        );
        assert_eq!(
            session.order_common_dependencies(2000).unwrap(),
            ordered.len()
        );
        session.clear();
        assert!(original.iter().all(|id| session.get(*id).is_none()));
        let next = make(&mut session);
        assert_ne!(next.owner, initialized.owner);
        session.order_common_dependencies(2000).unwrap();
        assert_eq!(session.list(0).first(), Some(&next.owner));
    }
}
