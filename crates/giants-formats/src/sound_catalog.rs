//! Bounded versions 0x33221101/02 external-file sound definitions in a BIN catalog.
//! Mixer, attenuation and pitch policies are not interpreted here.
use crate::{Error, slice, u32_at};

#[derive(Debug, PartialEq)]
pub struct Definition {
    pub id: u32,
    pub words: [u32; 9],
    pub filename: String,
}
#[derive(Debug, PartialEq)]
pub struct Binding {
    pub filename: String,
    pub descriptor: [u32; 6],
}
/// Prepare the measured fresh-table policy assuming successful payload reads.
/// Repeated IDs keep the first payload name and replace descriptor fields. This
/// does not load audio, create native handles or establish catalog load order.
pub fn prepare_fresh<'a>(
    definitions: impl IntoIterator<Item = &'a Definition>,
    alternate: bool,
) -> std::collections::BTreeMap<u32, Binding> {
    let mut result = std::collections::BTreeMap::new();
    for definition in definitions {
        let entry = result.entry(definition.id).or_insert_with(|| Binding {
            filename: definition.filename.clone(),
            descriptor: definition.descriptor_words(alternate),
        });
        entry.descriptor = definition.descriptor_words(alternate);
    }
    result
}
impl Definition {
    /// Six source words routed into the measured runtime descriptor. The
    /// alternate pair is present only in version 02; version 01 repeats the
    /// primary pair so that the dialog query cannot change its descriptor.
    pub fn descriptor_words(&self, alternate: bool) -> [u32; 6] {
        [
            self.words[1] | u32::from(self.id == 0x692),
            self.words[2],
            self.words[3],
            self.words[4],
            self.words[if alternate { 7 } else { 5 }],
            self.words[if alternate { 8 } else { 6 }],
        ]
    }
}
pub fn parse_catalog(data: &[u8]) -> Result<Vec<Definition>, Error> {
    if u32_at(data, 0)? != 0x1a0002e5 {
        return Err(Error("unsupported sound catalog"));
    }
    let at = u32_at(data, 12)? as usize;
    let header = slice(data, at, 20)?;
    let (stride, name_word) = match u32_at(header, 4)? {
        0x33221101 => (32, 7),
        0x33221102 => (40, 9),
        _ => return Err(Error("unsupported external sound version")),
    };
    let count = u32_at(header, 8)? as usize;
    let size = u32_at(header, 16)? as usize;
    if u32_at(header, 0)? != u32::MAX
        || u32_at(header, 12)? as usize != stride
        || count > 2000
        || size < 20 + count * stride
    {
        return Err(Error("unsupported external sound section"));
    }
    let section = slice(data, at, size)?;
    let mut result = Vec::new();
    for index in 0..count {
        let entry = slice(section, 20 + index * stride, stride)?;
        let id = u32_at(entry, 0)?;
        if id >= 2000 {
            return Err(Error("sound id outside native table"));
        }
        let name = u32_at(entry, name_word * 4)? as usize;
        if name < 20 + count * stride {
            return Err(Error("sound filename overlaps table"));
        }
        let tail = section
            .get(name..)
            .ok_or(Error("sound filename outside section"))?;
        let end = tail
            .iter()
            .take(128)
            .position(|b| *b == 0)
            .ok_or(Error("unterminated sound filename"))?;
        let raw = &tail[..end];
        if raw.is_empty()
            || !raw.is_ascii()
            || raw
                .iter()
                .any(|b| !b.is_ascii_alphanumeric() && !b"_.".contains(b))
            || !raw.to_ascii_lowercase().ends_with(b".wav")
        {
            return Err(Error("unsupported sound filename"));
        }
        result.push(Definition {
            id,
            words: std::array::from_fn(|i| {
                if i < name_word {
                    u32_at(entry, i * 4).unwrap()
                } else {
                    u32_at(entry, (i - 2) * 4).unwrap()
                }
            }),
            filename: String::from_utf8(raw.to_vec()).unwrap(),
        });
    }
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Vec<u8> {
        let mut data = vec![0; 32];
        data[..4].copy_from_slice(&0x1a0002e5u32.to_le_bytes());
        data[12..16].copy_from_slice(&32u32.to_le_bytes());
        for v in [u32::MAX, 0x33221102, 1, 40, 68] {
            data.extend(v.to_le_bytes());
        }
        for v in [
            1626u32,
            0,
            0,
            100f32.to_bits(),
            0,
            20f32.to_bits(),
            0,
            20f32.to_bits(),
            0,
            60,
        ] {
            data.extend(v.to_le_bytes());
        }
        data.extend(b"cue.wav\0");
        data
    }
    #[test]
    fn sound_offsets_are_relative_to_section_and_bounded() {
        let bytes = fixture();
        assert_eq!(parse_catalog(&bytes).unwrap()[0].filename, "cue.wav");
        for end in 0..bytes.len() {
            assert!(parse_catalog(&bytes[..end]).is_err());
        }
        for value in [0u32, 59, 68, u32::MAX] {
            let mut bad = bytes.clone();
            bad[88..92].copy_from_slice(&value.to_le_bytes());
            assert!(parse_catalog(&bad).is_err());
        }
    }
    #[test]
    fn version_one_has_no_alternate_pair_and_preserves_filename_boundary() {
        let mut data = fixture();
        data.drain(80..88);
        data[36..40].copy_from_slice(&0x33221101u32.to_le_bytes());
        data[44..48].copy_from_slice(&32u32.to_le_bytes());
        data[48..52].copy_from_slice(&60u32.to_le_bytes());
        data[80..84].copy_from_slice(&52u32.to_le_bytes());
        let d = parse_catalog(&data).unwrap();
        assert_eq!(d[0].filename, "cue.wav");
        assert_eq!(d[0].words[7..], [20f32.to_bits(), 0]);
        assert_eq!(d[0].descriptor_words(true), d[0].descriptor_words(false));
        assert_eq!(d[0].descriptor_words(false)[4], 20f32.to_bits());
        let mut special = Definition {
            id: 0x692,
            words: [0; 9],
            filename: "x.wav".into(),
        };
        assert_eq!(special.descriptor_words(false)[0], 1);
        special.id = 1;
        assert_eq!(special.descriptor_words(false)[0], 0);
    }
    #[test]
    fn duplicate_id_keeps_first_audio_and_last_descriptor() {
        let first = Definition {
            id: 5,
            words: [1; 9],
            filename: "first.wav".into(),
        };
        let last = Definition {
            id: 5,
            words: [2; 9],
            filename: "second.wav".into(),
        };
        let bank = prepare_fresh([&first, &last], false);
        assert_eq!(bank.len(), 1);
        assert_eq!(bank[&5].filename, "first.wav");
        assert_eq!(bank[&5].descriptor, [2; 6]);
    }
}
