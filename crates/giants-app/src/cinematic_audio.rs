//! Source voices and opening synchronization track. Device-mix parity remains open.
use bevy::{audio::Volume, prelude::*};
use std::{collections::BTreeMap, path::Path};

#[derive(Resource)]
pub struct Master {
    tracks: BTreeMap<String, MasterTrack>,
    name: String,
    position: f64,
    tail_clock: bool,
    generation: u64,
    revision: u64,
    director: usize,
}
struct MasterTrack {
    bytes: Vec<u8>,
    handle: Handle<AudioSource>,
}
#[derive(Component)]
pub struct PlayingMaster {
    queued_at: f64,
    seen_sink: bool,
}
impl Master {
    fn reset_clock(&mut self) {
        self.position = 0.;
        self.tail_clock = false;
    }
    fn sample_position(&mut self, source: Option<f64>, delta: f64, paused: bool) {
        if let Some(position) = source {
            if !self.tail_clock {
                self.position = position;
            }
        } else {
            if !self.tail_clock {
                info!(
                    "cinematic_preview_audio_tail=true previous_seconds={} policy=local_realtime_after_track_end native_timing_verified=false",
                    self.position
                );
                self.tail_clock = true;
            }
            if !paused {
                self.position += delta;
            }
        }
    }
    pub fn position(&self) -> f64 {
        self.position
    }
    pub fn load<'a>(
        root: &Path,
        programs: impl Iterator<Item = &'a giants_formats::flick::Program>,
    ) -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let index = giants_game::stream_resources::Voices::load(root)?;
        let mut tracks = BTreeMap::new();
        let mut first = None;
        for event in programs
            .flat_map(|p| &p.commands)
            .filter(|c| c.opcode == 52)
        {
            let name = event
                .label
                .as_ref()
                .ok_or("Synchronization track name missing")?;
            if name.is_empty() || !name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_') {
                return Err("Invalid synchronization track identifier".into());
            }
            if tracks.contains_key(name) {
                continue;
            }
            if first.is_none() {
                first = Some(name.clone());
            }
            let path = index.resolve(name)?;
            let bytes = std::fs::read(path)?;
            let pcm = giants_formats::wav::parse(&bytes)?;
            // The bounded PCM reader checks framing/channels/rate. The audio
            // decoder uses the actual header, including mono 16 kHz sync cues.
            println!("cinematic_master_input={:?}", path);
            println!("cinematic_master_source_input name={name} path={:?}", path);
            println!(
                "cinematic_master_source={} channels={} sample_rate={} samples={}",
                name,
                pcm.channels,
                pcm.sample_rate,
                pcm.samples.len()
            );
            tracks.insert(
                name.clone(),
                MasterTrack {
                    bytes,
                    handle: Handle::default(),
                },
            );
        }
        let Some(name) = first else { return Ok(None) };
        Ok(Some(Self {
            tracks,
            name,
            position: 0.,
            tail_clock: false,
            generation: 0,
            revision: 0,
            director: 0,
        }))
    }
    fn activate(&mut self, name: &str) -> Result<Handle<AudioSource>, &'static str> {
        let handle = self
            .tracks
            .get(name)
            .ok_or("Synchronization track was not prepared")?
            .handle
            .clone();
        self.name = name.to_owned();
        self.reset_clock();
        Ok(handle)
    }
}
pub fn setup_master(mut master: ResMut<Master>, mut sources: ResMut<Assets<AudioSource>>) {
    for track in master.tracks.values_mut() {
        track.handle = sources.add(AudioSource {
            bytes: track.bytes.clone().into(),
        });
    }
}
pub fn sample_master(
    mut master: ResMut<Master>,
    playing: Query<&AudioSink, With<PlayingMaster>>,
    cinematic: Res<crate::cinematic_camera::Cinematic>,
    time: Res<Time<Real>>,
) {
    let Some(sink) = playing.iter().next() else {
        return;
    };
    let source = if sink.empty() && cinematic.preview() && !cinematic.end_reached() {
        None
    } else {
        Some(sink.position().as_secs_f64())
    };
    master.sample_position(source, time.delta_secs_f64(), cinematic.paused());
}
#[allow(clippy::too_many_arguments)]
pub fn update_master(
    mut commands: Commands,
    cinematic: Res<crate::cinematic_camera::Cinematic>,
    mut master: ResMut<Master>,
    mut playing: Query<(Entity, &mut PlayingMaster, Option<&AudioSink>)>,
    time: Res<Time>,
    mut exit: MessageWriter<AppExit>,
    owner: Option<Res<crate::mission_opening::Owner>>,
) {
    if master.generation != cinematic.generation() {
        for (entity, _, _) in &mut playing {
            commands.entity(entity).despawn();
        }
        master.generation = cinematic.generation();
        master.director = cinematic.director_index();
        master.reset_clock();
        return;
    }
    for (entity, mut player, sink) in &mut playing {
        if let Some(sink) = sink {
            if master.director != cinematic.director_index() {
                info!(
                    "cinematic_master_carried director={} entity_bits={} position_seconds={}",
                    cinematic.director_index(),
                    entity.to_bits(),
                    sink.position().as_secs_f64()
                );
                master.director = cinematic.director_index();
            }
            if !player.seen_sink {
                info!(
                    "cinematic_master_sink={} active=true entity_bits={} volume_bits={}",
                    master.name,
                    entity.to_bits(),
                    sink.volume().to_linear().to_bits()
                );
                player.seen_sink = true;
            }
            if cinematic.paused() || cinematic.end_reached() {
                sink.pause();
            } else {
                sink.play();
            }
            if sink.empty() && !cinematic.end_reached() && !cinematic.preview() {
                error!("Synchronization track ended before the director");
                exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
            }
        } else if time.elapsed_secs_f64() - player.queued_at > 3. {
            error!("Synchronization track has no audio sink");
            exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
        }
    }
    if master.revision == cinematic.event_revision() {
        return;
    }
    master.revision = cinematic.event_revision();
    for event in cinematic
        .events()
        .iter()
        .filter(|e| e.kind == giants_game::flick::EventKind::SyncTrack)
    {
        if !master.tracks.contains_key(&event.name) || !cinematic.sync_started() {
            error!("Unexpected synchronization track");
            exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
            return;
        }
        info!(
            "cinematic_master_play={} tick={} gain_db100={} source_offset={}",
            event.name, event.tick, event.gain_db100, event.offset
        );
        for (entity, _, sink) in &mut playing {
            if let Some(sink) = sink {
                sink.stop();
            }
            commands.entity(entity).despawn();
        }
        let handle = master
            .activate(&event.name)
            .expect("prepared synchronization source");
        let mut entity = commands.spawn((
            AudioPlayer::new(handle),
            PlaybackSettings {
                muted: crate::mission_audio::muted(),
                volume: Volume::Decibels(event.gain_db100 as f32 / 100.),
                ..PlaybackSettings::ONCE
            },
            PlayingMaster {
                queued_at: time.elapsed_secs_f64(),
                seen_sink: false,
            },
        ));
        if let Some(owner) = &owner {
            entity.insert(crate::mission_opening::OwnedAudio(owner.0));
        }
    }
}

