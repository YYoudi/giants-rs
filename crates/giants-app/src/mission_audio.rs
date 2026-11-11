//! Ticket-owned local-player and fixed-position audio. A queued entity is not a successful backend
//! voice: start operations wait for actual Bevy AudioSink components.
use bevy::{
    audio::{AddAudioSource, Volume},
    prelude::*,
};
use giants_game::{
    arithmetic::Precision,
    audio_parameters::{self, Random},
    audio_sequence::{self, Backend, Playback, Route},
    mecc_audio::{self, JetInputs, JetRequest, Sequence},
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
mod owned;
pub(crate) use owned::OwnedAudio;
#[allow(unused_imports)] // Required-service adapter; normal mission gates remain unbound.
pub(crate) use owned::damage::SharedBackend;

pub fn muted() -> bool {
    std::env::var("GIANTS_AUDIO_MUTE").ok().as_deref() == Some("1")
}
/// Inspection may advance only frames that need no device operation. Every
/// attempted operation fails; this is never an acknowledged or queued voice.
pub(crate) struct UnavailableDevice;
impl Backend for UnavailableDevice {
    type Handle = u32;
    fn stop_sound(&mut self, _: u32, _: u64) -> Result<(), String> {
        Err("owned inspection audio device is not bound".into())
    }
    fn play(&mut self, _: u32, _: u64, _: Route) -> Result<u32, String> {
        Err("owned inspection audio device is not bound".into())
    }
    fn duration(&mut self, _: u32) -> Result<f32, String> {
        Err("owned inspection audio device is not bound".into())
    }
    fn volume(&mut self, _: u32, _: f32) -> Result<(), String> {
        Err("owned inspection audio device is not bound".into())
    }
    fn stop(&mut self, _: u32) -> Result<(), String> {
        Err("owned inspection audio device is not bound".into())
    }
}

#[derive(Message, Clone)]
pub(crate) struct Request {
    pub ticket: u64,
    pub event: Event,
}
#[derive(Clone)]
pub(crate) enum Event {
    InitialCarryPreview {
        parent: giants_game::resident_states::Id,
        child: giants_game::resident_states::Id,
    },
    PendingActorNotice {
        child: giants_game::resident_states::Id,
    },
    StopActorSound {
        owner: giants_game::resident_states::Id,
        sound: u32,
    },
    ActorSound {
        owner: giants_game::resident_states::Id,
        sound: u32,
        kind: giants_game::actor_sound::Kind,
    },
    NpcPosition {
        sound: u32,
        position: [f32; 3],
    },
    Fire(i32),
    Boost,
    Footstep(giants_game::mecc_footsteps::Cue),
    Impact(giants_game::mecc_impacts::Cue),
    Jet {
        active: bool,
        special: i32,
    },
}

#[derive(Resource, Default)]
pub(crate) struct ProbeStatus {
    pub ticket: Option<u64>,
    pub active_loop: bool,
    pub actor_started: u32,
    pub actor_active: usize,
}

pub struct Plugin;
impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        if app.is_plugin_added::<bevy::audio::AudioPlugin>() {
            app.add_audio_source::<crate::positional_pcm::Pcm>();
        } else {
            // Silent capture retains asset ownership but has no AudioOutput.
            app.init_asset::<crate::positional_pcm::Pcm>();
        }
        app.add_message::<Request>()
            .init_resource::<Engine>()
            .init_resource::<ProbeStatus>()
            .add_systems(
                Update,
                (collect, update)
                    .chain()
                    .after(crate::mission_player::PlayerUpdate)
                    .after(crate::mission_carry::AudioUpdate),
            );
        if muted() {
            app.add_systems(PreUpdate, silence);
            println!("audio_output_muted=true scope=all_bevy_audio");
        }
    }
}
fn silence(mut settings: Query<&mut PlaybackSettings>, mut sinks: Query<&mut AudioSink>) {
    for mut settings in &mut settings {
        settings.muted = true;
    }
    for mut sink in &mut sinks {
        sink.mute();
    }
}

struct Source {
    handle: Handle<AudioSource>,
    words: [u32; 6],
    rate: u32,
    frames: usize,
    pcm: std::sync::Arc<[i16]>,
    channels: u16,
}
struct SpatialVoice {
    handle: Handle<crate::positional_pcm::Pcm>,
    pan: crate::positional_pcm::Pan,
    position: [f32; 3],
    minimum: f32,
    maximum: f32,
    gain: f32,
}
struct Voice {
    owned_reservation: bool,
    deferred_parameters: bool,
    actor_owner: Option<giants_game::resident_states::Id>,
    local_owner: Option<u64>,
    actor_moved: bool,
    spatial: Option<SpatialVoice>,
    entity: Entity,
    sound: u32,
    gain: f32,
    frequency: u32,
    frames: usize,
    fraction: f32,
    ready: bool,
    started: bool,
    looping: bool,
}
enum Action {
    Actor {
        sound: u32,
        owner: giants_game::resident_states::Id,
        positional: bool,
    },
    One(u32),
    Position(u32, [f32; 3]),
    Start(Sequence),
    Interrupt,
}
struct Pending {
    action: Action,
    voices: VecDeque<u32>,
    frames: u32,
}
struct Mounted {
    ticket: u64,
    sources: BTreeMap<u32, Source>,
    unavailable: BTreeSet<u32>,
    voices: BTreeMap<u32, Voice>,
    next: u32,
    audio_random: Random,
    current: Option<Playback<u32>>,
    releasing: Vec<Playback<u32>>,
    pending: Option<Pending>,
    started: u32,
    ready: u32,
    fired: u32,
    boosts: u32,
    actor_started: u32,
}
#[derive(Resource, Default)]
struct Engine {
    mounted: Option<Mounted>,
    attempted: Option<u64>,
    requests: VecDeque<Request>,
}
impl Engine {
    fn retain_requests(&mut self, wanted: Option<u64>) {
        let failed = self.mounted.is_none() && self.attempted == wanted;
        self.requests
            .retain(|request| !failed && Some(request.ticket) == wanted);
    }
}

fn collect(mut messages: MessageReader<Request>, mut engine: ResMut<Engine>) {
    engine.requests.extend(messages.read().cloned());
}

