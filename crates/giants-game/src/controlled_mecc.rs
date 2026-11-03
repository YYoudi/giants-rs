//! Controller-owned motion projection, not a complete native state2 callback.
//! Pose/rates come from the actual physics producer after contact. Header caches
//! remain separate phases and must reject the old pose after publication.
#[derive(Clone, Copy, Debug)]
pub struct Frame {
    pub before: [f32; 3],
    pub position: [f32; 3],
    pub contact_previous: [f32; 3],
    pub angles: [f32; 3],
    pub scale: f32,
    pub velocity: [f32; 3],
    pub model_rates: [f32; 3],
}
#[derive(Clone, Copy, Debug)]
pub struct Owned {
    pub step: u64,
    pub frame: Frame,
    pub previous_history: [f32; 3],
}
impl Frame {
    pub(crate) fn validate(&self) -> Result<(), &'static str> {
        if !self
            .before
            .into_iter()
            .chain(self.position)
            .chain(self.contact_previous)
            .chain(self.angles)
            .chain([self.scale])
            .chain(self.velocity)
            .chain(self.model_rates)
            .all(f32::is_finite)
            || self.scale <= 0.
            || self.angles.iter().any(|v| v.abs() > 1_000_000.)
        {
            return Err("controlled Mecc projection outside finite acquired pose profile");
        }
        Ok(())
    }
}
impl Owned {
    pub(crate) fn current(&self, fields: &crate::actor_fields::Fields, step: u64) -> bool {
        self.step == step
            && fields.position.map(f32::to_bits) == self.frame.position.map(f32::to_bits)
            && fields.current.map(f32::to_bits) == self.frame.contact_previous.map(f32::to_bits)
            && fields.previous.map(f32::to_bits) == self.previous_history.map(f32::to_bits)
            && fields.angles.map(f32::to_bits) == self.frame.angles.map(f32::to_bits)
            && fields.scale.to_bits() == self.frame.scale.to_bits()
    }
}
