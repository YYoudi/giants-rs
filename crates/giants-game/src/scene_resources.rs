//! Common source loading for registered classes and Bin-tier resources.
//! Model slots are explicit until native LOD/visibility selection is established.
use crate::catalog_registry::{ResourceIndex, WorldClassRegistry};
use std::{collections::BTreeSet, path::Path};

pub struct SceneResources {
    pub world_bounds: giants_formats::world_bounds::Table,
    pub classes: WorldClassRegistry,
    pub archives: ResourceIndex,
    pub language: String,
}
pub struct ModelAsset {
    pub name: String,
    pub mesh: giants_formats::gbs::Mesh,
    pub textures: Vec<crate::LogoTexture>,
    pub alpha_textures: BTreeSet<String>,
}
/// Decoded CPU resources and provenance, before texture upload or actor creation.
pub struct CpuModel {
    pub name: String,
    pub geometry_source: crate::catalog_registry::Location,
    pub skeleton_source: Option<crate::catalog_registry::Location>,
    pub prepared: crate::model_loading::Prepared,
}
/// Dedicated collision selection and its optional decoded geometry.
pub struct CollisionModel {
    pub slot: usize,
    pub geometry: Option<(String, giants_formats::gbs::Mesh)>,
}
/// Initial authored, unbound pose. No live animation/contact/callback is applied.
pub struct AuthoredCollisionGeometry {
    pub slot: usize,
    pub name: String,
    pub mesh: giants_formats::gbs::Mesh,
    pub matrix: [f32; 16],
    pub triangles: Vec<[[f32; 3]; 3]>,
}
/// A declared image request survives absent files and unsupported decoders.
/// A decoded image is not an upload or a rendering capability.
pub struct ImageRequest {
    pub member: String,
    pub selected: Option<crate::catalog_registry::Location>,
    pub decoded: Result<giants_formats::tga::Image, String>,
}
impl ModelAsset {
    /// Categories for the explicitly chosen CPU upload profile. This does not
    /// infer live descriptor formats, object modifiers or queue membership.
    pub fn material_passes(&self) -> Vec<crate::render_parts::Pass> {
        self.mesh
            .subobjects
            .iter()
            .map(|part| {
                crate::render_parts::material_pass(
                    part.material.falloff,
                    self.alpha_textures
                        .contains(&part.material.texture.to_ascii_lowercase()),
                )
            })
            .collect()
    }
}
/// Explicit measured upload profile, not an inferred material/renderer policy.
#[derive(Clone, Copy)]
pub enum TextureEncoding {
    Rgb565,
    Rgb565OrArgb4444,
}
/// Serialized placement plus ordered direct class scale declarations. Runtime
/// state callbacks, collision corrections and animation bindings are not applied.
#[derive(Debug)]
pub struct SourcePlacement {
    pub source_offset: usize,
    pub class: i32,
    pub position: [f32; 3],
    pub yaw: f32,
    pub pitch_roll: Option<[f32; 2]>,
    /// Explicit mission command 7 override; None does not imply a zero default.
    pub mission_word_174: Option<u32>,
    /// Last mission command5 byte; absence retains the constructor default.
    pub mission_selector: Option<u8>,
    /// Last mission command6 raw header words; consumers decide their meaning.
    pub mission_values_f4: Option<[u32; 3]>,
    /// Declared exclusions only; live editor changes and inherited masks are separate.
    pub cinematic_exclusions: Vec<[u8; 32]>,
    pub scale: f32,
    pub class_catalog: std::path::PathBuf,
    pub class_commands: Vec<u8>,
    pub direct_flags: crate::object_lists::DirectFlags,
    pub declared_ground_offset: Result<f32, String>,
    pub state_seed: Result<crate::state_storage::Seed, String>,
    pub state_bindings: Result<Vec<crate::model_registry::StateBinding>, String>,
}
impl SourcePlacement {
    pub fn unadjusted_matrix(&self) -> Result<[f32; 16], &'static str> {
        crate::object_transform::euler(self.position, self.angles(), self.scale)
    }
    pub fn angles(&self) -> [f32; 3] {
        let [pitch, roll] = self.pitch_roll.unwrap_or([0., 0.]);
        [self.yaw, pitch, roll]
    }
}
impl SceneResources {
    /// Prepared bounds from the world companion. Mesh handles are shared by
    /// global reference ID, so later records for another spec can update the
    /// same model. Animation wrappers remain local to their spec and slot.
    /// This does not execute animation loading or live object callbacks.
    pub fn collision_resource_bounds(
        &self,
        class: i32,
        animation: Option<u8>,
    ) -> Result<giants_formats::world_bounds::Resolved, Box<dyn std::error::Error>> {
        let binding = self
            .classes
            .models
            .first_selection(class)
            .ok_or("bounds class absent")?
            .as_ref()
            .map_err(Clone::clone)?;
        let reference = binding
            .selection
            .slots
            .first()
            .and_then(|s| s.reference)
            .filter(|v| *v != 0)
            .ok_or("primary bounds reference absent")?;
        let (_, mesh) = self
            .model_mesh_slot(class, 0)?
            .ok_or("primary bounds mesh absent")?;
        let mut primary = mesh.bounds()?;
        for record in &self.world_bounds.records {
            let Some(definitions) = self.classes.models.specs.get(&record.spec) else {
                continue;
            };
            let selection = definitions
                .first()
                .ok_or("empty bounds spec")?
                .selection
                .as_ref()
                .map_err(Clone::clone)?;
            if selection
                .slots
                .iter()
                .any(|s| s.name.is_some() && s.reference == Some(reference))
            {
                primary = record.primary;
            }
        }
        let declared =
            animation.filter(|slot| binding.selection.animations.iter().any(|a| a.slot == *slot));
        Ok(giants_formats::world_bounds::Resolved {
            primary,
            animation: declared.map(|slot| {
                self.world_bounds
                    .animation_bounds(binding.spec_id, slot, primary)
            }),
        })
    }
    /// Explicit TGA inspection policy, independent of live DDS/backend choices.
    pub fn image_request(&self, name: &str) -> ImageRequest {
        let member = format!("{name}.tga");
        let selected = self
            .archives
            .select_bin(&member, &self.language)
            .cloned()
            .map_err(|error| error.to_string());
        match selected {
            Err(error) => ImageRequest {
                member,
                selected: None,
                decoded: Err(error),
            },
            Ok(selected) => {
                let decoded = self
                    .archives
                    .read_bin(&member, &self.language)
                    .map_err(|error| error.to_string())
                    .and_then(|bytes| {
                        giants_formats::tga::decode_typed(&bytes).map_err(|error| error.to_string())
                    });
                ImageRequest {
                    member,
                    selected: Some(selected),
                    decoded,
                }
            }
        }
    }
    pub fn load(
        root: &Path,
        catalog: &str,
        language: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let directory = root.join("Bin");
        let classes = WorldClassRegistry::load(&directory, catalog)?;
        let archives = ResourceIndex::scan(&directory)?;
        if !archives.issues.is_empty() {
            return Err("archive index contains unsupported inputs".into());
        }
        let bounds_path = directory.join(catalog).with_extension("abx");
        let world_bounds = match std::fs::read(bounds_path) {
            Ok(data) => giants_formats::world_bounds::parse(&data)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Default::default(),
            Err(e) => return Err(e.into()),
        };
        Ok(Self {
            world_bounds,
            classes,
            archives,
            language: language.into(),
        })
    }
    /// Allocate only declared state storage and inline word payloads. Other
    /// class attributes are retained as unexecuted; this is not a ready instance.
    pub fn class_state_seed(
        &self,
        class: i32,
    ) -> Result<crate::state_storage::Seed, Box<dyn std::error::Error>> {
        let definition = self
            .classes
            .models
            .first_program(class)
            .ok_or("class not registered")?;
        let program = definition
            .program
            .as_ref()
            .map_err(|e| format!("class {class}: {e}"))?;
        Ok(crate::state_storage::Seed::from_program(program)?)
    }
    pub fn class_state_bindings(
        &self,
        class: i32,
    ) -> Result<&[crate::model_registry::StateBinding], String> {
        self.classes
            .models
            .first_program(class)
            .ok_or("class not registered")?
            .bindings
            .as_deref()
            .map_err(Clone::clone)
    }
    /// Reproject offsets for the chosen payload profile, retaining targets from
    /// the original class-registration namespace. Never backfill a formerly
    /// missing target from resources registered later, and never write pointers.
    pub fn class_state_bindings_for(
        &self,
        class: i32,
        profile: crate::state_storage::Profile,
    ) -> Result<Vec<crate::model_registry::StateBinding>, String> {
        let definition = self
            .classes
            .models
            .first_program(class)
            .ok_or("class not registered")?;
        let program = definition.program.as_ref().map_err(Clone::clone)?;
        let seed = crate::state_storage::Seed::from_program_for(
            program,
            profile,
            crate::arithmetic::Precision::Double53,
        )?;
        let captured = definition.bindings.as_ref().map_err(Clone::clone)?;
        Ok(seed
            .unresolved
            .into_iter()
            .map(|reference| {
                let target = captured
                    .iter()
                    .find(|binding| binding.key == reference.key)
                    .map_or_else(
                        || Err("state key absent from captured registration namespace".into()),
                        |binding| binding.target.clone(),
                    );
                crate::model_registry::StateBinding {
                    offset: reference.offset,
                    key: reference.key,
                    target,
                }
            })
            .collect())
    }
    /// Read a captured effect target using the loaded image only. Framing does
    /// not materialize a state pointer or execute any effect command.
    pub fn bound_effect_program(
        &self,
        target: &crate::model_registry::StateTarget,
    ) -> Result<giants_formats::effect_program::Program, String> {
        let crate::model_registry::StateTarget::Effect {
            image,
            entry,
            offset,
        } = target
        else {
            return Err("target is not an effect program".into());
        };
        let definition = self
            .classes
            .models
            .effects
            .last()
            .ok_or("no active effect image")?;
        if definition.path != *image {
            return Err("effect image replacement is not modeled by this static context".into());
        }
        let symbol = definition
            .image
            .entries
            .get(*entry)
            .ok_or("bound effect entry outside image")?;
        if symbol.offset != *offset {
            return Err("bound effect offset differs from symbol".into());
        }
        definition.image.program(*offset).map_err(|e| e.to_string())
    }
    /// Resolve in the fixed active image of this loading context. A later
    /// image replacement is not guessed from a previously captured offset.
    pub fn effect_target(
        &self,
        source_image: &std::path::Path,
        name: &str,
    ) -> Result<crate::model_registry::StateTarget, String> {
        let definition = self
            .classes
            .models
            .effects
            .last()
            .ok_or("no active effect image")?;
        if definition.path != source_image {
            return Err("effect image replacement is not modeled by this static context".into());
        }
        let (entry, symbol) = definition
            .image
            .find(name)
            .ok_or_else(|| format!("effect symbol {name} missing in active image"))?;
        Ok(crate::model_registry::StateTarget::Effect {
            image: definition.path.clone(),
            entry,
            offset: symbol.offset,
        })
    }
    pub fn adaptive_effect_target(
        &self,
        source_image: &std::path::Path,
        name: &str,
        prefer_lite: bool,
    ) -> Result<crate::model_registry::StateTarget, String> {
        let definition = self
            .classes
            .models
            .effects
            .last()
            .ok_or("no active effect image")?;
        if definition.path != source_image {
            return Err("effect image replacement is not modeled by this static context".into());
        }
        let (entry, symbol) = definition
            .image
            .adaptive_find(name, prefer_lite)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| {
                format!("adaptive effect {name} has no target for preference {prefer_lite}")
            })?;
        Ok(crate::model_registry::StateTarget::Effect {
            image: definition.path.clone(),
            entry,
            offset: symbol.offset,
        })
    }
    pub fn placement_input(
        &self,
        placement: &giants_formats::mission::Placement,
    ) -> Result<SourcePlacement, Box<dyn std::error::Error>> {
        self.placement_input_with_state(placement, crate::state_storage::Seed::from_program)
    }
    pub(crate) fn placement_input_with_state(
        &self,
        placement: &giants_formats::mission::Placement,
        state: impl FnOnce(
            &giants_formats::class_program::Program,
        ) -> Result<crate::state_storage::Seed, String>,
    ) -> Result<SourcePlacement, Box<dyn std::error::Error>> {
        let class = i32::try_from(placement.object_type)?;
        let definition = self
            .classes
            .models
            .first_program(class)
            .ok_or("placement class not registered")?;
        let program = definition
            .program
            .as_ref()
            .map_err(|e| format!("class {class}: {e}"))?;
        let scale = match placement.scale {
            Some(value) => value,
            None => program.declared_scale()?,
        };
        if !scale.is_finite() || scale <= 0. {
            return Err("invalid source placement scale".into());
        }
        Ok(SourcePlacement {
            source_offset: placement.source_offset,
            class,
            position: placement.position,
            yaw: placement.yaw,
            pitch_roll: placement.pitch_roll,
            mission_word_174: None,
            mission_selector: None,
            mission_values_f4: None,
            cinematic_exclusions: Vec::new(),
            scale,
            class_catalog: definition.catalog.clone(),
            class_commands: program.records.iter().map(|r| r.opcode).collect(),
            direct_flags: crate::object_lists::DirectFlags::project(class, program),
            declared_ground_offset: program.declared_ground_offset().map_err(|e| e.to_string()),
            state_seed: state(program),
            state_bindings: definition.bindings.clone(),
        })
    }
    /// Load an explicitly declared animation filename through the same archive
    /// selection policy. This does not bind a skin or evaluate any pose.
    pub fn animation(
        &self,
        name: &str,
    ) -> Result<giants_formats::anm::Animation, Box<dyn std::error::Error>> {
        let bytes = self.archives.read_bin(name, &self.language)?;
        Ok(giants_formats::anm::parse(&bytes)?)
    }
    /// Explicit dependency-selected profile; unresolved links remain observable
    /// and evaluating one fails. This does not validate every auxiliary node.
    pub fn animation_for_bindings(
        &self,
        name: &str,
    ) -> Result<giants_formats::anm::Animation, Box<dyn std::error::Error>> {
        let bytes = self.archives.read_bin(name, &self.language)?;
        Ok(giants_formats::anm::parse_for_bindings(&bytes)?)
    }
    pub fn model_rgb565(
        &self,
        class: i32,
        slot: usize,
    ) -> Result<ModelAsset, Box<dyn std::error::Error>> {
        self.model_slot_rgb565(class, slot)?
            .ok_or_else(|| "selected model slot is empty".into())
    }
    /// A declared or implicit null slot is absence of geometry, not a filename
    /// to invent. RGB565 conversion is still an explicit caller-selected profile.
    pub fn model_slot_rgb565(
        &self,
        class: i32,
        slot: usize,
    ) -> Result<Option<ModelAsset>, Box<dyn std::error::Error>> {
        self.model_slot_with_encoding(class, slot, TextureEncoding::Rgb565)
    }
    /// Geometry-only read through the registered model/archive resolution path.
    /// Collision geometry uses the dedicated source selector, not render LOD.
    /// An empty selected slot remains empty; no primary-model fallback is invented.
    pub fn collision_model_mesh(
        &self,
        class: i32,
    ) -> Result<CollisionModel, Box<dyn std::error::Error>> {
        let binding = self
            .classes
            .models
            .first_selection(class)
            .ok_or("collision class not registered")?
            .as_ref()
            .map_err(Clone::clone)?;
        let slot = binding.selection.table()?.collision_slot()?;
        Ok(CollisionModel {
            slot,
            geometry: self.model_mesh_slot(class, slot)?,
        })
    }
    /// Geometry-only read through the registered model/archive resolution path.
    /// No texture upload profile or visibility decision is selected here.
    pub fn authored_collision_geometry(
        &self,
        input: &SourcePlacement,
    ) -> Result<Option<AuthoredCollisionGeometry>, Box<dyn std::error::Error>> {
        let selected = self.collision_model_mesh(input.class)?;
        let Some((name, mesh)) = selected.geometry else {
            return Ok(None);
        };
        let matrix = input.unadjusted_matrix()?;
        let pose = crate::collision_world::SourcePose::from_matrix(matrix)?;
        let triangles = crate::collision_geometry::world_triangles(
            &mesh,
            pose,
            crate::arithmetic::Precision::Double53,
        )?;
        Ok(Some(AuthoredCollisionGeometry {
            slot: selected.slot,
            name,
            mesh,
            matrix,
            triangles,
        }))
    }
    /// Geometry-only read through the registered model/archive resolution path.
    /// No texture upload profile or visibility decision is selected here.
    pub fn model_mesh_slot(
        &self,
        class: i32,
        slot: usize,
    ) -> Result<Option<(String, giants_formats::gbs::Mesh)>, Box<dyn std::error::Error>> {
        let binding = self
            .classes
            .models
            .first_selection(class)
            .ok_or("class not registered")?
            .as_ref()
            .map_err(|e| format!("class {class}: {e}"))?;
        self.selection_mesh_slot(&binding.selection, slot)
    }
    /// Use the captured selection itself: state resources need not be the
    /// specification selected for the actor's body. No class/body fallback.
    fn selection_mesh_slot(
        &self,
        selection: &crate::model_registry::SelectionSpec,
        slot: usize,
    ) -> Result<Option<(String, giants_formats::gbs::Mesh)>, Box<dyn std::error::Error>> {
        if slot >= 10 {
            return Err("model slot outside native table".into());
        }
        let Some(reference) = selection.slots.get(slot) else {
            return Ok(None);
        };
        let Some(name) = reference.name.as_ref() else {
            return Ok(None);
        };
        let bytes = self
            .archives
            .read_bin(&format!("{name}.gbs"), &self.language)?;
        let mesh = giants_formats::gbs::parse(&bytes)?;
        Ok(Some((name.clone(), mesh)))
    }
    /// Bin-archive inspection policy. An absent skeleton is a rigid model;
    /// an indexed but unreadable/ambiguous skeleton remains an error.
    pub fn model_cpu_slot(
        &self,
        class: i32,
        slot: usize,
        keep_normals: bool,
    ) -> Result<Option<CpuModel>, Box<dyn std::error::Error>> {
        let binding = self
            .classes
            .models
            .first_selection(class)
            .ok_or("class not registered")?
            .as_ref()
            .map_err(|e| format!("class {class}: {e}"))?;
        self.selection_cpu_slot(&binding.selection, slot, keep_normals)
    }
    pub(crate) fn selection_cpu_slot(
        &self,
        selection: &crate::model_registry::SelectionSpec,
        slot: usize,
        keep_normals: bool,
    ) -> Result<Option<CpuModel>, Box<dyn std::error::Error>> {
        let Some((name, mesh)) = self.selection_mesh_slot(selection, slot)? else {
            return Ok(None);
        };
        let members = crate::model_loading::Members::from_stem(&name)?;
        let geometry_source = self
            .archives
            .select_bin(&members.geometry, &self.language)?
            .clone();
        let skeleton_source = if self
            .archives
            .members
            .contains_key(&members.skeleton.to_ascii_lowercase())
        {
            Some(
                self.archives
                    .select_bin(&members.skeleton, &self.language)?
                    .clone(),
            )
        } else {
            None
        };
        let skin = if skeleton_source.is_some() {
            Some(giants_formats::skn::parse(
                &self.archives.read_bin(&members.skeleton, &self.language)?,
            )?)
        } else {
            None
        };
        let prepared = crate::model_loading::Prepared::from_decoded(mesh, skin, keep_normals)?;
        Ok(Some(CpuModel {
            name,
            geometry_source,
            skeleton_source,
            prepared,
        }))
    }
    pub fn model_slot_with_encoding(
        &self,
        class: i32,
        slot: usize,
        encoding: TextureEncoding,
    ) -> Result<Option<ModelAsset>, Box<dyn std::error::Error>> {
        self.model_slot_texture_profile(class, slot, encoding, false, false)
    }
    /// Explicit static-render capability for source null-texture diffuse parts.
    /// Missing named textures are still errors; no replacement asset is created.
    pub fn model_slot_with_untextured_diffuses(
        &self,
        class: i32,
        slot: usize,
        encoding: TextureEncoding,
    ) -> Result<Option<ModelAsset>, Box<dyn std::error::Error>> {
        self.model_slot_texture_profile(class, slot, encoding, true, false)
    }
    /// Animated bodies additionally admit the measured lit null-base profile.
    /// Normal-less variants and absent named images remain explicit failures.
    pub fn model_slot_for_body(
        &self,
        class: i32,
        slot: usize,
        encoding: TextureEncoding,
    ) -> Result<Option<ModelAsset>, Box<dyn std::error::Error>> {
        self.model_slot_texture_profile(class, slot, encoding, true, true)
    }
    fn model_slot_texture_profile(
        &self,
        class: i32,
        slot: usize,
        encoding: TextureEncoding,
        untextured_diffuses: bool,
        lit_null_base: bool,
    ) -> Result<Option<ModelAsset>, Box<dyn std::error::Error>> {
        let binding = self
            .classes
            .models
            .first_selection(class)
            .ok_or("class not registered")?
            .as_ref()
            .map_err(Clone::clone)?;
        self.selection_texture_profile(
            &binding.selection,
            slot,
            encoding,
            untextured_diffuses,
            lit_null_base,
        )
    }
    pub(crate) fn selection_texture_profile(
        &self,
        selection: &crate::model_registry::SelectionSpec,
        slot: usize,
        encoding: TextureEncoding,
        untextured_diffuses: bool,
        lit_null_base: bool,
    ) -> Result<Option<ModelAsset>, Box<dyn std::error::Error>> {
        let Some((name, mesh)) = self.selection_mesh_slot(selection, slot)? else {
            return Ok(None);
        };
        if untextured_diffuses
            && mesh.subobjects.iter().any(|p| {
                p.material.texture.is_empty()
                    && !crate::body_pipeline::untextured_diffuse(&p.material)
                    && !(lit_null_base
                        && mesh.normal_count != 0
                        && crate::body_pipeline::lit_null_base(&p.material))
            })
        {
            return Err("untextured material outside checked loading profile".into());
        }
        let names: BTreeSet<_> = mesh
            .subobjects
            .iter()
            .map(|p| p.material.texture.clone())
            .filter(|name| !untextured_diffuses || !name.is_empty())
            .collect();
        let mut textures = Vec::new();
        let mut alpha_textures = BTreeSet::new();
        for name in names {
            let bytes = self
                .archives
                .read_bin(&format!("{name}.tga"), &self.language)?;
            let image = giants_formats::tga::decode(&bytes)?;
            let (words, rgba) = if let Some(alpha) = image
                .alpha
                .as_ref()
                .filter(|_| matches!(encoding, TextureEncoding::Rgb565OrArgb4444))
            {
                alpha_textures.insert(name.to_ascii_lowercase());
                let words = giants_formats::pixels::pack_argb4444(&image.pixels, alpha)?;
                let rgba = giants_formats::pixels::rgba8_argb4444(&words);
                (words, rgba)
            } else {
                let words = giants_formats::pixels::pack_rgb565(&image.pixels)?;
                let rgba = giants_formats::pixels::rgba8(&words);
                (words, rgba)
            };
            textures.push(crate::LogoTexture {
                name,
                width: image.width,
                height: image.height,
                words,
                rgba,
            });
        }
        Ok(Some(ModelAsset {
            name,
            mesh,
            textures,
            alpha_textures,
        }))
    }
    /// Common coordinate-based consumer. The caller supplies selection precision;
    /// source thresholds retain the registry's separately checked initialization
    /// policy. Neither policy is inferred from the host renderer or a live game.
    pub fn select_positioned_model_rgb565(
        &self,
        class: i32,
        input: crate::lod::PositionInput,
        precision: crate::lod::Precision,
    ) -> Result<Option<ModelAsset>, Box<dyn std::error::Error>> {
        let table = self.selection_table(class)?;
        let selected = table.select_positioned(input, precision)?;
        let index = usize::try_from(selected)
            .map_err(|_| "negative selection requires explicit renderer policy")?;
        self.model_slot_rgb565(class, index)
    }
    fn selection_table(&self, class: i32) -> Result<crate::lod::Table, Box<dyn std::error::Error>> {
        let binding = self
            .classes
            .models
            .first_selection(class)
            .ok_or("class not registered")?
            .as_ref()
            .map_err(|e| format!("class {class}: {e}"))?;
        if !binding.selection.diagnostics.is_empty() {
            return Err("spec reports a native initialization diagnostic".into());
        }
        Ok(binding.selection.table()?)
    }
    /// Resolve geometry only; class, animation, node-link and FX programs are
    /// not executed here. The renderer must provide its validated distance input.
    pub fn select_model_rgb565(
        &self,
        class: i32,
        input: crate::lod::Input,
    ) -> Result<Option<ModelAsset>, Box<dyn std::error::Error>> {
        let selected = self.selection_table(class)?.select(input)?;
        let index = usize::try_from(selected)
            .map_err(|_| "negative selection requires explicit renderer policy")?;
        self.model_slot_rgb565(class, index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn profiled_state_bindings_keep_the_captured_namespace() {
        use crate::{
            model_registry::{ClassProgram, SpecDefinition, StateBinding, StateTarget},
            state_storage::{Profile, ReferenceKey},
        };
        let mut models = crate::model_registry::ModelRegistry::default();
        let program = giants_formats::class_program::parse(&[
            0x1e, 2, 0x1f, 20, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 0xff,
        ])
        .unwrap();
        models.programs.insert(
            2,
            vec![ClassProgram {
                catalog: "early.bin".into(),
                program: Ok(program),
                bindings: Ok(vec![StateBinding {
                    offset: 0x6c,
                    key: ReferenceKey::Spec(20),
                    target: Err("absent when class registered".into()),
                }]),
            }],
        );
        models.specs.insert(
            20,
            vec![SpecDefinition {
                catalog: "late.bin".into(),
                effect_namespace: None,
                models: Ok(vec![]),
                program: Err("later metadata".into()),
                selection: Err("later metadata".into()),
            }],
        );
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
        let debug = resources
            .class_state_bindings_for(2, Profile::Debug)
            .unwrap();
        assert_eq!(debug[0].offset, 0x70);
        assert_eq!(
            debug[0].target.as_ref().unwrap_err(),
            "absent when class registered"
        );
        let captured = resources.classes.models.programs.get_mut(&2).unwrap()[0]
            .bindings
            .as_mut()
            .unwrap();
        captured[0].target = Ok(StateTarget::Spec {
            id: 20,
            catalog: "captured.bin".into(),
        });
        let debug = resources
            .class_state_bindings_for(2, Profile::Debug)
            .unwrap();
        assert!(
            matches!(&debug[0].target,Ok(StateTarget::Spec{id:20,catalog}) if catalog==Path::new("captured.bin"))
        );
        assert_eq!(
            resources.class_state_bindings_for(2, Profile::Gog).unwrap()[0].offset,
            0x6c
        );
    }
    #[test]
    fn placement_override_is_absolute_and_callbacks_stay_unexecuted() {
        let mut models = crate::model_registry::ModelRegistry::default();
        let mut bytes = vec![0x17];
        bytes.extend(2f32.to_le_bytes());
        bytes.push(0x1e);
        bytes.push(7);
        bytes.push(0x17);
        bytes.extend(3f32.to_le_bytes());
        bytes.push(0x15);
        bytes.extend(0.5f32.to_le_bytes());
        bytes.push(0xff);
        models.programs.insert(
            7,
            vec![crate::model_registry::ClassProgram {
                catalog: "synthetic.bin".into(),
                bindings: Ok(vec![]),
                program: Ok(giants_formats::class_program::parse(&bytes).unwrap()),
            }],
        );
        let resources = SceneResources {
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
        let mut placement = giants_formats::mission::Placement {
            source_offset: 4,
            object_type: 7,
            position: [1., 2., 3.],
            yaw: 0.,
            pitch_roll: None,
            scale: None,
        };
        let source = resources.placement_input(&placement).unwrap();
        assert_eq!(source.scale, 3.);
        assert_eq!(source.class_commands, [0x17, 0x1e, 0x17, 0x15, 0xff]);
        assert_eq!(source.declared_ground_offset.as_ref().unwrap(), &0.5);
        assert_eq!(source.state_seed.as_ref().unwrap().storage.state(), 7);
        assert_eq!(source.position, placement.position);
        placement.scale = Some(1.5);
        assert_eq!(resources.placement_input(&placement).unwrap().scale, 1.5);
        placement.pitch_roll = Some([-30., 5.]);
        assert_eq!(
            resources.placement_input(&placement).unwrap().angles(),
            [0., -30., 5.]
        );
        placement.scale = Some(0.);
        assert!(resources.placement_input(&placement).is_err());
        placement.object_type = 8;
        assert!(resources.placement_input(&placement).is_err());
    }
}
