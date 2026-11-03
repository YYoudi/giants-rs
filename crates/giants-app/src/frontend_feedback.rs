//! Three source menu cues. Input actions are queued before page transitions.
use bevy::{audio::Volume, prelude::*};
use giants_game::{frontend::Screen, mission_loading::Phase};

pub struct Plugin;
impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<State>().add_systems(
            Update,
            feedback
                .after(crate::frontend_scene::runtime_events)
                .run_if(crate::mecc_session::frontend_available),
        );
    }
}
#[derive(Resource, Default)]
struct State {
    attempted: bool,
    cues: [Option<Handle<AudioSource>>; 3],
    previous: Option<(Screen, Option<usize>)>,
}
#[derive(Component)]
struct PlayingCue;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Cue {
    Back,
    Activate,
    Hover,
}
impl Cue {
    fn index(self) -> usize {
        match self {
            Self::Back => 0,
            Self::Activate => 1,
            Self::Hover => 2,
        }
    }
    fn id(self) -> u32 {
        match self {
            Self::Back => 1623,
            Self::Activate => 1625,
            Self::Hover => 1626,
        }
    }
}

fn load(root: &std::path::Path) -> Result<[Vec<u8>; 3], Box<dyn std::error::Error>> {
    let definitions = giants_formats::sound_catalog::parse_catalog(&std::fs::read(
        root.join("Bin/xx_permanentaliens.bin"),
    )?)?;
    let bytes = std::fs::read(root.join("Bin/xx_permanentaliens.gzp"))?;
    let archive = giants_formats::gzp::Archive::parse(&bytes)?;
    let mut cues = [Vec::new(), Vec::new(), Vec::new()];
    for cue in [Cue::Back, Cue::Activate, Cue::Hover] {
        let definition = definitions
            .iter()
            .find(|d| d.id == cue.id())
            .ok_or("Menu cue missing")?;
        let bytes = archive.read(&definition.filename)?;
        giants_formats::wav::parse(&bytes)?;
        cues[cue.index()] = bytes;
    }
    Ok(cues)
}
fn feedback(
    mut commands: Commands,
    mut scene: ResMut<crate::frontend_scene::Scene>,
    mut state: ResMut<State>,
    mut sources: ResMut<Assets<AudioSource>>,
) {
    if !state.attempted {
        state.attempted = true;
        match load(scene.data_root()) {
            Ok(bytes) => {
                state.cues = bytes.map(|bytes| {
                    Some(sources.add(AudioSource {
                        bytes: bytes.into(),
                    }))
                });
                info!(
                    "frontend_cues_loaded=true source_ids=1623,1625,1626 mixer_parity=unverified"
                );
            }
            Err(error) => warn!("frontend_selection_cue_failed={error}"),
        }
    }
    let current = scene.feedback_selection();
    let mut cues = std::mem::take(&mut scene.feedback_actions);
    let changed = scene.flow.phase == Phase::Menu
        && cues.is_empty()
        && state.previous.is_some_and(|previous| {
            previous.0 == current.0 && previous.1 != current.1 && current.1.is_some()
        });
    state.previous = (scene.flow.phase == Phase::Menu).then_some(current);
    if changed {
        cues.push(Cue::Hover);
    }
    // Transient cues finish naturally after leaving the menu, including the
    // validation that started loading. They despawn at the end of the sample.
    for cue in cues {
        let Some(source) = &state.cues[cue.index()] else {
            continue;
        };
        commands.spawn((
            PlayingCue,
            AudioPlayer::new(source.clone()),
            PlaybackSettings {
                muted: crate::mission_audio::muted(),
                volume: Volume::Linear(scene.feedback_volume()),
                ..PlaybackSettings::DESPAWN
            },
        ));
        info!(
            "frontend_cue_played={cue:?} source_id={} selected={:?}",
            cue.id(),
            current.1
        );
    }
}
