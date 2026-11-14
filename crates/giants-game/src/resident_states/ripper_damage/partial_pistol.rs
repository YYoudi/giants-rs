//! Bounded class38 partial-pistol contact into the acquired state23 callback.
//! Prototype PS2 `ripper_shot` remains unknown. An actual emitted shot keeps its
//! separate identity: it is never registered as a canonical state77 actor.
use super::*;
use crate::{blood_particles, hit_routing, mecc_pistol, projectile_retaliation, state23_hurt};

pub struct Services<'a, S> {
    pub blood: &'a mut blood_particles::Pool,
    pub sounds: &'a mut S,
    pub routing: hit_routing::Context,
    pub callback: state23_damage::Context,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Reference {
    Actor(Id),
    PartialShot { owner: Id, serial: u32 },
}
impl Reference {
    fn actor(self) -> Result<Id, String> {
        match self {
            Self::Actor(id) => Ok(id),
            Self::PartialShot { .. } => {
                Err("partial shot cannot become a Session actor reference".into())
            }
        }
    }
}
fn map_state<I, J>(
    s: state23_damage::State<I>,
    mut f: impl FnMut(I) -> Result<J, String>,
) -> Result<state23_damage::State<J>, String> {
    Ok(state23_damage::State {
        id: f(s.id)?,
        class: s.class,
        limit: s.limit,
        damage: s.damage,
        mode: s.mode,
        action: s.action,
        flags: s.flags,
        personality: s.personality,
        control: s.control,
        timer: s.timer,
        position: s.position,
        parent: s.parent.map(&mut f).transpose()?,
        owner: s
            .owner
            .map(|(id, class)| f(id).map(|id| (id, class)))
            .transpose()?,
        retaliation: s.retaliation.map(&mut f).transpose()?,
        target: s
            .target
            .map(|t| {
                f(t.id).map(|id| state23_damage::Target {
                    id,
                    state: t.state,
                    flags: t.flags,
                })
            })
            .transpose()?,
        group: s.group.map(&mut f).transpose()?,
        group_index: s.group_index,
        effect: s.effect.map(&mut f).transpose()?,
    })
}
struct Sounds<'a, S>(&'a mut S);
impl<S: state23_hurt::Sounds<Id>> state23_hurt::Sounds<Reference> for Sounds<'_, S> {
    fn request(
        &mut self,
        command: state23_hurt::Sound<Reference>,
    ) -> Result<state23_hurt::SoundResult, String> {
        self.0.request(match command {
            state23_hurt::Sound::Stop { actor, sound } => state23_hurt::Sound::Stop {
                actor: actor.actor()?,
                sound,
            },
            state23_hurt::Sound::Play { actor, sound } => state23_hurt::Sound::Play {
                actor: actor.actor()?,
                sound,
            },
        })
    }
}
impl Session {
    /// Caller resolves the actual partial collision key. Validate the completed
    /// victim and current controlled shooter before consuming the immutable
    /// emitted contact. This is not native state77 construction or launch.
    pub fn damage_ripper_from_partial_pistol<S: state23_hurt::Sounds<Id>>(
        &mut self,
        target: Id,
        completed_step: u64,
        player_step: u64,
        hit: mecc_pistol::OwnedHit,
        random: &mut Random,
        services: &mut Services<'_, S>,
    ) -> Result<Option<(state23_damage::Outcome, state23_hurt::Counts)>, String> {
        self.ripper_contact_snapshot(target, completed_step)?;
        self.check_ripper_damage_available(target)?;
        let shooter = hit.owner();
        self.controlled_mecc_motion(shooter, player_step)?;
        if hit.class() != 38
            || hit.serial() == 0
            || !self.mecc_health(shooter)?.alive()
            || services.routing.mode == 4
            || services.callback.player_is_state1
            || !hit.damage().is_finite()
            || hit.damage() <= 0.
            || self
                .partial_pistol_contacts
                .contains(&(shooter, hit.serial()))
        {
            return Err(
                "Ripper hit requires an unconsumed actual normal class38 pistol source".into(),
            );
        }
        self.partial_pistol_contacts.insert((shooter, hit.serial()));
        let shooter_flags = self
            .get(shooter)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap()
            .configuration[14];
        if hit_routing::range_rejected(
            hit.flags(),
            shooter_flags,
            hit.range(),
            hit.camera_origin(),
            hit.point(),
        )? {
            return Ok(None);
        }
        let route = hit_routing::resolve(self.hit_actor(target)?, 0, services.routing, |id| {
            self.hit_actor(id)
        })?;
        if route == hit_routing::Route::Suppressed {
            return Ok(None);
        }
        if route != (hit_routing::Route::Dispatch { target, kind: 0 }) {
            return Err(
                "partial pistol Ripper hit requires its real unforwarded unsuppressed wrapper"
                    .into(),
            );
        }
        let initial = self.state23_snapshot(target)?;
        if initial.parent.is_some() {
            return Err("attached Ripper requires its actual detachment service".into());
        }
        let mut views = Vec::new();
        for id in self.list(1) {
            let actor = self.get(id).ok_or("Ripper contact-list owner absent")?;
            if actor.seed.storage.state() == 23 {
                self.check_ripper_damage_available(id)?;
                let state = self.state23_snapshot(id)?;
                views.push(projectile_retaliation::Target {
                    id: Reference::Actor(id),
                    state: 23,
                    position: state.position,
                    retaliation: state.retaliation.map(Reference::Actor),
                });
            }
        }
        let fields = self
            .get(target)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap();
        let shape = state23_hurt::Shape {
            scale: fields.scale,
            center: f32::from_bits(fields.configuration[22]),
            height: f32::from_bits(fields.configuration[23]),
        };
        let mut state = map_state(initial, |id| Ok(Reference::Actor(id)))?;
        let attempt = Arc::new(());
        self.residents
            .get_mut(&target.serial)
            .unwrap()
            .ripper_damage
            .active = Some(attempt.clone());
        // Verified PC callback projection of the actual class38 shot's source
        // kind and owner only. The source identity remains PartialShot, never Id.
        let result = state23_hurt::apply(
            &mut state,
            hit.damage(),
            Some(state23_damage::Source {
                id: Reference::PartialShot {
                    owner: shooter,
                    serial: hit.serial(),
                },
                state: 77,
                owner: Some(Reference::Actor(shooter)),
            }),
            services.callback,
            random,
            &mut state23_hurt::Services {
                blood: services.blood,
                actors: &mut views,
                shape,
                sounds: &mut Sounds(services.sounds),
            },
        );
        // As in the canonical basic-shot adapter, keep preceding queue/pool,
        // victim and cohort writes even on a late service failure; never retry.
        let publication = (|| -> Result<(), String> {
            let state = map_state(state, Reference::actor)?;
            self.publish_ripper_damage_stage(target, &attempt, &state)?;
            for view in views {
                let id = view.id.actor()?;
                let credit = view.retaliation.map(Reference::actor).transpose()?;
                self.residents
                    .get_mut(&id.serial)
                    .unwrap()
                    .state23_combat
                    .as_mut()
                    .ok_or("Ripper credit state absent")?
                    .retaliation = credit;
            }
            Ok(())
        })();
        let result = match (result, publication) {
            (Err(error), _) | (_, Err(error)) => Err(error),
            (Ok(value), Ok(())) => Ok(Some(value)),
        };
        let operation = &mut self
            .residents
            .get_mut(&target.serial)
            .unwrap()
            .ripper_damage;
        operation.active = None;
        operation.failed = result.as_ref().err().cloned();
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Queue(Vec<u32>);
    impl state23_hurt::Sounds<Reference> for Queue {
        fn request(
            &mut self,
            request: state23_hurt::Sound<Reference>,
        ) -> Result<state23_hurt::SoundResult, String> {
            if let state23_hurt::Sound::Play { sound, .. } = request {
                self.0.push(sound);
            }
            Ok(state23_hurt::SoundResult::Absent)
        }
    }
    #[test]
    #[ignore = "Requires private GIANTS_RIPPER_PISTOL_CALLBACK_CONTROLS; original callback with supplied source77/cohort, absent sound registry and zero-count blood; no runtime readiness"]
    fn native_partial_pistol_callback_controls() {
        let text = std::fs::read_to_string(
            std::env::var_os("GIANTS_RIPPER_PISTOL_CALLBACK_CONTROLS").unwrap(),
        )
        .unwrap();
        let mut count = 0;
        for line in text.lines().filter(|l| !l.is_empty()) {
            let w: Vec<_> = line
                .split_whitespace()
                .map(|s| u32::from_str_radix(s, 16).unwrap())
                .collect();
            assert_eq!(w.len(), 12);
            let actor = |serial| Id {
                ticket: 719,
                serial,
            };
            let victim = Reference::Actor(actor(1));
            let owner = actor(3);
            let initial = if w[4] != 0 {
                None
            } else {
                Some(Reference::Actor(actor(4)))
            };
            let mut state = state23_damage::State {
                id: victim,
                class: w[0] as i32,
                limit: if w[0] == 73 { 40. } else { 90. },
                damage: 0.,
                mode: w[2],
                action: w[1],
                flags: 0,
                personality: 0,
                control: 0,
                timer: 0.,
                position: [0.; 3],
                parent: None,
                owner: None,
                retaliation: initial,
                target: None,
                group: None,
                group_index: -1,
                effect: None,
            };
            let mut views = [
                projectile_retaliation::Target {
                    id: victim,
                    state: 23,
                    position: [0.; 3],
                    retaliation: initial,
                },
                projectile_retaliation::Target {
                    id: Reference::Actor(actor(2)),
                    state: 23,
                    position: [10., 0., 0.],
                    retaliation: None,
                },
            ];
            let mut pool = blood_particles::Pool::new(blood_particles::Configuration {
                initial_delay: 0.,
                fade_seconds: 1.,
                gravity: 0.,
                friction: 0.,
                size: [1., 1.],
                vertical_speed: [0., 0.],
                count: [0, 0],
                spread: 0.,
            })
            .unwrap();
            let mut queue = Queue(vec![]);
            let mut random = Random(w[3]);
            let (_, counts) = state23_hurt::apply(
                &mut state,
                5.,
                Some(state23_damage::Source {
                    id: Reference::PartialShot { owner, serial: 1 },
                    state: 77,
                    owner: Some(Reference::Actor(owner)),
                }),
                state23_damage::Context {
                    player_is_state1: false,
                    drop_percent: 0,
                    death_timer: 4.,
                },
                &mut random,
                &mut state23_hurt::Services {
                    blood: &mut pool,
                    actors: &mut views,
                    shape: state23_hurt::Shape {
                        scale: 1.,
                        center: 0.,
                        height: 1.,
                    },
                    sounds: &mut queue,
                },
            )
            .unwrap();
            let tag = |id| match id {
                None => 0,
                Some(Reference::Actor(id)) if id == owner => 1,
                _ => 2,
            };
            assert_eq!(
                [
                    state.damage.to_bits(),
                    state.action,
                    state.mode,
                    tag(state.retaliation),
                    tag(views[1].retaliation),
                    random.0
                ],
                w[5..11],
                "case {count}"
            );
            assert_eq!(queue.0, if w[11] == 0 { vec![] } else { vec![w[11]] });
            assert_eq!(counts.particles, 0);
            assert!(pool.active().next().is_none());
            assert!(Reference::PartialShot { owner, serial: 1 }.actor().is_err());
            count += 1;
        }
        assert_eq!(count, 16);
    }
}
