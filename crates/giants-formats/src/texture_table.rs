//! Catalog texture declarations. No texture decoding or graphics allocation.
use crate::{Error, slice, u32_at};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub offset: usize,
    pub level: u8,
    pub flags: u8,
    pub name: String,
}
#[derive(Debug)]
pub struct Table {
    pub offset: usize,
    pub consumed: usize,
    pub entries: Vec<Entry>,
}
pub fn catalog(data: &[u8]) -> Result<Table, Error> {
    slice(data, 0, 32)?;
    if u32_at(data, 0)? != 0x1a0002e5 {
        return Err(Error("unsupported texture catalog header"));
    }
    let start = u32_at(data, 8)? as usize;
    if start < 32 {
        return Err(Error("texture table overlaps catalog header"));
    }
    let count = u32_at(data, start)? as usize;
    if count > 4096 {
        return Err(Error("texture table count budget exceeded"));
    }
    let mut at = start + 4;
    let mut entries = Vec::new();
    for _ in 0..count {
        let offset = at;
        let header = slice(data, at, 3)?;
        let length = usize::from(header[2]);
        if !(2..=32).contains(&length) {
            return Err(Error("texture name outside native stack profile"));
        }
        at += 3;
        let raw = slice(data, at, length)?;
        if raw[length - 1] != 0 || raw[..length - 1].contains(&0) || !raw.is_ascii() {
            return Err(Error("unsupported texture name framing"));
        }
        entries.push(Entry {
            offset,
            level: header[0],
            flags: header[1],
            name: String::from_utf8(raw[..length - 1].to_vec()).unwrap(),
        });
        at += length;
    }
    Ok(Table {
        offset: start,
        consumed: at - start,
        entries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_incomplete_or_unsafe_native_name_profiles() {
        let mut data = vec![0; 32];
        data[..4].copy_from_slice(&0x1a0002e5u32.to_le_bytes());
        data[8..12].copy_from_slice(&32u32.to_le_bytes());
        data.extend(1u32.to_le_bytes());
        data.extend([2, 4, 5]);
        data.extend(b"leaf\0");
        let table = catalog(&data).unwrap();
        assert_eq!(table.entries[0].name, "leaf");
        assert_eq!((table.entries[0].level, table.entries[0].flags), (2, 4));
        for end in 0..data.len() {
            assert!(catalog(&data[..end]).is_err());
        }
        data[38] = 33;
        assert!(catalog(&data).is_err());
        data[38] = 5;
        data[43] = b'x';
        assert!(catalog(&data).is_err());
    }
}
