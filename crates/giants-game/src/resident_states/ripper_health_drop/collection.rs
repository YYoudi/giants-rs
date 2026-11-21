//! Current controlled-Mecc collection continuation for actual health births.
//! Source identity and a computed contact are distinct from cue/controller/stat
//! acknowledgments. Late failures keep the committed prefix and cannot replay.
use super::*;
use crate::ripper_health_drop::{
    CollectionOutcome, CollectionServices, PendingCollection, PickupDefinition, PrimaryPhase,
    Statistic,
};

impl Session {
    pub(crate) fn finish_ripper_health_collection(
        &mut self,
        body: &Arc<crate::specification::Prepared>,
        pending: &PendingCollection,
        player: &crate::controlled_mecc_contacts::Snapshot,
        definition: PickupDefinition,
        services: &mut impl CollectionServices,
    ) -> Result<CollectionOutcome, String> {
        let item = pending.item;
        let step = pending.step;
        let actor = self
            .get(item)
            .ok_or("health collection owner absent or foreign")?;
        let owned = actor
            .ripper_health_drop
            .ok_or("health birth ownership absent")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("health collection fields absent")?;
        if owned.failed
            || !owned.initialized
            || actor.class != 785
            || actor.seed.storage.state() != 37
            || fields.configuration[20] != 37
            || owned.primary_phase != PrimaryPhase::CollectionPending
            || owned.primary_step != Some(step)
            || owned.primary_recipient.is_some()
            || !self
                .specification(item)
                .is_some_and(|s| Arc::ptr_eq(s, body))
            || player.step != step
            || owned.amount < 0
            || actor.seed.storage.data().is_none_or(|p| {
                p.len() != 24
                    || p[..4] != 36u32.to_le_bytes()
                    || p[4..8] != owned.amount.to_le_bytes()
                    || p[12..16] != [0; 4]
            })
        {
            return Err("health collection requires its unconsumed canonical prefix".into());
        }
        let motion = self.controlled_mecc_motion(player.owner, step)?;
        self.mecc_health(player.owner)?;
        let recipient = self
            .get(player.owner)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap();
        self.actor_current_affine(player.owner)?;
        if player.active != recipient.object_flags
            || player.personality != recipient.configuration[14]
            || motion.frame.position.map(f32::to_bits) != recipient.position.map(f32::to_bits)
        {
            return Err("health collection player packet is stale or foreign".into());
        }
        let metadata = self
            .residents
            .get_mut(&item.serial)
            .unwrap()
            .ripper_health_drop
            .as_mut()
            .unwrap();
        metadata.primary_phase = PrimaryPhase::Collecting;
        metadata.primary_recipient = Some(player.owner);
        let result = (|| {
            if !self.ripper_health_pickup_eligible(item, player.owner, step, definition)? {
                return Ok(CollectionOutcome::NotCollected);
            }
            let bounds = crate::volume_queries::Bounds {
                min: pending.bounds[..3].try_into().unwrap(),
                max: pending.bounds[3..].try_into().unwrap(),
            };
            if !crate::volume_queries::overlaps(player.bounds, bounds)
                .ok_or("health collection bounds are nonfinite")?
            {
                return Ok(CollectionOutcome::NotCollected);
            }
            let before = crate::controlled_mecc::HitState {
                health: self.mecc_health(player.owner)?,
                velocity: self
                    .controlled_mecc_motion(player.owner, step)?
                    .frame
                    .velocity,
            };
            services.local_cue(self, item, player.owner, 1596)?;
            self.validate_ripper_health_response(item, player.owner, step)?;
            let mut after = before;
            // The original stores integer quantity as f32 before subtraction.
            let damage = (f64::from(before.health.damage) - f64::from(owned.amount as f32)) as f32;
            after.health.damage = if damage < 0. { 0. } else { damage };
            self.residents
                .get_mut(&player.owner.serial)
                .unwrap()
                .mecc_damage = Some(after.health.damage);
            if !after.same_bits(before) {
                services.copy_health(
                    self,
                    crate::controlled_mecc::HitResponse {
                        source: item,
                        owner: player.owner,
                        step,
                        before,
                        after,
                        origin: crate::controlled_mecc::HitOrigin::RipperHealth,
                    },
                )?;
                self.validate_ripper_health_response(item, player.owner, step)?;
            }
            let flags = self
                .residents
                .get_mut(&item.serial)
                .unwrap()
                .construction_fields
                .as_mut()
                .unwrap();
            flags.object_flags |= 0x8000;
            let kind = if flags.control_flags & 2 != 0 {
                Statistic::Medium
            } else {
                Statistic::Large
            };
            services.statistic(self, player.owner, kind)?;
            self.validate_ripper_health_response(item, player.owner, step)?;
            Ok(CollectionOutcome::Collected)
        })();
        let metadata = self
            .residents
            .get_mut(&item.serial)
            .unwrap()
            .ripper_health_drop
            .as_mut()
            .unwrap();
        metadata.primary_recipient = None;
        if result.is_err() {
            metadata.failed = true;
            metadata.primary_phase = PrimaryPhase::Failed;
        } else {
            metadata.primary_phase = PrimaryPhase::Idle;
        }
        result
    }

    pub(crate) fn validate_ripper_health_response(
        &self,
        item: Id,
        recipient: Id,
        step: u64,
    ) -> Result<(), String> {
        let actor = self
            .get(item)
            .ok_or("health response source removed or foreign")?;
        let owned = actor
            .ripper_health_drop
            .ok_or("health response birth absent")?;
        if !owned.initialized
            || owned.failed
            || actor.class != 785
            || actor.seed.storage.state() != 37
            || owned.primary_step != Some(step)
            || owned.primary_phase != PrimaryPhase::Collecting
            || owned.primary_recipient != Some(recipient)
        {
            return Err("health response requires its actual running collection scope".into());
        }
        self.controlled_mecc_motion(recipient, step)?;
        self.mecc_health(recipient)?;
        Ok(())
    }
}
