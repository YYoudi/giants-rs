//! Measured writes into a supplied mission-state slot. No completion condition,
//! timer, cinematic playback or runtime readiness is executed here.
use giants_formats::{
    mission::{Mission, SlotWriteKind},
    world::WorldConfig,
};
use std::ops::Range;

pub const SLOT_SIZE: usize = 0x118;

/// Exact reference layout; the state-2 component word differs between builds.
#[derive(Clone, Copy)]
pub enum ReferenceLayout {
    Debug,
    Gog,
}

/// Explicitly supplied native current-actor state and component storage.
/// This is independent of the last placement in a serialized mission.
pub struct Actor36<'a> {
    pub state: u32,
    pub component: Option<&'a mut [u8]>,
}

/// Complete command36 write. `None` explicitly means no current actor;
/// callers with an unknown actor must not substitute `None`.
/// Validate both destinations before modifying either storage.
pub fn apply_flag36(
    data: &mut [u8],
    actor: Option<Actor36<'_>>,
    layout: ReferenceLayout,
) -> Result<(), String> {
    if data.len() < SLOT_SIZE {
        return Err("mission flag36 exceeds supplied slot storage".into());
    }
    let reset = match actor {
        Some(actor) => {
            let offset = match actor.state {
                2 => Some(match layout {
                    ReferenceLayout::Debug => 0x50,
                    ReferenceLayout::Gog => 0x4c,
                }),
                3 => Some(0xcec),
                _ => None,
            };
            if let Some(offset) = offset {
                let component = actor.component.ok_or("mission flag36 component missing")?;
                if component.len() < offset + 4 {
                    return Err("mission flag36 exceeds supplied component storage".into());
                }
                Some((&mut component[offset..offset + 4], 0u32))
            } else {
                None
            }
        }
        None => None,
    };
    if let Some((word, value)) = reset {
        word.copy_from_slice(&value.to_le_bytes());
    }
    data[0x114..0x118].copy_from_slice(&1u32.to_le_bytes());
    Ok(())
}

/// The caller owns initialization and supplies the selected slot's actual bytes.
/// Invalid input leaves storage unchanged; unrelated slot words are retained.
pub fn apply(data: &mut [u8], write: &SlotWriteKind) -> Result<Range<usize>, String> {
    apply_context(data, write, None)
}

/// Command 37 has a companion global write. Requiring explicit storage avoids
/// silently accepting only half of the native command. Both writes are atomic
/// with respect to validation; no readiness or behavior is inferred.
pub fn apply_with_global(
    data: &mut [u8],
    write: &SlotWriteKind,
    global: &mut u32,
) -> Result<Range<usize>, String> {
    apply_context(data, write, Some(global))
}

fn apply_context(
    data: &mut [u8],
    write: &SlotWriteKind,
    global: Option<&mut u32>,
) -> Result<Range<usize>, String> {
    let range = match write {
        SlotWriteKind::PlayerChild12(_) => {
            return Err("mission child12 requires an owned actor-service backend".into());
        }
        SlotWriteKind::PlayerProxy34 => {
            return Err("mission34 requires an explicit actor-service backend".into());
        }
        SlotWriteKind::ExpiryCinematic(_) => 0x90..0xb0,
        SlotWriteKind::Parameter13(_) => 0xd0..0xd4,
        SlotWriteKind::Flag36 => {
            return Err("mission flag36 requires explicit current actor context".into());
        }
        SlotWriteKind::Parameter15(_) => 0x100..0x104,
        SlotWriteKind::CompletionCinematic(_) => 0xd8..0xf8,
        SlotWriteKind::Flag37 => 0x104..0x108,
    };
    if data.len() < range.end {
        return Err("mission slot write exceeds supplied storage".into());
    }
    match write {
        SlotWriteKind::PlayerChild12(_) => unreachable!(),
        SlotWriteKind::PlayerProxy34 => unreachable!(),
        SlotWriteKind::ExpiryCinematic(name) => data[range.clone()].copy_from_slice(name),
        SlotWriteKind::Parameter13(value) => {
            data[range.clone()].copy_from_slice(&value.to_le_bytes())
        }
        SlotWriteKind::Flag36 => unreachable!(),
        SlotWriteKind::Parameter15(value) => {
            data[range.clone()].copy_from_slice(&value.to_le_bytes())
        }
        SlotWriteKind::CompletionCinematic(name) => data[range.clone()].copy_from_slice(name),
        SlotWriteKind::Flag37 => {
            let global =
                global.ok_or("mission flag37 requires explicit companion global storage")?;
            data[range.clone()].copy_from_slice(&1u32.to_le_bytes());
            *global = 1;
        }
    }
    Ok(range)
}

