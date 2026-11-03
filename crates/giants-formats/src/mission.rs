//! Bounded version-four mission placement stream. No mission VM execution.
//! Measured command profiles only; declarations do not execute the mission VM.
use crate::{Error, slice, u32_at};

#[derive(Debug, Clone)]
pub struct Placement {
    pub source_offset: usize,
    pub object_type: u32,
    pub position: [f32; 3],
    /// Yaw in degrees. Command 3 omits pitch/roll; command 4 supplies them.
    pub yaw: f32,
    pub pitch_roll: Option<[f32; 2]>,
    /// Absolute override, not a multiplier of the class default.
    pub scale: Option<f32>,
}

#[derive(Debug)]
pub struct Mission {
    pub commands: Vec<(usize, u8)>,
    pub placements: Vec<Placement>,
    pub locked_children: Vec<LockedChild>,
    pub orientations: Vec<Orientation>,
    pub object_words: Vec<ObjectWords>,
    /// Command20: named Flick exclusions, retained without running the loader.
    pub cinematic_exclusions: Vec<CinematicExclusion>,
    pub state_writes: Vec<StateWrite>,
    pub object_properties: Vec<ObjectProperty>,
    pub opening_cinematics: Vec<OpeningCinematic>,
    /// Writes into the selected world mission slot, not the current object.
    pub slot_writes: Vec<SlotWrite>,
    pub object_selectors: Vec<ObjectSelector>,
    /// Command 11, retained in source order without running continuity or spawn.
    pub starts: Vec<Start>,
}
#[derive(Debug, Clone)]
pub struct Start {
    pub source_offset: usize,
    pub words: [u32; 4],
}
#[derive(Debug, Clone)]
pub struct ObjectSelector {
    pub source_offset: usize,
    pub object: ObjectRef,
    /// Command 5 writes the unsigned byte to object +0xe8.
    pub value: u8,
}

#[derive(Debug, Clone)]
pub struct SlotWrite {
    pub source_offset: usize,
    pub kind: SlotWriteKind,
}
#[derive(Debug, Clone)]
pub enum SlotWriteKind {
    /// Command12 declares a class and constructs/links a child of the player.
    /// Its actor-service side effects must not be reduced to a slot write.
    PlayerChild12(u32),
    /// Command13 copies a raw word to the mission slot +0xd0.
    Parameter13(u32),
    /// Command 15: native signed integer conversion, stored at slot +0x100.
    /// Its downstream gameplay meaning remains unresolved.
    Parameter15(i32),
    /// Command 25: exact 32-byte request at slot +0xd8. No transition executed.
    CompletionCinematic([u8; 32]),
    /// Command19: exact 32-byte request at slot+0x90; no timer is executed.
    ExpiryCinematic([u8; 32]),
    /// Command34: conditional resident retirement, creation and player linking,
    /// followed by slot+0x110=1. Requires an explicit actor-service backend.
    PlayerProxy34,
    /// Command 36: slot +0x114 becomes one; a supplied current actor may reset
    /// a component word. Reading this declaration does not execute that reset.
    Flag36,
    /// Command 37: slot +0x104 and a companion global word are set to one.
    Flag37,
}

