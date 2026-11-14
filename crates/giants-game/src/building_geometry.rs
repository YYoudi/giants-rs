//! House health animation shared by rendering and the dedicated collision slot.
//! No thresholds, LOD replacements, hit points or meshes are invented here.
use crate::{arithmetic::Precision, scene_resources::SceneResources};
use std::collections::BTreeMap;

/// Local source positions for every loaded render LOD, plus source-ordered
/// triangles from the independent collision selector. World placement is owned
/// by the instance. Keeping these separate prevents a camera LOD changing hits.
pub struct Stage {
    pub fraction: f32,
    pub positions: BTreeMap<usize, Vec<[f32; 3]>>,
    pub collision_slot: usize,
    pub collision: Vec<[[f32; 3]; 3]>,
    pub identities: Vec<(u32, u32)>,
}
impl Stage {
    pub fn house(source: &SceneResources, class: i32, fraction: f32) -> Result<Self, String> {
        if source
            .class_state_seed(class)
            .map_err(|e| e.to_string())?
            .storage
            .state()
            != 61
        {
            return Err("house geometry requires source state61".into());
        }
        Self::health(source, class, fraction)
    }
    /// Acquired house/state101 and the admitted state98 classes use the same
    /// numeric health channel. Each caller supplies its own acquired fraction;
    /// barracks retain their distinct arithmetic. Unskinned models stay rigid.
    pub fn health(source: &SceneResources, class: i32, fraction: f32) -> Result<Self, String> {
        let state = source
            .class_state_seed(class)
            .map_err(|e| e.to_string())?
            .storage
            .state();
        if !(matches!(state, 61 | 101) || state == 98 && matches!(class, 1060 | 1226))
            || !fraction.is_finite()
            || !(0. ..=1.).contains(&fraction)
        {
            return Err("building health geometry outside acquired state/fraction profile".into());
        }
        let binding = source
            .classes
            .models
            .first_selection(class)
            .ok_or("house model table absent")?
            .as_ref()
            .map_err(Clone::clone)?;
        let pose = if binding.selection.animations.is_empty() {
            None
        } else {
            let mut pose = crate::model_pose::Prepared::load_normal_initial(
                source,
                state,
                class,
                &binding.selection,
            )?;
            pose.house_health(fraction)?;
            Some(pose)
        };
        Self::prepare(source, class, fraction, pose)
    }
    /// Initial idle tree pose, independent of health. The ordinary foliage
    /// renderer keeps its existing wind owner; later collision/wind cadence is
    /// not certified by this snapshot. Never use house health stages on a tree.
    pub fn tree_snapshot(source: &SceneResources, class: i32) -> Result<Self, String> {
        if !matches!(class, 102 | 664)
            || source
                .class_state_seed(class)
                .map_err(|e| e.to_string())?
                .storage
                .state()
                != 60
        {
            return Err("tree geometry outside acquired palm/stump profile".into());
        }
        let binding = source
            .classes
            .models
            .first_selection(class)
            .ok_or("tree model table absent")?
            .as_ref()
            .map_err(Clone::clone)?;
        let pose = if binding.selection.animations.is_empty() {
            None
        } else {
            Some(crate::model_pose::Prepared::load_normal_initial(
                source,
                60,
                class,
                &binding.selection,
            )?)
        };
        Self::prepare(source, class, 1., pose)
    }
    fn prepare(
        source: &SceneResources,
        class: i32,
        fraction: f32,
        pose: Option<crate::model_pose::Prepared>,
    ) -> Result<Self, String> {
        let binding = source
            .classes
            .models
            .first_selection(class)
            .ok_or("building model table absent")?
            .as_ref()
            .map_err(Clone::clone)?;
        let collision_slot = binding
            .selection
            .table()
            .map_err(str::to_owned)?
            .collision_slot()
            .map_err(str::to_owned)?;
        let mut positions = BTreeMap::new();
        let mut collision = None;
        let mut identities = vec![];
        for (slot, declaration) in binding.selection.slots.iter().enumerate() {
            if declaration.name.is_none() || (declaration.threshold < 0. && slot != collision_slot)
            {
                continue;
            }
            let (name, mut mesh) = source
                .model_mesh_slot(class, slot)
                .map_err(|e| e.to_string())?
                .ok_or("house model slot absent")?;
            if let Some(pose) = &pose {
                mesh.positions = pose.positions(source, &name, &mesh)?;
            } else if source
                .archives
                .members
                .contains_key(&format!("{}.skn", name.to_ascii_lowercase()))
            {
                return Err("skinned building requires its health animation resource".into());
            }
            positions.insert(slot, mesh.positions.clone());
            if slot == collision_slot {
                identities = mesh
                    .subobjects
                    .iter()
                    .enumerate()
                    .flat_map(|(part, sub)| {
                        (0..sub.triangles.len()).map(move |tri| (part as u32, tri as u32 + 1))
                    })
                    .collect();
                collision = Some(
                    crate::collision_geometry::world_triangles(
                        &mesh,
                        crate::collision_world::SourcePose {
                            basis: [1., 0., 0., 0., 1., 0., 0., 0., 1.],
                            position: [0.; 3],
                        },
                        Precision::Double53,
                    )
                    .map_err(str::to_owned)?,
                );
            }
        }
        Ok(Self {
            fraction,
            positions,
            collision_slot,
            collision: collision.ok_or("house collision geometry absent")?,
            identities,
        })
    }

