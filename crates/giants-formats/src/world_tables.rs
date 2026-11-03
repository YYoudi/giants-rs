//! Serialized cinematic/mission tables, without terrain preparation or callbacks.
use crate::{Error, slice, u32_at, world::WorldCinematic};

pub struct Tables {
    pub records: Vec<Record>,
    pub cinematics: Vec<WorldCinematic>,
    pub mission_slots: Vec<(usize, String)>,
    pub scenarios: Vec<Scenario>,
    pub block_end: usize,
    pub trailing_bytes: usize,
}
pub struct Record {
    pub offset: usize,
    pub opcode: u8,
    pub length: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scenario {
    pub source_offset: usize,
    /// Native dispatch: 1 cinematic, 2 mission, 3 world transition, 4 menu.
    /// Other values are preserved without assigning execution semantics.
    pub kind: u8,
    pub parameter: u32,
    pub name: String,
}
fn name(data: &[u8], at: usize, limit: usize) -> Result<(String, usize), Error> {
    let raw = data
        .get(at..)
        .ok_or(Error("world table string outside block"))?;
    let end = raw
        .iter()
        .take(limit)
        .position(|b| *b == 0)
        .ok_or(Error("unterminated world table string"))?;
    if !raw[..end].is_ascii() {
        return Err(Error("non-ASCII world table string"));
    }
    Ok((
        String::from_utf8(raw[..end].to_vec()).unwrap(),
        at + end + 1,
    ))
}
/// Shared bounded framing only; payload consumers remain separate.
pub(crate) fn record_length(data: &[u8], at: usize) -> Result<usize, Error> {
    let opcode = *slice(data, at, 1)?.first().unwrap();
    let length = match opcode {
        89 | 118 | 132 => name(data, at + 1, 128)?.1 - at + if opcode == 118 { 20 } else { 0 },
        103 | 104 => {
            let count = u32_at(data, at + 1)?;
            if count > 20 {
                return Err(Error("world table array outside checked budget"));
            }
            5 + count as usize * 4
        }
        23 | 73 | 82 | 83 | 96 | 107 | 117 => 5,
        29..=38 => 17,
        39 => 18,
        40 => 4,
        41 | 88 => 12,
        42 => 21,
        44 | 78 | 93 | 130 => 33,
        46 | 90 | 91 | 95 | 101 => 13,
        48 => 19,
        50 => 3,
        52..=54 | 114..=116 => 45,
        57 | 70 => 29,
        59 => 2,
        68 | 69 | 74 | 77 | 255 => 1,
        76 => 7,
        79 | 108..=111 => 65,
        92 => 38,
        102 => 9,
        119 | 120 => 49,
        121 => 93,
        _ => return Err(Error("unsupported world table opcode")),
    };
    slice(data, at, length)?;
    Ok(length)
}
pub fn parse(data: &[u8]) -> Result<Tables, Error> {
    slice(data, 0, 32)?;
    if u32_at(data, 0)? != 0x1a0002e5 {
        return Err(Error("unsupported world table version"));
    }
    let start = u32_at(data, 4)? as usize;
    if start < 32 {
        return Err(Error("world table block overlaps header"));
    }
    let size = u32_at(data, start)? as usize;
    if size > 16 * 1024 * 1024 {
        return Err(Error("world table block budget"));
    }
    slice(data, start + 4, size)?;
    let end = start + 4 + size;
    let data = &data[..end];
    let (_, at) = name(data, start + 4, 128)?;
    let (_, mut at) = name(data, at, 128)?;
    let mut result = Tables {
        records: Vec::new(),
        cinematics: Vec::new(),
        mission_slots: Vec::new(),
        scenarios: Vec::new(),
        block_end: end,
        trailing_bytes: 0,
    };
    while at < end {
        if result.records.len() >= 8192 {
            return Err(Error("world table command budget"));
        }
        let opcode = data[at];
        let length = record_length(data, at)?;
        slice(data, at, length)?;
        result.records.push(Record {
            offset: at,
            opcode,
            length,
        });
        if opcode == 79 {
            if result.cinematics.len() >= 128 {
                return Err(Error("world cinematic capacity"));
            }
            let (label, _) = name(&data[..at + 33], at + 1, 32)?;
            let (metadata, _) = name(&data[..at + 65], at + 33, 32)?;
            result.cinematics.push(WorldCinematic {
                source_offset: at,
                name: label,
                metadata,
            });
        } else if opcode == 92 {
            if result.scenarios.len() >= 27 {
                return Err(Error("world scenario capacity"));
            }
            let (label, _) = name(&data[..at + 38], at + 6, 32)?;
            result.scenarios.push(Scenario {
                source_offset: at,
                kind: data[at + 1],
                parameter: u32_at(data, at + 2)?,
                name: label,
            });
        } else if opcode == 93 {
            if result.mission_slots.len() >= 32 {
                return Err(Error("world mission capacity"));
            }
            let (label, _) = name(&data[..at + 33], at + 1, 32)?;
            if label.is_empty() {
                return Err(Error("empty world mission slot"));
            }
            result.mission_slots.push((at, label));
        }
        at += length;
        if opcode == 255 {
            result.trailing_bytes = end - at;
            return Ok(result);
        }
    }
    Err(Error("missing world table terminator"))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn input(block: &[u8]) -> Vec<u8> {
        let mut bytes = vec![0; 36];
        bytes[..4].copy_from_slice(&0x1a0002e5u32.to_le_bytes());
        bytes[4..8].copy_from_slice(&32u32.to_le_bytes());
        bytes[32..36].copy_from_slice(&(block.len() as u32).to_le_bytes());
        bytes.extend_from_slice(block);
        bytes
    }
    #[test]
    fn tables_do_not_require_terrain_and_ignore_opcodes_inside_opaque_payloads() {
        let mut block = b"box\0terrain\0".to_vec();
        block.push(110);
        block.extend([79; 64]);
        block.push(79);
        block.extend([0; 64]);
        let end = block.len();
        block[end - 64..end - 59].copy_from_slice(b"movie");
        block.push(255);
        let bytes = input(&block);
        let tables = parse(&bytes).unwrap();
        assert_eq!(tables.cinematics.len(), 1);
        assert_eq!(tables.cinematics[0].name, "movie");
        assert!(crate::world::parse_world(&bytes).is_err());
        for n in 0..bytes.len() {
            assert!(parse(&bytes[..n]).is_err());
        }
    }
    #[test]
    fn rejects_unknown_records_negative_array_lengths_and_unterminated_names() {
        for block in [
            b"a\0b\0\x01\xff".to_vec(),
            [b"a\0b\0\x67".as_slice(), &[255; 4], &[255]].concat(),
            [b"a\0b\0\x4f".as_slice(), &[65; 64], &[255]].concat(),
        ] {
            assert!(parse(&input(&block)).is_err());
        }
    }
    #[test]
    fn scenario_fields_preserve_order_offsets_and_unknown_kinds() {
        let mut block = b"a\0b\0".to_vec();
        for kind in [1, 2, 3, 4, 255] {
            block.extend([92, kind]);
            block.extend(0x12345678u32.to_le_bytes());
            block.extend([0; 32]);
        }
        block.push(255);
        let parsed = parse(&input(&block)).unwrap();
        assert_eq!(parsed.scenarios.len(), 5);
        assert_eq!(parsed.scenarios[4].kind, 255);
        assert_eq!(parsed.scenarios[0].parameter, 0x12345678);
        assert_eq!(parsed.scenarios[0].source_offset, 40);
        assert!(parsed.scenarios[0].name.is_empty());
        let at = 4 + 6;
        block[at..at + 32].fill(b'x');
        assert!(parse(&input(&block)).is_err());
    }
}
