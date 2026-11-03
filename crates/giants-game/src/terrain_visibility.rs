//! Source GTI terrain line-of-sight grid. Object collision is not included.
use crate::sun_glare::{Ray, normalize};
use giants_formats::{Error, gti::Terrain};

fn dot(a: [f32; 3], b: [f32; 3]) -> f64 {
    f64::from(a[1]) * f64::from(b[1])
        + f64::from(a[2]) * f64::from(b[2])
        + f64::from(a[0]) * f64::from(b[0])
}
fn inside(point: [f32; 3], normal: [f32; 3], triangle: [[f32; 3]; 3]) -> bool {
    let [x, y, z] = normal.map(f32::abs);
    let axes = if x <= y {
        if z < y { [0, 2] } else { [0, 1] }
    } else if z < x {
        [1, 2]
    } else {
        [0, 1]
    };
    let p = triangle.map(|v| axes.map(|i| v[i] - point[i]));
    let crossings = [(0, 1), (2, 1), (0, 2)]
        .into_iter()
        .filter(|&(i, j)| {
            let [x, y] = p[i];
            let [a, b] = p[j];
            (y >= 0.) != (b >= 0.)
                && ((x >= 0. && a >= 0.)
                    || ((x >= 0. || a >= 0.)
                        && f64::from(x)
                            - (f64::from(a) - f64::from(x)) * f64::from(y)
                                / (f64::from(b) - f64::from(y))
                            > 0.))
        })
        .count();
    crossings & 1 != 0
}
fn intersects(ray: &Ray, triangle: [[f32; 3]; 3]) -> bool {
    let [a, b, c] = triangle;
    let u: [f64; 3] = std::array::from_fn(|i| f64::from(b[i]) - f64::from(a[i]));
    let v: [f64; 3] = std::array::from_fn(|i| f64::from(c[i]) - f64::from(a[i]));
    let normal = normalize([
        (u[1] * v[2] - u[2] * v[1]) as f32,
        (u[2] * v[0] - u[0] * v[2]) as f32,
        (u[0] * v[1] - u[1] * v[0]) as f32,
    ]);
    let plane = (-dot(normal, a)) as f32;
    let first = (dot(normal, ray.start) + f64::from(plane)) as f32;
    let last = dot(normal, ray.end) + f64::from(plane);
    if last * f64::from(first) >= 0. {
        return false;
    }
    let direction: [f32; 3] = std::array::from_fn(|i| ray.end[i] - ray.start[i]);
    let denominator = dot(normal, direction) as f32;
    if denominator == 0. {
        return false;
    }
    let fraction = (-(f64::from(first) / f64::from(denominator))) as f32;
    if !(0. ..=1.).contains(&fraction) {
        return false;
    }
    let point = std::array::from_fn(|i| {
        ((f64::from(ray.end[i]) - f64::from(ray.start[i])) * f64::from(fraction)
            + f64::from(ray.start[i])) as f32
    });
    inside(point, normal, triangle)
}
fn cell_hit(terrain: &Terrain, ray: &Ray, cell: [i32; 2]) -> Result<bool, Error> {
    let [x, y] = cell;
    if x < 0 || y < 0 || x as usize >= terrain.width - 1 || y as usize >= terrain.height - 1 {
        return Ok(false);
    }
    let index = y as usize * terrain.width + x as usize;
    // Collision dispatch ignores tag7 even though the renderer has a triangle.
    let orders: &[[usize; 3]] = match terrain.tags[index] & 7 {
        1 => &[[1, 2, 0]],
        2 => &[[2, 1, 3]],
        3 => &[[3, 0, 1]],
        4 => &[[0, 3, 2]],
        5 => &[[3, 0, 1], [0, 3, 2]],
        6 => &[[1, 2, 0], [2, 1, 3]],
        _ => &[],
    };
    let corners = [
        index,
        index + 1,
        index + terrain.width,
        index + terrain.width + 1,
    ];
    for order in orders {
        let triangle = [
            terrain.position(corners[order[0]])?,
            terrain.position(corners[order[1]])?,
            terrain.position(corners[order[2]])?,
        ];
        if intersects(ray, triangle) {
            return Ok(true);
        }
    }
    Ok(false)
}
pub fn clear(terrain: &Terrain, ray: &Ray) -> Result<bool, Error> {
    if ray.start.into_iter().chain(ray.end).any(|v| !v.is_finite()) {
        return Err(Error("nonfinite terrain ray"));
    }
    let spacing = f64::from(terrain.spacing);
    let reciprocal = f64::from(1. / terrain.spacing);
    let mut cell = std::array::from_fn(|i| {
        ((f64::from(ray.start[i]) - f64::from(terrain.origin[i])) * reciprocal) as i32
    });
    let delta = [ray.end[0] - ray.start[0], ray.end[1] - ray.start[1]];
    let signs = delta.map(|v| if v < 0. { -1 } else { 1 });
    let major = usize::from(delta[0].abs() <= delta[1].abs());
    let minor = 1 - major;
    let mut remaining = delta[major].abs();
    if remaining == 0. {
        return Ok(true);
    }
    let ratio = (f64::from(delta[minor].abs()) / f64::from(remaining)) as f32;
    let mut offset = (((f64::from(ray.start[minor])
        - (f64::from(cell[minor]) * spacing + f64::from(terrain.origin[minor])))
        / spacing
        - 0.5)
        * f64::from(signs[minor])) as f32;
    for _ in 0..100_000 {
        remaining = (f64::from(remaining) - spacing) as f32;
        if cell_hit(terrain, ray, cell)? {
            return Ok(false);
        }
        if offset > 0.2 || offset < -0.2 {
            let mut neighbor = cell;
            neighbor[minor] += if offset > 0.2 {
                signs[minor]
            } else {
                -signs[minor]
            };
            if cell_hit(terrain, ray, neighbor)? {
                return Ok(false);
            }
        }
        offset += ratio;
        if offset >= 0.5 {
            offset -= 1.;
            cell[minor] += signs[minor];
        }
        cell[major] += signs[major];
        if remaining <= 0. {
            return Ok(true);
        }
    }
    Err(Error("terrain line-of-sight traversal budget"))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn terrain(tag: u8) -> Terrain {
        Terrain {
            source_flags: 3,
            width: 2,
            height: 2,
            origin: [0.; 2],
            spacing: 1.,
            height_bounds: [0.; 2],
            fallback_texture: String::new(),
            heights: vec![0.; 4],
            colors: vec![0; 4],
            tags: vec![tag, 0, 0, 0],
            consumed_bytes: 0,
            opaque_tail: vec![],
        }
    }
    #[test]
    fn six_collision_tags_and_ignored_render_tag_are_distinct() {
        for (tag, x, y) in [
            (1, 0.2, 0.1),
            (2, 0.8, 0.7),
            (3, 0.8, 0.2),
            (4, 0.2, 0.8),
            (5, 0.2, 0.8),
            (6, 0.8, 0.7),
        ] {
            let ray = Ray {
                direction: [0.; 3],
                start: [x, y, 1.],
                end: [x + 0.01, y + 0.01, -1.],
                dot: 0.,
            };
            assert!(cell_hit(&terrain(tag), &ray, [0, 0]).unwrap(), "tag{tag}");
            assert!(!clear(&terrain(tag), &ray).unwrap());
            assert!(clear(&terrain(7), &ray).unwrap());
        }
    }
    #[test]
    fn finite_segment_misses_do_not_occlude() {
        let ray = Ray {
            direction: [0.; 3],
            start: [0.2, 0.1, 2.],
            end: [0.3, 0.2, 1.],
            dot: 0.,
        };
        assert!(clear(&terrain(6), &ray).unwrap());
        assert!(!cell_hit(&terrain(6), &ray, [-1, 0]).unwrap());
        assert!(!cell_hit(&terrain(6), &ray, [2, 0]).unwrap());
    }
}
