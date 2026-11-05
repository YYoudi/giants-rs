//! Ordered camera clipping over explicitly prepared world-space mesh triangles.
//! List membership, current model/part eligibility and posed vertices belong to
//! the caller. This does not certify the live native object list.
use crate::{arithmetic::Precision, player_world_collision::Object, volume_queries::Bounds};

#[derive(Default, Debug)]
pub struct Visits {
    pub candidates: u32,
    pub triangles: u32,
    pub inclusion_tests: u32,
    pub hits: u32,
}
pub fn clip(
    eye: &mut [f32; 3],
    target: [f32; 3],
    controlled: u32,
    objects: &[Object],
) -> Result<Visits, String> {
    if controlled == 0 || !eye.iter().copied().chain(target).all(f32::is_finite) {
        return Err("camera clip requires a controlled identity and finite segment".into());
    }
    let bounds = Bounds {
        min: std::array::from_fn(|i| eye[i].min(target[i])),
        max: std::array::from_fn(|i| eye[i].max(target[i])),
    };
    let mut visits = Visits::default();
    for object in objects {
        let o = object.fields;
        if o.id == controlled
            || o.parent == controlled
            || o.object_flags & 0x4000c000 != 0
            || o.collision_flags & 0x800 != 0
            || o.collision_flags & 0x1008 == 0
            || !o.has_model
        {
            continue;
        }
        if !crate::volume_queries::overlaps(bounds, o.bounds)
            .ok_or("nonfinite camera object bounds")?
        {
            continue;
        }
        visits.candidates += 1;
        for triangle in &object.triangles {
            visits.triangles += 1;
            let vertices = triangle.vertices;
            if !vertices.iter().flatten().all(|v| v.is_finite()) {
                return Err("nonfinite camera triangle".into());
            }
            let n = crate::platform_queries::platform_normal(vertices, Precision::Double53)?;
            let [nx, ny, nz] = n.map(f64::from);
            let [a, _, _] = vertices;
            let plane =
                -((nz * f64::from(a[2]) + ny * f64::from(a[1])) + f64::from(a[0]) * nx) as f32;
            let dot = |p: [f32; 3]| {
                ((nx * f64::from(p[0]) + nz * f64::from(p[2])) + ny * f64::from(p[1]))
                    + f64::from(plane)
            };
            let end = dot(target) as f32;
            if dot(*eye) * f64::from(end) >= 0. {
                continue;
            }
            let dx = f64::from(eye[0]) - f64::from(target[0]);
            let dy = f64::from(eye[1]) - f64::from(target[1]);
            let dz = f64::from(eye[2]) - f64::from(target[2]);
            let denominator = (dy * ny + dx * nx) + dz * nz;
            let fraction = if denominator == 0. {
                0.
            } else {
                -(f64::from(end) / denominator) as f32
            };
            let length = ((dx * dx + dy * dy) + dz * dz).sqrt();
            let fraction = f64::from(fraction).max(4. / length);
            let point = std::array::from_fn(|i| {
                ((f64::from(eye[i]) - f64::from(target[i])) * fraction + f64::from(target[i]))
                    as f32
            });
            if !point.into_iter().all(f32::is_finite) {
                return Err("camera clip point overflow".into());
            }
            visits.inclusion_tests += 1;
            if crate::segment_queries::inside(point, n, vertices, Precision::Double53)? {
                *eye = point;
                visits.hits += 1;
            }
        }
    }
    Ok(visits)
}

/// Same static terrain and prepared rigid meshes as the owning caller. Empty
/// meshes mean a known empty cohort, not a substitute for unavailable geometry.
pub struct World<'a, 't> {
    pub terrain: &'a crate::terrain_queries::StaticTerrainQuery<'t>,
    pub controlled: u32,
    pub objects: &'a [Object],
    pub visits: Visits,
}
impl<'a, 't> World<'a, 't> {
    pub fn new(
        terrain: &'a crate::terrain_queries::StaticTerrainQuery<'t>,
        controlled: u32,
        objects: &'a [Object],
    ) -> Self {
        Self {
            terrain,
            controlled,
            objects,
            visits: Visits::default(),
        }
    }
}
impl crate::mecc_camera_update::World for World<'_, '_> {
    fn height(&mut self, xy: [f32; 2]) -> Result<f32, String> {
        self.terrain
            .sample(xy)
            .map(|s| s.height)
            .map_err(str::to_owned)
    }
    fn clip(&mut self, eye: &mut [f32; 3], target: [f32; 3]) -> Result<(), String> {
        self.visits = clip(eye, target, self.controlled, self.objects)?;
        Ok(())
    }
}
