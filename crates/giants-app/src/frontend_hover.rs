//! Source selection texture/lifetime profile, anchored to the current font's glyphs.
use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    reflect::TypePath,
    render::{render_resource::*, storage::ShaderBuffer},
    shader::ShaderRef,
    text::TextLayoutInfo,
};
#[derive(Component)]
pub(crate) struct SelectedLabel;
#[derive(Component)]
pub(crate) struct Particle(usize);
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct Bindings {
    #[texture(0)]
    #[sampler(1)]
    texture: Handle<Image>,
    #[storage(2, read_only)]
    opacity: Handle<ShaderBuffer>,
    #[uniform(3)]
    slot: UVec4,
}
pub type Material = crate::source_view::UniformMaterial<Bindings>;
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct Composite {
    #[texture(0)]
    #[sampler(1)]
    particles: Handle<Image>,
}
impl UiMaterial for Composite {
    fn fragment_shader() -> ShaderRef {
        "embedded://giants_app/frontend_hover_composite.wesl".into()
    }
    fn specialize(d: &mut RenderPipelineDescriptor, _: UiMaterialKey<Self>) {
        if let Some(fragment) = &mut d.fragment {
            for target in fragment.targets.iter_mut().flatten() {
                target.blend = Some(BlendState {
                    color: BlendComponent {
                        src_factor: BlendFactor::One,
                        dst_factor: BlendFactor::One,
                        operation: BlendOperation::Add,
                    },
                    alpha: BlendComponent {
                        src_factor: BlendFactor::Zero,
                        dst_factor: BlendFactor::One,
                        operation: BlendOperation::Add,
                    },
                });
            }
        }
    }
}
impl UiMaterial for Material {
    fn fragment_shader() -> ShaderRef {
        "embedded://giants_app/frontend_hover.wesl".into()
    }
    fn specialize(d: &mut RenderPipelineDescriptor, _: UiMaterialKey<Self>) {
        if let Some(fragment) = &mut d.fragment {
            for target in fragment.targets.iter_mut().flatten() {
                target.blend = Some(BlendState {
                    color: BlendComponent {
                        src_factor: BlendFactor::SrcAlpha,
                        dst_factor: BlendFactor::One,
                        operation: BlendOperation::Add,
                    },
                    alpha: BlendComponent {
                        src_factor: BlendFactor::Zero,
                        dst_factor: BlendFactor::One,
                        operation: BlendOperation::Add,
                    },
                });
            }
        }
    }
}
struct Life {
    age: f32,
    duration: f32,
    fade_in: f32,
    fade_out: f32,
}
#[derive(Resource)]
pub struct Runtime {
    lives: Vec<Life>,
    buffer: Handle<ShaderBuffer>,
    random: u32,
    screen: Option<giants_game::frontend::Screen>,
    scales: [f32; 2],
    profile: [f32; 6],
    quad: Vec2,
    glyph_cursor: usize,
}
impl Runtime {
    fn random(&mut self) -> f32 {
        self.random ^= self.random << 13;
        self.random ^= self.random >> 17;
        self.random ^= self.random << 5;
        (self.random >> 8) as f32 / 16777216.
    }
}
pub fn spawn(world: &mut World, root: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = std::fs::read(root.join("Bin/xx_menus.gzp"))?;
    let archive = giants_formats::gzp::Archive::parse(&bytes)?;
    let mesh = giants_formats::gbs::parse(&archive.read("EFX_menu.gbs")?)?;
    let low = mesh
        .positions
        .iter()
        .fold(Vec3::splat(f32::INFINITY), |a, p| {
            a.min(Vec3::from_array(*p))
        });
    let high = mesh
        .positions
        .iter()
        .fold(Vec3::splat(f32::NEG_INFINITY), |a, p| {
            a.max(Vec3::from_array(*p))
        });
    let quad = (high - low).truncate();
    let fx = giants_formats::effect_image::parse(&std::fs::read(root.join("Bin/fx.bin"))?)?;
    let effect = fx
        .find("fx_menu_flare")
        .ok_or("menu selection effect missing")?
        .1;
    let program = fx.program(effect.offset)?;
    if program.records.iter().map(|r| r.opcode).collect::<Vec<_>>() != [3, 6, 8, 5, 7, 2] {
        return Err("selection effect outside acquired program".into());
    }
    let scales: [f32; 2] = std::array::from_fn(|i| {
        f32::from_le_bytes(
            program.records[2].bytes[1 + i * 4..5 + i * 4]
                .try_into()
                .unwrap(),
        )
    });
    let profile: [f32; 6] = std::array::from_fn(|i| {
        f32::from_le_bytes(
            program.records[3].bytes[1 + i * 4..5 + i * 4]
                .try_into()
                .unwrap(),
        )
    });
    if scales
        .into_iter()
        .chain(profile)
        .any(|v| !v.is_finite() || v <= 0.)
    {
        return Err("invalid selection effect ranges".into());
    }
    let t = giants_formats::tga::decode(&archive.read("EFX_menu.tga")?)?;
    let rgba = t
        .pixels
        .as_chunks::<3>()
        .0
        .iter()
        .enumerate()
        .flat_map(|(i, c)| [c[0], c[1], c[2], t.alpha.as_ref().map_or(255, |a| a[i])])
        .collect();
    let mut image = Image::new(
        Extent3d {
            width: t.width,
            height: t.height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba,
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::default(),
    );
    image.sampler = crate::logo_material::sampler();
    let texture = world.resource_mut::<Assets<Image>>().add(image);
    let buffer = world
        .resource_mut::<Assets<ShaderBuffer>>()
        .add(crate::source_view::uniform_buffer(&[Vec4::ZERO; 100]));
    let presentation = world
        .query::<&MaterialNode<crate::world_composition::Presentation>>()
        .iter(world)
        .next()
        .ok_or("frontend presentation missing")?
        .0
        .clone();
    let target = world
        .resource::<Assets<crate::world_composition::Presentation>>()
        .get(&presentation)
        .ok_or("frontend target missing")?
        .image
        .clone();
    let cameras: Vec<_> = world
        .query::<(Entity, &Name)>()
        .iter(world)
        .filter(|(_, n)| n.as_str() == "Encoded world presentation")
        .map(|(e, _)| e)
        .collect();
    for entity in cameras {
        world
            .get_mut::<Camera>(entity)
            .ok_or("presentation camera missing")?
            .order = 2;
    }
    let particles = world
        .resource_mut::<Assets<Image>>()
        .add(Image::new_target_texture(
            1920,
            1080,
            TextureFormat::Rgba16Float,
            None,
        ));
    let overlay_camera = world
        .spawn((
            super::frontend_background::Owned,
            Name::new("Frontend bounded flame composition"),
            Camera2d,
            bevy::camera::RenderTarget::Image(target.into()),
            Camera {
                order: 1,
                clear_color: ClearColorConfig::None,
                ..default()
            },
            Msaa::Off,
        ))
        .id();
    let composite = world.resource_mut::<Assets<Composite>>().add(Composite {
        particles: particles.clone(),
    });
    world.spawn((
        super::frontend_background::Owned,
        UiTargetCamera(overlay_camera),
        MaterialNode(composite),
        Node {
            position_type: PositionType::Absolute,
            left: px(0.),
            top: px(0.),
            width: px(1920.),
            height: px(1080.),
            ..default()
        },
        LayoutConfig {
            use_rounding: false,
        },
    ));
    let camera = world
        .spawn((
            super::frontend_background::Owned,
            Name::new("Frontend flames encoded overlay"),
            Camera2d,
            bevy::camera::RenderTarget::Image(particles.into()),
            Camera {
                order: 0,
                clear_color: ClearColorConfig::Custom(Color::NONE),
                ..default()
            },
            Msaa::Off,
        ))
        .id();
    for index in 0..100 {
        let material = world
            .resource_mut::<Assets<Material>>()
            .add(Material::from(Bindings {
                texture: texture.clone(),
                opacity: buffer.clone(),
                slot: UVec4::new(index as u32, 0, 0, 0),
            }));
        world.spawn((
            super::frontend_background::Owned,
            Particle(index),
            UiTargetCamera(camera),
            MaterialNode(material),
            Node {
                position_type: PositionType::Absolute,
                display: Display::None,
                ..default()
            },
            LayoutConfig {
                use_rounding: false,
            },
        ));
    }
    world.insert_resource(Runtime {
        lives: (0..100)
            .map(|_| Life {
                age: 0.,
                duration: 0.,
                fade_in: 1.,
                fade_out: 1.,
            })
            .collect(),
        buffer,
        random: 0x4d454e55,
        screen: None,
        scales,
        profile,
        quad,
        glyph_cursor: 0,
    });
    Ok(())
}
pub fn update(
    time: Res<Time>,
    scene: Res<crate::frontend_scene::Scene>,
    runtime: Option<ResMut<Runtime>>,
    selected: Query<(&TextLayoutInfo, &ComputedNode, &UiGlobalTransform), With<SelectedLabel>>,
    windows: Query<&Window>,
    mut nodes: Query<(&Particle, &mut Node)>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
) {
    let Some(mut runtime) = runtime else { return };
    let screen = scene.feedback_selection().0;
    let changed = runtime.screen != Some(screen);
    if changed {
        runtime.glyph_cursor = 0;
    }
    runtime.screen = Some(screen);
    for life in &mut runtime.lives {
        life.age += time.delta_secs();
        if changed {
            life.duration = 0.;
        }
    }
    let extent = windows.single().map_or(Vec2::new(1920., 1080.), |w| {
        Vec2::new(w.physical_width() as f32, w.physical_height() as f32)
    });
    let scale = Vec2::new(1920., 1080.) / extent;
    let mut centers = Vec::new();
    if let Ok((layout, node, transform)) = selected.single() {
        centers.extend(
            layout
                .glyphs
                .iter()
                .map(|g| transform.transform_point2(g.position - node.size() * 0.5) * scale),
        );
    }
    let mut next = 0;
    let mut opacity = [Vec4::ZERO; 100];
    for (particle, mut node) in &mut nodes {
        let index = particle.0;
        if runtime.lives[index].age >= runtime.lives[index].duration && next < centers.len() {
            let center = centers[runtime.glyph_cursor % centers.len()];
            runtime.glyph_cursor = runtime.glyph_cursor.wrapping_add(1);
            next += 1;
            let p = runtime.profile;
            let duration = p[0] + runtime.random() * (p[1] - p[0]);
            let range = runtime.scales;
            let size = range[0] + runtime.random() * (range[1] - range[0]);
            let fade_in = p[2] + runtime.random() * (p[3] - p[2]);
            let fade_out = p[4] + runtime.random() * (p[5] - p[4]);
            // Measured native UI projection/depth; current font supplies centers.
            let dimensions = runtime.quad * size * (1920. * 1.263_638_5 / (2. * 404.));
            node.left = px(center.x - dimensions.x * 0.5);
            node.top = px(center.y - dimensions.y * 0.5);
            node.width = px(dimensions.x);
            node.height = px(dimensions.y);
            node.display = Display::Flex;
            runtime.lives[index] = Life {
                age: 0.,
                duration,
                fade_in,
                fade_out,
            };
        }
        let life = &runtime.lives[index];
        if life.age < life.duration {
            opacity[index].x = ((life.age / life.fade_in)
                .min((life.duration - life.age) / life.fade_out)
                .clamp(0., 1.)
                * 255.)
                .trunc()
                / 255.;
        } else {
            node.display = Display::None;
        }
    }
    if let Some(mut buffer) = buffers.get_mut(&runtime.buffer) {
        crate::source_view::write_uniform(&opacity, &mut buffer);
    }
}
