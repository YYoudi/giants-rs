//! Copy completed Ripper poses into the existing ordinary NPC body renderer.
use super::*;
use std::{collections::BTreeMap, sync::Arc};
#[derive(Default)]
pub(super) struct Geometry(BTreeMap<i32, Arc<giants_game::scene_resources::ModelAsset>>);
impl State {
    pub(in crate::mission_runtime) fn draws(
        &mut self,
        session: &mut Session,
        ticket: u64,
        resources: &giants_game::scene_resources::SceneResources,
    ) -> Result<Vec<super::super::npc::Draw>, String> {
        self.view(session, ticket)?;
        if self.driver.failed() {
            return Ok(vec![]);
        }
        let mut result = vec![];
        for owner in &self.bound.as_ref().unwrap().1 {
            let Some((number, epoch)) = self.driver.completed(owner.id) else {
                continue;
            };
            let actor = session.get(owner.id).ok_or("Ripper draw owner retired")?;
            let fields = actor
                .construction_fields
                .as_ref()
                .ok_or("Ripper draw fields absent")?;
            if !giants_game::draw_routes::admit(23, fields.object_flags, 0).ordinary {
                continue;
            }
            if session.actor_world_tick(owner.id)?.phase()
                != giants_game::actor_world_tick::Phase::Finished
                || session.actor_finalization(owner.id)?.phase()
                    != giants_game::resident_finalization::Phase::Idle
            {
                return Err("Ripper draw requires a completed ordinary frame".into());
            }
            let affine = session.actor_current_affine(owner.id)?;
            if affine.flags & 8 != 0 {
                return Err("Ripper mirrored culling consumer required".into());
            }
            let class = owner.declaration.class;
            let center = fields.position;
            if let std::collections::btree_map::Entry::Vacant(entry) = self.geometry.0.entry(class)
            {
                let specification = session
                    .specification(owner.id)
                    .ok_or("Ripper draw specification absent")?;
                let model = specification
                    .models
                    .first()
                    .and_then(Option::as_ref)
                    .ok_or("Ripper primary model absent")?;
                let geometry = resources
                    .model_slot_for_body(
                        class,
                        0,
                        giants_game::scene_resources::TextureEncoding::Rgb565OrArgb4444,
                    )
                    .map_err(|e| e.to_string())?
                    .ok_or("Ripper draw geometry absent")?;
                if geometry.name != model.name || geometry.mesh.normal_count != 0 {
                    return Err(
                        "Ripper geometry differs from the acquired unlit body profile".into(),
                    );
                }
                for part in &geometry.mesh.subobjects {
                    let surface = giants_game::body_pipeline::preview_surface(
                        giants_game::body_pipeline::Normals {
                            prepare: false,
                            context: false,
                        },
                        &part.material,
                        geometry
                            .alpha_textures
                            .contains(&part.material.texture.to_ascii_lowercase()),
                    )?;
                    if surface != giants_game::body_pipeline::Surface::Rgb {
                        return Err("Ripper body material requires an unacquired draw route".into());
                    }
                }
                entry.insert(Arc::new(geometry));
            }
            session.prepare_actor_positions(owner.id, 0, epoch, false)?;
            let local = session
                .actor_model_positions(owner.id, 0)?
                .ok_or("Ripper animated positions absent")?;
            let matrix = affine.matrix[..12].try_into().unwrap();
            // Copy now: the next actor may reuse this shared model pose buffer.
            let positions = local
                .iter()
                .map(|&point| {
                    let p = giants_game::uncached_pose::transform(
                        matrix,
                        point,
                        giants_game::arithmetic::Precision::Double53,
                    )?;
                    Ok([p[0], p[2], -p[1]])
                })
                .collect::<Result<Vec<_>, String>>()?;
            result.push(super::super::npc::Draw {
                class,
                transparency: 0.,
                id: super::super::npc::DrawOwner::Actor(owner.id),
                frame: number,
                geometry: self.geometry.0[&class].clone(),
                positions,
                center,
            });
        }
        Ok(result)
    }
}
