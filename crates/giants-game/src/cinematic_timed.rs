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
pub mod ground_factory;
pub mod world;

pub struct Bindings {
    requests: BTreeMap<usize, Request>,
    classes: BTreeMap<usize, (i32, Option<i32>)>,
    kinds: BTreeMap<usize, Kind>,
    actors: Projection,
    image: Arc<giants_formats::effect_image::Image>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Attached,
    Ground,
    Surface,
    Curved,
}
impl Bindings {
    /// Every visible root and descendant needs its own prepared render consumer.
    pub fn geometry_roles(&self) -> impl Iterator<Item = (usize, i32)> + '_ {
        self.classes.iter().flat_map(|(&offset, &(root, child))| {
            std::iter::once((offset, root)).chain(child.map(|class| (offset, class)))
        })
    }
    /// Source16 bounds this contact child to one birth before parent deletion.
    pub fn contact_child_class(&self, offset: usize) -> Option<i32> {
        if self.kinds.get(&offset) != Some(&Kind::Ground) {
            return None;
        }
        self.classes.get(&offset)?.1
    }
    pub fn role(&self, offset: usize, class: i32) -> Option<bool> {
        let &(parent, child) = self.classes.get(&offset)?;
        if class == parent {
            Some(true)
        } else if Some(class) == child {
            Some(false)
        } else {
            None
        }
    }
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
        let mut classes = BTreeMap::new();
        let mut kinds = BTreeMap::new();
        crate::actor_fields::Fields::load_class_zero(resources)?;
        for request in crate::cinematic_fx::declarations(bytes, &metadata)? {
            crate::cinematic_fx::validate_request_prefix(&metadata, &actors, &request)?;
            if request.slot >= 0 {
                if request.mode != 1 {
                    return Err("timed cinematic owner requires attached mode1".into());
                }
                let program = resources.bound_effect_program(&request.target(resources)?)?;
                let ops = program.records.iter().map(|r| r.opcode).collect::<Vec<_>>();
                let (kind, root, child) = if ops == [0x24, 6, 0x1c, 4, 0x0d, 7, 2] {
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
                    let (_, entry) = image
                        .find(&recipe.child)
                        .ok_or("timed retirement dependency absent")?;
                    let p = image.program(entry.offset).map_err(|e| e.to_string())?;
                    if p.records.iter().map(|r| r.opcode).collect::<Vec<_>>()
                        != [3, 6, 8, 4, 0x0e, 7, 2]
                    {
                        return Err("timed child requires its positioned callback profile".into());
                    }
                    (
                        Kind::Attached,
                        recipe.class,
                        Some(crate::released_effect::Recipe::project(&p)?.class),
                    )
                } else {
                    let root = i32::from_le_bytes(
                        program.records[0]
                            .bytes
                            .get(1..5)
                            .ok_or("source3 class absent")?
                            .try_into()
                            .unwrap(),
                    );
                    let name = request
                        .effect
                        .as_deref()
                        .ok_or("source3 effect name absent")?;
                    let (kind, child) = if ops == [3, 6, 5, 9, 0x12, 0x16, 0x0e, 0x5a, 7, 2] {
                        crate::effect_scene::Plan::project(&image, name)?;
                        let refs = program.references().map_err(|e| e.to_string())?;
                        let name = refs
                            .iter()
                            .find_map(|r| {
                                if let giants_formats::effect_program::ReferenceKey::Effect(name) =
                                    &r.key
                                {
                                    Some(name)
                                } else {
                                    None
                                }
                            })
                            .ok_or("ground child relationship absent")?;
                        let (_, entry) = image.find(name).ok_or("ground child absent")?;
                        let p = image.program(entry.offset).map_err(|e| e.to_string())?;
                        if p.records.iter().map(|r| r.opcode).collect::<Vec<_>>()
                            != [3, 6, 4, 0x17, 0x1c, 0x79, 7, 2]
                        {
                            return Err("ground child requires its real source79 profile".into());
                        }
                        let class =
                            i32::from_le_bytes(p.records[0].bytes[1..5].try_into().unwrap());
                        (Kind::Ground, Some(class))
                    } else if ops == [3, 6, 4, 0x17, 0x1c, 0x79, 7, 2] {
                        crate::effect_scene::Plan::project(&image, name)?;
                        (Kind::Surface, None)
                    } else if ops == [3, 6, 4, 0x36, 7, 2] || ops == [3, 6, 4, 0x17, 0x36, 7, 2] {
                        crate::released_effect::Recipe::project(&program)?;
                        (Kind::Curved, None)
                    } else {
                        return Err("cinematic source program has no canonical family owner".into());
                    };
                    (kind, root, child)
                };
                crate::effect_actor::Template::load(resources, root)?;
                if let Some(child) = child {
                    crate::effect_actor::Template::load(resources, child)?;
                }
                classes.insert(request.command_offset, (root, child));
                kinds.insert(request.command_offset, kind);
            }
            requests.insert(request.command_offset, request);
        }
        if requests.is_empty() {
            return Err("source has no timed cinematic requests".into());
        }
        Ok(Self {
            requests,
            classes,
            kinds,
            actors,
            image,
        })
    }
    pub fn queue(&self, scope: CinematicScope) -> Queue {
        Queue {
            scope,
            contexts: Pool::new(scope.ticket, Some(self.image.clone())),
            objects: BTreeMap::new(),
            positioned: BTreeMap::new(),
            supports: BTreeMap::new(),
            dispatched: BTreeSet::new(),
            children: Vec::new(),
            child_sources: BTreeMap::new(),
            last_child_frame: None,
            last_root_frame: None,
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
    positioned: BTreeMap<usize, Id>,
    supports: BTreeMap<usize, usize>,
    dispatched: BTreeSet<usize>,
    children: Vec<Id>,
    child_sources: BTreeMap<Id, usize>,
    last_child_frame: Option<u64>,
    last_root_frame: Option<u64>,
}
#[derive(Clone, Copy)]
pub struct WorldStep<'a> {
    pub number: u64,
    pub global_seconds: f32,
    pub areas: &'a [crate::object_time::SlowArea],
    pub progress: crate::actor_finalization::PhysicalProgress,
    pub pc_argument: Option<u32>,
}
/// A view of the canonical actor, not an independent visual simulation.
#[derive(Clone, Copy)]
pub struct Frame {
    pub owner: Id,
    pub world: Matrix,
    pub channels: crate::animation_channels::Channels,
    pub transparency: f32,
    pub fade: [f32; 4],
    pub object_flags: u32,
    pub scale: f32,
    pub epoch: u32,
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
        if !self.children.contains(&owner) {
            return None;
        }
        self.positioned_frame(owner, session)
    }
    fn positioned_frame(&self, owner: Id, session: &Session) -> Option<Frame> {
        if session.cinematic_timed_scope(owner) != Some(self.scope)
            || session.timed_effect_failure(owner).is_some()
            || !session.timed_effect_ordinary_header(owner)?.prepared
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
            scale: actor.construction_fields.as_ref()?.scale,
            epoch: actor.mecc_animation_stamp,
        })
    }
    /// Source-created descendants still require ordinary positioned preparation
    #[allow(clippy::too_many_arguments)]
    pub fn prepare_positioned_roots(
        &mut self,
        step: WorldStep<'_>,
        session: &mut Session,
        stamp: &mut crate::animation_epoch::Stamp,
        resources: &SceneResources,
        random: &mut Random,
        terrain: &giants_formats::gti::Terrain,
        prepared: &world::Prepared,
    ) -> Result<usize, String> {
        if session.ticket() != self.scope.ticket
            || self.last_child_frame != Some(step.number)
            || self.last_root_frame.is_some_and(|last| step.number <= last)
        {
            return Err("source3 root walk requires its unique actual world frame".into());
        }
        self.last_root_frame = Some(step.number);
        let before = self.children.len();
        for (&offset, &id) in &self.positioned {
            let fields = session
                .get(id)
                .and_then(|a| a.construction_fields.as_ref())
                .ok_or("source3 root fields absent")?;
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
            let previous = self.children.len();
            let result = {
                let mut factory = ground_factory::Factory {
                    scope: self.scope,
                    contexts: &mut self.contexts,
                    resources,
                    random,
                    newborns: &mut self.children,
                };
                let mut services = world::World::new(
                    terrain,
                    &prepared.objects,
                    &[],
                    &prepared.unresolved,
                    step.global_seconds,
                    step.areas,
                )?
                .with_ground_effects(&mut factory);
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
                    &mut services,
                )
            };
            for &child in &self.children[previous..] {
                self.child_sources.insert(child, offset);
            }
            result?;
        }
        Ok(self.children.len() - before)
    }
    /// Source-created descendants still require ordinary positioned preparation
    /// and drawing. Their existence is not an acknowledgement of those phases.
    pub fn children(&self) -> &[Id] {
        &self.children
    }
    pub fn child_source(&self, child: Id) -> Option<usize> {
        self.child_sources.get(&child).copied()
    }
    pub fn objects(&self) -> usize {
        self.objects.len() + self.positioned.len()
    }
    pub fn contexts(&self) -> usize {
        self.contexts.active().len()
    }
    pub fn dispatched(&self) -> usize {
        self.dispatched.len()
    }
    pub fn dispatched_offset(&self, offset: usize) -> bool {
        self.dispatched.contains(&offset)
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
        if binding
            .kinds
            .get(&offset)
            .is_some_and(|kind| *kind != Kind::Attached)
        {
            return Err("source3 dispatch needs its actual source clock and terrain".into());
        }
        self.dispatch_inner(binding, offset, None, resources, random, session)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn dispatch_source(
        &mut self,
        binding: &Bindings,
        offset: usize,
        source_seconds: f32,
        terrain: &giants_formats::gti::Terrain,
        resources: &SceneResources,
        random: &mut Random,
        session: &mut Session,
    ) -> Result<(), String> {
        self.dispatch_inner(
            binding,
            offset,
            Some((source_seconds, terrain)),
            resources,
            random,
            session,
        )
    }
    fn dispatch_inner(
        &mut self,
        binding: &Bindings,
        offset: usize,
        frame: Option<(f32, &giants_formats::gti::Terrain)>,
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
        let source = frame
            .map(|(seconds, _)| {
                let placement = binding.placement(offset, seconds)?;
                crate::cinematic_support::Support::prepare(resources, support, placement.matrix)
            })
            .transpose()?;
        let kind = *binding
            .kinds
            .get(&offset)
            .ok_or("cinematic family absent")?;
        if kind != Kind::Attached {
            let (seconds, terrain) = frame.ok_or("source3 requires actual source frame")?;
            let _ = seconds;
            let source = source.as_ref().ok_or("source3 support absent")?;
            let (Creation::Prepared(context), _) = self.contexts.prepare_cinematic_source3(
                name,
                support,
                source.object_flags(),
                random,
            )?
            else {
                return Err("source3 context absent".into());
            };
            let result = (|| {
                let context_ref = self
                    .contexts
                    .get(context)
                    .ok_or("source3 prepared context lost")?;
                let body = match kind {
                    Kind::Ground => crate::timed_effect_object::Timed::ground_cinematic(
                        resources,
                        context,
                        context_ref,
                        source,
                        random,
                    ),
                    Kind::Curved => crate::timed_effect_object::Timed::curved_cinematic(
                        resources,
                        context,
                        context_ref,
                        source,
                    ),
                    Kind::Surface => {
                        let query = crate::terrain_queries::StaticTerrainQuery::new(
                            terrain,
                            1. / terrain.spacing,
                            crate::arithmetic::Precision::Double53,
                        )?;
                        crate::timed_effect_object::Timed::surface_cinematic(
                            resources,
                            context,
                            context_ref,
                            source,
                            random,
                            |xy| {
                                crate::physical_ground::Ground::from_sample(
                                    query.sample(xy)?,
                                    terrain.width,
                                )
                            },
                        )
                    }
                    Kind::Attached => unreachable!(),
                }?;
                session.adopt_cinematic_timed_positioned(body, self.scope)
            })();
            self.contexts.release_prepared(context)?;
            let id = result?;
            self.positioned.insert(offset, id);
            self.supports.insert(support.slot, offset);
            self.dispatched.insert(offset);
            return Ok(());
        }
        let (creation, _) = self.contexts.prepare_cinematic_timed(
            name,
            support,
            source.as_ref().map_or(0, |s| s.object_flags()),
            u32::MAX,
            random,
        )?;
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
        if let Some(&id) = self.positioned.get(&offset) {
            return self.positioned_frame(id, session);
        }
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
            scale: fields.scale,
            epoch: actor.mecc_animation_stamp,
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
            child_sources: &mut self.child_sources,
            source_offset: 0,
        };
        for (&offset, &(owner, support)) in &self.objects {
            services.source_offset = offset;
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
        let retired = self.objects.len() + self.positioned.len() + self.children.len();
        self.objects.clear();
        self.positioned.clear();
        self.supports.clear();
        self.children.clear();
        self.child_sources.clear();
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
    child_sources: &'a mut BTreeMap<Id, usize>,
    source_offset: usize,
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
        let child = session.adopt_cinematic_timed_positioned(owned, self.scope)?;
        self.children.push(child);
        self.child_sources.insert(child, self.source_offset);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires GIANTS_MISSION_OWNED_ROOT; source timeline/real world with supplied clock/RNG, not GPU or mission acceptance"]
    fn installed_wife_mixed_source_events_have_canonical_roots_and_children() {
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
        let scene = packet.world_asset(&root).unwrap();
        let world = world::Prepared::load(&plan, &scene.terrain).unwrap();
        let bytes = std::fs::read(crate::flick::sequence_path(&root, "story2_savewife_6").unwrap())
            .unwrap();
        let binding = Bindings::load(resources, &bytes).unwrap();
        let animation = crate::animation::load_director(&root, "story2_savewife_6").unwrap();
        let [start, end, tpf, fps] = animation.timeline;
        let mut timeline = crate::flick::Timeline::with_start(
            giants_formats::flick::parse_preview(&bytes).unwrap(),
            tpf,
            start,
        )
        .unwrap();
        let scope = CinematicScope {
            ticket: 84,
            director: 4,
            generation: 1,
        };
        let mut queue = binding.queue(scope);
        let mut session = Session::new(scope.ticket);
        let mut random = Random(43); // Supplied composing fixture stream.
        let mut stamp = crate::animation_epoch::Stamp::default();
        let frames =
            (((end - start) as f64 / (f64::from(tpf) * f64::from(fps))) * 60.).ceil() as u64 + 1;
        let mut seen = BTreeSet::new();
        let mut born = 0;
        for number in 1..=frames {
            let seconds = ((number - 1) as f64 / 60.) as f32;
            let tick = (start as f64 + f64::from(seconds) * f64::from(tpf) * f64::from(fps))
                .min(f64::from(end)) as i32;
            let delta = if number == 1 { 0. } else { 1. / 60. };
            let step = WorldStep {
                number,
                global_seconds: delta,
                areas: &[],
                progress: crate::actor_finalization::PhysicalProgress::PrototypePs2Zero,
                pc_argument: None,
            };
            let mut services = world::World::new(
                &scene.terrain,
                &world.objects,
                &[],
                &world.unresolved,
                delta,
                &[],
            )
            .unwrap();
            queue
                .prepare_children(step, &mut session, &mut stamp, &mut services)
                .unwrap();
            let events = timeline.step(tick).unwrap();
            for event in events
                .events
                .iter()
                .filter(|e| e.kind == crate::flick::EventKind::Effect)
            {
                queue
                    .dispatch_source(
                        &binding,
                        event.offset,
                        seconds,
                        &scene.terrain,
                        resources,
                        &mut random,
                        &mut session,
                    )
                    .unwrap();
            }
            born += queue
                .prepare_positioned_roots(
                    step,
                    &mut session,
                    &mut stamp,
                    resources,
                    &mut random,
                    &scene.terrain,
                    &world,
                )
                .unwrap();
            queue
                .update_attached(
                    &binding,
                    seconds,
                    delta,
                    resources,
                    &mut random,
                    &mut session,
                    &mut stamp,
                )
                .unwrap();
            for offset in binding.offsets() {
                if let Some(frame) = queue.frame(offset, &session) {
                    seen.insert(session.get(frame.owner).unwrap().class);
                    assert!(frame.world.values.iter().all(|v| v.is_finite()));
                }
            }
            for &child in queue.children() {
                if let Some(frame) = queue.child_frame(child, &session) {
                    seen.insert(session.get(frame.owner).unwrap().class);
                }
            }
            let before = random.0;
            assert!(
                queue
                    .prepare_positioned_roots(
                        step,
                        &mut session,
                        &mut stamp,
                        resources,
                        &mut random,
                        &scene.terrain,
                        &world
                    )
                    .is_err()
            );
            assert_eq!(random.0, before);
        }
        for class in [829, 828, 914, 915, 916, 938] {
            assert!(
                seen.contains(&class),
                "canonical class {class} absent: {seen:?}"
            );
        }
        assert!(born > 0);
        assert_eq!(queue.dispatched(), binding.offsets().count());
        assert_eq!(queue.contexts(), 0);
        let owners = queue.objects() + queue.children().len();
        assert_eq!(queue.release(&mut session).unwrap(), owners);
        assert_eq!(queue.objects(), 0);
        assert_eq!(queue.contexts(), 0);
        println!(
            "wife_mixed_source_events frames={frames} classes={seen:?} ground_births={born} gpu_verified=false"
        );
    }
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
            assert_eq!(queue.child_source(child), Some(offset));
            assert_eq!(bindings.role(offset, 829), Some(true));
            assert_eq!(bindings.role(offset, 828), Some(false));
            assert_eq!(bindings.role(offset, 916), None);
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
        let prepared_world = world::Prepared::load(&plan, &world.terrain).unwrap();
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
        assert_eq!(prepared_world.objects.len(), objects.len());
        assert_eq!(prepared_world.unresolved.len(), unresolved.len());
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
                let mut services = world::World::new(
                    &world.terrain,
                    &prepared_world.objects,
                    &[],
                    &prepared_world.unresolved,
                    seconds,
                    &[],
                )
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
