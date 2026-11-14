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
mod source;
pub use source::OwnedHit;

pub struct Pistol {
    source_owner: Option<crate::resident_states::Id>,
    source_camera: Option<[f32; 3]>,
    emitted: std::collections::BTreeMap<u32, source::Emission>,
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
    /// Actual emitted partial shot; never a canonical state77 actor identity.
    /// Explicit synthetic/legacy hits have no such receipt.
    pub source: Option<OwnedHit>,
    pub object: u32,
    pub serial: u32,
    pub motion: crate::damage_impulse::Motion,
    pub older: [f32; 3],
    pub point: [f32; 3],
    pub damage: f32,
}

impl Pistol {
    pub fn source_camera(&mut self, position: [f32; 3]) -> Result<(), &'static str> {
        if !position.into_iter().all(f32::is_finite) {
            return Err("pistol source camera is nonfinite");
        }
        self.source_camera = Some(position);
        Ok(())
    }
    pub fn bind_source(&mut self, owner: crate::resident_states::Id) -> Result<(), &'static str> {
        if self.fired != 0
            || !self.shots.is_empty()
            || self.source_owner.is_some_and(|previous| previous != owner)
        {
            return Err("pistol source binding requires its original owner before firing");
        }
        self.source_owner = Some(owner);
        Ok(())
    }
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
            source_owner: None,
            source_camera: None,
            emitted: Default::default(),
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
        // A development reset is not a fresh Session/actor or firing epoch.
        self.source_owner = None;
        self.source_camera = None;
        self.emitted.clear();
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
        if self.source_owner.is_some() && self.source_camera.is_none() {
            return Err("bound pistol requires the actual source camera sample");
        }
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
            if let Some(owner) = self.source_owner {
                self.emitted.insert(
                    shot.serial,
                    source::Emission::new(owner, &self.definition, self.source_camera.unwrap()),
                );
            }
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
                let emitted = self.emitted.remove(&shot.serial);
                let mut hit = WorldHit {
                    source: None,
                    object,
                    serial: shot.serial,
                    motion: crate::damage_impulse::Motion {
                        current: query.point,
                        previous: shot.previous,
                    },
                    older: shot.older,
                    point: query.point,
                    damage: emitted
                        .as_ref()
                        .map_or(self.definition.direct_damage, source::Emission::damage),
                };
                hit.source = emitted.map(|emitted| emitted.finish(&hit));
                world_hits.push(hit);
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
            self.emitted.remove(&shot.serial);
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
                    player_type: None,
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

    fn source_owner(ticket: u64) -> crate::resident_states::Id {
        let seed = crate::state_storage::Seed::from_program_for(
            &giants_formats::class_program::parse(&[0x1e, 2, 255]).unwrap(),
            crate::state_storage::Profile::Debug,
            Precision::Double53,
        )
        .unwrap();
        crate::resident_states::Session::new(ticket)
            .register(2, &seed, &[], crate::object_lists::DirectFlags::default())
            .unwrap()
    }
    #[test]
    fn actual_contact_receipt_retains_shooter_and_refuses_changed_hit_or_late_binding() {
        let (catalog, terrain) = fixture();
        let owner = source_owner(711);
        for tamper in 0..4 {
            let mut pistol = Pistol::from_initial(Initial::non_base(true), &catalog).unwrap();
            pistol.bind_source(owner).unwrap();
            pistol.source_camera([1., 5., 5.]).unwrap();
            let mut frame = pistol
                .step(
                    0.05,
                    [1., 5., 5.],
                    [29., 5., 5.],
                    true,
                    &terrain,
                    &[wall(8.)],
                )
                .unwrap();
            let mut hit = frame.world_hits.pop().unwrap();
            assert!(pistol.bind_source(source_owner(712)).is_err());
            match tamper {
                1 => hit.damage += 1.,
                2 => hit.object += 1,
                3 => hit.motion.previous[0] += 1.,
                _ => {}
            }
            let result = hit.into_source();
            if tamper == 0 {
                let hit = result.unwrap();
                assert_eq!(
                    (hit.owner(), hit.serial(), hit.class(), hit.definition()),
                    (owner, 1, 38, 17)
                );
                assert_eq!(hit.damage(), 12.);
                assert_eq!(hit.point(), hit.motion().current);
            } else {
                assert!(result.is_err());
            }
        }
        let mut unbound = Pistol::from_initial(Initial::non_base(true), &catalog).unwrap();
        assert!(
            unbound
                .step(
                    0.05,
                    [1., 5., 5.],
                    [29., 5., 5.],
                    true,
                    &terrain,
                    &[wall(8.)]
                )
                .unwrap()
                .world_hits
                .pop()
                .unwrap()
                .into_source()
                .is_err()
        );
        unbound.reset();
        assert!(unbound.source_owner.is_none());
    }

    #[test]
    fn in_flight_source_keeps_emission_definition_and_camera_until_real_contact() {
        let (catalog, terrain) = fixture();
        let mut pistol = Pistol::from_initial(Initial::non_base(true), &catalog).unwrap();
        pistol.bind_source(source_owner(713)).unwrap();
        assert!(
            pistol
                .step(
                    0.01,
                    [1., 5., 5.],
                    [29., 5., 5.],
                    true,
                    &terrain,
                    &[wall(8.)]
                )
                .is_err()
        );
        pistol.source_camera([1., 5., 6.]).unwrap();
        let first = pistol
            .step(
                0.01,
                [1., 5., 5.],
                [29., 5., 5.],
                true,
                &terrain,
                &[wall(8.)],
            )
            .unwrap();
        assert!(first.fired && first.world_hits.is_empty());
        pistol.definition.direct_damage = 999.;
        pistol.definition.class = 47;
        pistol.definition.range = 1.;
        pistol.source_camera([100., 100., 100.]).unwrap();
        let hit = pistol
            .step(
                0.01,
                [1., 5., 5.],
                [29., 5., 5.],
                false,
                &terrain,
                &[wall(8.)],
            )
            .unwrap()
            .world_hits
            .pop()
            .unwrap()
            .into_source()
            .unwrap();
        assert_eq!(
            (hit.damage(), hit.class(), hit.range(), hit.camera_origin()),
            (12., 38, 700., [1., 5., 6.])
        );
        assert!(pistol.emitted.is_empty());
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
