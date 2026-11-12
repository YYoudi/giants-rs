//! Initial admitted state0 surfaces. Their acquired PC descriptor has no shot
//! callback; the dispatcher leaves the actor unchanged. Prototype PS2 `hurt_object` is an
//! unknown navigation route, not an attributed callback or imported layout.
//! Observation never enables collisions, changes geometry or supplies health.
use giants_game::{
    actor_fields::Fields, declared_mesh_scene::Prepared, instances::Plan,
    mecc_collision_candidates::Object,
};
use std::collections::BTreeMap;

struct Surface {
    class: i32,
    fields: Object,
    triangles: Vec<[[f32; 3]; 3]>,
    player: Option<(Vec<u32>, Vec<u32>)>,
    position: [f32; 3],
    configured_health: u32,
    contacts: u64,
    hits: u32,
    serial: Option<u32>,
}
pub(super) struct Surfaces(BTreeMap<u32, Surface>);

fn object_words(o: &Object) -> Vec<u32> {
    [
        o.id,
        o.object_flags,
        o.collision_flags,
        u32::from(o.has_model),
        o.parent,
    ]
    .into_iter()
    .chain(
        o.bounds
            .min
            .into_iter()
            .chain(o.bounds.max)
            .map(f32::to_bits),
    )
    .collect()
}
fn triangle_words(triangles: &[[[f32; 3]; 3]]) -> Vec<u32> {
    triangles
        .iter()
        .flatten()
        .flatten()
        .copied()
        .map(f32::to_bits)
        .collect()
}
fn player_words(o: &giants_game::player_world_collision::Object) -> Vec<u32> {
    std::iter::once(o.response_flags)
        .chain(o.triangles.iter().flat_map(|t| {
            [t.part, t.ordinal]
                .into_iter()
                .chain(t.vertices.into_iter().flatten().map(f32::to_bits))
        }))
        .collect()
}
impl Surfaces {
    pub(super) fn prepare(plan: &Plan, player: &Prepared, projectile: &Prepared) -> Self {
        let source = plan.resources();
        let mut surfaces = BTreeMap::new();
        for (record, object) in projectile.records.iter().zip(&projectile.objects) {
            if record.instance.state != 0 {
                continue;
            }
            let fields = (|| {
                let spec =
                    giants_game::specification::Prepared::load_registered(source, record.class)?;
                Fields::load(source, record.class, &spec)
            })();
            let fields = match fields {
                Ok(fields) => fields,
                Err(reason) => {
                    println!(
                        "mission_building_no_shot_need class={} reason={reason:?}",
                        record.class
                    );
                    continue;
                }
            };
            let physical = player
                .player_objects
                .iter()
                .find(|p| p.fields.id == object.fields.id);
            surfaces.insert(
                object.fields.id,
                Surface {
                    class: record.class,
                    fields: object.fields,
                    triangles: object.triangles.clone(),
                    player: physical.map(|p| (object_words(&p.fields), player_words(p))),
                    position: record.instance.fields.current,
                    configured_health: fields.configuration[12],
                    contacts: 0,
                    hits: 0,
                    serial: None,
                },
            );
        }
        println!(
            "mission_building_no_shot_prepared surfaces={} source_state=0 health_is_damage_policy=false",
            surfaces.len()
        );
        Self(surfaces)
    }
    pub(super) fn physical(&self, id: u32) -> bool {
        self.0.get(&id).is_some_and(|s| s.player.is_some())
    }
    pub(super) fn contacts(&self, id: u32) -> u64 {
        self.0.get(&id).map_or(0, |s| s.contacts)
    }
    pub(super) fn hits(&self, id: u32) -> u32 {
        self.0.get(&id).map_or(0, |s| s.hits)
    }
    pub(super) fn note_contact(&mut self, id: u32) {
        if let Some(s) = self.0.get_mut(&id) {
            s.contacts += 1;
            if s.contacts == 1 {
                println!(
                    "mission_building_player_contact object={id} class={} actual_narrow_phase=true source_no_shot=true",
                    s.class
                );
            }
        }
    }
    pub(super) fn note_hit(
        &mut self,
        ticket: u64,
        hit: &giants_game::mecc_pistol::WorldHit,
    ) -> bool {
        let Some(s) = self.0.get_mut(&hit.object) else {
            return false;
        };
        if s.serial != Some(hit.serial) {
            s.serial = Some(hit.serial);
            s.hits += 1;
            println!(
                "mission_building_no_shot_hit ticket={ticket} object={} class={} serial={} hits={} source_state=0 callback_present=false configured_health_bits={} damage_store=false geometry_update=false source_retirement=false",
                hit.object, s.class, hit.serial, s.hits, s.configured_health
            );
        }
        true
    }
    pub(super) fn target(
        &self,
        position: [f32; 3],
        selected: Option<u32>,
    ) -> Option<(u32, [f32; 3])> {
        let selected_class = std::env::var("GIANTS_MISSION_BUILDING_CLASS")
            .ok()
            .and_then(|s| s.parse::<i32>().ok());
        let (&id, s) = if let Some(id) = selected {
            self.0.get_key_value(&id)?
        } else {
            self.0
                .iter()
                .filter(|(_, s)| selected_class.is_none_or(|c| c == s.class))
                .min_by(|(_, a), (_, b)| {
                    let d = |s: &Surface| {
                        (s.position[0] - position[0]).hypot(s.position[1] - position[1])
                    };
                    d(a).total_cmp(&d(b))
                })?
        };
        let point = s
            .triangles
            .iter()
            .map(|t| {
                std::array::from_fn::<_, 3, _>(|i| {
                    ((f64::from(t[0][i]) + f64::from(t[1][i])) + f64::from(t[2][i])) as f32 / 3.
                })
            })
            .max_by(|a, b| a[2].total_cmp(&b[2]))?;
        Some((id, point))
    }
    pub(super) fn verify(
        &self,
        id: u32,
        players: &[giants_game::player_world_collision::Object],
        projectiles: &[giants_game::projectile_world::Object],
    ) -> Result<(), String> {
        let s = self.0.get(&id).ok_or("no-shot source surface absent")?;
        let p = projectiles
            .iter()
            .find(|p| p.id == id)
            .ok_or("no-shot projectile identity absent")?;
        if p.active != s.fields.object_flags
            || p.parent != s.fields.parent
            || p.personality != s.fields.collision_flags
            || p.mesh.id != id
            || p.bounds.min.map(f32::to_bits) != s.fields.bounds.min.map(f32::to_bits)
            || p.bounds.max.map(f32::to_bits) != s.fields.bounds.max.map(f32::to_bits)
            || p.mesh.static_world_triangles().map(triangle_words)
                != Some(triangle_words(&s.triangles))
        {
            return Err("no-shot projectile geometry or admission changed".into());
        }
        if let Some((fields, triangles)) = &s.player {
            let player = players
                .iter()
                .find(|p| p.fields.id == id)
                .ok_or("no-shot player identity absent")?;
            if &object_words(&player.fields) != fields || &player_words(player) != triangles {
                return Err("no-shot player geometry or admission changed".into());
            }
        } else if players.iter().any(|p| p.fields.id == id) {
            return Err("no-shot surface gained an unowned player mesh".into());
        }
        if s.hits < 3 {
            return Err("no-shot probe needs three actual distinct projectile hits".into());
        }
        Ok(())
    }
}
