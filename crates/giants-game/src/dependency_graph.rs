//! Static dependency graph. Required variants are a superset, not a visible draw set.
use crate::{catalog_registry::Location, scene_resources::SceneResources};
use giants_formats::{flick, mission};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct ClassOrigin {
    pub source: PathBuf,
    pub offset: usize,
    pub slot: usize,
    pub class_id: i32,
    pub kind: &'static str,
}
#[derive(Debug)]
pub struct Script {
    pub source: PathBuf,
    pub metadata: flick::DependencyMetadata,
}
#[derive(Debug)]
pub struct ResourceDependency {
    pub selected: Result<Location, String>,
    pub roles: BTreeSet<&'static str>,
    pub required: bool,
}
#[derive(Debug)]
pub struct ModelDependency {
    pub slot: usize,
    pub opcode: u8,
    pub reference: Option<u32>,
    pub threshold: f32,
    pub name: Option<String>,
    pub textures: Vec<String>,
    pub auxiliary_candidates: Vec<String>,
    pub error: Option<String>,
}
/// Dependencies of a specification reached through a state field. These are
/// potential resources, not evidence that the state will execute or draw them.
#[derive(Debug)]
pub struct StateSpecDependencies {
    pub selection: crate::model_registry::SelectionSpec,
    pub models: Vec<ModelDependency>,
}
#[derive(Debug)]
pub enum EffectReferenceTarget {
    AdaptiveEffect {
        image: PathBuf,
        normal: Result<(usize, u32), String>,
        preferred: Result<(usize, u32), String>,
    },
    Texture {
        catalog: PathBuf,
        entry: usize,
        name: String,
        flags: u8,
        files: Vec<String>,
        conditional: bool,
    },
    Class {
        id: i32,
        catalog: PathBuf,
    },
    Effect {
        image: PathBuf,
        entry: usize,
        offset: u32,
    },
}
#[derive(Debug)]
pub struct EffectEdge {
    pub image: PathBuf,
    pub program: u32,
    pub reference: giants_formats::effect_program::Reference,
    pub target: Result<EffectReferenceTarget, String>,
}
type MeshDependencies = BTreeMap<String, Result<BTreeSet<(String, &'static str)>, String>>;
#[derive(Debug, Default)]
pub struct DependencyGraph {
    pub mission_source: Option<PathBuf>,
    pub placements: Vec<mission::Placement>,
    /// Source-scoped declarations, not world-space placements or live actors.
    pub mission_children: Vec<mission::LockedChild>,
    pub mission_orientations: Vec<mission::Orientation>,
    pub mission_object_words: Vec<mission::ObjectWords>,
    pub mission_state_writes: Vec<mission::StateWrite>,
    pub mission_slot_writes: Vec<mission::SlotWrite>,
    pub mission_object_selectors: Vec<mission::ObjectSelector>,
    pub mission_starts: Vec<mission::Start>,
    pub mission_object_properties: Vec<mission::ObjectProperty>,
    pub origins: Vec<ClassOrigin>,
    pub scripts: Vec<Script>,
    pub classes: BTreeMap<i32, Result<Vec<ModelDependency>, String>>,
    pub resources: BTreeMap<String, ResourceDependency>,
    /// Complete declarations and provenance; non-model programs remain unexecuted.
    pub specifications: BTreeMap<i32, crate::model_registry::SelectionBinding>,
    /// Unique declared animation images, parsed without binding or evaluation.
    pub animations: BTreeMap<String, Result<giants_formats::anm::Animation, String>>,
    pub diagnostics: Vec<String>,
    pub state_bindings: BTreeMap<i32, Result<Vec<crate::model_registry::StateBinding>, String>>,
    /// Keyed by the identity captured at registration, independent of class geometry.
    pub state_specifications: BTreeMap<(u32, PathBuf), Result<StateSpecDependencies, String>>,
    /// Framed state-reachable FX programs; all commands remain unexecuted.
    pub effect_programs:
        BTreeMap<(PathBuf, u32), Result<giants_formats::effect_program::Program, String>>,
    /// Native-checked operand roles, without activation or execution.
    pub effect_references:
        BTreeMap<(PathBuf, u32), Result<Vec<giants_formats::effect_program::Reference>, String>>,
    pub effect_edges: Vec<EffectEdge>,
    pub state_seeds: BTreeMap<i32, Result<crate::state_storage::Seed, String>>,
}
fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 100
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
}
fn read_program(path: &Path) -> Result<Vec<u8>, String> {
    if path.metadata().map_err(|e| e.to_string())?.len() > 4 * 1024 * 1024 {
        return Err("dependency program exceeds input budget".into());
    }
    std::fs::read(path).map_err(|e| e.to_string())
}
impl DependencyGraph {
    /// All registered classes and all symbols of the active FX image. Explicit
    /// unsupported programs and missing targets are retained in the report.
    pub fn registered_effect_context(resources: &SceneResources) -> Result<Self, String> {
        let mut graph = Self::default();
        let roots = resources.classes.models.programs.keys().copied().collect();
        let active = resources
            .classes
            .models
            .effects
            .last()
            .ok_or("no active effect image")?;
        let mut effects = BTreeMap::new();
        for (entry, symbol) in active.image.entries.iter().enumerate() {
            let target = crate::model_registry::StateTarget::Effect {
                image: active.path.clone(),
                entry,
                offset: symbol.offset,
            };
            Self::enqueue_effect(&graph.effect_programs, &mut effects, target);
        }
        graph.expand_with_effects(resources, roots, effects, 8192, 65536)?;
        Ok(graph)
    }
    /// All registered class templates in one fixed loading context. This is a
    /// catalog-wide potential inventory, not a selected mission or visible set.
    pub fn registered_context(resources: &SceneResources) -> Result<Self, String> {
        let mut graph = Self::default();
        let roots = resources.classes.models.programs.keys().copied().collect();
        graph.expand(resources, roots, 8192, 65536)?;
        Ok(graph)
    }
    /// Mission and starting script are explicit inputs. This does not infer a
    /// mission selection rule or execute script control flow.
    pub fn build(
        root: &Path,
        resources: &SceneResources,
        mission: &str,
        first: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut files = BTreeMap::new();
        for entry in std::fs::read_dir(root.join("Bin"))? {
            let path = entry?.path();
            if !path.is_file() {
                continue;
            }
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if files.insert(name.to_ascii_lowercase(), path).is_some() {
                return Err("case-ambiguous input filename".into());
            }
        }
        let mut graph = Self::default();
        if !identifier(mission) || !identifier(first) {
            return Err("invalid dependency input identifier".into());
        }
        let mission_key = format!("wm_{mission}.bin").to_ascii_lowercase();
        if let Some(path) = files.get(&mission_key) {
            graph.mission_source = Some(path.clone());
            match read_program(path)
                .and_then(|data| mission::parse(&data).map_err(|e| e.to_string()))
            {
                Ok(mission) => {
                    for (slot, placement) in mission.placements.iter().enumerate() {
                        graph.origins.push(ClassOrigin {
                            source: path.clone(),
                            offset: placement.source_offset,
                            slot,
                            class_id: placement.object_type as i32,
                            kind: "placement",
                        });
                    }
                    for (slot, child) in mission.locked_children.iter().enumerate() {
                        graph.origins.push(ClassOrigin {
                            source: path.clone(),
                            offset: child.source_offset,
                            slot,
                            class_id: child.object_type as i32,
                            kind: "mission_locked_child",
                        });
                    }
                    graph.placements = mission.placements;
                    graph.mission_children = mission.locked_children;
                    graph.mission_orientations = mission.orientations;
                    graph.mission_object_words = mission.object_words;
                    graph.mission_state_writes = mission.state_writes;
                    graph.mission_slot_writes = mission.slot_writes;
                    graph.mission_object_selectors = mission.object_selectors;
                    graph.mission_starts = mission.starts;
                    graph.mission_object_properties = mission.object_properties;
                }
                Err(error) => graph
                    .diagnostics
                    .push(format!("mission {mission_key}: {error}")),
            }
        } else {
            graph
                .diagnostics
                .push(format!("missing mission {mission_key}"));
        }
        let mut next = Some(first.to_string());
        let mut visited = BTreeSet::new();
        while let Some(name) = next.take() {
            if !identifier(&name) {
                graph
                    .diagnostics
                    .push("invalid linked script identifier".into());
                break;
            }
            let key = format!("flk_{name}.bin").to_ascii_lowercase();
            if visited.len() >= 256 || !visited.insert(key.clone()) {
                graph
                    .diagnostics
                    .push(format!("script cycle or visit budget at {key}"));
                break;
            }
            let Some(path) = files.get(&key) else {
                graph.diagnostics.push(format!("missing script {key}"));
                break;
            };
            let metadata = match read_program(path)
                .and_then(|data| flick::discover_dependencies(&data).map_err(|e| e.to_string()))
            {
                Ok(value) => value,
                Err(error) => {
                    graph.diagnostics.push(format!("script {key}: {error}"));
                    break;
                }
            };
            for actor in &metadata.actors {
                let command = metadata
                    .commands
                    .iter()
                    .find(|c| c.opcode == 2 && c.words.get(2) == Some(&(actor.slot as u32)))
                    .ok_or("actor declaration origin missing")?;
                graph.origins.push(ClassOrigin {
                    source: path.clone(),
                    offset: command.offset,
                    slot: actor.slot,
                    class_id: actor.object_type,
                    kind: "declaration",
                });
            }
            next = metadata.next.clone();
            graph.scripts.push(Script {
                source: path.clone(),
                metadata,
            });
        }
        let ids: BTreeSet<_> = graph
            .origins
            .iter()
            .filter_map(|o| (o.kind == "placement" || o.class_id != -1).then_some(o.class_id))
            .collect();
        graph.expand(resources, ids, 8192, 65536)?;
        Ok(graph)
    }
    /// Fixed-context potential closure. No object/callback/script is executed.
    fn expand(
        &mut self,
        resources: &SceneResources,
        classes: BTreeSet<i32>,
        node_budget: usize,
        edge_budget: usize,
    ) -> Result<(), String> {
        self.expand_with_effects(
            resources,
            classes,
            BTreeMap::new(),
            node_budget,
            edge_budget,
        )
    }
    fn expand_with_effects(
        &mut self,
        resources: &SceneResources,
        mut classes: BTreeSet<i32>,
        mut effects: BTreeMap<(PathBuf, u32), crate::model_registry::StateTarget>,
        node_budget: usize,
        edge_budget: usize,
    ) -> Result<(), String> {
        let mut meshes = BTreeMap::new();
        while !classes.is_empty() || !effects.is_empty() {
            while let Some(id) = classes.pop_first() {
                if self.classes.contains_key(&id) {
                    continue;
                }
                self.check_budget(node_budget)?;
                self.class_dependencies(resources, id, &mut meshes);
                let bindings = self
                    .state_bindings
                    .get(&id)
                    .and_then(|b| b.as_ref().ok())
                    .cloned()
                    .unwrap_or_default();
                for binding in bindings {
                    match binding.target {
                        Ok(crate::model_registry::StateTarget::Spec { id, catalog }) => {
                            if !self
                                .state_specifications
                                .contains_key(&(id, catalog.clone()))
                            {
                                self.check_budget(node_budget)?;
                                self.state_spec_dependencies(resources, id, catalog, &mut meshes);
                            }
                        }
                        Ok(target @ crate::model_registry::StateTarget::Effect { .. }) => {
                            Self::enqueue_effect(&self.effect_programs, &mut effects, target);
                        }
                        Err(_) => {} // Retained in state_bindings, never healed.
                    }
                }
            }
            let Some((key, target)) = effects.pop_first() else {
                continue;
            };
            if self.effect_programs.contains_key(&key) {
                continue;
            }
            self.check_budget(node_budget)?;
            let program = resources.bound_effect_program(&target);
            let references = program
                .as_ref()
                .map_err(Clone::clone)
                .and_then(|p| p.references().map_err(|e| e.to_string()));
            if let Ok(references) = &references {
                for reference in references {
                    if self.effect_edges.len() >= edge_budget {
                        return Err("effect dependency edge budget exceeded".into());
                    }
                    let target = match &reference.key {
                        giants_formats::effect_program::ReferenceKey::AdaptiveEffect(name) => {
                            let mut targets = [false, true].map(|prefer_lite| {
                                resources.adaptive_effect_target(&key.0, name, prefer_lite)
                            });
                            for target in targets.iter().flatten() {
                                Self::enqueue_effect(
                                    &self.effect_programs,
                                    &mut effects,
                                    target.clone(),
                                );
                            }
                            let [normal, preferred] =
                                targets.each_mut().map(|target| match target {
                                    Ok(crate::model_registry::StateTarget::Effect {
                                        entry,
                                        offset,
                                        ..
                                    }) => Ok((*entry, *offset)),
                                    Ok(_) => unreachable!(),
                                    Err(error) => Err(error.clone()),
                                });
                            Ok(EffectReferenceTarget::AdaptiveEffect {
                                image: key.0.clone(),
                                normal,
                                preferred,
                            })
                        }
                        giants_formats::effect_program::ReferenceKey::Texture(name) => {
                            self.texture_dependencies(resources, name)
                        }
                        giants_formats::effect_program::ReferenceKey::Class(word) => {
                            let id = i32::from_le_bytes(word.to_le_bytes());
                            classes.insert(id);
                            resources
                                .classes
                                .models
                                .first_program(id)
                                .map(|d| EffectReferenceTarget::Class {
                                    id,
                                    catalog: d.catalog.clone(),
                                })
                                .ok_or_else(|| {
                                    format!("effect class {id} not registered in world context")
                                })
                        }
                        giants_formats::effect_program::ReferenceKey::Effect(name) => {
                            resources.effect_target(&key.0, name).map(|target| {
                                let crate::model_registry::StateTarget::Effect {
                                    image,
                                    entry,
                                    offset,
                                } = &target
                                else {
                                    unreachable!()
                                };
                                let resolved = EffectReferenceTarget::Effect {
                                    image: image.clone(),
                                    entry: *entry,
                                    offset: *offset,
                                };
                                Self::enqueue_effect(&self.effect_programs, &mut effects, target);
                                resolved
                            })
                        }
                    };
                    self.effect_edges.push(EffectEdge {
                        image: key.0.clone(),
                        program: key.1,
                        reference: reference.clone(),
                        target,
                    });
                }
            }
            self.effect_programs.insert(key.clone(), program);
            self.effect_references.insert(key, references);
        }
        Ok(())
    }
    fn texture_dependencies(
        &mut self,
        resources: &SceneResources,
        name: &str,
    ) -> Result<EffectReferenceTarget, String> {
        let definition = resources.classes.textures.lookup(name).ok_or_else(|| format!("texture {name} has no registered declaration; dynamic descriptor allocation is not modeled"))?;
        let label = &definition.declaration.name;
        if !identifier(label) {
            return Err("effect texture member name outside supported profile".into());
        }
        let tga = format!("{label}.tga").to_ascii_lowercase();
        let dds = format!("{label}.dds").to_ascii_lowercase();
        let conditional =
            definition.declaration.flags & 4 == 0 && resources.archives.members.contains_key(&dds);
        let files = if conditional {
            vec![tga, dds]
        } else {
            vec![tga]
        };
        for file in &files {
            self.resource(
                resources,
                file,
                if conditional {
                    "effect-texture-candidate"
                } else {
                    "effect-texture"
                },
                !conditional,
            );
        }
        Ok(EffectReferenceTarget::Texture {
            catalog: definition.catalog.clone(),
            entry: definition.entry,
            name: label.clone(),
            flags: definition.declaration.flags,
            files,
            conditional,
        })
    }
    fn enqueue_effect(
        visited: &BTreeMap<(PathBuf, u32), Result<giants_formats::effect_program::Program, String>>,
        pending: &mut BTreeMap<(PathBuf, u32), crate::model_registry::StateTarget>,
        target: crate::model_registry::StateTarget,
    ) {
        if let crate::model_registry::StateTarget::Effect { image, offset, .. } = &target {
            let key = (image.clone(), *offset);
            if !visited.contains_key(&key) {
                pending.entry(key).or_insert(target);
            }
        }
    }
    fn check_budget(&self, budget: usize) -> Result<(), String> {
        if self.classes.len() + self.state_specifications.len() + self.effect_programs.len()
            >= budget
        {
            Err("dependency node budget exceeded".into())
        } else {
            Ok(())
        }
    }
    fn class_dependencies(
        &mut self,
        resources: &SceneResources,
        id: i32,
        meshes: &mut MeshDependencies,
    ) {
        self.state_bindings
            .insert(id, resources.class_state_bindings(id).map(|v| v.to_vec()));
        self.state_seeds.insert(
            id,
            resources.class_state_seed(id).map_err(|e| e.to_string()),
        );
        let binding = match resources.classes.models.first_selection(id) {
            Some(Ok(value)) => value,
            Some(Err(error)) => {
                self.classes.insert(id, Err(error.clone()));
                return;
            }
            None => {
                self.classes
                    .insert(id, Err("class not registered in world context".into()));
                return;
            }
        };
        let models = self.selection_dependencies(resources, &binding.selection, meshes);
        self.classes.insert(id, Ok(models));
        self.specifications.insert(id, binding.clone());
    }
    fn state_spec_dependencies(
        &mut self,
        resources: &SceneResources,
        id: u32,
        catalog: PathBuf,
        meshes: &mut MeshDependencies,
    ) {
        let selected = resources
            .classes
            .models
            .specs
            .get(&id)
            .and_then(|entries| entries.first())
            .ok_or_else(|| "bound spec missing from registry".to_string())
            .and_then(|spec| {
                if spec.catalog != catalog {
                    return Err("bound spec provenance differs from registry".into());
                }
                spec.selection.clone()
            });
        let dependencies = selected.map(|selection| {
            let models = self.selection_dependencies(resources, &selection, meshes);
            StateSpecDependencies { selection, models }
        });
        self.state_specifications
            .insert((id, catalog), dependencies);
    }
    fn selection_dependencies(
        &mut self,
        resources: &SceneResources,
        selection: &crate::model_registry::SelectionSpec,
        meshes: &mut MeshDependencies,
    ) -> Vec<ModelDependency> {
        let mut models = Vec::new();
        for animation in &selection.animations {
            self.resource(resources, &animation.name, "declared-animation", true);
            self.animations
                .entry(animation.name.to_ascii_lowercase())
                .or_insert_with(|| {
                    resources
                        .animation(&animation.name)
                        .map_err(|e| e.to_string())
                });
        }
        for (slot, reference) in selection.slots.iter().enumerate() {
            let mut model = ModelDependency {
                slot,
                opcode: reference.opcode,
                reference: reference.reference,
                threshold: reference.threshold,
                name: reference.name.clone(),
                textures: Vec::new(),
                auxiliary_candidates: Vec::new(),
                error: None,
            };
            if let Some(name) = &reference.name {
                let member = format!("{name}.gbs");
                self.resource(resources, &member, "mesh", true);
                let dependencies = meshes
                    .entry(member.to_ascii_lowercase())
                    .or_insert_with(|| {
                        let raw = resources
                            .archives
                            .read_bin(&member, &resources.language)
                            .map_err(|e| e.to_string())?;
                        let mesh = giants_formats::gbs::parse(&raw).map_err(|e| e.to_string())?;
                        let mut textures = BTreeSet::new();
                        for part in mesh.subobjects {
                            for (texture, role) in [
                                (part.material.texture, "diffuse"),
                                (part.material.bump_texture, "bump"),
                            ] {
                                if !texture.is_empty() {
                                    textures.insert((format!("{texture}.tga"), role));
                                }
                            }
                        }
                        Ok::<_, String>(textures)
                    });
                match dependencies {
                    Ok(textures) => {
                        for (texture, role) in textures.iter() {
                            self.resource(resources, texture, role, true);
                            model.textures.push(texture.to_ascii_lowercase());
                        }
                    }
                    Err(error) => model.error = Some(error.clone()),
                }
                for (extension, role) in [("skn", "candidate-skin"), ("anm", "candidate-animation")]
                {
                    let key = format!("{name}.{extension}").to_ascii_lowercase();
                    if resources.archives.members.contains_key(&key) {
                        self.resource(resources, &key, role, false);
                        model.auxiliary_candidates.push(key);
                    }
                }
            }
            models.push(model);
        }
        models
    }
    fn resource(
        &mut self,
        resources: &SceneResources,
        member: &str,
        role: &'static str,
        required: bool,
    ) {
        let entry = self
            .resources
            .entry(member.to_ascii_lowercase())
            .or_insert_with(|| ResourceDependency {
                selected: resources
                    .archives
                    .select_bin(member, &resources.language)
                    .cloned(),
                roles: BTreeSet::new(),
                required: false,
            });
        entry.roles.insert(role);
        entry.required |= required;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn texture_dependencies_keep_conditional_files_and_missing_descriptors_explicit() {
        let mut resources = SceneResources {
            world_bounds: Default::default(),
            classes: crate::catalog_registry::WorldClassRegistry {
                visits: vec![],
                classes: Default::default(),
                models: Default::default(),
                textures: Default::default(),
            },
            archives: Default::default(),
            language: "English".into(),
        };
        let mut graph = DependencyGraph::default();
        assert!(
            graph
                .texture_dependencies(&resources, "not_registered")
                .unwrap_err()
                .contains("dynamic descriptor")
        );
        let mut data = vec![0; 32];
        data[..4].copy_from_slice(&0x1a0002e5u32.to_le_bytes());
        data[8..12].copy_from_slice(&32u32.to_le_bytes());
        data.extend(1u32.to_le_bytes());
        data.extend([0, 0, 5]);
        data.extend(b"Leaf\0");
        resources
            .classes
            .textures
            .register(Path::new("textures.bin"), &data)
            .unwrap();
        for file in ["leaf.tga", "leaf.dds"] {
            resources.archives.members.insert(
                file.into(),
                vec![Location {
                    archive: "Bin/a.gzp".into(),
                    member: file.into(),
                }],
            );
        }
        let target = graph.texture_dependencies(&resources, "LEAF").unwrap();
        assert!(matches!(
            target,
            EffectReferenceTarget::Texture {
                conditional: true,
                ..
            }
        ));
        assert!(
            graph
                .resources
                .values()
                .all(|r| !r.required && r.roles.contains("effect-texture-candidate"))
        );
        resources.archives.members.remove("leaf.dds");
        let mut cold = DependencyGraph::default();
        assert!(matches!(
            cold.texture_dependencies(&resources, "leaf").unwrap(),
            EffectReferenceTarget::Texture {
                conditional: false,
                ..
            }
        ));
        assert_eq!(cold.resources.len(), 1);
        assert!(cold.resources["leaf.tga"].required);
    }
    #[test]
    fn effect_closure_retains_cycles_missing_targets_budgets_and_image_policy() {
        use crate::model_registry::{ClassProgram, EffectDefinition, StateBinding, StateTarget};
        fn image(a: &[u8], b: &[u8]) -> giants_formats::effect_image::Image {
            let mut payload = b"A\0".to_vec();
            payload.extend(12u32.to_le_bytes());
            payload.extend(b"B\0");
            payload.extend((12 + a.len() as u32).to_le_bytes());
            payload.extend(a);
            payload.extend(b);
            let mut bytes = 1u32.to_le_bytes().to_vec();
            bytes.extend((payload.len() as u32).to_le_bytes());
            bytes.extend(2u32.to_le_bytes());
            bytes.extend(payload);
            giants_formats::effect_image::parse(&bytes).unwrap()
        }
        // Class2 is reachable only through attached creation, whose first word
        // deliberately names a different (missing) class-shaped value.
        let a = [
            0x24, 0xe7, 3, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0x0d, b'b', 0, 0x0d, b'm', 0, 2,
        ];
        let b = [0x0d, b'a', 0, 0x0d, b'b', 0, 3, 0xe7, 3, 0, 0, 2];
        let mut models = crate::model_registry::ModelRegistry::default();
        models.effects.push(EffectDefinition {
            path: "active.bin".into(),
            image: image(&a, &b),
        });
        for id in [1, 2] {
            models.programs.insert(
                id,
                vec![ClassProgram {
                    catalog: "class.bin".into(),
                    program: Ok(giants_formats::class_program::parse(&[0xff]).unwrap()),
                    bindings: Ok(vec![]),
                }],
            );
        }
        models.programs.get_mut(&1).unwrap()[0].bindings = Ok(vec![StateBinding {
            offset: 4,
            key: crate::state_storage::ReferenceKey::Effect("A".into()),
            target: Ok(StateTarget::Effect {
                image: "active.bin".into(),
                entry: 0,
                offset: 12,
            }),
        }]);
        let mut resources = SceneResources {
            world_bounds: Default::default(),
            classes: crate::catalog_registry::WorldClassRegistry {
                visits: vec![],
                classes: Default::default(),
                textures: Default::default(),
                models,
            },
            archives: Default::default(),
            language: "English".into(),
        };
        let mut graph = DependencyGraph::default();
        graph
            .expand(&resources, BTreeSet::from([1]), 16, 16)
            .unwrap();
        assert_eq!(graph.effect_programs.len(), 2);
        assert_eq!(
            graph.classes.keys().copied().collect::<Vec<_>>(),
            [1, 2, 999]
        );
        assert_eq!(graph.effect_edges.len(), 6);
        assert_eq!(
            graph
                .effect_edges
                .iter()
                .filter(|e| e.target.is_err())
                .count(),
            2
        );
        assert!(graph.classes[&999].is_err());
        assert!(matches!(
            resources.adaptive_effect_target(Path::new("active.bin"), "a", true),
            Ok(StateTarget::Effect { offset: 12, .. })
        ));
        // Both B->A and the B->B self edge survive deduplication.
        let b_offset = 12 + a.len() as u32;
        assert!(graph.effect_edges.iter().any(|e| e.program == b_offset
            && matches!(
                e.target,
                Ok(EffectReferenceTarget::Effect { offset: 12, .. })
            )));
        assert!(graph.effect_edges.iter().any(|e|e.program==b_offset && matches!(e.target,Ok(EffectReferenceTarget::Effect {offset,..}) if offset==b_offset)));
        assert!(
            DependencyGraph::default()
                .expand(&resources, BTreeSet::from([1]), 4, 16)
                .is_err()
        );
        assert!(
            DependencyGraph::default()
                .expand(&resources, BTreeSet::from([1]), 16, 2)
                .is_err()
        );
        resources.classes.models.effects.push(EffectDefinition {
            path: "replacement.bin".into(),
            image: image(&a, &b),
        });
        let mut replaced = DependencyGraph::default();
        replaced
            .expand(&resources, BTreeSet::from([1]), 16, 16)
            .unwrap();
        assert!(
            replaced.effect_programs[&("active.bin".into(), 12)]
                .as_ref()
                .unwrap_err()
                .contains("replacement")
        );
        assert!(replaced.effect_edges.is_empty());
        assert!(
            resources
                .adaptive_effect_target(Path::new("active.bin"), "a", true)
                .unwrap_err()
                .contains("replacement")
        );
        resources.classes.models.effects.clear();
        resources.classes.models.effects.push(EffectDefinition {
            path: "active.bin".into(),
            image: image(&[0xfe, 2], &b),
        });
        let mut unsupported = DependencyGraph::default();
        unsupported
            .expand(&resources, BTreeSet::from([1]), 16, 16)
            .unwrap();
        assert!(
            unsupported.effect_programs[&("active.bin".into(), 12)]
                .as_ref()
                .unwrap_err()
                .contains("unsupported")
        );
        assert!(unsupported.effect_references[&("active.bin".into(), 12)].is_err());
        assert!(unsupported.effect_edges.is_empty());
    }
    #[test]
    fn partial_graph_retains_missing_inputs_cycles_and_unknown_placement_classes() {
        let directory = std::env::temp_dir().join(format!(
            "giants-graph-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        std::fs::create_dir(directory.join("Bin")).unwrap();
        let resources = SceneResources {
            world_bounds: Default::default(),
            classes: crate::catalog_registry::WorldClassRegistry {
                visits: Vec::new(),
                classes: Default::default(),
                textures: Default::default(),
                models: Default::default(),
            },
            archives: Default::default(),
            language: "English".into(),
        };
        let missing = DependencyGraph::build(&directory, &resources, "empty", "loop").unwrap();
        assert_eq!(missing.diagnostics.len(), 2);
        let mut script = vec![0; 128];
        for (at, value) in [
            (0, 3u32),
            (4, 128),
            (8, 1),
            (12, 1),
            (20, 96),
            (24, 1),
            (28, 4),
            (36, 112),
            (40, 2),
            (44, 6),
            (60, u32::MAX),
            (64, 31),
            (68, 2),
            (72, 5),
            (76, 2),
        ] {
            script[at..at + 4].copy_from_slice(&value.to_le_bytes());
        }
        script[96..101].copy_from_slice(b"loop\0");
        std::fs::write(directory.join("Bin/flk_loop.bin"), script).unwrap();
        let mut mission = 4u32.to_le_bytes().to_vec();
        mission.extend([1, 3]);
        mission.extend((u32::MAX - 1).to_le_bytes());
        mission.extend([0; 16]);
        mission.push(2);
        std::fs::write(directory.join("Bin/wm_empty.bin"), &mission).unwrap();
        let graph = DependencyGraph::build(&directory, &resources, "empty", "loop").unwrap();
        assert_eq!(graph.scripts.len(), 1);
        assert_eq!(graph.placements.len(), 1);
        assert_eq!(graph.origins.len(), 2);
        assert!(graph.diagnostics.iter().any(|v| v.contains("cycle")));
        assert!(graph.classes[&-2].is_err());
        assert!(!graph.classes.contains_key(&-1));
        let mut nested = mission.clone();
        nested.pop();
        let child_offset = nested.len();
        nested.push(22);
        nested.extend((u32::MAX - 2).to_le_bytes());
        nested.extend([7, 255, 23, 2]);
        std::fs::write(directory.join("Bin/wm_empty.bin"), nested).unwrap();
        let children = DependencyGraph::build(&directory, &resources, "empty", "loop").unwrap();
        assert_eq!(children.placements.len(), 1);
        assert_eq!(
            children.mission_children[0].parent,
            mission::ObjectRef::Placement(0)
        );
        assert!(children.classes[&-3].is_err());
        assert!(
            children
                .origins
                .iter()
                .any(|o| o.kind == "mission_locked_child"
                    && o.class_id == -3
                    && o.offset == child_offset)
        );
        std::fs::write(directory.join("Bin/wm_empty.bin"), mission).unwrap();
        // NULL geometry is valid, while a declared absent animation is an
        // explicit required dependency failure, never a basename guess.
        let mut resources = resources;
        resources.classes.models.selections.insert(
            -2,
            vec![Ok(crate::model_registry::SelectionBinding {
                class_catalog: "synthetic.bin".into(),
                spec_id: 7,
                spec_catalog: "synthetic.bin".into(),
                selection: crate::model_registry::SelectionSpec {
                    slots: vec![crate::model_registry::SelectionSlot {
                        opcode: 0x51,
                        reference: Some(0),
                        name: None,
                        threshold: 0.,
                    }],
                    animations: vec![crate::model_registry::DeclaredAnimation {
                        slot: 1,
                        name: "absent.anm".into(),
                    }],
                    effects: vec![(0x7f, vec!["symbol".into()])],
                    non_model_commands: vec![0x52, 0x7f],
                    diagnostics: vec![],
                },
            })],
        );
        let declared = DependencyGraph::build(&directory, &resources, "empty", "loop").unwrap();
        assert!(declared.classes[&-2].as_ref().unwrap()[0].error.is_none());
        assert!(declared.resources["absent.anm"].required);
        assert!(declared.resources["absent.anm"].selected.is_err());
        assert!(declared.animations["absent.anm"].is_err());
        assert!(!declared.resources.contains_key("symbol"));
        assert_eq!(
            declared.specifications[&-2].selection.non_model_commands,
            [0x52, 0x7f]
        );
        // A state specification must contribute dependencies even when the
        // class's primary model cannot be resolved. A failed temporal binding
        // is retained and must not be healed by a present final registry entry.
        let selection = resources.classes.models.selections[&-2][0]
            .as_ref()
            .unwrap()
            .selection
            .clone();
        resources.classes.models.selections.clear();
        resources.classes.models.specs.insert(
            20,
            vec![crate::model_registry::SpecDefinition {
                catalog: "state.bin".into(),
                effect_namespace: None,
                models: Ok(vec![]),
                program: Err("not needed for this dependency control".into()),
                selection: Ok(selection),
            }],
        );
        resources.classes.models.programs.insert(
            -2,
            vec![crate::model_registry::ClassProgram {
                catalog: "class.bin".into(),
                program: Err("not needed for this dependency control".into()),
                bindings: Ok(vec![
                    crate::model_registry::StateBinding {
                        offset: 108,
                        key: crate::state_storage::ReferenceKey::Spec(20),
                        target: Ok(crate::model_registry::StateTarget::Spec {
                            id: 20,
                            catalog: "state.bin".into(),
                        }),
                    },
                    crate::model_registry::StateBinding {
                        offset: 112,
                        key: crate::state_storage::ReferenceKey::Spec(20),
                        target: Err("missing at registration".into()),
                    },
                ]),
            }],
        );
        let state = DependencyGraph::build(&directory, &resources, "empty", "loop").unwrap();
        assert!(state.classes[&-2].is_err());
        assert_eq!(state.state_specifications.len(), 1);
        assert_eq!(
            state.state_specifications[&(20, "state.bin".into())]
                .as_ref()
                .unwrap()
                .models
                .len(),
            1
        );
        assert!(state.resources["absent.anm"].required);
        assert!(state.animations["absent.anm"].is_err());
        assert!(
            state.state_bindings[&-2].as_ref().unwrap()[1]
                .target
                .is_err()
        );
        if let Ok(crate::model_registry::StateTarget::Spec { catalog, .. }) =
            &mut resources.classes.models.programs.get_mut(&-2).unwrap()[0]
                .bindings
                .as_mut()
                .unwrap()[0]
                .target
        {
            *catalog = "incorrect.bin".into();
        }
        let mismatch = DependencyGraph::build(&directory, &resources, "empty", "loop").unwrap();
        assert!(mismatch.state_specifications[&(20, "incorrect.bin".into())].is_err());
        assert!(!mismatch.resources.contains_key("absent.anm"));
        assert!(DependencyGraph::build(&directory, &resources, "../empty", "loop").is_err());
        std::fs::remove_file(directory.join("Bin/flk_loop.bin")).unwrap();
        std::fs::remove_file(directory.join("Bin/wm_empty.bin")).unwrap();
        std::fs::remove_dir(directory.join("Bin")).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }
}
