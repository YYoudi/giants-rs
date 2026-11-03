//! Bounded GText tables; translations retain source encoding and control bytes.
use crate::{Error, slice, u32_at};
use std::collections::BTreeMap;

pub type TextTable = BTreeMap<Vec<u8>, Vec<u8>>;

/// C-locale lookup over one validated language table. Values retain source
/// encoding; missing keys remain distinguishable from an empty translation.
#[derive(Debug, Clone)]
pub struct Catalog {
    entries: TextTable,
}
impl Catalog {
    pub fn parse(data: &[u8]) -> Result<Self, Error> {
        Self::from_table(parse(data)?)
    }
    pub fn from_table(table: TextTable) -> Result<Self, Error> {
        let mut entries = TextTable::new();
        for (key, text) in table {
            if key.is_empty() || key.len() > 127 || !key.is_ascii() || key.contains(&0) {
                return Err(Error("invalid GText lookup key"));
            }
            if entries.insert(key.to_ascii_lowercase(), text).is_some() {
                return Err(Error("case-insensitive duplicate GText key"));
            }
        }
        Ok(Self { entries })
    }
    pub fn get(&self, key: &[u8]) -> Option<&[u8]> {
        // Query strings are bounded ASCII C strings, like parsed table keys.
        if key.len() > 127 || !key.is_ascii() || key.contains(&0) {
            return None;
        }
        self.entries
            .get(&key.to_ascii_lowercase())
            .map(Vec::as_slice)
    }
    /// The ordinary native getter returns the caller's key when absent.
    pub fn resolve<'a>(&'a self, key: &'a [u8]) -> &'a [u8] {
        self.get(key).unwrap_or(key)
    }
}

pub fn parse(data: &[u8]) -> Result<TextTable, Error> {
    if !(12..=16 * 1024 * 1024).contains(&data.len()) {
        return Err(Error("GText size budget"));
    }
    let count = u32_at(data, 0)? as usize;
    let key_size = u32_at(data, 4)? as usize;
    let text_size = u32_at(data, 8)? as usize;
    if !(1..=65535).contains(&count) {
        return Err(Error("GText entry budget"));
    }
    let start = 12 + count * 8;
    let keys = slice(data, start, key_size)?;
    let texts = slice(data, start + key_size, text_size)?;
    if start + key_size + text_size != data.len() {
        return Err(Error("GText trailing data"));
    }
    fn string(pool: &[u8], at: usize) -> Result<&[u8], Error> {
        let tail = pool.get(at..).ok_or(Error("GText offset outside pool"))?;
        let n = tail
            .iter()
            .position(|&b| b == 0)
            .ok_or(Error("unterminated GText string"))?;
        Ok(&tail[..n])
    }
    let mut result = TextTable::new();
    for i in 0..count {
        let key = string(keys, u32_at(data, 12 + i * 8)? as usize)?;
        let text = string(texts, u32_at(data, 16 + i * 8)? as usize)?;
        if key.is_empty()
            || key.len() > 127
            || !key.is_ascii()
            || result.insert(key.to_vec(), text.to_vec()).is_some()
        {
            return Err(Error("invalid or duplicate GText key"));
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Vec<u8> {
        let mut b = Vec::new();
        for n in [2u32, 4, 4, 0, 0, 2, 2] {
            b.extend(n.to_le_bytes());
        }
        b.extend(b"a\0b\0x\0\xe9\0");
        b
    }
    #[test]
    fn preserves_bytes_and_rejects_bad_offsets_sizes_and_duplicates() {
        let b = fixture();
        assert_eq!(parse(&b).unwrap()[b"b".as_slice()], [0xe9]);
        for end in 0..b.len() {
            assert!(parse(&b[..end]).is_err());
        }
        for (at, value) in [(12, 4u32), (16, 4), (20, 0), (4, u32::MAX)] {
            let mut bad = b.clone();
            bad[at..at + 4].copy_from_slice(&value.to_le_bytes());
            assert!(parse(&bad).is_err());
        }
        let mut bad = b;
        *bad.last_mut().unwrap() = 1;
        assert!(parse(&bad).is_err());
    }

    #[test]
    fn catalog_preserves_empty_and_encoded_values_and_falls_back_to_key() {
        let mut table = TextTable::new();
        table.insert(b"Notice1".to_vec(), vec![0xe9, 1]);
        table.insert(b"Empty".to_vec(), vec![]);
        let catalog = Catalog::from_table(table).unwrap();
        assert_eq!(catalog.get(b"nOTICE1"), Some([0xe9, 1].as_slice()));
        assert_eq!(catalog.resolve(b"EMPTY"), b"");
        assert_eq!(catalog.resolve(b"Absent"), b"Absent");
        for bad in [b"Notice1\0".as_slice(), &[0xff], &[b'a'; 128]] {
            assert_eq!(catalog.get(bad), None);
        }
    }

    #[test]
    fn catalog_refuses_ambiguous_keys_instead_of_choosing_a_translation() {
        let mut table = TextTable::new();
        table.insert(b"Notice1".to_vec(), b"first".to_vec());
        table.insert(b"NOTICE1".to_vec(), b"second".to_vec());
        assert!(Catalog::from_table(table).is_err());
        let mut table = TextTable::new();
        table.insert(b"bad\0key".to_vec(), vec![]);
        assert!(Catalog::from_table(table).is_err());
    }
}
