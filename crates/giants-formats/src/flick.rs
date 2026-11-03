//! First-director FLK command framing and bounded string references.
//! This is the measured first-sequence profile, not a general FLK interpreter.
use crate::{Error, slice, u32_at};

#[derive(Debug, Clone)]
pub struct Command {
    pub offset: usize,
    pub opcode: u32,
    pub words: Vec<u32>,
    pub label: Option<String>,
}
#[derive(Debug, Clone)]
pub struct Program {
    pub commands: Vec<Command>,
    pub next: Option<String>,
    pub program_end: usize,
}
/// Declared effect-slot command. This is neither an allocated effect nor a
/// resolved actor reference. Negative slots retain their native no-op meaning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectCommand {
    pub offset: usize,
    pub slot: i32,
    pub name: Option<String>,
    pub mode: u32,
    pub actor: i32,
    pub reference: Option<String>,
}

/// Resolve command39 strings independently of executable-profile admission.
/// Resource lookup, current actor pose and effect creation are separate steps.
pub fn effect_commands(data: &[u8]) -> Result<Vec<EffectCommand>, Error> {
    let metadata = discover_dependencies(data)?;
    let end = metadata.commands.last().ok_or(Error("missing flick end"))?;
    let pool = end.offset + end.words.len() * 4 + 8..metadata.animation_region_offset;
    let effect_slots = u32_at(data, 16)?;
    let actor_slots = u32_at(data, 12)?;
    let mut requests = Vec::new();
    for c in &metadata.commands {
        if c.opcode != 39 {
            continue;
        }
        if c.words.len() != 5 {
            return Err(Error("invalid flick effect command size"));
        }
        let slot = c.words[0] as i32;
        let actor = c.words[3] as i32;
        if slot >= 0 && c.words[0] >= effect_slots || actor >= 0 && c.words[3] >= actor_slots {
            return Err(Error(
                "flick effect slot or actor outside declared capacity",
            ));
        }
        if c.words[2] > 1 {
            return Err(Error("unsupported flick effect mode"));
        }
        requests.push(EffectCommand {
            offset: c.offset,
            slot,
            name: label(data, c.words[1], pool.clone())?,
            mode: c.words[2],
            actor,
            reference: label(data, c.words[4], pool.clone())?,
        });
    }
    Ok(requests)
}
#[derive(Debug)]
pub struct DependencyMetadata {
    pub actors: Vec<crate::cast::Actor>,
    pub commands: Vec<Command>,
    pub next: Option<String>,
    pub opaque_blocks: u32,
    pub animation_region_offset: usize,
    pub runtime_profile_supported: bool,
}
/// Structural dependency discovery, separate from the strict executable profile.
/// Unknown command bodies and auxiliary blocks are retained, never interpreted.
pub fn discover_dependencies(data: &[u8]) -> Result<DependencyMetadata, Error> {
    if !(40..=4 * 1024 * 1024).contains(&data.len())
        || u32_at(data, 16)? > 4096
        || u32_at(data, 24)? != 1
        || u32_at(data, 28)? != 4
        || u32_at(data, 32)? != 0
    {
        return Err(Error("unsupported dependency discovery header"));
    }
    let actors = crate::cast::first_flick(data)?;
    let image = u32_at(data, 36)? as usize;
    if image < 40 || !image.is_multiple_of(4) || image >= data.len() {
        return Err(Error("invalid dependency image boundary"));
    }
    let mut at = 24;
    let mut commands = Vec::new();
    let mut ended = false;
    while at + 8 <= image {
        let opcode = u32_at(data, at)?;
        let size = u32_at(data, at + 4)? as usize;
        if commands.len() >= 8192
            || !(1..=53).contains(&opcode)
            || !(2..=4096).contains(&size)
            || at + size * 4 > image
        {
            return Err(Error("invalid dependency command record"));
        }
        let words = (2..size)
            .map(|i| u32_at(data, at + i * 4))
            .collect::<Result<Vec<_>, _>>()?;
        if matches!(opcode, 16 | 44 | 52) && size != 3 {
            return Err(Error("invalid dependency event size"));
        }
        if matches!(opcode, 47 | 51) && size != 3 || opcode == 49 && size != 2 {
            return Err(Error("invalid dependency credits size"));
        }
        commands.push(Command {
            offset: at,
            opcode,
            words,
            label: None,
        });
        at += size * 4;
        if opcode == 5 {
            if size != 2 {
                return Err(Error("invalid dependency end record"));
            }
            ended = true;
            break;
        }
    }
    if !ended {
        return Err(Error("missing dependency end record"));
    }
    for command in &mut commands {
        if matches!(command.opcode, 16 | 44 | 47 | 52) {
            command.label = label(data, command.words[0], at..image)?;
            if command.opcode != 47 && command.label.is_none() {
                return Err(Error("missing dependency event label"));
            }
        }
    }
    Ok(DependencyMetadata {
        actors,
        commands,
        next: label(data, u32_at(data, 20)?, at..image)?,
        opaque_blocks: u32_at(data, 16)?,
        animation_region_offset: image,
        runtime_profile_supported: parse(data).is_ok(),
    })
}
fn label(data: &[u8], offset: u32, pool: std::ops::Range<usize>) -> Result<Option<String>, Error> {
    if offset == 0 {
        return Ok(None);
    }
    let at = offset as usize;
    if !pool.contains(&at) {
        return Err(Error("flick string outside pool"));
    }
    let raw = slice(data, at, (pool.end - at).min(128))?;
    let end = raw
        .iter()
        .position(|b| *b == 0)
        .ok_or(Error("unterminated flick string"))?;
    if end == 0 || !raw[..end].is_ascii() {
        return Err(Error("invalid flick string"));
    }
    Ok(Some(String::from_utf8(raw[..end].to_vec()).unwrap()))
}

