//! Mission-owned projectile diagnostics and functional HUD. Native effects and
//! original HUD artwork remain separate from firing/physics capability.
use bevy::{asset::RenderAssetUsages, prelude::*};

#[derive(Resource, Default)]
struct State {
    mounted: Option<Mounted>,
}
struct Mounted {
    ticket: u64,
    entities: Vec<Entity>,
    label: Entity,
    mesh: Handle<Mesh>,
    image: Handle<Image>,
    material: Handle<crate::actor_scene::Material>,
    fired: u32,
    active: usize,
}
pub struct Plugin;
impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<State>().add_systems(
            Update,
            update
                .after(crate::mission_player::PlayerUpdate)
                .before(crate::source_view::ViewUpdate)
                .run_if(crate::mecc_session::frontend_available),
        );
    }
}
fn release(world: &mut World) {
    let Some(m) = world.resource_mut::<State>().mounted.take() else {
        return;
    };
    for entity in m.entities {
        world.despawn(entity);
    }
    world.resource_mut::<Assets<Mesh>>().remove(m.mesh.id());
    world.resource_mut::<Assets<Image>>().remove(m.image.id());
    world
        .resource_mut::<Assets<crate::actor_scene::Material>>()
        .remove(m.material.id());
    assert!(!world.resource::<Assets<Mesh>>().contains(m.mesh.id()));
    assert!(!world.resource::<Assets<Image>>().contains(m.image.id()));
    assert!(
        !world
            .resource::<Assets<crate::actor_scene::Material>>()
            .contains(m.material.id())
    );
    println!(
        "mission_weapon_released ticket={} fired={} last_active={} entities=4 remaining_assets=0 shared_font_preserved=true",
        m.ticket, m.fired, m.active
    );
}
fn mount(world: &mut World, ticket: u64) {
    use crate::mission_runtime::MissionOwned;
    let view = world
        .resource::<crate::source_view::SourceView>()
        .buffer
        .clone();
    let font = world
        .resource::<crate::frontend_scene::InspectionFont>()
        .0
        .clone();
    let image = world.resource_mut::<Assets<Image>>().add(Image::new_fill(
        bevy::render::render_resource::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        &[255; 4],
        bevy::render::render_resource::TextureFormat::Rgba8Unorm,
        RenderAssetUsages::default(),
    ));
    let material = world
        .resource_mut::<Assets<crate::actor_scene::Material>>()
        .add(crate::actor_scene::Material::from(
            crate::actor_scene::Bindings {
                texture: image.clone(),
                clip_from_bevy: view,
                validation: Vec4::new(1., 1. / 1920., -1. / 1080., 1.),
                fog: Vec4::new(0., 1., 1., 0.),
                fog_color: Vec4::ZERO,
                blend: false,
                force_additive: false,
            },
        ));
    let mesh = world.resource_mut::<Assets<Mesh>>().add(Mesh::new(
        bevy::render::render_resource::PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    ));
    let draw = world
        .spawn((
            MissionOwned(ticket),
            Name::new("Mission projectile preview"),
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            bevy::camera::visibility::NoFrustumCulling,
        ))
        .id();
    let camera = world
        .spawn((
            MissionOwned(ticket),
            Camera2d,
            Camera {
                order: 101,
                clear_color: ClearColorConfig::None,
                ..default()
            },
            Msaa::Off,
        ))
        .id();
    let root = world
        .spawn((
            MissionOwned(ticket),
            UiTargetCamera(camera),
            Node {
                position_type: PositionType::Absolute,
                left: px(24.),
                bottom: px(24.),
                padding: UiRect::all(px(12.)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.03, 0.04, 0.8)),
        ))
        .id();
    let label = world
        .spawn((
            ChildOf(root),
            Text::new(""),
            TextFont::from_font_size(FontSize::Px(20.)).with_font(font),
            TextColor(Color::WHITE),
        ))
        .id();
    let crosshair = world
        .spawn((
            crate::mission_runtime::MissionOwned(ticket),
            UiTargetCamera(camera),
            Node {
                position_type: PositionType::Absolute,
                left: px(0.),
                top: px(0.),
                width: percent(100.),
                height: percent(100.),
                ..default()
            },
        ))
        .id();
    for (width, height) in [(18., 2.), (2., 18.)] {
        world.spawn((
            ChildOf(crosshair),
            UiTargetCamera(camera),
            Node {
                position_type: PositionType::Absolute,
                left: percent(50.),
                top: percent(50.),
                margin: UiRect {
                    left: px(-width / 2.),
                    top: px(-height / 2.),
                    ..default()
                },
                width: px(width),
                height: px(height),
                ..default()
            },
            BackgroundColor(Color::WHITE),
        ));
    }
    world.resource_mut::<State>().mounted = Some(Mounted {
        ticket,
        entities: vec![draw, root, camera, crosshair],
        label,
        mesh,
        image,
        material,
        fired: 0,
        active: 0,
    });
    println!(
        "mission_weapon_mounted ticket={ticket} assets=3 borrowed_font=true hud=health_fuel_pistol effects=diagnostic"
    );
}
fn quad(v: &mut Vec<[f32; 3]>, colors: &mut Vec<[f32; 4]>, p: [Vec3; 4], color: [f32; 4]) {
    for i in [0, 1, 2, 0, 2, 3, 2, 1, 0, 3, 2, 0] {
        let p = p[i];
        v.push([p.x, p.z, -p.y]);
        colors.push(color);
    }
}
fn update(world: &mut World) {
    let frontend = world.resource::<crate::frontend_scene::Scene>();
    let wanted = world
        .resource::<crate::mission_showcase::State>()
        .controls(frontend.flow.phase)
        .then_some(frontend.flow.ticket)
        .filter(|t| {
            world
                .resource::<crate::mission_runtime::Runtime>()
                .active_ticket()
                == Some(*t)
        });
    if world
        .resource::<State>()
        .mounted
        .as_ref()
        .is_some_and(|m| Some(m.ticket) != wanted)
    {
        release(world);
    }
    let Some(ticket) = wanted else { return };
    let mut query = world.query::<&crate::mission_player::Controller>();
    let Some(controller) = query.iter(world).find(|c| c.prepared(ticket)) else {
        return;
    };
    let Some(pistol) = controller.pistol() else {
        return;
    };
    let (health, fuel, _boost) = controller.hud();
    let (fired, active) = (pistol.fired, pistol.shots.len());
    let mut v = Vec::new();
    let mut colors = Vec::new();
    for shot in &pistol.shots {
        let end = Vec3::from_array(shot.motion.position);
        let velocity = Vec3::from_array(shot.motion.velocity);
        let trail = (end - Vec3::from_array(shot.origin))
            .length()
            .min(velocity.length() * 0.015);
        let start = end - velocity.normalize_or_zero() * trail;
        let right = velocity.cross(Vec3::Z).normalize_or_zero() * 0.12;
        quad(
            &mut v,
            &mut colors,
            [start - right, end - right, end + right, start + right],
            [1., 0.85, 0.1, 1.],
        );
        let z = Vec3::Z * 0.12;
        quad(
            &mut v,
            &mut colors,
            [start - z, end - z, end + z, start + z],
            [1., 0.85, 0.1, 1.],
        );
    }
    for (p, _) in &pistol.impacts {
        let p = Vec3::from_array(*p);
        quad(
            &mut v,
            &mut colors,
            [
                p - Vec3::X * 0.8,
                p + Vec3::Z * 0.8,
                p + Vec3::X * 0.8,
                p - Vec3::Z * 0.8,
            ],
            [1., 0.35, 0.05, 1.],
        );
    }
    if world.resource::<State>().mounted.is_none() {
        mount(world, ticket);
    }
    let m = world.resource::<State>().mounted.as_ref().unwrap();
    let mesh = m.mesh.clone();
    let label = m.label;
    if v.is_empty() {
        v = vec![[0.; 3]; 3];
        colors = vec![[1.; 4]; 3];
    }
    if let Some(mut mesh) = world.resource_mut::<Assets<Mesh>>().get_mut(&mesh) {
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.; 2]; v.len()]);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, v);
    }
    world.get_mut::<Text>(label).unwrap().0 =
        format!("HEALTH {health:.0}    FUEL {fuel:.0}\nPISTOL");
    let mut state = world.resource_mut::<State>();
    let m = state.mounted.as_mut().unwrap();
    m.fired = fired;
    m.active = active;
}
