//! Shared immutable specification preparation. No actor, mutable event scope,
//! effect activation, texture upload or runtime readiness is manufactured here.
use crate::{
    animation_events, animation_links,
    animation_references::References,
    animation_template::{Model, Template},
    arithmetic::Precision,
    model_registry::SelectionSpec,
    scene_resources::{CpuModel, SceneResources},
};
use giants_formats::{anm::Animation, effect_image::Image, object_spec::Program};
use std::collections::BTreeMap;
type RegisteredSelection<'a> = (
    &'a crate::model_registry::SelectionBinding,
    &'a crate::model_registry::SpecDefinition,
);

pub struct Metadata {
    pub flags: u32,
    pub selection: SelectionSpec,
    pub references: References,
    pub events: Vec<animation_events::Track>,
    pub correction_count: usize,
}

impl Metadata {
    /// The effect namespace and arithmetic profile remain explicit inputs.
    pub fn prepare(
        program: &Program,
        selection: &SelectionSpec,
        effects: &Image,
    ) -> Result<Self, String> {
        if !selection.diagnostics.is_empty() {
            return Err("specification reports model declaration diagnostics".into());
        }
        selection.table().map_err(str::to_owned)?;
        Ok(Self {
            // Opaque source bit: its drawing semantics are not inferred here.
            flags: if program.records.iter().any(|r| r.opcode == 0x55) {
                16
            } else {
                0
            },
            selection: selection.clone(),
            references: References::from_program(program).map_err(|e| e.to_string())?,
            events: animation_events::prepare(program, effects, Precision::Double53)
                .map_err(|e| e.to_string())?,
            correction_count: program
                .records
                .iter()
                .filter(|r| r.opcode == 0x54 && r.bytes.get(1) != Some(&0))
                .count(),
        })
    }
}
pub struct Descriptor {
    pub name: String,
    pub animation: Animation,
    pub template: Template,
}
pub struct Prepared {
    pub class: i32,
    pub class_catalog: std::path::PathBuf,
    pub specification: u32,
    pub specification_catalog: std::path::PathBuf,
    pub metadata: Metadata,
    pub models: Vec<Option<CpuModel>>,
    pub animations: BTreeMap<u8, Descriptor>,
    pub corrections: Vec<animation_links::Link>,
    /// Explicit preparation gaps; an empty list still does not mean actor ready.
    pub needs: Vec<String>,
}
impl Prepared {
    pub fn load(resources: &SceneResources, class: i32, effects: &Image) -> Result<Self, String> {
        let (binding, definition) = Self::registered_selection(resources, class)?;
        Self::prepare_selection(resources, class, binding, definition, effects)
    }
    /// Load a body using the image captured at its first specification
    /// registration. Never repair missing history using a later active image.
    /// This prepares immutable resources, not a live actor or effect instance.
    pub fn load_registered(resources: &SceneResources, class: i32) -> Result<Self, String> {
        let (binding, definition) = Self::registered_selection(resources, class)?;
        let program = definition.program.as_ref().map_err(Clone::clone)?;
        if program
            .records
            .iter()
            .any(|r| matches!(r.opcode, 0x7f | 0x80))
        {
            let effects = resources.classes.models.specification_effects(definition)?;
            Self::prepare_selection(resources, class, binding, definition, effects)
        } else {
            // The namespace cannot affect a program with no effect references.
            let empty = giants_formats::effect_image::parse(&[1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])
                .map_err(|e| e.to_string())?;
            Self::prepare_selection(resources, class, binding, definition, &empty)
        }
    }
    fn registered_selection(
        resources: &SceneResources,
        class: i32,
    ) -> Result<RegisteredSelection<'_>, String> {
        let binding = resources
            .classes
            .models
            .first_selection(class)
            .ok_or("class selection absent")?
            .as_ref()
            .map_err(Clone::clone)?;
        let definition = resources
            .classes
            .models
            .specs
            .get(&binding.spec_id)
            .and_then(|v| v.first())
            .ok_or("specification absent")?;
        if definition.catalog != binding.spec_catalog {
            return Err("specification context identity changed".into());
        }
        Ok((binding, definition))
    }
    /// Prepare an additional specification captured in a class-state field.
    /// This owns CPU resources, never writes a native pointer or completes an
    /// actor constructor. Effect-bearing declarations are refused until their
    /// original effect namespace can be supplied, rather than using the latest.
    pub fn load_state_without_effects(
        resources: &SceneResources,
        class: i32,
        profile: crate::state_storage::Profile,
        offset: usize,
    ) -> Result<Self, String> {
        Self::load_state(resources, class, profile, offset, false)
    }
    /// Prepare a captured state specification with its own registered namespace.
    /// This is resource preparation, not pointer materialization or FX activation.
    pub fn load_state_registered(
        resources: &SceneResources,
        class: i32,
        profile: crate::state_storage::Profile,
        offset: usize,
    ) -> Result<Self, String> {
        Self::load_state(resources, class, profile, offset, true)
    }
    fn load_state(
        resources: &SceneResources,
        class: i32,
        profile: crate::state_storage::Profile,
        offset: usize,
        registered_effects: bool,
    ) -> Result<Self, String> {
        let references = resources.class_state_bindings_for(class, profile)?;
        let reference = references
            .iter()
            .find(|b| b.offset == offset)
            .ok_or("state specification field absent")?;
        let (id, catalog) = match (
            &reference.key,
            reference.target.as_ref().map_err(Clone::clone)?,
        ) {
            (
                crate::state_storage::ReferenceKey::Spec(key),
                crate::model_registry::StateTarget::Spec { id, catalog },
            ) if key == id => (*id, catalog),
            _ => return Err("state field is not a matching specification target".into()),
        };
        let definition = resources
            .classes
            .models
            .specs
            .get(&id)
            .and_then(|v| v.first())
            .ok_or("state specification absent")?;
        if &definition.catalog != catalog {
            return Err("state specification context identity changed".into());
        }
        let program = definition.program.as_ref().map_err(Clone::clone)?;
        let requires_effects = program
            .records
            .iter()
            .any(|r| matches!(r.opcode, 0x7f | 0x80));
        if requires_effects && !registered_effects {
            return Err("state specification requires its captured effect namespace".into());
        }
        let binding = crate::model_registry::SelectionBinding {
            class_catalog: resources
                .classes
                .models
                .first_program(class)
                .ok_or("class program absent")?
                .catalog
                .clone(),
            spec_id: id,
            spec_catalog: catalog.clone(),
            selection: definition.selection.as_ref().map_err(Clone::clone)?.clone(),
        };
        let empty = giants_formats::effect_image::parse(&[1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])
            .map_err(|e| e.to_string())?;
        let effects = if requires_effects {
            resources.classes.models.specification_effects(definition)?
        } else {
            &empty
        };
        Self::prepare_selection(resources, class, &binding, definition, effects)
    }
    fn prepare_selection(
        resources: &SceneResources,
        class: i32,
        binding: &crate::model_registry::SelectionBinding,
        definition: &crate::model_registry::SpecDefinition,
        effects: &Image,
    ) -> Result<Self, String> {
        let program = definition.program.as_ref().map_err(Clone::clone)?;
        let metadata = Metadata::prepare(program, &binding.selection, effects)?;
        let models = (0..metadata.selection.slots.len())
            .map(|slot| {
                resources
                    .selection_cpu_slot(&metadata.selection, slot, true)
                    .map_err(|e| e.to_string())
            })
            .collect::<Result<Vec<_>, _>>()?;
        let inputs = models
            .iter()
            .map(|m| match m {
                None => Model::Empty,
                Some(m) => match &m.prepared.skin {
                    Some(s) => Model::Skinned(s),
                    None => Model::Unskinned,
                },
            })
            .collect::<Vec<_>>();
        let mut animations = BTreeMap::new();
        let mut needs = Vec::new();
        for model in models.iter().flatten() {
            for need in &model.prepared.needs {
                needs.push(format!("model {}: {need:?}", model.name));
            }
        }
        for declared in &metadata.selection.animations {
            if animations.contains_key(&declared.slot) {
                continue;
            }
            let animation = resources
                .animation(&declared.name)
                .map_err(|e| e.to_string())?;
            let template = Template::prepare(&animation, &inputs, Precision::Double53)
                .map_err(|e| e.to_string())?;
            for need in &template.needs {
                needs.push(format!("animation {}: {need:?}", declared.name));
            }
            animations.insert(
                declared.slot,
                Descriptor {
                    name: declared.name.clone(),
                    animation,
                    template,
                },
            );
        }
        let sources = animations
            .iter()
            .map(|(slot, d)| (*slot, &d.animation))
            .collect();
        let corrections = animation_links::prepare(program, &sources).map_err(|e| e.to_string())?;
        for correction in &corrections {
            if !matches!(correction.outcome, animation_links::Outcome::Prepared(_)) {
                needs.push(format!(
                    "correction {}: {:?}",
                    correction.source_offset, correction.outcome
                ));
            }
        }
        Ok(Self {
            class,
            class_catalog: binding.class_catalog.clone(),
            specification: binding.spec_id,
            specification_catalog: binding.spec_catalog.clone(),
            metadata,
            models,
            animations,
            corrections,
            needs,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        model_registry::{ClassProgram, ModelRegistry, SpecDefinition, StateBinding, StateTarget},
        state_storage::{Profile, ReferenceKey},
    };

    fn resources() -> SceneResources {
        let mut models = ModelRegistry::default();
        models.programs.insert(
            2,
            vec![ClassProgram {
                catalog: "class.bin".into(),
                program: Ok(giants_formats::class_program::parse(&[
                    0x1e, 2, 0x1f, 20, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 255,
                ])
                .unwrap()),
                bindings: Ok(vec![StateBinding {
                    offset: 0x6c,
                    key: ReferenceKey::Spec(20),
                    target: Ok(StateTarget::Spec {
                        id: 20,
                        catalog: "captured.bin".into(),
                    }),
                }]),
            }],
        );
        models.specs.insert(
            20,
            vec![SpecDefinition {
                catalog: "captured.bin".into(),
                effect_namespace: None,
                models: Ok(vec![]),
                program: Ok(giants_formats::object_spec::parse(&[255]).unwrap()),
                selection: Ok(SelectionSpec {
                    slots: vec![],
                    animations: vec![],
                    effects: vec![],
                    non_model_commands: vec![],
                    diagnostics: vec![],
                }),
            }],
        );
        SceneResources {
            world_bounds: Default::default(),
            classes: crate::catalog_registry::WorldClassRegistry {
                visits: vec![],
                classes: Default::default(),
                textures: Default::default(),
                models,
            },
            archives: Default::default(),
            language: "English".into(),
        }
    }

    #[test]
    fn state_specification_does_not_require_or_fall_back_to_a_body_selection() {
        let resources = resources();
        let prepared =
            Prepared::load_state_without_effects(&resources, 2, Profile::Debug, 0x70).unwrap();
        assert_eq!(prepared.class, 2);
        assert_eq!(prepared.specification, 20);
        assert_eq!(prepared.class_catalog, std::path::Path::new("class.bin"));
        assert_eq!(
            prepared.specification_catalog,
            std::path::Path::new("captured.bin")
        );
        assert!(prepared.models.is_empty());
        assert!(Prepared::load_state_without_effects(&resources, 2, Profile::Debug, 0x6c).is_err());
        assert!(Prepared::load_state_without_effects(&resources, 2, Profile::Gog, 0x6c).is_ok());
    }

    #[test]
    fn state_specification_never_heals_an_absent_or_replaced_capture() {
        let mut resources = resources();
        resources.classes.models.specs.get_mut(&20).unwrap()[0].catalog = "later.bin".into();
        assert_eq!(
            Prepared::load_state_without_effects(&resources, 2, Profile::Debug, 0x70)
                .err()
                .unwrap(),
            "state specification context identity changed"
        );
        resources.classes.models.programs.get_mut(&2).unwrap()[0]
            .bindings
            .as_mut()
            .unwrap()[0]
            .target = Err("absent at registration".into());
        assert_eq!(
            Prepared::load_state_without_effects(&resources, 2, Profile::Debug, 0x70)
                .err()
                .unwrap(),
            "absent at registration"
        );
    }

    #[test]
    fn state_specification_requires_matching_keys_and_captured_effect_namespace() {
        let mut resources = resources();
        resources.classes.models.programs.get_mut(&2).unwrap()[0]
            .bindings
            .as_mut()
            .unwrap()[0]
            .target = Ok(StateTarget::Spec {
            id: 21,
            catalog: "captured.bin".into(),
        });
        assert!(Prepared::load_state_without_effects(&resources, 2, Profile::Debug, 0x70).is_err());
        resources.classes.models.programs.get_mut(&2).unwrap()[0]
            .bindings
            .as_mut()
            .unwrap()[0]
            .target = Ok(StateTarget::Spec {
            id: 20,
            catalog: "captured.bin".into(),
        });
        let program = resources.classes.models.specs.get_mut(&20).unwrap()[0]
            .program
            .as_mut()
            .unwrap();
        program.records.insert(
            0,
            giants_formats::object_spec::Record {
                offset: 0,
                opcode: 0x7f,
                kind: giants_formats::object_spec::Kind::Effects { names: vec![] },
                bytes: vec![],
            },
        );
        assert_eq!(
            Prepared::load_state_without_effects(&resources, 2, Profile::Debug, 0x70)
                .err()
                .unwrap(),
            "state specification requires its captured effect namespace"
        );
    }
}
