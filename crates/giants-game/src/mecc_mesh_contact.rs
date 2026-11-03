//! Mesh response on explicitly prepared ordered world triangles.
//! Model selection, animation, part eligibility and live fields remain upstream.
use crate::{arithmetic::Precision, platform_queries::platform_normal};
#[derive(Clone, Copy, Debug)]
pub struct State {
    pub current: [f32; 3],
    pub previous: [f32; 3],
    pub radius: f32,
    pub scale: f32,
    pub center_offset: f32,
    pub height: f32,
    pub ground_offset: f32,
    pub floor: f32,
    pub plane: [f32; 4],
    pub model_vertical_speed: f32,
    pub world_vertical_speed: f32,
    pub object_flags: u32,
    pub response_flags: u32,
    pub collision_flags: u32,
    pub physical: bool,
    pub hit: u32,
}
pub struct Response {
    pub state: State,
    pub changed: bool,
}
fn contains(p: [f32; 3], n: [f32; 3], t: [[f32; 3]; 3], precision: Precision) -> bool {
    let axes = if n[0].abs() <= n[1].abs() {
        if n[2].abs() < n[1].abs() {
            [0, 2]
        } else {
            [0, 1]
        }
    } else if n[2].abs() < n[0].abs() {
        [1, 2]
    } else {
        [0, 1]
    };
    let r = |x| precision.round(x);
    let q =
        t.map(|v| axes.map(|axis| f64::from(r(f64::from(v[axis]) - f64::from(p[axis])) as f32)));
    let mut count = 0;
    for [a, b] in [[0, 1], [2, 1], [0, 2]] {
        let [x, y] = q[a];
        let [nx, ny] = q[b];
        if (y >= 0.) != (ny >= 0.)
            && ((x >= 0. && nx >= 0.)
                || ((x >= 0. || nx >= 0.) && r(x - r(r(r(nx - x) * y) / r(ny - y))) > 0.))
        {
            count += 1;
        }
    }
    count & 1 != 0
}
pub fn resolve(
    mut state: State,
    object: u32,
    triangles: &[[[f32; 3]; 3]],
    precision: Precision,
) -> Result<Response, &'static str> {
    if object == 0
        || !state
            .current
            .into_iter()
            .chain(state.previous)
            .chain(state.plane)
            .chain([
                state.radius,
                state.scale,
                state.center_offset,
                state.height,
                state.ground_offset,
                state.floor,
                state.model_vertical_speed,
                state.world_vertical_speed,
            ])
            .all(f32::is_finite)
    {
        return Err("invalid prepared mesh contact input");
    }
    let r = |x| precision.round(x);
    let f = |x| f64::from(x);
    let stored = |x| f(r(x) as f32);
    let mut changed = false;
    for &triangle in triangles {
        let n = platform_normal(triangle, precision)?;
        let [nx, ny, nz] = n.map(f);
        let a = triangle[0].map(f);
        let d = stored(-r(r(r(nx * a[0]) + r(ny * a[1])) + r(nz * a[2])));
        let vertical = n[2].abs();
        let special = state.collision_flags & 0x8000 != 0;
        let dot = |p: [f32; 3], z: f64| r(r(r(r(z * nz) + r(nx * f(p[0]))) + r(ny * f(p[1]))) + d);
        if special || vertical < 0.75 {
            let center_wide = r(f(state.center_offset) * f(state.scale));
            let center = stored(center_wide);
            let old_distance = stored(dot(state.previous, r(center_wide + f(state.previous[2]))));
            let new_wide = dot(state.current, r(center + f(state.current[2])));
            let new_distance = stored(new_wide);
            if r(new_wide - f(state.radius)) < 0. && old_distance > 0. {
                let delta = std::array::from_fn::<_, 3, _>(|i| {
                    stored(f(state.current[i]) - f(state.previous[i]))
                });
                let denominator = r(r(r(delta[1] * ny) + r(delta[2] * nz)) + r(delta[0] * nx));
                let mut time = if denominator == 0. {
                    0.
                } else {
                    stored(-r(old_distance / denominator))
                };
                let point = [
                    r(r(delta[0] * time) + f(state.previous[0])) as f32,
                    r(r(delta[1] * time) + f(state.previous[1])) as f32,
                    r(r(r(delta[2] * time) + center) + f(state.previous[2])) as f32,
                ];
                if !contains(point, n, triangle, precision) {
                    continue;
                }
                if special {
                    if time <= f(0.99_f32) {
                        time = stored(time - f(0.01_f32));
                        let z = stored(r(r(delta[2] * time) + center) + f(state.previous[2]));
                        state.current = [
                            r(r(delta[0] * time) + f(state.previous[0])) as f32,
                            r(r(delta[1] * time) + f(state.previous[1])) as f32,
                            r(z - center) as f32,
                        ];
                        state.response_flags |= 0x800;
                        state.hit = object;
                        changed = true;
                    }
                    continue;
                }
                let push = if old_distance >= 0. {
                    r(f(state.radius) - new_distance)
                } else {
                    -r(new_distance + f(state.radius))
                };
                let wide =
                    std::array::from_fn::<_, 3, _>(|i| r(r(f(n[i]) * push) + f(state.current[i])));
                let candidate = wide.map(|v| v as f32);
                if wide[0] != f(state.current[0])
                    || candidate[1] != state.current[1]
                    || candidate[2] != state.current[2]
                {
                    state.current = candidate;
                    state.hit = object;
                    changed = true;
                }
            }
        }
        if vertical > 0.5 {
            let top = if state.current[2] < state.previous[2] {
                f(state.previous[2])
            } else {
                f(state.current[2])
            };
            let center_wide = r(f(state.center_offset) * f(state.scale));
            let center = stored(center_wide);
            let margin_wide = r(center_wide * f(0.4_f32));
            let margin = stored(margin_wide);
            let near = stored(r(r(margin_wide + top) - f(state.ground_offset)));
            let near_distance = stored(dot(state.current, near));
            let far = dot(state.current, r(r(top - 100.) - f(state.ground_offset)));
            if r(far * near_distance) < 0. {
                let delta = r(r(center * f(-0.3_f32)) - margin);
                let denominator = r(nz * delta);
                let time = if denominator == 0. {
                    0.
                } else {
                    -r(near_distance / denominator)
                };
                let z = r(r(time * delta) + near);
                let point = [state.current[0], state.current[1], z as f32];
                if z < f(state.floor) || !contains(point, n, triangle, precision) {
                    continue;
                }
                let target = r(f(point[2]) + f(state.ground_offset)) as f32;
                state.floor = point[2];
                state.plane = [n[0], n[1], n[2], d as f32];
                if state.model_vertical_speed > 0. || r(f(state.current[2]) - f(target)) > margin {
                    continue;
                }
                if state.physical {
                    state.world_vertical_speed = 0.;
                }
                if target != state.current[2] {
                    state.current[2] = target;
                    changed = true;
                }
                state.object_flags |= 0x80000;
            }
        }
        if vertical > 0.6 && state.object_flags & 0x80000 == 0 {
            let center = r(f(state.center_offset) * f(state.scale));
            let old_distance = stored(dot(state.previous, r(center + f(state.previous[2]))));
            let head_wide = r(f(state.height) * f(state.scale));
            let head = stored(head_wide);
            let head_distance = dot(state.current, r(head_wide + f(state.current[2])));
            if r(head_distance * old_distance) < 0. {
                let delta = [
                    stored(f(state.current[0]) - f(state.previous[0])),
                    stored(f(state.current[1]) - f(state.previous[1])),
                    r(f(state.current[2]) - f(state.previous[2])),
                ];
                let denominator = r(r(r(delta[1] * ny) + r(delta[2] * nz)) + r(delta[0] * nx));
                let time = if denominator == 0. {
                    0.
                } else {
                    -r(old_distance / denominator)
                };
                let point = [
                    r(r(delta[0] * time) + f(state.previous[0])) as f32,
                    r(r(delta[1] * time) + f(state.previous[1])) as f32,
                    r(r(r(delta[2] * time) + center) + f(state.previous[2])) as f32,
                ];
                if contains(point, n, triangle, precision) {
                    let z = r(f(point[2]) - head);
                    if point[0] != state.current[0]
                        || point[1] != state.current[1]
                        || z != f(state.current[2])
                    {
                        state.current[0] = point[0];
                        state.current[1] = point[1];
                        if z < f(state.current[2]) {
                            state.current[2] = z as f32;
                            if state.model_vertical_speed > 0. {
                                state.model_vertical_speed = 0.;
                            }
                        }
                        changed = true;
                    }
                }
            }
        }
    }
    Ok(Response { state, changed })
}
