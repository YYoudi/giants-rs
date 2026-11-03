//! Measured mission writes into explicitly supplied object state storage.
//! No state transition, callback, target resolution or behavior execution.
use giants_formats::mission::{StateList, StateWriteKind};
use std::ops::Range;

/// Returns the exact overwritten byte ranges. Invalid input leaves storage unchanged.
/// The caller establishes that this is the object's actual state payload.
pub fn apply(data: &mut [u8], write: &StateWriteKind) -> Result<Vec<Range<usize>>, String> {
    apply_in_state(data, None, write)
}

/// The state identifier is supplied explicitly; it is never inferred from size.
pub fn apply_with_state(
    data: &mut [u8],
    state: u32,
    write: &StateWriteKind,
) -> Result<Vec<Range<usize>>, String> {
    apply_in_state(data, Some(state), write)
}

fn apply_in_state(
    data: &mut [u8],
    state: Option<u32>,
    write: &StateWriteKind,
) -> Result<Vec<Range<usize>>, String> {
    let range = match write {
        StateWriteKind::Selector32(_) => {
            match state.ok_or("mission selector32 requires state identity")? {
                51 => 96..100,
                97 => 12..16,
                _ => return Ok(Vec::new()),
            }
        }
        StateWriteKind::Pair10And14(_) => 16..60,
        StateWriteKind::SetWord48 => 72..76,
        StateWriteKind::CountedWords { list, words } => {
            if words.len() > 20 {
                return Err("mission counted state words outside measured profile".into());
            }
            let base = match list {
                StateList::First => 0,
                StateList::Second => 164,
            };
            base..base + 84
        }
        StateWriteKind::Word(_) => 80..84,
        StateWriteKind::PrefixWords(words) => {
            if words.len() > 255 {
                return Err("mission state prefix exceeds byte count".into());
            }
            0..words.len() * 4
        }
    };
    if data.len() < range.end {
        return Err("mission write exceeds actual state storage".into());
    }
    match write {
        StateWriteKind::Selector32(value) => {
            data[range.clone()].copy_from_slice(&u32::from(*value).to_le_bytes());
        }
        StateWriteKind::Pair10And14(words) => {
            data[16..20].copy_from_slice(&words[0].to_le_bytes());
            data[20..24].copy_from_slice(&words[1].to_le_bytes());
            data[56..60].copy_from_slice(&1u32.to_le_bytes());
        }
        StateWriteKind::SetWord48 => data[72..76].copy_from_slice(&1u32.to_le_bytes()),
        StateWriteKind::CountedWords { words, .. } => {
            let base = range.start;
            data[base + 4..base + 84].fill(0xff);
            data[base..base + 4].copy_from_slice(&(words.len() as u32).to_le_bytes());
            for (i, word) in words.iter().enumerate() {
                data[base + 4 + 4 * i..base + 8 + 4 * i].copy_from_slice(&word.to_le_bytes());
            }
        }
        StateWriteKind::Word(word) => data[80..84].copy_from_slice(&word.to_le_bytes()),
        StateWriteKind::PrefixWords(words) => {
            for (i, word) in words.iter().enumerate() {
                data[4 * i..4 * i + 4].copy_from_slice(&word.to_le_bytes());
            }
        }
    }
    Ok(if matches!(write, StateWriteKind::Pair10And14(_)) {
        vec![16..24, 56..60]
    } else {
        vec![range]
    })
}