#[derive(Resource)]
pub struct Voices {
    sources: BTreeMap<String, Vec<u8>>,
    handles: BTreeMap<String, Handle<AudioSource>>,
    revision: u64,
    generation: u64,
}
#[derive(Component)]
pub struct PlayingVoice {
    name: String,
    seen_sink: bool,
    queued_at: f64,
    reported_end: bool,
}
impl Voices {
    pub fn load<'a>(
        root: &Path,
        programs: impl Iterator<Item = &'a giants_formats::flick::Program>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut sources = BTreeMap::new();
        let index = giants_game::stream_resources::Voices::load(root)?;
        let programs: Vec<_> = programs.collect();
        for command in programs.iter().flat_map(|p| &p.commands) {
            if command.opcode != 44 {
                continue;
            }
            let name = command.label.as_ref().ok_or("Voice name missing")?;
            if name.is_empty() || !name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_') {
                return Err("Invalid voice identifier".into());
            }
            let path = index.resolve(name)?;
            let data = std::fs::read(path)?;
            println!(
                "cinematic_voice_input name={} language={} path={:?}",
                name,
                index.language(),
                path
            );
            let pcm = giants_formats::wav::parse(&data)?;
            println!(
                "cinematic_voice_source={} sample_rate={} samples={} language={}",
                name,
                pcm.sample_rate,
                pcm.samples.len(),
                index.language()
            );
            sources.insert(name.clone(), data);
        }
        println!(
            "cinematic_voice_events=true voice_sources={} audio_master_sync={} subtitles_renderer=separate_bitmap_ui_preview native_subtitle_fidelity_verified=false",
            sources.len(),
            programs
                .iter()
                .any(|p| p.commands.iter().any(|c| c.opcode == 52))
        );
        Ok(Self {
            sources,
            handles: BTreeMap::new(),
            revision: 0,
            generation: 0,
        })
    }
}
pub fn setup(mut voices: ResMut<Voices>, mut sources: ResMut<Assets<AudioSource>>) {
    voices.handles = voices
        .sources
        .iter()
        .map(|(name, bytes)| {
            (
                name.clone(),
                sources.add(AudioSource {
                    bytes: bytes.clone().into(),
                }),
            )
        })
        .collect();
}
#[allow(clippy::too_many_arguments)]
pub fn update(
    mut commands: Commands,
    cinematic: Res<crate::cinematic_camera::Cinematic>,
    mut voices: ResMut<Voices>,
    mut playing: Query<(Entity, &mut PlayingVoice, Option<&AudioSink>)>,
    time: Res<Time>,
    mut exit: MessageWriter<AppExit>,
    mut world: ResMut<crate::world_scene::Scene>,
    owner: Option<Res<crate::mission_opening::Owner>>,
) {
    if voices.generation != cinematic.generation() {
        for (entity, _, _) in &mut playing {
            commands.entity(entity).despawn();
        }
        voices.generation = cinematic.generation();
    }
    for (entity, mut voice, sink) in &mut playing {
        if let Some(sink) = sink {
            if sink.empty() && !voice.reported_end {
                info!(
                    "cinematic_voice_finished={} position_seconds={}",
                    voice.name,
                    sink.position().as_secs_f64()
                );
                voice.reported_end = true;
                commands.entity(entity).despawn();
            }
            if !voice.seen_sink {
                info!(
                    "cinematic_voice_sink={} active=true volume_bits={}",
                    voice.name,
                    sink.volume().to_linear().to_bits()
                );
                voice.seen_sink = true;
            }
            if cinematic.paused() {
                sink.pause();
            } else {
                sink.play();
            }
        } else if time.elapsed_secs_f64() - voice.queued_at > 3. {
            error!("Voice playback has no audio sink: {}", voice.name);
            exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
        }
    }
    // A live end-of-sequence screenshot must not terminate the application
    // while the last line is still draining. Frozen visual controls are silent.
    if cinematic.end_reached() {
        world.cinematic_capture_ready = playing
            .iter()
            .all(|(_, _, sink)| sink.is_some_and(|sink| sink.empty()));
    }
    if voices.revision == cinematic.event_revision() {
        return;
    }
    voices.revision = cinematic.event_revision();
    for event in cinematic.events() {
        if event.kind != giants_game::flick::EventKind::Voice {
            continue;
        }
        let Some(handle) = voices.handles.get(&event.name) else {
            error!("Voice not prepared: {}", event.name);
            exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
            return;
        };
        info!(
            "cinematic_voice_play={} tick={} gain_db100={} source_offset={}",
            event.name, event.tick, event.gain_db100, event.offset
        );
        let mut entity = commands.spawn((
            AudioPlayer::new(handle.clone()),
            PlaybackSettings {
                muted: crate::mission_audio::muted(),
                volume: Volume::Decibels(event.gain_db100 as f32 / 100.),
                ..PlaybackSettings::ONCE
            },
            PlayingVoice {
                name: event.name.clone(),
                seen_sink: false,
                queued_at: time.elapsed_secs_f64(),
                reported_end: false,
            },
        ));
        if let Some(owner) = &owner {
            entity.insert(crate::mission_opening::OwnedAudio(owner.0));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Master, MasterTrack};
    use bevy::prelude::Handle;
    fn master() -> Master {
        Master {
            tracks: Default::default(),
            name: String::new(),
            position: 0.,
            tail_clock: false,
            generation: 0,
            revision: 0,
            director: 0,
        }
    }
    #[test]
    fn preview_clock_continues_after_empty_sink_and_respects_pause() {
        let mut master = master();
        master.sample_position(Some(10.), 0.1, false);
        master.sample_position(None, 0.25, false);
        assert_eq!(master.position(), 10.25);
        master.sample_position(None, 1., true);
        assert_eq!(master.position(), 10.25);
        master.sample_position(Some(0.), 1., false);
        assert_eq!(master.position(), 10.25);
        master.sample_position(None, 0.25, false);
        assert_eq!(master.position(), 10.5);
    }
    #[test]
    fn restart_restores_audio_clock_after_preview_tail() {
        let mut master = master();
        master.sample_position(None, 12., false);
        master.reset_clock();
        assert_eq!(master.position(), 0.);
        master.sample_position(Some(0.5), 0.1, false);
        assert_eq!(master.position(), 0.5);
        assert!(!master.tail_clock);
    }
    #[test]
    fn source_track_replacement_resets_tail_and_does_not_discard_prepared_tracks() {
        let mut master = master();
        for name in ["first", "next"] {
            master.tracks.insert(
                name.into(),
                MasterTrack {
                    bytes: vec![],
                    handle: Handle::default(),
                },
            );
        }
        master.activate("first").unwrap();
        master.sample_position(None, 12., false);
        assert!(master.tail_clock);
        master.activate("next").unwrap();
        assert_eq!(master.name, "next");
        assert_eq!(master.position(), 0.);
        assert!(!master.tail_clock);
        master.sample_position(Some(0.5), 0.1, false);
        assert_eq!(master.position(), 0.5);
        assert!(master.activate("absent").is_err());
        assert_eq!(master.position(), 0.5);
        assert_eq!(master.tracks.len(), 2);
    }
}
