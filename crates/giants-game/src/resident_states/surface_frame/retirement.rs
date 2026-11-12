//! Bounded marker retirement. Prototype PS2 `object_destroy` /
//! `object_free_all_allocated_data` are search candidates, not confirmed names
//! for PC debug-1.1 0x0049de20/0x004a08d0/0x004a05d0. Raw aliases remain private
//! and in the reviewed navigation index; no prototype ABI/layout is imported.
use super::*;

impl Session {
    fn surface_cleanup_preflight(
        &self,
        id: Id,
        started: bool,
        owner: Id,
    ) -> Result<Counts, String> {
        let actor = self.surface_frame_actor(id)?;
        let frame = actor
            .surface_frame
            .as_ref()
            .ok_or("surface retirement frame absent")?;
        let fields = actor.construction_fields.as_ref().unwrap();
        let Some(SurfaceLink::Marker {
            owner: actual,
            mode: 2,
            ..
        }) = actor.surface_link
        else {
            return Err("surface retirement requires actual closing completion".into());
        };
        if actual != owner
            || frame.view.cleanup_started != started
            || frame.view.failure.is_some()
            || frame.view.next_frame == 0
            || frame.history.is_some()
            || actor.world_tick_running
            || actor
                .world_tick
                .as_ref()
                .is_none_or(|tick| tick.phase() != crate::actor_world_tick::Phase::Finished)
            || fields.object_flags & 0x8000 == 0
            || actor.seed.storage.data().is_some()
            || self.declarations.values().any(|&declared| declared == id)
            || self.specification(id).is_none()
            || self.state_resources(id).is_none()
            || self
                .specification(id)
                .unwrap()
                .metadata
                .events
                .iter()
                .any(|track| !track.events.is_empty())
        {
            return Err(
                "surface retirement requires completed marked eventless state48 ownership".into(),
            );
        }
        let parent = self.ordinary_frame_actor(owner)?;
        let combat = self.state23_snapshot(owner)?;
        if parent.world_tick_running
            || parent.ripper_primary_running
            || self.actor_motion(owner)?.awaiting_motion()
            || parent.ripper_primary.as_ref().is_some_and(|primary| {
                primary.awaiting_motion()
                    || primary.surface_marker().is_some_and(|marker| marker != id)
                    || primary.state().retaliation == Some(id)
            })
            || [
                combat.parent,
                combat.retaliation,
                combat.target.map(|v| v.id),
                combat.group,
                combat.effect,
            ]
            .contains(&Some(id))
        {
            return Err(
                "surface retirement owner still has a running or unrelated reference".into(),
            );
        }
        let incoming = self.incoming_actor_references(id)?;
        if (!started
            && (parent.surface_link != Some(SurfaceLink::Owner { marker: id })
                || combat.owner != Some((id, 219))
                || incoming != [owner]))
            || (started
                && (parent.surface_link.is_some()
                    || combat.owner.is_some()
                    || !incoming.is_empty()
                    || parent
                        .ripper_primary
                        .as_ref()
                        .is_some_and(|primary| primary.surface_marker().is_some())))
        {
            return Err("surface retirement requires only its acquired callback relation".into());
        }
        let links = self.links.memberships(id.serial);
        if links.len() != 3
            || links
                .iter()
                .map(|link| link.1)
                .collect::<std::collections::BTreeSet<_>>()
                != [0, 1, 5].into_iter().collect()
        {
            return Err("surface retirement registrations differ from constructor mask11".into());
        }
        Ok(Counts {
            residents: 1,
            payload_bytes: 0,
            registrations: links.len(),
        })
    }

    /// Clear the witnessed marked callback reference before invoking real
    /// external release, then unlink/drop this marker only after revalidation.
    /// This admits no global cascade, source death or invented release provider.
    pub fn cleanup_surface_marker(
        &mut self,
        id: Id,
        services: &mut impl crate::actor_retirement::Services,
    ) -> Result<Counts, String> {
        self.cleanup_surface_markers(&[id], services)
    }

    /// Within this admitted cohort, clear every marked reference before any
    /// external release and retain actual common-list retirement order.
    pub fn cleanup_surface_markers(
        &mut self,
        ids: &[Id],
        services: &mut impl crate::actor_retirement::Services,
    ) -> Result<Counts, String> {
        let selected = ids
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        if selected.len() != ids.len() {
            return Err("duplicate surface retirement identity".into());
        }
        let ordered = self
            .list(0)
            .into_iter()
            .filter(|id| selected.contains(id))
            .collect::<Vec<_>>();
        if ordered.len() != ids.len() {
            return Err("surface retirement owner absent or foreign".into());
        }
        let prepared = ordered
            .into_iter()
            .map(|id| {
                let Some(SurfaceLink::Marker { owner, .. }) = self.surface_link(id) else {
                    return Err("surface retirement requires an owned callback child".into());
                };
                self.surface_cleanup_preflight(id, false, owner)
                    .map(|counts| (id, owner, counts))
            })
            .collect::<Result<Vec<_>, String>>()?;
        for &(id, _, _) in &prepared {
            self.residents
                .get_mut(&id.serial)
                .unwrap()
                .surface_frame
                .as_mut()
                .unwrap()
                .view
                .cleanup_started = true;
        }
        let result: Result<Counts, String> = (|| {
            // PC's common reference pass clears +274 when its referent has
            // bit15, before the later object-data and registration release.
            for &(id, owner, _) in &prepared {
                let parent = self.residents.get_mut(&owner.serial).unwrap();
                parent.surface_link = None;
                parent
                    .state23_combat
                    .as_mut()
                    .ok_or("surface callback combat owner absent")?
                    .owner = None;
                if let Some(primary) = &mut parent.ripper_primary {
                    primary.forget_retired_surface_marker(id)?;
                }
            }
            for &(id, owner, _) in &prepared {
                self.surface_cleanup_preflight(id, true, owner)?;
            }
            for &(id, owner, expected) in &prepared {
                services.release(self, id)?;
                self.surface_cleanup_preflight(id, true, owner)?;
                if self.links.remove_object(id.serial) != expected.registrations {
                    return Err("surface retirement registrations changed".into());
                }
                self.specifications
                    .remove(&id.serial)
                    .ok_or("surface retirement specification lost")?;
                self.state_resources
                    .remove(&id.serial)
                    .ok_or("surface retirement state resources lost")?;
                self.residents
                    .remove(&id.serial)
                    .ok_or("surface retirement actor lost")?;
            }
            Ok(Counts {
                residents: prepared.len(),
                payload_bytes: 0,
                registrations: prepared
                    .iter()
                    .map(|(_, _, counts)| counts.registrations)
                    .sum(),
            })
        })();
        if let Err(error) = &result {
            for (id, _, _) in prepared {
                if let Some(actor) = self.residents.get_mut(&id.serial) {
                    actor.surface_frame.as_mut().unwrap().view.failure = Some(error.clone());
                }
            }
        }
        result
    }
}
