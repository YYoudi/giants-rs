//! Source-ordered animation event tables; activation and effect execution are separate.
use crate::arithmetic::Precision;
use giants_formats::{
    Error,
    effect_image::Image,
    object_spec::{Kind, Program},
};
use std::collections::BTreeSet;

#[derive(Debug)]
pub struct EffectReference {
    pub choice: usize,
    pub name: String,
    pub target: Option<u32>,
}
#[derive(Debug)]
pub struct Event {
    pub source_offset: usize,
    pub kind: u32,
    pub time: f32,
    pub parameters: [u32; 6],
    pub effects: Vec<EffectReference>,
}
#[derive(Debug)]
pub struct Track {
    pub source_offset: usize,
    pub slot: u8,
    pub animation: String,
    pub events: Vec<Event>,
}

/// Stored event time uses a binary32 constant, not the animation's declared FPS.
/// This does not specify which game clock advances or dispatches the table.
pub fn stored_time(frame: i32, precision: Precision) -> f32 {
    precision.round(f64::from(frame) * f64::from(f32::from_bits(0x3d088889))) as f32
}

pub fn prepare(
    program: &Program,
    effects: &Image,
    precision: Precision,
) -> Result<Vec<Track>, Error> {
    let mut tracks: Vec<Track> = Vec::new();
    let mut slots = BTreeSet::new();
    for record in &program.records {
        if let Kind::Animation { slot, name } = &record.kind {
            if *slot >= 254 || !slots.insert(*slot) {
                return Err(Error("unsupported event animation slot reuse"));
            }
            tracks.push(Track {
                source_offset: record.offset,
                slot: *slot,
                animation: name.clone(),
                events: Vec::new(),
            });
            continue;
        }
        if !matches!(record.opcode, 0x56..=0x58 | 0x5c | 0x7f | 0x80) {
            continue;
        }
        let track = tracks
            .last_mut()
            .ok_or(Error("event without preceding animation"))?;
        let added = if record.opcode == 0x57 { 2 } else { 1 };
        if track.events.len() + added > 32 {
            return Err(Error("animation event table capacity"));
        }
        let bytes = &record.bytes;
        let word = |at: usize| -> Result<u32, Error> {
            Ok(u32::from_le_bytes(
                bytes
                    .get(at..at + 4)
                    .ok_or(Error("truncated animation event"))?
                    .try_into()
                    .unwrap(),
            ))
        };
        let time = stored_time(word(1)? as i32, precision);
        let mut event = Event {
            source_offset: record.offset,
            kind: 0,
            time,
            parameters: [0; 6],
            effects: Vec::new(),
        };
        if let Kind::Effects { names } = &record.kind {
            if names.len() != 5 {
                return Err(Error("effect event choice count"));
            }
            event.kind = if record.opcode == 0x7f { 0x40 } else { 0x80 };
            for (choice, name) in names.iter().enumerate() {
                let target = effects.find(name).map(|(_, entry)| entry.offset);
                event.parameters[choice] = target.unwrap_or(u32::MAX);
                event.effects.push(EffectReference {
                    choice,
                    name: name.clone(),
                    target,
                });
            }
            event.parameters[5] = word(
                bytes
                    .len()
                    .checked_sub(4)
                    .ok_or(Error("truncated effect flags"))?,
            )?;
        } else {
            event.parameters[0] = word(5)?;
            event.parameters[1] = if record.opcode == 0x56 || record.opcode == 0x57 {
                word(5)?
            } else {
                word(9)?
            };
            event.kind = match record.opcode {
                0x56 => 1,
                0x57 => 3,
                0x58 => 9,
                0x5c => 0x10001,
                _ => return Err(Error("unsupported event kind")),
            };
        }
        track.events.push(event);
        if record.opcode == 0x57 {
            let value = word(9)?;
            track.events.push(Event {
                source_offset: record.offset,
                kind: 5,
                time,
                parameters: [value, value, 0, 0, 0, 0],
                effects: Vec::new(),
            });
        }
    }
    Ok(tracks)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn effects() -> Image {
        let mut raw = Vec::new();
        for word in [1_u32, 7, 1] {
            raw.extend(word.to_le_bytes());
        }
        raw.extend(b"a\0");
        raw.extend(6_u32.to_le_bytes());
        raw.push(2);
        giants_formats::effect_image::parse(&raw).unwrap()
    }
    #[test]
    fn event_order_and_pair_expansion_preserve_source_order() {
        let mut raw = vec![0x52, 1, b'a', 0, 0x57];
        for word in [30_u32, 7, 9] {
            raw.extend(word.to_le_bytes());
        }
        raw.push(0x56);
        for word in [0_u32, 3] {
            raw.extend(word.to_le_bytes());
        }
        raw.push(255);
        let p = giants_formats::object_spec::parse(&raw).unwrap();
        let t = prepare(&p, &effects(), Precision::Double53).unwrap();
        assert_eq!(
            t[0].events.iter().map(|e| e.kind).collect::<Vec<_>>(),
            [3, 5, 1]
        );
        assert_eq!(t[0].events[0].time.to_bits(), t[0].events[1].time.to_bits());
        assert_eq!(t[0].events[0].parameters, [7, 7, 0, 0, 0, 0]);
        assert_eq!(t[0].events[1].parameters, [9, 9, 0, 0, 0, 0]);
        assert!(t[0].events[0].time > t[0].events[2].time);
    }
    #[test]
    fn orphan_and_excess_events_are_rejected_without_losing_earlier_entries() {
        let mut raw = vec![0x56];
        raw.extend([0_u8; 8]);
        raw.push(255);
        assert!(
            prepare(
                &giants_formats::object_spec::parse(&raw).unwrap(),
                &effects(),
                Precision::Double53
            )
            .is_err()
        );
        let mut raw = vec![0x52, 1, b'a', 0];
        for _ in 0..33 {
            raw.push(0x56);
            raw.extend([0_u8; 8]);
        }
        raw.push(255);
        assert!(
            prepare(
                &giants_formats::object_spec::parse(&raw).unwrap(),
                &effects(),
                Precision::Double53
            )
            .is_err()
        );
    }
}
