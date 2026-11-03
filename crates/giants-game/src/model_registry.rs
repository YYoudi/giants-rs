//! Ordered model/spec references. Prefix support is not complete command execution.
use giants_formats::cast;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct NameDefinition {
    pub catalog: PathBuf,
    pub name: Result<String, String>,
}
#[derive(Debug)]
pub struct SpecDefinition {
    pub catalog: PathBuf,
    /// Image active when this definition was registered, before later catalogs.
    pub effect_namespace: Option<EffectNamespace>,
    pub models: Result<Vec<cast::ModelReference>, String>,
    pub program: Result<giants_formats::object_spec::Program, String>,
    pub selection: Result<SelectionSpec, String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectNamespace {
    pub index: usize,
    pub path: PathBuf,
}
#[derive(Clone, Debug)]
pub struct SelectionSlot {
    pub opcode: u8,
    pub reference: Option<u32>,
    pub name: Option<String>,
    pub threshold: f32,
}
#[derive(Clone, Debug)]
pub struct DeclaredAnimation {
    pub slot: u8,
    pub name: String,
}
#[derive(Clone, Debug)]
pub struct SelectionSpec {
    pub slots: Vec<SelectionSlot>,
    pub animations: Vec<DeclaredAnimation>,
    pub effects: Vec<(u8, Vec<String>)>,
    /// These framed records have not been executed by the independent runtime.
    pub non_model_commands: Vec<u8>,
    pub diagnostics: Vec<String>,
}
impl SelectionSpec {
    pub fn table(&self) -> Result<crate::lod::Table, &'static str> {
        if self.slots.len() > 10 {
            return Err("model selection table exceeds ten slots");
        }
        let mut table = crate::lod::Table {
            present: [false; 10],
            thresholds: [0.; 10],
        };
        for (index, slot) in self.slots.iter().enumerate() {
            table.present[index] = slot.name.is_some();
            table.thresholds[index] = slot.threshold;
        }
        Ok(table)
    }
}
#[derive(Clone, Debug)]
pub struct SelectionBinding {
    pub class_catalog: PathBuf,
    pub spec_id: u32,
    pub spec_catalog: PathBuf,
    pub selection: SelectionSpec,
}
/// Initial specification identity captured at class registration, even when
/// its command projection or one of its resources is not supported yet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpecificationBinding {
    pub class_catalog: PathBuf,
    pub spec_id: u32,
    pub spec_catalog: PathBuf,
}
#[derive(Debug, Clone)]
pub struct ClassModels {
    pub class_catalog: PathBuf,
    pub spec_id: u32,
    pub spec_catalog: PathBuf,
    pub models: Vec<cast::ModelReference>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StateTarget {
    Spec {
        id: u32,
        catalog: PathBuf,
    },
    Effect {
        image: PathBuf,
        entry: usize,
        offset: u32,
    },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StateBinding {
    pub offset: usize,
    pub key: crate::state_storage::ReferenceKey,
    pub target: Result<StateTarget, String>,
}
#[derive(Debug)]
pub struct EffectDefinition {
    pub path: PathBuf,
    pub image: giants_formats::effect_image::Image,
}
#[derive(Debug)]
pub struct ClassProgram {
    pub catalog: PathBuf,
    pub program: Result<giants_formats::class_program::Program, String>,
    pub bindings: Result<Vec<StateBinding>, String>,
}
#[derive(Debug, Default)]
pub struct ModelRegistry {
    pub effects: Vec<EffectDefinition>,
    pub names: BTreeMap<u32, Vec<NameDefinition>>,
    pub specs: BTreeMap<u32, Vec<SpecDefinition>>,
    pub classes: BTreeMap<i32, Vec<Result<ClassModels, String>>>,
    pub selections: BTreeMap<i32, Vec<Result<SelectionBinding, String>>>,
    pub programs: BTreeMap<i32, Vec<ClassProgram>>,
    pub specifications: BTreeMap<i32, Vec<Result<SpecificationBinding, String>>>,
}
impl ModelRegistry {
    /// Event offsets belong to the image active during specification parsing.
    /// Runtime image replacement is not modeled by this fixed loading context.
    pub fn specification_effects(
        &self,
        definition: &SpecDefinition,
    ) -> Result<&giants_formats::effect_image::Image, String> {
        let captured = definition
            .effect_namespace
            .as_ref()
            .ok_or("specification had no effect namespace at registration")?;
        let active = self
            .effects
            .last()
            .ok_or("specification effect image absent")?;
        if captured.index != self.effects.len() - 1 || captured.path != active.path {
            return Err("specification effect image replacement is not modeled".into());
        }
        Ok(&active.image)
    }
    /// Enter-time effect selection precedes recursive catalog dependencies.
    pub fn enter_catalog(
        &mut self,
        catalog: &Path,
        data: &[u8],
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(mode) = cast::effect_selector(data)? {
            let filename = match mode {
                0 => "fx.bin",
                1 => "fx_world_dust.bin",
                _ => "fx_world_snow.bin",
            };
            let directory = catalog.parent().ok_or("catalog has no directory")?;
            let paths = std::fs::read_dir(directory)?
                .collect::<Result<Vec<_>, std::io::Error>>()?
                .into_iter()
                .filter(|r| {
                    r.file_name()
                        .to_string_lossy()
                        .eq_ignore_ascii_case(filename)
                })
                .map(|r| r.path())
                .collect::<Vec<_>>();
            if paths.len() != 1 {
                return Err(format!("missing or ambiguous effect image {filename}").into());
            }
            let path = paths[0].clone();
            if path.metadata()?.len() > 16 * 1024 * 1024 {
                return Err("effect image exceeds input budget".into());
            }
            let image = giants_formats::effect_image::parse(&std::fs::read(&path)?)?;
            self.effects.push(EffectDefinition { path, image });
        }
        Ok(())
    }
    /// Native table phases: models, specs, classes. References bind using the
    /// entries already registered at that phase, including earlier catalogs.
    pub fn register(&mut self, catalog: &Path, data: &[u8]) -> Result<(), giants_formats::Error> {
        let records = cast::catalog_references(data)?;
        let programs = cast::catalog_spec_programs(data)?;
        for (id, name) in records.names {
            self.names.entry(id).or_default().push(NameDefinition {
                catalog: catalog.into(),
                name: name.map_err(|e| e.to_string()),
            });
        }
        for ((id, prefix), (program_id, program)) in records.specs.into_iter().zip(programs) {
            debug_assert_eq!(id, program_id);
            let program = program.map_err(|e| e.to_string());
            let selection = program
                .as_ref()
                .map_err(Clone::clone)
                .and_then(|p| self.resolve_selection(p));
            let models = prefix.map_err(|e| e.to_string()).and_then(|prefix| {
                prefix
                    .into_iter()
                    .map(|record| {
                        let name = if record.reference == 0 {
                            None
                        } else {
                            let definition = self
                                .names
                                .get(&record.reference)
                                .and_then(|v| v.first())
                                .ok_or_else(|| {
                                    format!("model name {} not registered", record.reference)
                                })?;
                            Some(definition.name.clone()?)
                        };
                        Ok(cast::ModelReference {
                            reference: record.reference,
                            name,
                            payload: record.payload,
                        })
                    })
                    .collect()
            });
            self.specs.entry(id).or_default().push(SpecDefinition {
                catalog: catalog.into(),
                effect_namespace: self.effects.last().map(|image| EffectNamespace {
                    index: self.effects.len() - 1,
                    path: image.path.clone(),
                }),
                models,
                program,
                selection,
            });
        }
        for (id, program) in cast::catalog_class_programs(data)? {
            let program = program.map_err(|e| e.to_string());
            let bindings = program
                .as_ref()
                .map_err(Clone::clone)
                .and_then(crate::state_storage::Seed::from_program)
                .map(|seed| {
                    seed.unresolved
                        .into_iter()
                        .map(|r| {
                            let target = match &r.key {
                                crate::state_storage::ReferenceKey::Spec(id) => self
                                    .specs
                                    .get(id)
                                    .and_then(|v| v.first())
                                    .map(|v| StateTarget::Spec {
                                        id: *id,
                                        catalog: v.catalog.clone(),
                                    })
                                    .ok_or_else(|| {
                                        format!("spec {id} unavailable at class registration")
                                    }),
                                crate::state_storage::ReferenceKey::Effect(name) => self
                                    .effects
                                    .last()
                                    .ok_or_else(|| "effect image not loaded".to_string())
                                    .and_then(|d| {
                                        d.image
                                            .find(name)
                                            .map(|(entry, e)| StateTarget::Effect {
                                                image: d.path.clone(),
                                                entry,
                                                offset: e.offset,
                                            })
                                            .ok_or_else(|| format!("effect symbol {name} missing"))
                                    }),
                            };
                            StateBinding {
                                offset: r.offset,
                                key: r.key,
                                target,
                            }
                        })
                        .collect()
                });
            self.programs.entry(id).or_default().push(ClassProgram {
                catalog: catalog.into(),
                program,
                bindings,
            });
        }
        for (id, reference) in records.classes {
            let specification =
                reference
                    .as_ref()
                    .map_err(|e| e.to_string())
                    .and_then(|&spec_id| {
                        let spec = self
                            .specs
                            .get(&spec_id)
                            .and_then(|v| v.first())
                            .ok_or_else(|| format!("model spec {spec_id} not registered"))?;
                        Ok(SpecificationBinding {
                            class_catalog: catalog.into(),
                            spec_id,
                            spec_catalog: spec.catalog.clone(),
                        })
                    });
            self.specifications
                .entry(id)
                .or_default()
                .push(specification);
            let selection = reference
                .as_ref()
                .map_err(|e| e.to_string())
                .and_then(|&spec_id| {
                    let spec = self
                        .specs
                        .get(&spec_id)
                        .and_then(|v| v.first())
                        .ok_or_else(|| format!("model spec {spec_id} not registered"))?;
                    Ok(SelectionBinding {
                        class_catalog: catalog.into(),
                        spec_id,
                        spec_catalog: spec.catalog.clone(),
                        selection: spec.selection.clone()?,
                    })
                });
            self.selections.entry(id).or_default().push(selection);
            let binding = reference.map_err(|e| e.to_string()).and_then(|spec_id| {
                let spec = self
                    .specs
                    .get(&spec_id)
                    .and_then(|v| v.first())
                    .ok_or_else(|| format!("model spec {spec_id} not registered"))?;
                Ok(ClassModels {
                    class_catalog: catalog.into(),
                    spec_id,
                    spec_catalog: spec.catalog.clone(),
                    models: spec.models.clone()?,
                })
            });
            self.classes.entry(id).or_default().push(binding);
        }
        Ok(())
    }
    pub fn first_program(&self, id: i32) -> Option<&ClassProgram> {
        self.programs.get(&id)?.first()
    }
    /// Does not retry a failed registration against a later catalog. A bound
    /// specification may still need parsing, models, animation or execution.
    pub fn first_specification(&self, id: i32) -> Option<&Result<SpecificationBinding, String>> {
        self.specifications.get(&id)?.first()
    }
    /// Explicit debug profile. This prepares a pointer-free scalar template;
    /// model/state dependencies remain needs, not successful object creation.
    pub fn debug_scalar_template(
        &self,
        id: i32,
    ) -> Result<crate::class_template::DebugScalar, crate::class_template::Need> {
        let source = self
            .first_program(id)
            .ok_or(crate::class_template::Need::Class(id))?;
        let program = source
            .program
            .as_ref()
            .map_err(|e| crate::class_template::Need::Program(e.clone()))?;
        crate::class_template::DebugScalar::materialize(id, program)
    }
    pub fn first(&self, id: i32) -> Option<&Result<ClassModels, String>> {
        self.classes.get(&id)?.first()
    }
    pub fn first_selection(&self, id: i32) -> Option<&Result<SelectionBinding, String>> {
        self.selections.get(&id)?.first()
    }
    fn resolve_selection(
        &self,
        program: &giants_formats::object_spec::Program,
    ) -> Result<SelectionSpec, String> {
        use giants_formats::object_spec::Kind;
        let mut result = SelectionSpec {
            slots: Vec::new(),
            animations: Vec::new(),
            effects: Vec::new(),
            non_model_commands: Vec::new(),
            diagnostics: Vec::new(),
        };
        for record in &program.records {
            match &record.kind {
                Kind::Model { reference, angular } => {
                    let name = if record.opcode == 0x51 && *reference == 0 {
                        None
                    } else {
                        Some(
                            self.names
                                .get(reference)
                                .and_then(|v| v.first())
                                .ok_or_else(|| format!("model name {reference} not registered"))?
                                .name
                                .clone()?,
                        )
                    };
                    let threshold = match angular {
                        Some([angle, distance]) => crate::lod::threshold(
                            f32::from_bits(*angle),
                            f32::from_bits(*distance),
                        )?
                        .to_owned(),
                        None => match record.opcode {
                            0x59 => -3.,
                            0x5a => -1.,
                            0x5b => -2.,
                            _ => return Err("invalid model selection record".into()),
                        },
                    };
                    if record.opcode == 0x5b {
                        result
                            .diagnostics
                            .push("native diagnostic for model opcode5b".into());
                    }
                    result.slots.push(SelectionSlot {
                        opcode: record.opcode,
                        reference: Some(*reference),
                        name,
                        threshold,
                    });
                }
                Kind::Animation { slot, name } => {
                    if *slot >= 254 {
                        return Err("animation slot overlaps model table".into());
                    }
                    result.animations.push(DeclaredAnimation {
                        slot: *slot,
                        name: name.clone(),
                    });
                    result.non_model_commands.push(record.opcode);
                }
                Kind::Effects { names } => {
                    result.effects.push((record.opcode, names.clone()));
                    result.non_model_commands.push(record.opcode);
                }
                Kind::Data => result.non_model_commands.push(record.opcode),
                Kind::End => {
                    if result.slots.last().is_some_and(|v| v.name.is_some()) {
                        if result.slots.len() >= 10 {
                            return Err("no space for native implicit empty slot".into());
                        }
                        result.slots.push(SelectionSlot {
                            opcode: 0xff,
                            reference: None,
                            name: None,
                            threshold: 1500.,
                        });
                    }
                }
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn catalog(label: &[u8], with_spec: bool) -> Vec<u8> {
        let mut b = vec![0u8; 108];
        for (at, value) in [
            (0, 0x1a0002e5u32),
            (16, 32),
            (32, 72),
            (36, 1),
            (40, 28),
            (48, u32::from(with_spec)),
            (52, 40),
            (if with_spec { 56 } else { 52 }, 1),
            (if with_spec { 60 } else { 56 }, 60),
            (64, 7),
            (76, 9),
            (81, 7),
            (96, 123),
            (101, 9),
        ] {
            b[at..at + 4].copy_from_slice(&value.to_le_bytes());
        }
        b[68..74].copy_from_slice(label);
        b[80] = 0x51;
        b[93] = 255;
        b[100] = 1;
        b[105] = 255;
        b
    }
    #[test]
    fn selection_records_after_animation_and_implicit_empty_tail_are_retained() {
        let mut data = catalog(b"mesh0\0", true);
        let extra = b"\x52\x01walk.anm\0\x5a\x07\0\0\0";
        data.splice(93..93, extra.iter().copied());
        let len = data.len() as u32;
        data[32..36].copy_from_slice(&(len - 36).to_le_bytes());
        data[60..64].copy_from_slice(&(60 + extra.len() as u32).to_le_bytes());
        let mut registry = ModelRegistry::default();
        registry.register(Path::new("test.bin"), &data).unwrap();
        let selected = registry.selections[&123][0].as_ref().unwrap();
        assert_eq!(selected.selection.animations[0].name, "walk.anm");
        assert_eq!(selected.selection.slots.len(), 3);
        assert_eq!(selected.selection.slots[1].opcode, 0x5a);
        assert_eq!(selected.selection.slots[1].threshold, -1.);
        assert_eq!(selected.selection.slots[2].reference, None);
        assert_eq!(selected.selection.slots[2].threshold, 1500.);
        let table = selected.selection.table().unwrap();
        assert_eq!(table.present[..3], [true, true, false]);
    }
    #[test]
    fn global_first_name_and_spec_override_local_candidates_without_discarding_them() {
        let mut registry = ModelRegistry::default();
        registry
            .register(Path::new("first.bin"), &catalog(b"mesh0\0", true))
            .unwrap();
        registry
            .register(Path::new("second.bin"), &catalog(b"mesh1\0", true))
            .unwrap();
        assert_eq!(registry.names[&7].len(), 2);
        assert_eq!(registry.specs[&9].len(), 2);
        let second = registry.classes[&123][1].as_ref().unwrap();
        assert_eq!(second.class_catalog, Path::new("second.bin"));
        assert_eq!(second.spec_catalog, Path::new("first.bin"));
        assert_eq!(second.models[0].name.as_deref(), Some("mesh0"));
        assert_eq!(
            registry.specs[&9][1].models.as_ref().unwrap()[0]
                .name
                .as_deref(),
            Some("mesh0")
        );
    }
    #[test]
    fn missing_forward_reference_is_not_repaired_by_a_later_catalog() {
        let mut registry = ModelRegistry::default();
        registry
            .register(Path::new("early.bin"), &catalog(b"mesh0\0", false))
            .unwrap();
        assert!(registry.first(123).unwrap().is_err());
        registry
            .register(Path::new("late.bin"), &catalog(b"mesh1\0", true))
            .unwrap();
        assert!(registry.first(123).unwrap().is_err());
        assert!(registry.classes[&123][1].is_ok());
        assert!(registry.first_specification(123).unwrap().is_err());
        assert_eq!(
            registry.specifications[&123][1]
                .as_ref()
                .unwrap()
                .spec_catalog,
            Path::new("late.bin")
        );
    }
    #[test]
    fn specification_namespace_tracks_registration_and_refuses_replacement() {
        let mut registry = ModelRegistry::default();
        let empty =
            || giants_formats::effect_image::parse(&[1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]).unwrap();
        registry
            .register(Path::new("early.bin"), &catalog(b"mesh0\0", true))
            .unwrap();
        registry.effects.push(EffectDefinition {
            path: "dust.bin".into(),
            image: empty(),
        });
        registry
            .register(Path::new("later.bin"), &catalog(b"mesh1\0", true))
            .unwrap();
        let definitions = &registry.specs[&9];
        // A missing namespace on the first global definition is never healed.
        assert!(registry.specification_effects(&definitions[0]).is_err());
        assert_eq!(definitions[1].effect_namespace.as_ref().unwrap().index, 0);
        assert!(registry.specification_effects(&definitions[1]).is_ok());
        registry.effects.push(EffectDefinition {
            path: "snow.bin".into(),
            image: empty(),
        });
        assert_eq!(
            registry
                .specification_effects(&registry.specs[&9][1])
                .unwrap_err(),
            "specification effect image replacement is not modeled"
        );
        // Even reloading an identically named file is a different registration.
        registry.effects[1].path = "dust.bin".into();
        assert!(
            registry
                .specification_effects(&registry.specs[&9][1])
                .is_err()
        );
    }
    #[test]
    fn unsupported_spec_body_keeps_its_identity_and_first_registration() {
        let mut first = catalog(b"mesh0\0", true);
        first[80] = 0xaa;
        let mut registry = ModelRegistry::default();
        registry.register(Path::new("first.bin"), &first).unwrap();
        registry
            .register(Path::new("later.bin"), &catalog(b"mesh1\0", true))
            .unwrap();
        assert!(registry.first_selection(123).unwrap().is_err());
        for binding in &registry.specifications[&123] {
            let binding = binding.as_ref().unwrap();
            assert_eq!(binding.spec_id, 9);
            assert_eq!(binding.spec_catalog, Path::new("first.bin"));
        }
        assert!(registry.specs[&9][0].program.is_err());
        assert!(registry.specs[&9][1].program.is_ok());
    }
    #[test]
    fn state_spec_binding_does_not_adopt_later_registration() {
        fn with_attribute(mut data: Vec<u8>) -> Vec<u8> {
            data.truncate(100);
            data.push(1);
            data.extend(9u32.to_le_bytes());
            data.extend([0x1e, 2, 0x1f]);
            for v in [9u32, 1, 2, 3] {
                data.extend(v.to_le_bytes());
            }
            data.push(0xff);
            let size = (data.len() - 36) as u32;
            data[32..36].copy_from_slice(&size.to_le_bytes());
            data
        }
        let mut r = ModelRegistry::default();
        r.register(
            Path::new("early.bin"),
            &with_attribute(catalog(b"early\0", false)),
        )
        .unwrap();
        assert!(
            r.first_program(123).unwrap().bindings.as_ref().unwrap()[0]
                .target
                .is_err()
        );
        r.register(
            Path::new("later.bin"),
            &with_attribute(catalog(b"later\0", true)),
        )
        .unwrap();
        assert!(
            r.first_program(123).unwrap().bindings.as_ref().unwrap()[0]
                .target
                .is_err()
        );
        assert!(matches!(
            r.programs[&123][1].bindings.as_ref().unwrap()[0].target,
            Ok(StateTarget::Spec { id: 9, .. })
        ));
    }
}
