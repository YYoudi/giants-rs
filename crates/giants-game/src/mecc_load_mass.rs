//! Inventory-dependent mass calculation. The caller supplies resolved item
//! weights; this module does not invent a loadout or load the item table.
use crate::arithmetic::Precision;

#[derive(Clone, Copy, Debug)]
pub struct Input {
    pub base_mass: f32,
    pub base_gravity_mass: f32,
    pub special_mass: f32,
    /// Three utility slots followed by three weapon slots.
    pub item_weights: [i32; 6],
    pub special_weight: Option<i32>,
    pub response_flags: u32,
    pub object_flags: u32,
    pub has_physics: bool,
    pub attached: bool,
    pub previous: [f32; 2],
}

/// Returns [physical mass, gravity mass], preserving both when attached or
/// without a physics block. Slot getters and item lookup belong to the caller.
pub fn calculate(i: Input, precision: Precision) -> Result<[f32; 2], &'static str> {
    if ![i.base_mass, i.base_gravity_mass, i.special_mass]
        .into_iter()
        .chain(i.previous)
        .all(f32::is_finite)
        || i.base_mass <= 0.
        || !(-1_000_000. ..=1_000_000.).contains(&i.special_mass)
    {
        return Err("mass outside finite positive-base bounded profile");
    }
    let mut load = 0i32;
    for slot in 0..3 {
        load = load.wrapping_add(i.item_weights[slot]);
        load = load.wrapping_add(i.item_weights[slot + 3]);
    }
    load = load.wrapping_add(i.special_weight.unwrap_or(0));
    if i.response_flags & 0x800000 != 0 {
        load = load.wrapping_add(1);
    }
    let r = |v| precision.round(v);
    if i.object_flags & 0x20000 != 0 {
        load = load.wrapping_add(r(f64::from(i.special_mass) + 0.5).trunc() as i32);
    }
    if !i.has_physics || i.attached {
        return Ok(i.previous);
    }
    let mass = r(f64::from(i.base_mass) + f64::from(load)) as f32;
    let gravity =
        r(r(f64::from(i.base_gravity_mass) * f64::from(mass)) / f64::from(i.base_mass)) as f32;
    Ok([mass, gravity])
}
