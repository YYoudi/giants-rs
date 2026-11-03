//! Initial list membership from explicit object fields. This is not visibility.
use giants_formats::class_program::Program;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DirectFlags {
    pub active: u32,
    pub personality: u32,
}
impl DirectFlags {
    /// Direct flag writes only, starting from a zeroed class template. Other
    /// attributes, model resolution and state callbacks remain separate.
    pub fn project(class: i32, program: &Program) -> Self {
        let mut out = Self::default();
        for record in &program.records {
            out.personality |= match record.opcode {
                2 if matches!(class, 257 | 996 | 997 | 998 | 999 | 1029 | 1030) => 0x80000000,
                4 => {
                    0x400000
                        | if matches!(class, 1 | 2 | 3 | 215) {
                            0x40000
                        } else {
                            0
                        }
                }
                0x14 => 2,
                0x16 => 4,
                0x18 => 8,
                0x1d => 0x40,
                0x20 => 0x80,
                0x22 => 0x800,
                0x23 => 0x1000,
                0x2a | 0x39 => 0x20000,
                0x2b => 0x100,
                0x2e => 0x600,
                0x2f => 0x10000,
                0x33 => 0x80000,
                0x34 => 0x100000,
                0x35 => 0x200000,
                0x3d => 0x1000000,
                0x41 => 0x8000e00,
                0x51 => 0x28000e00,
                0x54 => 0x80000000,
                0x7d => 0x2000000,
                0x7e => 0x10000000,
                _ => 0,
            };
            out.active |= match record.opcode {
                0x1b => 0x10,
                0x41 | 0x51 => 0x40000010,
                0x52 => 0x20000,
                0x53 => 2,
                0x55 => 0x40000000,
                _ => 0,
            };
        }
        out
    }
}
/// Bits select constructor list heads; their gameplay meanings are not named.
/// Call with current fields to classify an actual instance. A projected class
/// input does not establish that subsequent initialization leaves it unchanged.
pub fn membership(class: i32, state: u32, flags: DirectFlags) -> u32 {
    let mut mask = if flags.personality & 0x20000 != 0 {
        0x11
    } else if flags.active & 0x4000c000 == 0 && flags.personality & 0x800 == 0 {
        0x10
    } else {
        0
    };
    mask |= match state {
        6 => 0x20,
        8 => 0x100,
        18 => 0x811,
        21 => 0x200,
        32 | 74 | 82 | 83 | 91 => 0x1000,
        84 => 0x400,
        _ => 0,
    };
    mask | match class {
        1 | 41 | 745 => 8,
        2 | 4 | 84 | 85 | 133 | 991 => 2,
        3 | 6 | 7 => 4,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn suppression_priority_and_state_memberships_remain_distinct() {
        let mut flags = DirectFlags::default();
        assert_eq!(membership(1000, 0, flags), 0x10);
        flags.active = 0x4000;
        assert_eq!(membership(1000, 0, flags), 0);
        assert_eq!(membership(1000, 18, flags), 0x811);
        flags.personality = 0x20000 | 0x800;
        assert_eq!(membership(3, 6, flags), 0x35);
        assert_eq!(membership(-1, 6, flags), 0x31);
    }
}
