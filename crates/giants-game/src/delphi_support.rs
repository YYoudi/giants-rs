//! Source-backed cold ownership for Delphi's acquired physical-support class.
//! This is construction only: update/initialization, current collision geometry,
//! scene keys and live scalar/header publication remain separate prerequisites.
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

pub struct Template {
    seed: Seed,
    bindings: Vec<StateBinding>,
    fields: Fields,
    flags: DirectFlags,
    specification: Arc<specification::Prepared>,
    states: Arc<state_resources::Prepared>,
}
impl Template {
    /// Load the actual class from this world's registered resource namespace.
    /// No class is substituted when this world does not declare the support.
    pub fn load(source: &SceneResources) -> Result<Self, String> {
        let class = 1189;
        let program = source
            .classes
            .models
            .first_program(class)
            .ok_or("Delphi support class absent from this world")?
            .program
            .as_ref()
            .map_err(Clone::clone)?;
        let specification = Arc::new(specification::Prepared::load_registered(source, class)?);
        let seed = Seed::from_program_for(program, Profile::Debug, Precision::Double53)?;
        let bindings = source.class_state_bindings_for(class, Profile::Debug)?;
        let fields = Fields::load(source, class, &specification)?;
        let states = Arc::new(state_resources::Prepared::load(
            source,
            class,
            Profile::Debug,
        )?);
        let template = Self {
            seed,
            bindings,
            fields,
            flags: DirectFlags::project(class, program),
            specification,
            states,
        };
        validate(&template.input())?;
        Ok(template)
    }
    fn input(&self) -> Construction<'_> {
        Construction {
            class: 1189,
            seed: &self.seed,
            bindings: &self.bindings,
            flags: self.flags,
            specification: self.specification.clone(),
            state_resources: self.states.clone(),
            fields: self.fields.instantiate(),
        }
    }
    /// Publish a cold resident with private payload and shared immutable models.
    /// This deliberately does not use or enable ordinary mission construction.
    pub fn construct(&self, session: &mut Session) -> Result<Id, String> {
        session.construct_delphi_support_cold(self.input())
    }
}

pub(crate) fn validate(input: &Construction<'_>) -> Result<(), String> {
    if input.class != 1189
        || input.fields.class != 1189
        || input.seed.storage.profile() != Profile::Debug
        || input.seed.storage.state() != 83
        || input.fields.configuration[20] != 83
        || !input.seed.diagnostics.is_empty()
        || !input.seed.unresolved.is_empty()
        || !input.bindings.is_empty()
        || input.seed.storage.data() != Some(&[0; 20])
        || !input.specification.metadata.references.names.is_empty()
        || !input.state_resources.needs().is_empty()
        || input.fields.shadow.is_some()
        || !input.fields.gunlist.is_empty()
    {
        return Err("Delphi support outside acquired cold zero-payload/resource profile".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "Requires GIANTS_DELPHI_ROOT; cold support ownership only, no update or world admission"]
    fn source_delphi_support_owns_private_payload_and_retains_ordinary_refusal() {
        let root = std::path::PathBuf::from(std::env::var("GIANTS_DELPHI_ROOT").unwrap());
        let absent = SceneResources::load(&root, "w_Rstory1.bin", "English").unwrap();
        assert!(Template::load(&absent).is_err());
        for world in ["w_Rstory2.bin", "w_Rstory4.bin", "w_Rstory5.bin"] {
            let source = SceneResources::load(&root, world, "English").unwrap();
            let mut template = Template::load(&source).unwrap();
            let mut session = Session::new(994);
            assert!(session.construct(template.input()).is_err());
            let ids: Vec<_> = (0..8)
                .map(|_| template.construct(&mut session).unwrap())
                .collect();
            assert_eq!(session.counts().payload_bytes, 160);
            assert_eq!(session.counts().registrations, 32);
            for list in 0..15 {
                assert_eq!(
                    session.list(list),
                    if [0, 1, 5, 13].contains(&list) {
                        ids.clone()
                    } else {
                        vec![]
                    }
                );
            }
            let mut payloads = std::collections::BTreeSet::new();
            for id in &ids {
                let actor = session.get(*id).unwrap();
                let data = actor.seed.storage.data().unwrap();
                assert_eq!(data, &[0; 20]);
                assert!(payloads.insert(data.as_ptr()));
                assert!(Arc::ptr_eq(
                    session.specification(*id).unwrap(),
                    &template.specification
                ));
                assert!(Arc::ptr_eq(
                    session.state_resources(*id).unwrap(),
                    &template.states
                ));
                assert!(session.actor_current_affine(*id).is_err());
                assert!(session.actor_environment(*id).is_err());
                assert!(session.delphi_physical(*id).is_none());
            }
            let before = session.counts();
            let owners = Arc::strong_count(&template.specification);
            template.fields.configuration[19] = 123;
            assert!(template.construct(&mut session).is_err());
            assert_eq!(session.counts(), before);
            assert_eq!(Arc::strong_count(&template.specification), owners);
            assert_eq!(session.list(0), ids);
            template.fields.configuration[19] = 0;
            drop(source);
            session.clear();
            assert_eq!(session.counts(), crate::resident_states::Counts::default());
            assert!(ids.iter().all(|id| session.get(*id).is_none()));
            let next = template.construct(&mut session).unwrap();
            assert!(!ids.contains(&next));
            assert!(session.get(next).is_some());
        }
    }
}
