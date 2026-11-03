//! Measured ANM static/TCB and rigid-parent LOOKAT path. Unsupported tracks fail.
pub use giants_formats::anm::Animation;
use giants_formats::{
    Error,
    anm::{Node, floats, read_bytes as slice, read_word as u32_at},
};
pub fn load_first_director(
    root: &std::path::Path,
) -> Result<Animation, Box<dyn std::error::Error>> {
    load_director(root, "story4_T_square_2")
}
pub fn load_director(
    root: &std::path::Path,
    sequence: &str,
) -> Result<Animation, Box<dyn std::error::Error>> {
    let path = crate::flick::sequence_path(root, sequence)?;
    Ok(giants_formats::anm::inline_flk(&crate::read_asset(path)?)?)
}

#[derive(Clone, Debug)]
pub struct Pose {
    pub position: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
    pub flags: u32,
}
/// Source TCB time easing at the controlled Double53 arithmetic profile.
/// This maps a key interval fraction, not elapsed game or cinematic time.
pub fn tcb_ease(t: f32, mut outgoing: f32, mut incoming: f32) -> Result<f32, Error> {
    if ![t, outgoing, incoming].into_iter().all(f32::is_finite)
        || !(0. ..=1.).contains(&t)
        || outgoing < 0.
        || incoming < 0.
    {
        return Err(Error(
            "TCB easing inputs outside measured nonnegative interval profile",
        ));
    }
    let sum = (f64::from(outgoing) + f64::from(incoming)) as f32;
    if !sum.is_finite() {
        return Err(Error("TCB easing sum overflow"));
    }
    if t == 0. || t == 1. || sum == 0. {
        return Ok(t);
    }
    if sum > 1. {
        let inverse = 1. / f64::from(sum);
        outgoing = (f64::from(outgoing) * inverse) as f32;
        incoming = (f64::from(incoming) * inverse) as f32;
    }
    let factor = 1. / ((2. - f64::from(outgoing)) - f64::from(incoming));
    let time = f64::from(t);
    Ok(if t < outgoing {
        ((factor / f64::from(outgoing)) * time) * time
    } else if time < 1. - f64::from(incoming) {
        (time * 2. - f64::from(outgoing)) * factor
    } else {
        let tail = 1. - time;
        1. - ((factor / f64::from(incoming)) * tail) * tail
    } as f32)
}

