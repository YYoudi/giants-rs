//! Exact-debug active-list decision under supplied node flags/owner validity.
//! Pool unlinking, devices, lifetime scheduling and owner identity are upstream.
#[derive(Clone, Copy, Debug)]
pub struct Decision {
    pub flags: u32,
    pub owner_present: bool,
    pub update: bool,
    pub stop: bool,
    pub release: bool,
    pub detached: bool,
}
pub fn decide(mut flags: u32, mut owner_present: bool, owner_valid: bool) -> Decision {
    let mut result = Decision {
        flags,
        owner_present,
        update: false,
        stop: false,
        release: false,
        detached: false,
    };
    if flags & 0x40000000 != 0 {
        result.release = true;
        return result;
    }
    if flags & 0x80000000 != 0 {
        return result;
    }
    if flags & 0x8000 != 0 && (!owner_present || !owner_valid) {
        flags &= !0x8000;
        if flags & 0x400 == 0 {
            result.flags = flags | 0x80000000;
            result.stop = true;
            return result;
        }
        owner_present = false;
        result.detached = true;
    }
    result.flags = flags;
    result.owner_present = owner_present;
    result.update = flags & 0x20000000 == 0;
    result
}
