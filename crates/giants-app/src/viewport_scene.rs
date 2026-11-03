//! Apply one cinematic viewport to all world cameras; optional owned captures.
use bevy::{
    ecs::system::SystemParam,
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};
#[derive(Default)]
pub struct State {
    last: Option<UVec4>,
    captures: usize,
}
#[derive(SystemParam)]
pub struct Timing<'w> {
    cinematic: Option<Res<'w, crate::cinematic_camera::Cinematic>>,
    pipelines: Res<'w, crate::PipelinesReady>,
    cube: Option<Res<'w, crate::reflection_cube::Build>>,
}
pub fn update(
    timing: Timing,
    scene: Res<crate::world_scene::Scene>,
    mut cameras: Query<&mut Camera, (With<Camera3d>, With<crate::world_scene::Rendered>)>,
    mut materials: ResMut<Assets<crate::world_composition::Presentation>>,
    mut state: Local<State>,
    mut commands: Commands,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(cinematic) = timing.cinematic else {
        return;
    };
    if timing.cube.as_ref().is_some_and(|b| !b.playback_ready()) {
        return;
    }
    let Some(camera) = &scene.validation_camera else {
        return;
    };
    let v = camera.viewport;
    if state.last == Some(v) && !cameras.iter_mut().any(|c| c.is_added()) {
        return;
    }
    for mut camera in &mut cameras {
        camera.viewport = Some(bevy::camera::Viewport {
            physical_position: v.xy(),
            physical_size: v.zw(),
            ..default()
        });
    }
    for (_, material) in materials.iter_mut() {
        material.frame_viewport = v.as_vec4();
    }
    info!(
        "cinematic_viewport_applied director={} tick={} rect={:?} cameras={} normal_timing_verified=false",
        cinematic.director_index(),
        cinematic.tick(),
        v.to_array(),
        cameras.iter().count()
    );
    state.last = Some(v);
    if timing.pipelines.0
        && state.captures < 3
        && let Some(folder) = std::env::var_os("GIANTS_VIEWPORT_CAPTURE")
    {
        let threshold = [1080, 944, 810][state.captures];
        if v.w > threshold {
            return;
        }
        let folder = std::path::PathBuf::from(folder);
        let path = folder.join(format!(
            "viewport-{:03}-y{:04}-h{:04}.png",
            state.captures, v.y, v.w
        ));
        if path.exists() || std::fs::create_dir_all(&folder).is_err() {
            error!("Viewport capture destination unavailable");
            exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
            return;
        }
        info!(
            "cinematic_viewport_capture index={} rect={:?} file={}",
            state.captures,
            v.to_array(),
            path.display()
        );
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
        state.captures += 1;
    }
}
