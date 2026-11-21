//! Ordinary pub consumers of the acquired source-owned body phase.
//! Prototype PS2 navigation and attribution remain in ordinary_building_body;
//! this adapter supplies actual map populations rather than native readiness.
use giants_game::{
    arithmetic::Precision,
    declared_residents::{Key, Scope},
    instances::Plan,
    resident_states::{Id, Session},
};
use std::sync::Arc;

#[derive(bevy::prelude::Message, Clone)]
pub(crate) struct Pose {
    pub ticket: u64,
    pub key: (bool, usize),
    pub position: [f32; 3],
    pub scale: f32,
    pub matrix: [f32; 16],
}
struct Entry {
    key: Key,
    object: u32,
    owner: Option<Id>,
}
pub(crate) struct Frame {
    pub pose: Pose,
    pub player: giants_game::player_world_collision::Object,
    pub configured_health: u32,
}
struct Platform {
    id: u32,
    active: u32,
    personality: u32,
    bounds: [[f32; 2]; 2],
    triangles: Vec<[[f32; 3]; 3]>,
    bits: Option<giants_game::footprint_builder::PlatformBits>,
}
pub(crate) struct Bodies {
    source: Arc<giants_game::scene_resources::SceneResources>,
    entries: Vec<Entry>,
    candidates: giants_game::declared_mesh_scene::Prepared,
    started: bool,
    steps: u64,
    last_sample: Option<u64>,
    failure: Option<String>,
}
impl Bodies {
    pub(crate) fn prepare(
        plan: &Plan,
        terrain: &giants_formats::gti::Terrain,
    ) -> Result<Self, String> {
        let mut entries = Vec::new();
        for (world, index, class) in plan
            .world_entries
            .iter()
            .enumerate()
            .map(|(i, p)| (true, i, p.declaration.object_type))
            .chain(
                plan.entries
                    .iter()
                    .enumerate()
                    .map(|(i, p)| (false, i, p.serialized.object_type)),
            )
        {
            if class != 53 {
                continue;
            }
            let object = u32::try_from(index)
                .map_err(|_| "pub declaration index overflow")?
                .checked_add(2)
                .ok_or("pub collision identity overflow")?
                | if world { 0x4000_0000 } else { 0 };
            entries.push(Entry {
                key: Key {
                    scope: if world { Scope::World } else { Scope::Mission },
                    index,
                },
                object,
                owner: None,
            });
        }
        Ok(Self {
            source: plan.resources().clone(),
            entries,
            candidates: giants_game::declared_mesh_scene::prepare_platform_candidates(
                plan, terrain,
            ),
            started: false,
            steps: 0,
            last_sample: None,
            failure: None,
        })
    }
    pub(crate) fn start(
        &mut self,
        session: &mut Session,
        terrain: &giants_formats::gti::Terrain,
    ) -> Result<(), String> {
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        if self.started {
            return Ok(());
        }
        self.started = true;
        let result: Result<(), String> = (|| {
            let query = giants_game::terrain_queries::StaticTerrainQuery::new(
                terrain,
                1. / terrain.spacing,
                Precision::Double53,
            )?;
            for entry in &mut self.entries {
                let owner = session
                    .declaration(entry.key)
                    .ok_or("authored pub owner absent")?;
                session.prepare_ordinary_building_body(owner, entry.object, &self.source, |p| {
                    let sample = query.sample([p[0], p[1]])?;
                    Ok((sample.height, sample.plane))
                })?;
                entry.owner = Some(owner);
                println!(
                    "mission_pub_body_initialized owner={owner:?} object={} source_state=7 collision=current_rigid primary_phase_only=true world_ready=false",
                    entry.object
                );
            }
            Ok(())
        })();
        if let Err(error) = &result {
            self.failure = Some(error.clone());
        }
        result
    }
    pub(crate) fn frames(&self, session: &Session, ticket: u64) -> Result<Vec<Frame>, String> {
        self.entries
            .iter()
            .map(|entry| {
                let owner = entry.owner.ok_or("pub body not initialized")?;
                let body = session.ordinary_building_body(owner)?;
                let f = body.fields();
                Ok(Frame {
                    pose: Pose {
                        ticket,
                        key: (entry.key.scope == Scope::World, entry.key.index),
                        position: f.current,
                        scale: f.scale,
                        matrix: body.affine()?.matrix,
                    },
                    player: session.ordinary_building_player_mesh(owner)?,
                    configured_health: session
                        .get(owner)
                        .unwrap()
                        .construction_fields
                        .as_ref()
                        .unwrap()
                        .configuration[12],
                })
            })
            .collect()
    }
    fn platforms(&self, session: &Session) -> Result<Vec<Platform>, String> {
        self.entries
            .iter()
            .map(|entry| {
                let owner = entry.owner.ok_or("pub platform owner absent")?;
                let body = session.ordinary_building_body(owner)?;
                let f = body.fields();
                let player = session.ordinary_building_player_mesh(owner)?;
                let triangles = player
                    .triangles
                    .iter()
                    .map(|t| t.vertices)
                    .collect::<Vec<_>>();
                let bounds = [[f.bounds[0], f.bounds[1]], [f.bounds[3], f.bounds[4]]];
                let policy = giants_game::footprint_policy::select(
                    giants_game::footprint_policy::Input {
                        existing: false,
                        object_flags: f.object_flags,
                        collision_flags: f.collision_flags,
                        object_kind: 7,
                        building_query: 0,
                        zero_gate: f.plane[2],
                        z: f.current[2],
                        floor: f.floor,
                        ground_offset: f.ground_offset,
                    },
                    Precision::Double53,
                )?;
                let bits = match policy {
                    giants_game::footprint_policy::Plan::Absent => None,
                    giants_game::footprint_policy::Plan::Raster3 => {
                        giants_game::footprint_builder::platform_bits(
                            bounds,
                            &triangles,
                            Precision::Double53,
                        )?
                    }
                    _ => {
                        return Err("pub platform moved outside its acquired raster profile".into());
                    }
                };
                Ok(Platform {
                    id: entry.object,
                    active: f.object_flags,
                    personality: f.collision_flags,
                    bounds,
                    triangles,
                    bits,
                })
            })
            .collect()
    }
    pub(crate) fn advance(
        &mut self,
        session: &mut Session,
        sample: u64,
        seconds: f32,
        areas: &[giants_game::object_time::SlowArea],
        terrain: &giants_formats::gti::Terrain,
        objects: &[giants_game::actor_environment::Collider],
    ) -> Result<(), String> {
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        if !self.started {
            return Err("pub ordinary phase has not started".into());
        }
        if self.last_sample == Some(sample) {
            return Ok(());
        }
        if self
            .last_sample
            .is_some_and(|old| old.checked_add(1) != Some(sample))
        {
            return Err("pub shared-clock sample skipped or moved backwards".into());
        }
        let result: Result<(), String> = (|| {
            let query = giants_game::terrain_queries::StaticTerrainQuery::new(
                terrain,
                1. / terrain.spacing,
                Precision::Double53,
            )?;
            for entry in &self.entries {
                // Refresh from actual completed owners in their admitted order.
                let platforms = self.platforms(session)?;
                let unprepared = self
                    .candidates
                    .objects
                    .iter()
                    .filter(|o| !self.entries.iter().any(|e| e.object == o.fields.id))
                    .map(|o| giants_game::platform_queries::UnpreparedPlatform {
                        id: o.fields.id,
                        object_flags: o.fields.object_flags,
                        collision_flags: o.fields.collision_flags,
                        bounds_xy: [
                            [o.fields.bounds.min[0], o.fields.bounds.min[1]],
                            [o.fields.bounds.max[0], o.fields.bounds.max[1]],
                        ],
                    })
                    .collect::<Vec<_>>();
                let owner = entry.owner.ok_or("pub ordinary owner absent")?;
                let h = session
                    .get(owner)
                    .unwrap()
                    .construction_fields
                    .as_ref()
                    .unwrap();
                let context = h.dispatch_context;
                let mut services = giants_game::actor_environment::FinalizationServices {
                    actor: entry.object,
                    state: 7,
                    slope_parameter: f32::from_bits(h.configuration[11]),
                    physics: None,
                    skip_object_responses: false,
                    objects,
                    ground: |p| {
                        giants_game::physical_ground::Ground::from_sample(
                            query.sample(p)?,
                            terrain.width,
                        )
                    },
                    platform: |p: [f32; 3]| {
                        if !self.candidates.needs.is_empty() {
                            return Err("pub platform candidate lacks an acquired exclusion bound");
                        }
                        let prepared = platforms
                            .iter()
                            .map(|v| {
                                Ok(giants_game::platform_queries::PreparedPlatform {
                                    id: v.id,
                                    object_flags: v.active,
                                    collision_flags: v.personality,
                                    bounds_xy: v.bounds,
                                    footprint_hit: v
                                        .bits
                                        .as_ref()
                                        .map(|b| b.contains([p[0], p[1]], Precision::Double53))
                                        .transpose()?
                                        .unwrap_or(false),
                                    triangles: &v.triangles,
                                })
                            })
                            .collect::<Result<Vec<_>, &'static str>>()?;
                        giants_game::platform_queries::sample_with_unprepared(
                            p,
                            &prepared,
                            &unprepared,
                            Precision::Double53,
                        )
                    },
                    seconds: |position| {
                        giants_game::object_time::seconds(
                            seconds,
                            Some(giants_game::object_time::Sample {
                                owner: context,
                                position,
                            }),
                            None,
                            areas,
                        )
                    },
                    release: |identity: Option<u32>| {
                        if identity.is_none() {
                            Ok(())
                        } else {
                            Err("pub footprint release owner is not acquired".into())
                        }
                    },
                    statistics: None,
                };
                session.advance_ordinary_building_body(
                    owner,
                    self.steps,
                    giants_game::resident_motion::Time {
                        seconds,
                        areas,
                        control_factor: 1.,
                        running_factor: 1.,
                        skip_heading: false,
                        controlled_actor: false,
                    },
                    &mut services,
                )?;
            }
            self.steps = self
                .steps
                .checked_add(1)
                .ok_or("pub phase counter overflow")?;
            self.last_sample = Some(sample);
            Ok(())
        })();
        if let Err(error) = &result {
            self.failure = Some(error.clone());
        }
        result
    }
}
