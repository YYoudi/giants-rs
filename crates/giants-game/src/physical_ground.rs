//! Native physical ground wrapper, with explicit repeated ground-query records.
//! Static/dynamic query selection, scene objects and live physics fields are upstream.
use crate::{arithmetic::Precision, plane_response};
#[derive(Clone, Copy)]
pub struct Ground {
    pub plane: [f32; 4],
    pub height: f32,
    /// Stable cell identity and subtriangle, compared for equality only.
    pub key: [u32; 2],
}
impl Ground {
    /// Logical cell identity preserves the native cell-pointer equality test.
    /// The query and width must belong to the same immutable terrain grid.
    pub fn from_sample(
        sample: crate::terrain_queries::GroundSample,
        width: usize,
    ) -> Result<Self, &'static str> {
        if width == 0 || sample.cell[0] >= width {
            return Err("invalid terrain cell identity");
        }
        let cell = sample.cell[1]
            .checked_mul(width)
            .and_then(|v| v.checked_add(sample.cell[0]))
            .and_then(|v| u32::try_from(v).ok())
            .ok_or("terrain cell identity overflow")?;
        Ok(Self {
            plane: sample.plane,
            height: sample.height,
            key: [cell, u32::from(sample.triangle)],
        })
    }
}
pub struct Input {
    pub position: [f32; 3],
    pub previous: [f32; 3],
    pub velocity: [f32; 3],
    pub scale: f32,
    pub ground_offset: f32,
    pub center_offset: f32,
    pub gravity_mass: f32,
    pub restitution: f32,
    pub retention: f32,
    pub force_z: f32,
    pub flags: u32,
    pub floor: f32,
    pub plane: [f32; 4],
}
pub struct Output {
    pub position: [f32; 3],
    pub previous: [f32; 3],
    pub velocity: [f32; 3],
    pub force_z: f32,
    pub flags: u32,
    pub floor: f32,
    pub plane: [f32; 4],
    pub queries: u32,
    pub terrain_responses: u32,
    pub junctions: u32,
}
/// Two adjacent 32768-word mantissa tables, generated from scalar sqrt seeds.
pub fn sqrt_table_word(index: u16) -> u32 {
    let index = u32::from(index);
    let seed = if index < 0x8000 {
        (index | 0x400000) << 8
    } else {
        ((index - 0x8000) | 0x3f8000) << 8
    };
    f32::from_bits(seed).sqrt().to_bits() & 0x7fffff
}
fn approximate_sqrt(value: f32) -> f32 {
    if value == 0. {
        return 0.;
    }
    let bits = value.to_bits();
    let mantissa = sqrt_table_word(((bits >> 8) & 0xffff) as u16);
    let exponent = ((bits.wrapping_add(0xc0800000) >> 1).wrapping_add(0x3f800000)) & 0x7f800000;
    f32::from_bits(mantissa | exponent)
}
fn intersection(p: [f32; 4], q: [f32; 4], precision: Precision) -> Option<[f32; 3]> {
    let r = |v| precision.round(v);
    let s = |v| f64::from(r(v) as f32);
    let p = p.map(f64::from);
    let q = q.map(f64::from);
    let det = r(r(r(q[1] * p[0]) - r(q[0] * p[1])) + r(s(0.) * p[2]));
    let saved = s(det);
    if det <= f64::from(0.000001_f32) && saved >= f64::from(-0.000001_f32) {
        return None;
    }
    let inverse = s(1. / saved);
    let x = r(
        r(r(r(r(p[2] * 0.) - p[1]) * q[3]) + r(r(r(p[1] * q[2]) - r(p[2] * q[1])) * 0.))
            + r(r(q[1] - r(q[2] * 0.)) * p[3]),
    );
    let y = r(
        r(r(s(p[0] - r(0. * p[2])) * q[3]) + r(s(r(q[0] * p[2]) - r(q[2] * p[0])) * 0.))
            + r(r(r(q[2] * 0.) - q[0]) * p[3]),
    );
    let z = r(r(
        r(s(r(p[1] * 0.) - r(p[0] * 0.)) * q[3]) + r(s(r(p[0] * q[1]) - r(p[1] * q[0])) * 0.)
    ) + r(s(r(q[0] * 0.) - r(0. * q[1])) * p[3]));
    Some([
        r(-x * inverse) as f32,
        r(-y * inverse) as f32,
        r(-z * inverse) as f32,
    ])
}
pub fn resolve(
    mut input: Input,
    precision: Precision,
    query: impl FnMut([f32; 2]) -> Result<Ground, &'static str>,
) -> Result<Output, &'static str> {
    input.flags &= 0xffff_ff81;
    resolve_preserving_flags(input, precision, query)
}
/// Direct native physical-ground body, after the caller prepares contact flags.
/// Unlike the legacy wrapper, preserve an object support bit produced upstream.
pub fn resolve_preserving_flags(
    input: Input,
    precision: Precision,
    mut query: impl FnMut([f32; 2]) -> Result<Ground, &'static str>,
) -> Result<Output, &'static str> {
    if !input
        .position
        .into_iter()
        .chain(input.previous)
        .chain(input.velocity)
        .chain(input.plane)
        .chain([
            input.scale,
            input.ground_offset,
            input.center_offset,
            input.gravity_mass,
            input.restitution,
            input.retention,
            input.force_z,
            input.floor,
        ])
        .all(f32::is_finite)
    {
        return Err("non-finite physical ground input");
    }
    let r = |v| precision.round(v);
    let f = |v| f64::from(v);
    let s = |v| f(r(v) as f32);
    let offset = r(f(input.scale) * f(input.ground_offset));
    let mut position = input.position;
    position[2] = r(f(position[2]) - offset) as f32;
    let mut previous = input.previous;
    previous[2] = r(f(previous[2]) - offset) as f32;
    let mut velocity = input.velocity;
    let mut flags = input.flags;
    let mut queries = 0;
    let mut responses = 0;
    let mut junctions = 0;
    let mut last: Option<Ground> = None;
    let mut separation = 0_f32;
    let mut ground;
    loop {
        ground = query([position[0], position[1]])?;
        queries += 1;
        if !ground
            .plane
            .into_iter()
            .chain([ground.height])
            .all(f32::is_finite)
        {
            return Err("non-finite ground query result");
        }
        if let Some(old) = last {
            if ground.key == old.key {
                break;
            }
            if let Some(point) = intersection(ground.plane, old.plane, precision) {
                position = input.position;
                position[2] = r(f(input.position[2]) - offset) as f32;
                velocity = input.velocity;
                let x = r(f(ground.plane[0]) + f(old.plane[0]));
                let y = s(f(ground.plane[1]) + f(old.plane[1]));
                let z = r(f(ground.plane[2]) + f(old.plane[2]));
                let squared = r(r(r(z * z) + r(y * y)) + r(x * x)) as f32;
                let inverse = r(1. / f(approximate_sqrt(squared)));
                let n = [
                    r(inverse * x) as f32,
                    r(inverse * y) as f32,
                    r(inverse * z) as f32,
                ];
                let d = -r(r(r(f(point[2]) * f(n[2])) + r(f(point[1]) * f(n[1])))
                    + r(f(point[0]) * f(n[0])));
                ground.plane = [n[0], n[1], n[2], d as f32];
                ground.key[0] = u32::MAX;
                junctions += 1;
            }
        }
        let out = plane_response::terrain(
            plane_response::Input {
                position,
                previous,
                velocity,
                plane: ground.plane,
                restitution: input.restitution,
                retention: input.retention,
                flags,
            },
            precision,
        )?;
        position = out.position;
        velocity = out.velocity;
        flags = out.flags;
        responses += 1;
        separation = out.separation as f32;
        if out.separation > 0. {
            break;
        }
        flags |= 4;
        if responses == 3 {
            break;
        }
        last = Some(ground);
    }
    let mut force_z = input.force_z;
    if junctions != 0 && velocity[2] > 0. {
        force_z = r(f(force_z) - r(f(input.gravity_mass) * f(32.17405_f32))) as f32;
    }
    if separation < 0.05_f32 {
        flags |= 8;
    }
    let cap = r(f(input.center_offset) * f(input.scale)) as f32;
    let cap_plane = [0., 0., 1., cap];
    let out = plane_response::terrain(
        plane_response::Input {
            position,
            previous,
            velocity,
            plane: cap_plane,
            restitution: input.restitution,
            retention: input.retention,
            flags,
        },
        precision,
    )?;
    position = out.position;
    velocity = out.velocity;
    flags = out.flags;
    let mut floor = input.floor;
    let mut plane = input.plane;
    if out.separation < f(0.05_f32) {
        flags |= 0x40;
        plane = cap_plane;
        floor =
            (-r(r(r(0. * f(input.position[1])) + r(f(input.position[0]) * 0.)) + f(cap))) as f32;
    } else if flags & 0x20 == 0 {
        floor = ground.height;
        plane = ground.plane;
    }
    position[2] = r(f(position[2]) + offset) as f32;
    previous[2] = r(offset + f(previous[2])) as f32;
    if !position
        .into_iter()
        .chain(previous)
        .chain(velocity)
        .chain(plane)
        .chain([floor, force_z])
        .all(f32::is_finite)
    {
        return Err("physical ground output overflow");
    }
    Ok(Output {
        position,
        previous,
        velocity,
        force_z,
        flags,
        floor,
        plane,
        queries,
        terrain_responses: responses,
        junctions,
    })
}
