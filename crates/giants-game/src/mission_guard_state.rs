//! Persistent PC process fields consumed by ordinary damage callbacks.
//! Prototype PS2 `mission_init` / `mission_fail` remain unknown name routes.
//! This is a bounded field projection, not full world/startup/UI reconstruction.
//! Initial image values alone grant no scenario scope. Configuration and source
//! entry have distinct writes; a new ticket never clears a failed/unknown world.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scope {
    pub ticket: u64,
    pub scenario: usize,
    pub slot: usize,
}

#[derive(Debug)]
pub struct Process {
    failed: u32,
    completed: u32,
    edit: u32,
    health_suppressed: u32,
    failure_elapsed: f32,
    configured: bool,
    scope: Option<Scope>,
    uncertainty: Option<&'static str>,
}

impl Process {
    /// The verified debug-PC image's zero-initialized process storage. Call once
    /// at application launch, before configuration dispatch; never per mission,
    /// on a missing service, or to recover an uncertain/failed process.
    pub fn start_process() -> Self {
        Self {
            failed: 0,
            completed: 0,
            edit: 0,
            health_suppressed: 0,
            failure_elapsed: 0.,
            configured: false,
            scope: None,
            uncertainty: None,
        }
    }

    /// Actual configuration-start prefix: it clears health suppression, while
    /// preserving failure, completion and edit. It acknowledges no later provider.
    /// The current normal CLI exposes none of the native cheat/edit writers.
    pub fn begin_configuration(&mut self) -> Result<(), String> {
        if self.configured || self.scope.is_some() {
            return Err("configuration guard prefix must belong to actual process startup".into());
        }
        self.health_suppressed = 0;
        self.configured = true;
        Ok(())
    }

