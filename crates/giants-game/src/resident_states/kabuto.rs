use super::{Id, Session};
use crate::{
    animation_channels::Blending,
    kabuto_player::{ColdFrame, Initialized},
    kabuto_resources::Resources,
    state_storage::Profile,
};
use std::sync::Arc;

#[cfg(test)]
#[path = "kabuto_transition_tests.rs"]
mod transition_tests;

#[cfg(test)]
#[path = "kabuto_growth_tests.rs"]
mod growth_tests;

#[cfg(test)]
#[path = "kabuto_motion_tests.rs"]
mod motion_tests;

#[path = "kabuto_classification.rs"]
mod classification;

impl Session {
    /// Common history entry before the bounded Kabuto motion prefix. No complete
    /// primary is acknowledged; a completed prefix cannot start another frame
    /// until the remaining native phases have a real consumer.
    pub fn begin_kabuto_motion_history(
        &mut self,
        owner: Id,
        number: u64,
        commands: [u8; 8],
    ) -> Result<bool, String> {
        let actor = self
            .get(owner)
            .ok_or("Kabuto history owner absent or foreign")?;
        let group = actor
            .kabuto_group
            .as_ref()
            .ok_or("Kabuto history group absent")?;
        if actor.class != 1
            || actor.seed.storage.state() != 1
            || group.cleaned_up
            || group.geometry_transition_failure.is_some()
            || group.motion_history.is_some()
            || group.motion_prefix_completed.is_some()
            || self
                .specification(owner)
                .is_none_or(|s| !Arc::ptr_eq(s, &group.declared_class))
        {
            return Err("Kabuto history requires its active group and unconsumed frame".into());
        }
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("Kabuto history fields absent")?;
        let entry = crate::actor_primary_gate::enter(
            fields.object_flags,
            fields.configuration[14],
            commands,
            group.motion.model.map(f32::to_bits),
            group.motion.previous_model.map(f32::to_bits),
        );
        let actor = self.residents.get_mut(&owner.serial).unwrap();
        let group = actor.kabuto_group.as_mut().unwrap();
        group.motion.commands = commands;
        if entry.invoke {
            group.motion.previous_model = entry.previous_rates.map(f32::from_bits);
            group.motion_history = Some(number);
            group.primary_started = true;
            actor.construction_fields.as_mut().unwrap().configuration[14] = entry.binding_flags;
        }
        Ok(entry.invoke)
    }

    pub(crate) fn advance_kabuto_motion_prefix(
        &mut self,
        owner: Id,
        resources: &Resources,
        number: u64,
        frame: crate::kabuto_motion_prefix::Frame<'_>,
    ) -> Result<(), String> {
        let actor = self
            .get(owner)
            .ok_or("Kabuto motion owner absent or foreign")?;
        let group = actor
            .kabuto_group
            .as_ref()
            .ok_or("Kabuto motion group absent")?;
        if actor.class != 1
            || actor.seed.storage.state() != 1
            || group.cleaned_up
            || group.geometry_transition_failure.is_some()
            || group.motion_history != Some(number)
            || group.motion_prefix_completed.is_some()
            || !Arc::ptr_eq(&group.declared_class, &resources.specification)
            || self
                .specification(owner)
                .is_none_or(|s| !Arc::ptr_eq(s, &resources.specification))
            || frame.parent_present
                != (actor.attachment.is_some() || actor.presentation_parent.is_some())
        {
            return Err(
                "Kabuto motion requires its factory-owned history and actual parent presence"
                    .into(),
            );
        }
        let payload = actor
            .seed
            .storage
            .data()
            .ok_or("Kabuto motion payload absent")?;
        let physics = group.physics.ok_or("Kabuto motion physics absent")?;
        if payload.len() != 844
            || physics.words()
                != crate::kabuto_physics::Physics::from_owned_payload(payload).words()
        {
            return Err("Kabuto motion physics and owned payload differ".into());
        }
        let prepared = crate::kabuto_motion_prefix::prepare(
            actor
                .construction_fields
                .as_ref()
                .ok_or("Kabuto motion fields absent")?,
            payload,
            group.motion,
            frame,
        )?;
        let actor = self.residents.get_mut(&owner.serial).unwrap();
        actor.construction_fields = Some(prepared.fields);
        actor
            .seed
            .storage
            .data_mut()
            .unwrap()
            .copy_from_slice(&prepared.payload);
        let group = actor.kabuto_group.as_mut().unwrap();
        group.physics = Some(crate::kabuto_physics::Physics::from_owned_payload(
            &prepared.payload,
        ));
        group.motion = prepared.state;
        group.motion_prefix_completed = Some(number);
        Ok(())
    }

