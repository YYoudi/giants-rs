//! Acquired timed callbacks, shared by staged objects and canonical Session actors.
//! Native clocks, physical/world services and stock admission remain explicit.
use crate::{
    actor_references::Owned,
    animation_epoch::Stamp,
    effect_actor::{Configured, Initial, Template},
    effect_scene::RetirementBinding,
    effect_visual::Step as VisualStep,
    procedural_aim::Matrix,
};
use giants_formats::effect_image::Image;
use std::sync::Arc;

/// Required native boundaries; no default success provider is installed.
pub trait Services<K> {
    /// Motion only. The affine store and required gravity phase follow it.
    fn physical(&mut self, object: &mut Initial<K>, seconds: f32) -> Result<(), String>;
    fn gravity(&mut self, object: &mut Initial<K>) -> Result<(), String>;
    fn particle_contact(&mut self, _: &mut Initial<K>) -> Result<(), String> {
        Err("particle requires its actual post-gravity ground-contact force owner".into())
    }
    fn ground_contact(
        &mut self,
        _: &mut Initial<K>,
        _: &Image,
        _: &crate::effect_ground_emission::Source,
    ) -> Result<(), String> {
        Err("ground effect requires its real contact and child services".into())
    }
    fn retire(
        &mut self,
        object: &Initial<K>,
        source: &Image,
        binding: Option<&RetirementBinding>,
    ) -> Result<(), String>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    FirstFrame,
    Updated,
    Marked,
    Inactive,
}

