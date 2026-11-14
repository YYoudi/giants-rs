//! Immutable provenance of a real partial-pistol world contact. No canonical
//! projectile allocation, native launch or full-world scheduling is implied.
use super::*;
use crate::{damage_impulse::Motion, resident_states::Id};

pub(super) struct Emission {
    owner: Id,
    definition: i32,
    class: i32,
    flags: u32,
    range: f32,
    camera: [f32; 3],
    damage: f32,
}
impl Emission {
    pub(super) fn new(owner: Id, definition: &Projectile, camera: [f32; 3]) -> Self {
        Self {
            owner,
            definition: definition.id,
            class: definition.class,
            flags: definition.flags,
            range: definition.range,
            camera,
            damage: definition.direct_damage,
        }
    }
    pub(super) fn damage(&self) -> f32 {
        self.damage
    }
    pub(super) fn finish(self, hit: &WorldHit) -> OwnedHit {
        OwnedHit {
            owner: self.owner,
            serial: hit.serial,
            definition: self.definition,
            class: self.class,
            flags: self.flags,
            range: self.range,
            camera: self.camera,
            object: hit.object,
            motion: hit.motion,
            older: hit.older,
            point: hit.point,
            damage: self.damage,
        }
    }
}

#[derive(Debug)]
pub struct OwnedHit {
    owner: Id,
    serial: u32,
    definition: i32,
    class: i32,
    flags: u32,
    range: f32,
    camera: [f32; 3],
    object: u32,
    motion: Motion,
    older: [f32; 3],
    point: [f32; 3],
    damage: f32,
}
impl OwnedHit {
    pub fn flags(&self) -> u32 {
        self.flags
    }
    pub fn range(&self) -> f32 {
        self.range
    }
    pub fn camera_origin(&self) -> [f32; 3] {
        self.camera
    }
    pub fn owner(&self) -> Id {
        self.owner
    }
    pub fn serial(&self) -> u32 {
        self.serial
    }
    pub fn definition(&self) -> i32 {
        self.definition
    }
    pub fn class(&self) -> i32 {
        self.class
    }
    pub fn object(&self) -> u32 {
        self.object
    }
    pub fn motion(&self) -> Motion {
        self.motion
    }
    pub fn older(&self) -> [f32; 3] {
        self.older
    }
    pub fn point(&self) -> [f32; 3] {
        self.point
    }
    pub fn damage(&self) -> f32 {
        self.damage
    }
    fn matches(&self, hit: &WorldHit) -> bool {
        self.serial == hit.serial
            && self.object == hit.object
            && self.damage.to_bits() == hit.damage.to_bits()
            && self.point.map(f32::to_bits) == hit.point.map(f32::to_bits)
            && self.older.map(f32::to_bits) == hit.older.map(f32::to_bits)
            && self.motion.current.map(f32::to_bits) == hit.motion.current.map(f32::to_bits)
            && self.motion.previous.map(f32::to_bits) == hit.motion.previous.map(f32::to_bits)
    }
}
impl WorldHit {
    pub fn into_source(mut self) -> Result<OwnedHit, &'static str> {
        let source = self
            .source
            .take()
            .ok_or("hit has no actual emitted pistol source")?;
        if !source.matches(&self) {
            return Err("pistol hit differs from its actual emitted contact");
        }
        Ok(source)
    }
}
