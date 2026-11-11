//! Draw only owned auxiliary callbacks. Static meat retains its actual
//! unskinned construction pose; ghosts use their consumed current ANM epoch.
use super::*;
impl State {
    pub(in crate::mission_runtime) fn effect_draws(
        &self,
        view: [f32; 16],
    ) -> Result<Vec<Draw>, String> {
        self.fx
            .draws()?
            .into_iter()
            .map(|draw| {
                let matrix = giants_game::camera_facing::draw_matrix(
                    draw.matrix.values,
                    draw.personality,
                    giants_game::camera_facing::Input {
                        view,
                        position: [
                            draw.matrix.values[3],
                            draw.matrix.values[7],
                            draw.matrix.values[11],
                        ],
                        scale: draw.size,
                        angle: 0.,
                    },
                    giants_game::arithmetic::Precision::Double53,
                )?;
                let positions = draw
                    .local
                    .into_iter()
                    .map(|p| {
                        let p = giants_game::uncached_pose::transform(
                            matrix[..12].try_into().unwrap(),
                            p,
                            giants_game::arithmetic::Precision::Double53,
                        )?;
                        Ok([p[0], p[2], -p[1]])
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                Ok(Draw {
                    class: draw.class,
                    transparency: draw.transparency,
                    id: DrawOwner::Effect(draw.key),
                    frame: draw.frame,
                    geometry: draw.geometry,
                    positions,
                    center: [matrix[3], matrix[7], matrix[11]],
                })
            })
            .collect()
    }
    pub(super) fn auxiliary_draws(
        &mut self,
        session: &mut Session,
        resources: &giants_game::scene_resources::SceneResources,
    ) -> Result<Vec<Draw>, String> {
        let mut draws = Vec::new();
        for id in session.list(0) {
            let actor = session.get(id).ok_or("auxiliary draw owner absent")?;
            let fields = actor.construction_fields.as_ref();
            let Some(fields) = fields else {
                continue;
            };
            if fields.object_flags & 0xc000 != 0 {
                continue;
            }
            let class = actor.class;
            let center = fields.position;
            let specification = session.specification(id);
            let (number, epoch, transparency, matrix) = if let Some(chunk) = session.vimp_chunk(id)
            {
                if chunk.number() == 0 {
                    continue;
                }
                if chunk.failed() {
                    return Err("meat draw callback quarantined".into());
                }
                let specification = specification.ok_or("meat draw specification absent")?;
                if specification
                    .models
                    .first()
                    .and_then(Option::as_ref)
                    .is_none_or(|m| m.skeleton_source.is_some())
                    || !specification.corrections.is_empty()
                    || specification
                        .animations
                        .values()
                        .any(|d| d.template.bindings.first().is_some_and(Option::is_some))
                    || specification
                        .metadata
                        .events
                        .iter()
                        .any(|t| !t.events.is_empty())
                {
                    return Err(
                        "meat drawing requires additional animated/corrected model preparation"
                            .into(),
                    );
                }
                session.prepare_actor_affine(id)?;
                (
                    chunk.number(),
                    0,
                    chunk.fade,
                    session.actor_current_affine(id)?.matrix,
                )
            } else if let Some(number) = session.vimp_ghost_number(id) {
                if number == 0 {
                    continue;
                }
                let draw = session.vimp_ghost_draw(id)?;
                (
                    draw.number,
                    draw.epoch,
                    draw.transparency,
                    draw.matrix.values,
                )
            } else {
                continue;
            };
            let specification = session
                .specification(id)
                .ok_or("auxiliary draw specification absent")?
                .clone();
            let model = specification
                .models
                .first()
                .and_then(Option::as_ref)
                .ok_or("auxiliary draw primary model absent")?;
            if let std::collections::btree_map::Entry::Vacant(entry) =
                self.auxiliary_geometry.entry(class)
            {
                let geometry = resources
                    .model_slot_for_body(
                        class,
                        0,
                        giants_game::scene_resources::TextureEncoding::Rgb565OrArgb4444,
                    )
                    .map_err(|e| e.to_string())?
                    .ok_or("auxiliary draw model absent")?;
                if geometry.name != model.name || geometry.mesh.normal_count != 0 {
                    return Err("auxiliary draw model identity or unlit capability differs".into());
                }
                for part in &geometry.mesh.subobjects {
                    giants_game::unlit_material::prepare_with_object_transparency(
                        &part.material,
                        geometry
                            .alpha_textures
                            .contains(&part.material.texture.to_ascii_lowercase()),
                        transparency,
                    )?;
                }
                entry.insert(Arc::new(geometry));
            }
            session.prepare_actor_positions(id, 0, epoch, false)?;
            let positions = session
                .actor_model_positions(id, 0)?
                .ok_or("auxiliary current positions absent")?
                .iter()
                .map(|&point| {
                    let p = giants_game::uncached_pose::transform(
                        matrix[..12].try_into().unwrap(),
                        point,
                        giants_game::arithmetic::Precision::Double53,
                    )?;
                    Ok([p[0], p[2], -p[1]])
                })
                .collect::<Result<Vec<_>, String>>()?;
            draws.push(Draw {
                class,
                transparency,
                id: DrawOwner::Actor(id),
                frame: number,
                geometry: self.auxiliary_geometry[&class].clone(),
                positions,
                center,
            });
        }
        Ok(draws)
    }
}
