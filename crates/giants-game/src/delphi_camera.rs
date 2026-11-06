//! Delphi's pre-heading authored-view override. The surrounding primary,
//! target production, ordinary camera update and renderer remain separate.
use crate::procedural_aim::Matrix;

#[derive(Clone, Debug, Default)]
pub struct State {
    pub enabled: bool,
    pub descriptor: Option<i32>,
    pub matrix_selected: bool,
    /// Retained across reset. Absence is never replaced by an identity pose.
    pub retained_view: Option<Matrix>,
    pub vertical_fov: Option<f32>,
    pub failure: Option<String>,
}
impl State {
    /// Only a selected active view may be consumed as a camera override.
    pub fn active_view(&self) -> Result<Option<Matrix>, &'static str> {
        if self.failure.is_some() {
            return Err("Delphi camera producer is quarantined");
        }
        if !self.enabled || !self.matrix_selected {
            return Ok(None);
        }
        self.retained_view
            .map(Some)
            .ok_or("selected Delphi view absent")
    }
}

pub struct Environment {
    pub local_owner: bool,
    /// Supplied source mode; only -1 permits this callback's view changes.
    pub mode: i32,
    pub buttons: u32,
    pub aspect: f32,
    /// An acquired actor identity boundary, not a target-selection producer.
    pub target: Option<crate::resident_states::Id>,
}
