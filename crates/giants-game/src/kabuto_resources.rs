//! Acquired cold Kabuto class resources and ordinary actor construction.
//! Special/player initialization, marker relocation, movement, growth and live
//! mission admission remain separate services.
use crate::{
    actor_fields::Fields,
    arithmetic::Precision,
    instances::Plan,
    model_registry::StateBinding,
    object_lists::DirectFlags,
    player_start,
    resident_states::{Construction, Id, Session},
    specification, state_resources,
    state_storage::{Profile, Seed},
};
use giants_formats::{mission::Mission, world::WorldConfig};
use std::sync::Arc;

pub struct Resources {
    pub start: player_start::Declared,
    pub start_words: [u32; 4],
    pub specification: Arc<specification::Prepared>,
    pub states: Arc<state_resources::Prepared>,
    fields: Fields,
    seed: Seed,
    bindings: Vec<StateBinding>,
    flags: DirectFlags,
}
impl Resources {
    pub fn load(plan: &Plan, world: &WorldConfig, mission: &Mission) -> Result<Self, String> {
        let start = player_start::declared_for_kind(plan, world, mission, 1)?;
        let resources = plan.resources();
        let program = resources
            .classes
            .models
            .first_program(1)
            .ok_or("Kabuto class declaration absent")?
            .program
            .as_ref()
            .map_err(Clone::clone)?;
        let seed = Seed::from_program_for(program, Profile::Debug, Precision::Double53)?;
        if seed.storage.state() != 1 || !seed.diagnostics.is_empty() {
            return Err("cold Kabuto requires its declared debug state 1 profile".into());
        }
        let specification = Arc::new(specification::Prepared::load_registered(resources, 1)?);
        if !specification.needs.is_empty() {
            return Err("Kabuto specification has unresolved preparation needs".into());
        }
        let fields = Fields::load(resources, 1, &specification)?;
        let bindings = resources.class_state_bindings_for(1, Profile::Debug)?;
        let states = Arc::new(state_resources::Prepared::load(
            resources,
            1,
            Profile::Debug,
        )?);
        if !states.needs().is_empty() {
            return Err("Kabuto state resources have unresolved preparation needs".into());
        }
        let flags = DirectFlags::project(1, program);
        Ok(Self {
            start,
            start_words: mission.starts[0].words,
            specification,
            states,
            fields,
            seed,
            bindings,
            flags,
        })
    }
    pub fn class_fields(&self) -> &Fields {
        &self.fields
    }
    pub fn state_template(&self) -> &Seed {
        &self.seed
    }

    /// Publish the acquired cold generic-to-Kabuto CPU fields on this factory's
    /// ordinary actor. Projection reset remains required, not acknowledged.
    /// This does not change a live actor's type or admit gameplay.
    pub fn initialize_player_fields(
        &self,
        session: &mut Session,
        owner: Id,
        frame: crate::kabuto_player::ColdFrame,
        blending: crate::animation_channels::Blending,
    ) -> Result<crate::kabuto_player::Initialized, String> {
        session.initialize_cold_kabuto_player_fields(owner, self, frame, blending)
    }

