//! Source-backed class8/state21 Vimp, initial primary and offline maturation.
//! Living-mode updates remain separate.
use crate::{
    actor_fields::Fields,
    arithmetic::Precision,
    model_registry::StateBinding,
    object_lists::DirectFlags,
    resident_states::{Construction, Id, Session},
    scene_resources::SceneResources,
    specification, state_resources,
    state_storage::{Profile, Seed},
};
use std::sync::Arc;
#[derive(Clone, Copy, Debug)]
pub struct Cold {
    pub context: u32,
    pub born: Option<Born>,
    pub initialized: bool,
    pub mode: u32,
    pub template_scale: f32,
    pub growth_steps: u64,
    pub radius: f32,
    pub damage: f32,
    pub commands: [u8; 8],
    pub wander_pending: bool,
    pub animation_stamp: u32,
    pub angular_speed: f32,
    pub previous_angular_speed: f32,
    /// Constructor-cleared contact word, retained until the physical owner
    /// produces its next value. This is not inferred from world population.
    pub contact_hit: u32,
    /// No constructor value is inferred for the response arrival timer.
    pub response_turn_timer: Option<f32>,
}
#[derive(Clone, Copy, Debug)]
pub struct Born {
    pub parent: Id,
    pub anchor: [f32; 3],
    pub delay: f32,
    pub uid: u32,
    /// Primary herd callback stores its two sampled offsets in 178/17c.
    pub offsets: [f32; 2],
}
pub struct Spawn {
    pub parent: Id,
    pub position: [f32; 3],
    pub uid: u32,
}
pub struct Factory {
    effects: Arc<giants_formats::effect_image::Image>,
    seed: Seed,
    bindings: Vec<StateBinding>,
    flags: DirectFlags,
    specification: Arc<specification::Prepared>,
    states: Arc<state_resources::Prepared>,
    fields: Fields,
    context: u32,
    growth_bounds: Option<crate::player_collision_radius::Bounds>,
}
impl Factory {
    /// Bind the namespace captured by this class's actual specification. Never
    /// select an image by filename or silently substitute a newer active image.
    pub fn load_registered(resources: &SceneResources) -> Result<Self, String> {
        let binding = resources
            .classes
            .models
            .first_selection(8)
            .ok_or("Vimp class selection absent")?
            .as_ref()
            .map_err(Clone::clone)?;
        let definition = resources
            .classes
            .models
            .specs
            .get(&binding.spec_id)
            .and_then(|v| v.first())
            .ok_or("Vimp specification absent")?;
        let effects = resources.classes.models.specification_effects(definition)?;
        Self::load(resources, effects)
    }
    /// Deferred animation effects share the exact class registration image.
    /// Creating a pool does not execute events or admit world callbacks.
    pub fn animation_contexts(&self, ticket: u64) -> crate::attached_effect_context::Pool<Id> {
        crate::attached_effect_context::Pool::new(ticket, Some(Arc::clone(&self.effects)))
    }
    /// The namespace active at registration is explicit; this does not infer normal mission preloading.
    pub fn load(
        resources: &SceneResources,
        effects: &giants_formats::effect_image::Image,
    ) -> Result<Self, String> {
        let program = resources
            .classes
            .models
            .first_program(8)
            .ok_or("Vimp source class absent")?
            .program
            .as_ref()
            .map_err(Clone::clone)?;
        let seed = Seed::from_program_for(program, Profile::Debug, Precision::Double53)?;
        let specification = Arc::new(specification::Prepared::load(resources, 8, effects)?);
        let states = Arc::new(state_resources::Prepared::load(
            resources,
            8,
            Profile::Debug,
        )?);
        let (fields, context) = Fields::load_with_context(resources, 8, &specification)?;
        let growth_bounds = if resources.world_bounds.records.iter().any(|r| r.spec == 8) {
            let bounds = resources
                .collision_resource_bounds(8, Some(1))
                .map_err(|e| e.to_string())?;
            Some(crate::player_collision_radius::Bounds {
                primary: bounds.primary,
                animation_one: bounds.animation,
            })
        } else {
            None
        };
        if seed.storage.state() != 21
            || seed.storage.data().map(<[u8]>::len) != Some(216)
            || !seed.diagnostics.is_empty()
            || !specification.needs.is_empty()
            || !states.needs().is_empty()
        {
            return Err("Vimp cold resources outside acquired profile".into());
        }
        Ok(Self {
            effects: Arc::new(effects.clone()),
            seed,
            bindings: resources.class_state_bindings_for(8, Profile::Debug)?,
            flags: DirectFlags::project(8, program),
            specification,
            states,
            fields,
            context,
            growth_bounds,
        })
    }
    pub(crate) fn owns_effect_contexts(
        &self,
        pool: &crate::attached_effect_context::Pool<Id>,
    ) -> bool {
        pool.uses_image(&self.effects)
    }
    pub fn birth(&self, session: &mut Session) -> Result<Id, String> {
        session.construct_vimp(
            Construction {
                class: 8,
                seed: &self.seed,
                bindings: &self.bindings,
                flags: self.flags,
                specification: self.specification.clone(),
                state_resources: self.states.clone(),
                fields: self.fields.instantiate(),
            },
            Cold {
                context: self.context,
                born: None,
                initialized: false,
                mode: 0,
                template_scale: self.fields.scale,
                growth_steps: 0,
                radius: 0.,
                damage: 0.,
                commands: [0; 8],
                wander_pending: false,
                animation_stamp: 0,
                angular_speed: 0.,
                previous_angular_speed: 0.,
                contact_hit: 0,
                response_turn_timer: None,
            },
        )
    }
    /// The real herd birth helper for configured source class49 parents.
    pub fn spawn(
        &self,
        session: &mut Session,
        input: Spawn,
        globals: &giants_formats::global_data::GlobalData,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<Id, String> {
        let (action, auxiliary) = session.herd_vimp_inputs(input.parent)?;
        if !input.position.into_iter().all(f32::is_finite) {
            return Err("Vimp spawn position is nonfinite".into());
        }
        let health = [0xbb8, 0xbbc, 0xbc0].map(|at| globals.scalar(at).map_err(|e| e.to_string()));
        let health = health.into_iter().collect::<Result<Vec<_>, _>>()?;
        let mut rng = *random;
        let interval = |r: &mut crate::audio_parameters::Random| {
            (f64::from(r.next_15()) * 20. * (1. / 32768.) + 10.) as f32
        };
        let delay = interval(&mut rng);
        let (stage, timer) = if action == 1 {
            let value = (rng.next_15() * 100) >> 15;
            (
                if value < 33 {
                    0
                } else if value < 66 {
                    1
                } else {
                    2
                },
                f32::from_bits(0x497423f0),
            )
        } else {
            (0, interval(&mut rng))
        };
        let scale = [0.2_f32, 0.5, 1.][stage];
        let mut seed = self.seed.instantiate()?;
        let body = seed.storage.data_mut().unwrap();
        for (at, value) in [
            (0x4c, scale.to_bits()),
            (0x50, scale.to_bits()),
            (0x54, timer.to_bits()),
            (0xa4, auxiliary),
        ] {
            body[at..at + 4].copy_from_slice(&value.to_le_bytes());
        }
        let mut fields = self.fields.instantiate();
        fields.previous = fields.current;
        fields.current = input.position;
        fields.position = input.position;
        fields.action = action;
        fields.configuration[12] = health[stage].to_bits();
        let id = session.construct_vimp(
            Construction {
                class: 8,
                seed: &seed,
                bindings: &self.bindings,
                flags: self.flags,
                specification: self.specification.clone(),
                state_resources: self.states.clone(),
                fields,
            },
            Cold {
                context: self.context,
                initialized: false,
                mode: 0,
                template_scale: self.fields.scale,
                growth_steps: 0,
                radius: 0.,
                damage: 0.,
                commands: [0; 8],
                wander_pending: false,
                animation_stamp: 0,
                angular_speed: 0.,
                previous_angular_speed: 0.,
                contact_hit: 0,
                response_turn_timer: None,
                born: Some(Born {
                    parent: input.parent,
                    anchor: input.position,
                    delay,
                    uid: if input.uid == 0 { u32::MAX } else { input.uid },
                    offsets: [0.; 2],
                }),
            },
        )?;
        *random = rng;
        Ok(id)
    }
    pub(crate) fn growth_bounds(
        &self,
        session: &Session,
        id: Id,
    ) -> Result<crate::player_collision_radius::Bounds, String> {
        if !session
            .specification(id)
            .is_some_and(|s| Arc::ptr_eq(s, &self.specification))
        {
            return Err("Vimp growth resources belong to another construction context".into());
        }
        self.growth_bounds
            .ok_or_else(|| "Vimp growth requires source world ABX bounds for specification8".into())
    }
}

pub struct GrowthTick<'a> {
    pub number: u64,
    pub global_delta: f32,
    pub slow_areas: &'a [crate::object_time::SlowArea],
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires GIANTS_NPC_ROOT; actual registered class namespace, missing and replaced-image refusals"]
    fn source_vimp_registered_factory_keeps_captured_image_and_refuses_substitutions() {
        let root = std::path::PathBuf::from(std::env::var_os("GIANTS_NPC_ROOT").unwrap());
        let mut resources = SceneResources::load(&root, "w_Story2.bin", "English").unwrap();
        let factory = Factory::load_registered(&resources).unwrap();
        let contexts = factory.animation_contexts(876);
        assert!(factory.owns_effect_contexts(&contexts));
        let binding = resources
            .classes
            .models
            .first_selection(8)
            .unwrap()
            .as_ref()
            .unwrap();
        let id = binding.spec_id;
        let saved = resources.classes.models.specs.get_mut(&id).unwrap()[0]
            .effect_namespace
            .take();
        assert!(
            Factory::load_registered(&resources)
                .err()
                .unwrap()
                .contains("no effect namespace")
        );
        resources.classes.models.specs.get_mut(&id).unwrap()[0].effect_namespace = saved;
        let last = resources.classes.models.effects.last().unwrap();
        let replacement = crate::model_registry::EffectDefinition {
            path: "controlled-replacement".into(),
            image: last.image.clone(),
        };
        resources.classes.models.effects.push(replacement);
        assert!(
            Factory::load_registered(&resources)
                .err()
                .unwrap()
                .contains("replacement")
        );
        assert!(factory.owns_effect_contexts(&contexts));
    }
}
