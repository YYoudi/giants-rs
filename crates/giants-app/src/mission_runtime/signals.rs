//! Ticket-owned clock/noise producers for the partial ordinary mission runtime.
//! Bevy's monotonic real-time frame snapshot is the portable clock boundary;
//! native world admission, other populations and their callbacks remain separate.
use giants_game::{
    arithmetic::Precision,
    frame_clock::{self, Modes},
    resident_states::Id,
    world_noise::Noise,
};

#[derive(Default)]
pub(super) struct State {
    owner: Option<Id>,
    clock: Option<frame_clock::State>,
    frames: u64,
    pub(super) noise: Noise<Id>,
    pub(super) areas: giants_game::slow_areas::Pool<Id>,
}
impl State {
    pub(super) fn sample(&mut self, owner: Id, observed_ms: u32) -> Result<(), String> {
        if self.owner.is_some_and(|old| old != owner) {
            return Err("mission clock controlled owner changed".into());
        }
        let previous = self.clock.unwrap_or(frame_clock::State {
            warmup: 0,
            last_ms: 0,
            deadline_ms: 0,
            virtual_ms: 0,
            seconds: 0.,
            smoothed_seconds: 0.,
            elapsed_seconds: 0.,
            blend: 0.,
        });
        // This ordinary application route exposes no native debug slow/fixed/
        // synthetic mode. Both initialization observations use the actual Bevy
        // frame snapshot; no artificial nonzero timestamp is substituted.
        let modes = Modes {
            synthetic: false,
            slow: false,
            fixed: false,
            scale: 1.,
        };
        let next = if previous.last_ms == 0 {
            frame_clock::first_frame(
                previous,
                modes,
                observed_ms,
                observed_ms,
                Precision::Double53,
            )
        } else {
            frame_clock::advance(previous, modes, observed_ms, Precision::Double53)
        }
        .map_err(str::to_owned)?;
        self.owner = Some(owner);
        self.clock = Some(next);
        self.frames += 1;
        // The owned namespace is advanced before ordinary actor callbacks.
        // Current normal input exposes no slow-area weapon/birth action; a
        // future producer must register actual owners rather than assume areas.
        let _retired = self
            .areas
            .advance(next.seconds, Precision::Double53)
            .map_err(str::to_owned)?;
        Ok(())
    }
    pub(super) fn publish_noise(&mut self, owner: Id, position: [f32; 3]) -> Result<f32, String> {
        if self.owner != Some(owner) {
            return Err("mission noise requires its actual clock owner".into());
        }
        let clock = self.clock.ok_or("mission noise clock not sampled")?;
        if !position.into_iter().all(f32::is_finite) {
            return Err("mission noise position is nonfinite".into());
        }
        self.noise.publish(owner, clock.elapsed_seconds, position);
        Ok(clock.elapsed_seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clock_and_emissions_keep_the_ticket_owner_and_reset_on_new_session() {
        use giants_game::{
            resident_states::Session,
            state_storage::{Profile, Seed, Storage},
        };
        let seed = Seed {
            storage: Storage::new(Profile::Debug),
            transitions: vec![],
            unexecuted: vec![],
            diagnostics: vec![],
            unresolved: vec![],
        };
        let mut session = Session::new(19);
        let owner = session.register(0, &seed, &[], Default::default()).unwrap();
        let foreign = Session::new(20)
            .register(0, &seed, &[], Default::default())
            .unwrap();
        let mut state = State::default();
        assert!(state.publish_noise(owner, [0.; 3]).is_err());
        state.sample(owner, 0).unwrap();
        assert_eq!(state.clock.unwrap().last_ms, 0);
        state.sample(owner, 12).unwrap();
        state.sample(owner, 28).unwrap();
        assert_eq!(state.frames, 3);
        let now = state.clock.unwrap().elapsed_seconds;
        assert_eq!(state.publish_noise(owner, [1., 2., 3.]).unwrap(), now);
        assert_eq!(
            (state.noise.owner, state.noise.position),
            (Some(owner), [1., 2., 3.])
        );
        assert!(state.sample(foreign, 29).is_err());
        assert!(state.publish_noise(foreign, [4.; 3]).is_err());
        assert!(state.publish_noise(owner, [f32::NAN; 3]).is_err());
        assert_eq!(state.noise.position, [1., 2., 3.]);
        let mut fresh = State::default();
        fresh.sample(foreign, 32).unwrap();
        assert_eq!(fresh.noise.owner, None);
        assert_eq!(fresh.noise.seconds, 0.);
        assert_eq!(fresh.frames, 1);
        fresh
            .areas
            .insert(
                giants_game::slow_areas::Birth {
                    owner: foreign,
                    center: [1., 2., 3.],
                    radius: 4.,
                    seconds: 0.02,
                    factor: 0.5,
                },
                Precision::Double53,
            )
            .unwrap();
        fresh.sample(foreign, 48).unwrap();
        assert_eq!(
            fresh
                .areas
                .snapshot(|id| if id == foreign {
                    Ok(77)
                } else {
                    Err("wrong owner".into())
                })
                .unwrap()[0]
                .source_owner,
            77
        );
        fresh.sample(foreign, 64).unwrap();
        assert!(fresh.areas.records().is_empty());
    }
}