fn mount(world: &mut World, ticket: u64) -> Result<Mounted, String> {
    let bank = world
        .resource::<crate::mission_runtime::Runtime>()
        .sound_bank(ticket)?;
    mount_bank(world, ticket, bank)
}
fn mount_source(world: &mut World, sound: giants_game::gameplay_sound_bank::Sound) -> Source {
    let handle = world
        .resource_mut::<Assets<AudioSource>>()
        .add(AudioSource {
            bytes: sound.wave.into(),
        });
    Source {
        handle,
        words: sound.binding.descriptor,
        pcm: sound.pcm.samples.clone().into(),
        channels: sound.pcm.channels,
        rate: sound.pcm.sample_rate,
        frames: sound.pcm.samples.len() / usize::from(sound.pcm.channels),
    }
}
/// Cache a true catalog miss only. Read/decode failures remain errors and do not
/// masquerade as native absence; no voice is acknowledged by source mounting.
fn ensure_actor_source(
    world: &mut World,
    mounted: &mut Mounted,
    sound: u32,
) -> Result<bool, String> {
    ensure_actor_source_with(world, mounted, sound, |world, ticket, sound| {
        world
            .resource_mut::<crate::mission_runtime::Runtime>()
            .resolve_actor_sound(ticket, sound)
    })
}
fn ensure_actor_source_with(
    world: &mut World,
    mounted: &mut Mounted,
    sound: u32,
    load: impl FnOnce(
        &mut World,
        u64,
        u32,
    ) -> Result<Option<giants_game::gameplay_sound_bank::Sound>, String>,
) -> Result<bool, String> {
    if mounted.sources.contains_key(&sound) {
        return Ok(true);
    }
    if mounted.unavailable.contains(&sound) {
        return Ok(false);
    }
    let source = load(world, mounted.ticket, sound)?;
    let Some(source) = source else {
        mounted.unavailable.insert(sound);
        println!(
            "mission_actor_audio_unavailable ticket={} sound={sound} catalog_id_absent=true backend_started=false",
            mounted.ticket
        );
        return Ok(false);
    };
    mounted.sources.insert(sound, mount_source(world, source));
    println!(
        "mission_actor_audio_source_loaded ticket={} sound={sound} on_demand=true backend_started=false",
        mounted.ticket
    );
    Ok(true)
}
fn mount_bank(
    world: &mut World,
    ticket: u64,
    bank: giants_game::gameplay_sound_bank::Bank,
) -> Result<Mounted, String> {
    let mut sources = BTreeMap::new();
    for (id, sound) in bank.sounds {
        sources.insert(id, mount_source(world, sound));
    }
    println!(
        "mission_audio_mounted ticket={ticket} sources={} scope=local_player_and_fixed_position rng_seed=1 live_rng_phase_verified=false",
        sources.len()
    );
    Ok(Mounted {
        ticket,
        sources,
        unavailable: BTreeSet::new(),
        voices: BTreeMap::new(),
        next: 0,
        audio_random: Random(1),
        current: None,
        releasing: Vec::new(),
        pending: None,
        started: 0,
        ready: 0,
        fired: 0,
        boosts: 0,
        actor_started: 0,
    })
}
fn release(world: &mut World, mounted: Mounted) {
    let count = mounted.voices.len();
    for voice in mounted.voices.values() {
        if let Some(sink) = world.get::<AudioSink>(voice.entity) {
            sink.stop();
        }
        if let Some(spatial) = &voice.spatial {
            world
                .resource_mut::<Assets<crate::positional_pcm::Pcm>>()
                .remove(spatial.handle.id());
        }
        let _ = world.despawn(voice.entity);
    }
    let spatial_remaining = world
        .get_resource::<Assets<crate::positional_pcm::Pcm>>()
        .map_or(0, |a| a.len());
    assert_eq!(
        spatial_remaining, 0,
        "positional assets survived mission release"
    );
    println!(
        "mission_audio_spatial_released ticket={} remaining_assets={spatial_remaining}",
        mounted.ticket
    );
    let mut sources = world.resource_mut::<Assets<AudioSource>>();
    for source in mounted.sources.values() {
        sources.remove(source.handle.id());
    }
    assert!(
        mounted
            .sources
            .values()
            .all(|source| !sources.contains(source.handle.id()))
    );
    println!(
        "mission_audio_released ticket={} voices={} sources={} started={} sink_ready={} fired={} boosts={} pending={} remaining_assets=0",
        mounted.ticket,
        count,
        mounted.sources.len(),
        mounted.started,
        mounted.ready,
        mounted.fired,
        mounted.boosts,
        mounted.pending.is_some()
    );
}

fn stage(
    world: &mut World,
    mounted: &mut Mounted,
    sound: u32,
    effects: f32,
    position: Option<[f32; 3]>,
) -> Result<u32, String> {
    let source = mounted
        .sources
        .get(&sound)
        .ok_or_else(|| format!("sound {sound} not prepared"))?;
    if source.words[0] & !1 != 0 {
        return Err("sound flags outside local-player playback profile".into());
    }
    let parameters = audio_parameters::prepare(
        source.words,
        source.rate,
        &mut mounted.audio_random,
        Precision::Double53,
    )?;
    if parameters.frequency == 0 {
        return Err("zero sound playback frequency".into());
    }
    let looping = parameters.flags & 1 != 0;
    if position.is_some_and(|p| !p.into_iter().all(f32::is_finite)) {
        return Err("nonfinite positional sound point".into());
    }
    if looping && position.is_some() {
        return Err("looping positional source outside impact profile".into());
    }
    let pcm = position
        .map(|_| crate::positional_pcm::Pcm::new(source.pcm.clone(), source.channels, source.rate))
        .transpose()?;
    let settings = PlaybackSettings {
        speed: parameters.frequency as f32 / source.rate as f32,
        volume: Volume::Linear(if muted() || position.is_some() {
            0.
        } else {
            parameters.gain * effects
        }),
        paused: true,
        muted: muted(),
        ..if looping {
            PlaybackSettings::LOOP
        } else {
            PlaybackSettings::ONCE
        }
    };
    let entity = world
        .spawn((
            crate::mission_runtime::MissionOwned(mounted.ticket),
            settings,
        ))
        .id();
    let spatial = if let Some(position) = position {
        let pcm = pcm.expect("validated positional PCM");
        let pan = pcm.pan.clone();
        let handle = world
            .resource_mut::<Assets<crate::positional_pcm::Pcm>>()
            .add(pcm);
        world.entity_mut(entity).insert(AudioPlayer(handle.clone()));
        Some(SpatialVoice {
            handle,
            pan,
            position,
            minimum: parameters.minimum_distance,
            maximum: parameters.maximum_distance,
            gain: 0.,
        })
    } else {
        world
            .entity_mut(entity)
            .insert(AudioPlayer::new(source.handle.clone()));
        None
    };
    mounted.next += 1;
    let id = mounted.next;
    mounted.voices.insert(
        id,
        Voice {
            owned_reservation: false,
            deferred_parameters: false,
            actor_owner: None,
            local_owner: None,
            actor_moved: false,
            spatial,
            entity,
            sound,
            gain: parameters.gain,
            frequency: parameters.frequency,
            frames: source.frames,
            fraction: 1.,
            ready: false,
            started: false,
            looping,
        },
    );
    println!(
        "mission_audio_queued ticket={} voice={id} sound={sound} frequency={} loop={looping} paused=true",
        mounted.ticket, parameters.frequency
    );
    Ok(id)
}

struct Device<'a> {
    world: &'a mut World,
    mounted: &'a mut Mounted,
    staged: &'a mut VecDeque<u32>,
    effects: f32,
}
impl Device<'_> {
    fn start_sound(&mut self, sound: u32, positional: bool) -> Result<u32, String> {
        let id = self
            .staged
            .pop_front()
            .ok_or("audio voice was not staged")?;
        let voice = self
            .mounted
            .voices
            .get_mut(&id)
            .ok_or("staged audio voice missing")?;
        if voice.sound != sound || !voice.ready || voice.spatial.is_some() != positional {
            return Err("audio voice has no actual ready sink or route mismatch".into());
        }
        let gain = audio_parameters::instance_gain(
            self.effects,
            voice.gain,
            voice.spatial.as_ref().map_or(1., |s| s.gain),
            voice.fraction,
            Precision::Double53,
        )?;
        let mut sink = self
            .world
            .get_mut::<AudioSink>(voice.entity)
            .ok_or("ready audio sink disappeared")?;
        sink.set_volume(Volume::Linear(if muted() { 0. } else { gain }));
        sink.play();
        voice.started = true;
        self.mounted.started += 1;
        if voice.actor_owner.is_some() {
            self.mounted.actor_started += 1;
        }
        println!(
            "mission_audio_started ticket={} voice={id} sound={sound} sink_observed=true muted={} route={}",
            self.mounted.ticket,
            muted(),
            if positional { "position" } else { "local" }
        );
        Ok(id)
    }
}
impl Backend for Device<'_> {
    type Handle = u32;
    fn stop_sound(&mut self, sound: u32, owner: u64) -> Result<(), String> {
        if owner != self.mounted.ticket {
            return Err("audio owner mismatch".into());
        }
        for voice in self
            .mounted
            .voices
            .values()
            .filter(|v| v.sound == sound && v.started)
        {
            if let Some(sink) = self.world.get::<AudioSink>(voice.entity) {
                sink.stop();
            }
        }
        Ok(())
    }
    fn play(&mut self, sound: u32, owner: u64, route: Route) -> Result<u32, String> {
        if route != Route::Local || owner != self.mounted.ticket {
            return Err("audio route outside local-player profile".into());
        }
        self.start_sound(sound, false)
    }
    fn duration(&mut self, id: u32) -> Result<f32, String> {
        let voice = self
            .mounted
            .voices
            .get(&id)
            .ok_or("duration voice missing")?;
        if !voice.started || self.world.get::<AudioSink>(voice.entity).is_none() {
            return Err("duration requires a started real sink".into());
        }
        let duration = (voice.frames as f64 / f64::from(voice.frequency)) as f32;
        println!(
            "mission_audio_duration ticket={} voice={id} seconds_bits={} source=pcm_frames_over_effective_frequency",
            self.mounted.ticket,
            duration.to_bits()
        );
        Ok(duration)
    }
    fn volume(&mut self, id: u32, fraction: f32) -> Result<(), String> {
        if id == 0 {
            return Ok(());
        }
        let voice = self
            .mounted
            .voices
            .get_mut(&id)
            .ok_or("fade voice missing")?;
        voice.fraction = fraction;
        let gain = audio_parameters::instance_gain(
            self.effects,
            voice.gain,
            1.,
            fraction,
            Precision::Double53,
        )?;
        if let Some(mut sink) = self.world.get_mut::<AudioSink>(voice.entity) {
            sink.set_volume(Volume::Linear(if muted() { 0. } else { gain }));
        }
        Ok(())
    }
    fn stop(&mut self, id: u32) -> Result<(), String> {
        if let Some(voice) = self.mounted.voices.remove(&id) {
            if let Some(sink) = self.world.get::<AudioSink>(voice.entity) {
                sink.stop();
            }
            if let Some(spatial) = &voice.spatial {
                self.world
                    .resource_mut::<Assets<crate::positional_pcm::Pcm>>()
                    .remove(spatial.handle.id());
            }
            let _ = self.world.despawn(voice.entity);
            println!(
                "mission_audio_stopped ticket={} voice={id}",
                self.mounted.ticket
            );
        }
        Ok(())
    }
}

