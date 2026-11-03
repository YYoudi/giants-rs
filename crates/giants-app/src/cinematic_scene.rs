//! Preloaded source data for director-specific render scenes.
use bevy::prelude::*;
use std::ops::{Deref, DerefMut};

/// Render ownership comes from prepared consumers, never from a class-name list.
pub struct AttachmentOwners {
    entries: Vec<std::collections::BTreeMap<usize, &'static str>>,
}
impl AttachmentOwners {
    pub fn new(directors: usize) -> Self {
        Self {
            entries: vec![Default::default(); directors],
        }
    }
    pub fn add(
        &mut self,
        director: usize,
        slots: impl IntoIterator<Item = usize>,
        consumer: &'static str,
    ) -> Result<(), &'static str> {
        let owners = self
            .entries
            .get_mut(director)
            .ok_or("attachment director absent")?;
        let slots: Vec<_> = slots.into_iter().collect();
        let unique: std::collections::BTreeSet<_> = slots.iter().copied().collect();
        if unique.len() != slots.len()
            || slots
                .iter()
                .any(|slot| *slot >= 32 || owners.contains_key(slot))
        {
            return Err("invalid or multiply owned attachment slot");
        }
        for slot in slots {
            owners.insert(slot, consumer);
        }
        Ok(())
    }
    pub fn owner(&self, director: usize, slot: usize) -> Option<&'static str> {
        self.entries.get(director)?.get(&slot).copied()
    }
}

#[cfg(test)]
mod ownership_tests {
    use super::AttachmentOwners;
    #[test]
    fn absent_slots_fall_back_and_ownership_is_director_local() {
        let mut owners = AttachmentOwners::new(2);
        owners.add(0, [7], "gun").unwrap();
        assert_eq!(owners.owner(0, 7), Some("gun"));
        assert_eq!(owners.owner(0, 8), None);
        assert_eq!(owners.owner(0, 9), None);
        assert_eq!(owners.owner(1, 7), None);
        owners.add(1, [8, 9], "spear").unwrap();
        assert_eq!(owners.owner(1, 8), Some("spear"));
        assert_eq!(owners.owner(0, 8), None);
        assert!(owners.add(0, [6, 7], "staff").is_err());
        assert_eq!(owners.owner(0, 6), None);
        assert!(owners.add(0, [8, 8], "spear").is_err());
        assert_eq!(owners.owner(0, 8), None);
        assert!(owners.add(0, [32], "staff").is_err());
        assert!(owners.add(2, [1], "staff").is_err());
    }
}

#[derive(Resource)]
pub struct Scenes<T: Send + Sync + 'static> {
    entries: Vec<T>,
    active: usize,
}
impl<T: Send + Sync + 'static> Scenes<T> {
    pub fn active_index(&self) -> usize {
        self.active
    }
    pub fn entry(&self, index: usize) -> Option<&T> {
        self.entries.get(index)
    }
    pub fn any(&self, predicate: impl FnMut(&T) -> bool) -> bool {
        self.entries.iter().any(predicate)
    }
    pub fn prepare(
        cinematic: &crate::cinematic_camera::Cinematic,
        mut load: impl FnMut(&crate::cinematic_camera::Segment) -> Result<T, Box<dyn std::error::Error>>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let entries = cinematic
            .segments()
            .iter()
            .map(&mut load)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { entries, active: 0 })
    }
    pub fn select(&mut self, index: usize) -> Result<bool, &'static str> {
        if index >= self.entries.len() {
            return Err("director scene not preloaded");
        }
        let changed = self.active != index;
        self.active = index;
        Ok(changed)
    }
}
impl<T: Send + Sync + 'static> Deref for Scenes<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.entries[self.active]
    }
}
impl<T: Send + Sync + 'static> DerefMut for Scenes<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.entries[self.active]
    }
}
