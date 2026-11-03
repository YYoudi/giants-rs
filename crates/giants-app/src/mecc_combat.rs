//! Explicit development pistol and target. No campaign readiness or native VFX claim.
use bevy::{asset::RenderAssetUsages, prelude::*};
use giants_game::{
    arithmetic::Precision,
    straight_projectile::{Event, Shot},
};
#[derive(Resource)]
pub(crate) struct Combat {
    definition: giants_formats::weapons::Projectile,
    shots: Vec<Shot>,
    impacts: Vec<(Vec3, f32)>,
    cooldown: f32,
    remainder: f64,
    fired: u32,
    hits: u32,
    object_hits: u32,
    world_hits: u32,
    world_replay: bool,
    world_free: bool,
    crossed: bool,
    damage_replay: bool,
    damage_pose: Option<u32>,
    mesh: Option<Handle<Mesh>>,
    replay: bool,
    pose: bool,
    impact_pose: bool,
    tick: u32,
    checked: bool,
}
#[derive(Component)]
struct Label;
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct CombatUpdate;
impl Combat {
    pub(crate) fn fresh(&self) -> bool {
        self.shots.is_empty()
            && self.impacts.is_empty()
            && self.fired == 0
            && self.hits == 0
            && self.world_hits == 0
            && self.cooldown == 0.
    }
    pub(crate) fn reset(&mut self) {
        self.shots.clear();
        self.impacts.clear();
        self.cooldown = 0.;
        self.fired = 0;
        self.hits = 0;
        self.object_hits = 0;
        self.world_hits = 0;
        self.crossed = false;
        self.remainder = 0.;
    }
    fn step(
        &mut self,
        terrain: &giants_formats::gti::Terrain,
        origin: [f32; 3],
        target: [f32; 3],
        held: bool,
        object: &mut crate::mecc_target::Target,
        actor: &crate::mecc_dev_scene::State,
    ) -> Result<(), &'static str> {
        let dt = 1. / 120.;
        self.cooldown =
            giants_game::mecc_fire::advance_clock(self.cooldown, dt, Precision::Double53)?;
        let (fire, next) = giants_game::mecc_fire::request(
            self.cooldown,
            self.definition.interval,
            self.definition.flags,
            true,
            false,
            held,
            Precision::Double53,
        )?;
        self.cooldown = next;
        if fire {
            if self.shots.len() >= 4096 {
                return Err("development projectile budget");
            }
            self.shots
                .push(Shot::new(&self.definition, origin, target)?);
            self.fired += 1;
            println!(
                "mecc_dev_fire shot={} tick={} cooldown={} launch_speed={} muzzle=weapon_reference_1 arm_aim=false effects_pending=true",
                self.fired, self.tick, self.cooldown, self.definition.launch_speed
            );
        }
        for (_, ttl) in &mut self.impacts {
            *ttl -= dt;
        }
        self.impacts.retain(|(_, ttl)| *ttl > 0.);
        let mut live = Vec::with_capacity(self.shots.len());
        for mut s in self.shots.drain(..) {
            let start = s.motion.position;
            let event = s.step(terrain, dt)?;
            let query = giants_game::projectile_world::query(
                u32::MAX,
                1,
                start,
                s.segment_end,
                if self.world_free {
                    &[]
                } else {
                    &actor.projectile_objects
                },
            )?;
            let target_point = object.trace(start, s.segment_end)?;
            let target_first = target_point.is_some_and(|point| {
                let distance = (Vec3::from_array(point) - Vec3::from_array(start)).length_squared();
                query.selected.is_none() || distance < query.distance_sq
            });
            if target_first {
                let point = object
                    .impact(start, s.segment_end, self.definition.direct_damage)?
                    .ok_or("test target changed during supplied query")?;
                self.object_hits += 1;
                self.hits += 1;
                self.impacts.push((Vec3::from_array(point), 0.7));
                continue;
            }
            if let Some(id) = query.selected {
                if actor
                    .projectile_test
                    .is_some_and(|(_, _, expected)| id != expected)
                {
                    return Err("source shot reached unexpected object");
                }
                self.world_hits += 1;
                self.hits += 1;
                self.impacts.push((Vec3::from_array(query.point), 0.7));
                println!(
                    "mecc_world_projectile_hit object={id} point={:?} distance_sq={} tag={} count={} list=declared_supplied damage=false visual=diagnostic",
                    query.point, query.distance_sq, query.tag, self.world_hits
                );
                continue;
            }
            if let Some((point, normal, _)) = actor.projectile_test {
                self.crossed |= (Vec3::from_array(s.segment_end) - Vec3::from_array(point))
                    .dot(Vec3::from_array(normal))
                    < -1.;
            }
            match event {
                Event::Flying => live.push(s),
                Event::Impact(p) => {
                    self.hits += 1;
                    self.impacts.push((Vec3::from_array(p), 0.7));
                    println!(
                        "mecc_dev_impact count={} point={p:?} world_query=declared_supplied visual=diagnostic",
                        self.hits
                    );
                }
                Event::Expired | Event::Outside => {}
            }
        }
        self.shots = live;
        Ok(())
    }
}
pub(crate) fn load(root: &std::path::Path) -> Result<Combat, Box<dyn std::error::Error>> {
    let c = giants_formats::weapons::parse_with_profile(
        &std::fs::read(root.join("Bin/weapondefs.bin"))?,
        giants_formats::weapons::Profile::Debug,
    )?;
    let icon = c.icons.get(&1).ok_or("development pistol icon missing")?;
    let definition = c
        .projectiles
        .get(&icon.projectile)
        .ok_or("development pistol projectile missing")?
        .clone();
    // Refuse an unsupported flight type instead of silently approximating gravity.
    Shot::new(&definition, [0.; 3], [0., 1., 0.])?;
    let mode = std::env::var("GIANTS_MECC_DEV_REPLAY").ok();
    let replay = matches!(
        mode.as_deref(),
        Some(
            "fire"
                | "fire-pose"
                | "fire-impact-pose"
                | "damage"
                | "damage-pose"
                | "damage-dead-pose"
                | "world-fire"
                | "world-fire-pose"
                | "world-fire-free"
        )
    );
    println!(
        "mecc_dev_weapon icon=1 projectile={} class={} interval={} speed={} lifetime={} loadout=development held_mesh_pending=false arm_aim=false first_person_overlay=false",
        definition.id,
        definition.class,
        definition.interval,
        definition.launch_speed,
        definition.lifetime
    );
    Ok(Combat {
        definition,
        shots: Vec::new(),
        impacts: Vec::new(),
        cooldown: 0.,
        remainder: 0.,
        fired: 0,
        hits: 0,
        object_hits: 0,
        world_hits: 0,
        world_replay: mode.as_deref().is_some_and(|s| s.starts_with("world-fire")),
        world_free: mode.as_deref() == Some("world-fire-free"),
        crossed: false,
        damage_replay: mode.as_deref().is_some_and(|s| s.starts_with("damage")),
        damage_pose: match mode.as_deref() {
            Some("damage-pose") => Some(300),
            Some("damage-dead-pose") => Some(720),
            _ => None,
        },
        mesh: None,
        replay,
        pose: matches!(
            mode.as_deref(),
            Some("fire-pose" | "fire-impact-pose" | "world-fire-pose")
        ),
        impact_pose: mode.as_deref() == Some("fire-impact-pose"),
        tick: 0,
        checked: false,
    })
}
pub fn install(app: &mut App, root: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    app.insert_resource(load(root)?).add_systems(
        Update,
        update
            .in_set(CombatUpdate)
            .after(crate::mecc_dev_scene::update)
            .before(crate::source_view::ViewUpdate)
            .run_if(crate::mecc_session::playing),
    );
    crate::mecc_target::install(app)?;
    Ok(())
}
#[allow(clippy::too_many_arguments)]
pub(crate) fn setup(
    mut commands: Commands,
    mut combat: ResMut<Combat>,
    view: Res<crate::source_view::SourceView>,
    scene: Res<crate::world_scene::Scene>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<crate::actor_scene::Material>>,
    mut fonts: ResMut<Assets<Font>>,
) {
    let mesh = meshes.add(
        Mesh::new(
            bevy::render::render_resource::PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0.; 3]; 3])
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.; 2]; 3])
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, vec![[1.; 4]; 3]),
    );
    combat.mesh = Some(mesh.clone());
    let image = images.add(Image::new_fill(
        bevy::render::render_resource::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        &[255, 255, 255, 255],
        bevy::render::render_resource::TextureFormat::Rgba8Unorm,
        RenderAssetUsages::default(),
    ));
    let fog = scene.asset.config.fog;
    commands.spawn((
        Name::new("Development projectile diagnostics"),
        Mesh3d(mesh),
        MeshMaterial3d(materials.add(crate::actor_scene::Material::from(
            crate::actor_scene::Bindings {
                texture: image,
                clip_from_bevy: view.buffer.clone(),
                validation: Vec4::new(1., 1. / 1920., -1. / 1080., 1.),
                fog: Vec4::new(fog.start, fog.end, 1., 0.),
                fog_color: Vec4::ZERO,
                blend: false,
                force_additive: false,
            },
        ))),
        bevy::camera::visibility::NoFrustumCulling,
    ));
    let camera = commands
        .spawn((
            Camera2d,
            Camera {
                order: 101,
                clear_color: ClearColorConfig::None,
                ..default()
            },
            Msaa::Off,
        ))
        .id();
    let path = std::env::var_os("GIANTS_INSPECTION_FONT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(crate::inspection_font_path);
    let font = fonts.add(Font::from_bytes(
        std::fs::read(path).expect("development combat font"),
    ));
    commands.spawn((
        UiTargetCamera(camera),
        Label,
        Text::new("PISTOL — DEVELOPMENT PREVIEW"),
        TextFont::from_font_size(FontSize::Px(18.)).with_font(font),
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            right: px(24.),
            bottom: px(24.),
            padding: UiRect::all(px(12.)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.03, 0.04, 0.88)),
    ));
    for (w, h) in [(18., 2.), (2., 18.)] {
        commands.spawn((
            UiTargetCamera(camera),
            Node {
                position_type: PositionType::Absolute,
                left: percent(50.),
                top: percent(50.),
                margin: UiRect {
                    left: px(-w / 2.),
                    top: px(-h / 2.),
                    ..default()
                },
                width: px(w),
                height: px(h),
                ..default()
            },
            BackgroundColor(Color::WHITE),
        ));
    }
}
fn quad(vertices: &mut Vec<[f32; 3]>, colors: &mut Vec<[f32; 4]>, p: [Vec3; 4], color: [f32; 4]) {
    for i in [0, 1, 2, 0, 2, 3, 2, 1, 0, 3, 2, 0] {
        let p = p[i];
        vertices.push([p.x, p.z, -p.y]);
        colors.push(color);
    }
}
#[allow(clippy::too_many_arguments)]
fn update(
    mut combat: ResMut<Combat>,
    mut object: ResMut<crate::mecc_target::Target>,
    actor: Res<crate::mecc_dev_scene::State>,
    incoming: Res<crate::mecc_incoming::Incoming>,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    frontend: Option<Res<crate::frontend_scene::Scene>>,
    mut scene: ResMut<crate::world_scene::Scene>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut labels: Query<&mut Text, With<Label>>,
    mut exit: MessageWriter<AppExit>,
) {
    let focused = windows.iter().any(|w| w.focused);
    if focused && keys.just_pressed(KeyCode::F5) {
        combat.reset();
        object.reset();
        println!("mecc_dev_combat_reset=true");
    }
    let (origin, mut target) = match actor.combat_pose() {
        Ok(pose) => pose,
        Err(error) => {
            error!("mecc_dev_muzzle_failed={error}");
            exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
            return;
        }
    };
    if combat.damage_replay {
        target = object.aim;
    }
    if let Some((point, _, _)) = actor.projectile_test {
        target = point;
    }
    if !(combat.replay && combat.checked) {
        combat.remainder += f64::from(time.delta_secs().min(0.1));
        while combat.remainder >= 1. / 120. {
            let held = if combat.damage_replay {
                (60..550).contains(&combat.tick)
            } else if combat.replay {
                (60..300).contains(&combat.tick)
            } else {
                focused
                    && crate::mecc_input::pressed(
                        frontend.as_deref().map(|s| s.mecc_controls()),
                        crate::mecc_input::Action::Fire,
                        &keys,
                        &buttons,
                        false,
                    )
            };
            if combat.damage_replay {
                if combat.tick == 720 {
                    let passed = combat.fired == 9
                        && combat.object_hits == 9
                        && object.deaths == 1
                        && !object.health.alive()
                        && combat.shots.is_empty();
                    println!(
                        "mecc_dev_damage_check passed={passed} shots={} object_hits={} deaths={} remaining={}",
                        combat.fired,
                        combat.object_hits,
                        object.deaths,
                        object.health.remaining()
                    );
                    if !passed {
                        exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
                        return;
                    }
                }
                if combat.tick == 780 && combat.damage_pose.is_none() {
                    combat.reset();
                    object.reset();
                    println!(
                        "mecc_dev_damage_reset passed={} remaining={} projectiles={}",
                        object.health.remaining() == 100.
                            && combat.shots.is_empty()
                            && object.deaths == 0,
                        object.health.remaining(),
                        combat.shots.len()
                    );
                }
                if combat.tick == 840 || combat.damage_pose == Some(combat.tick) {
                    combat.checked = true;
                    scene.cinematic_capture_ready = true;
                    println!(
                        "mecc_dev_damage_finished passed=true steps={} remaining={} pose={} owner_validation=false native_collision_dispatcher=false",
                        combat.tick,
                        object.health.remaining(),
                        combat.damage_pose.is_some()
                    );
                    break;
                }
            }
            if combat.world_replay && (combat.tick == 420 || combat.pose && combat.world_hits > 0) {
                let passed = if combat.world_free {
                    combat.fired == 4 && combat.world_hits == 0 && combat.crossed
                } else if combat.pose {
                    combat.world_hits == 1
                } else {
                    combat.fired == 4 && combat.world_hits == 4 && combat.shots.is_empty()
                };
                println!(
                    "mecc_world_projectile_finished passed={passed} shots={} world_hits={} active={} crossed={} empty_control={} source_face=true muzzle=animated_weapon native_trajectory=false",
                    combat.fired,
                    combat.world_hits,
                    combat.shots.len(),
                    combat.crossed,
                    combat.world_free
                );
                if !passed {
                    exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
                    return;
                }
                combat.checked = true;
                scene.cinematic_capture_ready = true;
                break;
            }
            if combat.replay && !combat.damage_replay && !combat.world_replay && combat.tick == 420
            {
                let passed = combat.fired == 4 && combat.hits == 4 && combat.shots.is_empty();
                println!(
                    "mecc_dev_fire_check passed={passed} shots={} hits={} world_query=declared_supplied",
                    combat.fired, combat.hits
                );
                if !passed {
                    exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
                    return;
                }
            }
            if combat.replay
                && !combat.damage_replay
                && !combat.world_replay
                && (combat.tick == 480
                    || (combat.pose && combat.tick == if combat.impact_pose { 80 } else { 61 }))
            {
                if !combat.pose {
                    combat.reset();
                }
                combat.checked = true;
                scene.cinematic_capture_ready = true;
                println!(
                    "mecc_dev_fire_finished passed=true pose={} steps={} active={} impacts={} owner_validation=false native_visual_parity=false",
                    combat.pose,
                    combat.tick,
                    combat.shots.len(),
                    combat.impacts.len()
                );
                break;
            }
            if let Err(e) = combat.step(
                &scene.asset.terrain,
                origin,
                target,
                held && actor.alive(),
                &mut object,
                &actor,
            ) {
                error!("mecc_dev_fire_error={e}");
                exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
                return;
            }
            combat.tick += 1;
            combat.remainder -= 1. / 120.;
        }
    }
    if combat.replay && !combat.checked {
        scene.cinematic_capture_ready = false;
    }
    let mut v = Vec::new();
    let mut colors = Vec::new();
    for (s, enemy) in combat
        .shots
        .iter()
        .map(|s| (s, false))
        .chain(incoming.shots.iter().map(|s| (s, true)))
    {
        let color = if enemy {
            [1., 0.1, 0.1, 1.]
        } else {
            [1., 0.85, 0.1, 1.]
        };
        let end = Vec3::from_array(s.motion.position);
        let velocity = Vec3::from_array(s.motion.velocity);
        let trail = (end - Vec3::from_array(s.origin))
            .length()
            .min(velocity.length() * 0.015);
        let start = end - velocity.normalize_or_zero() * trail;
        let right = velocity.cross(Vec3::Z).normalize_or_zero() * 0.12;
        quad(
            &mut v,
            &mut colors,
            [start - right, end - right, end + right, start + right],
            color,
        );
        quad(
            &mut v,
            &mut colors,
            [
                start - Vec3::Z * 0.12,
                end - Vec3::Z * 0.12,
                end + Vec3::Z * 0.12,
                start + Vec3::Z * 0.12,
            ],
            color,
        );
    }
    for (p, _) in &combat.impacts {
        let z = Vec3::Z * 0.8;
        let x = Vec3::X * 0.8;
        let y = Vec3::Y * 0.8;
        quad(
            &mut v,
            &mut colors,
            [*p - x, *p + z, *p + x, *p - z],
            [1., 0.35, 0.05, 1.],
        );
        quad(
            &mut v,
            &mut colors,
            [*p - y, *p + z, *p + y, *p - z],
            [1., 0.35, 0.05, 1.],
        );
    }
    if v.is_empty() {
        v = vec![[0.; 3]; 3];
        colors = vec![[1.; 4]; 3];
    }
    if let Some(mut mesh) = combat.mesh.as_ref().and_then(|h| meshes.get_mut(h)) {
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.; 2]; v.len()]);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, v);
    }
    for mut text in &mut labels {
        text.0 = format!(
            "PISTOL — DEVELOPMENT PREVIEW\nShots {}   Impacts {}\nFire: profile control   Tracer/impact markers",
            combat.fired, combat.hits
        );
        if object.enabled {
            text.0.push_str(&format!(
                "\nTarget HP {:.0}/{:.0}   Object hits {}\nStatic test target / F5 resets",
                object.health.remaining(),
                object.health.limit,
                combat.object_hits
            ));
        }
        text.0.push_str(&format!(
            "\nPlayer HP {:.0}/100   F8: test attacker {}{}",
            actor.remaining_health(),
            if incoming.enabled { "ON" } else { "OFF" },
            if actor.alive() {
                ""
            } else {
                "\nDEAD / F5: development restart"
            }
        ));
    }
}