    /// Allocate the ordinary cold class instance, with independent payload,
    /// references and shadow storage. This deliberately leaves the declaration's
    /// marker pose separate until the actual player relocation phase is acquired.
    pub fn construct_ordinary(&self, session: &mut Session) -> Result<Id, String> {
        session.construct(Construction {
            class: 1,
            seed: &self.seed,
            bindings: &self.bindings,
            flags: self.flags,
            specification: self.specification.clone(),
            state_resources: self.states.clone(),
            fields: self.fields.instantiate(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "Requires GIANTS_KABUTO_ROOT with user-supplied data; cold constructor and player fields only, no window or sound"]
    fn source_cold_kabuto_keeps_player_setup_separate_and_refuses_unacquired_profiles() {
        let root = std::path::PathBuf::from(std::env::var("GIANTS_KABUTO_ROOT").unwrap());
        let source = crate::world_source::resolve(&root, "Kstory1").unwrap();
        let menu = crate::mission_selection::MissionMenu::parse(
            &std::fs::read(root.join("Bin").join(source.catalog)).unwrap(),
            u32::MAX,
            true,
        )
        .unwrap();
        let packet =
            crate::mission_loading::prepare(&root, menu.request("Kstory1", 0).unwrap()).unwrap();
        let plan = packet.instance_plan(&root, "English").unwrap();
        let prepared = Resources::load(
            &plan,
            packet.world.as_ref().unwrap(),
            packet.mission.as_ref().unwrap(),
        )
        .unwrap();
        assert_eq!(prepared.start.kind, 1);
        let mut session = Session::new(941);
        let first = prepared.construct_ordinary(&mut session).unwrap();
        let second = prepared.construct_ordinary(&mut session).unwrap();
        assert_ne!(first, second);
        assert!(session.kabuto_damage_state(first).is_err());
        let actor = session.get(first).unwrap();
        assert_eq!(actor.class, 1);
        assert_eq!(actor.seed.storage.data(), prepared.seed.storage.data());
        assert_eq!(
            actor.construction_fields.as_ref().unwrap().position,
            prepared.fields.position
        );
        assert!(Arc::ptr_eq(
            session.specification(first).unwrap(),
            &prepared.specification
        ));
        assert!(Arc::ptr_eq(
            session.state_resources(first).unwrap(),
            &prepared.states
        ));
        let before = session.counts();
        let input = |class, seed| Construction {
            class,
            seed,
            bindings: &prepared.bindings,
            flags: prepared.flags,
            specification: prepared.specification.clone(),
            state_resources: prepared.states.clone(),
            fields: prepared.fields.instantiate(),
        };
        assert!(session.construct(input(3, &prepared.seed)).is_err());
        let mut wrong_profile = prepared.seed.instantiate().unwrap();
        wrong_profile.storage = crate::state_storage::Storage::new(Profile::Gog);
        wrong_profile.storage.transition(1).unwrap();
        assert!(session.construct(input(1, &wrong_profile)).is_err());
        let key = crate::declared_residents::Key {
            scope: crate::declared_residents::Scope::Mission,
            index: 0,
        };
        assert!(
            session
                .construct_declaration_with_state_writes(
                    key,
                    input(1, &prepared.seed),
                    &[&giants_formats::mission::StateWriteKind::Word(7)]
                )
                .is_err()
        );
        assert!(session.declaration(key).is_none());
        assert_eq!(session.counts(), before);
        assert!(session.mecc_physical_cycle(first).is_err());
        // The public CPU initializer is atomic, bound to the prepared factory,
        // and does not advance animation, physics or mission admission.
        let frame = crate::kabuto_player::ColdFrame {
            position: [1., 2., 3.],
            current: [4., 5., 6.],
            previous: [7., 8., 9.],
            angles: [10., 20., 30.],
        };
        let blending = crate::animation_channels::Blending::Enabled;
        let saved = format!("{:?}", session.get(first).unwrap().construction_fields);
        let payload = session
            .get(first)
            .unwrap()
            .seed
            .storage
            .data()
            .unwrap()
            .to_vec();
        assert!(session.current_reference_matrices(first).is_err());
        assert!(session.reference_matrix(first, 0).is_err());
        let mut invalid = frame;
        invalid.previous[0] = f32::NAN;
        assert!(
            prepared
                .initialize_player_fields(&mut session, first, invalid, blending)
                .is_err()
        );
        assert_eq!(
            format!("{:?}", session.get(first).unwrap().construction_fields),
            saved
        );
        assert!(!session.has_cold_kabuto_player_fields(first));
        assert!(session.kabuto_damage_state(first).is_err());
        let foreign = Resources::load(
            &plan,
            packet.world.as_ref().unwrap(),
            packet.mission.as_ref().unwrap(),
        )
        .unwrap();
        assert!(
            foreign
                .initialize_player_fields(&mut session, first, frame, blending)
                .is_err()
        );
        let mut other = Session::new(942);
        assert!(
            prepared
                .initialize_player_fields(&mut other, first, frame, blending)
                .is_err()
        );
        let receipt = prepared
            .initialize_player_fields(&mut session, first, frame, blending)
            .unwrap();
        assert_eq!(receipt.owner, first);
        assert!(receipt.camera_projection_reset_required);
        assert!(session.has_cold_kabuto_player_fields(first));
        assert_eq!(
            session.kabuto_damage_state(first).unwrap(),
            (
                0.,
                f32::from_bits(prepared.class_fields().configuration[12])
            )
        );
        assert!(session.kabuto_damage_state(second).is_err());
        assert!(other.kabuto_damage_state(first).is_err());
        assert!(!session.has_cold_kabuto_player_fields(second));
        assert_eq!(session.counts(), before);
        assert_eq!(
            session.get(first).unwrap().seed.storage.data().unwrap(),
            payload
        );
        assert!(session.current_reference_matrices(first).is_err());
        assert!(session.reference_matrix(first, 0).is_err());
        assert!(session.mecc_physical_cycle(first).is_err());
        let selected = format!("{:?}", session.get(first).unwrap().construction_fields);
        assert!(
            prepared
                .initialize_player_fields(&mut session, first, frame, blending)
                .is_err()
        );
        assert_eq!(
            format!("{:?}", session.get(first).unwrap().construction_fields),
            selected
        );
        session.clear();
        assert!(!session.has_cold_kabuto_player_fields(first));
        assert!(session.kabuto_damage_state(first).is_err());
        assert_eq!(session.counts().residents, 0);
        assert_eq!(session.state_resource_counts(), (0, 0));
        assert_eq!(session.actor_field_counts(), (0, 0, 0));
    }
}
