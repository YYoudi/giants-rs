//! Source FLK39 timed attachments over canonical Session bodies.
//! This owner does not certify a whole film: retirement descendants still need
//! the caller's real services, and drawing must consume these exact body fields.
use crate::{
    actor_links::Projection,
    attached_effect_context::{Creation, Pool},
    audio_parameters::Random,
    cinematic_fx::Request,
    procedural_aim::Matrix,
    resident_states::{CinematicScope, CinematicSupport, Id, Session, TimedEffectServices},
    scene_resources::SceneResources,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
pub mod world;

pub struct Bindings {
    requests: BTreeMap<usize, Request>,
    actors: Projection,
    image: Arc<giants_formats::effect_image::Image>,
}
impl Bindings {
    pub fn offsets(&self) -> impl Iterator<Item = usize> + '_ {
        self.requests.keys().copied()
    }
    /// Admit only complete timed-parent programs. A supported parent does not
    /// imply that its required retirement child has an executing owner.
    pub fn load(resources: &SceneResources, bytes: &[u8]) -> Result<Self, String> {
        giants_formats::flick::parse_preview(bytes).map_err(|e| e.to_string())?;
        let metadata =
            giants_formats::flick::discover_dependencies(bytes).map_err(|e| e.to_string())?;
        let image = Arc::new(
            resources
                .classes
                .models
                .effects
                .last()
                .ok_or("timed cinematic effect image absent")?
                .image
                .clone(),
        );
        let actors = crate::actor_links::Factory::new(resources, &image).project(bytes)?;
        let mut requests = BTreeMap::new();
        for request in crate::cinematic_fx::declarations(bytes, &metadata)? {
            crate::cinematic_fx::validate_request_prefix(&metadata, &actors, &request)?;
            if request.slot >= 0 {
                if request.mode != 1 {
                    return Err("timed cinematic owner requires attached mode1".into());
                }
                let program = resources.bound_effect_program(&request.target(resources)?)?;
                let recipe = crate::timed_effect::Recipe::project(&program)?;
                let class = resources
                    .classes
                    .models
                    .first_program(recipe.class)
                    .ok_or("timed cinematic class absent")?
                    .program
                    .as_ref()
                    .map_err(Clone::clone)?;
                recipe.validate_class(class)?;
                crate::effect_actor::Template::load(resources, recipe.class)?;
                // Check the source dependency without treating resolution as execution.
                if image.find(&recipe.child).is_none() {
                    return Err("timed cinematic retirement dependency absent".into());
                }
            }
            requests.insert(request.command_offset, request);
        }
        if requests.is_empty() {
            return Err("source has no timed cinematic requests".into());
        }
        Ok(Self {
            requests,
            actors,
            image,
        })
    }
    pub fn queue(&self, scope: CinematicScope) -> Queue {
        Queue {
            scope,
            contexts: Pool::new(scope.ticket, Some(self.image.clone())),
            objects: BTreeMap::new(),
            supports: BTreeMap::new(),
            dispatched: BTreeSet::new(),
            children: Vec::new(),
            last_child_frame: None,
        }
    }
    fn placement(
        &self,
        offset: usize,
        seconds: f32,
    ) -> Result<crate::cinematic_fx::ProxyPlacement, String> {
        if !seconds.is_finite() || seconds < 0. {
            return Err("timed cinematic support requires a finite current source clock".into());
        }
        let clocks = self
            .actors
            .actors
            .iter()
            .map(|(slot, actor)| {
                (
                    *slot,
                    if actor.director_slot.is_some() {
                        seconds
                    } else {
                        0.
                    },
                )
            })
            .collect();
        let locals = self
            .actors
            .actors
            .keys()
            .map(|slot| (*slot, None))
            .collect();
        self.requests
            .get(&offset)
            .ok_or("timed cinematic request absent")?
            .placement(&self.actors, &clocks, &locals)?
            .ok_or("timed cinematic request is inactive".into())
    }
}

