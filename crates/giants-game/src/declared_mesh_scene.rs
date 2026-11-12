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
    prepare_profile(plan, terrain, Profile::Environment)
}

/// Initial static building meshes for the physical player's actual object walk.
/// That walk does not read generic-object platform footprints. This grants no
/// generic actor platform/radial service or live-world registration. Prototype
/// PS2 `MeccPhysicsExApplyLocation` is an unknown attribution search route.
pub fn prepare_players(plan: &Plan, terrain: &giants_formats::gti::Terrain) -> Prepared {
    prepare_profile(plan, terrain, Profile::Player)
}

/// Source-declared platform geometry only. These are unresolved candidates,
/// not constructed footprints or admitted live platform surfaces. A consumer
/// must not turn their presence into an empty successful platform query.
pub fn prepare_platform_candidates(
    plan: &Plan,
    terrain: &giants_formats::gti::Terrain,
) -> Prepared {
    prepare_profile(plan, terrain, Profile::PlatformCandidates)
}

/// Projectile list admission is independent of player physics and footprint
/// membership. These posed meshes do not grant lateral or platform response.
pub fn prepare_projectiles(plan: &Plan, terrain: &giants_formats::gti::Terrain) -> Prepared {
    prepare_profile(plan, terrain, Profile::Projectiles)
}

enum Profile {
    Environment,
    Player,
    PlatformCandidates,
    Projectiles,
}
fn prepare_profile(
    plan: &Plan,
    terrain: &giants_formats::gti::Terrain,
    profile: Profile,
) -> Prepared {
    let platforms = matches!(profile, Profile::PlatformCandidates);
    let projectiles = matches!(profile, Profile::Projectiles);
    let players = matches!(profile, Profile::Player);
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
                || if projectiles {
                    flags.personality & 0x20000 == 0
                } else {
                    flags.personality & 0x800 != 0 || (!players && flags.personality & 0x1008 == 0)
                }
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
            let mut initial = projection.placed_with_scale(pose, scale_override)?;
            if players && flags.personality & 0x1008 == 0 && !matches!(initial.state, 61 | 101) {
                // Widen the cold player cohort for the acquired building
                // families, not arbitrary staged actors or scenery callbacks.
                return Ok(None);
            }
            let extended_player = players && (flags.personality & 0x1008 == 0
                || flags.personality & 8 != 0 && flags.personality & (0x80000 | 0x100000) == 0);
            if extended_player && !matches!(initial.state, 0 | 61 | 101) {
                return Err("player mesh declaration requires its live pose/state owner".into());
            }
            if projectiles && !matches!(initial.state, 0 | 61 | 74 | 83 | 98 | 101 | 102) {
                return Err("projectile declaration requires its live pose/state owner".into());
            }
            if initial.state == 61 || initial.state == 101 && !context.classes.models
                .first_selection(source.class).ok_or("building selection absent")?
                .as_ref().map_err(Clone::clone)?.selection.animations.is_empty() {
                // The acquired first house callback selects health channel1.
                // Its descriptor bounds and posed collision model must agree
                // with the intact visual, rather than the serialized bind pose.
                initial.fields.animation_slot = 1;
            }
            if initial.fields.object_flags & 0x20 != 0 {
                return Err("source cache producer required".into());
            }
            if initial.fields.collision_flags & 8 != 0
                && initial.fields.collision_flags & (0x80000 | 0x100000) == 0
                && !platforms && !projectiles && !players
            {
                return Err("platform footprint preparation required".into());
            }
            // Bounds1 / Pair2 take precedence over Raster3. Their acquired
            // membership readers cannot return platform bit1, even when the
            // declaration also has bit8. This admits the independently selected
            // solid collision mesh, not a fabricated walkable roof. Raster3
            // candidates still require their actual footprint producer.
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
            let (name, mut mesh) = selection
                .geometry
                .ok_or("selected collision slot has no geometry")?;
            if matches!(prepared.state, 61 | 101) {
                let binding = context.classes.models.first_selection(source.class)
                    .ok_or("building model selection absent")?
                    .as_ref().map_err(Clone::clone)?;
                if !binding.selection.animations.is_empty() {
                    let mut health = crate::model_pose::Prepared::load_normal_initial(
                        context, prepared.state as u8, source.class, &binding.selection,
                    )?;
                    health.house_health(1.)?;
                    mesh.positions = health.positions(context, &name, &mesh)?;
                } else if context.archives.members.contains_key(&format!("{}.skn",name.to_ascii_lowercase())) {
                    return Err("skinned building requires its health animation resource".into());
                }
            }
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
                if !projectiles {
                    output.player_objects.push(player_object);
                }
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

#[cfg(test)]
mod projectile_tests {
    #[test]
    #[ignore = "requires GIANTS_BUILDING_ROOT; source projectile meshes only, no runtime hits"]
    fn installed_projectile_admission_is_distinct_from_player_footprints() {
        let root = std::path::PathBuf::from(std::env::var("GIANTS_BUILDING_ROOT").unwrap());
        let menu = crate::mission_selection::MissionMenu::parse(
            &std::fs::read(root.join("Bin/w_Story2.bin")).unwrap(),
            0,
            true,
        )
        .unwrap();
        let packet =
            crate::mission_loading::prepare(&root, menu.request("Story2", 0).unwrap()).unwrap();
        let plan = packet.instance_plan(&root, "English").unwrap();
        let world = packet.world_asset(&root).unwrap();
        let physical = super::prepare(&plan, &world.terrain);
        let players = super::prepare_players(&plan, &world.terrain);
        let projectiles = super::prepare_projectiles(&plan, &world.terrain);
        assert!(projectiles.player_objects.is_empty());
        for class in [512, 544, 549, 1050] {
            assert!(players.records.iter().any(|r| r.class == class));
        }
        for old in &physical.objects {
            let player = players
                .objects
                .iter()
                .find(|p| p.fields.id == old.fields.id)
                .unwrap();
            assert_eq!(player.triangles, old.triangles);
        }
        for class in [512, 544, 549, 1050] {
            let count = projectiles
                .records
                .iter()
                .filter(|r| r.class == class)
                .count();
            assert!(
                count > 0,
                "missing projectile class{class}; {} declarations still need owners",
                projectiles.needs.len()
            );
            assert!(!physical.records.iter().any(|r| r.class == class));
        }
        for (record, object) in projectiles.records.iter().zip(&projectiles.objects) {
            assert_ne!(object.fields.collision_flags & 0x20000, 0);
            assert!(!object.triangles.is_empty());
            if record.class == 1050 {
                let f = record.instance.fields;
                let matrix = crate::uncached_pose::matrix(
                    crate::uncached_pose::Pose {
                        position: f.current,
                        angles: f.angles,
                        scale: f.scale,
                    },
                    crate::arithmetic::Precision::Double53,
                )
                .unwrap();
                let pose = crate::collision_world::SourcePose {
                    basis: [
                        matrix[0], matrix[1], matrix[2], matrix[4], matrix[5], matrix[6],
                        matrix[8], matrix[9], matrix[10],
                    ],
                    position: f.current,
                };
                let stage =
                    crate::building_geometry::Stage::health(plan.resources(), record.class, 1.)
                        .unwrap();
                assert_eq!(object.triangles, stage.placed(pose).unwrap());
            }
        }
    }
}
