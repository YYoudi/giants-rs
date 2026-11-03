//! Ordinary radial and cached-box responses, after candidate selection.
//! Target fields/cache and model bounds are owned inputs, not inferred meshes.
use crate::{
    arithmetic::Precision,
    contact_pose::Cached,
    mecc_mesh_contact::{Response, State},
    volume_queries::{self, Bounds},
};

#[derive(Clone, Copy, Debug)]
pub struct Radial {
    pub position: [f32; 3],
    pub radius: f32,
    pub scale: f32,
    pub height: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct Boxed {
    pub pose: Cached,
    pub scale: f32,
    pub bounds: Bounds,
}
#[derive(Clone, Copy, Debug)]
pub enum Volume {
    Radial(Radial),
    Boxed(Boxed),
}

pub fn resolve(mut state: State, object: u32, volume: Volume) -> Result<Response, &'static str> {
    if object == 0
        || !state
            .current
            .into_iter()
            .chain(state.previous)
            .chain([state.radius, state.scale, state.height])
            .all(f32::is_finite)
    {
        return Err("invalid actor volume input");
    }
    match volume {
        Volume::Boxed(target) => {
            let mut current = target.pose.to_local(state.current)?;
            let extent = (f64::from(state.radius) / f64::from(target.scale)) as f32;
            if !volume_queries::expanded_point(current, extent, target.bounds, Precision::Double53)
                .ok_or("box extent/bounds outside finite profile")?
            {
                return Ok(Response {
                    state,
                    changed: false,
                });
            }
            let previous = target.pose.to_local(state.previous)?;
            let x_min = (f64::from(previous[0]) - f64::from(target.bounds.min[0])).abs();
            let x_max = (f64::from(previous[0]) - f64::from(target.bounds.max[0])).abs();
            let y_min = (f64::from(previous[1]) - f64::from(target.bounds.min[1])).abs() as f32;
            let y_max = (f64::from(previous[1]) - f64::from(target.bounds.max[1])).abs() as f32;
            let x = if x_max > x_min {
                f64::from(target.bounds.min[0]) - f64::from(extent)
            } else {
                f64::from(target.bounds.max[0]) + f64::from(extent)
            };
            let y = if y_min < y_max {
                f64::from(target.bounds.min[1]) - f64::from(extent)
            } else {
                f64::from(target.bounds.max[1]) + f64::from(extent)
            };
            if (f64::from(current[0]) - x).abs() < (f64::from(current[1]) - y).abs() {
                current[0] = x as f32;
            } else {
                current[1] = y as f32;
            }
            state.current = target.pose.to_world(current)?;
            state.response_flags |= 0x800;
            state.hit = object;
            Ok(Response {
                state,
                changed: true,
            })
        }
        Volume::Radial(target) => {
            if !target
                .position
                .into_iter()
                .chain([target.radius, target.scale, target.height])
                .all(f32::is_finite)
            {
                return Err("invalid radial target");
            }
            let distance = planar_squared(state.current, target.position).sqrt() as f32;
            let radius = f64::from(target.radius) + f64::from(state.radius);
            if radius < f64::from(distance)
                || f64::from(target.height) * f64::from(target.scale)
                    + f64::from(target.position[2])
                    < f64::from(state.current[2])
                || f64::from(state.height) * f64::from(state.scale) + f64::from(state.current[2])
                    < f64::from(target.position[2])
            {
                return Ok(Response {
                    state,
                    changed: false,
                });
            }
            if state.collision_flags & 0x8000 != 0 {
                state.current = state.previous;
                state.response_flags |= 0x800;
                state.hit = object;
                return Ok(Response {
                    state,
                    changed: true,
                });
            }
            let old_squared = planar_squared(state.previous, target.position);
            if f64::from(distance) * f64::from(distance) > old_squared {
                return Ok(Response {
                    state,
                    changed: false,
                });
            }
            let ratio = radius / old_squared.sqrt();
            let theta = if ratio.abs() > 1. {
                95.
            } else {
                (f64::from(ratio as f32).asin() * f64::from(57.295776_f32)) as f32
            };
            let center_heading = heading(state.previous, target.position);
            let movement_heading = heading(state.previous, state.current);
            let plus = angle_distance(
                movement_heading,
                (f64::from(center_heading) + f64::from(theta)) as f32,
            ) as f32;
            let minus = angle_distance(
                movement_heading,
                (f64::from(center_heading) - f64::from(theta)) as f32,
            );
            let theta = if minus < f64::from(plus) {
                -theta
            } else {
                theta
            };
            let angle = (f64::from(center_heading) + f64::from(theta)) as f32;
            let index =
                ((f64::from(angle) * f64::from(45.511112_f32) + 0.5) as i32 & 0x3fff) as usize;
            let sine = (index as f64 * f64::from(0.0003834952_f32)).sin() as f32;
            let cosine = ((index + 4096) as f64 * f64::from(0.0003834952_f32)).sin() as f32;
            let movement = planar_squared(state.previous, state.current).sqrt();
            state.current[0] = (f64::from(sine) * movement + f64::from(state.previous[0])) as f32;
            state.current[1] = (f64::from(state.previous[1]) - f64::from(cosine) * movement) as f32;
            state.hit = object;
            if !state.current.into_iter().all(f32::is_finite) {
                return Err("radial response overflow");
            }
            Ok(Response {
                state,
                changed: true,
            })
        }
    }
}
fn planar_squared(a: [f32; 3], b: [f32; 3]) -> f64 {
    let dx = f64::from(b[0]) - f64::from(a[0]);
    let dy = f64::from(b[1]) - f64::from(a[1]);
    dx * dx + dy * dy
}
pub(crate) fn heading(a: [f32; 3], b: [f32; 3]) -> f32 {
    heading_wide(a, b) as f32
}
pub(crate) fn heading_wide(a: [f32; 3], b: [f32; 3]) -> f64 {
    let dx = f64::from(b[0]) - f64::from(a[0]);
    let dy = f64::from(b[1]) - f64::from(a[1]);
    let mut angle = if dx == 0. && dy == 0. {
        90.
    } else {
        dy.atan2(dx) * f64::from(57.295776_f32) + 90.
    };
    if angle > 360. {
        angle -= 360.;
    }
    if angle < 0. {
        angle += 360.;
    }
    if angle > f64::from(359.99_f32) {
        angle = 0.;
    }
    angle
}
fn angle_distance(a: f32, b: f32) -> f64 {
    let mut d = f64::from(a) - f64::from(b);
    if d < 0. {
        d += 360.;
    }
    if d > 360. {
        d -= 360.;
    }
    if d > 180. {
        d = 360. - d;
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;
    fn actor() -> State {
        State {
            current: [1., 1., 1.],
            previous: [5., 0., 1.],
            radius: 1.,
            scale: 1.,
            center_offset: 5.,
            height: 10.,
            ground_offset: 0.,
            floor: 0.,
            plane: [0., 0., 1., 0.],
            model_vertical_speed: -1.,
            world_vertical_speed: 3.,
            object_flags: 0,
            response_flags: 0x100,
            collision_flags: 0,
            physical: false,
            hit: 0,
        }
    }
    fn radial() -> Volume {
        Volume::Radial(Radial {
            position: [0.; 3],
            radius: 2.,
            scale: 1.,
            height: 10.,
        })
    }
    fn boxed(matrix: [f32; 16]) -> Volume {
        Volume::Boxed(Boxed {
            pose: Cached::new(0x20, 1, matrix).unwrap(),
            scale: 1.,
            bounds: Bounds {
                min: [-1.; 3],
                max: [1.; 3],
            },
        })
    }
    #[test]
    fn radial_tangent_keeps_the_side_height_and_requested_distance() {
        for sign in [-1., 1.] {
            let mut s = actor();
            s.current[1] *= sign;
            let out = resolve(s, 9, radial()).unwrap();
            assert!(out.changed);
            assert_eq!(out.state.current[2], s.current[2]);
            assert_eq!(out.state.response_flags, s.response_flags);
            assert_eq!(out.state.hit, 9);
            assert!(out.state.current[1] * sign > 0.);
            let moved = planar_squared(out.state.current, s.previous).sqrt();
            let requested = planar_squared(s.current, s.previous).sqrt();
            assert!((moved - requested).abs() < 0.00001);
        }
    }
    #[test]
    fn moving_outwards_is_not_rewound_but_special_collision_is() {
        let mut s = actor();
        s.previous = [1., 0., 2.];
        s.current = [2., 0., 3.];
        assert!(!resolve(s, 9, radial()).unwrap().changed);
        s.collision_flags |= 0x8000;
        let out = resolve(s, 9, radial()).unwrap();
        assert_eq!(out.state.current, s.previous);
        assert_eq!(out.state.response_flags, s.response_flags | 0x800);
    }
    #[test]
    fn vertical_overlap_is_inclusive_and_separate_from_planar_radius() {
        let mut s = actor();
        s.collision_flags = 0x8000;
        s.current[2] = 10.;
        assert!(resolve(s, 9, radial()).unwrap().changed);
        s.current[2] = f32::from_bits(10f32.to_bits() + 1);
        assert!(!resolve(s, 9, radial()).unwrap().changed);
    }
    #[test]
    fn box_ties_select_positive_y_and_transform_back_to_world() {
        let mut s = actor();
        s.current = [0.; 3];
        s.previous = [0.; 3];
        let identity = std::array::from_fn(|i| f32::from(i / 4 == i % 4));
        let out = resolve(s, 9, boxed(identity)).unwrap();
        assert_eq!(out.state.current, [0., 2., 0.]);
        assert_eq!(out.state.hit, 9);
        assert_eq!(out.state.response_flags, s.response_flags | 0x800);
        let rotation = [
            1., 0., 0., 0., 0., 0., -1., 0., 0., 1., 0., 0., 0., 0., 0., 1.,
        ];
        let out = resolve(s, 9, boxed(rotation)).unwrap();
        assert_eq!(out.state.current, [0., 0., 2.]);
    }
    #[test]
    fn zero_scale_box_does_not_supply_a_fake_contact() {
        let identity = std::array::from_fn(|i| f32::from(i / 4 == i % 4));
        let Volume::Boxed(mut target) = boxed(identity) else {
            unreachable!()
        };
        target.scale = 0.;
        assert!(resolve(actor(), 9, Volume::Boxed(target)).is_err());
    }
}
