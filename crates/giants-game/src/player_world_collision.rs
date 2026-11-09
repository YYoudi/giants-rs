//! Development composition of the physical player path with prepared rigid geometry.
//! Membership/list order, live callbacks and part eligibility remain explicit inputs.
use crate::{
    arithmetic::Precision, mecc_collision_candidates, mecc_location_dispatch as dispatch,
    physical_ground, player_mesh_contact as contact, volume_queries,
};
use std::cell::RefCell;
pub struct Object {
    pub fields: mecc_collision_candidates::Object,
    pub response_flags: u32,
    pub triangles: Vec<contact::Triangle>,
}
pub struct Input {
    pub state: contact::State,
    pub object_flags: u32,
    pub collision_flags: u32,
    pub ground_offset: f32,
    pub center_offset: f32,
    pub gravity_mass: f32,
    pub force_z: f32,
}
pub struct Output {
    pub state: contact::State,
    pub object_flags: u32,
    pub force_z: f32,
    pub mesh_responses: u32,
    pub ground_queries: u32,
    pub junctions: u32,
    pub terrain_responses: u32,
    pub passes: u32,
    pub fallback: bool,
    pub notifications: u32,
    pub collision_events: u32,
}
fn read_state(full: &contact::State, object_flags: u32) -> dispatch::State {
    dispatch::State {
        current: full.current,
        previous: full.previous,
        object_flags,
        physics_flags: full.physics_flags,
        support: full.support,
        hit: full.hit,
    }
}
fn write_state(full: &mut contact::State, narrow: &dispatch::State) {
    full.current = narrow.current;
    full.previous = narrow.previous;
    full.physics_flags = narrow.physics_flags;
    full.support = narrow.support;
    full.hit = narrow.hit;
}
fn pass(
    actor: u32,
    state: &mut contact::State,
    objects: &[Object],
    calls: &mut u32,
    events: &mut u32,
    event: &mut impl FnMut(&Object, &contact::State) -> Result<bool, &'static str>,
) -> Result<bool, &'static str> {
    let mut result = false;
    for object in objects {
        let o = object.fields;
        if o.id == actor
            || o.parent == actor
            || !o.has_model
            || o.object_flags & 0x4000_c000 != 0
            || o.collision_flags & 0x800 != 0
        {
            continue;
        }
        let bounds = volume_queries::Bounds {
            min: std::array::from_fn(|i| {
                state.current[i].min(state.previous[i]) - if i < 2 { state.radius } else { 0. }
            }),
            max: std::array::from_fn(|i| {
                state.current[i].max(state.previous[i])
                    + if i < 2 {
                        state.radius
                    } else {
                        state.height * state.scale
                    }
            }),
        };
        if !volume_queries::overlaps(bounds, o.bounds).ok_or("nonfinite physical player bounds")? {
            continue;
        }
        let current = state.current;
        let previous = state.previous;
        let length =
            std::array::from_fn::<_, 3, _>(|i| f64::from(current[i]) - f64::from(previous[i]))
                .into_iter()
                .map(|x| x * x)
                .sum::<f64>()
                .sqrt();
        let radius = state
            .radius
            .min((f64::from(state.height) * f64::from(state.scale) * 0.5) as f32);
        let step = if length == 0. {
            1.
        } else {
            (f64::from(radius) / length) as f32
        };
        let mut t = 0_f32;
        let mut count = 0;
        loop {
            let next = (t + step).min(1.);
            if next <= t || count >= 4096 {
                return Err("physical player subdivision budget");
            }
            t = next;
            count += 1;
            state.current = std::array::from_fn(|i| {
                ((1. - f64::from(t)) * f64::from(previous[i])
                    + f64::from(current[i]) * f64::from(t)) as f32
            });
            let response = contact::resolve_with_event(
                *state,
                o.id,
                object.response_flags,
                &object.triangles,
                |current| {
                    *events += 1;
                    event(object, current)
                },
            )?;
            *calls += 1;
            *state = response.state;
            result = response.retry;
            if result || t >= 1. {
                break;
            }
        }
    }
    Ok(result)
}
pub fn resolve(
    input: Input,
    objects: &[Object],
    query: impl FnMut([f32; 2]) -> Result<physical_ground::Ground, &'static str>,
) -> Result<Output, &'static str> {
    resolve_with_radius(input, objects, query, |state| Ok(state.radius))
}
/// Refresh the physical player's radius at each actual object-walk entry,
/// including an empty candidate list. Frozen location never enters that walk.
pub fn resolve_with_radius(
    input: Input,
    objects: &[Object],
    query: impl FnMut([f32; 2]) -> Result<physical_ground::Ground, &'static str>,
    radius: impl FnMut(&contact::State) -> Result<f32, &'static str>,
) -> Result<Output, &'static str> {
    resolve_for_actor(1, input, objects, query, radius)
}
/// Geometry identities belong to the caller's prepared scene. They are not
/// native pointers and need not equal a Session's allocation serial.
pub fn resolve_for_actor(
    actor: u32,
    input: Input,
    objects: &[Object],
    query: impl FnMut([f32; 2]) -> Result<physical_ground::Ground, &'static str>,
    radius: impl FnMut(&contact::State) -> Result<f32, &'static str>,
) -> Result<Output, &'static str> {
    resolve_for_actor_with_events(actor, input, objects, query, radius, |_, _| Ok(false))
}
/// Preserve source class callback policy instead of assuming every object event
/// is unhandled. Publication of effects on peer objects belongs to the caller.
pub fn resolve_for_actor_with_events(
    actor: u32,
    input: Input,
    objects: &[Object],
    mut query: impl FnMut([f32; 2]) -> Result<physical_ground::Ground, &'static str>,
    mut radius: impl FnMut(&contact::State) -> Result<f32, &'static str>,
    mut event: impl FnMut(&Object, &contact::State) -> Result<bool, &'static str>,
) -> Result<Output, &'static str> {
    if actor == 0 {
        return Err("physical player scene identity must be nonzero");
    }
    let full = RefCell::new(input.state);
    let mut calls = 0;
    let mut events = 0;
    let mut ground_result = None;
    let mut notifications = 0;
    let start = read_state(&input.state, input.object_flags & 0xf7e7_ffff);
    let result = dispatch::resolve(
        start,
        true,
        input.collision_flags,
        |s| {
            let mut full = full.borrow_mut();
            write_state(&mut full, s);
            full.radius = radius(&full)?;
            let retry = pass(
                actor,
                &mut full,
                objects,
                &mut calls,
                &mut events,
                &mut event,
            )?;
            *s = read_state(&full, s.object_flags);
            Ok(retry)
        },
        |s| {
            let mut full = full.borrow_mut();
            write_state(&mut full, s);
            let out = physical_ground::resolve_preserving_flags(
                physical_ground::Input {
                    position: full.current,
                    previous: full.previous,
                    velocity: full.velocity,
                    scale: full.scale,
                    ground_offset: input.ground_offset,
                    center_offset: input.center_offset,
                    gravity_mass: input.gravity_mass,
                    restitution: full.restitution,
                    retention: full.retention,
                    force_z: input.force_z,
                    flags: full.physics_flags,
                    floor: full.floor,
                    plane: full.plane,
                },
                Precision::Double53,
                &mut query,
            )?;
            full.current = out.position;
            full.previous = out.previous;
            full.velocity = out.velocity;
            full.floor = out.floor;
            full.plane = out.plane;
            full.physics_flags = out.flags;
            *s = read_state(&full, s.object_flags);
            ground_result = Some(out);
            Ok(())
        },
        |_| {
            notifications += 1;
            Ok(())
        },
    )?;
    let mut full = full.into_inner();
    write_state(&mut full, &result.state);
    let mut object_flags = result.state.object_flags;
    let relative =
        f64::from(full.current[2]) - f64::from(input.ground_offset) * f64::from(full.scale);
    if relative <= 0. {
        object_flags |= 0x100000;
        if relative <= -(f64::from(input.center_offset) * f64::from(full.scale)) {
            object_flags |= 0x8000000;
        }
    }
    if relative - f64::from(full.floor) < f64::from(0.1_f32) {
        object_flags |= 0x80000;
    }
    Ok(Output {
        state: full,
        object_flags,
        force_z: ground_result.as_ref().map_or(input.force_z, |g| g.force_z),
        mesh_responses: calls,
        ground_queries: ground_result.as_ref().map_or(0, |g| g.queries),
        junctions: ground_result.as_ref().map_or(0, |g| g.junctions),
        terrain_responses: ground_result.as_ref().map_or(0, |g| g.terrain_responses),
        passes: result.passes,
        fallback: result.fallback,
        notifications,
        collision_events: events,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn input(moving: bool) -> Input {
        Input {
            state: contact::State {
                current: [1., 0., 10.],
                previous: [if moving { 0. } else { 1. }, 0., 10.],
                velocity: [0.; 3],
                scale: 1.,
                radius: 1.,
                height: 10.,
                floor: 0.,
                plane: [0., 0., 1., 0.],
                physics_flags: 0,
                support: 0,
                hit: 0,
                support_part: 0,
                support_triangle: 0,
                restitution: 0.,
                retention: 1.,
            },
            object_flags: 0,
            collision_flags: 0,
            ground_offset: 0.,
            center_offset: 1.,
            gravity_mass: 45.,
            force_z: 0.,
        }
    }
    #[test]
    fn radius_refresh_obeys_walk_admission_even_without_candidates() {
        let mut calls = 0;
        let out = resolve_with_radius(
            input(true),
            &[],
            |_| {
                Ok(physical_ground::Ground {
                    plane: [0., 0., 1., 0.],
                    height: 0.,
                    key: [0, 0],
                })
            },
            |_| {
                calls += 1;
                Ok(3.)
            },
        )
        .unwrap();
        assert_eq!(calls, 1);
        assert_eq!(out.state.radius, 3.);
        let out = resolve_with_radius(
            input(false),
            &[],
            |_| Err("immobile query must not run"),
            |_| Err("immobile radius must not refresh"),
        )
        .unwrap();
        assert_eq!(out.state.radius, 1.);
    }
    #[test]
    fn prepared_identity_filters_self_instead_of_assuming_slot_one() {
        let objects = [Object {
            fields: mecc_collision_candidates::Object {
                id: 77,
                parent: 0,
                has_model: true,
                object_flags: 0,
                collision_flags: 0,
                bounds: volume_queries::Bounds {
                    min: [-100.; 3],
                    max: [100.; 3],
                },
            },
            response_flags: 0,
            triangles: vec![],
        }];
        let query = |_| {
            Ok(physical_ground::Ground {
                plane: [0., 0., 1., 0.],
                height: 0.,
                key: [0, 0],
            })
        };
        let self_view =
            resolve_for_actor(77, input(true), &objects, query, |s| Ok(s.radius)).unwrap();
        let other = resolve_for_actor(1, input(true), &objects, query, |s| Ok(s.radius)).unwrap();
        assert_eq!(self_view.mesh_responses, 0);
        assert!(other.mesh_responses > 0);
        assert!(resolve_for_actor(0, input(true), &objects, query, |s| Ok(s.radius)).is_err());
    }

    #[test]
    fn class_callback_observes_contact_support_and_preserves_peer_write() {
        let mut initial = input(true);
        initial.state.current = [0., 0., 0.];
        initial.state.previous = [0., 0., 1.];
        initial.state.velocity = [0., 0., -1.];
        // A nearby upward triangle establishes support before the event runs.
        let objects = [Object {
            fields: mecc_collision_candidates::Object {
                id: 77,
                parent: 0,
                has_model: true,
                object_flags: 0,
                collision_flags: 0,
                bounds: volume_queries::Bounds {
                    min: [-100.; 3],
                    max: [100.; 3],
                },
            },
            response_flags: 0,
            triangles: vec![contact::Triangle {
                vertices: [[-100., -100., 1.], [100., -100., 1.], [0., 100., 1.]],
                part: 3,
                ordinal: 4,
            }],
        }];
        let mut peer_support = 0;
        let out = resolve_for_actor_with_events(
            9,
            initial,
            &objects,
            |_| Err("terrain is disabled"),
            |s| Ok(s.radius),
            |object, state| {
                assert_eq!(object.fields.id, 77);
                assert_eq!(state.support, 77);
                assert_eq!(state.support_part, 3);
                assert_eq!(state.support_triangle, 4);
                let decision = crate::kabuto_contact::decide(
                    state.current[2],
                    state.scale,
                    Some(crate::kabuto_contact::Peer {
                        class: 2,
                        upper_z: 1.,
                        radius: 1.,
                        collision_mask: 0x3ffc,
                    }),
                )?;
                if decision.bind_peer_support {
                    peer_support = 9;
                }
                Ok(decision.handled)
            },
        );
        // The callback effect is retained even if a later provider fails.
        assert_eq!(out.err(), Some("terrain is disabled"));
        assert_eq!(peer_support, 9);
    }
}
