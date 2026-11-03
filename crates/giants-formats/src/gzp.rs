//! Measured GZP modes 1/2; see docs/GZP.md and tools/gzp.py for independent checks.
use crate::{Error, slice, u32_at};
const MAX_OUTPUT: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub unpacked_size: usize,
    packed_size: usize,
    offset: usize,
    mode: u32,
}
pub struct Archive<'a> {
    data: &'a [u8],
    entries: Vec<Entry>,
}
impl<'a> Archive<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Self, Error> {
        if u32_at(data, 0)? != 0x6608_f101 {
            return Err(Error("invalid GZP signature"));
        }
        let index = u32_at(data, 4)? as usize;
        if index < 8 {
            return Err(Error("index overlaps header"));
        }
        slice(data, index, 8)?;
        let count = u32_at(data, index + 4)? as usize;
        if count > (data.len() - index - 8) / 19 {
            return Err(Error("index count exceeds capacity"));
        }
        let mut at = index + 8;
        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            let row = slice(data, at, 18)?;
            let packed_size = u32_at(row, 0)? as usize;
            let unpacked_size = u32_at(row, 4)? as usize;
            let metadata = u32_at(row, 8)?;
            let offset = u32_at(row, 12)? as usize;
            let mode = row[16] as u32;
            let length = row[17] as usize;
            at += 18;
            let name = slice(data, at, length)?;
            if name.last() != Some(&0) || name[..name.len() - 1].contains(&0) || !name.is_ascii() {
                return Err(Error("invalid ASCII member name"));
            }
            at += length;
            if packed_size < 16
                || offset < 8
                || offset
                    .checked_add(packed_size)
                    .is_none_or(|end| end > index)
            {
                return Err(Error("member outside data area"));
            }
            if !matches!(mode, 1 | 2) || unpacked_size > MAX_OUTPUT {
                return Err(Error("unsupported mode or output budget"));
            }
            let block = slice(data, offset, 16)?;
            if u32_at(block, 0)? as usize != packed_size
                || u32_at(block, 4)? as usize != unpacked_size
                || u32_at(block, 8)? != metadata
                || u32_at(block, 12)? != mode
            {
                return Err(Error("block/index metadata mismatch"));
            }
            entries.push(Entry {
                name: String::from_utf8(name[..name.len() - 1].to_vec()).unwrap(),
                packed_size,
                unpacked_size,
                offset,
                mode,
            });
        }
        if at != data.len() {
            return Err(Error("trailing index data"));
        }
        let mut ranges: Vec<_> = entries
            .iter()
            .map(|e| (e.offset, e.offset + e.packed_size))
            .collect();
        ranges.sort_unstable();
        if ranges.windows(2).any(|r| r[0].1 > r[1].0) {
            return Err(Error("overlapping blocks"));
        }
        Ok(Self { data, entries })
    }
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }
    pub fn read(&self, name: &str) -> Result<Vec<u8>, Error> {
        let mut matches = self
            .entries
            .iter()
            .filter(|entry| entry.name.eq_ignore_ascii_case(name));
        let entry = matches.next().ok_or(Error("member not found"))?;
        if matches.next().is_some() {
            return Err(Error("ambiguous member name"));
        }
        let data = slice(self.data, entry.offset + 16, entry.packed_size - 16)?;
        if entry.mode == 2 {
            if data.len() != entry.unpacked_size {
                return Err(Error("raw length mismatch"));
            }
            Ok(data.to_vec())
        } else {
            decode_lzss(data, entry.unpacked_size)
        }
    }
}

pub fn decode_lzss(data: &[u8], expected: usize) -> Result<Vec<u8>, Error> {
    if expected > MAX_OUTPUT {
        return Err(Error("output budget exceeded"));
    }
    let mut ring = [b' '; 4096];
    let mut initialized = [false; 4096];
    initialized[..0xfee].fill(true);
    let mut write = 0xfee;
    let mut at = 0;
    let mut result = Vec::with_capacity(expected);
    while result.len() < expected {
        let flags = *data.get(at).ok_or(Error("missing flags"))?;
        at += 1;
        for bit in 0..8 {
            if result.len() == expected {
                break;
            }
            if flags & (1 << bit) != 0 {
                let value = *data.get(at).ok_or(Error("truncated literal"))?;
                at += 1;
                result.push(value);
                ring[write] = value;
                initialized[write] = true;
                write = (write + 1) & 4095;
            } else {
                let pair = slice(data, at, 2)?;
                at += 2;
                let offset = pair[0] as usize | ((pair[1] as usize & 0xf0) << 4);
                let length = (pair[1] as usize & 15) + 3;
                if length > expected - result.len() {
                    return Err(Error("reference exceeds output"));
                }
                for i in 0..length {
                    let source = (offset + i) & 4095;
                    if !initialized[source] {
                        return Err(Error("uninitialized native dictionary tail"));
                    }
                    let value = ring[source];
                    result.push(value);
                    ring[write] = value;
                    initialized[write] = true;
                    write = (write + 1) & 4095;
                }
            }
        }
    }
    if at != data.len() {
        return Err(Error("unconsumed compressed bytes"));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn literals_overlap_and_native_initialization() {
        assert_eq!(
            decode_lzss(&[7, b'A', b'B', b'C', 0xee, 0xf3], 9).unwrap(),
            b"ABCABCABC"
        );
        assert_eq!(decode_lzss(&[0, 0, 0], 3).unwrap(), b"   ");
        assert!(decode_lzss(&[0, 0xee, 0xf0], 3).is_err());
        assert!(decode_lzss(&[0, 0, 0], 2).is_err());
        assert!(decode_lzss(&[1], 1).is_err());
        assert!(decode_lzss(&[1, b'A', 0], 1).is_err());
    }
    #[test]
    fn synthetic_archive_and_truncations() {
        let mut data = Vec::new();
        for word in [0x6608_f101u32, 27, 19, 3, 0, 2] {
            data.extend(word.to_le_bytes());
        }
        data.extend(b"abc");
        data.extend(0u32.to_le_bytes());
        data.extend(1u32.to_le_bytes());
        for word in [19u32, 3, 0, 8] {
            data.extend(word.to_le_bytes());
        }
        data.extend([2, 2, b'x', 0]);
        assert_eq!(Archive::parse(&data).unwrap().read("x").unwrap(), b"abc");
        for end in 0..data.len() {
            assert!(Archive::parse(&data[..end]).is_err(), "prefix {end}");
        }
        let mut bad = data.clone();
        bad[16] = 1;
        assert!(Archive::parse(&bad).is_err());
    }
}
