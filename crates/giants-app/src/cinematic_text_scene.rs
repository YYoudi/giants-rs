//! Source text blocks and measured bitmap glyphs, presented as a UI preview.
//! Native GPU sampling, blend/fade ordering and timing parity remain unverified.
use bevy::{asset::RenderAssetUsages, prelude::*, render::render_resource::*};
use giants_game::{arithmetic::Precision, font_atlas::Atlas, font_rasterizer::Font};
use std::path::{Path, PathBuf};

#[derive(Resource, Default)]
pub(crate) struct CaptureImage(pub Option<Handle<Image>>);

struct State {
    root: PathBuf,
    table: Option<Result<giants_formats::gtext::TextTable, String>>,
    font: Option<Result<Font, &'static str>>,
    atlas: Atlas,
    image: Option<Handle<Image>>,
    nodes: Vec<Entity>,
    camera: Option<Entity>,
    last_notice: Option<(String, usize, String)>,
}
impl State {
    fn text(&mut self, frame: &giants_game::cinematic_text::Frame) -> Result<Vec<u8>, String> {
        let root = &self.root;
        let table = self.table.get_or_insert_with(|| {
            let voices = giants_game::stream_resources::Voices::load(root).map_err(|e| e.to_string())?;
            let language = if voices.language() == "Shared" { "English" } else { voices.language() };
            let bytes = std::fs::read(root.join("Bin").join(format!("GText{language}.bin")))
                .map_err(|e| e.to_string())?;
            let table = giants_formats::gtext::parse(&bytes).map_err(|e| e.to_string())?;
            println!("cinematic_text_language={} policy=installed_voice_policy native_selection_verified=false", language);
            Ok(table)
        });
        Ok(frame
            .resolve(table.as_ref().map_err(Clone::clone)?)
            .to_vec())
    }
    fn prepare(&mut self, text: &[u8]) -> Result<bool, String> {
        let font = self.font.get_or_insert_with(|| {
            let font = Font::new()?;
            println!("cinematic_text_font={:?} source_request=windows_D_24 native_gpu_verified=false", font.face());
            Ok(font)
        }).as_mut().map_err(|e| e.to_string())?;
        let mut changed = false;
        // Source startup warmup; do not preload a whole language table.
        for code in std::iter::once(b'A').chain(text.iter().copied().take_while(|c| *c != 0)) {
            if code == b'\n' {
                continue;
            }
            if self.atlas.glyphs[code as usize].width == 0 {
                let mask = font.render(code).map_err(str::to_owned)?;
                self.atlas.insert(code, &mask.bits).map_err(str::to_owned)?;
                // An error later in this string must not leave the next block
                // reusing a GPU image from before these successful insertions.
                self.image = None;
                changed = true;
            }
        }
        Ok(changed)
    }
    fn notice(&mut self, sequence: &str, offset: usize, message: String) {
        let key = (sequence.to_owned(), offset, message);
        if self.last_notice.as_ref() != Some(&key) {
            println!(
                "cinematic_text sequence={} offset={} {}",
                key.0, key.1, key.2
            );
            self.last_notice = Some(key);
        }
    }
}

pub fn install(app: &mut App, root: &Path) {
    app.insert_non_send(State {
        root: root.into(),
        table: None,
        font: None,
        atlas: Atlas::default(),
        image: None,
        nodes: Vec::new(),
        camera: None,
        last_notice: None,
    })
    .init_resource::<CaptureImage>()
    .add_systems(Update, update.after(crate::cinematic_camera::update));
}

