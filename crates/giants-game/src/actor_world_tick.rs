//! Exact-D common-list actor iteration. Required callback services remain real
//! dependencies; this policy does not supply behaviors, clocks or readiness.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Inactive,
    WorldAborted,
    UpdatedByPrimary,
    Updated,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Fresh,
    Primary,
    Secondary,
    ObjectTime,
    Animation,
    Finalization,
    Cleanup,
    Finished,
    Failed,
}
pub trait Services {
    fn flags(&self) -> Result<u32, String>;
    fn set_flags(&mut self, flags: u32) -> Result<(), String>;
    fn primary(&mut self) -> Result<(), String>;
    fn controlled_actor_present(&self) -> bool;
    fn secondary(&mut self) -> Result<(), String>;
    fn object_time(&mut self) -> Result<f32, String>;
    fn animation(&mut self, seconds: f32) -> Result<(), String>;
    fn finalization(&mut self) -> Result<(), String>;
}
#[derive(Debug)]
pub struct Tick {
    phase: Phase,
    failure: Option<String>,
}
impl Default for Tick {
    fn default() -> Self {
        Self::new()
    }
}
impl Tick {
    pub fn new() -> Self {
        Self {
            phase: Phase::Fresh,
            failure: None,
        }
    }
    pub fn phase(&self) -> Phase {
        self.phase
    }
    pub fn failure(&self) -> Option<&str> {
        self.failure.as_deref()
    }
    /// One admitted list iteration. Errors preserve already executed effects
    /// and latch this token against replay, including failures during cleanup.
    pub fn run(&mut self, services: &mut impl Services) -> Result<Outcome, String> {
        if self.phase != Phase::Fresh {
            return Err("actor world tick already consumed or failed".into());
        }
        let result = self.run_inner(services);
        match &result {
            Ok(_) => self.phase = Phase::Finished,
            Err(e) => {
                self.phase = Phase::Failed;
                self.failure = Some(e.clone());
            }
        }
        result
    }
    fn run_inner(&mut self, s: &mut impl Services) -> Result<Outcome, String> {
        let flags = s.flags()?;
        if flags & 0xc000 != 0 {
            return Ok(Outcome::Inactive);
        }
        self.phase = Phase::Primary;
        s.set_flags(flags & !0x40)?;
        s.primary()?;
        if !s.controlled_actor_present() {
            return Ok(Outcome::WorldAborted);
        }
        let flags = s.flags()?;
        if flags & 0xc000 != 0 {
            return Ok(Outcome::Inactive);
        }
        let updated = flags & 0x40 != 0;
        if !updated {
            self.phase = Phase::Secondary;
            s.secondary()?;
            self.phase = Phase::ObjectTime;
            let seconds = s.object_time()?;
            self.phase = Phase::Animation;
            s.animation(seconds)?;
            self.phase = Phase::Finalization;
            s.finalization()?;
        }
        self.phase = Phase::Cleanup;
        let flags = s.flags()?;
        s.set_flags(flags & !0x800)?;
        Ok(if updated {
            Outcome::UpdatedByPrimary
        } else {
            Outcome::Updated
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    struct F {
        flags: u32,
        calls: Vec<&'static str>,
        fail: Option<&'static str>,
    }
    impl F {
        fn call(&mut self, name: &'static str) -> Result<(), String> {
            self.calls.push(name);
            if self.fail == Some(name) {
                Err(name.into())
            } else {
                Ok(())
            }
        }
    }
    impl Services for F {
        fn flags(&self) -> Result<u32, String> {
            Ok(self.flags)
        }
        fn set_flags(&mut self, flags: u32) -> Result<(), String> {
            self.flags = flags;
            Ok(())
        }
        fn primary(&mut self) -> Result<(), String> {
            self.call("primary")
        }
        fn controlled_actor_present(&self) -> bool {
            true
        }
        fn secondary(&mut self) -> Result<(), String> {
            self.call("secondary")
        }
        fn object_time(&mut self) -> Result<f32, String> {
            self.call("time")?;
            Ok(1. / 60.)
        }
        fn animation(&mut self, _: f32) -> Result<(), String> {
            self.flags |= 0x40;
            self.call("animation")
        }
        fn finalization(&mut self) -> Result<(), String> {
            self.call("finalization")
        }
    }
    #[test]
    fn failing_service_keeps_partial_flags_and_never_replays_effects() {
        let mut f = F {
            flags: 0x840,
            calls: vec![],
            fail: Some("finalization"),
        };
        let mut t = Tick::new();
        assert!(t.run(&mut f).is_err());
        assert_eq!(t.failure(), Some("finalization"));
        assert_eq!(f.flags, 0x840);
        let before = f.calls.clone();
        assert!(t.run(&mut f).is_err());
        assert_eq!(f.calls, before);
        assert_eq!(t.phase(), Phase::Failed);
    }
    #[test]
    fn inactive_actor_retains_scratch_flags_without_callbacks() {
        let mut f = F {
            flags: 0xc840,
            calls: vec![],
            fail: None,
        };
        let mut t = Tick::new();
        assert_eq!(t.run(&mut f).unwrap(), Outcome::Inactive);
        assert_eq!(f.flags, 0xc840);
        assert!(f.calls.is_empty());
        assert!(t.run(&mut f).is_err());
    }
}
