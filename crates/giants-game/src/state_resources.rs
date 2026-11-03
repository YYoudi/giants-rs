//! Typed state-resource ownership over captured class bindings. No native
//! pointer is written and no effect, actor callback or readiness is activated.
use crate::{
    model_registry::{StateBinding, StateTarget},
    scene_resources::SceneResources,
    specification,
    state_storage::{Profile, ReferenceKey},
};
use std::{collections::BTreeMap, sync::Arc};

pub type Specifications = BTreeMap<usize, Result<Arc<specification::Prepared>, String>>;
enum Resource {
    Specification(Arc<specification::Prepared>),
    Effect(Arc<giants_formats::effect_program::Program>),
}
struct Slot {
    binding: StateBinding,
    resource: Result<Resource, String>,
}
pub struct Prepared {
    class: i32,
    profile: Profile,
    slots: BTreeMap<usize, Slot>,
}
impl std::fmt::Debug for Prepared {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StateResources")
            .field("class", &self.class)
            .field("profile", &self.profile)
            .field("references", &self.slots.len())
            .field("needs", &self.needs())
            .finish()
    }
}
impl Prepared {
    pub fn load(resources: &SceneResources, class: i32, profile: Profile) -> Result<Self, String> {
        Self::load_reusing(resources, class, profile, None)
    }
    pub fn load_reusing(
        resources: &SceneResources,
        class: i32,
        profile: Profile,
        specifications: Option<&Specifications>,
    ) -> Result<Self, String> {
        let bindings = resources.class_state_bindings_for(class, profile)?;
        let catalog = &resources
            .classes
            .models
            .first_program(class)
            .ok_or("state class program absent")?
            .catalog;
        let mut slots = BTreeMap::new();
        for binding in bindings {
            let offset = binding.offset;
            let resource = (|| -> Result<Resource, String> {
                let target = binding.target.as_ref().map_err(Clone::clone)?;
                match (&binding.key, target) {
                    (
                        ReferenceKey::Spec(key),
                        StateTarget::Spec {
                            id,
                            catalog: spec_catalog,
                        },
                    ) if key == id => {
                        let spec = match specifications.and_then(|s| s.get(&offset)) {
                            Some(prepared) => prepared.as_ref().map_err(Clone::clone)?.clone(),
                            None => Arc::new(specification::Prepared::load_state_registered(
                                resources, class, profile, offset,
                            )?),
                        };
                        if spec.class != class
                            || spec.class_catalog != *catalog
                            || spec.specification != *id
                            || spec.specification_catalog != *spec_catalog
                        {
                            return Err(
                                "state specification resource identity differs from capture".into(),
                            );
                        }
                        if !spec.needs.is_empty() {
                            return Err(
                                "state specification has unresolved preparation needs".into()
                            );
                        }
                        Ok(Resource::Specification(spec))
                    }
                    (ReferenceKey::Effect(name), target @ StateTarget::Effect { image, .. }) => {
                        if resources.effect_target(image, name)? != *target {
                            return Err(
                                "state effect resource identity differs from capture".into()
                            );
                        }
                        Ok(Resource::Effect(Arc::new(
                            resources.bound_effect_program(target)?,
                        )))
                    }
                    _ => Err("state resource key and captured target differ".into()),
                }
            })();
            if slots.insert(offset, Slot { binding, resource }).is_some() {
                return Err("duplicate state resource offset".into());
            }
        }
        Ok(Self {
            class,
            profile,
            slots,
        })
    }
    pub fn matches(&self, class: i32, profile: Profile, bindings: &[StateBinding]) -> bool {
        self.class == class
            && self.profile == profile
            && self.slots.len() == bindings.len()
            && bindings
                .iter()
                .all(|b| self.slots.get(&b.offset).is_some_and(|s| s.binding == *b))
    }
    pub fn needs(&self) -> Vec<(usize, &str)> {
        self.slots
            .iter()
            .filter_map(|(offset, s)| s.resource.as_ref().err().map(|e| (*offset, e.as_str())))
            .collect()
    }
    pub fn counts(&self) -> (usize, usize) {
        let specifications = self
            .slots
            .values()
            .filter(|s| matches!(s.resource, Ok(Resource::Specification(_))))
            .count();
        let effects = self
            .slots
            .values()
            .filter(|s| matches!(s.resource, Ok(Resource::Effect(_))))
            .count();
        (specifications, effects)
    }
    pub fn specification(&self, offset: usize) -> Result<&Arc<specification::Prepared>, String> {
        match self
            .slots
            .get(&offset)
            .ok_or("state resource offset absent")?
            .resource
            .as_ref()
            .map_err(Clone::clone)?
        {
            Resource::Specification(spec) => Ok(spec),
            _ => Err("state resource is not a specification".into()),
        }
    }
    /// Framed effect data only; execution and live pointer binding are separate.
    pub fn effect(
        &self,
        offset: usize,
    ) -> Result<&Arc<giants_formats::effect_program::Program>, String> {
        match self
            .slots
            .get(&offset)
            .ok_or("state resource offset absent")?
            .resource
            .as_ref()
            .map_err(Clone::clone)?
        {
            Resource::Effect(program) => Ok(program),
            _ => Err("state resource is not an effect".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn resources() -> SceneResources {
        use crate::model_registry::{ClassProgram, ModelRegistry, SelectionSpec, SpecDefinition};
        let mut models = ModelRegistry::default();
        models.programs.insert(
            2,
            vec![ClassProgram {
                catalog: "class.bin".into(),
                program: Ok(giants_formats::class_program::parse(&[
                    0x1e, 2, 0x1f, 20, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 255,
                ])
                .unwrap()),
                bindings: Ok(vec![StateBinding {
                    offset: 0x6c,
                    key: ReferenceKey::Spec(20),
                    target: Ok(StateTarget::Spec {
                        id: 20,
                        catalog: "spec.bin".into(),
                    }),
                }]),
            }],
        );
        models.specs.insert(
            20,
            vec![SpecDefinition {
                catalog: "spec.bin".into(),
                effect_namespace: None,
                models: Ok(vec![]),
                program: Ok(giants_formats::object_spec::parse(&[255]).unwrap()),
                selection: Ok(SelectionSpec {
                    slots: vec![],
                    animations: vec![],
                    effects: vec![],
                    non_model_commands: vec![],
                    diagnostics: vec![],
                }),
            }],
        );
        SceneResources {
            world_bounds: Default::default(),
            classes: crate::catalog_registry::WorldClassRegistry {
                visits: vec![],
                classes: Default::default(),
                textures: Default::default(),
                models,
            },
            archives: Default::default(),
            language: "English".into(),
        }
    }
    #[test]
    fn profile_capture_and_shared_resource_require_matching_identities() {
        let resources = resources();
        let original = Prepared::load(&resources, 2, Profile::Debug).unwrap();
        assert!(original.needs().is_empty());
        assert_eq!(original.counts(), (1, 0));
        assert_eq!(original.specification(0x70).unwrap().specification, 20);
        assert!(original.specification(0x6c).is_err());
        assert!(original.effect(0x70).is_err());
        let bindings = resources
            .class_state_bindings_for(2, Profile::Debug)
            .unwrap();
        assert!(original.matches(2, Profile::Debug, &bindings));
        assert!(!original.matches(2, Profile::Gog, &bindings));
        let mut reused = Specifications::new();
        reused.insert(0x70, Ok(Arc::clone(original.specification(0x70).unwrap())));
        let prepared =
            Prepared::load_reusing(&resources, 2, Profile::Debug, Some(&reused)).unwrap();
        assert!(Arc::ptr_eq(
            original.specification(0x70).unwrap(),
            prepared.specification(0x70).unwrap()
        ));
        // A wrong shared identity is refused, never healed by loading a new one.
        let mut wrong =
            specification::Prepared::load_state_registered(&resources, 2, Profile::Debug, 0x70)
                .unwrap();
        wrong.class = 11;
        reused.insert(0x70, Ok(Arc::new(wrong)));
        let failed = Prepared::load_reusing(&resources, 2, Profile::Debug, Some(&reused)).unwrap();
        assert_eq!(failed.counts(), (0, 0));
        assert!(
            failed
                .specification(0x70)
                .err()
                .unwrap()
                .contains("identity")
        );
    }
    #[test]
    fn later_definitions_do_not_heal_missing_captured_targets() {
        let mut resources = resources();
        resources.classes.models.programs.get_mut(&2).unwrap()[0]
            .bindings
            .as_mut()
            .unwrap()[0]
            .target = Err("absent at registration".into());
        let prepared = Prepared::load(&resources, 2, Profile::Debug).unwrap();
        assert_eq!(prepared.needs(), [(0x70, "absent at registration")]);
        assert_eq!(
            prepared.specification(0x70).err().unwrap(),
            "absent at registration"
        );
    }
    #[test]
    fn resident_binding_is_ticket_owned_and_never_writes_a_payload_pointer() {
        let resources = resources();
        let program = resources
            .classes
            .models
            .first_program(2)
            .unwrap()
            .program
            .as_ref()
            .unwrap();
        let seed = crate::state_storage::Seed::from_program_for(
            program,
            Profile::Debug,
            crate::arithmetic::Precision::Double53,
        )
        .unwrap();
        let before = seed.storage.data().unwrap().to_vec();
        let bindings = resources
            .class_state_bindings_for(2, Profile::Debug)
            .unwrap();
        let mut session = crate::resident_states::Session::new(19);
        let actor = session
            .register(
                2,
                &seed,
                &bindings,
                crate::object_lists::DirectFlags::default(),
            )
            .unwrap();
        let shared = Arc::new(Prepared::load(&resources, 2, Profile::Debug).unwrap());
        let weak = Arc::downgrade(&shared);
        session.bind_state_resources(actor, shared.clone()).unwrap();
        assert_eq!(session.state_resource_counts(), (1, 1));
        assert!(session.bind_state_resources(actor, shared.clone()).is_err());
        assert_eq!(
            session.get(actor).unwrap().seed.storage.data().unwrap(),
            before
        );
        assert_eq!(&before[0x70..0x74], &[0; 4]);
        let mut foreign = crate::resident_states::Session::new(20);
        let other = foreign
            .register(
                2,
                &seed,
                &bindings,
                crate::object_lists::DirectFlags::default(),
            )
            .unwrap();
        assert!(session.bind_state_resources(other, shared.clone()).is_err());
        assert!(session.state_resources(other).is_none());
        let mut changed = bindings.clone();
        changed[0].target = Err("absent at registration".into());
        let stale = session
            .register(
                2,
                &seed,
                &changed,
                crate::object_lists::DirectFlags::default(),
            )
            .unwrap();
        assert!(session.bind_state_resources(stale, shared.clone()).is_err());
        drop(shared);
        assert!(weak.upgrade().is_some());
        session.clear();
        assert!(weak.upgrade().is_none());
        assert_eq!(session.state_resource_counts(), (0, 0));
        assert!(session.state_resources(actor).is_none());
    }
    #[test]
    fn complete_construction_failure_never_publishes_half_bound_actors_or_resources() {
        use crate::{
            object_lists::DirectFlags,
            resident_states::{Construction, Session},
        };
        let resources = resources();
        let program = resources
            .classes
            .models
            .first_program(2)
            .unwrap()
            .program
            .as_ref()
            .unwrap();
        let seed = crate::state_storage::Seed::from_program_for(
            program,
            Profile::Debug,
            crate::arithmetic::Precision::Double53,
        )
        .unwrap();
        let bindings = resources
            .class_state_bindings_for(2, Profile::Debug)
            .unwrap();
        let states = Arc::new(Prepared::load(&resources, 2, Profile::Debug).unwrap());
        // Synthetic fixture has only a state specification; these tests exercise
        // ownership/validation, not a real class body's numerical equivalence.
        let body = states.specification(0x70).unwrap().clone();
        let mut configuration = crate::class_template::initial_configuration();
        configuration[20] = 2;
        let fields = crate::actor_fields::Fields {
            class: 2,
            configuration,
            scale: 1.,
            object_flags: 0,
            response_flags: 0x80000000,
            control_flags: 0,
            action: 0,
            shadow_reciprocal: 0.,
            shadow: None,
            gunlist: Arc::from([]),
            channels: crate::actor_context::initial_channels(None, false),
            position: [0.; 3],
            current: [0.; 3],
            previous: [0.; 3],
            angles: [0.; 3],
        };
        let mut session = Session::new(150);
        let first = session
            .construct(Construction {
                class: 2,
                seed: &seed,
                bindings: &bindings,
                flags: DirectFlags::default(),
                specification: body.clone(),
                state_resources: states.clone(),
                fields: fields.clone(),
            })
            .unwrap();
        let before = session.counts();
        let list = session.list(0);
        let body_count = Arc::strong_count(&body);
        let state_count = Arc::strong_count(&states);
        for invalid in 0..3 {
            let mut bad = fields.clone();
            match invalid {
                0 => bad.class = 11,
                1 => bad.configuration[19] = 1,
                _ => bad.response_flags = 0,
            }
            assert!(
                session
                    .construct(Construction {
                        class: 2,
                        seed: &seed,
                        bindings: &bindings,
                        flags: DirectFlags::default(),
                        specification: body.clone(),
                        state_resources: states.clone(),
                        fields: bad
                    })
                    .is_err()
            );
            assert_eq!(session.counts(), before);
            assert_eq!(session.list(0), list);
            assert_eq!(Arc::strong_count(&body), body_count);
            assert_eq!(Arc::strong_count(&states), state_count);
            assert_eq!(session.get(first).unwrap().channels, Some(fields.channels));
        }
        let second = session
            .construct(Construction {
                class: 2,
                seed: &seed,
                bindings: &bindings,
                flags: DirectFlags::default(),
                specification: body.clone(),
                state_resources: states.clone(),
                fields,
            })
            .unwrap();
        assert_ne!(first, second);
        assert_eq!(session.list(0), [first, second]);
        session.clear();
        assert_eq!(session.specification_count(), 0);
        assert_eq!(session.state_resource_counts(), (0, 0));
        assert_eq!(session.actor_field_counts(), (0, 0, 0));
        assert_eq!(Arc::strong_count(&body), body_count - 1);
        assert_eq!(Arc::strong_count(&states), state_count - 1);
    }
}
