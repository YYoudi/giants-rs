//! Diagnostic geometry viewer. Camera and shading are tools, not original behavior.
use crate::logo_material::{self, LogoMaterial};
use bevy::{
    asset::RenderAssetUsages, input::mouse::AccumulatedMouseMotion, mesh::Indices, prelude::*,
    render::render_resource::PrimitiveTopology,
};

#[derive(Resource)]
pub struct Scene {
    pub asset: giants_game::LogoAsset,
    pub oblique: bool,
    pub textures: Vec<giants_game::LogoTexture>,
}
#[derive(Component)]
pub struct FlyCamera {
    pub reset: Transform,
    pub speed: f32,
}
fn convert(v: [f32; 3]) -> [f32; 3] {
    [v[0], v[2], -v[1]]
}
fn camera_pose(oblique: bool) -> Transform {
    let position = if oblique {
        Vec3::new(280., 130., 360.)
    } else {
        Vec3::new(6.7, 0., 480.)
    };
    Transform::from_translation(position).looking_at(Vec3::new(6.7, 0., 0.), Vec3::Y)
}
pub fn setup(
    mut commands: Commands,
    scene: Res<Scene>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut logo_materials: ResMut<Assets<LogoMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let asset = &scene.asset;
    let source = &asset.mesh;
    let positions: Vec<[f32; 3]> = source
        .indices
        .iter()
        .map(|i| convert(source.positions[*i as usize]))
        .collect();
    let normals: Vec<[f32; 3]> = asset.normals.iter().copied().map(convert).collect();
    // This normal visualization makes the measured smoothing and depth observable.
    // It deliberately does not stand in for the original two-stage material.
    let colors: Vec<[f32; 4]> = normals
        .iter()
        .map(|n| {
            let n = Vec3::from_array(*n).normalize_or_zero();
            [
                0.2 + 0.8 * n.x.abs(),
                0.2 + 0.8 * n.y.abs(),
                0.2 + 0.8 * n.z.abs(),
                1.,
            ]
        })
        .collect();
    let material = materials.add(StandardMaterial {
        unlit: true,
        cull_mode: None,
        ..default()
    });
    let mut textures = std::collections::HashMap::new();
    for texture in &scene.textures {
        let mut image = Image::new(
            bevy::render::render_resource::Extent3d {
                width: texture.width,
                height: texture.height,
                depth_or_array_layers: 1,
            },
            bevy::render::render_resource::TextureDimension::D2,
            texture.rgba.clone(),
            bevy::render::render_resource::TextureFormat::Rgba8Unorm,
            RenderAssetUsages::default(),
        );
        image.sampler = logo_material::sampler();
        textures.insert(texture.name.to_ascii_lowercase(), images.add(image));
    }
    for sub in &source.subobjects {
        let mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions.clone())
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals.clone())
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, source.uv.clone())
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors.clone())
        .with_inserted_indices(Indices::U16(
            sub.triangles.iter().flatten().copied().collect(),
        ));
        let mut entity = commands.spawn((Name::new(sub.name.clone()), Mesh3d(meshes.add(mesh))));
        if textures.is_empty() {
            entity.insert(MeshMaterial3d(material.clone()));
        } else {
            let texture = textures
                .get(&sub.material.texture.to_ascii_lowercase())
                .expect("validated logo texture reference")
                .clone();
            entity.insert(MeshMaterial3d(logo_materials.add(LogoMaterial {
                parameters: logo_material::parameters(sub.material.colors, sub.material.power),
                texture,
            })));
        }
    }
    let pose = camera_pose(scene.oblique);
    commands.spawn((
        Camera3d::default(),
        Msaa::Off,
        bevy::core_pipeline::tonemapping::Tonemapping::None,
        bevy::core_pipeline::tonemapping::DebandDither::Disabled,
        pose,
        FlyCamera {
            reset: pose,
            speed: 100.,
        },
    ));
}
pub fn navigate(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    time: Res<Time>,
    mut cameras: Query<(&mut Transform, &FlyCamera)>,
    mut exit: MessageWriter<AppExit>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
    for (mut transform, camera) in &mut cameras {
        if keys.just_pressed(KeyCode::KeyR) {
            *transform = camera.reset;
            println!("camera_reset position={:?}", transform.translation);
        }
        if mouse.pressed(MouseButton::Right) {
            transform.rotate_y(-motion.delta.x * 0.003);
            transform.rotate_local_x(-motion.delta.y * 0.003);
        }
        // A complete quick tap can arrive within one frame. Consume its edge too.
        let active = |key| (keys.pressed(key) || keys.just_pressed(key)) as u8 as f32;
        let axis = Vec3::new(
            active(KeyCode::KeyD) - active(KeyCode::KeyA),
            active(KeyCode::Space) - active(KeyCode::ControlLeft),
            active(KeyCode::KeyS) - active(KeyCode::KeyW),
        );
        let speed = camera.speed
            * if keys.pressed(KeyCode::ShiftLeft) {
                3.
            } else {
                1.
            };
        let movement = transform.rotation * axis.normalize_or_zero() * speed * time.delta_secs();
        transform.translation += movement;
        if [
            KeyCode::KeyD,
            KeyCode::KeyA,
            KeyCode::KeyW,
            KeyCode::KeyS,
            KeyCode::Space,
            KeyCode::ControlLeft,
        ]
        .iter()
        .any(|key| keys.just_pressed(*key))
        {
            println!(
                "camera_input axis={axis:?} position={:?}",
                transform.translation
            );
        }
    }
}
