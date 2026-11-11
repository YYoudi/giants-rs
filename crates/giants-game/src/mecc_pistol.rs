//! Initial normal-Mecc pistol, using the measured fire gate and straight flight.
//! Object hits stop a shot; actor impact/damage services are not invented here.
use crate::{
    arithmetic::Precision,
    player_equipment::Initial,
    straight_projectile::{Event, Shot},
};
use giants_formats::{
    gti::Terrain,
    weapons::{Catalog, Projectile},
};

pub struct Pistol {
    pub definition: Projectile,
    pub shots: Vec<Shot>,
    pub impacts: Vec<([f32; 3], f32)>,
    pub fired: u32,
    pub world_hits: u32,
    pub ground_hits: u32,
    pub retired: u32,
    pub cooldown: f32,
}
pub enum Impact {
    World(crate::projectile_world::Query),
    Ground([f32; 3]),
}
pub struct Frame {
    pub fired: bool,
    pub impacts: Vec<Impact>,
    pub world_hits: Vec<WorldHit>,
}
/// Partial-pistol serial and supplied collision key remain distinct from
/// Session identities. The integration owner resolves the selected key.
pub struct WorldHit {
    pub object: u32,
    pub serial: u32,
    pub motion: crate::damage_impulse::Motion,
    pub older: [f32; 3],
    pub point: [f32; 3],
    pub damage: f32,
}

