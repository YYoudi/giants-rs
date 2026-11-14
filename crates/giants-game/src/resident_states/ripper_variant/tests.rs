//! Installed constructors with supplied flat terrain, eligibility, clocks and
//! captured/suppressed audio. No ordinary player/world admission is established.
use super::*;
use crate::{actor_fields::Fields, arithmetic::Precision, scene_resources::SceneResources};

#[derive(Default)]
struct Audio {
    calls: Vec<(bool, u32, u32)>,
    fail_play: Option<u32>,
    fail_stop: bool,
}
impl Services<Id, Effect> for Audio {
    fn owner_sound(
        &mut self,
        state: &State<Id, Effect>,
        cue: u32,
        _: &mut Random,
    ) -> Result<SoundOutcome, String> {
        self.calls.push((true, cue, state.entry.mode));
        if self.fail_play == Some(cue) {
            Err("supplied play failure".into())
        } else {
            Ok(SoundOutcome::Suppressed)
        }
    }
    fn stop_owner_sound(
        &mut self,
        state: &State<Id, Effect>,
        cue: u32,
        _: &mut Random,
    ) -> Result<SoundOutcome, String> {
        self.calls.push((false, cue, state.entry.mode));
        if self.fail_stop {
            Err("supplied stop failure".into())
        } else {
            Ok(SoundOutcome::Suppressed)
        }
    }
    fn remove_effect(
        &mut self,
        _: &State<Id, Effect>,
        _: Effect,
        _: &mut Random,
    ) -> Result<(), String> {
        Err("variant awakening must not retire dirt".into())
    }
}

struct ShotServices {
    effects: crate::attached_effect_context::Pool<Id>,
    quality: f32,
    lite_distance: f32,
    prepared: Option<Id>,
    cues: Vec<u32>,
    fail_sound: bool,
}
impl crate::primary_enemy_fire::Dependencies<Effect> for ShotServices {
    fn prepare_effect(
        &mut self,
        session: &mut Session,
        request: crate::enemy_projectile::EffectRequest<'_>,
        random: &mut Random,
    ) -> Result<crate::enemy_projectile::EffectOutcome, String> {
        assert_eq!(request.name, "fx_ripper_large_proj");
        assert_eq!(request.flags, 4);
        let fields = session
            .get(request.recipient)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap();
        assert_eq!((fields.class, fields.action), (232, 1));
        let (creation, _) = self.effects.prepare_extended(
            request.name,
            request.recipient,
            fields.object_flags,
            request.tag,
            crate::effect_initialization::Selection {
                active_objects: session.counts().residents as u32,
                quality: self.quality,
                lite_distance: self.lite_distance,
                camera: [0.; 3],
                recipient_position: fields.position,
            },
            random,
        )?;
        self.prepared = Some(request.recipient);
        Ok(match creation {
            crate::attached_effect_context::Creation::Prepared(_) => {
                crate::enemy_projectile::EffectOutcome::Prepared
            }
            crate::attached_effect_context::Creation::Absent => {
                crate::enemy_projectile::EffectOutcome::Rejected
            }
        })
    }
    fn positional_sound(
        &mut self,
        _: &mut Session,
        cue: u32,
        _: [f32; 3],
    ) -> Result<SoundOutcome, String> {
        assert!(matches!(cue, 1227 | 1228));
        self.cues.push(cue);
        if self.fail_sound {
            Err("supplied sound transport failure".into())
        } else {
            Ok(SoundOutcome::Suppressed)
        }
    }
}

