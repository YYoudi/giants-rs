use super::*;
use bevy::ecs::system::SystemState;
use giants_game::resident_states::MeccImpactVoice;
use std::time::{Duration, Instant};

struct Uncalled;
impl Backend for Uncalled {
    type Handle = u32;
    fn stop_sound(&mut self, _: u32, _: u64) -> Result<(), String> {
        panic!("nested call")
    }
    fn play(&mut self, _: u32, _: u64, _: Route) -> Result<u32, String> {
        panic!("nested call")
    }
    fn duration(&mut self, _: u32) -> Result<f32, String> {
        panic!("nested call")
    }
    fn volume(&mut self, _: u32, _: f32) -> Result<(), String> {
        panic!("nested call")
    }
    fn stop(&mut self, _: u32) -> Result<(), String> {
        panic!("nested call")
    }
}
#[test]
fn nested_backend_use_refuses_without_executing_a_second_operation() {
    let mut backend = Uncalled;
    let shared = SharedBackend::new(&mut backend);
    let held = shared.borrow().unwrap();
    assert!(
        shared
            .jet()
            .play(1504, 1, Route::Local)
            .unwrap_err()
            .contains("nested")
    );
    drop(held);
    assert!(shared.borrow().is_ok());
}

fn sources_without_device() -> World {
    let mut world = World::new();
    world.init_resource::<Assets<AudioSource>>();
    let mut mounted = crate::mission_audio::tests::fixture(&mut world, 872);
    for sound in DAMAGE_SOUNDS {
        let base = &mounted.sources[&0x5f7];
        let source = Source {
            handle: base.handle.clone(),
            pcm: base.pcm.clone(),
            words: [
                if sound >= 1683 { 32 } else { 0 },
                25f32.to_bits(),
                100f32.to_bits(),
                0,
                20f32.to_bits(),
                0,
            ],
            channels: base.channels,
            rate: base.rate,
            frames: base.frames,
        };
        mounted.sources.insert(sound, source);
    }
    reserve(&mut world, &mut mounted).unwrap();
    reserve_damage(&mut world, &mut mounted).unwrap();
    assert_eq!(mounted.voices.len(), 20);
    assert_eq!(mounted.audio_random.0, 1);
    world.insert_resource(Engine {
        mounted: Some(mounted),
        ..default()
    });
    world
}
#[test]
fn damage_reservations_without_a_device_remain_unstarted_and_draw_nothing() {
    let mut world = sources_without_device();
    let mut state = SystemState::<OwnedAudio>::new(&mut world);
    let mut audio = state.get_mut(&mut world).unwrap();
    assert!(!audio.damage_ready(872));
    assert!(
        audio
            .with_backend::<()>(872, 0.75, |_| panic!("no real sink"))
            .is_err()
    );
    let mounted = audio.engine.mounted.as_ref().unwrap();
    assert_eq!(mounted.started, 0);
    assert_eq!(mounted.audio_random.0, 1);
    assert!(
        mounted
            .voices
            .values()
            .all(|v| !v.started && v.local_owner.is_none())
    );
}

#[test]
fn explicit_catalog_miss_is_not_an_unprepared_source_or_device_acknowledgement() {
    let mut world = sources_without_device();
    let mut state = SystemState::<OwnedAudio>::new(&mut world);
    let OwnedAudio {
        mut engine,
        mut sinks,
        mut commands,
        ..
    } = state.get_mut(&mut world).unwrap();
    let mounted = engine.mounted.as_mut().unwrap();
    mounted.sources.remove(&1686);
    mounted.voices.retain(|_, v| v.sound != 1686);
    mounted.unavailable.insert(1686); // Explicit synthetic catalog-miss outcome.
    let mut device = ReservedDevice {
        mounted,
        sinks: &mut sinks,
        commands: &mut commands,
        effects: 0.75,
        prepared_sound: None,
    };
    assert_eq!(
        Backend::play(&mut device, 1686, 872, Route::Local).unwrap(),
        0
    );
    assert_eq!(device.mounted.started, 0);
    assert_eq!(device.mounted.audio_random.0, 1);
    device.mounted.unavailable.remove(&1686);
    assert!(Backend::play(&mut device, 1686, 872, Route::Local).is_err());
    assert_eq!(device.mounted.started, 0);
    assert_eq!(device.mounted.audio_random.0, 1);
}

