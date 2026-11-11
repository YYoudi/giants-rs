//! Prototype PS2 search key: `vimp_cleanup`. This file owns bounded PC
//! retirement; exact PS2/PC attribution remains unknown.
//! See docs/SYMBOL_NAVIGATION.md.
use super::*;
impl Session {
    /// State80 has no payload or state destructor. Expired loose meat can be
    /// unlinked only after real external owners release it; collection/carrying
    /// uses its separate acquired lifecycle and is not acknowledged here.
    pub fn cleanup_expired_vimp_chunk(
        &mut self,
        id: Id,
        services: &mut impl crate::actor_retirement::Services,
    ) -> Result<Counts, String> {
        self.cleanup_retired_vimp_chunk(id, false, services)
    }
    /// Actual collection is a separate terminal reason. It must not simulate
    /// expiry by changing fade/timer or skip real external-owner release.
    /// Prototype PS2 search key: `vimpchunk_destroy`; exact attribution unknown.
    pub fn cleanup_collected_vimp_chunk(
        &mut self,
        id: Id,
        services: &mut impl crate::actor_retirement::Services,
    ) -> Result<Counts, String> {
        self.cleanup_retired_vimp_chunk(id, true, services)
    }
    fn cleanup_retired_vimp_chunk(
        &mut self,
        id: Id,
        collected: bool,
        services: &mut impl crate::actor_retirement::Services,
    ) -> Result<Counts, String> {
        let preflight = |s: &Self, started: bool| -> Result<Counts, String> {
            let actor = s.get(id).ok_or("meat retirement actor absent or foreign")?;
            let chunk = actor
                .vimp_chunk
                .ok_or("meat retirement source ownership absent")?;
            let fields = actor
                .construction_fields
                .as_ref()
                .ok_or("meat retirement fields absent")?;
            if !(1117..=1119).contains(&actor.class)
                || actor.seed.storage.state() != 80
                || actor.seed.storage.data().is_some()
                || !actor.acquired_construction
                || chunk.failed != started
                || chunk.collected != collected
                || (!collected && chunk.fade != 1.)
                || fields.object_flags & 0x8000 == 0
                || fields.control_flags & 2 != 0
                || actor.attachment.is_some()
                || actor.presentation_parent.is_some()
                || s.declarations.values().any(|&value| value == id)
                || !s.incoming_actor_references(id)?.is_empty()
                || s.links.memberships(id.serial).len() != 1
                || s.links.memberships(id.serial)[0].1 != 0
            {
                return Err(
                    "meat retirement requires its witnessed terminal reason and unreferenced loose source ownership".into(),
                );
            }
            Ok(Counts {
                residents: 1,
                payload_bytes: 0,
                registrations: 1,
            })
        };
        let expected = preflight(self, false)?;
        self.residents
            .get_mut(&id.serial)
            .unwrap()
            .vimp_chunk
            .as_mut()
            .unwrap()
            .failed = true;
        services.release(self, id)?;
        preflight(self, true)?;
        self.residents.remove(&id.serial);
        self.state_resources.remove(&id.serial);
        self.specifications.remove(&id.serial);
        if self.links.remove_object(id.serial) != expected.registrations {
            return Err("meat unlink registrations changed".into());
        }
        Ok(expected)
    }
    /// Ordinary completed lethal callback. The two native state-destructor
    /// reference slots must actually be empty in this bounded no-target profile.
    pub fn cleanup_dead_vimp(
        &mut self,
        id: Id,
        services: &mut impl crate::actor_retirement::Services,
    ) -> Result<Counts, String> {
        let preflight = |s: &Self, started: bool| -> Result<Counts, String> {
            let actor = s.get(id).ok_or("Vimp retirement actor absent or foreign")?;
            let vimp = actor
                .vimp
                .ok_or("Vimp retirement source ownership absent")?;
            let fields = actor
                .construction_fields
                .as_ref()
                .ok_or("Vimp retirement fields absent")?;
            let body = actor
                .seed
                .storage
                .data()
                .ok_or("Vimp retirement payload absent")?;
            let limit = f32::from_bits(fields.configuration[12]);
            if actor.class != 8
                || actor.seed.storage.state() != 21
                || !actor.acquired_construction
                || body.len() != 216
                || actor.vimp_frame_stopped != started
                || fields.object_flags & 0x8000 == 0
                || !vimp.damage.is_finite()
                || !limit.is_finite()
                || limit <= 0.
                || vimp.damage < limit
                || actor.world_tick_running
                || body[0x78..0x7c] != [0; 4]
                || body[0x8c..0x90] != [0; 4]
                || actor.attachment.is_some()
                || actor.presentation_parent.is_some()
                || s.declarations.values().any(|&value| value == id)
                || !s.incoming_actor_references(id)?.is_empty()
            {
                return Err("Vimp retirement requires successful death and acquired empty target references".into());
            }
            let links = s.links.memberships(id.serial);
            if links.len() != 1 + actor.constructor_mask.count_ones() as usize
                || links.iter().filter(|l| l.1 == 0).count() != 1
                || links
                    .iter()
                    .any(|l| l.1 != 0 && (actor.constructor_mask & (1 << (l.1 - 1)) == 0))
            {
                return Err(
                    "Vimp retirement registrations differ from its actual construction".into(),
                );
            }
            Ok(Counts {
                residents: 1,
                payload_bytes: 216,
                registrations: links.len(),
            })
        };
        let expected = preflight(self, false)?;
        self.residents
            .get_mut(&id.serial)
            .unwrap()
            .vimp_frame_stopped = true;
        services.release(self, id)?;
        preflight(self, true)?;
        self.residents.remove(&id.serial);
        self.state_resources.remove(&id.serial);
        self.specifications.remove(&id.serial);
        if self.links.remove_object(id.serial) != expected.registrations {
            return Err("Vimp unlink registrations changed".into());
        }
        Ok(expected)
    }
}