/// Command 21 copies all 32 bytes into the selected mission slot. A zero first
/// byte and an absent command are distinct; string interpretation is a consumer concern.
#[derive(Debug)]
pub struct OpeningCinematic {
    pub source_offset: usize,
    pub name: [u8; 32],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectRef {
    Placement(usize),
    LockedChild(usize),
}
/// Command 22 changes the current object. The two byte selectors are preserved
/// without assuming a model-slot or attachment interpretation.
#[derive(Debug)]
pub struct LockedChild {
    pub source_offset: usize,
    pub parent: ObjectRef,
    pub object_type: u32,
    pub selectors: [u8; 2],
    pub scale: Option<f32>,
}
#[derive(Debug)]
pub struct Orientation {
    pub source_offset: usize,
    pub object: ObjectRef,
    pub angles: [f32; 3],
}
/// Command 6 copies these three words into the current object. Their runtime
/// interpretation is intentionally left unresolved by the data reader.
#[derive(Debug)]
pub struct ObjectWords {
    pub source_offset: usize,
    pub object: ObjectRef,
    pub words: [u32; 3],
}
#[derive(Debug)]
pub struct CinematicExclusion {
    pub source_offset: usize,
    pub object: ObjectRef,
    pub name: [u8; 32],
}
/// Command 7 writes the object word at +0x174. Its behavior meaning is unknown.
#[derive(Debug)]
pub struct ObjectProperty {
    pub source_offset: usize,
    pub object: ObjectRef,
    pub value: u32,
}
#[derive(Clone, Debug)]
pub struct StateWrite {
    pub source_offset: usize,
    pub object: ObjectRef,
    pub kind: StateWriteKind,
}
#[derive(Clone, Debug)]
pub enum StateWriteKind {
    /// Command32 widens one byte; its destination depends on the current state.
    /// State51 writes +0x60, state97 writes +0xc; other states do not write.
    Selector32(u8),
    /// Command26 copies raw words to state +0x10/+0x14 and sets +0x38 to1.
    /// Raw bits are preserved; downstream behavior remains unverified.
    Pair10And14([u32; 2]),
    /// Command 28 sets the current object's state word +0x48 to one.
    /// No behavior meaning or callback is inferred by this write.
    SetWord48,
    /// Commands 30/31, nonnegative count in the measured 0..=20 profile.
    CountedWords { list: StateList, words: Vec<u32> },
    /// Command 33; interpretation of the state word remains unspecified.
    Word(u32),
    /// Command 10 copies this prefix without storing its count or clearing a tail.
    PrefixWords(Vec<u32>),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateList {
    First,
    Second,
}

fn scalar(data: &[u8], offset: usize) -> Result<f32, Error> {
    let value = f32::from_bits(u32_at(data, offset)?);
    value
        .is_finite()
        .then_some(value)
        .ok_or(Error("non-finite mission scalar"))
}

pub fn parse(data: &[u8]) -> Result<Mission, Error> {
    if data.len() > 4 * 1024 * 1024 || u32_at(data, 0)? != 4 {
        return Err(Error("unsupported mission framing"));
    }
    if slice(data, 4, 1)? != [1] {
        return Err(Error("missing mission start"));
    }
    let mut result = Mission {
        commands: Vec::new(),
        placements: Vec::new(),
        locked_children: Vec::new(),
        orientations: Vec::new(),
        object_words: Vec::new(),
        cinematic_exclusions: Vec::new(),
        state_writes: Vec::new(),
        object_properties: Vec::new(),
        opening_cinematics: Vec::new(),
        slot_writes: Vec::new(),
        object_selectors: Vec::new(),
        starts: Vec::new(),
    };
    let mut current = None;
    let mut parents = Vec::new();
    let mut at = 4;
    while at < data.len() {
        if result.commands.len() >= 8192 {
            return Err(Error("mission command budget exceeded"));
        }
        let opcode = data[at];
        let size = match opcode {
            1 if at == 4 => 1,
            2 => 1,
            3 => 21,
            4 => 29,
            6 => 13,
            5 | 8 | 32 => 2,
            7 | 9 | 12 | 13 | 15 | 16 | 17 => 5,
            10 => 2 + usize::from(slice(data, at + 1, 1)?[0]) * 4,
            11 => 17,
            14 | 30 | 31 => {
                let count = u32_at(data, at + 1)? as usize;
                if count > 20 {
                    return Err(Error("mission array budget exceeded"));
                }
                5 + count * 4
            }
            19 | 20 | 21 | 25 => 33,
            22 => 7,
            23 | 28 | 34 | 36 | 37 => 1,
            24 => 13,
            33 => 5,
            26 | 38 => 9,
            _ => return Err(Error("unsupported mission opcode")),
        };
        let record = slice(data, at, size)?;
        result.commands.push((at, opcode));
        match opcode {
            12 | 13 => result.slot_writes.push(SlotWrite {
                source_offset: at,
                kind: if opcode == 12 {
                    SlotWriteKind::PlayerChild12(u32_at(record, 1)?)
                } else {
                    SlotWriteKind::Parameter13(u32_at(record, 1)?)
                },
            }),
            19 => result.slot_writes.push(SlotWrite {
                source_offset: at,
                kind: SlotWriteKind::ExpiryCinematic(record[1..33].try_into().unwrap()),
            }),
            34 => result.slot_writes.push(SlotWrite {
                source_offset: at,
                kind: SlotWriteKind::PlayerProxy34,
            }),
            36 => result.slot_writes.push(SlotWrite {
                source_offset: at,
                kind: SlotWriteKind::Flag36,
            }),
            37 => result.slot_writes.push(SlotWrite {
                source_offset: at,
                kind: SlotWriteKind::Flag37,
            }),
            32 => result.state_writes.push(StateWrite {
                source_offset: at,
                object: current.ok_or(Error("mission state selector without object"))?,
                kind: StateWriteKind::Selector32(record[1]),
            }),
            26 => result.state_writes.push(StateWrite {
                source_offset: at,
                object: current.ok_or(Error("mission state pair without object"))?,
                kind: StateWriteKind::Pair10And14([u32_at(record, 1)?, u32_at(record, 5)?]),
            }),
            5 => result.object_selectors.push(ObjectSelector {
                source_offset: at,
                object: current.ok_or(Error("mission selector without object"))?,
                value: record[1],
            }),
            11 => result.starts.push(Start {
                source_offset: at,
                words: [
                    u32_at(record, 1)?,
                    u32_at(record, 5)?,
                    u32_at(record, 9)?,
                    u32_at(record, 13)?,
                ],
            }),
            15 | 25 => result.slot_writes.push(SlotWrite {
                source_offset: at,
                kind: if opcode == 15 {
                    SlotWriteKind::Parameter15(u32_at(record, 1)? as i32)
                } else {
                    SlotWriteKind::CompletionCinematic(record[1..33].try_into().unwrap())
                },
            }),
            28 => result.state_writes.push(StateWrite {
                source_offset: at,
                object: current.ok_or(Error("mission state flag without object"))?,
                kind: StateWriteKind::SetWord48,
            }),
            20 => result.cinematic_exclusions.push(CinematicExclusion {
                source_offset: at,
                object: current.ok_or(Error("mission cinematic exclusion without object"))?,
                name: record[1..33].try_into().unwrap(),
            }),
            21 => result.opening_cinematics.push(OpeningCinematic {
                source_offset: at,
                name: record[1..33].try_into().unwrap(),
            }),
            3 | 4 => {
                current = Some(ObjectRef::Placement(result.placements.len()));
                result.placements.push(Placement {
                    source_offset: at,
                    object_type: u32_at(record, 1)?,
                    position: [scalar(record, 5)?, scalar(record, 9)?, scalar(record, 13)?],
                    yaw: scalar(record, 17)?,
                    pitch_roll: if opcode == 4 {
                        Some([scalar(record, 21)?, scalar(record, 25)?])
                    } else {
                        None
                    },
                    scale: None,
                });
            }
            6 => {
                result.object_words.push(ObjectWords {
                    source_offset: at,
                    object: current.ok_or(Error("mission words without object"))?,
                    words: [u32_at(record, 1)?, u32_at(record, 5)?, u32_at(record, 9)?],
                });
            }
            7 => result.object_properties.push(ObjectProperty {
                source_offset: at,
                object: current.ok_or(Error("mission property without object"))?,
                value: u32_at(record, 1)?,
            }),
            10 => {
                let count = usize::from(record[1]);
                result.state_writes.push(StateWrite {
                    source_offset: at,
                    object: current.ok_or(Error("mission prefix without object"))?,
                    kind: StateWriteKind::PrefixWords(
                        (0..count)
                            .map(|i| u32_at(record, 2 + i * 4))
                            .collect::<Result<_, _>>()?,
                    ),
                });
            }
            30 | 31 | 33 => {
                let kind = if opcode != 33 {
                    let count = u32_at(record, 1)? as usize;
                    StateWriteKind::CountedWords {
                        list: if opcode == 30 {
                            StateList::First
                        } else {
                            StateList::Second
                        },
                        words: (0..count)
                            .map(|i| u32_at(record, 5 + i * 4))
                            .collect::<Result<_, _>>()?,
                    }
                } else {
                    StateWriteKind::Word(u32_at(record, 1)?)
                };
                result.state_writes.push(StateWrite {
                    source_offset: at,
                    object: current.ok_or(Error("mission state write without object"))?,
                    kind,
                });
            }
            9 => {
                let scale = Some(scalar(record, 1)?);
                match current.ok_or(Error("mission scale without object"))? {
                    ObjectRef::Placement(i) => result.placements[i].scale = scale,
                    ObjectRef::LockedChild(i) => result.locked_children[i].scale = scale,
                }
            }
            22 => {
                let parent = current.ok_or(Error("mission child without parent"))?;
                // The native error-recovery branch beyond eight is not this profile.
                if parents.len() >= 8 {
                    return Err(Error("mission child nesting outside supported profile"));
                }
                parents.push(parent);
                current = Some(ObjectRef::LockedChild(result.locked_children.len()));
                result.locked_children.push(LockedChild {
                    source_offset: at,
                    parent,
                    object_type: u32_at(record, 1)?,
                    selectors: [record[5], record[6]],
                    scale: None,
                });
            }
            23 => {
                current = Some(
                    parents
                        .pop()
                        .ok_or(Error("mission child stack underflow"))?,
                );
            }
            24 => {
                result.orientations.push(Orientation {
                    source_offset: at,
                    object: current.ok_or(Error("mission orientation without object"))?,
                    angles: [scalar(record, 1)?, scalar(record, 5)?, scalar(record, 9)?],
                });
            }
            2 => {
                if at + size != data.len() {
                    return Err(Error("trailing mission bytes"));
                }
                return Ok(result);
            }
            _ => {}
        }
        at += size;
    }
    Err(Error("missing mission end"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn linked_actor12_keeps_repeated_unsigned_requests_without_object_scope() {
        let mut data = vec![4, 0, 0, 0, 1, 12];
        data.extend(583u32.to_le_bytes());
        data.push(12);
        data.extend(u32::MAX.to_le_bytes());
        data.push(2);
        let mission = parse(&data).unwrap();
        assert!(mission.placements.is_empty());
        assert_eq!(mission.slot_writes.len(), 2);
        for (write, (offset, class_id)) in
            mission.slot_writes.iter().zip([(5, 583), (10, u32::MAX)])
        {
            assert_eq!(write.source_offset, offset);
            assert!(matches!(write.kind, SlotWriteKind::PlayerChild12(value) if value == class_id));
        }
        for end in 0..data.len() {
            assert!(parse(&data[..end]).is_err());
        }
    }
    #[test]
    fn child_request_payload_is_not_interpreted_as_embedded_opcodes() {
        let raw = [4, 0, 0, 0, 1, 12, 2, 12, 13, 2, 13, 255, 255, 255, 255, 2];
        let parsed = parse(&raw).unwrap();
        assert_eq!(parsed.commands, vec![(4, 1), (5, 12), (10, 13), (15, 2)]);
        assert!(matches!(
            parsed.slot_writes[0].kind,
            SlotWriteKind::PlayerChild12(0x020d0c02)
        ));
        assert!(matches!(
            parsed.slot_writes[1].kind,
            SlotWriteKind::Parameter13(u32::MAX)
        ));
        for end in 0..raw.len() {
            assert!(parse(&raw[..end]).is_err());
        }
    }
    #[test]
    fn selector_scope_and_start_record_are_separate_from_object_state() {
        let mut bytes = sample();
        bytes.pop();
        bytes.extend([5, 1, 22]);
        bytes.extend(73u32.to_le_bytes());
        bytes.extend([0, 0, 5, 255, 23, 5, 2, 11]);
        for word in [2u32, 7, 1, u32::MAX] {
            bytes.extend(word.to_le_bytes());
        }
        bytes.push(2);
        let parsed = parse(&bytes).unwrap();
        assert_eq!(
            parsed
                .object_selectors
                .iter()
                .map(|s| (s.object, s.value))
                .collect::<Vec<_>>(),
            [
                (ObjectRef::Placement(0), 1),
                (ObjectRef::LockedChild(0), 255),
                (ObjectRef::Placement(0), 2)
            ]
        );
        assert_eq!(parsed.starts[0].words, [2, 7, 1, u32::MAX]);
        assert!(parse(&[4, 0, 0, 0, 1, 5, 1, 2]).is_err());
        for end in 0..bytes.len() {
            assert!(parse(&bytes[..end]).is_err());
        }
    }
    #[test]
    fn flag37_has_no_payload_and_does_not_require_an_object() {
        let data = [4, 0, 0, 0, 1, 37, 37, 2];
        let mission = parse(&data).unwrap();
        assert_eq!(mission.commands, [(4, 1), (5, 37), (6, 37), (7, 2)]);
        assert!(mission.placements.is_empty());
        assert_eq!(mission.slot_writes.len(), 2);
        assert!(
            mission
                .slot_writes
                .iter()
                .all(|w| matches!(w.kind, SlotWriteKind::Flag37))
        );
        for end in 0..data.len() {
            assert!(parse(&data[..end]).is_err());
        }
        assert!(parse(&[4, 0, 0, 0, 1, 37, 0, 2]).is_err());
    }
    #[test]
    fn slot_writes_are_independent_of_object_scope_and_flag_keeps_its_owner() {
        let mut bytes = vec![4, 0, 0, 0, 1, 15];
        bytes.extend(i32::MIN.to_le_bytes());
        bytes.push(25);
        bytes.extend([0xff; 32]);
        bytes.extend(&sample()[5..sample().len() - 1]);
        bytes.push(28);
        bytes.push(22);
        bytes.extend(73u32.to_le_bytes());
        bytes.extend([0, 1, 28, 23, 28, 2]);
        let mission = parse(&bytes).unwrap();
        assert!(matches!(
            mission.slot_writes[0].kind,
            SlotWriteKind::Parameter15(i32::MIN)
        ));
        match &mission.slot_writes[1].kind {
            SlotWriteKind::CompletionCinematic(name) => assert_eq!(*name, [0xff; 32]),
            _ => panic!("expected raw completion request"),
        }
        assert_eq!(
            mission
                .state_writes
                .iter()
                .map(|w| w.object)
                .collect::<Vec<_>>(),
            [
                ObjectRef::Placement(0),
                ObjectRef::LockedChild(0),
                ObjectRef::Placement(0)
            ]
        );
        assert!(parse(&[4, 0, 0, 0, 1, 28, 2]).is_err());
        for end in 0..bytes.len() {
            assert!(parse(&bytes[..end]).is_err());
        }
    }
    #[test]
    fn startup_copy_preserves_order_empty_and_non_string_bytes() {
        let mut bytes = vec![4, 0, 0, 0, 1, 21];
        let first = [0xff; 32];
        bytes.extend(first);
        bytes.push(21);
        let mut second = [0xa5; 32];
        second[0] = 0;
        bytes.extend(second);
        bytes.push(2);
        let mission = parse(&bytes).unwrap();
        assert!(mission.placements.is_empty());
        assert_eq!(mission.opening_cinematics.len(), 2);
        assert_eq!(mission.opening_cinematics[0].name, first);
        assert_eq!(mission.opening_cinematics[1].name, second);
        assert_eq!(mission.opening_cinematics[1].source_offset, 38);
        for end in 0..bytes.len() {
            assert!(parse(&bytes[..end]).is_err());
        }
    }
    fn sample() -> Vec<u8> {
        let mut bytes = 4u32.to_le_bytes().to_vec();
        bytes.extend([1, 3]);
        bytes.extend(42u32.to_le_bytes());
        for value in [1f32, -2., 3., 90.] {
            bytes.extend(value.to_le_bytes());
        }
        bytes.push(9);
        bytes.extend(1.5f32.to_le_bytes());
        bytes.push(2);
        bytes
    }

    #[test]
    fn state_selector_keeps_current_object_scope_and_one_byte_payload() {
        let mut bytes = sample();
        bytes.pop();
        bytes.extend([32, 255, 22]);
        bytes.extend(73u32.to_le_bytes());
        bytes.extend([0, 1, 32, 0, 23, 32, 127, 2]);
        let mission = parse(&bytes).unwrap();
        assert_eq!(
            mission
                .state_writes
                .iter()
                .map(|w| w.object)
                .collect::<Vec<_>>(),
            vec![
                ObjectRef::Placement(0),
                ObjectRef::LockedChild(0),
                ObjectRef::Placement(0)
            ]
        );
        assert!(matches!(
            mission.state_writes[0].kind,
            StateWriteKind::Selector32(255)
        ));
        assert!(matches!(
            mission.state_writes[1].kind,
            StateWriteKind::Selector32(0)
        ));
        assert!(matches!(
            mission.state_writes[2].kind,
            StateWriteKind::Selector32(127)
        ));
        for end in 0..bytes.len() {
            assert!(parse(&bytes[..end]).is_err());
        }
        assert!(parse(&[4, 0, 0, 0, 1, 32, 1, 2]).is_err());
    }
    #[test]
    fn placement_override_and_strict_framing() {
        let bytes = sample();
        let mission = parse(&bytes).unwrap();
        assert_eq!(mission.commands, [(4, 1), (5, 3), (26, 9), (31, 2)]);
        let object = &mission.placements[0];
        assert_eq!(object.object_type, 42);
        assert_eq!(object.position, [1., -2., 3.]);
        assert_eq!(object.yaw, 90.);
        assert_eq!(object.scale, Some(1.5));
        for end in 0..bytes.len() {
            assert!(parse(&bytes[..end]).is_err());
        }
        let mut invalid = bytes.clone();
        invalid.push(0);
        assert!(parse(&invalid).is_err());
        let mut invalid = bytes.clone();
        invalid[5] = 255;
        assert!(parse(&invalid).is_err());
        let mut invalid = bytes.clone();
        invalid[10..14].copy_from_slice(&f32::NAN.to_le_bytes());
        assert!(parse(&invalid).is_err());
        assert!(parse(&[4, 0, 0, 0, 1, 9, 0, 0, 128, 63, 2]).is_err());
        assert!(parse(&[4, 0, 0, 0, 1, 14, 21, 0, 0, 0, 2]).is_err());
    }
    #[test]
    fn child_scope_restores_parent_and_keeps_modifiers_on_their_owner() {
        let mut bytes = sample();
        bytes.pop();
        bytes.push(22);
        bytes.extend(73u32.to_le_bytes());
        bytes.extend([255, 7, 9]);
        bytes.extend(2f32.to_le_bytes());
        bytes.push(24);
        for v in [90f32, -30., 5.] {
            bytes.extend(v.to_le_bytes());
        }
        bytes.extend([23, 9]);
        bytes.extend(3f32.to_le_bytes());
        bytes.push(2);
        let mission = parse(&bytes).unwrap();
        assert_eq!(mission.placements[0].scale, Some(3.));
        let child = &mission.locked_children[0];
        assert_eq!(child.parent, ObjectRef::Placement(0));
        assert_eq!(child.selectors, [255, 7]);
        assert_eq!(child.scale, Some(2.));
        assert_eq!(mission.orientations[0].object, ObjectRef::LockedChild(0));
        assert_eq!(mission.orientations[0].angles, [90., -30., 5.]);
        for end in 32..bytes.len() {
            assert!(parse(&bytes[..end]).is_err());
        }
        assert!(parse(&[4, 0, 0, 0, 1, 23, 2]).is_err());
        assert!(parse(&[4, 0, 0, 0, 1, 22, 1, 0, 0, 0, 0, 0, 2]).is_err());
    }
    #[test]
    fn full_pose_and_raw_words_preserve_bits_and_current_owner() {
        let mut bytes = 4u32.to_le_bytes().to_vec();
        bytes.extend([1, 4]);
        bytes.extend(42u32.to_le_bytes());
        for value in [1f32, -2., 3., 90., -30., 5.] {
            bytes.extend(value.to_le_bytes());
        }
        bytes.push(6);
        let words = [0x7fc01234u32, 0xffffffff, 0x80000000];
        for word in words {
            bytes.extend(word.to_le_bytes());
        }
        bytes.push(2);
        let mission = parse(&bytes).unwrap();
        assert_eq!(mission.commands, [(4, 1), (5, 4), (34, 6), (47, 2)]);
        assert_eq!(mission.placements[0].pitch_roll, Some([-30., 5.]));
        assert_eq!(mission.object_words[0].object, ObjectRef::Placement(0));
        assert_eq!(mission.object_words[0].words, words);
        for end in 0..bytes.len() {
            assert!(parse(&bytes[..end]).is_err());
        }
        bytes[26..30].copy_from_slice(&f32::NAN.to_le_bytes());
        assert!(parse(&bytes).is_err());
        assert!(parse(&[4, 0, 0, 0, 1, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2]).is_err());
    }
    #[test]
    fn cinematic_exclusion_retains_raw_name_and_current_owner() {
        let mut bytes = 4u32.to_le_bytes().to_vec();
        bytes.extend([1, 3]);
        bytes.extend(590u32.to_le_bytes());
        bytes.extend([0; 16]);
        bytes.push(20);
        let name = [b'X'; 32];
        bytes.extend(name);
        bytes.push(2);
        let mission = parse(&bytes).unwrap();
        assert_eq!(
            mission.cinematic_exclusions[0].object,
            ObjectRef::Placement(0)
        );
        assert_eq!(mission.cinematic_exclusions[0].name, name);
        assert_eq!(mission.cinematic_exclusions[0].source_offset, 26);
        for end in 27..bytes.len() {
            assert!(parse(&bytes[..end]).is_err());
        }
    }
    #[test]
    fn state_write_counts_and_owners_remain_bounded() {
        let mut bytes = sample();
        bytes.pop();
        bytes.extend([30, 0, 0, 0, 0, 33]);
        bytes.extend(u32::MAX.to_le_bytes());
        bytes.push(2);
        let mission = parse(&bytes).unwrap();
        assert_eq!(mission.state_writes.len(), 2);
        assert_eq!(mission.state_writes[0].object, ObjectRef::Placement(0));
        assert!(
            matches!(&mission.state_writes[0].kind, StateWriteKind::CountedWords { words, .. } if words.is_empty())
        );
        for end in 32..bytes.len() {
            assert!(parse(&bytes[..end]).is_err());
        }
        bytes[32..36].copy_from_slice(&21u32.to_le_bytes());
        assert!(parse(&bytes).is_err());
        assert!(parse(&[4, 0, 0, 0, 1, 33, 0, 0, 0, 0, 2]).is_err());
    }
    #[test]
    fn prefix_byte_count_and_object_word_are_preserved() {
        let mut bytes = sample();
        bytes.pop();
        bytes.push(7);
        bytes.extend(u32::MAX.to_le_bytes());
        bytes.extend([10, 255]);
        for i in 0..255u32 {
            bytes.extend(i.to_le_bytes());
        }
        bytes.extend([10, 0, 2]);
        let mission = parse(&bytes).unwrap();
        assert_eq!(mission.object_properties[0].value, u32::MAX);
        assert_eq!(mission.object_properties[0].object, ObjectRef::Placement(0));
        assert!(
            matches!(&mission.state_writes[0].kind, StateWriteKind::PrefixWords(v) if v.len()==255 && v[254]==254)
        );
        assert!(
            matches!(&mission.state_writes[1].kind, StateWriteKind::PrefixWords(v) if v.is_empty())
        );
        for end in 32..bytes.len() {
            assert!(parse(&bytes[..end]).is_err());
        }
        assert!(parse(&[4, 0, 0, 0, 1, 7, 0, 0, 0, 0, 2]).is_err());
    }
}
