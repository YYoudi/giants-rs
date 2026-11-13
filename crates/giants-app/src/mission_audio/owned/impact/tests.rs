use super::*;
use bevy::ecs::system::SystemState;
use std::time::{Duration, Instant};

#[test]
fn reservations_without_device_stay_unstarted_and_consume_no_audio_rng() {
    let mut world = World::new();
    world.init_resource::<Assets<AudioSource>>();
    world.init_resource::<Assets<crate::positional_pcm::Pcm>>();
    let mut mounted = super::super::super::tests::fixture(&mut world, 963);
    reserve(&mut world, &mut mounted, &[0x5f7]).unwrap();
    reserve(&mut world, &mut mounted, &[0x5f7]).unwrap();
    assert_eq!(mounted.audio_random.0, 1);
    assert_eq!(mounted.voices.len(), 2);
    assert!(
        mounted
            .voices
            .values()
            .all(|v| v.deferred_parameters && !v.started && v.frequency == 0)
    );
    world.insert_resource(Engine {
        mounted: Some(mounted),
        ..default()
    });
    let mut state = SystemState::<OwnedAudio>::new(&mut world);
    assert!(
        !state
            .get_mut(&mut world)
            .unwrap()
            .impact_ready(963, &[0x5f7])
    );
    let mounted = world.resource_mut::<Engine>().mounted.take().unwrap();
    release(&mut world, mounted);
    assert!(
        world
            .resource::<Assets<crate::positional_pcm::Pcm>>()
            .is_empty()
    );
}

struct AnchorAudioDisabled;
#[test]
fn shared_impact_device_keeps_local_voice_missing_sink_and_catalog_miss_distinct() {
    use giants_game::resident_states::MeccImpactVoice;
    let mut world = World::new();
    world.init_resource::<Assets<AudioSource>>();
    world.init_resource::<Assets<crate::positional_pcm::Pcm>>();
    let mut mounted = crate::mission_audio::tests::fixture(&mut world, 976);
    let base = &mounted.sources[&0x5f7];
    mounted.sources.insert(
        1504,
        Source {
            handle: base.handle.clone(),
            pcm: base.pcm.clone(),
            words: base.words,
            channels: base.channels,
            rate: base.rate,
            frames: base.frames,
        },
    );
    // Explicit synthetic catalog misses, separate from a prepared source whose
    // reserved voice has no actual sink. Neither is a playback acknowledgment.
    mounted
        .unavailable
        .extend(DAMAGE_SOUNDS.into_iter().filter(|s| *s != 1504));
    reserve_damage(&mut world, &mut mounted).unwrap();
    world.insert_resource(Engine {
        mounted: Some(mounted),
        ..default()
    });
    let seed = giants_game::state_storage::Seed::from_program_for(
        &giants_formats::class_program::parse(&[0x1e, 0, 0xff]).unwrap(),
        giants_game::state_storage::Profile::Debug,
        Precision::Double53,
    )
    .unwrap();
    let mut session = Session::new(976);
    let local = session
        .register(
            2,
            &seed,
            &[],
            giants_game::object_lists::DirectFlags::default(),
        )
        .unwrap();
    let peer = session
        .register(
            2,
            &seed,
            &[],
            giants_game::object_lists::DirectFlags::default(),
        )
        .unwrap();
    let mut state = SystemState::<OwnedAudio>::new(&mut world);
    state
        .get_mut(&mut world)
        .unwrap()
        .with_impact(976, 0.75, Mat4::IDENTITY.to_cols_array(), local, |device| {
            assert!(device.play(&session, local, 1504).is_err());
            device.play(&session, local, 1686)?;
            assert!(device.play(&session, peer, 1504).is_err());
            assert_eq!(device.mounted.started, 0);
            assert_eq!(device.mounted.audio_random.0, 1);
            assert!(
                device
                    .mounted
                    .voices
                    .values()
                    .all(|v| !v.started && v.deferred_parameters)
            );
            Ok(())
        })
        .unwrap();
}

