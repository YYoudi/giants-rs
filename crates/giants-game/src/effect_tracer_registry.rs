//! Thirty source-order slots, retaining released trails until they can be reused.
use crate::effect_tracer::{Configuration, Segment, Track};
pub const SLOTS: usize = 30;
#[derive(Clone, Copy, Debug)]
pub struct Handle {
    slot: usize,
    generation: u64,
}
impl Handle {
    pub fn slot(self) -> usize {
        self.slot
    }
}
#[derive(Clone, Debug)]
struct Entry {
    track: Track,
    owners: u32,
    generation: u64,
}
#[derive(Clone, Debug)]
pub struct Registry {
    entries: [Option<Entry>; SLOTS],
}
impl Default for Registry {
    fn default() -> Self {
        Self {
            entries: std::array::from_fn(|_| None),
        }
    }
}
impl Registry {
    pub fn admit(&mut self, configuration: Configuration) -> Result<Option<Handle>, &'static str> {
        // Validate before changing a reusable slot, even when the registry is full.
        let fresh = Track::new(configuration.clone())?;
        for (slot, entry) in self.entries.iter_mut().enumerate() {
            if let Some(value) = entry {
                if value.owners != 0 || value.track.count() > 1 {
                    continue;
                }
                let generation = value
                    .generation
                    .checked_add(1)
                    .ok_or("tracer generation overflow")?;
                value.track.reset(configuration)?;
                value.owners = 1;
                value.generation = generation;
            } else {
                *entry = Some(Entry {
                    track: fresh,
                    owners: 1,
                    generation: 1,
                });
            }
            return Ok(Some(Handle {
                slot,
                generation: entry.as_ref().unwrap().generation,
            }));
        }
        Ok(None)
    }
    fn entry_mut(&mut self, handle: Handle) -> Result<&mut Entry, &'static str> {
        self.entries
            .get_mut(handle.slot)
            .and_then(Option::as_mut)
            .filter(|e| e.generation == handle.generation)
            .ok_or("stale tracer handle")
    }
    pub fn retain(&mut self, handle: Handle) -> Result<(), &'static str> {
        let entry = self.entry_mut(handle)?;
        if entry.owners == 0 {
            return Err("released tracer owner");
        }
        entry.owners = entry
            .owners
            .checked_add(1)
            .ok_or("tracer owner count overflow")?;
        Ok(())
    }
    pub fn release(&mut self, handle: Handle) -> Result<(), &'static str> {
        let entry = self.entry_mut(handle)?;
        entry.owners = entry.owners.checked_sub(1).ok_or("released tracer owner")?;
        Ok(())
    }
    pub fn sample(&mut self, handle: Handle, position: [f32; 3]) -> Result<(), &'static str> {
        let entry = self.entry_mut(handle)?;
        if entry.owners == 0 {
            return Err("released tracer owner");
        }
        entry.track.sample(position)
    }
    pub fn set_opacity(&mut self, handle: Handle, opacity: f32) -> Result<(), &'static str> {
        let entry = self.entry_mut(handle)?;
        if entry.owners == 0 {
            return Err("released tracer owner");
        }
        entry.track.set_opacity(opacity)
    }
    pub fn advance(&mut self, delta: f32) -> Result<Vec<(usize, Segment)>, &'static str> {
        if !delta.is_finite() || delta < 0. {
            return Err("invalid global tracer delta");
        }
        let mut segments = Vec::new();
        for (slot, entry) in self.entries.iter_mut().enumerate() {
            if let Some(e) = entry {
                segments.extend(e.track.advance(delta)?.into_iter().map(|s| (slot, s)));
            }
        }
        Ok(segments)
    }
    pub fn slots(&self) -> impl Iterator<Item = (usize, u32, &Track)> {
        self.entries
            .iter()
            .enumerate()
            .filter_map(|(i, e)| e.as_ref().map(|e| (i, e.owners, &e.track)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn config() -> Configuration {
        Configuration {
            lifetime: 1.,
            fade_start: 0.25,
            colors: [[1., 0., 0.], [0., 1., 0.]],
            min_interval: 0.01,
            half_width: 0.5,
        }
    }
    #[test]
    fn released_and_recycled_handles_cannot_mutate_a_new_owner() {
        let mut r = Registry::default();
        let old = r.admit(config()).unwrap().unwrap();
        r.release(old).unwrap();
        assert!(r.sample(old, [1., 2., 3.]).is_err());
        assert!(r.release(old).is_err());
        let new = r.admit(config()).unwrap().unwrap();
        assert_eq!(old.slot(), new.slot());
        assert!(r.sample(old, [1., 2., 3.]).is_err());
        assert!(r.release(old).is_err());
        r.sample(new, [4., 5., 6.]).unwrap();
        let (_, owners, track) = r.slots().next().unwrap();
        assert_eq!(owners, 1);
        assert_eq!(track.count(), 2);
    }
}
