//! Construct acquired state profiles from source declarations, including the
//! acquired herd/marker world properties. Other world modifiers, native load
//! flags, behavior and mission readiness remain separate.
//! Prototype PS2 `object_allocate_type` and `object_copy_personality` remain
//! unknown-attribution navigation routes to this bounded cold construction.
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
// Cold constructor coverage is separate from callback/frame admission. These
// additional class/state pairs have independently checked native copy/payload
// profiles; do not extend all state51/98 classes from a shared numeric state.
pub(crate) fn constructor_profile(class: i32, state: u8) -> bool {
    matches!(state, 0 | 23)
        || class == 215 && state == 51
        || matches!(class, 1060 | 1226) && state == 98
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
        if !constructor_profile(class, seed.storage.state())
            || (matches!(seed.storage.state(), 0 | 98) && seed.storage.data().is_some())
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
        if source.mission_word_174.is_some()
            && !(key.scope == Scope::Mission
                && self.seed.storage.state() == 98
                && matches!(source.class, 1060 | 1226))
        {
            return Err("declared actor word174 consumer is not implemented".into());
        }
        if matches!(self.seed.storage.state(), 51 | 98) && source.mission_values_f4.is_some() {
            return Err(
                "cold alert/building actor common F4 placement values remain unbound".into(),
            );
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
        let initial = crate::uncached_pose::Pose {
            position: fields.position,
            angles: fields.angles,
            scale: fields.scale,
        };
        fields.set_declared_scale(source.scale)?;
        if let Some(selector) = source.mission_selector {
            fields.action = u32::from(selector);
        }
        let input = Construction {
            class: source.class,
            seed: &self.seed,
            bindings: &self.bindings,
            flags: self.flags,
            specification: self.specification.clone(),
            state_resources: self.resources.clone(),
            fields,
        };
        let id = if key.scope == Scope::Mission {
            session.construct_mission_declaration_with_pose(key, input, writes, initial)
        } else {
            session.construct_declaration_with_state_writes(key, input, writes)
        }?;
        if let Some(value) = source.mission_word_174 {
            session.retain_barracks_word_174(key, id, value)?;
        }
        Ok(id)
    }
}

