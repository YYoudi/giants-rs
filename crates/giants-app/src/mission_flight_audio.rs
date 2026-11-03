//! Ticket-owned delayed projectile loops. Parent retirement projects the checked
//! linked-child cleanup profile; motion/contact remain the gameplay prototype.
use bevy::{
    audio::{AddAudioSource, Decodable, Volume},
    prelude::*,
};
use giants_game::{
    arithmetic::Precision,
    audio_parameters::{self, Random},
    projectile_flight_audio::{Driver, Recipe},
};
use std::{collections::BTreeMap, sync::Arc};
#[derive(Message, Clone, Copy)]
pub(crate) struct Born {
    pub ticket: u64,
    pub serial: u32,
    pub origin: [f32; 3],
}
#[derive(Asset, TypePath, Clone)]
struct FlightPcm(crate::positional_pcm::Pcm);
impl Decodable for FlightPcm {
    type Decoder = crate::positional_pcm::Decoder;
    fn decoder(&self) -> Self::Decoder {
        self.0.decoder()
    }
}
pub struct Plugin;
impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        if app.is_plugin_added::<bevy::audio::AudioPlugin>() {
            app.add_audio_source::<FlightPcm>();
        } else {
            app.init_asset::<FlightPcm>();
        }
        app.add_message::<Born>()
            .init_resource::<Engine>()
            .init_resource::<ProbeStatus>()
            .add_systems(
                Update,
                (collect, update)
                    .chain()
                    .after(crate::mission_player::PlayerUpdate),
            );
    }
}
#[derive(Resource, Default)]
pub(crate) struct ProbeStatus {
    pub ticket: Option<u64>,
    pub active_loop: bool,
}
#[derive(Resource, Default)]
struct Engine {
    mounted: Option<Mounted>,
    attempted: Option<u64>,
    born: Vec<Born>,
}
struct Source {
    samples: Arc<[i16]>,
    channels: u16,
    rate: u32,
    words: [u32; 6],
}
struct Voice {
    entity: Entity,
    handle: Handle<FlightPcm>,
    pan: crate::positional_pcm::Pan,
    gain: f32,
    minimum: f32,
    maximum: f32,
    started: bool,
    last_point: Option<([u32; 3], [u32; 16])>,
}
struct Owner {
    driver: Driver,
    voice: Option<Voice>,
}
struct Mounted {
    ticket: u64,
    recipe: Recipe,
    source: Source,
    owners: BTreeMap<u32, Owner>,
    random: Random,
    born: u32,
    started: u32,
    retired: u32,
    queued_cancel: u32,
    waiting_cancel: u32,
}
#[derive(Clone, Copy)]
struct Snapshot {
    serial: u32,
    position: [f32; 3],
}
fn collect(mut input: MessageReader<Born>, mut engine: ResMut<Engine>) {
    engine.born.extend(input.read().copied());
}
fn mount(
    world: &mut World,
    ticket: u64,
    definition: &giants_formats::weapons::Projectile,
) -> Result<Mounted, String> {
    let (recipe, mut bank) = world
        .resource::<crate::mission_runtime::Runtime>()
        .flight_bank(ticket, definition)?;
    let sound = bank
        .sounds
        .remove(&recipe.sound)
        .ok_or("flight source not resolved")?;
    if sound.binding.descriptor[0] != 1 {
        return Err("flight source outside measured plain-loop descriptor profile".into());
    }
    println!(
        "flight_audio_mounted ticket={ticket} sound={} source_count=1 descriptor={:?} rate={} frames={} delay_bits={} class={} rng_seed=1 live_rng_phase_verified=false motion_native=false",
        recipe.sound,
        sound.binding.descriptor,
        sound.pcm.sample_rate,
        sound.pcm.samples.len() / usize::from(sound.pcm.channels),
        recipe.delay.to_bits(),
        recipe.class_context[1]
    );
    Ok(Mounted {
        ticket,
        recipe,
        source: Source {
            samples: sound.pcm.samples.into(),
            channels: sound.pcm.channels,
            rate: sound.pcm.sample_rate,
            words: sound.binding.descriptor,
        },
        owners: BTreeMap::new(),
        random: Random(1),
        born: 0,
        started: 0,
        retired: 0,
        queued_cancel: 0,
        waiting_cancel: 0,
    })
}
fn discard(world: &mut World, voice: Voice) {
    if let Some(sink) = world.get::<AudioSink>(voice.entity) {
        sink.stop();
    }
    let _ = world.despawn(voice.entity);
    world
        .resource_mut::<Assets<FlightPcm>>()
        .remove(voice.handle.id());
}
fn release(world: &mut World, mounted: Mounted) {
    let handles: Vec<_> = mounted
        .owners
        .values()
        .filter_map(|o| o.voice.as_ref().map(|v| v.handle.id()))
        .collect();
    for owner in mounted.owners.into_values() {
        if let Some(voice) = owner.voice {
            discard(world, voice);
        }
    }
    assert!(
        handles
            .iter()
            .all(|h| !world.resource::<Assets<FlightPcm>>().contains(*h))
    );
    println!(
        "flight_audio_released ticket={} born={} started={} retired={} waiting_cancel={} queued_cancel={} source_count=1 remaining_assets=0",
        mounted.ticket,
        mounted.born,
        mounted.started,
        mounted.retired,
        mounted.waiting_cancel,
        mounted.queued_cancel
    );
}
fn stage(
    world: &mut World,
    ticket: u64,
    serial: u32,
    source: &Source,
    recipe: &Recipe,
    random: &mut Random,
) -> Result<Voice, String> {
    let before = random.0;
    let parameters =
        audio_parameters::prepare(source.words, source.rate, random, Precision::Double53)?;
    let pcm =
        crate::positional_pcm::Pcm::new(source.samples.clone(), source.channels, source.rate)?;
    let pan = pcm.pan.clone();
    let handle = world
        .resource_mut::<Assets<FlightPcm>>()
        .add(FlightPcm(pcm));
    let entity = world
        .spawn((
            crate::mission_runtime::MissionOwned(ticket),
            AudioPlayer(handle.clone()),
            PlaybackSettings {
                paused: true,
                muted: crate::mission_audio::muted(),
                volume: Volume::Linear(0.),
                speed: parameters.frequency as f32 / source.rate as f32,
                ..PlaybackSettings::LOOP
            },
        ))
        .id();
    println!(
        "flight_audio_queued ticket={ticket} projectile={serial} sound={} class={} frequency={} gain_bits={} minimum_bits={} maximum_bits={} random_before={before} random_after={} paused=true",
        recipe.sound,
        recipe.class_context[1],
        parameters.frequency,
        parameters.gain.to_bits(),
        parameters.minimum_distance.to_bits(),
        parameters.maximum_distance.to_bits(),
        random.0
    );
    Ok(Voice {
        entity,
        handle,
        pan,
        gain: parameters.gain,
        minimum: parameters.minimum_distance,
        maximum: parameters.maximum_distance,
        started: false,
        last_point: None,
    })
}
#[allow(clippy::too_many_arguments)]
fn process(
    world: &mut World,
    mounted: &mut Mounted,
    born: &[Born],
    parents: &[Snapshot],
    view: Option<[f32; 16]>,
    paused: bool,
    gain: f32,
    dt: f32,
) -> Result<(), String> {
    if !gain.is_finite() || !(0.0..=1.).contains(&gain) {
        return Err("flight gain outside unit profile".into());
    }
    for event in born {
        if event.ticket != mounted.ticket {
            continue;
        }
        if event.serial == 0
            || !event.origin.into_iter().all(f32::is_finite)
            || mounted.owners.contains_key(&event.serial)
        {
            return Err("invalid or duplicate projectile audio birth".into());
        }
        if mounted.owners.len() >= 64 {
            return Err("flight owner budget".into());
        }
        mounted.owners.insert(
            event.serial,
            Owner {
                driver: Driver::default(),
                voice: None,
            },
        );
        mounted.born += 1;
        println!(
            "flight_audio_born ticket={} projectile={} origin={:?}",
            mounted.ticket, event.serial, event.origin
        );
    }
    let mut retired = vec![];
    for (&serial, owner) in &mut mounted.owners {
        let Some(parent) = parents.iter().find(|s| s.serial == serial) else {
            retired.push(serial);
            continue;
        };
        if !parent.position.into_iter().all(f32::is_finite) {
            return Err("nonfinite projectile audio position".into());
        }
        if owner.voice.is_none() && !paused {
            let before = owner.driver.timer.to_bits();
            let request = owner
                .driver
                .update(&mounted.recipe, dt, Precision::Double53)?;
            println!(
                "flight_audio_step ticket={} projectile={serial} dt_bits={} before_timer={before} timer={} waiting={} request={request}",
                mounted.ticket,
                dt.to_bits(),
                owner.driver.timer.to_bits(),
                owner.driver.waiting()
            );
            if request {
                owner.voice = Some(stage(
                    world,
                    mounted.ticket,
                    serial,
                    &mounted.source,
                    &mounted.recipe,
                    &mut mounted.random,
                )?);
            }
        }
        let Some(voice) = &mut owner.voice else {
            continue;
        };
        let mut spatial_gain = 0.;
        if let Some(matrix) = view {
            let spatial = giants_game::audio_spatial::update(
                &giants_game::audio_spatial::Inputs {
                    matrix,
                    position: parent.position,
                    flags: 0x1000e,
                    previous_pan: 0.,
                    previous_gain: 0.,
                    minimum: voice.minimum,
                    maximum: voice.maximum,
                },
                Precision::Double53,
            )?;
            let units = giants_game::audio_spatial::pan_units(spatial.pan, Precision::Double53)?;
            voice.pan.set(units)?;
            spatial_gain = spatial.gain;
            if voice.last_point
                != Some((parent.position.map(f32::to_bits), matrix.map(f32::to_bits)))
            {
                println!(
                    "flight_audio_spatial ticket={} projectile={serial} point={:?} matrix_bits={:?} distance_bits={} pan_bits={} gain_bits={} pan_units={units} minimum_bits={} maximum_bits={}",
                    mounted.ticket,
                    parent.position,
                    matrix.map(f32::to_bits),
                    spatial.distance.to_bits(),
                    spatial.pan.to_bits(),
                    spatial.gain.to_bits(),
                    voice.minimum.to_bits(),
                    voice.maximum.to_bits()
                );
                voice.last_point =
                    Some((parent.position.map(f32::to_bits), matrix.map(f32::to_bits)));
            }
        }
        if let Some(mut sink) = world.get_mut::<AudioSink>(voice.entity) {
            sink.set_volume(Volume::Linear(if crate::mission_audio::muted() {
                0.
            } else {
                gain * voice.gain * spatial_gain
            }));
            if paused || view.is_none() {
                sink.pause();
            } else {
                sink.play();
                if !voice.started {
                    voice.started = true;
                    mounted.started += 1;
                    println!(
                        "flight_audio_started ticket={} projectile={serial} sound={} sink_observed=true parent_alive=true muted={}",
                        mounted.ticket,
                        mounted.recipe.sound,
                        crate::mission_audio::muted()
                    );
                }
            }
        }
    }
    for serial in retired {
        let owner = mounted.owners.remove(&serial).unwrap();
        mounted.retired += 1;
        let phase = match owner.voice {
            Some(v) => {
                let phase = if v.started {
                    "started"
                } else {
                    mounted.queued_cancel += 1;
                    "queued"
                };
                discard(world, v);
                phase
            }
            None => {
                mounted.waiting_cancel += 1;
                "waiting"
            }
        };
        println!(
            "flight_audio_retired ticket={} projectile={serial} phase={phase} parent_alive=false linked_child_cleanup=true",
            mounted.ticket
        );
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
        engine.born.retain(|b| Some(b.ticket) == wanted);
        let Some(ticket) = wanted else {
            return;
        };
        if engine.mounted.is_none() {
            if engine.attempted == Some(ticket) {
                engine.born.clear();
                return;
            }
            let definition = world
                .query::<&crate::mission_player::Controller>()
                .iter(world)
                .find(|c| c.prepared(ticket))
                .and_then(|c| c.pistol())
                .map(|p| p.definition.clone());
            let Some(definition) = definition else {
                return;
            };
            engine.attempted = Some(ticket);
            match mount(world, ticket, &definition) {
                Ok(m) => engine.mounted = Some(m),
                Err(error) => {
                    error!("flight_audio_prepare_failed ticket={ticket} error={error}");
                    engine.born.clear();
                    return;
                }
            }
        }
        let (view, parents, replay_dt) = world
            .query::<&crate::mission_player::Controller>()
            .iter(world)
            .find(|c| c.prepared(ticket))
            .map(|c| {
                (
                    Some(c.audio_view()),
                    c.pistol().map_or_else(Vec::new, |p| {
                        p.shots
                            .iter()
                            .map(|s| Snapshot {
                                serial: s.serial,
                                position: s.motion.position,
                            })
                            .collect::<Vec<_>>()
                    }),
                    c.flight_audio_replay_delta(),
                )
            })
            .unwrap_or_default();
        let born = std::mem::take(&mut engine.born);
        let mut mounted = engine.mounted.take().unwrap();
        if let Err(error) = process(
            world,
            &mut mounted,
            &born,
            &parents,
            view,
            paused,
            gain,
            replay_dt.unwrap_or(dt),
        ) {
            error!("flight_audio_failed ticket={ticket} error={error}");
            release(world, mounted);
            return;
        }
        let active_loop = mounted.owners.values().any(|o| {
            o.voice.as_ref().is_some_and(|v| {
                v.started
                    && world
                        .get::<AudioSink>(v.entity)
                        .is_some_and(|s| !s.empty() && !s.is_paused())
            })
        });
        *world.resource_mut::<ProbeStatus>() = ProbeStatus {
            ticket: Some(ticket),
            active_loop,
        };
        engine.mounted = Some(mounted);
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn silent_asset_owners_do_not_schedule_device_playback() {
        use bevy::{asset::AssetPlugin, prelude::*};
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<AudioSource>()
            .add_plugins((crate::mission_audio::Plugin, super::Plugin));
        app.finish();
        app.cleanup();
        assert!(!app.is_plugin_added::<bevy::audio::AudioPlugin>());
        assert!(app.world().resource::<Assets<AudioSource>>().is_empty());
        assert!(
            app.world()
                .resource::<Assets<crate::positional_pcm::Pcm>>()
                .is_empty()
        );
        assert!(
            app.world()
                .resource::<Assets<super::FlightPcm>>()
                .is_empty()
        );
        // The faulty registration added play_queued_audio_system here, which
        // required AudioOutput even with no players or hardware installed.
        for _ in 0..3 {
            app.world_mut().run_schedule(PostUpdate);
        }
    }
    use super::*;
    use std::time::{Duration, Instant};
    const VIEW: [f32; 16] = [
        1., 0., 0., 0., 0., 0., -1., 0., 0., 1., 0., 0., 0., 0., 0., 1.,
    ];
    fn fixture() -> Mounted {
        Mounted {
            ticket: 99,
            recipe: Recipe {
                delay: 0.02,
                class_context: [7, 91, 0],
                sound: 123,
            },
            source: Source {
                samples: vec![0; 800].into(),
                channels: 1,
                rate: 8000,
                words: [1, 0, 100f32.to_bits(), 0, 30f32.to_bits(), 450f32.to_bits()],
            },
            owners: BTreeMap::new(),
            random: Random(1),
            born: 0,
            started: 0,
            retired: 0,
            queued_cancel: 0,
            waiting_cancel: 0,
        }
    }
    fn born(serial: u32) -> Born {
        Born {
            ticket: 99,
            serial,
            origin: [3., 4., 0.],
        }
    }
    fn parent(serial: u32) -> Snapshot {
        Snapshot {
            serial,
            position: [3., 4., 0.],
        }
    }
    #[test]
    fn missing_backend_is_not_ready_and_both_delayed_and_queued_retirement_clean_up() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<FlightPcm>();
        let mut mounted = fixture();
        process(
            app.world_mut(),
            &mut mounted,
            &[born(1)],
            &[parent(1)],
            Some(VIEW),
            false,
            0.75,
            0.01,
        )
        .unwrap();
        let timer = mounted.owners[&1].driver.timer.to_bits();
        process(
            app.world_mut(),
            &mut mounted,
            &[],
            &[parent(1)],
            Some(VIEW),
            true,
            0.5,
            0.1,
        )
        .unwrap();
        assert_eq!(
            mounted.owners[&1].driver.timer.to_bits(),
            timer,
            "pause freezes waiting delay"
        );
        process(
            app.world_mut(),
            &mut mounted,
            &[],
            &[],
            Some(VIEW),
            false,
            0.5,
            0.1,
        )
        .unwrap();
        assert_eq!(mounted.waiting_cancel, 1);
        assert!(app.world().resource::<Assets<FlightPcm>>().is_empty());
        for step in 0..100 {
            app.update();
            let birth = if step == 0 { vec![born(2)] } else { vec![] };
            process(
                app.world_mut(),
                &mut mounted,
                &birth,
                &[parent(2)],
                Some(VIEW),
                false,
                0.75,
                0.01,
            )
            .unwrap();
        }
        assert_eq!(mounted.started, 0);
        assert_eq!(mounted.owners.len(), 1);
        assert_eq!(
            app.world().resource::<Assets<FlightPcm>>().len(),
            1,
            "no allocation per frame while admission waits"
        );
        let entity = mounted.owners[&2].voice.as_ref().unwrap().entity;
        process(
            app.world_mut(),
            &mut mounted,
            &[],
            &[],
            Some(VIEW),
            false,
            0.75,
            0.01,
        )
        .unwrap();
        assert_eq!(mounted.queued_cancel, 1);
        assert!(app.world().get_entity(entity).is_err());
        assert!(app.world().resource::<Assets<FlightPcm>>().is_empty());
        release(app.world_mut(), mounted);
    }
    #[test]
    #[ignore = "Requires a real audio device. Zero PCM only, global gain zero, no window."]
    fn real_loop_tracks_owner_pauses_and_cancels_active_and_pending() {
        assert!(
            !crate::mission_audio::muted(),
            "all fixture samples and global volume are zero; GIANTS_AUDIO_MUTE=0 enables logical gain inspection"
        );
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            bevy::audio::AudioPlugin {
                global_volume: bevy::audio::GlobalVolume::new(Volume::Linear(0.)),
                ..default()
            },
        ))
        .add_audio_source::<FlightPcm>();
        let mut mounted = fixture();
        let start = Instant::now();
        let mut first = true;
        while mounted.started == 0 && start.elapsed() < Duration::from_secs(5) {
            app.update();
            let birth = if first { vec![born(1)] } else { vec![] };
            first = false;
            process(
                app.world_mut(),
                &mut mounted,
                &birth,
                &[parent(1)],
                Some(VIEW),
                false,
                0.75,
                0.01,
            )
            .unwrap();
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(mounted.started, 1);
        let entity = mounted.owners[&1].voice.as_ref().unwrap().entity;
        let held = Instant::now();
        while held.elapsed() < Duration::from_millis(350) {
            app.update();
            process(
                app.world_mut(),
                &mut mounted,
                &[],
                &[Snapshot {
                    serial: 1,
                    position: [-3., 4., 0.],
                }],
                Some(VIEW),
                false,
                0.25,
                0.01,
            )
            .unwrap();
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(mounted.started, 1);
        assert_eq!(app.world().resource::<Assets<FlightPcm>>().len(), 1);
        let sink = app.world().get::<AudioSink>(entity).unwrap();
        assert!(!sink.empty(), "100ms PCM must keep looping after350ms");
        assert!((sink.volume().to_linear() - 0.25).abs() < 1e-6);
        process(
            app.world_mut(),
            &mut mounted,
            &[],
            &[parent(1)],
            Some(VIEW),
            true,
            0.5,
            0.01,
        )
        .unwrap();
        let sink = app.world().get::<AudioSink>(entity).unwrap();
        assert!(sink.is_paused());
        assert!((sink.volume().to_linear() - 0.5).abs() < 1e-6);
        process(
            app.world_mut(),
            &mut mounted,
            &[],
            &[parent(1)],
            Some(VIEW),
            false,
            0.75,
            0.01,
        )
        .unwrap();
        assert!(!app.world().get::<AudioSink>(entity).unwrap().is_paused());
        process(
            app.world_mut(),
            &mut mounted,
            &[],
            &[],
            Some(VIEW),
            false,
            0.75,
            0.01,
        )
        .unwrap();
        assert!(app.world().get_entity(entity).is_err());
        assert!(app.world().resource::<Assets<FlightPcm>>().is_empty());
        for step in 0..3 {
            let birth = if step == 0 { vec![born(2)] } else { vec![] };
            process(
                app.world_mut(),
                &mut mounted,
                &birth,
                &[parent(2)],
                Some(VIEW),
                false,
                0.75,
                0.01,
            )
            .unwrap();
        }
        assert!(!mounted.owners[&2].voice.as_ref().unwrap().started);
        process(
            app.world_mut(),
            &mut mounted,
            &[],
            &[],
            Some(VIEW),
            false,
            0.75,
            0.01,
        )
        .unwrap();
        assert_eq!(mounted.queued_cancel, 1);
        assert!(app.world().resource::<Assets<FlightPcm>>().is_empty());
        let born_at = Instant::now();
        let mut first = true;
        while mounted.started < 2 && born_at.elapsed() < Duration::from_secs(5) {
            app.update();
            let birth = if first { vec![born(3)] } else { vec![] };
            first = false;
            process(
                app.world_mut(),
                &mut mounted,
                &birth,
                &[parent(3)],
                Some(VIEW),
                false,
                0.75,
                0.01,
            )
            .unwrap();
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(mounted.started, 2);
        let active = mounted.owners[&3].voice.as_ref().unwrap().entity;
        release(app.world_mut(), mounted);
        app.update();
        assert!(app.world().get_entity(active).is_err());
        assert!(app.world().resource::<Assets<FlightPcm>>().is_empty());
        println!(
            "flight_real_sink_probe passed=true loop=true owner_motion=true pause_resume=true gain=true active_retirement=true queued_retirement=true active_mission_cancel=true zero_pcm=true window=false"
        );
    }
}
