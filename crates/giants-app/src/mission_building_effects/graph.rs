//! Mixed building effects use the actual prepared context and source graph.
//! Prototype PS2 `fx_logic`, `fx_move` and `fx_processOpCodes` remain unknown
//! attribution routes. Host-frame activation and terrain-only contact are
//! explicit preview policies, not canonical state78/world-fidelity claims.
use super::*;
mod preflight;
use giants_game::{effect_scene_runtime as mixed, static_geometry};

enum Geometry {
    Hidden,
    Particle {
        model: Arc<Model>,
        animation: Option<Box<giants_game::effect_animation::Playback>>,
    },
    Rigid {
        instance: Box<static_geometry::Instance>,
        assets: BTreeMap<usize, Arc<static_geometry::Asset>>,
    },
}
struct Node {
    serial: u64,
    context: u32,
    object_flags: u32,
    geometry: Geometry,
}
struct Retirement {
    binding: giants_game::effect_scene::RetirementBinding,
    point: [f32; 3],
    object_flags: u32,
    depth: usize,
}
pub(super) struct Graph {
    runtime: mixed::Runtime,
    references: mixed::References,
    nodes: Vec<Node>,
    depth: usize,
    frame: u64,
}
impl Graph {
    pub(super) fn active(&self) -> usize {
        self.runtime
            .bodies
            .iter()
            .flatten()
            .filter(|b| !b.expired())
            .count()
    }
    fn create(
        parent: &mut Runtime,
        runtime: mixed::Runtime,
        references: mixed::References,
        depth: usize,
    ) -> Result<Self, String> {
        if depth > 8
            || runtime.plan.lifetime_bounds().len() != 1
            || runtime.plan.requires_external_resume()
        {
            return Err(
                "building mixed context requires a bounded single phase/descendant depth".into(),
            );
        }
        if parent.active() + runtime.bodies[0].len() > 4096 {
            return Err("building mixed owned body budget exceeded".into());
        }
        let image = parent
            .image
            .as_ref()
            .ok_or("building mixed source image absent")?
            .clone();
        let mut nodes = vec![];
        for body in &runtime.bodies[0] {
            let c = &body.construction;
            let needs = &c.object_services;
            if !c.services.is_empty()
                || needs.logged_context
                || needs.source16
                || needs.source5a.is_some()
                || (body.curve_pending && !body.free_curve35_projection())
            {
                return Err("building mixed effect has unbound object/context services".into());
            }
            let playback = body
                .playback()
                .ok_or("building mixed duration-free owner is unbound")?;
            if let Some(child) = &needs.retirement
                && image.find(&child.effect).map(|(_, e)| e.offset) != Some(child.program_offset)
            {
                return Err("building mixed retirement source identity differs".into());
            }
            let model = parent.model(&playback.spawn)?;
            // This preview has no inherited external render mark. Do not drop
            // source flag inheritance when the class itself requires it.
            if model.object_flags & 0x400 != 0 {
                return Err("building mixed class needs inherited retirement flags".into());
            }
            let geometry = if c.hidden {
                Geometry::Hidden
            } else if model.geometry.mesh.normal_count != 0 {
                if c.lifetime.is_some_and(|[duration, fade_in, fade_out]| {
                    fade_in > 0. || fade_out > 0. && fade_out < duration
                }) {
                    return Err("building rigid effect needs its fading material".into());
                }
                if model.actor.flags.personality & 0x84 != 0 {
                    return Err("building rigid effect needs its facing owner".into());
                }
                let mut scene = static_geometry::Scene::constructed_rigid_draw(
                    &parent.source,
                    (false, 0),
                    c.class,
                    body.matrix().map_err(str::to_owned)?.values,
                    body.size(),
                )?;
                let instance = Box::new(scene.instances.remove(0));
                let assets = scene
                    .assets
                    .into_iter()
                    .map(|((_, slot), asset)| (slot, Arc::new(asset)))
                    .collect();
                Geometry::Rigid { instance, assets }
            } else {
                for part in &model.geometry.mesh.subobjects {
                    for transparency in [0., 1.] {
                        giants_game::unlit_material::prepare_with_object_transparency(
                            &part.material,
                            model
                                .geometry
                                .alpha_textures
                                .contains(&part.material.texture.to_ascii_lowercase()),
                            transparency,
                        )
                        .map_err(str::to_owned)?;
                    }
                }
                let animation = giants_game::effect_animation::Playback::new(
                    giants_game::actor_context::Prepared::class_initial(model.actor.clone()),
                )?;
                Geometry::Particle {
                    model: model.clone(),
                    animation: animation.map(Box::new),
                }
            };
            parent.next = parent
                .next
                .checked_add(1)
                .ok_or("building mixed identity exhausted")?;
            nodes.push(Node {
                serial: parent.next,
                context: model.context,
                object_flags: model.object_flags,
                geometry,
            });
        }
        Ok(Self {
            runtime,
            references,
            nodes,
            depth,
            frame: 0,
        })
    }
    fn advance(
        &mut self,
        delta: f32,
        areas: &[giants_game::object_time::SlowArea],
    ) -> Result<Vec<Retirement>, String> {
        let contexts: BTreeMap<_, _> = self.runtime.bodies[0]
            .iter()
            .zip(&self.nodes)
            .map(|(b, n)| (b.construction.class, n.context))
            .collect();
        let seconds = |body: &mixed::Body| -> Result<f32, String> {
            let p = body
                .playback()
                .ok_or("building mixed object clock absent")?;
            Ok(giants_game::object_time::seconds(
                delta,
                Some(giants_game::object_time::Sample {
                    owner: *contexts
                        .get(&body.construction.class)
                        .ok_or("building mixed time context absent")?,
                    position: p.motion.position,
                }),
                None,
                areas,
            ) as f32)
        };
        let times = self.runtime.bodies[0]
            .iter()
            .map(&seconds)
            .collect::<Result<Vec<_>, _>>()?;
        self.runtime
            .advance_bodies_resolved(delta, &self.references, |body, _| seconds(body))?;
        for ((node, body), dt) in self
            .nodes
            .iter_mut()
            .zip(&self.runtime.bodies[0])
            .zip(times)
        {
            if !body.expired()
                && let Geometry::Particle {
                    animation: Some(animation),
                    ..
                } = &mut node.geometry
            {
                animation.advance(dt).map_err(str::to_owned)?;
            }
        }
        self.frame += 1;
        self.runtime
            .take_retirements()
            .into_iter()
            .map(|request| {
                if request.phase != 0 {
                    return Err("building mixed retirement phase is unbound".into());
                }
                let node = self
                    .nodes
                    .get(request.object)
                    .ok_or("building mixed retired owner absent")?;
                Ok(Retirement {
                    binding: request.binding,
                    point: request.position,
                    object_flags: node.object_flags,
                    depth: self.depth + 1,
                })
            })
            .collect()
    }
    pub(super) fn draws(
        &self,
        view: [f32; 16],
        detail: Option<([f32; 3], f32)>,
    ) -> Result<Vec<crate::mission_runtime::NpcDraw>, String> {
        let mut draws = vec![];
        for (body, node) in self.runtime.bodies[0].iter().zip(&self.nodes) {
            if !body.visible() {
                continue;
            }
            let matrix = body.matrix().map_err(str::to_owned)?.values;
            let position = [matrix[3], matrix[7], matrix[11]];
            let (geometry, positions, rigid) = match &node.geometry {
                Geometry::Hidden => continue,
                Geometry::Rigid { instance, assets } => {
                    if body.transparency() != 0. {
                        return Err("building rigid effect needs its fading material".into());
                    }
                    let (eye, factor) = detail
                        .ok_or("building rigid effect requires the current source LOD camera")?;
                    let slot = instance
                        .table
                        .select_positioned(
                            giants_game::lod::PositionInput {
                                object_position: position,
                                camera_position: eye,
                                offset: 0.,
                                camera_factor: factor,
                                object_scale: body.size(),
                                quality: 2,
                                forced: -1,
                                allow_missing: false,
                            },
                            Precision::Double53,
                        )
                        .map_err(str::to_owned)?;
                    let asset = assets
                        .get(&usize::try_from(slot).map_err(|_| "building rigid negative LOD")?)
                        .ok_or("building rigid selected source LOD absent")?
                        .clone();
                    (
                        asset.model.clone(),
                        asset.model.mesh.positions.clone(),
                        Some(crate::mission_runtime::NpcRigidProfile { asset, matrix }),
                    )
                }
                Geometry::Particle { model, animation } => {
                    let values = giants_game::camera_facing::draw_matrix(
                        matrix,
                        model.actor.flags.personality,
                        giants_game::camera_facing::Input {
                            view,
                            position,
                            scale: body.size(),
                            angle: body.angles().map_or(0., |a| a[2]),
                        },
                        Precision::Double53,
                    )?;
                    let local = match animation.as_ref().map(|a| a.positions()).transpose()? {
                        Some(giants_game::actor_context::PositionUpdate::Replace(p)) => p,
                        _ => model.geometry.mesh.positions.clone(),
                    };
                    let positions = local
                        .into_iter()
                        .map(|p| {
                            let p = giants_game::uncached_pose::transform(
                                values[..12].try_into().unwrap(),
                                p,
                                Precision::Double53,
                            )?;
                            Ok([p[0], p[2], -p[1]])
                        })
                        .collect::<Result<Vec<_>, String>>()?;
                    (model.geometry.clone(), positions, None)
                }
            };
            draws.push(crate::mission_runtime::NpcDraw {
                rigid,
                class: body.construction.class,
                id: crate::mission_runtime::NpcDrawOwner::BuildingEffect(node.serial),
                geometry,
                positions,
                frame: self.frame,
                transparency: body.transparency(),
                center: position,
                texture: None,
            });
        }
        Ok(draws)
    }
}
impl Runtime {
    pub(super) fn consume_mixed(
        &mut self,
        ticket: u64,
        origin: giants_game::attached_effect_context::Id,
        point: [f32; 3],
        random: &mut Random,
    ) -> Result<Vec<crate::mission_audio::Event>, String> {
        self.consume_mixed_depth(ticket, origin, point, random, 0)
    }
    fn consume_mixed_depth(
        &mut self,
        ticket: u64,
        origin: giants_game::attached_effect_context::Id,
        point: [f32; 3],
        random: &mut Random,
        depth: usize,
    ) -> Result<Vec<crate::mission_audio::Event>, String> {
        let (runtime, references) = self.contexts.consume_positional_scene(
            origin,
            &self.source,
            self.terrain.clone(),
            random,
        )?;
        let mut sounds = vec![];
        for body in runtime.bodies.iter().flatten() {
            for sound in &body.construction.context_sounds {
                println!(
                    "mission_building_sound_request ticket={ticket} sound={} point={point:?} route=position sink_acknowledged=false",
                    sound.id
                );
                sounds.push(crate::mission_audio::Event::NpcPosition {
                    sound: sound.id,
                    position: point,
                });
            }
        }
        let graph = Graph::create(self, runtime, references, depth)?;
        self.births += graph.active() as u64;
        println!(
            "mission_building_mixed_burst ticket={ticket} depth={depth} bodies={} births={} source_graph=true native_fidelity=false",
            graph.active(),
            self.births
        );
        self.graphs.push(graph);
        Ok(sounds)
    }
    pub(super) fn advance_graphs(
        &mut self,
        ticket: u64,
        sample: u64,
        delta: f32,
        areas: &[giants_game::object_time::SlowArea],
        random: &mut Random,
    ) -> Result<Vec<crate::mission_audio::Event>, String> {
        let mut requests = vec![];
        let mut retired = 0;
        for graph in &mut self.graphs {
            let before = graph.active();
            requests.extend(graph.advance(delta, areas)?);
            retired += before - graph.active();
        }
        self.retired += retired as u64;
        self.graphs.retain(|g| g.active() != 0);
        let mut sounds = vec![];
        for Retirement {
            binding,
            point,
            object_flags,
            depth,
        } in requests
        {
            let image = self
                .image
                .as_ref()
                .ok_or("building mixed retirement image absent")?;
            if image.find(&binding.effect).map(|(_, e)| e.offset) != Some(binding.program_offset) {
                return Err("building mixed retirement image changed".into());
            }
            let (creation, _) = self.contexts.prepare_retirement_offset_invariant(
                Some(binding.program_offset),
                point,
                object_flags,
                0,
                random,
            )?;
            let Creation::Prepared(origin) = creation else {
                return Err("building mixed descendant absent".into());
            };
            sounds.extend(self.consume_mixed_depth(ticket, origin, point, random, depth)?);
        }
        if retired != 0 {
            println!(
                "mission_building_mixed_retired ticket={ticket} sample={sample} births={} retired={} active={} actual_mission_clock=true",
                self.births,
                self.retired,
                self.active()
            );
        }
        Ok(sounds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires GIANTS_BUILDING_ROOT; installed app graph admission, including pending rigid fades, no building hits/GPU/OS/audio sink"]
    fn installed_barracks_graphs_keep_unbound_rigid_fades_out_of_admission() {
        let root = std::path::PathBuf::from(std::env::var_os("GIANTS_BUILDING_ROOT").unwrap());
        let menu = giants_game::mission_selection::MissionMenu::parse(
            &std::fs::read(root.join("Bin/w_Story2.bin")).unwrap(),
            0,
            true,
        )
        .unwrap();
        let packet =
            giants_game::mission_loading::prepare(&root, menu.request("Story2", 1).unwrap())
                .unwrap();
        let world = packet.world_asset(&root).unwrap();
        let plan = packet.instance_plan(&root, "English").unwrap();
        let mut pending_fades = 0;
        for (name, class) in [
            ("fx_minibarracks", 1060),
            ("fx_barracks", 1060),
            ("fx_ripperbarracks", 1226),
        ] {
            let point = plan
                .entries
                .iter()
                .find(|p| p.serialized.object_type == class)
                .unwrap()
                .serialized
                .position;
            let source = plan.resources().clone();
            let image = source.classes.models.effects.last().unwrap();
            let (entry, definition) = image.image.find(name).unwrap();
            // Supplied direct program request from the real active image;
            // this is not a captured state98 payload or a building hit.
            let binding = giants_game::model_registry::StateBinding {
                offset: 0,
                key: giants_game::state_storage::ReferenceKey::Effect(name.into()),
                target: Ok(giants_game::model_registry::StateTarget::Effect {
                    image: image.path.clone(),
                    entry,
                    offset: definition.offset,
                }),
            };
            let mut runtime = Runtime::new(source, Arc::new(world.terrain.clone()));
            runtime.bind(91).unwrap();
            if let Err(error) = runtime.preflight_offset(&binding) {
                assert_eq!(error, "building rigid effect needs its fading material");
                assert_eq!(runtime.active(), 0);
                assert_eq!((runtime.births, runtime.retired), (0, 0));
                assert!(runtime.contexts.active().is_empty());
                pending_fades += 1;
                continue;
            }
            let mut random = Random(17);
            let (creation, _) = runtime
                .contexts
                .prepare_positional(
                    name,
                    point,
                    giants_game::effect_initialization::Selection {
                        active_objects: 0,
                        quality: 1.,
                        lite_distance: 10000.,
                        camera: point,
                        recipient_position: point,
                    },
                    &mut random,
                )
                .unwrap();
            let events = runtime
                .consume_prepared(91, creation, point, "BarracksControl", &mut random)
                .unwrap();
            assert!(!events.is_empty());
            let initial = runtime.births;
            let camera = Some(([point[0], point[1] - 200., point[2] + 30.], 1.));
            let draws = runtime
                .draws(bevy::prelude::Mat4::IDENTITY.to_cols_array(), camera)
                .unwrap();
            assert!(draws.iter().any(|d| d.rigid.is_some()));
            assert!(draws.iter().any(|d| d.rigid.is_none()));
            for sample in 0..3600 {
                runtime
                    .advance(
                        super::super::Frame {
                            ticket: 91,
                            sample,
                            delta: 1. / 60.,
                            areas: &[],
                            world: &world,
                        },
                        &mut random,
                    )
                    .unwrap();
                runtime
                    .draws(bevy::prelude::Mat4::IDENTITY.to_cols_array(), camera)
                    .unwrap();
                if runtime.active() == 0 {
                    break;
                }
            }
            assert!(runtime.births > initial);
            assert_eq!(runtime.active(), 0);
            assert_eq!(runtime.births, runtime.retired);
            assert!(runtime.graphs.is_empty());
            assert!(runtime.contexts.active().is_empty());
        }
        assert!(pending_fades > 0);
    }
    #[test]
    #[ignore = "requires GIANTS_BUILDING_ROOT; source mixed graph/terrain/resource ownership, no GPU/OS/audio-sink acceptance"]
    fn installed_guard_graph_draws_lit_debris_and_consumes_actual_contact_descendants() {
        let root = std::path::PathBuf::from(std::env::var_os("GIANTS_BUILDING_ROOT").unwrap());
        let menu = giants_game::mission_selection::MissionMenu::parse(
            &std::fs::read(root.join("Bin/w_Story2.bin")).unwrap(),
            0,
            true,
        )
        .unwrap();
        let packet =
            giants_game::mission_loading::prepare(&root, menu.request("Story2", 0).unwrap())
                .unwrap();
        let world = packet.world_asset(&root).unwrap();
        let plan = packet.instance_plan(&root, "English").unwrap();
        let point = plan
            .entries
            .iter()
            .find(|p| p.serialized.object_type == 1050)
            .unwrap()
            .serialized
            .position;
        let source = plan.resources().clone();
        let binding = source
            .class_state_bindings_for(1050, giants_game::state_storage::Profile::Debug)
            .unwrap()
            .into_iter()
            .find(|b| b.offset == 4)
            .unwrap();
        let mut runtime = Runtime::new(source, Arc::new(world.terrain.clone()));
        let mut random = Random(17);
        let events = runtime
            .emit_offset(91, &binding, point, &mut random)
            .unwrap();
        assert!(!events.is_empty());
        assert_eq!(runtime.graphs.len(), 1);
        let initial = runtime.births;
        let camera = Some(([point[0], point[1] - 200., point[2] + 30.], 1.));
        let draws = runtime
            .draws(bevy::prelude::Mat4::IDENTITY.to_cols_array(), camera)
            .unwrap();
        assert_eq!(draws.iter().filter(|d| d.rigid.is_some()).count(), 5);
        assert!(draws.iter().any(|d| d.rigid.is_none()));
        assert!(
            runtime
                .draws(bevy::prelude::Mat4::IDENTITY.to_cols_array(), None)
                .is_err()
        );
        let mut descendants = false;
        for sample in 0..3600 {
            let events = runtime
                .advance(
                    super::super::Frame {
                        ticket: 91,
                        sample,
                        delta: 1. / 60.,
                        areas: &[],
                        world: &world,
                    },
                    &mut random,
                )
                .unwrap();
            assert!(events.is_empty());
            let before = (runtime.active(), runtime.births, runtime.retired, random.0);
            assert!(
                runtime
                    .advance(
                        super::super::Frame {
                            ticket: 91,
                            sample,
                            delta: 1. / 60.,
                            areas: &[],
                            world: &world
                        },
                        &mut random
                    )
                    .unwrap()
                    .is_empty()
            );
            assert_eq!(
                (runtime.active(), runtime.births, runtime.retired, random.0),
                before
            );
            descendants |= runtime.births > initial;
            runtime
                .draws(bevy::prelude::Mat4::IDENTITY.to_cols_array(), camera)
                .unwrap();
            if runtime.active() == 0 {
                break;
            }
        }
        assert!(descendants);
        assert_eq!(runtime.active(), 0);
        assert_eq!(runtime.births, runtime.retired);
        assert!(runtime.graphs.is_empty());
        assert!(runtime.contexts.active().is_empty());
        assert!(
            runtime
                .draws(bevy::prelude::Mat4::IDENTITY.to_cols_array(), camera)
                .unwrap()
                .is_empty()
        );
        assert!(runtime.bind(92).is_err());
    }
}
