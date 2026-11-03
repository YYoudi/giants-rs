//! Explicit posed-mesh projectile candidates. Native object list membership,
//! model selection and animated collision-grid traversal remain caller inputs.
use crate::arithmetic::Precision;
/// State6's object-list filter, with current fields and pointer-equivalent IDs
/// supplied by the caller. It does not establish membership in the live list.
pub fn projectile_candidate(
    projectile: u32,
    owner: u32,
    object: u32,
    parent: u32,
    active: u32,
    personality: u32,
) -> bool {
    projectile != object
        && owner != object
        && parent != owner
        && personality & 0x20000 != 0
        && active & 0x4000_c000 == 0
}
pub fn bounds(
    start: [f32; 3],
    end: [f32; 3],
    min: [f32; 3],
    max: [f32; 3],
) -> Result<Option<[f32; 3]>, &'static str> {
    let mut point = [0.; 3];
    Ok(bounds_into(start, end, min, max, &mut point)?.then_some(point))
}
/// Preserve attempted plane stores even when the final bounds result is false.
pub fn bounds_into(
    start: [f32; 3],
    end: [f32; 3],
    min: [f32; 3],
    max: [f32; 3],
    point: &mut [f32; 3],
) -> Result<bool, &'static str> {
    if !start
        .into_iter()
        .chain(end)
        .chain(min)
        .chain(max)
        .all(f32::is_finite)
        || (0..3).any(|i| min[i] > max[i])
    {
        return Err("invalid segment bounds");
    }
    let code = |p: [f32; 3]| -> u8 {
        (0..3).fold(0, |code, i| {
            code | if p[i] < min[i] {
                1 << (i * 2)
            } else if p[i] > max[i] {
                2 << (i * 2)
            } else {
                0
            }
        })
    };
    let a = code(start);
    if a == 0 {
        *point = start;
        return Ok(true);
    }
    let b = code(end);
    if b == 0 {
        *point = end;
        return Ok(true);
    }
    if a & b != 0 {
        return Ok(false);
    }
    let different = a ^ b;
    for axis in 0..3 {
        for (bit, plane) in [(1 << (axis * 2), min[axis]), (2 << (axis * 2), max[axis])] {
            if different & bit == 0 {
                continue;
            }
            let t = (f64::from(plane) - f64::from(start[axis]))
                / (f64::from(end[axis]) - f64::from(start[axis]));
            *point = std::array::from_fn(|i| {
                ((f64::from(end[i]) - f64::from(start[i])) * t + f64::from(start[i])) as f32
            });
            point[axis] = plane;
            if (0..3)
                .filter(|i| *i != axis)
                .all(|i| point[i] >= min[i] && point[i] <= max[i])
            {
                return Ok(true);
            }
        }
    }
    Ok(false)
}
#[derive(Clone)]
pub struct Mesh {
    pub id: u32,
    min: [f32; 3],
    max: [f32; 3],
    triangles: Vec<[[f32; 3]; 3]>,
    local_pose: Option<crate::contact_pose::Cached>,
}
impl Mesh {
    pub(crate) fn validate_current_flags(&self, active: u32) -> Result<(), &'static str> {
        if self.local_pose.is_some() && active & 0x20 == 0 {
            return Err("cached contact mesh contradicts current actor flags");
        }
        Ok(())
    }
    pub fn center(&self) -> [f32; 3] {
        std::array::from_fn(|i| (self.min[i] + self.max[i]) * 0.5)
    }
    pub fn new(id: u32, triangles: Vec<[[f32; 3]; 3]>) -> Result<Self, &'static str> {
        if id == 0
            || triangles.is_empty()
            || triangles.len() > 1_000_000
            || !triangles.iter().flatten().flatten().all(|v| v.is_finite())
        {
            return Err("invalid prepared projectile mesh");
        }
        let min = std::array::from_fn(|i| {
            triangles
                .iter()
                .flatten()
                .map(|p| p[i])
                .fold(f32::INFINITY, f32::min)
        });
        let max = std::array::from_fn(|i| {
            triangles
                .iter()
                .flatten()
                .map(|p| p[i])
                .fold(f32::NEG_INFINITY, f32::max)
        });
        Ok(Self {
            id,
            min,
            max,
            triangles,
            local_pose: None,
        })
    }
    /// Dedicated source collision mesh with its current deformed local buffer.
    /// A missing pose/buffer is never replaced with bind or render geometry.
    /// Selection, cache/animation advancement and list membership are upstream.
    /// The stored world box supports center inspection only; the caller still
    /// supplies the native actor broad-phase bounds separately.
    pub fn from_local_positions(
        id: u32,
        model: &giants_formats::gbs::Mesh,
        positions: &[[f32; 3]],
        pose: crate::contact_pose::Cached,
    ) -> Result<Self, &'static str> {
        if positions.len() != model.positions.len()
            || !positions.iter().flatten().all(|v| v.is_finite())
        {
            return Err("current contact positions do not match the source model");
        }
        let mut triangles = Vec::new();
        for part in &model.subobjects {
            if triangles.len() + part.triangles.len() > 1_000_000 {
                return Err("contact triangle budget exceeded");
            }
            for face in &part.triangles {
                let mut points = [[0.; 3]; 3];
                for (point, index) in points.iter_mut().zip(face) {
                    let vertex = *model
                        .indices
                        .get(usize::from(*index))
                        .ok_or("contact triangle source index absent")?;
                    *point = *positions
                        .get(usize::from(vertex))
                        .ok_or("contact triangle current vertex absent")?;
                }
                triangles.push(points);
            }
        }
        let mut mesh = Self::new(id, triangles)?;
        let local_min = mesh.min;
        let local_max = mesh.max;
        mesh.min = [f32::INFINITY; 3];
        mesh.max = [f32::NEG_INFINITY; 3];
        for corner in 0..8 {
            let world = pose.to_world(std::array::from_fn(|axis| {
                if corner & (1 << axis) == 0 {
                    local_min[axis]
                } else {
                    local_max[axis]
                }
            }))?;
            for (axis, value) in world.into_iter().enumerate() {
                mesh.min[axis] = mesh.min[axis].min(value);
                mesh.max[axis] = mesh.max[axis].max(value);
            }
        }
        mesh.local_pose = Some(pose);
        Ok(mesh)
    }
    pub fn hit(
        &self,
        mut start: [f32; 3],
        mut end: [f32; 3],
    ) -> Result<Option<[f32; 3]>, &'static str> {
        if let Some(pose) = self.local_pose {
            start = pose.to_local(start)?;
            end = pose.to_local(end)?;
        } else if bounds(start, end, self.min, self.max)?.is_none() {
            return Ok(None);
        }
        self.hit_local(start, end)?
            .map(|point| match self.local_pose {
                Some(pose) => pose.to_world(point),
                None => Ok(point),
            })
            .transpose()
    }
    /// Triangle traversal in the supplied current local buffer. Viewport picking
    /// already transformed its ray and must not apply the cached pose twice.
    pub fn hit_local(
        &self,
        start: [f32; 3],
        mut end: [f32; 3],
    ) -> Result<Option<[f32; 3]>, &'static str> {
        let mut hit = false;
        for face in &self.triangles {
            if let Some(point) =
                crate::segment_queries::triangle(start, end, *face, Precision::Double53)?
            {
                end = point;
                hit = true;
            }
        }
        Ok(hit.then_some(end))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn current_local_buffer_is_used_and_missing_or_stale_inputs_are_refused() {
        use giants_formats::gbs::{Material, Mesh as Source, Subobject};
        let model = Source {
            positions: vec![[0., -1., -1.], [0., 1., -1.], [0., 0., 1.]],
            indices: vec![0, 1, 2],
            uv: vec![],
            rgb: vec![],
            groups: vec![[0; 5]],
            subobjects: vec![Subobject {
                name: "synthetic".into(),
                group: 0,
                triangles: vec![[0, 1, 2]],
                vertex_start: 0,
                vertex_count: 3,
                material: Material {
                    texture: String::new(),
                    bump_texture: String::new(),
                    falloff: 0.,
                    blend: 0.,
                    flags: 0,
                    colors: [0; 4],
                    power: 0.,
                },
            }],
            normal_count: 0,
            normal_words: vec![],
            normal_indices: vec![],
        };
        let pose = crate::contact_pose::Cached::new(
            0x20,
            0x12,
            [
                2., 0., 0., 10., 0., 2., 0., 0., 0., 0., 2., 0., 0., 0., 0., 1.,
            ],
        )
        .unwrap();
        let positions = [[2., -1., -1.], [2., 1., -1.], [2., 0., 1.]];
        let mesh = Mesh::from_local_positions(1, &model, &positions, pose).unwrap();
        assert_eq!(
            mesh.hit([10., 0., 0.], [20., 0., 0.]).unwrap(),
            Some([14., 0., 0.])
        );
        assert_eq!(
            mesh.hit_local([0., 0., 0.], [5., 0., 0.]).unwrap(),
            Some([2., 0., 0.])
        );
        assert_eq!(mesh.center(), [14., 0., 0.]);
        assert!(mesh.validate_current_flags(0).is_err());
        assert!(Mesh::from_local_positions(1, &model, &positions[..2], pose).is_err());
        let mut invalid = positions;
        invalid[0][0] = f32::NAN;
        assert!(Mesh::from_local_positions(1, &model, &invalid, pose).is_err());
    }
    #[test]
    fn nearer_faces_win_even_when_submitted_after_far_faces() {
        let face = |x| [[x, -1., -1.], [x, 1., -1.], [x, 0., 1.]];
        let mesh = Mesh::new(1, vec![face(8.), face(2.)]).unwrap();
        assert_eq!(
            mesh.hit([0., 0., 0.], [10., 0., 0.]).unwrap(),
            Some([2., 0., 0.])
        );
        assert!(mesh.hit([0., 4., 0.], [10., 4., 0.]).unwrap().is_none());
        assert!(mesh.hit([0., 0., 0.], [1., 0., 0.]).unwrap().is_none());
    }
}
