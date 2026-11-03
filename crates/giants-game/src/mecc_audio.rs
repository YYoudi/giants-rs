//! Measured Mecc sound requests, before mixer/device execution.
//! Inputs are supplied by the actual weapon/jet controller, never inferred from
//! a pressed key. Sequence allocation and playback remain backend operations.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sequence {
    pub start: u32,
    pub continuous: u32,
    pub release: u32,
    pub flags: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JetRequest {
    None,
    Start(Sequence),
    Interrupt,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct JetInputs {
    pub mounted: bool,
    pub gyro: bool,
    /// Result of the native-compatible thrust/fuel gate, not raw input.
    pub thrust_active: bool,
    pub suppressed: bool,
    pub sequence_exists: bool,
    pub special_item: i32,
}

/// Jet sound tail after force/fuel evaluation. Mounted/gyro paths leave the
/// ordinary sequence untouched here; owner destruction must clean it separately.
pub fn jet_request(input: JetInputs) -> JetRequest {
    if input.mounted || input.gyro {
        return JetRequest::None;
    }
    if !input.thrust_active || input.suppressed {
        return if input.sequence_exists {
            JetRequest::Interrupt
        } else {
            JetRequest::None
        };
    }
    if input.sequence_exists {
        return JetRequest::None;
    }
    let first = if input.special_item == 18 {
        0x5b5
    } else {
        0x5b0
    };
    JetRequest::Start(Sequence {
        start: first + 2,
        continuous: first + 1,
        release: first,
        flags: 3,
    })
}

/// Accepted base-pistol projectile selectors. The original producer supplies
/// an integer in [0, 100); this function does not introduce a new RNG policy.
pub fn pistol_request(projectile: i32, draw: u32) -> Result<u32, &'static str> {
    if !matches!(projectile, 17 | 42) {
        return Err("unsupported pistol sound projectile");
    }
    if draw >= 100 {
        return Err("pistol sound draw outside native range");
    }
    Ok(if draw < 50 { 0x5f7 } else { 0x5f8 })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn holding_thrust_does_not_reallocate_and_suppression_releases() {
        let mut input = JetInputs {
            thrust_active: true,
            ..default_inputs()
        };
        assert!(matches!(jet_request(input), JetRequest::Start(_)));
        input.sequence_exists = true;
        assert_eq!(jet_request(input), JetRequest::None);
        input.suppressed = true;
        assert_eq!(jet_request(input), JetRequest::Interrupt);
        input.sequence_exists = false;
        assert_eq!(jet_request(input), JetRequest::None);
    }

    fn default_inputs() -> JetInputs {
        JetInputs::default()
    }

    #[test]
    fn unsupported_draw_or_weapon_is_not_silently_replaced() {
        assert!(pistol_request(1, 0).is_err());
        assert!(pistol_request(17, 100).is_err());
        assert_eq!(pistol_request(17, 49), Ok(0x5f7));
        assert_eq!(pistol_request(42, 50), Ok(0x5f8));
    }
}
