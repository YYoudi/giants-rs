//! Narrow projection of the actual configuration-start reset, not completion of
//! the original configuration provider or an audio-device acknowledgement.
use bevy::prelude::Resource;
use giants_game::startup_playback::Playback;

#[derive(Resource)]
pub(crate) struct Configuration {
    playback: Playback,
}

impl Configuration {
    /// Called by launch before dispatching options. The Rust CLI currently
    /// accepts neither native no-play nor safe-mode options; unsupported options
    /// still fail there, and option values must never be scanned as options.
    pub(crate) fn begin() -> Self {
        let mut playback = Playback::default();
        playback.begin_configuration();
        Self { playback }
    }

    pub(crate) fn playback(&self) -> Playback {
        self.playback
    }
}
