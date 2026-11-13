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
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub(super) struct Driver {
    sample: Option<u64>,
    pub(super) stamp: Stamp,
    completed: BTreeMap<Id, (u64, u32)>,
    failure: Option<String>,
    markers: BTreeSet<Id>,
    pub(super) projectiles: BTreeSet<Id>,
    pub(super) projectile_failures: BTreeMap<Id, String>,
    pub(super) first_quarantine_step: Option<u64>,
    pub(super) actor_failures: BTreeMap<Id, String>,
    held_markers: BTreeSet<Id>,
    pub(super) flights: BTreeMap<Id, u64>,
}
impl Driver {
    pub(super) fn completed(&self, id: Id) -> Option<(u64, u32)> {
        self.completed.get(&id).copied()
    }
    pub(super) fn failed(&self) -> bool {
        self.failure.is_some()
    }
    pub(super) fn markers(&self) -> impl Iterator<Item = Id> + '_ {
        self.markers.iter().copied()
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
    marker_factory: &'a mut giants_game::primary_surface_marker::Factory<'static>,
    projectile_factory: &'a mut giants_game::primary_enemy_fire::Factory<'static>,
    combat: &'a mut dyn super::combat::Services,
}
struct Services<'a> {
    owner: Id,
    selection: giants_game::effect_initialization::Selection,
    contexts: &'a mut Pool<Id>,
    sounds: &'a mut Vec<crate::mission_audio::Event>,
}
struct Melee<'a, 's> {
    primary: &'a mut Services<'s>,
    combat: &'a mut dyn super::combat::Services,
}
type PrimaryState = primary::State<Id, giants_game::attached_effect_context::Id>;
impl primary::Services<Id, giants_game::attached_effect_context::Id> for Melee<'_, '_> {
    fn owner_sound(
        &mut self,
        state: &PrimaryState,
        cue: u32,
        random: &mut Random,
    ) -> Result<SoundOutcome, String> {
        primary::Services::owner_sound(self.primary, state, cue, random)
    }
    fn stop_owner_sound(
        &mut self,
        state: &PrimaryState,
        cue: u32,
        random: &mut Random,
    ) -> Result<SoundOutcome, String> {
        primary::Services::stop_owner_sound(self.primary, state, cue, random)
    }
    fn remove_effect(
        &mut self,
        state: &PrimaryState,
        effect: giants_game::attached_effect_context::Id,
        random: &mut Random,
    ) -> Result<(), String> {
        primary::Services::remove_effect(self.primary, state, effect, random)
    }
}
impl giants_game::resident_states::MeccEnemyHitServices for Melee<'_, '_> {
    fn gates(
        &mut self,
        session: &Session,
        target: Id,
    ) -> Result<giants_game::resident_states::MeccEnemyHitGates, String> {
        self.combat.gates(session, target)
    }
    fn recoil(
        &mut self,
        session: &Session,
        response: giants_game::controlled_mecc::HitResponse,
    ) -> Result<(), String> {
        self.combat.recoil(session, response)
    }
    fn voice(&mut self, session: &Session, target: Id, cue: u32) -> Result<(), String> {
        self.combat.voice(session, target, cue)
    }
}
impl giants_game::resident_states::RipperMeleeServices for Melee<'_, '_> {
    fn world_suppressed(&mut self, session: &Session) -> Result<bool, String> {
        self.combat.world_suppressed(session)
    }
    fn terminal(&mut self, session: &mut Session, source: Id, target: Id) -> Result<(), String> {
        if session.ripper_melee_terminal_no_effect(source, target)? {
            Ok(())
        } else {
            Err("Ripper melee requires its actual mission/credit terminal consumer".into())
        }
    }
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
        let (creation, _) = self.contexts.prepare_extended(
            "fx_ripper_dirt",
            self.owner,
            state.entry.flags,
            u32::MAX,
            giants_game::effect_initialization::Selection {
                recipient_position: state.entry.position,
                ..self.selection
            },
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
impl giants_game::primary_enemy_fire::Dependencies<giants_game::attached_effect_context::Id>
    for Services<'_>
{
    fn prepare_effect(
        &mut self,
        session: &mut Session,
        request: giants_game::enemy_projectile::EffectRequest<'_>,
        random: &mut Random,
    ) -> Result<giants_game::enemy_projectile::EffectOutcome, String> {
        if request.flags != 4 {
            return Err("Ripper launch FX differs from acquired attached flags4".into());
        }
        let fields = session
            .get(request.recipient)
            .and_then(|actor| actor.construction_fields.as_ref())
            .ok_or("Ripper projectile FX recipient absent")?;
        if !matches!(fields.class, 90 | 232) {
            return Err("Ripper projectile FX recipient class differs".into());
        }
        let (created, _) = self.contexts.prepare_extended(
            request.name,
            request.recipient,
            fields.object_flags,
            request.tag,
            giants_game::effect_initialization::Selection {
                active_objects: u32::try_from(session.counts().residents)
                    .map_err(|_| "Ripper FX count overflow")?,
                recipient_position: fields.position,
                ..self.selection
            },
            random,
        )?;
        Ok(match created {
            Creation::Absent => giants_game::enemy_projectile::EffectOutcome::Rejected,
            Creation::Prepared(_) => giants_game::enemy_projectile::EffectOutcome::Prepared,
        })
    }
    fn positional_sound(
        &mut self,
        _: &mut Session,
        sound: u32,
        position: [f32; 3],
    ) -> Result<SoundOutcome, String> {
        if !position.into_iter().all(f32::is_finite) {
            return Err("Ripper launch sound position nonfinite".into());
        }
        self.sounds
            .push(crate::mission_audio::Event::NpcPosition { sound, position });
        Ok(SoundOutcome::Submitted)
    }
    fn owner_sound(
        &mut self,
        _: &mut Session,
        owner: Id,
        _: &primary::State<Id, giants_game::attached_effect_context::Id>,
        cue: u32,
        _: &mut Random,
    ) -> Result<SoundOutcome, String> {
        if owner != self.owner {
            return Err("Ripper attack sound owner differs".into());
        }
        Ok(self.play(cue))
    }
    fn remove_effect(
        &mut self,
        _: &mut Session,
        owner: Id,
        effect: giants_game::attached_effect_context::Id,
        _: &mut Random,
    ) -> Result<(), String> {
        if owner != self.owner {
            return Err("Ripper attack effect owner differs".into());
        }
        self.contexts.release_prepared(effect).map(|_| ())
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
impl giants_game::primary_surface_marker::Dependencies<giants_game::attached_effect_context::Id>
    for Services<'_>
{
    fn surface_popup(
        &mut self,
        session: &mut Session,
        owner: Id,
        state: &primary::State<Id, giants_game::attached_effect_context::Id>,
        random: &mut Random,
    ) -> Result<giants_game::enemy_projectile::EffectOutcome, String> {
        if owner != self.owner || session.get(owner).is_none() {
            return Err("surface FX owner differs from callback".into());
        }
        let (created, _) = self.contexts.prepare_positional_invariant(
            "fx_ripper_popup",
            state.entry.position,
            random,
        )?;
        Ok(match created {
            Creation::Absent => giants_game::enemy_projectile::EffectOutcome::Rejected,
            Creation::Prepared(_) => giants_game::enemy_projectile::EffectOutcome::Prepared,
        })
    }
    fn remove_effect(
        &mut self,
        _: &mut Session,
        owner: Id,
        effect: giants_game::attached_effect_context::Id,
        _: &mut Random,
    ) -> Result<(), String> {
        if owner != self.owner {
            return Err("surface retirement owner differs from callback".into());
        }
        self.contexts.release_prepared(effect).map(|_| ())
    }
    fn owner_sound(
        &mut self,
        _: &mut Session,
        owner: Id,
        _: &primary::State<Id, giants_game::attached_effect_context::Id>,
        cue: u32,
        _: &mut Random,
    ) -> Result<SoundOutcome, String> {
        if owner != self.owner {
            return Err("surface sound owner differs from callback".into());
        }
        Ok(self.play(cue))
    }
}
impl Callbacks<'_, '_> {
    fn effect_selection(
        &self,
        session: &Session,
        id: Id,
    ) -> Result<giants_game::effect_initialization::Selection, String> {
        let fields = session
            .get(id)
            .and_then(|actor| actor.construction_fields.as_ref())
            .ok_or("Ripper effect recipient fields absent")?;
        Ok(giants_game::effect_initialization::Selection {
            active_objects: u32::try_from(session.counts().residents)
                .map_err(|_| "Ripper FX count overflow")?,
            quality: self.globals.scalar(0x13e0).map_err(|e| e.to_string())?,
            lite_distance: self.globals.scalar(0x1570).map_err(|e| e.to_string())?,
            camera: self.frame.camera,
            recipient_position: fields.position,
        })
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
        let selection = self.effect_selection(session, id)?;
        let mut services = Services {
            owner: id,
            selection,
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
            (73, 43) => session.update_ripper_retreat(
                id,
                input,
                scalar(0x288)?,
                self.random,
                &mut services,
            )?,
            (73, 44) => session.update_ripper_surfacing(
                id,
                input,
                giants_game::ripper_turn_completion::Parameters::from_globals(self.globals)
                    .map_err(|e| e.to_string())?,
                self.random,
                self.marker_factory,
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
                self.marker_factory,
                self.random,
                &mut services,
            )?,
            (73, 37..=39) => session.update_ripper_attack(
                id,
                input,
                giants_game::ripper_attack::Parameters::from_globals(self.globals)
                    .map_err(|e| e.to_string())?,
                self.projectile_factory,
                self.random,
                &mut services,
            )?,
            (230, 65) => session.update_ripper_variant_attack(
                id,
                input,
                giants_game::ripper_attack::VariantParameters {
                    close_range: scalar(0x3d8)?,
                    reconsider_range: scalar(0x104)?,
                    charge_percent: scalar(0x108)?,
                    cooldown: scalar(0x10c)?,
                    vertical_spread_percent: scalar(0x11c)?,
                },
                self.projectile_factory,
                self.random,
                &mut services,
            )?,
            (230, 66) => session.update_ripper_variant_cooldown(
                id,
                input,
                primary::VariantCooldownContext {
                    close_range: scalar(0x3d8)?,
                    retreat_timer: [scalar(0x110)?, scalar(0x114)?],
                    global_seconds: self.frame.seconds,
                    areas: self.frame.areas,
                },
                self.random,
                &mut services,
            )?,
            (230, 71) => {
                session.update_ripper_variant_retreat(id, input, self.random, &mut services)?
            }
            (230, 67) => session.update_ripper_variant_close(
                id,
                input,
                primary::VariantCloseContext {
                    attack_range: scalar(0x118)?,
                    alternate_threshold: scalar(0x128)?,
                },
                self.random,
                &mut services,
            )?,
            (230, 68) => session.update_ripper_variant_melee_with_mecc(
                id,
                input,
                primary::VariantMeleeContext {
                    attack_range: scalar(0x118)?,
                    damage: scalar(0x3dc)?,
                },
                self.frame.step,
                self.random,
                &mut Melee {
                    primary: &mut services,
                    combat: self.combat,
                },
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
        let selection = self.effect_selection(session, id)?;
        let combat = session.state23_combat(id).ok_or("Ripper combat absent")?;
        let class = session.get(id).unwrap().class;
        if !matches!((class, combat.mode), (73, 31..=34 | 37..=39 | 43..=44) | (230, 61..=69 | 71))
        {
            return Err(format!(
                "Ripper class{class} mode{} secondary requires an acquired ordinary selection",
                combat.mode
            ));
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
                selection,
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
    pub(in crate::mission_runtime) fn quarantine_step(&self) -> Option<u64> {
        self.driver.first_quarantine_step
    }
    pub(in crate::mission_runtime) fn actor_quarantined(&self, id: Id) -> bool {
        self.driver.actor_failures.contains_key(&id)
    }

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
        device: &mut impl super::combat::Services,
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
        let result = self.advance_inner(session, &frame, random, &mut sounds, device);
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
        device: &mut impl super::combat::Services,
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
            if !self.driver.actor_failures.contains_key(&owner.id) {
                session.begin_actor_frame_history(owner.id)?;
            }
        }
        for id in self.driver.markers.iter().copied() {
            if session.actor_world_flags(id)? & 0xc000 == 0 {
                let number = session.surface_marker_frame(id)?.next_frame;
                session.begin_surface_marker_history(id, number)?;
            }
        }
        for owner in owners {
            if self.driver.actor_failures.contains_key(&owner.id) {
                continue;
            }
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
                marker_factory: &mut self.marker_factory,
                projectile_factory: &mut self.projectile_factory,
                combat: device,
            };
            let outcome = match session.update_actor_world(owner.id, &mut callbacks) {
                Ok(outcome) => outcome,
                Err(error) => {
                    eprintln!(
                        "mission_ripper_actor_quarantined ticket={} step={} owner={:?} class={} error={error} frame_acknowledged=false independent_ai_continues=true",
                        frame.ticket, frame.step, owner.id, owner.declaration.class
                    );
                    self.driver.actor_failures.insert(owner.id, error);
                    self.driver.first_quarantine_step.get_or_insert(frame.step);
                    self.driver.completed.remove(&owner.id);
                    continue;
                }
            };
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
            if frame.step.is_multiple_of(240)
                && let Some(fields) = session
                    .get(owner.id)
                    .and_then(|actor| actor.construction_fields.as_ref())
            {
                println!(
                    "mission_ripper_motion ticket={} step={} owner={:?} class={} position={:?} mode={:?} completed_frame=true forced_pose=false",
                    frame.ticket,
                    frame.step,
                    owner.id,
                    owner.declaration.class,
                    fields.position,
                    session.state23_combat(owner.id).map(|state| state.mode)
                );
            }
        }
        self.advance_markers(session, frame, random, device)?;
        self.advance_projectile_flight(session, frame, random, sounds, device)?;
        self.advance_impact_effects(session, frame, random)?;
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
    fn advance_markers(
        &mut self,
        session: &mut Session,
        frame: &Frame<'_>,
        random: &mut Random,
        device: &mut impl giants_game::actor_retirement::Services,
    ) -> Result<(), String> {
        let owned: BTreeSet<_> = self
            .bound
            .as_ref()
            .unwrap()
            .1
            .iter()
            .map(|o| o.id)
            .collect();
        // Retain actual callback children; class alone never establishes ownership.
        let markers = session.list(0).into_iter().filter(|&id| matches!(
            session.surface_link(id), Some(giants_game::resident_states::SurfaceLink::Marker { owner, .. }) if owned.contains(&owner)
        )).collect::<Vec<_>>();
        let terrain = giants_game::terrain_queries::StaticTerrainQuery::new(
            &frame.world.terrain,
            1. / frame.world.terrain.spacing,
            giants_game::arithmetic::Precision::Double53,
        )?;
        let mut marked = Vec::new();
        for id in markers {
            if matches!(session.surface_link(id), Some(giants_game::resident_states::SurfaceLink::Marker { owner, .. }) if self.driver.actor_failures.contains_key(&owner))
            {
                self.driver.markers.remove(&id);
                self.driver.completed.remove(&id);
                if self.driver.held_markers.insert(id) {
                    eprintln!(
                        "mission_ripper_marker_held ticket={} owner={id:?} reason=quarantined_parent retained_until_mission_retirement=true cleanup_acknowledged=false",
                        frame.ticket
                    );
                }
                continue;
            }
            if self.driver.markers.insert(id) {
                if !self
                    .marker_factory
                    .captured_effect_image()?
                    .same_image(&self.image)
                {
                    return Err(
                        "surface event namespace differs from the owning context pool".into(),
                    );
                }
                session.prepare_surface_marker_world(id)?;
            }
            if session.actor_world_flags(id)? & 0xc000 != 0 {
                if session.actor_world_flags(id)? & 0x8000 != 0 {
                    marked.push(id);
                }
                continue;
            }
            let fields = session
                .get(id)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap();
            let context = fields.dispatch_context;
            let mut finalization = giants_game::actor_environment::FinalizationServices {
                actor: session.actor_collision_identity(id)?,
                state: 48,
                slope_parameter: f32::from_bits(fields.configuration[11]),
                physics: None,
                skip_object_responses: false,
                objects: frame.objects,
                ground: |point| {
                    giants_game::physical_ground::Ground::from_sample(
                        terrain.sample(point)?,
                        frame.world.terrain.width,
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
                        frame.seconds,
                        Some(giants_game::object_time::Sample {
                            owner: context,
                            position,
                        }),
                        None,
                        frame.areas,
                    )
                },
                release: |footprint: Option<u32>| {
                    if footprint.is_none() {
                        Ok(())
                    } else {
                        Err("surface footprint absent from partial scene registry".into())
                    }
                },
                statistics: None,
            };
            let number = session.surface_marker_frame(id)?.next_frame;
            let outcome = session.advance_surface_marker_world(
                id,
                giants_game::resident_states::SurfaceMarkerFrame {
                    number,
                    global_seconds: frame.seconds,
                    areas: frame.areas,
                    controlled: frame.controlled,
                    delay: self.globals.scalar(0x3d4).map_err(|e| e.to_string())?,
                    blending: giants_game::animation_channels::Blending::Disabled,
                },
                &mut self.driver.stamp,
                self.contexts.as_mut().unwrap(),
                random,
                &mut finalization,
            )?;
            match outcome {
                giants_game::actor_world_tick::Outcome::Updated => {
                    if let Some(epoch) = session.surface_marker_frame(id)?.animation_epoch {
                        self.driver.completed.insert(id, (number + 1, epoch));
                    }
                }
                giants_game::actor_world_tick::Outcome::Inactive
                    if session.actor_world_flags(id)? & 0x8000 != 0 =>
                {
                    self.driver.completed.remove(&id);
                    marked.push(id);
                }
                _ => return Err(format!("surface marker frame incomplete: {outcome:?}")),
            }
        }
        if !marked.is_empty() {
            struct Retirement<'a, D> {
                contexts: &'a mut giants_game::attached_effect_context::Pool<Id>,
                device: &'a mut D,
            }
            impl<D: giants_game::actor_retirement::Services> giants_game::actor_retirement::Services
                for Retirement<'_, D>
            {
                fn release(&mut self, session: &mut Session, id: Id) -> Result<(), String> {
                    self.contexts.release_recipient_prepared(id)?;
                    self.device.release(session, id)
                }
            }
            let counts = session.cleanup_surface_markers(
                &marked,
                &mut Retirement {
                    contexts: self
                        .contexts
                        .as_mut()
                        .ok_or("Ripper contexts absent at retirement")?,
                    device,
                },
            )?;
            for id in marked {
                self.driver.markers.remove(&id);
                self.driver.completed.remove(&id);
            }
            println!(
                "mission_ripper_markers_retired ticket={} residents={} registrations={} source=owned_completed_state48 source_death=false",
                frame.ticket, counts.residents, counts.registrations
            );
        }
        Ok(())
    }
}
