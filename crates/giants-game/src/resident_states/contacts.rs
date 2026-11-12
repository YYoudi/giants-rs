//! Shared dedicated-model geometry extraction after a family has validated its
//! completed frame. This advances no ANM, motion, clock, behavior or hit service.
use super::*;

impl Session {
    pub(super) fn contact_geometry(
        &mut self,
        id: Id,
        epoch: u32,
    ) -> Result<
        (
            crate::object_segment::Mesh,
            Vec<crate::player_mesh_contact::Triangle>,
        ),
        String,
    > {
        let specification = self
            .specification(id)
            .ok_or("contact specification absent")?
            .clone();
        let slot = specification.metadata.selection.table()?.collision_slot()?;
        let affine = self.actor_current_affine(id)?;
        let matrix = affine.matrix[..12].try_into().unwrap();
        self.prepare_actor_positions(id, slot, epoch, false)?;
        let local = self
            .actor_model_positions(id, slot)?
            .ok_or("collision positions absent")?;
        let model = specification
            .models
            .get(slot)
            .and_then(Option::as_ref)
            .ok_or("dedicated collision model absent")?;
        let mut triangles = Vec::new();
        for (part, object) in model.prepared.mesh.subobjects.iter().enumerate() {
            for (ordinal, face) in object.triangles.iter().enumerate() {
                let mut vertices = [[0.; 3]; 3];
                for (point, &index) in vertices.iter_mut().zip(face) {
                    let index = *model
                        .prepared
                        .mesh
                        .indices
                        .get(usize::from(index))
                        .ok_or("collision triangle index absent")?;
                    let position = *local
                        .get(usize::from(index))
                        .ok_or("collision triangle vertex absent")?;
                    *point = crate::footprint_builder::transform_cached(
                        matrix,
                        position,
                        crate::arithmetic::Precision::Double53,
                    )?;
                }
                triangles.push(crate::player_mesh_contact::Triangle {
                    vertices,
                    part: u32::try_from(part).map_err(|_| "collision part overflow")?,
                    ordinal: u32::try_from(ordinal).map_err(|_| "collision ordinal overflow")?,
                });
            }
        }
        Ok((self.actor_current_contact_mesh(id)?, triangles))
    }
}
