//! First positional sounds driven by the prototype's actual contact events.
//! Collision generation is still the development composition, not full state77.
use crate::{
    audio_parameters::Random, positional_effect_audio::Recipe, projectile_effect_requests,
    scene_resources::SceneResources,
};
use giants_formats::weapons::Projectile;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone)]
pub struct Cue {
    pub sound: u32,
    pub position: [f32; 3],
    pub effect: String,
    pub seeds: [u32; 2],
}
pub struct Prepared {
    definition: Projectile,
    recipes: BTreeMap<String, Recipe>,
    random: Random,
}
impl Prepared {
    pub fn load(resources: &SceneResources, definition: &Projectile) -> Result<Self, String> {
        if definition.class != 38 || !matches!(definition.id, 17 | 42) {
            return Err("impact audio outside checked projectile constructor profile".into());
        }
        let image = &resources
            .classes
            .models
            .effects
            .last()
            .ok_or("active effect image absent")?
            .image;
        let mut recipes = BTreeMap::new();
        for name in definition.effects[1..3].iter().flatten() {
            let (_, entry) = image.find(name).ok_or("projectile impact effect missing")?;
            recipes.insert(
                name.to_ascii_lowercase(),
                Recipe::prepare(&image.program(entry.offset).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?,
            );
        }
        Ok(Self {
            definition: definition.clone(),
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
    pub fn request(&mut self, code: u32, position: [f32; 3]) -> Result<Option<Cue>, String> {
        let request = projectile_effect_requests::message(
            &self.definition,
            self.definition.flags,
            code,
            position,
        )?;
        let Some(request) = request else {
            return Ok(None);
        };
        let recipe = self
            .recipes
            .get(&request.name.to_ascii_lowercase())
            .ok_or("impact effect recipe absent")?;
        let before = self.random.0;
        let sound = recipe.first_sound(&mut self.random);
        Ok(Some(Cue {
            sound,
            position,
            effect: request.name.into(),
            seeds: [before, self.random.0],
        }))
    }
}
