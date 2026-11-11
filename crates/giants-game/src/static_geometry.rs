//! Source-only rigid geometry capability. Initialization callbacks, part-node
//! transforms, runtime visibility and general object simulation remain separate.
use crate::{
    instances::Plan,
    scene_resources::{ModelAsset, SceneResources, SourcePlacement, TextureEncoding},
};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Copy)]
enum PosePolicy {
    Benchmark,
    Unanimated,
    NormalInitial,
}
pub type SourceKey = (bool, usize);
pub struct Asset {
    pub model: ModelAsset,
    pub normals: Option<Vec<[f32; 3]>>,
    pub initial_pose: Option<crate::model_pose::InitialPose>,
    pub reflection_descriptors: Vec<Option<crate::material_descriptor::Descriptor>>,
}
pub struct Vertices {
    pub positions: Vec<[f32; 3]>,
    pub uv: Vec<[f32; 2]>,
    pub rgb: Vec<[u8; 3]>,
    pub normals: Option<Vec<[f32; 3]>>,
}
impl Asset {
    /// A separately checked unlit composition; normal materials retain their
    /// own route even when the serialized flags have the same low byte.
    pub fn texture_color_profile(&self, part: usize) -> Option<crate::unlit_material::Profile> {
        let material = &self.model.mesh.subobjects[part].material;
        (self.normals.is_none() && material.flags == 0x40000010).then(|| {
            crate::unlit_material::static_texture_color(
                material,
                self.model
                    .alpha_textures
                    .contains(&material.texture.to_ascii_lowercase()),
            )
            .expect("validated static texture/color material")
        })
    }
    /// Explicit current preview policy for an opaque lit part: source blend
    /// constant, normal base and measured mode/toggles. General object and
    /// graphics-setting projection remain outside this source-only consumer.
    pub fn reflection_descriptor(
        &self,
        part: usize,
    ) -> Option<crate::material_descriptor::Descriptor> {
        self.reflection_descriptors[part]
    }
    /// Exact source triangle expansion, before the backend coordinate adapter.
    pub fn vertices(&self, part: usize) -> Vertices {
        let ids: Vec<_> = self.model.mesh.subobjects[part]
            .triangles
            .iter()
            .flatten()
            .map(|v| usize::from(*v))
            .collect();
        Vertices {
            positions: ids
                .iter()
                .map(|i| self.model.mesh.positions[usize::from(self.model.mesh.indices[*i])])
                .collect(),
            uv: ids.iter().map(|i| self.model.mesh.uv[*i]).collect(),
            rgb: ids.iter().map(|i| self.model.mesh.rgb[*i]).collect(),
            normals: self
                .normals
                .as_ref()
                .map(|n| ids.iter().map(|i| n[*i]).collect()),
        }
    }
}
pub struct Instance {
    pub key: SourceKey,
    pub class: i32,
    pub initial_state: u32,
    pub cinematic_exclusions: Vec<[u8; 32]>,
    pub position: [f32; 3],
    pub scale: f32,
    pub matrix: [f32; 16],
    pub table: crate::lod::Table,
    pub slots: Vec<usize>,
}
impl Instance {
    /// Explicit measured view profile: quality2, ordinary selection, zero offset,
    /// 53-bit arithmetic. No policy is inferred from a host graphics setting.
    pub fn detail(&self, eye: [f32; 3], factor: f32) -> Result<usize, &'static str> {
        let slot = self.table.select_positioned(
            crate::lod::PositionInput {
                object_position: self.position,
                camera_position: eye,
                offset: 0.,
                camera_factor: factor,
                object_scale: self.scale,
                quality: 2,
                forced: -1,
                allow_missing: false,
            },
            crate::arithmetic::Precision::Double53,
        )?;
        usize::try_from(slot).map_err(|_| "negative static model selection")
    }
}
pub struct Diagnostic {
    pub key: SourceKey,
    pub class: i32,
    pub result: String,
}
#[derive(Default)]
pub struct Scene {
    pub instances: Vec<Instance>,
    pub assets: BTreeMap<(i32, usize), Asset>,
    pub diagnostics: Vec<Diagnostic>,
}
impl Scene {
    pub fn prepare(plan: &Plan, covered: &BTreeSet<SourceKey>) -> Self {
        Self::prepare_profile(plan, covered, PosePolicy::Benchmark, None)
    }
    /// Cinematic scenery preview at the terrain-query stage of state27.
    /// The following gravity/time integration, reset and live callbacks are
    /// deliberately not claimed as executed. Never grants mission readiness.
    pub fn prepare_cinematic_ground_stage(
        plan: &Plan,
        covered: &BTreeSet<SourceKey>,
        terrain: &giants_formats::gti::Terrain,
    ) -> Self {
        Self::prepare_profile(plan, covered, PosePolicy::Benchmark, Some(terrain))
    }
    /// Normal mission staging has no measured animation warmup yet. Never use
    /// the benchmark inspection step to manufacture a normal initial pose.
    pub fn prepare_unanimated(plan: &Plan, covered: &BTreeSet<SourceKey>) -> Self {
        Self::prepare_profile(plan, covered, PosePolicy::Unanimated, None)
    }
    /// Explicit checked initial-visual profile. Does not execute AI or grant Ready.
    pub fn prepare_normal_initial(plan: &Plan, covered: &BTreeSet<SourceKey>) -> Self {
        Self::prepare_profile(plan, covered, PosePolicy::NormalInitial, None)
    }
    /// Normal mission staging with its real terrain available for generated
    /// marker objects. This still prepares initial visuals, never runtime Ready.
    pub fn prepare_normal_initial_with_terrain(
        plan: &Plan,
        covered: &BTreeSet<SourceKey>,
        terrain: &giants_formats::gti::Terrain,
    ) -> Self {
        Self::prepare_profile(plan, covered, PosePolicy::NormalInitial, Some(terrain))
    }
    fn prepare_profile(
        plan: &Plan,
        covered: &BTreeSet<SourceKey>,
        policy: PosePolicy,
        terrain: Option<&giants_formats::gti::Terrain>,
    ) -> Self {
        let mut out = Self::default();
        for p in &plan.entries {
            let key = (false, p.serialized.source_offset);
            let result = if covered.contains(&key) {
                Ok("covered by existing consumer".into())
            } else {
                p.source.as_ref().map_err(Clone::clone).and_then(|input| {
                    let matrix = p.transform(crate::instances::Adjustment::Authored)?.matrix;
                    out.add(key, input, matrix, p.resources(), policy, terrain)
                })
            };
            out.diagnostics.push(Diagnostic {
                key,
                class: p.serialized.object_type as i32,
                result: result.unwrap_or_else(|e| format!("unresolved: {e}")),
            });
        }
        match &plan.marker_buoys {
            Ok(requests) => {
                for request in requests {
                    let key = (false, request.source_offset);
                    let result = (|| {
                        let terrain =
                            terrain.ok_or("generated buoy terrain query is unavailable")?;
                        let query = crate::terrain_queries::StaticTerrainQuery::new(
                            terrain,
                            (1. / f64::from(terrain.spacing)) as f32,
                            crate::arithmetic::Precision::Double53,
                        )?;
                        let mut position = request.position;
                        position[2] = query.sample([position[0], position[1]])?.height.max(0.);
                        let placement = giants_formats::mission::Placement {
                            source_offset: request.source_offset,
                            object_type: 1097,
                            position,
                            yaw: request.yaw,
                            pitch_roll: None,
                            scale: None,
                        };
                        let source = plan
                            .resources()
                            .placement_input(&placement)
                            .map_err(|e| e.to_string())?;
                        let matrix = source.unadjusted_matrix()?;
                        self::Scene::add(
                            &mut out,
                            key,
                            &source,
                            matrix,
                            plan.resources(),
                            policy,
                            Some(terrain),
                        )
                    })();
                    out.diagnostics.push(Diagnostic {
                        key,
                        class: 1097,
                        result: format!(
                            "generated marker pair side={}; {}",
                            request.side,
                            result.unwrap_or_else(|e: String| format!("unresolved: {e}"))
                        ),
                    });
                }
            }
            Err(error) => out.diagnostics.push(Diagnostic {
                key: (false, 0),
                class: 1097,
                result: format!("marker graph unresolved: {error}"),
            }),
        }
        for p in &plan.world_entries {
            let key = (true, p.declaration.source_offset);
            let result = if covered.contains(&key) {
                Ok("covered by existing consumer".into())
            } else {
                p.class_input().and_then(|input| {
                    // The first constructor pose uses class angle defaults;
                    // later modifiers/callbacks are not applied here.
                    out.add(
                        key,
                        input,
                        p.initial_matrix()?,
                        p.resources(),
                        policy,
                        terrain,
                    )
                })
            };
            out.diagnostics.push(Diagnostic {
                key,
                class: p.declaration.object_type as i32,
                result: result.unwrap_or_else(|e| format!("unresolved: {e}")),
            });
            if matches!(policy, PosePolicy::Benchmark)
                && !covered.contains(&key)
                && let Ok(input) = p.class_input()
                && input.class == 985
            {
                let requests = p.initial_matrix().and_then(|matrix| {
                    crate::base_layout_scene::requests(p.resources(), input, matrix)
                });
                match requests {
                    Ok(requests) => {
                        for request in requests {
                            let result = request
                                .source
                                .unadjusted_matrix()
                                .map_err(String::from)
                                .and_then(|matrix| {
                                    out.add(
                                        key,
                                        &request.source,
                                        matrix,
                                        p.resources(),
                                        policy,
                                        terrain,
                                    )
                                });
                            out.diagnostics.push(Diagnostic {
                                    key, class:request.source.class,
                                    result:format!("layout reference={}; {}; construction preview; live visibility unverified",
                                        request.reference,result.unwrap_or_else(|e|format!("unresolved: {e}"))),
                                });
                        }
                    }
                    Err(error) => out.diagnostics.push(Diagnostic {
                        key,
                        class: input.class,
                        result: format!("layout construction unresolved: {error}"),
                    }),
                }
            }
        }
        out.prepare_race_laps(plan, policy, terrain);
        out
    }
    fn prepare_race_laps(
        &mut self,
        plan: &Plan,
        policy: PosePolicy,
        terrain: Option<&giants_formats::gti::Terrain>,
    ) {
        if matches!(policy, PosePolicy::Unanimated) {
            return;
        }
        let roots = match &plan.marker_buoys {
            Ok(requests) => requests.iter().filter(|r| r.ordinal == 0).count(),
            Err(_) => 0,
        };
        if roots == 0 {
            return;
        }
        let gates: Vec<_> = self
            .instances
            .iter()
            .filter(|i| i.class == 1147)
            .map(|i| {
                (
                    i.key,
                    i.matrix,
                    i.scale,
                    self.assets
                        .get(&(i.class, i.slots[0]))
                        .and_then(|a| a.initial_pose.as_ref())
                        .map(|p| p.tick),
                )
            })
            .collect();
        for (key, matrix, scale, tick) in gates {
            let result = (|| {
                if scale != 1. {
                    return Err("scaled race gate attachment remains unchecked".to_owned());
                }
                let tick = tick.ok_or("race gate initial animation pose unavailable")?;
                crate::instances::race_laps::matrices(plan.resources(), matrix, tick)
            })();
            match result {
                Ok(requests) => {
                    for _ in 0..roots {
                        for (slot, matrix) in &requests {
                            let result = (|| {
                                let placement = giants_formats::mission::Placement {
                                    source_offset: key.1,
                                    object_type: 1160,
                                    position: [matrix[3], matrix[7], matrix[11]],
                                    yaw: 0.,
                                    pitch_roll: None,
                                    scale: None,
                                };
                                let source = plan
                                    .resources()
                                    .placement_input(&placement)
                                    .map_err(|e| e.to_string())?;
                                if source.scale != 1. {
                                    return Err("scaled race indicator remains unchecked".into());
                                }
                                // The constructor explicitly selects channel1 at time0;
                                // do not apply the benchmark loop warmup to this child.
                                self.add(
                                    key,
                                    &source,
                                    *matrix,
                                    plan.resources(),
                                    PosePolicy::NormalInitial,
                                    terrain,
                                )
                            })();
                            self.diagnostics.push(Diagnostic { key, class: 1160,
                            result: format!("generated lap attachment parent_reference={slot}; {}; initial construction only; race progress/retirement and live parent clocks unverified",
                                result.unwrap_or_else(|e: String|format!("unresolved: {e}"))) });
                        }
                    }
                }
                Err(error) => self.diagnostics.push(Diagnostic {
                    key,
                    class: 1160,
                    result: format!("race lap attachment unresolved: {error}"),
                }),
            }
        }
    }
    fn add(
        &mut self,
        key: SourceKey,
        input: &SourcePlacement,
        mut matrix: [f32; 16],
        resources: &SceneResources,
        policy: PosePolicy,
        terrain: Option<&giants_formats::gti::Terrain>,
    ) -> Result<String, String> {
        if input.direct_flags.active & 0x4000c000 != 0 {
            return Ok(
                "initial template suppresses ordinary render entry; callbacks pending".into(),
            );
        }
        if input.direct_flags.personality & 0x84 != 0 {
            return Err("camera-facing instance needs current inputs".into());
        }
        let state = input
            .state_seed
            .as_ref()
            .map_err(Clone::clone)?
            .storage
            .state();
        // The state91 initializer hides the layout template unless the native
        // debug-template display switch is enabled. A layout mesh is not its
        // constructed buildings. The explicit reference-based preview below
        // still prepares supported children independently of this refusal.
        if state == 91 {
            return Err(
                "base template initializer suppresses ordinary model; debug-template display and building services remain separate"
                    .into(),
            );
        }
        if state == 94 && matches!(policy, PosePolicy::NormalInitial) {
            return Ok("destination helper suppressed by ordinary state94 visibility; objective volume remains owned separately".into());
        }
        // In normal mode the first formation callback suppresses its ordinary
        // source model before creating members. The marker is not a substitute
        // for that unimplemented service; retain an explicit preparation need.
        if state == 97 {
            return Err(
                "formation initial callback suppresses ordinary model; member creation backend pending"
                    .into(),
            );
        }
        if state == 60 {
            // Check the actual per-instance payload as well as the class seed:
            // ordered mission writes may have changed it after preparation.
            let program = resources
                .classes
                .models
                .first_program(input.class)
                .ok_or("class program missing")?
                .program
                .as_ref()
                .map_err(Clone::clone)?;
            crate::model_pose::idle_tree_channel(
                program,
                &input.state_seed.as_ref().map_err(Clone::clone)?.storage,
                false,
            )?;
        }
        let mut position = input.position;
        if state == 27 {
            let terrain = terrain.ok_or("placement adjustment callback pending")?;
            let query = crate::terrain_queries::StaticTerrainQuery::new(
                terrain,
                (1. / f64::from(terrain.spacing)) as f32,
                crate::arithmetic::Precision::Double53,
            )?;
            position[2] = query.sample([position[0], position[1]])?.height;
            matrix[11] = position[2];
        }
        let binding = resources
            .classes
            .models
            .first_selection(input.class)
            .ok_or("class model declaration missing")?
            .as_ref()
            .map_err(Clone::clone)?;
        if !binding.selection.diagnostics.is_empty() {
            return Err("model declaration reports initialization diagnostics".into());
        }
        if matches!(policy, PosePolicy::Unanimated) && !binding.selection.animations.is_empty() {
            return Err(
                "normal animation initialization remains unresolved; benchmark warmup not applied"
                    .into(),
            );
        }
        // The original vertex updater tests the first model's skeleton before
        // LOD/channel selection. An undeformed preview must not manufacture an
        // animation pose just to keep these vertices. Object/part callbacks and
        // visibility remain independent, explicitly unverified consumers.
        let unskinned_first = binding
            .selection
            .slots
            .first()
            .and_then(|model| model.name.as_ref())
            .is_some_and(|name| {
                !resources
                    .archives
                    .members
                    .contains_key(&format!("{}.skn", name.to_ascii_lowercase()))
            });
        let pose = if state == 18 && matches!(policy, PosePolicy::NormalInitial) {
            Some(crate::model_pose::Prepared::load_stationary_smartie(
                resources,
                input.class,
                &binding.selection,
                input.mission_selector,
            )?)
        } else if binding.selection.animations.is_empty() || unskinned_first {
            None
        } else if state == 27 && matches!(policy, PosePolicy::Benchmark) {
            Some(crate::model_pose::Prepared::load_ground_preview(
                resources,
                input.class,
                &binding.selection,
            )?)
        } else if state == 82 && matches!(policy, PosePolicy::Benchmark) {
            Some(crate::model_pose::Prepared::load_base_preview(
                resources,
                input.class,
                &binding.selection,
            )?)
        } else if matches!(policy, PosePolicy::NormalInitial) || matches!(state, 98 | 102) {
            Some(crate::model_pose::Prepared::load_normal_initial(
                resources,
                state,
                input.class,
                &binding.selection,
            )?)
        } else {
            Some(crate::model_pose::Prepared::load(
                resources,
                state,
                input.class,
                &binding.selection,
            )?)
        };
        let table = binding.selection.table().map_err(String::from)?;
        let mut prepared = Vec::new();
        let mut slots = Vec::new();
        for (slot, declaration) in binding.selection.slots.iter().enumerate() {
            if declaration.threshold < 0. {
                continue;
            }
            let Some(name) = &declaration.name else {
                continue;
            };
            if pose.is_none()
                && !unskinned_first
                && resources
                    .archives
                    .members
                    .contains_key(&format!("{}.skn", name.to_ascii_lowercase()))
            {
                return Err(format!("skinning dependency for {name}"));
            }
            let cache_key = (input.class, slot);
            if !self.assets.contains_key(&cache_key) {
                let mut model = resources
                    .model_slot_with_untextured_diffuses(
                        input.class,
                        slot,
                        TextureEncoding::Rgb565OrArgb4444,
                    )
                    .map_err(|e| e.to_string())?
                    .ok_or("resolved model slot unexpectedly empty")?;
                if let Some(pose) = &pose {
                    model.mesh.positions = pose.positions(resources, name, &model.mesh)?;
                }
                let mut reflection_descriptors = Vec::new();
                for part in &model.mesh.subobjects {
                    if !matches!(
                        part.material.flags,
                        0x40000000 | 0x40000002 | 0x40000020 | 0xc0000020 | 0x50000020 | 0xd0000020
                    ) {
                        // Material capability does not depend on whether the
                        // independently selected pose uses benchmark warmup.
                        let known_normal_surface = matches!(
                            crate::body_pipeline::preview_surface(
                                crate::body_pipeline::Normals::select(
                                    false,
                                    model.mesh.normal_count != 0,
                                    true,
                                    false
                                ),
                                &part.material,
                                model
                                    .alpha_textures
                                    .contains(&part.material.texture.to_ascii_lowercase()),
                            ),
                            Ok(crate::body_pipeline::Surface::Rgb
                                | crate::body_pipeline::Surface::Lit
                                | crate::body_pipeline::Surface::Cube { .. })
                                | Ok(crate::body_pipeline::Surface::AdditiveCube { .. }
                                    | crate::body_pipeline::Surface::AlphaCube { .. })
                        );
                        let known_static_color = model.mesh.normal_count == 0
                            && crate::unlit_material::static_texture_color(
                                &part.material,
                                model
                                    .alpha_textures
                                    .contains(&part.material.texture.to_ascii_lowercase()),
                            )
                            .is_ok();
                        if !known_normal_surface && !known_static_color {
                            return Err(format!(
                                "unsupported material flags {:08x}",
                                part.material.flags
                            ));
                        }
                    }
                    let pass = crate::render_parts::material_pass(
                        part.material.falloff,
                        model
                            .alpha_textures
                            .contains(&part.material.texture.to_ascii_lowercase()),
                    );
                    let descriptor = if part.material.flags & 0x10000000 != 0 {
                        if model.mesh.normal_count == 0 {
                            return Err("auxiliary material requires normals".into());
                        }
                        let (base_flags, constant, constant_guard) =
                            if pass == crate::render_parts::Pass::Opaque {
                                let (constant, constant_guard) =
                                    crate::material_descriptor::part_constant(part.material.blend)
                                        .map_err(String::from)?;
                                (
                                    if constant != 0 { 0x3094 } else { 0x3014 },
                                    constant,
                                    constant_guard,
                                )
                            } else {
                                crate::body_pipeline::preview_surface(
                                    crate::body_pipeline::Normals::select(false, true, true, false),
                                    &part.material,
                                    model
                                        .alpha_textures
                                        .contains(&part.material.texture.to_ascii_lowercase()),
                                )?;
                                crate::material_descriptor::inspection_base(
                                    crate::material_descriptor::InspectionInput {
                                        part_flags: part.material.flags,
                                        blend: part.material.blend,
                                        falloff: part.material.falloff,
                                        has_texture: !part.material.texture.is_empty(),
                                        texture_alpha: model
                                            .alpha_textures
                                            .contains(&part.material.texture.to_ascii_lowercase()),
                                        fog: true,
                                        pass_mask: pass.mask(),
                                    },
                                )?
                            };
                        let descriptor =
                            crate::material_descriptor::lit(crate::material_descriptor::LitInput {
                                part_flags: part.material.flags,
                                base_flags,
                                constant,
                                constant_guard,
                                mode: 0x5000,
                                toggle: false,
                                force_auxiliary: false,
                            });
                        descriptor.cube_blend_profile().map_err(String::from)?;
                        Some(descriptor)
                    } else {
                        None
                    };
                    reflection_descriptors.push(descriptor);
                    if (pass == crate::render_parts::Pass::ObjectOrMaterial)
                        != (part.material.flags == 0x40000002)
                        && !descriptor.is_some_and(|d| {
                            d.cube_blend_profile().is_ok_and(|(_, blend)| blend != 0)
                        })
                    {
                        return Err("object/material blend policy unresolved".into());
                    }
                }
                let normals = if model.mesh.normal_count == 0 {
                    None
                } else {
                    Some(model.mesh.render_normals().map_err(|e| e.to_string())?)
                };
                prepared.push((
                    cache_key,
                    Asset {
                        model,
                        normals,
                        initial_pose: pose.as_ref().map(|p| p.pose.clone()),
                        reflection_descriptors,
                    },
                ));
            }
            slots.push(slot);
        }
        if slots.is_empty() {
            return Ok("no drawable model in declaration".into());
        }
        self.assets.extend(prepared);
        self.instances.push(Instance {
            key,
            class: input.class,
            initial_state: u32::from(state),
            cinematic_exclusions: input.cinematic_exclusions.clone(),
            position,
            scale: input.scale,
            matrix,
            table,
            slots,
        });
        if state == 18 {
            return Ok(
                "stationary Smartie selected source start; blend, live animation and AI unverified"
                    .into(),
            );
        }
        if matches!(state, 23 | 38 | 51 | 105) {
            return Ok(format!(
                "source constructed actor pose prepared; state={state}; AI/object callbacks, burial and live visibility unverified"
            ));
        }
        if state == 60 {
            return Ok(format!(
                "source initial idle-tree geometry prepared; unconsumed_auxiliary_links={}; live callbacks/visibility unverified",
                pose.as_ref()
                    .map_or(0, |p| p.pose.unresolved_auxiliary_links)
            ));
        }
        Ok(if state == 27 {
            "source ground-query stage geometry prepared; gravity/time integration, reset and live callbacks unverified"
        } else if unskinned_first && !binding.selection.animations.is_empty() {
            "source undeformed geometry prepared; first model has no skeleton; animation/object/part callbacks and visibility unverified"
        } else if pose.as_ref().is_some_and(|p| p.pose.profile == "declared_loop_warmup") {
            "source declared-loop warmup geometry prepared; current callbacks/visibility unverified"
        } else if pose.as_ref().is_some_and(|p|p.pose.profile.starts_with("debug_normal_")) {
            "source normal initial visual prepared; live callbacks/visibility unverified"
        } else if pose.as_ref().is_some_and(|p|p.pose.profile == "source_base_declared_start") {
            "source declared base-channel pose prepared; live animation/state82 callbacks and visibility unverified"
        } else if pose.is_some() {
            "source intact-health geometry prepared; damage transitions/current visibility unverified"
        } else {
            "source rigid geometry prepared; current callbacks/visibility unverified"
        }.into())
    }
}
