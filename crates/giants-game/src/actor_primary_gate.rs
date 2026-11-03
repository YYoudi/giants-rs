//! Exact-D primary-wrapper prefix. The actual callback remains a required service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry {
    pub invoke: bool,
    pub binding_flags: u32,
    pub previous_rates: [u32; 4],
}
pub fn enter(
    object_flags: u32,
    binding_flags: u32,
    commands: [u8; 8],
    rates: [u32; 4],
    previous_rates: [u32; 4],
) -> Entry {
    let invoke = object_flags & 0xc000 == 0 && (binding_flags & 0x10 == 0 || commands != [0; 8]);
    Entry {
        invoke,
        binding_flags: if invoke {
            binding_flags & !0x10
        } else {
            binding_flags
        },
        previous_rates: if invoke { rates } else { previous_rates },
    }
}
