//! Bounded initialized PE data views shared by external character metadata readers.
//! Header tuples select a previously inspected layout, not authenticity or code execution.
use std::{io::Read, path::Path};
pub(crate) const MAX_IMAGE: usize = 4 * 1024 * 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Layout {
    DebugReference,
    Retail,
    Original,
}
pub(crate) fn word(bytes: &[u8], at: usize) -> Result<u32, String> {
    let end = at.checked_add(4).ok_or("PE offset overflow")?;
    Ok(u32::from_le_bytes(
        bytes
            .get(at..end)
            .ok_or("truncated PE data")?
            .try_into()
            .unwrap(),
    ))
}
fn half(bytes: &[u8], at: usize) -> Result<u16, String> {
    let end = at.checked_add(2).ok_or("PE offset overflow")?;
    Ok(u16::from_le_bytes(
        bytes
            .get(at..end)
            .ok_or("truncated PE header")?
            .try_into()
            .unwrap(),
    ))
}
pub(crate) struct DataSection<'a> {
    pub(crate) bytes: &'a [u8],
    pub(crate) rva: u32,
    pub(crate) base: u32,
}
impl DataSection<'_> {
    pub(crate) fn data(&self, rva: u32, len: usize) -> Result<&[u8], String> {
        let start = rva.checked_sub(self.rva).ok_or("data RVA below section")? as usize;
        self.bytes
            .get(start..start.checked_add(len).ok_or("data range overflow")?)
            .ok_or_else(|| "data outside initialized PE section".into())
    }
}

pub(crate) struct Image<'a> {
    pub(crate) data: DataSection<'a>,
    pub(crate) layout: Layout,
    data_virtual_len: u32,
}
pub(crate) fn read(path: &Path) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take((MAX_IMAGE + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    Ok(bytes)
}
impl<'a> Image<'a> {
    /// PE loader image before program initialization. Uninitialized data bytes
    /// are zero-filled; this executes neither CRT nor game startup writes.
    pub(crate) fn initial_word(&self, rva: u32) -> Result<u32, String> {
        let start = rva
            .checked_sub(self.data.rva)
            .ok_or("initial RVA below data")?;
        if start.checked_add(4).ok_or("initial data range overflow")? > self.data_virtual_len {
            return Err("initial word outside data virtual section".into());
        }
        Ok(u32::from_le_bytes(std::array::from_fn(|i| {
            self.data
                .bytes
                .get(start as usize + i)
                .copied()
                .unwrap_or(0)
        })))
    }
    pub(crate) fn decode(bytes: &'a [u8]) -> Result<Self, String> {
        if bytes.len() > MAX_IMAGE || bytes.get(..2) != Some(b"MZ") {
            return Err("unsupported PE image".into());
        }
        let pe = word(bytes, 0x3c)? as usize;
        if pe > MAX_IMAGE - 256
            || bytes.get(pe..pe + 4) != Some(b"PE\0\0")
            || half(bytes, pe + 4)? != 0x14c
            || half(bytes, pe + 24)? != 0x10b
            || half(bytes, pe + 20)? != 224
        {
            return Err("unsupported PE32 header".into());
        }
        let base = word(bytes, pe + 52)?;
        if base != 0x400000 {
            return Err("unsupported PE image base".into());
        }
        let (layout, expected_rva) =
            match (bytes.len(), word(bytes, pe + 8)?, word(bytes, pe + 80)?) {
                (1_650_688, 977_354_863, 2_506_752) => (Layout::DebugReference, 1_413_120),
                (1_630_208, 993_669_874, 2_498_560) => (Layout::Retail, 1_392_640),
                (1_634_304, 975_611_382, 2_490_368) => (Layout::Original, 1_396_736),
                _ => return Err("unacquired executable metadata layout".into()),
            };
        let count = half(bytes, pe + 6)? as usize;
        if count != 4 {
            return Err("unsupported PE section layout".into());
        }
        let mut found = None;
        let mut ranges = Vec::new();
        for i in 0..count {
            let at = pe + 248 + i * 40;
            let rva = word(bytes, at + 12)?;
            let raw_len = word(bytes, at + 16)? as usize;
            let raw = word(bytes, at + 20)? as usize;
            let end = raw.checked_add(raw_len).ok_or("section overflow")?;
            let rva_end = u64::from(rva) + u64::from(word(bytes, at + 8)?).max(raw_len as u64);
            if end > bytes.len()
                || raw < pe + 248 + count * 40
                || rva_end > u64::from(word(bytes, pe + 80)?)
            {
                return Err("invalid PE section bounds".into());
            }
            for &(old_raw, old_end, old_rva, old_rva_end) in &ranges {
                if (raw < old_end && old_raw < end)
                    || (u64::from(rva) < old_rva_end && old_rva < rva_end)
                {
                    return Err("overlapping PE sections".into());
                }
            }
            ranges.push((raw, end, u64::from(rva), rva_end));
            if bytes.get(at..at + 8) == Some(b".data\0\0\0") {
                if found.is_some()
                    || rva != expected_rva
                    || raw_len != 200_704
                    || word(bytes, at + 36)? & 0xe0000040 != 0xc0000040
                {
                    return Err("unsupported initialized data section".into());
                }
                found = Some((
                    DataSection {
                        bytes: &bytes[raw..end],
                        rva,
                        base,
                    },
                    word(bytes, at + 8)?.max(raw_len as u32),
                ));
            }
        }
        let (data, data_virtual_len) = found.ok_or("missing PE data section")?;
        Ok(Self {
            data,
            layout,
            data_virtual_len,
        })
    }
}

#[cfg(test)]
pub(crate) fn synthetic_image(layout: Layout) -> Vec<u8> {
    let (length, timestamp, image_size, data_rva) = match layout {
        Layout::DebugReference => (1_650_688, 977_354_863, 2_506_752, 1_413_120),
        Layout::Retail => (1_630_208, 993_669_874, 2_498_560, 1_392_640),
        Layout::Original => (1_634_304, 975_611_382, 2_490_368, 1_396_736),
    };
    let mut bytes = vec![0; length];
    bytes[..2].copy_from_slice(b"MZ");
    let put =
        |bytes: &mut [u8], at, value: u32| bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
    put(&mut bytes, 60, 128);
    bytes[128..132].copy_from_slice(b"PE\0\0");
    bytes[132..134].copy_from_slice(&0x14cu16.to_le_bytes());
    bytes[134..136].copy_from_slice(&4u16.to_le_bytes());
    put(&mut bytes, 136, timestamp);
    bytes[148..150].copy_from_slice(&224u16.to_le_bytes());
    bytes[152..154].copy_from_slice(&0x10bu16.to_le_bytes());
    put(&mut bytes, 180, 0x400000);
    put(&mut bytes, 208, image_size);
    // Arbitrary synthetic sections, with a deliberately relocated raw table.
    for (i, (rva, raw, len)) in [
        (4096, 1024, 512),
        (8192, 1536, 512),
        (data_rva, 4096, 200_704),
        (image_size - 4096, 204_800, 512),
    ]
    .into_iter()
    .enumerate()
    {
        let at = 376 + i * 40;
        put(&mut bytes, at + 8, len);
        put(&mut bytes, at + 12, rva);
        put(&mut bytes, at + 16, len);
        put(&mut bytes, at + 20, raw);
        if i == 2 {
            bytes[at..at + 8].copy_from_slice(b".data\0\0\0");
            put(&mut bytes, at + 36, 0xc0000040);
        }
    }
    bytes
}
