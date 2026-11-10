//! Source-domain constant effects in the canonical Session. Virtual Flick
//! support slots are not ordinary actor IDs, clocks or world-stock admission.
use super::*;
use crate::{
    attached_effect_object::{CinematicObject, Metadata},
    procedural_aim::Matrix,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Scope {
    pub ticket: u64,
    pub director: usize,
    pub generation: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Support {
    pub scope: Scope,
    /// Actual shared effect-support slot, distinct from the source actor slot.
    pub slot: usize,
}
pub(super) struct Owned {
    metadata: Metadata<Support>,
    durations: Vec<Option<f32>>,
    updates: u64,
    failure: Option<String>,
}
impl std::fmt::Debug for Owned {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CinematicConstant")
            .field("support", &self.metadata.parent)
            .field("updates", &self.updates)
            .field("failure", &self.failure)
            .finish_non_exhaustive()
    }
}
pub struct View<'a> {
    pub fields: &'a crate::actor_fields::Fields,
    pub metadata: &'a Metadata<Support>,
    pub world: Matrix,
    pub updates: u64,
}
impl Session {
    /// Move an unadvanced source-marked constant body into the one actor store.
    /// No finite lifetime, ordinary parent slot or native pool counter is invented.
    pub fn adopt_cinematic_constant(
        &mut self,
        object: CinematicObject<Support>,
    ) -> Result<Id, String> {
        let parts = object.into_parts()?;
        let f = &parts.fields;
        if parts.metadata.parent.scope.ticket != self.ticket
            || parts.metadata.tag != u32::MAX
            || f.class != 71
            || f.configuration[14] & 0x84 == 0
            || parts.seed.storage.profile() != Profile::Debug
            || parts.seed.storage.state() != 78
            || parts
                .seed
                .storage
                .data()
                .is_none_or(|data| data.len() != 268)
            || parts.metadata.emitter.is_some()
            || parts.metadata.emitter_request.is_some()
            || parts.metadata.render_flags != 0x1000
            || !parts
                .references
                .specification()
                .metadata
                .events
                .iter()
                .all(|e| e.events.is_empty())
        {
            return Err(
                "cinematic constant outside acquired source/constructor/ANM profile".into(),
            );
        }
        let flags = DirectFlags {
            active: f.object_flags,
            personality: f.configuration[14],
        };
        let mut staged = Session::new(self.ticket);
        staged.next = self.next;
        // The acquired attached constructor supplies zero extra memberships.
        // Actual common registration remains present; no fake parent is inserted.
        let id = staged.register_with_mask(f.class, &parts.seed, &parts.bindings, flags, 0)?;
        staged
            .specifications
            .insert(id.serial, OwnedSpecification(parts.references));
        staged.bind_state_resources(id, parts.states)?;
        let channels = f.channels;
        staged.bind_actor_fields(id, parts.fields)?;
        let actor = staged.residents.get_mut(&id.serial).unwrap();
        actor.seed = parts.seed;
        actor.channels = Some(channels);
        actor.acquired_construction = true;
        actor.cinematic_constant = Some(Owned {
            metadata: parts.metadata,
            durations: parts.durations,
            updates: 0,
            failure: None,
        });
        self.publish_staged(staged, id)
    }
    pub fn cinematic_constant_failure(&self, id: Id) -> Option<&str> {
        self.get(id)?
            .cinematic_constant
            .as_ref()?
            .failure
            .as_deref()
    }
    pub fn cinematic_constant_view(&self, id: Id, support: Support) -> Option<View<'_>> {
        let actor = self.get(id)?;
        let owned = actor.cinematic_constant.as_ref()?;
        if owned.failure.is_some() || owned.metadata.parent != support {
            return None;
        }
        Some(View {
            fields: actor.construction_fields.as_ref()?,
            metadata: &owned.metadata,
            world: owned.metadata.world_matrix?,
            updates: owned.updates,
        })
    }
    pub fn advance_cinematic_constant(
        &mut self,
        id: Id,
        support: Support,
        matrix: Matrix,
        seconds: f32,
    ) -> Result<(), String> {
        if !seconds.is_finite() || seconds < 0. {
            return Err("cinematic callback delta invalid".into());
        }
        let actor = self.get(id).ok_or("cinematic owner absent or foreign")?;
        let owned = actor
            .cinematic_constant
            .as_ref()
            .ok_or("cinematic constant absent")?;
        if owned.failure.is_some() || owned.metadata.parent != support {
            return Err("cinematic support foreign, stale or quarantined".into());
        }
        let actor = self.residents.get_mut(&id.serial).unwrap();
        let owned = actor.cinematic_constant.as_mut().unwrap();
        let fields = actor.construction_fields.as_mut().unwrap();
        let references = &mut self
            .specifications
            .get_mut(&id.serial)
            .ok_or("cinematic references absent")?
            .0;
        let result = crate::attached_effect_object::advance_constant(
            fields,
            &mut owned.metadata,
            references,
            &owned.durations,
            matrix,
            seconds,
        );
        // Retain the actual channel prefix, including a late reference failure.
        actor.channels = Some(fields.channels);
        match &result {
            Ok(()) => owned.updates += 1,
            Err(error) => owned.failure = Some(error.clone()),
        }
        result
    }
    /// Host cancellation of the source-only cohort. Native world retirement,
    /// capacity and stock accounting remain separate services.
    pub fn cancel_cinematic_scope(&mut self, scope: Scope) -> Result<usize, String> {
        if scope.ticket != self.ticket || self.common_walk_running {
            return Err("cinematic cancellation needs its idle owning Session".into());
        }
        let ids: std::collections::BTreeSet<_> = self
            .residents
            .iter()
            .filter_map(|(&serial, a)| {
                let constant = a
                    .cinematic_constant
                    .as_ref()
                    .is_some_and(|c| c.metadata.parent.scope == scope);
                let timed = a
                    .timed_effect
                    .as_ref()
                    .and_then(|m| m.cinematic_scope())
                    .is_some_and(|owner| owner == scope);
                (constant || timed).then_some(Id {
                    ticket: self.ticket,
                    serial,
                })
            })
            .collect();
        for id in &ids {
            if self
                .get(*id)
                .and_then(|a| a.timed_effect.as_ref())
                .is_some_and(|m| m.callback_pending())
            {
                return Err("cinematic cancellation cannot remove a pending timed callback".into());
            }
            if !self.incoming_actor_references(*id)?.is_empty() {
                return Err("cinematic source actor has a live typed dependent".into());
            }
        }
        for id in &ids {
            self.links.remove_object(id.serial);
            self.residents.remove(&id.serial);
            self.specifications.remove(&id.serial);
            self.state_resources.remove(&id.serial);
        }
        Ok(ids.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        attached_effect_context::{Creation, Pool},
        attached_effect_object::CinematicTemplate,
    };
    use std::{collections::BTreeMap, path::PathBuf, sync::Arc};

    #[test]
    #[ignore = "Requires GIANTS_FLICK_ROOT and private GIANTS_FLICK_PERSISTENT_COHORT TSV; silent ownership, not fidelity"]
    fn source_constant_cohort_moves_actual_body_and_isolates_source_scopes() {
        let root = PathBuf::from(std::env::var_os("GIANTS_FLICK_ROOT").unwrap());
        let manifest =
            std::fs::read_to_string(std::env::var_os("GIANTS_FLICK_PERSISTENT_COHORT").unwrap())
                .unwrap();
        let mut groups = BTreeMap::<String, Vec<String>>::new();
        for line in manifest.lines() {
            let (world, source) = line.split_once('\t').unwrap();
            groups.entry(world.into()).or_default().push(source.into());
        }
        let (mut sources, mut objects, mut lifecycles) = (0, 0, 0);
        for (world, sequences) in groups {
            let world = crate::world_source::resolve(&root, &world).unwrap();
            let resources =
                crate::scene_resources::SceneResources::load(&root, &world.catalog, "English")
                    .unwrap();
            for sequence in sequences {
                let bytes =
                    std::fs::read(crate::flick::sequence_path(&root, &sequence).unwrap()).unwrap();
                let binding =
                    crate::cinematic_persistent::Bindings::load(&resources, &bytes).unwrap();
                let metadata = giants_formats::flick::discover_dependencies(&bytes).unwrap();
                let requests = crate::cinematic_fx::declarations(&bytes, &metadata).unwrap();
                let scope = Scope {
                    ticket: 900,
                    director: sources,
                    generation: 1,
                };
                let mut session = Session::new(scope.ticket);
                for request in &requests {
                    if !binding.owns(request.command_offset) {
                        continue;
                    }
                    let support = Support {
                        scope,
                        slot: request.slot as usize,
                    };
                    let program = resources
                        .bound_effect_program(&request.target(&resources).unwrap())
                        .unwrap();
                    let recipe = crate::effect_attachment::Recipe::project(&program).unwrap();
                    let template = CinematicTemplate::load(&resources, recipe.class).unwrap();
                    let image = Arc::new(
                        resources
                            .classes
                            .models
                            .effects
                            .last()
                            .unwrap()
                            .image
                            .clone(),
                    );
                    let mut pool = Pool::new(scope.ticket, Some(image));
                    let Creation::Prepared(context) = pool
                        .prepare_cinematic(request.effect.as_deref().unwrap(), support)
                        .unwrap()
                    else {
                        panic!("context absent")
                    };
                    let object = template
                        .construct(context, pool.get(context).unwrap())
                        .unwrap();
                    let payload = object.seed.storage.data().unwrap().as_ptr();
                    let original_bytes = object.seed.storage.data().unwrap().to_vec();
                    let mut projection = template
                        .construct(context, pool.get(context).unwrap())
                        .unwrap();
                    let id = session.adopt_cinematic_constant(object).unwrap();
                    pool.release_prepared(context).unwrap();
                    assert_eq!(
                        session
                            .get(id)
                            .unwrap()
                            .seed
                            .storage
                            .data()
                            .unwrap()
                            .as_ptr(),
                        payload
                    );
                    assert!(session.cinematic_constant_view(id, support).is_none());
                    assert!(session.list(0).contains(&id));
                    for list in [1, 2, 3] {
                        assert!(!session.list(list).contains(&id));
                    }
                    assert!(
                        Session::new(scope.ticket + 1)
                            .advance_cinematic_constant(id, support, Matrix::identity(), 0.)
                            .is_err()
                    );
                    for altered in [
                        Support {
                            slot: support.slot + 1,
                            ..support
                        },
                        Support {
                            scope: Scope {
                                director: scope.director + 1,
                                ..scope
                            },
                            ..support
                        },
                        Support {
                            scope: Scope {
                                generation: 2,
                                ..scope
                            },
                            ..support
                        },
                    ] {
                        assert!(
                            session
                                .advance_cinematic_constant(id, altered, Matrix::identity(), 0.)
                                .is_err()
                        );
                    }
                    assert!(session.cinematic_constant_failure(id).is_none());
                    for (step, delta) in [0., 0.125, 0.5, 1000.].into_iter().enumerate() {
                        let mut matrix = Matrix::identity();
                        matrix.values[3] = step as f32 * 17.;
                        projection.update(matrix, delta).unwrap();
                        session
                            .advance_cinematic_constant(id, support, matrix, delta)
                            .unwrap();
                        let view = session.cinematic_constant_view(id, support).unwrap();
                        assert_eq!(
                            view.world.values.map(f32::to_bits),
                            projection.world_matrix().unwrap().values.map(f32::to_bits)
                        );
                        assert_eq!(view.fields.channels, projection.fields.channels);
                        assert_eq!(view.fields.position, projection.fields.position);
                        assert_eq!(view.fields.previous, projection.fields.previous);
                        assert_eq!(view.fields.control_flags, 0);
                        assert_eq!(view.updates, step as u64 + 1);
                        assert_eq!(
                            session.get(id).unwrap().seed.storage.data().unwrap(),
                            original_bytes
                        );
                    }
                    assert!(
                        session
                            .advance_cinematic_constant(id, support, Matrix::identity(), f32::NAN)
                            .is_err()
                    );
                    assert_eq!(
                        session
                            .cinematic_constant_view(id, support)
                            .unwrap()
                            .updates,
                        4
                    );
                    // A late parent bind failure keeps the actual callback prefix,
                    // hides the stale presentation and refuses another callback.
                    session
                        .residents
                        .get_mut(&id.serial)
                        .unwrap()
                        .construction_fields
                        .as_mut()
                        .unwrap()
                        .control_flags = 0x20;
                    let mut invalid = Matrix::identity();
                    invalid.values[3] = f32::NAN;
                    assert!(
                        session
                            .advance_cinematic_constant(id, support, invalid, 0.125)
                            .is_err()
                    );
                    assert_eq!(
                        session
                            .get(id)
                            .unwrap()
                            .construction_fields
                            .as_ref()
                            .unwrap()
                            .control_flags,
                        0
                    );
                    assert!(session.cinematic_constant_failure(id).is_some());
                    assert!(session.cinematic_constant_view(id, support).is_none());
                    assert!(
                        session
                            .advance_cinematic_constant(id, support, Matrix::identity(), 0.)
                            .is_err()
                    );
                    assert_eq!(
                        session
                            .get(id)
                            .unwrap()
                            .seed
                            .storage
                            .data()
                            .unwrap()
                            .as_ptr(),
                        payload
                    );
                    objects += 1;
                }
                let target = session.list(0)[0];
                let program = resources
                    .classes
                    .models
                    .first_program(71)
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
                // Synthetic unrelated registry peer, not a claimed native actor
                // constructor. The shared reference preflight must protect it.
                let peer = session
                    .register(
                        71,
                        &seed,
                        &[],
                        DirectFlags {
                            active: 0,
                            personality: 0,
                        },
                    )
                    .unwrap();
                session.set_presentation_parent(peer, Some(target)).unwrap();
                let before = session.list(0);
                assert!(session.cancel_cinematic_scope(scope).is_err());
                assert_eq!(session.list(0), before);
                session.set_presentation_parent(peer, None).unwrap();
                assert_eq!(
                    session.cancel_cinematic_scope(scope).unwrap(),
                    binding.persistent_requests()
                );
                assert!(session.get(peer).is_some());
                assert!(session.get(target).is_none());
                assert_eq!(session.residents.len(), 1);
                assert!(session.specifications.is_empty());
                assert!(session.state_resources.is_empty());
                assert_eq!(session.list(0), [peer]);
                // Two coexisting generations and a following fresh ticket must
                // retain only their own bodies. No ordinary parent is fabricated.
                for ticket in [901, 903] {
                    let mut session = Session::new(ticket);
                    let first = Scope { ticket, ..scope };
                    let second = Scope {
                        generation: 2,
                        ..first
                    };
                    let mut a = binding.queue(first);
                    let mut b = binding.queue(second);
                    for request in &requests {
                        a.dispatch(&binding, request.command_offset, 0., &mut session)
                            .unwrap();
                        b.dispatch(&binding, request.command_offset, 0., &mut session)
                            .unwrap();
                    }
                    for time in [0., 0.125, 0.25, 0.5] {
                        a.update(&binding, time, 0.125, &mut session).unwrap();
                        b.update(&binding, time, 0.125, &mut session).unwrap();
                        for request in &requests {
                            if binding.owns(request.command_offset) {
                                assert!(a.frame(&session, request.command_offset).is_some());
                                assert!(b.frame(&session, request.command_offset).is_some());
                            }
                        }
                    }
                    assert_eq!(a.contexts(), 0);
                    assert_eq!(
                        a.release(Some(&mut session)).unwrap(),
                        binding.persistent_requests()
                    );
                    assert_eq!(session.list(0).len(), binding.persistent_requests());
                    assert_eq!(a.objects(), 0);
                    b.update(&binding, 1., 0.125, &mut session).unwrap();
                    assert_eq!(
                        b.release(Some(&mut session)).unwrap(),
                        binding.persistent_requests()
                    );
                    assert!(session.list(0).is_empty());
                    assert!(session.residents.is_empty());
                    lifecycles += 2;
                }
                sources += 1;
            }
        }
        assert!(sources > 0 && objects > 0);
        eprintln!(
            "constant ownership sources={sources} requests={objects} queue_lifecycles={lifecycles}; source clocks/support supplied; no native admission/fidelity claim"
        );
    }
}
