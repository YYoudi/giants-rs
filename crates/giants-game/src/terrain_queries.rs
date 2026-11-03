//! Static native terrain-height query. Source coordinates use Z up.
//!
//! This is the complete non-ripple path of the measured query, including its
//! fallback plane and height floor. It does not test object geometry/volumes.
use crate::arithmetic::Precision;
use giants_formats::gti::Terrain;

pub const HEIGHT_FLOOR: f32 = -61.1;

#[derive(Clone, Copy, Debug)]
pub struct GroundSample {
    /// Clamped cell visited even when the requested coordinates are outside.
    pub cell: [usize; 2],
    /// Native subtriangle selector (1..4), or zero for the fallback plane.
    pub triangle: u8,
    pub plane: [f32; 4],
    /// Native floor is returned for holes, low terrain and outside coordinates.
    pub height: f32,
    pub in_bounds: bool,
}

pub struct StaticTerrainQuery<'a> {
    terrain: &'a Terrain,
    inverse_spacing: f32,
    precision: Precision,
    classified_topology: bool,
}
impl<'a> StaticTerrainQuery<'a> {
    /// `inverse_spacing` is the float32 reciprocal held by the native world.
    /// Accepting it explicitly avoids inferring its initialization precision.
    /// This view deliberately excludes the native dynamic ripple branch.
    pub fn new(
        terrain: &'a Terrain,
        inverse_spacing: f32,
        precision: Precision,
    ) -> Result<Self, &'static str> {
        let count = terrain
            .width
            .checked_mul(terrain.height)
            .ok_or("grid size overflow")?;
        if terrain.width < 2
            || terrain.height < 2
            || count > 262144
            || terrain.heights.len() != count
            || terrain.tags.len() != count
            || !terrain.spacing.is_finite()
            || terrain.spacing <= 0.
            || !inverse_spacing.is_finite()
            || inverse_spacing <= 0.
            || !terrain
                .origin
                .iter()
                .chain(&terrain.heights)
                .all(|v| v.is_finite())
        {
            return Err("invalid static terrain query grid");
        }
        Ok(Self {
            terrain,
            inverse_spacing,
            precision,
            classified_topology: terrain.tags.iter().all(|tag| tag & 7 != 7),
        })
    }
    pub fn dimensions(&self) -> [usize; 2] {
        [self.terrain.width, self.terrain.height]
    }
    pub(crate) fn terrain(&self) -> &Terrain {
        self.terrain
    }
    pub fn precision(&self) -> Precision {
        self.precision
    }
    /// D physical consumers require the post-decode topology, not raw tag7.
    pub fn classified_topology(&self) -> bool {
        self.classified_topology
    }
    pub fn sample(&self, point: [f32; 2]) -> Result<GroundSample, &'static str> {
        if !point.iter().all(|v| v.is_finite()) {
            return Err("non-finite terrain query");
        }
        let r = |x| self.precision.round(x);
        let t = self.terrain;
        // Only X is spilled to float32 here. The Y register stays wide.
        let raw_x =
            r(r(f64::from(point[0]) - f64::from(t.origin[0])) * f64::from(self.inverse_spacing))
                as f32;
        let raw_y =
            r(r(f64::from(point[1]) - f64::from(t.origin[1])) * f64::from(self.inverse_spacing));
        if !raw_x.is_finite() || !raw_y.is_finite() {
            return Err("terrain coordinate overflow");
        }
        let outside = raw_x < 0.
            || raw_y < 0.
            || f64::from(raw_x) >= (t.width - 1) as f64
            || raw_y >= (t.height - 1) as f64;
        // Clamp only after crossing the outer edge, preserving the final cell's fraction.
        let gx = if f64::from(raw_x) >= (t.width - 1) as f64 {
            (t.width - 2) as f64
        } else {
            f64::from(raw_x).max(0.)
        };
        let gy = if raw_y >= (t.height - 1) as f64 {
            (t.height - 2) as f64
        } else {
            raw_y.max(0.)
        };
        let ix = gx as usize;
        let iy = gy as usize;
        let fx = f64::from(r(gx - ix as f32 as f64) as f32);
        let fy = r(gy - iy as f32 as f64);
        let tag = t.tags[iy * t.width + ix] & 7;
        let selector = match tag {
            1 | 2 | 6 => 1 + u8::from(fx > r(1. - fy)),
            3..=5 => 4 - u8::from(fy < fx),
            _ => 0,
        };
        let triangle = if tag < 5 && selector != tag {
            0
        } else {
            selector
        };
        let corner = |x: usize, y: usize| {
            [
                r(r(x as f64 * f64::from(t.spacing)) + f64::from(t.origin[0])) as f32,
                r(r(y as f64 * f64::from(t.spacing)) + f64::from(t.origin[1])) as f32,
                t.heights[y * t.width + x],
            ]
        };
        let corners = [
            corner(ix, iy),
            corner(ix + 1, iy),
            corner(ix, iy + 1),
            corner(ix + 1, iy + 1),
        ];
        let plane = match triangle {
            1 => triangle_plane([corners[1], corners[2], corners[0]], self.precision),
            2 => triangle_plane([corners[2], corners[1], corners[3]], self.precision),
            3 => triangle_plane([corners[3], corners[0], corners[1]], self.precision),
            4 => triangle_plane([corners[0], corners[3], corners[2]], self.precision),
            _ => [0., 0., 1., -HEIGHT_FLOOR],
        };
        if !plane.iter().all(|v| v.is_finite()) {
            return Err("terrain plane overflow");
        }
        let height = if outside {
            HEIGHT_FLOOR
        } else if plane[2] == 0. {
            0.
        } else {
            let sum = r(r(r(f64::from(plane[1]) * f64::from(point[1]))
                + r(f64::from(point[0]) * f64::from(plane[0])))
                + f64::from(plane[3]));
            let value = -r(sum / f64::from(plane[2]));
            if value < f64::from(HEIGHT_FLOOR) {
                HEIGHT_FLOOR
            } else {
                value as f32
            }
        };
        if !height.is_finite() {
            return Err("terrain height overflow");
        }
        Ok(GroundSample {
            cell: [ix, iy],
            triangle,
            plane,
            height,
            in_bounds: !outside,
        })
    }
}

