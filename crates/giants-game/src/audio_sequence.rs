//! Measured flags-3 sound sequence lifecycle. Device operations are explicit
//! backend calls; timing follows the returned release-voice duration.
use crate::{arithmetic::Precision, mecc_audio::Sequence};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    Local,
    Attached,
}

pub trait Backend {
    type Handle: Copy + Default;
    fn stop_sound(&mut self, sound: u32, owner: u64) -> Result<(), String>;
    fn play(&mut self, sound: u32, owner: u64, route: Route) -> Result<Self::Handle, String>;
    fn duration(&mut self, handle: Self::Handle) -> Result<f32, String>;
    fn volume(&mut self, handle: Self::Handle, fraction: f32) -> Result<(), String>;
    fn stop(&mut self, handle: Self::Handle) -> Result<(), String>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Pending,
    Running,
    Releasing,
    Finished,
    Failed,
}

pub struct Playback<H> {
    pub definition: Sequence,
    pub owner: u64,
    pub route: Route,
    phase: Phase,
    continuous: Option<H>,
    remaining: f32,
    duration: f32,
}

impl<H: Copy + Default> Playback<H> {
    pub fn new(definition: Sequence, owner: u64, route: Route) -> Result<Self, String> {
        if definition.flags != 3 || owner == 0 {
            return Err("sound sequence outside measured flags-3 actor profile".into());
        }
        Ok(Self {
            definition,
            owner,
            route,
            phase: Phase::Pending,
            continuous: None,
            remaining: 1.,
            duration: 0.,
        })
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }
    pub fn remaining(&self) -> f32 {
        self.remaining
    }
    pub fn release_duration(&self) -> f32 {
        self.duration
    }

    fn finish<B: Backend<Handle = H> + ?Sized>(&mut self, backend: &mut B) -> Result<(), String> {
        if let Some(handle) = self.continuous {
            backend.stop(handle)?;
            self.continuous = None;
        }
        self.phase = Phase::Finished;
        Ok(())
    }

    /// Retire the continuous voice, including the local player's sequence.
    /// The backend's mission registry must also retire its one-shot voices.
    pub fn cancel<B: Backend<Handle = H> + ?Sized>(
        &mut self,
        backend: &mut B,
    ) -> Result<(), String> {
        self.finish(backend)
    }

    pub fn update<B: Backend<Handle = H> + ?Sized>(
        &mut self,
        dt: f32,
        paused: bool,
        attached_owner_deleted: bool,
        precision: Precision,
        backend: &mut B,
    ) -> Result<(), String> {
        if !dt.is_finite() || dt < 0. {
            return Err("sequence delta outside finite nonnegative profile".into());
        }
        if matches!(self.phase, Phase::Finished | Phase::Failed) {
            return Ok(());
        }
        // The native attached-object retirement precedes the pause gate.
        if self.route == Route::Attached && attached_owner_deleted {
            return self.finish(backend);
        }
        if paused {
            return Ok(());
        }
        let result = match self.phase {
            Phase::Pending => (|| {
                backend.stop_sound(self.definition.release, self.owner)?;
                backend.play(self.definition.start, self.owner, self.route)?;
                self.continuous =
                    Some(backend.play(self.definition.continuous, self.owner, self.route)?);
                self.phase = Phase::Running;
                Ok(())
            })(),
            Phase::Releasing => {
                let difference = precision.round(f64::from(self.remaining) - f64::from(dt));
                self.remaining = difference as f32;
                if difference <= 0. {
                    self.finish(backend)
                } else {
                    let fraction = precision.round(difference / f64::from(self.duration)) as f32;
                    backend.volume(self.continuous.unwrap_or_default(), fraction)
                }
            }
            _ => Ok(()),
        };
        if result.is_err() {
            self.phase = Phase::Failed;
        }
        result
    }

    /// Called once for the actual sequence token. The token is released to the
    /// controller immediately; the old voice may still be fading separately.
    pub fn interrupt<B: Backend<Handle = H> + ?Sized>(
        &mut self,
        backend: &mut B,
    ) -> Result<(), String> {
        if !matches!(self.phase, Phase::Pending | Phase::Running) {
            return Ok(());
        }
        let result = (|| {
            if self.route == Route::Attached {
                backend.stop_sound(self.definition.start, self.owner)?;
            }
            let release = backend.play(self.definition.release, self.owner, self.route)?;
            let duration = backend.duration(release)?;
            if !duration.is_finite() || duration < 0. {
                return Err("backend returned unsupported release duration".into());
            }
            self.duration = duration;
            self.remaining = duration;
            self.phase = Phase::Releasing;
            Ok(())
        })();
        if result.is_err() {
            self.phase = Phase::Failed;
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Retry {
        stops: usize,
    }
    impl Backend for Retry {
        type Handle = u32;
        fn stop_sound(&mut self, _: u32, _: u64) -> Result<(), String> {
            Ok(())
        }
        fn play(&mut self, _: u32, _: u64, _: Route) -> Result<u32, String> {
            Ok(7)
        }
        fn duration(&mut self, _: u32) -> Result<f32, String> {
            Ok(0.25)
        }
        fn volume(&mut self, _: u32, _: f32) -> Result<(), String> {
            Ok(())
        }
        fn stop(&mut self, handle: u32) -> Result<(), String> {
            assert_eq!(handle, 7);
            self.stops += 1;
            if self.stops == 1 {
                Err("temporary backend stop failure".into())
            } else {
                Ok(())
            }
        }
    }
    #[test]
    fn failed_backend_stop_retains_ownership_for_cleanup_retry() {
        let spec = Sequence {
            start: 1,
            continuous: 2,
            release: 3,
            flags: 3,
        };
        let mut voice = Playback::new(spec, 1, Route::Local).unwrap();
        let mut backend = Retry { stops: 0 };
        voice
            .update(0., false, false, Precision::Double53, &mut backend)
            .unwrap();
        assert!(voice.cancel(&mut backend).is_err());
        assert_eq!(voice.phase(), Phase::Running);
        voice.cancel(&mut backend).unwrap();
        assert_eq!(voice.phase(), Phase::Finished);
        assert_eq!(backend.stops, 2);
    }
}
