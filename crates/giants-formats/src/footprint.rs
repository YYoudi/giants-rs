//! Collision footprint profiles 1–5, derived from the identified PC query.
//! No movement solver, object flag policy or world-grid interpretation is implied.
use crate::{Error, slice};

pub struct Bitmap<'a> {
    pub bounds: [i16; 4],
    kind: u8,
    width: usize,
    stride: usize,
    data: &'a [u8],
    spans: Vec<SpanSet>,
}
struct SpanSet {
    bounds: [i16; 4],
    rows: Vec<(i32, u8)>,
    bounded: bool,
}
impl SpanSet {
    fn contains(&self, x: i32, y: i32) -> bool {
        if self.bounded
            && (x < i32::from(self.bounds[0])
                || x > i32::from(self.bounds[2])
                || y < i32::from(self.bounds[1])
                || y > i32::from(self.bounds[3]))
        {
            return false;
        }
        let row = i64::from(y) - i64::from(self.bounds[1]);
        let Some(&(start, width)) = usize::try_from(row).ok().and_then(|r| self.rows.get(r)) else {
            return false;
        };
        x >= start && x < start + i32::from(width)
    }
}
fn header(data: &[u8], at: usize) -> Result<[i16; 4], Error> {
    let b = slice(data, at, 9)?;
    Ok(std::array::from_fn(|i| {
        i16::from_le_bytes([b[1 + i * 2], b[2 + i * 2]])
    }))
}
/// The sentinel belongs to the next header in paired profile 2.
fn spans(data: &[u8], at: usize, bounded: bool) -> Result<(SpanSet, usize), Error> {
    let bounds = header(data, at)?;
    let mut cursor = at + 9;
    let mut start = i32::from(bounds[0]);
    let mut rows = Vec::new();
    loop {
        let delta = *data
            .get(cursor)
            .ok_or(Error("unterminated footprint spans"))? as i8;
        if delta == i8::MIN {
            return Ok((
                SpanSet {
                    bounds,
                    rows,
                    bounded,
                },
                cursor,
            ));
        }
        let width = *data
            .get(cursor + 1)
            .ok_or(Error("truncated footprint span"))?;
        if rows.len() >= 65536 {
            return Err(Error("footprint span budget"));
        }
        start += i32::from(delta);
        rows.push((start, width));
        cursor += 2;
    }
}
impl<'a> Bitmap<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Self, Error> {
        slice(data, 0, 9)?;
        let kind = data[0];
        if ![1, 2, 3, 4, 5].contains(&kind) {
            return Err(Error("unmeasured footprint profile"));
        }
        let bounds = header(data, 0)?;
        if [1, 2, 5].contains(&kind) {
            let (first, sentinel) = spans(data, 0, kind != 2)?;
            let (sets, end) = if kind == 2 {
                let (second, last) = spans(data, sentinel, true)?;
                (vec![first, second], last + 1)
            } else {
                (vec![first], sentinel + 1)
            };
            if end != data.len() {
                return Err(Error("footprint trailing data"));
            }
            return Ok(Self {
                bounds,
                kind,
                width: 0,
                stride: 0,
                data: &[],
                spans: sets,
            });
        }
        let width = i32::from(bounds[2]) - i32::from(bounds[0]) + 1;
        let height = i32::from(bounds[3]) - i32::from(bounds[1]) + 1;
        if width <= 0 || height <= 0 || i64::from(width) * i64::from(height) > 16 * 1024 * 1024 {
            return Err(Error("invalid footprint bounds/budget"));
        }
        let width = width as usize;
        // PC uses width >> 3 for packed row stride. Restrict to complete rows;
        // do not silently substitute ceil(width/8) for unmeasured edge layouts.
        if kind == 4 && !width.is_multiple_of(8) {
            return Err(Error("unaligned packed footprint width"));
        }
        let stride = if kind == 3 { width } else { width / 8 };
        let payload = slice(data, 9, stride * height as usize)?;
        if data.len() != 9 + payload.len() {
            return Err(Error("footprint trailing data"));
        }
        Ok(Self {
            bounds,
            kind,
            width,
            stride,
            data: payload,
            spans: Vec::new(),
        })
    }
    /// Platform membership uses bit 1 of byte profile 3, unlike solid occupancy.
    /// The original platform query does not accept packed profile 4.
    pub fn platform_contains(&self, x: i32, y: i32) -> bool {
        if self.kind != 3 {
            return false;
        }
        let x = i64::from(x) - i64::from(self.bounds[0]);
        let y = i64::from(y) - i64::from(self.bounds[1]);
        if x < 0
            || y < 0
            || x as usize >= self.width
            || y > i64::from(self.bounds[3]) - i64::from(self.bounds[1])
        {
            return false;
        }
        self.data[y as usize * self.stride + x as usize] & 2 != 0
    }
    pub fn contains(&self, x: i32, y: i32) -> bool {
        if !self.spans.is_empty() {
            return self.spans.iter().any(|set| set.contains(x, y));
        }
        let x = i64::from(x) - i64::from(self.bounds[0]);
        let y = i64::from(y) - i64::from(self.bounds[1]);
        if x < 0
            || x as usize >= self.width
            || y < 0
            || y > i64::from(self.bounds[3]) - i64::from(self.bounds[1])
        {
            return false;
        }
        if self.kind == 3 {
            self.data[y as usize * self.stride + x as usize] & 3 == 1
        } else {
            self.data[y as usize * self.stride + x as usize / 8] & (1 << (x % 8)) != 0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn blob(kind: u8, bounds: [i16; 4], payload: &[u8]) -> Vec<u8> {
        let mut b = vec![kind];
        b.extend(bounds.into_iter().flat_map(i16::to_le_bytes));
        b.extend(payload);
        b
    }
    #[test]
    fn platform_bit_is_distinct_from_solid_and_refuses_packed() {
        let data = blob(3, [0, 0, 7, 0], &[0, 1, 2, 3, 4, 5, 6, 7]);
        let map = Bitmap::parse(&data).unwrap();
        for x in 0..8 {
            assert_eq!(map.platform_contains(x, 0), x & 2 != 0);
        }
        assert!(!map.platform_contains(-1, 0));
        assert!(!map.platform_contains(8, 0));
        let packed = blob(4, [0, 0, 7, 0], &[255]);
        assert!(!Bitmap::parse(&packed).unwrap().platform_contains(1, 0));
    }
    #[test]
    fn byte_cells_exclusion_and_negative_origins() {
        let b = blob(3, [-1, -1, 1, 0], &[0, 1, 2, 3, 5, 7]);
        let f = Bitmap::parse(&b).unwrap();
        assert!(f.contains(0, -1));
        assert!(f.contains(0, 0));
        for p in [(-1, -1), (1, -1), (-1, 0), (1, 0), (-2, 0), (0, 1)] {
            assert!(!f.contains(p.0, p.1));
        }
        for end in 0..b.len() {
            assert!(Bitmap::parse(&b[..end]).is_err());
        }
    }
    #[test]
    fn packed_rows_and_rejected_profiles() {
        let b = blob(4, [0, 0, 15, 1], &[1, 128, 128, 1]);
        let f = Bitmap::parse(&b).unwrap();
        for p in [(0, 0), (15, 0), (7, 1), (8, 1)] {
            assert!(f.contains(p.0, p.1));
        }
        assert!(!f.contains(8, 0));
        assert!(!f.contains(0, 1));
        assert!(Bitmap::parse(&blob(4, [0, 0, 6, 0], &[255])).is_err());
        assert!(Bitmap::parse(&blob(1, [0, 0, 7, 0], &[255])).is_err());
    }
    #[test]
    fn paired_spans_share_the_sentinel_header_and_first_bounds_do_not_clip() {
        let mut b = blob(2, [-2, -1, -10, -10], &[1, 2]);
        b.extend(blob(128, [4, 2, 7, 2], &[1, 2, 128]));
        let map = Bitmap::parse(&b).unwrap();
        for p in [(-1, -1), (0, -1), (5, 2), (6, 2)] {
            assert!(map.contains(p.0, p.1));
            assert!(!map.platform_contains(p.0, p.1));
        }
        for p in [(-2, -1), (1, -1), (4, 2), (7, 2)] {
            assert!(!map.contains(p.0, p.1));
        }
        for end in 0..b.len() {
            assert!(Bitmap::parse(&b[..end]).is_err());
        }
    }
}
