//! Single-frame vertex-cache routing under explicit debug input fields.
//! Model/normal/skin guards, frame selection, multi-frame blending and the
//! runtime provenance of the force flag remain separate.
#[derive(Clone, Copy, Debug)]
pub struct Cache {
    pub epoch: u32,
    pub seconds: f32,
    pub animation: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    KeepEpoch,
    KeepTime,
    Deform,
}
impl Cache {
    pub fn single(
        &mut self,
        force: bool,
        epoch: u32,
        seconds: f32,
        animation: u32,
        descriptor_flags: u32,
    ) -> Result<Action, &'static str> {
        if !seconds.is_finite() || seconds < 0. {
            return Err("vertex clock outside finite nonnegative profile");
        }
        if force {
            self.epoch = 0;
            self.seconds = -1.;
            return Ok(Action::Deform);
        }
        if self.epoch == epoch {
            return Ok(Action::KeepEpoch);
        }
        self.epoch = epoch;
        if descriptor_flags & 2 != 0 {
            self.seconds = -1.;
            return Ok(Action::Deform);
        }
        if self.seconds == seconds && self.animation == animation {
            return Ok(Action::KeepTime);
        }
        self.seconds = seconds;
        self.animation = animation;
        Ok(Action::Deform)
    }
}
