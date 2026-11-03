//! Bounded rigid SKN profile. Weighted objects and multiple trees fail explicitly.
use crate::{Error, anm::floats, slice, u32_at};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug)]
pub struct Vertex {
    pub position: [f32; 3],
    pub index: usize,
}
#[derive(Debug)]
pub struct Cluster {
    pub offset: usize,
    pub handle: u16,
    pub bone: String,
    pub vertices: Vec<Vertex>,
}
#[derive(Debug)]
pub struct Object {
    pub offset: usize,
    pub name: String,
    pub vertex_count: usize,
    pub clusters: Vec<Cluster>,
}
#[derive(Debug)]
pub struct Skin {
    pub root: usize,
    pub handles: BTreeMap<u16, String>,
    pub objects: Vec<Object>,
}
fn word(data: &[u8], at: usize) -> Result<usize, Error> {
    Ok(u32_at(data, at)? as usize)
}
fn half(data: &[u8], at: usize) -> Result<u16, Error> {
    Ok(u16::from_le_bytes(slice(data, at, 2)?.try_into().unwrap()))
}
fn pointer(data: &[u8], at: usize, size: usize) -> Result<usize, Error> {
    let p = word(data, at)?;
    if p == 0 || !p.is_multiple_of(4) {
        return Err(Error("unaligned/null SKN pointer"));
    }
    slice(data, p, size)?;
    Ok(p)
}
fn names(data: &[u8], descriptor: usize, keys: usize) -> Result<Vec<(String, usize)>, Error> {
    let pool = pointer(data, descriptor + 4, 40)?;
    let count = word(data, pool + 28)?;
    if word(data, pool + 32)? != keys || !(1..=4096).contains(&count) {
        return Err(Error("unsupported SKN name table"));
    }
    let context = pointer(data, pool + 36, keys * 28)?;
    let array = pointer(data, context + 16, count * 4)?;
    let mut out = Vec::with_capacity(count);
    let mut labels = BTreeSet::new();
    let mut offsets = BTreeSet::new();
    for i in 0..count {
        let entry = pointer(data, array + i * 4, 8)?;
        let size = usize::from(half(data, entry)?);
        let record = slice(data, entry, size)?;
        if !size.is_multiple_of(4) {
            return Err(Error("unaligned SKN name record"));
        }
        let mut keys_at = Vec::new();
        for key in 0..keys {
            let at = usize::from(half(record, 2 + key * 2)?);
            if at < 2 + keys * 2 {
                return Err(Error("SKN name key overlaps header"));
            }
            slice(record, at, 4)?;
            keys_at.push(at);
        }
        let start = keys_at[0] + 4;
        let raw = record
            .get(start..)
            .ok_or(Error("SKN name outside record"))?;
        let end = raw
            .iter()
            .position(|v| *v == 0)
            .ok_or(Error("unterminated SKN name"))?;
        if !(1..=127).contains(&end) || !raw[..end].is_ascii() {
            return Err(Error("invalid SKN name"));
        }
        let name = String::from_utf8(raw[..end].to_vec()).unwrap();
        if !labels.insert(name.to_ascii_lowercase()) {
            return Err(Error("duplicate SKN name"));
        }
        let custom = word(record, keys_at[0])?;
        let value = if keys == 2 {
            let at = keys_at[1];
            let refs = half(record, at)?;
            let handle = half(record, at + 2)?;
            if refs == 0 || handle == 0 || custom != 0 {
                return Err(Error("invalid SKN handle record"));
            }
            usize::from(handle)
        } else {
            custom
        };
        offsets.insert(entry);
        out.push((name, value));
    }
    if keys == 2 {
        let second = pointer(data, context + 28 + 16, count * 4)?;
        let mut other = BTreeSet::new();
        for i in 0..count {
            other.insert(pointer(data, second + i * 4, 8)?);
        }
        if offsets != other || offsets.len() != count {
            return Err(Error("SKN name keys disagree"));
        }
    }
    Ok(out)
}
pub fn parse(data: &[u8]) -> Result<Skin, Error> {
    if !(40..=2 * 1024 * 1024).contains(&data.len()) || word(data, 0)? != data.len() {
        return Err(Error("invalid SKN image size"));
    }
    let group = pointer(data, 4, 32)?;
    let root = pointer(data, group + 28, 40)?;
    if word(data, root + 4)? != 6 || pointer(data, root + 12, 32)? != group {
        return Err(Error("unsupported SKN root"));
    }
    let mut handles = BTreeMap::new();
    for (name, value) in names(data, pointer(data, root + 16, 12)?, 2)? {
        if handles.insert(value as u16, name).is_some() {
            return Err(Error("duplicate SKN handle"));
        }
    }
    let trees = names(data, pointer(data, root + 20, 12)?, 1)?;
    let default = pointer(data, root + 24, 12)?;
    if trees.len() != word(data, root + 28)? || trees.len() != 1 || trees[0].1 != default {
        return Err(Error("unsupported SKN tree/default"));
    }
    let entries = names(data, default, 1)?;
    if entries.len() != word(data, root + 32)? {
        return Err(Error("SKN object count mismatch"));
    }
    let mut objects = Vec::new();
    let mut total_clusters = 0;
    for (name, at) in entries {
        slice(data, at, 20)?;
        if word(data, at + 4)? != root || word(data, at + 8)? != 0 {
            return Err(Error("weighted/invalid SKN object unsupported"));
        }
        let count = word(data, at + 12)?;
        let clusters = word(data, at + 16)?;
        if !(1..=65535).contains(&count) || !(1..=4096).contains(&clusters) {
            return Err(Error("SKN object budget"));
        }
        slice(data, at + 20, clusters * 4)?;
        let mut decoded = Vec::new();
        let mut indices = BTreeSet::new();
        for i in 0..clusters {
            let c = pointer(data, at + 20 + i * 4, 40)?;
            let handle = word(data, c + 8)?;
            let bone = u16::try_from(handle)
                .ok()
                .and_then(|h| handles.get(&h))
                .ok_or(Error("unknown SKN bone handle"))?;
            if word(data, c + 4)? != at {
                return Err(Error("SKN cluster backlink"));
            }
            floats::<6>(data, c + 12)?;
            let n = word(data, c + 36)?;
            if !(1..=count).contains(&n) {
                return Err(Error("SKN cluster vertex budget"));
            }
            slice(data, c + 40, n * 16)?;
            let mut vertices = Vec::with_capacity(n);
            for j in 0..n {
                let v = c + 40 + j * 16;
                let position = floats(data, v)?;
                let index = word(data, v + 12)?;
                if index >= count || !indices.insert(index) {
                    return Err(Error("invalid/overlapping rigid SKN vertex"));
                }
                vertices.push(Vertex { position, index });
            }
            decoded.push(Cluster {
                offset: c,
                handle: handle as u16,
                bone: bone.clone(),
                vertices,
            });
        }
        if indices.len() != count {
            return Err(Error("incomplete rigid SKN coverage"));
        }
        total_clusters += clusters;
        objects.push(Object {
            offset: at,
            name,
            vertex_count: count,
            clusters: decoded,
        });
    }
    if total_clusters != word(data, root + 36)? {
        return Err(Error("SKN cluster count mismatch"));
    }
    Ok(Skin {
        root,
        handles,
        objects,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn alloc(data: &mut Vec<u8>, size: usize) -> usize {
        let at = data.len();
        data.resize(at + ((size + 3) & !3), 0);
        at
    }
    fn set(data: &mut [u8], at: usize, value: usize) {
        data[at..at + 4].copy_from_slice(&(value as u32).to_le_bytes());
    }
    fn table(data: &mut Vec<u8>, name: &str, value: usize, handles: bool) -> usize {
        let desc = alloc(data, 12);
        let pool = alloc(data, 40);
        let keys = if handles { 2 } else { 1 };
        let ctx = alloc(data, keys * 28);
        let array = alloc(data, 4);
        set(data, desc + 4, pool);
        set(data, pool + 28, 1);
        set(data, pool + 32, keys);
        set(data, pool + 36, ctx);
        set(data, ctx + 16, array);
        if handles {
            set(data, ctx + 44, array);
        }
        let start = if handles { 8 } else { 4 };
        let size = ((start + 4 + name.len() + 1 + 3) & !3) + if handles { 4 } else { 0 };
        let entry = alloc(data, size);
        data[entry..entry + 2].copy_from_slice(&(size as u16).to_le_bytes());
        data[entry + 2..entry + 4].copy_from_slice(&(start as u16).to_le_bytes());
        if handles {
            data[entry + 4..entry + 6].copy_from_slice(&((size - 4) as u16).to_le_bytes());
            data[entry + size - 4..entry + size - 2].copy_from_slice(&1u16.to_le_bytes());
            data[entry + size - 2..entry + size].copy_from_slice(&(value as u16).to_le_bytes());
        } else {
            set(data, entry + start, value);
        }
        data[entry + start + 4..entry + start + 4 + name.len()].copy_from_slice(name.as_bytes());
        set(data, array, entry);
        desc
    }
    #[test]
    fn rigid_skin_checks_links_indices_weights_and_all_truncations() {
        let mut data = vec![0; 8];
        let group = alloc(&mut data, 32);
        let root = alloc(&mut data, 40);
        let object = alloc(&mut data, 24);
        let cluster = alloc(&mut data, 56);
        set(&mut data, object + 4, root);
        set(&mut data, object + 12, 1);
        set(&mut data, object + 16, 1);
        set(&mut data, object + 20, cluster);
        set(&mut data, cluster + 4, object);
        set(&mut data, cluster + 8, 1);
        set(&mut data, cluster + 36, 1);
        let default = table(&mut data, "mesh", object, false);
        let trees = table(&mut data, "Default Tree", default, false);
        let handles = table(&mut data, "bone", 1, true);
        let size = data.len();
        set(&mut data, 0, size);
        set(&mut data, 4, group);
        set(&mut data, group + 28, root);
        set(&mut data, root + 4, 6);
        set(&mut data, root + 12, group);
        set(&mut data, root + 16, handles);
        set(&mut data, root + 20, trees);
        set(&mut data, root + 24, default);
        set(&mut data, root + 28, 1);
        set(&mut data, root + 32, 1);
        set(&mut data, root + 36, 1);
        let skin = parse(&data).unwrap();
        assert_eq!(skin.objects[0].clusters[0].bone, "bone");
        for size in 8..data.len() {
            let mut cut = data[..size].to_vec();
            set(&mut cut, 0, size);
            assert!(parse(&cut).is_err());
        }
        for (at, value) in [
            (object + 4, root + 4),
            (object + 8, 1),
            (cluster + 4, object + 4),
            (cluster + 8, 2),
            (cluster + 52, 1),
            (root + 36, 2),
            (cluster + 40, 0x7fc00000),
        ] {
            let mut bad = data.clone();
            set(&mut bad, at, value);
            assert!(parse(&bad).is_err());
        }
    }
}