#[test]
#[ignore = "Requires GIANTS_NPC_ROOT; supplied mode65/channel/flat terrain, real projectile/FX/reference owners, suppressed audio, no gameplay"]
fn source_variant_primary_launch_owns_projectile_and_latches_late_failure() {
    let root = std::path::PathBuf::from(std::env::var_os("GIANTS_NPC_ROOT").unwrap());
    let resources = SceneResources::load(&root, "w_Story2.bin", "English").unwrap();
    let globals =
        giants_formats::global_data::parse(&std::fs::read(root.join("Bin/GData.gbn")).unwrap())
            .unwrap();
    let image = Arc::new(
        giants_formats::effect_image::parse(
            &std::fs::read(root.join("Bin/fx_world_dust.bin")).unwrap(),
        )
        .unwrap(),
    );
    let parameters = crate::ripper_attack::VariantParameters {
        close_range: globals.scalar(0x3d8).unwrap(),
        reconsider_range: globals.scalar(0x104).unwrap(),
        charge_percent: globals.scalar(0x108).unwrap(),
        cooldown: globals.scalar(0x10c).unwrap(),
        vertical_spread_percent: globals.scalar(0x11c).unwrap(),
    };
    for (ticket, fail_sound) in [(9301, false), (9302, true)] {
        let (mut session, id, peer) = source(&resources, ticket, true);
        // Explicit diagnostic entry into the verified attack branch. This is
        // not an ordinary producer of mode65, its clip or target acquisition.
        let mut combat = session.state23_snapshot(id).unwrap();
        combat.mode = 65;
        combat.retaliation = Some(peer);
        let actor = session.residents.get_mut(&id.serial).unwrap();
        actor.state23_combat = Some(combat);
        actor.channels.as_mut().unwrap()[0] = [50, 0x3edd_ddde, 0x3edd_dddd, 0, 0, 0, 0, 0, 0];
        let subset = session.perception_subset(&[id, peer]).unwrap();
        let frame = subset.frame().unwrap();
        let input = FrameInput {
            context: crate::state23_entry::Context {
                paused: false,
                native_mode: 0,
                parent_state: None,
                player: None,
                delta: f64::from(1f32 / 60.),
            },
            frame: &frame,
            range: globals.scalar(0xfc).unwrap(),
        };
        let mut services = ShotServices {
            effects: crate::attached_effect_context::Pool::new(ticket, Some(image.clone())),
            quality: globals.scalar(0x13e0).unwrap(),
            lite_distance: globals.scalar(0x1570).unwrap(),
            prepared: None,
            cues: vec![],
            fail_sound,
        };
        let mut factory = crate::primary_enemy_fire::Factory::new(&resources);
        let mut random = Random(17);
        let result = session.update_ripper_variant_attack(
            id,
            input,
            parameters,
            &mut factory,
            &mut random,
            &mut services,
        );
        assert_eq!(result.is_err(), fail_sound, "{result:?}");
        let shot = services
            .prepared
            .expect("real projectile must reach effect preparation");
        assert_eq!(session.get(shot).unwrap().class, 232);
        assert_eq!(
            session.enemy_projectile(shot).unwrap().phase,
            if fail_sound {
                crate::enemy_projectile::Phase::Failed
            } else {
                crate::enemy_projectile::Phase::Launched
            }
        );
        assert_eq!(services.effects.active().len(), 1);
        assert_eq!(services.cues.len(), 1);
        let machine = session.ripper_primary(id).unwrap();
        assert_eq!(machine.state().entry.mode, 65);
        assert_eq!(machine.failure().is_some(), fail_sound);
        if !fail_sound {
            let launch = session
                .enemy_projectile(shot)
                .unwrap()
                .launch
                .as_ref()
                .unwrap();
            assert_eq!(launch.source, id);
            assert!(launch.calculation.lifetime > 0.);
        }
        let count = session.counts();
        let seed = random.0;
        assert!(
            session
                .update_ripper_variant_attack(
                    id,
                    input,
                    parameters,
                    &mut factory,
                    &mut random,
                    &mut services
                )
                .is_err()
        );
        assert_eq!(session.counts(), count);
        assert_eq!(random.0, seed);
        assert_eq!(services.cues.len(), 1);
        session.clear();
        assert!(session.get(shot).is_none());
    }
}

