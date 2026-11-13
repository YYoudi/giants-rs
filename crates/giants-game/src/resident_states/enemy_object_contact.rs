//! Positive class90/232 object contact. Prototype PS2 `hurt_object` remains an
//! unknown correspondence; exact PC identities/old aliases stay private.
use super::*;
use crate::{
    audio_parameters::Random,
    enemy_projectile::{
        FlightOutcome, ObjectContactOutcome, ObjectImpact, ObjectImpactServices, Phase,
    },
};
impl Session {
    /// Exact no-effect case of the post-hit counter/terminal tail. This reads
    /// actual target flags/role/health, not default world guards or a fabricated
    /// mission success. A possible terminal still needs the real mission owner.
    pub fn enemy_object_terminal_no_effect(&self, source: Id, target: Id) -> Result<bool, String> {
        let actor = self
            .get(source)
            .ok_or("terminal source absent or foreign")?;
        let meta = actor
            .enemy_projectile
            .as_ref()
            .ok_or("terminal enemy constructor absent")?;
        if meta.phase != Phase::ObjectImpactServices
            || !matches!(meta.pending_flight,Some(FlightOutcome::ObjectContact {target:actual,..})if actual==target)
        {
            return Err("terminal proof requires its actual active object continuation".into());
        }
        let actor = self
            .get(target)
            .ok_or("terminal target absent or foreign")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("terminal target fields absent")?;
        if fields.object_flags & 0x400 != 0 {
            return Ok(false);
        }
        if fields.response_flags & 15 != 2 {
            return Ok(true);
        }
        if fields.object_flags & 0x8000 != 0 {
            return Ok(false);
        }
        let health = self
            .radial_actor(target)?
            .health
            .ok_or("terminal proof needs actual target health")?;
        if !health.into_iter().all(f32::is_finite) {
            return Err("terminal health outside finite profile".into());
        }
        Ok(health[0] < health[1])
    }
    /// Consume the actual retained actor contact without moving/requerying the
    /// projectile, inventing a shooter or importing the ground lifetime tail.
    /// Current copied class90/232 does not request the shell's explosion here.
    pub fn complete_enemy_projectile_object_contact(
        &mut self,
        id: Id,
        random: &mut Random,
        services: &mut impl ObjectImpactServices,
    ) -> Result<ObjectContactOutcome, String> {
        let actor = self
            .get(id)
            .ok_or("object-contact source absent or foreign")?;
        let metadata = actor
            .enemy_projectile
            .as_ref()
            .ok_or("enemy constructor absent")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("object-contact source fields absent")?;
        let Some(FlightOutcome::ObjectContact { target, point, tag }) = metadata.pending_flight
        else {
            return Err("object reaction requires the actual pending actor contact".into());
        };
        let owner = metadata
            .launch
            .as_ref()
            .and_then(|l| self.get(l.source))
            .ok_or("object-contact Ripper owner absent")?;
        let owner_fields = owner
            .construction_fields
            .as_ref()
            .ok_or("object-contact owner fields absent")?;
        let amount = f32::from_bits(fields.configuration[12]);
        let source_class = actor.class;
        if !actor.acquired_construction
            || actor.seed.storage.profile() != Profile::Debug
            || !matches!(actor.class, 90 | 232)
            || actor.seed.storage.state() != 6
            || actor.seed.storage.data().is_none_or(|v| v.len() != 36)
            || metadata.resource_class != actor.class
            || fields.class != actor.class
            || fields.configuration[20] != 6
            || metadata.phase != Phase::Launched
            || metadata.failure.is_some()
            || metadata.ground_impact.is_some()
            || metadata.object_impact.is_some()
            || metadata.ground_flight_clock.is_some()
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || fields.object_flags & 0xc000 != 0
            || !amount.is_finite()
            || amount <= 0.
            || !point.into_iter().all(f32::is_finite)
            || !owner.acquired_construction
            || owner.seed.storage.profile() != Profile::Debug
            || !matches!(owner.class, 73 | 230)
            || owner.seed.storage.state() != 23
            || owner_fields.class != owner.class
            || owner_fields.configuration[20] != 23
        {
            return Err(
                "object contact outside acquired positive unparented enemy-shot profile".into(),
            );
        }
        let target_actor = self
            .get(target)
            .ok_or("object-contact target absent or foreign")?;
        let target_fields = target_actor
            .construction_fields
            .as_ref()
            .ok_or("object-contact target fields absent")?;
        if !target_actor.acquired_construction
            || target_actor.seed.storage.profile() != Profile::Debug
            || target_actor.attachment.is_some()
            || target_actor.presentation_parent.is_some()
            || target_fields.object_flags & 0x400 != 0
            || target_fields.class != target_actor.class
            || target_fields.configuration[20] != u32::from(target_actor.seed.storage.state())
        {
            return Err(
                "object contact needs acquired unparented non-objective target ownership".into(),
            );
        }
        let secondary_immune = target_fields.configuration[15] & 0x4000000 != 0;
        let callback_disabled = target_fields.configuration[14] & 0x4000000 != 0;
        let request = ObjectImpact {
            source: id,
            target,
            point,
            tag,
            amount,
        };
        self.residents
            .get_mut(&id.serial)
            .unwrap()
            .enemy_projectile
            .as_mut()
            .unwrap()
            .phase = Phase::ObjectImpactServices;
        let result: Result<ObjectContactOutcome, String> = (|| {
            if services.world_suppressed(self)? || secondary_immune {
                return Ok(ObjectContactOutcome::Retained);
            }
            let outcome = if callback_disabled {
                ObjectContactOutcome::Retained
            } else {
                services.hurt(self, request, random)?;
                let source = self
                    .residents
                    .get_mut(&id.serial)
                    .ok_or("direct callback removed its source")?;
                if source.class != source_class
                    || source.seed.storage.state() != 6
                    || source
                        .enemy_projectile
                        .as_ref()
                        .is_none_or(|m| m.phase != Phase::ObjectImpactServices)
                {
                    return Err("direct callback changed source ownership".into());
                }
                let fields = source
                    .construction_fields
                    .as_mut()
                    .ok_or("direct callback removed source fields")?;
                if source.attachment.is_some() || source.presentation_parent.is_some() {
                    return Err("direct callback parented source".into());
                }
                fields.object_flags |= 0x8000;
                ObjectContactOutcome::Marked
            };
            services.terminal(self, target)?;
            let source = self.get(id).ok_or("terminal callback removed its source")?;
            if source.class != source_class
                || source.seed.storage.state() != 6
                || source.attachment.is_some()
                || source.presentation_parent.is_some()
                || source.construction_fields.as_ref().is_none_or(|f| {
                    f.class != source_class
                        || f.configuration[20] != 6
                        || (outcome == ObjectContactOutcome::Marked && f.object_flags & 0x8000 == 0)
                })
                || source.enemy_projectile.as_ref().is_none_or(|m| {
                    m.phase != Phase::ObjectImpactServices
                        || m.failure.is_some()
                        || m.ground_impact.is_some()
                        || m.object_impact.is_some()
                        || !matches!(m.pending_flight, Some(FlightOutcome::ObjectContact { target: actual, point: p, tag: t })
                            if actual == target && p == point && t == tag)
                })
            {
                return Err("terminal callback changed object-contact ownership".into());
            }
            Ok(outcome)
        })();
        match &result {
            Ok(outcome) => {
                let meta = self
                    .residents
                    .get_mut(&id.serial)
                    .and_then(|a| a.enemy_projectile.as_mut())
                    .ok_or("object source removed after services")?;
                meta.pending_flight = None;
                if *outcome == ObjectContactOutcome::Marked {
                    meta.object_impact = Some(request);
                    meta.phase = Phase::Retiring;
                } else {
                    meta.phase = Phase::Launched;
                }
            }
            Err(error) => {
                if let Some(meta) = self
                    .residents
                    .get_mut(&id.serial)
                    .and_then(|a| a.enemy_projectile.as_mut())
                {
                    meta.phase = Phase::Failed;
                    meta.failure = Some(error.clone());
                }
            }
        }
        result
    }
}
