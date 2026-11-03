//! Measured GBS options 6/7 and positive triangle-list runs only.
use crate::{Error, slice};

#[derive(Debug)]
pub struct Material {
    pub texture: String,
    pub bump_texture: String,
    pub falloff: f32,
    pub blend: f32,
    pub flags: u32,
    pub colors: [u32; 4],
    pub power: f32,
}
#[derive(Debug)]
pub struct Subobject {
    pub name: String,
    pub group: usize,
    pub triangles: Vec<[u16; 3]>,
    pub vertex_start: usize,
    pub vertex_count: usize,
    pub material: Material,
}
#[derive(Debug)]
pub struct Mesh {
    pub positions: Vec<[f32; 3]>,
    pub indices: Vec<u16>,
    pub uv: Vec<[f32; 2]>,
    pub rgb: Vec<[u8; 3]>,
    pub groups: Vec<[u32; 5]>,
    pub subobjects: Vec<Subobject>,
    pub normal_count: usize,
    pub normal_words: Vec<u16>,
    pub normal_indices: Vec<u16>,
}
struct Cursor<'a> {
    data: &'a [u8],
    at: usize,
}
impl<'a> Cursor<'a> {
    fn read(&mut self, size: usize) -> Result<&'a [u8], Error> {
        let result = slice(self.data, self.at, size)?;
        self.at += size;
        Ok(result)
    }
    fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(self.read(4)?.try_into().unwrap()))
    }
    fn u16(&mut self) -> Result<u16, Error> {
        Ok(u16::from_le_bytes(self.read(2)?.try_into().unwrap()))
    }
    fn f32(&mut self) -> Result<f32, Error> {
        let value = f32::from_bits(self.u32()?);
        if !value.is_finite() {
            return Err(Error("non-finite GBS scalar"));
        }
        Ok(value)
    }
    fn floats<const N: usize>(&mut self) -> Result<[f32; N], Error> {
        let mut out = [0.; N];
        for v in &mut out {
            *v = self.f32()?;
        }
        Ok(out)
    }
    fn count(&mut self, bytes: usize, limit: usize) -> Result<usize, Error> {
        let count = self.u32()? as usize;
        if count > limit || count * bytes > self.data.len() - self.at {
            return Err(Error("GBS count exceeds bounds or budget"));
        }
        Ok(count)
    }
    fn name(&mut self) -> Result<String, Error> {
        let bytes = self.read(32)?;
        let end = bytes
            .iter()
            .position(|b| *b == 0)
            .ok_or(Error("unterminated GBS name"))?;
        if !bytes[..end].is_ascii() {
            return Err(Error("non-ASCII GBS name"));
        }
        Ok(String::from_utf8(bytes[..end].to_vec()).unwrap())
    }
}
pub fn parse(data: &[u8]) -> Result<Mesh, Error> {
    let mut c = Cursor { data, at: 0 };
    if c.u32()? != 0xaa0100be {
        return Err(Error("unsupported GBS magic"));
    }
    let options = c.u32()?;
    if options != 6 && options != 7 {
        return Err(Error("unsupported GBS options"));
    }
    let count = c.count(12, 65535)?;
    if count == 0 {
        return Err(Error("empty GBS positions"));
    }
    let positions = (0..count)
        .map(|_| c.floats())
        .collect::<Result<Vec<_>, _>>()?;
    let (normal_count, normal_words) = if options & 1 != 0 {
        let normals = c.u32()? as usize;
        if normals > 65535 {
            return Err(Error("GBS normal budget exceeded"));
        }
        let words = c.count(2, 262140)?;
        (
            normals,
            (0..words).map(|_| c.u16()).collect::<Result<Vec<_>, _>>()?,
        )
    } else {
        (0, Vec::new())
    };
    let count = c.count(2, 65535)?;
    if count == 0 {
        return Err(Error("empty GBS render vertices"));
    }
    let indices = (0..count).map(|_| c.u16()).collect::<Result<Vec<_>, _>>()?;
    if indices.iter().any(|i| *i as usize >= positions.len()) {
        return Err(Error("GBS position index out of bounds"));
    }
    let normal_indices = if options & 1 != 0 {
        (0..count).map(|_| c.u16()).collect::<Result<Vec<_>, _>>()?
    } else {
        Vec::new()
    };
    let uv = (0..count)
        .map(|_| c.floats())
        .collect::<Result<Vec<_>, _>>()?;
    let rgb = (0..count)
        .map(|_| Ok(c.read(3)?.try_into().unwrap()))
        .collect::<Result<Vec<_>, Error>>()?;
    let count = c.count(20, 4096)?;
    let mut groups = Vec::new();
    for _ in 0..count {
        groups.push([c.u32()?, c.u32()?, c.u32()?, c.u32()?, c.u32()?]);
    }
    let count = c.count(48, 4096)?;
    let mut subobjects = Vec::new();
    for _ in 0..count {
        let name = c.name()?;
        let group = c.u32()? as usize;
        if group >= groups.len() {
            return Err(Error("GBS group index out of bounds"));
        }
        let triangle_count = c.u32()? as usize;
        if triangle_count > 65535 {
            return Err(Error("GBS triangle budget exceeded"));
        }
        let word_count = c.count(2, 262140)?;
        let words = (0..word_count)
            .map(|_| c.u16())
            .collect::<Result<Vec<_>, _>>()?;
        let mut triangles = Vec::new();
        let mut at = 0;
        while at < words.len() {
            let run = words[at] as usize;
            at += 1;
            if run == 0 || run & 0x8000 != 0 {
                return Err(Error("unsupported GBS triangle run"));
            }
            if triangles.len() + run > triangle_count || at + run * 3 > words.len() {
                return Err(Error("GBS triangle run out of bounds"));
            }
            for _ in 0..run {
                let triangle: [u16; 3] = words[at..at + 3].try_into().unwrap();
                if triangle.iter().any(|i| *i as usize >= indices.len()) {
                    return Err(Error("GBS triangle index out of bounds"));
                }
                triangles.push(triangle);
                at += 3;
            }
        }
        if triangles.len() != triangle_count {
            return Err(Error("GBS triangle count mismatch"));
        }
        let vertex_start = c.u32()? as usize;
        let vertex_count = c.u32()? as usize;
        if vertex_start + vertex_count > indices.len() {
            return Err(Error("GBS subobject vertex range out of bounds"));
        }
        let material = Material {
            texture: c.name()?,
            bump_texture: c.name()?,
            falloff: c.f32()?,
            blend: c.f32()?,
            flags: c.u32()?,
            colors: [c.u32()?, c.u32()?, c.u32()?, c.u32()?],
            power: c.f32()?,
        };
        subobjects.push(Subobject {
            name,
            group,
            triangles,
            vertex_start,
            vertex_count,
            material,
        });
    }
    if c.at != data.len() {
        return Err(Error("unconsumed GBS bytes"));
    }
    Ok(Mesh {
        positions,
        indices,
        uv,
        rgb,
        groups,
        subobjects,
        normal_count,
        normal_words,
        normal_indices,
    })
}