pub fn parse(data: &[u8]) -> Result<Program, Error> {
    parse_profile(data, false)
}

/// Playback preview: frame additional records without claiming their effects.
pub fn parse_preview(data: &[u8]) -> Result<Program, Error> {
    parse_profile(data, true)
}

fn parse_profile(data: &[u8], preview: bool) -> Result<Program, Error> {
    if data.len() > 4 * 1024 * 1024
        || u32_at(data, 0)? != 3
        || u32_at(data, 4)? as usize != data.len()
        || u32_at(data, 8)? != 1
        || u32_at(data, 12)? > 32
        || u32_at(data, 16)? > if preview { 4096 } else { 0 }
    {
        return Err(Error("unsupported first flick header"));
    }
    if u32_at(data, 24)? != 1 || u32_at(data, 28)? != 4 || u32_at(data, 32)? != 0 {
        return Err(Error("unsupported first flick director"));
    }
    let image = u32_at(data, 36)? as usize;
    if image < 40 || !image.is_multiple_of(4) || image >= data.len() {
        return Err(Error("invalid first flick image boundary"));
    }
    let mut at = 24;
    let mut commands = Vec::new();
    let mut terminated = false;
    while at < image {
        if commands.len() >= 8192 {
            return Err(Error("flick command budget"));
        }
        let opcode = u32_at(data, at)?;
        let size = u32_at(data, at + 4)? as usize;
        let expected = match opcode {
            1 | 7 | 21 | 25 => 4,
            2 | 3 => 6,
            4 | 41 => 5,
            5 | 6 | 14 | 15 => 2,
            9 | 11 | 12 | 16..=20 | 22 | 23 | 44 | 52 => 3,
            13 if preview => 4,
            27 if preview => 3,
            31 if preview => 4,
            32 if preview && matches!(size, 5 | 6 | 7 | 9) => size,
            33 if preview => 4,
            42 if preview => 5,
            40 if preview => 3,
            38 if preview => 3,
            45 | 46 if preview => 2,
            47 | 51 if preview => 3,
            49 | 53 if preview => 2,
            37 | 39 if preview => 7,
            34 => 9,
            _ => return Err(Error("unsupported first flick command")),
        };
        if size != expected || at + size * 4 > image {
            return Err(Error("invalid flick command size"));
        }
        let words = (2..size)
            .map(|i| u32_at(data, at + i * 4))
            .collect::<Result<Vec<_>, _>>()?;
        if opcode == 51 && !f32::from_bits(words[0]).is_finite() {
            return Err(Error("credits speed outside finite preview profile"));
        }
        if opcode == 13
            && words[0] as i32 >= 0
            && (words[0] >= u32_at(data, 12)? || words[1] >= 2000)
        {
            return Err(Error(
                "attached sound actor or sound outside table capacity",
            ));
        }
        if opcode == 27 && words[0] as i32 >= 0 && words[0] >= 2000 {
            return Err(Error("sound stop outside table capacity"));
        }
        if opcode == 32 {
            let slot = *words.last().ok_or(Error("offset actor slot absent"))? as i32;
            if slot >= 0 && slot as u32 >= u32_at(data, 12)? {
                return Err(Error("offset actor slot outside declarations"));
            }
        }
        if opcode == 33 && words[0] as i32 >= 0 && words[0] >= u32_at(data, 12)? {
            return Err(Error("scale actor slot outside declarations"));
        }
        if matches!(opcode, 11 | 12) && words[0] >= u32_at(data, 12)? {
            return Err(Error("visibility actor slot outside declarations"));
        }
        if opcode == 42 {
            if words[0] as i32 >= 0 && words[0] >= u32_at(data, 12)? {
                return Err(Error("transparency actor slot outside declarations"));
            }
            if words[0] as i32 >= 0
                && (!(0. ..=1.).contains(&f32::from_bits(words[1]))
                    || !(0. ..=3600.).contains(&f32::from_bits(words[2])))
            {
                return Err(Error("actor transparency outside envelope profile"));
            }
        }
        if opcode == 7 {
            let fraction = f32::from_bits(words[0]);
            let duration = f32::from_bits(words[1]);
            if !(0. ..=1.).contains(&fraction) || !(0. ..=3600.).contains(&duration) {
                return Err(Error("unsupported cinematic frame envelope"));
            }
        }
        commands.push(Command {
            offset: at,
            opcode,
            words,
            label: None,
        });
        at += size * 4;
        if opcode == 5 {
            terminated = true;
            break;
        }
    }
    if !terminated {
        return Err(Error("missing first flick end"));
    }
    let pool = at..image;
    for command in &mut commands {
        let pointer = match command.opcode {
            16 | 40 | 44 | 47 | 52 => Some(command.words[0]),
            2 | 25 => Some(command.words[1]),
            4 => Some(command.words[2]),
            _ => None,
        };
        if let Some(pointer) = pointer {
            command.label = label(data, pointer, pool.clone())?;
            if command.opcode == 40
                && command.label.as_ref().is_some_and(|name| {
                    !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                })
            {
                return Err(Error("unsupported completion hold label"));
            }
            if matches!(command.opcode, 16 | 44 | 52) && command.label.is_none() {
                return Err(Error("missing flick event label"));
            }
        }
        if command.opcode == 3 {
            label(data, command.words[1], pool.clone())?.ok_or(Error("missing attachment node"))?;
            let handle = label(data, command.words[3], pool.clone())?;
            if !preview && handle.is_none() {
                return Err(Error("missing attachment handle"));
            }
        }
    }
    Ok(Program {
        commands,
        next: label(data, u32_at(data, 20)?, pool)?,
        program_end: at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn numeric_audio_and_credits_toggle_are_bounded_preview_requests() {
        let mut data = vec![0; 128];
        for (at, word) in [
            (0, 3_u32),
            (4, 128),
            (8, 1),
            (12, 1),
            (24, 1),
            (28, 4),
            (36, 112),
            (40, 13),
            (44, 4),
            (48, 0),
            (52, 31),
            (56, 27),
            (60, 3),
            (64, 73),
            (68, 53),
            (72, 2),
            (76, 5),
            (80, 2),
        ] {
            data[at..at + 4].copy_from_slice(&word.to_le_bytes());
        }
        assert!(parse(&data).is_err());
        assert_eq!(parse_preview(&data).unwrap().commands.len(), 5);
        for (at, value) in [(48, 1_u32), (52, 2000), (64, 2000), (72, 3)] {
            let mut bad = data.clone();
            bad[at..at + 4].copy_from_slice(&value.to_le_bytes());
            assert!(parse_preview(&bad).is_err());
        }
        data[48..56].fill(255);
        data[64..68].fill(255);
        assert!(parse_preview(&data).is_ok());
    }
    fn effect_fixture() -> Vec<u8> {
        let mut data = vec![0; 176];
        for (at, word) in [
            (0, 3_u32),
            (4, 176),
            (8, 1),
            (12, 1),
            (16, 1),
            (24, 1),
            (28, 4),
            (36, 144),
            (40, 2),
            (44, 6),
            (60, u32::MAX),
            (64, 39),
            (68, 7),
            (72, 0),
            (76, 104),
            (80, 1),
            (84, 0),
            (88, 112),
            (92, 5),
            (96, 2),
        ] {
            data[at..at + 4].copy_from_slice(&word.to_le_bytes());
        }
        data[104..107].copy_from_slice(b"fx\0");
        data[112..116].copy_from_slice(b"ref\0");
        data
    }
    #[test]
    fn effect_slot_binding_keeps_both_strings_and_signed_absences() {
        let mut data = effect_fixture();
        let effects = effect_commands(&data).unwrap();
        assert_eq!(effects.len(), 1);
        assert_eq!(
            effects[0],
            EffectCommand {
                offset: 64,
                slot: 0,
                name: Some("fx".into()),
                mode: 1,
                actor: 0,
                reference: Some("ref".into())
            }
        );
        data[72..76].copy_from_slice(&u32::MAX.to_le_bytes());
        data[84..88].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(effect_commands(&data).unwrap()[0].slot, -1);
        assert_eq!(effect_commands(&data).unwrap()[0].actor, -1);
    }
    #[test]
    fn effect_binding_rejects_outside_pool_capacity_and_unknown_mode() {
        for (at, value) in [(76, 4_u32), (88, 144), (72, 1), (84, 1), (80, 2)] {
            let mut data = effect_fixture();
            data[at..at + 4].copy_from_slice(&value.to_le_bytes());
            assert!(effect_commands(&data).is_err());
        }
    }
    #[test]
    fn credits_preview_keeps_labels_sizes_and_finite_speed_without_runtime_admission() {
        let mut data = vec![0; 128];
        for (at, value) in [
            (0, 3u32),
            (4, 128),
            (8, 1),
            (12, 0),
            (24, 1),
            (28, 4),
            (36, 112),
            (40, 47),
            (44, 3),
            (48, 96),
            (52, 49),
            (56, 2),
            (60, 51),
            (64, 3),
            (68, 2f32.to_bits()),
            (72, 5),
            (76, 2),
        ] {
            data[at..at + 4].copy_from_slice(&value.to_le_bytes());
        }
        data[96..104].copy_from_slice(b"credits\0");
        assert!(parse(&data).is_err());
        let program = parse_preview(&data).unwrap();
        assert_eq!(program.commands[1].label.as_deref(), Some("credits"));
        let mut bad = data.clone();
        bad[48..52].copy_from_slice(&40u32.to_le_bytes());
        assert!(parse_preview(&bad).is_err());
        bad = data.clone();
        bad[68..72].copy_from_slice(&f32::NAN.to_bits().to_le_bytes());
        assert!(parse_preview(&bad).is_err());
        data[56..60].copy_from_slice(&3u32.to_le_bytes());
        assert!(parse_preview(&data).is_err());
    }
    #[test]
    fn completion_hold_is_preview_only_nullable_and_pool_bounded() {
        let mut data = vec![0; 128];
        for (at, value) in [
            (0, 3u32),
            (4, 128),
            (8, 1),
            (24, 1),
            (28, 4),
            (36, 112),
            (40, 40),
            (44, 3),
            (48, 84),
            (52, 5),
            (56, 2),
        ] {
            data[at..at + 4].copy_from_slice(&value.to_le_bytes());
        }
        data[84..89].copy_from_slice(b"hold\0");
        assert!(parse(&data).is_err());
        assert_eq!(
            parse_preview(&data).unwrap().commands[1].label.as_deref(),
            Some("hold")
        );
        for pointer in [59u32, 112, u32::MAX] {
            let mut invalid = data.clone();
            invalid[48..52].copy_from_slice(&pointer.to_le_bytes());
            assert!(parse_preview(&invalid).is_err());
        }
        data[48..52].fill(0);
        assert!(parse_preview(&data).unwrap().commands[1].label.is_none());
    }
    #[test]
    fn dependency_discovery_keeps_opaque_records_without_accepting_runtime_execution() {
        let mut data = vec![0; 128];
        for (at, value) in [
            (0, 3u32),
            (4, 128),
            (8, 1),
            (12, 1),
            (16, 2),
            (24, 1),
            (28, 4),
            (36, 112),
            (40, 2),
            (44, 6),
            (60, 42),
            (64, 31),
            (68, 2),
            (72, 44),
            (76, 3),
            (80, 96),
            (84, 5),
            (88, 2),
        ] {
            data[at..at + 4].copy_from_slice(&value.to_le_bytes());
        }
        data[96..102].copy_from_slice(b"voice\0");
        let metadata = discover_dependencies(&data).unwrap();
        assert_eq!(metadata.actors[0].object_type, 42);
        assert_eq!(metadata.opaque_blocks, 2);
        assert_eq!(metadata.commands[2].opcode, 31);
        assert_eq!(metadata.commands[3].label.as_deref(), Some("voice"));
        assert!(!metadata.runtime_profile_supported);
        let mut bad = data.clone();
        bad[80..84].copy_from_slice(&40u32.to_le_bytes());
        assert!(discover_dependencies(&bad).is_err());
        let mut bad = data;
        bad[68..72].copy_from_slice(&4097u32.to_le_bytes());
        assert!(discover_dependencies(&bad).is_err());
    }
    #[test]
    fn preview_scale_keeps_negative_sentinel_and_rejects_outside_actor_table() {
        let mut b = vec![0; 80];
        for (at, v) in [
            (0, 3u32),
            (4, 80),
            (8, 1),
            (12, 2),
            (24, 1),
            (28, 4),
            (36, 76),
            (40, 33),
            (44, 4),
            (48, 1),
            (52, 2f32.to_bits()),
            (56, 5),
            (60, 2),
        ] {
            b[at..at + 4].copy_from_slice(&v.to_le_bytes());
        }
        assert!(parse(&b).is_err());
        assert!(parse_preview(&b).is_ok());
        b[48..52].copy_from_slice(&2u32.to_le_bytes());
        assert!(parse_preview(&b).is_err());
        b[48..52].copy_from_slice(&u32::MAX.to_le_bytes());
        b[52..56].copy_from_slice(&f32::NAN.to_bits().to_le_bytes());
        assert!(parse_preview(&b).is_ok());
    }
    #[test]
    fn screen_fade_record_is_preview_only_and_size_bounded() {
        let mut b = vec![0; 96];
        for (at, v) in [
            (0, 3u32),
            (4, 96),
            (8, 1),
            (24, 1),
            (28, 4),
            (36, 92),
            (40, 37),
            (44, 7),
            (60, 1f32.to_bits()),
            (64, 0.25f32.to_bits()),
            (68, 5),
            (72, 2),
        ] {
            b[at..at + 4].copy_from_slice(&v.to_le_bytes());
        }
        assert!(parse(&b).is_err());
        assert_eq!(parse_preview(&b).unwrap().commands[1].opcode, 37);
        b[44..48].copy_from_slice(&6u32.to_le_bytes());
        assert!(parse_preview(&b).is_err());
    }
    #[test]
    fn preview_effect_framing_stays_separate_from_strict_runtime() {
        let mut data = fixture();
        // Replace the wait with a frame-compatible opaque effect (opcode38).
        data[40..44].copy_from_slice(&38u32.to_le_bytes());
        assert!(parse(&data).is_err());
        assert_eq!(parse_preview(&data).unwrap().commands[1].opcode, 38);
        data[16..20].copy_from_slice(&2u32.to_le_bytes());
        assert!(parse_preview(&data).is_ok());
        data[16..20].copy_from_slice(&4097u32.to_le_bytes());
        assert!(parse_preview(&data).is_err());
        data[16..20].copy_from_slice(&0u32.to_le_bytes());
        data[44..48].copy_from_slice(&4u32.to_le_bytes());
        assert!(parse_preview(&data).is_err());
    }
    pub fn fixture() -> Vec<u8> {
        let mut b = vec![0; 104];
        for (at, v) in [
            (0, 3),
            (4, 104),
            (8, 1),
            (24, 1),
            (28, 4),
            (36, 100),
            (40, 9),
            (44, 3),
            (48, 2),
            (52, 44),
            (56, 3),
            (60, 72),
            (64, 5),
            (68, 2),
        ] {
            b[at..at + 4].copy_from_slice(&u32::to_le_bytes(v));
        }
        b[72..78].copy_from_slice(b"voice\0");
        b
    }
    #[test]
    fn command_and_pool_bounds() {
        let b = fixture();
        let p = parse(&b).unwrap();
        assert_eq!(p.commands.len(), 4);
        assert_eq!(p.program_end, 72);
        assert_eq!(p.commands[2].label.as_deref(), Some("voice"));
        for n in 0..b.len() {
            assert!(parse(&b[..n]).is_err());
        }
        for (at, value) in [(44, 0), (60, 52), (68, 4), (64, 255), (36, 101), (20, 4)] {
            let mut x = b.clone();
            x[at..at + 4].copy_from_slice(&u32::to_le_bytes(value));
            assert!(parse(&x).is_err());
        }
        let mut x = b;
        x[72..100].fill(b'x');
        assert!(parse(&x).is_err());
    }
    #[test]
    fn actor_transparency_is_preview_only_and_validates_only_active_slots() {
        let mut data = vec![0; 96];
        for (at, value) in [
            (0, 3u32),
            (4, 96),
            (8, 1),
            (12, 1),
            (24, 1),
            (28, 4),
            (36, 92),
            (40, 42),
            (44, 5),
            (52, 0.75f32.to_bits()),
            (60, 5),
            (64, 2),
        ] {
            data[at..at + 4].copy_from_slice(&value.to_le_bytes());
        }
        assert!(parse(&data).is_err());
        assert!(parse_preview(&data).is_ok());
        data[48..52].copy_from_slice(&1u32.to_le_bytes());
        assert!(parse_preview(&data).is_err());
        data[48..52].copy_from_slice(&u32::MAX.to_le_bytes());
        data[52..56].copy_from_slice(&f32::NAN.to_bits().to_le_bytes());
        assert!(parse_preview(&data).is_ok());
        data[48..52].copy_from_slice(&0u32.to_le_bytes());
        assert!(parse_preview(&data).is_err());
    }
}
