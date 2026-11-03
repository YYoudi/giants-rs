//! Director-local render gates. Auxiliary input is retained without inferring
//! its meaning; the optional actor pass has no independent renderer consumer yet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Policy {
    pub actor_auxiliary_disabled: bool,
    pub lens_disabled: bool,
    pub auxiliary_word: u32,
}
impl Policy {
    pub fn apply(&mut self, opcode: u32, words: &[u32]) -> Result<(), &'static str> {
        match (opcode, words) {
            (6, []) => {
                self.actor_auxiliary_disabled = false;
                self.auxiliary_word = 0;
            }
            (45, []) => self.actor_auxiliary_disabled = true,
            (46, []) => self.lens_disabled = true,
            _ => return Err("unsupported cinematic render-policy command"),
        }
        Ok(())
    }
}