#[derive(Debug, Clone)]
pub struct Projection {
    pub index: usize,
    pub player_child12: Option<(usize, u32)>,
    /// Ordered requests, including repeats. No creation/link/retirement executed.
    pub linked_actors12: Vec<(usize, u32)>,
    /// Raw final command13 store, not a runtime behavior acknowledgement.
    pub parameter13: Option<(usize, u32)>,
    /// Last explicit declaration; absent does not invent a constructor default.
    pub parameter15: Option<(usize, i32)>,
    /// Exact bytes, including an explicit empty request or non-string bytes.
    pub completion_cinematic: Option<(usize, [u8; 32])>,
    /// Last declared slot/global flag request, not an executed game behavior.
    pub flag37: Option<usize>,
    /// Declaration only; no current-actor identity or reset is invented.
    pub flag36: Option<usize>,
    pub expiry_cinematic: Option<(usize, [u8; 32])>,
    /// Declaration only; creation and player callbacks are not executed here.
    pub player_proxy34: Option<usize>,
}

pub fn index(world: &WorldConfig, name: &str) -> Result<usize, String> {
    world
        .mission_names
        .iter()
        .position(|n| n == name)
        .ok_or_else(|| "mission name missing from world state slots".into())
}

pub fn project(world: &WorldConfig, mission: &Mission, name: &str) -> Result<Projection, String> {
    let mut out = Projection {
        index: index(world, name)?,
        player_child12: None,
        linked_actors12: Vec::new(),
        parameter13: None,
        parameter15: None,
        completion_cinematic: None,
        flag37: None,
        flag36: None,
        expiry_cinematic: None,
        player_proxy34: None,
    };
    for write in &mission.slot_writes {
        match &write.kind {
            SlotWriteKind::PlayerChild12(value) => {
                out.player_child12 = Some((write.source_offset, *value));
                out.linked_actors12.push((write.source_offset, *value));
            }
            SlotWriteKind::Parameter13(value) => {
                out.parameter13 = Some((write.source_offset, *value))
            }
            SlotWriteKind::ExpiryCinematic(name) => {
                out.expiry_cinematic = Some((write.source_offset, *name));
            }
            SlotWriteKind::PlayerProxy34 => out.player_proxy34 = Some(write.source_offset),
            SlotWriteKind::Flag36 => out.flag36 = Some(write.source_offset),
            SlotWriteKind::Parameter15(value) => {
                out.parameter15 = Some((write.source_offset, *value))
            }
            SlotWriteKind::CompletionCinematic(name) => {
                out.completion_cinematic = Some((write.source_offset, *name))
            }
            SlotWriteKind::Flag37 => out.flag37 = Some(write.source_offset),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn linked_actor12_never_applies_only_the_slot_half_without_actor_services() {
        let mut data = [0x55; SLOT_SIZE];
        let before = data;
        let mut global = 0xdeadbeef;
        for class_id in [0, 583, 742, u32::MAX] {
            assert!(apply(&mut data, &SlotWriteKind::PlayerChild12(class_id)).is_err());
            assert!(
                apply_with_global(
                    &mut data,
                    &SlotWriteKind::PlayerChild12(class_id),
                    &mut global
                )
                .is_err()
            );
            assert_eq!((data, global), (before, 0xdeadbeef));
        }
    }
    #[test]
    fn child12_is_refused_atomically_while_parameter13_preserves_raw_bits() {
        let mut slot = [0x55; SLOT_SIZE];
        let before = slot;
        assert!(apply(&mut slot, &SlotWriteKind::PlayerChild12(583)).is_err());
        assert_eq!(slot, before);
        assert!(apply(&mut slot[..0xd3], &SlotWriteKind::Parameter13(u32::MAX)).is_err());
        assert_eq!(slot, before);
        assert_eq!(
            apply(&mut slot, &SlotWriteKind::Parameter13(0x80000001)).unwrap(),
            0xd0..0xd4
        );
        assert_eq!(&slot[0xd0..0xd4], &0x80000001u32.to_le_bytes());
        assert_eq!(&slot[..0xd0], &before[..0xd0]);
        assert_eq!(&slot[0xd4..], &before[0xd4..]);
    }
    #[test]
    fn flag36_requires_context_and_validates_both_destinations_before_writing() {
        let mut slot = [0x55; SLOT_SIZE];
        let before = slot;
        let mut component = [0xaa; 0xcf0];
        let original = component;
        assert!(apply(&mut slot, &SlotWriteKind::Flag36).is_err());
        for (state, length) in [(2, 0x53), (3, 0xcef)] {
            assert!(
                apply_flag36(
                    &mut slot,
                    Some(Actor36 {
                        state,
                        component: Some(&mut component[..length]),
                    }),
                    ReferenceLayout::Debug
                )
                .is_err()
            );
            assert_eq!((slot, component), (before, original));
        }
        assert!(
            apply_flag36(
                &mut slot,
                Some(Actor36 {
                    state: 2,
                    component: None
                }),
                ReferenceLayout::Debug
            )
            .is_err()
        );
        assert_eq!(slot, before);
        apply_flag36(&mut slot, None, ReferenceLayout::Debug).unwrap();
        let mut expected = before;
        expected[0x114..0x118].copy_from_slice(&1u32.to_le_bytes());
        assert_eq!(slot, expected);
    }
    #[test]
    fn writes_preserve_other_slot_bytes_and_fail_atomically() {
        let mut bytes = [0x55; SLOT_SIZE];
        apply(&mut bytes, &SlotWriteKind::Parameter13(u32::MAX)).unwrap();
        assert_eq!(&bytes[0xd0..0xd4], &u32::MAX.to_le_bytes());
        let before = bytes;
        assert!(apply(&mut bytes[..0xd3], &SlotWriteKind::Parameter13(0)).is_err());
        assert_eq!(bytes, before);
        apply(&mut bytes, &SlotWriteKind::Parameter15(i32::MIN)).unwrap();
        apply(&mut bytes, &SlotWriteKind::CompletionCinematic([0xff; 32])).unwrap();
        assert_eq!(&bytes[0x100..0x104], &i32::MIN.to_le_bytes());
        assert_eq!(&bytes[0xd8..0xf8], &[0xff; 32]);
        assert_eq!(&bytes[0xf8..0x100], &[0x55; 8]);
        let old = bytes;
        assert!(apply(&mut bytes[..0x103], &SlotWriteKind::Parameter15(1)).is_err());
        assert!(
            apply(
                &mut bytes[..0xf7],
                &SlotWriteKind::CompletionCinematic([0; 32])
            )
            .is_err()
        );
        assert_eq!(bytes, old);
    }

    #[test]
    fn flag_requires_companion_and_preserves_both_storages_on_failure() {
        let mut bytes = [0x55; SLOT_SIZE];
        let before = bytes;
        let mut global = 0xdeadbeef;
        assert!(apply(&mut bytes, &SlotWriteKind::Flag37).is_err());
        assert_eq!(bytes, before);
        assert!(
            apply_with_global(&mut bytes[..0x107], &SlotWriteKind::Flag37, &mut global).is_err()
        );
        assert_eq!((bytes, global), (before, 0xdeadbeef));
        apply_with_global(&mut bytes, &SlotWriteKind::Flag37, &mut global).unwrap();
        assert_eq!(global, 1);
        let mut expected = before;
        expected[0x104..0x108].copy_from_slice(&1u32.to_le_bytes());
        assert_eq!(bytes, expected);
    }
}
