//! Type-3 raster over ordered, prepared world triangles and footprint neighbors.
//! Wrapper eligibility and model/pose preparation remain separate.
use crate::{
    arithmetic::Precision, collision_world::footprint_cell, platform_queries::platform_normal,
};
use giants_formats::footprint::Bitmap;

/// Ordered object-list entry. Identity must agree with the caller's self ID.
pub struct Neighbor<'a> {
    pub id: u32,
    pub object_flags: u32,
    pub collision_flags: u32,
    pub footprint: Option<&'a mut [u8]>,
}

/// Matrix cached on the native object when its affine-cache flag is set.
/// Its arithmetic order differs from the uncached source-pose helper.
pub fn transform_cached(
    matrix: [f32; 12],
    point: [f32; 3],
    precision: Precision,
) -> Result<[f32; 3], &'static str> {
    if !matrix.iter().chain(&point).all(|v| v.is_finite()) {
        return Err("non-finite cached affine input");
    }
    let r = |x| precision.round(x);
    let out = std::array::from_fn(|i| {
        let row = &matrix[i * 4..i * 4 + 4];
        r(r(r(
            r(f64::from(row[2]) * f64::from(point[2])) + r(f64::from(row[0]) * f64::from(point[0]))
        ) + r(f64::from(row[1]) * f64::from(point[1])))
            + f64::from(row[3])) as f32
    });
    if out.iter().any(|x| !x.is_finite()) {
        return Err("cached affine overflow");
    }
    Ok(out)
}

/// Produce bytes consumed directly by Bitmap::parse / StaticCollider::from_mesh.
/// Ground samples must obey the native terrain query contract. The caller is
/// explicitly declaring an empty list of other objects by choosing this API.
pub fn build_without_neighbors(
    bounds_xy: [[f32; 2]; 2],
    triangles: &[[[f32; 3]; 3]],
    precision: Precision,
    ground: impl FnMut([f32; 3]) -> Result<f32, &'static str>,
) -> Result<Option<Vec<u8>>, &'static str> {
    build_with_neighbors(0, bounds_xy, triangles, precision, &mut [], ground)
}

/// Only raster platform membership, deliberately distinct from a complete
/// footprint. It carries no solid, near-ground or reciprocal-neighbor state.
/// `collgrid_make_grid` / `collgrid_forcemake_grid` are prototype PS2 search
/// routes with unknown attribution; see the symbol navigation index.
pub struct PlatformBits {
    bytes: Vec<u8>,
}
impl PlatformBits {
    pub fn contains(&self, point: [f32; 2], precision: Precision) -> Result<bool, &'static str> {
        let [x, y] = footprint_cell(point, precision)?;
        Ok(Bitmap::parse(&self.bytes)
            .map_err(|_| "invalid owned platform bits")?
            .platform_contains(x, y))
    }
}
/// Project only bit1 of the acquired raster. Native terrain and neighbor writes
/// cannot change this bit; this API neither supplies those services nor claims
/// complete footprint construction or list registration. Eligibility and exact
/// prepared geometry remain required inputs at the composing owner.
pub fn platform_bits(
    bounds_xy: [[f32; 2]; 2],
    triangles: &[[[f32; 3]; 3]],
    precision: Precision,
) -> Result<Option<PlatformBits>, &'static str> {
    Ok(
        raster(bounds_xy, triangles, precision, RasterMode::PlatformOnly)?
            .map(|bytes| PlatformBits { bytes }),
    )
}
enum RasterMode<'a, 'b> {
    PlatformOnly,
    Full {
        ground: &'a mut dyn FnMut([f32; 3]) -> Result<f32, &'static str>,
        neighbors: &'a mut [Neighbor<'b>],
        metadata: Vec<Option<[i16; 4]>>,
    },
}

