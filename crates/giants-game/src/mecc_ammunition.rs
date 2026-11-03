//! Native inventory readers over explicit state/payload. Availability is not
//! a shot admission decision. Pickup, depletion of other weapons and mission
//! scheduling remain separate services.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    /// Non-normal state returns without writing the caller's output pointer.
    Unchanged,
    Null,
    Payload(usize),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Availability {
    pub amount: i32,
    pub slot: Slot,
}
fn word(payload: &[u8], at: usize) -> Result<u32, &'static str> {
    let bytes = payload
        .get(at..at + 4)
        .ok_or("inventory payload truncated")?;
    Ok(u32::from_le_bytes(bytes.try_into().unwrap()))
}
pub fn selected(state: u8, payload: &[u8]) -> Result<i32, &'static str> {
    if state != 2 {
        return Ok(0);
    }
    let slot = word(payload, 0x64)? as usize;
    if slot >= 4 {
        return Err("weapon selection outside four owned inventory slots");
    }
    Ok(word(payload, 0x180 + slot * 4)? as i32)
}
pub fn available(state: u8, payload: &[u8], icon: i32) -> Result<Availability, &'static str> {
    if state != 2 {
        return Ok(Availability {
            amount: 0,
            slot: Slot::Unchanged,
        });
    }
    let offset = match icon {
        3 | 37 => Some(0x14c),
        5 | 38 => Some(0x15c),
        6 | 39 => Some(0x160),
        7 | 43 => Some(0x170),
        8 => Some(0x150),
        9 => Some(0x154),
        14 | 40 => Some(0x164),
        15 | 41 => Some(0x168),
        16 | 42 => Some(0x16c),
        64 => Some(0x158),
        _ => None,
    };
    let Some(offset) = offset else {
        return Ok(Availability {
            amount: if icon == 19 { 999 } else { 1 },
            slot: Slot::Null,
        });
    };
    let raw = word(payload, offset)?;
    let amount = if offset == 0x15c {
        // Native helper truncates to a signed 64-bit integer, then this
        // getter consumes its low word. Invalid conversion yields the 64-bit
        // indefinite value, whose low word is zero.
        let value = f64::from(f32::from_bits(raw)).trunc();
        let limit = 9223372036854775808.;
        if value.is_finite() && (-limit..limit).contains(&value) {
            value as i64 as i32
        } else {
            0
        }
    } else {
        raw as i32
    };
    Ok(Availability {
        amount,
        slot: Slot::Payload(offset),
    })
}

/// Actual base-pistol use is a no-op. Never extend it to other icon IDs.
pub fn consume_base_pistol(icon: i32) -> Result<(), &'static str> {
    if icon == 1 {
        Ok(())
    } else {
        Err("other weapon depletion requires its actual producer")
    }
}