fn source(resources: &SceneResources, ticket: u64, eligible: bool) -> (Session, Id, Id) {
    source_for_class(resources, ticket, eligible, 230)
}
fn source_for_class(
    resources: &SceneResources,
    ticket: u64,
    eligible: bool,
    owner_class: i32,
) -> (Session, Id, Id) {
    let flat = giants_formats::gti::Terrain {
        source_flags: 0,
        width: 5,
        height: 5,
        origin: [-2000.; 2],
        spacing: 1000.,
        height_bounds: [0.; 2],
        fallback_texture: String::new(),
        heights: vec![0.; 25],
        colors: vec![0; 25],
        tags: vec![5; 25],
        consumed_bytes: 0,
        opaque_tail: vec![],
    };
    let terrain =
        crate::terrain_queries::StaticTerrainQuery::new(&flat, 0.001, Precision::Double53).unwrap();
    let mut session = Session::new(ticket);
    let mut ids = Vec::new();
    for (i, class) in [owner_class, if owner_class == 230 { 73 } else { 230 }]
        .into_iter()
        .enumerate()
    {
        let program = resources
            .classes
            .models
            .first_program(class)
            .unwrap()
            .program
            .as_ref()
            .unwrap();
        let seed = Seed::from_program_for(program, Profile::Debug, Precision::Double53).unwrap();
        let bindings = resources
            .class_state_bindings_for(class, Profile::Debug)
            .unwrap();
        let flags = DirectFlags::project(class, program);
        let specification =
            Arc::new(crate::specification::Prepared::load_registered(resources, class).unwrap());
        let state_resources = Arc::new(
            crate::state_resources::Prepared::load(resources, class, Profile::Debug).unwrap(),
        );
        let fields = Fields::load(resources, class, &specification)
            .unwrap()
            .placed(crate::collider_init::DeclaredPose {
                class: class as u32,
                position: if i == 0 { [0.; 3] } else { [80., 0., 0.] },
                yaw: 0.,
                pitch_roll: None,
            })
            .unwrap();
        let id = session
            .construct(Construction {
                class,
                seed: &seed,
                bindings: &bindings,
                flags,
                specification,
                state_resources,
                fields,
            })
            .unwrap();
        session.prepare_actor_affine(id).unwrap();
        session
            .prepare_actor_environment(id, resources, &terrain)
            .unwrap();
        ids.push(id);
    }
    if eligible {
        // Explicit diagnostic input, matching the native peer control. This
        // does not acquire an ordinary writer of the eligibility flag.
        session
            .residents
            .get_mut(&ids[1].serial)
            .unwrap()
            .construction_fields
            .as_mut()
            .unwrap()
            .configuration[14] |= 1;
    }
    let id = ids[0];
    session.prepare_actor_finalization(id).unwrap();
    session.prepare_actor_motion(id).unwrap();
    session.begin_actor_frame_history(id).unwrap();
    assert!(session.begin_actor_motion_history(id, [0; 8]).unwrap());
    session.prepare_ripper_primary(id).unwrap();
    (session, id, ids[1])
}

