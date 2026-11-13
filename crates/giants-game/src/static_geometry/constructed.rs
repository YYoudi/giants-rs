//! Render-resource projection for an already constructed rigid owner.
//! Prototype PS2 `object_render__FP6OBJECTiG7f32quat` / `fx_move` are unknown
//! attribution routes.
//! This reuses the existing source material/LOD path; it is not an actor
//! constructor, placement callback, collision registration or ready-world proof.
use super::{PosePolicy, Scene, SourceKey};
use crate::scene_resources::SceneResources;

impl Scene {
    /// Prepare one actual static replacement or kind8 debris draw. The caller
    /// supplies its current acquired world matrix and separate scalar LOD scale.
    /// No authored mission placement, animation clock or source flag is invented.
    pub fn constructed_rigid_draw(
        resources: &SceneResources,
        key: SourceKey,
        class: i32,
        matrix: [f32; 16],
        scale: f32,
    ) -> Result<Self, String> {
        if !matrix.into_iter().all(f32::is_finite) || !scale.is_finite() || scale <= 0. {
            return Err("constructed rigid draw has invalid current pose/scale".into());
        }
        let definition = resources
            .classes
            .models
            .first_program(class)
            .ok_or("constructed rigid class absent")?;
        let program = definition.program.as_ref().map_err(Clone::clone)?;
        let seed = crate::state_storage::Seed::from_program_for(
            program,
            crate::state_storage::Profile::Debug,
            crate::arithmetic::Precision::Double53,
        )
        .map_err(|e| e.to_string())?;
        let state = seed.storage.state();
        let kind = program
            .records
            .iter()
            .rev()
            .find(|r| r.opcode == 0x36)
            .and_then(|r| r.bytes.get(1))
            .copied();
        let idle_stump = state == 60
            && class == 664
            && seed
                .storage
                .data()
                .is_some_and(|d| d.len() == 76 && d.iter().all(|b| *b == 0));
        if state != 0 && !(state == 78 && kind == Some(8)) && !idle_stump {
            return Err("constructed rigid draw outside static/state78-kind8 profile".into());
        }
        let binding = resources
            .classes
            .models
            .first_selection(class)
            .ok_or("constructed rigid model declaration absent")?
            .as_ref()
            .map_err(Clone::clone)?;
        // The acquired vertex updater gates deformation on skeleton presence,
        // before channel/LOD selection. An ANM declaration alone does not turn
        // an entirely unskinned model set into animated vertex geometry.
        if binding
            .selection
            .slots
            .iter()
            .filter_map(|s| s.name.as_ref())
            .any(|name| {
                resources
                    .archives
                    .members
                    .contains_key(&format!("{}.skn", name.to_ascii_lowercase()))
            })
        {
            return Err("constructed rigid draw requires its actual skinned pose owner".into());
        }
        // This value object carries supplied constructor arguments into the
        // existing class/resource loader; it is never serialized as a mission
        // record or used to claim an authored actor birth.
        let placement = giants_formats::mission::Placement {
            source_offset: key.1,
            object_type: u32::try_from(class).map_err(|_| "negative rigid class")?,
            position: [matrix[3], matrix[7], matrix[11]],
            yaw: 0.,
            pitch_roll: None,
            scale: Some(scale),
        };
        let input = resources
            .placement_input(&placement)
            .map_err(|e| e.to_string())?;
        let mut scene = Self::default();
        scene.add(
            key,
            &input,
            matrix,
            resources,
            PosePolicy::NormalInitial,
            None,
        )?;
        if scene.instances.len() != 1 || scene.assets.is_empty() {
            return Err("constructed rigid source has no admitted draw resources".into());
        }
        Ok(scene)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "requires GIANTS_BUILDING_ROOT; render resources only, no constructor/world/GPU claim"]
    fn installed_guard_replacement_and_lit_debris_keep_their_source_render_profiles() {
        let root = std::path::PathBuf::from(std::env::var("GIANTS_BUILDING_ROOT").unwrap());
        let resources =
            crate::scene_resources::SceneResources::load(&root, "w_Story2.bin", "English").unwrap();
        let matrix = crate::object_transform::euler([10., 20., 30.], [45., 10., -5.], 2.).unwrap();
        for class in [1127, 1129, 1382, 1383, 1384, 1385, 1386] {
            let scene =
                super::Scene::constructed_rigid_draw(&resources, (false, 7), class, matrix, 2.)
                    .unwrap();
            assert_eq!(scene.instances[0].matrix, matrix);
            assert_eq!(scene.instances[0].scale, 2.);
            assert!(scene.assets.iter().all(|((owner, _), _)| *owner == class));
            if class >= 1382 {
                assert!(scene.assets.values().all(|a| a.normals.is_some()));
            }
        }
        for class in [593, 1050] {
            assert!(
                super::Scene::constructed_rigid_draw(&resources, (false, 7), class, matrix, 2.)
                    .is_err()
            );
        }
    }
}
