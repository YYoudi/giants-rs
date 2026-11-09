//! Acquired selected-spell range branch. Inventory/selection mutation and the
//! native default-kind branch remain separate producers.
use crate::projectile_definition::DebugDefinitions;

pub fn definition(selected: i32, kinds: [i32; 4]) -> Result<Option<i32>, String> {
    if selected <= 0 {
        return Ok(None);
    }
    let kind = kinds
        .get(usize::try_from(selected - 1).map_err(|_| "Delphi spell selection overflow")?)
        .ok_or("Delphi spell selection outside acquired four-slot producer")?;
    let id = match kind {
        0x2c => 0x2b,
        0x2d => 0x16,
        0x2e => 0x15,
        0x2f | 0x30 => 0x17,
        _ => {
            return Err("Delphi selected spell kind has an unacquired default range branch".into());
        }
    };
    Ok(Some(id))
}
pub fn resolve(
    selected: i32,
    kinds: [i32; 4],
    definitions: &DebugDefinitions,
) -> Result<f32, String> {
    let Some(id) = definition(selected, kinds)? else {
        return Ok(9000.);
    };
    let definition = definitions
        .entries
        .get(&id)
        .ok_or("Delphi spell range declaration absent")?;
    let value = f32::from_bits(definition.word(0x2c)?);
    if !value.is_finite() {
        return Err("Delphi spell range outside finite source profile".into());
    }
    Ok(value)
}
