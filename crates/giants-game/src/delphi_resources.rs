//! Declaration-backed Delphi resources, before character initialization.
//! No constructor children, physics, camera, controls, combat, water behavior or
//! gameplay admission is supplied by this preparation stage.
use crate::{
    actor_fields::Fields,
    arithmetic::Precision,
    instances::Plan,
    player_start, specification, state_resources,
    state_storage::{Profile, Seed},
};
use giants_formats::{mission::Mission, world::WorldConfig};
use std::sync::Arc;

pub struct Resources {
    pub start: player_start::Declared,
    /// All four command words are retained; equipment/continuity semantics are
    /// not inferred from a successful resource load.
    pub start_words: [u32; 4],
    pub fields: Fields,
    pub seed: Seed,
    pub specification: Arc<specification::Prepared>,
    pub states: Arc<state_resources::Prepared>,
}
impl Resources {
    pub fn load(plan: &Plan, world: &WorldConfig, mission: &Mission) -> Result<Self, String> {
        let start = player_start::declared_for_kind(plan, world, mission, 3)?;
        let start_words = mission.starts[0].words;
        let resources = plan.resources();
        let program = resources
            .classes
            .models
            .first_program(3)
            .ok_or("Delphi class declaration absent")?
            .program
            .as_ref()
            .map_err(Clone::clone)?;
        let seed = Seed::from_program_for(program, Profile::Debug, Precision::Double53)?;
        if seed.storage.state() != 3 || !seed.diagnostics.is_empty() {
            return Err("Delphi resource preparation requires the declared state 3 profile".into());
        }
        let specification = Arc::new(specification::Prepared::load_registered(resources, 3)?);
        if !specification.needs.is_empty() {
            return Err("Delphi specification has unresolved resource preparation needs".into());
        }
        let fields = Fields::load(resources, 3, &specification)?.placed(
            crate::collider_init::DeclaredPose {
                class: 3,
                position: start.position,
                yaw: start.angles[0],
                pitch_roll: Some([start.angles[1], start.angles[2]]),
            },
        )?;
        let states = Arc::new(state_resources::Prepared::load(
            resources,
            3,
            Profile::Debug,
        )?);
        if !states.needs().is_empty() {
            return Err("Delphi state resources have unresolved preparation needs".into());
        }
        Ok(Self {
            start,
            start_words,
            fields,
            seed,
            specification,
            states,
        })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "Requires GIANTS_DELPHI_ROOT with user-supplied game data; resource preparation only"]
    fn source_delphi_start_and_resources_remain_separate_from_mecc() {
        let root = std::path::PathBuf::from(std::env::var("GIANTS_DELPHI_ROOT").unwrap());
        let source = crate::world_source::resolve(&root, "Rstory1").unwrap();
        let menu = crate::mission_selection::MissionMenu::parse(
            &std::fs::read(root.join("Bin").join(source.catalog)).unwrap(),
            u32::MAX,
            true,
        )
        .unwrap();
        let packet =
            crate::mission_loading::prepare(&root, menu.request("Rstory1", 2).unwrap()).unwrap();
        let plan = packet.instance_plan(&root, "English").unwrap();
        let world = packet.world.as_ref().unwrap();
        let mission = packet.mission.as_ref().unwrap();
        assert!(crate::player_initial::Prepared::load(&root, &plan, world, mission).is_err());
        let prepared = super::Resources::load(&plan, world, mission).unwrap();
        assert_eq!(prepared.start.kind, 3);
        assert_eq!(prepared.fields.class, 3);
        assert_eq!(prepared.fields.position, prepared.start.position);
        assert_eq!(prepared.fields.angles, prepared.start.angles);
        assert_eq!(prepared.seed.storage.state(), 3);
        assert_eq!(
            prepared.seed.storage.profile(),
            crate::state_storage::Profile::Debug
        );
        assert!(!prepared.seed.unexecuted.is_empty());
        assert_eq!(prepared.specification.class, 3);
        assert!(prepared.specification.needs.is_empty());
        assert!(prepared.states.needs().is_empty());
        assert!(
            prepared
                .specification
                .animations
                .contains_key(&(prepared.fields.channels[0][0] as u8))
        );
    }
}
