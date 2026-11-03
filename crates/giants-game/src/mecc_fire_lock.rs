//! D firing-parent/reference selection over current ordered owned lists.
//! A selected parent is not a resolved muzzle, launch or completed fire service.
use crate::{mecc_wield::Child, projectile_definition::Definition};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Binding<K> {
    /// Attribution remains the firing actor even when an attached gun launches.
    pub source: K,
    pub parent: K,
    pub reference: i32,
}

pub fn select<K: Copy + Eq>(
    source: K,
    control_flags: u32,
    selector: u32,
    definition: &Definition,
    held: &[Child<K>],
    helpers: &[Child<K>],
) -> Result<Binding<K>, &'static str> {
    let projectile = selector as u8;
    if !(0..53).contains(&definition.id) || definition.id != i32::from(projectile) {
        return Err("fire lock definition does not match the effective selector");
    }
    let (candidate, reference) = match projectile {
        2 | 13 => (None, 0),
        24 => (
            helpers.iter().find(|child| {
                child.flags & 0x8000 == 0 && child.class == 991 && child.parent == Some(source)
            }),
            2,
        ),
        _ => {
            let hand = i32::from(control_flags & 0x100 == 0);
            (
                held.iter().find(|child| {
                    child.flags & 0x8000 == 0
                        && child.parent == Some(source)
                        && child.reference == hand
                }),
                1,
            )
        }
    };
    // The native fallback overwrites the reference as well, including the
    // earlier zero selected for declarations2/13. Do not retain that zero.
    Ok(
        if let Some(child) = candidate.filter(|child| child.class != definition.class()) {
            Binding {
                source,
                parent: child.id,
                reference,
            }
        } else {
            Binding {
                source,
                parent: source,
                reference: 1,
            }
        },
    )
}