impl Pistol {
    pub fn from_initial(equipment: Initial, catalog: &Catalog) -> Result<Self, &'static str> {
        let slot =
            usize::try_from(equipment.selected).map_err(|_| "invalid selected weapon slot")?;
        let icon = *equipment
            .weapons
            .get(slot)
            .ok_or("selected weapon slot outside inventory")?;
        if icon != 1 || equipment.utility != [-1; 3] || equipment.special != -1 {
            return Err("mission fire outside initial normal pistol/no-gyro profile");
        }
        let icon = catalog
            .icons
            .get(&icon)
            .ok_or("selected pistol icon absent")?;
        let definition = catalog
            .projectiles
            .get(&icon.projectile)
            .ok_or("selected pistol projectile absent")?
            .clone();
        Shot::new(&definition, [0.; 3], [0., 1., 0.])?;
        Ok(Self {
            definition,
            shots: Vec::new(),
            impacts: Vec::new(),
            fired: 0,
            world_hits: 0,
            ground_hits: 0,
            retired: 0,
            cooldown: 0.,
        })
    }
    pub fn reset(&mut self) {
        self.shots.clear();
        self.impacts.clear();
        self.fired = 0;
        self.world_hits = 0;
        self.ground_hits = 0;
        self.retired = 0;
        self.cooldown = 0.;
    }
    pub fn step(
        &mut self,
        dt: f32,
        origin: [f32; 3],
        aim: [f32; 3],
        held: bool,
        terrain: &Terrain,
        objects: &[crate::projectile_world::Object],
    ) -> Result<Frame, &'static str> {
        self.cooldown = crate::mecc_fire::advance_clock(self.cooldown, dt, Precision::Double53)?;
        // The complete native base-pistol ammo getter returns available=1 and
        // its use operation leaves the supplied no-gyro normal inventory intact.
        let (fired, next) = crate::mecc_fire::request(
            self.cooldown,
            self.definition.interval,
            self.definition.flags,
            true,
            false,
            held,
            Precision::Double53,
        )?;
        self.cooldown = next;
        if fired {
            if self.shots.len() >= 4096 {
                return Err("mission projectile budget");
            }
            let mut shot = Shot::new(&self.definition, origin, aim)?;
            self.fired = self
                .fired
                .checked_add(1)
                .ok_or("mission projectile serial overflow")?;
            shot.serial = self.fired;
            self.shots.push(shot);
        }
        for (_, ttl) in &mut self.impacts {
            *ttl -= dt;
        }
        self.impacts.retain(|(_, ttl)| *ttl > 0.);
        let mut impacts = Vec::new();
        let mut world_hits = Vec::new();
        let mut live = Vec::with_capacity(self.shots.len());
        for mut shot in self.shots.drain(..) {
            let start = shot.motion.position;
            let event = shot.step(terrain, dt)?;
            // Terrain runs first and can shorten this frame's segment. Match
            // the owned projectile path: objects beyond that stop are occluded.
            let query =
                crate::projectile_world::query(u32::MAX, 1, start, shot.motion.position, objects)?;
            if let Some(object) = query.selected {
                world_hits.push(WorldHit {
                    object,
                    serial: shot.serial,
                    motion: crate::damage_impulse::Motion {
                        current: query.point,
                        previous: shot.previous,
                    },
                    older: shot.older,
                    point: query.point,
                    damage: self.definition.direct_damage,
                });
                self.world_hits += 1;
                self.impacts.push((query.point, 0.7));
                impacts.push(Impact::World(query));
            } else {
                match event {
                    Event::Flying => {
                        live.push(shot);
                        continue;
                    }
                    Event::Impact(point) => {
                        self.ground_hits += 1;
                        self.impacts.push((point, 0.7));
                        impacts.push(Impact::Ground(point));
                    }
                    Event::Expired | Event::Outside => {}
                }
            }
            self.retired += 1;
        }
        self.shots = live;
        Ok(Frame {
            fired,
            impacts,
            world_hits,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Catalog, Terrain) {
        let definition = Projectile {
            id: 17,
            class: 38,
            flags: 0x20330,
            interval: 0.5,
            aim_response: 0.,
            launch_speed: 600.,
            lifetime: 999.,
            mass: 1.,
            gravity_mass: 0.,
            range: 700.,
            direct_damage: 12.,
            effects: Default::default(),
        };
        let catalog = Catalog {
            records: Vec::new(),
            commands_end: 0,
            projectiles: [(17, definition)].into_iter().collect(),
            icons: [(
                1,
                giants_formats::weapons::Icon {
                    id: 1,
                    records: Vec::new(),
                    description: None,
                    projectile: 17,
                    weight: 1,
                },
            )]
            .into_iter()
            .collect(),
            unprojected_records: 0,
            ignored_records: 0,
            inactive_records: 0,
        };
        let terrain = Terrain {
            source_flags: 3,
            width: 4,
            height: 2,
            origin: [0.; 2],
            spacing: 10.,
            height_bounds: [0.; 2],
            fallback_texture: String::new(),
            heights: vec![0.; 8],
            colors: vec![0; 8],
            tags: vec![6; 8],
            consumed_bytes: 0,
            opaque_tail: Vec::new(),
        };
        (catalog, terrain)
    }
    #[test]
    fn held_release_residual_cadence_and_reset_with_zero_initial_ammo() {
        let (catalog, terrain) = fixture();
        let mut p = Pistol::from_initial(Initial::non_base(true), &catalog).unwrap();
        let mut fired = Vec::new();
        for tick in 0..480 {
            if p.step(
                1. / 120.,
                [1., 5., 5.],
                [29., 5., 5.],
                (60..300).contains(&tick),
                &terrain,
                &[],
            )
            .unwrap()
            .fired
            {
                fired.push(tick);
            }
            assert_eq!(p.fired, p.retired + p.shots.len() as u32);
            let identities: std::collections::BTreeSet<_> =
                p.shots.iter().map(|s| s.serial).collect();
            assert_eq!(identities.len(), p.shots.len());
            assert!(identities.iter().all(|id| *id > 0 && *id <= p.fired));
        }
        assert_eq!(fired, vec![60, 121, 181, 241]);
        assert_eq!(p.retired, 4);
        p.reset();
        assert_eq!(p.fired, 0);
        assert_eq!(p.cooldown, 0.);
        assert!(p.shots.is_empty() && p.impacts.is_empty());
    }
    #[test]
    fn selected_equipment_does_not_grant_other_weapons_the_pistol_ammo_rule() {
        let (catalog, _) = fixture();
        let mut e = Initial::non_base(true);
        e.selected = 1;
        assert!(Pistol::from_initial(e, &catalog).is_err());
        e.selected = -1;
        assert!(Pistol::from_initial(e, &catalog).is_err());
        e.selected = 0;
        e.special = 2;
        assert!(Pistol::from_initial(e, &catalog).is_err());
        e.special = -1;
        e.utility[0] = 3;
        assert!(Pistol::from_initial(e, &catalog).is_err());
    }
    #[test]
    fn supplied_world_geometry_stops_and_retires_a_shot_without_damage() {
        let (catalog, terrain) = fixture();
        let mut p = Pistol::from_initial(Initial::non_base(true), &catalog).unwrap();
        let triangles = vec![
            [[8., 0., 0.], [8., 10., 0.], [8., 0., 10.]],
            [[8., 0., 10.], [8., 10., 0.], [8., 0., 0.]],
            [[8., 10., 0.], [8., 10., 10.], [8., 0., 10.]],
            [[8., 0., 10.], [8., 10., 10.], [8., 10., 0.]],
        ];
        let object = crate::projectile_world::Object {
            id: 2,
            parent: 0,
            active: 0,
            personality: 0x20000,
            bounds: crate::volume_queries::Bounds {
                min: [7.9, 0., 0.],
                max: [8.1, 10., 10.],
            },
            mesh: crate::object_segment::Mesh::new(2, triangles).unwrap(),
        };
        let frame = p
            .step(0.05, [1., 5., 5.], [29., 5., 5.], true, &terrain, &[object])
            .unwrap();
        assert!(matches!(frame.impacts.as_slice(),[Impact::World(q)] if q.selected==Some(2)));
        let hit = &frame.world_hits[0];
        assert_eq!((hit.object, hit.serial), (2, 1));
        assert_eq!(hit.damage, 12.);
        assert_eq!(hit.motion.previous, [1., 5., 5.]);
        assert_eq!(hit.older, [1., 5., 5.]);
        assert_eq!(hit.motion.current, hit.point);
        assert!(hit.point[0] >= 7.9 && hit.point[0] <= 8.1);
        assert!(p.shots.is_empty());
        assert_eq!((p.fired, p.retired, p.world_hits), (1, 1, 1));
    }

    fn wall(x: f32) -> crate::projectile_world::Object {
        let a = [x, 0., -10.];
        let b = [x, 10., -10.];
        let c = [x, 0., 10.];
        let d = [x, 10., 10.];
        crate::projectile_world::Object {
            id: 2,
            parent: 0,
            active: 0,
            personality: 0x20000,
            bounds: crate::volume_queries::Bounds {
                min: [x - 0.1, 0., -10.],
                max: [x + 0.1, 10., 10.],
            },
            mesh: crate::object_segment::Mesh::new(
                2,
                vec![[a, b, c], [c, b, a], [c, b, d], [d, b, c]],
            )
            .unwrap(),
        }
    }

    #[test]
    fn terrain_occludes_objects_beyond_its_contact_in_the_same_frame() {
        let (catalog, terrain) = fixture();
        let mut pistol = Pistol::from_initial(Initial::non_base(true), &catalog).unwrap();
        // This flight crosses z=0 at x=11, then reaches the supplied wall at x=18.
        // Prove the wall really is hittable along the unshortened movement ray.
        let mut control = Shot::new(&pistol.definition, [1., 5., 5.], [29., 5., -9.]).unwrap();
        assert!(matches!(
            control.step(&terrain, 0.05).unwrap(),
            Event::Impact(_)
        ));
        let objects = [wall(18.)];
        assert_eq!(
            crate::projectile_world::query(
                u32::MAX,
                1,
                [1., 5., 5.],
                control.segment_end,
                &objects
            )
            .unwrap()
            .selected,
            Some(2)
        );
        let frame = pistol
            .step(0.05, [1., 5., 5.], [29., 5., -9.], true, &terrain, &objects)
            .unwrap();
        assert!(
            matches!(frame.impacts.as_slice(), [Impact::Ground(point)] if point[2].abs() < 0.00001)
        );
        assert_eq!(
            (pistol.world_hits, pistol.ground_hits, pistol.retired),
            (0, 1, 1)
        );
        assert!(pistol.shots.is_empty());
    }

    #[test]
    fn object_before_terrain_still_takes_precedence() {
        let (catalog, terrain) = fixture();
        let mut pistol = Pistol::from_initial(Initial::non_base(true), &catalog).unwrap();
        let frame = pistol
            .step(
                0.05,
                [1., 5., 5.],
                [29., 5., -9.],
                true,
                &terrain,
                &[wall(8.)],
            )
            .unwrap();
        assert!(
            matches!(frame.impacts.as_slice(), [Impact::World(query)] if query.selected == Some(2) && (query.point[0] - 8.).abs() < 0.00001)
        );
        assert_eq!(
            (pistol.world_hits, pistol.ground_hits, pistol.retired),
            (1, 0, 1)
        );
        assert!(pistol.shots.is_empty());
    }
}
