//! Local-player audio projection of source animation/attached-effect events.
//! This does not claim visual effect creation or the original live RNG phase.
use crate::{
    animation_event_dispatch::{Anchor, Context, Request, State, Window},
    animation_events,
    arithmetic::Precision,
    audio_parameters::Random,
    scene_resources::SceneResources,
};
use giants_formats::{Error, effect_program::Program};
use std::collections::{BTreeMap, BTreeSet};

pub struct Recipe {
    pub class: u32,
    pub range: [u32; 2],
}
impl Recipe {
    /// Bounded straight-line owner effect profile; no symbol-name allowlist.
    /// Non-audio vector/flag commands are retained by the visual subsystem.
    pub fn prepare(program: &Program) -> Result<Self, Error> {
        let ops: Vec<_> = program.records.iter().map(|r| r.opcode).collect();
        if ops != [0x24, 6, 4, 0x21, 0x87, 7, 2] && ops != [0x24, 6, 0x87, 4, 0x21, 7, 2] {
            return Err(Error("effect outside checked owner-audio projection"));
        }
        let word = |record: usize, at: usize| -> Result<u32, Error> {
            let bytes = program.records[record]
                .bytes
                .get(at..at + 4)
                .ok_or(Error("truncated owner-audio operand"))?;
            Ok(u32::from_le_bytes(bytes.try_into().unwrap()))
        };
        if word(0, 9)? != u32::MAX {
            return Err(Error("owner-audio reference outside root profile"));
        }
        let vector = ops.iter().position(|op| *op == 4).unwrap();
        if !(0..3).all(|i| word(vector, 1 + i * 4).is_ok_and(|w| f32::from_bits(w).is_finite())) {
            return Err(Error("nonfinite delegated effect vector"));
        }
        let sound = ops.iter().position(|op| *op == 0x87).unwrap();
        let range = [word(sound, 1)?, word(sound, 5)?];
        if range[0] > range[1] || range[1] >= 2000 || range[1] - range[0] > 32 {
            return Err(Error("owner-audio range outside checked profile"));
        }
        Ok(Self {
            class: word(0, 5)?,
            range,
        })
    }
    pub fn sound(&self, random: &mut Random) -> u32 {
        // Original preparation traverses opcode87 in skip mode too.
        random.next_15();
        self.range[0] + ((random.next_15() * (self.range[1] - self.range[0] + 1)) >> 15)
    }
}

#[derive(Clone)]
pub struct Cue {
    pub slot: u8,
    pub source_offset: usize,
    pub effect: u32,
    pub sound: u32,
    pub marker_bits: u32,
    pub window: Window,
    pub duration: f32,
    pub wet: bool,
    pub material: Option<String>,
    pub seeds: [u32; 2],
}
pub struct Prepared {
    tracks: BTreeMap<u8, Vec<State>>,
    recipes: BTreeMap<u32, Recipe>,
    random: Random,
}
impl Prepared {
    pub fn load(resources: &SceneResources) -> Result<Self, String> {
        let binding = resources
            .classes
            .models
            .first_selection(2)
            .ok_or("Mecc selection absent")?
            .as_ref()
            .map_err(Clone::clone)?;
        let definition = resources
            .classes
            .models
            .specs
            .get(&binding.spec_id)
            .and_then(|v| v.first())
            .ok_or("Mecc specification absent")?;
        let program = definition.program.as_ref().map_err(Clone::clone)?;
        let effects = resources
            .classes
            .models
            .effects
            .last()
            .ok_or("active effect image absent")?;
        let source = animation_events::prepare(program, &effects.image, Precision::Double53)
            .map_err(|e| e.to_string())?;
        let mut tracks = BTreeMap::new();
        let mut recipes = BTreeMap::new();
        for track in source {
            let mut states = Vec::new();
            for event in track.events {
                if event.kind != 0x80 || event.parameters[5] != u32::MAX {
                    return Err("Mecc audio event outside checked attached-root profile".into());
                }
                for target in event.parameters[..5].iter().copied() {
                    if let std::collections::btree_map::Entry::Vacant(entry) = recipes.entry(target)
                    {
                        let recipe = Recipe::prepare(
                            &effects.image.program(target).map_err(|e| e.to_string())?,
                        )
                        .map_err(|e| e.to_string())?;
                        resources
                            .classes
                            .models
                            .first_program(recipe.class as i32)
                            .ok_or("owner effect class absent from loaded catalogs")?
                            .program
                            .as_ref()
                            .map_err(Clone::clone)?;
                        entry.insert(recipe);
                    }
                }
                states.push(State::from(&event));
            }
            tracks.insert(track.slot, states);
        }
        Ok(Self {
            tracks,
            recipes,
            random: Random(1),
        })
    }
    pub fn sound_ids(&self) -> BTreeSet<u32> {
        self.recipes
            .values()
            .flat_map(|r| r.range[0]..=r.range[1])
            .collect()
    }
    /// Called for the controlled local player only. Material absence is valid
    /// only when the contact service has no supporting mesh object.
    pub fn step(
        &mut self,
        slot: u8,
        window: Window,
        duration: f32,
        wet: bool,
        material: Option<&str>,
    ) -> Result<Vec<Cue>, Error> {
        let Some(events) = self.tracks.get_mut(&slot) else {
            return Err(Error("active Mecc event track not prepared"));
        };
        let context = Context {
            wet,
            material,
            position: [0.; 3],
            world: None,
            references: &[],
            contact_scalar: 0.,
            cinematic: false,
        };
        let mut result = Vec::new();
        for event in events {
            let dispatch = event.dispatch(window, &context)?;
            for request in dispatch.requests {
                let Request::Effect {
                    flags: 0x14,
                    target: Some(target),
                    anchor: Anchor::OwnerReference(-1),
                } = request
                else {
                    return Err(Error("Mecc event produced an unsupported audio route"));
                };
                let recipe = self
                    .recipes
                    .get(&target)
                    .ok_or(Error("effect audio recipe absent"))?;
                let before = self.random.0;
                let sound = recipe.sound(&mut self.random);
                result.push(Cue {
                    slot,
                    source_offset: event.source_offset,
                    effect: target,
                    sound,
                    marker_bits: event.time.to_bits(),
                    window,
                    duration,
                    wet,
                    material: material.map(str::to_owned),
                    seeds: [before, self.random.0],
                });
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_style_recipe_keeps_preparation_draw_and_inclusive_range() {
        let mut bytes = vec![0x24];
        for value in [77u32, 123, u32::MAX] {
            bytes.extend(value.to_le_bytes());
        }
        bytes.extend([6, 4]);
        for value in [2f32, 1., 3.] {
            bytes.extend(value.to_le_bytes());
        }
        bytes.extend([0x21, 0x87]);
        for value in [100u32, 101] {
            bytes.extend(value.to_le_bytes());
        }
        bytes.extend([7, 2]);
        let program = giants_formats::effect_program::parse(&bytes).unwrap();
        let recipe = Recipe::prepare(&program).unwrap();
        let mut random = Random(0);
        assert_eq!(recipe.sound(&mut random), 100);
        assert_eq!(random.0, 505908858);
        let mut altered = program;
        altered.records.insert(3, altered.records[3].clone());
        assert!(Recipe::prepare(&altered).is_err());
    }
}
