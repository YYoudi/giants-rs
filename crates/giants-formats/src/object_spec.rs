//! Bounded object-spec records. Framing is separate from animation/FX execution.
use crate::{Error, slice, u32_at};

#[derive(Clone, Debug)]
pub enum Kind {
    Model {
        reference: u32,
        angular: Option<[u32; 2]>,
    },
    Animation {
        slot: u8,
        name: String,
    },
    Effects {
        names: Vec<String>,
    },
    Data,
    End,
}
#[derive(Clone, Debug)]
pub struct Record {
    pub offset: usize,
    pub opcode: u8,
    pub kind: Kind,
    pub bytes: Vec<u8>,
}
#[derive(Clone, Debug)]
pub struct Program {
    pub records: Vec<Record>,
    pub consumed: usize,
}
impl Program {
    /// Ordered command53 bindings are indexed slots, not ANM tree order.
    pub fn reference_names(&self) -> Result<Vec<Option<String>>, Error> {
        let mut names = Vec::new();
        for r in &self.records {
            if r.opcode != 0x53 {
                continue;
            }
            let slot = usize::from(*r.bytes.get(1).ok_or(Error("truncated reference slot"))?);
            if slot >= 128 {
                return Err(Error("reference slot exceeds supported budget"));
            }
            let raw = slice(&r.bytes, 2, 16)?;
            let end = raw
                .iter()
                .position(|b| *b == 0)
                .ok_or(Error("unterminated reference name"))?;
            if !raw[..end].is_ascii() {
                return Err(Error("non-ASCII reference name"));
            }
            names.resize(names.len().max(slot + 1), None);
            names[slot] = Some(String::from_utf8(raw[..end].to_vec()).unwrap());
        }
        Ok(names)
    }
}
fn string(data: &[u8], at: &mut usize) -> Result<String, Error> {
    let tail = data.get(*at..).ok_or(Error("spec string outside input"))?;
    let n = tail
        .iter()
        .take(128)
        .position(|v| *v == 0)
        .ok_or(Error("unterminated spec string"))?;
    if !tail[..n].is_ascii() {
        return Err(Error("non-ASCII spec string"));
    }
    let value = String::from_utf8(tail[..n].to_vec()).unwrap();
    *at += n + 1;
    Ok(value)
}
/// Input begins after the spec ID. Stop exactly at FF, retaining all preceding
/// records; bytes after the terminator are not interpreted or scanned for tags.
pub fn parse(data: &[u8]) -> Result<Program, Error> {
    if data.len() > 4 * 1024 * 1024 {
        return Err(Error("spec input budget"));
    }
    let mut at = 0;
    let mut records = Vec::new();
    let mut models = 0;
    while at < data.len() {
        if records.len() >= 4096 {
            return Err(Error("spec record budget"));
        }
        let start = at;
        let opcode = data[at];
        at += 1;
        let kind = match opcode {
            0x51 | 0x59..=0x5b => {
                if models >= 10 {
                    return Err(Error("spec model slot budget"));
                }
                models += 1;
                let reference = u32_at(data, at)?;
                let angular = if opcode == 0x51 {
                    Some([u32_at(data, at + 4)?, u32_at(data, at + 8)?])
                } else {
                    None
                };
                at += if angular.is_some() { 12 } else { 4 };
                Kind::Model { reference, angular }
            }
            0x52 => {
                let slot = *slice(data, at, 1)?.first().unwrap();
                at += 1;
                let name = string(data, &mut at)?;
                Kind::Animation { slot, name }
            }
            0x53 | 0x54 | 0x55 | 0x56 | 0x57 | 0x58 | 0x5c => {
                let size = match opcode {
                    0x53 => 18,
                    0x54 => 23,
                    0x55 => 1,
                    0x56 => 9,
                    _ => 13,
                };
                slice(data, start, size)?;
                at = start + size;
                Kind::Data
            }
            0x7f | 0x80 => {
                slice(data, at, 4)?;
                at += 4;
                let mut names = Vec::new();
                for _ in 0..5 {
                    names.push(string(data, &mut at)?);
                }
                slice(data, at, 4)?;
                at += 4;
                Kind::Effects { names }
            }
            0xff => Kind::End,
            _ => return Err(Error("unsupported spec opcode")),
        };
        records.push(Record {
            offset: start,
            opcode,
            kind,
            bytes: slice(data, start, at - start)?.to_vec(),
        });
        if opcode == 0xff {
            return Ok(Program {
                records,
                consumed: at,
            });
        }
    }
    Err(Error("missing spec terminator"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn model_slots_animation_and_effect_framing_are_bounded() {
        let mut data = vec![0x51];
        for word in [7u32, 60f32.to_bits(), 700f32.to_bits()] {
            data.extend(word.to_le_bytes());
        }
        data.push(0x5a);
        data.extend(8u32.to_le_bytes());
        data.extend(b"\x52\x01walk.anm\0\x7f\0\0\0\0a\0\0c\0d\0e\0\0\0\0\0\xff");
        let program = parse(&data).unwrap();
        assert_eq!(program.records.len(), 5);
        assert!(
            matches!(&program.records[2].kind, Kind::Animation { slot: 1, name } if name=="walk.anm")
        );
        assert!(
            matches!(&program.records[3].kind, Kind::Effects { names } if names.len()==5 && names[1].is_empty())
        );
        for n in 0..data.len() {
            assert!(parse(&data[..n]).is_err());
        }
        let mut padded = data.clone();
        padded.extend([0x52, 0, 0]);
        assert_eq!(parse(&padded).unwrap().consumed, data.len());
        let mut bad = data;
        bad[18] = 0x60;
        assert!(parse(&bad).is_err());
    }
}

#[cfg(test)]
mod reference_tests {
    use super::*;
    fn declaration(slot: u8, name: &[u8]) -> Vec<u8> {
        let mut row = vec![0x53, slot];
        let mut bytes = [0; 16];
        bytes[..name.len()].copy_from_slice(name);
        row.extend(bytes);
        row
    }
    #[test]
    fn named_slots_keep_holes_and_replace_prior_binding() {
        let mut bytes = declaration(2, b"first");
        bytes.extend(declaration(0, b"origin"));
        bytes.extend(declaration(2, b"last"));
        bytes.push(0xff);
        assert_eq!(
            parse(&bytes).unwrap().reference_names().unwrap(),
            vec![Some("origin".into()), None, Some("last".into())]
        );
    }
    #[test]
    fn malformed_or_outside_profile_bindings_are_rejected() {
        for mut bytes in [
            declaration(128, b"x"),
            declaration(0, &[b'x'; 16]),
            declaration(0, &[0x80]),
        ] {
            bytes.push(0xff);
            assert!(parse(&bytes).unwrap().reference_names().is_err());
        }
        let program = Program {
            records: vec![Record {
                offset: 0,
                opcode: 0x53,
                kind: Kind::Data,
                bytes: vec![0x53],
            }],
            consumed: 1,
        };
        assert!(program.reference_names().is_err());
    }
}
