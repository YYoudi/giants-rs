//! Finite segment/triangle and single static-terrain-cell queries, source Z up.
//! No character volume, swept collision response or object broad phase.
use crate::{arithmetic::Precision, platform_queries::platform_normal};
use giants_formats::gti::Terrain;

pub fn triangle(
    start: [f32; 3],
    end: [f32; 3],
    vertices: [[f32; 3]; 3],
    precision: Precision,
) -> Result<Option<[f32; 3]>, &'static str> {
    if !start
        .iter()
        .chain(&end)
        .chain(vertices.iter().flatten())
        .all(|v| v.is_finite())
    {
        return Err("non-finite segment geometry");
    }
    let r = |v| precision.round(v);
    let normal = platform_normal(vertices, precision)?;
    let dot =
        |v: [f32; 3]| {
            r(r(r(f64::from(normal[0]) * f64::from(v[0]))
                + r(f64::from(normal[2]) * f64::from(v[2])))
                + r(f64::from(normal[1]) * f64::from(v[1])))
        };
    let plane = (-dot(vertices[0])) as f32;
    let first = r(dot(start) + f64::from(plane)) as f32;
    let last = r(dot(end) + f64::from(plane));
    if !plane.is_finite() || !first.is_finite() || !last.is_finite() {
        return Err("segment plane overflow");
    }
    // Strict sign change: coplanar segments and endpoints on the plane miss.
    if r(last * f64::from(first)) >= 0. {
        return Ok(None);
    }
    let delta = std::array::from_fn::<_, 3, _>(|i| r(f64::from(end[i]) - f64::from(start[i])));
    let stored = delta.map(|v| v as f32);
    if !stored.iter().all(|v| v.is_finite()) {
        return Err("segment direction overflow");
    }
    // X stays wide for this product; Y/Z are spilled before theirs. Denominator
    // remains an x87 register until the division, unlike the visibility helper.
    let denominator = r(r(
        r(delta[0] * f64::from(normal[0])) + r(f64::from(stored[1]) * f64::from(normal[1]))
    ) + r(f64::from(stored[2]) * f64::from(normal[2])));
    if denominator == 0. {
        return Ok(None);
    }
    let fraction = (-r(f64::from(first) / denominator)) as f32;
    if !(0. ..=1.).contains(&fraction) {
        return Ok(None);
    }
    let point = std::array::from_fn(|i| {
        r(r(f64::from(stored[i]) * f64::from(fraction)) + f64::from(start[i])) as f32
    });
    if !point.iter().all(|v| v.is_finite()) {
        return Err("segment hit overflow");
    }
    Ok(inside(point, normal, vertices, precision)?.then_some(point))
}

fn inside(
    point: [f32; 3],
    normal: [f32; 3],
    vertices: [[f32; 3]; 3],
    precision: Precision,
) -> Result<bool, &'static str> {
    let r = |v| precision.round(v);
    let [x, y, z] = normal.map(f32::abs);
    let axes = if x <= y {
        if z < y { [0, 2] } else { [0, 1] }
    } else if z < x {
        [1, 2]
    } else {
        [0, 1]
    };
    let q = vertices.map(|v| axes.map(|i| r(f64::from(v[i]) - f64::from(point[i])) as f32));
    if !q.iter().flatten().all(|v| v.is_finite()) {
        return Err("segment inclusion overflow");
    }
    let mut odd = false;
    for (i, j) in [(0, 1), (2, 1), (0, 2)] {
        let [x, y] = q[i].map(f64::from);
        let [a, b] = q[j].map(f64::from);
        if (y >= 0.) != (b >= 0.)
            && ((x >= 0. && a >= 0.)
                || ((x >= 0. || a >= 0.) && r(x - r(r(r(a - x) * y) / r(b - y))) > 0.))
        {
            odd = !odd;
        }
    }
    Ok(odd)
}

