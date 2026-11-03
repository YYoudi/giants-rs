//! Observed normal-startup stream, independent playback and profile music gain.
use bevy::{audio::Volume, prelude::*};
use giants_game::mission_loading::Phase;

pub struct Plugin;
impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<State>()
            .add_systems(Update, update.after(crate::frontend_scene::runtime_events));
    }
}
#[derive(Resource, Default)]
struct State {
    attempted: bool,
    source: Option<Handle<AudioSource>>,
}
#[derive(Component)]
struct Playing;
fn load(root: &std::path::Path) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let bytes = std::fs::read(root.join("Stream/Intro_island/intro_island.wav"))?;
    let pcm = giants_formats::wav::parse(&bytes)?;
    if pcm.channels != 2 || pcm.sample_rate != 22050 {
        return Err("menu stream outside observed PCM profile".into());
    }
    println!(
        "frontend_music_loaded channels={} rate={} frames={} native_loop_timing=unverified",
        pcm.channels,
        pcm.sample_rate,
        pcm.samples.len() / 2
    );
    Ok(bytes)
}
fn update(
    mut commands: Commands,
    scene: Res<crate::frontend_scene::Scene>,
    actor: Option<Res<crate::mecc_dev_scene::State>>,
    mut state: ResMut<State>,
    mut sources: ResMut<Assets<AudioSource>>,
    mut playing: Query<(Entity, Option<&mut AudioSink>), With<Playing>>,
) {
    if actor.is_some() || scene.flow.phase != Phase::Menu {
        for (entity, _) in &mut playing {
            commands.entity(entity).despawn();
            println!("frontend_music_stopped reason=leave_menu");
        }
        return;
    }
    if !state.attempted {
        state.attempted = true;
        match load(scene.data_root()) {
            Ok(bytes) => {
                state.source = Some(sources.add(AudioSource {
                    bytes: bytes.into(),
                }))
            }
            Err(error) => warn!("frontend_music_load_failed={error}"),
        }
    }
    let volume = Volume::Linear(scene.music_volume());
    if playing.is_empty() {
        if let Some(source) = &state.source {
            commands.spawn((
                Playing,
                Name::new("Frontend music"),
                AudioPlayer::new(source.clone()),
                PlaybackSettings {
                    muted: crate::mission_audio::muted(),
                    volume,
                    ..PlaybackSettings::LOOP
                },
            ));
            println!(
                "frontend_music_started preview_loop=true gain={}",
                scene.music_volume()
            );
        }
    } else {
        for (_, sink) in &mut playing {
            if let Some(mut sink) = sink
                && sink.volume() != volume
            {
                sink.set_volume(volume);
                println!("frontend_music_gain={}", scene.music_volume());
            }
        }
    }
}
