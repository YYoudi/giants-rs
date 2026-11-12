//! Declared Ripper component owners for ordinary missions. Construction/binding
//! alone acknowledges no AI, current pose, combat or rendered availability.
use giants_game::{
    declared_residents::{Key, Report, Scope},
    instances::Plan,
    resident_states::{Id, Session},
};

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
}
#[derive(Debug, PartialEq, Eq)]
pub(super) struct View {
    pub class73: usize,
    pub class230: usize,
}
impl State {
    pub(super) fn prepare(plan: &Plan) -> Result<Option<Self>, String> {
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
        Ok(Some(Self {
            expected,
            bound: None,
        }))
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
