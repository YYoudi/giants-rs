//! Two observed actor predicates over a caller-owned session snapshot. This
//! does not produce the session mode, slot table or live actor ownership.
pub struct Snapshot<'a> {
    pub mode: i32,
    pub declared_count: i32,
    pub current_key: u32,
    pub slots: &'a [u32],
}
fn matching_owner(snapshot: &Snapshot<'_>, owner_word: u32) -> Result<bool, String> {
    let mut selected = snapshot.declared_count.wrapping_sub(1);
    for i in 0..snapshot.declared_count {
        let key = snapshot
            .slots
            .get(i as usize)
            .ok_or("actor predicate required slot absent from snapshot")?;
        if *key == snapshot.current_key {
            selected = i;
            break;
        }
    }
    // The acquired helper tests only the low byte of the XOR, including the
    // count-minus-one fallback when the count is nonpositive or no key matches.
    Ok(owner_word as u8 == selected as u8)
}
pub fn first(snapshot: &Snapshot<'_>, owner_word: u32) -> Result<bool, String> {
    if snapshot.mode != 4 {
        Ok(true)
    } else {
        matching_owner(snapshot, owner_word)
    }
}
pub fn second(snapshot: &Snapshot<'_>, owner_word: u32) -> Result<bool, String> {
    if snapshot.mode != 4 {
        Ok(false)
    } else {
        matching_owner(snapshot, owner_word)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fallback_is_signed_count_minus_one_and_only_compares_the_low_byte() {
        let empty = Snapshot {
            mode: 4,
            declared_count: 0,
            current_key: 10,
            slots: &[],
        };
        assert!(first(&empty, 0x1234_00ff).unwrap());
        assert!(second(&empty, 255).unwrap());
        assert!(!first(&empty, 0).unwrap());
        let negative = Snapshot {
            declared_count: -2,
            ..empty
        };
        assert!(first(&negative, 253).unwrap());
    }
    #[test]
    fn first_match_avoids_unread_slots_and_missing_required_slot_is_not_a_mismatch() {
        let partial = Snapshot {
            mode: 4,
            declared_count: 3,
            current_key: 9,
            slots: &[9],
        };
        assert!(first(&partial, 256).unwrap());
        let missing = Snapshot {
            current_key: 7,
            ..partial
        };
        assert!(first(&missing, 2).is_err());
        let other_mode = Snapshot {
            mode: -1,
            ..missing
        };
        assert!(first(&other_mode, 0).unwrap());
        assert!(!second(&other_mode, 0).unwrap());
    }
}
