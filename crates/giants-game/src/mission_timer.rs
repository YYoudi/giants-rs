//! Acquired PC failure-countdown continuation, not a live world initializer.
//! Prototype PS2 `mission_fail_flick__Fv` is an unknown navigation candidate.
//! PC debug-reference-1.1 entry 0x004973c0; prior aliases remain private.
//! The caller owns startup, authored timer writes and quarantine after failure.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct State {
    pub remaining: f32,
    pub film_started: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct Gates {
    pub failed: u32,
    pub edit: u32,
}

/// Each reached boundary requires its actual owner. An empty authored film is
/// different from unavailable mission metadata; no default provider is supplied.
pub trait Services {
    fn seconds(&mut self) -> Result<f32, String>;
    fn failure_film(&mut self, state: &State) -> Result<Option<String>, String>;
    fn request_failure(&mut self, state: &mut State) -> Result<(), String>;
    fn start_film(&mut self, state: &mut State, name: &str) -> Result<(), String>;
    fn failure_menu(&mut self, state: &mut State) -> Result<(), String>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Suppressed,
    Counting,
    FailureRequested,
    FilmRequested,
    FailureMenu,
}

impl State {
    /// The acquired display caller invokes the helper only for a clear failure
    /// word and a strictly positive remaining time (unordered values skip it).
    /// This is not a fixed-tick scheduler or the complete display callback.
    pub fn advance_display_countdown(
        &mut self,
        gates: Gates,
        services: &mut impl Services,
    ) -> Result<Option<Outcome>, String> {
        if gates.failed != 0 || self.remaining.partial_cmp(&0.) != Some(std::cmp::Ordering::Greater)
        {
            return Ok(None);
        }
        self.advance(gates, services).map(Some)
    }

    /// Preserve the timer store before any expired-path service. Successful
    /// failure/film calls precede the native retry reset; failed calls retain
    /// their partial writes and must not be replayed by an owning scheduler.
    pub fn advance(
        &mut self,
        gates: Gates,
        services: &mut impl Services,
    ) -> Result<Outcome, String> {
        if gates.failed != 0 || gates.edit != 0 {
            return Ok(Outcome::Suppressed);
        }
        let seconds = services.seconds()?;
        let remaining = f64::from(self.remaining) - f64::from(seconds);
        if !self.remaining.is_finite() || !seconds.is_finite() || !(remaining as f32).is_finite() {
            return Err("mission timer requires finite active clock arithmetic".into());
        }
        self.remaining = remaining as f32;
        if remaining > 0. {
            return Ok(Outcome::Counting);
        }
        let film = services.failure_film(self)?;
        if self.film_started != 0 {
            services.failure_menu(self)?;
            return Ok(Outcome::FailureMenu);
        }
        if let Some(name) = film.filter(|name| !name.is_empty()) {
            services.start_film(self, &name)?;
            self.film_started = 1;
            self.remaining = 0.001;
            Ok(Outcome::FilmRequested)
        } else {
            services.request_failure(self)?;
            self.remaining = 0.001;
            Ok(Outcome::FailureRequested)
        }
    }
}

#[cfg(test)]
#[path = "mission_timer_tests.rs"]
mod tests;