    pub(crate) fn decide_kabuto_growth(
        &mut self,
        owner: Id,
        resources: &Resources,
        context: &crate::kabuto_geometry_mode::NotificationContext<'_>,
        mut observe: impl FnMut(
            crate::kabuto_growth::DecisionEvent,
            crate::kabuto_growth::DecisionState,
        ),
    ) -> Result<bool, String> {
        let actor = self
            .get(owner)
            .ok_or("Kabuto target owner absent or foreign")?;
        let group = actor
            .kabuto_group
            .as_ref()
            .ok_or("Kabuto target group absent")?;
        if actor.class != 1
            || actor.seed.storage.state() != 1
            || group.cleaned_up
            || group.geometry_transition_failure.is_some()
            || !Arc::ptr_eq(&group.declared_class, &resources.specification)
            || self
                .specification(owner)
                .is_none_or(|s| !Arc::ptr_eq(s, &resources.specification))
        {
            return Err("Kabuto target decision requires its active factory-owned group".into());
        }
        let payload = actor
            .seed
            .storage
            .data()
            .ok_or("Kabuto target payload absent")?;
        if payload.len() != 844 {
            return Err("Kabuto target payload profile differs".into());
        }
        let mut state = crate::kabuto_growth::DecisionState {
            trigger: f32::from_le_bytes(payload[0x94..0x98].try_into().unwrap()),
            target: f32::from_le_bytes(payload[0x158..0x15c].try_into().unwrap()),
        };
        let mut publish = |state: crate::kabuto_growth::DecisionState| {
            let actor = self.residents.get_mut(&owner.serial).unwrap();
            let payload = actor.seed.storage.data_mut().unwrap();
            payload[0x94..0x98].copy_from_slice(&state.trigger.to_le_bytes());
            payload[0x158..0x15c].copy_from_slice(&state.target.to_le_bytes());
            actor.kabuto_group.as_mut().unwrap().primary_started = true;
        };
        let changed = crate::kabuto_growth::decide(&mut state, context, |event, state| {
            publish(state);
            observe(event, state);
        })
        .map_err(str::to_owned)?;
        publish(state);
        Ok(changed)
    }

