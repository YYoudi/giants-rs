//! Version-one effect symbol image. Script framing is separate from execution.
use crate::{Error, slice, u32_at};
#[derive(Clone, Debug)]
pub struct Entry {
    pub name: String,
    pub offset: u32,
}
#[derive(Clone, Debug)]
pub struct Image {
    pub entries: Vec<Entry>,
    payload: Vec<u8>,
    body_start: usize,
}
impl Image {
    pub fn same_image(&self, other: &Self) -> bool {
        self.payload == other.payload
            && self.entries.len() == other.entries.len()
            && self
                .entries
                .iter()
                .zip(&other.entries)
                .all(|(a, b)| a.name == b.name && a.offset == b.offset)
    }
    /// Named creation may prefer a `_lite` symbol, then fall back to the normal
    /// first match. The caller supplies that preference; runtime gating is separate.
    pub fn adaptive_find(
        &self,
        name: &str,
        prefer_lite: bool,
    ) -> Result<Option<(usize, &Entry)>, Error> {
        if prefer_lite {
            if name.len() > 59 || !name.is_ascii() {
                return Err(Error(
                    "adaptive effect name exceeds native formatting profile",
                ));
            }
            if let Some(target) = self.find(&format!("{name}_lite"))
                && target.1.offset != u32::MAX
            {
                return Ok(Some(target));
            }
        }
        Ok(self.find(name))
    }
    pub fn find(&self, name: &str) -> Option<(usize, &Entry)> {
        self.entries
            .iter()
            .enumerate()
            .find(|(_, e)| e.name.eq_ignore_ascii_case(name))
    }
    pub fn program(&self, offset: u32) -> Result<crate::effect_program::Program, Error> {
        crate::effect_program::parse(self.program_tail(offset)?)
    }
    pub fn program_tail(&self, offset: u32) -> Result<&[u8], Error> {
        if (offset as usize) < self.body_start {
            return Err(Error("effect offset before program body"));
        }
        self.payload
            .get(offset as usize..)
            .filter(|v| !v.is_empty())
            .ok_or(Error("effect offset outside image"))
    }
}
pub fn parse(data: &[u8]) -> Result<Image, Error> {
    if data.len() > 16 * 1024 * 1024
        || u32_at(data, 0)? != 1
        || u32_at(data, 4)? as usize != data.len().saturating_sub(12)
    {
        return Err(Error("unsupported effect image framing"));
    }
    let count = u32_at(data, 8)? as usize;
    if count > 65536 {
        return Err(Error("effect entry budget"));
    }
    let payload = &data[12..];
    let mut at = 0;
    let mut entries = Vec::new();
    for _ in 0..count {
        let tail = payload
            .get(at..)
            .ok_or(Error("effect name outside image"))?;
        let end = tail
            .iter()
            .take(128)
            .position(|v| *v == 0)
            .ok_or(Error("effect name unterminated"))?;
        if end == 0 || !tail[..end].is_ascii() {
            return Err(Error("invalid effect name"));
        }
        let name = String::from_utf8(tail[..end].to_vec()).unwrap();
        at += end + 1;
        let offset = u32_at(payload, at)?;
        at += 4;
        entries.push(Entry { name, offset });
    }
    for entry in &entries {
        if (entry.offset as usize) < at || entry.offset as usize >= payload.len() {
            return Err(Error("effect program offset outside body"));
        }
    }
    slice(data, 12, payload.len())?;
    Ok(Image {
        entries,
        payload: payload.to_vec(),
        body_start: at,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn adaptive_suffix_limit_is_explicit_only_when_preference_formats_a_variant() {
        let mut payload = b"A\0".to_vec();
        payload.extend(6u32.to_le_bytes());
        payload.push(2);
        let mut data = 1u32.to_le_bytes().to_vec();
        data.extend((payload.len() as u32).to_le_bytes());
        data.extend(1u32.to_le_bytes());
        data.extend(payload);
        let image = parse(&data).unwrap();
        assert!(
            image
                .adaptive_find(&"x".repeat(59), true)
                .unwrap()
                .is_none()
        );
        assert!(image.adaptive_find(&"x".repeat(60), true).is_err());
        assert!(
            image
                .adaptive_find(&"x".repeat(60), false)
                .unwrap()
                .is_none()
        );
    }
    #[test]
    fn first_case_insensitive_symbol_and_offsets_are_bounded() {
        let mut payload = b"A\0".to_vec();
        payload.extend(12u32.to_le_bytes());
        payload.extend(b"a\0");
        payload.extend(13u32.to_le_bytes());
        payload.extend([3, 255]);
        let mut raw = 1u32.to_le_bytes().to_vec();
        raw.extend((payload.len() as u32).to_le_bytes());
        raw.extend(2u32.to_le_bytes());
        raw.extend(payload);
        let image = parse(&raw).unwrap();
        assert_eq!(image.find("a").unwrap().0, 0);
        assert_eq!(image.program_tail(12).unwrap(), [3, 255]);
        assert!(image.program_tail(0).is_err());
        for n in 0..raw.len() {
            assert!(parse(&raw[..n]).is_err());
        }
        raw[14..18].copy_from_slice(&0u32.to_le_bytes());
        assert!(parse(&raw).is_err());
    }
}
