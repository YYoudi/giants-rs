//! Cold Kabuto animation definitions from a user-supplied PE data section.
//! No native instructions are executed or copied. Layout recognition is not
//! cryptographic authentication; the original executable is never bundled.
//! This prepares immutable cold data, not the source's process-global reset hook.
use crate::specification;
use std::{io::Read, path::Path, sync::Arc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Definition {
    pub animation: u8,
    pub lower: i32,
    pub upper: i32,
    pub reference: i32,
    pub flags: u32,
    /// Opaque six-word external prefix, not a complete parameter schema,
    /// callable address or interpreted behavior.
    pub parameters: Option<[u32; 6]>,
}
#[derive(Debug)]
pub struct Definitions {
    entries: Vec<Definition>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub definition: Definition,
    /// Serialized ANM offset, never an original process pointer.
    pub node: Option<usize>,
}
pub struct ColdCache {
    specification: Arc<specification::Prepared>,
    entries: Vec<Entry>,
}
impl ColdCache {
    pub fn specification(&self) -> &Arc<specification::Prepared> {
        &self.specification
    }
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }
}
const MAX_IMAGE: usize = 4 * 1024 * 1024;
fn word(bytes: &[u8], at: usize) -> Result<u32, String> {
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
struct DataSection<'a> {
    bytes: &'a [u8],
    rva: u32,
    base: u32,
}
impl DataSection<'_> {
    fn data(&self, rva: u32, len: usize) -> Result<&[u8], String> {
        let start = rva.checked_sub(self.rva).ok_or("data RVA below section")? as usize;
        self.bytes
            .get(start..start.checked_add(len).ok_or("data range overflow")?)
            .ok_or_else(|| "data outside initialized PE section".into())
    }
    fn definitions(&self, table: u32, count: usize) -> Result<Definitions, String> {
        let rows = self.data(table, (count + 1).checked_mul(28).ok_or("table overflow")?)?;
        if word(rows, count * 28)? != 254 {
            return Err("missing Kabuto animation sentinel".into());
        }
        let mut entries = Vec::with_capacity(count);
        for row in rows[..count * 28].as_chunks::<28>().0 {
            let slot = word(row, 0)?;
            if slot > 255
                || slot == 254
                || entries
                    .iter()
                    .any(|e: &Definition| u32::from(e.animation) == slot)
            {
                return Err("invalid or duplicate Kabuto animation slot".into());
            }
            if word(row, 24)? != 0 {
                return Err("serialized animation cache must be cold".into());
            }
            let pointer = word(row, 20)?;
            let parameters = if pointer == 0 {
                None
            } else {
                let rva = pointer
                    .checked_sub(self.base)
                    .ok_or("parameter VA below image")?;
                let data = self.data(rva, 24)?;
                let mut values = [0; 6];
                for (i, value) in values.iter_mut().enumerate() {
                    *value = word(data, i * 4)?;
                }
                Some(values)
            };
            entries.push(Definition {
                animation: slot as u8,
                lower: word(row, 4)? as i32,
                upper: word(row, 8)? as i32,
                reference: word(row, 12)? as i32,
                flags: word(row, 16)?,
                parameters,
            });
        }
        Ok(Definitions { entries })
    }
}
impl Definitions {
    /// Explicit configurable file input; no search of archives or native launch.
    pub fn read(path: &Path) -> Result<Self, String> {
        let mut bytes = Vec::new();
        std::fs::File::open(path)
            .map_err(|e| e.to_string())?
            .take((MAX_IMAGE + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        Self::decode(&bytes)
    }
    pub fn entries(&self) -> &[Definition] {
        &self.entries
    }
    /// Three independently inspected PC metadata layouts. Header tuples select
    /// the data layout only; modified files are not authenticated by this API.
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
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
        let (table, expected_rva) = match (bytes.len(), word(bytes, pe + 8)?, word(bytes, pe + 80)?)
        {
            (1_650_688, 977_354_863, 2_506_752) => (0x160680, 1_413_120),
            (1_630_208, 993_669_874, 2_498_560) => (0x15a4d8, 1_392_640),
            (1_634_304, 975_611_382, 2_490_368) => (0x15c5f8, 1_396_736),
            _ => return Err("unacquired Kabuto executable metadata layout".into()),
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
                found = Some(DataSection {
                    bytes: &bytes[raw..end],
                    rva,
                    base,
                });
            }
        }
        found
            .ok_or("missing PE data section")?
            .definitions(table, 62)
    }
    /// Atomic immutable cold preparation, reusing the shared reference resolver.
    /// Missing animation slots retain their source definition. A declared slot
    /// whose reference cannot resolve refuses preparation; no fallback node.
    pub fn prepare(
        &self,
        specification: Arc<specification::Prepared>,
    ) -> Result<ColdCache, String> {
        if specification.class != 1 || specification.specification != 1 {
            return Err("Kabuto animation cache requires class/specification1".into());
        }
        let mut caches = std::collections::BTreeMap::new();
        let mut entries = Vec::with_capacity(self.entries.len());
        for definition in &self.entries {
            let mut entry = Entry {
                definition: definition.clone(),
                node: None,
            };
            if let Some(descriptor) = specification.animations.get(&definition.animation)
                && definition.reference >= 0
            {
                let cache = caches
                    .entry(definition.animation)
                    .or_insert_with(|| vec![None; specification.metadata.references.names.len()]);
                entry.node = specification
                    .metadata
                    .references
                    .resolve_cached(&descriptor.animation, cache, definition.reference as usize)
                    .map_err(|e| e.to_string())?
                    .node();
                if entry.node.is_none() {
                    return Err("Kabuto animation reference unavailable".into());
                }
                clamp(&mut entry.definition, descriptor.animation.timeline);
            }
            entries.push(entry);
        }
        Ok(ColdCache {
            specification,
            entries,
        })
    }
}
fn clamp(definition: &mut Definition, timeline: [i32; 4]) {
    definition.lower = definition.lower.max(timeline[0]);
    definition.upper = definition.upper.min(timeline[1].wrapping_sub(1));
}

