//! Canonical retained smoke construction and explicit stop marking only.
//! Object callbacks, descendants and eventual dead-object destruction are separate.
use super::*;
use crate::{attached_effect_context::Id as ContextId, effect_actor::Placement};

#[derive(Debug)]
pub(super) struct Metadata {
    pub origin: ContextId,
    pub parent: Id,
    pub reference: u32,
    pub tag: u32,
    pub render_flags: u32,
    pub matrix: crate::procedural_aim::Matrix,
    pub emission: crate::mecc_smoke_effect::Emission,
    pub stopped: bool,
}

impl Session {
    pub(crate) fn validate_smoke_parent_scope(&self, parent: Id) -> Result<(), String> {
        if parent.ticket != self.ticket {
            return Err("smoke parent belongs to another Session".into());
        }
        Ok(())
    }
    pub(crate) fn adopt_smoke(
        &mut self,
        staged: crate::mecc_smoke_effect::Staged<Id>,
    ) -> Result<Id, String> {
        let Placement::Attached {
            recipient,
            reference,
            tag,
            ..
        } = staged.initial.placement
        else {
            return Err("retained smoke requires actual attached placement".into());
        };
        if self
            .get(recipient)
            .and_then(|a| a.construction_fields.as_ref())
            .is_none()
        {
            return Err("retained smoke parent absent, foreign or unconstructed".into());
        }
        let initial = staged.initial;
        let template = staged.template;
        let mut pending = Self::new(self.ticket);
        pending.next = self.next;
        let id = pending.register_with_mask(
            initial.fields.class,
            &initial.seed,
            &template.bindings,
            DirectFlags {
                active: initial.fields.object_flags,
                personality: initial.fields.configuration[14],
            },
            0,
        )?;
        pending.bind_specification(id, template.specification)?;
        pending.bind_state_resources(id, template.states)?;
        let channels = initial.fields.channels;
        pending.bind_actor_fields(id, initial.fields)?;
        let actor = pending.residents.get_mut(&id.serial).unwrap();
        actor.seed = initial.seed;
        actor.channels = Some(channels);
        actor.presentation_parent = Some(recipient);
        actor.acquired_construction = true;
        actor.smoke_effect = Some(Metadata {
            origin: initial.origin,
            parent: recipient,
            reference,
            tag,
            render_flags: initial.render_flags,
            matrix: initial.matrix,
            emission: staged.emission,
            stopped: false,
        });
        let id = self.publish_staged(pending, id)?;
        // The VM adds this head registration after the constructor's common list.
        self.links
            .insert(14, id.serial, crate::object_links::Position::Head);
        Ok(id)
    }
    pub fn smoke_effect_origin(&self, id: Id) -> Option<ContextId> {
        Some(self.get(id)?.smoke_effect.as_ref()?.origin)
    }
    pub fn smoke_effect_emission(&self, id: Id) -> Option<&crate::mecc_smoke_effect::Emission> {
        Some(&self.get(id)?.smoke_effect.as_ref()?.emission)
    }
    pub fn smoke_effect_stopped(&self, id: Id) -> Option<bool> {
        Some(self.get(id)?.smoke_effect.as_ref()?.stopped)
    }
    pub fn smoke_effect_attachment(&self, id: Id) -> Option<(Id, u32, u32)> {
        let metadata = self.get(id)?.smoke_effect.as_ref()?;
        Some((metadata.parent, metadata.reference, metadata.tag))
    }
    /// Constructor-local cache only; no current parent reference was resolved.
    pub fn smoke_effect_initial_matrix(&self, id: Id) -> Option<crate::procedural_aim::Matrix> {
        Some(self.get(id)?.smoke_effect.as_ref()?.matrix)
    }
    pub(crate) fn validate_smoke(
        &self,
        id: Id,
        origin: ContextId,
        parent: Id,
    ) -> Result<(), String> {
        let actor = self
            .get(id)
            .ok_or("retained smoke actor absent or foreign")?;
        let metadata = actor
            .smoke_effect
            .as_ref()
            .ok_or("retained smoke ownership absent")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("retained smoke fields absent")?;
        let payload = actor
            .seed
            .storage
            .data()
            .ok_or("retained smoke payload absent")?;
        if metadata.origin != origin
            || metadata.parent != parent
            || metadata.stopped
            || actor.seed.storage.state() != 78
            || payload.len() != 268
            || actor.presentation_parent != Some(parent)
            || actor.attachment.is_some()
            || fields.control_flags & 1 != 0
            || metadata.render_flags & 6 != 0
            || fields.action != 0
            || payload[0x108..0x10c] != [0; 4]
            || !self.list(14).contains(&id)
        {
            return Err("retained smoke stop outside acquired unadvanced ownership profile".into());
        }
        Ok(())
    }
    pub(crate) fn stop_smoke(
        &mut self,
        id: Id,
        origin: ContextId,
        parent: Id,
    ) -> Result<(), String> {
        self.validate_smoke(id, origin, parent)?;
        let actor = self.residents.get_mut(&id.serial).unwrap();
        // The acquired bounded stop marks dead. It neither frees this actor nor
        // removes its registrations; the later dead-object pass owns those.
        actor.construction_fields.as_mut().unwrap().object_flags |= 0x8000;
        actor.smoke_effect.as_mut().unwrap().stopped = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        attached_effect_context::{Creation, DamageSmokePhase, Pool},
        audio_parameters::Random,
        scene_resources::SceneResources,
    };
    fn parent(session: &mut Session, resources: &SceneResources) -> Id {
        let program = resources
            .classes
            .models
            .first_program(73)
            .unwrap()
            .program
            .as_ref()
            .unwrap();
        let seed = Seed::from_program_for(
            program,
            Profile::Debug,
            crate::arithmetic::Precision::Double53,
        )
        .unwrap();
        let specification =
            Arc::new(crate::specification::Prepared::load_registered(resources, 73).unwrap());
        let fields = crate::actor_fields::Fields::load(resources, 73, &specification)
            .unwrap()
            .instantiate();
        session
            .construct(Construction {
                class: 73,
                seed: &seed,
                bindings: &resources
                    .class_state_bindings_for(73, Profile::Debug)
                    .unwrap(),
                flags: DirectFlags {
                    active: fields.object_flags,
                    personality: fields.configuration[14],
                },
                specification,
                state_resources: Arc::new(
                    crate::state_resources::Prepared::load(resources, 73, Profile::Debug).unwrap(),
                ),
                fields,
            })
            .unwrap()
    }
    #[test]
    #[ignore = "Requires GIANTS_MECC_WORLD_ROOT; supplied parent death and retained context cleanup, no world/particle callbacks"]
    fn source_mecc_smoke_parent_loss_retains_context_and_refuses_missing_storage() {
        let root = std::path::PathBuf::from(std::env::var_os("GIANTS_MECC_WORLD_ROOT").unwrap());
        let resources = SceneResources::load(&root, "w_Story2.bin", "English").unwrap();
        for active in [false, true] {
            let mut session = Session::new(974);
            let source = parent(&mut session, &resources);
            let peer_source = parent(&mut session, &resources);
            let mut pool = Pool::from_active_resources(975, &resources);
            let mut random = Random(42);
            let Creation::Prepared(id) = pool
                .prepare_extended_invariant("FX_mecc_smoking_light", source, 0, 3, &mut random)
                .unwrap()
                .0
            else {
                panic!("source absent")
            };
            let Creation::Prepared(peer) = pool
                .prepare_extended_invariant(
                    "FX_mecc_smoking_medium",
                    peer_source,
                    0,
                    3,
                    &mut random,
                )
                .unwrap()
                .0
            else {
                panic!("peer absent")
            };
            let emitter = active.then(|| {
                pool.activate_damage_smoke(id, &mut session, &resources, &mut random)
                    .unwrap()
            });
            let phase = if active {
                DamageSmokePhase::Active
            } else {
                DamageSmokePhase::Prepared
            };
            assert!(
                pool.refresh_damage_smoke_parent(id, &Session::new(976))
                    .is_err()
            );
            assert_eq!(pool.get(id).unwrap().damage_smoke_phase(), Some(phase));
            let original = session
                .get(source)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap()
                .object_flags;
            for mask in [0x4000, 0x80000000, 0x800000] {
                session
                    .residents
                    .get_mut(&source.serial)
                    .unwrap()
                    .construction_fields
                    .as_mut()
                    .unwrap()
                    .object_flags = original | mask;
                assert_eq!(
                    pool.refresh_damage_smoke_parent(id, &session).unwrap(),
                    phase
                );
                assert_eq!(pool.get(id).unwrap().recipient(), Some(source));
            }
            let before = random;
            let cursor = pool.get(id).unwrap().cursor();
            session
                .residents
                .get_mut(&source.serial)
                .unwrap()
                .construction_fields
                .as_mut()
                .unwrap()
                .object_flags = original | 0x8000;
            for _ in 0..2 {
                assert_eq!(
                    pool.refresh_damage_smoke_parent(id, &session).unwrap(),
                    DamageSmokePhase::ParentLost
                );
                assert_eq!(pool.get(id).unwrap().recipient(), None);
                assert_eq!(pool.get(id).unwrap().origin_recipient(), Some(source));
                assert_eq!(pool.get(id).unwrap().cursor(), cursor);
                assert_eq!(pool.get(peer).unwrap().recipient(), Some(peer_source));
            }
            assert!(
                pool.refresh_damage_smoke_parent(id, &Session::new(976))
                    .is_err()
            );
            assert!(
                pool.activate_damage_smoke(id, &mut session, &resources, &mut random)
                    .is_err()
            );
            assert_eq!(random.0, before.0);
            assert_eq!(
                pool.staged_copy().get(id).unwrap().damage_smoke_phase(),
                Some(DamageSmokePhase::ParentLost)
            );
            if let Some(emitter) = emitter {
                assert_eq!(session.smoke_effect_stopped(emitter), Some(false));
                assert_eq!(
                    session
                        .get(emitter)
                        .unwrap()
                        .construction_fields
                        .as_ref()
                        .unwrap()
                        .object_flags
                        & 0x8000,
                    0
                );
                pool.release_damage_smoke(id, &mut session).unwrap();
                assert_eq!(session.smoke_effect_stopped(emitter), Some(true));
                assert!(session.list(14).contains(&emitter));
            } else {
                pool.release_prepared(id).unwrap();
            }
            assert!(pool.get(id).is_none());
            pool.release_prepared(peer).unwrap();
            // Missing owner storage cannot be treated as an observed dead bit.
            let Creation::Prepared(missing) = pool
                .prepare_extended_invariant("FX_mecc_smoking_light", source, 0, 3, &mut random)
                .unwrap()
                .0
            else {
                panic!("missing request absent")
            };
            session.clear();
            assert!(pool.refresh_damage_smoke_parent(missing, &session).is_err());
            assert_eq!(
                pool.get(missing).unwrap().damage_smoke_phase(),
                Some(DamageSmokePhase::Prepared)
            );
            assert_eq!(pool.get(missing).unwrap().recipient(), Some(source));
            pool.release_prepared(missing).unwrap();
        }
    }
}
