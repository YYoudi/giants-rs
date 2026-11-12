//! Development-only mesh inputs from declared rigid poses. No campaign readiness,
//! live callbacks, part masks, radial responses or native list scheduling.
use crate::{
    arithmetic::Precision,
    collider_init::{self, DeclaredPose, GeometryResources, SourceTransform},
    collision_world::SourcePose,
    instances::Plan,
    mecc_collision_candidates::Object,
    mecc_mesh_dispatch::MeshObject,
    scene_resources::SourcePlacement,
    volume_queries::Bounds,
};

pub struct Record {
    pub key: (bool, usize),
    pub class: i32,
    pub name: String,
    pub slot: usize,
    pub instance: collider_init::Instance,
    /// Retained scalar override; not the parent link used by mesh candidates.
    pub mission_word_174: Option<u32>,
}
pub struct Need {
    pub key: (bool, usize),
    pub reason: String,
}
pub struct Prepared {
    pub objects: Vec<MeshObject>,
    pub records: Vec<Record>,
    pub needs: Vec<Need>,
    pub player_objects: Vec<crate::player_world_collision::Object>,
    /// Declared fresh profiles selected ahead of raster3. Their footprint
    /// membership reader cannot produce the platform bit; no geometry guessed.
    pub platform_non_raster: usize,
}

pub fn prepare(plan: &Plan, terrain: &giants_formats::gti::Terrain) -> Prepared {
    prepare_profile(plan, terrain, false)
}

/// Source-declared platform geometry only. These are unresolved candidates,
/// not constructed footprints or admitted live platform surfaces. A consumer
/// must not turn their presence into an empty successful platform query.
pub fn prepare_platform_candidates(
    plan: &Plan,
    terrain: &giants_formats::gti::Terrain,
) -> Prepared {
    prepare_profile(plan, terrain, true)
}

fn prepare_profile(
    plan: &Plan,
    terrain: &giants_formats::gti::Terrain,
    platforms: bool,
) -> Prepared {
    let mut output = Prepared {
        objects: vec![],
        records: vec![],
        needs: vec![],
        player_objects: vec![],
        platform_non_raster: 0,
    };
    let mut visit = |key: (bool, usize),
                     source: Result<&SourcePlacement, String>,
                     pose: DeclaredPose,
                     scale_override: Option<f32>| {
        let result = (|| -> Result<Option<(Record, MeshObject, crate::player_world_collision::Object)>, String> {
            let source = source?;
            let flags = source.direct_flags;
            if flags.active & 0x4000_c000 != 0
                || flags.personality & 0x800 != 0
                || flags.personality & 0x1008 == 0
            {
                return Ok(None);
            }
            if platforms && flags.personality & 8 == 0 {
                return Ok(None);
            }
            if platforms && flags.personality & (0x80000 | 0x100000) != 0 {
                // Footprint policy selects Bounds1 / Pair2 before Raster3.
                // Neither reader profile can return platform bit1. A missing
                // or height-refused footprint likewise cannot contribute it.
                output.platform_non_raster += 1;
                return Ok(None);
            }
            let context = plan.resources();
            let projection =
                collider_init::cold_projection(context, source.class).map_err(|e| e.to_string())?;
            let initial = projection.placed_with_scale(pose, scale_override)?;
            if initial.fields.object_flags & 0x20 != 0 {
                return Err("source cache producer required".into());
            }
            if initial.fields.collision_flags & 8 != 0 && !platforms {
                return Err("platform footprint preparation required".into());
            }
            let radius = context
                .collision_resource_bounds(source.class, Some(1))
                .map_err(|e| e.to_string())?;
            let slot = u8::try_from(initial.fields.animation_slot)
                .map_err(|_| "animation slot outside prepared profile")?;
            let active = context
                .collision_resource_bounds(source.class, Some(slot))
                .map_err(|e| e.to_string())?;
            let query = crate::terrain_queries::StaticTerrainQuery::new(
                terrain,
                1. / terrain.spacing,
                Precision::Double53,
            )?;
            let prepared = collider_init::initialize_geometry(
                initial,
                GeometryResources {
                    radius,
                    current_slot: u32::from(slot),
                    current_auxiliary: active.animation,
                },
                SourceTransform::Uncached,
                Precision::Double53,
                |p| {
                    let sample = query.sample([p[0], p[1]])?;
                    Ok((sample.height, sample.plane))
                },
            )?
            .instance;
            let selection = context
                .collision_model_mesh(source.class)
                .map_err(|e| e.to_string())?;
            let (name, mesh) = selection
                .geometry
                .ok_or("selected collision slot has no geometry")?;
            let m = crate::uncached_pose::matrix(
                crate::uncached_pose::Pose {
                    position: prepared.fields.current,
                    angles: prepared.fields.angles,
                    scale: prepared.fields.scale,
                },
                Precision::Double53,
            )?;
            let transform = SourcePose {
                basis: [m[0], m[1], m[2], m[4], m[5], m[6], m[8], m[9], m[10]],
                position: prepared.fields.current,
            };
            let triangles =
                crate::collision_geometry::world_triangles(&mesh, transform, Precision::Double53)?;
            let b = prepared.fields.bounds;
            let id = u32::try_from(key.1)
                .map_err(|_| "collider identity overflow")?
                .checked_add(2)
                .ok_or("collider identity overflow")?
                | if key.0 { 0x4000_0000 } else { 0 };
            let object = MeshObject {
                fields: Object {
                    id,
                    object_flags: prepared.fields.object_flags,
                    collision_flags: prepared.fields.collision_flags,
                    has_model: prepared.fields.model != 0,
                    parent: 0,
                    bounds: Bounds {
                        min: [b[0], b[1], b[2]],
                        max: [b[3], b[4], b[5]],
                    },
                },
                triangles,
            };
            let identities = mesh.subobjects.iter().enumerate().flat_map(|(part,subobject)| {
                (0..subobject.triangles.len()).map(move |tri|(part as u32,tri as u32+1))
            });
            let player_object = crate::player_world_collision::Object {
                fields:object.fields,response_flags:prepared.fields.response_flags,
                triangles:object.triangles.iter().zip(identities).map(|(vertices,(part,ordinal))| {
                    crate::player_mesh_contact::Triangle {vertices:*vertices,part,ordinal}
                }).collect(),
            };
            Ok(Some((
                Record {
                    key,
                    class: source.class,
                    name,
                    slot: selection.slot,
                    instance: prepared,
                    mission_word_174: source.mission_word_174,
                },
                object,
                player_object,
            )))
        })();
        match result {
            Ok(Some((record, object, player_object))) => {
                output.player_objects.push(player_object);
                output.objects.push(object);
                output.records.push(record);
            }
            Ok(None) => {}
            Err(reason) => output.needs.push(Need { key, reason }),
        }
    };
    for entry in &plan.entries {
        let p = &entry.serialized;
        visit(
            (false, entry.index),
            entry.source.as_ref().map_err(Clone::clone),
            DeclaredPose {
                class: p.object_type,
                position: p.position,
                yaw: p.yaw,
                pitch_roll: p.pitch_roll,
            },
            p.scale,
        );
    }
    for (index, entry) in plan.world_entries.iter().enumerate() {
        let p = &entry.declaration;
        visit(
            (true, index),
            entry.class_input(),
            DeclaredPose {
                class: p.object_type,
                position: p.position,
                yaw: p.yaw,
                pitch_roll: p.pitch_roll,
            },
            p.scale,
        );
    }
    output
}
