//! Acquired ordinary class73 branches and class230 modes0/61..68/71 over current views.
//! Shared prefix/perception and sound tails retain their class differences.
//! Later unsupported continuations require their actual services.
//! Prototype PS2 search key: `ripper_move`; exact PC correspondence unknown.
//! PC debug-1.1 0x004d61f0 remains the acquired behavior reference; its previous
//! alias is retained in docs/SYMBOL_NAVIGATION.md.
use crate::{
    audio_parameters::Random,
    enemy_projectile::SoundOutcome,
    ripper_perception::{Frame, Query},
    ripper_steering::{Orbit, State as Steering},
    ripper_turn_completion::{self, Parameters},
    state23_entry::{self, Context, State as Entry},
};
mod cooldown;
pub use cooldown::VariantCooldownContext;
#[derive(Clone, Debug)]
pub struct State<K, E> {
    pub entry: Entry,
    pub steering: Steering,
    pub orbit: Orbit,
    pub timer: f32,
    pub effect: Option<E>,
    pub retaliation: Option<K>,
    pub primary: [u32; 9],
    /// Independent D +184 scalar, not the class speed parameter at +194.
    pub parameter_184: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct TargetShape {
    pub scale: f32,
    pub center_offset: f32,
}
#[derive(Clone, Copy)]
pub struct AttackContext<'a, K> {
    pub parameters: crate::ripper_attack::Parameters,
    pub target_shapes: &'a std::collections::BTreeMap<K, TargetShape>,
}
#[derive(Clone, Copy)]
pub struct VariantAttackContext<'a, K> {
    pub parameters: crate::ripper_attack::VariantParameters,
    pub target_shapes: &'a std::collections::BTreeMap<K, TargetShape>,
}
#[derive(Clone, Copy)]
pub struct AwakeningContext {
    pub camera: [f32; 3],
    pub parameters: crate::ripper_awakening::Parameters,
    pub buried_speed_percent: f32,
}

#[derive(Clone, Copy)]
pub struct VariantAwakeningContext<'a> {
    pub camera: [f32; 3],
    pub camera_range: f32,
    pub global_seconds: f32,
    pub areas: &'a [crate::object_time::SlowArea],
}
/// Actual class-global radii used by class230 mode63. They must be read from
/// the installed global data, not inferred from model size or class73 orbit.
#[derive(Clone, Copy)]
pub struct VariantDecisionContext {
    pub close_range: f32,
    pub fire_range: f32,
}

/// Actual class-global melee range and alternate-attack decision threshold.
/// These are separate from the earlier approach/fire selection radii.
#[derive(Clone, Copy)]
pub struct VariantCloseContext {
    pub attack_range: f32,
    pub alternate_threshold: f32,
}
/// Actual melee reach and class-global damage; neither comes from model size
/// or projectile damage. Kind3 hurt remains a required consumer.
#[derive(Clone, Copy)]
pub struct VariantMeleeContext {
    pub attack_range: f32,
    pub damage: f32,
}

