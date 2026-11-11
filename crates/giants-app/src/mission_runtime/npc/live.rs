//! Portable first-interaction component in the existing partial Mecc mission.
//! It owns authored herds/markers and their children, never acknowledges the
//! other actor families as updated or certifies native whole-world admission.
use super::*;
use giants_game::{animation_epoch::Stamp, audio_parameters::Random, vimp_wander::Tick};
use std::collections::BTreeSet;
#[derive(Default)]
pub(super) struct Driver {
    sample: u64,
    number: u64,
    children: BTreeSet<Id>,
    stamp: Stamp,
    failure: Option<String>,
    entered: u64,
    returned: u64,
}
pub(in crate::mission_runtime) struct Frame<'a> {
    pub ticket: u64,
    pub sample: u64,
    pub controlled: Id,
    pub world: &'a giants_game::WorldAsset,
    pub objects: &'a [giants_game::actor_environment::Collider],
    pub tick: Tick<'a>,
}
struct SinglePlayer;
impl giants_game::vimp_living::Services for SinglePlayer {
    fn session(
        &mut self,
        _: &Session,
        _: Id,
        _: giants_game::vimp_response::Predicate,
    ) -> Result<giants_game::actor_session_predicates::Snapshot<'_>, String> {
        // This component is admitted only by the ordinary single-player Mecc
        // route. Multiplayer is not implemented; no network slot table is made.
        Ok(giants_game::actor_session_predicates::Snapshot {
            mode: 0,
            declared_count: 0,
            current_key: 0,
            slots: &[],
        })
    }
    fn response(
        &mut self,
        _: &mut Session,
        _: Id,
        _: Id,
        _: giants_game::resident_states::VimpResponseAction,
    ) -> Result<(), String> {
        Err("Vimp response notification requires the network action owner".into())
    }
    fn active(
        &mut self,
        _: &mut Session,
        _: Id,
        _: giants_game::vimp_active_response::Action,
    ) -> Result<(), String> {
        Err("Vimp active notification requires the network action owner".into())
    }
}
struct Floor<'a>(giants_game::terrain_queries::StaticTerrainQuery<'a>);
impl giants_game::herd_generator::Ground for Floor<'_> {
    fn height(&mut self, p: [f32; 3]) -> Result<f32, String> {
        Ok(self.0.sample([p[0], p[1]])?.height)
    }
}
struct Sounds(Vec<(u32, [f32; 3])>);
impl giants_game::positional_effect_audio::Service for Sounds {
    fn request(&mut self, sound: u32, position: [f32; 3], _: &mut Random) -> Result<(), String> {
        if !position.into_iter().all(f32::is_finite) {
            return Err("NPC positional sound is nonfinite".into());
        }
        // Same queued transport as ordinary player cues; the AudioSink service
        // alone acknowledges playback. Queueing never counts as an audible voice.
        self.0.push((sound, position));
        Ok(())
    }
}
impl State {
    pub(in crate::mission_runtime) fn live_hint(
        &self,
        session: &Session,
        position: [f32; 3],
        yaw: f32,
    ) -> Option<String> {
        if self.driver.failure.is_some() {
            return Some("Vimp interaction unavailable this entry. Other AI inactive.".into());
        }
        let nearest = self
            .driver
            .children
            .iter()
            .filter_map(|&id| session.get(id)?.construction_fields.as_ref())
            .min_by(|a, b| {
                let distance = |p: [f32; 3]| (p[0] - position[0]).hypot(p[1] - position[1]);
                distance(a.position).total_cmp(&distance(b.position))
            })?;
        let dx = nearest.position[0] - position[0];
        let dy = nearest.position[1] - position[1];
        let turn = (dx.atan2(-dy).to_degrees() - yaw + 180.).rem_euclid(360.) - 180.;
        let direction = if turn.abs() < 10. {
            "ahead".into()
        } else {
            format!(
                "{} {:.0} deg",
                if turn > 0. { "right" } else { "left" },
                turn.abs()
            )
        };
        Some(format!(
            "Vimps: {:.0} units, {direction}. Repeated shots nearby startle them. Other AI inactive.",
            dx.hypot(dy)
        ))
    }
    #[cfg(test)]
    pub(super) fn live_stats(&self) -> (u64, u64, u64) {
        (
            self.driver.number,
            self.driver.entered,
            self.driver.returned,
        )
    }

