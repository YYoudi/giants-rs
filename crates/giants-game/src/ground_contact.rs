//! Measured final contact stage, after external ground/platform queries.
//! This does not perform broad-phase collision, terrain sampling or state callbacks.
use crate::arithmetic::Precision;
pub const CONTACT_BIT: u32 = 0x0008_0000;
pub const CONTACT_ENTER_BIT: u32 = 0x1000_0000;
const CONTACT_TOLERANCE: f32 = 0.1;
#[derive(Clone, Copy, Debug)]
pub struct Input {
    pub z: f32,
    pub vertical_velocity: f32,
    pub flags: u32,
    pub lock_z: bool,
    pub ground: f32,
    pub plane: [f32; 4],
    pub vertical_offset: f32,
    /// A flagged result from the external platform query, not inferred here.
    pub platform: Option<f32>,
}
#[derive(Clone, Copy, Debug)]
pub struct Contact {
    pub z: f32,
    pub vertical_velocity: f32,
    pub flags: u32,
    pub ground: f32,
    pub plane: [f32; 4],
}
pub fn resolve(input: Input, precision: Precision) -> Result<Contact, &'static str> {
    if ![
        input.z,
        input.vertical_velocity,
        input.ground,
        input.vertical_offset,
    ]
    .into_iter()
    .chain(input.plane)
    .chain(input.platform)
    .all(f32::is_finite)
    {
        return Err("non-finite ground contact input");
    }
    let mut result = Contact {
        z: input.z,
        vertical_velocity: input.vertical_velocity,
        flags: input.flags,
        ground: input.ground,
        plane: input.plane,
    };
    if let Some(platform) = input.platform
        && platform < input.z
        && platform > input.ground
    {
        result.ground = platform;
        result.plane = [0., 0., 1., -platform];
    }
    // Keep the sum wide through the strict tolerance comparison. The native
    // code stores it to float32 only if the unlocked Z is actually updated.
    let target = precision.round(f64::from(input.vertical_offset) + f64::from(result.ground));
    let delta = precision.round(target - f64::from(input.z)).abs();
    if delta < f64::from(CONTACT_TOLERANCE) {
        if !input.lock_z {
            result.z = target as f32;
        }
        if result.flags & CONTACT_BIT == 0 {
            result.flags |= CONTACT_ENTER_BIT;
        }
        result.flags |= CONTACT_BIT;
        if result.vertical_velocity < 0. {
            result.vertical_velocity = 0.;
        }
    } else {
        result.flags &= !CONTACT_BIT;
    }
    if !result.z.is_finite() {
        return Err("ground contact Z overflow");
    }
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn input(z: f32) -> Input {
        Input {
            z,
            vertical_velocity: -2.,
            flags: 0,
            lock_z: false,
            ground: 0.,
            plane: [0., 0., 1., 0.],
            vertical_offset: 0.,
            platform: None,
        }
    }
    #[test]
    fn strict_contact_flags_lock_and_velocity() {
        for precision in [Precision::Single24, Precision::Double53] {
            let contact = resolve(input(0.05), precision).unwrap();
            assert_eq!(contact.z, 0.);
            assert_eq!(contact.vertical_velocity, 0.);
            assert_eq!(contact.flags, CONTACT_BIT | CONTACT_ENTER_BIT);
            let boundary = resolve(
                Input {
                    flags: CONTACT_BIT | 16,
                    ..input(0.1)
                },
                precision,
            )
            .unwrap();
            assert_eq!(boundary.z, 0.1);
            assert_eq!(boundary.flags, 16);
            assert_eq!(boundary.vertical_velocity, -2.);
            let locked = resolve(
                Input {
                    lock_z: true,
                    flags: CONTACT_BIT,
                    ..input(0.05)
                },
                precision,
            )
            .unwrap();
            assert_eq!(locked.z, 0.05);
            assert_eq!(locked.flags, CONTACT_BIT);
            assert_eq!(locked.vertical_velocity, 0.);
            let platform = resolve(
                Input {
                    platform: Some(1.),
                    ..input(1.05)
                },
                precision,
            )
            .unwrap();
            assert_eq!(platform.ground, 1.);
            assert_eq!(platform.plane, [0., 0., 1., -1.]);
            assert_eq!(platform.z, 1.);
            let above = resolve(
                Input {
                    platform: Some(2.),
                    ..input(1.05)
                },
                precision,
            )
            .unwrap();
            assert_eq!(above.ground, 0.);
            assert!(resolve(input(f32::NAN), precision).is_err());
        }
    }
}