#[test]
#[ignore = "Requires GIANTS_NPC_ROOT; explicit mode71/clock fixtures with actual linked marker constructors, not natural retreat or graphics"]
fn source_variant_retreat_marks_owned_child_without_acknowledging_cleanup() {
    let root = std::path::PathBuf::from(std::env::var_os("GIANTS_NPC_ROOT").unwrap());
    let resources = SceneResources::load(&root, "w_Story2.bin", "English").unwrap();
    for case in 0..4 {
        let (mut session, id, peer) = source(&resources, 740 + case, true);
        let marker = if case < 2 {
            let marker = crate::primary_surface_marker::Factory::new(&resources)
                .construct(&mut session, id)
                .unwrap();
            session.prepare_surface_marker_world(marker).unwrap();
            Some(marker)
        } else {
            None
        };
        let mut combat = session.state23_snapshot(id).unwrap();
        combat.mode = 71;
        combat.timer = 5.25;
        if case == 3 {
            combat.owner = Some((peer, 73));
        }
        let actor = session.residents.get_mut(&id.serial).unwrap();
        actor.state23_combat = Some(combat);
        actor.channels.as_mut().unwrap()[0] = [43, 1f32.to_bits(), 0, 0, 0, 0, 0, 0, 2];
        // Calling outside the real parent callback must not mark a child.
        assert!(session.mark_ripper_surface_marker_retirement(id).is_err());
        let count = session.counts();
        let before_peer = session.state23_snapshot(peer).unwrap().mode;
        let marker_channels = marker.map(|m| session.get(m).unwrap().channels);
        let subset = session.perception_subset(&[id, peer]).unwrap();
        let frame = subset.frame().unwrap();
        let input = FrameInput {
            context: crate::state23_entry::Context {
                paused: false,
                native_mode: 0,
                parent_state: None,
                player: None,
                delta: 0.016,
            },
            frame: &frame,
            range: 10000.,
        };
        let mut random = Random(1);
        let mut audio = Audio {
            fail_play: if case == 1 { Some(1235) } else { None },
            ..Default::default()
        };
        let result = session.update_ripper_variant_retreat(id, input, &mut random, &mut audio);
        assert_eq!(result.is_err(), case == 1);
        assert_eq!(random.0, 1);
        assert_eq!(session.counts(), count);
        assert_eq!(
            session.state23_combat(id).unwrap().mode,
            if case == 1 { 71 } else { 61 }
        );
        assert_eq!(audio.calls, [(true, 1235, 71)]);
        assert_eq!(session.state23_snapshot(peer).unwrap().mode, before_peer);
        if let Some(marker) = marker {
            assert!(
                matches!(session.surface_link(marker),Some(SurfaceLink::Marker{owner,mode:2,..}) if owner==id)
            );
            assert_eq!(
                session.get(marker).unwrap().channels,
                marker_channels.unwrap()
            );
            assert_eq!(
                session
                    .get(marker)
                    .unwrap()
                    .construction_fields
                    .as_ref()
                    .unwrap()
                    .object_flags
                    & 0x8000,
                0
            );
            assert_eq!(session.surface_marker_frame(marker).unwrap().next_frame, 0);
            assert!(
                session
                    .incoming_actor_references(marker)
                    .unwrap()
                    .contains(&id)
            );
        }
        let calls = audio.calls.clone();
        assert!(
            session
                .update_ripper_variant_retreat(id, input, &mut random, &mut audio)
                .is_err()
        );
        assert_eq!(audio.calls, calls);
    }
}

#[test]
#[ignore = "Requires GIANTS_NPC_ROOT; supplied mode43/finished channel and actual source marker constructor, not a natural encounter or completed cleanup"]
fn source_classic_retreat_marks_real_child_and_restores_buried_parameter() {
    let root = std::path::PathBuf::from(std::env::var_os("GIANTS_NPC_ROOT").unwrap());
    let resources = SceneResources::load(&root, "w_Story2.bin", "English").unwrap();
    for fail in [false, true] {
        let (mut session, id, peer) =
            source_for_class(&resources, 1900 + u64::from(fail), false, 73);
        let marker = crate::primary_surface_marker::Factory::new(&resources)
            .construct(&mut session, id)
            .unwrap();
        session.prepare_surface_marker_world(marker).unwrap();
        let mut combat = session.state23_snapshot(id).unwrap();
        combat.mode = 43;
        let actor = session.residents.get_mut(&id.serial).unwrap();
        actor.state23_combat = Some(combat);
        actor.channels.as_mut().unwrap()[0] = [43, 1f32.to_bits(), 0, 0, 0, 0, 0, 0, 2];
        assert!(session.mark_ripper_surface_marker_retirement(id).is_err());
        let count = session.counts();
        let position = session
            .get(id)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap()
            .position;
        let subset = session.perception_subset(&[id, peer]).unwrap();
        let frame = subset.frame().unwrap();
        let mut random = Random(1);
        let mut audio = Audio {
            fail_play: if fail { Some(1258) } else { None },
            ..Default::default()
        };
        let result = session.update_ripper_retreat(
            id,
            FrameInput {
                context: crate::state23_entry::Context {
                    paused: false,
                    native_mode: 0,
                    parent_state: None,
                    player: None,
                    delta: 0.016,
                },
                frame: &frame,
                range: 10000.,
            },
            50.,
            &mut random,
            &mut audio,
        );
        assert_eq!(result.is_err(), fail);
        assert_eq!(audio.calls, [(true, 1258, 43)]);
        assert_eq!(
            session.state23_combat(id).unwrap().mode,
            if fail { 43 } else { 31 }
        );
        assert!(
            matches!(session.surface_link(marker), Some(SurfaceLink::Marker { owner, mode: 2, .. }) if owner == id)
        );
        assert_eq!(
            session.state23_combat(id).unwrap().owner,
            Some((marker, 219))
        );
        assert_eq!(
            session
                .get(id)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap()
                .position,
            position
        );
        assert_eq!(session.counts(), count);
        assert_eq!(random.0, 1);
        // Marker closing/finalization/release remain their own later owners.
        assert_eq!(session.surface_marker_frame(marker).unwrap().next_frame, 0);
        if !fail {
            let state = session.ripper_primary(id).unwrap().state();
            assert_eq!(
                state.entry.parameter,
                (f64::from(state.entry.saved_parameter) * 0.5) as f32
            );
        }
    }
}