impl crate::state_storage::Seed {
    /// Overwritten raw words no longer retain their old symbolic class references.
    pub fn apply_mission_write(
        &mut self,
        write: &StateWriteKind,
    ) -> Result<Vec<Range<usize>>, String> {
        if matches!(write, StateWriteKind::PrefixWords(words) if words.is_empty()) {
            return Ok(Vec::new());
        }
        let state = self.storage.state();
        if matches!(write, StateWriteKind::Selector32(_)) && !matches!(state, 51 | 97) {
            return Ok(Vec::new());
        }
        let data = self
            .storage
            .data_mut()
            .ok_or("mission write has no allocated state")?;
        let ranges = apply_with_state(data, u32::from(state), write)?;
        self.unresolved.retain(|r| {
            ranges
                .iter()
                .all(|range| r.offset + 4 <= range.start || r.offset >= range.end)
        });
        Ok(ranges)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selector_requires_identity_and_preserves_unwritten_storage() {
        let write = StateWriteKind::Selector32(255);
        let mut data = [0x55; 128];
        assert!(apply(&mut data, &write).is_err());
        assert_eq!(data, [0x55; 128]);
        assert!(apply_with_state(&mut data[..99], 51, &write).is_err());
        assert_eq!(data, [0x55; 128]);
        assert_eq!(
            apply_with_state(&mut data, 51, &write).unwrap(),
            std::iter::once(96..100).collect::<Vec<_>>()
        );
        assert_eq!(&data[96..100], &255u32.to_le_bytes());
        assert_eq!(&data[..96], &[0x55; 96]);
        assert_eq!(&data[100..], &[0x55; 28]);
        assert_eq!(
            apply_with_state(&mut data, 97, &write).unwrap(),
            std::iter::once(12..16).collect::<Vec<_>>()
        );
        assert!(
            apply_with_state(&mut [], u32::MAX, &write)
                .unwrap()
                .is_empty()
        );
    }
    #[test]
    fn ignored_selector_needs_no_allocated_payload() {
        let program = giants_formats::class_program::parse(&[0xff]).unwrap();
        let mut seed = crate::state_storage::Seed::from_program(&program).unwrap();
        assert!(
            seed.apply_mission_write(&StateWriteKind::Selector32(1))
                .unwrap()
                .is_empty()
        );
    }
    #[test]
    fn pair_write_preserves_gap_and_is_atomic_on_short_storage() {
        let mut data = [0x55; 100];
        let write = StateWriteKind::Pair10And14([0x7fc01234, 0x80000000]);
        let before = data;
        assert!(apply(&mut data[..59], &write).is_err());
        assert_eq!(data, before);
        let ranges = apply(&mut data, &write).unwrap();
        assert_eq!(ranges, vec![16..24, 56..60]);
        assert_eq!(&data[16..20], &0x7fc01234u32.to_le_bytes());
        assert_eq!(&data[20..24], &0x80000000u32.to_le_bytes());
        assert_eq!(&data[24..56], &before[24..56]);
        assert_eq!(&data[56..60], &1u32.to_le_bytes());
    }
    #[test]
    fn flag_write_is_bounded_and_keeps_order_with_counted_state_words() {
        let mut data = [0x55; 100];
        apply(&mut data, &StateWriteKind::SetWord48).unwrap();
        assert_eq!(&data[72..76], &1u32.to_le_bytes());
        let before = data;
        assert!(apply(&mut data[..75], &StateWriteKind::SetWord48).is_err());
        assert_eq!(data, before);
        apply(
            &mut data,
            &StateWriteKind::CountedWords {
                list: StateList::First,
                words: vec![],
            },
        )
        .unwrap();
        assert_eq!(&data[72..76], &[255; 4]);
        apply(&mut data, &StateWriteKind::SetWord48).unwrap();
        assert_eq!(&data[72..76], &1u32.to_le_bytes());
        assert_eq!(&data[84..], &[0x55; 16]);
    }
    #[test]
    fn prefix_preserves_tail_and_empty_copy_needs_no_allocation() {
        let mut bytes = [0x55; 16];
        assert_eq!(
            apply(
                &mut bytes,
                &StateWriteKind::PrefixWords(vec![0x7fc01234, 9])
            )
            .unwrap(),
            std::iter::once(0..8).collect::<Vec<_>>()
        );
        assert_eq!(&bytes[..4], &0x7fc01234u32.to_le_bytes());
        assert_eq!(&bytes[8..], &[0x55; 8]);
        let old = bytes;
        assert!(apply(&mut bytes, &StateWriteKind::PrefixWords(vec![0; 5])).is_err());
        assert!(apply(&mut bytes, &StateWriteKind::PrefixWords(vec![0; 256])).is_err());
        assert_eq!(bytes, old);
        let program = giants_formats::class_program::parse(&[0xff]).unwrap();
        let mut seed = crate::state_storage::Seed::from_program(&program).unwrap();
        assert!(seed.storage.data().is_none());
        assert!(
            seed.apply_mission_write(&StateWriteKind::PrefixWords(vec![]))
                .unwrap()
                .is_empty()
        );
        assert!(seed.storage.data().is_none());
    }
    #[test]
    fn ordering_overlap_and_bounds_are_explicit() {
        let mut data = [0x55; 100];
        apply(&mut data, &StateWriteKind::Word(7)).unwrap();
        apply(
            &mut data,
            &StateWriteKind::CountedWords {
                list: StateList::First,
                words: vec![3, 4],
            },
        )
        .unwrap();
        assert_eq!(&data[..12], &[2, 0, 0, 0, 3, 0, 0, 0, 4, 0, 0, 0]);
        assert_eq!(&data[12..84], &[255; 72]);
        assert_eq!(&data[84..], &[0x55; 16]);
        apply(&mut data, &StateWriteKind::Word(8)).unwrap();
        assert_eq!(&data[80..84], &8u32.to_le_bytes());
        let old = data;
        assert!(apply(&mut data[..83], &StateWriteKind::Word(9)).is_err());
        assert!(
            apply(
                &mut data,
                &StateWriteKind::CountedWords {
                    list: StateList::Second,
                    words: vec![],
                }
            )
            .is_err()
        );
        assert!(
            apply(
                &mut data,
                &StateWriteKind::CountedWords {
                    list: StateList::First,
                    words: vec![0; 21]
                }
            )
            .is_err()
        );
        assert_eq!(data, old);
    }
}
