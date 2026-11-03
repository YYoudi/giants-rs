//! Native byte-space world target, converted once when presenting to the window.
use bevy::{
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, TextureFormat},
    shader::ShaderRef,
};
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct Presentation {
    #[texture(0)]
    #[sampler(1)]
    pub image: Handle<Image>,
    #[uniform(2)]
    pub glare: Vec4,
    #[uniform(3)]
    pub glare_viewport: Vec4,
    #[uniform(4)]
    pub frame_viewport: Vec4,
    #[uniform(5)]
    pub render_flags: Vec4,
    #[uniform(6)]
    pub fade: Vec4,
    #[uniform(7)]
    pub initial_fade: Vec4,
}
impl UiMaterial for Presentation {
    fn fragment_shader() -> ShaderRef {
        "embedded://giants_app/world_composition.wesl".into()
    }
}
pub fn spawn(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    materials: &mut Assets<Presentation>,
    order: isize,
) -> Handle<Image> {
    let mut image = Image::new_target_texture(1920, 1080, TextureFormat::Rgba8Unorm, None);
    image.sampler = bevy::image::ImageSampler::nearest();
    let target = images.add(image);
    let camera = commands
        .spawn((
            crate::world_scene::Rendered,
            Name::new("Encoded world presentation"),
            Camera2d,
            Camera { order, ..default() },
            Msaa::Off,
        ))
        .id();
    commands.spawn((
        crate::world_scene::Rendered,
        UiTargetCamera(camera),
        MaterialNode(materials.add(Presentation {
            image: target.clone(),
            glare: Vec4::ZERO,
            glare_viewport: Vec4::ZERO,
            frame_viewport: Vec4::ZERO,
            render_flags: Vec4::ZERO,
            fade: Vec4::ZERO,
            initial_fade: Vec4::ZERO,
        })),
        LayoutConfig {
            use_rounding: false,
        },
        Node {
            position_type: PositionType::Absolute,
            left: px(0.),
            top: px(0.),
            width: px(1920.),
            height: px(1080.),
            ..default()
        },
    ));
    target
}
pub fn update_policy(
    cinematic: Option<Res<crate::cinematic_camera::Cinematic>>,
    mut materials: ResMut<Assets<Presentation>>,
    mut previous: Local<Option<(String, giants_game::cinematic_render_policy::Policy)>>,
    mut previous_fade: Local<Option<(String, [u32; 5])>>,
    mut previous_initial: Local<Option<(String, [u32; 4])>>,
) {
    let policy = cinematic
        .as_ref()
        .map(|c| c.render_policy())
        .unwrap_or_default();
    let flags = Vec4::new(f32::from(policy.lens_disabled), 0., 0., 0.);
    let fade = cinematic.as_ref().map(|c| c.fade()).unwrap_or_default();
    let alpha = fade
        .alpha_byte(giants_game::arithmetic::Precision::Double53)
        .expect("validated cinematic fade envelope");
    let overlay = Vec4::new(
        f32::from(fade.color[0]),
        f32::from(fade.color[1]),
        f32::from(fade.color[2]),
        f32::from(alpha),
    ) / 255.;
    let initial = cinematic
        .as_ref()
        .map(|c| c.initial_fade())
        .unwrap_or_default();
    let initial_alpha = initial
        .alpha_byte(giants_game::arithmetic::Precision::Double53)
        .expect("validated initial fade");
    let initial_overlay = Vec4::new(0., 0., 0., f32::from(initial_alpha) / 255.);
    let changed = materials
        .iter()
        .filter(|(_, material)| {
            material.render_flags != flags
                || material.fade != overlay
                || material.initial_fade != initial_overlay
        })
        .map(|(id, _)| id)
        .collect::<Vec<_>>();
    for id in changed {
        if let Some(mut material) = materials.get_mut(id) {
            material.render_flags = flags;
            material.fade = overlay;
            material.initial_fade = initial_overlay;
        }
    }
    if let Some(cinematic) = cinematic {
        let initial_values = [
            initial.envelope.fraction.to_bits(),
            initial.envelope.target.to_bits(),
            initial.envelope.speed.to_bits(),
            u32::from(initial_alpha),
        ];
        let initial_state = (cinematic.sequence().to_owned(), initial_values);
        if previous_initial.as_ref() != Some(&initial_state) {
            println!(
                "cinematic_initial_fade sequence={} transparency_bits={} target_bits={} speed_bits={} alpha_byte={} frozen_inspection={} uniform_staged=true",
                cinematic.sequence(),
                initial_values[0],
                initial_values[1],
                initial_values[2],
                initial_alpha,
                !cinematic.realtime()
            );
            *previous_initial = Some(initial_state);
        }
        let values = [
            fade.envelope.fraction.to_bits(),
            fade.envelope.target.to_bits(),
            fade.envelope.speed.to_bits(),
            u32::from(fade.color[0]) << 16
                | u32::from(fade.color[1]) << 8
                | u32::from(fade.color[2]),
            u32::from(alpha),
        ];
        let state = (cinematic.sequence().to_owned(), values);
        if previous_fade.as_ref() != Some(&state) {
            println!(
                "cinematic_screen_fade sequence={} transparency_bits={} target_bits={} speed_bits={} color_rgb={},{},{} alpha_byte={} uniform_staged=true",
                cinematic.sequence(),
                values[0],
                values[1],
                values[2],
                fade.color[0],
                fade.color[1],
                fade.color[2],
                alpha
            );
            *previous_fade = Some(state);
        }
        let state = (cinematic.sequence().to_owned(), policy);
        if previous.as_ref() != Some(&state) {
            println!(
                "cinematic_render_policy sequence={} director={} actor_auxiliary_disabled={} lens_disabled={} auxiliary_word={} lens_mask_consumed=true actor_auxiliary_consumed=false auxiliary_word_consumed=false",
                cinematic.sequence(),
                cinematic.director_index(),
                policy.actor_auxiliary_disabled,
                policy.lens_disabled,
                policy.auxiliary_word
            );
            *previous = Some(state);
        }
    }
}
