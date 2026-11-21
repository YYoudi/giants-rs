use super::*;
use crate::mission_buildings::ordinary::Frame;

impl State {
    pub(crate) fn advance_ordinary_building_bodies(
        &mut self,
        session: &mut giants_game::resident_states::Session,
        number: u64,
        seconds: f32,
        areas: &[giants_game::object_time::SlowArea],
        terrain: &giants_formats::gti::Terrain,
    ) -> Result<Vec<Frame>, String> {
        self.buildings.ordinary.advance(
            session,
            number,
            seconds,
            areas,
            terrain,
            &self.environment_objects,
        )?;
        self.buildings.ordinary.frames(session, session.ticket())
    }
    pub(crate) fn publish_ordinary_building(&mut self, frame: &Frame) -> Result<(), String> {
        let fields = frame.player.fields;
        if let Some(object) = self.mesh_objects[..self.static_mesh_objects]
            .iter_mut()
            .find(|p| p.fields.id == fields.id)
        {
            object.fields = fields;
            object.response_flags = frame.player.response_flags;
            object.triangles = frame.player.triangles.clone();
        } else {
            self.mesh_objects.insert(
                self.static_mesh_objects,
                giants_game::player_world_collision::Object {
                    fields,
                    response_flags: frame.player.response_flags,
                    triangles: frame.player.triangles.clone(),
                },
            );
            self.static_mesh_objects += 1;
        }
        let triangles = frame
            .player
            .triangles
            .iter()
            .map(|t| t.vertices)
            .collect::<Vec<_>>();
        let mesh = giants_game::mecc_mesh_dispatch::MeshObject {
            fields,
            triangles: triangles.clone(),
        };
        if let Some(object) = self
            .environment_objects
            .iter_mut()
            .find(|o| o.mesh.fields.id == fields.id)
        {
            object.mesh = mesh;
        } else {
            self.environment_objects
                .push(giants_game::actor_environment::Collider {
                    mesh,
                    class: 53,
                    state: 7,
                    volume: None,
                });
        }
        if fields.collision_flags & 0x20000 != 0 {
            let projectile = giants_game::projectile_world::Object {
                id: fields.id,
                parent: fields.parent,
                active: fields.object_flags,
                personality: fields.collision_flags,
                bounds: fields.bounds,
                mesh: giants_game::object_segment::Mesh::new(fields.id, triangles)?,
            };
            if let Some((index, object)) = self.projectile_objects[..self.static_projectile_objects]
                .iter_mut()
                .enumerate()
                .find(|(_, p)| p.id == fields.id)
            {
                if self.static_projectile_declarations.get(index) != Some(&frame.pose.key) {
                    return Err("pub projectile identity belongs to another declaration".into());
                }
                *object = projectile;
            } else {
                self.projectile_objects
                    .insert(self.static_projectile_objects, projectile);
                self.static_projectile_declarations.push(frame.pose.key);
                self.static_projectile_objects += 1;
            }
        }
        self.buildings.publish_pub(frame);
        Ok(())
    }
}
