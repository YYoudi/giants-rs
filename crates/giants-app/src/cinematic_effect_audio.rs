//! Bounded FX readers: unresolved bindings stay2D, fixed points use software stereo.
//! Source preparation, reader creation, sink observation and fidelity differ.
mod stereo;
use bevy::{
    audio::{AddAudioSource, Volume},
    prelude::*,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Key {
    pub family: &'static str,
    pub generation: u64,
    pub director: usize,
    pub command: usize,
    pub group: usize,
    pub phase: usize,
    pub object: usize,
    pub record: usize,
}
#[derive(Clone)]
pub struct Request {
    pub key: Key,
    pub sequence: String,
    pub id: u32,
    pub binding: Binding,
    /// Acquired fixed creation point only. Missing mobile/context bindings
    /// stay unresolved rather than borrowing a render/reference translation.
    pub position: Option<[f32; 3]>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Binding {
    /// A source sound with no actor/object owner; the audio instance owns its tail.
    UnownedPosition,
    /// Resolve the live burst object, including admitted invisible supports.
    BurstObject,
    /// Native lifetime for the other services remains unresolved.
    Director,
}
#[derive(Clone, Copy, Debug)]
pub struct OwnerState {
    pub position: [f32; 3],
    pub valid: bool,
}
fn owner_stopped(owner: Option<OwnerState>) -> bool {
    giants_game::sound_owner::decide(0x2800e, owner.is_some(), owner.is_some_and(|o| o.valid)).stop
}
fn resolved_point(
    request: &Request,
    scene: Option<&crate::rigid_attachment_scene::Scene>,
) -> Option<[f32; 3]> {
    match request.binding {
        Binding::UnownedPosition => request.position,
        Binding::BurstObject => scene?
            .burst_audio_owner(&request.key)
            .filter(|o| o.valid)
            .map(|o| o.position),
        Binding::Director => None,
    }
}
fn cancel_reader(request: &Request, reset: bool, same_owner: bool) -> bool {
    reset && !(same_owner && request.binding == Binding::UnownedPosition)
}
#[derive(Resource, Default)]
pub struct Queue(pub Vec<Request>);
#[derive(Resource)]
struct State {
    root: PathBuf,
    enabled: bool,
    epoch: Option<(u64, usize, Option<u64>)>,
    world: Option<String>,
    bank: Result<BTreeMap<u32, Sample>, String>,
    seen: BTreeSet<Key>,
    seed: u32,
}
impl State {
    fn accept(
        &mut self,
        request: &Request,
        generation: u64,
        director: usize,
        sequence: &str,
    ) -> bool {
        request.key.generation == generation
            && request.key.director == director
            && request.sequence == sequence
            && self.seen.insert(request.key.clone())
    }
}
struct Sample {
    bytes: Vec<u8>,
    filename: String,
    words: [u32; 6],
    rate: u32,
    handle: Option<Handle<AudioSource>>,
    pcm: std::sync::Arc<[i16]>,
    channels: u16,
}
#[derive(Component)]
struct Playing {
    request: Request,
    queued_at: f64,
    seen_sink: bool,
    spatial: Option<SpatialReader>,
}
struct SpatialReader {
    handle: Handle<stereo::StereoSource>,
    controls: stereo::Controls,
    base_gain: f32,
    descriptor: [u32; 6],
    deferred: bool,
}
struct PreparedStereo {
    source: stereo::StereoSource,
    gain: f32,
    speed: f32,
    volume: i32,
    pan: i32,
}
fn prepare_stereo(
    sample: &Sample,
    point: [f32; 3],
    view: [f32; 16],
    seed: &mut u32,
) -> Result<PreparedStereo, &'static str> {
    let parameters = giants_game::sound_playback::one_shot(sample.words, sample.rate, seed)?;
    let (volume, pan) = spatial_controls(point, view, sample.words, parameters.gain)?;
    let source = stereo::StereoSource {
        samples: sample.pcm.clone(),
        channels: sample.channels,
        rate: sample.rate,
        controls: stereo::Controls::new(volume, pan)?,
    };
    source.validate()?;
    Ok(PreparedStereo {
        source,
        gain: parameters.gain,
        speed: parameters.frequency as f32 / sample.rate as f32,
        volume,
        pan,
    })
}
fn spatial_controls(
    point: [f32; 3],
    view: [f32; 16],
    descriptor: [u32; 6],
    base_gain: f32,
) -> Result<(i32, i32), &'static str> {
    let value = giants_game::sound_spatial::update(giants_game::sound_spatial::Input {
        flags: 0x1000e,
        hardware: false,
        fixed_position: point,
        owner_position: [0.; 3],
        previous: giants_game::sound_spatial::State {
            position: point,
            pan: 0.,
            attenuation: 1.,
        },
        distances: [descriptor[4], descriptor[5]].map(f32::from_bits),
        matrix: view,
    })?;
    let gain = giants_game::sound_device::mix_gain(base_gain, value.attenuation, 1., 1.)?;
    Ok((
        giants_game::sound_device::volume_millibels(gain)?,
        giants_game::sound_device::pan_units(value.pan)?,
    ))
}
fn remove_spatial(player: &Playing, sources: &mut Option<ResMut<Assets<stereo::StereoSource>>>) {
    if let (Some(reader), Some(sources)) = (&player.spatial, sources) {
        sources.remove(reader.handle.id());
    }
}