    pub(in crate::mission_runtime) fn advance_live(
        &mut self,
        session: &mut Session,
        frame: Frame<'_>,
        random: &mut Random,
    ) -> Result<Vec<(u32, [f32; 3])>, String> {
        self.view(session, frame.ticket)?;
        if !frame.world.name.eq_ignore_ascii_case("story2") {
            return Ok(vec![]);
        }
        if self.driver.sample == frame.sample {
            return Ok(vec![]);
        }
        if let Some(error) = &self.driver.failure {
            return Err(error.clone());
        }
        let result = self.advance_live_inner(session, frame, random);
        if let Err(error) = &result {
            self.driver.failure = Some(error.clone());
        }
        result
    }
    fn advance_live_inner(
        &mut self,
        session: &mut Session,
        frame: Frame<'_>,
        random: &mut Random,
    ) -> Result<Vec<(u32, [f32; 3])>, String> {
        let bound = self.bound.as_mut().unwrap();
        if frame.controlled.ticket() != frame.ticket || session.get(frame.controlled).is_none() {
            return Err("NPC controlled owner absent or foreign".into());
        }
        let owned: BTreeSet<_> = bound.declarations.iter().map(|(_, id, _)| *id).collect();
        let population = session.list(0);
        // This is an explicit component population, not a successful native
        // common-list traversal with skipped callbacks for unsupported families.
        for id in population.iter().copied() {
            if self.driver.children.contains(&id) {
                let number = session.vimp_frames(id).ok_or("NPC child replaced")?;
                session.prepare_vimp_world_history(id, number, &self.factory)?;
            } else if owned.contains(&id) {
                let number = session.herd_world_frame(id)?.next_frame;
                session.prepare_herd_world_history(id, number)?;
            }
        }
        let mut floor = Floor(giants_game::terrain_queries::StaticTerrainQuery::new(
            &frame.world.terrain,
            1. / frame.world.terrain.spacing,
            giants_game::arithmetic::Precision::Double53,
        )?);
        let mut visited = BTreeSet::new();
        loop {
            let next = session.list(0).into_iter().find(|id| {
                !visited.contains(id) && (owned.contains(id) || self.driver.children.contains(id))
            });
            let Some(id) = next else { break };
            if visited.len() >= 4096 {
                return Err("NPC component exceeded bounded actor budget".into());
            }
            visited.insert(id);
            if owned.contains(&id) {
                let number = session.herd_world_frame(id)?.next_frame;
                let (_, _, class) = bound
                    .declarations
                    .iter()
                    .find(|(_, owner, _)| *owner == id)
                    .ok_or("NPC declaration owner absent")?;
                let key = bound
                    .declarations
                    .iter()
                    .find(|(_, owner, _)| *owner == id)
                    .unwrap()
                    .0;
                let identity = u32::try_from(key.index)
                    .map_err(|_| "NPC declaration identity overflow")?
                    .checked_add(2)
                    .ok_or("NPC declaration identity overflow")?
                    | if key.scope == giants_game::declared_residents::Scope::World {
                        0x40000000
                    } else {
                        0
                    };
                let f = session
                    .get(id)
                    .unwrap()
                    .construction_fields
                    .as_ref()
                    .unwrap();
                let context = f.dispatch_context;
                let slope_parameter = f32::from_bits(f.configuration[11]);
                let query = giants_game::terrain_queries::StaticTerrainQuery::new(
                    &frame.world.terrain,
                    1. / frame.world.terrain.spacing,
                    giants_game::arithmetic::Precision::Double53,
                )?;
                let mut environment = giants_game::actor_environment::FinalizationServices {
                    actor: identity,
                    state: if *class == 49 { 17 } else { 0 },
                    slope_parameter,
                    physics: None,
                    skip_object_responses: false,
                    objects: frame.objects,
                    ground: |point| {
                        giants_game::physical_ground::Ground::from_sample(
                            query.sample(point)?,
                            frame.world.terrain.width,
                        )
                    },
                    // Declared rigid preparation explicitly excludes platform
                    // footprints; this is the same partial scene as the Mecc.
                    platform: |point| {
                        giants_game::platform_queries::sample_prepared(
                            point,
                            &[],
                            giants_game::arithmetic::Precision::Double53,
                        )
                    },
                    seconds: |position| {
                        giants_game::object_time::seconds(
                            frame.tick.global_delta,
                            Some(giants_game::object_time::Sample {
                                owner: context,
                                position,
                            }),
                            None,
                            frame.tick.slow_areas,
                        )
                    },
                    release: |id: Option<u32>| {
                        if id.is_none() {
                            Ok(())
                        } else {
                            Err("NPC footprint owner not in the partial scene registry".into())
                        }
                    },
                    statistics: None,
                };
                let p = session.advance_herd_world_after_history(
                    id,
                    giants_game::herd_frame::Frame {
                        number,
                        global_seconds: frame.tick.global_delta,
                        areas: frame.tick.slow_areas,
                        controlled: Some(frame.controlled),
                    },
                    &mut giants_game::herd_frame::Services {
                        factory: &self.factory,
                        globals: &self.globals,
                        random,
                        ground: &mut floor,
                        stamp: &mut self.driver.stamp,
                        finalization: &mut environment,
                    },
                )?;
                for child in p.born {
                    if !self.factory.owns(session, child) || !self.driver.children.insert(child) {
                        return Err("NPC birth owner missing, replaced or repeated".into());
                    }
                }
            } else {
                let number = session
                    .vimp_frames(id)
                    .ok_or("NPC child frame owner absent")?;
                let before = session.vimp(id).unwrap().mode;
                let mut ground = |point| {
                    giants_game::physical_ground::Ground::from_sample(
                        floor.0.sample(point)?,
                        frame.world.terrain.width,
                    )
                };
                let mut services = giants_game::vimp_frame::Services {
                    animation: giants_game::vimp_animation::Services {
                        blending: giants_game::animation_channels::Blending::Disabled,
                        contexts: &mut bound.contexts,
                        random,
                        stamp: &mut self.driver.stamp,
                    },
                    ground: &mut ground,
                    recovery_sounds: None,
                };
                let actor_frame = giants_game::vimp_frame::Frame {
                    number,
                    tick: frame.tick,
                    controlled_actor: Some(frame.controlled),
                };
                if number == 0 {
                    session.advance_vimp_living_newborn(
                        id,
                        actor_frame,
                        &self.factory,
                        &self.globals,
                        &mut services,
                        &mut SinglePlayer,
                    )?;
                } else {
                    session.advance_vimp_living_after_history(
                        id,
                        actor_frame,
                        &self.factory,
                        &self.globals,
                        &mut services,
                        &mut SinglePlayer,
                    )?;
                }
                let after = session.vimp(id).unwrap().mode;
                self.driver.entered += u64::from(before == 1 && after == 7);
                self.driver.returned += u64::from(before == 7 && after == 1);
                if before != after {
                    println!(
                        "mission_vimp_transition ticket={} owner={id:?} frame={number} before={before} after={after} source=normal_mecc_component native_world_verified=false",
                        frame.ticket
                    );
                }
            }
        }
        let mut sounds = Sounds(vec![]);
        let retired = bound
            .contexts
            .advance_positional_sounds(random, &mut sounds)?;
        self.driver.sample = frame.sample;
        self.driver.number += 1;
        if self.driver.number == 1 || self.driver.number.is_multiple_of(600) {
            println!(
                "mission_vimp_component ticket={} frames={} vimps={} consumed_callbacks={} sound_contexts_retired={retired} entered={} returned={} unsupported_other_ai_inactive=true native_world_verified=false",
                frame.ticket,
                self.driver.number,
                self.driver.children.len(),
                visited.len(),
                self.driver.entered,
                self.driver.returned
            );
        }
        Ok(sounds.0)
    }
}
