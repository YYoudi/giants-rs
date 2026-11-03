//! Physical player ellipsoid contact on explicitly prepared world geometry.
//! Scene membership, resource transforms and collision events remain upstream.
use crate::{arithmetic::Precision, plane_response};
#[derive(Clone, Copy, Debug)]
pub struct State {
    pub current: [f32; 3],
    pub previous: [f32; 3],
    pub velocity: [f32; 3],
    pub scale: f32,
    pub radius: f32,
    pub height: f32,
    pub floor: f32,
    pub plane: [f32; 4],
    pub physics_flags: u32,
    pub support: u32,
    pub hit: u32,
    pub support_part: u32,
    pub support_triangle: u32,
    pub restitution: f32,
    pub retention: f32,
}
pub struct Triangle {
    pub vertices: [[f32; 3]; 3],
    pub part: u32,
    pub ordinal: u32,
}
pub struct Output {
    pub state: State,
    pub retry: bool,
    pub reciprocal_hit: bool,
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    (a[0] * b[0] + a[1] * b[1]) + a[2] * b[2]
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
// Geometric closest feature of a triangle to the ellipsoid's normalized origin.
fn closest(t: [[f32; 3]; 3]) -> [f32; 3] {
    let [a, b, c] = t.map(|v| v.map(f64::from));
    let ab = sub(b, a);
    let ac = sub(c, a);
    let ap = a.map(|x| -x);
    let d1 = dot(ab, ap);
    let d2 = dot(ac, ap);
    if d1 <= 0. && d2 <= 0. {
        return a.map(|x| x as f32);
    }
    let bp = b.map(|x| -x);
    let d3 = dot(ab, bp);
    let d4 = dot(ac, bp);
    if d3 >= 0. && d4 <= d3 {
        return b.map(|x| x as f32);
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0. && d1 >= 0. && d3 <= 0. {
        let v = d1 / (d1 - d3);
        return std::array::from_fn(|i| (a[i] + v * ab[i]) as f32);
    }
    let cp = c.map(|x| -x);
    let d5 = dot(ab, cp);
    let d6 = dot(ac, cp);
    if d6 >= 0. && d5 <= d6 {
        return c.map(|x| x as f32);
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0. && d2 >= 0. && d6 <= 0. {
        let w = d2 / (d2 - d6);
        return std::array::from_fn(|i| (a[i] + w * ac[i]) as f32);
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0. && d4 - d3 >= 0. && d5 - d6 >= 0. {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return std::array::from_fn(|i| (b[i] + w * (c[i] - b[i])) as f32);
    }
    let inv = 1. / (va + vb + vc);
    let v = vb * inv;
    let w = vc * inv;
    std::array::from_fn(|i| ((a[i] + ab[i] * v) + ac[i] * w) as f32)
}
pub fn resolve(
    mut state: State,
    object: u32,
    object_response_flags: u32,
    triangles: &[Triangle],
    mut collision_event: impl FnMut() -> Result<bool, &'static str>,
) -> Result<Output, &'static str> {
    if object == 0
        || state.radius <= 0.
        || state.height <= 0.
        || state.scale <= 0.
        || !state
            .current
            .into_iter()
            .chain(state.previous)
            .chain(state.velocity)
            .chain(state.plane)
            .chain([
                state.radius,
                state.height,
                state.scale,
                state.floor,
                state.restitution,
                state.retention,
            ])
            .all(f32::is_finite)
    {
        return Err("invalid player contact input");
    }
    let half = (f64::from(state.height) * f64::from(state.scale) * 0.5) as f32;
    let radii = [state.radius, state.radius, half];
    let center = [
        state.current[0],
        state.current[1],
        (f64::from(state.current[2])
            + if state.physics_flags & 0x200 != 0 {
                0.
            } else {
                f64::from(half)
            }) as f32,
    ];
    let inv = radii.map(|x| (1. / f64::from(x)) as f32);
    let translation: [f32; 3] =
        std::array::from_fn(|i| (-f64::from(center[i]) * f64::from(inv[i])) as f32);
    let mut wall = 0.99_f32;
    let mut floor = 1.5_f32;
    let mut wall_point = [0.; 3];
    let mut floor_point = [0.; 3];
    let mut floor_id = [0; 2];
    for t in triangles {
        if !t.vertices.iter().flatten().all(|x| x.is_finite()) {
            return Err("nonfinite player triangle");
        }
        let [a, b, c] = t.vertices.map(|v| v.map(f64::from));
        let n = cross(sub(b, a), sub(c, a)).map(|x| x as f32);
        if n == [0.; 3] {
            continue;
        }
        let d = -dot(n.map(f64::from), a);
        let nw: [f64; 3] = std::array::from_fn(|i| f64::from(n[i]) * f64::from(radii[i]));
        let offset = dot(n.map(f64::from), center.map(f64::from)) + d;
        if offset < 0. || offset * offset / dot(nw, nw) >= 1.5 {
            continue;
        }
        let unit = t.vertices.map(|v| {
            std::array::from_fn(|i| {
                (f64::from(v[i]) * f64::from(inv[i]) + f64::from(translation[i])) as f32
            })
        });
        let q = closest(unit);
        let distance = dot(q.map(f64::from), q.map(f64::from)) as f32;
        if distance < wall {
            wall = distance;
            wall_point = q;
        }
        if distance < floor
            && q[2] < 0.
            && (f64::from(q[0]).powi(2) + f64::from(q[1]).powi(2)) / f64::from(q[2]).powi(2) < 4.
        {
            floor = distance;
            floor_point = q;
            floor_id = [t.part, t.ordinal];
        }
    }
    let plane = |q: [f32; 3], distance: f32| {
        let n: [f32; 3] = std::array::from_fn(|i| (-f64::from(q[i]) * f64::from(inv[i])) as f32);
        let w = (f64::from(distance) - dot(n.map(f64::from), center.map(f64::from))) as f32;
        let inverse = (1. / dot(n.map(f64::from), n.map(f64::from)).sqrt()) as f32;
        [n[0] * inverse, n[1] * inverse, n[2] * inverse, w * inverse]
    };
    if floor < 1.5 {
        state.plane = plane(floor_point, floor);
        state.floor = (-(f64::from(state.current[0]) * f64::from(state.plane[0])
            + f64::from(state.current[1]) * f64::from(state.plane[1])
            + f64::from(state.plane[3]))
            / f64::from(state.plane[2])) as f32;
        if floor < 1.15 {
            state.physics_flags |= 0x20;
            state.support = object;
            state.support_part = floor_id[0];
            state.support_triangle = floor_id[1];
        }
    }
    let mut out = Output {
        state,
        retry: false,
        reciprocal_hit: false,
    };
    if wall < 0.99 {
        let contact_plane = plane(wall_point, wall);
        let inverse = (1. / f64::from(wall).sqrt()) as f32;
        let q = wall_point.map(|x| x * inverse);
        let surface: [f32; 3] = std::array::from_fn(|i| {
            (f64::from(q[i]) * f64::from(radii[i]) + f64::from(center[i])) as f32
        });
        let shift: [f32; 3] = std::array::from_fn(|i| surface[i] - state.current[i]);
        if collision_event()? {
            out.state.physics_flags &= !0x20;
            out.state.support = 0;
        } else if dot(
            contact_plane[..3]
                .try_into()
                .unwrap_or([0.; 3])
                .map(f64::from),
            state.velocity.map(f64::from),
        ) < 0.
        {
            let response = plane_response::object(
                plane_response::Input {
                    position: std::array::from_fn(|i| shift[i] + state.current[i]),
                    previous: std::array::from_fn(|i| shift[i] + state.previous[i]),
                    velocity: state.velocity,
                    plane: contact_plane,
                    restitution: state.restitution,
                    retention: state.retention,
                    flags: state.physics_flags,
                },
                Precision::Double53,
            )?;
            out.state.current = std::array::from_fn(|i| response.position[i] - shift[i]);
            out.state.velocity = response.velocity;
            out.state.physics_flags = response.flags | 0x10;
            out.state.hit = object;
            out.retry = true;
            out.reciprocal_hit = object_response_flags & 0x10 != 0;
        }
    }
    if !out
        .state
        .current
        .into_iter()
        .chain(out.state.velocity)
        .chain(out.state.plane)
        .chain([out.state.floor])
        .all(f32::is_finite)
    {
        return Err("player contact overflow");
    }
    Ok(out)
}
