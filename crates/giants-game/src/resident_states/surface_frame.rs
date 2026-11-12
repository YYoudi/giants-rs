//! Owned state48 child frames. Prototype PS2 `mound_move` / `mound_animselect`
//! are unassigned search candidates for PC debug-1.1 0x004dc540/0x004dc630;
//! no prototype layout is imported. See the reviewed symbol navigation index.
use super::*;
mod retirement;
#[cfg(test)]
mod tests;

#[derive(Clone, Debug)]
pub struct View {
    pub next_frame: u64,
    pub animation_epoch: Option<u32>,
    pub failure: Option<String>,
    pub cleanup_started: bool,
}
#[derive(Debug)]
pub(super) struct Owned {
    pub state: crate::actor_finalization::State,
    view: View,
    history: Option<u64>,
}
pub struct Frame<'a> {
    pub number: u64,
    pub global_seconds: f32,
    pub areas: &'a [crate::object_time::SlowArea],
    pub controlled: Id,
    pub delay: f32,
    pub blending: crate::animation_channels::Blending,
}
struct Adapter<'a, 'b> {
    frame: Frame<'a>,
    stamp: &'b mut crate::animation_epoch::Stamp,
    contexts: &'b mut crate::attached_effect_context::Pool<Id>,
    random: &'b mut crate::audio_parameters::Random,
    finalization: &'b mut dyn crate::actor_finalization::Services,
}
impl ActorWorldServices for Adapter<'_, '_> {
    fn controlled_actor(&self) -> Option<Id> {
        Some(self.frame.controlled)
    }
    fn primary(&mut self, session: &mut Session, id: Id) -> Result<(), String> {
        let actor = session.surface_frame_actor(id)?;
        let fields = actor.construction_fields.as_ref().unwrap();
        // This constructor clears rates/commands. Neither acquired state48
        // callback writes them; an idle or externally modified profile refuses.
        let entry = crate::actor_primary_gate::enter(
            fields.object_flags,
            fields.configuration[14],
            [0; 8],
            [0; 4],
            [0; 4],
        );
        if !entry.invoke {
            return Err("surface primary outside acquired non-idle constructor".into());
        }
        let Some(SurfaceLink::Marker {
            owner,
            look_pitch,
            mode,
            timer,
        }) = actor.surface_link
        else {
            unreachable!()
        };
        let peer = session.get(owner).ok_or("surface frame owner absent")?;
        if peer.surface_link != Some(SurfaceLink::Owner { marker: id }) {
            return Err("surface frame relation changed".into());
        }
        let next = crate::surface_marker::primary(
            crate::surface_marker::State {
                mode,
                timer,
                flags: fields.object_flags,
                position: fields.position,
            },
            actor.channels.unwrap()[0],
            Some(crate::surface_marker::Owner {
                state: u32::from(peer.seed.storage.state()),
                position: peer
                    .construction_fields
                    .as_ref()
                    .ok_or("surface peer fields absent")?
                    .position,
            }),
            self.frame.delay,
            crate::object_time::seconds(
                self.frame.global_seconds,
                Some(crate::object_time::Sample {
                    owner: fields.dispatch_context,
                    position: fields.position,
                }),
                None,
                self.frame.areas,
            ),
        )?;
        let actor = session.residents.get_mut(&id.serial).unwrap();
        let fields = actor.construction_fields.as_mut().unwrap();
        fields.configuration[14] = entry.binding_flags;
        fields.object_flags = next.flags;
        actor.surface_link = Some(SurfaceLink::Marker {
            owner,
            look_pitch,
            mode: next.mode,
            timer: next.timer,
        });
        Ok(())
    }
    fn secondary(&mut self, session: &mut Session, id: Id) -> Result<(), String> {
        let actor = session.surface_frame_actor(id)?;
        let Some(SurfaceLink::Marker { mode, .. }) = actor.surface_link else {
            unreachable!()
        };
        let channels =
            crate::surface_marker::secondary(mode, actor.channels.unwrap(), self.frame.blending)?;
        let actor = session.residents.get_mut(&id.serial).unwrap();
        actor.channels = Some(channels);
        actor.construction_fields.as_mut().unwrap().channels = channels;
        Ok(())
    }
    fn object_time(&mut self, session: &Session, id: Id) -> Result<f32, String> {
        let fields = session
            .surface_frame_actor(id)?
            .construction_fields
            .as_ref()
            .unwrap();
        Ok(crate::object_time::seconds(
            self.frame.global_seconds,
            Some(crate::object_time::Sample {
                owner: fields.dispatch_context,
                position: fields.position,
            }),
            None,
            self.frame.areas,
        ) as f32)
    }
    fn animation(&mut self, session: &mut Session, id: Id, seconds: f32) -> Result<(), String> {
        let epoch = session.advance_bound_ripper_animation(
            id,
            seconds,
            [0.; 3],
            self.stamp,
            self.contexts,
            self.random,
        )?;
        session
            .residents
            .get_mut(&id.serial)
            .unwrap()
            .surface_frame
            .as_mut()
            .unwrap()
            .view
            .animation_epoch = epoch;
        Ok(())
    }
    fn finalization(&mut self, session: &mut Session, id: Id) -> Result<(), String> {
        let actor = session.surface_frame_actor(id)?;
        let fields = actor.construction_fields.as_ref().unwrap();
        let mut state = actor.surface_frame.as_ref().unwrap().state.clone();
        state.fields.current = fields.position;
        state.fields.previous = fields.current;
        state.fields.angles = fields.angles;
        state.fields.object_flags = fields.object_flags;
        state.fields.response_flags = fields.response_flags;
        state.fields.collision_flags = fields.configuration[14];
        state.fields.animation_slot = actor.channels.unwrap()[0][0];
        let result = crate::actor_finalization::finalize(
            state,
            crate::actor_finalization::Time {
                global_seconds: self.frame.global_seconds,
                context: fields.dispatch_context,
                areas: self.frame.areas,
            },
            self.finalization,
        );
        let (state, error) = match result {
            Ok(state) => (state, None),
            Err(error) => (*error.state, Some(error.message)),
        };
        let actor = session.residents.get_mut(&id.serial).unwrap();
        let header = actor.construction_fields.as_mut().unwrap();
        header.position = state.fields.current;
        header.current = state.fields.previous;
        header.angles = state.fields.angles;
        header.object_flags = state.fields.object_flags;
        header.response_flags = state.fields.response_flags;
        header.configuration[14] = state.fields.collision_flags;
        if error.is_none() {
            actor.affine_cache = Some(OwnedAffine {
                value: state.affine.ok_or("surface finalization affine absent")?,
                pose: affine_pose_words(header),
                external_orientation: false,
            });
        }
        actor.surface_frame.as_mut().unwrap().state = state;
        error.map_or(Ok(()), Err)
    }
}
impl Session {
    /// PC class230 mode71 writes only its real class219 child's mode2. The
    /// ordinary child frame and existing retirement batch still own closing,
    /// event/reference completion, external release and unlinking.
    pub(super) fn mark_ripper_surface_marker_retirement(
        &mut self,
        owner: Id,
    ) -> Result<(), String> {
        let source = self.ordinary_frame_actor(owner)?;
        let combat = self.state23_snapshot(owner)?;
        if source.class != 230 || !source.ripper_primary_running || combat.mode != 71 {
            return Err(
                "retreat marker mark requires the actual running class230 mode71 primary".into(),
            );
        }
        let Some((marker, _)) = combat.owner else {
            return Ok(());
        };
        let target = self
            .get(marker)
            .ok_or("retreat owner relation absent or foreign")?;
        if target.class != 219 {
            return Ok(());
        }
        let target = self.surface_frame_actor(marker)?;
        let frame = target
            .surface_frame
            .as_ref()
            .ok_or("retreat marker world not prepared")?;
        if source.surface_link != Some(SurfaceLink::Owner { marker })
            || !matches!(target.surface_link,Some(SurfaceLink::Marker{owner: actual,..}) if actual==owner)
            || target.world_tick_running
            || frame.view.failure.is_some()
            || frame.view.cleanup_started
        {
            return Err(
                "retreat marker lacks its actual reciprocal non-running state48 owner".into(),
            );
        }
        let actor = self.residents.get_mut(&marker.serial).unwrap();
        let Some(SurfaceLink::Marker { mode, .. }) = actor.surface_link.as_mut() else {
            unreachable!()
        };
        *mode = 2;
        Ok(())
    }
    fn surface_frame_actor(&self, id: Id) -> Result<&Resident, String> {
        let actor = self
            .get(id)
            .ok_or("surface frame actor absent or foreign")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("surface frame fields absent")?;
        if !actor.acquired_construction
            || actor.class != 219
            || actor.seed.storage.state() != 48
            || actor.seed.storage.profile() != Profile::Debug
            || actor.constructor_mask != 0x11
            || !matches!(actor.surface_link, Some(SurfaceLink::Marker { .. }))
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || actor.ordinary_motion.is_some()
            || actor.finalization.is_some()
            || fields.dispatch_context != 0
            || fields.object_flags & 0x10000 != 0
            || fields.configuration[14] & 0x10 != 0
            || actor.channels != Some(fields.channels)
        {
            return Err("surface frame outside acquired linked unparented constructor".into());
        }
        Ok(actor)
    }
    /// Bind only a newly constructed marker. Its first callback did not receive
    /// the earlier cohort history pass; retain the actual copied history.
    pub fn prepare_surface_marker_world(&mut self, id: Id) -> Result<(), String> {
        let actor = self.surface_frame_actor(id)?;
        if actor.surface_frame.is_some()
            || actor.world_tick.is_some()
            || !matches!(
                actor.surface_link,
                Some(SurfaceLink::Marker {
                    mode: 0,
                    timer: 0.,
                    ..
                })
            )
            || actor.ripper_animation_consumed
            || actor.ripper_animation_failed
        {
            return Err("surface world preparation requires fresh birth".into());
        }
        let header = actor.construction_fields.as_ref().unwrap();
        let specification = self
            .specification(id)
            .ok_or("surface specification absent")?;
        if !specification.needs.is_empty() || !specification.corrections.is_empty() {
            return Err("surface world resources outside acquired body profile".into());
        }
        let c = header.configuration;
        let mut fields = crate::collider_copy::Fields::zeroed();
        fields.class = 219;
        fields.model = specification.specification;
        fields.current = header.position;
        fields.previous = header.current;
        fields.scale = header.scale;
        fields.angles = header.angles;
        fields.object_flags = header.object_flags;
        fields.response_flags = header.response_flags;
        fields.collision_flags = c[14];
        fields.ground_offset = f32::from_bits(c[17]);
        fields.center_offset = f32::from_bits(c[22]);
        fields.height = f32::from_bits(c[23]);
        fields.packed_parameters = c[25];
        fields.radius_limit = f32::from_bits(c[26]);
        fields.radius_override = f32::from_bits(c[27]);
        fields.animation_slot = header.channels[0][0];
        let owned = Owned {
            state: crate::actor_finalization::State {
                fields,
                previous_physical_velocity: [0.; 3],
                friction: f32::from_bits(c[28]),
                parent_present: false,
                footprint: None,
                affine: None,
                physical_argument_word: None,
            },
            view: View {
                next_frame: 0,
                animation_epoch: None,
                failure: None,
                cleanup_started: false,
            },
            history: Some(0),
        };
        self.residents.get_mut(&id.serial).unwrap().surface_frame = Some(owned);
        Ok(())
    }
    pub fn surface_marker_frame(&self, id: Id) -> Result<&View, String> {
        Ok(&self
            .surface_frame_actor(id)?
            .surface_frame
            .as_ref()
            .ok_or("surface frame not prepared")?
            .view)
    }
    pub fn begin_surface_marker_history(&mut self, id: Id, number: u64) -> Result<(), String> {
        let actor = self.surface_frame_actor(id)?;
        let owned = actor.surface_frame.as_ref().ok_or("surface frame absent")?;
        if number == 0
            || number != owned.view.next_frame
            || owned.history.is_some()
            || owned.view.failure.is_some()
            || actor.world_tick_running
            || actor
                .world_tick
                .as_ref()
                .is_none_or(|t| t.phase() != crate::actor_world_tick::Phase::Finished)
            || actor.construction_fields.as_ref().unwrap().object_flags & 0xc000 != 0
        {
            return Err("surface history requires completed active current frame".into());
        }
        let actor = self.residents.get_mut(&id.serial).unwrap();
        actor
            .construction_fields
            .as_mut()
            .unwrap()
            .advance_position_history();
        actor.surface_frame.as_mut().unwrap().history = Some(number);
        actor.world_tick = None;
        actor.ripper_animation_consumed = false;
        Ok(())
    }
    pub fn advance_surface_marker_world(
        &mut self,
        id: Id,
        frame: Frame<'_>,
        stamp: &mut crate::animation_epoch::Stamp,
        contexts: &mut crate::attached_effect_context::Pool<Id>,
        random: &mut crate::audio_parameters::Random,
        finalization: &mut dyn crate::actor_finalization::Services,
    ) -> Result<crate::actor_world_tick::Outcome, String> {
        let actor = self.surface_frame_actor(id)?;
        let owned = actor.surface_frame.as_ref().ok_or("surface frame absent")?;
        if owned.history != Some(frame.number)
            || owned.view.next_frame != frame.number
            || frame.number == u64::MAX
            || owned.view.failure.is_some()
            || actor.world_tick_running
            || actor.world_tick.is_some()
            || !frame.global_seconds.is_finite()
            || frame.global_seconds < 0.
            || !frame.delay.is_finite()
            || self.get(frame.controlled).is_none()
        {
            return Err("surface frame requires fresh history and current clock/controller".into());
        }
        let actor = self.residents.get_mut(&id.serial).unwrap();
        actor.world_tick_running = true;
        actor.surface_frame.as_mut().unwrap().history = None;
        let mut tick = crate::actor_world_tick::Tick::new();
        let mut adapter = Adapter {
            frame,
            stamp,
            contexts,
            random,
            finalization,
        };
        let result = tick.run(&mut ActorWorldAdapter {
            session: self,
            source: id,
            services: &mut adapter,
        });
        let actor = self
            .residents
            .get_mut(&id.serial)
            .ok_or("surface frame owner removed")?;
        actor.world_tick_running = false;
        actor.world_tick = Some(tick);
        let owned = actor.surface_frame.as_mut().unwrap();
        match &result {
            Ok(_) => {
                owned.view.next_frame += 1;
                owned.state.fields.object_flags =
                    actor.construction_fields.as_ref().unwrap().object_flags;
            }
            Err(error) => owned.view.failure = Some(error.clone()),
        }
        result
    }
}
