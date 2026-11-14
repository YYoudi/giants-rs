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
        let result = session.damage_ripper_from_partial_pistol(
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
        );
        match result {
            Ok(Some((outcome, counts))) => {
                self.last_pistol_hit = Some((target, serial, step));
                println!(
                    "mission_ripper_pistol_hit ticket={ticket} owner={target:?} shooter={shooter:?} serial={serial} amount={amount} before_damage={before} after_damage={} outcome={outcome:?} particles={} credit_writes={} source=actual_partial_pistol canonical_projectile=false playback_acknowledged=false",
                    session.state23_combat(target).unwrap().damage,
                    counts.particles,
                    counts.credit_writes
                );
                Ok(std::mem::take(&mut self.sounds))
            }
            Ok(None) => {
                println!(
                    "mission_ripper_pistol_suppressed ticket={ticket} owner={target:?} serial={serial} source=actual_range_or_target_guard damage_acknowledged=false"
                );
                Ok(std::mem::take(&mut self.sounds))
            }
            Err(error) => {
                self.driver.quarantine_actor(target, step, error.clone());
                eprintln!(
                    "mission_ripper_pistol_refused ticket={ticket} owner={target:?} serial={serial} error={error} damage_acknowledged=false independent_ai_continues=true"
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