    pub(crate) fn advance_kabuto_growth(
        &mut self,
        owner: Id,
        resources: &Resources,
        globals: &giants_formats::global_data::GlobalData,
        bounds: giants_formats::world_bounds::Resolved,
        frame: crate::kabuto_growth::Frame<'_>,
    ) -> Result<crate::kabuto_growth::View, String> {
        let actor = self
            .get(owner)
            .ok_or("Kabuto growth owner absent or foreign")?;
        let group = actor
            .kabuto_group
            .as_ref()
            .ok_or("Kabuto growth group absent")?;
        if actor.class != 1
            || actor.seed.storage.state() != 1
            || group.cleaned_up
            || group.geometry_transition_failure.is_some()
            || !Arc::ptr_eq(&group.declared_class, &resources.specification)
            || self
                .specification(owner)
                .is_none_or(|s| !Arc::ptr_eq(s, &resources.specification))
        {
            return Err("Kabuto growth requires its active factory-owned group".into());
        }
        let mut fields = actor
            .construction_fields
            .as_ref()
            .ok_or("Kabuto growth fields absent")?
            .clone();
        fields.channels = actor.channels.ok_or("Kabuto growth channels absent")?;
        let mut payload = actor
            .seed
            .storage
            .data()
            .ok_or("Kabuto growth payload absent")?
            .to_vec();
        if payload.len() != 844 || !frame.target.is_finite() {
            return Err("Kabuto growth payload or target outside acquired profile".into());
        }
        let current = f32::from_le_bytes(payload[0x15c..0x160].try_into().unwrap());
        if !current.is_finite() {
            return Err("Kabuto growth current size is nonfinite".into());
        }
        let seconds = if current == frame.target {
            0.
        } else {
            crate::object_time_precision::seconds(
                frame.global_seconds,
                Some(crate::object_time_precision::Actor {
                    owner_id: fields.dispatch_context,
                    position: fields.position[..2].try_into().unwrap(),
                }),
                None,
                frame.areas,
                crate::arithmetic::Precision::Double53,
            )
            .map_err(str::to_owned)?
        };
        let view = crate::kabuto_growth::approach(current, frame.target, seconds)
            .map_err(str::to_owned)?;
        fields.object_flags |= 0x10000000;
        payload[0x158..0x15c].copy_from_slice(&frame.target.to_le_bytes());
        payload[0x15c..0x160].copy_from_slice(&view.current.to_le_bytes());
        let next = if view.refreshed {
            crate::kabuto_size::prepare(
                &fields,
                &payload,
                group.radius,
                Some(crate::kabuto_size::Source {
                    globals,
                    class_scale: resources.class_fields().scale,
                    bounds,
                }),
            )?
        } else {
            crate::kabuto_size::Prepared {
                fields,
                payload,
                radius: group.radius,
            }
        };
        let actor = self.residents.get_mut(&owner.serial).unwrap();
        actor
            .seed
            .storage
            .data_mut()
            .unwrap()
            .copy_from_slice(&next.payload);
        actor.construction_fields = Some(next.fields);
        let group = actor.kabuto_group.as_mut().unwrap();
        group.radius = next.radius;
        group.primary_started = true;
        Ok(view)
    }
    /// Select the second auxiliary from this owner's size parameter and the
    /// published actor positions. World pose/list production and ANM are separate.
    pub fn select_kabuto_auxiliary_animation(
        &mut self,
        owner: Id,
        local_player: Id,
    ) -> Result<f32, String> {
        use crate::kabuto_auxiliary_animation::{Input, Observers, prepare};
        let actor = self
            .get(owner)
            .ok_or("Kabuto auxiliary owner absent or foreign")?;
        let group = actor
            .kabuto_group
            .as_ref()
            .ok_or("Kabuto auxiliary group absent")?;
        if actor.class != 1
            || actor.seed.storage.state() != 1
            || group.cleaned_up
            || group.geometry_transition_failure.is_some()
        {
            return Err("Kabuto auxiliary group retired or quarantined".into());
        }
        let payload = actor
            .seed
            .storage
            .data()
            .ok_or("Kabuto auxiliary payload absent")?;
        if payload.len() != 844 {
            return Err("Kabuto auxiliary payload profile differs".into());
        }
        let radius = f32::from_le_bytes(payload[0x330..0x334].try_into().unwrap());
        let child = group.second;
        let auxiliary = self
            .get(child)
            .ok_or("Kabuto second auxiliary absent or foreign")?;
        if auxiliary.class != 1018
            || auxiliary.seed.storage.state() != 0
            || auxiliary.attachment.is_none_or(|a| {
                a.parent != owner || a.child_reference != 0 || a.parent_reference != 10
            })
            || self
                .specification(child)
                .is_none_or(|s| !s.animations.contains_key(&1))
        {
            return Err("Kabuto second auxiliary outside acquired binding".into());
        }
        let fields = auxiliary
            .construction_fields
            .as_ref()
            .ok_or("Kabuto auxiliary fields absent")?;
        let local = self
            .get(local_player)
            .and_then(|a| a.construction_fields.as_ref())
            .ok_or("Kabuto auxiliary local player absent or foreign")?;
        let positions = |list| {
            self.list(list)
                .into_iter()
                .map(|id| {
                    self.get(id)
                        .and_then(|a| a.construction_fields.as_ref())
                        .map(|f| f.position)
                        .ok_or_else(|| "Kabuto auxiliary list member fields absent".to_owned())
                })
                .collect::<Result<Vec<_>, String>>()
        };
        let first;
        let second;
        let observers = if owner == local_player {
            first = positions(2)?;
            second = positions(3)?;
            Observers::Local {
                first: &first,
                second: &second,
            }
        } else {
            Observers::Remote(local.position)
        };
        let next = prepare(
            Some(
                auxiliary
                    .channels
                    .as_ref()
                    .ok_or("Kabuto auxiliary channels absent")?,
            ),
            Input {
                position: fields.position,
                radius,
                observers,
            },
        )
        .map_err(str::to_owned)?
        .unwrap();
        let auxiliary = self.residents.get_mut(&child.serial).unwrap();
        auxiliary.channels = Some(next.channels);
        auxiliary.construction_fields.as_mut().unwrap().channels = next.channels;
        self.residents
            .get_mut(&owner.serial)
            .unwrap()
            .kabuto_group
            .as_mut()
            .unwrap()
            .primary_started = true;
        Ok(next.normalized_clock)
    }
    /// Acquired transition prefix. The mode and physical/contact samples are
    /// outputs of upstream phases; this does not schedule a primary or render.
    pub fn advance_kabuto_geometry_transition(
        &mut self,
        owner: Id,
        extra: &crate::kabuto_auxiliary::ExtraFactory,
        frame: crate::kabuto_geometry_transition::Frame<'_>,
        services: &mut crate::actor_animation_phase::Services<'_>,
        mut sound: impl FnMut(crate::kabuto_geometry_transition::Sound) -> Result<(), String>,
    ) -> Result<crate::kabuto_geometry_transition::View, String> {
        use crate::{
            animation_channels::Request,
            kabuto_geometry_transition::{Sound, SoundAction, View},
        };
        let actor = self
            .get(owner)
            .ok_or("Kabuto transition owner absent or foreign")?;
        let group = actor
            .kabuto_group
            .as_ref()
            .ok_or("Kabuto transition group absent")?;
        if actor.class != 1
            || actor.seed.storage.state() != 1
            || group.cleaned_up
            || group.geometry_transition_failure.is_some()
        {
            return Err("Kabuto transition group retired or quarantined".into());
        }
        if !frame.global_seconds.is_finite() || frame.global_seconds < 0. {
            return Err("Kabuto transition delta outside finite nonnegative profile".into());
        }
        let mut fields = actor
            .construction_fields
            .as_ref()
            .ok_or("Kabuto transition fields absent")?
            .clone();
        let payload = actor
            .seed
            .storage
            .data()
            .ok_or("Kabuto transition payload absent")?
            .to_vec();
        if payload.len() != 844 {
            return Err("Kabuto transition payload profile differs".into());
        }
        let specification = self
            .specification(owner)
            .ok_or("Kabuto transition specification absent")?
            .clone();
        let bank = self
            .kabuto_events
            .clone()
            .unwrap_or_else(|| crate::mecc_animation_events::Bank::new(specification.clone()));
        if !bank.belongs_to(&specification) {
            return Err("Kabuto event bank specification differs".into());
        }
        let mut pipeline = crate::actor_animation_phase::Pipeline {
            materials: frame.materials,
            specification,
            references: self.specifications.get(&owner.serial).unwrap().0.clone(),
            bank,
            contexts: services.contexts.staged_copy(),
            random: *services.random,
            channels: actor.channels.ok_or("Kabuto transition channels absent")?,
            owner,
            stamp: services.stamp,
            last_stamp: actor.mecc_animation_stamp,
            blending: services.blending,
        };
        fields.channels = pipeline.channels;
        self.residents
            .get_mut(&owner.serial)
            .unwrap()
            .kabuto_group
            .as_mut()
            .unwrap()
            .primary_started = true;
        self.residents
            .get_mut(&owner.serial)
            .unwrap()
            .seed
            .storage
            .data_mut()
            .unwrap()[0x228] = frame.mode;
        let mut view = View::default();
        let mut audio = |action, identifier, view: &mut View| {
            view.sound_calls += 1;
            sound(Sound {
                owner,
                identifier,
                action,
            })
        };
        fn select(
            p: &mut crate::actor_animation_phase::Pipeline<'_>,
            f: &mut crate::actor_fields::Fields,
            v: &mut View,
            slot: u8,
            flags: u32,
            blend_seconds: f32,
        ) -> Result<(), String> {
            crate::animation_channels::select_primary(
                &mut p.channels,
                Request {
                    slot,
                    flags,
                    speed_source: 0,
                    reference_speed: 0.,
                    blend_seconds,
                },
                p.blending,
            )
            .map_err(str::to_owned)?;
            f.channels = p.channels;
            v.selections += 1;
            Ok(())
        }
        fn tick(
            p: &crate::actor_animation_phase::Pipeline<'_>,
            v: &mut View,
        ) -> Result<i32, String> {
            let descriptor = p
                .specification
                .animations
                .get(&199)
                .ok_or("Kabuto gate descriptor absent")?;
            v.frame_queries += 1;
            crate::animation_frames::frame_at_time(
                crate::animation_frames::Descriptor {
                    timeline: descriptor.animation.timeline,
                    flags: descriptor.template.flags,
                },
                f32::from_bits(p.channels[0][1]),
            )
            .map_err(str::to_owned)
        }
        let result: Result<(), String> = (|| {
            match frame.mode {
                0 => {
                    match pipeline.channels[0][0] {
                        200 | 202 => {
                            audio(SoundAction::Start, 0x452, &mut view)?;
                            audio(SoundAction::Stop, 0x453, &mut view)?;
                            select(&mut pipeline, &mut fields, &mut view, 199, 0x30, 0.25)?;
                        }
                        199 => {
                            let flags = pipeline.channels[0][8];
                            if flags & 0x10 == 0 {
                                audio(SoundAction::Start, 0x452, &mut view)?;
                                audio(SoundAction::Stop, 0x453, &mut view)?;
                                select(&mut pipeline, &mut fields, &mut view, 199, 0x10, 0.)?;
                            } else if flags & 2 != 0 {
                                let word = |at| {
                                    u32::from_le_bytes(payload[at..at + 4].try_into().unwrap())
                                };
                                if word(0xc) != 0 {
                                    select(&mut pipeline, &mut fields, &mut view, 94, 0x20, 0.25)?;
                                } else if word(8) != 0 {
                                    select(&mut pipeline, &mut fields, &mut view, 125, 0x20, 0.25)?;
                                }
                                select(
                                    &mut pipeline,
                                    &mut fields,
                                    &mut view,
                                    if f32::from_bits(word(4)) == 0. {
                                        1
                                    } else {
                                        104
                                    },
                                    0x20,
                                    0.25,
                                )?;
                            }
                        }
                        _ => {}
                    }
                    if pipeline.channels[0][0] != 199 || tick(&pipeline, &mut view)? < 1600 {
                        view.marks += u32::from(self.mark_kabuto_extra(owner)?.is_some());
                    }
                }
                1 => {
                    match pipeline.channels[0][0] {
                        200 => {}
                        202 => {
                            audio(SoundAction::Stop, 0x453, &mut view)?;
                            select(&mut pipeline, &mut fields, &mut view, 200, 0x21, 0.25)?;
                        }
                        199 => {
                            if pipeline.channels[0][8] & 0x10 != 0 {
                                audio(SoundAction::Start, 0x455, &mut view)?;
                                select(&mut pipeline, &mut fields, &mut view, 199, 0, 0.)?;
                            } else if pipeline.channels[0][8] & 2 != 0 {
                                audio(SoundAction::Start, 0x453, &mut view)?;
                                select(&mut pipeline, &mut fields, &mut view, 200, 0x21, 0.25)?;
                            }
                        }
                        _ => {
                            audio(SoundAction::Start, 0x455, &mut view)?;
                            select(&mut pipeline, &mut fields, &mut view, 199, 0x20, 0.25)?;
                        }
                    }
                    if (pipeline.channels[0][0] != 199 || tick(&pipeline, &mut view)? > 1600)
                        && self.kabuto_group(owner).unwrap().extra.is_none()
                    {
                        extra.construct_for(self, owner)?;
                        view.creates += 1;
                    }
                }
                3 => {
                    if pipeline.channels[0][0] != 202 {
                        audio(SoundAction::Start, 0x453, &mut view)?;
                        select(&mut pipeline, &mut fields, &mut view, 202, 0x20, 0.25)?;
                    }
                    if self.kabuto_group(owner).unwrap().extra.is_none() {
                        extra.construct_for(self, owner)?;
                        view.creates += 1;
                    }
                }
                _ => {}
            }
            if pipeline.channels[0][8] & 0x100 != 0 {
                let seconds = crate::object_time_precision::seconds(
                    frame.global_seconds,
                    Some(crate::object_time_precision::Actor {
                        owner_id: fields.dispatch_context,
                        position: fields.position[..2].try_into().unwrap(),
                    }),
                    None,
                    frame.areas,
                    crate::arithmetic::Precision::Double53,
                )
                .map_err(str::to_owned)? as f32;
                let pass = pipeline.advance_sample(
                    &fields,
                    crate::actor_animation_phase::Sample {
                        model: frame.model,
                        affine: frame.affine,
                        support: frame.support,
                        support_part: frame.support_part,
                    },
                    seconds,
                )?;
                view.animation_processed = pass.processed;
                view.event_visits = pass.visits;
                fields.channels = pass.channels;
            }
            Ok(())
        })();
        // Preserve earlier selection, epoch, references, effects and RNG even if
        // a later provider fails. Retry requires a new Session rather than replay.
        fields.channels = pipeline.channels;
        view.stamp = pipeline.last_stamp;
        self.specifications.get_mut(&owner.serial).unwrap().0 = pipeline.references;
        self.kabuto_events = Some(pipeline.bank);
        *services.contexts = pipeline.contexts;
        *services.random = pipeline.random;
        let actor = self.residents.get_mut(&owner.serial).unwrap();
        actor.channels = Some(fields.channels);
        actor.construction_fields = Some(fields);
        actor.mecc_animation_stamp = view.stamp;
        if let Err(error) = result {
            actor
                .kabuto_group
                .as_mut()
                .unwrap()
                .geometry_transition_failure = Some(error.clone());
            return Err(error);
        }
        Ok(view)
    }
    /// Acquired frame query used by the geometry transition gate. Both the
    /// descriptor and clock come from this owner's resources/current channels.
    /// Selection, advancement and the transition decision remain caller phases.
    pub fn kabuto_geometry_frame(&self, owner: Id) -> Result<i32, String> {
        let actor = self
            .get(owner)
            .ok_or("Kabuto frame owner absent or foreign")?;
        if actor.class != 1
            || actor.seed.storage.state() != 1
            || actor
                .kabuto_group
                .as_ref()
                .is_none_or(|g| g.cleaned_up || g.geometry_transition_failure.is_some())
        {
            return Err("Kabuto frame requires an active owned group".into());
        }
        let channel = actor
            .channels
            .as_ref()
            .ok_or("Kabuto frame channels absent")?[0];
        if channel[0] != 199 {
            return Err("Kabuto geometry gate animation is not selected".into());
        }
        let specification = self
            .specification(owner)
            .ok_or("Kabuto frame specification absent")?;
        let descriptor = specification
            .animations
            .get(&199)
            .ok_or("Kabuto geometry gate descriptor absent")?;
        crate::animation_frames::frame_at_time(
            crate::animation_frames::Descriptor {
                timeline: descriptor.animation.timeline,
                flags: descriptor.template.flags,
            },
            f32::from_bits(channel[1]),
        )
        .map_err(str::to_owned)
    }
    pub(crate) fn construct_kabuto_extra(
        &mut self,
        owner: Id,
        input: super::Construction<'_>,
    ) -> Result<Id, String> {
        let actor = self
            .get(owner)
            .ok_or("Kabuto extra owner absent or foreign")?;
        let group = actor.kabuto_group.as_ref().ok_or("Kabuto group absent")?;
        if actor.class != 1
            || actor.seed.storage.state() != 1
            || group.cleaned_up
            || group.geometry_transition_failure.is_some()
            || group.extra.is_some()
            || input.class != 1199
            || input.seed.storage.state() != 0
        {
            return Err("Kabuto extra requires its live group and an empty reference".into());
        }
        let mut staged = Self::new(self.ticket);
        staged.next = self.next;
        let id = staged.construct_kabuto_auxiliary(input, 0x80000000)?;
        let actor = staged.residents.get_mut(&id.serial).unwrap();
        actor.construction_fields.as_mut().unwrap().response_flags |= 0x2000000;
        actor.attachment = Some(super::Attachment {
            parent: owner,
            child_reference: 0,
            parent_reference: 3,
            behavior: 0,
        });
        actor.presentation_parent = Some(owner);
        self.publish_staged(staged, id)?;
        self.residents
            .get_mut(&owner.serial)
            .unwrap()
            .kabuto_group
            .as_mut()
            .unwrap()
            .extra = Some(id);
        Ok(id)
    }
    /// Acquired reference-clear phase. Marking does not unlink the actor, clear
    /// its parent or retire its resources. Full dynamic cleanup stays separate.
    pub fn mark_kabuto_extra(&mut self, owner: Id) -> Result<Option<Id>, String> {
        let actor = self
            .get(owner)
            .ok_or("Kabuto extra owner absent or foreign")?;
        let group = actor.kabuto_group.as_ref().ok_or("Kabuto group absent")?;
        if actor.class != 1
            || actor.seed.storage.state() != 1
            || group.cleaned_up
            || group.geometry_transition_failure.is_some()
        {
            return Err("Kabuto extra group outside acquired phase".into());
        }
        let Some(id) = group.extra else {
            return Ok(None);
        };
        let extra = self
            .get(id)
            .ok_or("Kabuto extra reference absent or foreign")?;
        if extra.class != 1199
            || extra.seed.storage.state() != 0
            || extra.attachment.is_none_or(|a| a.parent != owner)
            || extra.construction_fields.is_none()
        {
            return Err("Kabuto extra reference outside acquired binding".into());
        }
        self.mark_wield_child(id)?;
        self.residents
            .get_mut(&owner.serial)
            .unwrap()
            .kabuto_group
            .as_mut()
            .unwrap()
            .extra = None;
        Ok(Some(id))
    }
    pub(crate) fn bind_kabuto_geometry_registry(
        &mut self,
        owner: Id,
        handle: crate::kabuto_geometry_registry::Handle,
        texture: Arc<crate::kabuto_texture::Texture>,
    ) -> Result<(), String> {
        let group = self
            .residents
            .get_mut(&owner.serial)
            .filter(|_| owner.ticket() == self.ticket)
            .and_then(|a| a.kabuto_group.as_mut())
            .ok_or("Kabuto registered group absent")?;
        if group.cleaned_up
            || group.geometry_registration.is_some()
            || group.procedural_texture.is_some()
        {
            return Err("Kabuto geometry already bound or retired".into());
        }
        group.geometry_registration = Some(handle);
        group.procedural_texture = Some(texture);
        Ok(())
    }
    /// Acquired custom destructor phase for a registered cold group. Marking
    /// retains actors/lists. The common state transition and GPU release remain
    /// separate; this does not retire the owner or admit another player.
    pub fn cleanup_registered_kabuto(
        &mut self,
        owner: Id,
        registry: &mut crate::kabuto_geometry_registry::Registry,
    ) -> Result<crate::kabuto_geometry_registry::Cleanup, String> {
        let actor = self
            .get(owner)
            .ok_or("Kabuto cleanup owner absent or foreign")?;
        let group = actor
            .kabuto_group
            .as_ref()
            .ok_or("Kabuto cleanup group absent")?;
        if actor.class != 1
            || actor.seed.storage.state() != 1
            || group.cleaned_up
            || group.primary_started
            || group.extra.is_some()
            || actor.world_tick.is_some()
            || actor.controlled_motion.is_some()
            || actor
                .seed
                .storage
                .data()
                .is_none_or(|p| p.len() != 844 || p[0x32c..0x330] != [0; 4])
        {
            return Err(
                "Kabuto cleanup outside acquired cold/empty-extra-reference profile".into(),
            );
        }
        let handle = group
            .geometry_registration
            .as_ref()
            .ok_or("Kabuto geometry registration absent")?
            .clone();
        if registry.owner(&handle)? != owner {
            return Err("Kabuto geometry owner mismatch".into());
        }
        let mut marked: Vec<_> = self
            .list(0)
            .into_iter()
            .filter(|id| {
                self.get(*id)
                    .and_then(|a| a.attachment)
                    .is_some_and(|a| a.parent == owner)
            })
            .collect();
        marked.extend(group.effects);
        marked.push(group.final_effect);
        if marked.iter().any(|id| {
            self.get(*id)
                .and_then(|a| a.construction_fields.as_ref())
                .is_none()
        }) {
            return Err("Kabuto cleanup child fields absent or foreign".into());
        }
        // No fallible publication follows preflight. Reuse the common deletion
        // bit writer; despite its earlier adapter name it has no class gate.
        for id in &marked {
            self.mark_wield_child(*id).unwrap();
        }
        let retirement = registry.retire(&handle).unwrap();
        let group = self
            .residents
            .get_mut(&owner.serial)
            .unwrap()
            .kabuto_group
            .as_mut()
            .unwrap();
        group.cleaned_up = true;
        group.geometry_registration = None;
        group.procedural_texture = None;
        // Source buffers cease to be active here, before common payload release.
        drop(std::mem::take(&mut group.geometry.vertices));
        drop(std::mem::take(&mut group.geometry.indices));
        Ok(crate::kabuto_geometry_registry::Cleanup {
            marked,
            material_retired: retirement.material_retired,
        })
    }
    pub(crate) fn construct_kabuto_auxiliary(
        &mut self,
        input: super::Construction<'_>,
        mask: u32,
    ) -> Result<Id, String> {
        let effect = matches!(
            (input.class, input.seed.storage.state(), mask),
            (866 | 980, 78, 0)
        );
        if !(effect
            || matches!(
                (input.class, input.seed.storage.state(), mask),
                (749 | 1018, 0, 1) | (1199, 0, 0x80000000)
            ))
            || input.seed.storage.profile() != Profile::Debug
            || !input.seed.diagnostics.is_empty()
            || !input.bindings.is_empty()
            || input.fields.object_flags != input.flags.active
            || input.fields.configuration[14] != input.flags.personality
            || (effect
                && input
                    .seed
                    .storage
                    .data()
                    .is_none_or(|p| p.len() != 268 || p.iter().any(|&b| b != 0)))
        {
            return Err(
                "Kabuto auxiliary requires its acquired class/state/mask and cold resources".into(),
            );
        }
        let mut staged = Self::new(self.ticket);
        staged.next = self.next;
        let id = staged.register_with_mask(
            input.class,
            input.seed,
            input.bindings,
            input.flags,
            mask,
        )?;
        let channels = input.fields.channels;
        staged.bind_specification(id, input.specification)?;
        staged.bind_state_resources(id, input.state_resources)?;
        staged.bind_actor_fields(id, input.fields)?;
        let actor = staged.residents.get_mut(&id.serial).unwrap();
        actor.channels = Some(channels);
        if effect {
            // The actual declared state is allocated first; only its observed
            // all-zero destructor profile is admitted. Live emitters are refused.
            actor.seed.storage.transition_with(0, |state, body| {
                if state == 78 && body.len() == 268 && body.iter().all(|&b| b == 0) {
                    Ok(())
                } else {
                    Err("Kabuto cold effect cleanup requires empty state78".into())
                }
            })?;
            actor.construction_fields.as_mut().unwrap().configuration[20] = 0;
        }
        actor.acquired_construction = true;
        self.publish_staged(staged, id)
    }
    pub fn kabuto_group(&self, id: Id) -> Option<&crate::kabuto_initial::Group> {
        self.get(id)?.kabuto_group.as_ref()
    }
    /// The actual seeded external-orientation cache, not an evaluated world
    /// transform or contact-readiness acknowledgement.
    pub fn kabuto_initial_effect_cache(
        &self,
        owner: Id,
        id: Id,
    ) -> Option<crate::actor_affine::Cache> {
        let group = self.kabuto_group(owner)?;
        if !group.effects.contains(&id) && group.final_effect != id {
            return None;
        }
        let actor = self.get(id)?;
        let fields = actor.construction_fields.as_ref()?;
        let cache = actor.affine_cache.as_ref()?;
        if actor.world_tick.is_some()
            || actor.controlled_motion.is_some()
            || actor.finalization.is_some()
            || cache.pose != super::affine_pose_words(fields)
            || cache.value.flags != 0
        {
            return None;
        }
        Some(cache.value)
    }
    pub(crate) fn stage_kabuto_group(
        &mut self,
        build: impl FnOnce(&mut Self) -> Result<Id, String>,
    ) -> Result<Id, String> {
        if self.residents.values().any(|a| a.kabuto_group.is_some()) {
            return Err(
                "Kabuto group already published; live type replacement is unacquired".into(),
            );
        }
        let mut staged = Self::new(self.ticket);
        staged.next = self.next;
        let owner = build(&mut staged)?;
        if staged.get(owner).is_none_or(|a| a.kabuto_group.is_none())
            || staged.residents.len() != 12
        {
            return Err("Kabuto cold group is incomplete".into());
        }
        let mut links = self.links.clone();
        for (&serial, actor) in &staged.residents {
            links.construct(
                serial,
                actor.class,
                u32::from(actor.seed.storage.state()),
                actor.initial_flags,
                actor.constructor_mask,
            )?;
        }
        // No fallible operation follows publication of the fully checked group.
        self.links = links;
        self.next = staged.next;
        self.residents.append(&mut staged.residents);
        self.specifications.append(&mut staged.specifications);
        self.state_resources.append(&mut staged.state_resources);
        Ok(owner)
    }
    pub(crate) fn bind_kabuto_attachment(
        &mut self,
        owner: Id,
        child: Id,
        second: bool,
    ) -> Result<(), String> {
        let context = self
            .get(owner)
            .filter(|a| a.class == 1 && a.kabuto_player_fields)
            .and_then(|a| a.construction_fields.as_ref())
            .ok_or("Kabuto attachment owner absent")?
            .dispatch_context;
        let actor = self.get(child).ok_or("Kabuto attachment child absent")?;
        if actor.class != if second { 1018 } else { 749 }
            || actor.seed.storage.state() != 0
            || !actor.acquired_construction
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
        {
            return Err("Kabuto attachment requires its fresh acquired child".into());
        }
        let actor = self.residents.get_mut(&child.serial).unwrap();
        let fields = actor.construction_fields.as_mut().unwrap();
        fields.dispatch_context = context;
        fields.configuration[14] |= if second { 0x20000000 } else { 0x24000000 };
        fields.object_flags |= 0x400000;
        fields.response_flags |= 0x2000000;
        actor.attachment = Some(super::Attachment {
            parent: owner,
            child_reference: 0,
            parent_reference: 10,
            behavior: 0,
        });
        actor.presentation_parent = Some(owner);
        Ok(())
    }
    pub(crate) fn bind_kabuto_effect(&mut self, owner: Id, id: Id) -> Result<(), String> {
        let context = self
            .get(owner)
            .filter(|a| a.class == 1 && a.kabuto_player_fields)
            .and_then(|a| a.construction_fields.as_ref())
            .ok_or("Kabuto effect owner absent")?
            .dispatch_context;
        let actor = self.get(id).ok_or("Kabuto effect absent")?;
        if !matches!(actor.class, 866 | 980)
            || actor.seed.storage.state() != 0
            || !actor.acquired_construction
            || actor.affine_cache.is_some()
        {
            return Err("Kabuto effect requires its cold reset constructor".into());
        }
        let actor = self.residents.get_mut(&id.serial).unwrap();
        let fields = actor.construction_fields.as_mut().unwrap();
        fields.dispatch_context = context;
        fields.configuration[14] |= 0xe00;
        fields.object_flags |= 0x40010000;
        actor.affine_cache = Some(super::OwnedAffine {
            value: crate::actor_affine::Cache {
                flags: 0,
                matrix: crate::procedural_aim::Matrix::identity().values,
            },
            pose: super::affine_pose_words(fields),
            external_orientation: true,
        });
        Ok(())
    }
    pub(crate) fn finish_kabuto_group(
        &mut self,
        id: Id,
        fields: crate::actor_fields::Fields,
        payload: Vec<u8>,
        group: crate::kabuto_initial::Group,
    ) -> Result<(), String> {
        let actor = self.get(id).ok_or("Kabuto group owner absent")?;
        if actor.class != 1
            || !actor.kabuto_player_fields
            || actor.kabuto_group.is_some()
            || actor
                .seed
                .storage
                .data()
                .is_none_or(|p| p.len() != payload.len())
            || payload.len() != 844
        {
            return Err("Kabuto group owner is not a fresh cold player".into());
        }
        let actor = self.residents.get_mut(&id.serial).unwrap();
        actor
            .seed
            .storage
            .data_mut()
            .unwrap()
            .copy_from_slice(&payload);
        actor.channels = Some(fields.channels);
        actor.construction_fields = Some(fields);
        actor.kabuto_group = Some(group);
        Ok(())
    }
    pub fn has_cold_kabuto_player_fields(&self, id: Id) -> bool {
        self.get(id).is_some_and(|a| a.kabuto_player_fields)
    }
    pub(crate) fn initialize_cold_kabuto_player_fields(
        &mut self,
        owner: Id,
        resources: &Resources,
        frame: ColdFrame,
        blending: Blending,
    ) -> Result<Initialized, String> {
        let actor = self
            .get(owner)
            .ok_or("Kabuto player owner absent or foreign")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("Kabuto player fields absent")?;
        if actor.class != 1
            || !actor.acquired_construction
            || actor.kabuto_player_fields
            || actor.seed.storage.profile() != Profile::Debug
            || actor.seed.storage.state() != 1
            || actor.seed.storage.data() != resources.state_template().storage.data()
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || actor.controlled_motion.is_some()
            || actor.ordinary_motion.is_some()
            || actor.environment.is_some()
            || actor.finalization.is_some()
            || actor.world_tick.is_some()
            || actor.world_tick_running
            || actor.affine_cache.is_some()
            || actor.mecc_physics_present
            || actor.mecc_animation_stamp != 0
            || actor.channels != Some(fields.channels)
            || fields.channels != resources.class_fields().channels
            || !self
                .specification(owner)
                .is_some_and(|s| Arc::ptr_eq(s, &resources.specification))
            || !self
                .state_resources(owner)
                .is_some_and(|s| Arc::ptr_eq(s, &resources.states))
            || !resources.specification.animations.contains_key(&1)
            || !self.specifications.get(&owner.serial).is_some_and(|s| {
                !s.0.sampled()
                    && s.0
                        .matrices()
                        .iter()
                        .all(|m| m.flags == 0 && m.values.iter().all(|v| v.to_bits() == 0))
            })
        {
            return Err("Kabuto player fields require their fresh unparented cold constructor and resources".into());
        }
        let next =
            crate::kabuto_player::project(fields, frame, resources.class_fields().scale, blending)?;
        let actor = self.residents.get_mut(&owner.serial).unwrap();
        actor.channels = Some(next.channels);
        actor.construction_fields = Some(next);
        actor.kabuto_player_fields = true;
        Ok(Initialized {
            owner,
            camera_projection_reset_required: true,
        })
    }
}
