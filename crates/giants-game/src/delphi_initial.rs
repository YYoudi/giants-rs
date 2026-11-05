//! Delphi's first-primary physical initialization, before movement or ticking.
use giants_formats::global_data::GlobalData;

#[derive(Clone, Copy, Debug)]
pub struct Parameters {
    pub physical: [f32; 7],
    /// Acquired signed upper clamp; later interpretation remains separate.
    pub limit: i32,
}
impl Parameters {
    pub fn from_globals(g: &GlobalData) -> Result<Self, String> {
        let mut physical = [0.; 7];
        for (value, offset) in physical
            .iter_mut()
            .zip([0x704, 0x708, 0x70c, 0x710, 0x734, 0x714, 0x718])
        {
            *value = g.scalar(offset).map_err(|e| e.to_string())?;
        }
        Ok(Self {
            physical,
            limit: *g
                .words
                .get(0x1670 / 4)
                .ok_or("Delphi initial limit absent")? as i32,
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Physics {
    words: [u32; 15],
}
impl Physics {
    pub fn words(self) -> [u32; 15] {
        self.words
    }
    pub(crate) fn read(payload: &[u8]) -> Result<Self, String> {
        if payload.len() != 4096 {
            return Err("Delphi physics requires the acquired debug payload".into());
        }
        Ok(Self {
            words: std::array::from_fn(|i| word(payload, 0xd00 + i * 4)),
        })
    }
}
pub struct Initialized {
    pub payload: Vec<u8>,
    pub physics: Physics,
    pub control_flags: u32,
}
fn word(payload: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(payload[at..at + 4].try_into().unwrap())
}
fn put(payload: &mut [u8], at: usize, value: u32) {
    payload[at..at + 4].copy_from_slice(&value.to_le_bytes());
}
/// Pure owned payload projection. Session supplies class/reference/replay guards.
/// The acquired initializer resets dirty physical words, preserving word five.
pub fn initialize(
    payload: &[u8],
    scale: f32,
    parameters: Parameters,
) -> Result<Initialized, String> {
    if !scale.is_finite() || scale <= 0. || !parameters.physical.into_iter().all(f32::is_finite) {
        return Err("Delphi initialization requires finite parameters and positive scale".into());
    }
    let mut physics = Physics::read(payload)?;
    crate::physical_initial::reset(&mut physics.words);
    for (index, value) in [1, 2, 10, 3, 12, 13, 14]
        .into_iter()
        .zip(parameters.physical)
    {
        physics.words[index] = value.to_bits();
    }
    let mut payload = payload.to_vec();
    for (index, value) in physics.words.into_iter().enumerate() {
        put(&mut payload, 0xd00 + index * 4, value);
    }
    put(&mut payload, 0xfac, scale.to_bits());
    put(&mut payload, 0xdac, parameters.limit.min(5) as u32);
    Ok(Initialized {
        payload,
        physics,
        control_flags: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dirty_initialization_preserves_opaque_word_and_unrelated_payload() {
        let payload = vec![0xa5; 4096];
        let mut p = Parameters {
            physical: [2., 3., 4., 5., 6., 7., 8.],
            limit: -19,
        };
        let result = initialize(&payload, 0.5, p).unwrap();
        assert_eq!(result.physics.words()[5], 0xa5a5a5a5);
        assert_eq!(result.physics.words()[4], 1f32.to_bits());
        assert_eq!(word(&result.payload, 0xdac) as i32, -19);
        assert_eq!(word(&result.payload, 0xfac), 0.5f32.to_bits());
        assert_eq!(result.control_flags, 0);
        for (at, &value) in result.payload.iter().enumerate() {
            if !(0xd00..0xd3c).contains(&at)
                && !(0xdac..0xdb0).contains(&at)
                && !(0xfac..0xfb0).contains(&at)
            {
                assert_eq!(value, 0xa5);
            }
        }
        p.limit = i32::MAX;
        assert_eq!(
            word(&initialize(&payload, 1., p).unwrap().payload, 0xdac),
            5
        );
    }
    #[test]
    fn refused_profile_does_not_mutate_the_input() {
        let payload = vec![0x7b; 4096];
        let mut p = Parameters {
            physical: [1.; 7],
            limit: 0,
        };
        assert!(initialize(&payload[..4095], 1., p).is_err());
        assert!(initialize(&payload, 0., p).is_err());
        p.physical[3] = f32::NAN;
        assert!(initialize(&payload, 1., p).is_err());
        assert!(payload.iter().all(|&b| b == 0x7b));
    }
}
