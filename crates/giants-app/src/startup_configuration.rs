//! Narrow projection of the actual configuration-start reset, not completion of
//! the original configuration provider or an audio-device acknowledgement.
use bevy::prelude::Resource;
use giants_game::startup_playback::Playback;

#[derive(Resource)]
pub(crate) struct Configuration {
    playback: Playback,
    combat_process: Option<giants_game::mission_guard_state::Process>,
}

impl Configuration {
    /// Called by launch before dispatching options. The Rust CLI currently
    /// accepts neither native no-play nor safe-mode options; unsupported options
    /// still fail there, and option values must never be scanned as options.
    pub(crate) fn begin() -> Self {
        let mut playback = Playback::default();
        playback.begin_configuration();
        let mut combat_process = giants_game::mission_guard_state::Process::start_process();
        combat_process
            .begin_configuration()
            .expect("fresh process configuration prefix");
        Self {
            playback,
            combat_process: Some(combat_process),
        }
    }

    pub(crate) fn playback(&self) -> Playback {
        self.playback
    }
    pub(crate) fn take_combat_process(
        &mut self,
    ) -> Option<giants_game::mission_guard_state::Process> {
        self.combat_process.take()
    }
}
