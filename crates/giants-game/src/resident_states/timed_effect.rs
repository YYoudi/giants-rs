//! Canonical Session storage for the acquired attached timed-effect dependency.
//! No world admission, stock policy or default physical/retirement provider.
use super::*;
use crate::{
    effect_actor::Placement,
    procedural_aim::Matrix,
    timed_effect_object::{Body, Boundary, Runtime, Step, Timed},
};
use giants_formats::effect_image::Image;

#[derive(Debug)]
pub(super) struct Metadata {
    origin: crate::attached_effect_context::Id,
    placement: Placement<Id>,
    render_flags: u32,
    matrix: Matrix,
    runtime: Runtime,
}
/// Services execute with the actual actor still in the canonical Session map.
/// A callback prefix is already published; errors retain it and forbid replay.
pub trait Services {
    /// `seconds` is the callback's sample. A physical provider retains its own
    /// acquired clock queries; equal samples in a fixture do not merge clocks.
    fn physical(&mut self, session: &mut Session, actor: Id, seconds: f32) -> Result<(), String>;
    fn retire(
        &mut self,
        session: &mut Session,
        actor: Id,
        source: &Image,
        binding: Option<&crate::effect_scene::RetirementBinding>,
    ) -> Result<(), String>;
}
fn borrow<'a>(
    actor: &'a mut Resident,
    references: &'a mut crate::actor_references::Owned,
) -> Result<(&'a mut Runtime, Body<'a, Id>), String> {
    let metadata = actor
        .timed_effect
        .as_mut()
        .ok_or("timed Session ownership absent")?;
    let parent_matches = match metadata.placement {
        Placement::Attached { recipient, .. } => actor.presentation_parent == Some(recipient),
        Placement::Position(_) => actor.presentation_parent.is_none(),
    };
    if !parent_matches || actor.attachment.is_some() {
        return Err("timed canonical parent relationship changed".into());
    }
    Ok((
        &mut metadata.runtime,
        Body {
            fields: actor
                .construction_fields
                .as_mut()
                .ok_or("timed Session fields absent")?,
            seed: &mut actor.seed,
            matrix: &mut metadata.matrix,
            channels: Some(
                actor
                    .channels
                    .as_mut()
                    .ok_or("timed Session channels absent")?,
            ),
            references,
            placement: metadata.placement,
            render_flags: metadata.render_flags,
        },
    ))
}
impl Session {
    /// Stage the acquired attached profile without native stock admission.
    pub(crate) fn adopt_timed_attached(&mut self, owned: Timed<Id>) -> Result<Id, String> {
        if !matches!(owned.initial.placement, Placement::Attached { .. }) {
            return Err("timed attached constructor requires actual parent binding".into());
        }
        self.adopt_timed(owned)
    }
    /// Stage the acquired positional profile. Caller Session selection and
    /// physical/environment/stock services remain explicit world prerequisites.
    pub(crate) fn adopt_timed_positioned(&mut self, owned: Timed<Id>) -> Result<Id, String> {
        if !matches!(owned.initial.placement, Placement::Position(_)) {
            return Err("timed positional constructor requires actual source position".into());
        }
        self.adopt_timed(owned)
    }
    fn adopt_timed(&mut self, mut owned: Timed<Id>) -> Result<Id, String> {
        let parent = match owned.initial.placement {
            Placement::Attached { recipient, .. } => {
                self.get(recipient)
                    .ok_or("timed constructor parent absent or foreign")?;
                Some(recipient)
            }
            Placement::Position(_) => None,
        };
        if owned.runtime.epoch != 0
            || owned.references.sampled()
            || owned.runtime.failure.is_some()
            || !owned.runtime.idle()
            || owned.initial.fields.control_flags & 0x20 == 0
        {
            return Err("timed Session construction requires an unadvanced owner".into());
        }
        // Guard the same actual source fields/payload before any list publication.
        owned.runtime.preflight(&Body {
            fields: &mut owned.initial.fields,
            seed: &mut owned.initial.seed,
            matrix: &mut owned.initial.matrix,
            channels: None,
            references: &mut owned.references,
            placement: owned.initial.placement,
            render_flags: owned.initial.render_flags,
        })?;
        let Timed {
            initial,
            runtime,
            references,
        } = owned;
        let flags = DirectFlags {
            active: initial.fields.object_flags,
            personality: initial.fields.configuration[14],
        };
        let mut staged = Session::new(self.ticket);
        staged.next = self.next;
        let id = staged.register_with_mask(
            initial.fields.class,
            &initial.seed,
            &runtime.bindings,
            flags,
            runtime.constructor_mask,
        )?;
        staged
            .specifications
            .insert(id.serial, OwnedSpecification(references));
        staged.bind_state_resources(id, runtime.states.clone())?;
        let channels = initial.fields.channels;
        staged.bind_actor_fields(id, initial.fields)?;
        let actor = staged.residents.get_mut(&id.serial).unwrap();
        // Replace the staging registration's seed with the original owned
        // payload allocation. Nothing else retains a live state copy.
        actor.seed = initial.seed;
        actor.channels = Some(channels);
        actor.presentation_parent = parent;
        actor.acquired_construction = true;
        if parent.is_none() {
            actor.affine_cache = Some(OwnedAffine {
                value: crate::actor_affine::Cache {
                    flags: initial.matrix.flags,
                    matrix: initial.matrix.values,
                },
                pose: affine_pose_words(actor.construction_fields.as_ref().unwrap()),
                external_orientation: true,
            });
        }
        actor.timed_effect = Some(Metadata {
            origin: initial.origin,
            placement: initial.placement,
            render_flags: initial.render_flags,
            matrix: initial.matrix,
            runtime,
        });
        self.publish_staged(staged, id)
    }
    pub fn timed_effect_origin(&self, id: Id) -> Option<crate::attached_effect_context::Id> {
        Some(self.get(id)?.timed_effect.as_ref()?.origin)
    }
    pub fn timed_effect_failure(&self, id: Id) -> Option<&str> {
        self.get(id)?
            .timed_effect
            .as_ref()?
            .runtime
            .failure
            .as_deref()
    }
    /// Actual acquired header velocity owner for the detached smoke profile.
    pub fn timed_effect_velocity(&self, id: Id) -> Option<[f32; 3]> {
        self.get(id)?.timed_effect.as_ref()?.runtime.velocity
    }
    /// Mutate only at the pending integration boundary, never from a viewer copy.
    pub fn timed_effect_velocity_mut(&mut self, id: Id) -> Result<&mut [f32; 3], String> {
        let metadata = self
            .residents
            .get_mut(&id.serial)
            .filter(|_| id.ticket == self.ticket)
            .and_then(|actor| actor.timed_effect.as_mut())
            .ok_or("timed velocity actor absent or foreign")?;
        if !metadata.runtime.awaiting_physical() {
            return Err("timed velocity integration is not pending".into());
        }
        metadata
            .runtime
            .velocity
            .as_mut()
            .ok_or("timed velocity owner absent".into())
    }
    pub fn timed_effect_transparency(&self, id: Id) -> Option<f32> {
        Some(self.get(id)?.timed_effect.as_ref()?.runtime.transparency)
    }
    pub fn timed_effect_matrix(&self, id: Id) -> Option<Matrix> {
        Some(self.get(id)?.timed_effect.as_ref()?.matrix)
    }
    /// The physical provider can mutate the actual inline fields/payload only
    /// at its pending boundary; this is not a general readiness override.
    pub fn timed_effect_physical_fields(
        &mut self,
        id: Id,
    ) -> Result<(&mut Seed, &mut crate::actor_fields::Fields), String> {
        self.get(id)
            .ok_or("timed physical actor absent or foreign")?;
        let actor = self.residents.get_mut(&id.serial).unwrap();
        let metadata = actor
            .timed_effect
            .as_ref()
            .ok_or("timed physical ownership absent")?;
        if !metadata.runtime.awaiting_physical()
            || actor
                .construction_fields
                .as_ref()
                .is_none_or(|f| f.control_flags & 0x80 == 0)
        {
            return Err("timed physical service is not pending".into());
        }
        Ok((&mut actor.seed, actor.construction_fields.as_mut().unwrap()))
    }
    /// Supplied object-local time and shared ANM stamp, not a world scheduler.
    /// The actor remains visible to the required real service and nested calls
    /// refuse while its callback is pending. No service is replayed after error.
    pub fn advance_timed_effect(
        &mut self,
        id: Id,
        seconds: f32,
        stamp: &mut crate::animation_epoch::Stamp,
        services: &mut impl Services,
    ) -> Result<Step, String> {
        self.get(id).ok_or("timed actor absent or foreign")?;
        let boundary = {
            let actor = self.residents.get_mut(&id.serial).unwrap();
            let references = &mut self
                .specifications
                .get_mut(&id.serial)
                .ok_or("timed references absent")?
                .0;
            let (runtime, mut body) = borrow(actor, references)?;
            runtime.begin(&mut body, seconds)?
        };
        if boundary == Boundary::Inactive {
            return Ok(Step::Inactive);
        }
        let result = match boundary {
            Boundary::Physical => services.physical(self, id, seconds),
            Boundary::Retirement => {
                let m = self.get(id).unwrap().timed_effect.as_ref().unwrap();
                let source = m.runtime.source.clone();
                let binding = m.runtime.retirement.clone();
                services.retire(self, id, &source, binding.as_ref())
            }
            Boundary::Animation => Ok(()),
            Boundary::Inactive => unreachable!(),
        };
        self.get(id)
            .ok_or("timed actor removed during required service")?;
        let actor = self.residents.get_mut(&id.serial).unwrap();
        let references = &mut self
            .specifications
            .get_mut(&id.serial)
            .ok_or("timed references removed during service")?
            .0;
        let (runtime, mut body) = borrow(actor, references)?;
        let next = runtime.finish(&mut body, seconds, stamp, result);
        // Preserve even a late reference failure's actual acquired stamp.
        let metadata = actor.timed_effect.as_ref().unwrap();
        actor.mecc_animation_stamp = metadata.runtime.epoch;
        // Only an actual callback matrix write refreshes the derived cache.
        // A physical move without that writer leaves the old pose token stale.
        // Completed matrix stores also survive a later ANM/reference failure.
        if metadata.runtime.matrix_written && matches!(metadata.placement, Placement::Position(_)) {
            actor.affine_cache = Some(OwnedAffine {
                value: crate::actor_affine::Cache {
                    flags: metadata.matrix.flags,
                    matrix: metadata.matrix.values,
                },
                pose: affine_pose_words(actor.construction_fields.as_ref().unwrap()),
                external_orientation: true,
            });
        }
        next
    }
    /// Shared bounded ordinary preparation: callback/retirement precedes the
    /// actual parent binding even when the actor becomes marked this pass.
    pub fn prepare_timed_attached(
        &mut self,
        id: Id,
        support: Matrix,
        support_scale: f32,
        seconds: f32,
        stamp: &mut crate::animation_epoch::Stamp,
        services: &mut impl Services,
    ) -> Result<Step, String> {
        self.get(id)
            .ok_or("timed parented actor absent or foreign")?;
        {
            let actor = self.residents.get_mut(&id.serial).unwrap();
            let references = &mut self
                .specifications
                .get_mut(&id.serial)
                .ok_or("timed parented references absent")?
                .0;
            let (runtime, mut body) = borrow(actor, references)?;
            if !runtime.begin_parented(&mut body, support, support_scale, seconds)? {
                return Ok(Step::Inactive);
            }
        }
        let next = self.advance_timed_effect(id, seconds, stamp, services)?;
        let actor = self
            .residents
            .get_mut(&id.serial)
            .ok_or("timed actor removed during preparation")?;
        let references = &mut self
            .specifications
            .get_mut(&id.serial)
            .ok_or("timed preparation references absent")?
            .0;
        let (runtime, mut body) = borrow(actor, references)?;
        runtime.finish_parented(&mut body, support, support_scale, seconds)?;
        let metadata = actor.timed_effect.as_ref().unwrap();
        actor.affine_cache = Some(OwnedAffine {
            value: crate::actor_affine::Cache {
                flags: metadata.matrix.flags,
                matrix: metadata.matrix.values,
            },
            pose: affine_pose_words(actor.construction_fields.as_ref().unwrap()),
            external_orientation: true,
        });
        Ok(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        animation_epoch::Stamp,
        attached_effect_context::{Creation, Pool},
        audio_parameters::Random,
        effect_actor::Template,
        scene_resources::SceneResources,
    };
    struct Provider {
        pool: Pool<Id>,
        random: Random,
        retirements: usize,
        fail: bool,
        positions: Vec<[f32; 3]>,
        child: Option<crate::attached_effect_context::Id>,
        // Explicit retained native-fixture boundaries, not world producers.
        motion: Option<([f32; 3], [f32; 3])>,
        physical_calls: usize,
        physical_fail: bool,
    }
    impl Services for Provider {
        fn physical(&mut self, session: &mut Session, id: Id, dt: f32) -> Result<(), String> {
            let (velocity, previous) = self
                .motion
                .as_mut()
                .ok_or("fixture motion boundary absent")?;
            let (seed, fields) = session.timed_effect_physical_fields(id)?;
            let data = crate::effect_actor::physical_data(seed)?;
            let p: [f32; 15] = std::array::from_fn(|i| {
                f32::from_le_bytes(data[i * 4..i * 4 + 4].try_into().unwrap())
            });
            let motion = crate::mecc_motion::integrate(
                crate::mecc_motion::Motion {
                    position: fields.position,
                    velocity: *velocity,
                    previous_velocity: *previous,
                    force: [p[6], p[7], p[8]],
                    mass: p[2],
                    speed_limit: p[10],
                    force_time_limit: p[11],
                    stored_dt: p[5],
                },
                dt,
                false,
                false,
                crate::arithmetic::Precision::Double53,
            )?;
            let gravity = crate::mecc_gravity::apply(
                crate::mecc_gravity::Input {
                    force: motion.force,
                    velocity: motion.velocity,
                    normal: [0.; 3],
                    gravity_mass: p[1],
                    mass: p[2],
                    friction_limit: 0.,
                    damping_xy: 0.,
                    damping_z: 0.,
                    force_time_limit: motion.force_time_limit,
                    object_flags: fields.object_flags,
                    physics_flags: 0,
                    project_contact: true,
                },
                crate::arithmetic::Precision::Double53,
            )?;
            *velocity = motion.velocity;
            *previous = motion.previous_velocity;
            fields.position = motion.position;
            let data = seed.storage.data_mut().unwrap();
            data[0xb4..0xb8].copy_from_slice(&motion.stored_dt.to_le_bytes());
            data[0xcc..0xd0].copy_from_slice(&gravity.force_time_limit.to_le_bytes());
            for (i, v) in gravity.force.into_iter().enumerate() {
                data[0xb8 + i * 4..0xbc + i * 4].copy_from_slice(&v.to_le_bytes());
            }
            self.physical_calls += 1;
            if self.physical_fail {
                Err("declared failure after actual physical writes".into())
            } else {
                Ok(())
            }
        }
        fn retire(
            &mut self,
            session: &mut Session,
            id: Id,
            source: &Image,
            binding: Option<&crate::effect_scene::RetirementBinding>,
        ) -> Result<(), String> {
            let actor = session
                .get(id)
                .ok_or("canonical actor disappeared at retirement")?;
            let fields = actor.construction_fields.as_ref().unwrap();
            assert_eq!(fields.object_flags & 0x8000, 0);
            let bytes = actor.seed.storage.data().unwrap();
            assert!(
                f32::from_le_bytes(bytes[8..12].try_into().unwrap())
                    >= f32::from_le_bytes(bytes[4..8].try_into().unwrap())
            );
            let m = actor.timed_effect.as_ref().unwrap();
            assert!(source.same_image(&m.runtime.source));
            self.positions.push(fields.position);
            self.retirements += 1;
            if let Some(binding) = binding {
                let (creation, _) = self.pool.prepare_retirement_offset_invariant(
                    Some(binding.program_offset),
                    fields.position,
                    fields.object_flags,
                    m.render_flags,
                    &mut self.random,
                )?;
                let Creation::Prepared(child) = creation else {
                    return Err("child context absent".into());
                };
                self.child = Some(child);
            }
            // The actor is still registered and its pending callback cannot
            // recursively emit again. No provider invocation should occur.
            assert!(
                session
                    .advance_timed_effect(id, 0., &mut Stamp::default(), self)
                    .is_err()
            );
            if self.fail {
                Err("declared failure after real child request".into())
            } else {
                Ok(())
            }
        }
    }
    fn parent(session: &mut Session, resources: &SceneResources) -> Id {
        // Real class/body/state owners. Motion and support pose remain supplied.
        let template = Template::load(resources, 829).unwrap();
        let fields = template.fields.instantiate();
        let id = session
            .register(
                829,
                &template.seed,
                &template.bindings,
                DirectFlags {
                    active: fields.object_flags,
                    personality: fields.configuration[14],
                },
            )
            .unwrap();
        session
            .bind_specification(id, template.specification)
            .unwrap();
        session.bind_state_resources(id, template.states).unwrap();
        session.bind_actor_fields(id, fields).unwrap();
        id
    }
    fn start(resources: &SceneResources, ticket: u64, seed: u32) -> (Session, Id, Provider) {
        let mut session = Session::new(ticket);
        let parent = parent(&mut session, resources);
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
        let mut service = Provider {
            pool: Pool::new(ticket + 200_000, Some(image)),
            random: Random(seed),
            retirements: 0,
            fail: false,
            positions: vec![],
            child: None,
            motion: None,
            physical_calls: 0,
            physical_fail: false,
        };
        let (creation, _) = service
            .pool
            .prepare_cinematic_timed("fx_shotgun_flk", parent, 0, u32::MAX, &mut service.random)
            .unwrap();
        let Creation::Prepared(origin) = creation else {
            panic!("source absent")
        };
        let before = service.random.0;
        let mut foreign = Session::new(ticket + 100_000);
        assert!(
            service
                .pool
                .activate_timed_attached(origin, &mut foreign, resources, &mut service.random)
                .is_err()
        );
        assert!(service.pool.get(origin).is_some());
        assert_eq!(service.random.0, before);
        assert_eq!(foreign.counts(), Counts::default());
        let id = service
            .pool
            .activate_timed_attached(origin, &mut session, resources, &mut service.random)
            .unwrap();
        assert!(service.pool.get(origin).is_none());
        assert_eq!(session.timed_effect_origin(id), Some(origin));
        let after = service.random.0;
        let counts = session.counts();
        assert!(
            service
                .pool
                .activate_timed_attached(origin, &mut session, resources, &mut service.random)
                .is_err()
        );
        assert_eq!(service.random.0, after);
        assert_eq!(session.counts(), counts);
        assert!(session.list(0).contains(&id));
        for list in 1..8 {
            assert!(!session.list(list).contains(&id));
        }
        (session, id, service)
    }
    fn projection(session: &Session, id: Id) -> Vec<u32> {
        let actor = session.get(id).unwrap();
        let f = actor.construction_fields.as_ref().unwrap();
        let m = actor.timed_effect.as_ref().unwrap();
        let mut got = f.configuration.to_vec();
        got.push(f.class as u32);
        got.extend(f.position.map(f32::to_bits));
        got.extend(f.current.map(f32::to_bits));
        got.extend(f.previous.map(f32::to_bits));
        got.push(f.scale.to_bits());
        got.extend(f.angles.map(f32::to_bits));
        got.extend([
            f.object_flags,
            f.response_flags,
            f.control_flags,
            m.render_flags,
        ]);
        got.extend(m.runtime.fade.map(f32::to_bits));
        got.push(m.runtime.transparency.to_bits());
        got.push(
            m.runtime
                .retirement
                .as_ref()
                .map_or(0, |r| r.program_offset),
        );
        got.extend(
            actor
                .seed
                .storage
                .data()
                .unwrap()
                .as_chunks::<4>()
                .0
                .iter()
                .map(|v| u32::from_le_bytes(*v)),
        );
        got.extend(actor.channels.unwrap().iter().flatten());
        got.push(m.matrix.flags);
        got.extend(m.matrix.values.map(f32::to_bits));
        got
    }
    #[test]
    #[ignore = "Requires GIANTS_FLICK_ROOT; silent canonical ownership and optional original-PC controls"]
    fn source_timed_session_preserves_canonical_state_and_quarantines_services() {
        let root = std::path::PathBuf::from(
            std::env::var_os("GIANTS_FLICK_ROOT").expect("GIANTS_FLICK_ROOT"),
        );
        let world = crate::world_source::resolve(&root, "Story4").unwrap();
        let resources = SceneResources::load(&root, &world.catalog, "English").unwrap();
        let (mut session, id, mut service) = start(&resources, 710, 7);
        let payload = session
            .get(id)
            .unwrap()
            .seed
            .storage
            .data()
            .unwrap()
            .as_ptr();
        let mut stamp = Stamp(u32::MAX - 1);
        assert_eq!(
            session
                .prepare_timed_attached(id, Matrix::identity(), 1., 0., &mut stamp, &mut service)
                .unwrap(),
            Step::FirstFrame
        );
        assert_ne!(session.get(id).unwrap().mecc_animation_stamp, 0);
        assert!(!session.current_reference_matrices(id).unwrap().is_empty());
        let before = session
            .get(id)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap()
            .position;
        let mut support = Matrix::identity();
        support.values[3] = 97.;
        let bytes = session.get(id).unwrap().seed.storage.data().unwrap();
        let life = f32::from_le_bytes(bytes[4..8].try_into().unwrap());
        service.fail = true;
        assert!(
            session
                .prepare_timed_attached(id, support, 1., life * 2., &mut stamp, &mut service)
                .is_err()
        );
        assert_eq!(service.positions, [before]);
        assert_eq!(service.retirements, 1);
        assert!(service.pool.get(service.child.unwrap()).is_some());
        assert!(session.timed_effect_failure(id).is_some());
        assert_eq!(
            session
                .get(id)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap()
                .position,
            before
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
        assert!(
            session
                .advance_timed_effect(id, 0., &mut stamp, &mut service)
                .is_err()
        );
        assert_eq!(service.retirements, 1);
        assert!(session.timed_effect_physical_fields(id).is_err());
        let mut foreign = Session::new(711);
        assert!(
            foreign
                .advance_timed_effect(id, 0., &mut stamp, &mut service)
                .is_err()
        );
        // The real retirement context stages a detached actor in the same
        // canonical registry. A late physical failure retains its actual bytes
        // and leaves the previous affine token stale; it cannot replay.
        let origin = service.child.take().unwrap();
        let child = service
            .pool
            .activate_timed_positioned(origin, &mut session, &resources)
            .unwrap();
        assert!(service.pool.get(origin).is_none());
        assert!(
            service
                .pool
                .activate_timed_positioned(origin, &mut session, &resources)
                .is_err()
        );
        assert_eq!(session.get(child).unwrap().constructor_mask, 0);
        assert!(session.actor_current_affine(child).is_ok());
        assert_eq!(
            session
                .advance_timed_effect(child, 0., &mut stamp, &mut service)
                .unwrap(),
            Step::FirstFrame
        );
        service.motion = Some(([0.; 3], [0.; 3]));
        service.physical_fail = true;
        assert!(
            session
                .advance_timed_effect(child, 0.01, &mut stamp, &mut service)
                .is_err()
        );
        assert_eq!(service.physical_calls, 1);
        assert!(session.timed_effect_failure(child).is_some());
        assert_eq!(
            &session.get(child).unwrap().seed.storage.data().unwrap()[0xb4..0xb8],
            &0.01f32.to_le_bytes()
        );
        assert!(session.actor_current_affine(child).is_err());
        assert!(
            session
                .advance_timed_effect(child, 0., &mut stamp, &mut service)
                .is_err()
        );
        assert_eq!(service.physical_calls, 1);
        let removed = session.clear();
        assert_eq!(removed.residents, 3);
        assert_eq!(session.counts().residents, 0);
        assert_eq!(session.specification_count(), 0);
        assert!(
            session
                .advance_timed_effect(id, 0., &mut stamp, &mut service)
                .is_err()
        );
        // Re-entry on the same Session keeps stale actor identities invalid.
        let next_parent = parent(&mut session, &resources);
        let (created, _) = service
            .pool
            .prepare_cinematic_timed(
                "fx_shotgun_flk",
                next_parent,
                0,
                u32::MAX,
                &mut service.random,
            )
            .unwrap();
        let Creation::Prepared(origin) = created else {
            panic!("second source absent")
        };
        let next = service
            .pool
            .activate_timed_attached(origin, &mut session, &resources, &mut service.random)
            .unwrap();
        assert_ne!(id, next);
        assert!(session.get(id).is_none());
        service.fail = false;
        assert_eq!(
            session
                .prepare_timed_attached(next, Matrix::identity(), 1., 0., &mut stamp, &mut service)
                .unwrap(),
            Step::FirstFrame
        );
        let bytes = session.get(next).unwrap().seed.storage.data().unwrap();
        let life = f32::from_le_bytes(bytes[4..8].try_into().unwrap());
        session
            .prepare_timed_attached(
                next,
                Matrix::identity(),
                1.,
                life * 2.,
                &mut stamp,
                &mut service,
            )
            .unwrap();
        let child = service
            .pool
            .activate_timed_positioned(service.child.take().unwrap(), &mut session, &resources)
            .unwrap();
        session
            .advance_timed_effect(child, 0., &mut stamp, &mut service)
            .unwrap();
        service.motion = Some(([0.; 3], [0.; 3]));
        service.physical_fail = false;
        // Deliberate unsupported ANM input after a successful physical service.
        // The completed size/matrix store must survive the later refusal.
        session
            .residents
            .get_mut(&child.serial)
            .unwrap()
            .channels
            .as_mut()
            .unwrap()[0][0] = 255;
        assert!(
            session
                .advance_timed_effect(child, 0.01, &mut stamp, &mut service)
                .is_err()
        );
        assert!(session.timed_effect_failure(child).is_some());
        assert!(session.actor_current_affine(child).is_ok());
        assert!(session.current_reference_matrices(child).is_err());
        assert_eq!(session.clear().residents, 3);
        assert_eq!(session.counts(), Counts::default());
        // Exact previously acquired original-PC rows; they are never committed.
        if let Some(path) = std::env::var_os("GIANTS_FLICK_TIMED_PARENT_CONTROLS") {
            let text = std::fs::read_to_string(path).unwrap();
            let mut current = None;
            let mut run: Option<(Session, Id, Provider)> = None;
            let mut total = 0;
            let mut words = 0;
            let mut stamp = Stamp(0xfffffff0);
            for line in text.lines() {
                let e = line
                    .split('\t')
                    .map(str::parse::<u32>)
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap();
                if current != Some(e[0]) {
                    if let Some((session, _, services)) = run.take() {
                        assert_eq!(services.physical_calls, 3);
                        assert_eq!(services.retirements, 2);
                        assert_eq!(session.counts().residents, 3);
                    }
                    run = Some(start(&resources, 1000 + u64::from(e[0]), e[3]));
                    current = Some(e[0]);
                }
                let (session, id, services) = run.as_mut().unwrap();
                if e[1] == 1 && e[2] == 0 {
                    let origin = services.child.take().unwrap();
                    *id = services
                        .pool
                        .activate_timed_positioned(origin, session, &resources)
                        .unwrap();
                    assert_eq!(session.get(*id).unwrap().constructor_mask, 0);
                    services.motion = Some(([0.; 3], [0.; 3]));
                }
                let seed = services.random.0;
                let step = if e[1] == 0 {
                    let mut support = Matrix::identity();
                    support.values[3] = f32::from_bits(e[7]);
                    support.values[7] = f32::from_bits(e[8]);
                    support.values[11] = f32::from_bits(e[9]);
                    session.prepare_timed_attached(
                        *id,
                        support,
                        1.,
                        f32::from_bits(e[4]),
                        &mut stamp,
                        services,
                    )
                } else {
                    session.advance_timed_effect(*id, f32::from_bits(e[4]), &mut stamp, services)
                }
                .unwrap();
                assert_eq!(services.random.0, seed);
                assert_eq!(
                    step,
                    if e[2] == 0 {
                        Step::FirstFrame
                    } else if (e[1] == 0 && e[2] == 3) || (e[1] == 1 && e[2] == 4) {
                        Step::Marked
                    } else {
                        Step::Updated
                    }
                );
                let got = projection(session, *id);
                assert_eq!(got.len(), e.len() - 33);
                for (i, (a, b)) in got.iter().zip(&e[33..]).enumerate() {
                    assert_eq!(a, b, "case {} step {} word {i}", e[0], e[2]);
                }
                if step == Step::Marked {
                    assert_eq!(services.retirements, if e[1] == 0 { 1 } else { 2 });
                    assert_eq!(
                        session
                            .advance_timed_effect(*id, 0., &mut stamp, services)
                            .unwrap(),
                        Step::Inactive
                    );
                }
                if e[1] == 1 && e[2] == 0 {
                    // Retained original ordinary post-preparation boundary;
                    // this fixture does not acquire its terrain producer.
                    let actor = session.residents.get_mut(&id.serial).unwrap();
                    let fields = actor.construction_fields.as_mut().unwrap();
                    fields.position = std::array::from_fn(|i| f32::from_bits(e[7 + i]));
                    fields.current = std::array::from_fn(|i| f32::from_bits(e[10 + i]));
                    fields.previous = std::array::from_fn(|i| f32::from_bits(e[13 + i]));
                    fields.object_flags ^= e[5];
                    let matrix = Matrix {
                        flags: e[16],
                        values: std::array::from_fn(|i| f32::from_bits(e[17 + i])),
                    };
                    actor.timed_effect.as_mut().unwrap().matrix = matrix;
                    actor.affine_cache = Some(OwnedAffine {
                        value: crate::actor_affine::Cache {
                            flags: matrix.flags,
                            matrix: matrix.values,
                        },
                        pose: affine_pose_words(fields),
                        external_orientation: true,
                    });
                }
                if e[1] == 1 && step == Step::Updated {
                    assert!(session.actor_current_affine(*id).is_ok());
                }
                words += got.len();
                total += 1;
            }
            let (session, _, services) = run.unwrap();
            assert_eq!(services.physical_calls, 3);
            assert_eq!(services.retirements, 2);
            assert_eq!(session.counts().residents, 3);
            assert_eq!(total, 288);
            assert_eq!(words, 50112);
            eprintln!(
                "canonical states={total} selected_words={words}; parent preparation and supplied-postpreparation detached callbacks"
            );
        }
    }
}
