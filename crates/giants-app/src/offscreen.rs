//! Explicit render diagnostic without a window or a presentation surface.
//! Retarget only window cameras; keep world/cube intermediate targets intact.
use bevy::{camera::RenderTarget, prelude::*, render::render_resource::TextureFormat};

#[derive(Resource)]
pub struct Output(pub Handle<Image>);

impl FromWorld for Output {
    fn from_world(world: &mut World) -> Self {
        let image = Image::new_target_texture(1920, 1080, TextureFormat::Rgba8UnormSrgb, None);
        Self(world.resource_mut::<Assets<Image>>().add(image))
    }
}

fn retarget(
    output: Res<Output>,
    mut commands: Commands,
    mut cameras: Query<(Entity, &mut RenderTarget), With<Camera>>,
    default_ui: Query<Entity, With<IsDefaultUiCamera>>,
) {
    let explicit_ui = !default_ui.is_empty();
    for (entity, mut target) in &mut cameras {
        if matches!(*target, RenderTarget::Window(_)) {
            *target = RenderTarget::Image(output.0.clone().into());
            // Automatic UI selection otherwise looks for a primary-window camera.
            if !explicit_ui {
                commands.entity(entity).insert(IsDefaultUiCamera);
            }
        }
    }
}

pub fn install(app: &mut App) {
    app.init_resource::<Output>()
        .add_plugins(bevy::app::ScheduleRunnerPlugin::run_loop(
            std::time::Duration::ZERO,
        ))
        .add_systems(
            PostUpdate,
            retarget.before(bevy::camera::CameraUpdateSystems),
        );
}