impl Mesh {
    /// Extrema over all source positions, preserving the first value on ties.
    pub fn bounds(&self) -> Result<[f32; 6], Error> {
        let first = self.positions.first().ok_or(Error("empty GBS bounds"))?;
        let mut bounds = [first[0], first[1], first[2], first[0], first[1], first[2]];
        for point in &self.positions {
            if !point.iter().all(|v| v.is_finite()) {
                return Err(Error("non-finite GBS bounds"));
            }
            for axis in 0..3 {
                if point[axis] < bounds[axis] {
                    bounds[axis] = point[axis];
                }
                if point[axis] > bounds[axis + 3] {
                    bounds[axis + 3] = point[axis];
                }
            }
        }
        Ok(bounds)
    }
    /// Area-weighted list normals: wide cross product, then ordered f32 additions.
    /// Only the contiguous, non-normalized profile demonstrated on the logo is supported.
    pub fn render_normals(&self) -> Result<Vec<[f32; 3]>, Error> {
        self.render_normals_with_positions(&self.positions)
    }

    /// Rebuild the same normal lists after rigid-cluster animation.
    /// Positions retain the source topology and are not normalized here.
    pub fn render_normals_with_positions(
        &self,
        positions: &[[f32; 3]],
    ) -> Result<Vec<[f32; 3]>, Error> {
        if positions.len() != self.positions.len()
            || !positions.iter().flatten().all(|v| v.is_finite())
        {
            return Err(Error("invalid GBS animated positions"));
        }
        if self.normal_count == 0 || self.normal_indices.len() != self.indices.len() {
            return Err(Error("normal definitions required"));
        }
        let mut normals = vec![[0f32; 3]; self.normal_count];
        let (mut position_end, mut normal_end, mut word_end) = (0, 0, 0);
        for (group, record) in self.groups.iter().enumerate() {
            let [
                position_start,
                position_count,
                normal_start,
                normal_count,
                word_start,
            ] = record.map(|v| v as usize);
            if position_start != position_end
                || normal_start != normal_end
                || word_start != word_end
                || normal_start + normal_count > normals.len()
                || position_start + position_count > self.positions.len()
            {
                return Err(Error("unsupported GBS normal group ranges"));
            }
            position_end += position_count;
            let mut faces = Vec::new();
            for sub in self.subobjects.iter().filter(|s| s.group == group) {
                for triangle in &sub.triangles {
                    let mut p = [[0f64; 3]; 3];
                    for (out, index) in p.iter_mut().zip(triangle) {
                        let index = self.indices[*index as usize] as usize;
                        if !(position_start..position_end).contains(&index) {
                            return Err(Error("GBS face outside normal group"));
                        }
                        *out = positions[index].map(f64::from);
                    }
                    let a = std::array::from_fn::<_, 3, _>(|i| p[1][i] - p[0][i]);
                    let b = std::array::from_fn::<_, 3, _>(|i| p[2][i] - p[0][i]);
                    faces.push([
                        (a[1] * b[2] - a[2] * b[1]) as f32,
                        (a[2] * b[0] - a[0] * b[2]) as f32,
                        (a[0] * b[1] - a[1] * b[0]) as f32,
                    ]);
                }
            }
            if faces.is_empty() {
                return Err(Error("empty GBS normal face group"));
            }
            let mut at = word_start;
            for normal in &mut normals[normal_start..normal_start + normal_count] {
                let count = *self
                    .normal_words
                    .get(at)
                    .ok_or(Error("truncated GBS normal definition"))?
                    as usize;
                at += 1;
                if count == 0 {
                    return Err(Error("empty GBS normal definition"));
                }
                let refs = self
                    .normal_words
                    .get(at..at + count)
                    .ok_or(Error("truncated GBS normal face list"))?;
                *normal = *faces
                    .get(refs[0] as usize)
                    .ok_or(Error("GBS normal face out of bounds"))?;
                for index in &refs[1..] {
                    let face = faces
                        .get(*index as usize)
                        .ok_or(Error("GBS normal face out of bounds"))?;
                    for i in 0..3 {
                        normal[i] += face[i];
                    }
                }
                if !normal.iter().all(|v| v.is_finite()) {
                    return Err(Error("non-finite generated GBS normal"));
                }
                at += count;
            }
            normal_end = normal_start + normal_count;
            word_end = at;
        }
        if normal_end != normals.len()
            || word_end != self.normal_words.len()
            || position_end != self.positions.len()
        {
            return Err(Error("unconsumed GBS normal groups"));
        }
        self.normal_indices
            .iter()
            .map(|i| {
                normals
                    .get(*i as usize)
                    .copied()
                    .ok_or(Error("GBS render normal out of bounds"))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Vec<u8> {
        let mut out = Vec::new();
        fn u(out: &mut Vec<u8>, v: u32) {
            out.extend(v.to_le_bytes());
        }
        fn h(out: &mut Vec<u8>, v: u16) {
            out.extend(v.to_le_bytes());
        }
        for v in [0xaa0100be, 7, 3] {
            u(&mut out, v);
        }
        for v in [0f32, 0., 0., 1., 0., 0., 0., 1., 0.] {
            u(&mut out, v.to_bits());
        }
        for v in [1, 2] {
            u(&mut out, v);
        }
        for v in [1, 0] {
            h(&mut out, v);
        }
        u(&mut out, 3);
        for v in [0, 1, 2, 0, 0, 0] {
            h(&mut out, v);
        }
        for v in [0f32, 0., 1., 0., 0., 1.] {
            u(&mut out, v.to_bits());
        }
        out.extend([255u8; 9]);
        for v in [1, 0, 3, 0, 1, 0, 1] {
            u(&mut out, v);
        }
        out.extend([0u8; 32]);
        for v in [0, 1, 4] {
            u(&mut out, v);
        }
        for v in [1, 0, 1, 2] {
            h(&mut out, v);
        }
        for v in [0, 3] {
            u(&mut out, v);
        }
        out.extend([0u8; 64 + 32]);
        out
    }
    #[test]
    fn triangle_normals_and_all_truncations() {
        let data = fixture();
        let mesh = parse(&data).unwrap();
        assert_eq!(mesh.subobjects[0].triangles, vec![[0, 1, 2]]);
        assert_eq!(mesh.render_normals().unwrap(), vec![[0., 0., 1.]; 3]);
        for end in 0..data.len() {
            assert!(parse(&data[..end]).is_err(), "truncation {end}");
        }
        let mut trailing = data;
        trailing.push(0);
        assert!(parse(&trailing).is_err());
    }
    #[test]
    fn rejects_bad_geometry_and_normal_references() {
        let data = fixture();
        for (offset, value) in [(4, 14u32), (8, u32::MAX), (12, f32::NAN.to_bits())] {
            let mut bad = data.clone();
            bad[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
            assert!(parse(&bad).is_err());
        }
        let mut mesh = parse(&data).unwrap();
        mesh.normal_words[1] = 1;
        assert!(mesh.render_normals().is_err());
        mesh.normal_words[1] = 0;
        mesh.normal_indices[0] = 1;
        assert!(mesh.render_normals().is_err());
    }

    #[test]
    fn animated_normals_follow_deformation_and_reject_invalid_positions() {
        let mesh = parse(&fixture()).unwrap();
        let mut positions = mesh.positions.clone();
        positions[2] = [0., 0., 2.];
        assert_eq!(
            mesh.render_normals_with_positions(&positions).unwrap(),
            vec![[0., -2., 0.]; 3]
        );
        assert!(mesh.render_normals_with_positions(&positions[..2]).is_err());
        positions[2][0] = f32::NAN;
        assert!(mesh.render_normals_with_positions(&positions).is_err());
    }
}
