//! Initial mission-player resources. This does not execute special children,
//! continuity, team formation, callbacks or grant playable readiness.
//! Weapon decoding explicitly uses the debug compatibility profile.
use crate::{instances::Plan, player_equipment, player_start};
use giants_formats::{anm::Animation, mission::Mission, world::WorldConfig};
use std::path::Path;

pub struct Prepared {
    pub start: player_start::Declared,
    pub requested_companions: i32,
    pub equipment: player_equipment::Initial,
    pub body: crate::cast::Body,
    pub animation: Animation,
    pub tree: String,
    pub positions: Vec<[f32; 3]>,
    pub matrix: [f32; 16],
    pub scale: f32,
    pub parameters: crate::mecc_initial::Parameters,
    pub state_seed: crate::state_storage::Seed,
    /// Captured identities at profile-correct offsets, not bound live pointers.
    pub state_bindings: Vec<crate::model_registry::StateBinding>,
    /// Owned extra CPU specifications; failures stay explicit. Not live state
    /// pointers or a rendered first-person view.
    pub state_specifications: crate::state_resources::Specifications,
    pub subjective: Result<crate::subjective_player::Resources, String>,
    pub weapon: crate::held_weapon::Weapon,
    pub weapon_normals: Vec<[f32; 3]>,
    pub weapon_matrix: [f32; 16],
    pub lights: Vec<crate::lighting::Directional>,
}
impl Prepared {
    pub fn load(
        root: &Path,
        plan: &Plan,
        world: &WorldConfig,
        mission: &Mission,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let start = player_start::declared(plan, world, mission)?;
        let command = mission
            .starts
            .iter()
            .find(|s| s.source_offset == start.start_offset)
            .ok_or("selected player start command missing")?;
        let requested_companions = command.words[1] as i32;
        if !(0..=4).contains(&requested_companions) {
            return Err("initial team quantity outside measured Mecc profile".into());
        }
        let resources = plan.resources();
        let program = resources
            .classes
            .models
            .first_program(start.kind)
            .ok_or("player class program missing")?
            .program
            .as_ref()
            .map_err(Clone::clone)?;
        let state_seed = crate::state_storage::Seed::from_program_for(
            program,
            crate::state_storage::Profile::Debug,
            crate::arithmetic::Precision::Double53,
        )?;
        if state_seed.storage.state() != 2 {
            return Err("player class does not initialize the controlled Mecc state".into());
        }
        let state_bindings =
            resources.class_state_bindings_for(start.kind, crate::state_storage::Profile::Debug)?;
        let state_specifications: std::collections::BTreeMap<_, _> = state_bindings
            .iter()
            .filter(|binding| matches!(binding.key, crate::state_storage::ReferenceKey::Spec(_)))
            .map(|binding| {
                (
                    binding.offset,
                    crate::specification::Prepared::load_state_registered(
                        resources,
                        start.kind,
                        crate::state_storage::Profile::Debug,
                        binding.offset,
                    )
                    .map(std::sync::Arc::new),
                )
            })
            .collect();
        let scale = program.declared_scale()?;
        let matrix = crate::object_transform::euler(start.position, start.angles, scale)?;
        let body = crate::cast::load_registered_body(resources, start.kind, 0)?;
        if body.mesh.normal_count != 0 {
            return Err("initial mission player outside controlled unlit body profile".into());
        }
        let selection = resources
            .classes
            .models
            .first_selection(start.kind)
            .ok_or("player specification missing")?
            .as_ref()
            .map_err(Clone::clone)?;
        let clips: Vec<_> = selection
            .selection
            .animations
            .iter()
            .filter(|a| a.slot == 1)
            .collect();
        if clips.len() != 1 {
            return Err("initial player animation missing or ambiguous".into());
        }
        let animation = resources.animation(&clips[0].name)?;
        let tree = animation
            .trees
            .iter()
            .find(|t| t.offset == animation.default_tree)
            .ok_or("initial player default animation tree missing")?
            .name
            .clone();
        let positions =
            crate::skinning::positions(&body.skin, &animation, &tree, animation.timeline[0])?;
        let globals =
            giants_formats::global_data::parse(&std::fs::read(root.join("Bin/GData.gbn"))?)?;
        let parameters = crate::mecc_initial::Parameters::from_globals(&globals)?;
        let equipment = player_equipment::Initial::non_base(true);
        let catalog = giants_formats::weapons::parse_with_profile(
            &std::fs::read(root.join("Bin/weapondefs.bin"))?,
            giants_formats::weapons::Profile::Debug,
        )?;
        let weapon = crate::held_weapon::Weapon::load(resources, &catalog, equipment.weapons[0])?;
        let subjective = state_specifications
            .get(&0x70)
            .ok_or_else(|| "subjective state specification absent".to_owned())
            .and_then(|s| s.as_ref().map_err(Clone::clone))
            .and_then(|spec| {
                crate::subjective_player::Resources::load(
                    resources,
                    &state_seed,
                    spec,
                    &catalog,
                    equipment.weapons[0],
                )
            });
        let lights = crate::lighting::first_world(world)?;
        let weapon_normals = crate::body_pipeline::prepare_normals(
            &weapon.model.mesh,
            &weapon.model.mesh.positions,
            false,
        )?;
        let weapon_matrix =
            weapon.matrix(&animation, &tree, animation.timeline[0], matrix, scale)?;
        Ok(Self {
            start,
            requested_companions,
            equipment,
            body,
            animation,
            tree,
            positions,
            matrix,
            scale,
            parameters,
            state_seed,
            state_bindings,
            state_specifications,
            subjective,
            weapon,
            weapon_normals,
            weapon_matrix,
            lights,
        })
    }
    /// Source skin positions expanded into the common Bevy coordinate adapter.
    pub fn vertices(&self, part: usize) -> Vec<[f32; 3]> {
        self.body.mesh.subobjects[part]
            .triangles
            .iter()
            .flatten()
            .map(|id| {
                let i = usize::from(self.body.mesh.indices[usize::from(*id)]);
                let p = self.positions[i];
                let m = self.matrix;
                let s = std::array::from_fn::<_, 3, _>(|r| {
                    m[r * 4] * p[0] + m[r * 4 + 1] * p[1] + m[r * 4 + 2] * p[2] + m[r * 4 + 3]
                });
                [s[0], s[2], -s[1]]
            })
            .collect()
    }
}
