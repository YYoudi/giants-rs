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
        let owners = self
            .bound
            .as_ref()
            .unwrap()
            .1
            .iter()
            .map(|owner| (owner.id, owner.declaration.class))
            .chain(self.driver.markers().map(|id| (id, 219)))
            .chain(
                self.driver
                    .projectiles
                    .iter()
                    .filter(|id| !self.driver.projectile_failures.contains_key(id))
                    .filter_map(|&id| session.get(id).map(|actor| (id, actor.class))),
            )
            .collect::<Vec<_>>();
        for (id, class) in owners {
            let shot = matches!(class, 90 | 232);
            let (number, epoch) = if shot {
                let completed = session.enemy_projectile_presentation(id)?;
                let Some(epoch) = completed.epoch else {
                    continue;
                };
                (completed.flight_revision, epoch)
            } else {
                let Some(completed) = self.driver.completed(id) else {
                    continue;
                };
                completed
            };
            let actor = session.get(id).ok_or("Ripper draw owner retired")?;
            let fields = actor
                .construction_fields
                .as_ref()
                .ok_or("Ripper draw fields absent")?;
            if !giants_game::draw_routes::admit(
                u32::from(actor.seed.storage.state()),
                fields.object_flags,
                0,
            )
            .ordinary
            {
                continue;
            }
            if !shot
                && (session.actor_world_tick(id)?.phase()
                    != giants_game::actor_world_tick::Phase::Finished
                    || (class != 219
                        && session.actor_finalization(id)?.phase()
                            != giants_game::resident_finalization::Phase::Idle)
                    || (class == 219 && session.surface_marker_frame(id)?.failure.is_some()))
            {
                return Err("Ripper draw requires a completed ordinary frame".into());
            }
            let affine = session.actor_current_affine(id)?;
            if affine.flags & 8 != 0 {
                return Err("Ripper mirrored culling consumer required".into());
            }
            let center = fields.position;
            if let std::collections::btree_map::Entry::Vacant(entry) = self.geometry.0.entry(class)
            {
                let specification = session
                    .specification(id)
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
                    if shot {
                        // The common auxiliary renderer retains the acquired
                        // transparent/additive program, rather than an opaque
                        // body approximation for the spit material.
                        giants_game::unlit_material::prepare_with_object_transparency(
                            &part.material,
                            geometry
                                .alpha_textures
                                .contains(&part.material.texture.to_ascii_lowercase()),
                            0.,
                        )?;
                        continue;
                    }
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
            session.prepare_actor_positions(id, 0, epoch, false)?;
            let local = session
                .actor_model_positions(id, 0)?
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
                rigid: None,
                texture: None,
                class,
                transparency: 0.,
                id: super::super::npc::DrawOwner::Actor(id),
                frame: number,
                geometry: self.geometry.0[&class].clone(),
                positions,
                center,
            });
        }
        Ok(result)
    }
}