pub struct StaticCellQuery<'a> {
    terrain: &'a Terrain,
    precision: Precision,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerrainHit {
    Triangle([i32; 2]),
    /// Native final Z<=0 gate after all visited cells miss. End is unchanged.
    BelowZero,
}
impl<'a> StaticCellQuery<'a> {
    pub fn new(terrain: &'a Terrain, precision: Precision) -> Result<Self, &'static str> {
        let count = terrain
            .width
            .checked_mul(terrain.height)
            .ok_or("segment grid size overflow")?;
        if terrain.width < 2
            || terrain.height < 2
            || count > 262144
            || terrain.heights.len() != count
            || terrain.tags.len() != count
            || !terrain.spacing.is_finite()
            || terrain.spacing <= 0.
            || !terrain
                .origin
                .iter()
                .chain(&terrain.heights)
                .all(|v| v.is_finite())
        {
            return Err("invalid static segment grid");
        }
        Ok(Self { terrain, precision })
    }
    pub(crate) fn terrain(&self) -> &Terrain {
        self.terrain
    }
    pub(crate) fn precision(&self) -> Precision {
        self.precision
    }
    /// Ordered native grid traversal, including the minor-axis neighbour test.
    /// A hit shortens end only when a triangle actually intersects the segment.
    pub fn segment(
        &self,
        start: [f32; 3],
        end: &mut [f32; 3],
        inverse_spacing: f32,
    ) -> Result<Option<TerrainHit>, &'static str> {
        self.segment_observed(start, end, inverse_spacing, |_| {})
    }
    /// Diagnostic visitor is monomorphized away for the ordinary query. It
    /// observes even out-of-grid cells, just as the native cell wrapper does.
    pub fn segment_observed(
        &self,
        start: [f32; 3],
        end: &mut [f32; 3],
        inverse_spacing: f32,
        mut visit: impl FnMut([i32; 2]),
    ) -> Result<Option<TerrainHit>, &'static str> {
        if !start.into_iter().chain(*end).all(f32::is_finite)
            || !inverse_spacing.is_finite()
            || inverse_spacing <= 0.
        {
            return Err("invalid terrain segment coordinates or reciprocal");
        }
        let coordinate = |point: [f32; 3]| -> Result<[i32; 2], &'static str> {
            let r = |v| self.precision.round(v);
            let mut result = [0; 2];
            for axis in 0..2 {
                let v = r(
                    r(f64::from(point[axis]) - f64::from(self.terrain.origin[axis]))
                        * f64::from(inverse_spacing),
                )
                .trunc();
                if !v.is_finite() || v < f64::from(i32::MIN) || v > f64::from(i32::MAX) {
                    return Err("terrain segment cell conversion outside signed32 profile");
                }
                result[axis] = v as i32;
            }
            Ok(result)
        };
        let mut cell = coordinate(start)?;
        let target = coordinate(*end)?;
        let delta = [
            i64::from(target[0]) - i64::from(cell[0]),
            i64::from(target[1]) - i64::from(cell[1]),
        ];
        let distance = delta.map(i64::abs);
        // Explicit resource refusal, never a fabricated miss/hit. Ordinary
        // projectile steps are far below this bound; signed loop overflow is
        // outside the acquired native traversal profile.
        if distance[0].max(distance[1]) > 524_287 {
            return Err("terrain segment exceeds bounded traversal profile");
        }
        let step = delta.map(|v| if v < 0 { -1 } else { 1 });
        let (major, minor) = if distance[0] > distance[1] {
            (0, 1)
        } else {
            (1, 0)
        };
        let mut error = 2 * distance[minor] - distance[major];
        let mut hit_end = *end;
        for _ in 0..=distance[major] {
            visit(cell);
            if self.cell(start, &mut hit_end, cell)? {
                *end = hit_end;
                return Ok(Some(TerrainHit::Triangle(cell)));
            }
            cell[major] = cell[major].wrapping_add(step[major]);
            if error >= 0 {
                visit(cell);
                if self.cell(start, &mut hit_end, cell)? {
                    *end = hit_end;
                    return Ok(Some(TerrainHit::Triangle(cell)));
                }
                cell[minor] = cell[minor].wrapping_add(step[minor]);
                error += 2 * (distance[minor] - distance[major]);
            } else {
                error += 2 * distance[minor];
            }
        }
        Ok((end[2] <= 0.).then_some(TerrainHit::BelowZero))
    }
    /// First native face hit in this cell shortens `end`; misses preserve it.
    /// This is not a nearest-hit search across cells or objects.
    pub fn cell(
        &self,
        start: [f32; 3],
        end: &mut [f32; 3],
        cell: [i32; 2],
    ) -> Result<bool, &'static str> {
        if !start.iter().chain(end.iter()).all(|v| v.is_finite()) {
            return Err("non-finite cell segment");
        }
        let t = self.terrain;
        let [x, y] = cell;
        if x < 0 || y < 0 || x as usize >= t.width - 1 || y as usize >= t.height - 1 {
            return Ok(false);
        }
        let (x, y) = (x as usize, y as usize);
        let orders: &[[usize; 3]] = match t.tags[y * t.width + x] & 7 {
            1 => &[[1, 2, 0]],
            2 => &[[2, 1, 3]],
            3 => &[[3, 0, 1]],
            4 => &[[0, 3, 2]],
            5 => &[[3, 0, 1], [0, 3, 2]],
            6 => &[[1, 2, 0], [2, 1, 3]],
            _ => &[],
        };
        let r = |v| self.precision.round(v);
        let mut corners = [[0.; 3]; 4];
        for (i, [cx, cy]) in [[x, y], [x + 1, y], [x, y + 1], [x + 1, y + 1]]
            .into_iter()
            .enumerate()
        {
            corners[i] = [
                r(r(cx as f64 * f64::from(t.spacing)) + f64::from(t.origin[0])) as f32,
                r(r(cy as f64 * f64::from(t.spacing)) + f64::from(t.origin[1])) as f32,
                t.heights[cy * t.width + cx],
            ];
        }
        for order in orders {
            if let Some(point) = triangle(start, *end, order.map(|i| corners[i]), self.precision)? {
                *end = point;
                return Ok(true);
            }
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn empty_grid() -> Terrain {
        Terrain {
            source_flags: 2,
            width: 4,
            height: 4,
            origin: [0.; 2],
            spacing: 1.,
            height_bounds: [0.; 2],
            fallback_texture: String::new(),
            heights: vec![0.; 16],
            colors: vec![0; 16],
            tags: vec![0; 16],
            consumed_bytes: 0,
            opaque_tail: vec![],
        }
    }
    #[test]
    fn native_neighbour_order_and_below_zero_fallback_are_distinct() {
        let grid = empty_grid();
        for precision in [Precision::Single24, Precision::Double53] {
            let query = StaticCellQuery::new(&grid, precision).unwrap();
            let mut end = [0.25, 0.5, 2.];
            let mut visits = vec![];
            assert_eq!(
                query
                    .segment_observed(end, &mut end, 1., |c| visits.push(c))
                    .unwrap(),
                None
            );
            assert_eq!(visits, [[0, 0], [0, 1]]);
            assert_eq!(end, [0.25, 0.5, 2.]);
            visits.clear();
            end = [2.2, 2.2, 2.];
            assert_eq!(
                query
                    .segment_observed([0.2, 0.2, 2.], &mut end, 1., |c| visits.push(c))
                    .unwrap(),
                None
            );
            assert_eq!(visits, [[0, 0], [0, 1], [1, 1], [1, 2], [2, 2], [2, 3]]);
            end = [0.25, 0.5, -0.0];
            assert_eq!(
                query.segment([0.25, 0.5, 1.], &mut end, 1.).unwrap(),
                Some(TerrainHit::BelowZero)
            );
            assert_eq!(end[2].to_bits(), (-0.0_f32).to_bits());
        }
    }
    #[test]
    fn invalid_or_unbounded_traversal_preserves_the_endpoint() {
        let grid = empty_grid();
        let query = StaticCellQuery::new(&grid, Precision::Double53).unwrap();
        let mut end = [1e9, 0., 1.];
        let original = end;
        assert!(query.segment([0.; 3], &mut end, 1.).is_err());
        assert_eq!(end, original);
        assert!(query.segment([f32::NAN; 3], &mut end, 1.).is_err());
        assert_eq!(end, original);
        assert!(query.segment([0.; 3], &mut end, 0.).is_err());
        assert_eq!(end, original);
    }
    #[test]
    fn plane_endpoints_coplanar_and_degenerate_are_not_hits() {
        let t = [[0., 0., 0.], [10., 0., 0.], [0., 10., 0.]];
        for precision in [Precision::Single24, Precision::Double53] {
            assert_eq!(
                triangle([1., 1., 1.], [1., 1., -1.], t, precision).unwrap(),
                Some([1., 1., 0.])
            );
            for (a, b) in [
                ([1., 1., 0.], [1., 1., -1.]),
                ([1., 1., 1.], [1., 1., 0.]),
                ([1., 1., 0.], [2., 2., 0.]),
            ] {
                assert!(triangle(a, b, t, precision).unwrap().is_none());
            }
            assert!(
                triangle([0., 0., 1.], [0., 0., -1.], [[0.; 3]; 3], precision)
                    .unwrap()
                    .is_none()
            );
        }
    }
    #[test]
    fn invalid_geometry_is_explicit() {
        assert!(
            triangle(
                [f32::NAN, 0., 0.],
                [0.; 3],
                [[0.; 3]; 3],
                Precision::Double53
            )
            .is_err()
        );
    }
}