/// Exclusive mutable borrow of the actual owner. Session channels may reside
/// outside the construction header snapshot; no payload or reference copy is made.
pub(crate) struct Body<'a, K> {
    pub fields: &'a mut crate::actor_fields::Fields,
    pub seed: &'a mut crate::state_storage::Seed,
    pub matrix: &'a mut Matrix,
    pub channels: Option<&'a mut crate::animation_channels::Channels>,
    pub references: &'a mut Owned,
    pub velocity: Option<&'a mut [f32; 3]>,
    pub placement: crate::effect_actor::Placement<K>,
    pub render_flags: u32,
    pub immediate_pending: bool,
}
impl<K> Body<'_, K> {
    fn channels(&self) -> &crate::animation_channels::Channels {
        self.channels.as_deref().unwrap_or(&self.fields.channels)
    }
    fn channels_mut(&mut self) -> &mut crate::animation_channels::Channels {
        self.channels
            .as_deref_mut()
            .unwrap_or(&mut self.fields.channels)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Boundary {
    Physical,
    Retirement,
    Animation,
    Inactive,
}
/// Callback-only metadata; live fields, payload, channels and references belong
/// to Body's actual owner. A pending boundary cannot start another callback.
pub(crate) struct Runtime {
    pub class: i32,
    /// Classify before effect-wrapper/VM stores; later flags do not relist it.
    pub constructor_mask: u32,
    pub matrix_written: bool,
    pub bindings: Vec<crate::model_registry::StateBinding>,
    pub states: Arc<crate::state_resources::Prepared>,
    pub fade: [f32; 4],
    pub transparency: f32,
    visual: crate::effect_visual::Visual,
    pub retirement: Option<RetirementBinding>,
    pub ground: Option<crate::effect_ground_emission::Source>,
    physical_callback: bool,
    pub source: Arc<Image>,
    durations: Vec<Option<f32>>,
    immutable_words: [u32; 3],
    size_flag: u32,
    side_speed: Option<u32>,
    pub(crate) render: crate::effect_render_parameters::Parameters,
    pub(crate) hidden: bool,
    pub(crate) particle_contact: bool,
    immediate_kind: bool,
    forward_speed: Option<u32>,

    pub epoch: u32,
    pub failure: Option<String>,
    pending: Option<(VisualStep, bool)>,
}
impl std::fmt::Debug for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TimedCallback")
            .field("class", &self.class)
            .field("epoch", &self.epoch)
            .field("failure", &self.failure)
            .field("pending", &self.pending)
            .finish_non_exhaustive()
    }
}
/// Acquired bounded resource retirement; no native pointer reuse is represented.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Destruction {
    pub origin: crate::attached_effect_context::Id,
    pub payload_bytes: usize,
    pub registrations: usize,
}
pub struct Timed<K> {
    pub(crate) immediate: Option<crate::attached_effect_context::Immediate<K>>,
    pub(crate) velocity: Option<[f32; 3]>,
    pub(crate) initial: Initial<K>,
    pub(crate) runtime: Runtime,
    pub(crate) references: Owned,
}
impl<K> Timed<K> {
    /// Move the actual body into another typed owner domain without cloning its
    /// payload, references, callback state or random stream.
    pub(crate) fn map_parent<L>(self, parent: impl FnOnce(K) -> L) -> Timed<L> {
        let mut parent = Some(parent);
        let placement = match self.initial.placement {
            crate::effect_actor::Placement::Attached {
                recipient,
                reference,
                tag,
                opaque,
            } => crate::effect_actor::Placement::Attached {
                recipient: parent.take().unwrap()(recipient),
                reference,
                tag,
                opaque,
            },
            crate::effect_actor::Placement::Position(p) => {
                crate::effect_actor::Placement::Position(p)
            }
        };
        Timed {
            immediate: self
                .immediate
                .map(|binding| crate::attached_effect_context::Immediate {
                    parent: parent.take().unwrap()(binding.parent),
                    reference: binding.reference,
                    own_reference: binding.own_reference,
                }),
            velocity: self.velocity,
            initial: Initial {
                origin: self.initial.origin,
                placement,
                seed: self.initial.seed,
                fields: self.initial.fields,
                render_flags: self.initial.render_flags,
                matrix: self.initial.matrix,
            },
            runtime: self.runtime,
            references: self.references,
        }
    }
}
impl<K: Copy> Timed<K> {
    /// One immediate source particle with an owned temporary parent binding.
    /// First-frame placement and real later contact remain required consumers.
    pub fn immediate_particle_object(
        resources: &crate::scene_resources::SceneResources,
        origin: crate::attached_effect_context::Id,
        context: &crate::attached_effect_context::Context<K>,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<Self, String> {
        source_scope(resources, context)?;
        let record = context
            .program
            .as_ref()
            .map_err(Clone::clone)?
            .records
            .first()
            .ok_or("immediate particle source empty")?;
        let class = i32::from_le_bytes(
            record
                .bytes
                .get(1..5)
                .ok_or("immediate particle class absent")?
                .try_into()
                .unwrap(),
        );
        let template = Template::load(resources, class)?;
        let configured = template.immediate_particle_object(origin, context, random)?;
        Self::new(template, configured)
    }
    /// Construct only this acquired root prefix. Source children/waits remain
    /// mandatory work for its composing context; no whole-program acknowledgement.
    pub fn mixed_root_object(
        resources: &crate::scene_resources::SceneResources,
        origin: crate::attached_effect_context::Id,
        context: &crate::attached_effect_context::Context<K>,
        offset: usize,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<Self, String> {
        source_scope(resources, context)?;
        let record = context
            .program
            .as_ref()
            .map_err(Clone::clone)?
            .records
            .iter()
            .find(|r| r.offset == offset && r.opcode == 3)
            .ok_or("mixed root constructor absent")?;
        let class = i32::from_le_bytes(
            record
                .bytes
                .get(1..5)
                .ok_or("mixed root class absent")?
                .try_into()
                .unwrap(),
        );
        let template = Template::load(resources, class)?;
        let configured = template.mixed_root_object(origin, context, offset, random)?;
        Self::new(template, configured)
    }
    /// Construct this one verified mixed-program object without consuming the
    /// context or acknowledging any other object, wait, renderer or world tick.
    /// Prototype PS2 `fx_move` / `fx_setupUVScrolling` / `fx_setupVerticleFade`
    /// remain unknown attributions; this binds only the acquired PC frame.
    pub fn scrolling_vertical_object(
        resources: &crate::scene_resources::SceneResources,
        origin: crate::attached_effect_context::Id,
        context: &crate::attached_effect_context::Context<K>,
        offset: usize,
    ) -> Result<Self, String> {
        source_scope(resources, context)?;
        let program = context.program.as_ref().map_err(Clone::clone)?;
        let record = program
            .records
            .iter()
            .find(|r| r.offset == offset && r.opcode == 3)
            .ok_or("scroll/fade source constructor absent")?;
        let class = i32::from_le_bytes(
            record
                .bytes
                .get(1..5)
                .ok_or("scroll/fade class operand absent")?
                .try_into()
                .unwrap(),
        );
        let template = Template::load(resources, class)?;
        let configured = template.scrolling_vertical_object(origin, context, offset)?;
        Self::new(template, configured)
    }
    /// Stage ownership while the caller retains the context transaction. The
    /// pool's consume method additionally guards and retires its live identity.
    pub fn attached(
        resources: &crate::scene_resources::SceneResources,
        origin: crate::attached_effect_context::Id,
        context: &crate::attached_effect_context::Context<K>,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<Self, String> {
        source_scope(resources, context)?;
        let program = context.program.as_ref().map_err(Clone::clone)?;
        let recipe = crate::timed_effect::Recipe::project(program)?;
        let template = Template::load(resources, recipe.class)?;
        let mut staged = *random;
        let configured = template.timed_attached(origin, context, &mut staged)?;
        let owned = Self::new(template, configured)?;
        *random = staged;
        Ok(owned)
    }

    pub fn positioned(
        resources: &crate::scene_resources::SceneResources,
        origin: crate::attached_effect_context::Id,
        context: &crate::attached_effect_context::Context<K>,
    ) -> Result<Self, String> {
        source_scope(resources, context)?;
        let program = context.program.as_ref().map_err(Clone::clone)?;
        let recipe = crate::released_effect::Recipe::project(program)?;
        let template = Template::load(resources, recipe.class)?;
        let configured = template.gravity_positioned(origin, context)?;
        Self::new(template, configured)
    }
    pub fn ground_positioned(
        resources: &crate::scene_resources::SceneResources,
        origin: crate::attached_effect_context::Id,
        context: &crate::attached_effect_context::Context<K>,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<Self, String> {
        source_scope(resources, context)?;
        let program = context.program.as_ref().map_err(Clone::clone)?;
        let class = program
            .records
            .first()
            .and_then(|r| r.bytes.get(1..5))
            .ok_or("ground source class absent")?;
        let template = Template::load(resources, i32::from_le_bytes(class.try_into().unwrap()))?;
        let mut staged = *random;
        let configured = template.ground_positioned(origin, context, &mut staged)?;
        let owned = Self::new(template, configured)?;
        *random = staged;
        Ok(owned)
    }
    pub fn surface_positioned(
        resources: &crate::scene_resources::SceneResources,
        origin: crate::attached_effect_context::Id,
        context: &crate::attached_effect_context::Context<K>,
        random: &mut crate::audio_parameters::Random,
        query: impl FnMut([f32; 2]) -> Result<crate::physical_ground::Ground, &'static str>,
    ) -> Result<Self, String> {
        source_scope(resources, context)?;
        let program = context.program.as_ref().map_err(Clone::clone)?;
        let class = program
            .records
            .first()
            .and_then(|r| r.bytes.get(1..5))
            .ok_or("surface class absent")?;
        let template = Template::load(resources, i32::from_le_bytes(class.try_into().unwrap()))?;
        let mut staged = *random;
        let configured = template.surface_positioned(origin, context, &mut staged, query);
        *random = staged;
        Self::new(template, configured?)
    }
    /// Acquired smoke descendant with explicit source-pool active RNG draws.
    pub fn smoke_positioned(
        resources: &crate::scene_resources::SceneResources,
        origin: crate::attached_effect_context::Id,
        context: &crate::attached_effect_context::Context<K>,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<Self, String> {
        source_scope(resources, context)?;
        let recipe = crate::mecc_smoke_particle::Recipe::project(
            context.program.as_ref().map_err(Clone::clone)?,
        )?;
        let template = Template::load(resources, recipe.class)?;
        let mut staged = *random;
        let configured = template.smoke_positioned(origin, context, &mut staged)?;
        let owned = Self::new(template, configured)?;
        *random = staged;
        Ok(owned)
    }
}

impl Timed<crate::resident_states::CinematicSupport> {
    pub fn curved_cinematic(
        resources: &crate::scene_resources::SceneResources,
        origin: crate::attached_effect_context::Id,
        context: &crate::attached_effect_context::Context<crate::resident_states::CinematicSupport>,
        support: &crate::cinematic_support::Support,
    ) -> Result<Self, String> {
        source_scope(resources, context)?;
        let program = context.program.as_ref().map_err(Clone::clone)?;
        let class = program
            .records
            .first()
            .and_then(|r| r.bytes.get(1..5))
            .ok_or("cinematic curved class absent")?;
        let template = Template::load(resources, i32::from_le_bytes(class.try_into().unwrap()))?;
        let configured = template.curved_cinematic(origin, context, support)?;
        Self::new(template, configured)
    }
    pub fn ground_cinematic(
        resources: &crate::scene_resources::SceneResources,
        origin: crate::attached_effect_context::Id,
        context: &crate::attached_effect_context::Context<crate::resident_states::CinematicSupport>,
        support: &crate::cinematic_support::Support,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<Self, String> {
        source_scope(resources, context)?;
        let program = context.program.as_ref().map_err(Clone::clone)?;
        let class = program
            .records
            .first()
            .and_then(|r| r.bytes.get(1..5))
            .ok_or("cinematic ground class absent")?;
        let template = Template::load(resources, i32::from_le_bytes(class.try_into().unwrap()))?;
        let mut staged = *random;
        let configured = template.ground_cinematic(origin, context, support, &mut staged)?;
        let body = Self::new(template, configured)?;
        *random = staged;
        Ok(body)
    }
    pub fn surface_cinematic(
        resources: &crate::scene_resources::SceneResources,
        origin: crate::attached_effect_context::Id,
        context: &crate::attached_effect_context::Context<crate::resident_states::CinematicSupport>,
        support: &crate::cinematic_support::Support,
        random: &mut crate::audio_parameters::Random,
        query: impl FnMut([f32; 2]) -> Result<crate::physical_ground::Ground, &'static str>,
    ) -> Result<Self, String> {
        source_scope(resources, context)?;
        let program = context.program.as_ref().map_err(Clone::clone)?;
        let class = program
            .records
            .first()
            .and_then(|r| r.bytes.get(1..5))
            .ok_or("cinematic surface class absent")?;
        let template = Template::load(resources, i32::from_le_bytes(class.try_into().unwrap()))?;
        let mut staged = *random;
        let configured = template.surface_cinematic(origin, context, support, &mut staged, query);
        *random = staged;
        Self::new(template, configured?)
    }
}

fn source_scope<K>(
    resources: &crate::scene_resources::SceneResources,
    context: &crate::attached_effect_context::Context<K>,
) -> Result<(), String> {
    let source = context.source_image();
    if !resources
        .classes
        .models
        .effects
        .iter()
        .any(|e| e.image.same_image(&source))
    {
        return Err("timed source image differs from constructor resources".into());
    }
    Ok(())
}

impl<K: Copy> Timed<K> {
    fn new(template: Template, configured: Configured<K>) -> Result<Self, String> {
        let initial = &configured.initial;
        if initial.fields.class != template.fields.class
            || !template
                .specification
                .metadata
                .events
                .iter()
                .all(|t| t.events.is_empty())
            || initial
                .fields
                .channels
                .iter()
                .any(|c| c[4] != 0 || c[5] != 0)
        {
            return Err("timed ownership outside acquired class/ANM callback profile".into());
        }
        let data = initial.seed.storage.data().ok_or("timed payload absent")?;
        if data.len() != 268 || initial.seed.storage.state() != 78 {
            return Err("timed ownership requires the acquired inline payload".into());
        }
        let immutable_words = [4, 0x10, 0x14].map(|at| word(data, at));
        let size_flag = match configured.visual.size_curve() {
            crate::effect_visual::SizeCurve::Linear => 8,
            crate::effect_visual::SizeCurve::EaseOut => 0x100000,
            _ => return Err("timed callback size curve outside acquired profile".into()),
        };
        if configured.immediate.is_some()
            && !matches!(
                initial.placement,
                crate::effect_actor::Placement::Position(_)
            )
        {
            return Err("immediate particle cannot also have a persistent attachment".into());
        }
        let side_speed = (initial.render_flags & 0x8000 != 0).then(|| word(data, 0xf4));
        let forward_speed = configured.forward_only.then(|| word(data, 0x1c));
        if configured.forward_only != configured.immediate.is_some()
            || (configured.particle_contact
                && (configured.ground.is_some() || !configured.forward_only))
        {
            return Err("timed launch/contact ownership outside acquired profile".into());
        }
        let physical_callback = initial.fields.control_flags & 0x80 != 0;
        let references = Owned::new(template.specification.clone())?;
        let mut durations = vec![None; 256];
        for (&slot, d) in &template.specification.animations {
            durations[usize::from(slot)] = Some(d.template.duration);
        }
        let constructor_mask = match configured.initial.placement {
            crate::effect_actor::Placement::Attached { .. } => 0,
            crate::effect_actor::Placement::Position(_) => crate::object_lists::membership(
                template.fields.class,
                u32::from(template.seed.storage.state()),
                crate::object_lists::DirectFlags {
                    active: template.fields.object_flags,
                    personality: template.fields.configuration[14],
                },
            ),
        };
        let mut owned = Self {
            immediate: configured.immediate,
            initial: configured.initial,
            velocity: (side_speed.is_some() || forward_speed.is_some()).then_some([0.; 3]),
            references,
            runtime: Runtime {
                class: template.fields.class,
                constructor_mask,
                matrix_written: false,
                bindings: template.bindings,
                states: template.states,
                fade: configured.fade,
                transparency: configured.transparency,
                visual: configured.visual,
                retirement: configured.retirement,
                ground: configured.ground,
                physical_callback,
                source: configured.source,
                durations,
                immutable_words,
                size_flag,
                side_speed,
                render: configured.render,
                hidden: configured.hidden,
                immediate_kind: configured.immediate.is_some(),
                particle_contact: configured.particle_contact,
                forward_speed,
                epoch: 0,
                failure: None,
                pending: None,
            },
        };
        let (runtime, body) = owned.borrow();
        runtime.preflight(&body)?;
        Ok(owned)
    }
    fn borrow(&mut self) -> (&mut Runtime, Body<'_, K>) {
        (
            &mut self.runtime,
            Body {
                velocity: self.velocity.as_mut(),
                fields: &mut self.initial.fields,
                seed: &mut self.initial.seed,
                matrix: &mut self.initial.matrix,
                channels: None,
                references: &mut self.references,
                placement: self.initial.placement,
                render_flags: self.initial.render_flags,
                immediate_pending: self.immediate.is_some(),
            },
        )
    }
    pub fn object(&self) -> &Initial<K> {
        &self.initial
    }
    /// Acquired ordinary writers only; does not admit the actor to a world.
    pub fn object_mut(&mut self) -> &mut Initial<K> {
        &mut self.initial
    }
    pub fn transparency(&self) -> f32 {
        self.runtime.transparency
    }
    pub fn fade(&self) -> [f32; 4] {
        self.runtime.fade
    }
    pub fn failure(&self) -> Option<&str> {
        self.runtime.failure.as_deref()
    }
    pub fn velocity(&self) -> Option<[f32; 3]> {
        self.velocity
    }
    pub fn retirement(&self) -> Option<&RetirementBinding> {
        self.runtime.retirement.as_ref()
    }
    pub fn animation_epoch(&self) -> u32 {
        self.runtime.epoch
    }
    pub fn state_resources(&self) -> &crate::state_resources::Prepared {
        &self.runtime.states
    }
    pub fn current_references(&self) -> Result<&[Matrix], String> {
        self.references
            .current_matrices(&self.initial.fields.channels)
    }
    pub fn advance(
        &mut self,
        seconds: f32,
        stamp: &mut Stamp,
        services: &mut impl Services<K>,
    ) -> Result<Step, String> {
        let boundary = {
            let (runtime, mut body) = self.borrow();
            runtime.begin(&mut body, seconds)?
        };
        if boundary == Boundary::Inactive {
            return Ok(Step::Inactive);
        }
        let mut result = match boundary {
            Boundary::Physical => services.physical(&mut self.initial, seconds),
            Boundary::Retirement => services.retire(
                &self.initial,
                &self.runtime.source,
                self.runtime.retirement.as_ref(),
            ),
            Boundary::Animation => Ok(()),
            Boundary::Inactive => unreachable!(),
        };
        if boundary == Boundary::Physical {
            result = {
                let (runtime, mut body) = self.borrow();
                runtime.finish_motion(&mut body, result)
            };
            if result.is_ok() {
                result = services.gravity(&mut self.initial);
            }
            if result.is_ok() && self.runtime.particle_contact {
                result = services.particle_contact(&mut self.initial);
            }
            if result.is_ok()
                && let Some(ground) = &self.runtime.ground
            {
                result = services.ground_contact(&mut self.initial, &self.runtime.source, ground);
            }
        }
        let (runtime, mut body) = self.borrow();
        runtime.finish(&mut body, seconds, stamp, result)
    }
    pub fn prepare_attached(
        &mut self,
        support: Matrix,
        support_scale: f32,
        seconds: f32,
        stamp: &mut Stamp,
        services: &mut impl Services<K>,
    ) -> Result<Step, String> {
        {
            let (runtime, mut body) = self.borrow();
            if !runtime.begin_parented(&mut body, support, support_scale, seconds)? {
                return Ok(Step::Inactive);
            }
        }
        let next = self.advance(seconds, stamp, services)?;
        let (runtime, mut body) = self.borrow();
        runtime.finish_parented(&mut body, support, support_scale, seconds)?;
        Ok(next)
    }
}
impl Runtime {
    pub(crate) fn expired_smoke(&self) -> bool {
        self.smoke_particle() && self.visual.expired() && self.idle() && self.failure.is_none()
    }

    pub(crate) fn smoke_particle(&self) -> bool {
        self.size_flag == 0x100000 && self.side_speed.is_some() && self.retirement.is_none()
    }

    pub(crate) fn awaiting_physical(&self) -> bool {
        matches!(self.pending, Some((VisualStep::VisibleState, _))) && !self.matrix_written
    }
    pub(crate) fn awaiting_gravity(&self) -> bool {
        matches!(self.pending, Some((VisualStep::VisibleState, _))) && self.matrix_written
    }
    pub(crate) fn idle(&self) -> bool {
        self.pending.is_none()
    }
    pub(crate) fn begin_parented<K>(
        &self,
        b: &mut Body<'_, K>,
        support: Matrix,
        support_scale: f32,
        seconds: f32,
    ) -> Result<bool, String> {
        self.preflight(b)?;
        if !matches!(b.placement, crate::effect_actor::Placement::Attached { .. }) {
            return Err("parented preparation requires actual attachment binding".into());
        }
        if b.fields.configuration[14] & 0x10 != 0
            || b.fields.position.map(f32::to_bits) != b.fields.current.map(f32::to_bits)
        {
            return Err("parented preparation needs acquired dispatcher/history producers".into());
        }
        if !seconds.is_finite()
            || seconds < 0.
            || !support_scale.is_finite()
            || support_scale <= 0.
            || support.flags & 0x20 != 0
            || !support.values.iter().all(|v| v.is_finite())
        {
            return Err("parented preparation outside finite pose/time profile".into());
        }
        if b.fields.object_flags & 0xc000 != 0 {
            return Ok(false);
        }
        b.fields.object_flags &= !0x40;
        Ok(true)
    }
    pub(crate) fn finish_parented<K>(
        &mut self,
        b: &mut Body<'_, K>,
        support: Matrix,
        support_scale: f32,
        seconds: f32,
    ) -> Result<(), String> {
        let crate::effect_actor::Placement::Attached { reference, .. } = &b.placement else {
            return Err("parented preparation requires actual attachment binding".into());
        };
        let reference = *reference as usize;
        let result = (|| -> Result<(), String> {
            // This profile has a real parent and physical storage, no relative
            // displacement. Reuse common finalization; any unexpected required
            // footprint/environment service refuses rather than succeeding.
            let own_channels = *b.channels();
            let f = &mut *b.fields;
            let mut fields = crate::collider_copy::Fields::zeroed();
            fields.class = f.class as u32;
            fields.current = f.position;
            fields.previous = f.current;
            fields.scale = f.scale;
            fields.angles = f.angles;
            fields.object_flags = f.object_flags;
            fields.response_flags = f.response_flags;
            fields.collision_flags = f.configuration[14];
            fields.ground_offset = f32::from_bits(f.configuration[17]);
            fields.center_offset = f32::from_bits(f.configuration[22]);
            fields.height = f32::from_bits(f.configuration[23]);
            fields.physical = true;
            let finalized = crate::actor_finalization::finalize(
                crate::actor_finalization::State {
                    fields,
                    previous_physical_velocity: [0.; 3],
                    friction: 0.,
                    parent_present: true,
                    footprint: None,
                    affine: None,
                    physical_argument_word: None,
                },
                crate::actor_finalization::Time {
                    global_seconds: seconds,
                    context: f.dispatch_context,
                    areas: &[],
                },
                &mut RequiredFinalization,
            )
            .map_err(|e| e.to_string())?;
            f.position = finalized.fields.current;
            f.object_flags = finalized.fields.object_flags;
            let own = b
                .references
                .current_matrices(&own_channels)?
                .get(reference)
                .copied()
                .ok_or("parented effect reference absent")?;
            *b.matrix = crate::effect_actor::bind_parented(f, support, support_scale, own)?;
            Ok(())
        })();
        if let Err(ref error) = result {
            self.failure = Some(error.clone());
        }
        result?;
        Ok(())
    }
    pub(crate) fn preflight<K>(&self, b: &Body<'_, K>) -> Result<(), String> {
        if self.failure.is_some() || self.pending.is_some() {
            return Err("timed callback is pending or quarantined after a late failure".into());
        }
        let data = b
            .seed
            .storage
            .data()
            .ok_or("timed callback payload absent")?;
        let allowed = self.render.control_mask()
            | 0x800000
            | 0x80
            | 0x20
            | self.size_flag
            | 0x04
            | 0x02
            | 0x01
            | if self.side_speed.is_some() || self.forward_speed.is_some() {
                0x40
            } else {
                0
            }
            | if self.particle_contact { 0x100 } else { 0 }
            | if self.ground.is_some() {
                0x0200_0200
            } else {
                0
            };
        let physical = self.physical_callback;
        if b.seed.storage.profile() != crate::state_storage::Profile::Debug
            || b.seed.storage.state() != 78
            || data.len() != 268
            || word(data, 0) != 1
            || b.fields.class != self.class
            || b.channels().iter().any(|c| c[4] != 0 || c[5] != 0)
            || b.fields.control_flags & !allowed != 0
            || b.fields.control_flags & 2 == 0
            || (b.fields.control_flags & 0x80 != 0) != physical
            || (physical && !matches!(b.placement, crate::effect_actor::Placement::Position(_)))
            || (b.fields.control_flags & (8 | 0x100000))
                != if self.immutable_words[1] != self.immutable_words[2] {
                    self.size_flag
                } else {
                    0
                }
            || (b.fields.control_flags & 1 != 0) != self.retirement.is_some()
            || !matches!(
                b.render_flags & !(0x8000 | if self.hidden { 0x10000 } else { 0 }),
                0 | 0x1000
            )
            || (b.render_flags & 0x10000 != 0) != self.hidden
            || (b.render_flags & 0x8000 != 0) != self.side_speed.is_some()
            || self.side_speed.is_some_and(|side| word(data, 0xf4) != side)
            || self
                .forward_speed
                .is_some_and(|speed| word(data, 0x1c) != speed)
            || (b.fields.control_flags & 0x100 != 0) != self.particle_contact
            || (self.immediate_kind
                && (b.fields.response_flags & 0x1000 != 0) != b.immediate_pending)
            || [4, 0x10, 0x14].map(|at| word(data, at)) != self.immutable_words
            || word(data, 8) != self.visual.age.to_bits()
            || data[0xf8..0x10c].iter().any(|&b| b != 0)
        {
            return Err("timed callback outside acquired clock/payload/service profile".into());
        }
        if let Some(ground) = &self.ground {
            ground.decision(b.fields, b.seed)?;
        }
        self.render.validate(b.fields, b.seed)?;
        Ok(())
    }

    /// Publish the acquired prefix before returning the required service. The
    /// actor remains registered while a Session provider executes that service.
    pub(crate) fn begin<K>(
        &mut self,
        b: &mut Body<'_, K>,
        seconds: f32,
    ) -> Result<Boundary, String> {
        if !seconds.is_finite() || seconds < 0. {
            return Err("timed callback requires finite nonnegative object time".into());
        }
        self.preflight(b)?;
        if b.fields.object_flags & 0xc000 != 0 {
            return Ok(Boundary::Inactive);
        }
        if b.immediate_pending && b.fields.control_flags & 0x20 == 0 {
            return Err(
                "immediate particle requires its real one-shot placement before another callback"
                    .into(),
            );
        }
        let mut visual = self.visual.clone();
        let next = visual.advance(seconds)?;
        let first = b.fields.control_flags & 0x20 != 0;
        if first != (next == VisualStep::FirstFrame) {
            return Err("timed constructor gate and lifetime phase disagree".into());
        }
        self.matrix_written = false;
        self.pending = Some((next, first));
        crate::effect_actor::callback_entry(b.fields, b.render_flags);
        b.seed.storage.data_mut().unwrap()[8..12].copy_from_slice(&visual.age.to_le_bytes());
        self.visual = visual;
        if next == VisualStep::Expired {
            return Ok(Boundary::Retirement);
        }
        if next == VisualStep::VisibleState {
            self.transparency = self.visual.transparency;
            b.fields.scale = self.visual.size;
            if self.fade[0] <= self.visual.age {
                b.fields.control_flags &= !0x800000;
            }
            if b.fields.control_flags & 0x40 != 0 {
                // The actual velocity/angle stores precede the integration
                // service. Keep them when that later service refuses.
                let initialize = (|| -> Result<(), String> {
                    let data = b
                        .seed
                        .storage
                        .data()
                        .ok_or("effect launch payload absent")?;
                    let forward = f32::from_bits(word(data, 0x1c));
                    let side = self.side_speed.map(f32::from_bits);
                    let velocity =
                        crate::effect_emitter::initial_velocity(*b.matrix, forward, side)?;
                    *b.velocity
                        .as_deref_mut()
                        .ok_or("effect launch owner absent")? = velocity;
                    b.fields.control_flags &= !0x40;
                    b.fields.angles = crate::effect_orientation::from_matrix(*b.matrix)?;
                    Ok(())
                })();
                if let Err(error) = initialize {
                    self.pending = None;
                    self.failure = Some(error.clone());
                    return Err(error);
                }
            }
            if b.fields.control_flags & 0x80 != 0 {
                return Ok(Boundary::Physical);
            }
        }
        Ok(Boundary::Animation)
    }
    /// Native physical translation/affine publication precedes gravity. Keep
    /// the callback pending through both services, including after late errors.
    pub(crate) fn finish_motion<K>(
        &mut self,
        b: &mut Body<'_, K>,
        service: Result<(), String>,
    ) -> Result<(), String> {
        let result: Result<(), String> = (|| {
            service?;
            if !self.awaiting_physical() || b.fields.control_flags & 0x80 == 0 {
                return Err("timed motion phase is not pending".into());
            }
            let (flags, cache) = crate::actor_affine::update(
                crate::uncached_pose::Pose {
                    position: b.fields.position,
                    angles: b.fields.angles,
                    scale: b.fields.scale,
                },
                b.fields.object_flags,
                Some(crate::actor_affine::Cache {
                    flags: b.matrix.flags,
                    matrix: b.matrix.values,
                }),
                crate::arithmetic::Precision::Double53,
            )
            .map_err(str::to_owned)?;
            b.fields.object_flags = flags;
            *b.matrix = Matrix {
                flags: cache.flags,
                values: cache.matrix,
            };
            self.matrix_written = true;
            Ok(())
        })();
        if let Err(ref error) = result {
            self.failure = Some(error.clone());
        }
        result
    }
    pub(crate) fn finish<K>(
        &mut self,
        b: &mut Body<'_, K>,
        seconds: f32,
        stamp: &mut Stamp,
        service: Result<(), String>,
    ) -> Result<Step, String> {
        // The motion-affine store is already published. Track only a new
        // post-gravity size/orientation write for the caller's next refresh.
        self.matrix_written = false;
        let result = (|| -> Result<Step, String> {
            let (next, first) = self
                .pending
                .take()
                .ok_or("timed callback has no pending service boundary")?;
            service?;
            if next == VisualStep::Expired {
                b.fields.object_flags |= 0x8000;
                return Ok(Step::Marked);
            }
            if next == VisualStep::VisibleState && b.fields.control_flags & self.size_flag != 0 {
                let mut matrix = crate::effect_orientation::matrix(
                    b.fields.position,
                    b.fields.angles,
                    b.fields.scale,
                )?;
                matrix.values[12..].copy_from_slice(&b.matrix.values[12..]);
                *b.matrix = matrix;
                self.matrix_written = true;
            }
            if next == VisualStep::VisibleState {
                self.render.advance(b.fields, b.seed, seconds)?;
            }
            let tick = crate::animation_channels::advance(
                b.channels_mut(),
                &self.durations,
                [],
                seconds,
                crate::arithmetic::Precision::Double53,
            )?;
            if tick.missing != 0 {
                return Err("timed ANM descriptor absent".into());
            }
            if tick.processed != 0 {
                self.epoch = stamp.advance();
                let channels = *b.channels();
                b.references.update_current(&channels, self.epoch)?;
            }
            Ok(if first {
                Step::FirstFrame
            } else {
                Step::Updated
            })
        })();
        if let Err(ref error) = result {
            self.failure = Some(error.clone());
        }
        result
    }
}
fn word(data: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(data[at..at + 4].try_into().unwrap())
}

struct RequiredFinalization;
impl crate::actor_finalization::Services for RequiredFinalization {
    fn release_footprint(&mut self, _: Option<u32>) -> Result<(), String> {
        Err("parented effect footprint producer unavailable".into())
    }
    fn environment(&mut self, _: &mut crate::actor_finalization::State) -> Result<(), String> {
        Err("parented effect environment producer unavailable".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Unavailable {
        physical: usize,
        retirement: usize,
    }
    impl Services<u32> for Unavailable {
        fn gravity(&mut self, _: &mut Initial<u32>) -> Result<(), String> {
            Err("gravity provider unavailable".into())
        }
        fn physical(&mut self, b: &mut Initial<u32>, _: f32) -> Result<(), String> {
            self.physical += 1;
            b.seed.storage.data_mut().unwrap()[0xb4..0xb8].copy_from_slice(&37f32.to_le_bytes());
            Err("declared late physical service failure".into())
        }
        fn retire(
            &mut self,
            _: &Initial<u32>,
            _: &Image,
            _: Option<&RetirementBinding>,
        ) -> Result<(), String> {
            self.retirement += 1;
            Err("declared late retirement service failure".into())
        }
    }

    #[test]
    #[ignore = "Requires GIANTS_FLICK_ROOT and private GIANTS_FLICK_TIMED_REGISTRATION_CONTROLS"]
    fn native_timed_positioned_constructor_keeps_registration_order() {
        use crate::{
            attached_effect_context::{Creation, Pool},
            audio_parameters::Random,
            resident_states::{Id, Session},
        };
        let root = std::path::PathBuf::from(
            std::env::var_os("GIANTS_FLICK_ROOT").expect("GIANTS_FLICK_ROOT"),
        );
        let control = std::env::var_os("GIANTS_FLICK_TIMED_REGISTRATION_CONTROLS")
            .expect("GIANTS_FLICK_TIMED_REGISTRATION_CONTROLS");
        let world = crate::world_source::resolve(&root, "Story4").unwrap();
        let resources =
            crate::scene_resources::SceneResources::load(&root, &world.catalog, "English").unwrap();
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
        let (_, entry) = image.find("fx_shotgun_flk").unwrap();
        let recipe =
            crate::timed_effect::Recipe::project(&image.program(entry.offset).unwrap()).unwrap();
        let (_, child) = image.find(&recipe.child).unwrap();
        let offset = child.offset;
        let text = std::fs::read_to_string(control).unwrap();
        let mut cases = 0;
        let mut discriminating = 0;
        for row in text.lines() {
            let e = row
                .split('\t')
                .map(str::parse::<u32>)
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            assert_eq!(e.len(), 4);
            let mut pool = Pool::<Id>::new(7000 + u64::from(e[0]), Some(image.clone()));
            let (creation, _) = pool
                .prepare_retirement_offset_invariant(
                    Some(offset),
                    [0.; 3],
                    0,
                    0x1000,
                    &mut Random(7),
                )
                .unwrap();
            let Creation::Prepared(origin) = creation else {
                panic!("positioned source absent")
            };
            let context = pool.get(origin).unwrap();
            let frame =
                crate::released_effect::Recipe::project(context.program.as_ref().unwrap()).unwrap();
            let mut template = Template::load(&resources, frame.class).unwrap();
            // Declared class-input variation, also run through the original
            // constructor. No installed source data is changed.
            template.fields.configuration[14] = e[1];
            let configured = template.gravity_positioned(origin, context).unwrap();
            let owned = Timed::new(template, configured).unwrap();
            let payload = owned.object().seed.storage.data().unwrap().as_ptr();
            let mut session = Session::new(8000 + u64::from(e[0]));
            let id = session.adopt_timed_positioned(owned).unwrap();
            let actor = session.get(id).unwrap();
            assert_eq!(actor.constructor_mask, e[2]);
            assert_eq!(
                actor.construction_fields.as_ref().unwrap().configuration[14],
                e[3]
            );
            assert_eq!(actor.seed.storage.data().unwrap().as_ptr(), payload);
            for list in 0..15 {
                assert_eq!(
                    session.list(list).contains(&id),
                    list == 0 || e[2] & (1 << (list - 1)) != 0
                );
            }
            let fields = actor.construction_fields.as_ref().unwrap();
            let after = crate::object_lists::membership(
                fields.class,
                78,
                crate::object_lists::DirectFlags {
                    active: fields.object_flags,
                    personality: fields.configuration[14],
                },
            );
            discriminating += usize::from(after != e[2]);
            assert_eq!(session.clear().residents, 1);
            pool.release_prepared(origin).unwrap();
            cases += 1;
        }
        assert_eq!(cases, 8);
        assert_eq!(discriminating, 6);
        eprintln!("native constructor/list cases={cases} pre/post discriminating={discriminating}");
    }

    #[test]
    #[ignore = "Requires GIANTS_FLICK_ROOT; silent source ownership/failure controls, no world admission"]
    fn source_timed_effect_owns_resources_and_quarantines_late_services() {
        use crate::attached_effect_context::{Creation, Pool};
        use std::{path::PathBuf, sync::Arc};
        let root = PathBuf::from(std::env::var_os("GIANTS_FLICK_ROOT").expect("GIANTS_FLICK_ROOT"));
        let world = crate::world_source::resolve(&root, "Story4").unwrap();
        let resources =
            crate::scene_resources::SceneResources::load(&root, &world.catalog, "English").unwrap();
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
        let mut pool = Pool::<u32>::new(400, Some(image));
        let mut random = crate::audio_parameters::Random(7);
        let (creation, _) = pool
            .prepare_cinematic_timed("fx_shotgun_flk", 9, 0, u32::MAX, &mut random)
            .unwrap();
        let Creation::Prepared(origin) = creation else {
            panic!("timed source absent")
        };
        let mut parent = pool
            .consume_timed_attached(origin, &resources, &mut random)
            .unwrap();
        assert!(pool.get(origin).is_none());
        let after_birth = random.0;
        assert!(
            pool.consume_timed_attached(origin, &resources, &mut random)
                .is_err()
        );
        assert_eq!(random.0, after_birth);
        let (creation, _) = pool
            .prepare_retirement_offset_invariant(
                Some(parent.retirement().unwrap().program_offset),
                parent.object().fields.position,
                parent.object().fields.object_flags,
                parent.object().render_flags,
                &mut random,
            )
            .unwrap();
        let Creation::Prepared(origin) = creation else {
            panic!("retirement source absent")
        };
        let mut child = pool.consume_timed_positioned(origin, &resources).unwrap();
        assert!(pool.get(origin).is_none());
        assert!(pool.consume_timed_positioned(origin, &resources).is_err());
        // Context and external resource owners can retire; actual class/state/
        // reference/image owners must remain alive in each timed object.
        drop(pool);
        drop(resources);
        let mut services = Unavailable {
            physical: 0,
            retirement: 0,
        };
        let mut stamp = Stamp(u32::MAX - 1);
        assert_eq!(
            parent
                .prepare_attached(Matrix::identity(), 1., 0., &mut stamp, &mut services)
                .unwrap(),
            Step::FirstFrame
        );
        assert!(parent.current_references().is_ok());
        let duration = f32::from_bits(word(parent.object().seed.storage.data().unwrap(), 4));
        let prepared_position = parent.object().fields.position;
        let mut later_support = Matrix::identity();
        later_support.values[3] = 97.;
        assert!(
            parent
                .prepare_attached(later_support, 1., duration * 2., &mut stamp, &mut services)
                .is_err()
        );
        assert!(parent.failure().is_some());
        assert_eq!(parent.object().fields.position, prepared_position);
        assert_eq!(parent.object().fields.object_flags & 0x8000, 0);
        assert!(f32::from_bits(word(parent.object().seed.storage.data().unwrap(), 8)) >= duration);
        assert!(parent.advance(0., &mut stamp, &mut services).is_err());
        assert_eq!(services.retirement, 1);
        assert_eq!(
            child.advance(0., &mut stamp, &mut services).unwrap(),
            Step::FirstFrame
        );
        assert_ne!(child.animation_epoch(), 0);
        assert!(child.current_references().is_ok());
        assert!(child.advance(0.01, &mut stamp, &mut services).is_err());
        assert_eq!(child.object().physical_words().unwrap()[5], 37f32.to_bits());
        assert!(child.advance(0.01, &mut stamp, &mut services).is_err());
        assert_eq!(services.physical, 1);
    }
}
