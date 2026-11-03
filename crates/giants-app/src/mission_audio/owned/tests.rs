use super::*;
use bevy::ecs::system::SystemState;
use std::time::{Duration, Instant};
fn install(world: &mut World) {
    let mut mounted = super::super::tests::fixture(world, 77);
    for source in mounted.sources.values_mut() {
        assert!(source.pcm.iter().all(|v| *v == 0));
        source.words[1] = 16f32.to_bits();
        source.words[3] = 20f32.to_bits();
    }
    reserve(world, &mut mounted).unwrap();
    assert_eq!(mounted.audio_random.0, 1);
    assert_eq!(mounted.voices.len(), 8);
    world.insert_resource(Engine {
        mounted: Some(mounted),
        ..default()
    });
}
#[test]
fn reserved_entities_without_device_never_acknowledge_or_draw_random() {
    let mut world = World::new();
    world.init_resource::<Assets<AudioSource>>();
    install(&mut world);
    let mut state = SystemState::<OwnedAudio>::new(&mut world);
    {
        let mut provider = state.get_mut(&mut world).unwrap();
        assert!(!provider.ready(77));
        assert!(
            provider
                .with_backend::<()>(77, 0.75, |_| panic!("unready provider executed callback"))
                .is_err()
        );
        let mounted = provider.engine.mounted.as_ref().unwrap();
        assert_eq!(mounted.audio_random.0, 1);
        assert_eq!(mounted.started, 0);
        assert!(
            mounted
                .voices
                .values()
                .all(|v| v.deferred_parameters && v.frequency == 0 && !v.started)
        );
    }
    state.apply(&mut world);
    let legacy = world.resource_scope(|world, mut engine: Mut<Engine>| {
        stage(world, engine.mounted.as_mut().unwrap(), 0x5f7, 0.75, None).unwrap()
    });
    let foreign = world.spawn_empty().id();
    {
        let mut provider = state.get_mut(&mut world).unwrap();
        provider.quarantine(78);
        assert_eq!(provider.engine.mounted.as_ref().unwrap().voices.len(), 9);
        provider.quarantine(77);
        let mounted = provider.engine.mounted.as_ref().unwrap();
        assert_eq!(mounted.voices.len(), 1);
        assert!(mounted.voices.contains_key(&legacy));
    }
    state.apply(&mut world);
    assert!(world.get_entity(foreign).is_ok());
    let mounted = world.resource_mut::<Engine>().mounted.take().unwrap();
    release(&mut world, mounted);
    assert!(world.resource::<Assets<AudioSource>>().is_empty());
}
#[test]
#[ignore = "Requires a real audio device; only synthetic zero PCM, global gain zero, no window"]
fn reserved_real_sinks_bind_rng_at_play_and_drive_owned_pool() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::audio::AudioPlugin {
            global_volume: bevy::audio::GlobalVolume::new(Volume::Linear(0.)),
            ..default()
        },
    ));
    install(app.world_mut());
    let mut state = SystemState::<OwnedAudio>::new(app.world_mut());
    let deadline = Instant::now();
    loop {
        app.update();
        if state.get_mut(app.world_mut()).unwrap().ready(77) {
            break;
        }
        assert!(
            deadline.elapsed() < Duration::from_secs(5),
            "reserved voices did not acquire real sinks"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        app.world()
            .resource::<Engine>()
            .mounted
            .as_ref()
            .unwrap()
            .audio_random
            .0,
        1
    );
    let seed = giants_game::state_storage::Seed::from_program_for(
        &giants_formats::class_program::parse(&[0x1e, 0, 0xff]).unwrap(),
        giants_game::state_storage::Profile::Debug,
        Precision::Double53,
    )
    .unwrap();
    let mut session = giants_game::resident_states::Session::new(77);
    let actor = session
        .register(
            2,
            &seed,
            &[],
            giants_game::object_lists::DirectFlags::default(),
        )
        .unwrap();
    let mut pool = giants_game::mecc_jet_owned::Pool::new(806);
    let mut reference_random = Random(1);
    let mut expected = BTreeMap::new();
    {
        let mounted = app.world().resource::<Engine>().mounted.as_ref().unwrap();
        for sound in [0x5b2, 0x5b1, 0x5b0, 0x5b3] {
            let source = &mounted.sources[&sound];
            expected.insert(
                sound,
                audio_parameters::prepare(
                    source.words,
                    source.rate,
                    &mut reference_random,
                    Precision::Double53,
                )
                .unwrap(),
            );
        }
    }
    {
        let mut provider = state.get_mut(app.world_mut()).unwrap();
        assert!(
            provider
                .with_backend(77, 0.75, |backend| backend.duration(1))
                .is_err()
        );
        let id = provider
            .with_backend(77, 0.75, |backend| {
                pool.create(
                    actor,
                    77,
                    Sequence {
                        start: 0x5b2,
                        continuous: 0x5b1,
                        release: 0x5b0,
                        flags: 3,
                    },
                    backend,
                )
            })
            .unwrap();
        provider
            .with_backend(77, 0.75, |backend| {
                pool.update(1, 0., false, Precision::Double53, backend)
            })
            .unwrap();
        assert_eq!(provider.engine.mounted.as_ref().unwrap().started, 2);
        provider
            .with_backend(77, 0.75, |backend| pool.interrupt(id, actor, backend))
            .unwrap();
        provider
            .with_backend(77, 0.75, |backend| backend.play(0x5b3, 77, Route::Local))
            .unwrap();
        let mounted = provider.engine.mounted.as_ref().unwrap();
        assert_eq!(mounted.audio_random.0, reference_random.0);
        assert_eq!(mounted.started, 4);
        for voice in mounted.voices.values().filter(|v| v.started) {
            let params = &expected[&voice.sound];
            assert_eq!(voice.frequency, params.frequency);
            assert_eq!(voice.gain.to_bits(), params.gain.to_bits());
            let sink = provider.sinks.get(voice.entity).unwrap();
            assert_eq!(sink.speed(), params.frequency as f32 / 8000.);
            assert!(!sink.is_paused());
        }
        assert_eq!(pool.views()[0].phase, audio_sequence::Phase::Releasing);
        provider
            .with_backend(77, 0.75, |backend| {
                pool.update(2, 0.01, false, Precision::Double53, backend)
            })
            .unwrap();
        provider
            .with_backend(77, 0.75, |backend| {
                pool.update(3, 1., false, Precision::Double53, backend)
            })
            .unwrap();
        assert!(pool.views().is_empty());
    }
    state.apply(app.world_mut());
    let mounted = app
        .world_mut()
        .resource_mut::<Engine>()
        .mounted
        .take()
        .unwrap();
    assert!(
        !mounted
            .voices
            .values()
            .any(|v| v.sound == 0x5b1 && v.started)
    );
    release(app.world_mut(), mounted);
    app.update();
    assert!(app.world().resource::<Assets<AudioSource>>().is_empty());
    println!(
        "owned_reserved_audio_probe passed=true actual_sinks=true zero_pcm=true global_gain_zero=true reserve_rng_unchanged=true played=4 pool_release_complete=true window=false"
    );
}
