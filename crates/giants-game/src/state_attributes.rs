//! State-payload effects only; pointers are retained as unresolved typed keys.
use crate::{
    arithmetic::Precision,
    state_storage::{Profile, Reference, ReferenceKey, Seed},
};
use giants_formats::class_program::Record;
fn word(data: &[u8], at: usize) -> Result<u32, String> {
    Ok(u32::from_le_bytes(
        data.get(at..at + 4)
            .ok_or("state attribute word truncated")?
            .try_into()
            .unwrap(),
    ))
}
fn text(data: &[u8], at: usize) -> Result<(String, usize), String> {
    let tail = data.get(at..).ok_or("state attribute string missing")?;
    let n = tail
        .iter()
        .position(|v| *v == 0)
        .ok_or("state attribute string unterminated")?;
    Ok((
        std::str::from_utf8(&tail[..n])
            .map_err(|_| "invalid attribute text")?
            .into(),
        at + n + 1,
    ))
}
impl Seed {
    fn write(&mut self, offset: usize, bytes: &[u8]) -> Result<(), String> {
        let data = self
            .storage
            .data_mut()
            .ok_or("state attribute has no storage")?;
        data.get_mut(offset..offset + bytes.len())
            .ok_or("state attribute outside storage")?
            .copy_from_slice(bytes);
        self.unresolved
            .retain(|r| r.offset + 4 <= offset || r.offset >= offset + bytes.len());
        Ok(())
    }
    fn put(&mut self, offset: usize, value: u32) -> Result<(), String> {
        self.write(offset, &value.to_le_bytes())
    }
    fn pending(&mut self, offset: usize, key: ReferenceKey) -> Result<(), String> {
        self.put(offset, 0)?;
        self.unresolved.push(Reference { offset, key });
        Ok(())
    }
    fn copy_word(&mut self, to: usize, record: &Record, from: usize) -> Result<(), String> {
        self.put(to, word(&record.bytes, from)?)
    }
    fn name(&mut self, offset: usize, name: &str) -> Result<(), String> {
        let mut bytes = [0; 32];
        let n = name.len().min(32);
        bytes[..n].copy_from_slice(&name.as_bytes()[..n]);
        self.write(offset, &bytes)
    }
    pub(crate) fn apply_attribute(&mut self, r: &Record, p: Precision) -> Result<bool, String> {
        let state = self.storage.state();
        match r.opcode {
            0x1f | 0x28 => {
                let base = match (r.opcode, state) {
                    (0x1f, 1) => Some(0xa0),
                    (0x1f, 2) => Some(match self.storage.profile() {
                        Profile::Debug => 0x70,
                        Profile::Gog => 0x6c,
                    }),
                    (0x1f, 3) => Some(0xfb4),
                    (0x28, 1) => Some(0xa4),
                    _ => None,
                };
                if let Some(base) = base {
                    self.pending(base, ReferenceKey::Spec(word(&r.bytes, 1)?))?;
                    let target = if state == 1 {
                        if r.opcode == 0x28 { 0xb4 } else { 0xa8 }
                    } else {
                        base + 4
                    };
                    for i in 0..3 {
                        self.copy_word(target + 4 * i, r, 5 + 4 * i)?;
                    }
                } else {
                    self.diagnostics
                        .push("state spec attribute incompatible with current state".into());
                }
                // Only the GOG state2 branch also modifies object-level data.
                // The debug branch ends after the supplied payload writes.
                if r.opcode == 0x1f && state == 2 && self.storage.profile() == Profile::Gog {
                    self.unexecuted.push(r.opcode);
                }
            }
            0x3c => {
                if matches!(state, 6 | 43 | 57) {
                    for (to, from) in [(0x10, 1), (0xc, 5), (0x18, 9), (0x14, 13)] {
                        self.copy_word(to, r, from)?;
                    }
                    let v = |at| word(&r.bytes, at).map(f32::from_bits);
                    let (a, b, c, d) = (v(1)?, v(5)?, v(9)?, v(13)?);
                    if ![a, b, c, d].into_iter().all(f32::is_finite) {
                        return Err("non-finite state coefficient input".into());
                    }
                    let slope = p.round(
                        p.round(f64::from(a) - f64::from(c)) / p.round(f64::from(b) - f64::from(d)),
                    );
                    let intercept = p.round(f64::from(a) - p.round(slope * f64::from(b))) as f32;
                    if !(slope as f32).is_finite() || !intercept.is_finite() {
                        return Err("unsupported state coefficient division".into());
                    }
                    self.put(0x1c, (slope as f32).to_bits())?;
                    self.put(0x20, intercept.to_bits())?;
                } else {
                    self.diagnostics
                        .push("state coefficient attribute incompatible with current state".into());
                }
            }
            0x7a => {
                if matches!(state, 60 | 101) {
                    self.copy_word(0, r, 1)?;
                }
            }
            0x7b | 0x7c => {
                if matches!(state, 60 | 101) {
                    let (name, _) = text(&r.bytes, 1)?;
                    self.pending(
                        if r.opcode == 0x7b { 4 } else { 8 },
                        ReferenceKey::Effect(name),
                    )?;
                }
            }
            0x81 => {
                if state == 58 {
                    for (to, from) in [(0, 1), (4, 5), (8, 9), (0x14, 13), (0x18, 17)] {
                        self.copy_word(to, r, from)?;
                    }
                }
            }
            0x82 => {
                if state == 58 {
                    for (to, from) in [(0xc, 1), (0x10, 5), (0x1c, 9), (0x20, 13)] {
                        self.copy_word(to, r, from)?;
                    }
                }
            }
            0x83 => {
                if matches!(state, 60 | 101) {
                    self.put(0xc, 1)?;
                }
            }
            0x85 => {
                if state == 5 {
                    let (name, end) = text(&r.bytes, 1)?;
                    self.pending(0, ReferenceKey::Effect(name))?;
                    for i in 0..3 {
                        self.copy_word(4 + 4 * i, r, end + 4 * i)?;
                    }
                }
            }
            0x86 | 0x87 => {
                if state == 58 {
                    let (name, end) = text(&r.bytes, if r.opcode == 0x86 { 5 } else { 1 })?;
                    let counter = if r.opcode == 0x86 { 0x34 } else { 0x38 };
                    let count = word(
                        self.storage.data().ok_or("state names have no storage")?,
                        counter,
                    )? as i32;
                    if count < 0 {
                        return Err("negative state name index".into());
                    }
                    if count < 10 {
                        if r.opcode == 0x86 {
                            self.copy_word(0, r, 1)?;
                            self.copy_word(0x14, r, end)?;
                            self.copy_word(0x18, r, end + 4)?;
                            self.name(0x3c + count as usize * 32, &name)?;
                        } else {
                            self.name(0x17c + count as usize * 32, &name)?;
                            self.copy_word(0x1c, r, end)?;
                            self.copy_word(0x20, r, end + 4)?;
                        }
                        self.put(counter, count as u32 + 1)?;
                    }
                }
            }
            0x88 => {
                if state == 5 {
                    for i in 0..3 {
                        self.copy_word(0x14 + i * 4, r, 1 + i * 4)?;
                    }
                }
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pending_keys_overwrite_clear_and_bounded_name_tables() {
        let p = giants_formats::class_program::parse(b"\x1e\x65\x7bone\0\x7btwo\0\xff").unwrap();
        let seed = Seed::from_program(&p).unwrap();
        assert_eq!(seed.unresolved.len(), 1);
        assert_eq!(seed.unresolved[0].key, ReferenceKey::Effect("two".into()));
        assert_eq!(seed.storage.data().unwrap()[4..8], [0; 4]);
        let mut raw = b"\x1e\x65\x7bone\0\x44\x02\0\0\0".to_vec();
        raw.extend([9; 8]);
        raw.push(0xff);
        let p = giants_formats::class_program::parse(&raw).unwrap();
        assert!(Seed::from_program(&p).unwrap().unresolved.is_empty());
        let mut raw = vec![0x1e, 58];
        for i in 0..12 {
            raw.push(0x87);
            raw.extend(format!("item{i}\0").bytes());
            raw.extend([0; 8]);
        }
        raw.push(0xff);
        let p = giants_formats::class_program::parse(&raw).unwrap();
        let seed = Seed::from_program(&p).unwrap();
        assert_eq!(word(seed.storage.data().unwrap(), 0x38).unwrap(), 10);
        assert_eq!(&seed.storage.data().unwrap()[0x17c..0x181], b"item0");
    }
}
