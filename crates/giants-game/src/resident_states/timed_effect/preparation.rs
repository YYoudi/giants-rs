//! Ordinary detached preparation over the canonical timed owner. World clocks,
//! geometry, population and the selected physical progress semantics stay explicit.
use super::{CinematicScope, Id, Placement, Session};
use crate::{actor_environment, actor_finalization, collider_copy};

/// Dynamic words retained from ordinary allocation. Class configuration, pose,
/// physical velocity/plane and inline physics have their existing single owners.
#[derive(Clone, Debug)]
pub struct Header {
    pub prepared: bool,
    pub model_rates: [f32; 4],
    pub previous_model_rates: [f32; 4],
    pub floor: f32,
    pub radius: f32,
    pub hit: u32,
    pub support: u32,
    pub footprint: Option<u32>,
    pub statistics: Option<actor_environment::Statistics>,
    preparing: bool,
}
impl Header {
    pub(super) fn preparing(&self) -> bool {
        self.preparing
    }
    pub(super) fn allocated() -> Self {
        Self {
            prepared: false,
            model_rates: [0.; 4],
            previous_model_rates: [0.; 4],
            floor: 0.,
            radius: 0.,
            hit: 0,
            support: 0,
            footprint: None,
            statistics: None,
            preparing: false,
        }
    }
}
pub struct Input<'a> {
    pub seconds: f32,
    pub finalization_time: actor_finalization::Time<'a>,
    /// Acquired PC caller argument, NOT a Rust identity, invented pointer or dt.
    pub physical_argument_word: u32,
}
/// Explicit portable progress policy, without manufacturing a PC argument word.
/// All physical, environment, footprint and clock services remain mandatory.
pub struct SourceInput<'a> {
    pub seconds: f32,
    pub finalization_time: actor_finalization::Time<'a>,
    pub progress: actor_finalization::PhysicalProgress,
    pub pc_argument: Option<u32>,
}
impl<'a> From<Input<'a>> for SourceInput<'a> {
    fn from(input: Input<'a>) -> Self {
        Self {
            seconds: input.seconds,
            finalization_time: input.finalization_time,
            progress: actor_finalization::PhysicalProgress::PcCallerArgument,
            pc_argument: Some(input.physical_argument_word),
        }
    }
}
pub trait Services: super::Services {
    fn release_footprint(
        &mut self,
        session: &Session,
        actor: Id,
        footprint: Option<u32>,
    ) -> Result<(), String>;
    /// Resolve the real owning world's environment. Return a retained failure
    /// from the shared resolver; never supply a default successful terrain.
    fn environment(
        &mut self,
        session: &Session,
        actor: Id,
        input: actor_environment::Input,
    ) -> Result<actor_environment::Output, actor_environment::Failure>;
}
struct Adapter<'a, S> {
    session: &'a mut Session,
    actor: Id,
    services: &'a mut S,
    physics: actor_environment::Physics,
    identity: u32,
}
impl<S: Services> actor_finalization::Services for Adapter<'_, S> {
    fn release_footprint(&mut self, identity: Option<u32>) -> Result<(), String> {
        self.services
            .release_footprint(self.session, self.actor, identity)
    }
    fn environment(&mut self, state: &mut actor_finalization::State) -> Result<(), String> {
        // The real actor remains registered, with its completed callback and
        // finalizer prefix visible while external world services run.
        self.session
            .commit_timed_finalization(self.actor, state, self.physics, false)?;
        let result = self.services.environment(
            self.session,
            self.actor,
            actor_environment::Input {
                actor: self.identity,
                state: 78,
                fields: state.fields,
                // The acquired timed profile has inline physical storage;
                // the nonphysical slope branch is unreachable here.
                slope_parameter: 0.,
                physics: Some(self.physics),
                skip_object_responses: false,
            },
        );
        let (output, error) = match result {
            Ok(output) => (output, None),
            Err(error) => (*error.output, Some(error.message)),
        };
        let physics = output
            .physics
            .ok_or("timed environment lost its owned physical projection")?;
        state.fields = output.fields;
        self.physics = physics;
        self.session
            .residents
            .get_mut(&self.actor.serial)
            .unwrap()
            .timed_effect
            .as_mut()
            .unwrap()
            .ordinary
            .statistics = Some(output.statistics);
        error.map_or(Ok(()), |message| Err(message.into()))
    }
}
impl Session {
    /// Common history for this exact cinematic body's next owned frame. This
    /// does not run its callback or make a failed preparation replayable.
    pub fn begin_cinematic_timed_frame(
        &mut self,
        id: Id,
        scope: CinematicScope,
    ) -> Result<(), String> {
        let actor = self.get(id).ok_or("cinematic timed history actor absent")?;
        let metadata = actor
            .timed_effect
            .as_ref()
            .ok_or("cinematic timed history metadata absent")?;
        if scope.ticket != self.ticket
            || metadata.cinematic_scope != Some(scope)
            || !metadata.runtime.idle()
            || metadata.runtime.failure.is_some()
            || metadata.ordinary.preparing
            || self.common_walk_running
        {
            return Err("cinematic timed history requires its idle current owner".into());
        }
        self.residents
            .get_mut(&id.serial)
            .unwrap()
            .construction_fields
            .as_mut()
            .ok_or("cinematic timed history fields absent")?
            .advance_position_history();
        Ok(())
    }
    pub fn timed_effect_ordinary_header(&self, id: Id) -> Option<&Header> {
        Some(&self.get(id)?.timed_effect.as_ref()?.ordinary)
    }
    /// Callback, common finalization and the unparented attachment tail. This
    /// consumes explicitly supplied clocks/caller semantics, not world admission.
    pub fn prepare_timed_positioned(
        &mut self,
        id: Id,
        input: Input<'_>,
        stamp: &mut crate::animation_epoch::Stamp,
        services: &mut impl Services,
    ) -> Result<crate::timed_effect_object::Step, String> {
        if self.cinematic_timed_scope(id).is_some() {
            return Err("cinematic positional preparation needs its current source scope".into());
        }
        self.prepare_timed_positioned_bound(id, input.into(), stamp, services)
    }
    /// The acquired positional preparation still requires its actual physical,
    /// terrain/environment, clock and retirement providers. Only ownership is
    /// qualified here; no missing service or world admission is acknowledged.
    pub fn prepare_cinematic_timed_positioned(
        &mut self,
        id: Id,
        scope: CinematicScope,
        input: Input<'_>,
        stamp: &mut crate::animation_epoch::Stamp,
        services: &mut impl Services,
    ) -> Result<crate::timed_effect_object::Step, String> {
        if scope.ticket != self.ticket || self.cinematic_timed_scope(id) != Some(scope) {
            return Err("cinematic positional preparation has a stale or foreign scope".into());
        }
        self.prepare_timed_positioned_bound(id, input.into(), stamp, services)
    }
    /// Source-based progress semantics are selected visibly by the composing
    /// game. This changes neither stock/terrain services nor scene readiness.
    pub fn prepare_cinematic_timed_positioned_source(
        &mut self,
        id: Id,
        scope: CinematicScope,
        input: SourceInput<'_>,
        stamp: &mut crate::animation_epoch::Stamp,
        services: &mut impl Services,
    ) -> Result<crate::timed_effect_object::Step, String> {
        if scope.ticket != self.ticket || self.cinematic_timed_scope(id) != Some(scope) {
            return Err("cinematic positional preparation has a stale or foreign scope".into());
        }
        self.prepare_timed_positioned_bound(id, input, stamp, services)
    }
    fn prepare_timed_positioned_bound(
        &mut self,
        id: Id,
        input: SourceInput<'_>,
        stamp: &mut crate::animation_epoch::Stamp,
        services: &mut impl Services,
    ) -> Result<crate::timed_effect_object::Step, String> {
        let identity = u32::try_from(id.serial).map_err(|_| "timed collision identity overflow")?;
        {
            let actor = self
                .get(id)
                .ok_or("timed preparation actor absent or foreign")?;
            let m = actor
                .timed_effect
                .as_ref()
                .ok_or("timed preparation owner absent")?;
            let f = actor
                .construction_fields
                .as_ref()
                .ok_or("timed preparation fields absent")?;
            if !matches!(m.placement, Placement::Position(_))
                || actor.presentation_parent.is_some()
                || actor.attachment.is_some()
                || m.ordinary.preparing
                || !m.runtime.idle()
                || m.runtime.failure.is_some()
                || f.configuration[14] & 0x10 != 0
                || input.finalization_time.context != f.dispatch_context
            {
                return Err(
                    "timed ordinary preparation needs acquired current dispatch/parent ownership"
                        .into(),
                );
            }
            if !input.seconds.is_finite()
                || input.seconds < 0.
                || !input.finalization_time.global_seconds.is_finite()
                || (matches!(
                    input.progress,
                    actor_finalization::PhysicalProgress::PcCallerArgument
                ) && input
                    .pc_argument
                    .is_none_or(|word| !f32::from_bits(word).is_finite()))
            {
                return Err("timed ordinary preparation outside finite time/caller profile".into());
            }
            // Validate the inline storage before publishing any preparation.
            crate::effect_actor::physical_data(&actor.seed)?;
            if f.object_flags & 0xc000 != 0 {
                return Ok(crate::timed_effect_object::Step::Inactive);
            }
        }
        let actor = self.residents.get_mut(&id.serial).unwrap();
        actor.construction_fields.as_mut().unwrap().object_flags &= !0x40;
        let h = &mut actor.timed_effect.as_mut().unwrap().ordinary;
        h.preparing = true;
        h.prepared = false;
        h.statistics = None;
        h.previous_model_rates = h.model_rates;
        let result = (|| {
            let step = self.advance_timed_effect_bound(id, input.seconds, stamp, services)?;
            let actor = self
                .get(id)
                .ok_or("timed actor removed during ordinary callback")?;
            let f = actor.construction_fields.as_ref().unwrap();
            let m = actor.timed_effect.as_ref().unwrap();
            let c = f.configuration;
            let mut fields = collider_copy::Fields::zeroed();
            fields.class = f.class as u32;
            fields.model = identity;
            fields.current = f.position;
            fields.previous = f.current;
            fields.scale = f.scale;
            fields.angles = f.angles;
            fields.object_flags = f.object_flags;
            fields.response_flags = f.response_flags;
            fields.collision_flags = c[14];
            fields.ground_offset = f32::from_bits(c[17]);
            fields.center_offset = f32::from_bits(c[22]);
            fields.height = f32::from_bits(c[23]);
            fields.packed_parameters = c[25];
            fields.radius_limit = f32::from_bits(c[26]);
            fields.radius_override = f32::from_bits(c[27]);
            fields.radius = m.ordinary.radius;
            fields.floor = m.ordinary.floor;
            fields.plane = m.physical.plane;
            fields.model_vertical_speed = m.ordinary.model_rates[0];
            fields.velocity = m.physical.velocity;
            fields.hit = m.ordinary.hit;
            fields.support = m.ordinary.support;
            fields.physical = true;
            fields.animation_slot = actor.channels.as_ref().unwrap()[0][0];
            let state = actor_finalization::State {
                fields,
                previous_physical_velocity: m.physical.previous_velocity,
                friction: f32::from_bits(c[28]),
                parent_present: false,
                footprint: m.ordinary.footprint,
                affine: Some(crate::actor_affine::Cache {
                    flags: m.matrix.flags,
                    matrix: m.matrix.values,
                }),
                physical_argument_word: input.pc_argument,
            };
            let p = crate::effect_actor::physical_data(&actor.seed)?;
            let word = |i: usize| u32::from_le_bytes(p[i * 4..i * 4 + 4].try_into().unwrap());
            let physics = actor_environment::Physics {
                gravity_mass: f32::from_bits(word(1)),
                restitution: f32::from_bits(word(3)),
                retention: f32::from_bits(word(4)),
                force_z: f32::from_bits(word(8)),
                flags: word(0),
            };
            let mut adapter = Adapter {
                session: self,
                actor: id,
                services,
                physics,
                identity,
            };
            let result = actor_finalization::finalize_with_progress(
                state,
                input.finalization_time,
                input.progress,
                &mut adapter,
            );
            let physics = adapter.physics;
            match result {
                Ok(state) => {
                    self.commit_timed_finalization(id, &state, physics, true)?;
                    // The acquired attachment helper returns immediately without
                    // a parent. No invented support or additional writer runs.
                    Ok(step)
                }
                Err(error) => {
                    self.commit_timed_finalization(id, &error.state, physics, false)?;
                    Err(error.to_string())
                }
            }
        })();
        if let Some(m) = self
            .residents
            .get_mut(&id.serial)
            .and_then(|a| a.timed_effect.as_mut())
        {
            m.ordinary.preparing = false;
            if let Err(error) = &result {
                m.runtime.failure = Some(error.clone());
            }
        }
        result
    }
    fn commit_timed_finalization(
        &mut self,
        id: Id,
        state: &actor_finalization::State,
        physics: actor_environment::Physics,
        affine_completed: bool,
    ) -> Result<(), String> {
        let actor = self
            .residents
            .get_mut(&id.serial)
            .ok_or("timed finalization owner removed")?;
        let f = actor.construction_fields.as_mut().unwrap();
        let m = actor.timed_effect.as_mut().unwrap();
        f.position = state.fields.current;
        f.current = state.fields.previous;
        f.object_flags = state.fields.object_flags;
        f.response_flags = state.fields.response_flags;
        m.physical.velocity = state.fields.velocity;
        m.physical.previous_velocity = state.previous_physical_velocity;
        m.physical.plane = state.fields.plane;
        m.ordinary.floor = state.fields.floor;
        m.ordinary.model_rates[0] = state.fields.model_vertical_speed;
        m.ordinary.hit = state.fields.hit;
        m.ordinary.support = state.fields.support;
        m.ordinary.footprint = state.footprint;
        let data = actor
            .seed
            .storage
            .data_mut()
            .ok_or("timed finalization inline payload absent")?;
        data[0xa0..0xa4].copy_from_slice(&physics.flags.to_le_bytes());
        data[0xc0..0xc4].copy_from_slice(&physics.force_z.to_le_bytes());
        if affine_completed {
            let cache = state.affine.ok_or("timed finalized affine absent")?;
            m.matrix.flags = cache.flags;
            m.matrix.values = cache.matrix;
            actor.affine_cache = Some(super::OwnedAffine {
                value: cache,
                pose: super::affine_pose_words(f),
                external_orientation: f.object_flags & 0x10000 != 0,
            });
            m.ordinary.prepared = true;
        }
        Ok(())
    }
}