fn actor_position(
    world: &World,
    ticket: u64,
    owner: giants_game::resident_states::Id,
) -> Option<[f32; 3]> {
    world
        .get_resource::<crate::mission_runtime::Runtime>()?
        .actor_audio_position(ticket, owner)
}
/// Exact typed-owner/sample selection. Stop does not walk attachment parents,
/// load a new source, consume random numbers or acknowledge an unstarted voice.
fn stop_actor_sound(
    world: &mut World,
    mounted: &mut Mounted,
    owner: giants_game::resident_states::Id,
    sound: u32,
) -> Result<Vec<u32>, String> {
    let ids = mounted
        .voices
        .iter()
        .filter_map(|(&id, v)| (v.actor_owner == Some(owner) && v.sound == sound).then_some(id))
        .collect::<Vec<_>>();
    let pending = mounted.pending.as_ref().is_some_and(|p|
        matches!(p.action, Action::Actor { owner: who, sound: cue, .. } if who==owner && cue==sound));
    if pending {
        mounted.pending = None;
    }
    for &id in &ids {
        Device {
            world,
            mounted,
            staged: &mut VecDeque::new(),
            effects: 0.,
        }
        .stop(id)?;
    }
    println!(
        "mission_actor_audio_stopped ticket={} owner={owner:?} sound={sound} voices={} pending_cancelled={pending}",
        mounted.ticket,
        ids.len()
    );
    Ok(ids)
}
fn drain_actor_stops(
    world: &mut World,
    mounted: &mut Mounted,
    requests: &mut VecDeque<Request>,
) -> Result<(), String> {
    while let Some(Request {
        ticket,
        event: Event::StopActorSound { owner, sound },
    }) = requests.front()
    {
        let (ticket, owner, sound) = (*ticket, *owner, *sound);
        requests.pop_front();
        if ticket == mounted.ticket {
            stop_actor_sound(world, mounted, owner, sound)?;
        }
    }
    Ok(())
}
fn process(
    world: &mut World,
    mounted: &mut Mounted,
    requests: &mut VecDeque<Request>,
    paused: bool,
    effects: f32,
    dt: f32,
) -> Result<(), String> {
    // A missing device must not accumulate an unbounded history of shots.
    // This is an explicit new-runtime failure budget, not a native timing rule.
    if requests.len() > 256 {
        return Err("audio command backlog exceeds 256 while admission is pending".into());
    }
    let view = world
        .query::<&crate::mission_player::Controller>()
        .iter(world)
        .find(|c| c.prepared(mounted.ticket))
        .map(|c| c.audio_view());
    // Preserve the existing retire-before-update behavior through the same
    // scoped command used by actor callbacks. Never stop another actor's cue.
    let mut retired = Vec::new();
    for v in mounted.voices.values() {
        if let Some(owner) = v.actor_owner
            && actor_position(world, mounted.ticket, owner).is_none()
            && !retired.contains(&(owner, v.sound))
        {
            retired.push((owner, v.sound));
        }
    }
    if let Some(Pending {
        action: Action::Actor { owner, sound, .. },
        ..
    }) = &mounted.pending
        && actor_position(world, mounted.ticket, *owner).is_none()
        && !retired.contains(&(*owner, *sound))
    {
        retired.push((*owner, *sound));
    }
    for (owner, sound) in retired {
        let mut command = VecDeque::from([Request {
            ticket: mounted.ticket,
            event: Event::StopActorSound { owner, sound },
        }]);
        drain_actor_stops(world, mounted, &mut command)?;
    }
    drain_actor_stops(world, mounted, requests)?;
    let mut finished = Vec::new();
    for (&id, voice) in &mut mounted.voices {
        if let Some(owner) = voice.actor_owner {
            let Some(position) = actor_position(world, mounted.ticket, owner) else {
                if let Some(sink) = world.get::<AudioSink>(voice.entity) {
                    sink.stop();
                }
                finished.push(id);
                println!(
                    "mission_actor_audio_retired ticket={} voice={id}",
                    mounted.ticket
                );
                continue;
            };
            if let Some(spatial) = &mut voice.spatial {
                if voice.started
                    && !voice.actor_moved
                    && spatial.position != position
                    && world.get::<AudioSink>(voice.entity).is_some()
                {
                    println!(
                        "mission_actor_audio_follow ticket={} voice={id} before={:?} after={position:?} actual_sink=true",
                        mounted.ticket, spatial.position
                    );
                    voice.actor_moved = true;
                }
                spatial.position = position;
            }
        }
        if let Some(spatial) = &mut voice.spatial
            && !voice.deferred_parameters
        {
            if let Some(matrix) = view {
                if voice.owned_reservation && voice.actor_owner.is_some() {
                    let state =
                        giants_game::sound_spatial::update(giants_game::sound_spatial::Input {
                            flags: 0x2800e,
                            hardware: false,
                            fixed_position: [0.; 3],
                            owner_position: spatial.position,
                            previous: giants_game::sound_spatial::State {
                                position: spatial.position,
                                pan: 0.,
                                attenuation: 0.,
                            },
                            distances: [spatial.minimum, spatial.maximum],
                            matrix,
                        })?;
                    spatial
                        .pan
                        .set(giants_game::sound_device::pan_units(state.pan)?)?;
                    spatial.gain = state.attenuation;
                } else {
                    let values = giants_game::audio_spatial::update(
                        &giants_game::audio_spatial::Inputs {
                            matrix,
                            position: spatial.position,
                            flags: 0x1000e,
                            previous_pan: 0.,
                            previous_gain: 0.,
                            minimum: spatial.minimum,
                            maximum: spatial.maximum,
                        },
                        Precision::Double53,
                    )?;
                    spatial.pan.set(giants_game::audio_spatial::pan_units(
                        values.pan,
                        Precision::Double53,
                    )?)?;
                    spatial.gain = values.gain;
                    if !voice.started {
                        println!(
                            "mission_audio_spatial ticket={} voice={id} distance_bits={} pan_bits={} gain_bits={} point={:?} matrix_bits={:?} minimum_bits={} maximum_bits={} pan_units={}",
                            mounted.ticket,
                            values.distance.to_bits(),
                            values.pan.to_bits(),
                            values.gain.to_bits(),
                            spatial.position,
                            matrix.map(f32::to_bits),
                            spatial.minimum.to_bits(),
                            spatial.maximum.to_bits(),
                            giants_game::audio_spatial::pan_units(values.pan, Precision::Double53)?
                        );
                    }
                }
            } else {
                spatial.gain = 0.;
            }
        }
        if let Some(mut sink) = world.get_mut::<AudioSink>(voice.entity) {
            if !voice.ready {
                voice.ready = true;
                mounted.ready += 1;
                if !voice.owned_reservation {
                    println!(
                        "mission_audio_sink_ready ticket={} voice={id} speed={} muted={}",
                        mounted.ticket,
                        sink.speed(),
                        sink.is_muted()
                    );
                } else {
                    println!(
                        "mission_audio_reservation_ready ticket={} voice={id} sound={} paused=true",
                        mounted.ticket, voice.sound
                    );
                }
            }
            if voice.started {
                if paused {
                    sink.pause();
                } else {
                    sink.play();
                }
                let gain = audio_parameters::instance_gain(
                    effects,
                    voice.gain,
                    voice.spatial.as_ref().map_or(1., |s| s.gain),
                    voice.fraction,
                    Precision::Double53,
                )?;
                sink.set_volume(Volume::Linear(if muted() { 0. } else { gain }));
                if !voice.looping && sink.empty() {
                    finished.push(id);
                }
            }
        }
    }
    // Duration has already been consumed before a release one-shot can finish.
    for id in finished {
        if let Some(voice) = mounted.voices.remove(&id) {
            if let Some(spatial) = &voice.spatial {
                world
                    .resource_mut::<Assets<crate::positional_pcm::Pcm>>()
                    .remove(spatial.handle.id());
            }
            let _ = world.despawn(voice.entity);
        }
    }
    if paused {
        return Ok(());
    }
    for _ in 0..64 {
        drain_actor_stops(world, mounted, requests)?;
        if mounted.pending.is_none() {
            let Some(request) = requests.pop_front() else {
                break;
            };
            if request.ticket != mounted.ticket {
                continue;
            }
            let action = match request.event {
                Event::StopActorSound { owner, sound } => {
                    stop_actor_sound(world, mounted, owner, sound)?;
                    None
                }
                Event::InitialCarryPreview { parent, child } => {
                    let (services, [before, choice, after]) = world
                        .resource_mut::<crate::mission_runtime::Runtime>()
                        .take_pickup_services(mounted.ticket, parent, child)?;
                    for service in services.into_iter().rev() {
                        match service {
                            giants_game::resident_states::PickupService::Sound {
                                owner,
                                id,
                                kind,
                            } => requests.push_front(Request {
                                ticket: mounted.ticket,
                                event: Event::ActorSound {
                                    owner,
                                    sound: id,
                                    kind,
                                },
                            }),
                            giants_game::resident_states::PickupService::LocalNotice { child } => {
                                requests.push_front(Request {
                                    ticket: mounted.ticket,
                                    event: Event::PendingActorNotice { child },
                                })
                            }
                        }
                    }
                    println!(
                        "mission_actor_audio_preview ticket={} choice={choice} seed_before={before} seed_after={} live_rng_phase_verified=false local_notice_executed=false",
                        mounted.ticket, after
                    );
                    None
                }
                Event::PendingActorNotice { child } => {
                    println!(
                        "mission_actor_notice_pending ticket={} child={child:?} executed=false ready=false",
                        mounted.ticket
                    );
                    None
                }
                Event::ActorSound { owner, sound, kind } => {
                    let route = world
                        .resource::<crate::mission_runtime::Runtime>()
                        .actor_sound_route(mounted.ticket, owner, kind, true)?;
                    let route = if route.is_some() && !ensure_actor_source(world, mounted, sound)? {
                        None
                    } else {
                        route
                    };
                    match route {
                        Some(giants_game::actor_sound::Route::Local(root)) => Some(Action::Actor {
                            sound,
                            owner: root,
                            positional: false,
                        }),
                        Some(giants_game::actor_sound::Route::Attached(Some(owner))) => {
                            Some(Action::Actor {
                                sound,
                                owner,
                                positional: true,
                            })
                        }
                        Some(giants_game::actor_sound::Route::Attached(None)) => {
                            return Err("unbound actor audio owner has no playback backend".into());
                        }
                        None => {
                            println!(
                                "mission_actor_audio_skipped ticket={} sound={sound} source_or_route_unavailable=true backend_started=false",
                                mounted.ticket
                            );
                            None
                        }
                    }
                }
                Event::Fire(projectile) => {
                    mounted.fired += 1;
                    let draw = world
                        .resource_mut::<crate::mission_runtime::Runtime>()
                        .variant_choice(mounted.ticket)?;
                    Some(Action::One(mecc_audio::pistol_request(projectile, draw)?))
                }
                Event::NpcPosition { sound, position } => {
                    println!(
                        "mission_npc_sound_request ticket={} sound={sound} route=position playback_acknowledged=false",
                        mounted.ticket
                    );
                    if ensure_actor_source(world, mounted, sound)? {
                        Some(Action::Position(sound, position))
                    } else {
                        None
                    }
                }
                Event::Footstep(cue) => {
                    println!(
                        "mission_audio_footstep ticket={} slot={} offset={} effect={} marker_bits={} sound={} previous_bits={} current_bits={} duration_bits={} reverse={} wrapped={} wet={} material={:?} seed_before={} seed_after={} route=local_player_root visual_effect_created=false",
                        mounted.ticket,
                        cue.slot,
                        cue.source_offset,
                        cue.effect,
                        cue.marker_bits,
                        cue.sound,
                        cue.window.previous.to_bits(),
                        cue.window.current.to_bits(),
                        cue.duration.to_bits(),
                        cue.window.reverse,
                        cue.window.wrapped,
                        cue.wet,
                        cue.material.as_deref().unwrap_or("-"),
                        cue.seeds[0],
                        cue.seeds[1]
                    );
                    Some(Action::One(cue.sound))
                }
                Event::Impact(cue) => {
                    println!(
                        "mission_audio_impact ticket={} sound={} effect={} point={:?} seed_before={} seed_after={} route=position collision_profile=development visual_effect_created=false",
                        mounted.ticket,
                        cue.sound,
                        cue.effect,
                        cue.position,
                        cue.seeds[0],
                        cue.seeds[1]
                    );
                    Some(Action::Position(cue.sound, cue.position))
                }
                Event::Boost => {
                    mounted.boosts += 1;
                    Some(Action::One(0x5b3))
                }
                Event::Jet { active, special } => match mecc_audio::jet_request(JetInputs {
                    thrust_active: active,
                    sequence_exists: mounted.current.is_some(),
                    special_item: special,
                    ..Default::default()
                }) {
                    JetRequest::None => None,
                    JetRequest::Start(sequence) => Some(Action::Start(sequence)),
                    JetRequest::Interrupt => Some(Action::Interrupt),
                },
            };
            let Some(action) = action else { continue };
            let sounds = match action {
                Action::One(id) | Action::Position(id, _) | Action::Actor { sound: id, .. } => {
                    vec![id]
                }
                Action::Start(s) => vec![s.start, s.continuous],
                Action::Interrupt => vec![
                    mounted
                        .current
                        .as_ref()
                        .ok_or("jet sequence missing")?
                        .definition
                        .release,
                ],
            };
            let mut voices = VecDeque::new();
            for sound in sounds {
                let id = stage(
                    world,
                    mounted,
                    sound,
                    effects,
                    match action {
                        Action::Position(_, p) => Some(p),
                        Action::Actor {
                            owner,
                            positional: true,
                            ..
                        } => Some(
                            actor_position(world, mounted.ticket, owner)
                                .ok_or("actor sound position not published")?,
                        ),
                        _ => None,
                    },
                )?;
                if let Action::Actor { owner, .. } = action {
                    mounted.voices.get_mut(&id).unwrap().actor_owner = Some(owner);
                }
                voices.push_back(id);
            }
            mounted.pending = Some(Pending {
                action,
                voices,
                frames: 0,
            });
        }
        // A following stop can cancel this staged actor request even when no
        // device has produced its sink. Later start requests remain ordered.
        if matches!(
            requests.front(),
            Some(Request {
                event: Event::StopActorSound { .. },
                ..
            })
        ) {
            continue;
        }
        let ready = mounted.pending.as_ref().unwrap().voices.iter().all(|id| {
            mounted
                .voices
                .get(id)
                .is_some_and(|v| v.ready && (v.spatial.is_none() || view.is_some()))
        });
        if !ready {
            let pending = mounted.pending.as_mut().unwrap();
            pending.frames += 1;
            if pending.frames == 600 {
                warn!(
                    "mission_audio_waiting_for_sink ticket={} frames=600 backend_success=false",
                    mounted.ticket
                );
            }
            break;
        }
        let mut pending = mounted.pending.take().unwrap();
        match pending.action {
            Action::Actor {
                sound,
                owner,
                positional,
            } => {
                let voice = Device {
                    world,
                    mounted,
                    staged: &mut pending.voices,
                    effects,
                }
                .start_sound(sound, positional)?;
                println!(
                    "mission_actor_audio_started ticket={} voice={voice} sound={sound} owner={owner:?} route={} actual_sink=true actor_backend_ready=false",
                    mounted.ticket,
                    if positional { "attached" } else { "local_root" }
                );
            }
            Action::Position(sound, _) => {
                Device {
                    world,
                    mounted,
                    staged: &mut pending.voices,
                    effects,
                }
                .start_sound(sound, true)?;
            }
            Action::One(sound) => {
                let owner = mounted.ticket;
                Device {
                    world,
                    mounted,
                    staged: &mut pending.voices,
                    effects,
                }
                .play(sound, owner, Route::Local)?;
            }
            Action::Start(definition) => {
                let mut sequence = Playback::new(definition, mounted.ticket, Route::Local)?;
                sequence.update(
                    0.,
                    false,
                    false,
                    Precision::Double53,
                    &mut Device {
                        world,
                        mounted,
                        staged: &mut pending.voices,
                        effects,
                    },
                )?;
                mounted.current = Some(sequence);
            }
            Action::Interrupt => {
                let mut sequence = mounted.current.take().ok_or("jet sequence missing")?;
                sequence.interrupt(&mut Device {
                    world,
                    mounted,
                    staged: &mut pending.voices,
                    effects,
                })?;
                mounted.releasing.push(sequence);
            }
        }
        assert!(pending.voices.is_empty());
    }
    let mut releasing = std::mem::take(&mut mounted.releasing);
    for sequence in &mut releasing {
        sequence.update(
            dt,
            false,
            false,
            Precision::Double53,
            &mut Device {
                world,
                mounted,
                staged: &mut VecDeque::new(),
                effects,
            },
        )?;
    }
    releasing.retain(|s| s.phase() != audio_sequence::Phase::Finished);
    mounted.releasing = releasing;
    Ok(())
}

