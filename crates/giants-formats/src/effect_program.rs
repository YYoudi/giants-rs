//! Bounded effect command framing. No object/effect/animation execution.
use crate::{Error, slice};
#[derive(Clone, Debug)]
pub struct Record {
    pub offset: usize,
    pub opcode: u8,
    pub bytes: Vec<u8>,
}
#[derive(Clone, Debug)]
pub struct Program {
    pub records: Vec<Record>,
    pub consumed: usize,
}
/// Potential reference operands only; activation and execution are not inferred.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReferenceKey {
    Class(u32),
    Effect(String),
    AdaptiveEffect(String),
    Texture(String),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reference {
    pub offset: usize,
    pub opcode: u8,
    pub key: ReferenceKey,
}
impl Program {
    /// Only native-checked reference roles are classified. Other numeric/string
    /// operands remain opaque, so this is not a complete dependency closure.
    pub fn references(&self) -> Result<Vec<Reference>, Error> {
        let mut references = Vec::new();
        for record in &self.records {
            let key = match record.opcode {
                0x03 => ReferenceKey::Class(crate::u32_at(&record.bytes, 1)?),
                // Attached creation has an opaque first operand, then its class.
                0x24 => ReferenceKey::Class(crate::u32_at(&record.bytes, 5)?),
                0x7a => {
                    let end = string_end(&record.bytes, 1)?;
                    ReferenceKey::AdaptiveEffect(
                        String::from_utf8(record.bytes[1..end - 1].to_vec())
                            .map_err(|_| Error("invalid adaptive effect name"))?,
                    )
                }
                0x76 | 0x78 => {
                    let end = string_end(&record.bytes, 1)?;
                    ReferenceKey::Texture(
                        String::from_utf8(record.bytes[1..end - 1].to_vec())
                            .map_err(|_| Error("invalid effect texture name"))?,
                    )
                }
                0x0a | 0x0d | 0x10 | 0x1d | 0x1e | 0x27 | 0x28 | 0x56 | 0x57 | 0x5a | 0x5b
                | 0x5c | 0x70 | 0x71 => {
                    let start = if matches!(record.opcode, 0x0a | 0x10 | 0x70) {
                        5
                    } else {
                        1
                    };
                    let end = string_end(&record.bytes, start)?;
                    ReferenceKey::Effect(
                        String::from_utf8(record.bytes[start..end - 1].to_vec())
                            .map_err(|_| Error("invalid effect reference name"))?,
                    )
                }
                _ => continue,
            };
            references.push(Reference {
                offset: record.offset,
                opcode: record.opcode,
                key,
            });
        }
        Ok(references)
    }
}
fn string_end(data: &[u8], at: usize) -> Result<usize, Error> {
    let tail = data.get(at..).ok_or(Error("effect string outside input"))?;
    let end = tail
        .iter()
        .take(128)
        .position(|b| *b == 0)
        .ok_or(Error("unterminated effect string"))?;
    if !tail[..end].is_ascii() {
        return Err(Error("non-ASCII effect string"));
    }
    Ok(at + end + 1)
}
/// The tail starts at a dictionary's payload-relative script offset. Stop only
/// at command02, not the next named entry. Reject unknown commands, never resync.
/// Supported layouts reflect the checked source profile; numeric parameters and
/// runtime diagnostic conditions are deliberately not interpreted here.
pub fn parse(data: &[u8]) -> Result<Program, Error> {
    if data.len() > 16 * 1024 * 1024 {
        return Err(Error("effect input budget"));
    }
    let mut at = 0;
    let mut records = Vec::new();
    while at < data.len() {
        if records.len() >= 4096 {
            return Err(Error("effect command budget"));
        }
        let start = at;
        let opcode = data[at];
        at = match opcode {
            0x02 | 0x06 | 0x07 | 0x15 | 0x16 | 0x21 | 0x3e | 0x48 | 0x49 | 0x4a | 0x55 | 0x58
            | 0x59 | 0x84 | 0x85 => start + 1,
            0x03 | 0x0b | 0x0e | 0x11 | 0x17 | 0x19 | 0x1b | 0x25 | 0x26 | 0x2a | 0x2c | 0x2e
            | 0x30 | 0x38 | 0x3c | 0x47 | 0x5f | 0x67 | 0x69 => start + 5,
            0x08 | 0x0f | 0x12 | 0x13 | 0x18 | 0x1c | 0x22 | 0x29 | 0x2b | 0x34 | 0x36 | 0x3a
            | 0x40 | 0x6f | 0x79 | 0x80 | 0x83 | 0x87 => start + 9,
            0x04 | 0x24 | 0x32 | 0x39 => start + 13,
            0x09 | 0x35 | 0x37 | 0x88 => start + 17,
            0x46 | 0x7c | 0x81 => start + 21,
            0x05 | 0x20 | 0x33 | 0x3d | 0x74 => start + 25,
            0x0a => string_end(data, start + 5)? + 4,
            0x0d => string_end(data, start + 1)?,
            0x10 => string_end(data, start + 5)? + 4,
            0x1d => string_end(data, start + 1)? + 4,
            0x1e => string_end(data, start + 1)? + 8,
            0x27 => string_end(data, start + 1)? + 8,
            0x28 => string_end(data, start + 1)? + 16,
            0x56 => string_end(data, start + 1)? + 12,
            0x57 => string_end(data, start + 1)? + 24,
            0x5a => string_end(data, start + 1)? + 4,
            0x5b => string_end(data, start + 1)? + 12,
            0x5c => string_end(data, start + 1)? + 24,
            0x70 => string_end(data, start + 5)? + 4,
            0x71 => string_end(data, start + 1)? + 12,
            0x76 => string_end(data, start + 1)? + 8,
            0x78 => string_end(data, start + 1)? + 8,
            0x7a => string_end(data, start + 1)? + 24,
            _ => return Err(Error("unsupported effect opcode")),
        };
        let bytes = slice(data, start, at - start)?.to_vec();
        records.push(Record {
            offset: start,
            opcode,
            bytes,
        });
        if opcode == 2 {
            return Ok(Program {
                records,
                consumed: at,
            });
        }
    }
    Err(Error("missing effect end command"))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn attached_class_operand_is_distinct_from_spawn_metadata() {
        let mut bytes = vec![0x24];
        for word in [0x12345678u32, 794, 99] {
            bytes.extend(word.to_le_bytes());
        }
        bytes.push(2);
        let references = parse(&bytes).unwrap().references().unwrap();
        assert_eq!(references.len(), 1);
        assert_eq!(references[0].key, ReferenceKey::Class(794));
        assert_eq!(references[0].opcode, 0x24);
        assert_eq!(references[0].offset, 0);
    }
    #[test]
    fn bounds_unknown_commands_and_terminator_are_explicit() {
        let mut data = vec![3];
        data.extend(862u32.to_le_bytes());
        data.push(0x10);
        data.extend(1u32.to_le_bytes());
        data.extend(b"child\0");
        data.extend(2u32.to_le_bytes());
        data.extend([6, 7, 2, 0xfe]);
        let p = parse(&data).unwrap();
        assert_eq!(p.records.len(), 5);
        let references = p.references().unwrap();
        assert_eq!(references.len(), 2);
        assert_eq!(references[0].key, ReferenceKey::Class(862));
        assert_eq!(references[1].key, ReferenceKey::Effect("child".into()));
        assert_eq!(references[1].offset, 5);
        assert_eq!(p.consumed, data.len() - 1);
        for end in 0..p.consumed {
            assert!(parse(&data[..end]).is_err(), "{end}");
        }
        assert!(parse(&[0xfe, 2]).is_err());
        assert!(parse(&[0x0d, 255, 0, 2]).is_err());
        let mut long = vec![0x0d];
        long.extend([b'a'; 128]);
        long.extend([0, 2]);
        assert!(parse(&long).is_err());
        let mut budget = vec![6; 4096];
        budget.push(2);
        assert!(parse(&budget).is_err());
    }
}