fn load(root: &Path, world: &str) -> Result<BTreeMap<u32, Sample>, String> {
    let source = giants_game::world_source::resolve(root, world).map_err(|e| e.to_string())?;
    let language = giants_game::stream_resources::Voices::load(root).map_err(|e| e.to_string())?;
    let bank =
        giants_game::sound_resources::load(root, &source.catalog, language.language(), false, 1)
            .map_err(|e| e.to_string())?;
    bank.samples
        .into_iter()
        .map(|(id, sample)| {
            let prepared = sample.prepared.ok_or("sample_preparation_disabled")?;
            Ok((
                id,
                Sample {
                    bytes: sample.wave,
                    filename: sample.filename,
                    words: prepared.descriptor,
                    rate: prepared.pcm.sample_rate,
                    handle: None,
                    channels: prepared.pcm.channels,
                    pcm: prepared.pcm.samples.into(),
                },
            ))
        })
        .collect::<Result<BTreeMap<_, _>, String>>()
}

pub fn install(app: &mut App, root: &Path, enabled: bool, initial_world: Option<&str>) {
    if enabled {
        app.add_audio_source::<stereo::StereoSource>();
    }
    app.init_resource::<Queue>()
        .insert_resource(State {
            root: root.into(),
            enabled,
            epoch: None,
            world: initial_world.map(str::to_owned),
            bank: initial_world
                .filter(|_| enabled)
                .map_or_else(|| Err("bank_not_loaded".into()), |world| load(root, world)),
            seen: BTreeSet::new(),
            seed: 1,
        })
        .add_systems(
            Update,
            update
                .after(crate::rigid_attachment_scene::update)
                .after(crate::mission_film::draw)
                .after(crate::cinematic_audio::update),
        );
}

fn clear_bank(state: &mut State, sources: &mut Option<ResMut<Assets<AudioSource>>>) {
    if let (Ok(bank), Some(sources)) = (&state.bank, sources) {
        for sample in bank.values() {
            if let Some(handle) = &sample.handle {
                sources.remove(handle.id());
            }
        }
    }
    state.world = None;
    state.bank = Err("bank_not_loaded".into());
}

