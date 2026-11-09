use super::*;
use crate::kabuto_reference_query as query;
impl Session {
    pub(crate) fn query_kabuto_reference(
        &self,
        owner: Id,
        resources: &Resources,
        number: u64,
        ray: query::Ray,
        mut bounds: impl FnMut(&Session, Id) -> Result<([f32; 3], [f32; 3]), String>,
    ) -> Result<query::Selection<Id>, String> {
        let a = self
            .get(owner)
            .ok_or("Kabuto query owner absent or foreign")?;
        let g = a.kabuto_group.as_ref().ok_or("Kabuto query group absent")?;
        if a.class != 1
            || a.seed.storage.state() != 1
            || g.cleaned_up
            || g.geometry_transition_failure.is_some()
            || g.motion_history != Some(number)
            || g.resource_phase_completed != Some(number)
            || !Arc::ptr_eq(&g.declared_class, &resources.specification)
            || self
                .specification(owner)
                .is_none_or(|s| !Arc::ptr_eq(s, &resources.specification))
        {
            return Err("Kabuto query requires its factory-owned completed resource phase".into());
        }
        let p = a
            .seed
            .storage
            .data()
            .filter(|p| p.len() == 844)
            .ok_or("Kabuto query payload absent")?;
        let source = query::Source {
            identity: owner,
            parent: a.attachment.map(|a| a.parent),
            masks: std::array::from_fn(|i| {
                u32::from_le_bytes(p[0x334 + i * 4..0x338 + i * 4].try_into().unwrap())
            }),
        };
        let mut candidates = Vec::new();
        for id in self.list(0) {
            if id == owner {
                continue;
            }
            let a = self.get(id).ok_or("Kabuto query list actor absent")?;
            let f = a
                .construction_fields
                .as_ref()
                .ok_or("Kabuto query candidate fields absent")?;
            if f.object_flags & 0x4000c000 != 0 {
                continue;
            }
            if a.seed.storage.profile() != Profile::Debug {
                return Err("Kabuto query candidate outside acquired profile".into());
            }
            let parent = a
                .attachment
                .map(|a| a.parent)
                .map(|parent| {
                    self.get(parent)
                        .map(|a| (parent, u32::from(a.seed.storage.state())))
                        .ok_or("Kabuto query candidate parent absent or foreign")
                })
                .transpose()?;
            let mut candidate = query::Candidate {
                identity: id,
                state: u32::from(a.seed.storage.state()),
                flags: f.object_flags,
                category_mask: f.configuration[16],
                parent,
                lower: [0.; 3],
                upper: [0.; 3],
            };
            if !query::eligible(&source, Some(&candidate), 7) {
                continue;
            }
            // No first-model or radius fallback. The caller must produce the
            // current acquired header bounds for this exact owned candidate.
            (candidate.lower, candidate.upper) = bounds(self, id)?;
            candidates.push(Some(candidate));
        }
        query::select(&source, &candidates, ray)
    }
}
