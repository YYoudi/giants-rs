//! Real ordinary partial-pistol contacts, shared blood pool and audio queue.
//! Prototype PS2 `ripper_shot` remains an unknown correspondence.
use super::*;
use giants_game::{audio_parameters::Random, state23_hurt};
pub(in crate::mission_runtime) struct Hit {
    pub ticket: u64,
    pub step: u64,
    pub target: Id,
    pub projectile: giants_game::mecc_pistol::WorldHit,
    pub routing: giants_game::hit_routing::Context,
    pub camera: [f32; 3],
}
struct Sounds<'a>(&'a mut Vec<crate::mission_audio::Event>);
impl state23_hurt::Sounds<Id> for Sounds<'_> {
    fn request(
        &mut self,
        request: state23_hurt::Sound<Id>,
    ) -> Result<state23_hurt::SoundResult, String> {
        use crate::mission_audio::Event;
        self.0.push(match request {
            state23_hurt::Sound::Stop { actor, sound } => Event::StopActorSound {
                owner: actor,
                sound,
            },
            state23_hurt::Sound::Play { actor, sound } => Event::ActorSound {
                owner: actor,
                sound,
                kind: giants_game::actor_sound::Kind::Owner,
            },
        });
        // Actual queue submission, never an audible-playback acknowledgment.
        Ok(state23_hurt::SoundResult::Submitted)
    }
}
impl State {
    pub(in crate::mission_runtime) fn death_snapshot(&self) -> Option<(Id, u32, u64)> {
        self.last_death
    }
    pub(in crate::mission_runtime) fn unsupported_lethal_snapshot(
        &self,
    ) -> Option<(Id, u32, u64, Option<u64>)> {
        self.last_unsupported_lethal.map(|(id, serial, step)| {
            let completed = (!self.actor_quarantined(id))
                .then(|| self.driver.completed(id).map(|frame| frame.0))
                .flatten();
            (id, serial, step, completed)
        })
    }
    pub(in crate::mission_runtime) fn audio_position(
        &self,
        session: &Session,
        id: Id,
    ) -> Option<[f32; 3]> {
        if self.driver.failed()
            || self.actor_quarantined(id)
            || !self.bound.as_ref()?.1.iter().any(|owner| owner.id == id)
        {
            return None;
        }
        self.driver.completed(id)?;
        let fields = session.get(id)?.construction_fields.as_ref()?;
        (fields.object_flags & 0x8000 == 0 && fields.position.into_iter().all(f32::is_finite))
            .then_some(fields.position)
    }
    pub(in crate::mission_runtime) fn pistol_hit_snapshot(&self) -> Option<(Id, u32, u64, bool)> {
        self.last_pistol_hit.map(|(id, serial, step)| {
            (
                id,
                serial,
                step,
                !self.actor_quarantined(id) && self.driver.completed(id).is_some(),
            )
        })
    }
    pub(in crate::mission_runtime) fn hit(
        &mut self,
        session: &mut Session,
        hit: Hit,
        shared_blood: Option<&mut giants_game::blood_particles::Pool>,
        random: &mut Random,
    ) -> Result<Vec<crate::mission_audio::Event>, String> {
        let Hit {
            ticket,
            step,
            target,
            projectile: hit,
            routing,
            camera,
        } = hit;
        self.view(session, ticket)?;
        if self.driver.failed()
            || self.actor_quarantined(target)
            || !self
                .bound
                .as_ref()
                .unwrap()
                .1
                .iter()
                .any(|o| o.id == target)
        {
            return Err("Ripper hit outside actual active component".into());
        }
        let completed = self
            .driver
            .completed(target)
            .ok_or("Ripper hit requires completed target frame")?
            .0;
        let source = hit.into_source()?;
        let shooter = source.owner();
        let serial = source.serial();
        let amount = source.damage();
        let before = session
            .state23_combat(target)
            .ok_or("Ripper combat state absent")?
            .damage;
        let callback = giants_game::state23_damage::Context {
            player_is_state1: false, // This entry verifies the actual controlled Mecc.
            drop_percent: *self
                .globals
                .words
                .get(0x1080 / 4)
                .ok_or("Ripper drop parameter absent")? as i32,
            death_timer: self.globals.scalar(0x10f4).map_err(|e| e.to_string())?,
        };
        let mut lethal_sounds = vec![];
        let mut lethal = death::Services {
            contexts: self
                .contexts
                .as_mut()
                .ok_or("Ripper context owner absent")?,
            factory: self.health_factory.as_ref(),
            image: &self.image,
            effects: &mut self.death_effects,
            sounds: &mut lethal_sounds,
            quality: self.globals.scalar(0x13e0).map_err(|e| e.to_string())?,
            lite_distance: self.globals.scalar(0x1570).map_err(|e| e.to_string())?,
            camera,
            random: *random,
        };
        let result = session.damage_ripper_from_partial_pistol_with_death(
            target,
            completed,
            step,
            source,
            random,
            &mut giants_game::resident_states::PartialPistolServices {
                // Keep the common pool/capacity where the Vimp owner exists;
                // this component's pool serves Ripper-only scenes.
                blood: shared_blood.unwrap_or(&mut self.blood),
                sounds: &mut Sounds(&mut self.sounds),
                routing,
                callback,
            },
            &mut lethal,
        );
        self.sounds.extend(lethal_sounds);
        match result {
            Ok(giants_game::resident_states::PartialPistolContact::Applied(outcome, counts)) => {
                self.last_pistol_hit = Some((target, serial, step));
                println!(
                    "mission_ripper_pistol_hit ticket={ticket} owner={target:?} shooter={shooter:?} serial={serial} amount={amount} before_damage={before} after_damage={} outcome={outcome:?} particles={} credit_writes={} source=actual_partial_pistol canonical_projectile=false playback_acknowledged=false",
                    session.state23_combat(target).unwrap().damage,
                    counts.particles,
                    counts.credit_writes
                );
                Ok(std::mem::take(&mut self.sounds))
            }
            Ok(giants_game::resident_states::PartialPistolContact::UnsupportedLethal) => {
                self.last_unsupported_lethal = Some((target, serial, step));
                println!(
                    "mission_ripper_pistol_unsupported_lethal ticket={ticket} owner={target:?} serial={serial} step={step} damage={before} amount={amount} damage_acknowledged=false death_acknowledged=false callback_started=false actor_quarantined=false"
                );
                Ok(std::mem::take(&mut self.sounds))
            }
            Ok(giants_game::resident_states::PartialPistolContact::UnavailableLethal(reason)) => {
                self.last_unsupported_lethal = Some((target, serial, step));
                println!(
                    "mission_ripper_pistol_lethal_unavailable ticket={ticket} owner={target:?} serial={serial} reason={reason} callback_started=false actor_quarantined=false"
                );
                Ok(std::mem::take(&mut self.sounds))
            }
            Ok(giants_game::resident_states::PartialPistolContact::Died) => {
                self.last_pistol_hit = Some((target, serial, step));
                self.last_death = Some((target, serial, step));
                self.driver.remove_completed(target);
                // The callback changed source flags after finalization. Refresh
                // the owned projection before another ordinary collision query.
                session.refresh_actor_environment_geometry(target)?;
                println!(
                    "mission_ripper_pistol_death ticket={ticket} owner={target:?} serial={serial} step={step} callback_completed=true retirement_completed=false"
                );
                Ok(std::mem::take(&mut self.sounds))
            }
            Ok(giants_game::resident_states::PartialPistolContact::Suppressed) => {
                println!(
                    "mission_ripper_pistol_suppressed ticket={ticket} owner={target:?} serial={serial} source=actual_range_or_target_guard damage_acknowledged=false"
                );
                Ok(std::mem::take(&mut self.sounds))
            }
            Err(error) => {
                self.driver.quarantine_actor(target, step, error.clone());
                eprintln!(
                    "mission_ripper_pistol_refused ticket={ticket} owner={target:?} serial={serial} error={error} damage_acknowledged=false affected_actor_quarantined=true other_actors_continue=true"
                );
                Err(error)
            }
        }
    }
    pub(in crate::mission_runtime) fn blood_quads(
        &self,
    ) -> Vec<giants_game::blood_particles::Quad> {
        self.blood.quads().collect()
    }
    pub(super) fn advance_blood(
        &mut self,
        frame: &live::Frame<'_>,
        random: &mut Random,
    ) -> Result<(), String> {
        let ground = giants_game::terrain_queries::StaticTerrainQuery::new(
            &frame.world.terrain,
            1. / frame.world.terrain.spacing,
            giants_game::arithmetic::Precision::Double53,
        )?;
        self.blood.update(
            random,
            |position| {
                Ok(giants_game::object_time::seconds(
                    frame.seconds,
                    None,
                    Some(position),
                    frame.areas,
                ) as f32)
            },
            |position| Ok(ground.sample([position[0], position[1]])?.height),
        )?;
        self.blood.purge();
        Ok(())
    }
}
