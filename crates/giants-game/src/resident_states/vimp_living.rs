//! Primary-only invocation identity. The public isolated helpers keep their
//! original refusal rules; this scope cannot be constructed by a runtime caller.
use super::*;

pub(super) struct Scope {
    actor: Id,
    frame: u64,
    parent: Id,
    context: u32,
    uid: u32,
    pub(super) threat: u64,
    pub(super) response: u64,
    pub(super) vector: u64,
    pub(super) active: u64,
}
impl Scope {
    pub(super) fn enter(session: &Session, id: Id) -> Result<Self, String> {
        let a = session.get(id).ok_or("living primary owner absent")?;
        let v = a.vimp.ok_or("living primary source construction absent")?;
        if !a.world_tick_running
            || a.vimp_frame_stopped
            || v.wander_pending
            || a.vimp_prepared_history.is_some()
            || a.vimp_threat_failure.is_some()
            || a.vimp_vector_running
            || a.vimp_response_running
            || a.vimp_active_running
        {
            return Err("living scope requires the actual unconsumed common primary".into());
        }
        let born = v.born.ok_or("living primary requires actual herd birth")?;
        let scope = Self {
            actor: id,
            frame: a.vimp_frames,
            parent: born.parent,
            context: v.context,
            uid: born.uid,
            threat: a.vimp_threat_steps,
            response: a.vimp_response_steps,
            vector: a.vimp_vector_steps,
            active: a.vimp_active_steps,
        };
        scope.check(session, id)?;
        Ok(scope)
    }
    pub(super) fn check(&self, session: &Session, id: Id) -> Result<(), String> {
        let a = session
            .get(id)
            .ok_or("living primary owner retired or foreign")?;
        let v = a.vimp.ok_or("living primary source owner replaced")?;
        if id != self.actor
            || !a.world_tick_running
            || a.vimp_frames != self.frame
            || a.vimp_frame_stopped
            || a.vimp_prepared_history.is_some()
            || a.class != 8
            || a.seed.storage.profile() != Profile::Debug
            || a.seed.storage.state() != 21
            || !a.acquired_construction
            || !v.initialized
            || !matches!(v.mode, 1 | 7)
            || v.context != self.context
            || v.born
                .is_none_or(|b| b.parent != self.parent || b.uid != self.uid)
            || a.attachment.is_some()
            || a.presentation_parent.is_some()
        {
            return Err("living primary invocation identity or phase changed".into());
        }
        session.herd_vimp_inputs(self.parent)?;
        Ok(())
    }
}
struct Backend<'a, 't> {
    scope: &'a Scope,
    tick: crate::vimp_wander::Tick<'t>,
    external: &'a mut dyn crate::vimp_living::Services,
}
impl VimpResponseServices for Backend<'_, '_> {
    fn session(
        &mut self,
        s: &Session,
        id: Id,
        q: crate::vimp_response::Predicate,
    ) -> Result<crate::actor_session_predicates::Snapshot<'_>, String> {
        self.scope.check(s, id)?;
        self.external.session(s, id, q)
    }
    fn action(
        &mut self,
        s: &mut Session,
        id: Id,
        parent: Id,
        a: VimpResponseAction,
    ) -> Result<(), String> {
        self.scope.check(s, id)?;
        self.external.response(s, id, parent, a)
    }
}
impl VimpVectorServices for Backend<'_, '_> {
    fn session(
        &mut self,
        s: &Session,
        id: Id,
        q: crate::vimp_response::Predicate,
    ) -> Result<crate::actor_session_predicates::Snapshot<'_>, String> {
        VimpResponseServices::session(self, s, id, q)
    }
    fn action(
        &mut self,
        s: &mut Session,
        id: Id,
        parent: Id,
        a: crate::vimp_vector_response::Action,
    ) -> Result<(), String> {
        VimpResponseServices::action(self, s, id, parent, VimpResponseAction::Vector(a))
    }
}
impl VimpActiveServices for Backend<'_, '_> {
    fn session(
        &mut self,
        s: &Session,
        id: Id,
        q: crate::vimp_response::Predicate,
    ) -> Result<crate::actor_session_predicates::Snapshot<'_>, String> {
        VimpResponseServices::session(self, s, id, q)
    }
    fn seconds(&mut self, s: &Session, id: Id) -> Result<f64, String> {
        self.actor_seconds(s, id)
    }
    fn orient(&mut self, s: &mut Session, id: Id) -> Result<(), String> {
        self.scope.check(s, id)?;
        let a = s.residents.get_mut(&id.serial).unwrap();
        let v = a.vimp.as_mut().unwrap();
        if v.commands[1] != 0 {
            return Err("living look producer outside acquired zero-look profile".into());
        }
        crate::vimp_recovery::orient(a.construction_fields.as_mut().unwrap(), v, self.tick)
    }
    fn action(
        &mut self,
        s: &mut Session,
        id: Id,
        a: crate::vimp_active_response::Action,
    ) -> Result<(), String> {
        self.scope.check(s, id)?;
        self.external.active(s, id, a)
    }
}
impl VimpActiveMovementServices for Backend<'_, '_> {
    fn movement_seconds(
        &mut self,
        s: &Session,
        id: Id,
        clock: crate::vimp_movement::Clock,
    ) -> Result<f64, String> {
        match clock {
            crate::vimp_movement::Clock::Global => Ok(f64::from(self.tick.global_delta)),
            crate::vimp_movement::Clock::Actor => self.actor_seconds(s, id),
        }
    }
}
impl Backend<'_, '_> {
    fn actor_seconds(&self, s: &Session, id: Id) -> Result<f64, String> {
        self.scope.check(s, id)?;
        let a = s.get(id).unwrap();
        let v = a.vimp.unwrap();
        Ok(crate::object_time::seconds(
            self.tick.global_delta,
            Some(crate::object_time::Sample {
                owner: v.context,
                position: a.construction_fields.as_ref().unwrap().position,
            }),
            None,
            self.tick.slow_areas,
        ))
    }
}
impl Session {
    pub(crate) fn begin_vimp_living_primary(
        &mut self,
        id: Id,
        tick: crate::vimp_wander::Tick<'_>,
        factory: &crate::vimp_actor::Factory,
        globals: &giants_formats::global_data::GlobalData,
        random: &mut crate::audio_parameters::Random,
        external: &mut dyn crate::vimp_living::Services,
    ) -> Result<(), String> {
        use crate::vimp_wander::{put, scalar, squared, word};
        let scope = Scope::enter(self, id)?;
        let mut backend = Backend {
            scope: &scope,
            tick,
            external,
        };
        factory.growth_bounds(self, id)?;
        let a = self.get(id).unwrap();
        let v = a.vimp.unwrap();
        let f = a.construction_fields.as_ref().unwrap();
        let b = a.seed.storage.data().unwrap();
        if b.len() != 216
            || word(b, 0) & 0x40 != 0
            || !v.damage.is_finite()
            || f.object_flags & 0x81008000 != 0
            || word(b, 0xa8) != 0
            || globals.scalar(0x91c).map_err(|e| e.to_string())? > 0.
        {
            return Err(
                "living primary requires acquired dry finite mode1/7 prefix without feeding countdown"
                    .into(),
            );
        }
        if !tick.global_delta.is_finite()
            || tick.global_delta < 0.
            || !tick.control_factor.is_finite()
        {
            return Err("living primary clock/control outside acquired finite profile".into());
        }
        let initial_mode = v.mode;
        if initial_mode == 7 && word(b, 0x78) != 0 {
            return Err("living linked target requires its current typed owner".into());
        }
        // The source heals before its mode dispatch, using the wide owner
        // clock. Hurt modes retain their existing separate no-healing path.
        let damage = crate::vimp_living_scalars::regenerate(
            initial_mode,
            v.damage,
            backend.actor_seconds(self, id)?,
            globals.scalar(0x964).map_err(|e| e.to_string())?,
        )?;
        self.residents
            .get_mut(&id.serial)
            .unwrap()
            .vimp
            .as_mut()
            .unwrap()
            .damage = damage;
        let seconds = f64::from(backend.actor_seconds(self, id)? as f32);
        let b = self.get(id).unwrap().seed.storage.data().unwrap();
        let mut produce_commands = false;
        if initial_mode == 1 {
            let remaining = f64::from(scalar(b, 0xac)) - seconds;
            let a = self.residents.get_mut(&id.serial).unwrap();
            let b = a.seed.storage.data_mut().unwrap();
            put(b, 0xac, (remaining as f32).to_bits());
            put(
                b,
                0xc0,
                globals.scalar(0x9bc).map_err(|e| e.to_string())?.to_bits(),
            );
            if remaining <= 0. && word(b, 0xb0) == 0 {
                a.construction_fields.as_mut().unwrap().configuration[14] &= !0x400;
            }
            let considered = self.consider_vimp_threats_in_scope(
                id,
                scope.threat,
                crate::vimp_threat::Signals {
                    now: tick.now,
                    noise_at: tick.noise_time,
                    noise_position: tick.noise_position,
                },
                globals,
                Some(&scope),
            )?;
            let handled = if considered
                || self
                    .get(id)
                    .unwrap()
                    .construction_fields
                    .as_ref()
                    .unwrap()
                    .control_flags
                    != 0
            {
                self.respond_vimp_in_scope(
                    id,
                    VimpResponseRequest {
                        number: scope.response,
                        noise_position: tick.noise_position,
                    },
                    globals,
                    random,
                    &mut backend,
                    Some(&scope),
                )?
            } else {
                false
            };
            if handled {
                self.residents
                    .get_mut(&id.serial)
                    .unwrap()
                    .construction_fields
                    .as_mut()
                    .unwrap()
                    .configuration[14] |= 0x400;
            } else {
                put(
                    self.residents
                        .get_mut(&id.serial)
                        .unwrap()
                        .seed
                        .storage
                        .data_mut()
                        .unwrap(),
                    0x58,
                    0,
                );
                let number = self.get(id).unwrap().vimp.unwrap().growth_steps;
                self.advance_vimp_growth_offline(
                    id,
                    crate::vimp_actor::GrowthTick {
                        number,
                        global_delta: tick.global_delta,
                        slow_areas: tick.slow_areas,
                    },
                    factory,
                    globals,
                    random,
                )?;
                let a = self.residents.get_mut(&id.serial).unwrap();
                let b = a.seed.storage.data_mut().unwrap();
                let alert = f64::from(scalar(b, 0x94))
                    - f64::from(globals.scalar(0x96c).map_err(|e| e.to_string())?) * seconds;
                put(
                    b,
                    0x94,
                    (if alert < 0. { 0. } else { alert as f32 }).to_bits(),
                );
                let position = a.construction_fields.as_ref().unwrap().position;
                let active = word(b, 0xb0) != 0;
                let index = word(b, 0x98) as usize;
                let target = [scalar(b, 0x40), scalar(b, 0x44), scalar(b, 0x48)];
                let marker_id = self
                    .herd_markers(scope.parent)
                    .ok_or("living route absent")?
                    .get(index)
                    .copied()
                    .flatten()
                    .ok_or("living marker absent")?;
                let marker = self
                    .herd_marker(marker_id)
                    .ok_or("living marker metadata absent")?;
                let marker_position = self
                    .get(marker_id)
                    .unwrap()
                    .construction_fields
                    .as_ref()
                    .unwrap()
                    .position;
                let radius = f64::from(f32::from_bits(marker.values[1]));
                let arrived = if active {
                    squared(position, marker_position) <= radius * radius
                } else {
                    squared(position, target) <= 100.
                };
                if arrived {
                    // Feeding's mode/audio transition remains an explicit producer gap.
                    if !active && f32::from_bits(marker.values[2]) != 0. {
                        return Err(
                            "living arrival feeding transition requires its audio/mode producer"
                                .into(),
                        );
                    }
                    let b = self
                        .residents
                        .get_mut(&id.serial)
                        .unwrap()
                        .seed
                        .storage
                        .data_mut()
                        .unwrap();
                    if word(b, 0xa0) == 0 {
                        put(b, 0x9c, word(b, 0x9c).wrapping_add(1));
                    }
                    put(b, 0xa0, 0);
                    self.retarget_vimp_in_scope(id, &scope, random, &mut backend)?;
                }
            }
            produce_commands = !handled;
        } else {
            let number = self.get(id).unwrap().vimp.unwrap().growth_steps;
            self.advance_vimp_growth_offline(
                id,
                crate::vimp_actor::GrowthTick {
                    number,
                    global_delta: tick.global_delta,
                    slow_areas: tick.slow_areas,
                },
                factory,
                globals,
                random,
            )?;
        }
        self.advance_vimp_movement_response_in_scope(
            id,
            scope.active,
            VimpActiveMovement {
                produce_commands,
                contact: VimpMovementContact::Owned,
            },
            globals,
            random,
            &mut backend,
            Some(&scope),
        )?;
        scope.check(self, id)?;
        let a = self.residents.get_mut(&id.serial).unwrap();
        let v = a.vimp.as_mut().unwrap();
        if v.commands[1] != 0 {
            return Err("living look continuation requires its producer".into());
        }
        v.wander_pending = true;
        let position = a.construction_fields.as_ref().unwrap().position;
        if let Some(p) = a.vimp_physical.as_mut() {
            p.rearm(&tick, position, v.context);
        } else {
            a.vimp_physical = Some(crate::vimp_physical::Owned::new(&tick, position, v.context));
        }
        Ok(())
    }
}
impl Session {
    pub(crate) fn advance_vimp_living_animation(
        &mut self,
        id: Id,
        factory: &crate::vimp_actor::Factory,
        globals: &giants_formats::global_data::GlobalData,
        services: crate::vimp_animation::Services<'_>,
        external: &mut dyn crate::vimp_living::Services,
    ) -> Result<(), String> {
        if self
            .get(id)
            .and_then(|a| a.vimp)
            .is_some_and(|v| v.mode == 7)
        {
            self.select_vimp_noise_animation(
                id,
                globals,
                services.blending,
                services.random,
                external,
            )?;
            // The return visit never falls through to locomotion selection.
            self.advance_vimp_animation_selected(id, factory, globals, services, false)
        } else {
            self.advance_vimp_animation_offline(id, factory, globals, services)
        }
    }
    fn select_vimp_noise_animation(
        &mut self,
        id: Id,
        globals: &giants_formats::global_data::GlobalData,
        blending: crate::animation_channels::Blending,
        random: &mut crate::audio_parameters::Random,
        external: &mut dyn crate::vimp_living::Services,
    ) -> Result<(), String> {
        use crate::vimp_wander::{put, word};
        let a = self.get(id).ok_or("noise ANM owner absent")?;
        if a.vimp.is_none_or(|v| v.mode != 7)
            || !a.world_tick_running
            || a.vimp_physical.as_ref().is_none_or(|p| {
                !p.view.physics_consumed || p.view.animation_consumed || p.view.failure.is_some()
            })
        {
            return Err("noise ANM requires actual unconsumed primary physical phase".into());
        }
        let f = a.construction_fields.as_ref().unwrap();
        let channels = a.channels.unwrap();
        if f.object_flags & 0x80008000 != 0
            || channels != f.channels
            || a.vimp.unwrap().commands[1] != 0
        {
            return Err("noise ANM requires acquired visible zero-look source profile".into());
        }
        if !matches!(channels[0][0], 55 | 56) {
            let slot = if a.seed.storage.data().unwrap()[0x88] != 0 {
                55
            } else {
                56
            };
            // The generic selector's absent-descriptor guard is an acquired no-op.
            if self
                .specification(id)
                .unwrap()
                .animations
                .contains_key(&slot)
            {
                let mut channels = channels;
                crate::animation_channels::select_primary(
                    &mut channels,
                    crate::animation_channels::Request {
                        slot,
                        flags: 0x20,
                        speed_source: 0,
                        reference_speed: 0.,
                        blend_seconds: f32::from_bits(0x3f2aaaab),
                    },
                    blending,
                )?;
                let a = self.residents.get_mut(&id.serial).unwrap();
                a.channels = Some(channels);
                a.construction_fields.as_mut().unwrap().channels = channels;
            }
        } else if channels[0][8] & 2 != 0
            && crate::actor_session_predicates::first(
                &external.session(self, id, crate::vimp_response::Predicate::First)?,
                a.vimp.unwrap().born.unwrap().uid,
            )?
        {
            let priority = word(a.seed.storage.data().unwrap(), 0xd4) as i32;
            if priority > 1 {
                return Err("noise ANM high-priority return requires its active-response notification producer".into());
            }
            let low = globals.scalar(0x91c).map_err(|e| e.to_string())?;
            let high = globals.scalar(0x920).map_err(|e| e.to_string())?;
            let a = self.residents.get_mut(&id.serial).unwrap();
            a.vimp.as_mut().unwrap().mode = 1;
            let b = a.seed.storage.data_mut().unwrap();
            put(b, 0xd4, 0);
            put(b, 0x94, 0);
            let delay = crate::vimp_response::random_delay(random, low, high)?;
            put(b, 0x64, delay.to_bits());
            put(b, 0x74, 0);
            a.construction_fields.as_mut().unwrap().configuration[14] |= 0x400;
            put(b, 0xac, 1_f32.to_bits());
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    struct Caller;
    impl crate::vimp_living::Services for Caller {
        fn session(
            &mut self,
            _: &Session,
            _: Id,
            _: crate::vimp_response::Predicate,
        ) -> Result<crate::actor_session_predicates::Snapshot<'_>, String> {
            Ok(crate::actor_session_predicates::Snapshot {
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
            _: VimpResponseAction,
        ) -> Result<(), String> {
            Err("unexpected response action in selector control".into())
        }
        fn active(
            &mut self,
            _: &mut Session,
            _: Id,
            _: crate::vimp_active_response::Action,
        ) -> Result<(), String> {
            Err("unexpected active action in selector control".into())
        }
    }
    #[test]
    #[ignore = "requires GIANTS_NPC_ROOT and GIANTS_NPC_NOISE_ANIMATION_CONTROL; private native selector cases, synthetic owned phase/session/fields, no runtime admission"]
    fn source_vimp_noise_selector_matches_private_native_requests_and_return_words() {
        use crate::vimp_wander::{put, word};
        let (mut s, _, mut globals, _, children, _) =
            super::super::vimp_history_tests::source_births(true);
        let id = children[0];
        for slot in [55, 56] {
            assert!(s.specification(id).unwrap().animations.contains_key(&slot));
        }
        let input = std::fs::read_to_string(
            std::env::var_os("GIANTS_NPC_NOISE_ANIMATION_CONTROL").unwrap(),
        )
        .unwrap();
        let mut cases = 0;
        for line in input.lines() {
            let v = line
                .split_whitespace()
                .map(|x| u32::from_str_radix(x, 16).unwrap())
                .collect::<Vec<_>>();
            assert_eq!(v.len(), 22);
            let a = s.residents.get_mut(&id.serial).unwrap();
            let actor = a.vimp.as_mut().unwrap();
            actor.mode = 7;
            actor.born.as_mut().unwrap().uid = v[1];
            actor.commands = [0; 8];
            let tick = crate::vimp_wander::Tick {
                global_delta: 0.01,
                control_factor: 0.5,
                skip_heading: false,
                slow_areas: &[],
                now: 2.,
                noise_time: 0.,
                noise_position: [0.; 3],
            };
            let mut phase = crate::vimp_physical::Owned::new(&tick, [0.; 3], actor.context);
            phase.view.physics_consumed = true;
            a.vimp_physical = Some(phase);
            a.world_tick_running = true;
            let mut channels = [[0; 9]; 4];
            for c in &mut channels {
                c[0] = 254;
            }
            channels[0][0] = v[2];
            channels[0][8] = v[3];
            a.channels = Some(channels);
            let f = a.construction_fields.as_mut().unwrap();
            f.object_flags = 0;
            f.configuration[14] = v[9];
            f.channels = channels;
            let b = a.seed.storage.data_mut().unwrap();
            for (at, value) in [
                (0x88, v[4]),
                (0xd4, v[5]),
                (0x94, v[6]),
                (0x64, v[7]),
                (0x74, v[8]),
                (0xac, v[10]),
            ] {
                put(b, at, value);
            }
            globals.words[0x91c / 4] = v[11];
            globals.words[0x920 / 4] = v[12];
            let mut random = crate::audio_parameters::Random(v[0]);
            s.select_vimp_noise_animation(
                id,
                &globals,
                crate::animation_channels::Blending::Disabled,
                &mut random,
                &mut Caller,
            )
            .unwrap();
            let a = s.get(id).unwrap();
            let b = a.seed.storage.data().unwrap();
            let actual = [
                a.vimp.unwrap().mode,
                word(b, 0xd4),
                word(b, 0x94),
                word(b, 0x64),
                word(b, 0x74),
                a.construction_fields.as_ref().unwrap().configuration[14],
                word(b, 0xac),
                random.0,
            ];
            assert_eq!(actual, v[13..21], "native case {cases}");
            if v[21] == u32::MAX {
                assert_eq!(a.channels.unwrap(), channels);
            } else {
                assert_eq!(a.channels.unwrap()[0][0], v[21]);
            }
            cases += 1;
        }
        assert!(cases >= 768);
        println!(
            "private_native_noise_selector cases={cases} compared_words={} numeric_wrapper_boundary=request_only gameplay=false",
            cases * 9
        );
    }
    #[test]
    #[ignore = "requires GIANTS_NPC_ROOT; actual source herd births, supplied flat terrain/clock/control/noise/session and absent sound-bank boundary; no ordinary admission"]
    fn source_vimp_living_noise_cycles_consume_common_frames_and_effects() {
        let (mut s, factory, globals, _, children, mut random) =
            super::super::vimp_history_tests::source_births(false);
        let mut contexts = factory.animation_contexts(660);
        let mut stamp = crate::animation_epoch::Stamp::default();
        let mut noise = crate::world_noise::Noise::default();
        let parent = s.vimp(children[0]).unwrap().born.unwrap().parent;
        let mut entered = 0;
        let mut returned = 0;
        let mut effects = 0;
        for number in 0..3600 {
            let now = 2. + number as f32 / 60.;
            if number >= 30 && number % 30 == 0 {
                noise.publish(
                    parent,
                    now,
                    s.get(children[0])
                        .unwrap()
                        .construction_fields
                        .as_ref()
                        .unwrap()
                        .position,
                );
            }
            // Whole supplied cohort history precedes every callback.
            if number != 0 {
                for &id in &children {
                    s.prepare_vimp_world_history(id, number, &factory).unwrap();
                }
            }
            for &id in &children {
                let before = s.vimp(id).unwrap().mode;
                let frame = crate::vimp_frame::Frame {
                    number,
                    controlled_actor: Some(parent),
                    tick: crate::vimp_wander::Tick {
                        global_delta: 1. / 60.,
                        control_factor: 0.5,
                        skip_heading: false,
                        slow_areas: &[],
                        now,
                        noise_time: noise.seconds,
                        noise_position: noise.position,
                    },
                };
                let mut ground = |_| {
                    Ok(crate::physical_ground::Ground {
                        height: 7.25,
                        plane: [0., 0., 1., -7.25],
                        key: [0, 5],
                    })
                };
                let mut services = crate::vimp_frame::Services {
                    animation: crate::vimp_animation::Services {
                        blending: crate::animation_channels::Blending::Disabled,
                        contexts: &mut contexts,
                        random: &mut random,
                        stamp: &mut stamp,
                    },
                    ground: &mut ground,
                    recovery_sounds: None,
                };
                let tick = frame.tick;
                let outcome = if number == 0 {
                    s.advance_vimp_living_newborn(
                        id,
                        frame,
                        &factory,
                        &globals,
                        &mut services,
                        &mut Caller,
                    )
                } else {
                    s.advance_vimp_living_after_history(
                        id,
                        frame,
                        &factory,
                        &globals,
                        &mut services,
                        &mut Caller,
                    )
                };
                assert_eq!(
                    outcome.unwrap(),
                    crate::actor_world_tick::Outcome::UpdatedByPrimary
                );
                assert_eq!(s.completed_vimp_frame(id).unwrap().number, number + 1);
                let mode = s.vimp(id).unwrap().mode;
                entered += usize::from(before == 1 && mode == 7);
                returned += usize::from(before == 7 && mode == 1);
                assert!(
                    s.advance_vimp_living_after_history(
                        id,
                        crate::vimp_frame::Frame {
                            number,
                            controlled_actor: Some(parent),
                            tick
                        },
                        &factory,
                        &globals,
                        &mut services,
                        &mut Caller
                    )
                    .is_err()
                );
            }
            effects += contexts
                .advance_positional_sounds(
                    &mut random,
                    &mut crate::positional_effect_audio::AbsentBank,
                )
                .unwrap();
            assert!(contexts.active().is_empty());
        }
        assert!(entered > 2 && returned > 2 && effects > 0);
        let id = children[0];
        assert!(Scope::enter(&s, id).is_err());
        let serial = id.serial;
        let current = s.residents.get_mut(&serial).unwrap();
        current.world_tick_running = true;
        current.vimp.as_mut().unwrap().wander_pending = false;
        let scope = Scope::enter(&s, id).unwrap();
        assert!(scope.check(&s, children[1]).is_err());
        s.residents.get_mut(&serial).unwrap().vimp_frames += 1;
        assert!(scope.check(&s, id).is_err());
        println!(
            "supplied_living_cohort callbacks={} entered={entered} returned={returned} retired_sound_contexts={effects} pending_contexts=0 gameplay=false",
            children.len() * 3600
        );
    }
}
