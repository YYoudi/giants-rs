//! Startup no-play projection, distinct from gain, capture muting and readiness.
//! The caller supplies a complete option token at option-dispatch time. This is
//! not a command-line lexer or an implementation of every safe-mode side effect.
//! Missing audio services and capture muting must never select this native mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Playback {
    suppressed: bool,
}
impl Playback {
    /// Projection of the native configuration initializer's bulk reset. Invoke
    /// at configuration startup, never as an audio-service failure fallback.
    pub fn begin_configuration(&mut self) {
        self.suppressed = false;
    }
    /// Recognize only options known to set the native no-play field. An
    /// unrecognized token leaves this projection unchanged and remains caller work.
    pub fn observe_startup_option(&mut self, token: &str) -> bool {
        if token.eq_ignore_ascii_case("-noplay") || token.eq_ignore_ascii_case("-safemode") {
            self.suppressed = true;
            true
        } else {
            false
        }
    }
    pub fn suppressed(self) -> bool {
        self.suppressed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn startup_projection_is_case_insensitive_exact_and_sticky() {
        let mut playback = Playback::default();
        for token in [
            "-nosound",
            "-noload",
            "-noplayX",
            " -noplay",
            "-noplay\0",
            "",
        ] {
            assert!(!playback.observe_startup_option(token));
            assert!(!playback.suppressed());
        }
        assert!(playback.observe_startup_option("-NoPlAy"));
        assert!(playback.suppressed());
        assert!(!playback.observe_startup_option("-noload"));
        assert!(playback.suppressed());
        let mut safe = Playback::default();
        assert!(safe.observe_startup_option("-SAFEMODE"));
        assert!(safe.suppressed());
        safe.begin_configuration();
        assert!(!safe.suppressed());
    }
}
