//! Required tracked external release before acquired core actor retirement.
//! World cascade, FX/audio cleanup and scheduling are caller-owned prerequisites.

/// Release actual actor-bound FX/audio/render handles, or report the missing
/// service. Success permits core unlink/drop only after ownership is rechecked.
pub trait Services {
    fn release(
        &mut self,
        session: &mut crate::resident_states::Session,
        actor: crate::resident_states::Id,
    ) -> Result<(), String>;
}
