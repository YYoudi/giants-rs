//! Straight, zero-gravity projectile slice with development terrain traversal.
//! Source cell/triangle tests are reused; native object broad phase is not implied.
use crate::{arithmetic::Precision, mecc_motion::Motion, segment_queries::StaticCellQuery};
use giants_formats::{gti::Terrain, weapons::Projectile};
pub struct Shot {
    /// Acquired mode2 orientation; translation follows the actual motion below.
    pub launch_matrix: [f32; 16],
    /// Ticket-local runtime identity; not an original binary object address.
    pub serial: u32,
    pub origin: [f32; 3],
    /// Unshortened movement segment, before the development terrain query.
    pub segment_end: [f32; 3],
    pub motion: Motion,
    pub remaining: f32,
}
pub enum Event {
    Flying,
    Impact([f32; 3]),
    Expired,
    Outside,
}
impl Shot {
    pub fn new(p: &Projectile, origin: [f32; 3], target: [f32; 3]) -> Result<Self, &'static str> {
        if p.flags & !0x20330 != 0
            || p.gravity_mass != 0.
            || ![p.mass, p.lifetime, p.launch_speed, p.range]
                .into_iter()
                .all(f32::is_finite)
            || p.mass <= 0.
            || p.lifetime <= 0.
            || p.range <= 0.
        {
            return Err("projectile outside straight zero-gravity development profile");
        }
        let launch = crate::projectile_launch::Straight::new(origin, target, p.launch_speed)
            .map_err(|_| "projectile outside acquired straight launch profile")?;
        let v = launch.velocity;
        Ok(Self {
            launch_matrix: launch.matrix.values,
            serial: 0,
            origin,
            segment_end: origin,
            motion: Motion {
                position: origin,
                velocity: v,
                previous_velocity: v,
                force: [0.; 3],
                mass: p.mass,
                speed_limit: 0.,
                force_time_limit: 0.,
                stored_dt: 0.,
            },
            remaining: p.lifetime,
        })
    }
    pub fn world_matrix(&self) -> [f32; 16] {
        let mut matrix = self.launch_matrix;
        for (axis, value) in self.motion.position.into_iter().enumerate() {
            matrix[axis * 4 + 3] = value;
        }
        matrix
    }
    pub fn step(&mut self, terrain: &Terrain, dt: f32) -> Result<Event, &'static str> {
        let start = self.motion.position;
        self.motion =
            crate::mecc_motion::integrate(self.motion, dt, false, false, Precision::Double53)?;
        let mut end = self.motion.position;
        self.segment_end = end;
        let q = StaticCellQuery::new(terrain, Precision::Double53)?;
        let cells = |p: [f32; 3]| {
            [
                ((p[0] - terrain.origin[0]) / terrain.spacing).floor() as i32,
                ((p[1] - terrain.origin[1]) / terrain.spacing).floor() as i32,
            ]
        };
        let a = cells(start);
        let b = cells(end);
        let lo = [a[0].min(b[0]).max(0), a[1].min(b[1]).max(0)];
        let hi = [
            a[0].max(b[0]).min(terrain.width as i32 - 2),
            a[1].max(b[1]).min(terrain.height as i32 - 2),
        ];
        if i64::from((hi[0] - lo[0] + 1).max(0)) * i64::from((hi[1] - lo[1] + 1).max(0)) > 4096 {
            return Err("projectile terrain candidate budget");
        }
        let mut hit = false;
        for y in lo[1]..=hi[1] {
            for x in lo[0]..=hi[0] {
                hit |= q.cell(start, &mut end, [x, y])?;
            }
        }
        if hit {
            self.motion.position = end;
            return Ok(Event::Impact(end));
        }
        self.remaining = (f64::from(self.remaining) - f64::from(dt)) as f32;
        if self.remaining < 0. {
            return Ok(Event::Expired);
        }
        if b[0] < 0
            || b[1] < 0
            || b[0] >= terrain.width as i32 - 1
            || b[1] >= terrain.height as i32 - 1
        {
            return Ok(Event::Outside);
        }
        Ok(Event::Flying)
    }
}