#[test]
#[ignore = "Requires GIANTS_NPC_ROOT; source constructors with supplied mode41/target and actual owned primary, not a natural encounter or completed melee"]
fn source_classic_close_keeps_owner_and_uses_actual_target_before_motion() {
    let root = std::path::PathBuf::from(std::env::var_os("GIANTS_NPC_ROOT").unwrap());
    let resources = SceneResources::load(&root, "w_Story2.bin", "English").unwrap();
    for near in [false, true] {
        let (mut session, id, peer) =
            source_for_class(&resources, 1920 + u64::from(near), true, 73);
        let marker = crate::primary_surface_marker::Factory::new(&resources)
            .construct(&mut session, id)
            .unwrap();
        let mut combat = session.state23_snapshot(id).unwrap();
        combat.mode = 41;
        combat.retaliation = Some(peer);
        session
            .residents
            .get_mut(&id.serial)
            .unwrap()
            .state23_combat = Some(combat);
        let initial = session
            .get(id)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap()
            .position;
        let target = session
            .get(peer)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap()
            .position;
        let subset = session.perception_subset(&[id, peer]).unwrap();
        let frame = subset.frame().unwrap();
        let mut random = Random(1);
        let mut audio = Audio::default();
        session
            .update_ripper_classic_close(
                id,
                FrameInput {
                    context: crate::state23_entry::Context {
                        paused: false,
                        native_mode: 0,
                        parent_state: None,
                        player: None,
                        delta: 0.016,
                    },
                    frame: &frame,
                    range: 10000.,
                },
                if near { 10000. } else { 0. },
                &mut random,
                &mut audio,
            )
            .unwrap();
        assert_eq!(
            session.state23_combat(id).unwrap().mode,
            if near { 42 } else { 41 }
        );
        assert_eq!(
            session
                .ripper_primary(id)
                .unwrap()
                .state()
                .steering
                .target_position,
            target
        );
        assert_eq!(
            session
                .get(id)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap()
                .position,
            initial
        );
        assert_eq!(
            session.state23_combat(id).unwrap().owner,
            Some((marker, 219))
        );
        assert_eq!(
            audio.calls,
            if near { vec![(true, 1254, 41)] } else { vec![] }
        );
        assert_eq!(random.0, 1);
        assert!(session.ripper_primary(id).unwrap().awaiting_motion());
    }
}

