//! Ordered anonymous object lists and reverse memberships. Registration is
//! independent of current visibility; later flag changes do not reclassify it.
use std::collections::BTreeMap;

pub type ObjectId = u64;
pub type ListId = u32;
pub type LinkId = u64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Position {
    Head,
    Tail,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Registration {
    pub list: ListId,
    pub object: ObjectId,
}
#[derive(Clone, Copy, Debug)]
struct Node {
    registration: Registration,
    next: Option<LinkId>,
}
#[derive(Clone, Default, Debug)]
pub struct Links {
    next: LinkId,
    lists: BTreeMap<ListId, Vec<LinkId>>,
    objects: BTreeMap<ObjectId, Vec<LinkId>>,
    nodes: BTreeMap<LinkId, Node>,
}
impl Links {
    /// Every registration is distinct, including duplicate object/list pairs.
    /// Forward insertion can be at either end; reverse membership is always LIFO.
    pub fn insert(&mut self, list: ListId, object: ObjectId, at: Position) -> LinkId {
        self.next = self
            .next
            .checked_add(1)
            .expect("object link identity exhausted");
        let id = self.next;
        let entries = self.lists.entry(list).or_default();
        let next = match at {
            Position::Head => entries.first().copied(),
            Position::Tail => None,
        };
        if at == Position::Tail
            && let Some(previous) = entries.last()
        {
            self.nodes.get_mut(previous).unwrap().next = Some(id);
        }
        match at {
            Position::Head => entries.insert(0, id),
            Position::Tail => entries.push(id),
        }
        self.objects.entry(object).or_default().push(id);
        self.nodes.insert(
            id,
            Node {
                registration: Registration { list, object },
                next,
            },
        );
        id
    }
    pub fn list(&self, list: ListId) -> Vec<ObjectId> {
        self.lists
            .get(&list)
            .into_iter()
            .flatten()
            .map(|id| self.nodes[id].registration.object)
            .collect()
    }
    pub fn memberships(&self, object: ObjectId) -> Vec<(LinkId, ListId)> {
        self.objects
            .get(&object)
            .into_iter()
            .flatten()
            .rev()
            .map(|id| (*id, self.nodes[id].registration.list))
            .collect()
    }
    /// Opaque local registration identity, captured once at traversal entry.
    pub fn first_node(&self, list: ListId) -> Option<LinkId> {
        self.lists
            .get(&list)
            .and_then(|entries| entries.first())
            .copied()
    }
    pub fn node_object(&self, list: ListId, node: LinkId) -> Result<ObjectId, &'static str> {
        let entry = self.nodes.get(&node).ok_or("list node is retired")?;
        if entry.registration.list != list {
            return Err("list node belongs to another list");
        }
        Ok(entry.registration.object)
    }
    /// Read after the callback. Head insertion does not redirect this edge;
    /// tail insertion and removal of future registrations do update it.
    pub fn next_node(&self, list: ListId, node: LinkId) -> Result<Option<LinkId>, &'static str> {
        self.node_object(list, node)?;
        Ok(self.nodes[&node].next)
    }
    /// Acquired dependency pass: capture the successor before moving a node.
    /// The newest reverse membership selects the parent's registration. A
    /// dependent already anywhere after that parent stays in place, including
    /// when a later registration of the same object satisfies the search.
    /// The explicit host budget is not a native limit; errors retain moves.
    pub fn order_dependencies(
        &mut self,
        list: ListId,
        budget: usize,
        mut parent: impl FnMut(ObjectId) -> Result<Option<ObjectId>, String>,
    ) -> Result<usize, String> {
        if budget == 0 {
            return Err("dependency order requires a positive visit budget".into());
        }
        let mut cursor = self.first_node(list);
        let mut visits = 0;
        while let Some(node) = cursor {
            if visits == budget {
                return Err("dependency order visit budget exhausted".into());
            }
            visits += 1;
            let object = self.node_object(list, node).map_err(str::to_owned)?;
            cursor = self.next_node(list, node).map_err(str::to_owned)?;
            let Some(parent) = parent(object)? else {
                continue;
            };
            if parent == object {
                return Err("self-dependent object outside acquired ordering profile".into());
            }
            let Some((anchor, _)) = self
                .memberships(parent)
                .into_iter()
                .find(|(_, membership)| *membership == list)
            else {
                continue;
            };
            let mut following = self.next_node(list, anchor).map_err(str::to_owned)?;
            let mut found = false;
            while let Some(candidate) = following {
                if self.node_object(list, candidate).map_err(str::to_owned)? == object {
                    found = true;
                    break;
                }
                following = self.next_node(list, candidate).map_err(str::to_owned)?;
            }
            if !found {
                // Preserve the node, reverse registrations and every other list.
                let entries = self.lists.get_mut(&list).expect("owned dependency list");
                let index = entries.iter().position(|id| *id == node).unwrap();
                if index > 0 {
                    self.nodes.get_mut(&entries[index - 1]).unwrap().next = cursor;
                }
                entries.remove(index);
                let index = entries.iter().position(|id| *id == anchor).unwrap();
                let following = self.nodes[&anchor].next;
                self.nodes.get_mut(&anchor).unwrap().next = Some(node);
                self.nodes.get_mut(&node).unwrap().next = following;
                entries.insert(index + 1, node);
            }
        }
        Ok(visits)
    }
    fn unlink_node(&mut self, id: LinkId) {
        let entry = self.nodes.remove(&id).expect("owned object link");
        let entries = self
            .lists
            .get_mut(&entry.registration.list)
            .expect("owned list");
        let index = entries
            .iter()
            .position(|node| *node == id)
            .expect("owned list entry");
        if index > 0 {
            self.nodes.get_mut(&entries[index - 1]).unwrap().next = entry.next;
        }
        entries.remove(index);
    }
    pub fn remove_object(&mut self, object: ObjectId) -> usize {
        let Some(entries) = self.objects.remove(&object) else {
            return 0;
        };
        for id in &entries {
            self.unlink_node(*id);
        }
        entries.len()
    }
    /// Remove the newest reverse membership matching this list, as the native
    /// list/object helper does. Other registrations keep their identities/order.
    pub fn remove_membership(&mut self, list: ListId, object: ObjectId) -> bool {
        let Some(entries) = self.objects.get_mut(&object) else {
            return false;
        };
        let Some(index) = entries
            .iter()
            .rposition(|id| self.nodes[id].registration.list == list)
        else {
            return false;
        };
        let id = entries.remove(index);
        if entries.is_empty() {
            self.objects.remove(&object);
        }
        self.unlink_node(id);
        true
    }
    /// Common list0 is unconditional. Native constructor registration appends
    /// in bit order; only the14 low bits create additional memberships.
    pub fn construct(
        &mut self,
        object: ObjectId,
        class: i32,
        state: u32,
        flags: crate::object_lists::DirectFlags,
        requested: u32,
    ) -> Result<u32, &'static str> {
        if self.objects.contains_key(&object) {
            return Err("constructor object already registered");
        }
        let mask = if requested == 0x80000000 {
            crate::object_lists::membership(class, state, flags)
        } else {
            requested
        } & 0x3fff;
        self.insert(0, object, Position::Tail);
        for bit in 0..14 {
            if mask & (1 << bit) != 0 {
                self.insert(bit + 1, object, Position::Tail);
            }
        }
        Ok(mask)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dependency_moves_keep_registration_identity_and_captured_successor() {
        let mut links = Links::default();
        let child = links.insert(0, 3, Position::Tail);
        let middle = links.insert(0, 2, Position::Tail);
        let parent = links.insert(0, 1, Position::Tail);
        let other = links.insert(4, 3, Position::Tail);
        let reverse = links.memberships(3);
        let mut visited = Vec::new();
        links
            .order_dependencies(0, 12, |object| {
                visited.push(object);
                Ok(match object {
                    3 => Some(2),
                    2 => Some(1),
                    _ => None,
                })
            })
            .unwrap();
        assert_eq!(visited, [3, 2, 3, 1, 2, 3]);
        assert_eq!(links.list(0), [1, 2, 3]);
        assert_eq!(links.first_node(0), Some(parent));
        assert_eq!(links.next_node(0, parent), Ok(Some(middle)));
        assert_eq!(links.next_node(0, middle), Ok(Some(child)));
        assert_eq!(links.next_node(0, child), Ok(None));
        assert_eq!(links.memberships(3), reverse);
        assert_eq!(links.first_node(4), Some(other));
        links.remove_object(2);
        assert_eq!(links.next_node(0, parent), Ok(Some(child)));
    }

    #[test]
    fn dependency_duplicates_use_newest_parent_and_object_suffix() {
        let mut links = Links::default();
        for object in [2, 1, 2, 1] {
            links.insert(0, object, Position::Tail);
        }
        links
            .order_dependencies(0, 20, |object| Ok((object == 2).then_some(1)))
            .unwrap();
        assert_eq!(links.list(0), [1, 2, 1, 2]);
        // A duplicate dependent after the selected parent satisfies the native
        // object-identity search even if this registration precedes the parent.
        let mut links = Links::default();
        for object in [2, 1, 2] {
            links.insert(0, object, Position::Tail);
        }
        links
            .order_dependencies(0, 20, |object| Ok((object == 2).then_some(1)))
            .unwrap();
        assert_eq!(links.list(0), [2, 1, 2]);
    }

    #[test]
    fn dependency_failure_keeps_moves_and_budget_bounds_cycles() {
        let mut links = Links::default();
        for object in [3, 2, 1] {
            links.insert(0, object, Position::Tail);
        }
        let error = links
            .order_dependencies(0, 10, |object| match object {
                3 => Ok(Some(1)),
                _ => Err("provider failure".into()),
            })
            .unwrap_err();
        assert_eq!(error, "provider failure");
        assert_eq!(links.list(0), [2, 1, 3]);
        let mut links = Links::default();
        for object in [1, 2] {
            links.insert(0, object, Position::Tail);
        }
        assert!(
            links
                .order_dependencies(0, 8, |object| Ok(Some(3 - object)))
                .unwrap_err()
                .contains("budget exhausted")
        );
        assert_eq!(links.list(0).len(), 2);
    }
    #[test]
    fn newest_duplicate_removal_updates_successor_without_reusing_retired_identity() {
        let mut links = Links::default();
        let first = links.insert(0, 10, Position::Tail);
        let second = links.insert(0, 20, Position::Tail);
        let duplicate = links.insert(0, 10, Position::Tail);
        links.insert(1, 10, Position::Head);
        assert!(links.remove_membership(0, 10));
        assert_eq!(links.next_node(0, first), Ok(Some(second)));
        assert_eq!(links.next_node(0, second), Ok(None));
        assert!(links.next_node(0, duplicate).is_err());
        let replacement = links.insert(0, 30, Position::Tail);
        assert_ne!(replacement, duplicate);
        assert_eq!(links.next_node(0, second), Ok(Some(replacement)));
        assert_eq!(links.list(1), [10]);
        links.remove_object(10);
        assert_eq!(links.first_node(0), Some(second));
        assert!(links.next_node(0, first).is_err());
        assert_eq!(links.list(0), [20, 30]);
        assert!(links.list(1).is_empty());
    }
}
