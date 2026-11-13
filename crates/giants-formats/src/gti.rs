//! GTI v3, no bounding boxes, RGB cell profile. Uninterpreted native-ignored tail retained.
use crate::{Error, slice, u32_at};
pub const DEFAULT_SLOPE_THRESHOLDS: [f32; 2] = [0.64278764, 0.49999997];
/// Float32 cross-product stores precede normalization in native 0x4511D0.
pub fn face_class(points: [[f32; 3]; 3]) -> Result<u8, Error> {
    face_class_with_thresholds(points, DEFAULT_SLOPE_THRESHOLDS)
}
pub fn face_class_with_thresholds(
    points: [[f32; 3]; 3],
    thresholds: [f32; 2],
) -> Result<u8, Error> {
    if !thresholds.iter().all(|v| v.is_finite()) {
        return Err(Error("non-finite slope thresholds"));
    }
    let [a, b, c] = points.map(|p| p.map(f64::from));
    let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let cross = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ]
    .map(|n| n as f32 as f64);
    let length = cross.iter().map(|n| n * n).sum::<f64>().sqrt();
    if !length.is_finite() || length == 0. {
        return Err(Error("degenerate terrain face"));
    }
    let [x, y, z] = cross.map(|n| (n / length) as f32);
    if z >= thresholds[0] {
        return Ok(0);
    }
    let direction = if y.abs() > x.abs() {
        if y < 0. { 1 } else { 2 }
    } else if x < 0. {
        3
    } else {
        4
    };
    Ok(direction + if z >= thresholds[1] { 0 } else { 4 })
}
#[derive(Clone)]
pub struct Terrain {
    /// Original header flags. Bit0 consumers are not inferred from RGB decoding.
    pub source_flags: u8,
    pub width: usize,
    pub height: usize,
    pub origin: [f32; 2],
    pub spacing: f32,
    pub height_bounds: [f32; 2],
    pub fallback_texture: String,
    pub heights: Vec<f32>,
    pub colors: Vec<u32>,
    pub tags: Vec<u8>,
    pub consumed_bytes: usize,
    pub opaque_tail: Vec<u8>,
}
fn scalar(data: &[u8], at: usize) -> Result<f32, Error> {
    let value = f32::from_bits(u32_at(data, at)?);
    if !value.is_finite() {
        return Err(Error("non-finite GTI scalar"));
    }
    Ok(value)
}
pub fn parse(data: &[u8]) -> Result<Terrain, Error> {
    slice(data, 0, 96)?;
    if u32_at(data, 0)? != 0x94965003 || u32_at(data, 4)? != 0 || !matches!(data[52], 2 | 3) {
        return Err(Error("unsupported GTI profile"));
    }
    let ox = scalar(data, 8)?;
    let oy = scalar(data, 12)?;
    let height_bounds = [scalar(data, 16)?, scalar(data, 20)?];
    let nx = u32_at(data, 24)? as usize;
    let ny = u32_at(data, 28)? as usize;
    let width = nx.checked_add(2).ok_or(Error("GTI dimension overflow"))?;
    let height = ny.checked_add(2).ok_or(Error("GTI dimension overflow"))?;
    let count = width
        .checked_mul(height)
        .ok_or(Error("GTI dimension overflow"))?;
    let spacing = scalar(data, 32)?;
    if nx == 0 || ny == 0 || count > 262144 || spacing <= 0. || height_bounds[0] > height_bounds[1]
    {
        return Err(Error("GTI dimensions or bounds outside profile"));
    }
    let raw = &data[64..96];
    let end = raw
        .iter()
        .position(|v| *v == 0)
        .ok_or(Error("unterminated GTI texture name"))?;
    if !raw[..end].is_ascii() {
        return Err(Error("non-ASCII GTI texture name"));
    }
    let fallback_texture = String::from_utf8(raw[..end].to_vec()).unwrap();
    let mut heights = vec![-4096.; count];
    let mut colors = vec![0; count];
    let mut tags = vec![0; count];
    let (mut at, mut cell) = (96, 0);
    let total = nx * ny;
    while cell < total {
        let run = *slice(data, at, 1)?.first().unwrap() as i8;
        at += 1;
        if run < 0 {
            cell += -(run as i16) as usize;
            if cell > total {
                return Err(Error("GTI skip exceeds grid"));
            }
        } else {
            let literal_count = run as usize + 1;
            if cell + literal_count > total {
                return Err(Error("GTI literal run exceeds grid"));
            }
            let run_data = slice(data, at, literal_count * 8)?;
            for record in run_data.as_chunks::<8>().0 {
                let value = f32::from_le_bytes(record[..4].try_into().unwrap());
                if !value.is_finite() || record[4] > 7 {
                    return Err(Error("unsupported GTI height or topology tag"));
                }
                let index = (cell / nx + 1) * width + cell % nx + 1;
                heights[index] = value;
                tags[index] = record[4];
                colors[index] = 0xff000000
                    | ((record[5] as u32) << 16)
                    | ((record[6] as u32) << 8)
                    | record[7] as u32;
                cell += 1;
            }
            at += literal_count * 8;
        }
    }
    if data.len() - at > 1024 {
        return Err(Error("GTI opaque tail exceeds budget"));
    }
    let origin = [ox - spacing, oy - spacing];
    if !origin.iter().all(|v| v.is_finite()) {
        return Err(Error("non-finite GTI origin"));
    }
    Ok(Terrain {
        source_flags: data[52],
        width,
        height,
        origin,
        spacing,
        height_bounds,
        fallback_texture,
        heights,
        colors,
        tags,
        consumed_bytes: at,
        opaque_tail: data[at..].to_vec(),
    })
}
impl Terrain {
    pub fn position(&self, index: usize) -> Result<[f32; 3], Error> {
        let height = *self
            .heights
            .get(index)
            .ok_or(Error("GTI position index out of bounds"))?;
        let x = ((index % self.width) as f64 * self.spacing as f64 + self.origin[0] as f64) as f32;
        let y = ((index / self.width) as f64 * self.spacing as f64 + self.origin[1] as f64) as f32;
        if !x.is_finite() || !y.is_finite() {
            return Err(Error("non-finite GTI position"));
        }
        Ok([x, y, height])
    }
    /// Ordered triples demonstrated as native face-normal constructor inputs.
    /// Renderer draw equivalence remains a separate validation step.
    pub fn triangles(&self) -> Result<Vec<[u32; 3]>, Error> {
        let mut triangles = Vec::new();
        for (index, tag) in self.tags.iter().enumerate() {
            let orders: &[[usize; 3]] = match tag {
                0 => continue,
                1 => &[[1, 2, 0]],
                2 => &[[2, 1, 3]],
                3 => &[[3, 0, 1]],
                4 | 7 => &[[0, 3, 2]],
                5 => &[[0, 3, 2], [3, 0, 1]],
                6 => &[[1, 2, 0], [2, 1, 3]],
                _ => return Err(Error("unsupported GTI triangle tag")),
            };
            if index % self.width == self.width - 1 || index / self.width == self.height - 1 {
                return Err(Error("GTI triangle crosses grid boundary"));
            }
            let corners = [index, index + 1, index + self.width, index + self.width + 1];
            for order in orders {
                let triangle = order.map(|i| corners[i] as u32);
                if triangle.iter().any(|i| self.heights[*i as usize] == -4096.) {
                    return Err(Error("GTI triangle references sentinel height"));
                }
                triangles.push(triangle);
            }
        }
        Ok(triangles)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(tag: u8) -> Vec<u8> {
        let mut data = vec![0; 96];
        for (at, value) in [(0, 0x94965003u32), (24, 2), (28, 2), (52, 3)] {
            data[at..at + 4].copy_from_slice(&value.to_le_bytes());
        }
        for (at, value) in [(20, 3f32), (32, 40.)] {
            data[at..at + 4].copy_from_slice(&value.to_le_bytes());
        }
        data[64..74].copy_from_slice(b"synthetic\0");
        data.push(3);
        for i in 0..4 {
            data.extend((i as f32).to_le_bytes());
            data.extend([if i == 0 { tag } else { 0 }, 255, 0, 0]);
        }
        data
    }
    #[test]
    fn run_grid_colors_positions_and_truncations() {
        let data = fixture(5);
        let t = parse(&data).unwrap();
        assert_eq!((t.width, t.height), (4, 4));
        assert_eq!(t.origin, [-40., -40.]);
        assert_eq!(t.colors[5], 0xffff0000);
        assert_eq!(t.position(10).unwrap(), [40., 40., 3.]);
        assert_eq!(t.triangles().unwrap(), vec![[5, 10, 9], [10, 5, 6]]);
        for end in 0..data.len() {
            assert!(parse(&data[..end]).is_err(), "truncation {end}");
        }
        let mut tail = data;
        tail.extend([42; 4]);
        assert_eq!(parse(&tail).unwrap().opaque_tail, [42; 4]);
    }
    #[test]
    fn rejects_bad_profiles_counts_and_runs() {
        for (offset, value) in [
            (0, 0x94965002u32),
            (4, 1),
            (24, u32::MAX),
            (32, 0),
            (97, f32::NAN.to_bits()),
        ] {
            let mut data = fixture(5);
            data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
            assert!(parse(&data).is_err());
        }
        let mut data = fixture(5);
        data[96] = 128;
        assert!(parse(&data).is_err());
        assert!(parse(&fixture(8)).is_err());
    }
    #[test]
    fn every_measured_topology_dispatch_is_explicit() {
        for tag in 1..=7 {
            let t = parse(&fixture(tag)).unwrap();
            assert_eq!(
                t.triangles().unwrap().len(),
                if tag == 5 || tag == 6 { 2 } else { 1 }
            );
        }
    }
}
