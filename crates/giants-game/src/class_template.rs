//! Pointer-free scalar class preparation for the verified debug profile.
//! This is configuration, not a C++ object layout or a rendered actor.
use giants_formats::class_program::Program;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Need {
    Class(i32),
    Program(String),
    Specification(u32),
    State(u8),
    Command(u8),
    Framing,
}
#[derive(Debug, Clone)]
pub struct DebugScalar {
    /// Configuration words in the independently controlled debug field view.
    /// Commands that introduce pointers or callbacks are refused explicitly.
    pub configuration: [u32; 30],
    pub scale_bits: u32,
}
pub(crate) fn initial_configuration() -> [u32; 30] {
    let mut configuration = [0; 30];
    for slot in [2, 3, 8, 9, 21, 22, 23] {
        configuration[slot] = 1.0f32.to_bits();
    }
    for slot in [4, 7, 10] {
        configuration[slot] = 32.0f32.to_bits();
    }
    for slot in [11, 12, 28] {
        configuration[slot] = 100.0f32.to_bits();
    }
    configuration[13] = 100;
    configuration[24] = 30.0f32.to_bits();
    configuration
}
/// Fixed scalar stores shared with ordinary actor field preparation. The
/// integer conversion uses the low word of a truncating signed 64-bit result;
/// invalid x87 conversions produce integer-indefinite (whose low word is zero).
pub(crate) fn apply_configuration_extension(
    configuration: &mut [u32; 30],
    record: &giants_formats::class_program::Record,
) -> Result<(), &'static str> {
    let word = || -> Result<u32, &'static str> {
        Ok(u32::from_le_bytes(
            record
                .bytes
                .get(1..5)
                .ok_or("class configuration operand truncated")?
                .try_into()
                .unwrap(),
        ))
    };
    let integer = |bits| crate::arithmetic::integer_low_word(f64::from(f32::from_bits(bits)));
    match record.opcode {
        0x4e => configuration[16] |= 0xf0,
        0x15 => configuration[17] = word()?,
        0x4f => configuration[16] |= 0x0f,
        0x50 => configuration[16] |= 0x0f00,
        0x25 => configuration[27] = word()?,
        0x26 => {
            configuration[25] = (configuration[25] & 0x00ff_ffff) | ((word()? & 0xff) << 24);
            configuration[26] = u32::from_le_bytes(
                record
                    .bytes
                    .get(5..9)
                    .ok_or("class configuration second operand truncated")?
                    .try_into()
                    .unwrap(),
            );
        }
        0x31 => {
            let value = word()?;
            if configuration[13] == integer(configuration[12]) {
                configuration[13] = integer(value);
            }
            configuration[12] = value;
        }
        0x36 => {
            let byte = *record
                .bytes
                .get(1)
                .ok_or("class configuration byte absent")?;
            configuration[25] = (configuration[25] & !0x00ff_0000) | (u32::from(byte) << 16);
        }
        0x69 => configuration[13] = word()?,
        _ => return Err("unsupported class configuration extension"),
    }
    Ok(())
}
impl DebugScalar {
    pub fn materialize(class: i32, program: &Program) -> Result<Self, Need> {
        let mut out = Self {
            configuration: initial_configuration(),
            scale_bits: 1.0f32.to_bits(),
        };
        let mut ended = false;
        for record in &program.records {
            let word = |at: usize| -> Result<u32, Need> {
                let bytes = record.bytes.get(at..at + 4).ok_or(Need::Framing)?;
                Ok(u32::from_le_bytes(
                    bytes.try_into().map_err(|_| Need::Framing)?,
                ))
            };
            match record.opcode {
                1 => return Err(Need::Specification(word(1)?)),
                4 => {
                    out.configuration[1] = word(1)?;
                    out.configuration[0] = word(5)?;
                    out.configuration[25] |= 1 << 16;
                    out.configuration[14] |= 0x400000
                        | if matches!(class, 1 | 2 | 3 | 215) {
                            0x40000
                        } else {
                            0
                        };
                }
                5 => out.configuration[4] = word(1)?,
                6 => out.configuration[2] = word(1)?,
                7 => out.configuration[3] = word(1)?,
                8 => {
                    out.configuration[6] = word(1)?;
                    out.configuration[5] = word(5)?;
                }
                9 => out.configuration[7] = word(1)?,
                10 => out.configuration[8] = word(1)?,
                11 => out.configuration[10] = word(1)?,
                25 => out.configuration[11] = word(1)?,
                30 => {
                    let state = *record.bytes.get(1).ok_or(Need::Framing)?;
                    if state != 7 {
                        return Err(Need::State(state));
                    }
                    out.configuration[20] = u32::from(state);
                }
                0x17 => out.scale_bits = word(1)?,
                0x15 | 0x25 | 0x26 | 0x31 | 0x36 | 0x4e | 0x4f | 0x50 | 0x69 => {
                    apply_configuration_extension(&mut out.configuration, record)
                        .map_err(|_| Need::Framing)?;
                }
                255 => {
                    ended = true;
                    break;
                }
                command => return Err(Need::Command(command)),
            }
        }
        if !ended {
            return Err(Need::Framing);
        }
        if out.configuration[20] != 7 {
            return Err(Need::State(out.configuration[20] as u8));
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ground_offset_is_an_ordered_bit_copy_and_masks_preserve_other_bits() {
        let mut commands = vec![0x15];
        commands.extend(12.5f32.to_le_bytes());
        commands.extend([0x4f, 0x50, 0x4e, 0x4f, 0x15]);
        commands.extend(0x7fc0_0042u32.to_le_bytes());
        commands.push(0xff);
        let program = giants_formats::class_program::parse(&commands).unwrap();
        let mut configuration = initial_configuration();
        configuration[16] = 0xa5a5_5a5a;
        for record in program.records.iter().take(program.records.len() - 1) {
            apply_configuration_extension(&mut configuration, record).unwrap();
        }
        assert_eq!(configuration[16], 0xa5a5_5fff);
        assert_eq!(configuration[17], 0x7fc0_0042);
        let expected = initial_configuration();
        for index in (0..30).filter(|i| ![16, 17].contains(i)) {
            assert_eq!(configuration[index], expected[index]);
        }
        let malformed = giants_formats::class_program::Record {
            offset: 0,
            opcode: 0x15,
            bytes: vec![0x15, 0],
        };
        assert!(apply_configuration_extension(&mut configuration, &malformed).is_err());
        assert_eq!(configuration[17], 0x7fc0_0042);
    }
    #[test]
    fn resource_or_state_dependency_prevents_a_ready_scalar_template() {
        for (bytes, expected) in [
            (vec![1, 42, 0, 0, 0, 255], Need::Specification(42)),
            (vec![30, 2, 255], Need::State(2)),
        ] {
            let program = giants_formats::class_program::parse(&bytes).unwrap();
            assert_eq!(DebugScalar::materialize(0, &program).unwrap_err(), expected);
        }
    }
}
