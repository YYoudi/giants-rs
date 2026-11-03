//! Gravity, contact-plane projection, bounded tangential friction and damping.
//! The caller supplies established contact flags/normal and initialized fields.
use crate::arithmetic::Precision;
#[derive(Clone, Copy, Debug)]
pub struct Input {
    pub force: [f32; 3],
    pub velocity: [f32; 3],
    pub normal: [f32; 3],
    pub gravity_mass: f32,
    pub mass: f32,
    pub friction_limit: f32,
    pub damping_xy: f32,
    pub damping_z: f32,
    pub force_time_limit: f32,
    pub object_flags: u32,
    pub physics_flags: u32,
    pub project_contact: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct Output {
    pub force: [f32; 3],
    pub force_time_limit: f32,
}
pub fn apply(i: Input, precision: Precision) -> Result<Output, &'static str> {
    if !i
        .force
        .into_iter()
        .chain(i.velocity)
        .chain(i.normal)
        .chain([
            i.gravity_mass,
            i.mass,
            i.friction_limit,
            i.damping_xy,
            i.damping_z,
            i.force_time_limit,
        ])
        .all(f32::is_finite)
        || i.mass <= 0.
        || i.friction_limit < 0.
        || i.damping_xy < 0.
        || i.damping_z < 0.
    {
        return Err("gravity outside finite positive-mass nonnegative-damping profile");
    }
    let mut out = Output {
        force: i.force,
        force_time_limit: i.force_time_limit,
    };
    if i.object_flags & 0x10 != 0 {
        return Ok(out);
    }
    let r = |v| precision.round(v);
    out.force[2] =
        r(f64::from(out.force[2]) - r(f64::from(i.gravity_mass) * f64::from(32.17405f32))) as f32;
    if i.project_contact && i.physics_flags & 0x68 != 0 {
        let f = out.force.map(f64::from);
        let n = i.normal.map(f64::from);
        let dot = r(r(r(f[2] * n[2]) + r(f[1] * n[1])) + r(f[0] * n[0]));
        if dot < 0. {
            let projected: [f64; 3] = std::array::from_fn(|a| r(f[a] - r(dot * n[a])));
            out.force = projected.map(|v| v as f32);
            let x = f64::from(out.force[0]);
            let y = f64::from(out.force[1]);
            let z = projected[2];
            let length = r(r(r(r(z * z) + r(y * y)) + r(x * x)).sqrt()) as f32;
            let factor = if length > i.friction_limit {
                r(f64::from(i.friction_limit) / f64::from(length))
            } else {
                1.
            };
            out.force[0] = r(f64::from(out.force[0]) - r(x * factor)) as f32;
            out.force[1] = r(f64::from(out.force[1]) - r(y * factor)) as f32;
            out.force[2] = r(f64::from(out.force[2]) - r(factor * z)) as f32;
        }
    }
    for a in 0..3 {
        let damping = if a == 2 { i.damping_z } else { i.damping_xy };
        out.force[a] =
            r(f64::from(out.force[a]) - r(f64::from(i.velocity[a]) * f64::from(damping))) as f32;
    }
    for damping in [i.damping_xy, i.damping_z] {
        if damping != 0. {
            let cap = r(f64::from(i.mass) / f64::from(damping));
            if out.force_time_limit == 0. || cap < f64::from(out.force_time_limit) {
                out.force_time_limit = cap as f32;
            }
        }
    }
    if !out
        .force
        .into_iter()
        .chain([out.force_time_limit])
        .all(f32::is_finite)
    {
        return Err("gravity output overflow");
    }
    Ok(out)
}
