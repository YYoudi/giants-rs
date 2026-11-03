//! Per-placement source state and explicitly adjusted geometry. No object callbacks.
use crate::{
    arithmetic::Precision,
    ground_contact,
    scene_resources::{ModelAsset, SceneResources, SourcePlacement, TextureEncoding},
};
use giants_formats::mission::Placement;
use std::sync::Arc;

pub struct Planned {
    context: Arc<SceneResources>,
    pub index: usize,
    pub serialized: Placement,
    pub source: Result<SourcePlacement, String>,
}
/// World declarations share class metadata and state templates. A first
/// constructor pose is distinct from geometry after world modifiers/callbacks.
pub struct WorldPlanned {
    pub declaration: giants_formats::world::WorldPlacement,
    metadata: Planned,
}
impl WorldPlanned {
    pub fn resources(&self) -> &SceneResources {
        &self.metadata.context
    }
    /// Source projection for class/state/resource diagnostics only. In
    /// particular its yaw-only transform must not replace the world declaration.
    pub fn class_input(&self) -> Result<&SourcePlacement, String> {
        self.metadata.source.as_ref().map_err(Clone::clone)
    }
    pub fn declared_models(&self) -> Result<&crate::model_registry::SelectionBinding, String> {
        self.metadata.declared_models()
    }
    /// First constructor pose for the zeroed class-angle profile. Source class
    /// angle defaults and copy/override stores are independently controlled.
    /// World modifiers, callbacks, contact and live animation remain separate.
    pub fn initial_matrix(&self) -> Result<[f32; 16], String> {
        self.authored_matrix([0., 0.])
    }
    /// Declared pose plus explicit inherited angles. This does not run world
    /// modifiers, contact, callbacks, animation or camera-facing construction.
    pub fn authored_matrix(&self, inherited_pitch_roll: [f32; 2]) -> Result<[f32; 16], String> {
        let [pitch, roll] = self.declaration.pitch_roll.unwrap_or(inherited_pitch_roll);
        crate::object_transform::euler(
            self.declaration.position,
            [self.declaration.yaw, pitch, roll],
            self.class_input()?.scale,
        )
        .map_err(String::from)
    }
}
pub struct Plan {
    resources: Arc<SceneResources>,
    pub entries: Vec<Planned>,
    pub world_entries: Vec<WorldPlanned>,
    pub mission_state_results: Vec<MissionWriteResult>,
    pub mission_object_results: Vec<MissionWriteResult>,
    pub mission_exclusion_results: Vec<MissionWriteResult>,
    pub marker_buoys: Result<Vec<marker_buoys::Spawn>, String>,
    mission_state_writes: Vec<giants_formats::mission::StateWrite>,
    templates: std::collections::BTreeMap<u32, Result<crate::state_storage::Seed, String>>,
}
pub mod marker_buoys;
pub mod race_laps;
pub struct MissionWriteResult {
    pub source_offset: usize,
    pub object: giants_formats::mission::ObjectRef,
    pub result: Result<(), String>,
}
impl Plan {
    pub fn resources(&self) -> &Arc<SceneResources> {
        &self.resources
    }
    /// Load the already demonstrated first mission slot. This is not general
    /// campaign selection or a replacement for the data-driven prepare API.
    pub fn load_first(
        root: &std::path::Path,
        world: &crate::WorldAsset,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mission = crate::props::first_mission(root, world)?;
        let context = Arc::new(SceneResources::load(root, &world.catalog, "English")?);
        Ok(Self::prepare_scene(
            &context,
            &mission,
            &world.config.placements,
        ))
    }
    /// Shared mission/world declarations, preserving their different pose contracts.
    pub fn prepare_scene(
        resources: &Arc<SceneResources>,
        mission: &giants_formats::mission::Mission,
        world_declarations: &[giants_formats::world::WorldPlacement],
    ) -> Self {
        let mut all = mission.placements.clone();
        all.extend(world_declarations.iter().map(|p| Placement {
            source_offset: p.source_offset,
            object_type: p.object_type,
            position: p.position,
            yaw: p.yaw,
            pitch_roll: None,
            scale: p.scale,
        }));
        let mut plan = Self::prepare(resources, &all);
        plan.world_entries = plan
            .entries
            .split_off(mission.placements.len())
            .into_iter()
            .zip(world_declarations.iter().cloned())
            .map(|(mut metadata, declaration)| {
                if let Ok(source) = &mut metadata.source {
                    source
                        .cinematic_exclusions
                        .clone_from(&declaration.cinematic_exclusions);
                }
                WorldPlanned {
                    metadata,
                    declaration,
                }
            })
            .collect();
        plan.apply_mission_state(&mission.state_writes);
        plan.apply_mission_properties(&mission.object_properties);
        plan.apply_mission_header(&mission.object_selectors, &mission.object_words);
        plan.apply_mission_exclusions(&mission.cinematic_exclusions);
        plan.marker_buoys = marker_buoys::from_mission(mission);
        plan
    }
    /// Preserve every source placement and each error. No deduplication of objects
    /// sharing a class, and no geometry, final transform or visibility is assumed.
    pub fn prepare(resources: &Arc<SceneResources>, placements: &[Placement]) -> Self {
        let mut templates = std::collections::BTreeMap::new();
        let entries = placements
            .iter()
            .enumerate()
            .map(|(index, p)| Planned {
                context: Arc::clone(resources),
                index,
                serialized: p.clone(),
                source: resources
                    .placement_input_with_state(p, |program| {
                        templates
                            .entry(p.object_type)
                            .or_insert_with(|| crate::state_storage::Seed::from_program(program))
                            .as_ref()
                            .map_err(Clone::clone)?
                            .instantiate()
                    })
                    .map_err(|e| e.to_string()),
            })
            .collect();
        Self {
            resources: Arc::clone(resources),
            entries,
            world_entries: Vec::new(),
            mission_state_results: Vec::new(),
            mission_object_results: Vec::new(),
            mission_exclusion_results: Vec::new(),
            marker_buoys: Ok(Vec::new()),
            mission_state_writes: Vec::new(),
            templates,
        }
    }
    pub fn prepare_mission(
        resources: &Arc<SceneResources>,
        mission: &giants_formats::mission::Mission,
    ) -> Self {
        let mut plan = Self::prepare(resources, &mission.placements);
        plan.apply_mission_state(&mission.state_writes);
        plan.apply_mission_properties(&mission.object_properties);
        plan.apply_mission_header(&mission.object_selectors, &mission.object_words);
        plan.apply_mission_exclusions(&mission.cinematic_exclusions);
        plan.marker_buoys = marker_buoys::from_mission(mission);
        plan
    }
    /// Apply ordered writes only to actual per-instance storage. A child has no
    /// instance here yet, so it retains an explicit error rather than using its parent.
    fn apply_mission_state(&mut self, writes: &[giants_formats::mission::StateWrite]) {
        self.mission_state_writes.extend_from_slice(writes);
        use giants_formats::mission::ObjectRef;
        for write in writes {
            let result = (|| {
                let ObjectRef::Placement(index) = write.object else {
                    return Err("mission child state has not been instantiated".into());
                };
                let entry = self
                    .entries
                    .get_mut(index)
                    .ok_or("mission state owner missing")?;
                let source = entry.source.as_mut().map_err(|e| e.clone())?;
                let seed = source.state_seed.as_mut().map_err(|e| e.clone())?;
                let ranges = seed.apply_mission_write(&write.kind)?;
                if let Ok(bindings) = source.state_bindings.as_mut() {
                    bindings.retain(|b| {
                        ranges
                            .iter()
                            .all(|range| b.offset + 4 <= range.start || b.offset >= range.end)
                    });
                }
                Ok(())
            })();
            self.mission_state_results.push(MissionWriteResult {
                source_offset: write.source_offset,
                object: write.object,
                result,
            });
        }
    }
    fn apply_mission_properties(&mut self, properties: &[giants_formats::mission::ObjectProperty]) {
        use giants_formats::mission::ObjectRef;
        for property in properties {
            let result = (|| {
                let ObjectRef::Placement(index) = property.object else {
                    return Err("mission child property has not been instantiated".into());
                };
                let entry = self
                    .entries
                    .get_mut(index)
                    .ok_or("mission property owner missing")?;
                let source = entry.source.as_mut().map_err(|e| e.clone())?;
                source.mission_word_174 = Some(property.value);
                Ok(())
            })();
            self.mission_object_results.push(MissionWriteResult {
                source_offset: property.source_offset,
                object: property.object,
                result,
            });
        }
    }
    fn apply_mission_exclusions(
        &mut self,
        exclusions: &[giants_formats::mission::CinematicExclusion],
    ) {
        for exclusion in exclusions {
            let result = (|| {
                let giants_formats::mission::ObjectRef::Placement(index) = exclusion.object else {
                    return Err("mission child exclusion has not been instantiated".into());
                };
                let source = self
                    .entries
                    .get_mut(index)
                    .ok_or("mission exclusion owner missing")?
                    .source
                    .as_mut()
                    .map_err(|e| e.clone())?;
                source.cinematic_exclusions.push(exclusion.name);
                Ok(())
            })();
            self.mission_exclusion_results.push(MissionWriteResult {
                source_offset: exclusion.source_offset,
                object: exclusion.object,
                result,
            });
        }
    }
    fn apply_mission_header(
        &mut self,
        selectors: &[giants_formats::mission::ObjectSelector],
        words: &[giants_formats::mission::ObjectWords],
    ) {
        use giants_formats::mission::ObjectRef;
        for (offset, object, selector, values) in selectors
            .iter()
            .map(|s| (s.source_offset, s.object, Some(s.value), None))
            .chain(
                words
                    .iter()
                    .map(|w| (w.source_offset, w.object, None, Some(w.words))),
            )
        {
            let result = (|| {
                let ObjectRef::Placement(index) = object else {
                    return Err("mission child header owner has not been instantiated".into());
                };
                let source = self
                    .entries
                    .get_mut(index)
                    .ok_or("mission header owner missing")?
                    .source
                    .as_mut()
                    .map_err(|e| e.clone())?;
                if let Some(value) = selector {
                    source.mission_selector = Some(value);
                }
                if let Some(value) = values {
                    source.mission_values_f4 = Some(value);
                }
                Ok(())
            })();
            self.mission_object_results.push(MissionWriteResult {
                source_offset: offset,
                object,
                result,
            });
        }
        self.mission_object_results.sort_by_key(|r| r.source_offset);
    }
    pub fn state_templates(
        &self,
    ) -> &std::collections::BTreeMap<u32, Result<crate::state_storage::Seed, String>> {
        &self.templates
    }
    /// Preserve input commands for fresh Debug payload initialization.
    pub fn mission_state_writes(&self) -> &[giants_formats::mission::StateWrite] {
        &self.mission_state_writes
    }
    pub fn unique_class(&self, class: u32) -> Result<&Planned, String> {
        let mut entries = self
            .entries
            .iter()
            .filter(|p| p.serialized.object_type == class);
        let result = entries
            .next()
            .ok_or_else(|| format!("instance class {class} missing"))?;
        if entries.next().is_some() {
            return Err(format!("instance class {class} is not unique"));
        }
        Ok(result)
    }
}
/// Explicit caller profile. Authored preserves source coordinates; it does not
/// establish that callbacks or collision would leave them unchanged.
pub enum Adjustment {
    Authored,
    Contact {
        ground: f32,
        plane: [f32; 4],
        platform: Option<f32>,
        vertical_velocity: f32,
        flags: u32,
        lock_z: bool,
        precision: Precision,
    },
}
pub struct GeometryInstance {
    pub model: Option<ModelAsset>,
    pub position: [f32; 3],
    pub matrix: [f32; 16],
    pub contact: Option<ground_contact::Contact>,
}
pub struct PlacementTransform {
    pub position: [f32; 3],
    pub matrix: [f32; 16],
    pub contact: Option<ground_contact::Contact>,
}
impl Planned {
    pub fn resources(&self) -> &SceneResources {
        &self.context
    }
    /// Native prepass uses primary-model bounds, independently of selected LOD.
    /// Source bounds do not incorporate later skin or physics changes.
    pub fn primary_bounds(&self) -> Result<[f32; 6], Box<dyn std::error::Error>> {
        let source = self.source.as_ref().map_err(Clone::clone)?;
        let (_, mesh) = self
            .context
            .model_mesh_slot(source.class, 0)?
            .ok_or("primary model has no bounds")?;
        Ok(mesh.bounds()?)
    }
    /// Initial class-field projection only; not a visible or active-object test.
    pub fn projected_membership(&self) -> Result<u32, String> {
        let source = self.source.as_ref().map_err(Clone::clone)?;
        let state = source
            .state_seed
            .as_ref()
            .map_err(Clone::clone)?
            .storage
            .state();
        Ok(crate::object_lists::membership(
            source.class,
            state.into(),
            source.direct_flags,
        ))
    }
    pub fn declared_models(&self) -> Result<&crate::model_registry::SelectionBinding, String> {
        let source = self.source.as_ref().map_err(Clone::clone)?;
        self.context
            .classes
            .models
            .first_selection(source.class)
            .ok_or("class model declaration missing".to_string())?
            .as_ref()
            .map_err(Clone::clone)
    }
    pub fn transform(&self, adjustment: Adjustment) -> Result<PlacementTransform, String> {
        let source = self.source.as_ref().map_err(Clone::clone)?;
        let mut position = source.position;
        let contact = match adjustment {
            Adjustment::Authored => None,
            Adjustment::Contact {
                ground,
                plane,
                platform,
                vertical_velocity,
                flags,
                lock_z,
                precision,
            } => {
                let contact = ground_contact::resolve(
                    ground_contact::Input {
                        z: position[2],
                        vertical_velocity,
                        flags,
                        lock_z,
                        ground,
                        plane,
                        vertical_offset: *source
                            .declared_ground_offset
                            .as_ref()
                            .map_err(Clone::clone)?,
                        platform,
                    },
                    precision,
                )?;
                position[2] = contact.z;
                Some(contact)
            }
        };
        let matrix = crate::object_transform::euler(position, source.angles(), source.scale)?;
        Ok(PlacementTransform {
            position,
            matrix,
            contact,
        })
    }
    fn static_ground_adjustment(
        &self,
        world: &crate::collision_world::StaticGroundWorld<'_>,
        motion: crate::collision_world::ContactMotion,
    ) -> Result<Adjustment, String> {
        let source = self.source.as_ref().map_err(Clone::clone)?;
        let sample = world.sample(source.position)?;
        Ok(Adjustment::Contact {
            ground: sample.terrain.height,
            plane: sample.terrain.plane,
            platform: sample.platform.object.map(|_| sample.platform.height),
            vertical_velocity: motion.vertical_velocity,
            flags: motion.flags,
            lock_z: motion.lock_z,
            precision: world.precision(),
        })
    }
    /// Source placement plus measured static queries/final contact. This does not
    /// execute class callbacks, movement, lateral collisions or slope response.
    pub fn transform_on_static_ground(
        &self,
        world: &crate::collision_world::StaticGroundWorld<'_>,
        motion: crate::collision_world::ContactMotion,
    ) -> Result<PlacementTransform, String> {
        self.transform(self.static_ground_adjustment(world, motion)?)
    }
    /// Shared geometry path, explicit model slot and already assembled colliders.
    pub fn geometry_on_static_ground(
        &self,
        slot: usize,
        encoding: TextureEncoding,
        world: &crate::collision_world::StaticGroundWorld<'_>,
        motion: crate::collision_world::ContactMotion,
    ) -> Result<GeometryInstance, Box<dyn std::error::Error>> {
        self.geometry(
            slot,
            encoding,
            self.static_ground_adjustment(world, motion)?,
        )
    }
    /// Slot and texture encoding are supplied by an already measured consumer.
    /// Empty geometry remains valid absence; no LOD/renderer policy is invented.
    pub fn geometry(
        &self,
        slot: usize,
        encoding: TextureEncoding,
        adjustment: Adjustment,
    ) -> Result<GeometryInstance, Box<dyn std::error::Error>> {
        let source = self.source.as_ref().map_err(Clone::clone)?;
        let PlacementTransform {
            position,
            matrix,
            contact,
        } = self.transform(adjustment)?;
        let model = self
            .context
            .model_slot_with_encoding(source.class, slot, encoding)?;
        Ok(GeometryInstance {
            model,
            position,
            matrix,
            contact,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repeated_class_objects_have_independent_state_and_errors_preserve_following_entries() {
        let mut models = crate::model_registry::ModelRegistry::default();
        models.programs.insert(
            7,
            vec![crate::model_registry::ClassProgram {
                catalog: "synthetic.bin".into(),
                bindings: Ok(vec![]),
                program: Ok(giants_formats::class_program::parse(&[0x1e, 4, 0xff]).unwrap()),
            }],
        );
        let mut resources = Arc::new(SceneResources {
            world_bounds: Default::default(),
            classes: crate::catalog_registry::WorldClassRegistry {
                visits: vec![],
                classes: Default::default(),
                models,
                textures: Default::default(),
            },
            archives: Default::default(),
            language: "English".into(),
        });
        let placement = Placement {
            source_offset: 4,
            object_type: 7,
            position: [1., 2., 3.],
            yaw: 0.,
            pitch_roll: None,
            scale: Some(1.),
        };
        let mut missing = placement.clone();
        missing.object_type = 8;
        missing.source_offset = 8;
        let mut plan = Plan::prepare(&resources, &[placement.clone(), missing, placement]);
        assert!(Arc::get_mut(&mut resources).is_none());
        assert_eq!(plan.entries.len(), 3);
        assert!(plan.entries[1].source.is_err());
        assert!(plan.entries[2].source.is_ok());
        assert_eq!(plan.state_templates().len(), 1);
        assert!(plan.unique_class(7).is_err());
        plan.entries[0]
            .source
            .as_mut()
            .unwrap()
            .state_seed
            .as_mut()
            .unwrap()
            .storage
            .data_mut()
            .unwrap()[0] = 99;
        assert_eq!(
            plan.entries[2]
                .source
                .as_ref()
                .unwrap()
                .state_seed
                .as_ref()
                .unwrap()
                .storage
                .data()
                .unwrap()[0],
            0
        );
        assert_eq!(
            plan.state_templates()[&7]
                .as_ref()
                .unwrap()
                .storage
                .data()
                .unwrap()[0],
            0
        );
        let authored = plan.entries[2].transform(Adjustment::Authored).unwrap();
        assert_eq!(authored.position, [1., 2., 3.]);
        assert!(authored.contact.is_none());
        let adjusted = plan.entries[2]
            .transform(Adjustment::Contact {
                ground: 3.05,
                plane: [0., 0., 1., -3.05],
                platform: None,
                vertical_velocity: 0.,
                flags: 0,
                lock_z: false,
                precision: Precision::Double53,
            })
            .unwrap();
        assert_eq!(adjusted.position, [1., 2., 3.05]);
        assert!(adjusted.contact.is_some());
        use giants_formats::mission::{ObjectRef, StateWrite, StateWriteKind};
        let source = plan.entries[0].source.as_mut().unwrap();
        let seed = source.state_seed.as_mut().unwrap();
        seed.unresolved = [0, 80, 88]
            .map(|offset| crate::state_storage::Reference {
                offset,
                key: crate::state_storage::ReferenceKey::Spec(123),
            })
            .to_vec();
        source.state_bindings = Ok([0, 80, 88]
            .map(|offset| crate::model_registry::StateBinding {
                offset,
                key: crate::state_storage::ReferenceKey::Spec(123),
                target: Err("synthetic unresolved class reference".into()),
            })
            .to_vec());
        plan.apply_mission_state(&[
            StateWrite {
                source_offset: 20,
                object: ObjectRef::Placement(0),
                kind: StateWriteKind::CountedWords {
                    list: giants_formats::mission::StateList::First,
                    words: vec![7],
                },
            },
            StateWrite {
                source_offset: 30,
                object: ObjectRef::LockedChild(0),
                kind: StateWriteKind::Word(12),
            },
            StateWrite {
                source_offset: 40,
                object: ObjectRef::Placement(1),
                kind: StateWriteKind::Word(13),
            },
        ]);
        assert!(plan.mission_state_results[0].result.is_ok());
        assert!(plan.mission_state_results[1].result.is_err());
        assert!(plan.mission_state_results[2].result.is_err());
        let source = plan.entries[0].source.as_ref().unwrap();
        let seed = source.state_seed.as_ref().unwrap();
        assert_eq!(
            seed.unresolved.iter().map(|r| r.offset).collect::<Vec<_>>(),
            [88]
        );
        assert_eq!(
            source
                .state_bindings
                .as_ref()
                .unwrap()
                .iter()
                .map(|r| r.offset)
                .collect::<Vec<_>>(),
            [88]
        );
        assert_eq!(
            &seed.storage.data().unwrap()[..8],
            &[1, 0, 0, 0, 7, 0, 0, 0]
        );
        assert_eq!(
            &plan.entries[2]
                .source
                .as_ref()
                .unwrap()
                .state_seed
                .as_ref()
                .unwrap()
                .storage
                .data()
                .unwrap()[..8],
            &[0; 8]
        );
        assert_eq!(
            &plan.state_templates()[&7]
                .as_ref()
                .unwrap()
                .storage
                .data()
                .unwrap()[..8],
            &[0; 8]
        );
        use giants_formats::mission::ObjectProperty;
        plan.apply_mission_properties(&[
            ObjectProperty {
                source_offset: 50,
                object: ObjectRef::Placement(0),
                value: 0,
            },
            ObjectProperty {
                source_offset: 60,
                object: ObjectRef::LockedChild(0),
                value: 8,
            },
            ObjectProperty {
                source_offset: 70,
                object: ObjectRef::Placement(0),
                value: u32::MAX,
            },
        ]);
        assert!(plan.mission_object_results[0].result.is_ok());
        assert!(plan.mission_object_results[1].result.is_err());
        assert!(plan.mission_object_results[2].result.is_ok());
        use giants_formats::mission::CinematicExclusion;
        plan.apply_mission_exclusions(&[
            CinematicExclusion {
                source_offset: 70,
                object: ObjectRef::Placement(0),
                name: [b'A'; 32],
            },
            CinematicExclusion {
                source_offset: 71,
                object: ObjectRef::LockedChild(0),
                name: [b'B'; 32],
            },
            CinematicExclusion {
                source_offset: 72,
                object: ObjectRef::Placement(2),
                name: [b'C'; 32],
            },
        ]);
        assert!(plan.mission_exclusion_results[0].result.is_ok());
        assert!(plan.mission_exclusion_results[1].result.is_err());
        assert!(plan.mission_exclusion_results[2].result.is_ok());
        assert_eq!(
            plan.entries[0]
                .source
                .as_ref()
                .unwrap()
                .cinematic_exclusions,
            [[b'A'; 32]]
        );
        assert_eq!(
            plan.entries[2]
                .source
                .as_ref()
                .unwrap()
                .cinematic_exclusions,
            [[b'C'; 32]]
        );
        assert_eq!(
            plan.entries[0].source.as_ref().unwrap().mission_word_174,
            Some(u32::MAX)
        );
        assert_eq!(
            plan.entries[2].source.as_ref().unwrap().mission_word_174,
            None
        );
        use giants_formats::mission::{ObjectSelector, ObjectWords};
        plan.apply_mission_header(
            &[
                ObjectSelector {
                    source_offset: 80,
                    object: ObjectRef::Placement(0),
                    value: 255,
                },
                ObjectSelector {
                    source_offset: 90,
                    object: ObjectRef::LockedChild(0),
                    value: 7,
                },
                ObjectSelector {
                    source_offset: 100,
                    object: ObjectRef::Placement(0),
                    value: 0,
                },
            ],
            &[
                ObjectWords {
                    source_offset: 110,
                    object: ObjectRef::Placement(0),
                    words: [0, 0x80000000, 0x41a00000],
                },
                ObjectWords {
                    source_offset: 120,
                    object: ObjectRef::LockedChild(0),
                    words: [1; 3],
                },
            ],
        );
        assert_eq!(
            plan.entries[0].source.as_ref().unwrap().mission_selector,
            Some(0)
        );
        assert_eq!(
            plan.entries[2].source.as_ref().unwrap().mission_selector,
            None
        );
        assert_eq!(
            plan.entries[0].source.as_ref().unwrap().mission_values_f4,
            Some([0, 0x80000000, 0x41a00000])
        );
        assert_eq!(
            plan.entries[2].source.as_ref().unwrap().mission_values_f4,
            None
        );
        assert!(
            plan.mission_object_results
                .iter()
                .filter(|r| r.source_offset == 90 || r.source_offset == 120)
                .all(|r| r.result.is_err())
        );
    }
}
