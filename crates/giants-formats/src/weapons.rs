//! Bounded weapon command stream and selected effective declaration fields.
//! Numeric physics fields remain in their ordered records, not a simulation.
use crate::{Error, slice, u32_at};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct Record {
    pub offset: usize,
    pub opcode: u32,
    pub args: Vec<u32>,
    pub words: usize,
}
#[derive(Clone, Debug)]
pub struct Projectile {
    pub id: i32,
    pub class: i32,
    pub flags: u32,
    pub interval: f32,
    pub aim_response: f32,
    pub launch_speed: f32,
    pub lifetime: f32,
    pub mass: f32,
    pub gravity_mass: f32,
    pub range: f32,
    pub direct_damage: f32,
    pub effects: [Option<String>; 6],
}
#[derive(Clone, Debug)]
pub struct Icon {
    pub id: i32,
    pub records: Vec<usize>,
    pub description: Option<String>,
    pub projectile: i32,
    pub weight: i32,
}
#[derive(Clone, Debug)]
pub struct Catalog {
    pub records: Vec<Record>,
    pub commands_end: usize,
    pub projectiles: BTreeMap<i32, Projectile>,
    pub icons: BTreeMap<i32, Icon>,
    pub unprojected_records: usize,
    pub ignored_records: usize,
    pub inactive_records: usize,
}
fn name(data: &[u8], offset: u32, start: usize) -> Result<Option<String>, Error> {
    if offset == 0 {
        return Ok(None);
    }
    let offset = usize::try_from(offset).map_err(|_| Error("weapon string offset"))?;
    if offset < start {
        return Err(Error("weapon string overlaps commands"));
    }
    let tail = data
        .get(offset..)
        .ok_or(Error("weapon string outside file"))?;
    let size = tail
        .iter()
        .take(32)
        .position(|b| *b == 0)
        .ok_or(Error("weapon string exceeds native capacity"))?;
    if !tail[..size]
        .iter()
        .all(|b| b.is_ascii_graphic() || *b == b' ')
    {
        return Err(Error("unsupported weapon string encoding"));
    }
    Ok((size != 0).then(|| String::from_utf8(tail[..size].to_vec()).unwrap()))
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    Gog,
    Debug,
}
pub fn parse(data: &[u8]) -> Result<Catalog, Error> {
    parse_with_profile(data, Profile::Gog)
}
pub fn parse_with_profile(data: &[u8], profile: Profile) -> Result<Catalog, Error> {
    if data.len() > 4 * 1024 * 1024
        || u32_at(data, 0)? != 1
        || u32_at(data, 4)? as usize != data.len()
    {
        return Err(Error("unsupported weapon catalog header"));
    }
    let mut records = Vec::new();
    let mut at = 8;
    loop {
        if records.len() >= 8192 {
            return Err(Error("weapon command budget"));
        }
        let opcode = u32_at(data, at)?;
        let words = u32_at(data, at + 4)? as usize;
        if !(2..=32).contains(&words) {
            return Err(Error("weapon command length"));
        }
        let bytes = slice(data, at, words * 4)?;
        let minimum = match opcode {
            0 | 2 | 0x25 | 0x39 => 2,
            9 => 6,
            0x38 | 0x3a => 4,
            _ => 3,
        };
        if words < minimum || (opcode == 0 && words != 2) {
            return Err(Error("weapon command arguments"));
        }
        records.push(Record {
            offset: at,
            opcode,
            args: bytes[8..]
                .as_chunks::<4>()
                .0
                .iter()
                .map(|b| u32::from_le_bytes(*b))
                .collect(),
            words,
        });
        at += words * 4;
        if opcode == 0 {
            break;
        }
    }
    let mut out = Catalog {
        records,
        commands_end: at,
        projectiles: BTreeMap::new(),
        icons: BTreeMap::new(),
        unprojected_records: 0,
        ignored_records: 0,
        inactive_records: 0,
    };
    let (mut projectile, mut icon) = (None, None);
    for r in &out.records {
        match r.opcode {
            0 => {}
            1 => {
                let id = r.args[0] as i32;
                if !(0..53).contains(&id) {
                    return Err(Error("projectile identifier outside native table"));
                }
                out.projectiles.insert(
                    id,
                    Projectile {
                        id,
                        class: 38,
                        flags: 0,
                        interval: 0.5,
                        aim_response: 0.,
                        launch_speed: 0.,
                        lifetime: 2.5,
                        mass: 0.,
                        gravity_mass: 0.,
                        range: 1000.,
                        direct_damage: 0.,
                        effects: Default::default(),
                    },
                );
                projectile = Some(id);
            }
            2 => projectile = None,
            0x24 => {
                let id = r.args[0] as i32;
                if !(-1..75).contains(&id) {
                    return Err(Error("icon identifier outside native table"));
                }
                out.icons.insert(
                    id,
                    Icon {
                        id,
                        records: Vec::new(),
                        description: None,
                        projectile: -1,
                        weight: 0,
                    },
                );
                icon = Some(id);
            }
            0x25 => icon = None,
            0x26..=0x30 | 0x36 => {
                if let Some(id) = icon {
                    let i = out.icons.get_mut(&id).unwrap();
                    i.records.push(r.offset);
                    if r.opcode == 0x30 {
                        i.description = name(data, r.args[0], at)?;
                    } else if r.opcode == 0x2c {
                        i.weight = r.args[0] as i32;
                    } else if r.opcode == 0x2b {
                        i.projectile = r.args[0] as i32;
                    } else {
                        out.unprojected_records += 1;
                    }
                } else {
                    out.inactive_records += 1;
                }
            }
            3 | 4 | 0xd | 0x16 | 0x17 | 0x1a | 0x1d | 0x31 | 0x35 | 0x39 | 0x3a => {
                let Some(id) = projectile else {
                    out.inactive_records += 1;
                    continue;
                };
                let p = out.projectiles.get_mut(&id).unwrap();
                match r.opcode {
                    3 => p.class = r.args[0] as i32,
                    4 => p.flags |= r.args[0],
                    0x1a => p.aim_response = f32::from_bits(r.args[0]),
                    0x39 => {
                        p.flags |= 0x20000;
                        p.lifetime = 999.;
                    }
                    0xd => {
                        p.interval = if profile == Profile::Gog && matches!(id, 16 | 24) {
                            0.4
                        } else {
                            f32::from_bits(r.args[0])
                        }
                    }
                    _ => {
                        let slot = match r.opcode {
                            0x16 => 0,
                            0x17 => 1,
                            0x35 => 2,
                            0x1d => 3,
                            0x31 => 4,
                            0x3a => 5,
                            _ => unreachable!(),
                        };
                        p.effects[slot] = name(data, r.args[0], at)?;
                        if profile == Profile::Gog && r.opcode == 0x16 && id == 34 {
                            p.effects[1] = Some("fx_spell_2".into());
                            p.effects[2] = Some("fx_spell_2".into());
                        }
                        if r.opcode == 0x3a {
                            out.unprojected_records += 1;
                        }
                    }
                }
            }
            8 | 0xa | 0xe | 0xf | 0x18 | 0x22 => {
                if let Some(id) = projectile {
                    let p = out.projectiles.get_mut(&id).unwrap();
                    let value = f32::from_bits(r.args[0]);
                    match r.opcode {
                        8 => p.lifetime = value,
                        0xa => {
                            p.direct_damage = match (profile, id) {
                                (Profile::Gog, 48) => 400.,
                                (Profile::Gog, 38) => 80.,
                                (Profile::Gog, 15) => 100.,
                                (Profile::Gog, 52) => 30.,
                                (Profile::Gog, 12) => 1200.,
                                (Profile::Gog, 16) => 24.,
                                (Profile::Gog, 24) => 40.,
                                _ => value,
                            }
                        }
                        0xe => p.mass = value,
                        0xf => p.gravity_mass = value,
                        0x18 => p.range = value,
                        0x22 => p.launch_speed = value,
                        _ => unreachable!(),
                    }
                } else {
                    out.inactive_records += 1;
                }
            }
            5..=7
            | 9
            | 0x10..=0x15
            | 0x19
            | 0x1b..=0x1c
            | 0x1e..=0x21
            | 0x23
            | 0x32
            | 0x33
            | 0x37..=0x38 => {
                if projectile.is_some() {
                    out.unprojected_records += 1;
                } else {
                    out.inactive_records += 1;
                }
            }
            _ => out.ignored_records += 1,
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::parse;
    fn file(words: &[u32]) -> Vec<u8> {
        let mut out = vec![1u32, ((words.len() + 2) * 4) as u32];
        out.extend_from_slice(words);
        out.into_iter().flat_map(u32::to_le_bytes).collect()
    }
    #[test]
    fn rejects_nonadvancing_or_truncated_records() {
        for words in [&[1, 0][..], &[1, 1][..], &[1, 3][..], &[0, 2, 0][..]] {
            let data = file(words);
            if words == [0, 2, 0] {
                assert!(parse(&data).is_ok());
            } else {
                assert!(parse(&data).is_err());
            }
        }
        for size in 0..16 {
            assert!(parse(&vec![0; size]).is_err());
        }
    }
    #[test]
    fn rejects_pointers_into_commands_and_missing_strings() {
        for pointer in [4, 8, 40, u32::MAX] {
            assert!(parse(&file(&[1, 3, 0, 0x16, 3, pointer, 2, 2, 0, 2])).is_err());
        }
        assert!(parse(&file(&[1, 3, 0, 0x16, 3, 0, 2, 2, 0, 2])).is_ok());
    }
    #[test]
    fn rejects_native_table_underflow_and_overflow() {
        for id in [u32::MAX, 53, i32::MAX as u32] {
            assert!(parse(&file(&[1, 3, id, 0, 2])).is_err());
        }
        for id in [u32::MAX - 1, 75, i32::MAX as u32] {
            assert!(parse(&file(&[0x24, 3, id, 0, 2])).is_err());
        }
    }
}
