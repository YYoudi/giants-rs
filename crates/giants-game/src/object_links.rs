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
