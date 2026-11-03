//! Global word tables. Field semantics belong to their separately checked consumers.
use crate::{Error, slice, u32_at};
pub struct GlobalData {
    pub words: Vec<u32>,
    pub array_words: Vec<u32>,
}
impl GlobalData {
    pub fn scalar(&self, byte_offset: usize) -> Result<f32, Error> {
        if !byte_offset.is_multiple_of(4) {
            return Err(Error("unaligned global scalar"));
        }
        let word = *self
            .words
            .get(byte_offset / 4)
            .ok_or(Error("global scalar outside table"))?;
        let scalar = f32::from_bits(word);
        if !scalar.is_finite() {
            return Err(Error("global scalar outside finite profile"));
        }
        Ok(scalar)
    }
}
pub fn parse(bytes: &[u8]) -> Result<GlobalData, Error> {
    if u32_at(bytes, 0)? != 0x7a330000 {
        return Err(Error("unsupported global table version"));
    }
    let first = u32_at(bytes, 4)? as usize;
    let second = u32_at(bytes, 8)? as usize;
    if first > 65536 || second > 65536 {
        return Err(Error("global word table budget"));
    }
    let total = 12 + (first + second) * 4;
    if bytes.len() != total {
        return Err(Error("global table length mismatch"));
    }
    let words = slice(bytes, 12, (first + second) * 4)?
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b))
        .collect::<Vec<_>>();
    Ok(GlobalData {
        words: words[..first].to_vec(),
        array_words: words[first..].to_vec(),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn partitions_raw_words_without_treating_integer_fields_as_floats() {
        let mut bytes = Vec::new();
        for w in [0x7a330000, 2, 1, 7, 20f32.to_bits(), u32::MAX] {
            bytes.extend(w.to_le_bytes());
        }
        let g = parse(&bytes).unwrap();
        assert_eq!(g.words, [7, 20f32.to_bits()]);
        assert_eq!(g.array_words, [u32::MAX]);
        assert_eq!(g.scalar(4).unwrap(), 20.);
        assert!(g.scalar(2).is_err());
        for n in 0..bytes.len() {
            assert!(parse(&bytes[..n]).is_err());
        }
        bytes.extend([0; 4]);
        assert!(parse(&bytes).is_err());
    }
}
