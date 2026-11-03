//! Indexed GB2 environment mesh profile, established on world.gb2/sky.
use crate::{Error, slice, u32_at};
pub struct Binding {
    pub id: u8,
    pub container: String,
    pub object: String,
}
pub struct Mesh {
    pub name: String,
    pub texture: String,
    pub positions: Vec<[f32; 3]>,
    pub uv: Vec<[f32; 2]>,
    pub triangles: Vec<[u32; 3]>,
}
fn string(data: &[u8]) -> Result<String, Error> {
    let end = data
        .iter()
        .position(|v| *v == 0)
        .ok_or(Error("unterminated GB2 name"))?;
    if end == 0 || !data[..end].is_ascii() {
        return Err(Error("invalid GB2 name"));
    }
    Ok(String::from_utf8(data[..end].to_vec()).unwrap())
}
pub fn bindings(data: &[u8]) -> Result<Vec<Binding>, Error> {
    if u32_at(data, 0)? != 0x1a0002dd || u32_at(data, 4)? as usize != data.len() - 8 {
        return Err(Error("unsupported GameObj header"));
    }
    let mut at = 8;
    let mut out = Vec::new();
    while at < data.len() {
        let id = data[at];
        at += 1;
        if id == 255 {
            if at != data.len() {
                return Err(Error("trailing GameObj bytes"));
            }
            return Ok(out);
        }
        if !(1..=9).contains(&id) || out.len() >= 256 {
            return Err(Error("unsupported GameObj command"));
        }
        let container = string(slice(data, at, (data.len() - at).min(128))?)?;
        at += container.len() + 1;
        let object = string(slice(data, at, (data.len() - at).min(128))?)?;
        at += object.len() + 1;
        out.push(Binding {
            id,
            container,
            object,
        });
    }
    Err(Error("missing GameObj terminator"))
}
pub fn parse(data: &[u8], selected: &str) -> Result<Mesh, Error> {
    if u32_at(data, 0)? != 0xaa0100ab {
        return Err(Error("unsupported GB2 version"));
    }
    let count = u32_at(data, 4)? as usize;
    if count == 0 || count > 4096 {
        return Err(Error("GB2 entry budget"));
    }
    let header_end = 8 + count * 4;
    slice(data, 0, header_end)?;
    let mut previous = header_end;
    let mut chosen = None;
    for i in 0..count {
        let start = u32_at(data, 8 + i * 4)? as usize;
        let end = if i + 1 < count {
            u32_at(data, 12 + i * 4)? as usize
        } else {
            data.len()
        };
        if start != previous || end <= start {
            return Err(Error("invalid GB2 entry bounds"));
        }
        let record = slice(data, start, end - start)?;
        let name = string(slice(record, 0, 16)?)?;
        if name.eq_ignore_ascii_case(selected) {
            if chosen.is_some() {
                return Err(Error("duplicate GB2 selection"));
            }
            chosen = Some(record);
        }
        previous = end;
    }
    let record = chosen.ok_or(Error("GB2 object missing"))?;
    if [
        u32_at(record, 16)?,
        u32_at(record, 20)?,
        u32_at(record, 24)?,
    ] != [2, 0, 0]
    {
        return Err(Error("unsupported GB2 mesh flags"));
    }
    let name = string(slice(record, 0, 16)?)?;
    let texture = string(slice(record, 28, 16)?)?;
    let vertices = u32_at(record, 44)? as usize;
    let faces = u32_at(record, 48)? as usize;
    if vertices == 0
        || vertices > 65535
        || faces == 0
        || faces > 131072
        || 52 + vertices * 20 + faces * 12 != record.len()
    {
        return Err(Error("GB2 mesh budget or length"));
    }
    fn array<const N: usize>(data: &[u8], at: usize) -> Result<[f32; N], Error> {
        let mut out = [0.; N];
        for (i, v) in out.iter_mut().enumerate() {
            *v = f32::from_bits(u32_at(data, at + i * 4)?);
            if !v.is_finite() {
                return Err(Error("non-finite GB2 scalar"));
            }
        }
        Ok(out)
    }
    let positions = (0..vertices)
        .map(|i| array(record, 52 + i * 12))
        .collect::<Result<Vec<_>, _>>()?;
    let uv = (0..vertices)
        .map(|i| array(record, 52 + vertices * 12 + i * 8))
        .collect::<Result<Vec<_>, _>>()?;
    let mut triangles = Vec::with_capacity(faces);
    for i in 0..faces {
        let mut triangle = [0; 3];
        for (j, v) in triangle.iter_mut().enumerate() {
            *v = u32_at(record, 52 + vertices * 20 + i * 12 + j * 4)?;
            if *v as usize >= vertices {
                return Err(Error("GB2 index outside vertices"));
            }
        }
        triangles.push(triangle);
    }
    Ok(Mesh {
        name,
        texture,
        positions,
        uv,
        triangles,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Vec<u8> {
        let mut b = Vec::new();
        for v in [0xaa0100abu32, 1, 12] {
            b.extend(v.to_le_bytes());
        }
        let mut r = vec![0; 52];
        r[..3].copy_from_slice(b"sky");
        r[16..20].copy_from_slice(&2u32.to_le_bytes());
        r[28..31].copy_from_slice(b"tex");
        r[44..48].copy_from_slice(&3u32.to_le_bytes());
        r[48..52].copy_from_slice(&1u32.to_le_bytes());
        for v in [0f32, 0., 0., 1., 0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 1.] {
            r.extend(v.to_le_bytes());
        }
        for v in [0u32, 1, 2] {
            r.extend(v.to_le_bytes());
        }
        b.extend(r);
        b
    }
    #[test]
    fn mesh_and_rejections() {
        let b = fixture();
        let m = parse(&b, "SKY").unwrap();
        assert_eq!(m.triangles, [[0, 1, 2]]);
        assert_eq!(m.uv[2], [0., 1.]);
        for n in 0..b.len() {
            assert!(parse(&b[..n], "sky").is_err());
        }
        let mut bad = b.clone();
        bad[64..68].copy_from_slice(&f32::NAN.to_le_bytes());
        assert!(parse(&bad, "sky").is_err());
        let n = bad.len();
        bad = b;
        bad[n - 4..].copy_from_slice(&3u32.to_le_bytes());
        assert!(parse(&bad, "sky").is_err());
    }
    #[test]
    fn binding_bounds() {
        let body = b"\x01world.gb2\0sky\0\xff";
        let mut b = 0x1a0002ddu32.to_le_bytes().to_vec();
        b.extend((body.len() as u32).to_le_bytes());
        b.extend(body);
        let v = bindings(&b).unwrap();
        assert_eq!(v[0].container, "world.gb2");
        for n in 0..b.len() {
            assert!(bindings(&b[..n]).is_err());
        }
    }
}
