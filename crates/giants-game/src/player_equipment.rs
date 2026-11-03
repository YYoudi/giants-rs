//! Cold normal-Mecc equipment, before mission continuity or multiplayer-base rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Initial {
    pub selected: i32,
    pub ammo: [i32; 10],
    pub utility: [i32; 3],
    pub weapons: [i32; 4],
    pub special: i32,
}
impl Initial {
    pub fn non_base(controlled: bool) -> Self {
        Self {
            selected: 0,
            ammo: [0; 10],
            utility: [-1; 3],
            weapons: [if controlled { 1 } else { 2 }, -1, -1, -1],
            special: -1,
        }
    }
    pub fn words(self) -> [u32; 19] {
        let mut words = [0; 19];
        words[0] = self.selected as u32;
        for (dst, value) in words[1..].iter_mut().zip(
            self.ammo
                .into_iter()
                .chain(self.utility)
                .chain(self.weapons)
                .chain([self.special]),
        ) {
            *dst = value as u32;
        }
        words
    }
}