/// Build the current bitmap and mark bit3 in the first matching neighbor's
/// bitmap. List order matters. Neighbor mutations before a callback error are
/// retained; callers needing a transaction must supply copies explicitly.
pub fn build_with_neighbors(
    self_id: u32,
    bounds_xy: [[f32; 2]; 2],
    triangles: &[[[f32; 3]; 3]],
    precision: Precision,
    neighbors: &mut [Neighbor<'_>],
    mut ground: impl FnMut([f32; 3]) -> Result<f32, &'static str>,
) -> Result<Option<Vec<u8>>, &'static str> {
    if neighbors.len() > 100_000 || (self_id == 0 && !neighbors.is_empty()) {
        return Err("invalid neighbor identity/budget");
    }
    if !bounds_xy
        .iter()
        .flatten()
        .chain(triangles.iter().flatten().flatten())
        .all(|v| v.is_finite())
    {
        return Err("non-finite raster input");
    }
    if bounds_xy[0][0] == bounds_xy[1][0] {
        return Ok(None);
    }
    let mut metadata = Vec::with_capacity(neighbors.len());
    for neighbor in neighbors.iter() {
        let layout = if neighbor.id == self_id
            || neighbor.object_flags & 0x8000 != 0
            || neighbor.collision_flags & 8 == 0
        {
            None
        } else if let Some(bytes) = neighbor.footprint.as_deref() {
            if bytes.first() != Some(&3) {
                None
            } else {
                let bitmap = Bitmap::parse(bytes).map_err(|_| "invalid neighbor footprint")?;
                Some(bitmap.bounds)
            }
        } else {
            None
        };
        metadata.push(layout);
    }
    raster(
        bounds_xy,
        triangles,
        precision,
        RasterMode::Full {
            ground: &mut ground,
            neighbors,
            metadata,
        },
    )
}
fn raster(
    bounds_xy: [[f32; 2]; 2],
    triangles: &[[[f32; 3]; 3]],
    precision: Precision,
    mut mode: RasterMode<'_, '_>,
) -> Result<Option<Vec<u8>>, &'static str> {
    if !bounds_xy
        .iter()
        .flatten()
        .chain(triangles.iter().flatten().flatten())
        .all(|v| v.is_finite())
    {
        return Err("non-finite raster input");
    }
    if bounds_xy[0][0] == bounds_xy[1][0] {
        return Ok(None);
    }
    if (0..2).any(|i| bounds_xy[0][i] > bounds_xy[1][i]) || triangles.len() > 100_000 {
        return Err("invalid raster bounds or triangle budget");
    }
    let min = footprint_cell(bounds_xy[0], precision)?;
    let max = footprint_cell(bounds_xy[1], precision)?;
    let coordinates = [min[0], min[1], max[0], max[1]];
    if coordinates.iter().any(|&v| i16::try_from(v).is_err()) {
        return Err("raster bounds outside supported header");
    }
    let width = i64::from(max[0]) - i64::from(min[0]) + 1;
    let height = i64::from(max[1]) - i64::from(min[1]) + 1;
    if width <= 0 || height <= 0 || width * height > 1_000_000 {
        return Err("raster allocation budget");
    }
    let mut bytes = vec![0; 9 + (width * height) as usize];
    bytes[0] = 3;
    for (i, v) in coordinates.iter().enumerate() {
        bytes[1 + i * 2..3 + i * 2].copy_from_slice(&(*v as i16).to_le_bytes());
    }
    let r = |x| precision.round(x);
    let add = |a: f32, b: f32| r(f64::from(a) + f64::from(b)) as f32;
    let mut visits = 0usize;
    for &triangle in triangles {
        let normal = platform_normal(triangle, precision)?;
        let [nx, ny, nz] = normal.map(f64::from);
        let a = triangle[0].map(f64::from);
        // Original dot stores a float32 plane before sorting the vertices.
        let plane = (-r(r(r(a[0] * nx) + r(ny * a[1])) + r(nz * a[2]))) as f32;
        let [mut a, mut b, mut c] = triangle;
        if b[1] < a[1] {
            std::mem::swap(&mut a, &mut b);
        }
        if c[1] < b[1] {
            std::mem::swap(&mut b, &mut c);
        }
        if b[1] < a[1] {
            std::mem::swap(&mut a, &mut b);
        }
        let row_limit = r(r(f64::from(c[1]) + 5.) - 0.5);
        let mut y = a[1];
        while f64::from(y) < row_limit {
            visits += 1;
            if visits > 2_000_000 {
                return Err("raster scan budget");
            }
            if b[1] < y && r(r(f64::from(y) - 5.) + 0.5) < f64::from(b[1]) {
                y = b[1];
            }
            if c[1] < y {
                y = c[1];
            }
            let interpolate = |start: [f32; 3], end: [f32; 3]| {
                r(r(r(r(f64::from(end[0]) - f64::from(start[0]))
                    * r(f64::from(start[1]) - f64::from(y)))
                    / r(f64::from(start[1]) - f64::from(end[1])))
                    + f64::from(start[0])) as f32
            };
            let (left, right) = if a[1] == c[1] {
                (a[0].min(b[0]).min(c[0]), a[0].max(b[0]).max(c[0]))
            } else {
                let first = interpolate(a, c);
                let second = if y < b[1] || b[1] == c[1] {
                    interpolate(a, b)
                } else {
                    interpolate(b, c)
                };
                if first > second {
                    (second, first)
                } else {
                    (first, second)
                }
            };
            let limit = r(r(f64::from(right) + 5.) - 0.5) as f32;
            if !left.is_finite() || !right.is_finite() || !limit.is_finite() {
                return Err("raster span overflow");
            }
            // This scan coordinate spills to float32 before floor, unlike X.
            let row = (r(r(f64::from(y) * f64::from(0.2f32)) + 0.5) as f32).floor() as i32;
            let mut x = left;
            while x < limit {
                if right < x {
                    x = right;
                }
                visits += 1;
                if visits > 2_000_000 {
                    return Err("raster scan budget");
                }
                let column = footprint_cell([x, 0.], precision)?[0];
                if column >= min[0] && column <= max[0] && row >= min[1] && row <= max[1] {
                    let z = if normal[2] == 0. {
                        a[2]
                    } else {
                        (-r(
                            r(r(r(f64::from(y) * ny) + r(f64::from(x) * nx)) + f64::from(plane))
                                / nz,
                        )) as f32
                    };
                    if !z.is_finite() {
                        return Err("raster plane overflow");
                    }
                    let pixel = &mut bytes[9
                        + ((i64::from(row - min[1]) * width) + i64::from(column - min[0]))
                            as usize];
                    match &mut mode {
                        RasterMode::PlatformOnly => {
                            if normal[2] > 0.64278f32 {
                                *pixel |= 2;
                            }
                        }
                        RasterMode::Full {
                            ground,
                            neighbors,
                            metadata,
                        } => {
                            let terrain = ground([x, y, z])?;
                            if !terrain.is_finite() {
                                return Err("non-finite raster ground");
                            }
                            let dz = r(f64::from(z) - f64::from(terrain));
                            if normal[2] <= 0.64278f32 {
                                if dz <= 8. {
                                    *pixel |= 1;
                                }
                            } else {
                                *pixel |= 2;
                                if (x == left || x == right)
                                    && (y == a[1] || y == c[1])
                                    && dz < 0.5
                                    && dz > -2.
                                {
                                    *pixel |= 4;
                                }
                                for (neighbor, layout) in neighbors.iter_mut().zip(metadata.iter())
                                {
                                    let Some([x0, y0, x1, y1]) = *layout else {
                                        continue;
                                    };
                                    if column < i32::from(x0)
                                        || column > i32::from(x1)
                                        || row < i32::from(y0)
                                        || row > i32::from(y1)
                                    {
                                        continue;
                                    }
                                    let index = 9
                                        + ((row - i32::from(y0))
                                            * (i32::from(x1) - i32::from(x0) + 1)
                                            + column
                                            - i32::from(x0))
                                            as usize;
                                    let other = &mut neighbor
                                        .footprint
                                        .as_deref_mut()
                                        .ok_or("missing validated footprint")?[index];
                                    if *other & 2 != 0 && *other & 4 == 0 {
                                        *other |= 8;
                                        *pixel |= 8;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
                let next = add(x, 5.);
                if next <= x {
                    return Err("non-progressing raster column");
                }
                x = next;
            }
            let next = add(y, 5.);
            if next <= y {
                return Err("non-progressing raster row");
            }
            y = next;
        }
    }
    Ok(Some(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collision_world::{ColliderFields, SourcePose, StaticCollider};
    use giants_formats::gbs::{Material, Mesh, Subobject};

    #[test]
    #[ignore = "requires private exact-PC raster controls"]
    fn native_platform_bits_match_without_claiming_other_channels() {
        let controls = std::fs::read_to_string(
            std::env::var_os("GIANTS_PLATFORM_BITS_CONTROLS").expect("private controls path"),
        )
        .unwrap();
        let mut count = 0;
        for line in controls.lines() {
            let mut words = line.split_whitespace();
            let float = |value: &str| f32::from_bits(u32::from_str_radix(value, 16).unwrap());
            let bounds =
                std::array::from_fn(|_| std::array::from_fn(|_| float(words.next().unwrap())));
            let size = words.next().unwrap().parse::<usize>().unwrap();
            let triangles = (0..size)
                .map(|_| {
                    std::array::from_fn(|_| std::array::from_fn(|_| float(words.next().unwrap())))
                })
                .collect::<Vec<_>>();
            let expected = words.next().unwrap();
            assert!(words.next().is_none());
            let bits = platform_bits(bounds, &triangles, Precision::Double53).unwrap();
            if expected == "none" {
                assert!(bits.is_none());
            } else {
                let bytes = (0..expected.len())
                    .step_by(2)
                    .map(|i| u8::from_str_radix(&expected[i..i + 2], 16).unwrap())
                    .collect::<Vec<_>>();
                let bits = bits.unwrap();
                assert_eq!(bits.bytes, bytes, "native raster control {count}");
                let map = Bitmap::parse(&bytes).unwrap();
                for y in i32::from(map.bounds[1]) - 1..=i32::from(map.bounds[3]) + 1 {
                    for x in i32::from(map.bounds[0]) - 1..=i32::from(map.bounds[2]) + 1 {
                        assert_eq!(
                            bits.contains([x as f32 * 5., y as f32 * 5.], Precision::Double53)
                                .unwrap(),
                            map.platform_contains(x, y)
                        );
                    }
                }
            }
            count += 1;
        }
        assert_eq!(count, 420);
    }

    #[test]
    fn constructed_bitmap_drives_existing_source_collider_membership() {
        let precision = Precision::Double53;
        let triangle = [[0., 0., 2.], [10., 0., 2.], [0., 10., 2.]];
        let bounds = [[0., 0.], [10., 10.]];
        let bytes = build_without_neighbors(bounds, &[triangle], precision, |_| Ok(0.))
            .unwrap()
            .unwrap();
        let mesh = Mesh {
            positions: triangle.to_vec(),
            indices: vec![2, 0, 1],
            uv: vec![],
            rgb: vec![],
            groups: vec![],
            subobjects: vec![Subobject {
                name: String::new(),
                group: 0,
                triangles: vec![[1, 2, 0]],
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
        let collider = StaticCollider::from_mesh(
            ColliderFields {
                id: 1,
                object_flags: 0,
                collision_flags: 8,
                bounds_xy: bounds,
            },
            &mesh,
            SourcePose {
                basis: [1., 0., 0., 0., 1., 0., 0., 0., 1.],
                position: [0.; 3],
            },
            &bytes,
            precision,
        )
        .unwrap();
        assert!(collider.prepared([1., 1., 3.]).unwrap().footprint_hit);
        assert!(!collider.prepared([9., 9., 3.]).unwrap().footprint_hit);
        assert_eq!(collider.world_triangles(), &[triangle]);
        // An uncached source object reaches a real platform query without
        // inventing a cache bit. Translation in Z preserves this XY footprint.
        let uncached = |flags| crate::collision_world::UncachedSource {
            id: 56,
            object_flags: flags,
            collision_flags: 8,
            models: crate::collision_bounds::Models {
                primary: [0., 0., 2., 10., 10., 2.],
                auxiliary: None,
                slot: 254,
            },
            pose: crate::uncached_pose::Pose {
                position: [0., 0., 7.],
                angles: [0.; 3],
                scale: 1.,
            },
        };
        assert!(
            StaticCollider::from_uncached_source(uncached(0x20), &mesh, &bytes, precision).is_err()
        );
        let collider =
            StaticCollider::from_uncached_source(uncached(0), &mesh, &bytes, precision).unwrap();
        let candidate = collider.prepared([1., 1., 10.]).unwrap();
        let surface =
            crate::platform_queries::sample_prepared([1., 1., 10.], &[candidate], precision)
                .unwrap();
        assert_eq!(surface.height, 9.);
        assert_eq!(surface.object, Some(56));
        let matrix = [0., -1., 0., 10., 1., 0., 0., 20., 0., 0., 2., 3.];
        let world_triangle = triangle.map(|p| transform_cached(matrix, p, precision).unwrap());
        let bytes = build_without_neighbors(
            [[0., 20.], [10., 30.]],
            &[world_triangle],
            precision,
            |_| Ok(0.),
        )
        .unwrap()
        .unwrap();
        let collider = StaticCollider::from_cached_primary(
            crate::collision_world::CachedPrimary {
                id: 55,
                object_flags: 0x20,
                collision_flags: 8,
                bounds: [0., 0., 2., 10., 10., 2.],
                matrix,
            },
            &mesh,
            &bytes,
            precision,
        )
        .unwrap();
        assert_eq!(collider.world_triangles(), &[world_triangle]);
        let candidate = collider.prepared([9., 21., 8.]).unwrap();
        assert_eq!(candidate.bounds_xy, [[0., 20.], [10., 30.]]);
        assert!(candidate.footprint_hit);
        let surface =
            crate::platform_queries::sample_prepared([9., 21., 8.], &[candidate], precision)
                .unwrap();
        assert_eq!(surface.height, 7.);
        assert_eq!(surface.object, Some(55));
    }

    #[test]
    fn unsupported_inputs_fail_before_a_partial_bitmap_can_escape() {
        assert!(
            build_without_neighbors(
                [[0., 0.], [1., 1.]],
                &[[[f32::NAN; 3]; 3]],
                Precision::Double53,
                |_| Ok(0.)
            )
            .is_err()
        );
        assert!(
            build_without_neighbors([[-1e8, 0.], [1e8, 1.]], &[], Precision::Double53, |_| Ok(
                0.
            ))
            .is_err()
        );
        assert!(transform_cached([f32::INFINITY; 12], [0.; 3], Precision::Double53).is_err());
    }

    #[test]
    fn ground_failure_keeps_prior_neighbor_mutations_explicit() {
        let mut data = vec![3, 0, 0, 0, 0, 2, 0, 2, 0];
        data.extend([2; 9]);
        let mut neighbors = [Neighbor {
            id: 2,
            object_flags: 0,
            collision_flags: 8,
            footprint: Some(&mut data),
        }];
        let mut calls = 0;
        let result = build_with_neighbors(
            1,
            [[0., 0.], [10., 10.]],
            &[[[0., 0., 5.], [10., 0., 5.], [0., 10., 5.]]],
            Precision::Double53,
            &mut neighbors,
            |_| {
                calls += 1;
                if calls == 1 {
                    Ok(0.)
                } else {
                    Err("terrain unavailable")
                }
            },
        );
        assert_eq!(result.unwrap_err(), "terrain unavailable");
        assert_eq!(data[9], 10);
        assert_eq!(data[10], 2);
    }
}