fn update(
    mut commands: Commands,
    cinematic: Option<ResMut<crate::cinematic_camera::Cinematic>>,
    mut state: NonSendMut<State>,
    mut images: ResMut<Assets<Image>>,
    mut capture: ResMut<CaptureImage>,
    mut nodes: Query<(&mut Node, &mut ImageNode, &mut Visibility)>,
    mut cameras: Query<&mut Camera>,
) {
    // Old nodes are hidden for absent blocks, errors, resets and mission exit.
    for &entity in &state.nodes {
        if let Ok((_, _, mut visible)) = nodes.get_mut(entity) {
            *visible = Visibility::Hidden;
        }
    }
    capture.0 = None;
    if let Some(entity) = state.camera
        && let Ok(mut camera) = cameras.get_mut(entity)
    {
        camera.is_active = cinematic.is_some();
    }
    let Some(mut cinematic) = cinematic else {
        return;
    };
    let frame = match cinematic.render_text_frame() {
        Ok(Some(frame)) => frame,
        Ok(None) => return,
        Err(error) => {
            state.notice(
                cinematic.sequence(),
                0,
                format!("request_satisfied=false needs={error:?}"),
            );
            return;
        }
    };
    let prepared = state.text(&frame).and_then(|text| {
        let changed = state.prepare(&text)?;
        let layout = giants_game::font_layout::Context {
            atlas: &state.atlas,
            extent: [1920, 1080],
            reference_height: 24.,
            precision: Precision::Double53,
        }
        .layout(frame.words, &text)
        .map_err(str::to_owned)?;
        Ok((changed, layout))
    });
    let (changed, layout) = match prepared {
        Ok(value) => value,
        Err(error) => {
            state.notice(
                cinematic.sequence(),
                frame.command_offset,
                format!("request_satisfied=false needs={error:?}"),
            );
            return;
        }
    };
    if changed || state.image.is_none() {
        let pixels = state
            .atlas
            .pixels
            .iter()
            .flat_map(|p| {
                if *p == 0 {
                    [0, 0, 0, 0]
                } else {
                    [255, 255, 255, 255]
                }
            })
            .collect();
        let mut image = Image::new(
            Extent3d {
                width: 256,
                height: 128,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            pixels,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        // D captures on two worlds request anisotropic min/mag at level1,
        // wrapping U/V and no mipmapping. Use the corresponding linear
        // single-level preview; device sampling/blend parity is still separate.
        image.sampler =
            bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
                mag_filter: bevy::image::ImageFilterMode::Linear,
                min_filter: bevy::image::ImageFilterMode::Linear,
                mipmap_filter: bevy::image::ImageFilterMode::Nearest,
                address_mode_u: bevy::image::ImageAddressMode::Repeat,
                address_mode_v: bevy::image::ImageAddressMode::Repeat,
                lod_max_clamp: 0.,
                ..default()
            });
        // Fresh id lets capture preparation distinguish a newly uploaded atlas
        // from the previous version. Unreferenced old handles can be reclaimed.
        state.image = Some(images.add(image));
    }
    let image = state.image.as_ref().unwrap().clone();
    capture.0 = Some(image.clone());
    let camera = *state.camera.get_or_insert_with(|| {
        commands
            .spawn((
                Name::new("Cinematic bitmap text preview"),
                Camera2d,
                Camera {
                    order: 100,
                    clear_color: ClearColorConfig::None,
                    ..default()
                },
                Msaa::Off,
            ))
            .id()
    });
    for (index, draw) in layout.draws.iter().enumerate() {
        let [left, right, top, bottom] = draw.rect;
        let [x, y, width, height] = draw.atlas_rect;
        let color = draw.color;
        let node = Node {
            position_type: PositionType::Absolute,
            left: px(left),
            top: px(top),
            width: px(right - left),
            height: px(bottom - top),
            ..default()
        };
        let graphic = ImageNode {
            image: image.clone(),
            rect: Some(Rect::new(
                f32::from(x),
                f32::from(y),
                f32::from(x) + f32::from(width),
                f32::from(y) + f32::from(height),
            )),
            color: Color::srgba_u8(
                (color >> 16) as u8,
                (color >> 8) as u8,
                color as u8,
                (color >> 24) as u8,
            ),
            ..default()
        };
        if let Some(&entity) = state.nodes.get(index)
            && let Ok((mut old_node, mut old_image, mut visible)) = nodes.get_mut(entity)
        {
            *old_node = node;
            *old_image = graphic;
            *visible = Visibility::Visible;
        } else {
            let entity = commands
                .spawn((
                    node,
                    graphic,
                    Visibility::Visible,
                    UiTargetCamera(camera),
                    LayoutConfig {
                        use_rounding: false,
                    },
                ))
                .id();
            if let Some(slot) = state.nodes.get_mut(index) {
                *slot = entity;
            } else {
                state.nodes.push(entity);
            }
        }
    }
    state.notice(
        cinematic.sequence(),
        frame.command_offset,
        format!(
            "glyph_rectangles={} renderer=bitmap_ui_preview native_gpu_verified=false",
            layout.draws.len()
        ),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(target_os = "windows")]
    #[test]
    fn rejected_control_after_a_new_glyph_invalidates_the_old_gpu_atlas() {
        let mut app = App::new();
        install(&mut app, Path::new("unused-test-data"));
        let mut state = app.world_mut().non_send_mut::<State>();
        state.image = Some(Handle::default());
        assert!(state.prepare(b"A\t").is_err());
        assert_ne!(state.atlas.glyphs[b'A' as usize].width, 0);
        assert!(state.image.is_none());
        assert!(!state.prepare(b"A").unwrap());
        assert!(state.image.is_none());
    }
    #[test]
    fn removing_the_cinematic_hides_pooled_text_without_loading_font_or_tables() {
        let mut app = App::new();
        app.init_resource::<Assets<Image>>();
        install(&mut app, Path::new("unused-test-data"));
        let node = app
            .world_mut()
            .spawn((Node::default(), ImageNode::default(), Visibility::Visible))
            .id();
        let camera = app.world_mut().spawn(Camera::default()).id();
        {
            let mut state = app.world_mut().non_send_mut::<State>();
            state.nodes.push(node);
            state.camera = Some(camera);
        }
        app.world_mut().resource_mut::<CaptureImage>().0 = Some(Handle::default());
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(node).unwrap(),
            Visibility::Hidden
        );
        assert!(!app.world().get::<Camera>(camera).unwrap().is_active);
        assert!(app.world().resource::<CaptureImage>().0.is_none());
        let state = app.world().non_send::<State>();
        assert!(state.font.is_none() && state.table.is_none());
    }
}
