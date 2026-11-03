//! Standard native plane response, with the terrain-call profile selected.
//! Object-specific deep-penetration correction and volume selection are separate.
use crate::arithmetic::Precision;
#[derive(Clone, Copy, Debug)]
pub struct Input {
    pub position: [f32; 3],
    pub previous: [f32; 3],
    pub velocity: [f32; 3],
    pub plane: [f32; 4],
    pub restitution: f32,
    pub retention: f32,
    pub flags: u32,
}
#[derive(Clone, Copy, Debug)]
pub struct Output {
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub flags: u32,
    pub separation: f64,
}
pub fn terrain(input: Input, precision: Precision) -> Result<Output, &'static str> {
    respond(input, precision, false)
}
/// Object-contact profile, including the native deep-penetration bit400 gate.
pub fn object(input: Input, precision: Precision) -> Result<Output, &'static str> {
    respond(input, precision, true)
}
fn respond(input: Input, precision: Precision, object: bool) -> Result<Output, &'static str> {
    let i = input;
    if !i
        .position
        .into_iter()
        .chain(i.previous)
        .chain(i.velocity)
        .chain(i.plane)
        .chain([i.restitution, i.retention])
        .all(f32::is_finite)
    {
        return Err("non-finite plane response input");
    }
    let r = |v| precision.round(v);
    let p = i.position.map(f64::from);
    let old = i.previous.map(f64::from);
    let n = [i.plane[0], i.plane[1], i.plane[2]].map(f64::from);
    let distance = |v: [f64; 3]| {
        r(r(r(r(v[2] * n[2]) + r(n[1] * v[1])) + r(v[0] * n[0])) + f64::from(i.plane[3]))
    };
    let separation = distance(p);
    let mut out = Output {
        position: i.position,
        velocity: i.velocity,
        flags: i.flags,
        separation,
    };
    if separation < 0. {
        let bounce = f64::from(r(f64::from(i.restitution) + 1.) as f32);
        let previous_distance = distance(old);
        let mut intersection: [f64; 3] = if previous_distance < 0. {
            std::array::from_fn(|a| r(p[a] - r(separation * n[a])))
        } else {
            let t = f64::from(r(previous_distance / r(previous_distance - separation)) as f32);
            std::array::from_fn(|a| r(r(r(p[a] - old[a]) * t) + old[a]))
        };
        intersection[2] = f64::from(intersection[2] as f32);
        let corrected = [
            r(p[0] - r(r(bounce * n[0]) * separation)),
            f64::from(r(p[1] - r(r(bounce * n[1]) * separation)) as f32),
            f64::from(r(p[2] - r(r(bounce * separation) * n[2])) as f32),
        ];
        out.position = std::array::from_fn(|a| {
            r(r(r(corrected[a] - intersection[a]) * f64::from(i.retention)) + intersection[a])
                as f32
        });
        if object && i.flags & 0x400 != 0 && previous_distance < f64::from(-0.1_f32) {
            let stored = |x| f64::from(r(x) as f32);
            let corrected: [f64; 3] =
                std::array::from_fn(|a| stored(f64::from(out.position[a]) - old[a]));
            let original: [f64; 3] = std::array::from_fn(|a| stored(p[a] - old[a]));
            let dot =
                |a: [f64; 3], b: [f64; 3]| r(r(r(a[0] * b[0]) + r(a[1] * b[1])) + r(a[2] * b[2]));
            let product = stored(dot(original, corrected));
            let length2 = stored(dot(corrected, corrected));
            if product <= 0.
                || length2 <= f64::from(0.001_f32)
                || stored(dot(original, original)) <= f64::from(0.001_f32)
            {
                out.position = i.previous;
            } else {
                let factor = stored(product / r(length2.sqrt())).min(1.) - 1.;
                out.position = std::array::from_fn(|a| {
                    r(r(corrected[a] * factor) + f64::from(out.position[a])) as f32
                });
            }
        }
        let v = i.velocity.map(f64::from);
        let dot = r(r(r(n[1] * v[1]) + r(v[2] * n[2])) + r(v[0] * n[0]));
        if dot < 0. {
            let corrected = [
                r(v[0] - r(r(dot * n[0]) * bounce)),
                r(v[1] - r(r(dot * n[1]) * bounce)),
                f64::from(r(v[2] - r(r(dot * bounce) * n[2])) as f32),
            ];
            out.velocity = corrected.map(|v| r(v * f64::from(i.retention)) as f32);
        }
        out.flags |= 2;
    }
    if !out
        .position
        .into_iter()
        .chain(out.velocity)
        .all(f32::is_finite)
        || !out.separation.is_finite()
    {
        return Err("plane response overflow");
    }
    Ok(out)
}
