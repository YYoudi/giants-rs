//! Ordinary declared Ripper frames in the partial Mecc scene. This component
//! admits only acquired continuations and never acknowledges another AI family.
use super::*;
use giants_game::{
    animation_epoch::Stamp,
    attached_effect_context::{Creation, Pool},
    audio_parameters::Random,
    enemy_projectile::SoundOutcome,
    resident_states::ActorWorldServices,
    ripper_primary_turn::{self as primary, FrameInput},
};
use std::collections::BTreeMap;

#[derive(Default)]
pub(super) struct Driver {
    sample: Option<u64>,
    stamp: Stamp,
    completed: BTreeMap<Id, (u64, u32)>,
    failure: Option<String>,
}
impl Driver {
    pub(super) fn completed(&self, id: Id) -> Option<(u64, u32)> {
        self.completed.get(&id).copied()
    }
    pub(super) fn failed(&self) -> bool {
        self.failure.is_some()
    }
    pub(in crate::mission_runtime) fn refuse_draw(&mut self, ticket: u64, error: String) {
        if self.failure.is_none() {
            eprintln!("mission_ripper_draw_refused ticket={ticket} error={error}");
        }
        self.failure = Some(error);
        self.completed.clear();
    }
}
pub(in crate::mission_runtime) struct Frame<'a> {
    pub ticket: u64,
    pub sample: u64,
    pub step: u64,
    pub controlled: Id,
    pub camera: [f32; 3],
    pub seconds: f32,
    pub areas: &'a [giants_game::object_time::SlowArea],
    pub objects: &'a [giants_game::actor_environment::Collider],
    pub world: &'a giants_game::WorldAsset,
    pub resources: &'a giants_game::scene_resources::SceneResources,
}
struct Callbacks<'a, 'f> {
    frame: &'a Frame<'f>,
    globals: &'a giants_formats::global_data::GlobalData,
    random: &'a mut Random,
    contexts: &'a mut Pool<Id>,
    stamp: &'a mut Stamp,
    sounds: &'a mut Vec<crate::mission_audio::Event>,
    identity: u32,
    epoch: Option<u32>,
}
struct Services<'a> {
    owner: Id,
    contexts: &'a mut Pool<Id>,
    sounds: &'a mut Vec<crate::mission_audio::Event>,
}
impl Services<'_> {
    fn play(&mut self, cue: u32) -> SoundOutcome {
        self.sounds.push(crate::mission_audio::Event::ActorSound {
            owner: self.owner,
            sound: cue,
            kind: giants_game::actor_sound::Kind::Owner,
        });
        // A transport submission is not AudioSink/voice availability.
        SoundOutcome::Submitted
    }
}
impl primary::Services<Id, giants_game::attached_effect_context::Id> for Services<'_> {
    fn prepare_awakening_dirt(
        &mut self,
        state: &primary::State<Id, giants_game::attached_effect_context::Id>,
        random: &mut Random,
    ) -> Result<Option<giants_game::attached_effect_context::Id>, String> {
        let (creation, _) = self.contexts.prepare_extended_invariant(
            "fx_ripper_dirt",
            self.owner,
            state.entry.flags,
            u32::MAX,
            random,
        )?;
        Ok(match creation {
            Creation::Absent => None,
            Creation::Prepared(id) => Some(id),
        })
    }
    fn remove_effect(
        &mut self,
        _: &primary::State<Id, giants_game::attached_effect_context::Id>,
        effect: giants_game::attached_effect_context::Id,
        _: &mut Random,
    ) -> Result<(), String> {
        self.contexts.release_prepared(effect).map(|_| ())
    }
    fn owner_sound(
        &mut self,
        _: &primary::State<Id, giants_game::attached_effect_context::Id>,
        cue: u32,
        _: &mut Random,
    ) -> Result<SoundOutcome, String> {
        Ok(self.play(cue))
    }
    fn stop_owner_sound(
        &mut self,
        _: &primary::State<Id, giants_game::attached_effect_context::Id>,
        cue: u32,
        _: &mut Random,
    ) -> Result<SoundOutcome, String> {
        self.sounds
            .push(crate::mission_audio::Event::StopActorSound {
                owner: self.owner,
                sound: cue,
            });
        Ok(SoundOutcome::Submitted)
    }
}
impl giants_game::resident_states::State23AnimationServices for Services<'_> {
    fn fire(
        &mut self,
        _: &mut Session,
        _: Id,
        _: &giants_game::state23_animation::State,
        _: [f32; 3],
        _: f32,
        _: &mut Random,
    ) -> Result<giants_game::state23_animation::FireOutcome, String> {
        Err("Ripper secondary firing continuation is not admitted".into())
    }
}
impl ActorWorldServices for Callbacks<'_, '_> {
    fn controlled_actor(&self) -> Option<Id> {
        Some(self.frame.controlled)
    }
    fn primary(&mut self, session: &mut Session, id: Id) -> Result<(), String> {
        let commands = session.actor_motion(id)?.commands();
        if !session.begin_actor_motion_history(id, commands)? {
            return Err("Ripper primary idle gate has no acquired motion continuation".into());
        }
        if session.ripper_primary(id).is_err() {
            session.prepare_ripper_primary(id)?;
        }
        let owned = session.controlled_mecc_motion(self.frame.controlled, self.frame.step)?;
        let player = session
            .get(self.frame.controlled)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap();
        let context = giants_game::state23_entry::Context {
            paused: false,
            native_mode: 0,
            parent_state: None,
            player: Some(giants_game::state23_entry::Player {
                state: 2,
                control: player.control_flags,
                position: player.position,
                motion: [owned.frame.model_rates[1], owned.frame.model_rates[2]],
            }),
            delta: f64::from(self.frame.seconds),
        };
        // Explicit current owned player subset. Other staged NPCs have no live
        // callbacks and must not become fabricated ready perception providers.
        let subset = session.perception_subset(&[self.frame.controlled])?;
        let perception = subset.frame()?;
        let class = session.get(id).unwrap().class;
        let mode = session
            .state23_combat(id)
            .map_or(session.ripper_primary(id)?.state().entry.mode, |combat| {
                combat.mode
            });
        let scalar = |offset| self.globals.scalar(offset).map_err(|e| e.to_string());
        let input = FrameInput {
            context,
            frame: &perception,
            range: scalar(if class == 73 { 0xe0 } else { 0xfc })?,
        };
        let mut services = Services {
            owner: id,
            contexts: self.contexts,
            sounds: self.sounds,
        };
        let terrain = giants_game::terrain_queries::StaticTerrainQuery::new(
            &self.frame.world.terrain,
            1. / self.frame.world.terrain.spacing,
            giants_game::arithmetic::Precision::Double53,
        )?;
        match (class, mode) {
            (73, 0 | 31) => session.update_ripper_awakening_on_static_terrain(
                id,
                primary::AwakeningFrame {
                    context,
                    frame: &perception,
                    range: input.range,
                    parameters: primary::AwakeningContext {
                        camera: self.frame.camera,
                        parameters: giants_game::ripper_awakening::Parameters::from_globals(
                            self.globals,
                        )
                        .map_err(|e| e.to_string())?,
                        buried_speed_percent: scalar(0x288)?,
                    },
                },
                &terrain,
                self.random,
                &mut services,
            )?,
            (73, 32) => session.update_ripper_turn(
                id,
                input,
                giants_game::ripper_turn_completion::Parameters::from_globals(self.globals)
                    .map_err(|e| e.to_string())?,
                self.random,
                &mut services,
            )?,
            (73, 33) => session.update_ripper_pursuit(
                id,
                input,
                scalar(0xe4)?,
                self.random,
                &mut services,
            )?,
            (73, 34) => session.update_ripper_emergence_on_static_terrain(
                id,
                input,
                scalar(0x288)?,
                &terrain,
                self.random,
                &mut services,
            )?,
            (230, 0 | 61) => session.update_ripper_variant_awakening(
                id,
                input,
                primary::VariantAwakeningContext {
                    camera: self.frame.camera,
                    camera_range: scalar(0x144)?,
                    global_seconds: self.frame.seconds,
                    areas: self.frame.areas,
                },
                &mut giants_game::primary_surface_marker::Factory::new(self.frame.resources),
                self.random,
                &mut services,
            )?,
            (230, 62) => {
                session.update_ripper_variant_turn(id, input, self.random, &mut services)?
            }
            (230, 63 | 64) => {
                let parameters = primary::VariantDecisionContext {
                    close_range: scalar(0x3d8)?,
                    fire_range: scalar(0x100)?,
                };
                if mode == 63 {
                    session.update_ripper_variant_decision(
                        id,
                        input,
                        parameters,
                        self.random,
                        &mut services,
                    )?
                } else {
                    session.update_ripper_variant_pursuit(
                        id,
                        input,
                        parameters,
                        self.random,
                        &mut services,
                    )?
                }
            }
            _ => {
                return Err(format!(
                    "Ripper class{class} mode{mode} ordinary continuation not acquired"
                ));
            }
        };
        session.advance_ripper_common_motion(
            id,
            giants_game::resident_motion::Time {
                seconds: self.frame.seconds,
                areas: self.frame.areas,
                // Same partial-scene heading policy as the ordinary Vimp driver.
                control_factor: 0.5,
                skip_heading: false,
                controlled_actor: false,
                // Read only for state2 running; this driver admits state23 only.
                running_factor: 1.,
            },
        )
    }
    fn secondary(&mut self, session: &mut Session, id: Id) -> Result<(), String> {
        let combat = session.state23_combat(id).ok_or("Ripper combat absent")?;
        let class = session.get(id).unwrap().class;
        if !matches!((class, combat.mode), (73, 31..=34 | 44) | (230, 61..=65)) {
            return Err("Ripper secondary requires an acquired ordinary selection".into());
        }
        let scalar = |offset| self.globals.scalar(offset).map_err(|e| e.to_string());
        session.update_state23_animation(
            id,
            giants_game::state23_animation::Context {
                parent_state: None,
                target_position: None,
                // Session rejects the mode20 leap before these unused arguments.
                leap_reference_speed: 0.,
                leap_model_speed: 0.,
                vertical_spread_percent: scalar(if class == 230 { 0x11c } else { 0xf0 })?,
                cooldown: [scalar(0xd0c)?, scalar(0xd10)?],
                repeat_timer: scalar(0xd14)?,
                blending: giants_game::animation_channels::Blending::Disabled,
            },
            self.random,
            &mut Services {
                owner: id,
                contexts: self.contexts,
                sounds: self.sounds,
            },
        )
    }
    fn object_time(&mut self, session: &Session, id: Id) -> Result<f32, String> {
        let fields = session
            .get(id)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap();
        Ok(giants_game::object_time::seconds(
            self.frame.seconds,
            Some(giants_game::object_time::Sample {
                owner: fields.dispatch_context,
                position: fields.position,
            }),
            None,
            self.frame.areas,
        ) as f32)
    }
    fn animation(&mut self, session: &mut Session, id: Id, seconds: f32) -> Result<(), String> {
        self.epoch = session.advance_ripper_animation(
            id,
            seconds,
            self.stamp,
            self.contexts,
            self.random,
        )?;
        Ok(())
    }
    fn finalization(&mut self, session: &mut Session, id: Id) -> Result<(), String> {
        let fields = session
            .get(id)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap();
        let context = fields.dispatch_context;
        let query = giants_game::terrain_queries::StaticTerrainQuery::new(
            &self.frame.world.terrain,
            1. / self.frame.world.terrain.spacing,
            giants_game::arithmetic::Precision::Double53,
        )?;
        let mut environment = giants_game::actor_environment::FinalizationServices {
            actor: self.identity,
            state: 23,
            slope_parameter: f32::from_bits(fields.configuration[11]),
            physics: None,
            skip_object_responses: false,
            objects: self.frame.objects,
            ground: |point| {
                giants_game::physical_ground::Ground::from_sample(
                    query.sample(point)?,
                    self.frame.world.terrain.width,
                )
            },
            platform: |point| {
                giants_game::platform_queries::sample_prepared(
                    point,
                    &[],
                    giants_game::arithmetic::Precision::Double53,
                )
            },
            seconds: |position| {
                giants_game::object_time::seconds(
                    self.frame.seconds,
                    Some(giants_game::object_time::Sample {
                        owner: context,
                        position,
                    }),
                    None,
                    self.frame.areas,
                )
            },
            release: |footprint: Option<u32>| {
                if footprint.is_none() {
                    Ok(())
                } else {
                    Err("Ripper footprint absent from partial scene registry".into())
                }
            },
            statistics: None,
        };
        session.finalize_actor(
            id,
            giants_game::actor_finalization::Time {
                global_seconds: self.frame.seconds,
                context,
                areas: self.frame.areas,
            },
            &mut environment,
        )
    }
}
impl State {
    pub(in crate::mission_runtime) fn refuse_draw(&mut self, ticket: u64, error: String) {
        self.driver.refuse_draw(ticket, error);
    }
    pub(in crate::mission_runtime) fn take_sounds(&mut self) -> Vec<crate::mission_audio::Event> {
        std::mem::take(&mut self.sounds)
    }
    pub(in crate::mission_runtime) fn advance_live(
        &mut self,
        session: &mut Session,
        frame: Frame<'_>,
        random: &mut Random,
    ) -> Result<Vec<crate::mission_audio::Event>, String> {
        self.view(session, frame.ticket)?;
        self.preparation
            .as_ref()
            .ok_or("Ripper cold resources absent")?
            .as_ref()
            .map_err(Clone::clone)?;
        if let Some(error) = &self.driver.failure {
            return Err(error.clone());
        }
        if self.driver.sample == Some(frame.sample) {
            return Ok(vec![]);
        }
        session.controlled_mecc_motion(frame.controlled, frame.step)?;
        let mut sounds = self.take_sounds();
        let result = self.advance_inner(session, &frame, random, &mut sounds);
        self.sounds = sounds;
        if let Err(error) = &result {
            self.driver.failure = Some(error.clone());
            self.driver.completed.clear();
        }
        result.map(|()| self.take_sounds())
    }
    fn advance_inner(
        &mut self,
        session: &mut Session,
        frame: &Frame<'_>,
        random: &mut Random,
        sounds: &mut Vec<crate::mission_audio::Event>,
    ) -> Result<(), String> {
        let owners = session
            .list(0)
            .into_iter()
            .filter_map(|id| {
                self.bound
                    .as_ref()
                    .unwrap()
                    .1
                    .iter()
                    .find(|owner| owner.id == id)
                    .copied()
            })
            .collect::<Vec<_>>();
        if owners.len() != self.expected.len() {
            return Err("Ripper common-list owner absent".into());
        }
        // All owned position histories precede this component's callbacks.
        for owner in &owners {
            session.begin_actor_frame_history(owner.id)?;
        }
        for owner in owners {
            let identity = u32::try_from(owner.declaration.key.index)
                .map_err(|_| "Ripper identity overflow")?
                .checked_add(2)
                .ok_or("Ripper identity overflow")?
                | if owner.declaration.key.scope == Scope::World {
                    0x4000_0000
                } else {
                    0
                };
            let mut callbacks = Callbacks {
                frame,
                globals: &self.globals,
                random,
                contexts: self.contexts.as_mut().ok_or("Ripper contexts absent")?,
                stamp: &mut self.driver.stamp,
                sounds,
                identity,
                epoch: None,
            };
            let outcome = session.update_actor_world(owner.id, &mut callbacks)?;
            if outcome != giants_game::actor_world_tick::Outcome::Updated {
                return Err(format!(
                    "Ripper frame did not complete ordinary finalization: {outcome:?}"
                ));
            }
            session.refresh_actor_environment_geometry(owner.id)?;
            if let Some(epoch) = callbacks.epoch {
                let number = self.driver.completed.get(&owner.id).map_or(1, |v| v.0 + 1);
                self.driver.completed.insert(owner.id, (number, epoch));
            }
        }
        if self.driver.sample.is_none() {
            println!(
                "mission_ripper_component ticket={} completed_actors={} source=normal_mecc_component current_owned_pose=true perception=controlled_player_subset active_fx=false combat_complete=false native_world_verified=false",
                frame.ticket,
                self.driver.completed.len()
            );
        }
        self.driver.sample = Some(frame.sample);
        Ok(())
    }
}
