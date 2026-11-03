//! Concrete nonlethal callback services: real blood pool and source propagation.
//! Hit eligibility, actor ownership, collision snapshots and deaths are outside.
use crate::{audio_parameters::Random, blood_particles, projectile_retaliation, state23_damage};

#[derive(Clone, Copy, Debug)]
pub enum Sound<I> {
    Stop { actor: I, sound: u32 },
    Play { actor: I, sound: u32 },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundResult {
    /// Actual lookup/suppression found no descriptor; not audible playback.
    Absent,
    Submitted,
}
/// An unimplemented backend must return Err, not a fabricated Absent/Submitted.
pub trait Sounds<I> {
    fn request(&mut self, request: Sound<I>) -> Result<SoundResult, String>;
}
#[derive(Clone, Copy, Debug)]
pub struct Shape {
    pub scale: f32,
    pub center: f32,
    pub height: f32,
}
pub struct Services<'a, I, S> {
    pub blood: &'a mut blood_particles::Pool,
    /// Unique current views of the native contact-list members, including the
    /// victim only if it belongs to that list. Completeness remains supplied.
    pub actors: &'a mut [projectile_retaliation::Target<I>],
    pub shape: Shape,
    pub sounds: &'a mut S,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Counts {
    pub particles: usize,
    pub credit_writes: usize,
    pub sound_submissions: usize,
}
struct Running<'a, 'b, I, S> {
    services: &'a mut Services<'b, I, S>,
    counts: Counts,
    propagated: bool,
}
impl<I: Copy + Eq, S: Sounds<I>> state23_damage::Services<I> for Running<'_, '_, I, S> {
    fn execute(
        &mut self,
        actor: &state23_damage::State<I>,
        service: state23_damage::Service<I>,
        random: &mut Random,
    ) -> Result<(), String> {
        use state23_damage::Service;
        match service {
            Service::StopAttached { actor, sound } => {
                self.counts.sound_submissions += usize::from(
                    self.services.sounds.request(Sound::Stop { actor, sound })?
                        == SoundResult::Submitted,
                );
            }
            Service::PlayAttached { actor, sound } => {
                self.counts.sound_submissions += usize::from(
                    self.services.sounds.request(Sound::Play { actor, sound })?
                        == SoundResult::Submitted,
                );
            }
            Service::Blood { .. } => {
                let shape = self.services.shape;
                self.counts.particles += self.services.blood.emit(
                    blood_particles::Actor {
                        position: actor.position,
                        scale: shape.scale,
                        center: shape.center,
                        height: shape.height,
                    },
                    random,
                )?;
            }
            Service::PropagateSource { owner, .. } => {
                if let Some(view) = self.services.actors.iter_mut().find(|v| v.id == actor.id) {
                    // Action1 can have assigned this credit earlier in the same
                    // callback. Publish that change before scanning the list.
                    view.retaliation = actor.retaliation;
                }
                self.counts.credit_writes +=
                    projectile_retaliation::propagate(actor.position, owner, self.services.actors)
                        .map_err(str::to_owned)?;
                self.propagated = true;
            }
            _ => return Err("service outside nonlethal hurt profile".into()),
        }
        Ok(())
    }
}
/// Execute the acquired callback with concrete local services. This is not the
/// general hit wrapper. Death is refused before mutation. Once execution starts,
/// sound/RNG/pool/cohort changes are not rolled back on failure; never blindly
/// retry a partially executed callback.
pub fn apply<I: Copy + Eq, S: Sounds<I>>(
    actor: &mut state23_damage::State<I>,
    amount: f32,
    source: Option<state23_damage::Source<I>>,
    context: state23_damage::Context,
    random: &mut Random,
    services: &mut Services<'_, I, S>,
) -> Result<(state23_damage::Outcome, Counts), String> {
    if actor.damage < actor.limit
        && f64::from(actor.damage) + f64::from(amount) >= f64::from(actor.limit)
    {
        return Err("lethal hit requires destruction and mission services".into());
    }
    let shape = services.shape;
    if ![shape.scale, shape.center, shape.height]
        .into_iter()
        .all(f32::is_finite)
        || shape.scale <= 0.
        || shape.height < 0.
    {
        return Err("invalid current blood shape".into());
    }
    projectile_retaliation::validate(actor.position, services.actors).map_err(str::to_owned)?;
    if services.actors.iter().any(|v| {
        v.id == actor.id
            && (v.state != 23 || v.position != actor.position || v.retaliation != actor.retaliation)
    }) {
        return Err("hurt victim view differs from current callback state".into());
    }
    let mut running = Running {
        services,
        counts: Counts::default(),
        propagated: false,
    };
    let result = state23_damage::apply(actor, amount, source, context, random, &mut running);
    // Keep the local victim and its list view coherent even if the last sound
    // request failed after blood/credit had already been published.
    if let Some(view) = running
        .services
        .actors
        .iter_mut()
        .find(|v| v.id == actor.id)
    {
        if running.propagated {
            actor.retaliation = view.retaliation;
        } else {
            view.retaliation = actor.retaliation;
        }
    }
    result.map(|outcome| (outcome, running.counts))
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Backend {
        fail_play: bool,
        calls: usize,
    }
    impl Sounds<u32> for Backend {
        fn request(&mut self, command: Sound<u32>) -> Result<SoundResult, String> {
            self.calls += 1;
            if self.fail_play && matches!(command, Sound::Play { .. }) {
                Err("sink failed".into())
            } else {
                Ok(SoundResult::Absent)
            }
        }
    }
    fn actor() -> state23_damage::State<u32> {
        state23_damage::State {
            id: 1,
            class: 73,
            limit: 40.,
            damage: 0.,
            mode: 0,
            action: 0,
            flags: 0x20,
            personality: 0x20000,
            control: 0,
            timer: 0.,
            position: [0.; 3],
            parent: None,
            owner: None,
            retaliation: None,
            target: None,
            group: None,
            group_index: 0,
            effect: None,
        }
    }
    fn pool() -> blood_particles::Pool {
        blood_particles::Pool::new(blood_particles::Configuration {
            initial_delay: 0.1,
            fade_seconds: 1.,
            gravity: 1.,
            friction: 0.,
            size: [1., 1.],
            vertical_speed: [1., 1.],
            count: [1, 1],
            spread: 1.,
        })
        .unwrap()
    }
    const SHAPE: Shape = Shape {
        scale: 1.,
        center: 1.,
        height: 2.,
    };
    const CONTEXT: state23_damage::Context = state23_damage::Context {
        player_is_state1: false,
        drop_percent: 0,
        death_timer: 0.,
    };
    #[test]
    fn failure_after_blood_and_credit_preserves_partial_state_without_success() {
        let mut state = actor();
        let mut pool = pool();
        let mut views = [projectile_retaliation::Target {
            id: 1,
            state: 23,
            position: [0.; 3],
            retaliation: None,
        }];
        let mut random = Random(1);
        let mut backend = Backend {
            fail_play: true,
            calls: 0,
        };
        let result = apply(
            &mut state,
            12.,
            Some(state23_damage::Source {
                id: 2,
                state: 77,
                owner: Some(3),
            }),
            CONTEXT,
            &mut random,
            &mut Services {
                blood: &mut pool,
                actors: &mut views,
                shape: SHAPE,
                sounds: &mut backend,
            },
        );
        assert_eq!(result.unwrap_err(), "sink failed");
        assert_eq!(state.damage, 12.);
        assert_eq!(state.retaliation, Some(3));
        assert_eq!(views[0].retaliation, Some(3));
        assert_eq!(pool.active().count(), 1);
        assert_ne!(random.0, 1);
        assert_eq!(backend.calls, 6);
    }
    #[test]
    fn lethal_or_stale_input_is_refused_before_services() {
        let mut state = actor();
        state.damage = 32.;
        let mut pool = pool();
        let mut views = [projectile_retaliation::Target {
            id: 1,
            state: 23,
            position: [0.; 3],
            retaliation: None,
        }];
        let mut random = Random(1);
        let mut backend = Backend {
            fail_play: false,
            calls: 0,
        };
        let mut services = Services {
            blood: &mut pool,
            actors: &mut views,
            shape: SHAPE,
            sounds: &mut backend,
        };
        assert!(apply(&mut state, 12., None, CONTEXT, &mut random, &mut services).is_err());
        services.actors[0].position[0] = 1.;
        assert!(apply(&mut state, 1., None, CONTEXT, &mut random, &mut services).is_err());
        assert_eq!(state.damage, 32.);
        assert_eq!(pool.active().count(), 0);
        assert_eq!(random.0, 1);
        assert_eq!(backend.calls, 0);
    }
}