#[cfg(test)]
mod tests {
    use super::*;
    fn synthetic_image() -> Vec<u8> {
        let mut bytes = vec![0; 1_650_688];
        bytes[..2].copy_from_slice(b"MZ");
        let put = |bytes: &mut [u8], at, value: u32| {
            bytes[at..at + 4].copy_from_slice(&value.to_le_bytes())
        };
        put(&mut bytes, 60, 128);
        bytes[128..132].copy_from_slice(b"PE\0\0");
        bytes[132..134].copy_from_slice(&0x14cu16.to_le_bytes());
        bytes[134..136].copy_from_slice(&4u16.to_le_bytes());
        put(&mut bytes, 136, 977_354_863);
        bytes[148..150].copy_from_slice(&224u16.to_le_bytes());
        bytes[152..154].copy_from_slice(&0x10bu16.to_le_bytes());
        put(&mut bytes, 180, 0x400000);
        put(&mut bytes, 208, 2_506_752);
        // Arbitrary synthetic sections, with a deliberately relocated raw table.
        for (i, (rva, raw, len)) in [
            (4096, 1024, 512),
            (8192, 1536, 512),
            (1_413_120, 4096, 200_704),
            (2_469_888, 204_800, 512),
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
        let table = 4096 + 0x160680 - 1_413_120;
        for i in 0..62 {
            put(&mut bytes, table + i * 28, i as u32);
            put(&mut bytes, table + i * 28 + 12, u32::MAX);
        }
        put(&mut bytes, table + 62 * 28, 254);
        bytes
    }
    #[test]
    fn external_pe_table_uses_file_mapping_and_refuses_malformed_sections() {
        let valid = synthetic_image();
        assert_eq!(Definitions::decode(&valid).unwrap().entries().len(), 62);
        for (at, value) in [
            (136, 0),                        // Unknown layout.
            (376 + 2 * 40 + 36, 0xe0000040), // Executable data.
            (376 + 2 * 40 + 20, u32::MAX),   // Truncated raw section.
            (376 + 3 * 40 + 20, 4096),       // Aliased file ranges.
            (376 + 3 * 40 + 12, 1_413_120),  // Aliased RVAs.
            (4096 + 0x160680 - 1_413_120 + 62 * 28, 253),
            (4096 + 0x160680 - 1_413_120 + 28, 0), // Duplicate slot.
            (4096 + 0x160680 - 1_413_120 + 20, 0x401000), // Parameter outside data.
        ] {
            let mut bytes = valid.clone();
            bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
            assert!(
                Definitions::decode(&bytes).is_err(),
                "accepted mutation at {at}"
            );
        }
    }
    #[test]
    fn data_mapping_uses_rva_and_preserves_signed_ranges_and_opaque_data() {
        // Synthetic data only: raw file offsets deliberately differ from RVAs.
        let mut data = vec![0; 112];
        let put = |data: &mut [u8], at, v: u32| data[at..at + 4].copy_from_slice(&v.to_le_bytes());
        for (i, v) in [7, (-20i32) as u32, 4, (-1i32) as u32, 9, 0x402050, 0]
            .into_iter()
            .enumerate()
        {
            put(&mut data, i * 4, v);
        }
        put(&mut data, 28, 254);
        put(&mut data, 80, 123);
        let section = DataSection {
            bytes: &data,
            rva: 0x2000,
            base: 0x400000,
        };
        let defs = section.definitions(0x2000, 1).unwrap();
        let mut definition = defs.entries[0].clone();
        assert_eq!(definition.reference, -1);
        assert_eq!(definition.parameters.unwrap()[0], 123);
        clamp(&mut definition, [8, 40, 0, 0]);
        assert_eq!((definition.lower, definition.upper), (8, 4));
        assert_eq!(definition.flags, 9);
        assert!(section.definitions(0x1fff, 1).is_err());
        assert!(section.data(u32::MAX, 24).is_err());
        put(&mut data, 24, 1);
        assert!(
            DataSection {
                bytes: &data,
                rva: 0x2000,
                base: 0x400000
            }
            .definitions(0x2000, 1)
            .is_err()
        );
    }
    #[test]
    fn malformed_external_images_refuse_without_panicking() {
        for len in [0, 1, 2, 64, 256, 512, MAX_IMAGE + 1] {
            let mut data = vec![0; len];
            if len >= 2 {
                data[..2].copy_from_slice(b"MZ");
            }
            if len >= 64 {
                data[60..64].copy_from_slice(&u32::MAX.to_le_bytes());
            }
            assert!(Definitions::decode(&data).is_err());
        }
    }
}
