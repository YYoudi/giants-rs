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
    if !old && !ranged && !flat_blocks(&ops) && !sound_block && !positional_sound {
        return Err("effect initial skip pass outside acquired complete profile".into());
    }
    for record in program
        .records
        .iter()
        .filter(|r| matches!(r.opcode, 0x87 | 0x2b))
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
        if !matches!(record.opcode, 0x28 | 0x5c) {
            continue;
        }
        let at = record.bytes[1..]
            .iter()
            .position(|&b| b == 0)
            .ok_or("effect interval name unterminated")?
            + 2;
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
    let mut staged = *random;
    let mut values = intervals.iter_mut();
    for record in &program.records {
        if matches!(record.opcode, 0x87 | 0x2b) {
            // Native integer sound selection consumes a draw even in the
            // null-object pass. It neither plays audio nor stores the choice.
            staged.next_15();
            continue;
        }
        if !matches!(record.opcode, 0x28 | 0x5c) {
            continue;
        }
        let i = values.next().unwrap();
        i.value =
            (f64::from(staged.next_15()) * (f64::from(i.high) - f64::from(i.low)) * (1. / 32768.)
                + f64::from(i.low)) as f32;
        if !i.value.is_finite() {
            return Err("effect initial interval result is not finite".into());
        }
    }
    *random = staged;
    Ok(intervals)
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
