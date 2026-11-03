//! Owned D Mecc parameter refresh and inventory mass, before movement.
//! No actor pointer, timestep, collision result or completed frame is supplied.
use crate::{arithmetic::Precision, mecc_initial::Parameters, mecc_inventory::Inventory};
use giants_formats::{global_data::GlobalData, weapons::Catalog};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Physics {
    /// Native inline scalar words0..38. The pointer to this owned block is
    /// represented by Session presence, never stored in the payload.
    words: [u32; 15],
}
impl Physics {
    pub fn words(&self) -> [u32; 15] {
        self.words
    }
    pub fn mass(&self) -> f32 {
        f32::from_bits(self.words[2])
    }
    pub fn gravity_mass(&self) -> f32 {
        f32::from_bits(self.words[1])
    }
    pub(crate) fn from_payload(payload: &[u8]) -> Result<Self, String> {
        if payload.len() != 524 {
            return Err("Mecc physics requires D payload".into());
        }
        Ok(Self {
            words: std::array::from_fn(|i| word(payload, 0x84 + i * 4)),
        })
    }
}
pub(crate) struct Prepared {
    pub payload: Vec<u8>,
    pub configuration: [u32; 30],
    pub physics: Physics,
}
fn word(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
}
fn put(data: &mut [u8], offset: usize, value: u32) {
    data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}
pub(crate) struct Refresh<'a> {
    pub generation: u32,
    pub globals: &'a GlobalData,
    pub catalog: &'a Catalog,
    pub response_flags: u32,
    pub control_flags: u32,
}
pub(crate) fn prepare(
    payload: &[u8],
    present: bool,
    mut configuration: [u32; 30],
    context: Refresh<'_>,
) -> Result<Prepared, String> {
    let Refresh {
        generation,
        globals,
        catalog,
        response_flags,
        control_flags,
    } = context;
    if payload.len() != 524 {
        return Err("Mecc physics requires acquired D payload".into());
    }
    let parameters = Parameters::from_globals(globals).map_err(|e| e.to_string())?;
    if !parameters
        .words()
        .into_iter()
        .map(f32::from_bits)
        .all(f32::is_finite)
        || parameters.mass <= 0.
    {
        return Err("Mecc parameters outside finite positive-mass profile".into());
    }
    let mut body = payload.to_vec();
    let mut physics = Physics::from_payload(payload)?;
    if word(payload, 0xcc) != generation {
        if !present {
            if physics.words != [0; 15] {
                return Err("new Mecc physics requires untouched inline storage".into());
            }
            physics.words[1] = 1f32.to_bits();
            physics.words[2] = 1f32.to_bits();
            physics.words[4] = 1f32.to_bits();
            put(&mut body, 0x4c, 100f32.to_bits());
        }
        for (index, value) in [
            (1, parameters.gravity_mass),
            (2, parameters.mass),
            (3, parameters.restitution),
            (10, parameters.speed_limit),
            (12, parameters.friction_limit),
            (13, parameters.damping_xy),
            (14, parameters.damping_z),
        ] {
            physics.words[index] = value.to_bits();
        }
        configuration[0] = parameters.control_gain.to_bits();
        configuration[1] = parameters.control_speed.to_bits();
        configuration[4] = parameters.slope_power.to_bits();
        put(&mut body, 0xcc, generation);
    } else if !present {
        return Err("unchanged generation leaves native Mecc physics absent".into());
    }
    let inventory = Inventory {
        utility: std::array::from_fn(|i| word(payload, 0x174 + i * 4) as i32),
        weapons: std::array::from_fn(|i| word(payload, 0x180 + i * 4) as i32),
        special: word(payload, 0x190) as i32,
    };
    let weights = inventory.weights(catalog)?;
    let masses = crate::mecc_load_mass::calculate(
        crate::mecc_load_mass::Input {
            base_mass: parameters.mass,
            base_gravity_mass: parameters.gravity_mass,
            special_mass: globals.scalar(0xacc).map_err(|e| e.to_string())?,
            item_weights: weights.slots,
            special_weight: weights.special,
            response_flags,
            object_flags: control_flags,
            has_physics: true,
            attached: false,
            previous: [physics.mass(), physics.gravity_mass()],
        },
        Precision::Double53,
    )?;
    if !masses.into_iter().all(f32::is_finite) || masses[0] <= 0. {
        return Err("Mecc inventory mass outside finite positive profile".into());
    }
    physics.words[2] = masses[0].to_bits();
    physics.words[1] = masses[1].to_bits();
    for (i, value) in physics.words.into_iter().enumerate() {
        put(&mut body, 0x84 + i * 4, value);
    }
    Ok(Prepared {
        payload: body,
        configuration,
        physics,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Vec<u8>, GlobalData, Catalog) {
        let mut payload = vec![0; 524];
        for offset in (0x174..=0x190).step_by(4) {
            put(&mut payload, offset, u32::MAX);
        }
        let mut globals = GlobalData {
            words: vec![0; 0xacc / 4 + 1],
            array_words: vec![],
        };
        for (offset, value) in [
            (0x674, 45f32),
            (0x678, 20.),
            (0x67c, 800.),
            (0x684, 1000.),
            (0x5f0, 3.),
            (0x698, 3.5),
            (0x688, 200.),
            (0x6d8, 35.),
            (0x68c, 9.),
        ] {
            globals.words[offset / 4] = value.to_bits();
        }
        let catalog = Catalog {
            records: vec![],
            commands_end: 0,
            projectiles: Default::default(),
            icons: Default::default(),
            unprojected_records: 0,
            ignored_records: 0,
            inactive_records: 0,
        };
        (payload, globals, catalog)
    }
    fn run(
        payload: &[u8],
        present: bool,
        configuration: [u32; 30],
        generation: u32,
        globals: &GlobalData,
        catalog: &Catalog,
    ) -> Result<Prepared, String> {
        prepare(
            payload,
            present,
            configuration,
            Refresh {
                generation,
                globals,
                catalog,
                response_flags: 0,
                control_flags: 0,
            },
        )
    }
    #[test]
    fn generation_refresh_preserves_existing_force_and_consumed_shield() {
        let (payload, mut globals, catalog) = fixture();
        assert!(run(&payload, false, [0; 30], 0, &globals, &catalog).is_err());
        let first = run(&payload, false, [0; 30], 1, &globals, &catalog).unwrap();
        assert_eq!(word(&first.payload, 0x4c), 100f32.to_bits());
        let mut body = first.payload;
        put(&mut body, 0x84 + 0x18, 7f32.to_bits());
        put(&mut body, 0x4c, 42f32.to_bits());
        globals.words[0x688 / 4] = 333f32.to_bits();
        let same = run(&body, true, first.configuration, 1, &globals, &catalog).unwrap();
        assert_eq!(same.payload, body);
        assert_eq!(same.configuration[0], 200f32.to_bits());
        let next = run(&body, true, first.configuration, 2, &globals, &catalog).unwrap();
        assert_eq!(next.physics.words[6], 7f32.to_bits());
        assert_eq!(word(&next.payload, 0x4c), 42f32.to_bits());
        assert_eq!(next.configuration[0], 333f32.to_bits());
    }
    #[test]
    fn invalid_inventory_refuses_preparation_without_touching_source() {
        let (mut payload, globals, catalog) = fixture();
        put(&mut payload, 0x174, 75);
        let before = payload.clone();
        assert!(run(&payload, false, [0; 30], 1, &globals, &catalog).is_err());
        assert_eq!(payload, before);
    }
}