fn update(world: &mut World) {
    *world.resource_mut::<ProbeStatus>() = ProbeStatus::default();
    let wanted = world
        .resource::<crate::mission_runtime::Runtime>()
        .active_ticket();
    let frontend = world.resource::<crate::frontend_scene::Scene>();
    let paused = !(world
        .resource::<crate::mission_showcase::State>()
        .controls(frontend.flow.phase)
        || owned::requested()
            && std::env::var("GIANTS_MISSION_REPLAY").as_deref() == Ok("owned-player-inspection"));
    let effects = frontend.feedback_volume();
    let dt = world.resource::<Time>().delta_secs().min(0.1);
    world.resource_scope(|world, mut engine: Mut<Engine>| {
        if engine
            .mounted
            .as_ref()
            .is_some_and(|m| Some(m.ticket) != wanted)
        {
            release(world, engine.mounted.take().unwrap());
        }
        engine.retain_requests(wanted);
        if engine.mounted.is_none() {
            let Some(ticket) = wanted else { return };
            if engine.attempted == Some(ticket) {
                return;
            }
            engine.attempted = Some(ticket);
            match mount(world, ticket) {
                Ok(mounted) => engine.mounted = Some(mounted),
                Err(error) => {
                    error!("mission_audio_prepare_failed ticket={ticket} error={error}");
                    return;
                }
            }
        }
        let mut mounted = engine.mounted.take().unwrap();
        if owned::requested()
            && let Err(error) = (|| -> Result<(), String> {
                owned::reserve(world, &mut mounted)?;
                owned::reserve_damage(world, &mut mounted)?;
                Ok(())
            })()
        {
            error!(
                "mission_owned_audio_reserve_failed ticket={} error={error}",
                mounted.ticket
            );
            release(world, mounted);
            return;
        }
        if (owned::requested()
            || world
                .resource::<crate::mission_runtime::Runtime>()
                .normal_landing_supported(mounted.ticket))
            && let Err(error) = (|| -> Result<(), String> {
                let cues = world
                    .resource::<crate::mission_runtime::Runtime>()
                    .owned_impact_cues(mounted.ticket)?;
                owned::impact::reserve(world, &mut mounted, &cues)
            })()
        {
            error!(
                "mission_impact_audio_reserve_failed ticket={} error={error}",
                mounted.ticket
            );
            release(world, mounted);
            return;
        }
        if let Err(error) = process(
            world,
            &mut mounted,
            &mut engine.requests,
            paused,
            effects,
            dt,
        ) {
            error!(
                "mission_audio_failed ticket={} error={error}",
                mounted.ticket
            );
            release(world, mounted);
        } else {
            let active_loop = mounted.voices.values().any(|v| {
                v.looping
                    && v.started
                    && world
                        .get::<AudioSink>(v.entity)
                        .is_some_and(|sink| !sink.empty() && !sink.is_paused())
            });
            *world.resource_mut::<ProbeStatus>() = ProbeStatus {
                ticket: Some(mounted.ticket),
                active_loop,
                actor_started: mounted.actor_started,
                actor_active: mounted
                    .voices
                    .values()
                    .filter(|v| {
                        v.actor_owner.is_some()
                            && v.started
                            && world
                                .get::<AudioSink>(v.entity)
                                .is_some_and(|s| !s.empty() && !s.is_paused())
                    })
                    .count(),
            };
            engine.mounted = Some(mounted);
        }
    });
}

