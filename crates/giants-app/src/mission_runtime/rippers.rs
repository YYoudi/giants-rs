//! Declared Ripper component owners for ordinary missions. Construction/binding
//! alone acknowledges no AI, current pose, combat or rendered availability.
use giants_game::{
    declared_residents::{Key, Report, Scope},
    instances::Plan,
    resident_states::{Id, Session},
};
mod contacts;
mod draw;
pub(super) mod live;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Declaration {
    key: Key,
    class: i32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Owner {
    declaration: Declaration,
    id: Id,
}
pub(super) struct State {
    expected: Vec<Declaration>,
    bound: Option<(u64, Vec<Owner>)>,
    preparation: Option<Result<(), String>>,
    globals: giants_formats::global_data::GlobalData,
    image: std::sync::Arc<giants_formats::effect_image::Image>,
    contexts: Option<giants_game::attached_effect_context::Pool<Id>>,
    driver: live::Driver,
    geometry: draw::Geometry,
    sounds: Vec<crate::mission_audio::Event>,
    marker_factory: giants_game::primary_surface_marker::Factory<'static>,
}
#[derive(Debug, PartialEq, Eq)]
pub(super) struct View {
    pub class73: usize,
    pub class230: usize,
}
impl State {
    pub(super) fn prepare(root: &std::path::Path, plan: &Plan) -> Result<Option<Self>, String> {
        let mut expected = Vec::new();
        for (index, entry) in plan.world_entries.iter().enumerate() {
            if matches!(entry.declaration.object_type, 73 | 230) {
                let source = entry.class_input()?;
                expected.push(Declaration {
                    key: Key {
                        scope: Scope::World,
                        index,
                    },
                    class: source.class,
                });
            }
        }
        for (index, entry) in plan.entries.iter().enumerate() {
            if matches!(entry.serialized.object_type, 73 | 230) {
                let source = entry.source.as_ref().map_err(Clone::clone)?;
                expected.push(Declaration {
                    key: Key {
                        scope: Scope::Mission,
                        index,
                    },
                    class: source.class,
                });
            }
        }
        if expected.is_empty() {
            return Ok(None);
        }
        if expected.iter().any(|d| !matches!(d.class, 73 | 230)) {
            return Err("Ripper declaration resolves to a different source class".into());
        }
        let globals = giants_formats::global_data::parse(
            &std::fs::read(root.join("Bin/GData.gbn")).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        let models = &plan.resources().classes.models;
        let mut image: Option<&giants_formats::effect_image::Image> = None;
        for class in expected
            .iter()
            .map(|d| d.class)
            .collect::<std::collections::BTreeSet<_>>()
        {
            let selected = models
                .first_selection(class)
                .ok_or("Ripper class selection absent")?
                .as_ref()
                .map_err(Clone::clone)?;
            let definition = models
                .specs
                .get(&selected.spec_id)
                .and_then(|s| s.first())
                .ok_or("Ripper specification absent")?;
            let captured = models.specification_effects(definition)?;
            if image.is_some_and(|prior| !std::ptr::eq(prior, captured)) {
                return Err("Ripper classes have different captured effect images".into());
            }
            image = Some(captured);
        }
        Ok(Some(Self {
            expected,
            bound: None,
            preparation: None,
            globals,
            image: std::sync::Arc::new(image.unwrap().clone()),
            contexts: None,
            driver: Default::default(),
            geometry: Default::default(),
            sounds: vec![],
            marker_factory: giants_game::primary_surface_marker::Factory::shared(
                plan.resources().clone(),
            ),
        }))
    }
    /// Prepare the real terrain/geometry and cold motion owners before a frame
    /// starts. This advances no history, primary, animation or clock, and does
    /// not acknowledge an active NPC or a completed renderable pose.
    pub(super) fn initialize(
        &mut self,
        session: &mut Session,
        ticket: u64,
        resources: &giants_game::scene_resources::SceneResources,
        world: &giants_game::WorldAsset,
    ) -> Result<(), String> {
        self.view(session, ticket)?;
        if self.preparation.is_some() {
            return Err(
                "Ripper resource initialization already attempted; fresh entry required".into(),
            );
        }
        let result: Result<(), String> = (|| {
            let terrain = giants_game::terrain_queries::StaticTerrainQuery::new(
                &world.terrain,
                1. / world.terrain.spacing,
                giants_game::arithmetic::Precision::Double53,
            )?;
            for owner in &self.bound.as_ref().unwrap().1 {
                session.prepare_actor_affine(owner.id)?;
                session.prepare_actor_environment(owner.id, resources, &terrain)?;
                session.prepare_actor_finalization(owner.id)?;
                session.prepare_actor_motion(owner.id)?;
            }
            Ok(())
        })();
        // A late error may already have prepared preceding owners. Keep that
        // state, report failure and prohibit whole-component replay.
        self.preparation = Some(result.clone());
        if result.is_ok() {
            self.contexts = Some(giants_game::attached_effect_context::Pool::new(
                ticket,
                Some(self.image.clone()),
            ));
        }
        result
    }
    fn check(session: &Session, ticket: u64, owner: Owner) -> Result<(), String> {
        let actor = session
            .get(owner.id)
            .ok_or("Ripper declaration owner absent or foreign")?;
        if owner.id.ticket() != ticket
            || session.declaration(owner.declaration.key) != Some(owner.id)
            || actor.class != owner.declaration.class
            || actor.seed.storage.profile() != giants_game::state_storage::Profile::Debug
            || actor.seed.storage.state() != 23
            || actor
                .seed
                .storage
                .data()
                .is_none_or(|data| data.len() != 88)
            || actor
                .construction_fields
                .as_ref()
                .is_none_or(|f| f.class != owner.declaration.class)
            || session
                .specification(owner.id)
                .is_none_or(|s| s.class != owner.declaration.class || !s.needs.is_empty())
            || session
                .state_resources(owner.id)
                .is_none_or(|s| !s.needs().is_empty())
        {
            return Err("Ripper binding differs from the canonical declared constructor".into());
        }
        Ok(())
    }
    pub(super) fn bind(
        &mut self,
        ticket: u64,
        report: &Report,
        session: &Session,
    ) -> Result<View, String> {
        if self.bound.is_some() {
            return Err(
                "Ripper component already bound; re-entry requires fresh preparation".into(),
            );
        }
        let owners = self
            .expected
            .iter()
            .map(|&declaration| {
                let mut records = report.records.iter().filter(|r| r.key == declaration.key);
                let record = records
                    .next()
                    .ok_or("Ripper declaration construction missing")?;
                if records.next().is_some() || record.class != Some(declaration.class) {
                    return Err("Ripper construction report identity is ambiguous".into());
                }
                let owner = Owner {
                    declaration,
                    id: *record.result.as_ref().map_err(Clone::clone)?,
                };
                Self::check(session, ticket, owner)?;
                Ok(owner)
            })
            .collect::<Result<Vec<_>, String>>()?;
        self.bound = Some((ticket, owners));
        self.view(session, ticket)
    }
    pub(super) fn view(&self, session: &Session, ticket: u64) -> Result<View, String> {
        let (bound_ticket, owners) = self.bound.as_ref().ok_or("Ripper declarations not bound")?;
        if *bound_ticket != ticket {
            return Err("Ripper component ticket differs".into());
        }
        for &owner in owners {
            Self::check(session, ticket, owner)?;
        }
        Ok(View {
            class73: owners.iter().filter(|o| o.declaration.class == 73).count(),
            class230: owners.iter().filter(|o| o.declaration.class == 230).count(),
        })
    }
}

#[cfg(test)]
mod tests;
