//! Acquired unparented Kabuto auxiliary constructors. Effect actors are allocated
//! in their declared cold state before the acquired zero-state cleanup.
//! Parent binding, material/geometry setup and complete player initialization
//! remain separate phases.
use crate::{
    actor_fields::Fields,
    arithmetic::Precision,
    model_registry::StateBinding,
    object_lists::DirectFlags,
    resident_states::{Construction, Id, Session},
    scene_resources::SceneResources,
    specification, state_resources,
    state_storage::{Profile, Seed},
};
use std::sync::Arc;
#[derive(Clone, Copy, Debug)]
pub enum Kind {
    First,
    Second,
    Effect,
    FinalEffect,
}
struct Template {
    class: i32,
    seed: Seed,
    bindings: Vec<StateBinding>,
    flags: DirectFlags,
    specification: Arc<specification::Prepared>,
    states: Arc<state_resources::Prepared>,
    fields: Fields,
}
impl Template {
    fn load(r: &SceneResources, class: i32, state: u8) -> Result<Self, String> {
        let program = r
            .classes
            .models
            .first_program(class)
            .ok_or("Kabuto auxiliary class absent")?
            .program
            .as_ref()
            .map_err(Clone::clone)?;
        let seed = Seed::from_program_for(program, Profile::Debug, Precision::Double53)?;
        let specification = Arc::new(specification::Prepared::load_registered(r, class)?);
        let states = Arc::new(state_resources::Prepared::load(r, class, Profile::Debug)?);
        let fields = Fields::load(r, class, &specification)?;
        let bindings = r.class_state_bindings_for(class, Profile::Debug)?;
        if seed.storage.state() != state
            || !seed.diagnostics.is_empty()
            || !specification.needs.is_empty()
            || !states.needs().is_empty()
            || !bindings.is_empty()
            || (state == 78
                && seed
                    .storage
                    .data()
                    .is_none_or(|p| p.len() != 268 || p.iter().any(|&b| b != 0)))
        {
            return Err("Kabuto auxiliary resources outside acquired cold profile".into());
        }
        Ok(Self {
            class,
            seed,
            bindings,
            flags: DirectFlags::project(class, program),
            specification,
            states,
            fields,
        })
    }
    fn input(&self) -> Construction<'_> {
        Construction {
            class: self.class,
            seed: &self.seed,
            bindings: &self.bindings,
            flags: self.flags,
            specification: self.specification.clone(),
            state_resources: self.states.clone(),
            fields: self.fields.instantiate(),
        }
    }
}
pub struct Factory {
    templates: [Template; 4],
}
impl Factory {
    pub fn load(r: &SceneResources) -> Result<Self, String> {
        Ok(Self {
            templates: [
                Template::load(r, 749, 0)?,
                Template::load(r, 1018, 0)?,
                Template::load(r, 866, 78)?,
                Template::load(r, 980, 78)?,
            ],
        })
    }
    /// Construct one unparented cold auxiliary. This does not publish its later
    /// Kabuto-specific parent/reference/flags or make an actor gameplay-ready.
    pub fn construct(&self, session: &mut Session, kind: Kind) -> Result<Id, String> {
        let (index, mask) = match kind {
            Kind::First => (0, 1),
            Kind::Second => (1, 1),
            Kind::Effect => (2, 0),
            Kind::FinalEffect => (3, 0),
        };
        session.construct_kabuto_auxiliary(self.templates[index].input(), mask)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "Requires GIANTS_KABUTO_ROOT with user-supplied data; no window or sound"]
    fn source_kabuto_auxiliaries_construct_declared_states_then_clear_empty_effects() {
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
        let mut factory = Factory::load(plan.resources()).unwrap();
        let mut session = Session::new(944);
        let mut ids = Vec::new();
        for (index, kind) in [Kind::First, Kind::Second, Kind::Effect, Kind::FinalEffect]
            .into_iter()
            .enumerate()
        {
            let id = factory.construct(&mut session, kind).unwrap();
            ids.push(id);
            let actor = session.get(id).unwrap();
            assert_eq!(actor.seed.storage.state(), 0);
            assert!(actor.seed.storage.data().is_none());
            assert_eq!(
                actor.construction_fields.as_ref().unwrap().configuration[20],
                0
            );
            assert!(Arc::ptr_eq(
                session.specification(id).unwrap(),
                &factory.templates[index].specification
            ));
            assert!(Arc::ptr_eq(
                session.state_resources(id).unwrap(),
                &factory.templates[index].states
            ));
            assert!(session.current_reference_matrices(id).is_err());
        }
        assert_eq!(session.counts().residents, 4);
        let before = session.counts();
        let lists = (0..15).map(|i| session.list(i).len()).collect::<Vec<_>>();
        assert!(
            session
                .construct_kabuto_auxiliary(factory.templates[0].input(), 0)
                .is_err()
        );
        factory.templates[2].seed.storage.data_mut().unwrap()[0] = 1;
        assert!(factory.construct(&mut session, Kind::Effect).is_err());
        assert_eq!(session.counts(), before);
        assert_eq!(
            (0..15).map(|i| session.list(i).len()).collect::<Vec<_>>(),
            lists
        );
        factory.templates[2].seed.storage.data_mut().unwrap()[0] = 0;
        let next = factory.construct(&mut session, Kind::Effect).unwrap();
        assert!(!ids.contains(&next));
        session.clear();
        assert_eq!(session.counts().residents, 0);
        assert_eq!(session.state_resource_counts(), (0, 0));
        assert_eq!(session.actor_field_counts(), (0, 0, 0));
    }
}