/// Complete affine step-key payloads. Before the first key the original
/// interval selector supplies the last key; there is no interpolation here.
pub fn affine_step_track(data: &[u8], at: usize, tick: i32) -> Result<Pose, Error> {
    if slice(data, at, 4)? != [10, 0, 0, 0] {
        return Err(Error("unsupported complete affine track profile"));
    }
    let count = u32_at(data, at + 8)? as usize;
    if !(1..=4096).contains(&count) {
        return Err(Error("affine step key budget"));
    }
    let mut keys = Vec::with_capacity(count);
    for index in 0..count {
        let pointer = u32_at(data, at + 12 + index * 4)? as usize;
        if pointer == 0 || !pointer.is_multiple_of(4) {
            return Err(Error("unaligned affine step key"));
        }
        slice(data, pointer, 48)?;
        keys.push((u32_at(data, pointer)? as i32, pointer));
    }
    if keys.windows(2).any(|pair| pair[0].0 >= pair[1].0) {
        return Err(Error("unordered affine step keys"));
    }
    let index = keys
        .iter()
        .rposition(|(time, _)| *time <= tick)
        .unwrap_or(count - 1);
    let pointer = keys[index].1;
    let flags = u32_at(data, pointer + 44)?;
    if flags & !31 != 0 {
        return Err(Error("unsupported affine step flags"));
    }
    Ok(Pose {
        position: floats(data, pointer + 4)?,
        rotation: floats(data, pointer + 16)?,
        scale: floats(data, pointer + 32)?,
        flags,
    })
}
#[derive(Debug)]
pub struct Camera {
    pub view: [f32; 16],
    pub projection: [f32; 16],
    pub origin: [f32; 3],
    pub viewport: [u32; 4],
    pub lod_scale: f32,
}
fn d(v: f32) -> f64 {
    f64::from(v)
}
pub(crate) fn simd_trig(angle: f32, cosine: bool) -> f32 {
    let x = if cosine {
        angle.abs() + std::f32::consts::FRAC_PI_2
    } else {
        angle.abs()
    };
    let x = x * f32::from_bits(0x3f22f983);
    let quadrant = x as i32;
    let mut x = (x - quadrant as f32).min(1.);
    if quadrant & 1 != 0 {
        x = 1. - x;
    }
    let x2 = x * x;
    let mut v = x2 * f32::from_bits(0xbb996966);
    v += f32::from_bits(0x3da335e3);
    v *= x2;
    v += f32::from_bits(0xbf255de7);
    v *= x2;
    v += std::f32::consts::FRAC_PI_2;
    let value = x * v;
    if (quadrant & 2 != 0) ^ (!cosine && angle.is_sign_negative()) {
        -value
    } else {
        value
    }
}
pub(crate) fn normalize(v: [f32; 3]) -> Result<[f32; 3], Error> {
    let sum = v.into_iter().map(|x| d(x) * d(x)).sum::<f64>() as f32;
    let length = d(sum).sqrt() as f32;
    if length == 0. || !length.is_finite() {
        return Err(Error("degenerate LOOKAT vector"));
    }
    let inv = (1. / d(length)) as f32;
    Ok(v.map(|x| (d(x) * d(inv)) as f32))
}
pub(crate) fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [(1, 2), (2, 0), (0, 1)].map(|(j, k)| (d(a[j]) * d(b[k]) - d(a[k]) * d(b[j])) as f32)
}
pub(crate) fn matrix_quat(m: [[f32; 3]; 3]) -> Result<[f32; 4], Error> {
    let trace = d(m[0][0]) + d(m[1][1]) + d(m[2][2]);
    let mut q = [0.; 4];
    if trace > 0. {
        let s = d((trace + 1.) as f32).sqrt();
        let r = 0.5 / s;
        q[3] = (0.5 * s) as f32;
        for (i, (j, k)) in [(1, 2), (2, 0), (0, 1)].into_iter().enumerate() {
            q[i] = ((d(m[k][j]) - d(m[j][k])) * r) as f32;
        }
    } else {
        let mut i = usize::from(m[0][0] < m[1][1]);
        if m[i][i] < m[2][2] {
            i = 2;
        }
        let j = (i + 1) % 3;
        let k = (i + 2) % 3;
        let s = d(((d(m[i][i]) - (d(m[k][k]) + d(m[j][j]))) + 1.) as f32).sqrt();
        let r = 0.5 / s;
        q[i] = (0.5 * s) as f32;
        q[3] = ((d(m[k][j]) - d(m[j][k])) * r) as f32;
        q[j] = ((d(m[j][i]) + d(m[i][j])) * r) as f32;
        q[k] = ((d(m[k][i]) + d(m[i][k])) * r) as f32;
    }
    if !q.iter().all(|v| v.is_finite()) {
        return Err(Error("invalid LOOKAT quaternion"));
    }
    Ok(q)
}
pub(crate) fn quat_mul(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    let [x, y, z, w] = a.map(d);
    let [u, v, r, s] = b.map(d);
    [
        (s * x + w * u + y * r) - z * v,
        (z * u + y * s + w * v) - x * r,
        (x * v + z * s + w * r) - y * u,
        ((s * w - x * u) - y * v) - z * r,
    ]
    .map(|v| v as f32)
}
pub fn pose_matrix(p: &Pose) -> [f32; 16] {
    let t = p.position;
    let mut m = [
        1., 0., 0., t[0], 0., 1., 0., t[1], 0., 0., 1., t[2], 0., 0., 0., 1.,
    ];
    if p.flags & 1 != 0 {
        let [x, y, z, w] = p.rotation.map(d);
        let xx = d((2. * x * x) as f32);
        let yy = d((2. * y * y) as f32);
        let zz = 2. * z * z;
        let xy = d((2. * y * x) as f32);
        let xz = d((2. * z * x) as f32);
        let yz = 2. * z * y;
        let xw = d((2. * x * w) as f32);
        let yw = d((2. * y * w) as f32);
        let zw = d((2. * z * w) as f32);
        for (i, v) in [
            (0, 1. - (yy + zz)),
            (1, xy - zw),
            (2, xz + yw),
            (4, xy + zw),
            (5, 1. - (zz + xx)),
            (6, yz - xw),
            (8, xz - yw),
            (9, yz + xw),
            (10, 1. - (yy + xx)),
        ] {
            m[i] = v as f32;
        }
    }
    if p.flags & 16 == 0 {
        m[3] = 0.;
        m[7] = 0.;
        m[11] = 0.;
    }
    if p.flags & 10 != 0 {
        let sign = if p.flags & 8 != 0 { -1. } else { 1. };
        for row in 0..3 {
            for col in 0..3 {
                m[row * 4 + col] = (d(m[row * 4 + col]) * d(p.scale[col]) * sign) as f32;
            }
        }
    }
    m
}
/// Measured affine inverse. Flags select identity, rigid transpose, or the
/// scalar cofactor inverse. Singular and non-affine inputs remain errors.
pub fn inverse_affine(m: [f32; 16], flags: u32) -> Result<[f32; 16], Error> {
    if flags & !31 != 0 || m.iter().any(|v| !v.is_finite()) || m[12..] != [0., 0., 0., 1.] {
        return Err(Error("unsupported affine inverse input"));
    }
    let mut out = [0.; 16];
    out[15] = 1.;
    if flags & 6 != 0 {
        let [a, b, c, _, d0, e, f, _, g, h, i, ..] = m.map(d);
        let ei_fh = i * e - f * h;
        let det = (ei_fh * a - (i * d0 - g * f) * b) + (d0 * h - g * e) * c;
        if det == 0. || !det.is_finite() {
            return Err(Error("singular affine inverse"));
        }
        let r = 1. / det;
        let cofactors = [
            ei_fh,
            h * c - i * b,
            f * b - c * e,
            g * f - i * d0,
            i * a - g * c,
            d0 * c - f * a,
            d0 * h - g * e,
            g * b - a * h,
            a * e - d0 * b,
        ];
        for (index, value) in [0, 1, 2, 4, 5, 6, 8, 9, 10].into_iter().zip(cofactors) {
            out[index] = (value * r) as f32;
        }
    } else {
        for row in 0..3 {
            for col in 0..3 {
                out[row * 4 + col] = if flags & 15 == 0 {
                    f32::from(row == col)
                } else {
                    m[col * 4 + row]
                };
            }
        }
    }
    if flags & 16 != 0 {
        for row in 0..3 {
            out[row * 4 + 3] = -((d(out[row * 4 + 2]) * d(m[11]) + d(out[row * 4]) * d(m[3]))
                + d(out[row * 4 + 1]) * d(m[7])) as f32;
        }
    }
    if out.iter().any(|v| !v.is_finite()) {
        return Err(Error("nonfinite affine inverse"));
    }
    Ok(out)
}
#[derive(Clone)]
struct Key {
    tick: i32,
    ease_in: f32,
    ease_out: f32,
    outgoing: [f32; 3],
    incoming: [f32; 3],
    value: [f32; 3],
}
fn vector_bezier(data: &[u8], at: usize, tick: i32) -> Result<[f32; 3], Error> {
    let count = u32_at(data, at + 8)? as usize;
    if !(1..=4096).contains(&count) {
        return Err(Error("invalid Bezier key count"));
    }
    let mut keys = Vec::with_capacity(count);
    for i in 0..count {
        let p = u32_at(data, at + 12 + i * 4)? as usize;
        if !p.is_multiple_of(4) {
            return Err(Error("unaligned Bezier key"));
        }
        let v = floats::<9>(data, p + 8)?;
        keys.push((u32_at(data, p)? as i32, v, u32_at(data, p + 4)?));
    }
    if keys.windows(2).any(|w| w[0].0 >= w[1].0) {
        return Err(Error("unordered Bezier keys"));
    }
    if tick <= keys[0].0 {
        return Ok(keys[0].1[6..9].try_into().unwrap());
    }
    if tick >= keys[count - 1].0 {
        return Ok(keys[count - 1].1[6..9].try_into().unwrap());
    }
    let pair = keys
        .windows(2)
        .find(|w| w[0].0 <= tick && tick < w[1].0)
        .ok_or(Error("Bezier interval missing"))?;
    let (a, b) = (&pair[0], &pair[1]);
    let t = ((i64::from(tick) - i64::from(a.0)) as f64 / (i64::from(b.0) - i64::from(a.0)) as f64)
        as f32;
    let u = (1. - d(t)) as f32;
    // The PC sampler holds an endpoint for either of these tangent modes.
    // Other key bits do not enter the sampled-value calculation.
    if a.2 & 0x1c00 == 0x800 || b.2 & 0x380 == 0x100 {
        return Ok(if t == 1. { &b.1[6..9] } else { &a.1[6..9] }
            .try_into()
            .unwrap());
    }
    let t2 = (d(t) * d(t)) as f32;
    let factor = ((i64::from(b.0) - i64::from(a.0)) as f64 * d(1f32 / 3.)) as f32;
    Ok(std::array::from_fn(|i| {
        let p0 = d(a.1[6 + i]);
        let p1 = d(b.1[6 + i]);
        let c0 = p0 + d(factor) * d(a.1[3 + i]);
        let c1 = p1 + d(factor) * d(b.1[i]);
        let lead = (c0 * (3. * d(t)) + p0 * d(u)) * d(u) + c1 * (3. * d(t2));
        lead * d(u) + p1 * d(t) * d(t2)
    })
    .map(|v| v as f32))
}
fn vector(data: &[u8], at: usize, tick: i32) -> Result<[f32; 3], Error> {
    if slice(data, at, 4)? == [6, 1, 1, 0] {
        let count = u32_at(data, at + 8)? as usize;
        if !(1..=4096).contains(&count) {
            return Err(Error("invalid linear vector key count"));
        }
        let mut keys = Vec::with_capacity(count);
        for i in 0..count {
            let p = u32_at(data, at + 12 + i * 4)? as usize;
            if !p.is_multiple_of(4) {
                return Err(Error("unaligned linear vector key"));
            }
            keys.push((u32_at(data, p)? as i32, floats::<3>(data, p + 4)?));
        }
        if keys.windows(2).any(|w| {
            w[0].0 >= w[1].0 || i64::from(w[1].0) - i64::from(w[0].0) > i64::from(i32::MAX)
        }) {
            return Err(Error("unordered or oversized linear vector interval"));
        }
        let (a, b, t) = if tick < keys[0].0 {
            (&keys[count - 1], &keys[0], 1.)
        } else if tick >= keys[count - 1].0 {
            (&keys[count - 1], &keys[0], 0.)
        } else {
            let right = keys.partition_point(|k| k.0 <= tick);
            let (a, b) = (&keys[right - 1], &keys[right]);
            (
                a,
                b,
                ((i64::from(tick) - i64::from(a.0)) as f64
                    / (i64::from(b.0) - i64::from(a.0)) as f64) as f32,
            )
        };
        let u = (1. - d(t)) as f32;
        return Ok(std::array::from_fn(|i| {
            (d(b.1[i]) * d(t) + d(a.1[i]) * d(u)) as f32
        }));
    }
    if slice(data, at, 4)? == [5, 1, 1, 0] {
        return vector_bezier(data, at, tick);
    }
    if slice(data, at, 4)? != [4, 1, 1, 0] {
        return Err(Error("unsupported vector track profile"));
    }
    let count = u32_at(data, at + 8)? as usize;
    if !(1..=4096).contains(&count) {
        return Err(Error("invalid TCB key count"));
    }
    let mut keys = Vec::with_capacity(count);
    for i in 0..count {
        let p = u32_at(data, at + 12 + i * 4)? as usize;
        if !p.is_multiple_of(4) {
            return Err(Error("unaligned TCB key"));
        }
        let v = floats::<14>(data, p + 4)?;
        keys.push(Key {
            tick: u32_at(data, p)? as i32,
            ease_in: v[3],
            ease_out: v[4],
            outgoing: v[5..8].try_into().unwrap(),
            incoming: v[8..11].try_into().unwrap(),
            value: v[11..14].try_into().unwrap(),
        });
    }
    if keys.windows(2).any(|w| w[0].tick >= w[1].tick) {
        return Err(Error("unordered TCB keys"));
    }
    if tick <= keys[0].tick {
        return Ok(keys[0].value);
    }
    if tick >= keys[count - 1].tick {
        return Ok(keys[count - 1].value);
    }
    let pair = keys
        .windows(2)
        .find(|w| w[0].tick <= tick && tick < w[1].tick)
        .ok_or(Error("TCB interval missing"))?;
    let (a, b) = (&pair[0], &pair[1]);
    let t = d(tcb_ease(
        ((i64::from(tick) - i64::from(a.tick)) as f64
            / (i64::from(b.tick) - i64::from(a.tick)) as f64) as f32,
        a.ease_out,
        b.ease_in,
    )?);
    let t2 = t * t;
    let t3 = t2 * t;
    let u = 2. * t3 - 3. * t2;
    let weights = [u + 1., -u, (t - 2. * t2) + t3, t3 - t2].map(|v| d(v as f32));
    Ok(std::array::from_fn(|i| {
        (((weights[0] * d(a.value[i]) + weights[3] * d(b.incoming[i]))
            + weights[1] * d(b.value[i]))
            + weights[2] * d(a.outgoing[i])) as f32
    }))
}
/// Bounded raw vector-track entry point for independent controls.
pub fn vector_track(data: &[u8], at: usize, tick: i32) -> Result<[f32; 3], Error> {
    vector(data, at, tick)
}
fn slerp(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    let dot = ((d(a[3]) * d(b[3]) + d(a[2]) * d(b[2])) + d(a[1]) * d(b[1])) + d(a[0]) * d(b[0]);
    if dot + 1. <= d(1e-6f32) {
        return b;
    }
    let dot = d(dot as f32);
    let (wa, wb) = if 1. - dot > d(1e-6f32) {
        let angle = dot.acos() as f32;
        let inv = (1. / d(simd_trig(angle, false))) as f32;
        let wa = (d(simd_trig(((1. - d(t)) * d(angle)) as f32, false)) * d(inv)) as f32;
        let wb = d(simd_trig((d(t) * d(angle)) as f32, false)) * d(inv);
        (wa, wb)
    } else {
        ((1. - d(t)) as f32, d(t))
    };
    std::array::from_fn(|i| (d(wa) * d(a[i]) + wb * d(b[i])) as f32)
}
struct QuatKey {
    tick: i32,
    values: [f32; 17],
}
/// Evaluate the measured TCB or linear quaternion track profiles. Linear
/// clamping still calls the native-style spherical interpolation arithmetic.
pub fn quaternion_track(data: &[u8], at: usize, tick: i32) -> Result<[f32; 4], Error> {
    if slice(data, at, 4)? == [9, 1, 1, 0] {
        return quaternion_linear(data, at, tick);
    }
    let profile = slice(data, at, 4)?;
    let repeating = profile == [7, 3, 3, 1];
    if profile != [7, 1, 1, 0] && !repeating {
        return Err(Error("unsupported quaternion track profile"));
    }
    let count = u32_at(data, at + 8)? as usize;
    if !(1..=4096).contains(&count) {
        return Err(Error("invalid quaternion key count"));
    }
    let mut keys = Vec::with_capacity(count);
    for i in 0..count {
        let p = u32_at(data, at + 12 + i * 4)? as usize;
        if !p.is_multiple_of(4) {
            return Err(Error("unaligned quaternion key"));
        }
        keys.push(QuatKey {
            tick: u32_at(data, p)? as i32,
            values: floats(data, p + 4)?,
        });
    }
    if keys.windows(2).any(|w| w[0].tick >= w[1].tick) {
        return Err(Error("unordered quaternion keys"));
    }
    let mut tick = tick;
    if repeating {
        let first = keys[0].tick;
        let last = keys[count - 1].tick;
        let span = last
            .checked_sub(first)
            .ok_or(Error("quaternion repeat span overflow"))?;
        if span != 0 {
            if tick < first {
                let delta = last
                    .checked_sub(tick)
                    .ok_or(Error("quaternion repeat time overflow"))?;
                tick = last - delta % span;
            } else if tick > last {
                let delta = tick
                    .checked_sub(first)
                    .ok_or(Error("quaternion repeat time overflow"))?;
                tick = first + delta % span;
            }
        }
    }
    // Native clamping still calls SQUAD, including its non-unit trig roundoff.
    let (a, b, t) = if tick < keys[0].tick {
        (&keys[count - 1], &keys[0], 1.)
    } else if tick >= keys[count - 1].tick {
        (&keys[count - 1], &keys[0], 0.)
    } else {
        let pair = keys
            .windows(2)
            .find(|w| w[0].tick <= tick && tick < w[1].tick)
            .ok_or(Error("quaternion interval missing"))?;
        let (a, b) = (&pair[0], &pair[1]);
        let t = ((i64::from(tick) - i64::from(a.tick)) as f64
            / (i64::from(b.tick) - i64::from(a.tick)) as f64) as f32;
        (a, b, t)
    };
    let t = tcb_ease(t, a.values[4], b.values[3])?;
    let values = slerp(
        a.values[13..17].try_into().unwrap(),
        b.values[13..17].try_into().unwrap(),
        t,
    );
    let tangents = slerp(
        a.values[5..9].try_into().unwrap(),
        b.values[9..13].try_into().unwrap(),
        t,
    );
    let result = slerp(values, tangents, (2. * (1. - d(t)) * d(t)) as f32);
    if !result.iter().all(|v| v.is_finite()) {
        return Err(Error("non-finite quaternion evaluation"));
    }
    Ok(result)
}
fn quaternion_linear(data: &[u8], at: usize, tick: i32) -> Result<[f32; 4], Error> {
    let count = u32_at(data, at + 8)? as usize;
    if !(1..=4096).contains(&count) {
        return Err(Error("invalid linear quaternion key count"));
    }
    let mut keys = Vec::with_capacity(count);
    for i in 0..count {
        let p = u32_at(data, at + 12 + i * 4)? as usize;
        if !p.is_multiple_of(4) {
            return Err(Error("unaligned linear quaternion key"));
        }
        keys.push((u32_at(data, p)? as i32, floats::<4>(data, p + 4)?));
    }
    if keys.windows(2).any(|w| w[0].0 >= w[1].0) {
        return Err(Error("unordered linear quaternion keys"));
    }
    let (a, b, t) = if tick < keys[0].0 {
        (&keys[count - 1].1, &keys[0].1, 1.)
    } else if tick >= keys[count - 1].0 {
        (&keys[count - 1].1, &keys[0].1, 0.)
    } else {
        let pair = keys
            .windows(2)
            .find(|w| w[0].0 <= tick && tick < w[1].0)
            .ok_or(Error("linear quaternion interval missing"))?;
        let t = ((i64::from(tick) - i64::from(pair[0].0)) as f64
            / (i64::from(pair[1].0) - i64::from(pair[0].0)) as f64) as f32;
        (&pair[0].1, &pair[1].1, t)
    };
    let result = slerp(*a, *b, t);
    if !result.iter().all(|v| v.is_finite()) {
        return Err(Error("non-finite linear quaternion evaluation"));
    }
    Ok(result)
}
pub struct Evaluator<'a> {
    pub animation: &'a Animation,
}
impl Evaluator<'_> {
    pub fn affine(&self, offset: usize, tick: i32) -> Result<Pose, Error> {
        self.affine_inner(offset, tick, &mut Vec::new())
    }
    fn affine_inner(
        &self,
        offset: usize,
        tick: i32,
        stack: &mut Vec<usize>,
    ) -> Result<Pose, Error> {
        if stack.contains(&offset) || stack.len() >= 128 {
            return Err(Error("cyclic/deep ANM evaluation"));
        }
        stack.push(offset);
        let n = self.animation.node(offset)?;
        let data = self.animation.bytes();
        if n.transform > 3 || n.callback != 0 {
            return Err(Error("unsupported ANM node evaluator/callback"));
        }
        if n.transform == 1 {
            let mut pose = affine_step_track(data, n.tracks[1], tick)?;
            for value in &mut pose.rotation[..3] {
                *value = -*value;
            }
            stack.pop();
            return Ok(pose);
        }
        let mut p = Pose {
            position: n.translation,
            rotation: std::array::from_fn(|i| if i < 3 { -n.rotation[i] } else { n.rotation[i] }),
            scale: n.scale,
            flags: n.flags,
        };
        if n.tracks[0] != 0 {
            p.position = vector(data, n.tracks[0], tick)?;
            p.flags |= 16;
        }
        if n.transform == 3 {
            let target = self.matrix_inner(n.target, tick, stack)?;
            let parent = if n.parent != 0 {
                let value = self.matrix_inner(n.parent, tick, stack)?;
                Some((self.matrix_flags(n.parent, tick)?, value))
            } else {
                None
            };
            self.resolve_look_at(n, tick, &mut p, target, parent)?;
        } else if n.tracks[1] != 0 {
            p.rotation = quaternion_track(data, n.tracks[1], tick)?;
            for v in &mut p.rotation[..3] {
                *v = -*v;
            }
            p.flags |= 1;
        }
        if n.tracks[2] != 0 {
            p.scale = vector(data, n.tracks[2], tick)?;
            if p.flags & 8 != 0 {
                p.scale = p.scale.map(|v| -v);
            }
            p.flags |= 2;
        }
        stack.pop();
        Ok(p)
    }
    fn resolve_look_at(
        &self,
        n: &giants_formats::anm::Node,
        tick: i32,
        p: &mut Pose,
        mut target: [f32; 16],
        parent: Option<(u32, [f32; 16])>,
    ) -> Result<(), Error> {
        let data = self.animation.bytes();
        let mut up_axis = [0., 0., 1.];
        if let Some((flags, parent)) = parent {
            let inverse = inverse_affine(parent, flags)?;
            let position = [target[3], target[7], target[11]];
            for (row, order) in [[1, 0, 2], [0, 1, 2], [2, 1, 0]].into_iter().enumerate() {
                let [a, b, c] = order;
                // The original transform accumulates each row in its own
                // order, retaining wide intermediates until the final store.
                target[row * 4 + 3] = (((d(inverse[row * 4 + a]) * d(position[a])
                    + d(inverse[row * 4 + b]) * d(position[b]))
                    + d(inverse[row * 4 + c]) * d(position[c]))
                    + d(inverse[row * 4 + 3])) as f32;
            }
            up_axis = [inverse[2], inverse[6], inverse[10]];
        }
        let back = normalize(std::array::from_fn(|i| {
            (d(p.position[i]) - d(target[i * 4 + 3])) as f32
        }))?;
        let right = normalize(cross(up_axis, back))?;
        let up = cross(back, right);
        if n.tracks[1] != 0 {
            let roll = crate::scalar_track::evaluate(data, n.tracks[1], tick)?;
            if roll.abs() > 1e6 {
                return Err(Error("LOOKAT roll outside profile"));
            }
            let half = roll * 0.5;
            let basis = std::array::from_fn(|i| [right[i], up[i], back[i]]);
            p.rotation = quat_mul(
                matrix_quat(basis)?,
                [0., 0., simd_trig(half, false), simd_trig(half, true)],
            );
        }
        Ok(())
    }
    /// Evaluate the local LOOKAT pose using the retained matrices selected by
    /// the caller's hierarchy/epoch consumer, including mixed parent caches.
    pub(crate) fn affine_with_look_at_matrices(
        &self,
        offset: usize,
        tick: i32,
        target: [f32; 16],
        parent: Option<(u32, [f32; 16])>,
    ) -> Result<Pose, Error> {
        let n = self.animation.node(offset)?;
        if n.transform != 3 || n.callback != 0 {
            return Err(Error("resolved LOOKAT requires an ordinary declared node"));
        }
        let data = self.animation.bytes();
        let mut p = Pose {
            position: n.translation,
            rotation: std::array::from_fn(|i| if i < 3 { -n.rotation[i] } else { n.rotation[i] }),
            scale: n.scale,
            flags: n.flags,
        };
        if n.tracks[0] != 0 {
            p.position = vector(data, n.tracks[0], tick)?;
            p.flags |= 16;
        }
        self.resolve_look_at(n, tick, &mut p, target, parent)?;
        if n.tracks[2] != 0 {
            p.scale = vector(data, n.tracks[2], tick)?;
            if p.flags & 8 != 0 {
                p.scale = p.scale.map(|v| -v);
            }
            p.flags |= 2;
        }
        Ok(p)
    }
    pub fn matrix_flags(&self, offset: usize, tick: i32) -> Result<u32, Error> {
        let mut chain = Vec::new();
        let mut at = offset;
        while at != 0 {
            if chain.len() >= 128 || chain.contains(&at) {
                return Err(Error("cyclic/deep matrix flag chain"));
            }
            chain.push(at);
            at = self.animation.node(at)?.parent;
        }
        let mut flags = 0;
        for node in chain.into_iter().rev() {
            let local = self.affine(node, tick)?.flags;
            let union = flags | local;
            let mut result = union ^ ((union ^ flags ^ local) & 8);
            if flags & 2 != 0 && local & 1 != 0 {
                result |= 4;
            }
            flags = result;
        }
        Ok(flags)
    }
    pub fn matrix(&self, offset: usize, tick: i32) -> Result<[f32; 16], Error> {
        self.matrix_inner(offset, tick, &mut Vec::new())
    }
    /// Shared ordinary-node cache keyed by the caller's native epoch. A node
    /// already stamped with this epoch keeps its matrix even if tick changes.
    /// Procedural/LOOKAT updates require their own acquired retained backend.
    pub fn retained_matrix(
        &self,
        offset: usize,
        tick: i32,
        epoch: u32,
        cache: &mut std::collections::BTreeMap<usize, (u32, [f32; 16])>,
    ) -> Result<[f32; 16], Error> {
        fn visit(
            e: &Evaluator<'_>,
            offset: usize,
            tick: i32,
            epoch: u32,
            cache: &mut std::collections::BTreeMap<usize, (u32, [f32; 16])>,
            active: &mut Vec<usize>,
        ) -> Result<[f32; 16], Error> {
            if let Some((stamp, matrix)) = cache.get(&offset)
                && *stamp == epoch
            {
                return Ok(*matrix);
            }
            if active.contains(&offset) || active.len() >= 128 {
                return Err(Error("retained animation parent cycle/depth"));
            }
            active.push(offset);
            let node = e.animation.node(offset)?;
            if !matches!(node.transform, 0 | 2) || node.callback != 0 {
                return Err(Error(
                    "retained position cache requires ordinary nodes without callbacks",
                ));
            }
            let parent = if node.parent != 0 {
                Some(visit(e, node.parent, tick, epoch, cache, active)?)
            } else {
                None
            };
            let mut matrix = pose_matrix(&e.affine(offset, tick)?);
            if let Some(parent) = parent {
                matrix = std::array::from_fn(|i| {
                    (0..4)
                        .map(|k| d(parent[i / 4 * 4 + k]) * d(matrix[k * 4 + i % 4]))
                        .sum::<f64>() as f32
                });
            }
            cache.insert(offset, (epoch, matrix));
            active.pop();
            Ok(matrix)
        }
        visit(self, offset, tick, epoch, cache, &mut Vec::new())
    }
    /// Apply measured procedural axis-angle rotations after local ANM sampling.
    /// Ordinary affine samples and the unmodified matrix path stay independent.
    pub fn matrix_with_post_rotations(
        &self,
        offset: usize,
        tick: i32,
        rotations: &std::collections::BTreeMap<usize, [f32; 4]>,
    ) -> Result<[f32; 16], Error> {
        self.post_rotation_matrix_cached(
            offset,
            tick,
            rotations,
            &mut std::collections::BTreeMap::new(),
        )
    }
    pub fn post_rotation_matrices(
        &self,
        tick: i32,
        rotations: &std::collections::BTreeMap<usize, [f32; 4]>,
    ) -> Result<std::collections::BTreeMap<usize, [f32; 16]>, Error> {
        let mut cache = std::collections::BTreeMap::new();
        for node in &self.animation.nodes {
            self.post_rotation_matrix_cached(node.offset, tick, rotations, &mut cache)?;
        }
        Ok(cache)
    }
    fn post_rotation_matrix_cached(
        &self,
        offset: usize,
        tick: i32,
        rotations: &std::collections::BTreeMap<usize, [f32; 4]>,
        cache: &mut std::collections::BTreeMap<usize, [f32; 16]>,
    ) -> Result<[f32; 16], Error> {
        if let Some(matrix) = cache.get(&offset) {
            return Ok(*matrix);
        }
        let node = self.animation.node(offset)?;
        if !matches!(node.transform, 0 | 2) {
            return Err(Error("procedural matrix path requires ordinary ANM nodes"));
        }
        let mut pose = self.affine(offset, tick)?;
        if let Some([x, y, z, angle]) = rotations.get(&offset).copied() {
            if ![x, y, z, angle].into_iter().all(f32::is_finite) {
                return Err(Error("non-finite procedural axis-angle"));
            }
            let half = angle * 0.5;
            let sine = simd_trig(half, false);
            let rotation = [
                (d(sine) * d(x)) as f32,
                (d(sine) * d(y)) as f32,
                (d(sine) * d(z)) as f32,
                simd_trig(half, true),
            ];
            pose.rotation = quat_mul(pose.rotation, rotation);
            pose.flags |= 1;
        }
        let mut matrix = pose_matrix(&pose);
        if node.parent != 0 {
            let parent = self.post_rotation_matrix_cached(node.parent, tick, rotations, cache)?;
            matrix = std::array::from_fn(|i| {
                (0..4)
                    .map(|k| d(parent[i / 4 * 4 + k]) * d(matrix[k * 4 + i % 4]))
                    .sum::<f64>() as f32
            });
        }
        cache.insert(offset, matrix);
        Ok(matrix)
    }
    fn matrix_inner(
        &self,
        offset: usize,
        tick: i32,
        stack: &mut Vec<usize>,
    ) -> Result<[f32; 16], Error> {
        let n = self.animation.node(offset)?;
        let mut m = pose_matrix(&self.affine_inner(offset, tick, stack)?);
        if n.parent != 0 {
            let parent = self.matrix_inner(n.parent, tick, stack)?;
            m = std::array::from_fn(|i| {
                (0..4)
                    .map(|k| d(parent[i / 4 * 4 + k]) * d(m[k * 4 + i % 4]))
                    .sum::<f64>() as f32
            });
        }
        Ok(m)
    }
    /// Evaluate target callbacks in an object-owned cache. The source ANM is
    /// immutable; two objects using it must not share their resulting matrices.
    pub fn aimed_matrices(
        &self,
        tick: i32,
        world: crate::procedural_aim::Matrix,
        target: Option<([f32; 4], u32)>,
        bindings: &std::collections::BTreeMap<usize, crate::procedural_aim::Axes>,
    ) -> Result<std::collections::BTreeMap<usize, crate::procedural_aim::Matrix>, Error> {
        fn visit(
            e: &Evaluator<'_>,
            offset: usize,
            frame: (i32, crate::procedural_aim::Matrix, Option<([f32; 4], u32)>),
            bindings: &std::collections::BTreeMap<usize, crate::procedural_aim::Axes>,
            cache: &mut std::collections::BTreeMap<usize, crate::procedural_aim::Matrix>,
            active: &mut Vec<usize>,
        ) -> Result<crate::procedural_aim::Matrix, Error> {
            let (tick, world, target) = frame;
            if let Some(m) = cache.get(&offset) {
                return Ok(*m);
            }
            if active.len() >= 256 || active.contains(&offset) {
                return Err(Error("aimed animation parent cycle/budget"));
            }
            active.push(offset);
            let node = e.animation.node(offset)?;
            if node.transform == 3 {
                let mut pending = vec![offset];
                let mut seen = std::collections::BTreeSet::new();
                while let Some(at) = pending.pop() {
                    if !seen.insert(at) {
                        continue;
                    }
                    if seen.len() > 256 {
                        return Err(Error("aimed LOOKAT dependency budget"));
                    }
                    if bindings.contains_key(&at) {
                        return Err(Error("LOOKAT affected by procedural aiming is unsupported"));
                    }
                    let dependency = e.animation.node(at)?;
                    if dependency.parent != 0 {
                        pending.push(dependency.parent);
                    }
                    if dependency.transform == 3 && dependency.target != 0 {
                        pending.push(dependency.target);
                    }
                }
            }
            let parent = if node.parent != 0 {
                Some(visit(e, node.parent, frame, bindings, cache, active)?)
            } else {
                None
            };
            let mut pose = e.affine(offset, tick)?;
            if let Some(axes) = bindings.get(&offset) {
                pose = crate::procedural_aim::apply(
                    &pose,
                    world,
                    parent,
                    target,
                    axes.forward,
                    axes.up,
                )
                .map_err(Error)?;
            }
            let matrix = crate::procedural_aim::local_matrix(&pose, parent).map_err(Error)?;
            cache.insert(offset, matrix);
            active.pop();
            Ok(matrix)
        }
        if bindings
            .keys()
            .any(|offset| self.animation.node(*offset).is_err())
        {
            return Err(Error("aiming binding outside animation"));
        }
        let mut cache = std::collections::BTreeMap::new();
        for node in &self.animation.nodes {
            visit(
                self,
                node.offset,
                (tick, world, target),
                bindings,
                &mut cache,
                &mut Vec::new(),
            )?;
        }
        Ok(cache)
    }
    pub fn camera(&self, tick: i32) -> Result<Camera, Error> {
        let node = self.animation.default_node("Camera01")?;
        self.camera_node(
            node,
            tick,
            false,
            crate::cinematic_viewport::Context::INSPECTION,
        )
    }
    pub fn camera_named(&self, tree: Option<&str>, name: &str, tick: i32) -> Result<Camera, Error> {
        self.camera_named_in_context(
            tree,
            name,
            tick,
            crate::cinematic_viewport::Context::INSPECTION,
        )
    }
    pub fn camera_named_in_context(
        &self,
        tree: Option<&str>,
        name: &str,
        tick: i32,
        context: crate::cinematic_viewport::Context,
    ) -> Result<Camera, Error> {
        let node = self.animation.named_node(tree, name)?;
        self.camera_node(node, tick, false, context)
    }
    pub fn camera_in_context(
        &self,
        tick: i32,
        context: crate::cinematic_viewport::Context,
        preview: bool,
    ) -> Result<Camera, Error> {
        self.camera_node(
            self.animation.default_node("Camera01")?,
            tick,
            preview,
            context,
        )
    }
    pub fn camera_named_preview_in_context(
        &self,
        tree: Option<&str>,
        name: &str,
        tick: i32,
        context: crate::cinematic_viewport::Context,
        preview: bool,
    ) -> Result<Camera, Error> {
        self.camera_node(
            self.animation.named_node(tree, name)?,
            tick,
            preview,
            context,
        )
    }
    /// Preview shares the measured FOV evaluator with strict camera evaluation.
    /// Unsupported scalar profiles remain errors, never serialized-value fallbacks.
    pub fn camera_preview(&self, tick: i32) -> Result<Camera, Error> {
        self.camera_node(
            self.animation.default_node("Camera01")?,
            tick,
            true,
            crate::cinematic_viewport::Context::INSPECTION,
        )
    }
    /// Serialized default or measured scalar track, selected by the camera node.
    pub fn camera_fov(&self, offset: usize, tick: i32) -> Result<f32, Error> {
        let (base, track) = self
            .animation
            .node(offset)?
            .camera
            .ok_or(Error("named camera is not a camera node"))?;
        let fov = if track == 0 {
            base
        } else {
            crate::scalar_track::evaluate(self.animation.bytes(), track, tick)?
        };
        if !(0. ..std::f32::consts::PI).contains(&fov) || fov == 0. {
            return Err(Error("invalid evaluated camera FOV"));
        }
        Ok(fov)
    }
    fn camera_node(
        &self,
        node: &Node,
        tick: i32,
        _preview: bool,
        context: crate::cinematic_viewport::Context,
    ) -> Result<Camera, Error> {
        let fov = self.camera_fov(node.offset, tick)?;
        let m = self.matrix(node.offset, tick)?;
        // Camera nodes use the same flag-selected affine inverse as other
        // hierarchy matrices. A rigid transpose loses inherited scale/shear.
        let inverse = inverse_affine(m, self.matrix_flags(node.offset, tick)?)?;
        let mut view = std::array::from_fn(|i| inverse[(i % 4) * 4 + i / 4]);
        for row in 0..4 {
            view[row * 4 + 2] = -view[row * 4 + 2];
        }
        let origin = std::array::from_fn(|r| {
            (((d(view[r * 4 + 3]) - d(view[r * 4 + 2]) * d(view[14]))
                - d(view[13]) * d(view[r * 4 + 1]))
                - d(view[12]) * d(view[r * 4])) as f32
        });
        let far = ((12500.
            - (0..3)
                .map(|i| d(origin[i]) * d(view[i * 4 + 2]))
                .sum::<f64>()) as f32)
            .min(25000.);
        let lens = context.lens(fov, far)?;
        Ok(Camera {
            view,
            projection: lens.projection,
            origin,
            viewport: lens.viewport,
            lod_scale: lens.lod_scale,
        })
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn affine_steps_preserve_prestart_selection_and_refuse_unframed_flags() {
        let mut raw = vec![0u8; 116];
        raw[0] = 10;
        for (at, value) in [
            (8, 2u32),
            (12, 20),
            (16, 68),
            (20, 10),
            (68, 20),
            (64, 17),
            (112, 17),
        ] {
            raw[at..at + 4].copy_from_slice(&value.to_le_bytes());
        }
        raw[24..28].copy_from_slice(&7f32.to_le_bytes());
        raw[72..76].copy_from_slice(&12f32.to_le_bytes());
        assert_eq!(affine_step_track(&raw, 0, 9).unwrap().position[0], 12.);
        assert_eq!(affine_step_track(&raw, 0, 19).unwrap().position[0], 7.);
        assert!(affine_step_track(&raw[..115], 0, 20).is_err());
        raw[112..116].copy_from_slice(&32u32.to_le_bytes());
        assert!(affine_step_track(&raw, 0, 20).is_err());
    }
    #[test]
    fn easing_preserves_endpoints_and_refuses_unmeasured_negative_parameters() {
        assert_eq!(tcb_ease(0., 2., 2.).unwrap(), 0.);
        assert_eq!(tcb_ease(1., 2., 2.).unwrap(), 1.);
        assert_eq!(tcb_ease(0.25, 0., 1.).unwrap(), 0.4375);
        assert!(tcb_ease(0.5, -1., 0.).is_err());
        assert!(tcb_ease(f32::NAN, 0., 0.).is_err());
    }
    #[test]
    fn linear_vector_track_clamps_and_rejects_bad_keys() {
        let mut raw = vec![0; 52];
        raw[..4].copy_from_slice(&[6, 1, 1, 0]);
        raw[8..12].copy_from_slice(&2u32.to_le_bytes());
        raw[12..16].copy_from_slice(&20u32.to_le_bytes());
        raw[16..20].copy_from_slice(&36u32.to_le_bytes());
        for (at, tick, value) in [(20, 0i32, [2f32, 4., 6.]), (36, 10i32, [4f32, 8., 12.])] {
            raw[at..at + 4].copy_from_slice(&tick.to_le_bytes());
            for (i, v) in value.into_iter().enumerate() {
                raw[at + 4 + i * 4..at + 8 + i * 4].copy_from_slice(&v.to_le_bytes());
            }
        }
        assert_eq!(vector_track(&raw, 0, -1).unwrap(), [2., 4., 6.]);
        assert_eq!(vector_track(&raw, 0, 5).unwrap(), [3., 6., 9.]);
        assert_eq!(vector_track(&raw, 0, 11).unwrap(), [4., 8., 12.]);
        raw[36..40].copy_from_slice(&0i32.to_le_bytes());
        assert!(vector_track(&raw, 0, 5).is_err());
        raw[36..40].copy_from_slice(&10i32.to_le_bytes());
        raw[24..28].copy_from_slice(&f32::NAN.to_le_bytes());
        assert!(vector_track(&raw, 0, 5).is_err());
    }
    use super::*;
    #[test]
    fn bezier_derivatives_and_endpoint_clamping() {
        let mut bytes = vec![5, 1, 1, 0, 0, 0, 0, 0];
        for value in [2u32, 20, 64] {
            bytes.extend(value.to_le_bytes());
        }
        for (tick, incoming, outgoing, value) in [
            (0i32, [0.; 3], [1.5, 3., 4.5], [0.; 3]),
            (2, [-1.5, -3., -4.5], [0.; 3], [3., 6., 9.]),
        ] {
            bytes.extend(tick.to_le_bytes());
            bytes.extend(0u32.to_le_bytes());
            for x in incoming.into_iter().chain(outgoing).chain(value) {
                bytes.extend(f32::to_le_bytes(x));
            }
        }
        assert_eq!(vector(&bytes, 0, -1).unwrap(), [0.; 3]);
        assert_eq!(vector(&bytes, 0, 1).unwrap(), [1.5, 3., 4.5]);
        assert_eq!(vector(&bytes, 0, 3).unwrap(), [3., 6., 9.]);
        bytes[24] = 1;
        assert_eq!(vector(&bytes, 0, 1).unwrap(), [1.5, 3., 4.5]);
        bytes[24..28].copy_from_slice(&0x800u32.to_le_bytes());
        assert_eq!(vector(&bytes, 0, 1).unwrap(), [0.; 3]);
        assert_eq!(vector(&bytes, 0, 2).unwrap(), [3., 6., 9.]);
        bytes[24..28].copy_from_slice(&0u32.to_le_bytes());
        bytes[68..72].copy_from_slice(&0x100u32.to_le_bytes());
        assert_eq!(vector(&bytes, 0, 1).unwrap(), [0.; 3]);
    }
    #[test]
    fn measured_simd_identity_and_invalid_vector_inputs_are_explicit() {
        assert_eq!(simd_trig(0., true).to_bits(), 0.9998432f32.to_bits());
        assert_eq!(simd_trig(0., false), 0.);
        assert!(normalize([0.; 3]).is_err());
        assert!(vector(&[4, 1, 1, 0], 0, 0).is_err());
        let mut empty = vec![0; 12];
        empty[..4].copy_from_slice(&[4, 1, 1, 0]);
        assert!(vector(&empty, 0, 0).is_err());
    }
    #[test]
    fn affine_inverse_rejects_singular_and_non_affine_inputs() {
        let mut m = [0.; 16];
        m[15] = 1.;
        assert!(inverse_affine(m, 2).is_err());
        m[0] = 1.;
        m[5] = 1.;
        m[10] = 1.;
        m[12] = 1.;
        assert!(inverse_affine(m, 17).is_err());
        m[12] = 0.;
        m[0] = f32::NAN;
        assert!(inverse_affine(m, 0).is_err());
        m[0] = 1.;
        assert!(inverse_affine(m, 32).is_err());
    }
    #[test]
    fn affine_inverse_nonuniform_scale_and_mirror_restore_points() {
        let m = [
            2., 0., 0., 8., 0., -4., 0., 12., 0., 0., 0.5, -6., 0., 0., 0., 1.,
        ];
        let inverse = inverse_affine(m, 26).unwrap();
        assert_eq!([inverse[0], inverse[5], inverse[10]], [0.5, -0.25, 2.]);
        assert_eq!([inverse[3], inverse[7], inverse[11]], [-4., 3., 12.]);
        for source in [[0., 0., 0.], [1., -2., 3.], [-64., 32., 8.]] {
            let world = std::array::from_fn::<_, 3, _>(|r| m[r * 4 + r] * source[r] + m[r * 4 + 3]);
            let restored = std::array::from_fn::<_, 3, _>(|r| {
                inverse[r * 4 + r] * world[r] + inverse[r * 4 + 3]
            });
            assert_eq!(restored, source);
        }
    }
    fn repeat_fixture(first: i32, last: i32) -> Vec<u8> {
        let mut bytes = vec![0; 164];
        bytes[..4].copy_from_slice(&[7, 3, 3, 1]);
        bytes[8..12].copy_from_slice(&2u32.to_le_bytes());
        for (index, tick, q) in [
            (0, first, [0., 0., 0., 1.]),
            (1, last, [0., 0.25, 0., 0.96824586]),
        ] {
            let at = 20 + 72 * index;
            bytes[12 + index * 4..16 + index * 4].copy_from_slice(&(at as u32).to_le_bytes());
            bytes[at..at + 4].copy_from_slice(&tick.to_le_bytes());
            for base in [24, 40, 56] {
                for (j, value) in q.into_iter().enumerate() {
                    bytes[at + base + j * 4..at + base + j * 4 + 4]
                        .copy_from_slice(&f32::to_le_bytes(value));
                }
            }
        }
        bytes
    }
    #[test]
    fn repeating_quaternion_wraps_interior_and_preserves_endpoint_asymmetry() {
        let bytes = repeat_fixture(-20, 80);
        let reference = quaternion_track(&bytes, 0, 17).unwrap().map(f32::to_bits);
        for t in [-283, -183, -83, 117, 217, 317] {
            assert_eq!(
                quaternion_track(&bytes, 0, t).unwrap().map(f32::to_bits),
                reference
            );
        }
        assert_eq!(
            quaternion_track(&bytes, 0, -120).unwrap().map(f32::to_bits),
            quaternion_track(&bytes, 0, 80).unwrap().map(f32::to_bits)
        );
        assert_eq!(
            quaternion_track(&bytes, 0, 180).unwrap().map(f32::to_bits),
            quaternion_track(&bytes, 0, -20).unwrap().map(f32::to_bits)
        );
    }
    #[test]
    fn repeating_quaternion_rejects_unmeasured_profiles_and_signed_overflow() {
        let mut bytes = repeat_fixture(0, 100);
        assert!(quaternion_track(&bytes, 0, i32::MIN).is_err());
        bytes[3] = 0;
        assert!(quaternion_track(&bytes, 0, 50).is_err());
        let bytes = repeat_fixture(i32::MIN, i32::MAX);
        assert!(quaternion_track(&bytes, 0, 0).is_err());
    }
}
