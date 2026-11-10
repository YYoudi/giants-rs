//! Mission resource/identity binding, before admission to common world callbacks.
//! No population, clock, random stream or successful actor update is fabricated.
use giants_game::{
    attached_effect_context::Pool,
    declared_residents::{Key, Report, Scope},
    instances::Plan,
    resident_states::{Id, Session},
    vimp_actor::Factory,
};

pub(super) struct State {
    factory: Factory,
    globals: giants_formats::global_data::GlobalData,
    expected: Vec<(Key, i32)>,
    bound: Option<Binding>,
}
struct Binding {
    ticket: u64,
    declarations: Vec<(Key, Id, i32)>,
    contexts: Pool<Id>,
}
#[derive(Debug, PartialEq, Eq)]
pub(super) struct View {
    pub herds: usize,
    pub markers: usize,
    pub vimps: usize,
    pub effect_contexts: usize,
}
impl State {
    pub fn prepare(root: &std::path::Path, plan: &Plan) -> Result<Option<Self>, String> {
        let mut expected: Vec<_> = plan
            .world_entries
            .iter()
            .enumerate()
            .filter(|(_, p)| matches!(p.declaration.object_type, 49 | 679))
            .map(|(index, p)| {
                (
                    Key {
                        scope: Scope::World,
                        index,
                    },
                    p.declaration.object_type as i32,
                )
            })
            .collect();
        if !expected.iter().any(|(_, class)| *class == 49) {
            return Ok(None);
        }
        expected.extend(plan.entries.iter().enumerate().filter_map(|(index, p)| {
            let class = p.source.as_ref().ok()?.class;
            matches!(class, 49 | 679).then_some((
                Key {
                    scope: Scope::Mission,
                    index,
                },
                class,
            ))
        }));
        let globals = giants_formats::global_data::parse(
            &std::fs::read(root.join("Bin/GData.gbn")).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        let factory = Factory::load_registered(plan.resources())?;
        Ok(Some(Self {
            factory,
            globals,
            expected,
            bound: None,
        }))
    }
    pub fn bind(
        &mut self,
        ticket: u64,
        report: &Report,
        session: &Session,
    ) -> Result<View, String> {
        if self.bound.is_some() {
            return Err(
                "NPC mission resources already bound; new entry requires fresh preparation".into(),
            );
        }
        // Validate actual source constants before exposing resources to a future
        // birth callback. This does not advance its clock or select a seed.
        for at in [0xbb8, 0xbbc, 0xbc0] {
            if !self
                .globals
                .scalar(at)
                .map_err(|e| e.to_string())?
                .is_finite()
            {
                return Err("NPC birth parameter is nonfinite".into());
            }
        }
        let mut declarations = Vec::with_capacity(self.expected.len());
        for &(key, class) in &self.expected {
            let record = report
                .records
                .iter()
                .find(|r| r.key == key)
                .ok_or("NPC declaration construction missing")?;
            let id = *record
                .result
                .as_ref()
                .map_err(|e| format!("NPC declaration construction refused: {e}"))?;
            Self::check(session, ticket, key, id, class)?;
            declarations.push((key, id, class));
        }
        self.bound = Some(Binding {
            ticket,
            declarations,
            contexts: self.factory.animation_contexts(ticket),
        });
        self.view(session, ticket)
    }
    fn check(session: &Session, ticket: u64, key: Key, id: Id, class: i32) -> Result<(), String> {
        let actor = session
            .get(id)
            .ok_or("NPC declaration owner absent or foreign")?;
        if id.ticket() != ticket
            || session.declaration(key) != Some(id)
            || actor.class != class
            || (class == 49 && session.herd_clock(id).is_none())
            || (class == 679 && session.herd_marker(id).is_none())
        {
            return Err("NPC declaration identity or source construction changed".into());
        }
        Ok(())
    }
    pub fn view(&self, session: &Session, ticket: u64) -> Result<View, String> {
        let b = self
            .bound
            .as_ref()
            .ok_or("NPC mission resources not bound")?;
        if b.ticket != ticket {
            return Err("NPC mission ticket differs".into());
        }
        for &(key, id, class) in &b.declarations {
            Self::check(session, ticket, key, id, class)?;
        }
        Ok(View {
            herds: b
                .declarations
                .iter()
                .filter(|(_, _, class)| *class == 49)
                .count(),
            markers: b
                .declarations
                .iter()
                .filter(|(_, _, class)| *class == 679)
                .count(),
            // Count for diagnostics only; never rediscover an owner from class.
            vimps: session
                .list(0)
                .into_iter()
                .filter(|&id| session.vimp(id).is_some())
                .count(),
            effect_contexts: b.contexts.active().len(),
        })
    }
}
