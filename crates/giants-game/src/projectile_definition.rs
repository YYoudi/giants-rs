//! Effective debug-reference projectile value records. All numeric declaration
//! words are preserved; exposing a word does not establish its consumer's
//! meaning. These records contain inline strings, never native actor pointers.
use giants_formats::weapons::{Catalog, Profile};
use std::collections::BTreeMap;
const BYTES: usize = 0x154;
const NAMES: [usize; 6] = [0x78, 0x98, 0xb8, 0xd8, 0xf8, 0x130];
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Definition {
    pub id: i32,
    values: [u8; BYTES],
}
impl Definition {
    fn defaults(id: i32) -> Self {
        let mut out = Self {
            id,
            values: [0; BYTES],
        };
        out.put(0, 38);
        out.put(0x118, u32::MAX);
        out.put(0x11c, u32::MAX);
        for (offset, value) in [
            (0xc, 50_f32),
            (0x10, 50.),
            (0x14, 1.),
            (0x28, 2.5),
            (0x2c, 1000.),
            (0x30, 150.),
            (0x34, 1.),
            (0x38, 5.),
            (0x3c, 40.),
            (0x44, 0.5),
            (0x48, 0.5),
            (0x50, 1.),
            (0x60, 1.),
        ] {
            out.put(offset, value.to_bits());
        }
        out
    }
    fn put(&mut self, offset: usize, value: u32) {
        self.values[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    pub fn bytes(&self) -> &[u8; BYTES] {
        &self.values
    }
    pub fn word(&self, offset: usize) -> Result<u32, String> {
        if !offset.is_multiple_of(4)
            || !(offset < 0x78 || (0x118..0x130).contains(&offset) || offset == 0x150)
        {
            return Err("projectile word outside numeric declaration fields".into());
        }
        Ok(u32::from_le_bytes(
            self.values[offset..offset + 4].try_into().unwrap(),
        ))
    }
    pub fn class(&self) -> i32 {
        self.word(0).expect("class field") as i32
    }
    pub fn flags(&self) -> u32 {
        self.word(4).expect("flags field")
    }
    pub fn effect(&self, slot: usize) -> Option<&str> {
        let start = *NAMES.get(slot)?;
        let tail = &self.values[start..start + 32];
        let end = tail.iter().position(|b| *b == 0)?;
        (end != 0).then(|| std::str::from_utf8(&tail[..end]).expect("validated source name"))
    }
    fn set_name(
        &mut self,
        slot: usize,
        data: &[u8],
        offset: u32,
        commands_end: usize,
    ) -> Result<(), String> {
        let start = NAMES[slot];
        if offset == 0 {
            self.values[start] = 0;
            return Ok(());
        }
        let offset = offset as usize;
        if offset < commands_end {
            return Err("projectile name overlaps commands".into());
        }
        let tail = data.get(offset..).ok_or("projectile name outside file")?;
        let size = tail
            .iter()
            .take(32)
            .position(|b| *b == 0)
            .ok_or("projectile name outside inline capacity")?;
        if !tail[..size]
            .iter()
            .all(|b| b.is_ascii_graphic() || *b == b' ')
        {
            return Err("projectile name outside ASCII profile".into());
        }
        // Overwrites include NUL but preserve bytes beyond it, exactly like
        // repeated source declarations. A null name clears only its first byte.
        self.values[start..start + size + 1].copy_from_slice(&tail[..size + 1]);
        Ok(())
    }
}
pub struct DebugDefinitions {
    pub catalog: Catalog,
    pub entries: BTreeMap<i32, Definition>,
}
impl DebugDefinitions {
    pub fn parse(data: &[u8]) -> Result<Self, String> {
        let catalog = giants_formats::weapons::parse_with_profile(data, Profile::Debug)
            .map_err(|e| e.to_string())?;
        let mut entries = BTreeMap::new();
        let mut current = None;
        for record in &catalog.records {
            match record.opcode {
                1 => {
                    let id = record.args[0] as i32;
                    entries.insert(id, Definition::defaults(id));
                    current = Some(id);
                    continue;
                }
                2 => {
                    current = None;
                    continue;
                }
                _ => {}
            }
            let Some(id) = current else {
                continue;
            };
            let out = entries.get_mut(&id).expect("selected definition");
            let arg = |i: usize| {
                record
                    .args
                    .get(i)
                    .copied()
                    .ok_or("projectile operand absent".to_string())
            };
            let offset = match record.opcode {
                3 => Some(0),
                5 => Some(0xc),
                6 => Some(0x10),
                8 => Some(0x28),
                0xa => Some(0x40),
                0xd => Some(0x44),
                0xe => Some(0x54),
                0xf => Some(0x58),
                0x10 => Some(0x5c),
                0x11 => Some(0x64),
                0x12 => Some(0x68),
                0x13 => Some(0x6c),
                0x14 => Some(0x70),
                0x15 => Some(0x74),
                0x18 => Some(0x2c),
                0x19 => Some(0x120),
                0x1a => Some(0x124),
                0x1b => Some(0x128),
                0x1c => Some(0x12c),
                0x1e => Some(0x24),
                0x1f => Some(0x18),
                0x20 => Some(0x1c),
                0x21 => Some(0x20),
                0x22 => Some(8),
                0x23 => Some(0x60),
                0x32 => Some(0x14),
                0x33 => Some(0x50),
                0x37 => Some(0x4c),
                _ => None,
            };
            if let Some(offset) = offset {
                out.put(offset, arg(0)?);
                continue;
            }
            match record.opcode {
                4 => out.put(4, out.flags() | arg(0)?),
                7 => {
                    let bits = arg(0)?;
                    if f32::from_bits(bits).is_nan() {
                        return Err("projectile broadcast NaN outside checked x87 profile".into());
                    }
                    for at in [0x18, 0x1c, 0x20] {
                        out.put(at, bits);
                    }
                }
                9 => {
                    for i in 0..4 {
                        out.put(0x30 + i * 4, arg(i)?);
                    }
                }
                0x16 | 0x17 | 0x35 | 0x1d | 0x31 | 0x3a => {
                    let slot = match record.opcode {
                        0x16 => 0,
                        0x17 => 1,
                        0x35 => 2,
                        0x1d => 3,
                        0x31 => 4,
                        0x3a => 5,
                        _ => unreachable!(),
                    };
                    out.set_name(slot, data, arg(0)?, catalog.commands_end)?;
                    if record.opcode == 0x3a {
                        out.put(0x150, arg(1)?);
                    }
                }
                0x38 => {
                    out.put(0x118, arg(0)?);
                    out.put(0x11c, arg(1)?);
                }
                0x39 => {
                    out.put(4, out.flags() | 0x20000);
                    out.put(0x28, 999_f32.to_bits());
                }
                // Icon writes and retired opcodes do not mutate the selected
                // projectile record. Their existing catalog reader is retained.
                _ => {}
            }
        }
        Ok(Self { catalog, entries })
    }
}