    /// Acquired scenario-start global prefix. The caller must supply the actual
    /// parsed scenario/slot and lifecycle event. This does not complete the
    /// scenario loader or acknowledge opening-film, save, music or UI services.
    pub fn begin_scenario(&mut self, scope: Scope) -> Result<(), String> {
        if !self.configured || self.scope.is_some() || scope.scenario >= 27 || scope.slot >= 32 {
            return Err(
                "mission guards require configured process and fresh source scenario scope".into(),
            );
        }
        self.completed = 0;
        self.failure_elapsed = 0.;
        self.scope = Some(scope);
        Ok(())
    }
    pub fn scope(&self) -> Option<Scope> {
        self.scope
    }
    pub fn leave_scenario(&mut self, ticket: u64) -> Result<(), String> {
        self.require_ticket(ticket)?;
        self.scope = None;
        Ok(())
    }
    fn require_ticket(&self, ticket: u64) -> Result<(), String> {
        if self.scope.is_none_or(|s| s.ticket != ticket) {
            return Err("mission guard scope is absent or foreign".into());
        }
        Ok(())
    }
    fn require_known(&self, ticket: u64) -> Result<(), String> {
        self.require_ticket(ticket)?;
        if let Some(reason) = self.uncertainty {
            return Err(format!("mission guard producer unbound: {reason}"));
        }
        Ok(())
    }
    /// An actually reached unported writer invalidates knowledge. A portable
    /// objective counter must not be substituted for native completion fields.
    /// Retiring/re-entering a scenario preserves this uncertainty.
    pub fn require_unbound_transition(
        &mut self,
        ticket: u64,
        reason: &'static str,
    ) -> Result<(), String> {
        self.require_ticket(ticket)?;
        self.observe_unbound_writer(reason);
        Ok(())
    }
    /// A reached process-global writer also matters when no combat scenario
    /// was bound (for example a mission without Rippers). Invalidation grants
    /// no flag value and cannot be undone by cancellation or a new ticket.
    pub fn observe_unbound_writer(&mut self, reason: &'static str) {
        self.uncertainty.get_or_insert(reason);
    }
    pub fn world_suppressed(&self, ticket: u64) -> Result<bool, String> {
        self.require_known(ticket)?;
        Ok(self.failed != 0 || self.completed != 0)
    }
    pub fn receiver_gates(&self, ticket: u64) -> Result<(bool, bool), String> {
        self.require_known(ticket)?;
        Ok((self.edit != 0, self.health_suppressed != 0))
    }
    /// Actual failure-producer invocation only, with the current source count
    /// and actual frame sample. The required menu remains an external consumer;
    /// its absent acknowledgment must not clear these fields.
    pub fn request_failure(
        &mut self,
        ticket: u64,
        counter: i32,
        required: i32,
        seconds: f32,
    ) -> Result<crate::mission_failure::Outcome, String> {
        self.require_known(ticket)?;
        let mut state = crate::mission_failure::State {
            failed: self.failed,
            elapsed: self.failure_elapsed,
        };
        let result = state
            .request(crate::mission_failure::Context {
                counter,
                required,
                completed: self.completed,
                seconds,
            })
            .map_err(str::to_owned)?;
        self.failed = state.failed;
        self.failure_elapsed = state.elapsed;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn scope(ticket: u64) -> Scope {
        Scope {
            ticket,
            scenario: 3,
            slot: 1,
        }
    }
    #[test]
    fn image_defaults_never_supply_scenario_readiness_or_reset_a_failure() {
        let mut p = Process::start_process();
        assert!(p.world_suppressed(1).is_err());
        assert!(p.begin_scenario(scope(1)).is_err());
        p.begin_configuration().unwrap();
        assert!(p.world_suppressed(1).is_err());
        p.begin_scenario(scope(1)).unwrap();
        assert_eq!(p.world_suppressed(1), Ok(false));
        assert_eq!(p.receiver_gates(1), Ok((false, false)));
        assert!(p.begin_scenario(scope(2)).is_err());
        assert!(p.world_suppressed(2).is_err());
        assert_eq!(
            p.request_failure(1, 0, 1, 1.).unwrap(),
            crate::mission_failure::Outcome::Pending
        );
        assert_eq!(p.world_suppressed(1), Ok(true));
        p.leave_scenario(1).unwrap();
        p.begin_scenario(scope(2)).unwrap();
        assert_eq!(p.world_suppressed(2), Ok(true));
        assert!(p.begin_configuration().is_err());
    }
    #[test]
    fn unsupported_transition_is_not_completed_flag_or_reentry_recovery() {
        let mut p = Process::start_process();
        p.begin_configuration().unwrap();
        p.begin_scenario(scope(1)).unwrap();
        p.require_unbound_transition(1, "completion clock/precheck writer")
            .unwrap();
        assert_eq!(p.completed, 0);
        assert!(p.world_suppressed(1).is_err());
        assert!(p.receiver_gates(1).is_err());
        p.leave_scenario(1).unwrap();
        p.begin_scenario(scope(2)).unwrap();
        assert!(p.world_suppressed(2).is_err());
    }
    #[test]
    fn unbound_global_writer_without_combat_scope_stays_unknown_on_entry() {
        let mut p = Process::start_process();
        p.begin_configuration().unwrap();
        p.observe_unbound_writer("source completion in a mission without combat scope");
        p.begin_scenario(scope(1)).unwrap();
        assert!(p.world_suppressed(1).is_err());
        assert!(p.receiver_gates(1).is_err());
    }
    #[test]
    #[ignore = "Requires GIANTS_MISSION_GUARD_PREFIX_CONTROLS; original configuration/scenario prefixes with poisoned process words, not full native startup/world readiness"]
    fn original_guard_prefix_controls() {
        let data =
            std::fs::read_to_string(std::env::var("GIANTS_MISSION_GUARD_PREFIX_CONTROLS").unwrap())
                .unwrap();
        let mut count = 0;
        for line in data.lines() {
            let w: Vec<u32> = line
                .split_whitespace()
                .map(|v| u32::from_str_radix(v, 16).unwrap())
                .collect();
            assert_eq!(w.len(), 14);
            let mut p = Process {
                failed: w[0],
                completed: w[1],
                edit: w[2],
                health_suppressed: w[3],
                failure_elapsed: f32::from_bits(w[4]),
                configured: false,
                scope: None,
                uncertainty: None,
            };
            p.begin_configuration().unwrap();
            assert_eq!(
                [
                    p.failed,
                    p.completed,
                    p.edit,
                    p.health_suppressed,
                    p.failure_elapsed.to_bits()
                ],
                w[5..10]
            );
            p.begin_scenario(scope(1)).unwrap();
            assert_eq!(
                [p.failed, p.completed, p.edit, p.health_suppressed],
                w[10..14]
            );
            assert_eq!(p.failure_elapsed.to_bits(), 0);
            count += 1;
        }
        assert_eq!(count, 4);
    }
}
