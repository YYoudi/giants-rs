//! Projection of the original renderer's packed feature word.
//! Inputs are initialized renderer fields, not a direct Vulkan/DXGI capability
//! mapping. Adapter workarounds affecting other renderer fields are excluded.

#[derive(Clone, Copy, Debug)]
pub struct Input {
    /// D: 468,470,474,478,434,458,460,47c,454 (hex).
    /// G: 8cc,8d4,8d8,8dc,89c,8bc,8c4,8e0,8b8.
    pub features: [u32; 9],
    pub vendor: u32,
    pub device: u32,
}

pub fn word(input: Input) -> u32 {
    let f = input.features.map(|v| v != 0);
    let mut result = 0x4e409 | if f[0] { 2 } else { 0 };
    if f[1] && f[8] {
        result |= 4;
    }
    for (present, bit) in f[2..8].iter().zip([0x20, 0x10, 0x100, 0x80, 0x40, 0x800]) {
        if *present {
            result |= bit;
        }
    }
    if f[8] {
        result |= 0x1000;
    } else {
        result &= !0x8000;
    }
    match input.vendor {
        0x102b => match input.device {
            0x521 => result &= !0x40000,
            0x525 => result &= !0x840,
            _ => {}
        },
        0x5333 if input.device == 0x9102 => result = (result & !0x4000) | 0x20000,
        0x1002 if ati_legacy(input.device) => result &= !0x80,
        0x121a => {
            result &= !8;
            match input.device {
                5 => result &= !0x20,
                1 | 2 => result &= !0xc000,
                3 => result &= !0x8000,
                _ => {}
            }
            result = (result & !0x400) | 0x200;
        }
        0x8086 if input.device == 0x7123 => result &= !0x4000,
        0x1163 if input.device == 0x2000 => result &= !0x4000,
        _ => {}
    }
    result
}

fn ati_legacy(device: u32) -> bool {
    matches!(
        device,
        0x5041
            ..=0x5058
                | 0x5245
                | 0x5246
                | 0x534b
                | 0x534c
                | 0x534d
                | 0x524b
                | 0x524c
                | 0x5345
                | 0x5346
                | 0x5347
                | 0x4c4d
                | 0x4c52
                | 0x4c42
                | 0x4c49
                | 0x4c50
                | 0x4c47
                | 0x474d
                | 0x474f
                | 0x4752
                | 0x474e
                | 0x4742
                | 0x4744
                | 0x4749
                | 0x4750
                | 0x4757
                | 0x4759
                | 0x475a
                | 0x4756
                | 0x5656
                | 0x4755
                | 0x4754
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn secondary_channel_requires_initialized_control_and_survives_modern_adapter() {
        let mut input = Input {
            features: [1; 9],
            vendor: 0x1002,
            device: 0x744c,
        };
        assert_eq!(word(input), 0x4fdff);
        assert!(crate::terrain_material::unlit_secondary_color(0x68016, word(input)).unwrap());
        input.features[8] = 0;
        assert!(!crate::terrain_material::unlit_secondary_color(0x68016, word(input)).unwrap());
    }
    #[test]
    fn legacy_adapter_overrides_do_not_apply_to_adjacent_ids() {
        let mut input = Input {
            features: [1; 9],
            vendor: 0x121a,
            device: 3,
        };
        assert_eq!(word(input) & 0x8000, 0);
        input.device = 4;
        assert_ne!(word(input) & 0x8000, 0);
        input.vendor = 0x1002;
        input.device = 0x5058;
        assert_eq!(word(input) & 0x80, 0);
        input.device = 0x5059;
        assert_ne!(word(input) & 0x80, 0);
    }
}
