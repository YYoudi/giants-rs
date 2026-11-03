//! Ticket-owned global and regional ambient streams. Native driver steps are committed only
//! after every newly requested voice has an actual ready Bevy sink.
use bevy::{audio::Volume, prelude::*};
use giants_game::{
    ambient_streams::{Backend, Config, Driver},
    arithmetic::Precision,
};
use std::collections::{BTreeMap, BTreeSet};

pub struct Plugin;
impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Engine>()
            .init_resource::<ProbeStatus>()
            .add_systems(Update, update.after(crate::mission_player::PlayerUpdate));
    }
}
#[derive(Resource, Default)]
pub(crate) struct ProbeStatus {
    pub ticket: Option<u64>,
    pub loop_started: bool,
    pub accents: u32,
}
struct Source {
    handle: Handle<AudioSource>,
}
struct Voice {
    entity: Entity,
    name: String,
    looping: bool,
    started: bool,
    stopping: bool,
    slot: usize,
    raw_gain: f32,
}
struct Pending {
    next: Driver,
    ids: Vec<u32>,
    dt: f32,
    before: [u32; 5],
    polled: [bool; 2],
    listener: [f32; 3],
    services: Vec<String>,
    stops: Vec<u32>,
    volumes: Vec<(u32, f32)>,
}
struct Emitter {
    key: usize,
    class: i32,
    position: [f32; 3],
    config: Config,
    driver: Driver,
    pending: Option<Pending>,
}
struct Mounted {
    ticket: u64,
    sources: BTreeMap<String, Source>,
    emitters: Vec<Emitter>,
    voices: BTreeMap<u32, Voice>,
    next: u32,
    started: u32,
    accents: u32,
}
#[derive(Resource, Default)]
struct Engine {
    mounted: Option<Mounted>,
    attempted: Option<u64>,
}
struct Planned {
    id: u32,
    name: String,
    looping: bool,
    slot: usize,
}
struct Planner {
    next: u32,
    playing: BTreeMap<u32, bool>,
    requests: Vec<Planned>,
    free_slots: Vec<usize>,
    services: Vec<String>,
    stops: Vec<u32>,
    volumes: Vec<(u32, f32)>,
}
impl Backend for Planner {
    fn play(&mut self, name: &str, _argument: u32, looping: bool) -> u32 {
        let name = name.to_ascii_lowercase();
        if self.free_slots.is_empty() || self.next >= i32::MAX as u32 {
            self.services
                .push(format!("start:{name}:{}:{}", u8::from(looping), u32::MAX));
            return u32::MAX;
        }
        let slot = self.free_slots.remove(0);
        self.next += 1;
        self.requests.push(Planned {
            id: self.next,
            name: name.clone(),
            looping,
            slot,
        });
        self.services
            .push(format!("start:{name}:{}:{}", u8::from(looping), self.next));
        self.playing.insert(self.next, true);
        self.next
    }
    fn playing(&mut self, id: u32) -> bool {
        let result = self.playing.get(&id).copied().unwrap_or(false);
        self.services
            .push(format!("query:{id}:{}", u8::from(result)));
        result
    }
    fn stop(&mut self, id: u32) {
        self.services.push(format!("stop:{id}"));
        self.stops.push(id);
        self.playing.insert(id, false);
    }
    fn volume(&mut self, id: u32, gain: f32) {
        self.services
            .push(format!("volume:{id}:{}", gain.to_bits()));
        self.volumes.push((id, gain));
    }
}
// Raw regional gain may exceed one. Saturation follows global gain multiplication.
// DirectSound integer-dB quantization remains a separately documented backend limit.
fn volume(raw: f32, global: f32) -> Volume {
    Volume::Linear(if crate::mission_audio::muted() {
        0.
    } else {
        (raw * global).clamp(0., 1.)
    })
}
fn mount(world: &mut World, ticket: u64) -> Result<Mounted, String> {
    let configurations = world
        .resource::<crate::mission_runtime::Runtime>()
        .ambient_configs(ticket)?;
    let root = world
        .resource::<crate::frontend_scene::Scene>()
        .data_root()
        .to_owned();
    let resolver = giants_game::stream_resources::Voices::load(&root).map_err(|e| e.to_string())?;
    let mut mounted = Mounted {
        ticket,
        sources: BTreeMap::new(),
        emitters: vec![],
        voices: BTreeMap::new(),
        next: 0,
        started: 0,
        accents: 0,
    };
    let mut names = BTreeSet::new();
    for declaration in &configurations {
        let config = &declaration.config;
        names.extend(
            config
                .beds
                .iter()
                .chain(&config.accents)
                .filter(|n| !n.eq_ignore_ascii_case("dummy"))
                .map(|n| n.to_ascii_lowercase()),
        );
    }
    if names.len() > 128 || configurations.len() > 128 {
        return Err("ambient preparation budget".into());
    }
    // Validate everything before registering any asset, so a failed mount leaks none.
    let mut waves = vec![];
    let mut bytes_total = 0;
    for name in names {
        let path = resolver.resolve(&name).map_err(|e| e.to_string())?;
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        bytes_total += bytes.len();
        if bytes_total > 128 * 1024 * 1024 {
            return Err("ambient PCM preparation byte budget".into());
        }
        let pcm = giants_formats::wav::parse(&bytes).map_err(|e| e.to_string())?;
        waves.push((
            name,
            bytes,
            pcm.samples.len() / usize::from(pcm.channels),
            pcm.sample_rate,
        ));
    }
    for (name, bytes, frames, rate) in waves {
        println!("mission_ambient_source ticket={ticket} name={name} frames={frames} rate={rate}");
        let handle = world
            .resource_mut::<Assets<AudioSource>>()
            .add(AudioSource {
                bytes: bytes.into(),
            });
        mounted.sources.insert(name, Source { handle });
    }
    for crate::mission_runtime::AmbientDeclaration {
        key,
        class,
        position,
        config,
    } in configurations
    {
        println!(
            "mission_ambient_emitter ticket={ticket} emitter={key} class={class} position_bits={},{},{} regional={} dummy={}",
            position[0].to_bits(),
            position[1].to_bits(),
            position[2].to_bits(),
            config.region.is_some(),
            config.dummy()
        );
        mounted.emitters.push(Emitter {
            key,
            class,
            position,
            config,
            driver: Driver::new(1),
            pending: None,
        });
    }
    println!(
        "mission_ambient_mounted ticket={ticket} emitters={} sources={} admission=actual_sink rng_seed=1 live_phase_verified=false activation_policy=gameplay",
        mounted.emitters.len(),
        mounted.sources.len()
    );
    Ok(mounted)
}
fn release(world: &mut World, mounted: Mounted) {
    for voice in mounted.voices.values() {
        if let Some(sink) = world.get::<AudioSink>(voice.entity) {
            sink.stop();
        }
        let _ = world.despawn(voice.entity);
    }
    let mut assets = world.resource_mut::<Assets<AudioSource>>();
    for source in mounted.sources.values() {
        assets.remove(source.handle.id());
    }
    assert!(
        mounted
            .sources
            .values()
            .all(|s| !assets.contains(s.handle.id()))
    );
    println!(
        "mission_ambient_released ticket={} voices={} sources={} started={} accents={} pending={} remaining_assets=0",
        mounted.ticket,
        mounted.voices.len(),
        mounted.sources.len(),
        mounted.started,
        mounted.accents,
        mounted.emitters.iter().any(|e| e.pending.is_some())
    );
}
#[allow(clippy::too_many_arguments)]
fn process(
    world: &mut World,
    mounted: &mut Mounted,
    paused: bool,
    gain: f32,
    dt: f32,
    listener: [f32; 3],
) -> Result<(), String> {
    if !gain.is_finite() || !(0.0..=1.).contains(&gain) || !listener.into_iter().all(f32::is_finite)
    {
        return Err("ambient gain/listener outside finite profile".into());
    }
    let mut playing = BTreeMap::new();
    let mut finished = vec![];
    for (&id, voice) in &mounted.voices {
        if voice.stopping {
            finished.push(id);
            continue;
        }
        if let Some(mut sink) = world.get_mut::<AudioSink>(voice.entity) {
            playing.insert(id, voice.started && !sink.empty());
            sink.set_volume(volume(voice.raw_gain, gain));
            if voice.started {
                if paused {
                    sink.pause();
                } else {
                    sink.play();
                }
                if !voice.looping && sink.empty() {
                    finished.push(id);
                }
            }
        }
    }
    for id in finished {
        if let Some(v) = mounted.voices.remove(&id) {
            if let Some(sink) = world.get::<AudioSink>(v.entity) {
                sink.stop();
            }
            let _ = world.despawn(v.entity);
            println!(
                "mission_ambient_finished ticket={} voice={id} name={} slot={} retired=true",
                mounted.ticket, v.name, v.slot
            );
        }
    }
    if paused {
        return Ok(());
    }
    for emitter in &mut mounted.emitters {
        // A staged request is not a successful native handle. Abandon it if its
        // region is left before admission; keep its slot occupied until retirement.
        if emitter.pending.as_ref().is_some_and(|p| !p.ids.is_empty())
            && let Some([_, maximum]) = emitter.config.region
            && giants_game::lod::distance_from_positions(
                listener,
                emitter.position,
                0.,
                Precision::Double53,
            )
            .map_err(str::to_owned)?
                > maximum
        {
            let pending = emitter.pending.take().unwrap();
            for id in pending.ids {
                if let Some(voice) = mounted.voices.get_mut(&id) {
                    if let Some(sink) = world.get::<AudioSink>(voice.entity) {
                        sink.stop();
                    }
                    voice.stopping = true;
                    println!(
                        "mission_ambient_abandoned ticket={} emitter={} voice={id} reason=region_left_before_admission",
                        mounted.ticket, emitter.key
                    );
                }
            }
        }
        if emitter.pending.is_none() {
            let before = [
                emitter.driver.phase,
                emitter.driver.timer.to_bits(),
                emitter.driver.random.0,
                emitter.driver.bed,
                emitter.driver.accent,
            ];
            let polled = [
                playing.get(&emitter.driver.bed).copied().unwrap_or(false),
                playing
                    .get(&emitter.driver.accent)
                    .copied()
                    .unwrap_or(false),
            ];
            let mut next = emitter.driver.clone();
            let mut planner = Planner {
                next: mounted.next,
                playing: playing.clone(),
                requests: vec![],
                free_slots: (0..10)
                    .filter(|slot| !mounted.voices.values().any(|v| v.slot == *slot))
                    .collect(),
                services: vec![],
                stops: vec![],
                volumes: vec![],
            };
            next.update_at(
                &emitter.config,
                dt,
                0,
                Precision::Double53,
                Some((listener, emitter.position)),
                &mut planner,
            )?;
            let mut ids = vec![];
            for request in planner.requests {
                let source = mounted
                    .sources
                    .get(&request.name)
                    .ok_or("ambient source not prepared")?;
                let entity = world
                    .spawn((
                        crate::mission_runtime::MissionOwned(mounted.ticket),
                        AudioPlayer::new(source.handle.clone()),
                        PlaybackSettings {
                            paused: true,
                            muted: crate::mission_audio::muted(),
                            volume: volume(1., gain),
                            ..if request.looping {
                                PlaybackSettings::LOOP
                            } else {
                                PlaybackSettings::ONCE
                            }
                        },
                    ))
                    .id();
                println!(
                    "mission_ambient_queued ticket={} emitter={} class={} voice={} name={} loop={} paused=true slot={}",
                    mounted.ticket,
                    emitter.key,
                    emitter.class,
                    request.id,
                    request.name,
                    request.looping,
                    request.slot
                );
                mounted.voices.insert(
                    request.id,
                    Voice {
                        entity,
                        name: request.name,
                        looping: request.looping,
                        started: false,
                        stopping: false,
                        slot: request.slot,
                        raw_gain: 1.,
                    },
                );
                ids.push(request.id);
            }
            mounted.next = planner.next;
            emitter.pending = Some(Pending {
                next,
                ids,
                dt,
                before,
                polled,
                listener,
                services: planner.services,
                stops: planner.stops,
                volumes: planner.volumes,
            });
        }
        let pending = emitter.pending.as_ref().unwrap();
        if !pending.ids.iter().all(|id| {
            mounted
                .voices
                .get(id)
                .is_some_and(|v| world.get::<AudioSink>(v.entity).is_some())
        }) {
            continue;
        }
        let pending = emitter.pending.take().unwrap();
        for &(id, raw) in &pending.volumes {
            if !raw.is_finite() || raw < 0. {
                return Err("ambient raw volume outside finite nonnegative profile".into());
            }
            if let Some(voice) = mounted.voices.get_mut(&id) {
                voice.raw_gain = raw;
                if let Some(mut sink) = world.get_mut::<AudioSink>(voice.entity) {
                    sink.set_volume(volume(raw, gain));
                }
            }
        }
        for &id in &pending.stops {
            if let Some(voice) = mounted.voices.get_mut(&id) {
                if let Some(sink) = world.get::<AudioSink>(voice.entity) {
                    sink.stop();
                }
                voice.stopping = true;
                playing.insert(id, false);
                println!(
                    "mission_ambient_stopped ticket={} emitter={} voice={id} slot={} release_policy=next_process",
                    mounted.ticket, emitter.key, voice.slot
                );
            }
        }
        for id in pending.ids {
            let voice = mounted
                .voices
                .get_mut(&id)
                .ok_or("staged ambient voice lost")?;
            let sink = world
                .get::<AudioSink>(voice.entity)
                .ok_or("actual ambient sink lost")?;
            sink.play();
            voice.started = true;
            mounted.started += 1;
            playing.insert(id, true);
            if !voice.looping {
                mounted.accents += 1;
            }
            println!(
                "mission_ambient_started ticket={} emitter={} voice={id} name={} loop={} sink_observed=true muted={}",
                mounted.ticket,
                emitter.key,
                voice.name,
                voice.looping,
                crate::mission_audio::muted()
            );
        }
        emitter.driver = pending.next;
        println!(
            "mission_ambient_step ticket={} emitter={} dt_bits={} before_phase={} before_timer={} before_random={} before_bed={} before_accent={} bed_playing={} accent_playing={} phase={} timer={} random={} bed={} accent={} gain_bits={} dummy={} listener_bits={},{},{} services={}",
            mounted.ticket,
            emitter.key,
            pending.dt.to_bits(),
            pending.before[0],
            pending.before[1],
            pending.before[2],
            pending.before[3],
            pending.before[4],
            pending.polled[0],
            pending.polled[1],
            emitter.driver.phase,
            emitter.driver.timer.to_bits(),
            emitter.driver.random.0,
            emitter.driver.bed,
            emitter.driver.accent,
            emitter.driver.gain.to_bits(),
            emitter.driver.dummy,
            pending.listener[0].to_bits(),
            pending.listener[1].to_bits(),
            pending.listener[2].to_bits(),
            if pending.services.is_empty() {
                "-".to_owned()
            } else {
                pending.services.join(";")
            }
        );
    }
    if mounted.voices.len() > 10 {
        return Err("ambient stream capacity exceeded".into());
    }
    Ok(())
}
fn update(world: &mut World) {
    *world.resource_mut::<ProbeStatus>() = ProbeStatus::default();
    let wanted = world
        .resource::<crate::mission_runtime::Runtime>()
        .active_ticket();
    let scene = world.resource::<crate::frontend_scene::Scene>();
    let paused = !world
        .resource::<crate::mission_showcase::State>()
        .controls(scene.flow.phase);
    let gain = scene.feedback_volume();
    let dt = world.resource::<Time>().delta_secs().min(0.1);
    world.resource_scope(|world, mut engine: Mut<Engine>| {
        if engine
            .mounted
            .as_ref()
            .is_some_and(|m| Some(m.ticket) != wanted)
        {
            release(world, engine.mounted.take().unwrap());
        }
        if engine.mounted.is_none() {
            let Some(ticket) = wanted else { return };
            if engine.attempted == Some(ticket) {
                return;
            }
            engine.attempted = Some(ticket);
            match mount(world, ticket) {
                Ok(m) => engine.mounted = Some(m),
                Err(error) => {
                    error!("mission_ambient_prepare_failed ticket={ticket} error={error}");
                    return;
                }
            }
        }
        let Some(mut mounted) = engine.mounted.take() else {
            return;
        };
        let listener = world
            .query::<&crate::mission_player::Controller>()
            .iter(world)
            .find(|c| c.prepared(mounted.ticket))
            .map(|c| c.audio_listener());
        let Some(listener) = listener else {
            engine.mounted = Some(mounted);
            return;
        };
        if let Err(error) = process(world, &mut mounted, paused, gain, dt, listener) {
            error!(
                "mission_ambient_failed ticket={} error={error}",
                mounted.ticket
            );
            release(world, mounted);
            return;
        }
        let loop_started = mounted.voices.values().any(|v| {
            v.looping
                && v.started
                && world
                    .get::<AudioSink>(v.entity)
                    .is_some_and(|s| !s.empty() && !s.is_paused())
        });
        *world.resource_mut::<ProbeStatus>() = ProbeStatus {
            ticket: Some(mounted.ticket),
            loop_started,
            accents: mounted.accents,
        };
        engine.mounted = Some(mounted);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};
    fn fixture(world: &mut World) -> Mounted {
        let frames = 800u32;
        let mut wave = Vec::new();
        wave.extend(b"RIFF");
        wave.extend((36 + frames * 2).to_le_bytes());
        wave.extend(b"WAVEfmt ");
        wave.extend(16u32.to_le_bytes());
        wave.extend(1u16.to_le_bytes());
        wave.extend(1u16.to_le_bytes());
        wave.extend(8000u32.to_le_bytes());
        wave.extend(16000u32.to_le_bytes());
        wave.extend(2u16.to_le_bytes());
        wave.extend(16u16.to_le_bytes());
        wave.extend(b"data");
        wave.extend((frames * 2).to_le_bytes());
        wave.resize(44 + frames as usize * 2, 0);
        let mut sources = BTreeMap::new();
        for name in ["bed", "accent"] {
            let handle = world
                .resource_mut::<Assets<AudioSource>>()
                .add(AudioSource {
                    bytes: wave.clone().into(),
                });
            sources.insert(name.to_owned(), Source { handle });
        }
        Mounted {
            ticket: 91,
            sources,
            emitters: vec![Emitter {
                key: 7,
                class: 123,
                position: [0.; 3],
                config: Config {
                    beds: vec!["bed".into()],
                    accents: vec!["accent".into()],
                    delays: [0.02, 0.04],
                    region: None,
                },
                driver: Driver::new(1),
                pending: None,
            }],
            voices: BTreeMap::new(),
            next: 0,
            started: 0,
            accents: 0,
        }
    }
    #[test]
    fn absent_backend_freezes_admission_without_duplicate_voices_and_cancel_cleans_assets() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<AudioSource>();
        let mut mounted = fixture(app.world_mut());
        for _ in 0..6 {
            process(app.world_mut(), &mut mounted, false, 0.75, 0.01, [0.; 3]).unwrap();
        }
        assert_eq!(mounted.emitters[0].driver.phase, 5);
        assert_eq!(mounted.emitters[0].driver.bed, 0);
        assert!(mounted.emitters[0].pending.is_some());
        let state = (
            mounted.emitters[0].driver.timer.to_bits(),
            mounted.emitters[0].driver.random.0,
        );
        for _ in 0..100 {
            app.update();
            process(app.world_mut(), &mut mounted, false, 0.5, 0.01, [0.; 3]).unwrap();
        }
        assert_eq!(mounted.next, 1);
        assert_eq!(mounted.voices.len(), 1);
        assert_eq!(mounted.started, 0);
        assert_eq!(
            state,
            (
                mounted.emitters[0].driver.timer.to_bits(),
                mounted.emitters[0].driver.random.0
            )
        );
        process(app.world_mut(), &mut mounted, true, 0.25, 0.01, [0.; 3]).unwrap();
        assert_eq!(mounted.started, 0);
        let entities: Vec<_> = mounted.voices.values().map(|v| v.entity).collect();
        release(app.world_mut(), mounted);
        assert!(entities.iter().all(|e| app.world().get_entity(*e).is_err()));
        assert!(app.world().resource::<Assets<AudioSource>>().is_empty());
    }
    #[test]
    fn regions_refuse_full_pool_abandon_unadmitted_requests_and_reuse_after_retirement() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<AudioSource>();
        let mut mounted = fixture(app.world_mut());
        mounted.emitters[0].config.region = Some([10., 20.]);
        mounted.emitters[0].config.accents.clear();
        let config = mounted.emitters[0].config.clone();
        mounted.emitters = (0..11)
            .map(|key| Emitter {
                key,
                class: 123,
                position: [0.; 3],
                config: config.clone(),
                driver: Driver::new(1),
                pending: None,
            })
            .collect();
        for _ in 0..6 {
            process(app.world_mut(), &mut mounted, false, 0.25, 0.01, [0.; 3]).unwrap();
        }
        assert_eq!(mounted.voices.len(), 10);
        assert_eq!(mounted.started, 0);
        assert_eq!(mounted.emitters[10].driver.bed, u32::MAX);
        process(
            app.world_mut(),
            &mut mounted,
            false,
            0.25,
            0.01,
            [21., 0., 0.],
        )
        .unwrap();
        assert_eq!(mounted.voices.len(), 10);
        assert!(mounted.voices.values().all(|v| v.stopping));
        assert!(mounted.emitters.iter().all(|e| e.pending.is_none()));
        process(
            app.world_mut(),
            &mut mounted,
            false,
            0.25,
            0.01,
            [21., 0., 0.],
        )
        .unwrap();
        assert!(mounted.voices.is_empty());
        process(app.world_mut(), &mut mounted, false, 0.25, 0.01, [0.; 3]).unwrap();
        assert_eq!(mounted.voices.len(), 10);
        assert!(mounted.voices.keys().all(|id| *id > 10));
        assert_eq!(mounted.started, 0);
        release(app.world_mut(), mounted);
        assert!(app.world().resource::<Assets<AudioSource>>().is_empty());
    }
    #[test]
    #[ignore = "Requires real audio device; synthetic zero PCM, global gain zero, no window."]
    fn regional_real_sink_gain_boundary_exit_reentry_and_cancellation() {
        assert!(
            !crate::mission_audio::muted(),
            "zero PCM fixture, use GIANTS_AUDIO_MUTE=0"
        );
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            bevy::audio::AudioPlugin {
                global_volume: bevy::audio::GlobalVolume::new(Volume::Linear(0.)),
                ..default()
            },
        ));
        let mut mounted = fixture(app.world_mut());
        mounted.emitters[0].config.region = Some([10., 20.]);
        mounted.emitters[0].config.accents.clear();
        let start = Instant::now();
        while mounted.started == 0 && start.elapsed() < Duration::from_secs(5) {
            app.update();
            process(app.world_mut(), &mut mounted, false, 0.25, 0.01, [0.; 3]).unwrap();
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(mounted.started, 1);
        let old = mounted.voices.values().next().unwrap().entity;
        assert_eq!(mounted.emitters[0].driver.gain, 2.);
        assert!(
            (app.world()
                .get::<AudioSink>(old)
                .unwrap()
                .volume()
                .to_linear()
                - 0.5)
                .abs()
                < 1e-6
        );
        process(app.world_mut(), &mut mounted, true, 0.75, 0.01, [0.; 3]).unwrap();
        let sink = app.world().get::<AudioSink>(old).unwrap();
        assert!(sink.is_paused());
        assert_eq!(sink.volume().to_linear(), 1.);
        process(
            app.world_mut(),
            &mut mounted,
            false,
            0.25,
            0.01,
            [20., 0., 0.],
        )
        .unwrap();
        assert!(!mounted.voices.values().next().unwrap().stopping);
        assert_eq!(
            app.world()
                .get::<AudioSink>(old)
                .unwrap()
                .volume()
                .to_linear(),
            0.
        );
        process(
            app.world_mut(),
            &mut mounted,
            false,
            0.25,
            0.01,
            [21., 0., 0.],
        )
        .unwrap();
        assert!(mounted.voices.values().next().unwrap().stopping);
        assert_eq!(mounted.emitters[0].driver.bed, 0);
        process(
            app.world_mut(),
            &mut mounted,
            false,
            0.25,
            0.01,
            [21., 0., 0.],
        )
        .unwrap();
        assert!(mounted.voices.is_empty());
        assert!(app.world().get_entity(old).is_err());
        let start = Instant::now();
        while mounted.started < 2 && start.elapsed() < Duration::from_secs(5) {
            app.update();
            process(app.world_mut(), &mut mounted, false, 0.25, 0.01, [0.; 3]).unwrap();
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(mounted.started, 2);
        assert_eq!(mounted.voices.len(), 1);
        release(app.world_mut(), mounted);
        app.update();
        assert!(app.world().resource::<Assets<AudioSource>>().is_empty());
        println!(
            "regional_ambient_device_probe passed=true actual_sink=true gain_above_one=true post_product_clamp=true boundary_zero=true strict_exit=true deferred_release=true reentry=true cancellation=true zero_pcm=true global_gain_zero=true window=false"
        );
    }
    #[test]
    #[ignore = "Requires a real audio device. Zero PCM only, global gain zero, no window."]
    fn real_sink_loop_accent_pause_volume_and_pending_cancellation() {
        assert!(
            !crate::mission_audio::muted(),
            "fixture is all zero PCM; use GIANTS_AUDIO_MUTE=0 for gain inspection"
        );
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            bevy::audio::AudioPlugin {
                global_volume: bevy::audio::GlobalVolume::new(Volume::Linear(0.)),
                ..default()
            },
        ));
        let mut mounted = fixture(app.world_mut());
        let started = Instant::now();
        while mounted.accents == 0 && started.elapsed() < Duration::from_secs(5) {
            app.update();
            process(app.world_mut(), &mut mounted, false, 0.75, 0.01, [0.; 3]).unwrap();
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(
            mounted.accents > 0,
            "actual bed and accent sinks must both start"
        );
        let bed = mounted.voices.values().find(|v| v.looping).unwrap().entity;
        let held = Instant::now();
        while held.elapsed() < Duration::from_millis(350) {
            app.update();
            process(app.world_mut(), &mut mounted, true, 0.25, 0.01, [0.; 3]).unwrap();
            std::thread::sleep(Duration::from_millis(5));
        }
        let before = (
            mounted.emitters[0].driver.timer.to_bits(),
            mounted.emitters[0].driver.random.0,
        );
        let sink = app.world().get::<AudioSink>(bed).unwrap();
        assert!(sink.is_paused());
        assert!(!sink.empty());
        assert!((sink.volume().to_linear() - 0.25).abs() < 1e-6);
        process(app.world_mut(), &mut mounted, true, 0.5, 0.01, [0.; 3]).unwrap();
        assert_eq!(
            before,
            (
                mounted.emitters[0].driver.timer.to_bits(),
                mounted.emitters[0].driver.random.0
            )
        );
        process(app.world_mut(), &mut mounted, false, 0.5, 0.01, [0.; 3]).unwrap();
        assert!(!app.world().get::<AudioSink>(bed).unwrap().is_paused());
        let held = Instant::now();
        while held.elapsed() < Duration::from_millis(350) {
            app.update();
            process(app.world_mut(), &mut mounted, false, 0.5, 0.01, [0.; 3]).unwrap();
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(
            !app.world().get::<AudioSink>(bed).unwrap().empty(),
            "100ms bed must wrap"
        );
        assert_eq!(mounted.voices.values().filter(|v| v.looping).count(), 1);
        release(app.world_mut(), mounted);
        assert!(app.world().resource::<Assets<AudioSource>>().is_empty());
        let mut mounted = fixture(app.world_mut());
        for _ in 0..6 {
            process(app.world_mut(), &mut mounted, false, 0.75, 0.01, [0.; 3]).unwrap();
        }
        assert!(mounted.emitters[0].pending.is_some());
        assert_eq!(mounted.started, 0);
        release(app.world_mut(), mounted);
        app.update();
        assert!(app.world().resource::<Assets<AudioSource>>().is_empty());
        println!(
            "ambient_real_sink_probe passed=true actual_loop=true actual_accent=true loop_wrap=true pause=true gain=true active_cancel=true pending_cancel=true synthetic_zero_pcm=true window=false"
        );
    }
}