/// World declarations precede mission declarations in this subset. The global
/// native order, including dynamic actors, is not certified by that ordering.
pub fn construct_zero(plan: &Plan, session: &mut Session) -> Report {
    construct_states(plan, session, |state| state == 0, false)
}
pub fn construct_acquired(plan: &Plan, session: &mut Session) -> Report {
    construct_states(
        plan,
        session,
        |state| matches!(state, 0 | 23 | 51 | 98),
        true,
    )
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
            && (!acquired(seed.storage.state())
                || !constructor_profile(s.class, seed.storage.state()))
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires GIANTS_BUILDING_ROOT; The Wife cold actor identities/payloads/list ownership, not first primary/AI/audio/GPU readiness"]
    fn installed_wife_alert_actors_and_barracks_have_cold_owned_construction() {
        let root = std::path::PathBuf::from(std::env::var_os("GIANTS_BUILDING_ROOT").unwrap());
        let menu = crate::mission_selection::MissionMenu::parse(
            &std::fs::read(root.join("Bin/w_Story2.bin")).unwrap(),
            0,
            true,
        )
        .unwrap();
        let packet =
            crate::mission_loading::prepare(&root, menu.request("Story2", 1).unwrap()).unwrap();
        let plan = packet.instance_plan(&root, "English").unwrap();
        let mut session = Session::new(411);
        let report = construct_acquired(&plan, &mut session);
        let records = report
            .records
            .iter()
            .filter(|r| matches!(r.class, Some(215 | 1060 | 1226)))
            .collect::<Vec<_>>();
        assert_eq!(records.len(), 8);
        assert_eq!(records.iter().filter(|r| r.class == Some(215)).count(), 6);
        for record in records {
            let id = record
                .result
                .as_ref()
                .unwrap_or_else(|e| panic!("{:?}: {e}", record.key));
            let actor = session.get(*id).unwrap();
            let fields = actor.construction_fields.as_ref().unwrap();
            let source = plan.entries[record.key.index].source.as_ref().unwrap();
            assert_eq!(record.key.scope, Scope::Mission);
            assert_eq!(fields.position, source.position);
            assert_eq!(
                fields.action,
                u32::from(source.mission_selector.unwrap_or(0))
            );
            assert!(session.list(1).contains(id));
            assert!(session.actor_world_tick(*id).is_err());
            match actor.class {
                215 => {
                    assert_eq!(actor.seed.storage.state(), 51);
                    let data = actor.seed.storage.data().unwrap();
                    assert_eq!(data.len(), 576);
                    assert!(data.iter().all(|b| *b == 0));
                }
                1060 | 1226 => {
                    assert_eq!(actor.seed.storage.state(), 98);
                    assert!(actor.seed.storage.data().is_none());
                }
                _ => unreachable!(),
            }
        }
        // Supplied alert requests over the actual constructed nonempty list.
        // These identities are test targets, not a fabricated player binding.
        let list = session.list(1);
        let rippers: Vec<_> = list
            .iter()
            .copied()
            .filter(|id| session.get(*id).unwrap().seed.storage.state() == 23)
            .collect();
        let team: Vec<_> = list
            .iter()
            .copied()
            .filter(|id| session.get(*id).unwrap().class == 215)
            .collect();
        let building = list
            .iter()
            .copied()
            .find(|id| session.get(*id).unwrap().class == 1060)
            .unwrap();
        assert_eq!(team.len(), 6);
        assert!(rippers.len() >= 2);
        let (first, second) = (rippers[0], rippers[1]);
        session
            .alert_team_from_building(building, 100_000., first)
            .unwrap();
        session
            .alert_rippers_from_building(building, 100_000., first)
            .unwrap();
        for id in team.iter().chain(&rippers) {
            assert_eq!(session.building_alert_target(*id).unwrap(), Some(first));
        }
        session
            .alert_team_from_building(building, 100_000., second)
            .unwrap();
        session
            .alert_rippers_from_building(building, 100_000., second)
            .unwrap();
        for id in &team {
            assert_eq!(session.building_alert_target(*id).unwrap(), Some(second));
            let actor = session.get(*id).unwrap();
            assert_eq!(
                actor.construction_fields.as_ref().unwrap().control_flags & 0x21,
                0x21
            );
            assert_eq!(actor.seed.storage.data().unwrap()[4..8], [0; 4]);
            assert!(session.actor_world_tick(*id).is_err());
        }
        for id in &rippers {
            assert_eq!(session.building_alert_target(*id).unwrap(), Some(first));
            assert!(session.state23_combat(*id).is_none());
            assert_eq!(
                session.get(*id).unwrap().seed.storage.data().unwrap()[12..16],
                [0; 4]
            );
        }
        assert_eq!(session.list(1), list);
        let globals =
            giants_formats::global_data::parse(&std::fs::read(root.join("Bin/GData.gbn")).unwrap())
                .unwrap();
        let stock = crate::building_barracks::InitialParameters::load(&globals).unwrap();
        let mut requested = Vec::new();
        for owner in list
            .iter()
            .copied()
            .filter(|id| matches!(session.get(*id).unwrap().class, 1060 | 1226))
            .collect::<Vec<_>>()
        {
            assert!(
                !session
                    .initialize_barracks(owner, stock, true, &mut |id, cue| {
                        requested.push((id, cue));
                        Ok(())
                    })
                    .unwrap()
            );
            assert!(session.barracks_state(owner).is_err());
            assert!(
                session
                    .initialize_barracks(owner, stock, false, &mut |id, cue| {
                        requested.push((id, cue));
                        Ok(())
                    })
                    .unwrap()
            );
            let state = session.barracks_state(owner).unwrap();
            let actor = session.get(owner).unwrap();
            let expected = if actor.class == 1060 {
                (0, stock.mini, 0x6bf)
            } else {
                (4, 12, 0x4db)
            };
            assert_eq!(
                (state.selector, state.remaining, requested.last().unwrap().1),
                expected
            );
            assert_eq!((state.phase, state.damage), (1, 0.));
            assert_eq!(actor.channels.unwrap()[0][1], 1_f32.to_bits());
            assert_eq!(actor.channels.unwrap()[0][8], 0x184);
            assert!(session.actor_world_tick(owner).is_err());
            assert!(
                session
                    .initialize_barracks(owner, stock, false, &mut |_, _| Err(
                        "must not replay".into()
                    ))
                    .is_err()
            );
        }
        assert_eq!(requested.len(), 2);
    }
}
