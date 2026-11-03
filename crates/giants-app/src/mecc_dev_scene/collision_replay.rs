//! Explicit development trajectory. Face choice is test setup, not native AI.
use super::*;
pub(super) struct Walk {
    object: u32,
    point: Vec3,
    normal: Vec3,
    free: bool,
    pose: bool,
    initial_distance: f32,
    extent: f32,
    contact_distance: Option<f32>,
    finished: Option<bool>,
}
impl State {
    pub(super) fn begin_world_shot(
        &mut self,
        terrain: &giants_formats::gti::Terrain,
    ) -> Result<(), &'static str> {
        self.begin_collision_walk(terrain)?;
        let walk = self
            .collision_walk
            .take()
            .ok_or("source shot fixture absent")?;
        self.projectile_test = Some((walk.point.to_array(), walk.normal.to_array(), walk.object));
        println!(
            "mecc_world_shot_start object={} aim={:?} camera=development source_face=true muzzle=animated_weapon",
            walk.object, walk.point
        );
        Ok(())
    }
    pub(super) fn begin_collision_walk(
        &mut self,
        terrain: &giants_formats::gti::Terrain,
    ) -> Result<(), &'static str> {
        let query = giants_game::terrain_queries::StaticTerrainQuery::new(
            terrain,
            1. / terrain.spacing,
            Precision::Double53,
        )?;
        // A source building, selected by declaration identity, not a generated wall.
        let object = self
            .mesh_objects
            .iter()
            .find(|o| o.fields.id == 12)
            .ok_or("declared building absent")?;
        let object_id = object.fields.id;
        let mid = (v3(object.fields.bounds.min) + v3(object.fields.bounds.max)) * 0.5;
        let mut selected = None;
        for triangle in &object.triangles {
            let [a, b, c] = triangle.vertices.map(v3);
            let Some(normal) = (b - a).cross(c - a).try_normalize() else {
                continue;
            };
            let center = (a + b + c) / 3.;
            if normal.z.abs() >= 0.3 || normal.dot(center - mid) <= 0. {
                continue;
            }
            let ground = query.sample([center.x, center.y])?;
            if !ground.in_bounds || ground.plane[2] < 0.99 {
                continue;
            }
            let mut contact = Vec3::new(
                center.x,
                center.y,
                ground.height + self.ground_offset + self.collision_dimensions[0] * self.scale,
            );
            contact -= normal * normal.dot(contact - a);
            // Keep the projected body center inside a source face, away from edges.
            let cross =
                [(a, b), (b, c), (c, a)].map(|(u, v)| normal.dot((v - u).cross(contact - u)));
            if cross.into_iter().any(|x| x < 0.01) {
                continue;
            }
            let outside = contact + normal * (self.collision.radius + 8.);
            let g = query.sample([outside.x, outside.y])?;
            if !g.in_bounds || g.plane[2] < 0.99 {
                continue;
            }
            if selected
                .as_ref()
                .is_none_or(|(_, best, _, _): &(Vec3, Vec3, _, Vec3)| normal.z.abs() < best.z.abs())
            {
                selected = Some((contact, normal, g, outside));
            }
        }
        let (point, normal, ground, outside) =
            selected.ok_or("no grounded source-face approach")?;
        self.start.position = [outside.x, outside.y, ground.height + self.ground_offset];
        self.initial_yaw = (-normal.x).atan2(normal.y);
        self.initial_floor = ground.height;
        self.initial_normal = ground.plane[..3].try_into().unwrap();
        self.initial_plane = ground.plane;
        self.reset_pose();
        let free =
            std::env::var("GIANTS_MECC_DEV_REPLAY").ok().as_deref() == Some("collision-free");
        let pose =
            std::env::var("GIANTS_MECC_DEV_REPLAY").ok().as_deref() == Some("collision-pose");
        let half = self.collision_dimensions[1] * self.scale * 0.5;
        let extent = Vec3::new(
            normal.x * self.collision.radius,
            normal.y * self.collision.radius,
            normal.z * half,
        )
        .length();
        let initial_distance = normal.dot(
            v3(self.start.position) + Vec3::Z * (self.collision_dimensions[1] * self.scale * 0.5)
                - point,
        );
        println!(
            "mecc_collision_walk_start object={} position={:?} point={:?} normal={:?} radius={} initial_distance={} empty_control={} native_trajectory=false",
            object_id,
            self.start.position,
            point.to_array(),
            normal.to_array(),
            self.collision.radius,
            initial_distance,
            free
        );
        self.collision_walk = Some(Walk {
            object: object_id,
            point,
            normal,
            free,
            pose,
            initial_distance,
            extent,
            contact_distance: None,
            finished: None,
        });
        if free {
            self.mesh_objects.clear();
        }
        Ok(())
    }
    pub(super) fn collision_walk_tick(&mut self, tick: u32) -> Result<Option<bool>, &'static str> {
        let walk = self
            .collision_walk
            .as_mut()
            .ok_or("collision replay absent")?;
        if walk.finished.is_some() {
            return Ok(walk.finished);
        }
        let distance = walk.normal.dot(
            v3(self.motion.position) + Vec3::Z * (self.collision_dimensions[1] * self.scale * 0.5)
                - walk.point,
        );
        if !distance.is_finite() || !self.motion.velocity.into_iter().all(f32::is_finite) {
            return Err("nonfinite collision walk");
        }
        if tick == 240 {
            let moved = walk.initial_distance - distance > 2.;
            let passed = moved
                && if walk.free {
                    distance < -self.collision.radius && self.mesh_responses == 0
                } else {
                    distance >= walk.extent - 0.1
                        && distance <= walk.extent + 0.5
                        && self.mesh_responses > 0
                };
            println!(
                "mecc_collision_walk_contact passed={passed} object={} distance={distance} radius={} mesh_responses={} empty_control={} position={:?} physical_extent={} support={} physics_flags={:x}",
                walk.object,
                self.collision.radius,
                self.mesh_responses,
                walk.free,
                self.motion.position,
                walk.extent,
                self.support,
                self.physics_flags
            );
            walk.contact_distance = Some(distance);
            if !passed || walk.pose {
                walk.finished = Some(passed);
                return Ok(Some(passed));
            }
        }
        if tick == 360 {
            let returned =
                distance - walk.contact_distance.ok_or("contact checkpoint absent")? > 2.;
            let passed = returned && self.motion.position.into_iter().all(f32::is_finite);
            println!(
                "mecc_collision_walk_finished passed={passed} steps=360 distance={distance} mesh_responses={} empty_control={} native_trajectory_parity=false",
                self.mesh_responses, walk.free
            );
            walk.finished = Some(passed);
            return Ok(Some(passed));
        }
        Ok(None)
    }
}