#[test]
#[ignore = "Requires GIANTS_MECC_WORLD_ROOT, GIANTS_AUDIO_MUTE=1 and a real audio device; no window"]
fn source_owned_mecc_damage_voice_uses_real_local_sinks_and_unique_reuse() {
    assert!(muted());
    let root = std::path::PathBuf::from(std::env::var_os("GIANTS_MECC_WORLD_ROOT").unwrap());
    let resources =
        giants_game::scene_resources::SceneResources::load(&root, "w_Story1.bin", "English")
            .unwrap();
    let (language, fallback) =
        giants_game::gameplay_sound_bank::voice_language(&resources.archives, "English").unwrap();
    println!(
        "owned_damage_voice_language selected={language} fallback={fallback} native_startup_verified=false"
    );
    let bank = giants_game::gameplay_sound_bank::Bank::prepare(
        &resources.classes.visits,
        &resources.archives,
        &language,
        false,
        SOUNDS.into_iter().chain(DAMAGE_SOUNDS),
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
    app.init_resource::<Assets<crate::positional_pcm::Pcm>>();
    let mut mounted = mount_bank(app.world_mut(), 872, bank).unwrap();
    reserve(app.world_mut(), &mut mounted).unwrap();
    reserve_damage(app.world_mut(), &mut mounted).unwrap();
    assert_eq!(mounted.started, 0);
    assert_eq!(mounted.audio_random.0, 1);
    app.insert_resource(Engine {
        mounted: Some(mounted),
        ..default()
    });
    let mut state = SystemState::<OwnedAudio>::new(app.world_mut());
    let deadline = Instant::now();
    loop {
        app.update();
        let audio = state.get_mut(app.world_mut()).unwrap();
        if audio.ready(872) && audio.damage_ready(872) {
            break;
        }
        assert!(
            deadline.elapsed() < Duration::from_secs(8),
            "real local sinks not ready"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    let seed = giants_game::state_storage::Seed::from_program_for(
        &giants_formats::class_program::parse(&[0x1e, 0, 0xff]).unwrap(),
        giants_game::state_storage::Profile::Debug,
        Precision::Double53,
    )
    .unwrap();
    let mut session = Session::new(872);
    let source = session
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
    let mut vm = Random(42);
    let mut expected_vm = vm;
    expected_vm.next_15();
    let shared_cue = 1683 + ((expected_vm.next_15() * 100) >> 15) / 25;
    let mut expected_audio = Random(1);
    let mut frequencies: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    {
        let mounted = app.world().resource::<Engine>().mounted.as_ref().unwrap();
        for sound in [1505, shared_cue, 1504, 1504, 0x5b3] {
            let src = &mounted.sources[&sound];
            let p = audio_parameters::prepare(
                src.words,
                src.rate,
                &mut expected_audio,
                Precision::Double53,
            )
            .unwrap();
            frequencies.entry(sound).or_default().push(p.frequency);
        }
    }
    {
        let mut audio = state.get_mut(app.world_mut()).unwrap();
        audio
            .with_backend(872, 0.75, |backend| {
                let shared = SharedBackend::new(backend);
                let mut voice = shared.voice(source, 872);
                let mut timers = giants_game::mecc_impact_voice::State {
                    damage: 55.,
                    limit: 100.,
                    pain_timer: -1.,
                    shared_timer: -1.,
                };
                giants_game::mecc_impact_voice::after_damage(
                    &mut timers,
                    &mut vm,
                    |sound, _, _| voice.play(&session, source, sound),
                )?;
                assert_eq!(timers.shared_timer, 10.);
                voice.play(&session, source, shared_cue)?; // Actual live instance reuse.
                voice.play(&session, source, 1504)?;
                voice.play(&session, source, 1504)?; // Distinct nonunique starts.
                assert!(voice.play(&session, peer, 1504).is_err());
                assert!(
                    shared
                        .voice(source, 999)
                        .play(&session, source, 1504)
                        .is_err()
                );
                shared.jet().play(0x5b3, 872, Route::Local)?;
                Ok(())
            })
            .unwrap();
        // The ordinary Ripper path reborrows this same local backend from its
        // impact/retirement device. Reusing the actual unique voice must add
        // neither a start nor audio RNG draws, and still needs the live sink.
        audio
            .with_impact(
                872,
                0.75,
                Mat4::IDENTITY.to_cols_array(),
                source,
                |device| device.play(&session, source, shared_cue),
            )
            .unwrap();
        let mounted = audio.engine.mounted.as_ref().unwrap();
        assert_eq!(mounted.started, 5);
        assert_eq!(mounted.audio_random.0, expected_audio.0);
        assert_eq!(vm.0, expected_vm.0);
        let mut actual: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        for voice in mounted.voices.values().filter(|v| v.started) {
            assert_eq!(voice.local_owner, Some(872));
            let sink = audio.sinks.get(voice.entity).unwrap();
            assert!(!sink.is_paused() && !sink.empty() && sink.is_muted());
            assert_eq!(sink.volume(), Volume::Linear(0.));
            actual.entry(voice.sound).or_default().push(voice.frequency);
        }
        for v in frequencies.values_mut() {
            v.sort_unstable();
        }
        for v in actual.values_mut() {
            v.sort_unstable();
        }
        assert_eq!(actual, frequencies);
    }
    let unrelated = app
        .world()
        .resource::<Engine>()
        .mounted
        .as_ref()
        .unwrap()
        .voices
        .values()
        .find(|v| v.sound == 0x5b3 && v.started)
        .unwrap()
        .entity;
    app.world_mut().entity_mut(unrelated).remove::<AudioSink>();
    {
        let mut audio = state.get_mut(app.world_mut()).unwrap();
        let before = audio.engine.mounted.as_ref().unwrap().audio_random.0;
        audio
            .with_backend(872, 0.75, |backend| {
                SharedBackend::new(backend)
                    .voice(source, 872)
                    .play(&session, source, shared_cue)
            })
            .unwrap();
        assert_eq!(
            audio.engine.mounted.as_ref().unwrap().audio_random.0,
            before
        );
        assert_eq!(audio.engine.mounted.as_ref().unwrap().started, 5);
    }
    let victim = app
        .world()
        .resource::<Engine>()
        .mounted
        .as_ref()
        .unwrap()
        .voices
        .values()
        .find(|v| v.sound == shared_cue && v.started)
        .unwrap()
        .entity;
    app.world_mut().entity_mut(victim).remove::<AudioSink>();
    {
        let mut audio = state.get_mut(app.world_mut()).unwrap();
        let before = audio.engine.mounted.as_ref().unwrap().audio_random.0;
        assert!(
            audio
                .with_backend(872, 0.75, |backend| SharedBackend::new(backend)
                    .voice(source, 872)
                    .play(&session, source, shared_cue))
                .is_err()
        );
        assert_eq!(
            audio.engine.mounted.as_ref().unwrap().audio_random.0,
            before
        );
        assert_eq!(audio.engine.mounted.as_ref().unwrap().started, 5);
        audio.quarantine(872);
    }
    state.apply(app.world_mut());
    let mounted = app
        .world_mut()
        .resource_mut::<Engine>()
        .mounted
        .take()
        .unwrap();
    assert!(mounted.voices.is_empty());
    release(app.world_mut(), mounted);
    assert!(app.world().resource::<Assets<AudioSource>>().is_empty());
    println!(
        "owned_mecc_damage_voice passed=true real_starts=5 unique_reuses=2 vm_rng_separate=true late_sink_failure_retained=true window=false muted=true"
    );
}

#[test]
fn native_no_play_handles_cues_without_backend_borrow_or_audio_draws() {
    use giants_game::{mecc_impact_voice as voice, startup_playback::Playback};
    let seed = giants_game::state_storage::Seed::from_program_for(
        &giants_formats::class_program::parse(&[0x1e, 0, 0xff]).unwrap(),
        giants_game::state_storage::Profile::Debug,
        Precision::Double53,
    )
    .unwrap();
    let mut session = Session::new(872);
    let source = session
        .register(
            2,
            &seed,
            &[],
            giants_game::object_lists::DirectFlags::default(),
        )
        .unwrap();
    let mut backend = Uncalled;
    let shared = SharedBackend::new(&mut backend);
    let mut playback = Playback::default();
    assert!(playback.observe_startup_option("-noplay"));
    let mut local = shared.voice_with_playback(source, 872, playback);
    // Suppression never needs the device, even while another owner holds it.
    let held = shared.borrow().unwrap();
    let mut state = voice::State {
        damage: 15.,
        limit: 100.,
        pain_timer: -1.,
        shared_timer: -1.,
    };
    let mut vm = Random(42);
    let mut expected = vm;
    expected.next_15();
    expected.next_15();
    voice::after_damage(&mut state, &mut vm, |cue, _, _| {
        local.play(&session, source, cue)
    })
    .unwrap();
    assert_eq!(vm.0, expected.0);
    assert!(state.pain_timer >= 1. && state.pain_timer < 2.);
    assert_eq!(state.shared_timer, 10.);
    let mut foreign_session = Session::new(873);
    let foreign = foreign_session
        .register(
            2,
            &seed,
            &[],
            giants_game::object_lists::DirectFlags::default(),
        )
        .unwrap();
    assert!(local.play(&session, foreign, 1504).is_err());
    assert!(local.play(&session, source, 999).is_err());
    drop(held);
    assert!(shared.borrow().is_ok());
}