pub struct Queue {
    scope: CinematicScope,
    contexts: Pool<CinematicSupport>,
    objects: BTreeMap<usize, (Id, CinematicSupport)>,
    supports: BTreeMap<usize, usize>,
    dispatched: BTreeSet<usize>,
    children: Vec<Id>,
    last_child_frame: Option<u64>,
}
pub struct WorldStep<'a> {
    pub number: u64,
    pub global_seconds: f32,
    pub areas: &'a [crate::object_time::SlowArea],
    pub progress: crate::actor_finalization::PhysicalProgress,
    pub pc_argument: Option<u32>,
}
/// A view of the canonical actor, not an independent visual simulation.
pub struct Frame {
    pub owner: Id,
    pub world: Matrix,
    pub channels: crate::animation_channels::Channels,
    pub transparency: f32,
    pub fade: [f32; 4],
    pub object_flags: u32,
}
impl Queue {
    /// Advance existing descendants once per actual composing-world frame.
    /// Call this before the parent walk if births enter the next frame's walk.
    /// No clocks, footprints, collisions or environment outputs are defaulted.
    pub fn prepare_children(
        &mut self,
        step: WorldStep<'_>,
        session: &mut Session,
        stamp: &mut crate::animation_epoch::Stamp,
        services: &mut impl crate::resident_states::TimedEffectPreparationServices,
    ) -> Result<(), String> {
        if session.ticket() != self.scope.ticket
            || self
                .last_child_frame
                .is_some_and(|last| step.number <= last)
        {
            return Err("cinematic child frame is foreign or already consumed".into());
        }
        if !step.global_seconds.is_finite() || step.global_seconds < 0. {
            return Err("cinematic child frame requires finite nonnegative global time".into());
        }
        // A later failure retains completed callbacks and cannot replay the walk.
        self.last_child_frame = Some(step.number);
        for &id in &self.children {
            let fields = session
                .get(id)
                .and_then(|a| a.construction_fields.as_ref())
                .ok_or("cinematic child fields absent")?;
            let context = fields.dispatch_context;
            let seconds = crate::object_time::seconds(
                step.global_seconds,
                Some(crate::object_time::Sample {
                    owner: context,
                    position: fields.position,
                }),
                None,
                step.areas,
            ) as f32;
            session.begin_cinematic_timed_frame(id, self.scope)?;
            session.prepare_cinematic_timed_positioned_source(
                id,
                self.scope,
                crate::resident_states::TimedEffectSourcePreparation {
                    seconds,
                    finalization_time: crate::actor_finalization::Time {
                        global_seconds: step.global_seconds,
                        context,
                        areas: step.areas,
                    },
                    progress: step.progress,
                    pc_argument: step.pc_argument,
                },
                stamp,
                services,
            )?;
        }
        Ok(())
    }
    pub fn child_frame(&self, owner: Id, session: &Session) -> Option<Frame> {
        if !self.children.contains(&owner)
            || session.cinematic_timed_scope(owner) != Some(self.scope)
            || session.timed_effect_failure(owner).is_some()
            || session
                .timed_effect_ordinary_header(owner)?
                .statistics
                .is_none()
        {
            return None;
        }
        let actor = session.get(owner)?;
        Some(Frame {
            owner,
            world: session.timed_effect_matrix(owner)?,
            channels: actor.channels?,
            transparency: session.timed_effect_transparency(owner)?,
            fade: session.timed_effect_fade(owner)?,
            object_flags: actor.construction_fields.as_ref()?.object_flags,
        })
    }
    /// Source-created descendants still require ordinary positioned preparation
    /// and drawing. Their existence is not an acknowledgement of those phases.
    pub fn children(&self) -> &[Id] {
        &self.children
    }
    pub fn objects(&self) -> usize {
        self.objects.len()
    }
    pub fn contexts(&self) -> usize {
        self.contexts.active().len()
    }
    pub fn dispatched(&self) -> usize {
        self.dispatched.len()
    }
    /// No random seed, epoch or timestep is invented here. The composing world
    /// owns them; constructor and active passes consume the same supplied stream.
    pub fn dispatch(
        &mut self,
        binding: &Bindings,
        offset: usize,
        resources: &SceneResources,
        random: &mut Random,
        session: &mut Session,
    ) -> Result<(), String> {
        if session.ticket() != self.scope.ticket {
            return Err("timed cinematic queue lost its Session ticket".into());
        }
        if self.dispatched.contains(&offset) {
            return Err("timed cinematic request dispatched twice".into());
        }
        let request = binding
            .requests
            .get(&offset)
            .ok_or("timed cinematic event absent")?;
        if request.slot < 0 {
            self.dispatched.insert(offset);
            return Ok(());
        }
        let support = CinematicSupport {
            scope: self.scope,
            slot: request.slot as usize,
        };
        let name = request
            .effect
            .as_deref()
            .ok_or("timed cinematic name absent")?;
        let (creation, _) =
            self.contexts
                .prepare_cinematic_timed(name, support, 0, u32::MAX, random)?;
        let Creation::Prepared(context) = creation else {
            return Err("timed cinematic program absent from its source image".into());
        };
        let object = match self
            .contexts
            .consume_timed_attached(context, resources, random)
        {
            Ok(object) => object,
            Err(error) => {
                self.contexts.release_prepared(context)?;
                return Err(error);
            }
        };
        let owner = session.adopt_cinematic_timed(object)?;
        self.objects.insert(offset, (owner, support));
        self.supports.insert(support.slot, offset);
        self.dispatched.insert(offset);
        Ok(())
    }
    /// Actual callbacks see the latest source binding for each reused slot.
    /// A failing service is retained by Session; this never converts it to end.
    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        binding: &Bindings,
        seconds: f32,
        delta: f32,
        session: &mut Session,
        stamp: &mut crate::animation_epoch::Stamp,
        services: &mut impl TimedEffectServices,
    ) -> Result<(), String> {
        if session.ticket() != self.scope.ticket {
            return Err("timed cinematic update has a foreign Session".into());
        }
        if !seconds.is_finite() || seconds < 0. || !delta.is_finite() || delta < 0. {
            return Err("timed cinematic update requires finite nonnegative clocks".into());
        }
        for &(owner, support) in self.objects.values() {
            let source = *self
                .supports
                .get(&support.slot)
                .ok_or("timed source support absent")?;
            let placement = binding.placement(source, seconds)?;
            session.prepare_cinematic_timed(
                owner,
                support,
                placement.matrix,
                delta,
                stamp,
                services,
            )?;
        }
        Ok(())
    }
    pub fn frame(&self, offset: usize, session: &Session) -> Option<Frame> {
        let &(owner, support) = self.objects.get(&offset)?;
        if session.cinematic_timed_support(owner) != Some(support) {
            return None;
        }
        let actor = session.get(owner)?;
        let fields = actor.construction_fields.as_ref()?;
        Some(Frame {
            owner,
            world: session.timed_effect_matrix(owner)?,
            channels: actor.channels?,
            transparency: session.timed_effect_transparency(owner)?,
            fade: session.timed_effect_fade(owner)?,
            object_flags: fields.object_flags,
        })
    }
    /// Execute attached callbacks and consume each retirement into its actual
    /// source-configured positional Session body. Descendants are not ticked in
    /// this parent walk; the composing world must prepare them separately.
    #[allow(clippy::too_many_arguments)]
    pub fn update_attached(
        &mut self,
        binding: &Bindings,
        seconds: f32,
        delta: f32,
        resources: &SceneResources,
        random: &mut Random,
        session: &mut Session,
        stamp: &mut crate::animation_epoch::Stamp,
    ) -> Result<(), String> {
        if session.ticket() != self.scope.ticket {
            return Err("timed cinematic update has a foreign Session".into());
        }
        if !seconds.is_finite() || seconds < 0. || !delta.is_finite() || delta < 0. {
            return Err("timed cinematic update requires finite nonnegative clocks".into());
        }
        let mut services = Retirement {
            scope: self.scope,
            image: &binding.image,
            contexts: &mut self.contexts,
            resources,
            random,
            children: &mut self.children,
        };
        for &(owner, support) in self.objects.values() {
            let source = *self
                .supports
                .get(&support.slot)
                .ok_or("timed source support absent")?;
            let placement = binding.placement(source, seconds)?;
            session.prepare_cinematic_timed(
                owner,
                support,
                placement.matrix,
                delta,
                stamp,
                &mut services,
            )?;
        }
        Ok(())
    }
    /// Source-marked scope cancellation never emits an expired object's child.
    pub fn release(&mut self, session: &mut Session) -> Result<usize, String> {
        if session.ticket() != self.scope.ticket {
            return Err("timed cinematic release has a foreign Session".into());
        }
        session.cancel_cinematic_scope(self.scope)?;
        let retired = self.objects.len() + self.children.len();
        self.objects.clear();
        self.supports.clear();
        self.children.clear();
        for context in self.contexts.active().to_vec() {
            self.contexts.release_prepared(context)?;
        }
        Ok(retired)
    }
}

