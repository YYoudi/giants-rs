//! Typed Mecc slots resolved against a cold equipment declaration catalog.
//! Pickup/drop scheduling and mission inventory initialization are separate.
use giants_formats::weapons::Catalog;

#[derive(Clone, Copy, Debug)]
pub struct Inventory {
    pub utility: [i32; 3],
    pub weapons: [i32; 3],
    pub special: i32,
}
#[derive(Clone, Copy, Debug)]
pub struct Weights {
    pub slots: [i32; 6],
    pub special: Option<i32>,
}
impl Inventory {
    pub fn weights(self, catalog: &Catalog) -> Result<Weights, &'static str> {
        let weight = |id| {
            if !(-1..75).contains(&id) {
                return Err("inventory item outside native table");
            }
            // Undeclared cold entries retain their initial zero weight. A
            // declared -1 entry is still read by the native utility/weapon getters.
            Ok(catalog.icons.get(&id).map(|i| i.weight).unwrap_or(0))
        };
        let mut slots = [0; 6];
        for (n, id) in self.utility.into_iter().chain(self.weapons).enumerate() {
            slots[n] = weight(id)?;
        }
        let special = if self.special > 0 {
            Some(weight(self.special)?)
        } else {
            None
        };
        Ok(Weights { slots, special })
    }
}
