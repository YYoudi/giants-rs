//! Shared nonzero ANM cache epoch, owned by the composing world/services.
//! It must not be restarted for each actor or each animation pass.
#[derive(Default)]
pub struct Stamp(pub u32);
impl Stamp {
    pub(crate) fn advance(&mut self) -> u32 {
        self.0 = self.0.wrapping_add(1);
        if self.0 == 0 {
            self.0 = 1;
        }
        self.0
    }
}
