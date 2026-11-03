//! Static prepared terrain receiver. Native clipping keeps strict boundary tests.
use crate::cinematic_shadow::Mapping;
use giants_formats::gti::Terrain;

#[derive(Clone, Copy, Debug)]
pub struct Vertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
}
impl Vertex {
    fn mix(self, next: Self, t: f64) -> Self {
        let mix = |a: f32, b: f32| ((1. - t) * f64::from(a) + t * f64::from(b)) as f32;
        Self {
            position: std::array::from_fn(|i| mix(self.position[i], next.position[i])),
            uv: std::array::from_fn(|i| mix(self.uv[i], next.uv[i])),
        }
    }
}
/// Triangle output in source coordinates. No RGB channel is invented: the
/// observed Flick projection texture stage uses texture and texture-factor.
pub struct Geometry {
    pub vertices: Vec<Vertex>,
    pub visited_cells: usize,
}
// Convex receiver footprint, converted to the native1/32-cell integer grid.
// Each edge keeps the extreme X reached over the current row, not only its
// intersection at the centre of that row.
fn spans(points: &[[i64; 2]]) -> Result<Vec<[i64; 3]>, &'static str> {
    struct Edge {
        index: usize,
        remaining: i64,
        step: i64,
        start: i64,
        extreme: i64,
        left: bool,
    }
    impl Edge {
        fn load(&mut self, p: &[[i64; 2]], top: usize) -> Result<(), &'static str> {
            while self.remaining == 0 && self.index != top {
                let from = p[self.index];
                self.index = if self.left {
                    (self.index + p.len() - 1) % p.len()
                } else {
                    (self.index + 1) % p.len()
                };
                let to = p[self.index];
                let x = from[0] << 8;
                self.extreme = if self.left {
                    self.extreme.min(x)
                } else {
                    self.extreme.max(x)
                };
                self.remaining = (to[1] >> 5) - (from[1] >> 5);
                if self.remaining < 0 {
                    return Err("shadow footprint is not monotonic convex");
                }
                if self.remaining == 0 {
                    continue;
                }
                self.step = ((to[0] - from[0]) << 13) / (to[1] - from[1]);
                self.start = x;
                if (self.left && self.step < 0) || (!self.left && self.step > 0) {
                    self.start += ((!from[1] & 31) * self.step) >> 5;
                    self.extreme = self.start;
                } else {
                    self.start -= ((from[1] & 31) * self.step) >> 5;
                }
            }
            Ok(())
        }
        fn advance(&mut self, p: &[[i64; 2]]) {
            self.remaining -= 1;
            self.start += self.step;
            self.extreme = self.start;
            if self.remaining == 0
                && ((self.left && self.step < 0) || (!self.left && self.step > 0))
            {
                self.extreme = p[self.index][0] << 8;
            }
        }
    }
    let bottom = (0..points.len())
        .min_by_key(|&i| points[i][1])
        .ok_or("empty footprint")?;
    let top = (0..points.len())
        .max_by_key(|&i| (points[i][1], std::cmp::Reverse(i)))
        .unwrap();
    let x = points[bottom][0] << 8;
    let mut left = Edge {
        index: bottom,
        remaining: 0,
        step: 0,
        start: x,
        extreme: x,
        left: true,
    };
    let mut right = Edge {
        left: false,
        ..left
    };
    let mut y = points[bottom][1] >> 5;
    let mut result = Vec::new();
    loop {
        left.load(points, top)?;
        right.load(points, top)?;
        let rows = left.remaining.min(right.remaining);
        if rows == 0 {
            result.push([y, left.extreme >> 13, right.extreme >> 13]);
            break;
        }
        for _ in 0..rows {
            result.push([y, left.extreme >> 13, right.extreme >> 13]);
            y += 1;
            left.advance(points);
            right.advance(points);
            if result.len() > 16384 {
                return Err("shadow footprint row budget exceeded");
            }
        }
    }
    Ok(result)
}
fn distance(vertex: Vertex, plane: [f32; 3]) -> f64 {
    f64::from(vertex.uv[1]) * f64::from(plane[1])
        + f64::from(vertex.uv[0]) * f64::from(plane[0])
        + f64::from(plane[2])
}
fn clip(input: &[Vertex], plane: [f32; 3]) -> Vec<Vertex> {
    if input.len() < 3 {
        return Vec::new();
    }
    let mut output = Vec::with_capacity(input.len() + 2);
    let mut previous = *input.last().unwrap();
    let mut before = distance(previous, plane) as f32;
    for &current in input {
        let after = distance(current, plane);
        // The crossing test distinguishes negative values; the admission test
        // is strictly positive. A zero boundary is not silently made inclusive.
        if (before < 0.) != (after < 0.) {
            output.push(previous.mix(current, f64::from(before) / (f64::from(before) - after)));
        }
        if after > 0. {
            output.push(current);
        }
        previous = current;
        before = after as f32;
    }
    output
}
/// Native XY-only UV calculation; height is used for drawing the receiver.
fn triangle(
    points: [[f32; 3]; 3],
    mapping: &Mapping,
    clip_required: bool,
) -> Result<Vec<Vertex>, &'static str> {
    let m = &mapping.world_to_texture.values;
    if points.iter().flatten().chain(m).any(|v| !v.is_finite()) {
        return Err("non-finite shadow terrain triangle");
    }
    let mut vertices = points
        .map(|position| Vertex {
            position,
            uv: [0, 4].map(|i| {
                (f64::from(position[0]) * f64::from(m[i])
                    + f64::from(position[1]) * f64::from(m[i + 1])
                    + f64::from(m[i + 3])) as f32
            }),
        })
        .to_vec();
    if vertices.iter().flat_map(|v| v.uv).any(|v| !v.is_finite()) {
        return Err("shadow terrain UV overflow");
    }
    // Native fast path submits the original triangle when every UV is inside.
    if !clip_required {
        return Ok(vertices);
    }
    for plane in [[1., 0., 0.], [0., 1., 0.], [-1., 0., 1.], [0., -1., 1.]] {
        vertices = clip(&vertices, plane);
    }
    let mut triangles = Vec::new();
    for i in 1..vertices.len().saturating_sub(1) {
        triangles.extend([vertices[0], vertices[i], vertices[i + 1]]);
    }
    Ok(triangles)
}
/// Input tags must be the prepared native grid, including the established7→4
/// conversion. This inspection profile includes the whole static grid.
pub fn project(terrain: &Terrain, mapping: &Mapping) -> Result<Geometry, &'static str> {
    project_in_bounds(
        terrain,
        mapping,
        [
            0,
            terrain.width.saturating_sub(1),
            0,
            terrain.height.saturating_sub(1),
        ],
    )
}
/// Bounds established by the exact-debug prepared terrain loader. Dynamic
/// changes to the world rectangle and runtime height modifiers remain separate.
pub fn project_loaded(terrain: &Terrain, mapping: &Mapping) -> Result<Geometry, &'static str> {
    project_in_bounds(
        terrain,
        mapping,
        [
            0,
            terrain.width.saturating_sub(2),
            0,
            terrain.height.saturating_sub(2),
        ],
    )
}
/// Explicit native world rectangle, in prepared grid coordinates, ordered
/// minimum X, maximum X, minimum Y, maximum Y. No placement is adjusted to fit.
pub fn project_in_bounds(
    terrain: &Terrain,
    mapping: &Mapping,
    bounds: [usize; 4],
) -> Result<Geometry, &'static str> {
    let count = terrain
        .width
        .checked_mul(terrain.height)
        .ok_or("shadow grid overflow")?;
    if terrain.width < 2
        || terrain.height < 2
        || count > 262144
        || terrain.heights.len() != count
        || terrain.tags.len() != count
        || terrain.tags.iter().any(|tag| *tag > 6)
        || !terrain.spacing.is_finite()
        || terrain.spacing <= 0.
        || terrain
            .origin
            .iter()
            .chain(mapping.footprint.iter().flatten())
            .any(|v| !v.is_finite())
    {
        return Err("shadow receiver grid outside profile");
    }
    if bounds[0] > bounds[1]
        || bounds[2] > bounds[3]
        || bounds[1] >= terrain.width
        || bounds[3] >= terrain.height
    {
        return Err("shadow receiver bounds outside prepared grid");
    }
    let mut result = Geometry {
        vertices: Vec::new(),
        visited_cells: 0,
    };
    let inverse = (1. / f64::from(terrain.spacing)) as f32;
    let mut grid_points = Vec::with_capacity(4);
    for from in mapping.footprint {
        let mut point = from;
        for axis in 0..2 {
            let grid = ((f64::from(from[axis]) - f64::from(terrain.origin[axis]))
                * f64::from(inverse)) as f32;
            if !grid.is_finite() || grid.abs() > 32768. {
                return Err("shadow receiver grid conversion outside profile");
            }
            point[axis] = grid;
        }
        grid_points.push(point);
    }
    // Original world-boundary clipping precedes conversion to integer cells.
    for (axis, sign, bound) in [
        (0, 1., -(bounds[0] as f32)),
        (1, 1., -(bounds[2] as f32)),
        (0, -1., bounds[1] as f32),
        (1, -1., bounds[3] as f32),
    ] {
        if grid_points.is_empty() {
            return Ok(result);
        }
        let mut output = Vec::new();
        let mut previous = *grid_points.last().unwrap();
        let dist = |p: [f32; 3]| f64::from(p[axis]) * sign + f64::from(bound);
        let mut before = dist(previous) as f32;
        for current in grid_points {
            let after = dist(current);
            if (before < 0.) != (after < 0.) {
                let t = f64::from(before) / (f64::from(before) - after);
                output.push(std::array::from_fn(|i| {
                    ((1. - t) * f64::from(previous[i]) + t * f64::from(current[i])) as f32
                }));
            }
            if after > 0. {
                output.push(current);
            }
            previous = current;
            before = after as f32;
        }
        grid_points = output;
    }
    if grid_points.len() < 3 {
        return Ok(result);
    }
    let fixed = grid_points
        .iter()
        .map(|p| [0, 1].map(|axis| (f64::from(p[axis]) * 32.).trunc() as i64))
        .collect::<Vec<_>>();
    for [y, x0, x1] in spans(&fixed)? {
        if y < 0 || y >= terrain.height as i64 - 1 {
            continue;
        }
        let y = y as usize;
        for x in x0.max(0)..=x1.min(terrain.width as i64 - 2) {
            let x = x as usize;
            result.visited_cells += 1;
            if result.visited_cells > 16384 {
                return Err("shadow receiver cell budget exceeded");
            }
            let index = y * terrain.width + x;
            let orders: &[[usize; 3]] = match terrain.tags[index] {
                0 | 7 => continue,
                1 => &[[2, 1, 0]],
                2 => &[[1, 2, 3]],
                3 => &[[3, 0, 2]],
                4 => &[[0, 3, 1]],
                5 => &[[0, 3, 1], [3, 0, 2]],
                6 => &[[2, 1, 0], [1, 2, 3]],
                _ => return Err("shadow receiver tag outside native table"),
            };
            let indices = [
                index,
                index + terrain.width,
                index + 1,
                index + terrain.width + 1,
            ];
            let mut corners = [[0.; 3]; 4];
            for i in 0..4 {
                corners[i] = terrain
                    .position(indices[i])
                    .map_err(|_| "shadow terrain position unavailable")?;
            }
            let m = &mapping.world_to_texture.values;
            let clip_required = corners.iter().any(|p| {
                [0, 4].iter().any(|&i| {
                    let uv = (f64::from(p[0]) * f64::from(m[i])
                        + f64::from(p[1]) * f64::from(m[i + 1])
                        + f64::from(m[i + 3])) as f32;
                    !(0. ..=1.).contains(&uv)
                })
            });
            for order in orders {
                let points = order.map(|i| corners[i]);
                if points.iter().any(|p| p[2] == -4096.) {
                    return Err("shadow triangle references sentinel height");
                }
                result
                    .vertices
                    .extend(triangle(points, mapping, clip_required)?);
                if result.vertices.len() > 900_000 {
                    return Err("shadow receiver vertex budget exceeded");
                }
            }
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::procedural_aim::Matrix;
    #[test]
    fn outside_and_unclipped_triangles_preserve_their_meaning() {
        let mapping = Mapping::prepare(Matrix::identity(), [0., 0., 1., 1.]).unwrap();
        assert!(
            triangle([[2., 2., 0.], [3., 3., 0.], [2., 3., 0.]], &mapping, true)
                .unwrap()
                .is_empty()
        );
        let p = [[0., 0., 2.], [1., 1., 3.], [0., 1., 4.]];
        let got = triangle(p, &mapping, false).unwrap();
        assert_eq!(got.len(), 3);
        assert_eq!(got[0].position, p[0]);
        assert_eq!(got[2].uv, [0., 0.]);
        assert!(triangle([[f32::NAN; 3]; 3], &mapping, true).is_err());
    }
}
