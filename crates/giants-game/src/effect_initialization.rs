//! Acquired null-object skip pass for complete effect programs.
//! This executes source interval commands; it creates no actor or emitter.
use crate::audio_parameters::Random;
use giants_formats::effect_program::Program;

#[derive(Clone, Copy, Debug)]
pub struct Selection {
    pub active_objects: u32,
    pub quality: f32,
    pub lite_distance: f32,
    pub camera: [f32; 3],
    pub recipient_position: [f32; 3],
}
impl Selection {
    pub fn prefer_lite(&self) -> Result<bool, String> {
        if self.active_objects > i32::MAX as u32
            || !self
                .camera
                .into_iter()
                .chain(self.recipient_position)
                .chain([self.quality, self.lite_distance])
                .all(f32::is_finite)
        {
            return Err("effect selection outside acquired finite count/pose profile".into());
        }
        let ratio = f64::from(self.active_objects) / 3000.;
        // The native load test short-circuits the camera query.
        if f64::from(self.quality) < ratio {
            return Ok(true);
        }
        let d = std::array::from_fn::<_, 3, _>(|i| {
            f64::from(self.camera[i]) - f64::from(self.recipient_position[i])
        });
        let squared = (d[0] * d[0] + d[1] * d[1]) + d[2] * d[2];
        Ok(squared > f64::from(self.lite_distance) * f64::from(self.lite_distance))
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Interval {
    /// Source record offset; a fixed two-range command yields two ordered entries.
    pub offset: usize,
    pub opcode: u8,
    pub low: f32,
    pub high: f32,
    pub value: f32,
}
// Complete flat spawn blocks whose operations all skip before side effects.
// Opcode04 is a vector write here, not an unconditional random interval.
fn flat_blocks(ops: &[u8]) -> bool {
    let mut at = 0;
    let mut blocks = 0;
    while ops.get(at..at + 2) == Some(&[3, 6]) {
        at += 2;
        if ops.get(at..at + 3) != Some(&[0x1b, 0x21, 4])
            && ops.get(at..at + 3) != Some(&[0x21, 0x1b, 4])
        {
            return false;
        }
        at += 3;
        let start = at;
        while ops.get(at) == Some(&0x10) {
            at += 1;
        }
        if at == start || ops.get(at) != Some(&7) {
            return false;
        }
        at += 1;
        blocks += 1;
    }
    blocks > 0 && ops.get(at..) == Some(&[2])
}
/// Admit the whole acquired frame, never a name, address or first opcode.
/// Range operands are decoded and validated before any random state commits.
pub fn skip(program: &Program, random: &mut Random) -> Result<Vec<Interval>, String> {
    let ops = program.records.iter().map(|r| r.opcode).collect::<Vec<_>>();
    let old = matches!(
        ops.as_slice(),
        [2] | [0x24, 6, 0x1b, 7, 2] | [0x24, 6, 0x1b, 0x27, 7, 2]
    );
    let ranged = ops.len() >= 10
        && ops[..5] == [0x24, 6, 0x1b, 0x21, 0x28]
        && ops[5..ops.len() - 4].iter().all(|&op| op == 0x5c)
        && ops[ops.len() - 4..] == [0x49, 7, 0x4a, 2];
    let sound_block = matches!(
        ops.as_slice(),
        [0x24, 6, 4, 0x21, 0x87, 7, 2]
            | [0x24, 6, 0x87, 4, 0x21, 7, 2]
            | [0x24, 6, 0x21, 0x87, 4, 7, 2]
    );
    let positional_sound = ops.as_slice() == [0x2b, 2];
    let contact = ops.as_slice()
        == [
            0x24, 6, 0x1b, 0x17, 0x21, 0x28, 0x28, 0x83, 0x49, 7, 0x4a, 2,
        ];
    let positional_impact = ops.as_slice() == [3, 6, 0x83, 9, 4, 0x32, 7, 2];
    let sized_retirement = ops.as_slice() == [0x24, 6, 0x1c, 4, 0x0d, 7, 2];
    let fixed_size_gravity = ops.as_slice() == [3, 6, 8, 4, 0x0e, 7, 2];
    let damage_smoke = ops.as_slice() == [0x24, 6, 0x21, 0x28, 0x49, 7, 0x4a, 2];
    let smoke_particle = ops.as_slice() == [3, 6, 4, 0x37, 0x0f, 0x80, 7, 2];
    // Complete source contact frames. Child commands are skipped here; each
    // actual immediate child owns its own later preparation and activation.
    let pistol_burst = ops.as_slice()
        == [
            0x2b, 3, 6, 8, 4, 7, 3, 6, 0x1b, 0x21, 4, 0x10, 0x10, 0x10, 7, 3, 6, 0x21, 4, 0x10,
            0x10, 0x10, 0x10, 7, 2,
        ];
    let impact_child = matches!(
        ops.as_slice(),
        [3, 6, 4, 0x33, 0x1c, 0x13, 0x12, 0x0e, 0x19, 7, 2] | [3, 6, 5, 8, 0x0e, 7, 2]
    );
    let pistol_splash = ops.as_slice() == [0x2b, 3, 6, 0x37, 5, 0x46, 7, 2];
    let ghost = ops.as_slice() == [3, 6, 8, 0x49, 0x55, 4, 7, 0x4a, 2];
    if ghost {
        crate::vimp_ghost::Recipe::project(program)?;
    }
    if !old
        && !ranged
        && !flat_blocks(&ops)
        && !sound_block
        && !positional_sound
        && !contact
        && !positional_impact
        && !sized_retirement
        && !fixed_size_gravity
        && !damage_smoke
        && !smoke_particle
        && !pistol_burst
        && !impact_child
        && !pistol_splash
        && !ghost
    {
        return Err("effect initial skip pass outside acquired complete profile".into());
    }
    if smoke_particle {
        // Lifetime and side-speed operands are traversed but not sampled in
        // the null-object pass. Side speed samples only after its active gate.
        for index in [2, 5] {
            for bytes in program.records[index].bytes[1..].as_chunks::<4>().0 {
                if !f32::from_le_bytes(*bytes).is_finite() {
                    return Err("smoke particle skip operand is nonfinite".into());
                }
            }
        }
    }
    for record in program
        .records
        .iter()
        .filter(|r| matches!(r.opcode, 0x83 | 0x87 | 0x2b))
    {
        let operands = record
            .bytes
            .get(1..9)
            .ok_or("effect sound operands truncated")?;
        let low = u32::from_le_bytes(operands[..4].try_into().unwrap());
        let high = u32::from_le_bytes(operands[4..].try_into().unwrap());
        if high > i32::MAX as u32 || low > high {
            return Err("effect sound selection outside acquired ordered signed range".into());
        }
    }
    let mut intervals = vec![];
    for record in &program.records {
        if !matches!(
            record.opcode,
            5 | 9 | 0x12 | 0x1c | 0x28 | 0x33 | 0x5c | 0x37 | 0x0f
        ) {
            continue;
        }
        let at = if matches!(record.opcode, 5 | 9 | 0x12 | 0x1c | 0x33 | 0x37 | 0x0f) {
            1
        } else {
            record.bytes[1..]
                .iter()
                .position(|&b| b == 0)
                .ok_or("effect interval name unterminated")?
                + 2
        };
        let word = |offset| -> Result<f32, String> {
            Ok(f32::from_le_bytes(
                record
                    .bytes
                    .get(offset..offset + 4)
                    .ok_or("effect interval truncated")?
                    .try_into()
                    .unwrap(),
            ))
        };
        // The fixed-layout command consumes both ranges before its absent-
        // object gate. Preserve their order within the same source record.
        for at in (0..if matches!(record.opcode, 5 | 0x33) {
            3
        } else if matches!(record.opcode, 9 | 0x37) {
            2
        } else {
            1
        })
            .map(|i| at + i * 8)
        {
            let low = word(at)?;
            let high = word(at + 4)?;
            if ![low, high].into_iter().all(f32::is_finite) {
                return Err("effect initial interval is not finite".into());
            }
            intervals.push(Interval {
                offset: record.offset,
                opcode: record.opcode,
                low,
                high,
                value: 0.,
            });
        }
    }
    let mut staged = *random;
    let mut values = intervals.iter_mut();
    for record in &program.records {
        if matches!(record.opcode, 0x83 | 0x87 | 0x2b) {
            // Native integer selection consumes a draw even in the null-object
            // pass. The selected value is not stored on an absent object.
            staged.next_15();
            continue;
        }
        if !matches!(
            record.opcode,
            5 | 9 | 0x12 | 0x1c | 0x28 | 0x33 | 0x5c | 0x37 | 0x0f
        ) {
            continue;
        }
        for _ in 0..if matches!(record.opcode, 5 | 0x33) {
            3
        } else if matches!(record.opcode, 9 | 0x37) {
            2
        } else {
            1
        } {
            let i = values.next().unwrap();
            i.value = (f64::from(staged.next_15())
                * (f64::from(i.high) - f64::from(i.low))
                * (1. / 32768.)
                + f64::from(i.low)) as f32;
            if !i.value.is_finite() {
                return Err("effect initial interval result is not finite".into());
            }
        }
    }
    *random = staged;
    Ok(intervals)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn damage_smoke_skip_consumes_one_interval_without_active_side_effects() {
        let mut bytes = vec![0x24];
        bytes.extend([0; 12]);
        bytes.extend([6, 0x21, 0x28]);
        bytes.extend(b"synthetic\0");
        for value in [0.1f32.to_bits(), 0.2f32.to_bits(), 0, 0] {
            bytes.extend(value.to_le_bytes());
        }
        bytes.extend([0x49, 7, 0x4a, 2]);
        let mut program = giants_formats::effect_program::parse(&bytes).unwrap();
        let mut random = Random(1729);
        let mut expected = random;
        let value = (f64::from(expected.next_15()) * (f64::from(0.2f32) - f64::from(0.1f32))
            / 32768.
            + f64::from(0.1f32)) as f32;
        let intervals = skip(&program, &mut random).unwrap();
        assert_eq!(intervals.len(), 1);
        assert_eq!(intervals[0].value.to_bits(), value.to_bits());
        assert_eq!(random.0, expected.0);
        program.records[2].opcode = 0x1b;
        let before = random.0;
        assert!(skip(&program, &mut random).is_err());
        assert_eq!(random.0, before);
    }
    #[test]
    fn fixed_size_gravity_preparation_has_no_draw_or_active_object_effects() {
        let mut bytes = vec![3];
        bytes.extend(42u32.to_le_bytes());
        bytes.extend([6, 8]);
        bytes.extend(2f32.to_le_bytes());
        bytes.extend(5f32.to_le_bytes());
        bytes.push(4);
        for value in [0.75f32, 0.15, 0.6] {
            bytes.extend(value.to_le_bytes());
        }
        bytes.push(0x0e);
        bytes.extend(0.5f32.to_le_bytes());
        bytes.extend([7, 2]);
        let mut program = giants_formats::effect_program::parse(&bytes).unwrap();
        let mut random = Random(17);
        assert!(skip(&program, &mut random).unwrap().is_empty());
        assert_eq!(random.0, 17);
        program.records[4].opcode = 0x16;
        assert!(skip(&program, &mut random).is_err());
        assert_eq!(random.0, 17);
    }
    #[test]
    fn sized_retirement_skip_draws_before_absent_object_and_keeps_failure_atomic() {
        let mut bytes = vec![0x24];
        bytes.extend([0; 12]);
        bytes.extend([6, 0x1c]);
        bytes.extend(6f32.to_le_bytes());
        bytes.extend(0.01f32.to_le_bytes());
        bytes.push(4);
        bytes.extend([0; 12]);
        bytes.push(0x0d);
        bytes.extend(b"synthetic_retirement\0");
        bytes.extend([7, 2]);
        let program = giants_formats::effect_program::parse(&bytes).unwrap();
        let mut random = Random(1);
        let intervals = skip(&program, &mut random).unwrap();
        assert_eq!(random.0, 2_745_024);
        assert_eq!(intervals.len(), 1);
        assert_eq!(intervals[0].opcode, 0x1c);
        assert!(intervals[0].value > 0.01 && intervals[0].value < 6.);

        bytes[19..23].copy_from_slice(&f32::NAN.to_le_bytes());
        let invalid = giants_formats::effect_program::parse(&bytes).unwrap();
        assert!(skip(&invalid, &mut random).is_err());
        assert_eq!(random.0, 2_745_024);

        let mut unacquired = program;
        unacquired.records[4].opcode = 0x16;
        assert!(skip(&unacquired, &mut random).is_err());
        assert_eq!(random.0, 2_745_024);
    }
    #[test]
    fn positional_impact_ranges_follow_sound_draw_and_reject_late_invalid_operands() {
        let mut bytes = vec![3];
        bytes.extend(42u32.to_le_bytes());
        bytes.extend([6, 0x83]);
        for value in [3u32, 7] {
            bytes.extend(value.to_le_bytes());
        }
        bytes.push(9);
        for value in [0f32, 32768., 0., 32768.] {
            bytes.extend(value.to_le_bytes());
        }
        for op in [4, 0x32] {
            bytes.push(op);
            bytes.extend([0; 12]);
        }
        bytes.extend([7, 2]);
        let mut program = giants_formats::effect_program::parse(&bytes).unwrap();
        let mut random = Random(1);
        let intervals = skip(&program, &mut random).unwrap();
        assert_eq!(intervals.len(), 2);
        assert_eq!(intervals[0].value, 18467.);
        assert_eq!(intervals[1].value, 6334.);
        let committed = random.0;
        let range = program.records.iter_mut().find(|r| r.opcode == 9).unwrap();
        range.bytes[13..17].copy_from_slice(&f32::NAN.to_le_bytes());
        assert!(skip(&program, &mut random).is_err());
        assert_eq!(random.0, committed);
        program
            .records
            .iter_mut()
            .find(|r| r.opcode == 9)
            .unwrap()
            .bytes
            .truncate(13);
        assert!(skip(&program, &mut random).is_err());
        assert_eq!(random.0, committed);
    }
    #[test]
    fn contact_skip_consumes_both_intervals_then_integer_selection_atomically() {
        let mut bytes = vec![0x24];
        for value in [0u32, 9, 0] {
            bytes.extend(value.to_le_bytes());
        }
        bytes.extend([6, 0x1b]);
        bytes.extend(1f32.to_le_bytes());
        bytes.push(0x17);
        bytes.extend(0u32.to_le_bytes());
        bytes.push(0x21);
        for name in [b"A\0", b"B\0"] {
            bytes.push(0x28);
            bytes.extend(name);
            for value in [1f32.to_bits(), 2f32.to_bits(), 0, 0] {
                bytes.extend(value.to_le_bytes());
            }
        }
        bytes.push(0x83);
        for value in [2u32, 5] {
            bytes.extend(value.to_le_bytes());
        }
        bytes.extend([0x49, 7, 0x4a, 2]);
        let mut program = giants_formats::effect_program::parse(&bytes).unwrap();
        let mut random = Random(17);
        let mut expected = random;
        for _ in 0..3 {
            expected.next_15();
        }
        let intervals = skip(&program, &mut random).unwrap();
        assert_eq!(intervals.len(), 2);
        assert_eq!(random.0, expected.0);
        program
            .records
            .iter_mut()
            .find(|r| r.opcode == 0x83)
            .unwrap()
            .bytes[5..9]
            .copy_from_slice(&1u32.to_le_bytes());
        assert!(skip(&program, &mut random).is_err());
        assert_eq!(random.0, expected.0);
    }
    #[test]
    fn sound_before_vector_in_a_complete_block_keeps_the_shared_draw_and_guards() {
        let mut bytes = vec![0x24];
        for value in [0u32, 9, 0] {
            bytes.extend(value.to_le_bytes());
        }
        bytes.extend([6, 0x21, 0x87]);
        for value in [3u32, 7] {
            bytes.extend(value.to_le_bytes());
        }
        bytes.push(4);
        for value in [1.0f32, 2., 3.] {
            bytes.extend(value.to_le_bytes());
        }
        bytes.extend([7, 2]);
        let mut program = giants_formats::effect_program::parse(&bytes).unwrap();
        let mut random = Random(1234);
        let mut expected = random;
        expected.next_15();
        assert!(skip(&program, &mut random).unwrap().is_empty());
        assert_eq!(random.0, expected.0);
        program
            .records
            .iter_mut()
            .find(|r| r.opcode == 0x87)
            .unwrap()
            .bytes[5..9]
            .copy_from_slice(&2u32.to_le_bytes());
        assert!(skip(&program, &mut random).is_err());
        assert_eq!(random.0, expected.0);
        bytes.insert(bytes.len() - 2, 0x15);
        assert!(
            skip(
                &giants_formats::effect_program::parse(&bytes).unwrap(),
                &mut random
            )
            .is_err()
        );
        assert_eq!(random.0, expected.0);
    }
    #[test]
    fn late_invalid_range_cannot_consume_the_earlier_valid_range() {
        let mut bytes = vec![0x24];
        for word in [1u32, 2, 3] {
            bytes.extend(word.to_le_bytes());
        }
        bytes.extend([6, 0x1b]);
        bytes.extend(0.1f32.to_le_bytes());
        bytes.push(0x21);
        bytes.push(0x28);
        bytes.extend(b"synthetic\0");
        for word in [0.1f32.to_bits(), 0.2f32.to_bits(), 0, 0] {
            bytes.extend(word.to_le_bytes());
        }
        bytes.push(0x5c);
        bytes.extend(b"synthetic\0");
        for word in [0.1f32.to_bits(), f32::NAN.to_bits(), 0, 0, 0, 0] {
            bytes.extend(word.to_le_bytes());
        }
        bytes.extend([0x49, 7, 0x4a, 2]);
        let program = giants_formats::effect_program::parse(&bytes).unwrap();
        let mut random = Random(1729);
        assert!(skip(&program, &mut random).is_err());
        assert_eq!(random.0, 1729);
    }

    #[test]
    fn smoke_particle_skip_samples_size_and_gravity_but_not_active_side_speed() {
        let mut bytes = vec![3];
        bytes.extend(777u32.to_le_bytes());
        bytes.extend([6, 4]);
        for value in [1f32, 0., 0.5] {
            bytes.extend(value.to_le_bytes());
        }
        bytes.push(0x37);
        for value in [2f32, 2., 4., 4.] {
            bytes.extend(value.to_le_bytes());
        }
        bytes.push(0x0f);
        for value in [-5f32, -5.] {
            bytes.extend(value.to_le_bytes());
        }
        bytes.push(0x80);
        for value in [1000f32, 2000.] {
            bytes.extend(value.to_le_bytes());
        }
        bytes.extend([7, 2]);
        let program = giants_formats::effect_program::parse(&bytes).unwrap();
        let mut random = Random(42);
        let samples = skip(&program, &mut random).unwrap();
        assert_eq!(
            samples
                .iter()
                .map(|s| (s.opcode, s.value))
                .collect::<Vec<_>>(),
            vec![(0x37, 2.), (0x37, 4.), (0x0f, -5.)]
        );
        let mut expected = Random(42);
        for _ in 0..3 {
            expected.next_15();
        }
        assert_eq!(random.0, expected.0);
        let mut invalid = program.clone();
        invalid.records[5].bytes[1..5].copy_from_slice(&f32::NAN.to_le_bytes());
        assert!(skip(&invalid, &mut random).is_err());
        assert_eq!(random.0, expected.0);
    }
}
