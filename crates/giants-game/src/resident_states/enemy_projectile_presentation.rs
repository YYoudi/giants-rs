//! Source-created state6 clock and affine consumers after actual flight.
//! Prototype PS2 search routes `object_animate_check__FP6OBJECTP9Animation` and
//! `object_apply_location` retain their separately recorded uncertain scope.
//! This does not acknowledge a whole actor frame, impact, FX or damage service.
use super::*;

impl Session {
    /// Consume exactly one current flight's animation/affine phases. Only the
    /// acquired event-free, reference-free class90/232 source profile is admitted.
    /// The composing world supplies its persistent ANM stamp, never a made-up
    /// vertex epoch. Duplicate, skipped and failed passes cannot replay clocks.
    pub fn advance_enemy_projectile_presentation(
        &mut self,
        id: Id,
        stamp: &mut crate::animation_epoch::Stamp,
    ) -> Result<crate::enemy_projectile::Presentation, String> {
        use crate::enemy_projectile::{Phase, Presentation};
        let actor = self.get(id).ok_or("projectile presentation owner absent")?;
        let meta = actor
            .enemy_projectile
            .as_ref()
            .ok_or("projectile presentation constructor absent")?;
        let launch = meta.launch.as_ref().ok_or("projectile launch absent")?;
        let revision = meta.flight_revision;
        if !actor.acquired_construction
            || !matches!(actor.class, 90 | 232)
            || actor.seed.storage.profile() != Profile::Debug
            || actor.seed.storage.state() != 6
            || meta.phase != Phase::Launched
            || meta.initial_environment.is_none()
            || meta.pending_flight.is_some()
            || meta.presentation_failure.is_some()
            || revision == 0
            || meta.presentation.map_or(0, |v| v.flight_revision) != revision - 1
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || self.get(launch.source).is_none()
        {
            return Err("projectile presentation requires the next successful live flight".into());
        }
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("projectile presentation fields absent")?;
        if fields.class != actor.class
            || fields.object_flags & 0xc000 != 0
            || fields.response_flags & 0x80000000 != 0
        {
            return Err("projectile presentation requires current initialized fields".into());
        }
        let specification = self
            .specification(id)
            .ok_or("projectile presentation specification absent")?
            .clone();
        // These actual two specifications declare no references or events. Do
        // not replace a newly required event service with successful emptiness.
        if !specification.metadata.references.names.is_empty()
            || specification
                .metadata
                .events
                .iter()
                .any(|track| !track.events.is_empty())
        {
            return Err("projectile presentation event/reference consumer required".into());
        }
        let dt = meta
            .animation_seconds
            .ok_or("projectile flight clock absent")?;
        let mut channels = actor.channels.ok_or("projectile channels absent")?;
        let mut durations = vec![None; 256];
        for (&slot, descriptor) in &specification.animations {
            durations[usize::from(slot)] = Some(descriptor.template.duration);
        }
        let tick = crate::animation_channels::advance(
            &mut channels,
            &durations,
            launch.rates,
            dt,
            crate::arithmetic::Precision::Double53,
        )?;
        if tick.missing != 0 {
            return Err("projectile source animation descriptor absent".into());
        }
        let epoch = if tick.processed != 0 {
            Some(stamp.advance())
        } else {
            meta.presentation.and_then(|v| v.epoch)
        };
        // Preserve actual clock and epoch stores if a later cache consumer fails.
        let actor = self.residents.get_mut(&id.serial).unwrap();
        actor.channels = Some(channels);
        actor.construction_fields.as_mut().unwrap().channels = channels;
        let result: Result<(), String> = (|| {
            if tick.processed != 0 {
                self.specifications
                    .get_mut(&id.serial)
                    .unwrap()
                    .0
                    .update_current(&channels, epoch.unwrap())?;
            }
            self.prepare_actor_affine(id)
        })();
        let meta = self
            .residents
            .get_mut(&id.serial)
            .unwrap()
            .enemy_projectile
            .as_mut()
            .unwrap();
        if let Err(error) = result {
            meta.presentation_failure = Some(error.clone());
            return Err(error);
        }
        let completed = Presentation {
            flight_revision: revision,
            epoch,
        };
        meta.presentation = Some(completed);
        Ok(completed)
    }

    /// Validate the exact current flight/pose before copying animated vertices.
    pub fn enemy_projectile_presentation(
        &self,
        id: Id,
    ) -> Result<crate::enemy_projectile::Presentation, String> {
        let meta = self.enemy_projectile(id).ok_or("projectile owner absent")?;
        let frame = meta
            .presentation
            .ok_or("projectile clock/affine not consumed")?;
        if meta.phase != crate::enemy_projectile::Phase::Launched
            || meta.pending_flight.is_some()
            || meta.presentation_failure.is_some()
            || frame.flight_revision != meta.flight_revision
        {
            return Err("projectile presentation is stale or failed".into());
        }
        self.actor_current_affine(id)?;
        Ok(frame)
    }
}
