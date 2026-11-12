//! Ordinary controller contact composition. Prototype PS2 collision-name lookup
//! is recorded privately; attribution remains unknown. Reuses the acquired PC
//! affine, bounds, binding and segment kernels without asserting state2 parity.
//! `collide_precalc_object` / `object_calc_collision_box` are lookup candidates,
//! not confirmed names for this composition or the PC state6 callback.
use super::*;

impl Session {
    pub fn controlled_mecc_contact_snapshot(
        &mut self,
        id: Id,
        step: u64,
        resources: &crate::controlled_mecc_contacts::Resources,
        sample: crate::controlled_mecc_contacts::Sample<'_>,
    ) -> Result<crate::controlled_mecc_contacts::Snapshot, String> {
        self.controlled_mecc_motion(id, step)?;
        let specification = self
            .specification(id)
            .ok_or("Mecc contact specification absent")?;
        if !Arc::ptr_eq(specification, &resources.specification) {
            return Err("Mecc contact resources belong to another canonical specification".into());
        }
        let descriptor = specification
            .animations
            .get(&sample.slot)
            .ok_or("Mecc contact current descriptor absent")?;
        let tree = descriptor
            .animation
            .trees
            .iter()
            .find(|tree| tree.offset == descriptor.animation.default_tree)
            .ok_or("Mecc contact current default tree absent")?;
        if descriptor.name != sample.name
            || tree.name != sample.tree
            || !sample.seconds.is_finite()
            || sample.seconds < 0.
        {
            return Err("Mecc contact sample differs from the normal selected descriptor".into());
        }
        // Validate that the current normal body tree and the dedicated-model
        // descriptor belong to the same prepared selection, not a render LOD.
        let body_binding = descriptor
            .template
            .bindings
            .first()
            .and_then(Option::as_ref)
            .ok_or("Mecc contact normal body binding absent")?;
        if body_binding.tree != tree.offset {
            return Err("Mecc contact normal body uses another animation tree".into());
        }
        let slot = specification.metadata.selection.table()?.collision_slot()?;
        let model = specification
            .models
            .get(slot)
            .and_then(Option::as_ref)
            .ok_or("Mecc dedicated collision model absent")?;
        let binding = descriptor
            .template
            .bindings
            .get(slot)
            .and_then(Option::as_ref)
            .ok_or("Mecc dedicated collision binding absent")?;
        let skin = model
            .prepared
            .skin
            .as_ref()
            .ok_or("Mecc collision skin absent")?;
        let tick = crate::model_pose::seconds_tick(descriptor.animation.timeline, sample.seconds)?;
        let positions =
            crate::skinning::positions_bound(skin, &descriptor.animation, binding, tick)
                .map_err(|e| e.to_string())?;
        let model_bounds = resources.bounds.bounds(u32::from(sample.slot))?;
        // Clone the Arc before the actual cache writer borrows this Session.
        let specification = specification.clone();
        self.prepare_actor_affine(id)?;
        let affine = self.actor_current_affine(id)?;
        let fields = self.get(id).unwrap().construction_fields.as_ref().unwrap();
        let bounds = crate::collision_bounds::cached(
            Some(crate::collision_bounds::Models {
                primary: model_bounds.primary,
                auxiliary: model_bounds.current_auxiliary,
                slot: u32::from(sample.slot),
            }),
            affine.matrix[..12].try_into().unwrap(),
            crate::arithmetic::Precision::Double53,
        )?;
        let pose =
            crate::contact_pose::Cached::new(fields.object_flags, affine.flags, affine.matrix)?;
        let mesh = crate::object_segment::Mesh::from_local_positions(
            u32::try_from(id.serial).map_err(|_| "Mecc contact identity overflow")?,
            &specification.models[slot].as_ref().unwrap().prepared.mesh,
            &positions,
            pose,
        )?;
        Ok(crate::controlled_mecc_contacts::Snapshot {
            owner: id,
            step,
            slot: sample.slot,
            tick,
            active: fields.object_flags,
            personality: fields.configuration[14],
            bounds: crate::volume_queries::Bounds {
                min: bounds[..3].try_into().unwrap(),
                max: bounds[3..].try_into().unwrap(),
            },
            mesh,
        })
    }
}
