//! Admission of ordinary scene objects by a director, before common renderer
//! gates. Explicit current fields are inputs; no constructor or callback runs.
use crate::arithmetic::Precision;

pub const INITIAL_RANGE: f32 = 1000.;

pub struct Input {
    pub class: i32,
    pub state: u32,
    pub flick_slot: i32,
    pub exclusion_words: [u32; 3],
    pub position: [f32; 3],
    pub eye: [f32; 3],
    pub range: f32,
}

/// The fourth slot group shares the third word in the measured reference.
/// A negative/out-of-range director slot has no exclusion bit.
pub fn exclusion_bit(slot: i32) -> Option<(usize, u32)> {
    (0..128)
        .contains(&slot)
        .then(|| ((slot as usize / 32).min(2), 1 << (slot % 32)))
}

/// Resolve source names using the first case-insensitive world-table match.
/// Missing names remain explicit errors, not guessed slots or empty masks.
pub fn exclusion_words(
    names: &[[u8; 32]],
    table: &[giants_formats::world::WorldCinematic],
) -> Result<[u32; 3], &'static str> {
    let mut words = [0; 3];
    for raw in names {
        let end = raw
            .iter()
            .position(|byte| *byte == 0)
            .ok_or("unterminated Flick exclusion name")?;
        let name =
            std::str::from_utf8(&raw[..end]).map_err(|_| "non-ASCII Flick exclusion name")?;
        if !name.is_ascii() {
            return Err("non-ASCII Flick exclusion name");
        }
        let slot = table
            .iter()
            .take(128)
            .position(|entry| entry.name.eq_ignore_ascii_case(name))
            .ok_or("Flick exclusion name absent from world table")?;
        let (word, bit) =
            exclusion_bit(slot as i32).ok_or("Flick exclusion slot outside capacity")?;
        words[word] |= bit;
    }
    Ok(words)
}

pub fn admit(input: &Input, precision: Precision) -> Result<bool, &'static str> {
    if matches!(
        input.class,
        2 | 11
            | 21
            | 22
            | 73
            | 123
            | 124
            | 210
            | 215
            | 229
            | 230
            | 240
            | 522
            | 562
            | 583
            | 666
            | 693
            | 742
            | 827
            | 1047
            | 1048
            | 1049
            | 1065
            | 1066
            | 1076
            | 1080
            | 1094
            | 1142
            | 1159
    ) || input.state == 97
    {
        return Ok(false);
    }
    if let Some((word, bit)) = exclusion_bit(input.flick_slot)
        && input.exclusion_words[word] & bit != 0
    {
        return Ok(false);
    }
    if matches!(input.class, 955 | 1139) {
        return Ok(true);
    }
    if !input
        .position
        .into_iter()
        .chain(input.eye)
        .chain([input.range])
        .all(f32::is_finite)
    {
        return Err("ordinary object distance requires finite current inputs");
    }
    let delta = std::array::from_fn::<_, 3, _>(|i| {
        precision.round(f64::from(input.position[i]) - f64::from(input.eye[i]))
    });
    let square = delta.map(|value| precision.round(value * value));
    let distance = precision.round(precision.round(square[0] + square[1]) + square[2]);
    let range = f64::from(input.range);
    Ok(distance < precision.round(range * range))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn names_bind_to_first_table_match_and_missing_names_remain_errors() {
        let table = vec![
            giants_formats::world::WorldCinematic {
                source_offset: 1,
                name: "Example".into(),
                metadata: String::new(),
            },
            giants_formats::world::WorldCinematic {
                source_offset: 66,
                name: "EXAMPLE".into(),
                metadata: String::new(),
            },
        ];
        let mut raw = [0; 32];
        raw[..7].copy_from_slice(b"example");
        assert_eq!(exclusion_words(&[raw, raw], &table).unwrap(), [1, 0, 0]);
        raw[0] = b'?';
        assert!(exclusion_words(&[raw], &table).is_err());
        assert!(exclusion_words(&[[b'X'; 32]], &table).is_err());
    }
    #[test]
    fn radius_boundary_and_fourth_mask_group_are_explicit() {
        let mut input = Input {
            class: 590,
            state: 61,
            flick_slot: 96,
            exclusion_words: [0; 3],
            position: [1000., 0., 0.],
            eye: [0.; 3],
            range: INITIAL_RANGE,
        };
        assert!(!admit(&input, Precision::Double53).unwrap());
        input.position[0] = f32::from_bits(1000f32.to_bits() - 1);
        assert!(admit(&input, Precision::Double53).unwrap());
        input.exclusion_words[2] = 1;
        assert!(!admit(&input, Precision::Double53).unwrap());
        input.flick_slot = 128;
        assert!(admit(&input, Precision::Double53).unwrap());
    }
}
