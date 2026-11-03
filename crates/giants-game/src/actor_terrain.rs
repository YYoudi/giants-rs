//! Nonphysical actor terrain stage, after object responses. Static terrain,
//! measured 53-bit arithmetic; dynamic ripples and object dispatch are upstream.
use crate::{collider_copy::Fields, physical_ground::Ground};

pub struct Output {
    pub fields: Fields,
    pub queries: u32,
    pub time_queries: u32,
    pub slope_responses: u32,
    pub segment_responses: u32,
}

/// `initial` is the query at the previous position retained before object
/// responses. Its first key word identifies the cell, independently of triangle.
/// Providers must use the current owned world and the actor-local time policy.
pub fn resolve(
    fields: Fields,
    slope_parameter: f32,
    initial: Ground,
    mut ground: impl FnMut([f32; 2]) -> Result<Ground, &'static str>,
    mut seconds: impl FnMut([f32; 3]) -> f64,
) -> Result<Output, &'static str> {
    if fields.physical {
        return Err("nonphysical actor terrain profile required");
    }
    if !fields
        .current
        .into_iter()
        .chain(fields.previous)
        .chain([
            fields.scale,
            fields.ground_offset,
            fields.center_offset,
            fields.floor,
            slope_parameter,
        ])
        .chain(fields.plane)
        .all(f32::is_finite)
    {
        return Err("non-finite actor terrain input");
    }
    let mut out = Output {
        fields,
        queries: 0,
        time_queries: 0,
        slope_responses: 0,
        segment_responses: 0,
    };
    let follow = fields.object_flags & 0x80000 != 0 && fields.current[2] <= fields.previous[2];
    if follow {
        follow_slope(&mut out, initial.plane, slope_parameter, &mut seconds);
    } else {
        falling(&mut out, slope_parameter, &mut ground, &mut seconds)?;
    }
    let mut current = sample(&mut out, &mut ground, None)?;
    let changed_cell = current.key[0] != initial.key[0];
    if follow {
        if changed_cell && follow_slope(&mut out, current.plane, slope_parameter, &mut seconds) {
            current = sample(&mut out, &mut ground, None)?;
        }
        project(&mut out.fields, current.plane);
    } else if changed_cell {
        falling(&mut out, slope_parameter, &mut ground, &mut seconds)?;
    }
    Ok(out)
}

