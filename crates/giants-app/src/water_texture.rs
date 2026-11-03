//! Diagnostic preview of the five-layer water texture producer, at native size.
use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, Extent3d, ShaderType, TextureDimension, TextureFormat},
    shader::ShaderRef,
};

#[derive(Clone, ShaderType, Debug)]
pub struct Parameters {
    pub phases: [Vec4; 5],
    pub colors: [Vec4; 5],
}
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct Material {
    #[texture(0)]
    #[sampler(1)]
    pub source: Handle<Image>,
    #[uniform(2)]
    pub parameters: Parameters,
}
impl UiMaterial for Material {
    fn fragment_shader() -> ShaderRef {
        "embedded://giants_app/water_texture.wesl".into()
    }
}
#[derive(Resource)]
pub struct Scene {
    pub asset: giants_game::water::TextureAsset,
    pub phases: [[f32; 2]; 5],
    pub animate: bool,
}
#[derive(Resource)]
pub struct Animation {
    motion: giants_game::water::Motion,
    material: Handle<Material>,
    steps: u64,
}
pub fn read_phases(path: &std::path::Path) -> Result<[[f32; 2]; 5], Box<dyn std::error::Error>> {
    let bytes = std::fs::read(path)?;
    parse_phases(&bytes)
}
fn parse_phases(bytes: &[u8]) -> Result<[[f32; 2]; 5], Box<dyn std::error::Error>> {
    if bytes.len() != 48 || &bytes[..8] != b"GWPH0001" {
        return Err("Invalid water phase control header/size".into());
    }
    let mut phases = [[0.; 2]; 5];
    for (value, raw) in phases
        .iter_mut()
        .flatten()
        .zip(bytes[8..].as_chunks::<4>().0)
    {
        *value = f32::from_le_bytes(*raw);
        if !value.is_finite() || value.abs() > 32. {
            return Err("Water phase outside supported control range".into());
        }
    }
    Ok(phases)
}
pub fn setup(
    mut commands: Commands,
    scene: Res<Scene>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<Material>>,
) {
    let image = source_image(&scene.asset);
    let parameters = parameters(&scene.asset, scene.phases, false);
    let material = materials.add(Material {
        source: images.add(image),
        parameters,
    });
    if scene.animate {
        commands.insert_resource(Animation {
            motion: giants_game::water::Motion {
                phases: scene.phases,
            },
            material: material.clone(),
            steps: 0,
        });
    }
    commands.spawn(Camera2d);
    commands.spawn((
        MaterialNode(material),
        LayoutConfig {
            use_rounding: false,
        },
        Node {
            position_type: PositionType::Absolute,
            left: px(832.),
            top: px(412.),
            width: px(256.),
            height: px(256.),
            ..default()
        },
    ));
}
pub fn animate(
    scene: Res<Scene>,
    animation: Option<ResMut<Animation>>,
    pipelines: Res<crate::PipelinesReady>,
    mut materials: ResMut<Assets<Material>>,
) {
    if !pipelines.0 {
        return;
    }
    let Some(mut animation) = animation else {
        return;
    };
    let Some(mut material) = materials.get_mut(&animation.material) else {
        return;
    };
    // Native producer draws first, then advances. Present the current state.
    material.parameters = parameters(&scene.asset, animation.motion.phases, false);
    animation
        .motion
        .advance(1. / 60., scene.asset.water.values[4], 0.)
        .expect("validated world water motion");
    animation.steps += 1;
    if animation.steps.is_multiple_of(60) {
        info!("water_animation_steps={}", animation.steps);
    }
}
pub fn exit(keys: Res<ButtonInput<KeyCode>>, mut events: MessageWriter<AppExit>) {
    if keys.just_pressed(KeyCode::Escape) {
        events.write(AppExit::Success);
    }
}
pub fn source_image(asset: &giants_game::water::TextureAsset) -> Image {
    let rgba16 = |words: &[u16]| -> Vec<u8> {
        words
            .iter()
            .flat_map(|word| {
                [
                    f32::from((word >> 11) & 31) / 31.,
                    f32::from((word >> 5) & 63) / 63.,
                    f32::from(word & 31) / 31.,
                    1.,
                ]
                .into_iter()
                .flat_map(|v| half::f16::from_f32(v).to_le_bytes())
            })
            .collect()
    };
    let mut image = Image::new(
        Extent3d {
            width: 256,
            height: 256,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba16(&asset.levels[0].words),
        TextureFormat::Rgba16Float,
        RenderAssetUsages::default(),
    );
    image.texture_descriptor.mip_level_count = asset.levels.len() as u32;
    image.data = Some(
        asset
            .levels
            .iter()
            .flat_map(|level| rgba16(&level.words))
            .collect(),
    );
    image.sampler = bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
        address_mode_u: bevy::image::ImageAddressMode::Repeat,
        address_mode_v: bevy::image::ImageAddressMode::Repeat,
        min_filter: bevy::image::ImageFilterMode::Linear,
        mag_filter: bevy::image::ImageFilterMode::Linear,
        mipmap_filter: bevy::image::ImageFilterMode::Linear,
        anisotropy_clamp: 16,
        ..default()
    });
    image
}
pub fn parameters(
    asset: &giants_game::water::TextureAsset,
    phases: [[f32; 2]; 5],
    encoded: bool,
) -> Parameters {
    Parameters {
        phases: phases.map(|p| Vec4::new(p[0], p[1], if encoded { 1. } else { 0. }, 0.)),
        colors: std::array::from_fn(|i| {
            let c = asset.water.colors[i + 1];
            Vec4::new(
                c[0] as f32 / 255.,
                c[1] as f32 / 255.,
                c[2] as f32 / 255.,
                1.,
            )
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_phases;
    #[test]
    fn phase_control_rejects_truncation_extra_bytes_and_nonfinite() {
        let mut raw = b"GWPH0001".to_vec();
        raw.extend([0u8; 40]);
        assert_eq!(parse_phases(&raw).unwrap(), [[0.; 2]; 5]);
        for end in 0..raw.len() {
            assert!(parse_phases(&raw[..end]).is_err());
        }
        for value in [f32::NAN, f32::INFINITY, 33.] {
            let mut invalid = raw.clone();
            invalid[8..12].copy_from_slice(&value.to_le_bytes());
            assert!(parse_phases(&invalid).is_err());
        }
        raw.push(0);
        assert!(parse_phases(&raw).is_err());
    }
}
