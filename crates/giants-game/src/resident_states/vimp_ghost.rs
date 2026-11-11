use super::*;
impl Session {
    pub(crate) fn construct_vimp_ghost(
        &mut self,
        input: Construction<'_>,
        metadata: crate::vimp_ghost::Owned,
    ) -> Result<Id, String> {
        if input.class != 859
            || input.seed.storage.profile() != Profile::Debug
            || input.seed.storage.state() != 78
            || input.seed.storage.data().is_none_or(|d| d.len() != 268)
            || crate::object_lists::membership(input.class, 78, input.flags) != 0
            || metadata.number != 0
            || metadata.failure.is_some()
            || self.residents.values().any(|a| {
                a.vimp_ghost
                    .as_ref()
                    .is_some_and(|m| m.origin == metadata.origin)
            })
            || input
                .specification
                .metadata
                .events
                .iter()
                .any(|t| !t.events.is_empty())
            || input.fields.channels.iter().any(|c| c[4] != 0 || c[5] != 0)
        {
            return Err("ghost birth outside acquired unlisted event-free class859 profile".into());
        }
        let mut staged = Self::new(self.ticket);
        staged.next = self.next;
        let id =
            staged.register_with_mask(input.class, input.seed, input.bindings, input.flags, 0)?;
        staged.bind_specification(id, input.specification)?;
        staged.bind_state_resources(id, input.state_resources)?;
        let fields = input.fields;
        let channels = fields.channels;
        let cache = OwnedAffine {
            value: crate::actor_affine::Cache {
                flags: metadata.matrix.flags,
                matrix: metadata.matrix.values,
            },
            pose: affine_pose_words(&fields),
            external_orientation: true,
        };
        staged.bind_actor_fields(id, fields)?;
        let actor = staged.residents.get_mut(&id.serial).unwrap();
        actor.channels = Some(channels);
        actor.affine_cache = Some(cache);
        actor.vimp_ghost = Some(metadata);
        actor.acquired_construction = true;
        self.publish_staged(staged, id)
    }
    pub fn resume_vimp_ghost(
        &mut self,
        id: Id,
        contexts: &mut crate::attached_effect_context::Pool<Id>,
    ) -> Result<(), String> {
        let actor = self.get(id).ok_or("ghost resume actor absent or foreign")?;
        let metadata = actor
            .vimp_ghost
            .as_ref()
            .ok_or("ghost source ownership absent")?;
        if metadata.failure.is_some() || metadata.cleanup_started || !metadata.state.waiting() {
            return Err("ghost resume failed, repeated or already retiring".into());
        }
        let context = contexts
            .get(metadata.origin)
            .ok_or("ghost retained context absent or stale")?;
        let recipe =
            crate::vimp_ghost::Recipe::project(context.program.as_ref().map_err(Clone::clone)?)?;
        if context.flags() != 0x20000 || recipe.continuation != metadata.recipe.continuation {
            return Err("ghost source continuation changed before resume".into());
        }
        let origin = metadata.origin;
        let life = metadata.recipe.ending;
        let mut state = metadata.state.clone();
        state.resume()?;
        let actor = self.residents.get_mut(&id.serial).unwrap();
        let fields = actor.construction_fields.as_mut().unwrap();
        fields.control_flags = state.control_flags();
        let data = actor.seed.storage.data_mut().unwrap();
        data[4..8].copy_from_slice(&life.lifetime.to_le_bytes());
        data[8..12].copy_from_slice(&0f32.to_le_bytes());
        let metadata = actor.vimp_ghost.as_mut().unwrap();
        metadata.state = state;
        // Resume executes the deferred lifetime commands and terminates the
        // context; the effect actor remains alive for its ending interval.
        contexts.complete_positional_impact(origin)?;
        Ok(())
    }
    pub fn advance_vimp_ghost(
        &mut self,
        id: Id,
        number: u64,
        seconds: f32,
        stamp: &mut crate::animation_epoch::Stamp,
    ) -> Result<crate::effect_visual::Step, String> {
        let actor = self
            .get(id)
            .ok_or("ghost callback actor absent or foreign")?;
        let metadata = actor
            .vimp_ghost
            .as_ref()
            .ok_or("ghost callback ownership absent")?;
        let fields = actor.construction_fields.as_ref().unwrap();
        if metadata.failure.is_some()
            || metadata.cleanup_started
            || metadata.number != number
            || number == u64::MAX
            || fields.object_flags & 0xc000 != 0
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || actor.class != 859
            || actor.seed.storage.state() != 78
        {
            return Err("ghost callback phase or owner differs".into());
        }
        let mut metadata = metadata.clone();
        let mut fields = fields.clone();
        let mut channels = actor.channels.ok_or("ghost channels absent")?;
        let specification = self.specification(id).unwrap().clone();
        let result = (|| -> Result<crate::effect_visual::Step, String> {
            let step = metadata.state.advance(seconds)?;
            crate::effect_actor::callback_entry(&mut fields, 0);
            fields.control_flags = metadata.state.control_flags();
            if step == crate::effect_visual::Step::Expired {
                fields.object_flags |= 0x8000;
            } else {
                if step == crate::effect_visual::Step::VisibleState {
                    fields.scale = metadata.state.size;
                    metadata.matrix = crate::effect_orientation::matrix(
                        fields.position,
                        fields.angles,
                        fields.scale,
                    )?;
                }
                let mut durations = vec![None; 256];
                for (&slot, descriptor) in &specification.animations {
                    durations[usize::from(slot)] = Some(descriptor.template.duration);
                }
                let tick = crate::animation_channels::advance(
                    &mut channels,
                    &durations,
                    [],
                    seconds,
                    crate::arithmetic::Precision::Double53,
                )?;
                if tick.missing != 0 {
                    return Err("ghost current ANM descriptor absent".into());
                }
                if tick.processed != 0 {
                    metadata.epoch = stamp.advance();
                    self.specifications
                        .get_mut(&id.serial)
                        .unwrap()
                        .0
                        .update_current(&channels, metadata.epoch)?;
                }
            }
            metadata.number += 1;
            Ok(step)
        })();
        if let Err(error) = &result {
            metadata.failure = Some(error.clone());
        }
        fields.channels = channels;
        let actor = self.residents.get_mut(&id.serial).unwrap();
        let data = actor.seed.storage.data_mut().unwrap();
        data[8..12].copy_from_slice(&metadata.state.age().to_le_bytes());
        actor.affine_cache = Some(OwnedAffine {
            value: crate::actor_affine::Cache {
                flags: metadata.matrix.flags,
                matrix: metadata.matrix.values,
            },
            pose: affine_pose_words(&fields),
            external_orientation: true,
        });
        actor.channels = Some(channels);
        actor.construction_fields = Some(fields);
        actor.vimp_ghost = Some(metadata);
        result
    }
    pub fn vimp_ghost_draw(&self, id: Id) -> Result<crate::vimp_ghost::Draw, String> {
        let actor = self.get(id).ok_or("ghost draw actor absent or foreign")?;
        let metadata = actor.vimp_ghost.as_ref().ok_or("ghost draw owner absent")?;
        if metadata.failure.is_some()
            || metadata.number == 0
            || metadata.cleanup_started
            || actor.construction_fields.as_ref().unwrap().object_flags & 0xc000 != 0
        {
            return Err("ghost draw requires a completed live callback".into());
        }
        Ok(crate::vimp_ghost::Draw {
            number: metadata.number,
            epoch: metadata.epoch,
            matrix: metadata.matrix,
            transparency: metadata.state.transparency,
        })
    }
    pub fn cleanup_vimp_ghost(
        &mut self,
        id: Id,
        services: &mut impl crate::actor_retirement::Services,
    ) -> Result<Counts, String> {
        let preflight = |session: &Self, started: bool| -> Result<Counts, String> {
            let actor = session
                .get(id)
                .ok_or("ghost cleanup actor absent or foreign")?;
            let metadata = actor
                .vimp_ghost
                .as_ref()
                .ok_or("ghost cleanup owner absent")?;
            if metadata.failure.is_some()
                || metadata.cleanup_started != started
                || metadata.state.waiting()
                || actor.construction_fields.as_ref().unwrap().object_flags & 0x8000 == 0
                || actor.attachment.is_some()
                || actor.presentation_parent.is_some()
            {
                return Err("ghost cleanup requires its resumed expired unparented owner".into());
            }
            if session.declarations.values().any(|&value| value == id)
                || !session.incoming_actor_references(id)?.is_empty()
                || session.links.memberships(id.serial).len() != 1
                || session.links.memberships(id.serial)[0].1 != 0
            {
                return Err(
                    "ghost retirement still has references or different registrations".into(),
                );
            }
            Ok(Counts {
                residents: 1,
                payload_bytes: 268,
                registrations: 1,
            })
        };
        let expected = preflight(self, false)?;
        self.residents
            .get_mut(&id.serial)
            .unwrap()
            .vimp_ghost
            .as_mut()
            .unwrap()
            .cleanup_started = true;
        let result: Result<Counts, String> = (|| {
            services.release(self, id)?;
            preflight(self, true)?;
            self.residents.remove(&id.serial);
            self.state_resources.remove(&id.serial);
            self.specifications.remove(&id.serial);
            if self.links.remove_object(id.serial) != expected.registrations {
                return Err("ghost unlink registrations changed".into());
            }
            Ok(expected)
        })();
        if let Err(error) = &result
            && let Some(metadata) = self
                .residents
                .get_mut(&id.serial)
                .and_then(|a| a.vimp_ghost.as_mut())
        {
            metadata.failure = Some(error.clone());
        }
        result
    }
}