struct Retirement<'a> {
    scope: CinematicScope,
    image: &'a giants_formats::effect_image::Image,
    contexts: &'a mut Pool<CinematicSupport>,
    resources: &'a SceneResources,
    random: &'a mut Random,
    children: &'a mut Vec<Id>,
}
impl TimedEffectServices for Retirement<'_> {
    fn physical(&mut self, _: &mut Session, _: Id, _: f32) -> Result<(), String> {
        Err("attached cinematic owner cannot supply detached physical motion".into())
    }
    fn gravity(&mut self, _: &mut Session, _: Id) -> Result<(), String> {
        Err("attached cinematic owner cannot supply detached gravity".into())
    }
    fn retire(
        &mut self,
        session: &mut Session,
        actor: Id,
        source: &giants_formats::effect_image::Image,
        binding: Option<&crate::effect_scene::RetirementBinding>,
    ) -> Result<(), String> {
        if session.cinematic_timed_scope(actor) != Some(self.scope)
            || !source.same_image(self.image)
        {
            return Err("timed retirement has a foreign source owner".into());
        }
        let Some(binding) = binding else {
            return Ok(());
        };
        let fields = session
            .get(actor)
            .and_then(|a| a.construction_fields.as_ref())
            .ok_or("timed retirement parent fields absent")?;
        let render_flags = session
            .timed_effect_render_flags(actor)
            .ok_or("timed retirement parent render fields absent")?;
        let (creation, _) = self.contexts.prepare_retirement_offset_invariant(
            Some(binding.program_offset),
            fields.position,
            fields.object_flags,
            render_flags,
            self.random,
        )?;
        let Creation::Prepared(context) = creation else {
            return Err("timed retirement source child absent".into());
        };
        let owned = match self
            .contexts
            .consume_timed_positioned(context, self.resources)
        {
            Ok(owned) => owned,
            Err(error) => {
                self.contexts.release_prepared(context)?;
                return Err(error);
            }
        };
        self.children
            .push(session.adopt_cinematic_timed_positioned(owned, self.scope)?);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// These branches must not be invoked by the selected attached, pre-expiry
    /// fixture. Rejecting them does not acknowledge a retirement child or motion.
    struct Unprovided;
    impl TimedEffectServices for Unprovided {
        fn physical(&mut self, _: &mut Session, _: Id, _: f32) -> Result<(), String> {
            Err("unprovided physical boundary".into())
        }
        fn gravity(&mut self, _: &mut Session, _: Id) -> Result<(), String> {
            Err("unprovided gravity boundary".into())
        }
        fn retire(
            &mut self,
            _: &mut Session,
            _: Id,
            _: &giants_formats::effect_image::Image,
            _: Option<&crate::effect_scene::RetirementBinding>,
        ) -> Result<(), String> {
            Err("unprovided retirement child".into())
        }
    }
    #[test]
    #[ignore = "requires a user-supplied compatible installation"]
    fn installed_wife_timed_parents_have_canonical_bodies_and_scoped_cleanup() {
        let root = std::path::PathBuf::from(std::env::var_os("GIANTS_MISSION_OWNED_ROOT").unwrap());
        let menu = crate::mission_selection::MissionMenu::parse(
            &std::fs::read(root.join("Bin/w_Story2.bin")).unwrap(),
            0,
            true,
        )
        .unwrap();
        let packet =
            crate::mission_loading::prepare(&root, menu.request("Story2", 1).unwrap()).unwrap();
        let plan = packet.instance_plan(&root, "English").unwrap();
        let resources = plan.resources();
        for (director, name) in [(2, "story2_savewife_4"), (3, "story2_savewife_5")] {
            let bytes = std::fs::read(crate::flick::sequence_path(&root, name).unwrap()).unwrap();
            let bindings = Bindings::load(resources, &bytes).unwrap();
            let scope = CinematicScope {
                ticket: 72,
                director,
                generation: 3,
            };
            let mut queue = bindings.queue(scope);
            let mut session = Session::new(scope.ticket);
            let mut foreign = Session::new(scope.ticket + 1);
            let mut random = Random(13); // Supplied control stream, not a live seed claim.
            let offsets = bindings.offsets().take(2).collect::<Vec<_>>();
            assert_eq!(offsets.len(), 2);
            let before = random.0;
            assert!(
                queue
                    .dispatch(&bindings, offsets[0], resources, &mut random, &mut foreign)
                    .is_err()
            );
            assert_eq!(random.0, before);
            for offset in &offsets {
                queue
                    .dispatch(&bindings, *offset, resources, &mut random, &mut session)
                    .unwrap();
            }
            assert_eq!(queue.objects(), 2);
            assert_eq!(queue.contexts(), 0);
            let before = random.0;
            assert!(
                queue
                    .dispatch(&bindings, offsets[0], resources, &mut random, &mut session)
                    .is_err()
            );
            assert_eq!(random.0, before);
            let mut stamp = crate::animation_epoch::Stamp::default();
            queue
                .update(&bindings, 0., 0., &mut session, &mut stamp, &mut Unprovided)
                .unwrap();
            queue
                .update(
                    &bindings,
                    0.01,
                    0.01,
                    &mut session,
                    &mut stamp,
                    &mut Unprovided,
                )
                .unwrap();
            for offset in &offsets {
                let frame = queue.frame(*offset, &session).unwrap();
                assert_eq!(session.get(frame.owner).unwrap().class, 829);
                assert_eq!(
                    frame.channels,
                    session.get(frame.owner).unwrap().channels.unwrap()
                );
                assert!(frame.world.values.iter().all(|v| v.is_finite()));
                assert!(frame.world.values[3] != 0. || frame.world.values[7] != 0.);
                assert_eq!(frame.object_flags & 0xc000, 0);
                assert!(frame.transparency.is_finite());
            }
            assert!(queue.release(&mut foreign).is_err());
            assert_eq!(queue.objects(), 2);
            assert_eq!(queue.release(&mut session).unwrap(), 2);
            for offset in offsets {
                assert!(queue.frame(offset, &session).is_none());
            }
            assert_eq!(queue.contexts(), 0);
            assert_eq!(queue.objects(), 0);

            // A new scoped owner reaches its real lifetime boundary. Refusing
            // the required child must quarantine it, not acknowledge retirement.
            let mut queue = bindings.queue(CinematicScope {
                generation: 4,
                ..scope
            });
            let offset = bindings.offsets().next().unwrap();
            queue
                .dispatch(&bindings, offset, resources, &mut random, &mut session)
                .unwrap();
            queue
                .update(&bindings, 0., 0., &mut session, &mut stamp, &mut Unprovided)
                .unwrap();
            let owner = queue.frame(offset, &session).unwrap().owner;
            let error = queue
                .update(
                    &bindings,
                    0.02,
                    1000.,
                    &mut session,
                    &mut stamp,
                    &mut Unprovided,
                )
                .unwrap_err();
            assert!(error.contains("unprovided retirement child"), "{error}");
            assert!(session.get(owner).is_some());
            assert!(session.timed_effect_failure(owner).is_some());
            // A real scope cancellation can discard the quarantined actor;
            // it is distinct from acknowledging its failed lifetime callback.
            assert_eq!(queue.release(&mut session).unwrap(), 1);
            assert!(session.get(owner).is_none());

            let mut queue = bindings.queue(CinematicScope {
                generation: 5,
                ..scope
            });
            queue
                .dispatch(&bindings, offset, resources, &mut random, &mut session)
                .unwrap();
            queue
                .update_attached(
                    &bindings,
                    0.,
                    0.,
                    resources,
                    &mut random,
                    &mut session,
                    &mut stamp,
                )
                .unwrap();
            queue
                .update_attached(
                    &bindings,
                    0.02,
                    1000.,
                    resources,
                    &mut random,
                    &mut session,
                    &mut stamp,
                )
                .unwrap();
            assert_eq!(queue.children().len(), 1);
            let child = queue.children()[0];
            assert_eq!(session.get(child).unwrap().class, 828);
            assert_eq!(session.cinematic_timed_scope(child).unwrap().generation, 5);
            assert_ne!(
                session
                    .get(child)
                    .unwrap()
                    .construction_fields
                    .as_ref()
                    .unwrap()
                    .control_flags
                    & 0x20,
                0
            );
            assert_eq!(queue.contexts(), 0);
            let parent = queue.frame(offset, &session).unwrap().owner;
            assert!(session.timed_effect_failure(parent).is_none());
            queue
                .update_attached(
                    &bindings,
                    0.03,
                    0.01,
                    resources,
                    &mut random,
                    &mut session,
                    &mut stamp,
                )
                .unwrap();
            assert_eq!(queue.children().len(), 1);
            assert_eq!(queue.release(&mut session).unwrap(), 2);
            assert!(session.get(child).is_none());
            assert!(session.get(parent).is_none());
        }
        let bytes = std::fs::read(crate::flick::sequence_path(&root, "story2_savewife_6").unwrap())
            .unwrap();
        assert!(Bindings::load(resources, &bytes).is_err());
    }

    #[test]
    #[ignore = "requires a user-supplied compatible installation"]
    fn installed_wife_children_use_actual_world_and_retained_platform_candidates() {
        let root = std::path::PathBuf::from(std::env::var_os("GIANTS_MISSION_OWNED_ROOT").unwrap());
        let menu = crate::mission_selection::MissionMenu::parse(
            &std::fs::read(root.join("Bin/w_Story2.bin")).unwrap(),
            0,
            true,
        )
        .unwrap();
        let packet =
            crate::mission_loading::prepare(&root, menu.request("Story2", 1).unwrap()).unwrap();
        let plan = packet.instance_plan(&root, "English").unwrap();
        let resources = plan.resources();
        let world = crate::load_world(&root, "Story2").unwrap();
        let solids = crate::declared_mesh_scene::prepare(&plan, &world.terrain);
        let platforms =
            crate::declared_mesh_scene::prepare_platform_candidates(&plan, &world.terrain);
        for need in &platforms.needs {
            eprintln!("platform_candidate_need {:?}: {}", need.key, need.reason);
        }
        assert!(
            platforms.needs.is_empty(),
            "unbounded source platforms cannot be discarded"
        );
        assert_eq!(platforms.objects.len() + platforms.platform_non_raster, 48);
        let unresolved = platforms
            .objects
            .iter()
            .map(|object| crate::platform_queries::UnpreparedPlatform {
                id: object.fields.id,
                object_flags: object.fields.object_flags,
                collision_flags: object.fields.collision_flags,
                bounds_xy: [
                    [object.fields.bounds.min[0], object.fields.bounds.min[1]],
                    [object.fields.bounds.max[0], object.fields.bounds.max[1]],
                ],
            })
            .collect::<Vec<_>>();
        let objects = solids
            .records
            .iter()
            .zip(&solids.objects)
            .map(|(record, object)| crate::actor_environment::Collider {
                class: record.class as u32,
                state: record.instance.state,
                volume: None,
                mesh: crate::mecc_mesh_dispatch::MeshObject {
                    fields: object.fields,
                    triangles: object.triangles.clone(),
                },
            })
            .collect::<Vec<_>>();
        assert!(!objects.is_empty());
        for (director, name) in [(2, "story2_savewife_4"), (3, "story2_savewife_5")] {
            let bytes = std::fs::read(crate::flick::sequence_path(&root, name).unwrap()).unwrap();
            let bindings = Bindings::load(resources, &bytes).unwrap();
            let scope = CinematicScope {
                ticket: 73,
                director,
                generation: 1,
            };
            let mut queue = bindings.queue(scope);
            let mut session = Session::new(scope.ticket);
            let mut random = Random(13); // Explicit dependency fixture input.
            let mut stamp = crate::animation_epoch::Stamp::default();
            let offset = bindings.offsets().next().unwrap();
            queue
                .dispatch(&bindings, offset, resources, &mut random, &mut session)
                .unwrap();
            queue
                .update_attached(
                    &bindings,
                    0.,
                    0.,
                    resources,
                    &mut random,
                    &mut session,
                    &mut stamp,
                )
                .unwrap();
            queue
                .update_attached(
                    &bindings,
                    0.02,
                    1000.,
                    resources,
                    &mut random,
                    &mut session,
                    &mut stamp,
                )
                .unwrap();
            let child = queue.children()[0];
            assert!(queue.child_frame(child, &session).is_none());
            let mut queries = 0;
            for number in 1..=80 {
                let seconds = if number == 1 { 0. } else { 1. / 60. };
                let mut services =
                    world::World::new(&world.terrain, &objects, &[], &unresolved, seconds, &[])
                        .unwrap();
                queue
                    .prepare_children(
                        WorldStep {
                            number,
                            global_seconds: seconds,
                            areas: &[],
                            progress: crate::actor_finalization::PhysicalProgress::PrototypePs2Zero,
                            pc_argument: None,
                        },
                        &mut session,
                        &mut stamp,
                        &mut services,
                    )
                    .unwrap();
                queries += services.calls.environment;
                if number == 2 {
                    assert!(queue.prepare_children(WorldStep {
                        number,global_seconds:seconds,areas:&[],
                        progress:crate::actor_finalization::PhysicalProgress::PrototypePs2Zero,
                        pc_argument:None,
                    },&mut session,&mut stamp,&mut services).is_err());
                }
                assert!(queue.child_frame(child, &session).is_some());
            }
            assert!(queries > 0);
            assert_ne!(
                queue.child_frame(child, &session).unwrap().object_flags & 0x8000,
                0
            );
            queue.release(&mut session).unwrap();
            assert!(session.get(child).is_none());
        }
    }
}
