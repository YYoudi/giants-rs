//! Bounded common retirement after the actual Ripper corpse callback.
//! Prototype PS2 `object_destroy` / `object_free_all_allocated_data` remain
//! unknown navigation candidates for the acquired PC common cleanup. State23
//! has no state destructor. No prototype layout or signature is imported.
use super::*;

#[derive(Debug, Default)]
pub(super) struct Owned {
    pub(super) marked: bool,
    attempt: Option<Arc<()>>,
    failure: Option<String>,
}

#[cfg(test)]
mod tests;

impl Session {
    fn ripper_cleanup_preflight(
        &self,
        id: Id,
        key: crate::declared_residents::Key,
        attempt: Option<&Arc<()>>,
    ) -> Result<Counts, String> {
        self.check_ripper_damage_available(id)?;
        let actor = self
            .get(id)
            .ok_or("Ripper retirement owner absent or foreign")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("Ripper retirement fields absent")?;
        let combat = self.state23_snapshot(id)?;
        let owned = &actor.ripper_retirement;
        let same_attempt = match (&owned.attempt, attempt) {
            (None, None) => true,
            (Some(current), Some(expected)) => Arc::ptr_eq(current, expected),
            _ => false,
        };
        if !matches!(actor.class, 73 | 230)
            || actor.seed.storage.profile() != Profile::Debug
            || actor.seed.storage.state() != 23
            || actor
                .seed
                .storage
                .data()
                .is_none_or(|data| data.len() != 88)
            || !actor.acquired_construction
            || !owned.marked
            || owned.failure.is_some()
            || !same_attempt
            || combat.mode != 94
            || !combat.damage.is_finite()
            || !combat.limit.is_finite()
            || combat.limit <= 0.
            || combat.damage < combat.limit
            || fields.object_flags & 0x8000 == 0
            || actor.world_tick_running
            || actor.ripper_primary_running
            || actor
                .ripper_primary
                .as_ref()
                .is_none_or(|p| p.failure().is_some() || p.state().effect.is_some())
            || actor
                .world_tick
                .as_ref()
                .is_none_or(|t| t.phase() != crate::actor_world_tick::Phase::Finished)
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || actor.surface_link.is_some()
            || combat.parent.is_some()
            || combat.owner.is_some()
            || combat.group.is_some()
            || combat.effect.is_some()
            || self.specification(id).is_none()
            || self.state_resources(id).is_none()
            || self.declaration(key) != Some(id)
            || self.retired_declaration(key).is_some()
            || self
                .declarations
                .values()
                .filter(|&&value| value == id)
                .count()
                != 1
        {
            return Err("Ripper retirement requires its witnessed completed unparented corpse and unchanged declaration".into());
        }
        // The common reference pass is not silently replaced by deletion. This
        // bounded path admits no remaining incoming relation or outgoing marked
        // referent. Other cleanup/cascade consumers must acquire those writes.
        if !self.incoming_actor_references(id)?.is_empty() {
            return Err("Ripper corpse still has incoming actor references".into());
        }
        for reference in [combat.retaliation, combat.target.map(|t| t.id)]
            .into_iter()
            .flatten()
        {
            if self.actor_world_flags(reference)? & 0x8000 != 0 {
                return Err("Ripper corpse requires marked-reference clearing".into());
            }
        }
        let links = self.links.memberships(id.serial);
        if links.len() != 1 + actor.constructor_mask.count_ones() as usize
            || links.iter().filter(|link| link.1 == 0).count() != 1
            || links
                .iter()
                .any(|link| link.1 != 0 && actor.constructor_mask & (1 << (link.1 - 1)) == 0)
        {
            return Err("Ripper retirement registrations differ from source construction".into());
        }
        Ok(Counts {
            residents: 1,
            payload_bytes: 88,
            registrations: links.len(),
        })
    }

    /// Read-only preflight; rejection runs no external release and grants no
    /// retirement. Pending physical/ANM phases are not declared complete: the
    /// source bit15 path intentionally skips them before common core release.
    pub fn check_dead_ripper_retirement(
        &self,
        id: Id,
        key: crate::declared_residents::Key,
    ) -> Result<Counts, String> {
        self.ripper_cleanup_preflight(id, key, None)
    }

    /// Release real external owners, revalidate the exact attempt, then drop
    /// source data/specification ownership and unlink all constructor lists.
    /// A failed external release retains its prefix and cannot be replayed.
    pub fn cleanup_dead_ripper(
        &mut self,
        id: Id,
        key: crate::declared_residents::Key,
        services: &mut impl crate::actor_retirement::Services,
    ) -> Result<Counts, String> {
        let expected = self.ripper_cleanup_preflight(id, key, None)?;
        let attempt = Arc::new(());
        self.residents
            .get_mut(&id.serial)
            .unwrap()
            .ripper_retirement
            .attempt = Some(attempt.clone());
        let result: Result<Counts, String> = (|| {
            services.release(self, id)?;
            let after = self.ripper_cleanup_preflight(id, key, Some(&attempt))?;
            if after != expected {
                return Err("Ripper retirement ownership changed during external release".into());
            }
            self.residents.remove(&id.serial).unwrap();
            self.specifications.remove(&id.serial).unwrap();
            self.state_resources.remove(&id.serial).unwrap();
            assert_eq!(self.links.remove_object(id.serial), expected.registrations);
            assert_eq!(self.declarations.remove(&key), Some(id));
            assert!(self.retired_declarations.insert(key, id).is_none());
            Ok(expected)
        })();
        if let Err(error) = &result
            && let Some(actor) = self.residents.get_mut(&id.serial)
            && actor
                .ripper_retirement
                .attempt
                .as_ref()
                .is_some_and(|a| Arc::ptr_eq(a, &attempt))
        {
            actor.ripper_retirement.failure = Some(error.clone());
        }
        result
    }
}
