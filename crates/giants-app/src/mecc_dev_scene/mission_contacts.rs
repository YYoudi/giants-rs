use super::*;
use giants_game::{resident_states::Id, vimp_contacts::Snapshot};

pub(super) struct Contact {
    pub key: u32,
    pub owner: Id,
}

impl State {
    pub(crate) fn apply_building_hit(
        &mut self,
        hit: &crate::mission_buildings::HitResult,
    ) -> Result<(), String> {
        // Allocate from the same monotonic namespace as NPC contact keys.
        // Validate the fresh object before publishing either collision state.
        let replacement = if let Some(index) = hit.replacement {
            let id = self.next_contact_key;
            let next = id
                .checked_add(1)
                .ok_or("replacement collision identity overflow")?;
            if self.mesh_objects.iter().any(|o| o.fields.id == id)
                || self.projectile_objects.iter().any(|o| o.id == id)
            {
                return Err("replacement collision identity occupied".into());
            }
            Some((
                index,
                id,
                next,
                self.buildings.replacement_player(index, id)?,
            ))
        } else {
            None
        };
        if hit.physical {
            let object = self
                .mesh_objects
                .iter_mut()
                .find(|o| o.fields.id == hit.object)
                .ok_or("building player collision identity missing")?;
            object.fields.object_flags = hit.active;
            object.triangles = hit
                .triangles
                .iter()
                .zip(&hit.identities)
                .map(
                    |(v, (part, ordinal))| giants_game::player_mesh_contact::Triangle {
                        vertices: *v,
                        part: *part,
                        ordinal: *ordinal,
                    },
                )
                .collect();
        }
        let projectile = self
            .projectile_objects
            .iter_mut()
            .find(|o| o.id == hit.object)
            .ok_or("building projectile collision identity missing")?;
        projectile.active = hit.active;
        if !hit.triangles.is_empty() {
            projectile.mesh =
                giants_game::object_segment::Mesh::new(hit.object, hit.triangles.clone())?;
        }
        if hit.environment {
            let environment = self
                .environment_objects
                .iter_mut()
                .find(|o| o.mesh.fields.id == hit.object)
                .ok_or("building environment collision identity missing")?;
            environment.mesh.fields.object_flags = hit.active;
            environment.mesh.triangles = hit.triangles.clone();
        }
        if let Some((index, id, next, player)) = replacement {
            self.buildings.publish_replacement(index, id)?;
            // Persistent building objects precede the transient NPC suffix.
            // The next NPC snapshot must not truncate this new owner away.
            self.mesh_objects.insert(self.static_mesh_objects, player);
            self.static_mesh_objects += 1;
            self.next_contact_key = next;
        }
        Ok(())
    }
    pub(crate) fn mission_npc_contact_responses(&self) -> u64 {
        self.npc_mesh_responses
    }
    /// Replace only this host sample's completed NPC shapes. Prepared-scene
    /// keys occupy a separate namespace from typed Session identities.
    pub(crate) fn set_mission_contacts(
        &mut self,
        ticket: u64,
        snapshots: Vec<Snapshot>,
    ) -> Result<(), String> {
        if snapshots.len() > 4096 {
            return Err("mission contact population budget".into());
        }
        let mut keys = self.npc_contact_keys.clone();
        let mut next = self.next_contact_key;
        let mut contacts = Vec::new();
        let mut objects = Vec::new();
        let mut projectiles = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        for snapshot in snapshots {
            let actor = snapshot.actor;
            if actor.id.ticket() != ticket
                || !seen.insert(actor.id)
                || actor.class != 8
                || actor.state != 21
                || actor.parent.is_some()
            {
                return Err("mission contact identity or acquired Vimp profile differs".into());
            }
            if actor.flags.active & 0x4000_c000 != 0 {
                continue;
            }
            if snapshot.response_flags & 0x10 != 0 {
                return Err(
                    "mission contact requires reciprocal peer-reference publication".into(),
                );
            }
            let key = if let Some(&key) = keys.get(&actor.id) {
                key
            } else {
                let key = next;
                next = next
                    .checked_add(1)
                    .ok_or("mission collision identity budget")?;
                keys.insert(actor.id, key);
                key
            };
            objects.push(giants_game::player_world_collision::Object {
                fields: giants_game::mecc_collision_candidates::Object {
                    id: key,
                    parent: 0,
                    has_model: true,
                    object_flags: actor.flags.active,
                    collision_flags: actor.flags.personality,
                    bounds: snapshot.bounds,
                },
                response_flags: snapshot.response_flags,
                triangles: snapshot.triangles,
            });
            // The ordinary partial pistol retains its acquired base
            // constructor context zero and the single-player filter.
            if giants_game::projectile_contacts::accepts(
                &actor,
                &actor,
                0,
                giants_game::projectile_contacts::Environment {
                    mode: 0,
                    override_flag: 0,
                },
            ) {
                projectiles.push(giants_game::projectile_world::Object {
                    id: key,
                    parent: 0,
                    active: actor.flags.active,
                    personality: actor.flags.personality,
                    bounds: snapshot.bounds,
                    mesh: snapshot.mesh,
                });
            }
            contacts.push(Contact {
                key,
                owner: actor.id,
            });
        }
        self.mesh_objects.truncate(self.static_mesh_objects);
        self.mesh_objects.extend(objects);
        self.projectile_objects
            .truncate(self.static_projectile_objects);
        self.projectile_objects.extend(projectiles);
        self.npc_contacts = contacts;
        self.npc_contact_keys = keys;
        self.next_contact_key = next;
        Ok(())
    }

    pub(crate) fn mission_npc_hit_owner(&self, key: u32) -> Option<Id> {
        self.npc_contacts
            .iter()
            .find(|contact| contact.key == key)
            .map(|contact| contact.owner)
    }
}