#[allow(clippy::too_many_arguments)]
fn update(
    mut commands: Commands,
    cinematic: Option<Res<crate::cinematic_camera::Cinematic>>,
    mut world: Option<ResMut<crate::world_scene::Scene>>,
    rigid_scene: Option<Res<crate::rigid_attachment_scene::Scene>>,
    owner: Option<Res<crate::mission_opening::Owner>>,
    mut state: ResMut<State>,
    mut queue: ResMut<Queue>,
    mut sources: Option<ResMut<Assets<AudioSource>>>,
    mut stereo_sources: Option<ResMut<Assets<stereo::StereoSource>>>,
    mut players: Query<(Entity, &mut Playing, Option<&AudioSink>)>,
    time: Res<Time>,
) {
    let epoch = cinematic.as_ref().filter(|_| world.is_some()).map(|c| {
        (
            c.generation(),
            c.director_index(),
            owner.as_ref().map(|o| o.0),
        )
    });
    let world_changed = state.world.as_ref().is_some_and(|previous| {
        world
            .as_ref()
            .is_some_and(|current| previous != &current.asset.name)
    });
    let reset = state.epoch != epoch || world_changed;
    let same_owner = !world_changed
        && matches!((state.epoch, epoch), (Some((g1, _, o1)), Some((g2, _, o2))) if g1 == g2 && o1 == o2);
    if reset {
        if state.epoch.map(|(g, _, o)| (g, o)) != epoch.map(|(g, _, o)| (g, o)) {
            state.seed = 1;
        }
        for (entity, player, sink) in &mut players {
            if !cancel_reader(&player.request, reset, same_owner) {
                println!(
                    "cinematic_fx_audio_retained sequence={} id={} reason=unowned_position_reader current_director={}",
                    player.request.sequence,
                    player.request.id,
                    epoch.unwrap().1
                );
                continue;
            }
            if let Some(sink) = sink {
                sink.stop();
            }
            remove_spatial(&player, &mut stereo_sources);
            println!(
                "cinematic_fx_audio_cleanup sequence={} id={} reason=owner_or_director_changed",
                player.request.sequence, player.request.id
            );
            commands.entity(entity).despawn();
        }
        state.seen.clear();
        state.epoch = epoch;
    }
    if world_changed {
        clear_bank(&mut state, &mut sources);
    }
    let (Some(cinematic), Some(world)) = (cinematic, world.as_mut()) else {
        queue.0.clear();
        clear_bank(&mut state, &mut sources);
        return;
    };
    let requests = std::mem::take(&mut queue.0);
    let rigid_scene = rigid_scene.as_deref();
    let mut drained = true;
    for (entity, mut player, sink) in &mut players {
        if cancel_reader(&player.request, reset, same_owner) {
            continue;
        }
        if player.request.binding == Binding::BurstObject
            && player.spatial.is_some()
            && owner_stopped(rigid_scene.and_then(|s| s.burst_audio_owner(&player.request.key)))
        {
            if let Some(sink) = sink {
                sink.stop();
            }
            remove_spatial(&player, &mut stereo_sources);
            println!(
                "cinematic_fx_audio_cleanup sequence={} id={} reason=burst_owner_invalid native_cadence_verified=false",
                player.request.sequence, player.request.id
            );
            commands.entity(entity).despawn();
            continue;
        }
        if let Some(point) = resolved_point(&player.request, rigid_scene)
            && let Some(spatial) = &mut player.spatial
        {
            let result = cinematic
                .camera()
                .map_err(|e| e.to_string())
                .and_then(|camera| {
                    spatial_controls(point, camera.view, spatial.descriptor, spatial.base_gain)
                        .and_then(|(volume, pan)| spatial.controls.set(volume, pan))
                        .map_err(str::to_owned)
                });
            if let Err(reason) = result {
                if !spatial.deferred {
                    warn!("cinematic_fx_audio_stereo_update_deferred reason={reason}");
                }
                spatial.deferred = true;
            } else {
                spatial.deferred = false;
            }
        }
        if let Some(sink) = sink {
            if !player.seen_sink {
                println!(
                    "cinematic_fx_audio_sink sequence={} id={} command={} group={} phase={} object={} record={} engine_sink_observed=true spatialization_pending=true audible_fidelity_verified=false",
                    player.request.sequence,
                    player.request.id,
                    player.request.key.command,
                    player.request.key.group,
                    player.request.key.phase,
                    player.request.key.object,
                    player.request.key.record
                );
                player.seen_sink = true;
            }
            if sink.empty() {
                remove_spatial(&player, &mut stereo_sources);
                println!(
                    "cinematic_fx_audio_finished sequence={} id={}",
                    player.request.sequence, player.request.id
                );
                commands.entity(entity).despawn();
            } else {
                drained = false;
                if cinematic.paused() && !cinematic.end_reached() {
                    sink.pause();
                } else {
                    sink.play();
                }
            }
        } else {
            drained = false;
            if time.elapsed_secs_f64() - player.queued_at > 3. {
                remove_spatial(&player, &mut stereo_sources);
                warn!(
                    "cinematic_fx_audio_refused id={} reason=reader_has_no_sink",
                    player.request.id
                );
                commands.entity(entity).despawn();
            }
        }
    }
    if cinematic.end_reached() {
        world.cinematic_capture_ready &= drained && requests.is_empty();
    }
    if requests.is_empty() {
        return;
    }
    if state.world.as_deref() != Some(world.asset.name.as_str()) {
        clear_bank(&mut state, &mut sources);
        state.world = Some(world.asset.name.clone());
        if state.enabled && cinematic.preview() {
            state.bank = load(&state.root, &world.asset.name);
        }
    }
    let mut audio_seed = state.seed;
    if !state.enabled
        && requests
            .iter()
            .any(|r| resolved_point(r, rigid_scene).is_some())
        && state.bank.is_err()
    {
        // CPU-only parameter inspection is permitted without creating audio
        // assets, players or a device. It does not consume the audio RNG.
        state.bank = load(&state.root, &world.asset.name);
    }
    for request in requests {
        if !state.accept(
            &request,
            cinematic.generation(),
            cinematic.director_index(),
            cinematic.sequence(),
        ) {
            continue;
        }
        let point = resolved_point(&request, rigid_scene);
        if let Some(point) = point {
            let inspected = (|| -> Result<_, String> {
                let sample = state
                    .bank
                    .as_ref()
                    .map_err(Clone::clone)?
                    .get(&request.id)
                    .ok_or("sound_id_not_loaded")?;
                let camera = cinematic.camera().map_err(|e| e.to_string())?;
                let distances = [sample.words[4], sample.words[5]];
                let value = giants_game::sound_spatial::update(giants_game::sound_spatial::Input {
                    flags: 0x1000e,
                    hardware: false,
                    fixed_position: point,
                    owner_position: [0.; 3],
                    previous: giants_game::sound_spatial::State {
                        position: point,
                        pan: 0.,
                        attenuation: 1.,
                    },
                    distances: distances.map(f32::from_bits),
                    matrix: camera.view,
                })
                .map_err(str::to_owned)?;
                Ok((value, camera.view.map(f32::to_bits), distances))
            })();
            match inspected {
                Ok((value, view, distances)) => println!(
                    "{} sequence={} id={} command={} group={} phase={} object={} position_bits={:?} pan_bits={} attenuation_bits={} view_bits={:?} distance_bits={:?} input_only=true sink_spatialization_pending=true native_cadence_verified=false",
                    if request.binding == Binding::BurstObject {
                        "cinematic_fx_audio_owner_input"
                    } else {
                        "cinematic_fx_audio_spatial_input"
                    },
                    request.sequence,
                    request.id,
                    request.key.command,
                    request.key.group,
                    request.key.phase,
                    request.key.object,
                    value.position.map(f32::to_bits),
                    value.pan.to_bits(),
                    value.attenuation.to_bits(),
                    view,
                    distances
                ),
                Err(reason) => println!(
                    "cinematic_fx_audio_spatial_input_deferred sequence={} id={} command={} reason={reason:?} position_bits={:?} sink_spatialization_pending=true",
                    request.sequence,
                    request.id,
                    request.key.command,
                    point.map(f32::to_bits)
                ),
            }
        }
        if state.enabled
            && cinematic.preview()
            && let Some(point) = point
        {
            let resolved = (|| -> Result<_, String> {
                let sample = state
                    .bank
                    .as_ref()
                    .map_err(Clone::clone)?
                    .get(&request.id)
                    .ok_or("sound_id_not_loaded")?;
                let camera = cinematic.camera().map_err(|e| e.to_string())?;
                let prepared = prepare_stereo(sample, point, camera.view, &mut audio_seed)
                    .map_err(str::to_owned)?;
                let controls = prepared.source.controls.clone();
                let sources = stereo_sources
                    .as_mut()
                    .ok_or("stereo_audio_sources_unavailable")?;
                let handle = sources.add(prepared.source);
                Ok((
                    SpatialReader {
                        handle,
                        controls,
                        base_gain: prepared.gain,
                        descriptor: sample.words,
                        deferred: false,
                    },
                    prepared.speed,
                    prepared.gain,
                    prepared.volume,
                    prepared.pan,
                ))
            })();
            match resolved {
                Ok((spatial, speed, gain, volume, pan)) => {
                    println!(
                        "cinematic_fx_audio_queued sequence={} id={} command={} group={} phase={} object={} record={} preview_2d=false software_stereo=true source_gain_bits={} playback_speed_bits={} volume_millibels={} pan_units={} supplied_audio_rng=true spatialization_pending=true engine_sink_observed=false audible_fidelity_verified=false",
                        request.sequence,
                        request.id,
                        request.key.command,
                        request.key.group,
                        request.key.phase,
                        request.key.object,
                        request.key.record,
                        gain.to_bits(),
                        speed.to_bits(),
                        volume,
                        pan
                    );
                    let mut entity = commands.spawn((
                        AudioPlayer::<stereo::StereoSource>(spatial.handle.clone()),
                        PlaybackSettings {
                            volume: Volume::Linear(1.),
                            speed,
                            ..PlaybackSettings::ONCE
                        },
                        Playing {
                            request,
                            queued_at: time.elapsed_secs_f64(),
                            seen_sink: false,
                            spatial: Some(spatial),
                        },
                    ));
                    if let Some(owner) = &owner {
                        entity.insert(crate::mission_opening::OwnedAudio(owner.0));
                    }
                }
                Err(reason) => warn!(
                    "cinematic_fx_audio_refused sequence={} id={} reason={reason} request_satisfied=false",
                    request.sequence, request.id
                ),
            }
            continue;
        }
        let resolved = (|| -> Result<(Handle<AudioSource>, f32, f32, String), String> {
            if !state.enabled {
                return Err("audio_plugin_disabled".into());
            }
            if !cinematic.preview() {
                return Err("native_spatial_profile_pending".into());
            }
            let bank = state.bank.as_mut().map_err(|e| e.clone())?;
            let sample = bank.get_mut(&request.id).ok_or("sound_id_not_loaded")?;
            let parameters =
                giants_game::sound_playback::one_shot(sample.words, sample.rate, &mut audio_seed)
                    .map_err(str::to_owned)?;
            if sample.handle.is_none() {
                let sources = sources.as_mut().ok_or("audio_sources_unavailable")?;
                sample.handle = Some(sources.add(AudioSource {
                    bytes: sample.bytes.clone().into(),
                }));
            }
            Ok((
                sample.handle.as_ref().unwrap().clone(),
                parameters.gain,
                parameters.frequency as f32 / sample.rate as f32,
                sample.filename.clone(),
            ))
        })();
        match resolved {
            Ok((handle, volume, speed, filename)) => {
                println!(
                    "cinematic_fx_audio_queued sequence={} id={} filename={filename} command={} group={} phase={} object={} record={} preview_2d=true source_gain_bits={} playback_speed_bits={} supplied_audio_rng=true spatialization_pending=true engine_sink_observed=false audible_fidelity_verified=false",
                    request.sequence,
                    request.id,
                    request.key.command,
                    request.key.group,
                    request.key.phase,
                    request.key.object,
                    request.key.record,
                    volume.to_bits(),
                    speed.to_bits()
                );
                let mut entity = commands.spawn((
                    AudioPlayer::new(handle),
                    PlaybackSettings {
                        volume: Volume::Linear(volume),
                        speed,
                        ..PlaybackSettings::ONCE
                    },
                    Playing {
                        request,
                        queued_at: time.elapsed_secs_f64(),
                        seen_sink: false,
                        spatial: None,
                    },
                ));
                if let Some(owner) = &owner {
                    entity.insert(crate::mission_opening::OwnedAudio(owner.0));
                }
            }
            Err(reason) => warn!(
                "cinematic_fx_audio_refused sequence={} id={} reason={reason} request_satisfied=false",
                request.sequence, request.id
            ),
        }
    }
    state.seed = audio_seed;
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request() -> Request {
        Request {
            key: Key {
                family: "mixed",
                generation: 7,
                director: 2,
                command: 16,
                group: 3,
                phase: 0,
                object: 0,
                record: 64,
            },
            sequence: "synthetic".into(),
            id: 931,
            binding: Binding::Director,
            position: None,
        }
    }
    #[test]
    fn source_creation_is_consumed_once_and_old_directors_do_not_play() {
        let mut app = App::new();
        install(&mut app, Path::new("absent"), false, None);
        let mut state = app.world_mut().resource_mut::<State>();
        let request = request();
        assert!(!state.accept(&request, 8, 2, "synthetic"));
        assert!(!state.accept(&request, 7, 3, "synthetic"));
        assert!(!state.accept(&request, 7, 2, "different"));
        assert!(state.seen.is_empty());
        assert!(state.accept(&request, 7, 2, "synthetic"));
        assert!(!state.accept(&request, 7, 2, "synthetic"));
    }
    #[test]
    fn only_unowned_position_readers_survive_a_director_cut() {
        let mut request = request();
        assert!(cancel_reader(&request, true, true));
        request.binding = Binding::BurstObject;
        assert!(cancel_reader(&request, true, true));
        assert!(!cancel_reader(&request, false, true));
        assert!(!owner_stopped(Some(OwnerState {
            position: [7., -5., 3.],
            valid: true
        })));
        assert!(owner_stopped(Some(OwnerState {
            position: [7., -5., 3.],
            valid: false
        })));
        assert!(owner_stopped(None));
        request.binding = Binding::UnownedPosition;
        assert!(!cancel_reader(&request, true, true));
        assert!(cancel_reader(&request, true, false));
        assert!(!cancel_reader(&request, false, false));
    }
    #[test]
    fn removed_cinematic_releases_only_owned_readers_and_assets() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Assets<AudioSource>>()
            .init_resource::<Assets<stereo::StereoSource>>();
        install(&mut app, Path::new("absent"), false, None);
        let own = app
            .world_mut()
            .resource_mut::<Assets<AudioSource>>()
            .add(AudioSource {
                bytes: Vec::new().into(),
            });
        let other = app
            .world_mut()
            .resource_mut::<Assets<AudioSource>>()
            .add(AudioSource {
                bytes: Vec::new().into(),
            });
        let mut unowned = request();
        let controls = stereo::Controls::new(0, 0).unwrap();
        let source = stereo::StereoSource {
            samples: std::sync::Arc::from([0i16]),
            channels: 1,
            rate: 22050,
            controls: controls.clone(),
        };
        let spatial_own = app
            .world_mut()
            .resource_mut::<Assets<stereo::StereoSource>>()
            .add(source.clone());
        let spatial_other = app
            .world_mut()
            .resource_mut::<Assets<stereo::StereoSource>>()
            .add(source);
        unowned.binding = Binding::UnownedPosition;
        let reader = app
            .world_mut()
            .spawn(Playing {
                request: unowned,
                queued_at: 0.,
                seen_sink: false,
                spatial: Some(SpatialReader {
                    handle: spatial_own.clone(),
                    controls,
                    base_gain: 1.,
                    descriptor: [0; 6],
                    deferred: false,
                }),
            })
            .id();
        let unrelated = app.world_mut().spawn(AudioPlayer::new(other.clone())).id();
        {
            let mut state = app.world_mut().resource_mut::<State>();
            state.epoch = Some((7, 2, None));
            state.world = Some("synthetic".into());
            state.bank = Ok(BTreeMap::from([(
                931,
                Sample {
                    bytes: Vec::new(),
                    filename: "synthetic.wav".into(),
                    words: [0; 6],
                    rate: 22050,
                    handle: Some(own.clone()),
                    pcm: std::sync::Arc::from([]),
                    channels: 1,
                },
            )]));
        }
        app.world_mut().resource_mut::<Queue>().0.push(request());
        app.update();
        assert!(app.world().get_entity(reader).is_err());
        assert!(app.world().get_entity(unrelated).is_ok());
        assert!(
            !app.world()
                .resource::<Assets<AudioSource>>()
                .contains(own.id())
        );
        assert!(
            app.world()
                .resource::<Assets<AudioSource>>()
                .contains(other.id())
        );
        assert!(app.world().resource::<Queue>().0.is_empty());
        let spatial = app.world().resource::<Assets<stereo::StereoSource>>();
        assert!(!spatial.contains(spatial_own.id()));
        assert!(spatial.contains(spatial_other.id()));
    }
    #[test]
    fn fixed_point_uses_new_listener_without_restarting_source_samples() {
        use bevy::audio::Decodable;
        let point = [0.; 3];
        let near = Mat4::IDENTITY.to_cols_array();
        let mut far = near;
        far[12] = 100.;
        let mut descriptor = [0; 6];
        descriptor[4] = 10f32.to_bits();
        descriptor[5] = 150f32.to_bits();
        let (volume, pan) = spatial_controls(point, near, descriptor, 1.).unwrap();
        let controls = stereo::Controls::new(volume, pan).unwrap();
        let source = stereo::StereoSource {
            samples: std::sync::Arc::from([16384, 8192]),
            channels: 1,
            rate: 22050,
            controls: controls.clone(),
        };
        let mut decoder = source.decoder();
        assert_eq!(decoder.next(), Some(0.5));
        assert_eq!(decoder.next(), Some(0.5));
        let (volume, pan) = spatial_controls(point, far, descriptor, 1.).unwrap();
        controls.set(volume, pan).unwrap();
        let next = decoder.next().unwrap();
        assert!(next > 0.024 && next < 0.026);
        assert_eq!(decoder.next(), Some(next));
        assert_eq!(decoder.next(), None);
    }
    #[test]
    fn actual_stereo_factory_applies_source_gain_once_and_refuses_unresolved_sample_policies() {
        use bevy::audio::Decodable;
        let mut sample = Sample {
            bytes: Vec::new(),
            filename: "synthetic.wav".into(),
            words: [0, 0, 50f32.to_bits(), 0, 10f32.to_bits(), 150f32.to_bits()],
            rate: 22050,
            handle: None,
            pcm: std::sync::Arc::from([16384i16]),
            channels: 1,
        };
        let mut view = Mat4::IDENTITY.to_cols_array();
        view[12] = 100.;
        let mut seed = 1;
        let prepared = prepare_stereo(&sample, [0.; 3], view, &mut seed).unwrap();
        assert_eq!((prepared.gain, prepared.speed, seed), (0.5, 1., 1));
        let output = prepared.source.decoder().collect::<Vec<_>>();
        assert_eq!(output.len(), 2);
        assert!(output[0] > 0.024 && output[0] < 0.026);
        assert_eq!(output[0], output[1]);
        sample.words[0] = 1;
        assert!(prepare_stereo(&sample, [0.; 3], view, &mut seed).is_err());
    }
    #[test]
    fn unresolved_policies_do_not_become_default_full_volume_cues() {
        let base = [0, 0, 100f32.to_bits(), 0, 0, 0];
        assert_eq!(
            giants_game::sound_playback::one_shot(base, 22050, &mut 1)
                .unwrap()
                .gain,
            1.
        );
        let mut half = base;
        half[2] = 50f32.to_bits();
        assert_eq!(
            giants_game::sound_playback::one_shot(half, 22050, &mut 1)
                .unwrap()
                .gain,
            0.5
        );
        for (field, value) in [(0, 1), (0, 32), (0, 64), (2, f32::NAN.to_bits())] {
            let mut unsupported = base;
            unsupported[field] = value;
            assert!(giants_game::sound_playback::one_shot(unsupported, 22050, &mut 1).is_err());
        }
    }
}