    pub fn placed(
        &self,
        pose: crate::collision_world::SourcePose,
    ) -> Result<Vec<[[f32; 3]; 3]>, String> {
        self.collision
            .iter()
            .map(|tri| {
                let mut result = [[0.; 3]; 3];
                for (out, point) in result.iter_mut().zip(tri) {
                    *out = pose
                        .transform(*point, Precision::Double53)
                        .map_err(str::to_owned)?;
                }
                Ok(result)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires GIANTS_BUILDING_ROOT; state101 health resources only, no gameplay"]
    fn installed_state101_health_preserves_rigid_shapes_and_animated_damage() {
        let root = std::path::PathBuf::from(std::env::var("GIANTS_BUILDING_ROOT").unwrap());
        let resources = SceneResources::load(&root, "w_Story2.bin", "English").unwrap();
        for class in [507, 514, 515, 1050] {
            let intact = Stage::health(&resources, class, 1.).unwrap();
            let damaged = Stage::health(&resources, class, 0.5).unwrap();
            assert!(!intact.collision.is_empty());
            assert_eq!(intact.collision_slot, damaged.collision_slot);
            assert_eq!(intact.identities, damaged.identities);
            if class == 1050 {
                assert_ne!(intact.positions, damaged.positions);
            } else {
                assert_eq!(intact.positions, damaged.positions);
                assert_eq!(intact.collision, damaged.collision);
            }
            assert!(Stage::house(&resources, class, 1.).is_err());
        }
    }
    #[test]
    #[ignore = "requires GIANTS_BUILDING_ROOT; source preparation only, not gameplay"]
    fn installed_houses_share_health_pose_with_the_declared_collision_selector() {
        let root = std::path::PathBuf::from(std::env::var("GIANTS_BUILDING_ROOT").unwrap());
        let menu = crate::mission_selection::MissionMenu::parse(
            &std::fs::read(root.join("Bin/w_Story1.bin")).unwrap(),
            0,
            true,
        )
        .unwrap();
        let packet =
            crate::mission_loading::prepare(&root, menu.request("Story1", 0).unwrap()).unwrap();
        let plan = packet.instance_plan(&root, "English").unwrap();
        let world = packet.world_asset(&root).unwrap();
        let colliders = crate::declared_mesh_scene::prepare(&plan, &world.terrain);
        for class in [590, 591, 592, 593, 1002, 1003] {
            let intact = Stage::house(plan.resources(), class, 1.).unwrap();
            let damaged = Stage::house(plan.resources(), class, 0.5).unwrap();
            assert_ne!(intact.positions[&0], damaged.positions[&0]);
            assert_ne!(intact.collision, damaged.collision);
            assert_eq!(intact.collision.len(), intact.identities.len());
            let source = plan.resources().collision_model_mesh(class).unwrap();
            assert_eq!(intact.collision_slot, source.slot);
            let selected = colliders
                .records
                .iter()
                .zip(&colliders.objects)
                .filter(|(record, _)| record.class == class)
                .collect::<Vec<_>>();
            assert!(
                !selected.is_empty(),
                "declared house class {class} lost its collider"
            );
            for (record, object) in selected {
                assert_eq!(record.instance.fields.animation_slot, 1);
                assert!(crate::object_segment::projectile_candidate(
                    u32::MAX,
                    1,
                    object.fields.id,
                    object.fields.parent,
                    object.fields.object_flags,
                    object.fields.collision_flags,
                ));
                let f = record.instance.fields;
                let m = crate::uncached_pose::matrix(
                    crate::uncached_pose::Pose {
                        position: f.current,
                        angles: f.angles,
                        scale: f.scale,
                    },
                    Precision::Double53,
                )
                .unwrap();
                let posed = intact
                    .placed(crate::collision_world::SourcePose {
                        basis: [m[0], m[1], m[2], m[4], m[5], m[6], m[8], m[9], m[10]],
                        position: f.current,
                    })
                    .unwrap();
                assert_eq!(posed, object.triangles);
                let player = colliders
                    .player_objects
                    .iter()
                    .find(|o| o.fields.id == object.fields.id)
                    .unwrap();
                assert_eq!(player.triangles.len(), posed.len());
                for (tri, vertices) in player.triangles.iter().zip(posed) {
                    assert_eq!(tri.vertices, vertices);
                }
            }
        }
        // A source state0 wall remains solid and targetable without receiving
        // an invented house damage callback. Raster3 dependencies stay visible.
        assert!(
            colliders
                .records
                .iter()
                .any(|r| r.class == 553 && r.instance.state == 0)
        );
        assert!(
            colliders
                .needs
                .iter()
                .any(|n| n.reason == "platform footprint preparation required")
        );
    }
}