#[test]
#[ignore = "Requires GIANTS_NPC_ROOT; source class230, flat terrain/peer eligibility and audio suppression supplied, no ordinary gameplay"]
fn source_variant_awakening_owns_marker_and_retains_late_failures() {
    let root = std::path::PathBuf::from(std::env::var_os("GIANTS_NPC_ROOT").unwrap());
    let resources = SceneResources::load(&root, "w_Story2.bin", "English").unwrap();
    let globals =
        giants_formats::global_data::parse(&std::fs::read(root.join("Bin/GData.gbn")).unwrap())
            .unwrap();
    for case in 0..6 {
        let (mut session, id, peer) = source(&resources, 690 + case, case != 0);
        let initial = session
            .get(id)
            .unwrap()
            .construction_fields
            .clone()
            .unwrap();
        let subset = session.perception_subset(&[id, peer]).unwrap();
        let frame = subset.frame().unwrap();
        let mut random = Random(314);
        let mut audio = Audio {
            fail_play: match case {
                2 => Some(1235),
                3 => Some(1246),
                _ => None,
            },
            fail_stop: case == 4,
            ..Default::default()
        };
        let mut factory = crate::primary_surface_marker::Factory::new(&resources);
        let areas = [crate::object_time::SlowArea {
            source_owner: 0,
            center: [0.; 3],
            radius_squared: 100.,
            factor: 0.,
        }];
        let context = crate::state23_entry::Context {
            paused: false,
            native_mode: 0,
            parent_state: None,
            player: None,
            delta: 0.1,
        };
        let parameters = VariantAwakeningContext {
            camera: [0.; 3],
            camera_range: globals.scalar(0x144).unwrap(),
            global_seconds: 0.1,
            areas: if case == 5 { &areas } else { &[] },
        };
        let result = session.update_ripper_variant_awakening(
            id,
            FrameInput {
                context,
                frame: &frame,
                range: globals.scalar(0x140).unwrap(),
            },
            parameters,
            &mut factory,
            &mut random,
            &mut audio,
        );
        assert_eq!(result.is_err(), matches!(case, 2..=4));
        let machine = session.ripper_primary(id).unwrap();
        let state = machine.state();
        let expected_mode = match case {
            0 | 3 | 5 => 61,
            2 => 0,
            _ => 62,
        };
        assert_eq!(state.entry.mode, expected_mode);
        assert_eq!(session.state23_combat(id).unwrap().mode, expected_mode);
        assert_eq!(
            session
                .get(id)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap()
                .dispatch_context,
            100
        );
        assert_eq!(state.entry.parameter.to_bits(), initial.configuration[1]);
        assert_eq!(random.0, 314);
        let marker = machine.surface_marker();
        assert_eq!(marker.is_some(), matches!(case, 1 | 4));
        if let Some(marker) = marker {
            let m = session.get(marker).unwrap();
            assert_eq!(
                (m.class, m.seed.storage.state(), m.constructor_mask),
                (219, 48, 0x11)
            );
            let fields = m.construction_fields.as_ref().unwrap();
            assert_eq!(fields.position, initial.position);
            assert_eq!(fields.current, initial.current);
            assert_eq!(fields.previous, initial.previous);
            assert_eq!(fields.angles, initial.angles);
            assert_eq!(fields.scale.to_bits(), initial.scale.to_bits());
            assert_eq!(
                session.state23_combat(id).unwrap().owner,
                Some((marker, 219))
            );
            assert!(
                matches!(session.surface_link(marker), Some(SurfaceLink::Marker { owner, .. }) if owner == id)
            );
            assert!(!session.actor_motion(id).unwrap().surface_reset());
            assert_eq!(
                audio.calls,
                [(true, 1235, 0), (true, 1246, 61), (false, 1235, 62)]
            );
        }
        if case == 5 {
            // Entry context100 makes the owner0 area apply to the second clock;
            // reusing the earlier supplied delta would incorrectly create a marker.
            assert_eq!(state.timer.to_bits(), 0);
        }
        let count = session.counts();
        let calls = audio.calls.len();
        assert!(
            session
                .update_ripper_variant_awakening(
                    id,
                    FrameInput {
                        context,
                        frame: &frame,
                        range: globals.scalar(0x140).unwrap()
                    },
                    parameters,
                    &mut factory,
                    &mut random,
                    &mut audio
                )
                .is_err()
        );
        assert_eq!(session.counts(), count);
        assert_eq!(audio.calls.len(), calls);
        session.clear();
        assert!(session.ripper_primary(id).is_err());
    }
}