#[test]
fn retirement_cancels_unstarted_actor_voice_without_acknowledging_a_sink() {
    let mut world = World::new();
    world.init_resource::<Assets<AudioSource>>();
    world.init_resource::<Assets<crate::positional_pcm::Pcm>>();
    let mut mounted = super::super::super::tests::fixture(&mut world, 973);
    reserve(&mut world, &mut mounted, &[0x5f7]).unwrap();
    reserve(&mut world, &mut mounted, &[0x5f7]).unwrap();
    let program = giants_formats::class_program::parse(&[0xff]).unwrap();
    let seed = giants_game::state_storage::Seed::from_program_for(
        &program,
        giants_game::state_storage::Profile::Debug,
        Precision::Double53,
    )
    .unwrap();
    let mut session = Session::new(973);
    let owner = session.register(0, &seed, &[], Default::default()).unwrap();
    let peer = session.register(0, &seed, &[], Default::default()).unwrap();
    let keys = mounted.voices.keys().copied().collect::<Vec<_>>();
    mounted.voices.get_mut(&keys[0]).unwrap().actor_owner = Some(owner);
    mounted.voices.get_mut(&keys[1]).unwrap().actor_owner = Some(peer);
    let owned_entity = mounted.voices[&keys[0]].entity;
    mounted.pending = Some(Pending {
        action: Action::Actor {
            sound: 0x5f7,
            owner,
            positional: true,
        },
        voices: VecDeque::from([keys[0]]),
        frames: 0,
    });
    world.insert_resource(Engine {
        mounted: Some(mounted),
        ..default()
    });
    let mut state = SystemState::<OwnedAudio>::new(&mut world);
    state
        .get_mut(&mut world)
        .unwrap()
        .with_impact(973, 0.75, Mat4::IDENTITY.to_cols_array(), owner, |device| {
            giants_game::actor_retirement::Services::release(device, &mut session, owner)
        })
        .unwrap();
    state.apply(&mut world);
    let mounted = world.resource::<Engine>().mounted.as_ref().unwrap();
    assert!(mounted.pending.is_none());
    assert!(!mounted.voices.contains_key(&keys[0]));
    assert!(mounted.voices.contains_key(&keys[1]));
    assert_eq!(
        (
            mounted.started,
            mounted.actor_started,
            mounted.audio_random.0
        ),
        (0, 0, 1)
    );
    assert!(world.get_entity(owned_entity).is_err());
    assert!(
        session.get(owner).is_some(),
        "audio release does not unlink core storage"
    );
    let mounted = world.resource_mut::<Engine>().mounted.take().unwrap();
    release(&mut world, mounted);
}
impl positional_impact::Cue for AnchorAudioDisabled {
    fn play(&mut self, _: &mut Session, _: Id, _: u32, _: &mut Random) -> Result<(), String> {
        Ok(())
    }
}
fn context(
    pool: &mut giants_game::attached_effect_context::Pool<Id>,
    random: &mut Random,
) -> giants_game::attached_effect_context::Id {
    let (creation, _) = pool
        .prepare_positional(
            "fx_mecc_hard_land",
            [-32., 16., 5.],
            giants_game::effect_initialization::Selection {
                active_objects: 0,
                quality: 1.,
                lite_distance: 500.,
                camera: [-32., 16., 5.],
                recipient_position: [-32., 16., 5.],
            },
            random,
        )
        .unwrap();
    let giants_game::attached_effect_context::Creation::Prepared(id) = creation else {
        panic!("impact context")
    };
    id
}
#[test]
#[ignore = "requires GIANTS_MECC_WORLD_ROOT; no audio device is created"]
fn source_owned_impact_audio_refuses_queued_pcm_without_device_or_audio_draws() {
    let root = std::env::var_os("GIANTS_MECC_WORLD_ROOT").expect("GIANTS_MECC_WORLD_ROOT");
    let resources = giants_game::scene_resources::SceneResources::load(
        std::path::Path::new(&root),
        "w_Story1.bin",
        "English",
    )
    .unwrap();
    let [low, high] = positional_impact::cue_range(&resources, "fx_mecc_hard_land")
        .unwrap()
        .unwrap();
    let bank = giants_game::gameplay_sound_bank::Bank::prepare(
        &resources.classes.visits,
        &resources.archives,
        "English",
        false,
        low..=high,
    )
    .unwrap();
    let mut world = World::new();
    world.init_resource::<Assets<AudioSource>>();
    world.init_resource::<Assets<crate::positional_pcm::Pcm>>();
    let mut mounted = mount_bank(&mut world, 968, bank).unwrap();
    reserve(&mut world, &mut mounted, &(low..=high).collect::<Vec<_>>()).unwrap();
    world.insert_resource(Engine {
        mounted: Some(mounted),
        ..default()
    });
    let mut state = SystemState::<OwnedAudio>::new(&mut world);
    let mut s = Session::new(968);
    let mut pool =
        giants_game::attached_effect_context::Pool::from_active_resources(969, &resources);
    let mut factory = positional_impact::Factory::new(&resources);
    let mut anchor_random = Random(1);
    let anchor = context(&mut pool, &mut anchor_random);
    let local = factory
        .activate(
            &mut s,
            &mut pool,
            anchor,
            &mut anchor_random,
            &mut AnchorAudioDisabled,
        )
        .unwrap();
    let mut vm = Random(1);
    let pending = context(&mut pool, &mut vm);
    let matrix = [
        1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
    ];
    assert!(
        state
            .get_mut(&mut world)
            .unwrap()
            .with_impact(968, 0.75, matrix, local, |device| factory
                .activate(&mut s, &mut pool, pending, &mut vm, device))
            .is_err()
    );
    let failed = factory.actor(pending).unwrap();
    assert!(s.positional_impact(failed).unwrap().failure.is_some());
    assert!(pool.get(pending).is_some());
    assert!(pool.release_prepared(pending).is_err());
    let mut expected = Random(1);
    for _ in 0..4 {
        expected.next_15();
    }
    assert_eq!(vm.0, expected.0);
    {
        let provider = state.get_mut(&mut world).unwrap();
        let mounted = provider.engine.mounted.as_ref().unwrap();
        assert_eq!(mounted.audio_random.0, 1);
        assert_eq!(mounted.started, 0);
        assert!(
            mounted
                .voices
                .values()
                .all(|v| !v.started && v.deferred_parameters)
        );
    }
    let before = vm.0;
    assert!(
        state
            .get_mut(&mut world)
            .unwrap()
            .with_impact(968, 0.75, matrix, local, |device| factory
                .activate(&mut s, &mut pool, pending, &mut vm, device))
            .is_err()
    );
    assert_eq!(vm.0, before);
    state.apply(&mut world);
    let mounted = world.resource_mut::<Engine>().mounted.take().unwrap();
    release(&mut world, mounted);
    s.clear();
}
#[test]
#[ignore = "requires GIANTS_MECC_WORLD_ROOT, GIANTS_AUDIO_MUTE=1 and an actual audio device"]
fn source_owned_impact_audio_starts_real_sinks_keeps_rng_separate_and_stops_exact_owner() {
    assert!(muted(), "set GIANTS_AUDIO_MUTE=1 for silent device control");
    let root = std::env::var_os("GIANTS_MECC_WORLD_ROOT").expect("GIANTS_MECC_WORLD_ROOT");
    let resources = giants_game::scene_resources::SceneResources::load(
        std::path::Path::new(&root),
        "w_Story1.bin",
        "English",
    )
    .unwrap();
    let [low, high] = positional_impact::cue_range(&resources, "fx_mecc_hard_land")
        .unwrap()
        .unwrap();
    assert!(high - low < 32);
    let cues = (low..=high).collect::<Vec<_>>();
    let bank = giants_game::gameplay_sound_bank::Bank::prepare(
        &resources.classes.visits,
        &resources.archives,
        "English",
        false,
        cues.iter().copied(),
    )
    .unwrap();
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::audio::AudioPlugin {
            global_volume: bevy::audio::GlobalVolume::new(Volume::Linear(0.)),
            ..default()
        },
    ));
    app.add_audio_source::<crate::positional_pcm::Pcm>();
    let mut mounted = mount_bank(app.world_mut(), 964, bank).unwrap();
    reserve(app.world_mut(), &mut mounted, &cues).unwrap();
    reserve(app.world_mut(), &mut mounted, &cues).unwrap();
    assert_eq!(mounted.audio_random.0, 1);
    assert_eq!(mounted.started, 0);
    app.insert_resource(Engine {
        mounted: Some(mounted),
        ..default()
    });
    let mut state = SystemState::<OwnedAudio>::new(app.world_mut());
    let start = Instant::now();
    loop {
        app.update();
        if state
            .get_mut(app.world_mut())
            .unwrap()
            .impact_ready(964, &cues)
        {
            break;
        }
        assert!(
            start.elapsed() < Duration::from_secs(8),
            "positional reservations got no actual sinks"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let mut session = Session::new(964);
    let mut pool =
        giants_game::attached_effect_context::Pool::from_active_resources(966, &resources);
    let mut factory = positional_impact::Factory::new(&resources);
    let mut anchor_random = Random(1);
    let anchor_context = context(&mut pool, &mut anchor_random);
    let local = factory
        .activate(
            &mut session,
            &mut pool,
            anchor_context,
            &mut anchor_random,
            &mut AnchorAudioDisabled,
        )
        .unwrap();
    let matrix = [
        1., 0., 0., 0., 0., 0., 1., 0., 0., 1., 0., 0., 0., 0., 0., 1.,
    ];
    let mut vm = Random(1);
    let mut actors = vec![];
    let mut expected_audio = Random(1);
    for _ in 0..2 {
        let before = vm.0;
        let context = context(&mut pool, &mut vm);
        let id = state
            .get_mut(app.world_mut())
            .unwrap()
            .with_impact(964, 0.75, matrix, local, |device| {
                factory.activate(&mut session, &mut pool, context, &mut vm, device)
            })
            .unwrap();
        let mut expected_vm = Random(before);
        for _ in 0..6 {
            expected_vm.next_15();
        }
        assert_eq!(vm.0, expected_vm.0);
        let provider = state.get_mut(app.world_mut()).unwrap();
        let mounted = provider.engine.mounted.as_ref().unwrap();
        let voice = mounted
            .voices
            .values()
            .find(|v| v.actor_owner == Some(id))
            .unwrap();
        let source = &mounted.sources[&voice.sound];
        let expected = audio_parameters::prepare(
            source.words,
            source.rate,
            &mut expected_audio,
            Precision::Double53,
        )
        .unwrap();
        assert_eq!(voice.frequency, expected.frequency);
        assert_eq!(mounted.audio_random.0, expected_audio.0);
        assert!(voice.started && !voice.deferred_parameters);
        let sink = provider.sinks.get(voice.entity).unwrap();
        assert!(sink.is_muted());
        assert_eq!(sink.volume(), Volume::Linear(0.));
        assert_eq!(sink.speed(), expected.frequency as f32 / source.rate as f32);
        assert!(!sink.is_paused());
        println!(
            "impact_ready_probe owner={id:?} sound={} frequency={} seed={} pan_position={:?}",
            voice.sound,
            voice.frequency,
            mounted.audio_random.0,
            voice.spatial.as_ref().unwrap().position
        );
        actors.push(id);
    }
    assert_eq!(
        state
            .get_mut(app.world_mut())
            .unwrap()
            .engine
            .mounted
            .as_ref()
            .unwrap()
            .actor_started,
        2
    );
    let mut stamp = giants_game::animation_epoch::Stamp::default();
    for seconds in [0., 1.] {
        state
            .get_mut(app.world_mut())
            .unwrap()
            .with_impact(964, 0.75, matrix, local, |device| {
                session.advance_positional_impact(actors[0], seconds, &mut stamp, device)
            })
            .unwrap();
    }
    state
        .get_mut(app.world_mut())
        .unwrap()
        .with_impact(964, 0.75, matrix, local, |device| {
            session.cleanup_positional_impact(actors[0], device)
        })
        .unwrap();
    {
        let provider = state.get_mut(app.world_mut()).unwrap();
        let mounted = provider.engine.mounted.as_ref().unwrap();
        assert!(
            !mounted
                .voices
                .values()
                .any(|v| v.actor_owner == Some(actors[0]))
        );
        assert_eq!(
            mounted
                .voices
                .values()
                .filter(|v| v.actor_owner == Some(actors[1]))
                .count(),
            1
        );
        assert_eq!(mounted.audio_random.0, expected_audio.0);
    }
    state.apply(app.world_mut());
    app.update();
    // A disappearing real sink is a late stop failure, not a successful release.
    let entity = {
        let provider = state.get_mut(app.world_mut()).unwrap();
        provider
            .engine
            .mounted
            .as_ref()
            .unwrap()
            .voices
            .values()
            .find(|v| v.actor_owner == Some(actors[1]))
            .unwrap()
            .entity
    };
    app.world_mut().entity_mut(entity).remove::<AudioSink>();
    state
        .get_mut(app.world_mut())
        .unwrap()
        .with_impact(964, 0.75, matrix, local, |device| {
            session.advance_positional_impact(actors[1], 0., &mut stamp, device)
        })
        .unwrap();
    assert!(
        state
            .get_mut(app.world_mut())
            .unwrap()
            .with_impact(964, 0.75, matrix, local, |device| session
                .advance_positional_impact(actors[1], 1., &mut stamp, device))
            .is_err()
    );
    assert!(
        session
            .positional_impact(actors[1])
            .unwrap()
            .failure
            .is_some()
    );
    assert!(session.get(actors[1]).is_some());
    assert!(
        state
            .get_mut(app.world_mut())
            .unwrap()
            .with_impact(964, 0.75, matrix, local, |device| session
                .advance_positional_impact(actors[1], 1., &mut stamp, device))
            .is_err()
    );
    state.apply(app.world_mut());
    let mounted = app
        .world_mut()
        .resource_mut::<Engine>()
        .mounted
        .take()
        .unwrap();
    release(app.world_mut(), mounted);
    app.update();
    assert!(
        app.world()
            .resource::<Assets<crate::positional_pcm::Pcm>>()
            .is_empty()
    );
    assert!(app.world().resource::<Assets<AudioSource>>().is_empty());
    session.clear();
    println!(
        "owned_impact_ready_audio_probe passed=true actual_sinks=true muted=true separate_rng=true exact_owner_stop=true late_failure_retained=true assets_after_flush=0 window=false"
    );
}