#[cfg(test)]
pub(crate) fn install_zero_reserved_fixture(world: &mut World, ticket: u64) {
    let mut mounted = tests::fixture(world, ticket);
    assert!(
        mounted
            .sources
            .values()
            .all(|s| s.pcm.iter().all(|v| *v == 0))
    );
    owned::reserve(world, &mut mounted).unwrap();
    world.insert_resource(Engine {
        mounted: Some(mounted),
        ..default()
    });
}
#[cfg(test)]
pub(crate) fn refill_zero_reserved_fixture(world: &mut World) {
    world.resource_scope(|world, mut engine: Mut<Engine>| {
        owned::reserve(world, engine.mounted.as_mut().unwrap()).unwrap();
    });
}
#[cfg(test)]
pub(crate) fn finish_zero_reserved_fixture(world: &mut World) -> u32 {
    let mounted = world.resource_mut::<Engine>().mounted.take().unwrap();
    let started = mounted.started;
    release(world, mounted);
    assert!(world.resource::<Assets<AudioSource>>().is_empty());
    started
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn silent_wave() -> Vec<u8> {
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
        wave
    }
    pub(super) fn fixture(world: &mut World, ticket: u64) -> Mounted {
        let mut sources = BTreeMap::new();
        for id in [
            0x5f7, 0x5f8, 0x5b0, 0x5b1, 0x5b2, 0x5b3, 0x5b5, 0x5b6, 0x5b7,
        ] {
            let handle = world
                .resource_mut::<Assets<AudioSource>>()
                .add(AudioSource {
                    bytes: silent_wave().into(),
                });
            sources.insert(
                id,
                Source {
                    handle,
                    words: [
                        u32::from(id == 0x5b1 || id == 0x5b6),
                        0,
                        100f32.to_bits(),
                        0,
                        20f32.to_bits(),
                        0,
                    ],
                    pcm: vec![0; 800].into(),
                    channels: 1,
                    rate: 8000,
                    frames: 800,
                },
            );
        }
        Mounted {
            ticket,
            sources,
            unavailable: BTreeSet::new(),
            voices: BTreeMap::new(),
            next: 0,
            audio_random: Random(1),
            current: None,
            releasing: Vec::new(),
            pending: None,
            started: 0,
            ready: 0,
            fired: 0,
            boosts: 0,
            actor_started: 0,
        }
    }
    fn drive(
        app: &mut App,
        mounted: &mut Mounted,
        requests: &mut VecDeque<Request>,
        paused: bool,
        volume: f32,
    ) {
        app.update();
        process(app.world_mut(), mounted, requests, paused, volume, 0.01).unwrap();
    }
    fn await_state(
        app: &mut App,
        mounted: &mut Mounted,
        requests: &mut VecDeque<Request>,
        check: impl Fn(&Mounted) -> bool,
    ) {
        let started = Instant::now();
        while !check(mounted) && started.elapsed() < Duration::from_secs(5) {
            drive(app, mounted, requests, false, 0.75);
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(
            check(mounted),
            "real audio sinks did not reach the required state"
        );
    }

    fn audio_test_owners(ticket: u64) -> Vec<giants_game::resident_states::Id> {
        let program = giants_formats::class_program::parse(&[0xff]).unwrap();
        let seed = giants_game::state_storage::Seed::from_program_for(
            &program,
            giants_game::state_storage::Profile::Debug,
            Precision::Double53,
        )
        .unwrap();
        let mut session = giants_game::resident_states::Session::new(ticket);
        (0..5)
            .map(|_| {
                session
                    .register(
                        0,
                        &seed,
                        &[],
                        giants_game::object_lists::DirectFlags::default(),
                    )
                    .unwrap()
            })
            .collect()
    }
    #[test]
    fn actor_stop_cancels_pending_voice_without_touching_peer_or_later_start() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<AudioSource>();
        let mut mounted = fixture(app.world_mut(), 92);
        let owners = audio_test_owners(92);
        let cue = 0x5b0;
        let first = stage(app.world_mut(), &mut mounted, cue, 0.75, None).unwrap();
        let peer = stage(app.world_mut(), &mut mounted, cue, 0.75, None).unwrap();
        let local = stage(app.world_mut(), &mut mounted, cue, 0.75, None).unwrap();
        mounted.voices.get_mut(&first).unwrap().actor_owner = Some(owners[0]);
        mounted.voices.get_mut(&peer).unwrap().actor_owner = Some(owners[1]);
        mounted.pending = Some(Pending {
            action: Action::Actor {
                owner: owners[0],
                sound: cue,
                positional: false,
            },
            voices: VecDeque::from([first]),
            frames: 100,
        });
        let seed = mounted.audio_random.0;
        let mut requests = VecDeque::from([
            Request {
                ticket: 93,
                event: Event::StopActorSound {
                    owner: owners[1],
                    sound: cue,
                },
            },
            Request {
                ticket: 92,
                event: Event::StopActorSound {
                    owner: owners[0],
                    sound: cue,
                },
            },
            Request {
                ticket: 92,
                event: Event::ActorSound {
                    owner: owners[0],
                    sound: cue,
                    kind: giants_game::actor_sound::Kind::Owner,
                },
            },
        ]);
        drain_actor_stops(app.world_mut(), &mut mounted, &mut requests).unwrap();
        assert!(mounted.pending.is_none());
        assert!(!mounted.voices.contains_key(&first));
        assert!(mounted.voices.contains_key(&peer) && mounted.voices.contains_key(&local));
        assert_eq!(requests.len(), 1, "a later explicit start must stay queued");
        assert_eq!(mounted.audio_random.0, seed);
        assert_eq!((mounted.ready, mounted.started), (0, 0));
        assert!(
            stop_actor_sound(app.world_mut(), &mut mounted, owners[0], cue)
                .unwrap()
                .is_empty()
        );
        release(app.world_mut(), mounted);
        assert!(app.world().resource::<Assets<AudioSource>>().is_empty());
    }
    #[test]
    #[ignore = "Requires explicit retained exact-D stop cases; no device or window."]
    fn reference_actor_stop_selection_matches_native_cases() {
        let path = std::path::PathBuf::from(
            std::env::var_os("GIANTS_ACTOR_STOP_CASES").expect("explicit native cases required"),
        );
        let data = std::fs::read_to_string(path).unwrap();
        let rows = data.lines().collect::<Vec<_>>();
        assert_eq!(rows.len(), 1024);
        let owners = audio_test_owners(94);
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<AudioSource>();
        let mut stopped = 0;
        for row in rows {
            let words = row
                .split_whitespace()
                .map(|s| s.parse::<u32>().unwrap())
                .collect::<Vec<_>>();
            let (sound, owner, count) = (words[0], words[1] as usize, words[2] as usize);
            assert!(count <= 64);
            let mut mounted = fixture(app.world_mut(), 94);
            for v in words[3..3 + count * 3].as_chunks::<3>().0 {
                let id = stage(app.world_mut(), &mut mounted, v[0], 0.75, None).unwrap();
                mounted.voices.get_mut(&id).unwrap().actor_owner = if v[1] == 0 {
                    None
                } else {
                    Some(owners[v[1] as usize - 1])
                };
            }
            let actual =
                stop_actor_sound(app.world_mut(), &mut mounted, owners[owner - 1], sound).unwrap();
            let expected = &words[4 + count * 3..];
            assert_eq!(words[3 + count * 3] as usize, expected.len());
            assert_eq!(actual, expected);
            assert_eq!(mounted.voices.len() + actual.len(), count);
            stopped += actual.len();
            release(app.world_mut(), mounted);
            assert!(app.world().resource::<Assets<AudioSource>>().is_empty());
        }
        assert_eq!(stopped, 324);
        println!(
            "actor_audio_stop_native_probe passed=true cases=1024 voices=8166 selected=324 exact_typed_owner=true queued_voice_control=true device_stop_abi_verified=false window=false"
        );
    }
    #[test]
    #[ignore = "Requires owned oracle, real audio device, GIANTS_AUDIO_MUTE=1; no window."]
    fn reference_actor_stop_preserves_peer_real_sink() {
        assert!(muted());
        let root = std::path::PathBuf::from(
            std::env::var_os("GIANTS_AUDIO_REFERENCE_ROOT").expect("owned oracle required"),
        );
        let bin = root.join("Bin");
        let classes =
            giants_game::catalog_registry::WorldClassRegistry::load(&bin, "w_Story2.bin").unwrap();
        let archives = giants_game::catalog_registry::ResourceIndex::scan(&bin).unwrap();
        let (language, _) =
            giants_game::gameplay_sound_bank::voice_language(&archives, "English").unwrap();
        let registry =
            giants_game::gameplay_sound_bank::Registry::prepare(&classes.visits, false).unwrap();
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            bevy::audio::AudioPlugin {
                global_volume: bevy::audio::GlobalVolume::new(Volume::Linear(0.)),
                ..default()
            },
        ));
        let mut mounted = mount_bank(
            app.world_mut(),
            95,
            giants_game::gameplay_sound_bank::Bank {
                sounds: BTreeMap::new(),
            },
        )
        .unwrap();
        let cue = 1587;
        assert!(
            ensure_actor_source_with(app.world_mut(), &mut mounted, cue, |_, _, sound| registry
                .resolve(&archives, &language, sound))
            .unwrap()
        );
        assert_eq!(mounted.sources[&cue].words[0], 1);
        let owners = audio_test_owners(95);
        let mut ids = Vec::new();
        for target in 1..=2 {
            let id = stage(app.world_mut(), &mut mounted, cue, 0.75, None).unwrap();
            mounted.pending = Some(Pending {
                action: Action::One(cue),
                voices: VecDeque::from([id]),
                frames: 0,
            });
            await_state(&mut app, &mut mounted, &mut VecDeque::new(), |m| {
                m.started == target
            });
            ids.push(id);
        }
        for (&id, &owner) in ids.iter().zip(&owners) {
            mounted.voices.get_mut(&id).unwrap().actor_owner = Some(owner);
        }
        let first = mounted.voices[&ids[0]].entity;
        let peer = mounted.voices[&ids[1]].entity;
        let source = mounted.sources[&cue].handle.clone();
        assert_eq!(
            stop_actor_sound(app.world_mut(), &mut mounted, owners[0], cue).unwrap(),
            vec![ids[0]]
        );
        assert!(app.world().get_entity(first).is_err());
        let sink = app
            .world()
            .get::<AudioSink>(peer)
            .expect("peer sink preserved");
        assert!(!sink.empty() && !sink.is_paused());
        assert_eq!(sink.volume().to_linear(), 0.);
        assert!(
            app.world()
                .resource::<Assets<AudioSource>>()
                .contains(source.id())
        );
        release(app.world_mut(), mounted);
        assert!(app.world().get_entity(peer).is_err());
        assert!(app.world().resource::<Assets<AudioSource>>().is_empty());
        println!(
            "actor_audio_stop_sink_probe passed=true sound=1587 started=2 stopped=1 peer_preserved=true sample_preserved_until_release=true actual_sinks=true muted=true assets_after_release=0 nonpositional_lifecycle_control=true native_spatial_fidelity_verified=false window=false"
        );
    }
    #[test]
    fn demanded_actor_sources_cache_absence_but_not_failures_and_wait_for_backend() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<AudioSource>();
        let mut mounted = fixture(app.world_mut(), 91);
        let initial = app.world().resource::<Assets<AudioSource>>().len();
        let seed = mounted.audio_random.0;
        assert!(
            !ensure_actor_source_with(app.world_mut(), &mut mounted, 1200, |_, ticket, id| {
                assert_eq!((ticket, id), (91, 1200));
                Ok(None)
            })
            .unwrap()
        );
        assert!(
            !ensure_actor_source_with(app.world_mut(), &mut mounted, 1200, |_, _, _| panic!(
                "catalog miss should be cached"
            ))
            .unwrap()
        );
        assert!(
            ensure_actor_source_with(app.world_mut(), &mut mounted, 1201, |_, _, _| Err(
                "declared WAV failed to load".into()
            ))
            .is_err()
        );
        assert!(!mounted.unavailable.contains(&1201));
        assert!(
            ensure_actor_source_with(app.world_mut(), &mut mounted, 1201, |_, _, _| {
                let wave = silent_wave();
                let pcm = giants_formats::wav::parse(&wave).unwrap();
                Ok(Some(giants_game::gameplay_sound_bank::Sound {
                    binding: giants_formats::sound_catalog::Binding {
                        filename: "synthetic.wav".into(),
                        descriptor: [0, 0, 100f32.to_bits(), 0, 20f32.to_bits(), 0],
                    },
                    location: giants_game::catalog_registry::Location {
                        archive: "synthetic.gzp".into(),
                        member: "synthetic.wav".into(),
                    },
                    wave,
                    pcm,
                }))
            })
            .unwrap()
        );
        assert!(
            ensure_actor_source_with(app.world_mut(), &mut mounted, 1201, |_, _, _| panic!(
                "prepared source should be reused"
            ))
            .unwrap()
        );
        assert_eq!(
            mounted.audio_random.0, seed,
            "resource lookup must not consume audio RNG"
        );
        assert_eq!(
            app.world().resource::<Assets<AudioSource>>().len(),
            initial + 1
        );
        assert!(mounted.voices.is_empty());
        stage(app.world_mut(), &mut mounted, 1201, 0.75, None).unwrap();
        assert_eq!((mounted.ready, mounted.started), (0, 0));
        assert!(
            mounted
                .voices
                .values()
                .all(|v| app.world().get::<AudioSink>(v.entity).is_none())
        );
        release(app.world_mut(), mounted);
        assert!(app.world().resource::<Assets<AudioSource>>().is_empty());
    }
    #[test]
    fn no_audio_backend_never_acknowledges_and_backlog_is_bounded() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<AudioSource>();
        let mut mounted = fixture(app.world_mut(), 90);
        let mut requests = VecDeque::from([Request {
            ticket: 90,
            event: Event::Boost,
        }]);
        for _ in 0..4 {
            drive(&mut app, &mut mounted, &mut requests, false, 0.75);
        }
        assert_eq!(mounted.voices.len(), 1);
        assert_eq!(mounted.ready, 0);
        assert_eq!(mounted.started, 0);
        assert!(mounted.pending.is_some());
        requests.extend((0..257).map(|_| Request {
            ticket: 90,
            event: Event::Boost,
        }));
        assert!(
            process(
                app.world_mut(),
                &mut mounted,
                &mut requests,
                false,
                0.75,
                0.01
            )
            .is_err()
        );
        let entities: Vec<_> = mounted.voices.values().map(|v| v.entity).collect();
        release(app.world_mut(), mounted);
        assert!(entities.iter().all(|e| app.world().get_entity(*e).is_err()));
        assert!(app.world().resource::<Assets<AudioSource>>().is_empty());
        println!(
            "audio_no_backend_probe passed=true queued=1 ready=0 started=0 backlog_failure=true assets_after_release=0"
        );
    }

    #[test]
    #[ignore = "Requires owned oracle, real audio device, GIANTS_AUDIO_MUTE=1; no window."]
    fn reference_on_demand_actor_source_reaches_muted_sink() {
        assert!(muted());
        let root = std::path::PathBuf::from(
            std::env::var_os("GIANTS_AUDIO_REFERENCE_ROOT").expect("owned oracle required"),
        );
        let bin = root.join("Bin");
        let classes =
            giants_game::catalog_registry::WorldClassRegistry::load(&bin, "w_Story2.bin").unwrap();
        let archives = giants_game::catalog_registry::ResourceIndex::scan(&bin).unwrap();
        let (language, _) =
            giants_game::gameplay_sound_bank::voice_language(&archives, "English").unwrap();
        let registry =
            giants_game::gameplay_sound_bank::Registry::prepare(&classes.visits, false).unwrap();
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            bevy::audio::AudioPlugin {
                global_volume: bevy::audio::GlobalVolume::new(Volume::Linear(0.)),
                ..default()
            },
        ));
        let mut mounted = mount_bank(
            app.world_mut(),
            79,
            giants_game::gameplay_sound_bank::Bank {
                sounds: BTreeMap::new(),
            },
        )
        .unwrap();
        assert!(mounted.sources.is_empty());
        let seed = mounted.audio_random.0;
        assert!(
            ensure_actor_source_with(app.world_mut(), &mut mounted, 1250, |_, ticket, sound| {
                assert_eq!(ticket, 79);
                registry.resolve(&archives, &language, sound)
            })
            .unwrap()
        );
        assert_eq!(mounted.audio_random.0, seed);
        assert!(mounted.sources[&1250].pcm.iter().any(|v| *v != 0));
        assert!(
            ensure_actor_source_with(app.world_mut(), &mut mounted, 1250, |_, _, _| panic!(
                "source already loaded"
            ))
            .unwrap()
        );
        assert_eq!((mounted.ready, mounted.started), (0, 0));
        let voice = stage(app.world_mut(), &mut mounted, 1250, 0.75, None).unwrap();
        mounted.pending = Some(Pending {
            action: Action::One(1250),
            voices: VecDeque::from([voice]),
            frames: 0,
        });
        let mut requests = VecDeque::new();
        await_state(&mut app, &mut mounted, &mut requests, |m| m.started == 1);
        let entity = mounted.voices[&voice].entity;
        let sink = app
            .world()
            .get::<AudioSink>(entity)
            .expect("real sink required");
        assert_eq!(sink.volume().to_linear(), 0.);
        assert!(app.world().get::<PlaybackSettings>(entity).unwrap().muted);
        release(app.world_mut(), mounted);
        assert!(app.world().resource::<Assets<AudioSource>>().is_empty());
        assert!(app.world().get_entity(entity).is_err());
        println!(
            "actor_audio_demand_probe passed=true sound=1250 initially_preloaded=false cache_reused=true actual_sink=true muted=true window=false assets_after_release=0 nonpositional_readiness_control=true native_attachment_mix_verified=false"
        );
    }
    #[test]
    #[ignore = "Requires explicit oracle root, real audio device, GIANTS_AUDIO_MUTE=1; global gain zero, no window."]
    fn reference_special18_bank_reaches_muted_real_sinks_and_retires() {
        assert!(
            muted(),
            "real assets require GIANTS_AUDIO_MUTE=1 before loading"
        );
        let root = std::path::PathBuf::from(
            std::env::var_os("GIANTS_AUDIO_REFERENCE_ROOT")
                .expect("explicit owned oracle root required"),
        );
        let bin = root.join("Bin");
        let classes =
            giants_game::catalog_registry::WorldClassRegistry::load(&bin, "w_story1.bin").unwrap();
        let archives = giants_game::catalog_registry::ResourceIndex::scan(&bin).unwrap();
        assert!(archives.issues.is_empty());
        let bank = giants_game::gameplay_sound_bank::Bank::prepare(
            &classes.visits,
            &archives,
            "French",
            false,
            [
                0x5f7, 0x5f8, 0x5b0, 0x5b1, 0x5b2, 0x5b3, 0x5b5, 0x5b6, 0x5b7,
            ],
        )
        .unwrap();
        assert_eq!(bank.sounds.len(), 9);
        let fingerprint = |samples: &[i16]| {
            samples
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .fold(0xcbf29ce484222325u64, |hash, byte| {
                    (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
                })
        };
        let expected: BTreeMap<_,_> = bank.sounds.iter().map(|(&id, sound)| {
            assert!(sound.pcm.samples.iter().any(|v| *v != 0), "real payload, not silent fixture");
            let hash = fingerprint(&sound.pcm.samples);
            println!("reference_bank_payload id={id} name={} rate={} channels={} samples={} fingerprint={hash} words={:?}", sound.binding.filename, sound.pcm.sample_rate, sound.pcm.channels, sound.pcm.samples.len(), sound.binding.descriptor);
            (id, (hash, sound.pcm.samples.len(), sound.pcm.sample_rate, sound.pcm.channels, sound.binding.descriptor))
        }).collect();
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            bevy::audio::AudioPlugin {
                global_volume: bevy::audio::GlobalVolume::new(Volume::Linear(0.)),
                ..default()
            },
        ));
        let mut mounted = mount_bank(app.world_mut(), 78, bank).unwrap();
        for (&id, source) in &mounted.sources {
            let expected = expected[&id];
            assert_eq!(
                (
                    fingerprint(&source.pcm),
                    source.pcm.len(),
                    source.rate,
                    source.channels,
                    source.words
                ),
                expected
            );
        }
        let mut requests = VecDeque::from([Request {
            ticket: 78,
            event: Event::Jet {
                active: true,
                special: 18,
            },
        }]);
        await_state(&mut app, &mut mounted, &mut requests, |m| {
            m.voices
                .values()
                .any(|v| v.sound == 0x5b6 && v.looping && v.started)
        });
        let loop_entity = mounted
            .voices
            .values()
            .find(|v| v.sound == 0x5b6 && v.looping)
            .unwrap()
            .entity;
        assert!(mounted.started >= 2);
        let count = mounted.next;
        let duration =
            mounted.sources[&0x5b6].frames as f64 / f64::from(mounted.sources[&0x5b6].rate);
        assert!(duration > 0. && duration < 10.);
        let hold = Instant::now();
        while hold.elapsed().as_secs_f64() < duration + 0.2 {
            drive(&mut app, &mut mounted, &mut requests, false, 0.75);
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(mounted.next, count, "holding must not allocate per frame");
        let sink = app.world().get::<AudioSink>(loop_entity).unwrap();
        assert!(!sink.empty());
        assert_eq!(sink.volume().to_linear(), 0.);
        assert!(
            app.world()
                .get::<PlaybackSettings>(loop_entity)
                .unwrap()
                .muted
        );
        requests.push_back(Request {
            ticket: 78,
            event: Event::Jet {
                active: false,
                special: 18,
            },
        });
        await_state(&mut app, &mut mounted, &mut requests, |m| {
            m.current.is_none() && m.pending.is_none()
        });
        assert!(mounted.started >= 3);
        await_state(&mut app, &mut mounted, &mut requests, |m| {
            m.releasing.is_empty()
        });
        assert!(app.world().get_entity(loop_entity).is_err());
        let count = mounted.started;
        release(app.world_mut(), mounted);
        app.update();
        assert!(app.world().resource::<Assets<AudioSource>>().is_empty());
        println!(
            "reference_special18_probe passed=true prepared=9 payloads_bound=true started={count} loop_wrap=true release_fade=true actual_sinks=true muted=true global_gain_zero=true window=false pickup_verified=false"
        );
    }
    #[test]
    #[ignore = "Requires a real audio device. Synthetic zero PCM only; no window or game assets."]
    fn real_sinks_pause_volume_special_loop_and_active_cancellation() {
        assert!(
            !muted(),
            "use GIANTS_AUDIO_MUTE=0 for logical-gain inspection; all fixture PCM samples are zero"
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
        let mut mounted = fixture(app.world_mut(), 77);
        let mut requests = VecDeque::from([Request {
            ticket: 77,
            event: Event::Jet {
                active: true,
                special: 18,
            },
        }]);
        await_state(&mut app, &mut mounted, &mut requests, |m| {
            m.current.is_some()
        });
        assert_eq!(mounted.started, 2);
        assert_eq!(mounted.ready, 2);
        let loop_entity = mounted
            .voices
            .values()
            .find(|v| v.sound == 0x5b6)
            .unwrap()
            .entity;
        let held = Instant::now();
        while held.elapsed() < Duration::from_millis(350) {
            drive(&mut app, &mut mounted, &mut requests, false, 0.25);
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(
            mounted.next, 2,
            "holding the jet must not allocate per frame"
        );
        let sink = app.world().get::<AudioSink>(loop_entity).unwrap();
        assert!(
            !sink.empty(),
            "the 100ms source must still loop after 350ms"
        );
        assert!((sink.volume().to_linear() - 0.25).abs() < 1e-6);
        drive(&mut app, &mut mounted, &mut requests, true, 0.5);
        let sink = app.world().get::<AudioSink>(loop_entity).unwrap();
        assert!(sink.is_paused());
        assert!((sink.volume().to_linear() - 0.5).abs() < 1e-6);
        drive(&mut app, &mut mounted, &mut requests, false, 0.25);
        assert!(
            !app.world()
                .get::<AudioSink>(loop_entity)
                .unwrap()
                .is_paused()
        );

        requests.push_back(Request {
            ticket: 77,
            event: Event::Jet {
                active: false,
                special: 18,
            },
        });
        await_state(&mut app, &mut mounted, &mut requests, |m| {
            m.current.is_none() && m.pending.is_none()
        });
        assert_eq!(mounted.releasing.len(), 1);
        let remaining = mounted.releasing[0].remaining().to_bits();
        drive(&mut app, &mut mounted, &mut requests, true, 0.125);
        assert_eq!(
            mounted.releasing[0].remaining().to_bits(),
            remaining,
            "pause must freeze the fade"
        );
        await_state(&mut app, &mut mounted, &mut requests, |m| {
            m.releasing.is_empty()
        });
        assert!(app.world().get_entity(loop_entity).is_err());

        requests.extend(
            [
                Event::Jet {
                    active: true,
                    special: -1,
                },
                Event::Boost,
                Event::Fire(17),
                Event::Fire(17),
            ]
            .into_iter()
            .map(|event| Request { ticket: 77, event }),
        );
        await_state(&mut app, &mut mounted, &mut requests, |m| {
            m.current.is_some() && m.pending.is_none() && m.started == 8
        });
        assert!(requests.is_empty());
        assert!(mounted.voices.values().any(|v| {
            v.looping
                && v.started
                && app
                    .world()
                    .get::<AudioSink>(v.entity)
                    .is_some_and(|s| !s.empty())
        }));
        let entities: Vec<_> = mounted.voices.values().map(|v| v.entity).collect();
        release(app.world_mut(), mounted);
        app.update();
        assert!(entities.iter().all(|e| app.world().get_entity(*e).is_err()));
        assert!(app.world().resource::<Assets<AudioSource>>().is_empty());

        let mut mounted = fixture(app.world_mut(), 78);
        let mut requests = VecDeque::from([Request {
            ticket: 78,
            event: Event::Boost,
        }]);
        process(
            app.world_mut(),
            &mut mounted,
            &mut requests,
            false,
            0.75,
            0.01,
        )
        .unwrap();
        assert_eq!(mounted.started, 0);
        assert!(mounted.pending.is_some());
        assert!(
            mounted
                .voices
                .values()
                .all(|v| app.world().get::<AudioSink>(v.entity).is_none())
        );
        let entities: Vec<_> = mounted.voices.values().map(|v| v.entity).collect();
        release(app.world_mut(), mounted);
        app.update();
        assert!(entities.iter().all(|e| app.world().get_entity(*e).is_err()));
        assert!(app.world().resource::<Assets<AudioSource>>().is_empty());
        println!(
            "audio_real_sink_probe passed=true special18=true loop_wrap=true no_per_frame_allocation=true pause_resume=true volume_change=true fade_paused=true active_cancel=true queued_cancel=true synthetic_zero_pcm=true window=false"
        );
    }
    #[test]
    fn failed_audio_attempt_drops_commands_but_a_fresh_ticket_can_start() {
        let request = |ticket| Request {
            ticket,
            event: Event::Boost,
        };
        let mut engine = Engine {
            attempted: Some(1),
            requests: VecDeque::from([request(1), request(2)]),
            ..Default::default()
        };
        engine.retain_requests(Some(1));
        assert!(engine.requests.is_empty());
        engine.requests.extend([request(1), request(2)]);
        engine.retain_requests(Some(2));
        assert_eq!(engine.requests.len(), 1);
        assert_eq!(engine.requests[0].ticket, 2);
        engine.retain_requests(None);
        assert!(engine.requests.is_empty());
    }
}
