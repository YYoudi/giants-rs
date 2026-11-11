//! Actual state84 meat/ghost lifetime for a population without mana-eligible
//! players. Delphi/other mana collection keeps its explicit service prerequisite.
//! Prototype PS2 search key: `vimpsoul_move`. This bounded no-mana branch is
//! a navigation target, not a confirmed complete PS2 function equivalent.
//! Exact PC attribution remains unknown; see docs/SYMBOL_NAVIGATION.md.
use super::*;
impl Session {
    pub fn advance_vimp_death_proxy_without_mana(
        &mut self,
        id: Id,
        number: u64,
        globals: &giants_formats::global_data::GlobalData,
        contexts: &mut crate::attached_effect_context::Pool<Id>,
        factory: &crate::vimp_ghost::Factory<'_>,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<Option<Id>, String> {
        let actor = self.get(id).ok_or("soul actor absent or foreign")?;
        let metadata = actor
            .vimp_death_proxy
            .ok_or("soul source construction absent")?;
        let fields = actor.construction_fields.as_ref().unwrap();
        if actor.class != 818
            || actor.seed.storage.state() != 84
            || fields.configuration[20] != 84
            || fields.object_flags & 0x8000 != 0
            || metadata.parameters[0] != 1
            || metadata.failed
            || metadata.number != number
            || number == u64::MAX
            || fields.control_flags > 2
            || metadata.ghost.is_some() != metadata.context.is_some()
        {
            return Err("soul callback outside contiguous acquired death-ghost profile".into());
        }
        // This is the actual source player-list and header predicate. An
        // eligible family requires the unimplemented mana/collection services;
        // it is never acknowledged as an empty or successful player lookup.
        for player in self.list(3) {
            let actor = self.get(player).ok_or("soul player-list owner absent")?;
            let fields = actor
                .construction_fields
                .as_ref()
                .ok_or("soul player fields absent")?;
            if fields.configuration[20] == 3 || actor.class == 1080 {
                return Err(
                    "soul mana-eligible population requires its transfer/collection providers"
                        .into(),
                );
            }
        }
        let position = fields.position;
        let tier = fields.control_flags;
        let radius = globals.scalar(0xbcc).map_err(|e| e.to_string())?;
        let mut meat_fields = Vec::new();
        for meat in self.list(0) {
            let actor = self.get(meat).ok_or("soul common-list owner absent")?;
            if !(1117..=1119).contains(&actor.class) {
                continue;
            }
            let chunk = actor
                .vimp_chunk
                .ok_or("soul meat source update fields absent")?;
            let fields = actor
                .construction_fields
                .as_ref()
                .ok_or("soul meat header absent")?;
            meat_fields.push(crate::vimp_death_proxy::Meat {
                class: actor.class,
                transparency: chunk.fade,
                control: fields.control_flags,
                position: fields.position,
            });
        }
        let near = crate::vimp_death_proxy::meat_present(position, radius, &meat_fields)?;
        let result = (|| -> Result<Option<Id>, String> {
            if near {
                if metadata.ghost.is_none() && metadata.parameters[1] != 0 {
                    let name = match tier {
                        2 => "fx_vimp_ghost_large",
                        1 => "fx_vimp_ghost_medium",
                        _ => "fx_vimp_ghost_small",
                    };
                    let crate::attached_effect_context::Creation::Prepared(context) =
                        contexts.prepare_vimp_ghost_invariant(name, position, random)?
                    else {
                        return Err("soul source ghost context absent".into());
                    };
                    // Retain preparation even if active construction fails;
                    // quarantine forbids retrying an already consumed request.
                    self.residents
                        .get_mut(&id.serial)
                        .unwrap()
                        .vimp_death_proxy
                        .as_mut()
                        .unwrap()
                        .context = Some(context);
                    let ghost = factory.activate(self, contexts, context)?;
                    let actor = self.residents.get_mut(&id.serial).unwrap();
                    actor.vimp_death_proxy.as_mut().unwrap().ghost = Some(ghost);
                    actor.seed.storage.data_mut().unwrap()[..4]
                        .copy_from_slice(&1u32.to_le_bytes());
                    return Ok(Some(ghost));
                }
            } else {
                if let Some(ghost) = metadata.ghost {
                    self.resume_vimp_ghost(ghost, contexts)?;
                    let actor = self.residents.get_mut(&id.serial).unwrap();
                    let proxy = actor.vimp_death_proxy.as_mut().unwrap();
                    proxy.ghost = None;
                    proxy.context = None;
                }
                self.residents
                    .get_mut(&id.serial)
                    .unwrap()
                    .construction_fields
                    .as_mut()
                    .unwrap()
                    .object_flags |= 0x8000;
            }
            Ok(None)
        })();
        let metadata = self
            .residents
            .get_mut(&id.serial)
            .unwrap()
            .vimp_death_proxy
            .as_mut()
            .unwrap();
        if result.is_ok() {
            metadata.number += 1;
        } else {
            metadata.failed = true;
        }
        result
    }
    pub fn cleanup_vimp_death_proxy(
        &mut self,
        id: Id,
        services: &mut impl crate::actor_retirement::Services,
    ) -> Result<Counts, String> {
        let preflight = |s: &Self| -> Result<Counts, String> {
            let actor = s.get(id).ok_or("soul retirement owner absent or foreign")?;
            let metadata = actor
                .vimp_death_proxy
                .ok_or("soul retirement source absent")?;
            if metadata.failed
                || metadata.ghost.is_some()
                || metadata.context.is_some()
                || actor.seed.storage.state() != 84
                || actor.class != 818
                || actor.construction_fields.as_ref().unwrap().object_flags & 0x8000 == 0
                || s.declarations.values().any(|&value| value == id)
                || !s.incoming_actor_references(id)?.is_empty()
            {
                return Err("soul retirement has unfinished source/external references".into());
            }
            let links = s.links.memberships(id.serial);
            if links.len() != 2
                || !links.iter().any(|l| l.1 == 0)
                || !links.iter().any(|l| l.1 == 11)
            {
                return Err("soul retirement registrations differ".into());
            }
            Ok(Counts {
                residents: 1,
                payload_bytes: 4,
                registrations: 2,
            })
        };
        let expected = preflight(self)?;
        let result: Result<Counts, String> = (|| {
            services.release(self, id)?;
            preflight(self)?;
            self.residents.remove(&id.serial);
            self.state_resources.remove(&id.serial);
            self.specifications.remove(&id.serial);
            if self.links.remove_object(id.serial) != expected.registrations {
                return Err("soul unlink registrations changed".into());
            }
            Ok(expected)
        })();
        if result.is_err()
            && let Some(proxy) = self
                .residents
                .get_mut(&id.serial)
                .and_then(|a| a.vimp_death_proxy.as_mut())
        {
            proxy.failed = true;
        }
        result
    }
}
