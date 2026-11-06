//! Owned normal-cold procedural geometry registrations. Ordered membership and
//! last-entry CPU material retirement are acquired; GPU/world dispatch is not.
use crate::{audio_parameters::Random, kabuto_texture, resident_states::Id};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Handle {
    scope: Arc<()>,
    serial: u64,
}

impl PartialEq for Handle {
    fn eq(&self, other: &Self) -> bool {
        self.serial == other.serial && Arc::ptr_eq(&self.scope, &other.scope)
    }
}
impl Eq for Handle {}
pub struct Registry {
    ticket: u64,
    scope: Arc<()>,
    next: u64,
    entries: Vec<(Handle, Id)>,
    texture: Option<Arc<kabuto_texture::Texture>>,
}
#[derive(Debug)]
pub struct Retirement {
    pub owner: Id,
    /// The active registry drops its material. This is not a GPU receipt;
    /// explicit external snapshots can still retain CPU image data.
    pub material_retired: bool,
}
#[derive(Debug)]
pub struct Cleanup {
    pub marked: Vec<Id>,
    pub material_retired: bool,
}
impl Registry {
    pub fn new(ticket: u64) -> Self {
        Self {
            ticket,
            scope: Arc::new(()),
            next: 1,
            entries: Vec::new(),
            texture: None,
        }
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn order(&self) -> impl Iterator<Item = Id> + '_ {
        self.entries.iter().map(|(_, owner)| *owner)
    }
    pub fn texture(&self) -> Option<&Arc<kabuto_texture::Texture>> {
        self.texture.as_ref()
    }
    pub fn owner(&self, handle: &Handle) -> Result<Id, String> {
        if !Arc::ptr_eq(&self.scope, &handle.scope) {
            return Err("Kabuto geometry belongs to another registry".into());
        }
        self.entries
            .iter()
            .find(|(h, _)| h == handle)
            .map(|(_, id)| *id)
            .ok_or_else(|| "Kabuto geometry registration is retired or absent".into())
    }
    pub fn register(&mut self, owner: Id, random: &mut Random) -> Result<Handle, String> {
        if owner.ticket() != self.ticket {
            return Err("Kabuto geometry owner has a foreign ticket".into());
        }
        let next = self
            .next
            .checked_add(1)
            .ok_or("Kabuto geometry identity exhausted")?;
        let handle = Handle {
            scope: self.scope.clone(),
            serial: self.next,
        };
        if self.texture.is_none() {
            self.texture = Some(Arc::new(kabuto_texture::prepare(random)));
        }
        self.entries.push((handle.clone(), owner));
        self.next = next;
        Ok(handle)
    }
    pub fn retire(&mut self, handle: &Handle) -> Result<Retirement, String> {
        let owner = self.owner(handle)?;
        let at = self.entries.iter().position(|(h, _)| h == handle).unwrap();
        self.entries.remove(at);
        let material_retired = self.entries.is_empty();
        if material_retired {
            self.texture = None;
        }
        Ok(Retirement {
            owner,
            material_retired,
        })
    }
    // Only an exclusive caller may stage and replace this state. A public Clone
    // could create competing registries with colliding future handle identities.
    pub(crate) fn staged(&self) -> Self {
        Self {
            ticket: self.ticket,
            scope: self.scope.clone(),
            next: self.next,
            entries: self.entries.clone(),
            texture: self.texture.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        object_lists::DirectFlags,
        resident_states::Session,
        state_storage::{Profile, Seed, Storage},
    };
    fn owner(session: &mut Session) -> Id {
        let seed = Seed {
            storage: Storage::new(Profile::Debug),
            transitions: vec![],
            unexecuted: vec![],
            diagnostics: vec![],
            unresolved: vec![],
        };
        session
            .register(
                0,
                &seed,
                &[],
                DirectFlags {
                    active: 0,
                    personality: 0,
                },
            )
            .unwrap()
    }
    #[test]
    fn ordered_retirement_releases_only_last_material_and_never_reuses_handles() {
        let mut session = Session::new(42);
        let owners = [
            owner(&mut session),
            owner(&mut session),
            owner(&mut session),
        ];
        let mut registry = Registry::new(42);
        let mut random = Random(1);
        let a = registry.register(owners[0], &mut random).unwrap();
        let state = random.0;
        let weak = Arc::downgrade(registry.texture().unwrap());
        let b = registry.register(owners[1], &mut random).unwrap();
        let c = registry.register(owners[2], &mut random).unwrap();
        assert_eq!(random.0, state);
        assert!(!registry.retire(&b).unwrap().material_retired);
        assert_eq!(registry.order().collect::<Vec<_>>(), [owners[0], owners[2]]);
        assert!(registry.retire(&b).is_err());
        assert!(!registry.retire(&a).unwrap().material_retired);
        assert!(weak.upgrade().is_some());
        assert!(registry.retire(&c).unwrap().material_retired);
        assert!(weak.upgrade().is_none());
        let next = registry.register(owners[0], &mut random).unwrap();
        assert_ne!(next, a);
        assert_ne!(random.0, state);
        assert!(registry.retire(&a).is_err());
    }
    #[test]
    fn foreign_scopes_tickets_and_identity_exhaustion_preserve_state() {
        let mut session = Session::new(42);
        let id = owner(&mut session);
        let mut random = Random(7);
        let mut registry = Registry::new(42);
        let handle = registry.register(id, &mut random).unwrap();
        let mut other = Registry::new(42);
        assert!(other.retire(&handle).is_err());
        assert!(other.is_empty());
        let state = random.0;
        let mut wrong_ticket = Registry::new(43);
        assert!(wrong_ticket.register(id, &mut random).is_err());
        assert_eq!(random.0, state);
        registry.next = u64::MAX;
        assert!(registry.register(id, &mut random).is_err());
        assert_eq!(random.0, state);
        assert_eq!(registry.len(), 1);
    }
}