/// Float32 edge spills, wide cross product, and original dot-product ordering.
fn triangle_plane(points: [[f32; 3]; 3], precision: Precision) -> [f32; 4] {
    let r = |x| precision.round(x);
    let [a, b, c] = points;
    let edge = |v: [f32; 3]| {
        std::array::from_fn::<_, 3, _>(|i| f64::from(r(f64::from(v[i]) - f64::from(a[i])) as f32))
    };
    let u = edge(b);
    let v = edge(c);
    let mut n = [
        r(r(v[2] * u[1]) - r(v[1] * u[2])),
        r(r(v[0] * u[2]) - r(u[0] * v[2])),
        r(r(u[0] * v[1]) - r(v[0] * u[1])),
    ];
    let length = r(r(r(r(n[2] * n[2]) + r(n[1] * n[1])) + r(n[0] * n[0])).sqrt());
    if length > 0. {
        let inverse = r(1. / length);
        n = n.map(|value| r(inverse * value));
    }
    let d =
        -r(r(r(n[0] * f64::from(a[0])) + r(n[2] * f64::from(a[2]))) + r(n[1] * f64::from(a[1])));
    [n[0] as f32, n[1] as f32, n[2] as f32, d as f32]
}

#[cfg(test)]
mod tests {
    use super::*;
    fn grid(tag: u8) -> Terrain {
        Terrain {
            source_flags: 3,
            width: 2,
            height: 2,
            origin: [0., 0.],
            spacing: 1.,
            height_bounds: [0., 10.],
            fallback_texture: String::new(),
            heights: vec![0., 0., 0., 10.],
            colors: vec![],
            tags: vec![tag, 0, 0, 0],
            consumed_bytes: 0,
            opaque_tail: vec![],
        }
    }
    #[test]
    fn saddle_selects_distinct_native_diagonals_and_holes() {
        let a = grid(5);
        let b = grid(6);
        let hole = grid(1);
        let query = |t| {
            StaticTerrainQuery::new(t, 1., Precision::Double53)
                .unwrap()
                .sample([0.25, 0.25])
                .unwrap()
        };
        assert_eq!(query(&a).height, 2.5);
        assert_eq!(query(&b).height, 0.);
        assert_eq!(query(&hole).triangle, 1);
        let q = StaticTerrainQuery::new(&hole, 1., Precision::Double53).unwrap();
        assert_eq!(q.sample([0.8, 0.8]).unwrap().triangle, 0);
        assert_eq!(q.sample([0.8, 0.8]).unwrap().height, HEIGHT_FLOOR);
        assert!(!q.sample([1., 0.5]).unwrap().in_bounds);
        assert!(!q.sample([-0.01, 0.5]).unwrap().in_bounds);
    }
    #[test]
    fn query_supplies_existing_contact_stage_without_rendering() {
        use crate::ground_contact::{self, CONTACT_BIT, Input};
        let terrain = grid(6);
        let sample = StaticTerrainQuery::new(&terrain, 1., Precision::Double53)
            .unwrap()
            .sample([0.25, 0.25])
            .unwrap();
        let contact = ground_contact::resolve(
            Input {
                z: sample.height + 0.05,
                vertical_velocity: -1.,
                flags: 0,
                lock_z: false,
                ground: sample.height,
                plane: sample.plane,
                vertical_offset: 0.,
                platform: None,
            },
            Precision::Double53,
        )
        .unwrap();
        assert_eq!(contact.z, sample.height);
        assert_eq!(contact.plane, sample.plane);
        assert_ne!(contact.flags & CONTACT_BIT, 0);
    }
    #[test]
    fn topology_seven_and_invalid_queries_are_explicit() {
        let mut t = grid(7);
        let q = StaticTerrainQuery::new(&t, 1., Precision::Double53).unwrap();
        assert_eq!(q.sample([0.5, 0.5]).unwrap().triangle, 0);
        assert!(q.sample([f32::NAN, 0.]).is_err());
        t.heights.pop();
        assert!(StaticTerrainQuery::new(&t, 1., Precision::Double53).is_err());
    }
}