fn sample(
    out: &mut Output,
    ground: &mut impl FnMut([f32; 2]) -> Result<Ground, &'static str>,
    point: Option<[f32; 3]>,
) -> Result<Ground, &'static str> {
    let point = point.unwrap_or(out.fields.current);
    let result = ground([point[0], point[1]])?;
    out.queries += 1;
    if !result
        .plane
        .into_iter()
        .chain([result.height])
        .all(f32::is_finite)
    {
        return Err("non-finite actor terrain query");
    }
    Ok(result)
}
fn slide(out: &mut Output, plane: [f32; 4], seconds: &mut impl FnMut([f32; 3]) -> f64) {
    let position = out.fields.current;
    let nx = f64::from(plane[0]);
    let ny = f64::from(plane[1]);
    // Both local-time queries precede the XY writes. Only X displacement is
    // spilled to float32; Y remains wide through addition of previous Y.
    let dx = (seconds(position) * nx * nx * nx * 30.) as f32;
    let dy = seconds(position) * ny * ny * ny * 30.;
    out.time_queries += 2;
    out.slope_responses += 1;
    out.fields.object_flags |= 0x1000_0000;
    out.fields.current[0] = (f64::from(out.fields.previous[0]) + f64::from(dx)) as f32;
    out.fields.current[1] = (f64::from(out.fields.previous[1]) + dy) as f32;
}
fn follow_slope(
    out: &mut Output,
    plane: [f32; 4],
    parameter: f32,
    seconds: &mut impl FnMut([f32; 3]) -> f64,
) -> bool {
    out.fields.response_flags &= !0x10000;
    if plane[2] < 0.64278_f32 && parameter > 0. {
        slide(out, plane, seconds);
        out.fields.response_flags |= 0x10000;
        return true;
    }
    if f64::from(plane[2]) < 0.9659 {
        for (axis, normal) in plane.into_iter().take(2).enumerate() {
            let n = f64::from(normal);
            let attenuation = (1. - n * n * f64::from(parameter) * f64::from(0.02_f32)) as f32;
            let attenuation = if attenuation < 0.2 { 0.2 } else { attenuation };
            let delta = f64::from(out.fields.current[axis]) - f64::from(out.fields.previous[axis]);
            let delta = if delta > 0. && normal < 0. || delta < 0. && normal > 0. {
                delta * f64::from(attenuation)
            } else {
                delta
            };
            out.fields.current[axis] = (f64::from(out.fields.previous[axis]) + delta) as f32;
        }
        return true;
    }
    false
}
fn plane_height(plane: [f32; 4], point: [f32; 3]) -> f32 {
    if plane[2] == 0. {
        return 0.;
    }
    (-((f64::from(point[1]) * f64::from(plane[1]) + f64::from(point[0]) * f64::from(plane[0]))
        + f64::from(plane[3]))
        / f64::from(plane[2])) as f32
}
fn project(fields: &mut Fields, plane: [f32; 4]) {
    let floor = plane_height(plane, fields.current).max(-61.1);
    let depth = -(f64::from(fields.center_offset) * f64::from(fields.scale));
    let raised_floor = if f64::from(floor) < depth {
        depth as f32
    } else {
        floor
    };
    let movement = (f64::from(fields.current[0]) - f64::from(fields.previous[0])).abs()
        + (f64::from(fields.current[1]) - f64::from(fields.previous[1])).abs();
    let tolerance = movement.max(3.);
    let target = (f64::from(raised_floor) + f64::from(fields.ground_offset)) as f32;
    if f64::from(fields.current[2]) - tolerance < f64::from(target) {
        fields.current[2] = target;
    }
    fields.floor = floor;
    fields.plane = plane;
}
fn falling(
    out: &mut Output,
    parameter: f32,
    ground: &mut impl FnMut([f32; 2]) -> Result<Ground, &'static str>,
    seconds: &mut impl FnMut([f32; 3]) -> f64,
) -> Result<(), &'static str> {
    let current = sample(out, ground, None)?;
    if out.fields.floor < current.height {
        out.fields.floor = current.height;
        out.fields.plane = current.plane;
        if out.fields.current[2] <= current.height
            && current.plane[2] < 0.64278_f32
            && parameter > 0.
        {
            slide(out, current.plane, seconds);
            return Ok(());
        }
    }
    if out.fields.floor < -61.1 {
        out.fields.floor = -61.1;
    }
    let depth = (-(f64::from(out.fields.center_offset) * f64::from(out.fields.scale))) as f32;
    let target = f64::from(out.fields.floor.max(depth)) + f64::from(out.fields.ground_offset);
    if target > f64::from(out.fields.current[2]) {
        if out.fields.collision_flags & 0x8000 != 0 {
            segment(out, ground)?;
            out.fields.response_flags |= 0x800;
        } else {
            out.fields.current[2] = target as f32;
        }
    }
    Ok(())
}
fn segment(
    out: &mut Output,
    ground: &mut impl FnMut([f32; 2]) -> Result<Ground, &'static str>,
) -> Result<(), &'static str> {
    let old = out.fields.previous;
    let new = out.fields.current;
    let mut plane = sample(out, ground, Some(old))?.plane;
    let old_distance = |p: [f32; 4]| {
        (((f64::from(p[2]) * f64::from(old[2]) + f64::from(p[1]) * f64::from(old[1]))
            + f64::from(p[0]) * f64::from(old[0]))
            + f64::from(p[3])) as f32
    };
    let new_distance = |p: [f32; 4]| {
        ((f64::from(p[0]) * f64::from(new[0]) + f64::from(p[2]) * f64::from(new[2]))
            + f64::from(p[1]) * f64::from(new[1]))
            + f64::from(p[3])
    };
    let mut old_d = old_distance(plane);
    let mut new_d = new_distance(plane);
    if f64::from(old_d) * new_d > 0. {
        plane = sample(out, ground, Some(new))?.plane;
        old_d = old_distance(plane);
        new_d = new_distance(plane);
    }
    if f64::from(old_d) * new_d <= 0. {
        let dx = f64::from(new[0]) - f64::from(old[0]);
        let denominator = ((f64::from(new[2]) - f64::from(old[2])) * f64::from(plane[2])
            + (f64::from(new[1]) - f64::from(old[1])) * f64::from(plane[1]))
            + dx * f64::from(plane[0]);
        let fraction = if denominator == 0. {
            0.
        } else {
            -f64::from(old_d) / denominator
        };
        for axis in 0..3 {
            out.fields.current[axis] = ((f64::from(new[axis]) - f64::from(old[axis])) * fraction
                + f64::from(old[axis])) as f32;
        }
    }
    out.segment_responses += 1;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn flat() -> Ground {
        Ground {
            plane: [0., 0., 1., 0.],
            height: 0.,
            key: [0, 1],
        }
    }
    fn actor() -> Fields {
        let mut s = Fields::zeroed();
        s.model = 1;
        s.scale = 1.;
        s.center_offset = 8.;
        s.plane = flat().plane;
        s.current = [1., 1., 0.];
        s.previous = [0., 0., 1.];
        s
    }
    #[test]
    fn grounded_attenuation_opposes_the_normal() {
        let plane = [0.6, 0., 0.8, 0.];
        let mut s = actor();
        s.previous = [10., 0., 1.];
        s.current = [0., 0., 0.];
        s.object_flags = 0x80000;
        let ground = Ground { plane, ..flat() };
        let out = resolve(
            s,
            100.,
            ground,
            |_| Ok(ground),
            |_| panic!("moderate slope needs no time"),
        )
        .unwrap();
        assert!(out.fields.current[0] > 7.);
        s.previous[0] = 0.;
        s.current[0] = 10.;
        let out = resolve(
            s,
            100.,
            ground,
            |_| Ok(ground),
            |_| panic!("moderate slope needs no time"),
        )
        .unwrap();
        assert_eq!(out.fields.current[0], 10.);
    }
    #[test]
    fn triangle_change_does_not_repeat_slope_response() {
        let mut s = actor();
        s.object_flags = 0x80000;
        let mut ground = flat();
        ground.key[1] = 2;
        let out = resolve(s, 0., flat(), |_| Ok(ground), |_| unreachable!()).unwrap();
        assert_eq!(out.queries, 1);
        assert_eq!(out.fields.current[2], 0.);
    }
    #[test]
    fn steep_slope_samples_time_twice_before_writing_xy() {
        let mut s = actor();
        s.object_flags = 0x80000;
        let plane = [0.8, 0., 0.6, 0.];
        let ground = Ground { plane, ..flat() };
        let mut points = Vec::new();
        let out = resolve(
            s,
            1.,
            ground,
            |_| Ok(ground),
            |p| {
                points.push(p);
                0.1
            },
        )
        .unwrap();
        assert_eq!(points, vec![s.current; 2]);
        assert_eq!(out.time_queries, 2);
        assert_eq!(out.fields.response_flags & 0x10000, 0x10000);
        assert_ne!(out.fields.object_flags & 0x10000000, 0);
    }
    #[test]
    fn falling_segment_crossing_differs_from_vertical_snap() {
        let mut s = actor();
        s.previous = [0., 0., 1.];
        s.current = [2., 0., -1.];
        s.collision_flags = 0x8000;
        let out = resolve(s, 0., flat(), |_| Ok(flat()), |_| unreachable!()).unwrap();
        assert_eq!(out.fields.current, [1., 0., 0.]);
        assert_eq!(out.segment_responses, 1);
        assert_eq!(out.fields.response_flags & 0x800, 0x800);
        s.collision_flags = 0;
        let out = resolve(s, 0., flat(), |_| Ok(flat()), |_| unreachable!()).unwrap();
        assert_eq!(out.fields.current, [2., 0., 0.]);
    }
    #[test]
    fn grounded_projection_uses_strict_movement_tolerance() {
        let mut s = actor();
        s.object_flags = 0x80000;
        s.current = [0., 0., 3.];
        s.previous = [0., 0., 3.];
        let out = resolve(s, 0., flat(), |_| Ok(flat()), |_| unreachable!()).unwrap();
        assert_eq!(out.fields.current[2], 3.);
        s.current[2] = 2.999;
        let out = resolve(s, 0., flat(), |_| Ok(flat()), |_| unreachable!()).unwrap();
        assert_eq!(out.fields.current[2], 0.);
    }
}