#[derive(Clone, Copy)]
pub struct AwakeningFrame<'a, K> {
    pub context: Context,
    pub frame: &'a Frame<'a, K>,
    pub range: f32,
    pub parameters: AwakeningContext,
}
#[derive(Clone, Copy)]
pub struct FrameInput<'a, K> {
    pub context: Context,
    pub frame: &'a Frame<'a, K>,
    pub range: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ripper_perception::Candidate;
    include!("ripper_primary_turn/close_tests.rs");
    include!("ripper_primary_turn/melee_tests.rs");
    struct Backend {
        removals: u32,
        sounds: u32,
        fail_sound: bool,
    }
    impl Services<u32, u32> for Backend {
        fn remove_effect(
            &mut self,
            state: &State<u32, u32>,
            effect: u32,
            _: &mut Random,
        ) -> Result<(), String> {
            assert_eq!((state.entry.mode, state.effect, effect), (32, Some(7), 7));
            self.removals += 1;
            Ok(())
        }
        fn owner_sound(
            &mut self,
            state: &State<u32, u32>,
            cue: u32,
            _: &mut Random,
        ) -> Result<SoundOutcome, String> {
            assert_eq!((state.entry.mode, state.effect, cue), (33, None, 1261));
            assert_eq!(state.entry.parameter, 10.);
            self.sounds += 1;
            if self.fail_sound {
                Err("sound backend unavailable".into())
            } else {
                Ok(SoundOutcome::ResourceUnavailable)
            }
        }
    }
    fn state() -> State<u32, u32> {
        State {
            entry: Entry {
                class: 73,
                context: 4,
                mode: 32,
                action: 0,
                flags: 0x3000000,
                control: 0,
                packed: 0,
                position: [0.; 3],
                parameter: 10.,
                active_limit: 100.,
                saved_parameter: 80.,
                retained_timer: 5.,
                target: None,
            },
            steering: Steering {
                position: [0.; 3],
                heading: 0.,
                flags: 0x3000000,
                personality: 0x40,
                commands: [0; 8],
                target_heading: 0.,
                target_position: [0.; 3],
            },
            orbit: Orbit {
                angle: 0.,
                radius: 0.,
            },
            timer: 0.,
            effect: Some(7),
            retaliation: Some(42),
            primary: [40, 0, 0, 0, 0, 0, 0, 0, 0],
            parameter_184: 0.,
        }
    }
    struct InitialAudio {
        calls: u32,
        fail: bool,
    }
    impl Services<u32, u32> for InitialAudio {
        fn owner_sound(
            &mut self,
            s: &State<u32, u32>,
            cue: u32,
            _: &mut Random,
        ) -> Result<SoundOutcome, String> {
            assert_eq!((s.entry.context, s.entry.mode, cue), (100, 0, 1258));
            assert_eq!((s.entry.active_limit, s.entry.saved_parameter), (100., 80.));
            self.calls += 1;
            if self.fail {
                Err("initial audio provider failed".into())
            } else {
                Ok(SoundOutcome::ResourceUnavailable)
            }
        }
        fn remove_effect(
            &mut self,
            _: &State<u32, u32>,
            _: u32,
            _: &mut Random,
        ) -> Result<(), String> {
            Err("unexpected retirement".into())
        }
    }
    #[test]
    fn initial_sound_and_later_ground_failure_keep_ordered_state_without_replay() {
        let candidates = [candidate()];
        let frame = Frame::new(&candidates, &[42]).unwrap();
        for fail in [true, false] {
            let mut s = state();
            s.entry.mode = 0;
            s.effect = None;
            let mut m = Machine::new(s);
            let mut random = Random(12);
            let mut services = InitialAudio { calls: 0, fail };
            let a = AwakeningContext {
                camera: [0.; 3],
                parameters: crate::ripper_awakening::Parameters {
                    camera_range: 50.,
                    slope_decision: -1.,
                    orbit_radius: [20., 40.],
                    seconds: 3.,
                },
                buried_speed_percent: 50.,
            };
            assert!(
                m.advance_awakening(context(), &frame, 500., a, &mut random, &mut services)
                    .is_err()
            );
            assert_eq!(services.calls, 1);
            assert!(m.failure().is_some());
            assert_eq!(m.state().entry.context, 100);
            if fail {
                assert_eq!(
                    (
                        m.state().entry.mode,
                        m.state().entry.active_limit,
                        m.state().entry.saved_parameter
                    ),
                    (0, 100., 80.)
                );
            } else {
                assert_eq!(
                    (
                        m.state().entry.mode,
                        m.state().entry.active_limit,
                        m.state().entry.saved_parameter
                    ),
                    (31, 0., 10.)
                );
            }
            let seed = random.0;
            assert!(
                m.advance_awakening(context(), &frame, 500., a, &mut random, &mut services)
                    .is_err()
            );
            assert_eq!((services.calls, random.0), (1, seed));
            assert!(m.advance_motion(motion(), &environment()).is_err());
        }
    }
    struct SurfaceFixture {
        contexts: crate::attached_effect_context::Pool<u32>,
        reset: crate::actor_surface_reset::State,
        sounds: u32,
        resets: u32,
    }
    fn surface_fixture(position: [f32; 3]) -> SurfaceFixture {
        SurfaceFixture {
            contexts: crate::attached_effect_context::Pool::new(1001, None),
            reset: crate::actor_surface_reset::State {
                position,
                current: [2.; 3],
                previous: [3.; 3],
                rates: [4.; 4],
                previous_rates: [5.; 4],
                velocity: [6.; 3],
                previous_velocity: [7.; 3],
            },
            sounds: 0,
            resets: 0,
        }
    }
    impl Services<u32, u32> for SurfaceFixture {
        fn surface_popup(
            &mut self,
            s: &State<u32, u32>,
            random: &mut Random,
        ) -> Result<crate::enemy_projectile::EffectOutcome, String> {
            use crate::attached_effect_context::Creation;
            use crate::effect_initialization::Selection;
            let (result, intervals) = self.contexts.prepare_positional(
                "fx_ripper_popup",
                s.entry.position,
                Selection {
                    active_objects: 0,
                    quality: 1.,
                    lite_distance: 500.,
                    camera: [0.; 3],
                    recipient_position: s.entry.position,
                },
                random,
            )?;
            assert_eq!(result, Creation::Absent);
            assert!(intervals.is_empty());
            Ok(crate::enemy_projectile::EffectOutcome::Rejected)
        }
        fn reset_surface_motion(&mut self, s: &State<u32, u32>) -> Result<(), String> {
            assert_eq!(s.entry.mode, 32);
            assert_eq!(s.parameter_184, 0.);
            assert_eq!(s.steering.personality & 0x800, 0);
            self.reset = crate::actor_surface_reset::reset(self.reset);
            self.resets += 1;
            Ok(())
        }
        fn remove_effect(
            &mut self,
            _: &State<u32, u32>,
            _: u32,
            _: &mut Random,
        ) -> Result<(), String> {
            Err("fixture owns no dirt context".into())
        }
        fn owner_sound(
            &mut self,
            s: &State<u32, u32>,
            cue: u32,
            _: &mut Random,
        ) -> Result<SoundOutcome, String> {
            assert_eq!((s.entry.mode, cue), (44, 1267));
            self.sounds += 1;
            // Explicit empty sound registry fixture.
            Ok(SoundOutcome::ResourceUnavailable)
        }
    }
    #[test]
    fn static_ground_does_not_acquit_missing_dirt_fire_audio_or_marker_services() {
        let grid = giants_formats::gti::Terrain {
            source_flags: 0,
            width: 2,
            height: 2,
            origin: [0.; 2],
            spacing: 1.,
            height_bounds: [0.; 2],
            fallback_texture: String::new(),
            heights: vec![0.; 4],
            colors: vec![],
            tags: vec![5; 4],
            consumed_bytes: 0,
            opaque_tail: vec![],
        };
        let query = crate::terrain_queries::StaticTerrainQuery::new(
            &grid,
            1.,
            crate::arithmetic::Precision::Double53,
        )
        .unwrap();
        let mut backend = Backend {
            removals: 0,
            sounds: 0,
            fail_sound: false,
        };
        let mut provider = crate::ripper_ground::StaticGround {
            query: &query,
            services: &mut backend,
        };
        let mut state = state();
        let mut random = Random(1);
        assert_eq!(provider.ground_normal_z(&state).unwrap(), 1.);
        assert!(
            provider
                .prepare_awakening_dirt(&state, &mut random)
                .is_err()
        );
        assert!(provider.fire(&state, [0.; 3], 0., &mut random).is_err());
        assert!(
            provider
                .stop_owner_sound(&state, 1256, &mut random)
                .is_err()
        );
        assert!(provider.construct_surface_marker(&state).is_err());
        assert!(provider.reset_surface_motion(&state).is_err());
        state.entry.position[0] = f32::NAN;
        assert!(provider.ground_normal_z(&state).is_err());
        assert_eq!((backend.sounds, backend.removals, random.0), (0, 0, 1));
    }
    #[test]
    fn positive_surface_refuses_missing_marker_after_real_reset_and_never_replays() {
        let mut s = state();
        s.entry.mode = 44;
        s.entry.position[2] = 1.;
        s.steering.position = s.entry.position;
        s.steering.personality = 0x840;
        s.parameter_184 = 72.;
        s.effect = None;
        let mut backend = SurfaceFixture {
            contexts: crate::attached_effect_context::Pool::new(1001, None),
            reset: crate::actor_surface_reset::State {
                position: s.entry.position,
                current: [2.; 3],
                previous: [3.; 3],
                rates: [4.; 4],
                previous_rates: [5.; 4],
                velocity: [6.; 3],
                previous_velocity: [7.; 3],
            },
            sounds: 0,
            resets: 0,
        };
        let candidates = [candidate()];
        let frame = Frame::new(&candidates, &[42]).unwrap();
        let mut machine = Machine::new(s);
        let mut random = Random(12);
        assert!(
            machine
                .advance_surfacing(
                    context(),
                    &frame,
                    500.,
                    parameters(),
                    &mut random,
                    &mut backend
                )
                .is_err()
        );
        assert_eq!(
            machine.failure(),
            Some("actual owned surfacing marker unavailable")
        );
        assert_eq!((backend.sounds, backend.resets, random.0), (1, 1, 12));
        assert_eq!(backend.reset.current, [0., 0., 1.]);
        assert_eq!(backend.reset.previous, [2.; 3]);
        assert_eq!(backend.reset.rates, [0.; 4]);
        assert_eq!(backend.reset.velocity, [0.; 3]);
        assert_eq!(machine.state().entry.mode, 32);
        assert_eq!(machine.state().entry.parameter, 10.);
        assert_eq!(machine.state().parameter_184, 0.);
        assert!(machine.surface_marker().is_none());
        assert!(
            machine
                .advance_surfacing(
                    context(),
                    &frame,
                    500.,
                    parameters(),
                    &mut random,
                    &mut backend
                )
                .is_err()
        );
        assert!(machine.advance_motion(motion(), &environment()).is_err());
        assert_eq!((backend.sounds, backend.resets, random.0), (1, 1, 12));
    }
    #[test]
    fn surface_turn_preflight_failure_after_external_services_keeps_partial_state() {
        let mut s = state();
        s.entry.mode = 44;
        s.effect = None;
        s.primary[8] = 2;
        s.parameter_184 = 72.;
        let mut backend = surface_fixture(s.entry.position);
        let candidates = [candidate()];
        let frame = Frame::new(&candidates, &[42]).unwrap();
        let mut machine = Machine::new(s);
        let mut random = Random(12);
        let mut bad = parameters();
        bad.seconds = f32::NAN;
        assert!(
            machine
                .advance_surfacing(context(), &frame, 500., bad, &mut random, &mut backend)
                .is_err()
        );
        assert!(machine.failure().is_some());
        assert_eq!(machine.state().entry.mode, 32);
        assert_eq!(machine.state().parameter_184, 0.);
        assert_eq!((backend.sounds, backend.resets), (1, 1));
        assert!(
            machine
                .advance_surfacing(
                    context(),
                    &frame,
                    500.,
                    parameters(),
                    &mut random,
                    &mut backend
                )
                .is_err()
        );
        assert_eq!((backend.sounds, backend.resets), (1, 1));
    }
    fn candidate() -> Candidate<u32> {
        Candidate {
            id: 42,
            class: 2,
            state: 2,
            active: 0,
            personality: 1,
            control: 0,
            position: [10., 0., 0.],
            damage: 0.,
            limit: 100.,
            parent_present: false,
        }
    }
    #[test]
    fn variant_turn_consumes_current_channel_without_class73_effects_or_speed_restore() {
        struct NoServices;
        impl Services<u32, u32> for NoServices {
            fn remove_effect(
                &mut self,
                _: &State<u32, u32>,
                _: u32,
                _: &mut Random,
            ) -> Result<(), String> {
                Err("unexpected effect removal".into())
            }
            fn owner_sound(
                &mut self,
                _: &State<u32, u32>,
                _: u32,
                _: &mut Random,
            ) -> Result<SoundOutcome, String> {
                Err("unexpected sound request".into())
            }
        }
        for completed in [false, true] {
            for has_target in [false, true] {
                let mut s = state();
                s.entry.class = 230;
                s.entry.mode = 62;
                s.entry.parameter = 17.;
                s.retaliation = has_target.then_some(42);
                s.effect = None;
                s.primary = [0; 9];
                s.primary[8] = if completed { 2 } else { 0 };
                let actors = if has_target {
                    vec![candidate()]
                } else {
                    vec![]
                };
                let order = if has_target { vec![42] } else { vec![] };
                let frame = Frame::new(&actors, &order).unwrap();
                let mut machine = Machine::new(s);
                let mut random = Random(51);
                let input = FrameInput {
                    context: context(),
                    frame: &frame,
                    range: 500.,
                };
                assert_eq!(
                    machine
                        .advance_variant_turn(input, &mut random, &mut NoServices)
                        .unwrap(),
                    Outcome::CompletedVariantTurn
                );
                let result = machine.state();
                assert_eq!(result.entry.mode, if completed { 63 } else { 62 });
                assert_eq!(result.entry.parameter, 17.);
                assert_eq!(result.steering.personality & 0x40, 0);
                assert_eq!(result.entry.flags & 0x03000000, 0x01000000);
                assert_eq!(result.steering.commands[2], 0);
                assert_eq!(result.steering.commands[0], 32);
                assert_eq!(random.0, 51);
                assert!(machine.awaiting_motion());
                assert!(
                    machine
                        .advance_variant_turn(input, &mut random, &mut NoServices)
                        .is_err()
                );
            }
        }
    }
    fn context() -> Context {
        Context {
            paused: false,
            native_mode: 0,
            parent_state: None,
            player: None,
            delta: 0.1,
        }
    }
    #[test]
    fn variant_decision_preserves_radius_boundaries_and_latches_failed_lost_sound() {
        struct LostSound(bool);
        impl Services<u32, u32> for LostSound {
            fn remove_effect(
                &mut self,
                _: &State<u32, u32>,
                _: u32,
                _: &mut Random,
            ) -> Result<(), String> {
                Err("unexpected removal".into())
            }
            fn owner_sound(
                &mut self,
                s: &State<u32, u32>,
                cue: u32,
                _: &mut Random,
            ) -> Result<SoundOutcome, String> {
                assert_eq!((s.entry.mode, cue), (63, 1248));
                if self.0 {
                    Err("supplied transport failure".into())
                } else {
                    Ok(SoundOutcome::Suppressed)
                }
            }
        }
        let parameters = VariantDecisionContext {
            close_range: 10.,
            fire_range: 20.,
        };
        for (distance, expected) in [(9.99, 67), (10., 65), (19.99, 65), (20., 64)] {
            let mut candidate = candidate();
            candidate.position = [distance, 0., 0.];
            let actors = [candidate];
            let frame = Frame::new(&actors, &[42]).unwrap();
            let mut s = state();
            s.entry.class = 230;
            s.entry.mode = 63;
            s.effect = None;
            let mut m = Machine::new(s);
            let mut random = Random(17);
            m.advance_variant_decision(
                FrameInput {
                    context: context(),
                    frame: &frame,
                    range: 100.,
                },
                parameters,
                &mut random,
                &mut LostSound(true),
            )
            .unwrap();
            assert_eq!(m.state().entry.mode, expected);
            assert_eq!(random.0, 17);
        }
        let frame = Frame::new(&[], &[]).unwrap();
        for fail in [false, true] {
            let mut s = state();
            s.entry.class = 230;
            s.entry.mode = 63;
            s.retaliation = None;
            let mut m = Machine::new(s);
            let mut random = Random(17);
            let input = FrameInput {
                context: context(),
                frame: &frame,
                range: 100.,
            };
            assert_eq!(
                m.advance_variant_decision(input, parameters, &mut random, &mut LostSound(fail))
                    .is_err(),
                fail
            );
            assert_eq!(m.state().entry.mode, if fail { 63 } else { 71 });
            assert_eq!(m.failure().is_some(), fail);
            assert!(
                m.advance_variant_decision(input, parameters, &mut random, &mut LostSound(false))
                    .is_err()
            );
        }
    }
    fn parameters() -> Parameters {
        Parameters {
            radius: [20., 40.],
            seconds: 3.,
        }
    }
    fn motion() -> crate::actor_common_motion::State {
        crate::actor_common_motion::State {
            position: [999.; 3],
            heading: 180.,
            look_pitch: 0.,
            rates: [0.; 4],
            previous_rates: [0.; 4],
            active: 0,
            personality: 0,
            state: 0,
            context: 99,
            parent_present: false,
            controls: [-32, 0, 32, 0],
            target_heading: 0.,
            target_position: [999.; 2],
        }
    }
    fn environment() -> crate::actor_common_motion::Environment<'static> {
        crate::actor_common_motion::Environment {
            parameters: crate::actor_common_motion::Parameters {
                acceleration: 10.,
                speed: 999.,
                braking: 1.,
                attenuation: 1.,
                turn_acceleration: 10.,
                turn_speed: 20.,
                turn_braking: 1.,
                gravity: 0.,
            },
            global_seconds: 0.1,
            areas: &[],
            control_factor: 1.,
            running_factor: 1.,
            skip_heading: false,
            controlled_actor: false,
        }
    }
    #[test]
    fn pure_failure_rolls_back_prefix_and_motion_consumes_own_commands_once() {
        let mut machine = Machine::new(state());
        let mut random = Random(12);
        let mut backend = Backend {
            removals: 0,
            sounds: 0,
            fail_sound: false,
        };
        let empty = Frame::new(&[], &[]).unwrap();
        assert!(
            machine
                .advance(
                    context(),
                    &empty,
                    500.,
                    parameters(),
                    &mut random,
                    &mut backend
                )
                .is_err()
        );
        assert_eq!(machine.state().entry.retained_timer, 5.);
        assert!(machine.failure().is_none());
        let candidates = [candidate()];
        let frame = Frame::new(&candidates, &[42]).unwrap();
        machine
            .advance(
                context(),
                &frame,
                500.,
                parameters(),
                &mut random,
                &mut backend,
            )
            .unwrap();
        assert_eq!((backend.removals, backend.sounds, random.0), (0, 0, 12));
        assert_eq!(machine.state().entry.parameter, 80.);
        assert!(machine.publish_primary_channel([1; 9]).is_err());
        assert!(
            machine
                .advance(
                    context(),
                    &frame,
                    500.,
                    parameters(),
                    &mut random,
                    &mut backend
                )
                .is_err()
        );
        let next = machine.advance_motion(motion(), &environment()).unwrap();
        assert_eq!(next.context, 4);
        assert_eq!(next.position, [0.; 3]);
        assert_eq!(next.rates[1], 0.);
        assert!(machine.advance_motion(motion(), &environment()).is_err());
        let channel = [2, 0.7_f32.to_bits(), 0.5_f32.to_bits(), 0, 0, 0, 0, 0, 2];
        machine.publish_primary_channel(channel).unwrap();
        assert_eq!(machine.state().primary, channel);
        assert_eq!(random.0, 12);
    }
    #[test]
    fn sound_failure_keeps_effect_retirement_draws_and_pre_restore_state_without_replay() {
        let mut s = state();
        s.primary = [2, 0.7_f32.to_bits(), 0.5_f32.to_bits(), 0, 0, 0, 0, 0, 2];
        let mut machine = Machine::new(s);
        let mut random = Random(12);
        let mut backend = Backend {
            removals: 0,
            sounds: 0,
            fail_sound: true,
        };
        let candidates = [candidate()];
        let frame = Frame::new(&candidates, &[42]).unwrap();
        assert!(
            machine
                .advance(
                    context(),
                    &frame,
                    500.,
                    parameters(),
                    &mut random,
                    &mut backend
                )
                .is_err()
        );
        assert_eq!(
            (
                machine.state().entry.mode,
                machine.state().effect,
                machine.state().entry.parameter
            ),
            (33, None, 10.)
        );
        assert_eq!(machine.state().timer, 3.);
        assert_ne!(random.0, 12);
        let seed = random.0;
        assert!(
            machine
                .advance(
                    context(),
                    &frame,
                    500.,
                    parameters(),
                    &mut random,
                    &mut backend
                )
                .is_err()
        );
        assert!(machine.advance_motion(motion(), &environment()).is_err());
        assert_eq!((backend.removals, backend.sounds, random.0), (1, 1, seed));
        assert!(machine.publish_primary_channel([0; 9]).is_err());
        assert_eq!(machine.state().primary[8], 2);
    }
    #[test]
    fn rotation_and_pursuit_continue_through_attack_until_real_fire_service_required() {
        let mut s = state();
        s.primary[8] = 2;
        let mut machine = Machine::new(s);
        let mut random = Random(12);
        let mut backend = Backend {
            removals: 0,
            sounds: 0,
            fail_sound: false,
        };
        let actors = [candidate()];
        let frame = Frame::new(&actors, &[42]).unwrap();
        machine
            .advance(
                context(),
                &frame,
                500.,
                parameters(),
                &mut random,
                &mut backend,
            )
            .unwrap();
        assert_eq!(machine.state().entry.mode, 33);
        machine.advance_motion(motion(), &environment()).unwrap();
        machine.publish_primary_channel([0; 9]).unwrap();
        let before = random.0;
        assert_eq!(
            machine
                .advance_pursuit(context(), &frame, 500., 100., &mut random, &mut backend)
                .unwrap(),
            Outcome::CompletedMode33
        );
        assert_eq!(machine.state().entry.mode, 37);
        assert_ne!(random.0, before);
        assert_eq!(machine.state().entry.parameter, 80.);
        assert!(machine.state().effect.is_none());
        assert_eq!((backend.removals, backend.sounds), (1, 0));
        machine.advance_motion(motion(), &environment()).unwrap();
        assert!(
            machine
                .advance_pursuit(context(), &frame, 500., 100., &mut random, &mut backend)
                .is_err()
        );
        let attack = crate::ripper_attack::Parameters {
            repeat_percent: 0.,
            charge_percent: 100.,
            announce_percent: 100.,
            vertical_spread_percent: 75.,
            orbit: parameters(),
        };
        let empty_shapes = std::collections::BTreeMap::new();
        let timer = machine.state().entry.retained_timer;
        assert!(
            machine
                .advance_attack(
                    context(),
                    &frame,
                    500.,
                    AttackContext {
                        parameters: attack,
                        target_shapes: &empty_shapes
                    },
                    &mut random,
                    &mut backend
                )
                .is_err()
        );
        assert!(machine.failure().is_none());
        assert_eq!(
            machine.state().entry.retained_timer.to_bits(),
            timer.to_bits()
        );
        let shapes = std::collections::BTreeMap::from([(
            42,
            TargetShape {
                scale: 1.,
                center_offset: 2.,
            },
        )]);
        let attack_context = AttackContext {
            parameters: attack,
            target_shapes: &shapes,
        };
        machine
            .advance_attack(
                context(),
                &frame,
                500.,
                attack_context,
                &mut random,
                &mut backend,
            )
            .unwrap();
        assert_eq!(machine.state().entry.mode, 38);
        machine.advance_motion(motion(), &environment()).unwrap();
        machine
            .publish_primary_channel([1, 0, 0, 0, 0, 0, 0, 0, 2])
            .unwrap();
        machine
            .advance_attack(
                context(),
                &frame,
                500.,
                attack_context,
                &mut random,
                &mut backend,
            )
            .unwrap();
        assert_eq!(machine.state().entry.mode, 39);
        machine.advance_motion(motion(), &environment()).unwrap();
        machine
            .publish_primary_channel([50, 0x3edd_ddde, 0x3edd_dddd, 0, 0, 0, 0, 0, 0])
            .unwrap();
        let seed = random.0;
        assert!(
            machine
                .advance_attack(
                    context(),
                    &frame,
                    500.,
                    attack_context,
                    &mut random,
                    &mut backend
                )
                .is_err()
        );
        assert_eq!(
            machine.failure(),
            Some("actual primary projectile service unavailable")
        );
        assert!(
            machine
                .advance_attack(
                    context(),
                    &frame,
                    500.,
                    attack_context,
                    &mut random,
                    &mut backend
                )
                .is_err()
        );
        assert!(machine.advance_motion(motion(), &environment()).is_err());
        assert_eq!((backend.removals, backend.sounds, random.0), (1, 0, seed));
    }
    #[test]
    fn variant_entry_requeries_time_and_retains_a_required_marker_failure() {
        struct Capture(Vec<u32>);
        impl Services<u32, u32> for Capture {
            fn owner_sound(
                &mut self,
                _: &State<u32, u32>,
                cue: u32,
                _: &mut Random,
            ) -> Result<SoundOutcome, String> {
                self.0.push(cue);
                Ok(SoundOutcome::Suppressed)
            }
            fn remove_effect(
                &mut self,
                _: &State<u32, u32>,
                _: u32,
                _: &mut Random,
            ) -> Result<(), String> {
                Err("unexpected effect retirement".into())
            }
            fn construct_surface_marker(&mut self, _: &State<u32, u32>) -> Result<u32, String> {
                self.0.push(219);
                Err("actual marker provider unavailable".into())
            }
        }
        let candidates = [candidate()];
        let frame = Frame::new(&candidates, &[42]).unwrap();
        for owner in [0, 100] {
            let mut s = state();
            s.entry.class = 230;
            s.entry.mode = 0;
            s.entry.context = 0;
            s.effect = None;
            s.retaliation = None;
            let mut machine = Machine::new(s);
            let mut random = Random(42);
            let mut services = Capture(Vec::new());
            let areas = [crate::object_time::SlowArea {
                source_owner: owner,
                center: [0.; 3],
                radius_squared: 100.,
                factor: 0.,
            }];
            let mut context = context();
            context.delta = if owner == 0 { 0.1 } else { 0. };
            let parameters = VariantAwakeningContext {
                camera: [0.; 3],
                camera_range: 50.,
                global_seconds: 0.1,
                areas: &areas,
            };
            let input = FrameInput {
                context,
                frame: &frame,
                range: 500.,
            };
            let result =
                machine.advance_variant_awakening(input, parameters, &mut random, &mut services);
            assert_eq!(result.is_err(), owner == 100);
            assert_eq!(machine.state().entry.context, 100);
            assert_eq!(machine.state().entry.parameter, 10.);
            assert_eq!(machine.state().entry.saved_parameter, 80.);
            assert_eq!(random.0, 42);
            if owner == 0 {
                assert_eq!(machine.state().entry.mode, 61);
                assert_eq!(machine.state().timer.to_bits(), 0);
                assert!(machine.awaiting_motion());
                assert_eq!(services.0, [1235]);
            } else {
                assert_eq!(machine.state().entry.mode, 62);
                assert_eq!(machine.state().parameter_184.to_bits(), 0);
                assert_eq!(
                    machine.failure(),
                    Some("actual marker provider unavailable")
                );
                assert!(!machine.awaiting_motion());
                assert_eq!(services.0, [1235, 1246, 219]);
            }
            let calls = services.0.clone();
            assert!(
                machine
                    .advance_variant_awakening(input, parameters, &mut random, &mut services)
                    .is_err()
            );
            assert_eq!(services.0, calls);
        }
    }

    #[derive(Default)]
    struct CooldownAudio {
        calls: u32,
        fail: bool,
    }
    impl Services<u32, u32> for CooldownAudio {
        fn owner_sound(
            &mut self,
            s: &State<u32, u32>,
            cue: u32,
            _: &mut Random,
        ) -> Result<SoundOutcome, String> {
            assert_eq!((s.entry.mode, cue), (66, 1248));
            assert!(s.timer.is_sign_negative());
            self.calls += 1;
            if self.fail {
                Err("actual supplied sound failure".into())
            } else {
                Ok(SoundOutcome::Suppressed)
            }
        }
        fn remove_effect(
            &mut self,
            _: &State<u32, u32>,
            _: u32,
            _: &mut Random,
        ) -> Result<(), String> {
            Err("unexpected cooldown effect".into())
        }
    }
    #[test]
    fn variant_cooldown_obeys_exact_time_and_distance_boundaries() {
        for (timer, seconds, distance, expected) in [
            (0., 0., 80., 66),
            (0.125, 0.125, 80., 66),
            (0., 0.125, 34., 67),
            (0., 0.125, 35., 71),
        ] {
            let mut s = state();
            s.entry.class = 230;
            s.entry.mode = 66;
            s.timer = timer;
            s.primary = [1, 0, 0, 0, 0, 0, 0, 0, 0];
            let mut c = candidate();
            c.position = [distance, 0., 0.];
            let actors = [c];
            let frame = Frame::new(&actors, &[42]).unwrap();
            let mut m = Machine::new(s);
            let mut r = Random(42);
            let mut audio = CooldownAudio::default();
            m.advance_variant_cooldown(
                FrameInput {
                    context: context(),
                    frame: &frame,
                    range: 1000.,
                },
                VariantCooldownContext {
                    close_range: 35.,
                    retreat_timer: [2., 6.],
                    global_seconds: seconds,
                    areas: &[],
                },
                &mut r,
                &mut audio,
            )
            .unwrap();
            assert_eq!(m.state().entry.mode, expected);
            assert_eq!(audio.calls, u32::from(expected == 71));
            let mut expected_random = Random(42);
            if expected == 71 {
                crate::vimp_response::random_delay(&mut expected_random, 2., 6.).unwrap();
            }
            assert_eq!(r.0, expected_random.0);
        }
    }
    #[test]
    fn variant_cooldown_uses_current_object_time_and_wide_expiry_before_store() {
        let actors = [candidate()];
        let frame = Frame::new(&actors, &[42]).unwrap();
        for (owner, timer, seconds, expected) in [
            (9, 0.015, 0.02, 66),
            (4, 0.015, 0.02, 71),
            (9, 0., f32::from_bits(1), 71),
        ] {
            let mut s = state();
            s.entry.class = 230;
            s.entry.mode = 66;
            s.timer = timer;
            s.primary = [1, 0, 0, 0, 0, 0, 0, 0, 0];
            let areas = [crate::object_time::SlowArea {
                source_owner: owner,
                center: [0.; 3],
                radius_squared: 1.,
                factor: 0.5,
            }];
            let mut m = Machine::new(s);
            let mut r = Random(1);
            let mut audio = CooldownAudio::default();
            m.advance_variant_cooldown(
                FrameInput {
                    context: context(),
                    frame: &frame,
                    range: 1000.,
                },
                VariantCooldownContext {
                    close_range: 1.,
                    retreat_timer: [2., 6.],
                    global_seconds: seconds,
                    areas: &areas,
                },
                &mut r,
                &mut audio,
            )
            .unwrap();
            assert_eq!(m.state().entry.mode, expected);
            assert_eq!(audio.calls, u32::from(expected == 71));
        }
    }
    #[test]
    fn variant_cooldown_sound_failure_keeps_negative_timer_without_draw_or_replay() {
        let actors = [candidate()];
        let frame = Frame::new(&actors, &[42]).unwrap();
        let mut s = state();
        s.entry.class = 230;
        s.entry.mode = 66;
        s.timer = 0.;
        s.primary = [1, 0, 0, 0, 0, 0, 0, 0, 0];
        let mut m = Machine::new(s);
        let mut r = Random(1);
        let mut audio = CooldownAudio {
            fail: true,
            ..Default::default()
        };
        let input = FrameInput {
            context: context(),
            frame: &frame,
            range: 1000.,
        };
        let p = VariantCooldownContext {
            close_range: 1.,
            retreat_timer: [2., 6.],
            global_seconds: 0.1,
            areas: &[],
        };
        assert!(
            m.advance_variant_cooldown(input, p, &mut r, &mut audio)
                .is_err()
        );
        assert_eq!(m.state().entry.mode, 66);
        assert!(m.state().timer < 0.);
        assert_eq!(r.0, 1);
        assert!(!m.awaiting_motion());
        assert!(
            m.advance_variant_cooldown(input, p, &mut r, &mut audio)
                .is_err()
        );
        assert_eq!(audio.calls, 1);
        assert_eq!(r.0, 1);
    }

    #[derive(Default)]
    struct RetreatAudio {
        events: Vec<&'static str>,
        fail_mark: bool,
        fail_sound: bool,
    }
    impl Services<u32, u32> for RetreatAudio {
        fn mark_surface_marker_retirement(&mut self, s: &State<u32, u32>) -> Result<(), String> {
            assert_eq!(s.entry.mode, 71);
            assert_ne!(s.steering.personality & 0x40, 0);
            self.events.push("mark");
            if self.fail_mark {
                Err("actual marker mark failed".into())
            } else {
                Ok(())
            }
        }
        fn owner_sound(
            &mut self,
            s: &State<u32, u32>,
            cue: u32,
            _: &mut Random,
        ) -> Result<SoundOutcome, String> {
            assert_eq!((s.entry.mode, cue), (71, 1235));
            self.events.push("sound");
            if self.fail_sound {
                Err("actual retreat sound failed".into())
            } else {
                Ok(SoundOutcome::Suppressed)
            }
        }
        fn remove_effect(
            &mut self,
            _: &State<u32, u32>,
            _: u32,
            _: &mut Random,
        ) -> Result<(), String> {
            Err("unexpected retreat effect".into())
        }
    }
    #[test]
    fn variant_retreat_obeys_strict_animation_threshold_and_completion_order() {
        let actors = [candidate()];
        let frame = Frame::new(&actors, &[42]).unwrap();
        let threshold = 0.73333335_f32;
        for (seconds, wrapped, events, mode, personality) in [
            (threshold, false, vec![], 71, 0),
            (
                f32::from_bits(threshold.to_bits() + 1),
                false,
                vec!["mark"],
                71,
                0x40,
            ),
            (1., true, vec!["mark", "sound"], 61, 0x840),
            (0., true, vec!["sound"], 61, 0x840),
        ] {
            let mut s = state();
            s.entry.class = 230;
            s.entry.mode = 71;
            s.steering.personality = 0;
            s.timer = 5.25;
            s.primary = [
                43,
                seconds.to_bits(),
                0,
                0,
                0,
                0,
                0,
                0,
                if wrapped { 2 } else { 0 },
            ];
            let mut m = Machine::new(s);
            let mut r = Random(1);
            let mut audio = RetreatAudio::default();
            m.advance_variant_retreat(
                FrameInput {
                    context: context(),
                    frame: &frame,
                    range: 1000.,
                },
                &mut r,
                &mut audio,
            )
            .unwrap();
            assert_eq!(audio.events, events);
            assert_eq!(
                (m.state().entry.mode, m.state().steering.personality),
                (mode, personality)
            );
            assert_eq!(m.state().timer, 5.25);
            assert_eq!(r.0, 1);
        }
    }
    #[test]
    fn variant_retreat_latches_partial_mark_and_sound_failures_without_replay() {
        let actors = [candidate()];
        let frame = Frame::new(&actors, &[42]).unwrap();
        for fail_mark in [false, true] {
            let mut s = state();
            s.entry.class = 230;
            s.entry.mode = 71;
            s.steering.personality = 0;
            s.primary = [43, 1f32.to_bits(), 0, 0, 0, 0, 0, 0, 2];
            let mut m = Machine::new(s);
            let mut r = Random(1);
            let mut audio = RetreatAudio {
                fail_mark,
                fail_sound: !fail_mark,
                ..Default::default()
            };
            let input = FrameInput {
                context: context(),
                frame: &frame,
                range: 1000.,
            };
            assert!(
                m.advance_variant_retreat(input, &mut r, &mut audio)
                    .is_err()
            );
            assert_eq!(
                (m.state().entry.mode, m.state().steering.personality),
                (71, 0x40)
            );
            assert_eq!(r.0, 1);
            assert!(!m.awaiting_motion());
            let events = audio.events.clone();
            assert!(
                m.advance_variant_retreat(input, &mut r, &mut audio)
                    .is_err()
            );
            assert_eq!(audio.events, events);
        }
    }
}
pub trait Services<K, E> {
    /// Read this owner's actual relation and mark a class219 child mode2 when
    /// present. No unlink, callback completion or resource release is implied.
    fn mark_surface_marker_retirement(&mut self, _state: &State<K, E>) -> Result<(), String> {
        Err("actual surface marker retirement-mark provider unavailable".into())
    }
    /// Passive query trace only. This acknowledges no terrain service and
    /// supplies no result; the real ground provider still owns the sample.
    fn observe_ground(&mut self, _state: &State<K, E>) {}
    fn prepare_awakening_dirt(
        &mut self,
        _state: &State<K, E>,
        _random: &mut Random,
    ) -> Result<Option<E>, String> {
        Err("actual awakening dirt preparation unavailable".into())
    }
    fn surface_popup(
        &mut self,
        _state: &State<K, E>,
        _random: &mut Random,
    ) -> Result<crate::enemy_projectile::EffectOutcome, String> {
        Err("actual positional surfacing effect unavailable".into())
    }
    fn reset_surface_motion(&mut self, _state: &State<K, E>) -> Result<(), String> {
        Err("actual owned surfacing motion reset unavailable".into())
    }
    /// Must construct, copy pose and publish the reciprocal owned identities.
    fn construct_surface_marker(&mut self, _state: &State<K, E>) -> Result<K, String> {
        Err("actual owned surfacing marker unavailable".into())
    }
    fn ground_normal_z(&mut self, _state: &State<K, E>) -> Result<f32, String> {
        Err("actual primary terrain normal unavailable".into())
    }
    fn stop_owner_sound(
        &mut self,
        _state: &State<K, E>,
        _cue: u32,
        _random: &mut Random,
    ) -> Result<SoundOutcome, String> {
        Err("actual primary owner audio stop unavailable".into())
    }
    fn remove_effect(
        &mut self,
        state: &State<K, E>,
        effect: E,
        random: &mut Random,
    ) -> Result<(), String>;
    /// Submission, real resource absence and suppression are distinct. An
    /// unavailable backend is an error, not an implicit handled request.
    fn owner_sound(
        &mut self,
        state: &State<K, E>,
        cue: u32,
        random: &mut Random,
    ) -> Result<SoundOutcome, String>;
    /// A provider that has not implemented real projectile lookup/creation
    /// refuses the request. It never reports resource absence implicitly.
    fn fire(
        &mut self,
        _state: &State<K, E>,
        _target: [f32; 3],
        _vertical_spread_percent: f32,
        _random: &mut Random,
    ) -> Result<crate::state23_animation::FireOutcome, String> {
        Err("actual primary projectile service unavailable".into())
    }
    /// The source has already changed to mode66 when this kind3 request runs.
    /// No hitpoint/projectile is invented, and absent hurt ownership refuses.
    fn melee(
        &mut self,
        _state: &State<K, E>,
        _target: K,
        _amount: f32,
        _random: &mut Random,
    ) -> Result<(), String> {
        Err("actual Ripper kind3 melee hurt service unavailable".into())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Skipped,
    CompletedMode32,
    CompletedMode33,
    CompletedEmergence,
    CompletedSurfacing,
    CompletedAwakening,
    CompletedAttack,
    CompletedVariantAwakening,
    CompletedVariantTurn,
    CompletedVariantDecision,
    CompletedVariantPursuit,
    CompletedVariantAttack,
    CompletedVariantCooldown,
    CompletedVariantClose,
    CompletedVariantMelee,
    CompletedVariantRetreat,
}
#[derive(Clone, Debug)]
pub struct Machine<K, E> {
    state: State<K, E>,
    failed: Option<String>,
    completed: bool,
    surface_reset: bool,
    surface_marker: Option<K>,
}
fn overlay<K, E: Copy>(state: &mut State<K, E>, inner: &ripper_turn_completion::State<E>) {
    state.entry.mode = inner.mode;
    state.entry.flags = inner.steering.flags;
    state.entry.packed = u32::from_le_bytes(inner.steering.commands[..4].try_into().unwrap());
    state.steering = inner.steering;
    state.orbit = inner.orbit;
    state.timer = inner.timer;
    state.effect = inner.effect;
}
struct Adapter<'a, K, E, S> {
    state: &'a State<K, E>,
    services: &'a mut S,
}
fn overlay_awakening<K, E: Copy>(s: &mut State<K, E>, inner: &crate::ripper_awakening::State<E>) {
    s.entry.mode = inner.mode;
    s.parameter_184 = inner.visibility;
    s.effect = inner.effect;
    s.orbit = inner.orbit;
    s.timer = inner.timer;
}
impl<K: Copy, E: Copy, S: Services<K, E>> crate::ripper_awakening::Services<E>
    for Adapter<'_, K, E, S>
{
    fn ground_normal_z(
        &mut self,
        inner: &crate::ripper_awakening::State<E>,
    ) -> Result<f32, String> {
        let mut s = self.state.clone();
        overlay_awakening(&mut s, inner);
        self.services.ground_normal_z(&s)
    }
    fn prepare_dirt(
        &mut self,
        inner: &crate::ripper_awakening::State<E>,
        r: &mut Random,
    ) -> Result<Option<E>, String> {
        let mut s = self.state.clone();
        overlay_awakening(&mut s, inner);
        self.services.prepare_awakening_dirt(&s, r)
    }
    fn audio(
        &mut self,
        inner: &crate::ripper_awakening::State<E>,
        request: crate::ripper_awakening::Audio,
        r: &mut Random,
    ) -> Result<(), String> {
        let mut s = self.state.clone();
        overlay_awakening(&mut s, inner);
        match request {
            crate::ripper_awakening::Audio::Play(c) => self.services.owner_sound(&s, c, r),
            crate::ripper_awakening::Audio::Stop(c) => self.services.stop_owner_sound(&s, c, r),
        }
        .map(|_| ())
    }
}
fn overlay_pursuit<K: Copy, E>(state: &mut State<K, E>, inner: &crate::ripper_pursuit::State<K>) {
    state.entry.mode = inner.mode;
    state.entry.flags = inner.steering.flags;
    state.entry.control = inner.control;
    state.entry.packed = u32::from_le_bytes(inner.steering.commands[..4].try_into().unwrap());
    state.steering = inner.steering;
    state.retaliation = inner.retaliation;
}
impl<K: Copy + Ord, E: Copy, S: Services<K, E>> crate::ripper_pursuit::Services<K>
    for Adapter<'_, K, E, S>
{
    fn owner_sound(
        &mut self,
        inner: &crate::ripper_pursuit::State<K>,
        cue: u32,
        random: &mut Random,
    ) -> Result<SoundOutcome, String> {
        let mut snapshot = self.state.clone();
        overlay_pursuit(&mut snapshot, inner);
        self.services.owner_sound(&snapshot, cue, random)
    }
}
fn overlay_attack<K: Copy, E>(state: &mut State<K, E>, inner: &crate::ripper_attack::State<K>) {
    overlay_pursuit(state, &inner.actor);
    state.timer = inner.timer;
    state.orbit = inner.actor.orbit;
}
impl<K: Copy + Ord, E: Copy, S: Services<K, E>> crate::ripper_attack::Services<K>
    for Adapter<'_, K, E, S>
{
    fn owner_sound(
        &mut self,
        inner: &crate::ripper_attack::State<K>,
        cue: u32,
        random: &mut Random,
    ) -> Result<SoundOutcome, String> {
        let mut snapshot = self.state.clone();
        overlay_attack(&mut snapshot, inner);
        self.services.owner_sound(&snapshot, cue, random)
    }
    fn fire(
        &mut self,
        inner: &crate::ripper_attack::State<K>,
        target: [f32; 3],
        spread: f32,
        random: &mut Random,
    ) -> Result<crate::state23_animation::FireOutcome, String> {
        let mut snapshot = self.state.clone();
        overlay_attack(&mut snapshot, inner);
        self.services.fire(&snapshot, target, spread, random)
    }
}
fn overlay_emergence<K: Copy, E: Copy>(
    state: &mut State<K, E>,
    inner: &crate::ripper_emergence::State<K, E>,
) {
    overlay_pursuit(state, &inner.actor);
    state.effect = inner.effect;
}
impl<K: Copy, E: Copy, S: Services<K, E>> crate::ripper_emergence::Services<K, E>
    for Adapter<'_, K, E, S>
{
    fn remove_effect(
        &mut self,
        inner: &crate::ripper_emergence::State<K, E>,
        effect: E,
        random: &mut Random,
    ) -> Result<(), String> {
        let mut state = self.state.clone();
        overlay_emergence(&mut state, inner);
        self.services.remove_effect(&state, effect, random)
    }
    fn ground_normal_z(
        &mut self,
        inner: &crate::ripper_emergence::State<K, E>,
    ) -> Result<f32, String> {
        let mut state = self.state.clone();
        overlay_emergence(&mut state, inner);
        self.services.ground_normal_z(&state)
    }
    fn audio(
        &mut self,
        inner: &crate::ripper_emergence::State<K, E>,
        request: crate::ripper_awakening::Audio,
        random: &mut Random,
    ) -> Result<(), String> {
        let mut state = self.state.clone();
        overlay_emergence(&mut state, inner);
        match request {
            crate::ripper_awakening::Audio::Play(cue) => {
                self.services.owner_sound(&state, cue, random)
            }
            crate::ripper_awakening::Audio::Stop(cue) => {
                self.services.stop_owner_sound(&state, cue, random)
            }
        }
        .map(|_| ())
    }
}
#[derive(Clone, Copy)]
enum Branch<'a, K> {
    Awakening(AwakeningContext),
    VariantAwakening(VariantAwakeningContext<'a>),
    VariantTurn,
    VariantDecision(VariantDecisionContext),
    VariantPursuit(VariantDecisionContext),
    VariantAttack(VariantAttackContext<'a, K>),
    VariantCooldown(VariantCooldownContext<'a>),
    VariantClose(VariantCloseContext),
    VariantMelee(VariantMeleeContext),
    VariantRetreat,
    Turn(Parameters),
    Pursuit(f32),
    Emergence(f32),
    Surfacing(Parameters),
    Attack(AttackContext<'a, K>),
}
impl<K: Copy, E: Copy, S: Services<K, E>> ripper_turn_completion::Services<E>
    for Adapter<'_, K, E, S>
{
    fn remove_effect(
        &mut self,
        inner: &ripper_turn_completion::State<E>,
        effect: E,
        random: &mut Random,
    ) -> Result<(), String> {
        let mut snapshot = self.state.clone();
        overlay(&mut snapshot, inner);
        self.services.remove_effect(&snapshot, effect, random)
    }
}
fn footsteps(primary: [u32; 9]) -> Result<bool, &'static str> {
    if primary[0] != 2 {
        return Ok(false);
    }
    let now = f32::from_bits(primary[1]);
    let previous = f32::from_bits(primary[2]);
    if !now.is_finite() || !previous.is_finite() {
        return Err("primary sound clock outside finite profile");
    }
    Ok([f32::from_bits(0x3f222222), f32::from_bits(0x3ffbbbbc)]
        .into_iter()
        .any(|at| now >= at && previous < at))
}
impl<K: Copy + Ord, E: Copy> Machine<K, E> {
    pub fn advance_variant_melee(
        &mut self,
        input: FrameInput<'_, K>,
        parameters: VariantMeleeContext,
        random: &mut Random,
        services: &mut impl Services<K, E>,
    ) -> Result<Outcome, String> {
        if !parameters.attack_range.is_finite() || !parameters.damage.is_finite() {
            return Err("variant melee requires actual finite class-global parameters".into());
        }
        self.advance_inner(
            input.context,
            input.frame,
            input.range,
            Branch::VariantMelee(parameters),
            random,
            services,
        )
    }
    pub fn advance_variant_close(
        &mut self,
        input: FrameInput<'_, K>,
        parameters: VariantCloseContext,
        random: &mut Random,
        services: &mut impl Services<K, E>,
    ) -> Result<Outcome, String> {
        if !parameters.attack_range.is_finite() || !parameters.alternate_threshold.is_finite() {
            return Err("variant close requires actual finite class-global parameters".into());
        }
        self.advance_inner(
            input.context,
            input.frame,
            input.range,
            Branch::VariantClose(parameters),
            random,
            services,
        )
    }
    pub fn advance_variant_retreat(
        &mut self,
        input: FrameInput<'_, K>,
        random: &mut Random,
        services: &mut impl Services<K, E>,
    ) -> Result<Outcome, String> {
        if !f32::from_bits(self.state.primary[1]).is_finite() {
            return Err("variant retreat current animation clock is nonfinite".into());
        }
        self.advance_inner(
            input.context,
            input.frame,
            input.range,
            Branch::VariantRetreat,
            random,
            services,
        )
    }
    pub fn advance_variant_attack(
        &mut self,
        input: FrameInput<'_, K>,
        attack: VariantAttackContext<'_, K>,
        random: &mut Random,
        services: &mut impl Services<K, E>,
    ) -> Result<Outcome, String> {
        self.advance_inner(
            input.context,
            input.frame,
            input.range,
            Branch::VariantAttack(attack),
            random,
            services,
        )
    }
    pub fn advance_variant_pursuit(
        &mut self,
        input: FrameInput<'_, K>,
        parameters: VariantDecisionContext,
        random: &mut Random,
        services: &mut impl Services<K, E>,
    ) -> Result<Outcome, String> {
        if !parameters.fire_range.is_finite() || parameters.fire_range < 0. {
            return Err("variant pursuit requires actual finite fire range".into());
        }
        self.advance_inner(
            input.context,
            input.frame,
            input.range,
            Branch::VariantPursuit(parameters),
            random,
            services,
        )
    }
    pub fn advance_variant_decision(
        &mut self,
        input: FrameInput<'_, K>,
        parameters: VariantDecisionContext,
        random: &mut Random,
        services: &mut impl Services<K, E>,
    ) -> Result<Outcome, String> {
        if ![parameters.close_range, parameters.fire_range]
            .into_iter()
            .all(|v| v.is_finite() && v >= 0.)
        {
            return Err("variant decision requires actual finite class radii".into());
        }
        self.advance_inner(
            input.context,
            input.frame,
            input.range,
            Branch::VariantDecision(parameters),
            random,
            services,
        )
    }
    /// Class230 mode62 uses the current primary channel completion bit. This
    /// produces steering only; motion, channel advancement and mode63 remain
    /// separate consumers. No class73 speed restoration or dirt removal.
    pub fn advance_variant_turn(
        &mut self,
        input: FrameInput<'_, K>,
        random: &mut Random,
        services: &mut impl Services<K, E>,
    ) -> Result<Outcome, String> {
        self.advance_inner(
            input.context,
            input.frame,
            input.range,
            Branch::VariantTurn,
            random,
            services,
        )
    }
    /// Class230's ordinary mode0/61 producer. Mode62 and subsequent behavior
    /// require their own continuations; marker construction is a real service.
    pub fn advance_variant_awakening(
        &mut self,
        input: FrameInput<'_, K>,
        parameters: VariantAwakeningContext<'_>,
        random: &mut Random,
        services: &mut impl Services<K, E>,
    ) -> Result<Outcome, String> {
        if !parameters
            .camera
            .into_iter()
            .chain([
                parameters.camera_range,
                parameters.global_seconds,
                self.state.timer,
            ])
            .all(f32::is_finite)
            || parameters.camera_range < 0.
            || parameters.global_seconds < 0.
            || parameters.areas.iter().any(|a| {
                !a.center
                    .into_iter()
                    .chain([a.factor, a.radius_squared])
                    .all(f32::is_finite)
                    || a.factor < 0.
                    || a.radius_squared < 0.
            })
        {
            return Err("variant awakening requires finite camera/range/timer".into());
        }
        self.advance_inner(
            input.context,
            input.frame,
            input.range,
            Branch::VariantAwakening(parameters),
            random,
            services,
        )
    }
    pub fn advance_awakening(
        &mut self,
        context: Context,
        frame: &Frame<'_, K>,
        range: f32,
        awakening: AwakeningContext,
        random: &mut Random,
        services: &mut impl Services<K, E>,
    ) -> Result<Outcome, String> {
        if !awakening.buried_speed_percent.is_finite() {
            return Err("awakening speed parameter not finite".into());
        }
        self.advance_inner(
            context,
            frame,
            range,
            Branch::Awakening(awakening),
            random,
            services,
        )
    }
    pub fn new(state: State<K, E>) -> Self {
        Self {
            state,
            failed: None,
            completed: false,
            surface_reset: false,
            surface_marker: None,
        }
    }
    pub fn state(&self) -> &State<K, E> {
        &self.state
    }
    /// The owned world-list alert writes this relation between callbacks. It
    /// neither consumes a primary nor acknowledges motion or another phase.
    pub(crate) fn bind_alert_target(&mut self, target: K) {
        self.state.retaliation = Some(target);
    }
    pub fn failure(&self) -> Option<&str> {
        self.failed.as_deref()
    }
    pub fn surface_marker(&self) -> Option<K> {
        self.surface_marker
    }
    /// The common reference pass cleared the corresponding owned callback
    /// relation. This is bookkeeping for that acquired operation, not a death
    /// transition or an acknowledgement of actor/resource destruction.
    pub(crate) fn forget_retired_surface_marker(&mut self, marker: K) -> Result<(), String>
    where
        K: PartialEq,
    {
        if self.surface_marker.is_some_and(|current| current != marker) {
            return Err("surface marker alias belongs to another actor".into());
        }
        self.surface_marker = None;
        Ok(())
    }
    pub fn awaiting_motion(&self) -> bool {
        self.completed && self.failed.is_none()
    }
    pub(crate) fn refresh_owned_frame(&mut self, state: State<K, E>) -> Result<(), String> {
        if self.failed.is_some() || self.completed {
            return Err("owned frame requires an idle unfailed primary".into());
        }
        self.state = state;
        Ok(())
    }
    pub(crate) fn accept_owned_motion(
        &mut self,
        position: [f32; 3],
        heading: f32,
        flags: u32,
        personality: u32,
    ) {
        debug_assert!(self.awaiting_motion());
        self.state.entry.position = position;
        self.state.steering.position = position;
        self.state.steering.heading = heading;
        self.state.entry.flags = flags;
        self.state.steering.flags = flags;
        self.state.steering.personality = personality;
        self.completed = false;
        self.surface_reset = false;
    }
    pub fn advance_surfacing(
        &mut self,
        context: Context,
        frame: &Frame<'_, K>,
        range: f32,
        parameters: Parameters,
        random: &mut Random,
        services: &mut impl Services<K, E>,
    ) -> Result<Outcome, String> {
        self.advance_inner(
            context,
            frame,
            range,
            Branch::Surfacing(parameters),
            random,
            services,
        )
    }
    /// Actual reduced-speed parameter comes from the current global reader.
    pub fn advance_emergence(
        &mut self,
        context: Context,
        frame: &Frame<'_, K>,
        range: f32,
        buried_speed_percent: f32,
        random: &mut Random,
        services: &mut impl Services<K, E>,
    ) -> Result<Outcome, String> {
        if !buried_speed_percent.is_finite() {
            return Err("emergence speed parameter is not finite".into());
        }
        self.advance_inner(
            context,
            frame,
            range,
            Branch::Emergence(buried_speed_percent),
            random,
            services,
        )
    }
    /// Refresh this view from the actor's real animation producer between
    /// frames. This advances no clock and cannot rearm a failed/pending phase.
    pub fn publish_primary_channel(&mut self, channel: [u32; 9]) -> Result<(), String> {
        if self.failed.is_some() || self.completed {
            return Err("animation view requires an idle unfailed primary continuation".into());
        }
        self.state.primary = channel;
        Ok(())
    }
    pub fn advance(
        &mut self,
        context: Context,
        frame: &Frame<'_, K>,
        range: f32,
        parameters: Parameters,
        random: &mut Random,
        services: &mut impl Services<K, E>,
    ) -> Result<Outcome, String> {
        self.advance_inner(
            context,
            frame,
            range,
            Branch::Turn(parameters),
            random,
            services,
        )
    }
    /// Continue the same owned state after mode32 enters mode33. The class
    /// decision parameter is supplied by its real global-parameter reader.
    pub fn advance_pursuit(
        &mut self,
        context: Context,
        frame: &Frame<'_, K>,
        range: f32,
        attack_percent: f32,
        random: &mut Random,
        services: &mut impl Services<K, E>,
    ) -> Result<Outcome, String> {
        self.advance_inner(
            context,
            frame,
            range,
            Branch::Pursuit(attack_percent),
            random,
            services,
        )
    }
    pub fn advance_attack(
        &mut self,
        context: Context,
        frame: &Frame<'_, K>,
        range: f32,
        attack: AttackContext<'_, K>,
        random: &mut Random,
        services: &mut impl Services<K, E>,
    ) -> Result<Outcome, String> {
        self.advance_inner(
            context,
            frame,
            range,
            Branch::Attack(attack),
            random,
            services,
        )
    }
    fn advance_inner(
        &mut self,
        context: Context,
        frame: &Frame<'_, K>,
        range: f32,
        branch: Branch<'_, K>,
        random: &mut Random,
        services: &mut impl Services<K, E>,
    ) -> Result<Outcome, String> {
        if let Some(error) = &self.failed {
            return Err(format!("primary mode32 service already failed: {error}"));
        }
        if self.completed {
            return Err("completed primary awaits its common-motion consumer".into());
        }
        let original = &self.state;
        let mode = match branch {
            Branch::VariantAwakening(_) => {
                if !matches!(original.entry.mode, 0 | 61) {
                    return Err("variant awakening primary requires mode0/61".into());
                }
                original.entry.mode
            }
            Branch::VariantTurn => 62,
            Branch::VariantDecision(_) => 63,
            Branch::VariantPursuit(_) => 64,
            Branch::VariantAttack(_) => 65,
            Branch::VariantCooldown(_) => 66,
            Branch::VariantClose(_) => 67,
            Branch::VariantMelee(_) => 68,
            Branch::VariantRetreat => 71,
            Branch::Awakening(_) => {
                if !matches!(original.entry.mode, 0 | 31) {
                    return Err("awakening primary requires mode0/31".into());
                }
                original.entry.mode
            }
            Branch::Turn(_) => 32,
            Branch::Pursuit(_) => 33,
            Branch::Emergence(_) => 34,
            Branch::Surfacing(_) => 44,
            Branch::Attack(_) => {
                if !matches!(original.entry.mode, 37..=39) {
                    return Err("attack primary requires mode37/38/39".into());
                }
                original.entry.mode
            }
        };
        let class = if matches!(
            branch,
            Branch::VariantAwakening(_)
                | Branch::VariantTurn
                | Branch::VariantDecision(_)
                | Branch::VariantPursuit(_)
                | Branch::VariantAttack(_)
                | Branch::VariantCooldown(_)
                | Branch::VariantClose(_)
                | Branch::VariantMelee(_)
                | Branch::VariantRetreat
        ) {
            230
        } else {
            73
        };
        if original.entry.class != class
            || original.entry.mode != mode
            || original.entry.action != 0
        {
            return Err(
                "complete primary requires the matching acquired class/action0 continuation".into(),
            );
        }
        let Some(prefix) = state23_entry::ordinary_prefix(original.entry, context)? else {
            self.completed = true;
            return Ok(Outcome::Skipped);
        };
        if original.entry.flags != original.steering.flags
            || original.entry.position.map(f32::to_bits)
                != original.steering.position.map(f32::to_bits)
            || original.entry.packed
                != u32::from_le_bytes(original.steering.commands[..4].try_into().unwrap())
        {
            return Err("primary views disagree on owned fields".into());
        }
        // Pure preflight stages stay local until a service may have effects.
        let sound = footsteps(original.primary)?;
        let mut state = original.clone();
        state.entry = prefix;
        state.steering.flags = prefix.flags;
        state.steering.commands[..4].copy_from_slice(&prefix.packed.to_le_bytes());
        let initializing =
            matches!(branch, Branch::Awakening(_) | Branch::VariantAwakening(_)) && mode == 0;
        if initializing {
            let plan = state23_entry::plan(original.entry, context)?;
            state.entry = plan
                .before_sound
                .ok_or("initial entry sound snapshot absent")?;
            if let Err(error) =
                services.owner_sound(&state, if class == 73 { 1258 } else { 1235 }, random)
            {
                self.state = state;
                self.failed = Some(error.clone());
                return Err(error);
            }
            state.entry = plan.after;
        }
        let perception = (|| {
            if state
                .retaliation
                .map(|id| frame.candidate(id).map(|a| a.active & 0x8000 != 0))
                .transpose()?
                .unwrap_or(false)
            {
                state.retaliation = None;
            }
            if state.retaliation.is_none() {
                state.retaliation = frame.select(Query {
                    position: prefix.position,
                    retained_timer: prefix.retained_timer,
                    range,
                    previous: None,
                })?;
            }
            Ok::<_, String>(())
        })();
        if let Err(error) = perception {
            if initializing {
                self.state = state;
                self.failed = Some(error.clone());
            }
            return Err(error);
        }
        if let Branch::Surfacing(_) = branch {
            // These calls can mutate owned external objects. Preserve their
            // ordered partial state and forbid wholesale replay on any error.
            let result = (|| {
                services.surface_popup(&state, random)?;
                if let Some(effect) = state.effect {
                    services.remove_effect(&state, effect, random)?;
                }
                state.effect = None;
                services.owner_sound(&state, 1267, random)?;
                state.entry.mode = 32;
                state.parameter_184 = 0.;
                state.steering.personality &= !0x800;
                // 4041e0 is a RET on D; no invented ground projection.
                services.reset_surface_motion(&state)?;
                self.surface_reset = true;
                if state.entry.position[2] > 0. {
                    self.surface_marker = Some(services.construct_surface_marker(&state)?);
                }
                Ok::<_, String>(())
            })();
            if let Err(error) = result {
                self.state = state;
                self.failed = Some(error.clone());
                return Err(error);
            }
        }
        if let Branch::VariantAwakening(a) = branch {
            let delta = std::array::from_fn::<_, 3, _>(|i| {
                f64::from(a.camera[i]) - f64::from(state.entry.position[i])
            });
            let squared = (delta[0] * delta[0] + delta[1] * delta[1]) + delta[2] * delta[2];
            state.parameter_184 = if squared > f64::from(a.camera_range) * f64::from(a.camera_range)
            {
                1.
            } else {
                0.
            };
            // Entry can replace the dispatch context before this second time
            // query. Do not reuse the prefix's earlier object-time sample.
            let seconds = crate::object_time::seconds(
                a.global_seconds,
                Some(crate::object_time::Sample {
                    owner: state.entry.context,
                    position: state.entry.position,
                }),
                None,
                a.areas,
            );
            let remaining = f64::from(state.timer) - seconds;
            state.timer = remaining as f32;
            if remaining < 0. && state.retaliation.is_some() {
                let result = (|| {
                    services.owner_sound(&state, 1246, random)?;
                    state.entry.mode = 62;
                    state.parameter_184 = 0.;
                    state.steering.personality &= !0x800;
                    // The debug history-tail hook is empty. The real marker
                    // constructor/copy/link does not write the source motion.
                    self.surface_marker = Some(services.construct_surface_marker(&state)?);
                    services.stop_owner_sound(&state, 1235, random)?;
                    Ok::<_, String>(())
                })();
                if let Err(error) = result {
                    self.state = state;
                    self.failed = Some(error.clone());
                    return Err(error);
                }
            }
        } else if let Branch::VariantRetreat = branch {
            if f32::from_bits(state.primary[1]) > 0.73333335_f32 {
                state.steering.personality |= 0x40;
                if let Err(error) = services.mark_surface_marker_retirement(&state) {
                    self.state = state;
                    self.failed = Some(error.clone());
                    return Err(error);
                }
            }
            if state.primary[8] & 2 != 0 {
                // The PC debug profile's ground-projection entry is a RET,
                // as in the acquired class73 surfacing path. No guessed pose.
                if let Err(error) = services.owner_sound(&state, 1235, random) {
                    self.state = state;
                    self.failed = Some(error.clone());
                    return Err(error);
                }
                state.entry.mode = 61;
                state.steering.personality |= 0x840;
            }
        } else if let Branch::VariantCooldown(parameters) = branch {
            let plan = cooldown::prepare(&state, frame, parameters)?;
            state.timer = plan.remaining;
            if plan.expired {
                if plan.close {
                    state.entry.mode = 67;
                } else {
                    if let Err(error) = services.owner_sound(&state, 1248, random) {
                        self.state = state;
                        self.failed = Some(error.clone());
                        return Err(error);
                    }
                    state.entry.mode = 71;
                    // The same acquired PC interval helper already serves the
                    // Vimp response. Retain its historical API and wide range
                    // arithmetic; the actual EXE RNG is supplied by this owner.
                    match crate::vimp_response::random_delay(
                        random,
                        parameters.retreat_timer[0],
                        parameters.retreat_timer[1],
                    ) {
                        Ok(timer) => state.timer = timer,
                        Err(error) => {
                            self.state = state;
                            self.failed = Some(error.clone());
                            return Err(error);
                        }
                    }
                }
            }
        } else if let Branch::VariantDecision(parameters) | Branch::VariantPursuit(parameters) =
            branch
        {
            let target = state
                .retaliation
                .map(|id| frame.candidate(id))
                .transpose()?;
            let (lost, control) = crate::ripper_pursuit::target_control(
                target,
                state.entry.retained_timer,
                state.entry.control,
            )?;
            state.entry.control = control;
            if lost {
                if let Err(error) = services.owner_sound(&state, 1248, random) {
                    self.state = state;
                    self.failed = Some(error.clone());
                    return Err(error);
                }
                state.entry.mode = 71;
                state.retaliation = None;
            } else {
                let target = target.unwrap();
                let dx = f64::from(target.position[0]) - f64::from(state.entry.position[0]);
                let dy = f64::from(target.position[1]) - f64::from(state.entry.position[1]);
                let distance = (dx * dx + dy * dy) as f32;
                if !distance.is_finite() {
                    return Err("variant decision distance overflow".into());
                }
                if matches!(branch, Branch::VariantPursuit(_)) {
                    state.steering =
                        crate::ripper_steering::chase_location(state.steering, target.position)?;
                    state.entry.flags = state.steering.flags;
                    state.entry.packed =
                        u32::from_le_bytes(state.steering.commands[..4].try_into().unwrap());
                    state.entry.mode =
                        if f64::from(distance) < f64::from(parameters.fire_range).powi(2) {
                            65
                        } else {
                            64
                        };
                } else {
                    state.entry.mode =
                        if f64::from(distance) < f64::from(parameters.close_range).powi(2) {
                            67
                        } else if f64::from(distance) < f64::from(parameters.fire_range).powi(2) {
                            65
                        } else {
                            64
                        };
                }
            }
        } else if let Branch::VariantClose(parameters) = branch {
            let target = state
                .retaliation
                .map(|id| frame.candidate(id))
                .transpose()?;
            let (lost, control) = crate::ripper_pursuit::target_control(
                target,
                state.entry.retained_timer,
                state.entry.control,
            )?;
            state.entry.control = control;
            if lost {
                if let Err(error) = services.owner_sound(&state, 1248, random) {
                    self.state = state;
                    self.failed = Some(error.clone());
                    return Err(error);
                }
                state.entry.mode = 71;
                state.retaliation = None;
            } else {
                let target = target.unwrap();
                let dx = f64::from(target.position[0]) - f64::from(state.entry.position[0]);
                let dy = f64::from(target.position[1]) - f64::from(state.entry.position[1]);
                let distance = (dx * dx + dy * dy) as f32;
                if !distance.is_finite() {
                    return Err("variant close distance overflow".into());
                }
                state.steering =
                    crate::ripper_steering::close_chase(state.steering, target.position)?;
                state.entry.flags = state.steering.flags;
                state.entry.packed =
                    u32::from_le_bytes(state.steering.commands[..4].try_into().unwrap());
                if f64::from(distance) < f64::from(parameters.attack_range).powi(2) {
                    let roll = (random.next_15() * 100) >> 15;
                    if parameters.alternate_threshold <= roll as f32 {
                        if let Err(error) = services.owner_sound(&state, 1231, random) {
                            self.state = state;
                            self.failed = Some(error.clone());
                            return Err(error);
                        }
                        state.entry.mode = 68;
                    } else {
                        state.entry.mode = 69;
                    }
                }
            }
        } else if let Branch::VariantMelee(parameters) = branch {
            let target = state
                .retaliation
                .map(|id| frame.candidate(id))
                .transpose()?;
            let (lost, control) = crate::ripper_pursuit::target_control(
                target,
                state.entry.retained_timer,
                state.entry.control,
            )?;
            state.entry.control = control;
            if lost {
                if let Err(error) = services.owner_sound(&state, 1248, random) {
                    self.state = state;
                    self.failed = Some(error.clone());
                    return Err(error);
                }
                state.entry.mode = 71;
                state.retaliation = None;
            } else {
                let target = target.unwrap();
                state.steering =
                    crate::ripper_steering::face_contact_target(state.steering, target.position)?;
                state.entry.flags = state.steering.flags;
                state.entry.packed =
                    u32::from_le_bytes(state.steering.commands[..4].try_into().unwrap());
                if state.primary[8] & 2 != 0 {
                    let delta = std::array::from_fn::<_, 3, _>(|i| {
                        f64::from(target.position[i]) - f64::from(state.entry.position[i])
                    });
                    let distance =
                        ((delta[0] * delta[0] + delta[1] * delta[1]) + delta[2] * delta[2]) as f32;
                    if !distance.is_finite() {
                        return Err("variant melee distance overflow".into());
                    }
                    state.entry.mode = 66;
                    if f64::from(distance) < f64::from(parameters.attack_range).powi(2)
                        && let Err(error) =
                            services.melee(&state, target.id, parameters.damage, random)
                    {
                        self.state = state;
                        self.failed = Some(error.clone());
                        return Err(error);
                    }
                }
            }
        } else if let Branch::VariantTurn = branch {
            let target = state
                .retaliation
                .map(|id| frame.candidate(id).map(|c| c.position))
                .transpose()?
                .unwrap_or([99999.; 3]);
            // This branch retains the angular difference wide until the
            // command input store, unlike class73's mode32 helper.
            state.steering = crate::ripper_steering::aim_at_point(state.steering, target)?;
            state.steering.personality &= !0x40;
            state.entry.flags = state.steering.flags;
            state.entry.packed =
                u32::from_le_bytes(state.steering.commands[..4].try_into().unwrap());
            if state.primary[8] & 2 != 0 {
                state.entry.mode = 63;
            }
        } else if let Branch::Awakening(a) = branch {
            let target = state
                .retaliation
                .map(|id| frame.candidate(id).map(|c| c.position))
                .transpose()?;
            let mut inner = crate::ripper_awakening::Machine::new(crate::ripper_awakening::State {
                mode: state.entry.mode,
                position: state.entry.position,
                visibility: state.parameter_184,
                target,
                effect: state.effect,
                orbit: state.orbit,
                timer: state.timer,
            });
            let result = inner.advance(
                a.camera,
                a.parameters,
                random,
                &mut Adapter {
                    state: &state,
                    services,
                },
            );
            if let Err(error) = result {
                if initializing || inner.failure().is_some() {
                    overlay_awakening(&mut state, inner.state());
                    self.state = state;
                    self.failed = Some(error.clone());
                }
                return Err(error);
            }
            overlay_awakening(&mut state, inner.state());
        } else if matches!(branch, Branch::Attack(_) | Branch::VariantAttack(_)) {
            let shapes = match branch {
                Branch::Attack(a) => a.target_shapes,
                Branch::VariantAttack(a) => a.target_shapes,
                _ => unreachable!(),
            };
            let target = state
                .retaliation
                .map(|id| {
                    let shape = shapes
                        .get(&id)
                        .ok_or("attack target shape absent from current frame")?;
                    Ok::<_, String>(crate::ripper_attack::Target {
                        actor: *frame.candidate(id)?,
                        scale: shape.scale,
                        center_offset: shape.center_offset,
                    })
                })
                .transpose()?;
            let mut inner = crate::ripper_attack::Machine::new(crate::ripper_attack::State {
                actor: crate::ripper_pursuit::State {
                    class: state.entry.class,
                    mode: state.entry.mode,
                    steering: state.steering,
                    control: state.entry.control,
                    retained_timer: state.entry.retained_timer,
                    orbit: state.orbit,
                    retaliation: state.retaliation,
                },
                timer: state.timer,
            });
            let mut adapter = Adapter {
                state: &state,
                services,
            };
            let result = match branch {
                Branch::Attack(a) => {
                    inner.advance(target, state.primary, a.parameters, random, &mut adapter)
                }
                Branch::VariantAttack(a) => {
                    inner.advance_variant(target, state.primary, a.parameters, random, &mut adapter)
                }
                _ => unreachable!(),
            };
            if let Err(error) = result {
                if inner.failure().is_some() {
                    overlay_attack(&mut state, inner.state());
                    self.state = state;
                    self.failed = Some(error.clone());
                }
                return Err(error);
            }
            overlay_attack(&mut state, inner.state());
        } else if let Branch::Emergence(_) = branch {
            let mut inner = crate::ripper_emergence::Machine::new(crate::ripper_emergence::State {
                actor: crate::ripper_pursuit::State {
                    class: state.entry.class,
                    mode: state.entry.mode,
                    steering: state.steering,
                    control: state.entry.control,
                    retained_timer: state.entry.retained_timer,
                    orbit: state.orbit,
                    retaliation: state.retaliation,
                },
                effect: state.effect,
            });
            let result = inner.advance(
                frame,
                random,
                &mut Adapter {
                    state: &state,
                    services,
                },
            );
            if let Err(error) = result {
                if inner.failure().is_some() {
                    overlay_emergence(&mut state, inner.state());
                    self.state = state;
                    self.failed = Some(error.clone());
                }
                return Err(error);
            }
            overlay_emergence(&mut state, inner.state());
        } else if let Branch::Pursuit(percent) = branch {
            let mut inner = crate::ripper_pursuit::Machine::new(crate::ripper_pursuit::State {
                class: state.entry.class,
                mode: state.entry.mode,
                steering: state.steering,
                control: state.entry.control,
                retained_timer: state.entry.retained_timer,
                orbit: state.orbit,
                retaliation: state.retaliation,
            });
            let result = inner.advance(
                frame,
                percent,
                random,
                &mut Adapter {
                    state: &state,
                    services,
                },
            );
            if let Err(error) = result {
                if inner.failure().is_some() {
                    overlay_pursuit(&mut state, inner.state());
                    self.state = state;
                    self.failed = Some(error.clone());
                }
                return Err(error);
            }
            overlay_pursuit(&mut state, inner.state());
        } else if let Branch::Turn(parameters) | Branch::Surfacing(parameters) = branch {
            let target = state
                .retaliation
                .map(|id| frame.candidate(id).map(|a| a.position))
                .transpose()?;
            let mut branch = ripper_turn_completion::Machine::new(ripper_turn_completion::State {
                class: 73,
                mode: 32,
                steering: state.steering,
                orbit: state.orbit,
                timer: state.timer,
                effect: state.effect,
                target,
            });
            let result = branch.advance(
                state.primary[8],
                parameters,
                random,
                &mut Adapter {
                    state: &state,
                    services,
                },
            );
            if let Err(error) = result {
                if mode == 44 || branch.failure().is_some() {
                    overlay(&mut state, branch.state());
                    self.state = state;
                    self.failed = Some(error.clone());
                }
                return Err(error);
            }
            overlay(&mut state, branch.state());
        }
        if sound
            && let Err(error) =
                services.owner_sound(&state, if class == 73 { 1261 } else { 1238 }, random)
        {
            self.state = state;
            self.failed = Some(error.clone());
            return Err(error);
        }
        if class == 73 {
            state.entry.parameter = state.entry.saved_parameter;
        }
        let reduced = match branch {
            Branch::Emergence(p) => Some(p),
            Branch::Awakening(a) => Some(a.buried_speed_percent),
            _ => None,
        };
        if let Some(percent) = reduced
            && matches!(state.entry.mode, 31 | 34 | 36)
        {
            state.entry.parameter = (f64::from(percent)
                * f64::from(state.entry.saved_parameter)
                * f64::from(0.01_f32)) as f32;
        }
        self.state = state;
        self.completed = true;
        Ok(if matches!(branch, Branch::VariantAwakening(_)) {
            Outcome::CompletedVariantAwakening
        } else if matches!(branch, Branch::VariantTurn) {
            Outcome::CompletedVariantTurn
        } else if matches!(branch, Branch::VariantDecision(_)) {
            Outcome::CompletedVariantDecision
        } else if matches!(branch, Branch::VariantPursuit(_)) {
            Outcome::CompletedVariantPursuit
        } else if matches!(branch, Branch::VariantAttack(_)) {
            Outcome::CompletedVariantAttack
        } else if matches!(branch, Branch::VariantCooldown(_)) {
            Outcome::CompletedVariantCooldown
        } else if matches!(branch, Branch::VariantClose(_)) {
            Outcome::CompletedVariantClose
        } else if matches!(branch, Branch::VariantMelee(_)) {
            Outcome::CompletedVariantMelee
        } else if matches!(branch, Branch::VariantRetreat) {
            Outcome::CompletedVariantRetreat
        } else if matches!(branch, Branch::Awakening(_)) {
            Outcome::CompletedAwakening
        } else if mode == 32 {
            Outcome::CompletedMode32
        } else if mode == 33 {
            Outcome::CompletedMode33
        } else if mode == 34 {
            Outcome::CompletedEmergence
        } else if mode == 44 {
            Outcome::CompletedSurfacing
        } else {
            Outcome::CompletedAttack
        })
    }
    /// Consume this callback's actual commands/targets/parameter restoration.
    /// Incoming rates/look history and the remaining class/time environment
    /// are real upstream inputs; no native command words are reused here.
    pub fn advance_motion(
        &mut self,
        mut incoming: crate::actor_common_motion::State,
        environment: &crate::actor_common_motion::Environment<'_>,
    ) -> Result<crate::actor_common_motion::State, String> {
        if self.failed.is_some() || !self.completed {
            return Err("common motion requires a completed unconsumed primary".into());
        }
        let state = &self.state;
        if self.surface_reset {
            incoming.rates = [0.; 4];
            incoming.previous_rates = [0.; 4];
        }
        incoming.position = state.entry.position;
        incoming.heading = state.steering.heading;
        incoming.active = state.entry.flags;
        incoming.personality = state.steering.personality;
        incoming.state = 23;
        incoming.context = state.entry.context;
        incoming.controls = std::array::from_fn(|i| state.steering.commands[i] as i8);
        incoming.target_heading = state.steering.target_heading;
        incoming.target_position = state.steering.target_position[..2].try_into().unwrap();
        let environment = crate::actor_common_motion::Environment {
            parameters: crate::actor_common_motion::Parameters {
                speed: state.entry.parameter,
                ..environment.parameters
            },
            global_seconds: environment.global_seconds,
            areas: environment.areas,
            control_factor: environment.control_factor,
            running_factor: environment.running_factor,
            skip_heading: environment.skip_heading,
            controlled_actor: environment.controlled_actor,
        };
        let result = crate::actor_common_motion::step(incoming, &environment)?;
        self.state.entry.position = result.position;
        self.state.steering.position = result.position;
        self.state.steering.heading = result.heading;
        self.state.entry.flags = result.active;
        self.state.steering.flags = result.active;
        self.state.steering.personality = result.personality;
        self.completed = false;
        self.surface_reset = false;
        Ok(result)
    }
}
