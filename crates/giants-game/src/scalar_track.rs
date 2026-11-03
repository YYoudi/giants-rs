//! Measured scalar ANM tracks: TCB, Bezier and linear, clamp modes 1/1.
//! Constant tracks override node defaults. TCB time easing uses the shared kernel.
use giants_formats::{
    Error,
    anm::{floats, read_bytes, read_word},
};
struct Key {
    tick: i32,
    values: Vec<f32>,
    flags: u32,
}
pub fn evaluate(data: &[u8], at: usize, tick: i32) -> Result<f32, Error> {
    let header = read_bytes(data, at, 4)?;
    if !(1..=3).contains(&header[0]) || header[1..] != [1, 1, 0] {
        return Err(Error("unsupported scalar track profile"));
    }
    let kind = header[0];
    let count = read_word(data, at + 8)? as usize;
    if count == 0 {
        return Ok(floats::<1>(data, at + 12)?[0]);
    }
    if count > 4096 {
        return Err(Error("scalar key count outside budget"));
    }
    let mut keys = Vec::with_capacity(count);
    for i in 0..count {
        let p = read_word(data, at + 12 + i * 4)? as usize;
        if !p.is_multiple_of(4) {
            return Err(Error("unaligned scalar key"));
        }
        let values = match kind {
            1 => floats::<8>(data, p + 4)?.to_vec(),
            2 => floats::<3>(data, p + 8)?.to_vec(),
            _ => floats::<1>(data, p + 4)?.to_vec(),
        };
        keys.push(Key {
            tick: read_word(data, p)? as i32,
            values,
            flags: if kind == 2 {
                read_word(data, p + 4)?
            } else {
                0
            },
        });
    }
    if keys.windows(2).any(|w| {
        w[0].tick >= w[1].tick || i64::from(w[1].tick) - i64::from(w[0].tick) > i64::from(i32::MAX)
    }) {
        return Err(Error("unordered or oversized scalar interval"));
    }
    let (a, b, t) = if tick < keys[0].tick {
        (&keys[count - 1], &keys[0], 1.)
    } else if tick >= keys[count - 1].tick {
        (&keys[count - 1], &keys[0], 0.)
    } else {
        let right = keys.partition_point(|k| k.tick <= tick);
        let (a, b) = (&keys[right - 1], &keys[right]);
        (
            a,
            b,
            ((i64::from(tick) - i64::from(a.tick)) as f64
                / (i64::from(b.tick) - i64::from(a.tick)) as f64) as f32,
        )
    };
    let d = f64::from;
    let u = (1. - d(t)) as f32;
    let result = match kind {
        1 => {
            let t = d(crate::animation::tcb_ease(t, a.values[4], b.values[3])?);
            let t2 = t * t;
            let t3 = t2 * t;
            let x = 2. * t3 - 3. * t2;
            let w = [x + 1., -x, (t - 2. * t2) + t3, t3 - t2].map(|v| d(v as f32));
            // The scalar callback accumulates tangents before endpoint values.
            (((w[3] * d(b.values[6]) + w[2] * d(a.values[5])) + w[1] * d(b.values[7]))
                + w[0] * d(a.values[7])) as f32
        }
        2 => {
            if a.flags & 0x1c00 == 0x800 || b.flags & 0x380 == 0x100 {
                if t == 1. { b.values[2] } else { a.values[2] }
            } else {
                let t2 = (d(t) * d(t)) as f32;
                let factor = ((i64::from(b.tick) - i64::from(a.tick)) as f64 * d(1f32 / 3.)) as f32;
                let p0 = d(a.values[2]);
                let p1 = d(b.values[2]);
                let c0 = p0 + d(factor) * d(a.values[1]);
                let c1 = p1 + d(factor) * d(b.values[0]);
                let lead = (c0 * (3. * d(t)) + p0 * d(u)) * d(u) + c1 * (3. * d(t2));
                (lead * d(u) + p1 * d(t) * d(t2)) as f32
            }
        }
        _ => (d(b.values[0]) * d(t) + d(a.values[0]) * d(u)) as f32,
    };
    if !result.is_finite() {
        return Err(Error("scalar interpolation overflow"));
    }
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn constant_tracks_and_linear_clamps_keep_measured_values() {
        for kind in 1..=3 {
            let mut raw = vec![0; 16];
            raw[..4].copy_from_slice(&[kind, 1, 1, 0]);
            raw[12..16].copy_from_slice(&0.25f32.to_le_bytes());
            assert_eq!(evaluate(&raw, 0, i32::MIN).unwrap(), 0.25);
        }
        let mut raw = vec![0; 36];
        raw[..4].copy_from_slice(&[3, 1, 1, 0]);
        raw[8..12].copy_from_slice(&2u32.to_le_bytes());
        raw[12..16].copy_from_slice(&20u32.to_le_bytes());
        raw[16..20].copy_from_slice(&28u32.to_le_bytes());
        for (at, time, value) in [(20, 0i32, 2f32), (28, 10, 4.)] {
            raw[at..at + 4].copy_from_slice(&time.to_le_bytes());
            raw[at + 4..at + 8].copy_from_slice(&value.to_le_bytes());
        }
        assert_eq!(evaluate(&raw, 0, -1).unwrap(), 2.);
        assert_eq!(evaluate(&raw, 0, 5).unwrap(), 3.);
        assert_eq!(evaluate(&raw, 0, 11).unwrap(), 4.);
        raw[28..32].copy_from_slice(&0i32.to_le_bytes());
        assert!(evaluate(&raw, 0, 5).is_err());
        raw[28..32].copy_from_slice(&10i32.to_le_bytes());
        raw[32..36].copy_from_slice(&f32::NAN.to_le_bytes());
        assert!(evaluate(&raw, 0, 5).is_err());
    }
    #[test]
    fn unknown_modes_negative_easing_and_oversized_counts_do_not_fall_back() {
        let mut raw = vec![0; 52];
        raw[..4].copy_from_slice(&[1, 1, 1, 0]);
        raw[8..12].copy_from_slice(&1u32.to_le_bytes());
        raw[12..16].copy_from_slice(&16u32.to_le_bytes());
        raw[36..40].copy_from_slice(&(-1f32).to_le_bytes());
        assert!(evaluate(&raw, 0, 0).is_err());
        raw[36..40].fill(0);
        raw[1] = 2;
        assert!(evaluate(&raw, 0, 0).is_err());
        raw[1] = 1;
        raw[8..12].copy_from_slice(&4097u32.to_le_bytes());
        assert!(evaluate(&raw, 0, 0).is_err());
    }
}
