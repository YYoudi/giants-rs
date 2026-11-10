//! Canonical Session storage for acquired attached and detached timed effects.
//! No world admission, stock policy or default physical/retirement provider.
use super::*;
use crate::{
    effect_actor::Placement,
    procedural_aim::Matrix,
    timed_effect_object::{Body, Boundary, Runtime, Step, Timed},
};
use giants_formats::effect_image::Image;
mod preparation;
pub use preparation::{
    Header as OrdinaryHeader, Input as Preparation, Services as PreparationServices,
};

#[derive(Debug)]
pub(super) struct Metadata {
    origin: crate::attached_effect_context::Id,
    placement: Placement<Parent>,
    render_flags: u32,
    matrix: Matrix,
    runtime: Runtime,
    physical: PhysicalHeader,
    motion_done: bool,
    gravity_done: bool,
    ordinary: OrdinaryHeader,
    cinematic_scope: Option<CinematicScope>,
}
/// A virtual Flick support must never be registered as an ordinary actor.
#[derive(Clone, Copy, Debug)]
enum Parent {
    Actor(Id),
    Cinematic(CinematicSupport),
}
impl Metadata {
    pub(super) fn callback_pending(&self) -> bool {
        !self.runtime.idle() || self.ordinary.preparing()
    }
    pub(super) fn cinematic_scope(&self) -> Option<CinematicScope> {
        self.cinematic_scope
    }
    pub(super) fn cinematic_support(&self) -> Option<CinematicSupport> {
        match self.placement {
            Placement::Attached {
                recipient: Parent::Cinematic(support),
                ..
            } => Some(support),
            _ => None,
        }
    }
}
/// Dynamic header words cleared by ordinary allocation, distinct from the
/// inline physical payload. Environment/world producers remain prerequisites.
#[derive(Clone, Debug)]
pub struct PhysicalHeader {
    pub velocity: [f32; 3],
    pub previous_velocity: [f32; 3],
    pub plane: [f32; 4],
}
impl PhysicalHeader {
    fn allocated() -> Self {
        Self {
            velocity: [0.; 3],
            previous_velocity: [0.; 3],
            plane: [0.; 4],
        }
    }
}
/// Services execute with the actual actor still in the canonical Session map.
/// A callback prefix is already published; errors retain it and forbid replay.
pub trait Services {
    /// `seconds` is the callback's sample. A physical provider retains its own
    /// acquired clock queries; equal samples in a fixture do not merge clocks.
    fn physical(&mut self, session: &mut Session, actor: Id, seconds: f32) -> Result<(), String>;
    /// Runs after the actual affine has been published, before ANM.
    fn gravity(&mut self, session: &mut Session, actor: Id) -> Result<(), String>;
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
) -> Result<(&'a mut Runtime, Body<'a, Parent>), String> {
    let metadata = actor
        .timed_effect
        .as_mut()
        .ok_or("timed Session ownership absent")?;
    let parent_matches = match metadata.placement {
        Placement::Attached {
            recipient: Parent::Actor(parent),
            ..
        } => actor.presentation_parent == Some(parent) && metadata.cinematic_scope.is_none(),
        Placement::Attached {
            recipient: Parent::Cinematic(support),
            ..
        } => actor.presentation_parent.is_none() && metadata.cinematic_scope == Some(support.scope),
        Placement::Position(_) => actor.presentation_parent.is_none(),
    };
    if !parent_matches || actor.attachment.is_some() {
        return Err("timed canonical parent relationship changed".into());
    }
    Ok((
        &mut metadata.runtime,
        Body {
            velocity: Some(&mut metadata.physical.velocity),
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
    fn adopt_timed(&mut self, owned: Timed<Id>) -> Result<Id, String> {
        self.adopt_timed_bound(owned.map_parent(Parent::Actor), None)
    }
    /// Transfer an actual unadvanced source-marked body, retaining its virtual
    /// support identity. This does not supply stock admission or a world tick.
    pub fn adopt_cinematic_timed(&mut self, owned: Timed<CinematicSupport>) -> Result<Id, String> {
        let Placement::Attached { recipient, tag, .. } = owned.initial.placement else {
            return Err("cinematic timed body requires its actual virtual support".into());
        };
        if recipient.scope.ticket != self.ticket
            || tag != u32::MAX
            || owned.initial.render_flags != 0x1000
        {
            return Err("cinematic timed body outside its source owner/mark profile".into());
        }
        self.adopt_timed_bound(owned.map_parent(Parent::Cinematic), Some(recipient.scope))
    }
    /// A retirement child keeps the actual source scope while its placement is
    /// positional. Its canonical body does not acquire a fictional actor parent.
    pub fn adopt_cinematic_timed_positioned<K>(
        &mut self,
        owned: Timed<K>,
        scope: CinematicScope,
    ) -> Result<Id, String> {
        if scope.ticket != self.ticket
            || !matches!(owned.initial.placement, Placement::Position(_))
            || owned.initial.render_flags != 0x1000
        {
            return Err("cinematic positional body outside its source owner/mark profile".into());
        }
        self.adopt_timed_bound(
            owned.map_parent(|_| unreachable!("validated positional body has no recipient")),
            Some(scope),
        )
    }
    fn adopt_timed_bound(
        &mut self,
        mut owned: Timed<Parent>,
        cinematic_scope: Option<CinematicScope>,
    ) -> Result<Id, String> {
        let parent = match owned.initial.placement {
            Placement::Attached {
                recipient: Parent::Actor(recipient),
                ..
            } => {
                self.get(recipient)
                    .ok_or("timed constructor parent absent or foreign")?;
                Some(recipient)
            }
            Placement::Attached {
                recipient: Parent::Cinematic(support),
                ..
            } => {
                if support.scope.ticket != self.ticket {
                    return Err("timed virtual support belongs to a foreign Session".into());
                }
                None
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
            velocity: owned.velocity.as_mut(),
            fields: &mut owned.initial.fields,
            seed: &mut owned.initial.seed,
            matrix: &mut owned.initial.matrix,
            channels: None,
            references: &mut owned.references,
            placement: owned.initial.placement,
            render_flags: owned.initial.render_flags,
        })?;
        let Timed {
            velocity,
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
            physical: PhysicalHeader {
                velocity: velocity.unwrap_or([0.; 3]),
                ..PhysicalHeader::allocated()
            },
            motion_done: false,
            gravity_done: false,
            ordinary: OrdinaryHeader::allocated(),
            cinematic_scope,
        });
        self.publish_staged(staged, id)
    }
    pub fn timed_effect_origin(&self, id: Id) -> Option<crate::attached_effect_context::Id> {
        Some(self.get(id)?.timed_effect.as_ref()?.origin)
    }
    pub fn cinematic_timed_support(&self, id: Id) -> Option<CinematicSupport> {
        self.get(id)?.timed_effect.as_ref()?.cinematic_support()
    }
    pub fn cinematic_timed_scope(&self, id: Id) -> Option<CinematicScope> {
        self.get(id)?.timed_effect.as_ref()?.cinematic_scope()
    }
    pub fn timed_effect_failure(&self, id: Id) -> Option<&str> {
        self.get(id)?
            .timed_effect
            .as_ref()?
            .runtime
            .failure
            .as_deref()
    }
    /// Canonical physical-header velocity shared by the owned timed profiles.
    pub fn timed_effect_velocity(&self, id: Id) -> Option<[f32; 3]> {
        Some(self.get(id)?.timed_effect.as_ref()?.physical.velocity)
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
        Ok(&mut metadata.physical.velocity)
    }
    pub fn timed_effect_fade(&self, id: Id) -> Option<[f32; 4]> {
        Some(self.get(id)?.timed_effect.as_ref()?.runtime.fade)
    }
    pub fn timed_effect_render_flags(&self, id: Id) -> Option<u32> {
        Some(self.get(id)?.timed_effect.as_ref()?.render_flags)
    }
    pub fn timed_effect_transparency(&self, id: Id) -> Option<f32> {
        Some(self.get(id)?.timed_effect.as_ref()?.runtime.transparency)
    }
    pub fn timed_effect_matrix(&self, id: Id) -> Option<Matrix> {
        Some(self.get(id)?.timed_effect.as_ref()?.matrix)
    }
    pub fn timed_effect_physical_header(&self, id: Id) -> Option<&PhysicalHeader> {
        Some(&self.get(id)?.timed_effect.as_ref()?.physical)
    }
    /// Required motion boundary over the actual owned header and inline block.
    /// The caller supplies the independently resolved physical clock sample.
    pub fn advance_timed_effect_motion(&mut self, id: Id, seconds: f64) -> Result<(), String> {
        let m = self
            .get(id)
            .and_then(|a| a.timed_effect.as_ref())
            .ok_or("timed physical header absent")?;
        if m.motion_done {
            return Err("timed owned motion already consumed".into());
        }
        let header = m.physical.clone();
        let (seed, fields) = self.timed_effect_physical_fields(id)?;
        let p = crate::effect_actor::physical_data(seed)?;
        let p: [f32; 15] = std::array::from_fn(|i| {
            f32::from_bits(u32::from_le_bytes(p[i * 4..i * 4 + 4].try_into().unwrap()))
        });
        let motion = crate::mecc_motion::integrate_with_time(
            crate::mecc_motion::Motion {
                position: fields.position,
                velocity: header.velocity,
                previous_velocity: header.previous_velocity,
                force: [p[6], p[7], p[8]],
                mass: p[2],
                speed_limit: p[10],
                force_time_limit: p[11],
                stored_dt: p[5],
            },
            seconds,
            false,
            false,
            crate::arithmetic::Precision::Double53,
        )?;
        fields.position = motion.position;
        let data = seed.storage.data_mut().unwrap();
        data[0xb4..0xb8].copy_from_slice(&motion.stored_dt.to_le_bytes());
        data[0xcc..0xd0].copy_from_slice(&motion.force_time_limit.to_le_bytes());
        for (i, v) in motion.force.into_iter().enumerate() {
            data[0xb8 + i * 4..0xbc + i * 4].copy_from_slice(&v.to_le_bytes());
        }
        let header = &mut self
            .residents
            .get_mut(&id.serial)
            .unwrap()
            .timed_effect
            .as_mut()
            .unwrap()
            .physical;
        header.velocity = motion.velocity;
        header.previous_velocity = motion.previous_velocity;
        self.residents
            .get_mut(&id.serial)
            .unwrap()
            .timed_effect
            .as_mut()
            .unwrap()
            .motion_done = true;
        Ok(())
    }
    /// Gravity follows the affine store and uses the retained physical header.
    /// No contact/terrain update or world admission is supplied by this method.
    pub fn apply_timed_effect_gravity(&mut self, id: Id) -> Result<(), String> {
        let actor = self
            .get(id)
            .ok_or("timed gravity actor absent or foreign")?;
        let m = actor
            .timed_effect
            .as_ref()
            .ok_or("timed gravity metadata absent")?;
        if !m.runtime.awaiting_gravity()
            || !m.motion_done
            || m.gravity_done
            || actor
                .construction_fields
                .as_ref()
                .is_none_or(|f| f.control_flags & 0x80 == 0)
        {
            return Err("timed gravity service is not pending".into());
        }
        let header = m.physical.clone();
        let actor = self.residents.get_mut(&id.serial).unwrap();
        let p = crate::effect_actor::physical_data(&actor.seed)?;
        let words: [u32; 15] =
            std::array::from_fn(|i| u32::from_le_bytes(p[i * 4..i * 4 + 4].try_into().unwrap()));
        let p = words.map(f32::from_bits);
        let gravity = crate::mecc_gravity::apply(
            crate::mecc_gravity::Input {
                force: [p[6], p[7], p[8]],
                velocity: header.velocity,
                normal: [header.plane[0], header.plane[1], header.plane[2]],
                gravity_mass: p[1],
                mass: p[2],
                friction_limit: p[9],
                damping_xy: p[12],
                damping_z: p[13],
                force_time_limit: p[11],
                object_flags: actor.construction_fields.as_ref().unwrap().object_flags,
                physics_flags: words[0],
                project_contact: true,
            },
            crate::arithmetic::Precision::Double53,
        )?;
        actor.timed_effect.as_mut().unwrap().gravity_done = true;
        let data = actor.seed.storage.data_mut().unwrap();
        data[0xcc..0xd0].copy_from_slice(&gravity.force_time_limit.to_le_bytes());
        for (i, v) in gravity.force.into_iter().enumerate() {
            data[0xb8 + i * 4..0xbc + i * 4].copy_from_slice(&v.to_le_bytes());
        }
        Ok(())
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
    pub fn smoke_particle_ids(&self) -> Vec<Id> {
        self.list(0)
            .into_iter()
            .filter(|id| {
                self.get(*id)
                    .and_then(|a| a.timed_effect.as_ref())
                    .is_some_and(|m| m.runtime.smoke_particle())
            })
            .collect()
    }
    /// The acquired common and separate state78 effect passes have distinct
    /// flag gates. Outer visibility/model selection remain caller prerequisites.
    pub fn smoke_particle_draw_state(
        &self,
        id: Id,
    ) -> Result<Option<crate::smoke_particle_draw::State>, String> {
        let actor = self.get(id).ok_or("smoke draw actor absent or foreign")?;
        let m = actor
            .timed_effect
            .as_ref()
            .ok_or("smoke draw ownership absent")?;
        if !m.runtime.smoke_particle()
            || !matches!(m.placement, Placement::Position(_))
            || !self.list(0).contains(&id)
        {
            return Err("smoke draw outside acquired canonical profile".into());
        }
        if m.runtime.failure.is_some() || !m.runtime.idle() {
            return Err("smoke draw callback pending or quarantined".into());
        }
        let f = actor
            .construction_fields
            .as_ref()
            .ok_or("smoke draw header absent")?;
        if actor.seed.storage.profile() != Profile::Debug
            || actor.seed.storage.state() != 78
            || actor
                .seed
                .storage
                .data()
                .is_none_or(|data| data.len() != 268)
            || f.class != actor.class
        {
            return Err("smoke draw header/payload profile changed".into());
        }
        let world = u8::from(f.object_flags & 0x4000c080 == 0);
        let effect = u8::from(f.object_flags & 0x8000 == 0 && m.render_flags & 0x1000 != 0);
        let passes = world + effect;
        if passes == 0 {
            return Ok(None);
        }
        Ok(Some(crate::smoke_particle_draw::State {
            class: actor.class,
            passes,
            world: m.matrix,
            personality: f.configuration[14],
            position: f.position,
            scale: f.scale,
            angle: f.angles[0],
            transparency: m.runtime.transparency,
            epoch: m.runtime.epoch,
        }))
    }
    /// Destroy only a completed expired smoke child whose actual origin retired.
    /// This is a cleanup-phase operation, not a common-world traversal.
    pub fn destroy_expired_smoke_particle(
        &mut self,
        id: Id,
        contexts: &crate::attached_effect_context::Pool<Id>,
    ) -> Result<crate::timed_effect_object::Destruction, String> {
        let actor = self
            .get(id)
            .ok_or("expired smoke actor absent or foreign")?;
        let m = actor
            .timed_effect
            .as_ref()
            .ok_or("expired smoke ownership absent")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("expired smoke header absent")?;
        let data = actor
            .seed
            .storage
            .data()
            .ok_or("expired smoke payload absent")?;
        let lists = self
            .links
            .memberships(id.serial)
            .into_iter()
            .map(|(_, list)| list)
            .collect::<Vec<_>>();
        let expected = (0..14)
            .rev()
            .filter(|bit| m.runtime.constructor_mask & (1 << bit) != 0)
            .map(|bit| bit + 1)
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        if self.common_walk_running
            || self.common_walk_failure.is_some()
            || !m.runtime.expired_smoke()
            || !matches!(m.placement, Placement::Position(_))
            || !contexts.retired(m.origin)
            || !contexts.uses_image(&m.runtime.source)
            || actor.seed.storage.profile() != Profile::Debug
            || actor.seed.storage.state() != 78
            || data.len() != 268
            || data[0xf8..0x10c].iter().any(|v| *v != 0)
            || fields.object_flags & 0x8000 == 0
            || fields.control_flags & 0x20 != 0
            || fields.configuration[14] & 0x20 != 0
            || fields.shadow.is_some()
            || fields.action != 0
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || lists != expected
            || !self.specifications.contains_key(&id.serial)
            || !self.state_resources.contains_key(&id.serial)
            || self.declarations.values().any(|declared| *declared == id)
            || self.residents.values().any(|other| {
                other.presentation_parent == Some(id)
                    || other
                        .attachment
                        .is_some_and(|attachment| attachment.parent == id)
            })
        {
            return Err("smoke destruction requires completed expiry, retired origin and acquired independent cleanup profile".into());
        }
        let result = crate::timed_effect_object::Destruction {
            origin: m.origin,
            payload_bytes: data.len(),
            registrations: lists.len(),
        };
        // Recheck all immutable callback/payload guards before any ownership leaves.
        {
            let actor = self.residents.get_mut(&id.serial).unwrap();
            let references = &mut self.specifications.get_mut(&id.serial).unwrap().0;
            let (runtime, body) = borrow(actor, references)?;
            runtime.preflight(&body)?;
        }
        let actor = self.residents.get_mut(&id.serial).unwrap();
        // All optional state-owned blocks are absent in this complete profile.
        actor
            .seed
            .storage
            .transition_with(0, |_, _| Ok(()))
            .expect("checked expired smoke teardown into empty storage is infallible");
        self.specifications.remove(&id.serial);
        self.state_resources.remove(&id.serial);
        drop(
            self.residents
                .get_mut(&id.serial)
                .unwrap()
                .timed_effect
                .take(),
        );
        let removed = self.links.remove_object(id.serial);
        debug_assert_eq!(removed, result.registrations);
        self.residents.remove(&id.serial);
        Ok(result)
    }
    /// Complete acquired detached smoke callback using the common actual header,
    /// motion, affine and gravity owners. Clock producers remain caller-owned.
    /// The physical clock is required only if this callback reaches integration.
    pub fn advance_smoke_particle(
        &mut self,
        id: Id,
        seconds: f32,
        physical_seconds: Option<f64>,
        stamp: &mut crate::animation_epoch::Stamp,
    ) -> Result<Step, String> {
        let m = self
            .get(id)
            .and_then(|a| a.timed_effect.as_ref())
            .ok_or("smoke particle owner absent or foreign")?;
        if !m.runtime.smoke_particle() || !matches!(m.placement, Placement::Position(_)) {
            return Err("callback outside acquired detached smoke particle".into());
        }
        struct Source {
            physical_seconds: Option<f64>,
        }
        impl Services for Source {
            fn physical(&mut self, session: &mut Session, id: Id, _: f32) -> Result<(), String> {
                session.advance_timed_effect_motion(
                    id,
                    self.physical_seconds
                        .ok_or("smoke physical clock producer absent")?,
                )
            }
            fn gravity(&mut self, session: &mut Session, id: Id) -> Result<(), String> {
                session.apply_timed_effect_gravity(id)
            }
            fn retire(
                &mut self,
                _: &mut Session,
                _: Id,
                _: &Image,
                binding: Option<&crate::effect_scene::RetirementBinding>,
            ) -> Result<(), String> {
                // This complete source frame has no retirement program. The
                // native callback marks it dead without requesting another effect.
                if binding.is_some() {
                    Err("smoke retirement program outside acquired frame".into())
                } else {
                    Ok(())
                }
            }
        }
        self.advance_timed_effect(id, seconds, stamp, &mut Source { physical_seconds })
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
        if self.cinematic_timed_scope(id).is_some() {
            return Err("cinematic timed callback needs its current virtual support".into());
        }
        self.advance_timed_effect_bound(id, seconds, stamp, services)
    }
    fn advance_timed_effect_bound(
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
            let boundary = runtime.begin(&mut body, seconds)?;
            if boundary != Boundary::Inactive {
                let m = actor.timed_effect.as_mut().unwrap();
                m.motion_done = false;
                m.gravity_done = false;
            }
            boundary
        };
        if boundary == Boundary::Inactive {
            return Ok(Step::Inactive);
        }
        let mut result = match boundary {
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
        if boundary == Boundary::Physical {
            let actor = self
                .residents
                .get_mut(&id.serial)
                .ok_or("timed actor removed during motion service")?;
            let references = &mut self
                .specifications
                .get_mut(&id.serial)
                .ok_or("timed references removed during motion service")?
                .0;
            let (runtime, mut body) = borrow(actor, references)?;
            result = runtime.finish_motion(&mut body, result);
            self.refresh_timed_cache(id)?;
            if result.is_ok() {
                result = services.gravity(self, id);
            }
        }
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
        self.refresh_timed_cache(id)?;
        next
    }
    fn refresh_timed_cache(&mut self, id: Id) -> Result<(), String> {
        let actor = self
            .residents
            .get_mut(&id.serial)
            .ok_or("timed actor removed during callback")?;
        let metadata = actor
            .timed_effect
            .as_ref()
            .ok_or("timed metadata removed during callback")?;
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
        Ok(())
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
        if self.cinematic_timed_scope(id).is_some() {
            return Err("ordinary timed preparation cannot bind a virtual Flick support".into());
        }
        self.prepare_timed_parented_bound(id, support, support_scale, seconds, stamp, services)
    }
    /// Reuse the acquired parented callback on the actual canonical body. The
    /// matrix belongs to this exact source support; its native source scale is
    /// one. No ordinary parent ID, epoch, world admission or RNG is fabricated.
    pub fn prepare_cinematic_timed(
        &mut self,
        id: Id,
        support: CinematicSupport,
        matrix: Matrix,
        seconds: f32,
        stamp: &mut crate::animation_epoch::Stamp,
        services: &mut impl Services,
    ) -> Result<Step, String> {
        if support.scope.ticket != self.ticket || self.cinematic_timed_support(id) != Some(support)
        {
            return Err("cinematic timed preparation has a stale or foreign support".into());
        }
        self.prepare_timed_parented_bound(id, matrix, 1., seconds, stamp, services)
    }
    fn prepare_timed_parented_bound(
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
        let next = self.advance_timed_effect_bound(id, seconds, stamp, services)?;
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
    type PhaseControls = Arc<std::collections::BTreeMap<(u32, u32, u32, u32), Vec<u32>>>;
    struct TerrainProvider<'a> {
        effects: Provider,
        terrain: &'a giants_formats::gti::Terrain,
        queries: u32,
        releases: u32,
        fail_platform: bool,
    }
    impl Services for TerrainProvider<'_> {
        fn physical(
            &mut self,
            session: &mut Session,
            actor: Id,
            seconds: f32,
        ) -> Result<(), String> {
            self.effects.physical(session, actor, seconds)
        }
        fn gravity(&mut self, session: &mut Session, actor: Id) -> Result<(), String> {
            self.effects.gravity(session, actor)
        }
        fn retire(
            &mut self,
            session: &mut Session,
            actor: Id,
            source: &Image,
            binding: Option<&crate::effect_scene::RetirementBinding>,
        ) -> Result<(), String> {
            self.effects.retire(session, actor, source, binding)
        }
    }
    impl PreparationServices for TerrainProvider<'_> {
        fn release_footprint(
            &mut self,
            session: &Session,
            actor: Id,
            footprint: Option<u32>,
        ) -> Result<(), String> {
            assert!(session.get(actor).is_some());
            assert_eq!(footprint, None);
            self.releases += 1;
            Ok(())
        }
        fn environment(
            &mut self,
            session: &Session,
            actor: Id,
            input: crate::actor_environment::Input,
        ) -> Result<crate::actor_environment::Output, crate::actor_environment::Failure> {
            assert_eq!(
                session
                    .get(actor)
                    .unwrap()
                    .construction_fields
                    .as_ref()
                    .unwrap()
                    .position,
                input.fields.current
            );
            let terrain = self.terrain;
            let q = crate::terrain_queries::StaticTerrainQuery::new(
                terrain,
                1. / terrain.spacing,
                crate::arithmetic::Precision::Double53,
            )
            .unwrap();
            let fail_platform = self.fail_platform;
            crate::actor_environment::resolve_retaining(
                input,
                &[],
                |xy| {
                    self.queries += 1;
                    crate::physical_ground::Ground::from_sample(q.sample(xy)?, terrain.width)
                },
                |point| {
                    if fail_platform {
                        return Err("declared late platform failure");
                    }
                    crate::platform_queries::sample_prepared(
                        point,
                        &[],
                        crate::arithmetic::Precision::Double53,
                    )
                },
                |_| panic!("physical fixture must retain its own clock queries"),
            )
        }
    }
    struct Provider {
        pool: Pool<Id>,
        random: Random,
        retirements: usize,
        fail: bool,
        positions: Vec<[f32; 3]>,
        child: Option<crate::attached_effect_context::Id>,
        physical_calls: usize,
        physical_fail: bool,
        gravity_fail: bool,
        phase_controls: Option<PhaseControls>,
        phase_key: [u32; 3],
    }
    impl Provider {
        fn assert_phase(&self, session: &Session, id: Id, phase: u32) {
            let Some(controls) = &self.phase_controls else {
                return;
            };
            let actor = session.get(id).unwrap();
            let f = actor.construction_fields.as_ref().unwrap();
            let m = actor.timed_effect.as_ref().unwrap();
            let mut got = Vec::new();
            got.extend(f.position.map(f32::to_bits));
            got.push(f.scale.to_bits());
            got.extend(f.angles.map(f32::to_bits));
            got.extend([f.object_flags, f.response_flags, f.control_flags]);
            got.extend(f.current.map(f32::to_bits));
            got.extend(f.previous.map(f32::to_bits));
            got.extend(m.physical.velocity.map(f32::to_bits));
            got.extend(m.physical.previous_velocity.map(f32::to_bits));
            got.extend(m.physical.plane.map(f32::to_bits));
            got.push(m.matrix.flags);
            got.extend(m.matrix.values.map(f32::to_bits));
            got.extend(
                crate::effect_actor::physical_data(&actor.seed)
                    .unwrap()
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|w| u32::from_le_bytes(*w)),
            );
            let [case, kind, step] = self.phase_key;
            assert_eq!(
                &got,
                controls.get(&(case, kind, step, phase)).unwrap(),
                "native intermediate case {case} child {kind} step {step} phase {phase}"
            );
        }
    }
    impl Services for Provider {
        fn physical(&mut self, session: &mut Session, id: Id, dt: f32) -> Result<(), String> {
            self.assert_phase(session, id, 0);
            session.advance_timed_effect_motion(id, f64::from(dt))?;
            self.assert_phase(session, id, 1);
            assert!(
                session
                    .advance_timed_effect_motion(id, f64::from(dt))
                    .is_err()
            );
            self.physical_calls += 1;
            if self.physical_fail {
                Err("declared failure after actual physical writes".into())
            } else {
                Ok(())
            }
        }
        fn gravity(&mut self, session: &mut Session, id: Id) -> Result<(), String> {
            assert!(session.actor_current_affine(id).is_ok());
            assert!(session.timed_effect_physical_fields(id).is_err());
            self.assert_phase(session, id, 2);
            session.apply_timed_effect_gravity(id)?;
            self.assert_phase(session, id, 3);
            assert!(session.apply_timed_effect_gravity(id).is_err());
            if self.gravity_fail {
                Err("declared failure after actual gravity writes".into())
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
            if let Some(scope) = session.cinematic_timed_scope(id) {
                assert!(session.cancel_cinematic_scope(scope).is_err());
                assert!(session.get(id).is_some());
            }
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
            physical_calls: 0,
            physical_fail: false,
            gravity_fail: false,
            phase_controls: None,
            phase_key: [0; 3],
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
    #[ignore = "Requires GIANTS_FLICK_ROOT; silent source-support ownership and callback checks, no world admission"]
    fn source_timed_virtual_support_is_not_an_ordinary_parent() {
        let root = std::path::PathBuf::from(std::env::var_os("GIANTS_FLICK_ROOT").unwrap());
        let world = crate::world_source::resolve(&root, "Story4").unwrap();
        let resources = SceneResources::load(&root, &world.catalog, "English").unwrap();
        let (mut session, ordinary, mut service) = start(&resources, 4201, 19);
        let scope = CinematicScope {
            ticket: 4201,
            director: 3,
            generation: 7,
        };
        let support = CinematicSupport { scope, slot: 1 };
        let mut pool = Pool::from_active_resources(4201, &resources);
        let (created, _) = pool
            .prepare_cinematic_timed("fx_shotgun_flk", support, 0, u32::MAX, &mut service.random)
            .unwrap();
        let Creation::Prepared(origin) = created else {
            panic!("source context absent")
        };
        let owned = Timed::attached(
            &resources,
            origin,
            pool.get(origin).unwrap(),
            &mut service.random,
        )
        .unwrap();
        let payload = owned.initial.seed.storage.data().unwrap().as_ptr();
        let id = session.adopt_cinematic_timed(owned).unwrap();
        pool.release_prepared(origin).unwrap();
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
        assert_eq!(session.get(id).unwrap().presentation_parent, None);
        assert_eq!(session.cinematic_timed_support(id), Some(support));
        assert!(session.get(ordinary).is_some());
        let peer_scope = CinematicScope {
            generation: 9,
            ..scope
        };
        let peer_support = CinematicSupport {
            scope: peer_scope,
            ..support
        };
        let (created, _) = pool
            .prepare_cinematic_timed(
                "fx_shotgun_flk",
                peer_support,
                0,
                u32::MAX,
                &mut service.random,
            )
            .unwrap();
        let Creation::Prepared(peer_origin) = created else {
            panic!("peer context absent")
        };
        let seed = service.random.0;
        let mut foreign = Session::new(4202);
        assert!(
            pool.activate_cinematic_timed_attached(
                peer_origin,
                &mut foreign,
                &resources,
                &mut service.random,
            )
            .is_err()
        );
        assert_eq!(foreign.counts(), Counts::default());
        assert_eq!(service.random.0, seed);
        assert!(pool.get(peer_origin).is_some());
        let peer = pool
            .activate_cinematic_timed_attached(
                peer_origin,
                &mut session,
                &resources,
                &mut service.random,
            )
            .unwrap();
        assert!(pool.get(peer_origin).is_none());
        assert_eq!(session.cinematic_timed_support(peer), Some(peer_support));
        let before = projection(&session, id);
        let mut stamp = Stamp::default();
        assert!(
            session
                .advance_timed_effect(id, 0.01, &mut stamp, &mut service)
                .is_err()
        );
        assert!(
            session
                .prepare_timed_attached(id, Matrix::identity(), 1., 0.01, &mut stamp, &mut service,)
                .is_err()
        );
        for wrong in [
            CinematicSupport { slot: 2, ..support },
            CinematicSupport {
                scope: CinematicScope {
                    ticket: 4202,
                    ..scope
                },
                ..support
            },
            CinematicSupport {
                scope: CinematicScope {
                    director: 4,
                    ..scope
                },
                ..support
            },
            CinematicSupport {
                scope: CinematicScope {
                    generation: 8,
                    ..scope
                },
                ..support
            },
        ] {
            assert!(
                session
                    .prepare_cinematic_timed(
                        id,
                        wrong,
                        Matrix::identity(),
                        0.01,
                        &mut stamp,
                        &mut service,
                    )
                    .is_err()
            );
            assert_eq!(projection(&session, id), before);
            assert_eq!(stamp.0, 0);
        }
        let mut matrix = Matrix::identity();
        matrix.values[3] = 11.;
        matrix.values[7] = -7.;
        matrix.values[11] = 23.;
        for _ in 0..2 {
            session
                .prepare_cinematic_timed(id, support, matrix, 0.01, &mut stamp, &mut service)
                .unwrap();
        }
        assert!(stamp.0 > 0);
        session
            .prepare_cinematic_timed(id, support, matrix, 0.1, &mut stamp, &mut service)
            .unwrap();
        assert_eq!(service.retirements, 1);
        let origin = service.child.unwrap();
        let mut foreign = Session::new(4202);
        assert!(
            service
                .pool
                .activate_cinematic_timed_positioned(origin, scope, &mut foreign, &resources,)
                .is_err()
        );
        assert_eq!(foreign.counts(), Counts::default());
        assert!(service.pool.get(origin).is_some());
        let child = service
            .pool
            .activate_cinematic_timed_positioned(origin, scope, &mut session, &resources)
            .unwrap();
        assert!(service.pool.get(origin).is_none());
        assert_eq!(session.timed_effect_origin(child), Some(origin));
        assert_eq!(session.cinematic_timed_scope(child), Some(scope));
        assert_eq!(session.cinematic_timed_support(child), None);
        assert_eq!(session.get(child).unwrap().presentation_parent, None);
        assert!(
            session
                .advance_timed_effect(child, 0., &mut stamp, &mut service)
                .is_err()
        );
        assert_eq!(session.get(id).unwrap().presentation_parent, None);
        assert_eq!(session.cinematic_timed_support(id), Some(support));
        assert_eq!(
            session
                .cancel_cinematic_scope(CinematicScope {
                    generation: 8,
                    ..scope
                })
                .unwrap(),
            0
        );
        let retired_before = service.retirements;
        assert_eq!(session.cancel_cinematic_scope(scope).unwrap(), 2);
        assert!(session.get(id).is_none());
        assert!(session.get(child).is_none());
        assert!(session.get(ordinary).is_some());
        assert_eq!(service.retirements, retired_before);
        assert_eq!(session.cancel_cinematic_scope(scope).unwrap(), 0);
        assert!(session.get(peer).is_some());
        assert_eq!(session.cancel_cinematic_scope(peer_scope).unwrap(), 1);
        assert!(session.get(peer).is_none());
    }

    #[test]
    #[ignore = "Requires installed data and private original-PC ordinary preparation controls; silent, no world admission"]
    fn source_timed_positioned_preparation_uses_owned_header_and_terrain() {
        let root = std::path::PathBuf::from(std::env::var_os("GIANTS_FLICK_ROOT").unwrap());
        let world = crate::world_source::resolve(&root, "Story4").unwrap();
        let resources = SceneResources::load(&root, &world.catalog, "English").unwrap();
        let manifest =
            std::fs::read_to_string(std::env::var_os("GIANTS_FLICK_TIMED_TERRAINS").unwrap())
                .unwrap();
        let terrains: Vec<_> = manifest
            .lines()
            .map(|line| {
                let (archive, member) = line.split_once('\t').unwrap();
                let bytes = std::fs::read(archive).unwrap();
                let bytes = giants_formats::gzp::Archive::parse(&bytes)
                    .unwrap()
                    .read(member)
                    .unwrap();
                let mut terrain = giants_formats::gti::parse(&bytes).unwrap();
                // Acquired static prepared-grid boundary; dynamic ripples and real
                // mission mesh/platform populations remain separate prerequisites.
                for tag in &mut terrain.tags {
                    if *tag == 7 {
                        *tag = 4;
                    }
                }
                terrain
            })
            .collect();
        let rows: Vec<Vec<u32>> = std::fs::read_to_string(
            std::env::var_os("GIANTS_FLICK_TIMED_PREPARATION_CONTROLS").unwrap(),
        )
        .unwrap()
        .lines()
        .map(|line| {
            line.split_whitespace()
                .map(|w| w.parse().unwrap())
                .collect()
        })
        .collect();
        assert_eq!(terrains.len(), 16);
        assert_eq!(rows.len(), 1152);
        let mut total = 0;
        let mut total_queries = 0;
        // First pass compares entire owned continuations, not injected native
        // entries. Second pass discriminates retention/quarantine after contact.
        for failure_pass in [false, true] {
            let mut current = None;
            let mut run: Option<(Session, Id, TerrainProvider<'_>, Stamp)> = None;
            for e in &rows {
                assert_eq!(e.len(), 208);
                if failure_pass && e[0] != 0 {
                    break;
                }
                if current != Some(e[0]) {
                    if let Some((mut session, _, services, _)) = run.take() {
                        assert_eq!(services.effects.retirements, 2);
                        let removed = session.clear();
                        assert_eq!(removed.residents, 3);
                        assert_eq!(session.counts(), Counts::default());
                    }
                    let (session, id, effects) = start(&resources, 8000 + u64::from(e[0]), e[4]);
                    run = Some((
                        session,
                        id,
                        TerrainProvider {
                            effects,
                            terrain: &terrains[e[1] as usize],
                            queries: 0,
                            releases: 0,
                            fail_platform: false,
                        },
                        Stamp(0xfffffff0),
                    ));
                    current = Some(e[0]);
                }
                let (session, id, services, stamp) = run.as_mut().unwrap();
                if e[2] == 1 && e[3] == 0 {
                    let origin = services.effects.child.take().unwrap();
                    *id = services
                        .effects
                        .pool
                        .activate_timed_positioned(origin, session, &resources)
                        .unwrap();
                }
                let query_before = services.queries;
                let result = if e[2] == 0 {
                    let mut support = Matrix::identity();
                    for i in 0..3 {
                        support.values[i * 4 + 3] = f32::from_bits(e[7 + i]);
                    }
                    session.prepare_timed_attached(
                        *id,
                        support,
                        1.,
                        f32::from_bits(e[5]),
                        stamp,
                        services,
                    )
                } else {
                    services.fail_platform = failure_pass && e[3] == 2;
                    let context = session
                        .get(*id)
                        .unwrap()
                        .construction_fields
                        .as_ref()
                        .unwrap()
                        .dispatch_context;
                    session.prepare_timed_positioned(
                        *id,
                        Preparation {
                            seconds: f32::from_bits(e[5]),
                            finalization_time: crate::actor_finalization::Time {
                                global_seconds: f32::from_bits(e[5]),
                                context,
                                areas: &[],
                            },
                            physical_argument_word: e[6],
                        },
                        stamp,
                        services,
                    )
                };
                if failure_pass && e[2] == 1 && e[3] == 2 {
                    assert!(
                        result
                            .unwrap_err()
                            .contains("declared late platform failure")
                    );
                    let actor = session.get(*id).unwrap();
                    assert!(session.timed_effect_failure(*id).is_some());
                    assert!(session.actor_current_affine(*id).is_err());
                    let h = session.timed_effect_physical_header(*id).unwrap();
                    assert_eq!(h.plane.map(f32::to_bits), e[199..203]);
                    let bytes = actor.seed.storage.data().unwrap();
                    assert_eq!(
                        u32::from_le_bytes(bytes[0xa0..0xa4].try_into().unwrap()),
                        e[104]
                    );
                    assert_eq!(
                        u32::from_le_bytes(bytes[0xc0..0xc4].try_into().unwrap()),
                        e[112]
                    );
                    let before = (
                        services.queries,
                        services.releases,
                        services.effects.physical_calls,
                    );
                    let context = actor.construction_fields.as_ref().unwrap().dispatch_context;
                    assert!(
                        session
                            .prepare_timed_positioned(
                                *id,
                                Preparation {
                                    seconds: 0.,
                                    finalization_time: crate::actor_finalization::Time {
                                        global_seconds: 0.,
                                        context,
                                        areas: &[]
                                    },
                                    physical_argument_word: e[6]
                                },
                                stamp,
                                services
                            )
                            .is_err()
                    );
                    assert_eq!(
                        before,
                        (
                            services.queries,
                            services.releases,
                            services.effects.physical_calls
                        )
                    );
                    session.clear();
                    break;
                }
                result.unwrap();
                let got = projection(session, *id);
                assert_eq!(got.len(), 174);
                for (i, (a, b)) in got.iter().zip(&e[10..184]).enumerate() {
                    assert_eq!(
                        a, b,
                        "case {} child {} step {} projection word {i}",
                        e[0], e[2], e[3]
                    );
                }
                let h = session.timed_effect_ordinary_header(*id).unwrap();
                let p = session.timed_effect_physical_header(*id).unwrap();
                let mut dynamic = h
                    .model_rates
                    .into_iter()
                    .chain(h.previous_model_rates)
                    .chain(p.velocity)
                    .chain(p.previous_velocity)
                    .chain([h.floor])
                    .chain(p.plane)
                    .chain([h.radius])
                    .map(f32::to_bits)
                    .collect::<Vec<_>>();
                dynamic.extend([h.footprint.unwrap_or(0), h.hit, h.support]);
                assert_eq!(
                    &dynamic,
                    &e[184..207],
                    "case {} child {} step {} dynamic header",
                    e[0],
                    e[2],
                    e[3]
                );
                assert_eq!(services.queries - query_before, e[207]);
                if !failure_pass {
                    total += 1;
                    total_queries += services.queries - query_before;
                }
            }
            if let Some((mut session, _, services, _)) = run {
                if !failure_pass {
                    assert_eq!(services.effects.retirements, 2);
                }
                session.clear();
                assert_eq!(session.counts(), Counts::default());
            }
        }
        assert_eq!(total, 1152);
        assert_eq!(total_queries, 902);
        eprintln!(
            "owned ordinary preparation states={total} selected_words={} ground_queries={total_queries}; supplied clocks/caller word/support; empty mesh/platform populations",
            total * 197
        );
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
        // A late gravity failure retains both the preceding affine store and
        // actual force stores. Neither motion nor gravity can be replayed.
        let (mut late, parent_id, mut provider) = start(&resources, 730, 1);
        late.prepare_timed_attached(
            parent_id,
            Matrix::identity(),
            1.,
            0.,
            &mut stamp,
            &mut provider,
        )
        .unwrap();
        let lifetime = f32::from_bits(u32::from_le_bytes(
            late.get(parent_id).unwrap().seed.storage.data().unwrap()[4..8]
                .try_into()
                .unwrap(),
        ));
        late.prepare_timed_attached(
            parent_id,
            Matrix::identity(),
            1.,
            lifetime * 2.,
            &mut stamp,
            &mut provider,
        )
        .unwrap();
        let origin = provider.child.take().unwrap();
        let child = provider
            .pool
            .activate_timed_positioned(origin, &mut late, &resources)
            .unwrap();
        assert_eq!(
            late.timed_effect_physical_header(child).unwrap().velocity,
            [0.; 3]
        );
        late.advance_timed_effect(child, 0., &mut stamp, &mut provider)
            .unwrap();
        provider.gravity_fail = true;
        assert!(
            late.advance_timed_effect(child, 0.01, &mut stamp, &mut provider)
                .is_err()
        );
        assert!(late.actor_current_affine(child).is_ok());
        assert!(
            late.timed_effect_failure(child)
                .unwrap()
                .contains("gravity")
        );
        let payload = late
            .get(child)
            .unwrap()
            .seed
            .storage
            .data()
            .unwrap()
            .to_vec();
        assert!(
            late.advance_timed_effect(child, 0.01, &mut stamp, &mut provider)
                .is_err()
        );
        assert!(late.advance_timed_effect_motion(child, 0.01).is_err());
        assert!(late.apply_timed_effect_gravity(child).is_err());
        assert_eq!(
            late.get(child).unwrap().seed.storage.data().unwrap(),
            payload
        );
        late.clear();
        assert_eq!(late.counts(), Counts::default());
        let header_controls = std::env::var_os("GIANTS_FLICK_TIMED_HEADER_CONTROLS").map(|path| {
            std::fs::read_to_string(path)
                .unwrap()
                .lines()
                .map(|line| {
                    let words = line
                        .split_whitespace()
                        .map(str::parse::<u32>)
                        .collect::<Result<Vec<_>, _>>()
                        .unwrap();
                    assert_eq!(words.len(), 9);
                    ((words[0], words[1], words[2]), words[3..].to_vec())
                })
                .collect::<std::collections::BTreeMap<_, _>>()
        });
        let phase_controls: Option<PhaseControls> =
            std::env::var_os("GIANTS_FLICK_TIMED_PHASE_CONTROLS").map(|path| {
                Arc::new(
                    std::fs::read_to_string(path)
                        .unwrap()
                        .lines()
                        .map(|line| {
                            let w = line
                                .split_whitespace()
                                .map(str::parse::<u32>)
                                .collect::<Result<Vec<_>, _>>()
                                .unwrap();
                            assert_eq!(w.len(), 62);
                            ((w[0], w[1], w[2], w[3]), w[4..].to_vec())
                        })
                        .collect(),
                )
            });
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
                }
                services.phase_controls = phase_controls.clone();
                services.phase_key = [e[0], e[1], e[2]];
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
                if let Some(controls) = &header_controls {
                    let h = session.timed_effect_physical_header(*id).unwrap();
                    let got = h
                        .velocity
                        .into_iter()
                        .chain(h.previous_velocity)
                        .map(f32::to_bits)
                        .collect::<Vec<_>>();
                    assert_eq!(
                        &got,
                        controls.get(&(e[0], e[1], e[2])).unwrap(),
                        "native physical header case {} child {} step {}",
                        e[0],
                        e[1],
                        e[2]
                    );
                }
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
            if let Some(controls) = &phase_controls {
                assert_eq!(controls.len(), 384);
                eprintln!(
                    "native intermediate phases={} words={}",
                    controls.len(),
                    controls.len() * 58
                );
            }
            eprintln!(
                "canonical states={total} selected_words={words}; parent preparation and supplied-postpreparation detached callbacks"
            );
        }
    }
    #[test]
    #[ignore = "Requires GIANTS_MECC_WORLD_ROOT; actual expired smoke cleanup, peer/weak owners, origin generations and refusal prefixes; no renderer"]
    fn source_mecc_expired_smoke_particle_releases_only_owned_resources() {
        let root = std::path::PathBuf::from(std::env::var_os("GIANTS_MECC_WORLD_ROOT").unwrap());
        let resources =
            crate::scene_resources::SceneResources::load(&root, "w_Story2.bin", "English").unwrap();
        for name in [
            "fx_mecc_smoke_lite",
            "fx_mecc_smoke_medium",
            "fx_mecc_smoke_heavy",
        ] {
            let offset = resources
                .classes
                .models
                .effects
                .iter()
                .find_map(|e| e.image.find(name).map(|(_, entry)| entry.offset))
                .unwrap();
            let mut pool =
                crate::attached_effect_context::Pool::<Id>::from_active_resources(950, &resources);
            let mut session = Session::new(951);
            let mut random = crate::audio_parameters::Random(42);
            let (creation, _) = pool
                .prepare_retirement_offset_invariant(
                    Some(offset),
                    [17., -24., 30.],
                    0,
                    0,
                    &mut random,
                )
                .unwrap();
            let crate::attached_effect_context::Creation::Prepared(origin) = creation else {
                panic!("missing child")
            };
            let actor = pool
                .activate_smoke_particle(origin, &mut session, &resources, &mut random)
                .unwrap();
            let class = session.get(actor).unwrap().class;
            // A supplied cold shared-class peer; no ordinary state78 admission.
            let t = crate::effect_actor::Template::load(&resources, class).unwrap();
            let peer = session
                .construct_prechecked(
                    Construction {
                        class,
                        seed: &t.seed,
                        bindings: &t.bindings,
                        flags: DirectFlags {
                            active: t.fields.object_flags,
                            personality: t.fields.configuration[14],
                        },
                        specification: t.specification,
                        state_resources: t.states,
                        fields: t.fields.instantiate(),
                    },
                    &[],
                    None,
                )
                .unwrap();
            let peer_data = session
                .get(peer)
                .unwrap()
                .seed
                .storage
                .data()
                .unwrap()
                .to_vec();
            let peer_flags = session
                .get(peer)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap()
                .object_flags;
            let weak_spec = Arc::downgrade(session.specification(actor).unwrap());
            let weak_state = Arc::downgrade(session.state_resources(actor).unwrap());
            let initial = session.counts();
            assert!(
                session
                    .destroy_expired_smoke_particle(actor, &pool)
                    .is_err()
            );
            assert_eq!(session.counts().payload_bytes, initial.payload_bytes);
            let mut stamp = crate::animation_epoch::Stamp(0);
            session
                .advance_smoke_particle(actor, 0.01, None, &mut stamp)
                .unwrap();
            for dt in [0.01, 0.05, 0.2] {
                session
                    .advance_smoke_particle(actor, dt, Some(f64::from(dt)), &mut stamp)
                    .unwrap();
            }
            session
                .advance_smoke_particle(actor, 10., None, &mut stamp)
                .unwrap();
            let foreign =
                crate::attached_effect_context::Pool::from_active_resources(952, &resources);
            assert!(
                session
                    .destroy_expired_smoke_particle(actor, &foreign)
                    .is_err()
            );
            let mut walk = session.new_common_walk(4).unwrap();
            session
                .walk_common(&mut walk, &mut |live, _| {
                    assert!(live.destroy_expired_smoke_particle(actor, &pool).is_err());
                    Ok(crate::object_list_walk::Flow::Stop)
                })
                .unwrap();
            session
                .residents
                .get_mut(&peer.serial)
                .unwrap()
                .presentation_parent = Some(actor);
            assert!(
                session
                    .destroy_expired_smoke_particle(actor, &pool)
                    .is_err()
            );
            session
                .residents
                .get_mut(&peer.serial)
                .unwrap()
                .presentation_parent = None;
            session
                .residents
                .get_mut(&actor.serial)
                .unwrap()
                .seed
                .storage
                .data_mut()
                .unwrap()[0xf8] = 1;
            assert!(
                session
                    .destroy_expired_smoke_particle(actor, &pool)
                    .is_err()
            );
            session
                .residents
                .get_mut(&actor.serial)
                .unwrap()
                .seed
                .storage
                .data_mut()
                .unwrap()[0xf8] = 0;
            assert!(weak_spec.upgrade().is_some() && weak_state.upgrade().is_some());
            let (creation, _) = pool
                .prepare_retirement_offset_invariant(Some(offset), [0.; 3], 0, 0, &mut random)
                .unwrap();
            let crate::attached_effect_context::Creation::Prepared(newer) = creation else {
                panic!("missing new request")
            };
            assert_ne!(newer, origin);
            let saved = random;
            let before = session.counts();
            let report = session
                .destroy_expired_smoke_particle(actor, &pool)
                .unwrap();
            assert_eq!(report.origin, origin);
            assert_eq!(report.payload_bytes, 268);
            assert_eq!(report.registrations, 1);
            assert_eq!(session.counts().residents + 1, before.residents);
            assert_eq!(
                session.counts().payload_bytes + report.payload_bytes,
                before.payload_bytes
            );
            assert_eq!(
                session.counts().registrations + report.registrations,
                before.registrations
            );
            assert!(weak_spec.upgrade().is_none() && weak_state.upgrade().is_none());
            assert!(session.get(actor).is_none() && session.specification(actor).is_none());
            assert!(session.state_resources(actor).is_none());
            assert!((0..15).all(|list| !session.list(list).contains(&actor)));
            assert!(
                session
                    .destroy_expired_smoke_particle(actor, &pool)
                    .is_err()
            );
            assert_eq!(
                session.get(peer).unwrap().seed.storage.data().unwrap(),
                peer_data
            );
            assert_eq!(
                session
                    .get(peer)
                    .unwrap()
                    .construction_fields
                    .as_ref()
                    .unwrap()
                    .object_flags,
                peer_flags
            );
            assert!(
                session.specification(peer).is_some() && session.state_resources(peer).is_some()
            );
            assert!(pool.get(newer).is_some());
            assert_eq!(random.0, saved.0);
        }
    }
    #[test]
    #[ignore = "Requires GIANTS_MECC_WORLD_ROOT; actual smoke draw identity, two-pass admission and failed/expired refusal, no graphics"]
    fn source_mecc_smoke_draw_preserves_identity_and_pass_multiplicity() {
        let root = std::path::PathBuf::from(std::env::var_os("GIANTS_MECC_WORLD_ROOT").unwrap());
        let r =
            crate::scene_resources::SceneResources::load(&root, "w_Story2.bin", "English").unwrap();
        let offset = r
            .classes
            .models
            .effects
            .iter()
            .find_map(|e| e.image.find("fx_mecc_smoke_lite").map(|(_, v)| v.offset))
            .unwrap();
        let mut pool = crate::attached_effect_context::Pool::<Id>::from_active_resources(980, &r);
        let mut session = Session::new(981);
        let mut random = crate::audio_parameters::Random(42);
        let mut create = |session: &mut Session| {
            let (c, _) = pool
                .prepare_retirement_offset_invariant(
                    Some(offset),
                    [17., -24., 30.],
                    0,
                    0x1000,
                    &mut random,
                )
                .unwrap();
            let crate::attached_effect_context::Creation::Prepared(c) = c else {
                panic!("child absent")
            };
            pool.activate_smoke_particle(c, session, &r, &mut random)
                .unwrap()
        };
        let actor = create(&mut session);
        let peer = create(&mut session);
        assert_eq!(
            session
                .smoke_particle_draw_state(actor)
                .unwrap()
                .unwrap()
                .passes,
            1
        );
        let geometry = crate::smoke_particle_draw::Geometry::load(&r, &session, actor, 0).unwrap();
        let view = Matrix::identity().values;
        assert!(geometry.frame(&mut session, peer, view).is_err());
        let mut stamp = crate::animation_epoch::Stamp(0);
        session
            .advance_smoke_particle(actor, 0.01, None, &mut stamp)
            .unwrap();
        assert_eq!(
            geometry
                .frame(&mut session, actor, view)
                .unwrap()
                .unwrap()
                .passes,
            2
        );
        session
            .residents
            .get_mut(&actor.serial)
            .unwrap()
            .construction_fields
            .as_mut()
            .unwrap()
            .object_flags |= 0x4000;
        assert_eq!(
            geometry
                .frame(&mut session, actor, view)
                .unwrap()
                .unwrap()
                .passes,
            1
        );
        session
            .residents
            .get_mut(&actor.serial)
            .unwrap()
            .construction_fields
            .as_mut()
            .unwrap()
            .object_flags &= !0x4000;
        assert!(
            session
                .advance_smoke_particle(actor, 0.1, None, &mut stamp)
                .is_err()
        );
        assert!(geometry.frame(&mut session, actor, view).is_err());
        session
            .advance_smoke_particle(peer, 0.01, None, &mut stamp)
            .unwrap();
        session
            .advance_smoke_particle(peer, 10., None, &mut stamp)
            .unwrap();
        assert!(session.smoke_particle_draw_state(peer).unwrap().is_none());
    }
}
