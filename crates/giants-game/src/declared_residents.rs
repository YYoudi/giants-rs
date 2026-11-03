//! Construct acquired state profiles from source declarations, including the
//! acquired herd/marker world properties. Other world modifiers, native load
//! flags, behavior and mission readiness remain separate.
use crate::{
    actor_fields::Fields,
    instances::Plan,
    object_lists::DirectFlags,
    resident_states::{Construction, Id, Session},
    scene_resources::{SceneResources, SourcePlacement},
    specification, state_resources,
    state_storage::{Profile, Seed},
};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Scope {
    World,
    Mission,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Key {
    pub scope: Scope,
    pub index: usize,
}
pub struct Record {
    pub key: Key,
    pub source_offset: Option<usize>,
    pub class: Option<i32>,
    pub result: Result<Id, String>,
}
#[derive(Default)]
pub struct Report {
    pub records: Vec<Record>,
    pub other_states: usize,
}
impl Report {
    pub fn constructed(&self) -> usize {
        self.records.iter().filter(|r| r.result.is_ok()).count()
    }
    pub fn refused(&self) -> usize {
        self.records.len() - self.constructed()
    }
}

struct Template {
    seed: Seed,
    bindings: Vec<crate::model_registry::StateBinding>,
    flags: DirectFlags,
    specification: Arc<specification::Prepared>,
    resources: Arc<state_resources::Prepared>,
    fields: Fields,
}
impl Template {
    fn load(resources: &SceneResources, class: i32) -> Result<Self, String> {
        let program = resources
            .classes
            .models
            .first_program(class)
            .ok_or("declared actor class absent")?
            .program
            .as_ref()
            .map_err(Clone::clone)?;
        let seed = Seed::from_program_for(
            program,
            Profile::Debug,
            crate::arithmetic::Precision::Double53,
        )?;
        if !matches!(seed.storage.state(), 0 | 23)
            || (seed.storage.state() == 0 && seed.storage.data().is_some())
            || !seed.diagnostics.is_empty()
        {
            return Err("declared template has unsupported state data".into());
        }
        let specification = Arc::new(specification::Prepared::load_registered(resources, class)?);
        if !specification.needs.is_empty() {
            return Err(format!(
                "declared actor specification: {:?}",
                specification.needs
            ));
        }
        let state_resources = Arc::new(state_resources::Prepared::load(
            resources,
            class,
            Profile::Debug,
        )?);
        if !state_resources.needs().is_empty() {
            return Err(format!(
                "declared actor state resources: {:?}",
                state_resources.needs()
            ));
        }
        let fields = Fields::load(resources, class, &specification)?;
        Ok(Self {
            seed,
            bindings: resources.class_state_bindings_for(class, Profile::Debug)?,
            flags: DirectFlags::project(class, program),
            specification,
            resources: state_resources,
            fields,
        })
    }
    fn construct(
        &self,
        session: &mut Session,
        key: Key,
        source: &SourcePlacement,
        pitch_roll: Option<[f32; 2]>,
        writes: &[&giants_formats::mission::StateWriteKind],
    ) -> Result<Id, String> {
        if source.mission_word_174.is_some() {
            return Err("declared actor word174 consumer is not implemented".into());
        }
        if source.direct_flags != self.flags {
            return Err("declared actor flags override is not constructed".into());
        }
        let mut fields = self.fields.placed(crate::collider_init::DeclaredPose {
            class: source.class as u32,
            position: source.position,
            yaw: source.yaw,
            pitch_roll,
        })?;
        fields.set_declared_scale(source.scale)?;
        if let Some(selector) = source.mission_selector {
            fields.action = u32::from(selector);
        }
        session.construct_declaration_with_state_writes(
            key,
            Construction {
                class: source.class,
                seed: &self.seed,
                bindings: &self.bindings,
                flags: self.flags,
                specification: self.specification.clone(),
                state_resources: self.resources.clone(),
                fields,
            },
            writes,
        )
    }
}

/// World declarations precede mission declarations in this subset. The global
/// native order, including dynamic actors, is not certified by that ordering.
pub fn construct_zero(plan: &Plan, session: &mut Session) -> Report {
    construct_states(plan, session, |state| state == 0, false)
}
pub fn construct_acquired(plan: &Plan, session: &mut Session) -> Report {
    construct_states(plan, session, |state| matches!(state, 0 | 23), true)
}
fn construct_states(
    plan: &Plan,
    session: &mut Session,
    acquired: impl Fn(u8) -> bool,
    source_herds: bool,
) -> Report {
    let mut templates: BTreeMap<i32, Result<Template, String>> = BTreeMap::new();
    let mut herd = None;
    let mut markers = None;
    let mut report = Report::default();
    let world = plan.world_entries.iter().enumerate().map(|(index, e)| {
        (
            Key {
                scope: Scope::World,
                index,
            },
            e.class_input(),
            e.declaration.pitch_roll,
        )
    });
    let mission = plan.entries.iter().enumerate().map(|(index, e)| {
        (
            Key {
                scope: Scope::Mission,
                index,
            },
            e.source.as_ref().map_err(Clone::clone),
            e.source.as_ref().ok().and_then(|s| s.pitch_roll),
        )
    });
    for (key, source, pitch_roll) in world.chain(mission) {
        let herd_placement = (source_herds && key.scope == Scope::World)
            .then(|| &plan.world_entries[key.index].declaration)
            .filter(|p| matches!(p.object_type, 49 | 679));
        if herd_placement.is_none()
            && let Ok(s) = &source
            && let Ok(seed) = &s.state_seed
            && !acquired(seed.storage.state())
        {
            report.other_states += 1;
            continue;
        }
        let class = source.as_ref().ok().map(|s| s.class);
        let source_offset = source.as_ref().ok().map(|s| s.source_offset);
        let result = (|| {
            if session.declaration(key).is_some() {
                return Err("actor declaration already constructed".into());
            }
            if let Some(placement) = herd_placement {
                return session.construct_source_declaration(key, |session| {
                    match placement.object_type {
                        49 => herd
                            .get_or_insert_with(|| {
                                crate::herd_generator::Factory::load(plan.resources())
                            })
                            .as_ref()
                            .map_err(Clone::clone)?
                            .place(session, placement),
                        679 => markers
                            .get_or_insert_with(|| {
                                crate::herd_markers::Factory::load(plan.resources())
                            })
                            .as_ref()
                            .map_err(Clone::clone)?
                            .place(session, placement),
                        _ => unreachable!("filtered herd placement"),
                    }
                });
            }
            let source = source?;
            // State payloads and writes are reconstructed in Debug below.
            // Historical projection diagnostics cannot decide their validity.
            for update in &plan.mission_object_results {
                if key.scope == Scope::Mission
                    && update.object == giants_formats::mission::ObjectRef::Placement(key.index)
                    && update.result.is_err()
                {
                    return Err("declared actor has unresolved mission writes".into());
                }
            }
            let writes: Vec<_> = plan
                .mission_state_writes()
                .iter()
                .filter(|w| {
                    key.scope == Scope::Mission
                        && w.object == giants_formats::mission::ObjectRef::Placement(key.index)
                })
                .map(|w| &w.kind)
                .collect();
            if source_herds && key.scope == Scope::Mission && source.class == 679 {
                if !writes.is_empty() {
                    return Err(
                        "mission marker state writes outside acquired header-only profile".into(),
                    );
                }
                return session.construct_source_declaration(key, |session| {
                    markers
                        .get_or_insert_with(|| crate::herd_markers::Factory::load(plan.resources()))
                        .as_ref()
                        .map_err(Clone::clone)?
                        .place_mission(
                            session,
                            &plan.entries[key.index].serialized,
                            source.mission_selector,
                            source.mission_word_174,
                            source.mission_values_f4,
                        )
                });
            }
            templates
                .entry(source.class)
                .or_insert_with(|| Template::load(plan.resources(), source.class))
                .as_ref()
                .map_err(Clone::clone)?
                .construct(session, key, source, pitch_roll, &writes)
        })();
        report.records.push(Record {
            key,
            source_offset,
            class,
            result,
        });
    }
    report
}
