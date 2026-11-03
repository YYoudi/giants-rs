//! Record discovery only: no actor projection, callbacks or mission readiness.
use crate::{Error, slice, u32_at};
pub struct Record {
    pub offset: usize,
    pub opcode: u8,
    pub payload: Vec<u8>,
}
pub struct Metadata {
    pub records: Vec<Record>,
    /// Exact fixed payloads of commands19/21/25. Only21 has startup semantics.
    pub fixed_payloads: Vec<(usize, u8, [u8; 32])>,
    pub deferred_records: Vec<usize>,
}
pub fn deferred(opcode: u8) -> bool {
    matches!(opcode, 12 | 13 | 15 | 19 | 25 | 26 | 28 | 32 | 34 | 36 | 37)
}
pub fn parse(data: &[u8]) -> Result<Metadata, Error> {
    if !(6..=4 * 1024 * 1024).contains(&data.len()) || u32_at(data, 0)? != 4 || data[4] != 1 {
        return Err(Error("unsupported mission metadata framing"));
    }
    let mut result = Metadata {
        records: Vec::new(),
        fixed_payloads: Vec::new(),
        deferred_records: Vec::new(),
    };
    let mut at = 4;
    while at < data.len() {
        if result.records.len() >= 8192 {
            return Err(Error("mission metadata command budget"));
        }
        let opcode = data[at];
        let length = match opcode {
            1 if at == 4 => 1,
            2 | 23 | 28 | 34 | 36 | 37 => 1,
            3 => 21,
            4 => 29,
            5 | 8 | 32 => 2,
            6 | 24 => 13,
            7 | 9 | 12 | 13 | 15 | 16 | 17 | 33 => 5,
            10 => 2 + usize::from(slice(data, at + 1, 1)?[0]) * 4,
            11 => 17,
            14 | 30 | 31 => {
                let count = u32_at(data, at + 1)?;
                if count > 20 {
                    return Err(Error("mission metadata array budget"));
                }
                5 + count as usize * 4
            }
            19..=21 | 25 => 33,
            22 => 7,
            26 | 38 => 9,
            _ => return Err(Error("unsupported mission metadata opcode")),
        };
        let record = slice(data, at, length)?;
        result.records.push(Record {
            offset: at,
            opcode,
            payload: record[1..].to_vec(),
        });
        if matches!(opcode, 19 | 21 | 25) {
            result
                .fixed_payloads
                .push((at, opcode, record[1..33].try_into().unwrap()));
        }
        if deferred(opcode) {
            result.deferred_records.push(at);
        }
        at += length;
        if opcode == 2 {
            if at != data.len() {
                return Err(Error("trailing mission metadata bytes"));
            }
            return Ok(result);
        }
    }
    Err(Error("missing mission metadata terminator"))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn discovery_retains_opaque_bytes_but_never_weakens_full_reader() {
        let mut bytes = vec![4, 0, 0, 0, 1, 19];
        bytes.extend([0x15; 32]);
        bytes.push(2);
        let m = parse(&bytes).unwrap();
        assert_eq!(m.fixed_payloads, [(5, 19, [0x15; 32])]);
        assert_eq!(m.deferred_records, [5]);
        let typed = crate::mission::parse(&bytes).unwrap();
        assert!(matches!(
            typed.slot_writes[0].kind,
            crate::mission::SlotWriteKind::ExpiryCinematic(name) if name == [0x15; 32]
        ));
        // Metadata keeps this deferred: typed decoding does not execute its
        // gameplay consumer, even after the slot write is established.
        let opaque = [4, 0, 0, 0, 1, 13, 0, 0, 0, 0, 2];
        assert!(parse(&opaque).is_ok());
        assert_eq!(parse(&opaque).unwrap().deferred_records, [5]);
        assert!(matches!(
            crate::mission::parse(&opaque).unwrap().slot_writes[0].kind,
            crate::mission::SlotWriteKind::Parameter13(0)
        ));
        for n in 0..bytes.len() {
            assert!(parse(&bytes[..n]).is_err());
        }
        bytes.push(0);
        assert!(parse(&bytes).is_err());
    }
    #[test]
    fn array_count_and_repeated_start_are_rejected() {
        assert!(parse(&[4, 0, 0, 0, 1, 30, 255, 255, 255, 255, 2]).is_err());
        assert!(parse(&[4, 0, 0, 0, 1, 1, 2]).is_err());
    }
}
