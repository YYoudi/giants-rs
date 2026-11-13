//! Ordinary partial-controller ownership of the shared voice/pain countdowns.
//! Prototype PS2 `mecc_logic` is an unknown navigation relationship. The PC
//! countdown fragment and global initializer are independently acquired; no
//! prototype signature/layout or complete primary scheduling is imported.
use crate::resident_states::{Id, Session};

/// One application-lifetime clock, transferred through each active Session.
/// Only the controlled actor's real frame samples advance it; menu idle time
/// and duplicate preparation do not. Active cues can update its Session value.
pub struct Clock {
    saved: f32,
    owner: Option<Id>,
    failure: Option<String>,
}
impl Default for Clock {
    fn default() -> Self {
        Self {
            // Exact debug-PC initialized data, independently read from the
            // pinned image. This is ten seconds, not an assumed zero reset.
            saved: 10.,
            owner: None,
            failure: None,
        }
    }
}
impl Clock {
    pub fn failure(&self) -> Option<&str> {
        self.failure.as_deref()
    }
    pub fn saved(&self) -> f32 {
        self.saved
    }

    pub fn advance(
        &mut self,
        session: &mut Session,
        owner: Id,
        step: u64,
        sample: u64,
        seconds: f32,
    ) -> Result<bool, String> {
        if self.failure.is_some() {
            return Err("controlled Mecc voice clock already failed".into());
        }
        if self.owner.is_some_and(|old| old != owner) {
            return Err("controlled Mecc voice clock already belongs to another live actor".into());
        }
        session.mecc_health(owner)?;
        session.controlled_mecc_motion(owner, step)?;
        if sample == 0 || !seconds.is_finite() || seconds < 0. {
            return Err(
                "controlled Mecc voice clock requires its actual finite frame sample".into(),
            );
        }
        if self.owner.is_none() {
            if session.mecc_countdown_world().is_some() {
                return Err(
                    "controlled Mecc voice clock cannot replace another shared timer owner".into(),
                );
            }
            session.prepare_mecc_countdown_world(self.saved)?;
            self.owner = Some(owner);
        }
        let result = session.advance_controlled_mecc_hit_clock(owner, step, sample, seconds);
        if let Err(error) = &result {
            self.failure = Some(error.clone());
        }
        result
    }

    /// Preserve the actual post-cue value before Session teardown. No failure
    /// reset, guessed value or new mission's timer can substitute for this owner.
    pub fn release(&mut self, session: &Session, ticket: u64) -> Result<bool, String> {
        let Some(owner) = self.owner else {
            return Ok(false);
        };
        if owner.ticket() != ticket || session.ticket() != ticket {
            return Err("controlled Mecc clock release ticket is foreign".into());
        }
        // The shared value belongs to the Session, independently of the
        // actor's health/type or subsequent retirement. Never synthesize a live
        // actor merely to preserve an application-lifetime countdown.
        let result: Result<bool, String> = (|| {
            let actual = session
                .mecc_countdown_world()
                .ok_or("controlled Mecc voice clock disappeared before release")?;
            if !actual.is_finite() {
                return Err("controlled Mecc voice clock release is nonfinite".into());
            }
            self.saved = actual;
            self.owner = None;
            Ok(true)
        })();
        if let Err(error) = &result {
            self.failure = Some(error.clone());
        }
        result
    }
}
