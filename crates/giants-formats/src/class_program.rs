//! Bounded class command framing. This is not class/state callback execution.
use crate::{Error, slice, u32_at};
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
fn string_end(data: &[u8], at: usize) -> Result<usize, Error> {
    let tail = data.get(at..).ok_or(Error("class string outside input"))?;
    let end = tail
        .iter()
        .take(128)
        .position(|b| *b == 0)
        .ok_or(Error("unterminated class string"))?;
    if !tail[..end].is_ascii() {
        return Err(Error("non-ASCII class string"));
    }
    Ok(at + end + 1)
}
/// Input starts after the class identifier; end at FF without scanning past an
/// unknown opcode. Commands are supported by framing, not assumed implemented.
pub fn parse(data: &[u8]) -> Result<Program, Error> {
    if data.len() > 4 * 1024 * 1024 {
        return Err(Error("class input budget"));
    }
    let mut at = 0;
    let mut records = Vec::new();
    while at < data.len() {
        if records.len() >= 4096 {
            return Err(Error("class command budget"));
        }
        let start = at;
        let opcode = data[at];
        at = match opcode {
            0x02 => string_end(data, start + 5)?,
            0x7b | 0x7c => string_end(data, start + 1)?,
            0x57 => string_end(data, start + 1)? + 4,
            0x85 => string_end(data, start + 1)? + 12,
            0x86 => string_end(data, start + 5)? + 8,
            0x87 => string_end(data, start + 1)? + 8,
            0x44 => {
                let count = u32_at(data, start + 1)? as usize;
                if count > 1024 {
                    return Err(Error("class inline word budget"));
                }
                start + 5 + count * 4
            }
            0x01
            | 0x05..=0x07
            | 0x09..=0x0b
            | 0x0d
            | 0x15
            | 0x17
            | 0x19
            | 0x1a
            | 0x25
            | 0x29
            | 0x31
            | 0x32
            | 0x37
            | 0x39
            | 0x3b
            | 0x45
            | 0x69
            | 0x7a => start + 5,
            0x04 | 0x08 | 0x1c | 0x21 | 0x26 | 0x33..=0x35 => start + 9,
            0x03 | 0x38 | 0x88 => start + 13,
            0x1f | 0x28 | 0x3c | 0x82 => start + 17,
            0x81 => start + 21,
            0x13 | 0x1e | 0x36 => start + 2,
            0x14
            | 0x16
            | 0x18
            | 0x1b
            | 0x1d
            | 0x20
            | 0x22
            | 0x23
            | 0x2a
            | 0x2b
            | 0x2e
            | 0x2f
            | 0x3d
            | 0x41
            | 0x43
            | 0x46
            | 0x49
            | 0x4d
            | 0x4e
            | 0x4f..=0x55
            | 0x7d..=0x7f
            | 0x83
            | 0x84
            | 0xff => start + 1,
            _ => return Err(Error("unsupported class opcode")),
        };
        let bytes = slice(data, start, at - start)?.to_vec();
        records.push(Record {
            offset: start,
            opcode,
            bytes,
        });
        if opcode == 0xff {
            return Ok(Program {
                records,
                consumed: at,
            });
        }
    }
    Err(Error("missing class terminator"))
}
impl Program {
    /// Ordered explicit center/height declarations. A later model binding
    /// invalidates an earlier declaration: its derived defaults need that model.
    /// Callbacks and model-derived dimensions are not inferred here.
    pub fn declared_collision_dimensions(&self) -> Result<Option<[f32; 2]>, Error> {
        let mut dimensions = None;
        for record in &self.records {
            match record.opcode {
                1 => dimensions = None,
                0x21 => {
                    let value = [
                        f32::from_bits(u32_at(&record.bytes, 1)?),
                        f32::from_bits(u32_at(&record.bytes, 5)?),
                    ];
                    if !value.into_iter().all(f32::is_finite) {
                        return Err(Error("non-finite collision dimensions"));
                    }
                    dimensions = Some(value);
                }
                _ => {}
            }
        }
        Ok(dimensions)
    }
    /// Ordered direct scale assignments only. State callbacks are not executed.
    pub fn declared_scale(&self) -> Result<f32, Error> {
        let mut scale = 1.;
        for record in &self.records {
            if record.opcode == 0x17 {
                scale = f32::from_bits(u32_at(&record.bytes, 1)?);
            }
        }
        if !scale.is_finite() || scale <= 0. {
            return Err(Error("invalid declared class scale"));
        }
        Ok(scale)
    }
    /// Direct ground-offset declarations only. Model-derived offsets need the
    /// bound model and remain explicit; no callback is executed by this accessor.
    pub fn declared_ground_offset(&self) -> Result<f32, Error> {
        let mut offset = 0.;
        for record in &self.records {
            match record.opcode {
                0x15 => offset = f32::from_bits(u32_at(&record.bytes, 1)?),
                0x3a => return Err(Error("ground offset requires bound model")),
                _ => {}
            }
        }
        if !offset.is_finite() {
            return Err(Error("invalid declared ground offset"));
        }
        Ok(offset)
    }
    /// Direct spec assignments can occur beyond the first command. Their runtime
    /// effects and callback interactions are not inferred by this accessor.
    pub fn declared_specs(&self) -> Vec<u32> {
        self.records
            .iter()
            .filter(|r| r.opcode == 1)
            .map(|r| u32_at(&r.bytes, 1).unwrap())
            .collect()
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn command84_retains_its_byte_without_consuming_the_next_declaration() {
        let program = parse(&[0x84, 0x13, 7, 0xff, 0x90]).unwrap();
        assert_eq!(program.consumed, 4);
        assert_eq!(program.records[0].bytes, [0x84]);
        assert_eq!(program.records[1].offset, 1);
        assert_eq!(program.records[1].bytes, [0x13, 7]);
        assert!(parse(&[0x84]).is_err());
        assert!(parse(&[0x84, 0x90, 0xff]).is_err());
    }
    #[test]
    fn later_model_binding_invalidates_explicit_collision_dimensions() {
        let declaration = [
            vec![0x21],
            8_f32.to_le_bytes().to_vec(),
            10_f32.to_le_bytes().to_vec(),
        ]
        .concat();
        let binding = [vec![1], 2_u32.to_le_bytes().to_vec()].concat();
        let overridden = [declaration.clone(), binding.clone(), vec![255]].concat();
        assert_eq!(
            parse(&overridden)
                .unwrap()
                .declared_collision_dimensions()
                .unwrap(),
            None
        );
        let explicit = [binding, declaration, vec![255]].concat();
        assert_eq!(
            parse(&explicit)
                .unwrap()
                .declared_collision_dimensions()
                .unwrap(),
            Some([8., 10.])
        );
    }
    use super::*;
    #[test]
    fn variable_commands_unknowns_and_later_scale_assignments() {
        let mut bytes = vec![1];
        bytes.extend(7u32.to_le_bytes());
        bytes.push(0x17);
        bytes.extend(2f32.to_le_bytes());
        bytes.extend(b"\x86\0\0\0\0symbol\0");
        bytes.extend([0; 8]);
        bytes.push(0x44);
        bytes.extend(2u32.to_le_bytes());
        bytes.extend([0; 8]);
        bytes.push(0x17);
        bytes.extend(3f32.to_le_bytes());
        bytes.push(0x4e);
        bytes.push(0xff);
        let program = parse(&bytes).unwrap();
        assert_eq!(program.declared_scale().unwrap(), 3.);
        assert_eq!(program.declared_specs(), [7]);
        for n in 0..bytes.len() {
            assert!(parse(&bytes[..n]).is_err());
        }
        let mut trailing = bytes.clone();
        trailing.extend([0x17, 0, 0]);
        assert_eq!(parse(&trailing).unwrap().consumed, bytes.len());
        bytes[10] = 0x90;
        assert!(parse(&bytes).is_err());
        assert!(parse(&[0x44, 0xff, 0xff, 0xff, 0xff]).is_err());
    }
}
