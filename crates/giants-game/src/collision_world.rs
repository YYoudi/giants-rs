//! Source-mesh/footprint adapter joining measured static ground queries.
//!
//! Footprint construction, model selection, source pose/bounds and object flags
//! remain explicit inputs. This is not the native movement/collision dispatcher.
use crate::{
    arithmetic::Precision,
    ground_contact,
    platform_queries::{self, PreparedPlatform},
    terrain_queries::{GroundSample, StaticTerrainQuery},
};
use giants_formats::{footprint::Bitmap, gbs::Mesh, gti::Terrain};

#[derive(Clone, Copy)]
pub struct SourcePose {
    pub basis: [f32; 9],
    pub position: [f32; 3],
}
impl SourcePose {
    /// Row-major source object transform, not a Bevy/GPU matrix.
    pub fn from_matrix(matrix: [f32; 16]) -> Result<Self, &'static str> {
        if !matrix.iter().all(|v| v.is_finite()) || matrix[12..] != [0., 0., 0., 1.] {
            return Err("unsupported source affine matrix");
        }
        Ok(Self {
            basis: [
                matrix[0], matrix[1], matrix[2], matrix[4], matrix[5], matrix[6], matrix[8],
                matrix[9], matrix[10],
            ],
            position: [matrix[3], matrix[7], matrix[11]],
        })
    }
    pub fn transform(
        self,
        point: [f32; 3],
        precision: Precision,
    ) -> Result<[f32; 3], &'static str> {
        if !point
            .iter()
            .chain(&self.basis)
            .chain(&self.position)
            .all(|v| v.is_finite())
        {
            return Err("non-finite source collider transform");
        }
        let r = |x| precision.round(x);
        let p = point.map(f64::from);
        let m = self.basis.map(f64::from);
        let sum = |a, b, c, d| r(r(r(a + b) + c) + d) as f32;
        let result = [
            sum(
                r(m[0] * p[0]),
                r(m[1] * p[1]),
                r(m[2] * p[2]),
                f64::from(self.position[0]),
            ),
            sum(
                r(m[4] * p[1]),
                r(m[3] * p[0]),
                r(m[5] * p[2]),
                f64::from(self.position[1]),
            ),
            sum(
                r(m[7] * p[1]),
                r(m[6] * p[0]),
                r(m[8] * p[2]),
                f64::from(self.position[2]),
            ),
        ];
        if !result.iter().all(|v| v.is_finite()) {
            return Err("source collider transform overflow");
        }
        Ok(result)
    }
}
#[derive(Clone, Copy)]
pub struct ColliderFields {
    pub id: u32,
    pub object_flags: u32,
    pub collision_flags: u32,
    pub bounds_xy: [[f32; 2]; 2],
}
/// Explicit primary collision model and matrix-cache profile.
/// The caller must establish primary-model selection and the source object flags.
pub struct CachedPrimary {
    pub id: u32,
    pub object_flags: u32,
    pub collision_flags: u32,
    pub bounds: [f32; 6],
    pub matrix: [f32; 12],
}
/// Explicit model bounds/slot and a source object whose affine cache is absent.
pub struct UncachedSource {
    pub id: u32,
    pub object_flags: u32,
    pub collision_flags: u32,
    pub models: crate::collision_bounds::Models,
    pub pose: crate::uncached_pose::Pose,
}
pub struct StaticCollider<'a> {
    fields: ColliderFields,
    footprint: Bitmap<'a>,
    triangles: Vec<[[f32; 3]; 3]>,
    precision: Precision,
}
impl<'a> StaticCollider<'a> {
    /// The bytes must describe an already constructed native footprint.
    /// Missing/unbuilt footprints are not silently replaced by a bounding box.
    pub fn from_mesh(
        fields: ColliderFields,
        mesh: &Mesh,
        pose: SourcePose,
        footprint: &'a [u8],
        precision: Precision,
    ) -> Result<Self, &'static str> {
        let positions = mesh
            .positions
            .iter()
            .map(|v| pose.transform(*v, precision))
            .collect::<Result<Vec<_>, _>>()?;
        Self::from_positions(fields, mesh, positions, footprint, precision)
    }
    /// Cached source transforms drive both the primary bounds writer and vertices.
    /// This path does not reuse the different uncached SourcePose arithmetic.
    pub fn from_cached_primary(
        input: CachedPrimary,
        mesh: &Mesh,
        footprint: &'a [u8],
        precision: Precision,
    ) -> Result<Self, &'static str> {
        if input.object_flags & 0x20 == 0 {
            return Err("source affine cache flag absent");
        }
        let bounds = crate::collision_bounds::cached(
            Some(crate::collision_bounds::Models {
                primary: input.bounds,
                auxiliary: None,
                slot: 254,
            }),
            input.matrix,
            precision,
        )?;
        let fields = ColliderFields {
            id: input.id,
            object_flags: input.object_flags,
            collision_flags: input.collision_flags,
            bounds_xy: [[bounds[0], bounds[1]], [bounds[3], bounds[4]]],
        };
        let positions = mesh
            .positions
            .iter()
            .map(|p| crate::footprint_builder::transform_cached(input.matrix, *p, precision))
            .collect::<Result<Vec<_>, _>>()?;
        Self::from_positions(fields, mesh, positions, footprint, precision)
    }
    /// Reconstruct basis and bounds without setting the object's cache bit.
    /// Model eligibility, active bounds slot, footprint and source fields remain supplied.
    pub fn from_uncached_source(
        input: UncachedSource,
        mesh: &Mesh,
        footprint: &'a [u8],
        precision: Precision,
    ) -> Result<Self, &'static str> {
        if input.object_flags & 0x20 != 0 {
            return Err("uncached source object has affine cache flag");
        }
        let bounds = crate::collision_bounds::uncached(Some(input.models), input.pose, precision)?;
        let fields = ColliderFields {
            id: input.id,
            object_flags: input.object_flags,
            collision_flags: input.collision_flags,
            bounds_xy: [[bounds[0], bounds[1]], [bounds[3], bounds[4]]],
        };
        let matrix = crate::uncached_pose::matrix(input.pose, precision)?;
        let positions = mesh
            .positions
            .iter()
            .map(|p| crate::uncached_pose::transform(matrix, *p, precision))
            .collect::<Result<Vec<_>, _>>()?;
        Self::from_positions(fields, mesh, positions, footprint, precision)
    }
    fn from_positions(
        fields: ColliderFields,
        mesh: &Mesh,
        positions: Vec<[f32; 3]>,
        footprint: &'a [u8],
        precision: Precision,
    ) -> Result<Self, &'static str> {
        if fields.id == 0
            || !fields.bounds_xy.iter().flatten().all(|v| v.is_finite())
            || fields.bounds_xy[0][0] > fields.bounds_xy[1][0]
            || fields.bounds_xy[0][1] > fields.bounds_xy[1][1]
        {
            return Err("invalid collider fields");
        }
        let footprint = Bitmap::parse(footprint).map_err(|_| "unsupported source footprint")?;
        let mut triangles = Vec::new();
        for part in &mesh.subobjects {
            if triangles.len() + part.triangles.len() > 1_000_000 {
                return Err("collision triangle budget exceeded");
            }
            for triangle in &part.triangles {
                let mut points = [[0.; 3]; 3];
                for (dst, index) in points.iter_mut().zip(triangle) {
                    let vertex = *mesh
                        .indices
                        .get(*index as usize)
                        .ok_or("collision index outside source table")?
                        as usize;
                    *dst = *positions
                        .get(vertex)
                        .ok_or("collision vertex outside source model")?;
                }
                triangles.push(points);
            }
        }
        Ok(Self {
            fields,
            footprint,
            triangles,
            precision,
        })
    }
    pub fn world_triangles(&self) -> &[[[f32; 3]; 3]] {
        &self.triangles
    }
    pub fn prepared(&self, point: [f32; 3]) -> Result<PreparedPlatform<'_>, &'static str> {
        let [x, y] = footprint_cell([point[0], point[1]], self.precision)?;
        Ok(PreparedPlatform {
            id: self.fields.id,
            object_flags: self.fields.object_flags,
            collision_flags: self.fields.collision_flags,
            bounds_xy: self.fields.bounds_xy,
            footprint_hit: self.footprint.platform_contains(x, y),
            triangles: &self.triangles,
        })
    }
}
/// Native shifted/floored 5-unit footprint grid, including precision at ties.
pub fn footprint_cell(point: [f32; 2], precision: Precision) -> Result<[i32; 2], &'static str> {
    let r = |x| precision.round(x);
    let mut out = [0; 2];
    for (dst, v) in out.iter_mut().zip(point) {
        if !v.is_finite() {
            return Err("non-finite footprint coordinate");
        }
        let scaled = r(r(f64::from(v) * f64::from(0.2f32)) + 0.5).floor();
        if scaled < f64::from(i32::MIN) || scaled > f64::from(i32::MAX) {
            return Err("footprint coordinate overflow");
        }
        *dst = scaled as i32;
    }
    Ok(out)
}
#[derive(Clone, Copy, Default)]
pub struct ContactMotion {
    pub vertical_velocity: f32,
    pub flags: u32,
    pub lock_z: bool,
}
pub struct GroundSurfaces {
    pub terrain: GroundSample,
    pub platform: platform_queries::PlatformSample,
}
pub struct GroundResult {
    pub surfaces: GroundSurfaces,
    pub contact: ground_contact::Contact,
}
pub struct StaticGroundWorld<'a> {
    terrain: StaticTerrainQuery<'a>,
    objects: Vec<StaticCollider<'a>>,
    precision: Precision,
}
impl<'a> StaticGroundWorld<'a> {
    pub fn new(
        terrain: &'a Terrain,
        inverse_spacing: f32,
        objects: Vec<StaticCollider<'a>>,
        precision: Precision,
    ) -> Result<Self, &'static str> {
        for o in &objects {
            if !matches!(
                (o.precision, precision),
                (Precision::Single24, Precision::Single24)
                    | (Precision::Double53, Precision::Double53)
            ) {
                return Err("collider precision differs from ground world");
            }
        }
        Ok(Self {
            terrain: StaticTerrainQuery::new(terrain, inverse_spacing, precision)?,
            objects,
            precision,
        })
    }
    pub fn precision(&self) -> Precision {
        self.precision
    }
    pub fn sample(&self, point: [f32; 3]) -> Result<GroundSurfaces, &'static str> {
        if !point.iter().all(|v| v.is_finite()) {
            return Err("non-finite world ground point");
        }
        let terrain = self.terrain.sample([point[0], point[1]])?;
        let candidates = self
            .objects
            .iter()
            .map(|o| o.prepared(point))
            .collect::<Result<Vec<_>, _>>()?;
        let platform = platform_queries::sample_prepared(point, &candidates, self.precision)?;
        Ok(GroundSurfaces { terrain, platform })
    }
    /// Query/contact composition after caller-supplied upstream object motion.
    pub fn resolve(
        &self,
        point: [f32; 3],
        vertical_offset: f32,
        motion: ContactMotion,
    ) -> Result<GroundResult, &'static str> {
        let surfaces = self.sample(point)?;
        let contact = ground_contact::resolve(
            ground_contact::Input {
                z: point[2],
                vertical_velocity: motion.vertical_velocity,
                flags: motion.flags,
                lock_z: motion.lock_z,
                ground: surfaces.terrain.height,
                plane: surfaces.terrain.plane,
                vertical_offset,
                platform: surfaces.platform.object.map(|_| surfaces.platform.height),
            },
            self.precision,
        )?;
        Ok(GroundResult { surfaces, contact })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn footprint_grid_ties_keep_precision_and_negative_coordinate_rules() {
        assert_eq!(
            footprint_cell([-2.5, 2.5], Precision::Single24).unwrap(),
            [0, 1]
        );
        assert_eq!(
            footprint_cell([-2.5, 2.5], Precision::Double53).unwrap(),
            [-1, 1]
        );
        assert!(footprint_cell([f32::NAN, 0.], Precision::Double53).is_err());
    }
    #[test]
    fn source_affine_transform_keeps_translation_and_axis_order() {
        let p = SourcePose::from_matrix([
            0., -2., 0., 10., 3., 0., 0., 20., 0., 0., 4., 30., 0., 0., 0., 1.,
        ])
        .unwrap();
        assert_eq!(
            p.transform([1., 2., 3.], Precision::Double53).unwrap(),
            [6., 23., 42.]
        );
    }
}
