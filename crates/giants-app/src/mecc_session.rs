//! Explicit development session ownership, separate from campaign tickets.
use bevy::{asset::AssetId, ecs::system::RunSystemOnce, prelude::*};
use std::collections::HashSet;

#[derive(Component)]
struct Owned;
#[derive(Component)]
struct MenuHint;
#[derive(Resource, Default)]
pub(crate) struct Session {
    assets: Option<OwnedAssets>,
    replay_return: bool,
    awaiting_menu: bool,
    root: std::path::PathBuf,
    parked: Option<crate::world_scene::Scene>,
    start_in_menu: bool,
    replay_cycles: Option<bool>,
    mounts: u32,
    returns: u32,
    phase_since: f32,
    done: bool,
    tools_open: bool,
}
macro_rules! owned_assets {
    ($($field:ident : $ty:ty),* $(,)?) => {
        #[derive(Default)]
        struct OwnedAssets { $( $field: HashSet<AssetId<$ty>>, )* }
        impl OwnedAssets {
            fn snapshot(world: &World) -> Self {
                Self { $( $field: world.resource::<Assets<$ty>>().ids().collect(), )* }
            }
            fn added_since(self, before: &Self) -> Self {
                Self { $( $field: self.$field.difference(&before.$field).copied().collect(), )* }
            }
            fn release(&self, world: &mut World) -> usize {
                let mut count = 0;
                $( for id in &self.$field {
                    let mut assets = world.resource_mut::<Assets<$ty>>();
                    // Unreferenced setup assets may already have been collected
                    // by Bevy. The invariant is absence after release, not that
                    // every snapshot ID still existed until this frame.
                    count += usize::from(assets.remove(*id).is_some());
                    assert!(!assets.contains(*id), "session asset survived release");
                } )*
                count
            }
            fn count(&self) -> usize { 0 $( + self.$field.len() )* }
        }
    }
}
owned_assets! {
    meshes: Mesh, images: Image, fonts: Font,
    actors: crate::actor_scene::Material,
    rigid_rgb: crate::prop_scene::Material,
    rigid_lit: crate::scenery_scene::Material,
    rigid_cube: crate::sword_scene::Material,
    weapons: crate::actor_scene::LitMaterial,
    terrain: crate::world_scene::WorldColorMaterial,
    sky: crate::sky_scene::SkyMaterial,
    underside: crate::underside_scene::UndersideMaterial,
    water: crate::water_scene::Material,
    patterns: crate::water_texture::Material,
    presentation: crate::world_composition::Presentation,
    buffers: bevy::render::storage::ShaderBuffer,
}
pub fn playing(world: &World) -> bool {
    world.contains_resource::<crate::mecc_dev_scene::State>()
}
pub fn frontend_available(world: &World) -> bool {
    !playing(world)
}
pub fn install(app: &mut App, root: std::path::PathBuf, start_in_menu: bool) {
    let mode = std::env::var("GIANTS_MECC_DEV_REPLAY").ok();
    let replay_cycles = match mode.as_deref() {
        Some("session-cycles") => Some(false),
        Some("session-cycles-play") => Some(true),
        _ => None,
    };
    app.insert_resource(Session {
        root,
        replay_return: mode.as_deref() == Some("session-menu"),
        start_in_menu: start_in_menu || replay_cycles.is_some(),
        replay_cycles,
        ..default()
    })
    .add_systems(Startup, startup.after(crate::frontend_scene::setup))
    .add_systems(
        Update,
        drive
            .before(crate::mecc_dev_scene::update)
            .before(crate::frontend_scene::input)
            .before(crate::frontend_scene::runtime_events),
    );
}
pub fn capture_allowed(world: &World) -> bool {
    let Some(session) = world.get_resource::<Session>() else {
        return true;
    };
    if session.replay_return && playing(world) {
        return false;
    }
    if session.replay_cycles.is_some() && !session.done {
        return false;
    }
    if crate::frontend_scene::kabuto_body_inspection_replay()
        && world
            .get_resource::<crate::mission_runtime::Runtime>()
            .is_some_and(|runtime| runtime.kabuto_body().is_some())
        && world
            .get_resource::<crate::world_scene::Scene>()
            .is_some_and(|scene| {
                scene.camera_source == "declared_start_debug_camera"
                    && scene.cinematic_capture_ready
            })
    {
        return true;
    }
    !session.awaiting_menu
        || world
            .get_resource::<crate::world_scene::Scene>()
            .is_some_and(|s| {
                matches!(
                    s.camera_source,
                    "frontend_intro_source_preview"
                        | "mission_opening"
                        | "mission_player_inspection"
                        | "mission_player_live"
                ) && s.cinematic_capture_ready
            })
}
fn startup(world: &mut World) {
    if world.resource::<Session>().start_in_menu {
        release(world);
        world.resource_mut::<Session>().awaiting_menu = true;
        println!("mecc_development_frontend_entry=true key=F7 campaign_ready=false");
    } else {
        mount(world);
    }
}
fn mount(world: &mut World) {
    // Exclusive bounded setup: concurrent menu/mission work cannot be adopted.
    let before = OwnedAssets::snapshot(world);
    let entities: HashSet<_> = world.query::<Entity>().iter(world).collect();
    macro_rules! setup {
        ($system:path) => {
            world.run_system_once($system).expect(stringify!($system));
        };
    }
    setup!(crate::world_scene::setup);
    setup!(crate::mecc_dev_scene::setup);
    setup!(crate::static_scene::setup);
    setup!(crate::mecc_dev_scene::setup_hud);
    setup!(crate::mecc_combat::setup);
    setup!(crate::mecc_target::setup);
    setup!(crate::mecc_weapon::setup);
    let added = adopt_entities(world, &entities);
    let cameras: Vec<_> = world
        .query_filtered::<Entity, With<crate::frontend_scene::FrontendCamera>>()
        .iter(world)
        .collect();
    for entity in cameras {
        world.entity_mut(entity).remove::<IsDefaultUiCamera>();
    }
    let assets = OwnedAssets::snapshot(world).added_since(&before);
    println!(
        "mecc_session_mounted entities={} assets={} campaign_ready=false",
        added,
        assets.count()
    );
    world.resource_mut::<Session>().assets = Some(assets);
    let now = world.resource::<Time>().elapsed_secs();
    let mut session = world.resource_mut::<Session>();
    session.mounts += 1;
    session.phase_since = now;
    session.awaiting_menu = false;
}
fn adopt_entities(world: &mut World, entities: &HashSet<Entity>) -> usize {
    // Bevy 0.20 stores resources on entities too. They must be removed through
    // remove_resource, never adopted/despawned as rendered session objects.
    let added: Vec<_> = world
        .query_filtered::<Entity, Without<bevy::ecs::resource::IsResource>>()
        .iter(world)
        .filter(|e| !entities.contains(e))
        .collect();
    for entity in &added {
        world.entity_mut(*entity).insert(Owned);
    }
    added.len()
}
fn release(world: &mut World) -> (usize, usize) {
    for mut cursor in world
        .query::<&mut bevy::window::CursorOptions>()
        .iter_mut(world)
    {
        cursor.visible = true;
        cursor.grab_mode = bevy::window::CursorGrabMode::None;
    }

    let entities: Vec<_> = world
        .query_filtered::<Entity, With<Owned>>()
        .iter(world)
        .collect();
    for entity in &entities {
        // Parents recursively remove owned UI children; the list may include those children.
        if world.get_entity(*entity).is_ok() {
            world.despawn(*entity);
        }
    }
    assert_eq!(
        world
            .query_filtered::<Entity, With<Owned>>()
            .iter(world)
            .count(),
        0
    );
    world.remove_resource::<crate::mecc_weapon::Draws>();
    world.remove_resource::<crate::mecc_incoming::Incoming>();
    world.remove_resource::<crate::mecc_combat::Combat>();
    world.remove_resource::<crate::mecc_target::Target>();
    world.remove_resource::<crate::mecc_dev_scene::State>();
    world.remove_resource::<crate::static_scene::Scene>();
    world.remove_resource::<crate::water_scene::RuntimeWater>();
    if let Some(scene) = world.remove_resource::<crate::world_scene::Scene>() {
        world.resource_mut::<Session>().parked = Some(scene);
    }
    let assets = world.resource_mut::<Session>().assets.take();
    let released = assets.as_ref().map_or(0, |a| a.release(world));
    let tracked = assets.as_ref().map_or(0, OwnedAssets::count);
    println!(
        "mecc_session_assets_released removed={released} already_absent={} remaining_owned=0",
        tracked - released
    );
    (entities.len(), released)
}
fn drive(world: &mut World) {
    if !playing(world) {
        menu(world);
        return;
    }
    let focused = world.query::<&Window>().iter(world).any(|w| w.focused);
    let now = world.resource::<Time>().elapsed_secs();
    let session = world.resource::<Session>();
    let age = now - session.phase_since;
    let final_play = session.replay_cycles == Some(true) && session.mounts == 3;
    if final_play && age >= 1. && !session.done {
        world.resource_mut::<Session>().done = true;
        println!(
            "mecc_session_cycles_finished passed=true mounts=3 returns=2 endpoint=gameplay owner_validation=false"
        );
    }
    let replay =
        world.resource::<Session>().replay_return && world.resource::<Time>().elapsed_secs() >= 3.;
    let cycle_return =
        world.resource::<Session>().replay_cycles.is_some() && age >= 1.5 && !final_play;
    if !replay
        && !cycle_return
        && (!focused
            || !world
                .resource::<ButtonInput<KeyCode>>()
                .just_pressed(KeyCode::Escape))
    {
        return;
    }
    let (entities, assets) = release(world);
    let mut session = world.resource_mut::<Session>();
    session.awaiting_menu = true;
    session.returns += 1;
    session.phase_since = now;
    let cameras: Vec<_> = world
        .query_filtered::<Entity, With<crate::frontend_scene::FrontendCamera>>()
        .iter(world)
        .collect();
    for entity in cameras {
        world.entity_mut(entity).insert(IsDefaultUiCamera);
    }
    clear_inputs(world);
    println!(
        "mecc_session_return_menu entities={entities} assets={assets} remaining_owned=0 campaign_ack=false"
    );
}
fn menu_ready(world: &World) -> bool {
    world
        .get_resource::<crate::world_scene::Scene>()
        .is_some_and(|s| {
            s.camera_source == "frontend_intro_source_preview" && s.cinematic_capture_ready
        })
}
fn eligible(
    phase: giants_game::mission_loading::Phase,
    screen: giants_game::frontend::Screen,
    runtime_idle: bool,
) -> bool {
    phase == giants_game::mission_loading::Phase::Menu
        && runtime_idle
        && matches!(
            screen,
            giants_game::frontend::Screen::Main | giants_game::frontend::Screen::PlayerIds
        )
}
fn tools(world: &mut World, hints: &[Entity]) -> (bool, bool) {
    let click = world
        .resource::<ButtonInput<MouseButton>>()
        .just_pressed(MouseButton::Left);
    let point = world
        .query::<&Window>()
        .iter(world)
        .find(|w| w.focused)
        .and_then(|w| w.cursor_position().map(|p| (p, w.width())));
    let open = world.resource::<Session>().tools_open;
    let action = point.and_then(|(p, width)| {
        if !click || p.x < width - 344. || p.x > width - 24. || p.y < 12. {
            return None;
        }
        if p.y < 52. {
            Some(0)
        } else if open && p.y < 252. {
            Some(1 + ((p.y - 52.) / 40.) as usize)
        } else {
            None
        }
    });
    let mut launch = false;
    if let Some(action) = action {
        world
            .resource_mut::<ButtonInput<MouseButton>>()
            .reset(MouseButton::Left);
        for e in hints {
            world.despawn(*e);
        }
        world.resource_mut::<Session>().tools_open = action == 0 && !open;
        match action {
            1 => {
                if let Err(error) = world
                    .resource_mut::<crate::frontend_scene::Scene>()
                    .browse_preview_missions()
                {
                    world
                        .resource_mut::<crate::frontend_scene::Scene>()
                        .preview_notice(error);
                }
                return (false, true);
            }
            2 => {
                if let Err(error) = world
                    .resource_mut::<crate::frontend_scene::Scene>()
                    .browse_completion_replays()
                {
                    world
                        .resource_mut::<crate::frontend_scene::Scene>()
                        .preview_notice(error);
                }
                return (false, true);
            }
            3 => launch = true,
            4 => {
                match crate::desktop::change_installation() {
                    Ok(true) => {
                        world.write_message(AppExit::Success);
                    }
                    Ok(false) => {}
                    Err(error) => world
                        .resource_mut::<crate::frontend_scene::Scene>()
                        .preview_notice(error.to_string()),
                }
                return (false, true);
            }
            _ => {}
        }
    }
    if hints.is_empty() || action.is_some() {
        let camera = world
            .query_filtered::<Entity, With<crate::frontend_scene::FrontendCamera>>()
            .iter(world)
            .next()
            .expect("frontend camera");
        let font = world
            .resource::<crate::frontend_scene::InspectionFont>()
            .0
            .clone();
        let open = world.resource::<Session>().tools_open;
        let panel = world
            .spawn((
                MenuHint,
                UiTargetCamera(camera),
                Node {
                    position_type: PositionType::Absolute,
                    right: px(24.),
                    top: px(12.),
                    width: px(320.),
                    height: px(if open { 240. } else { 40. }),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.02, 0.03, 0.04, 0.88)),
            ))
            .id();
        let labels: &[&str] = if open {
            &[
                "Preview tools",
                "Browse all missions",
                "Completion cinematics",
                "Mecc test arena",
                "Game installation...",
                "Close",
            ]
        } else {
            &["Preview tools"]
        };
        for (row, label) in labels.iter().enumerate() {
            world.spawn((
                ChildOf(panel),
                Text::new(*label),
                TextFont::from_font_size(FontSize::Px(20.)).with_font(font.clone()),
                TextColor(Color::WHITE),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(12.),
                    top: px(row as f32 * 40. + 8.),
                    ..default()
                },
            ));
        }
    }
    (launch, false)
}
fn menu(world: &mut World) {
    let ready = eligible(
        world.resource::<crate::frontend_scene::Scene>().flow.phase,
        world
            .resource::<crate::frontend_scene::Scene>()
            .feedback_selection()
            .0,
        world.resource::<crate::mission_runtime::Runtime>().idle(),
    );
    let hints: Vec<_> = world
        .query_filtered::<Entity, With<MenuHint>>()
        .iter(world)
        .collect();
    if !ready {
        for e in hints {
            world.despawn(e);
        }
        return;
    }
    let (launch_from_menu, stop) = tools(world, &hints);
    if stop {
        return;
    }
    let now = world.resource::<Time>().elapsed_secs();
    let session = world.resource::<Session>();
    if session.replay_cycles == Some(false) && session.returns == 3 && menu_ready(world) {
        if !session.done {
            world.resource_mut::<Session>().done = true;
            println!(
                "mecc_session_cycles_finished passed=true mounts=3 returns=3 endpoint=frontend owner_validation=false"
            );
        }
        return;
    }
    let replay = session.replay_cycles.is_some()
        && !session.done
        && menu_ready(world)
        && now - session.phase_since >= 0.35;
    let focused = world.query::<&Window>().iter(world).any(|w| w.focused);
    if replay
        || launch_from_menu
        || (focused
            && world
                .resource::<ButtonInput<KeyCode>>()
                .just_pressed(KeyCode::F7))
    {
        match launch(world) {
            Ok(()) => {}
            Err(error) => {
                error!("mecc_development_launch_refused={error}");
                if replay {
                    world.write_message(AppExit::Error(std::num::NonZeroU8::MIN));
                }
            }
        }
    }
}
fn launch(world: &mut World) -> Result<(), Box<dyn std::error::Error>> {
    let root = world.resource::<Session>().root.clone();
    let mut scene = world
        .resource_mut::<Session>()
        .parked
        .take()
        .ok_or("development world recipe absent")?;
    // Prepare all CPU inputs before releasing the menu. A refused reader leaves
    // the current frontend and its real mission flow usable.
    let prepared = (|| {
        let actor = crate::mecc_dev_scene::State::load(&root, &scene.asset)?;
        let rigid = crate::static_scene::Scene::from_prepared(
            actor.rigid_source.clone(),
            &scene.asset.config,
        )?;
        if rigid.needs_cube() {
            return Err("Mecc initial rigid reflection provider is not prepared".into());
        }
        let combat = crate::mecc_combat::load(&root)?;
        let incoming = crate::mecc_incoming::load(&root)?;
        let target = crate::mecc_target::load(&actor, &scene)?;
        Ok::<_, Box<dyn std::error::Error>>((actor, combat, incoming, target, rigid))
    })();
    let (actor, combat, mut incoming, target, rigid) = match prepared {
        Ok(prepared) => prepared,
        Err(error) => {
            world.resource_mut::<Session>().parked = Some(scene);
            return Err(error);
        }
    };
    assert!(
        actor.remaining_health() == 100.
            && actor.deaths == 0
            && combat.fresh()
            && incoming.fresh()
            && target.health.damage == 0.
    );
    let (camera, origin) = actor.camera();
    scene.validation_camera = Some(camera);
    scene.camera_origin = Some(origin);
    scene.camera_source = "mecc_development_follow";
    scene.cinematic_capture_ready = true;
    crate::frontend_background::release_for_development(world);
    let hints: Vec<_> = world
        .query_filtered::<Entity, With<MenuHint>>()
        .iter(world)
        .collect();
    for e in hints {
        world.despawn(e);
    }
    let elements: Vec<_> = world
        .query_filtered::<Entity, With<crate::frontend_scene::FrontendElement>>()
        .iter(world)
        .collect();
    for e in elements {
        world.entity_mut(e).insert(Visibility::Hidden);
    }
    let cameras: Vec<_> = world
        .query_filtered::<Entity, With<crate::frontend_scene::FrontendCamera>>()
        .iter(world)
        .collect();
    for e in cameras {
        world
            .get_mut::<Camera>(e)
            .expect("frontend camera")
            .is_active = false;
    }
    let replay = world.resource::<Session>().replay_cycles.is_some();
    if replay {
        incoming.enabled = true;
    }
    world.insert_resource(scene);
    world.insert_resource(actor);
    world.insert_resource(rigid);
    world.insert_resource(combat);
    world.insert_resource(incoming);
    world.insert_resource(target);
    world.init_resource::<crate::mecc_weapon::Draws>();
    clear_inputs(world);
    mount(world);
    println!(
        "mecc_session_launch fresh_health=100 projectiles=0 launch=development campaign_ack=false test_attacker={replay}"
    );
    Ok(())
}
fn clear_inputs(world: &mut World) {
    world.resource_mut::<ButtonInput<KeyCode>>().reset_all();
    world.resource_mut::<ButtonInput<MouseButton>>().reset_all();
    world
        .resource_mut::<Messages<bevy::input::keyboard::KeyboardInput>>()
        .clear();
    world
        .resource_mut::<bevy::input::mouse::AccumulatedMouseMotion>()
        .delta = Vec2::ZERO;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tool_clicks_consume_input_and_only_explicit_arena_action_requests_launch() {
        let mut world = World::new();
        world.init_resource::<Session>();
        world.init_resource::<ButtonInput<MouseButton>>();
        world.insert_resource(crate::frontend_scene::InspectionFont(Handle::default()));
        world.spawn(crate::frontend_scene::FrontendCamera);
        let mut window = Window {
            resolution: bevy::window::WindowResolution::new(1280, 720),
            focused: true,
            ..default()
        };
        window.set_cursor_position(Some(Vec2::new(980., 30.)));
        let window = world.spawn(window).id();
        world
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        assert_eq!(tools(&mut world, &[]), (false, false));
        assert!(world.resource::<Session>().tools_open);
        assert!(
            !world
                .resource::<ButtonInput<MouseButton>>()
                .just_pressed(MouseButton::Left)
        );
        let panels: Vec<_> = world
            .query_filtered::<Entity, With<MenuHint>>()
            .iter(&world)
            .collect();
        assert_eq!(panels.len(), 1);
        world
            .get_mut::<Window>(window)
            .unwrap()
            .set_cursor_position(Some(Vec2::new(980., 150.))); // Arena follows both cinematic tools.
        world
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        assert_eq!(tools(&mut world, &panels), (true, false));
        assert!(!world.resource::<Session>().tools_open);
        assert!(
            !world
                .resource::<ButtonInput<MouseButton>>()
                .just_pressed(MouseButton::Left)
        );
    }
    #[test]
    fn development_launch_does_not_intercept_bindings_or_mission_flow() {
        use giants_game::{frontend::Screen, mission_loading::Phase};
        assert!(eligible(Phase::Menu, Screen::Main, true));
        assert!(eligible(Phase::Menu, Screen::PlayerIds, true));
        assert!(!eligible(Phase::Menu, Screen::Main, false));
        for screen in [
            Screen::Controls(0),
            Screen::NewPlayer,
            Screen::Options,
            Screen::LoadMissions,
            Screen::MissionLoading,
        ] {
            assert!(!eligible(Phase::Menu, screen, true));
        }
        for phase in [
            Phase::AwaitingRuntime,
            Phase::Cinematic,
            Phase::Objectives,
            Phase::Gameplay,
        ] {
            assert!(!eligible(phase, Screen::Main, true));
        }
    }
    #[test]
    fn refused_launch_keeps_existing_entities_and_input() {
        let mut world = World::new();
        world.init_resource::<Session>();
        world.init_resource::<ButtonInput<MouseButton>>();
        world
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        let foreign = world.spawn_empty().id();
        assert!(launch(&mut world).is_err());
        assert!(world.get_entity(foreign).is_ok());
        assert!(
            world
                .resource::<ButtonInput<MouseButton>>()
                .pressed(MouseButton::Left)
        );
        assert!(!playing(&world));
    }
    #[test]
    fn resource_entities_created_during_mount_are_never_adopted() {
        #[derive(Resource, Default)]
        struct LateResource;
        let mut world = World::new();
        let shared = world.spawn_empty().id();
        let before = world.query::<Entity>().iter(&world).collect();
        world.init_resource::<LateResource>();
        let rendered = world.spawn_empty().id();
        assert_eq!(adopt_entities(&mut world, &before), 1);
        assert!(world.get::<Owned>(rendered).is_some());
        assert!(world.get::<Owned>(shared).is_none());
        let owned: Vec<_> = world
            .query_filtered::<Entity, With<Owned>>()
            .iter(&world)
            .collect();
        for entity in owned {
            world.despawn(entity);
        }
        assert!(world.contains_resource::<LateResource>());
        world.remove_resource::<LateResource>();
        assert!(!world.contains_resource::<LateResource>());
    }
    #[test]
    fn menu_transition_discards_held_fire_movement_and_mouse_delta() {
        let mut world = World::new();
        world.init_resource::<ButtonInput<KeyCode>>();
        world.init_resource::<ButtonInput<MouseButton>>();
        world.init_resource::<Messages<bevy::input::keyboard::KeyboardInput>>();
        world.insert_resource(bevy::input::mouse::AccumulatedMouseMotion {
            delta: Vec2::new(30., -50.),
        });
        world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyW);
        world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        world
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        clear_inputs(&mut world);
        assert!(
            !world
                .resource::<ButtonInput<KeyCode>>()
                .any_pressed([KeyCode::KeyW, KeyCode::Escape])
        );
        assert!(
            !world
                .resource::<ButtonInput<MouseButton>>()
                .pressed(MouseButton::Left)
        );
        assert!(
            !world
                .resource::<ButtonInput<MouseButton>>()
                .just_pressed(MouseButton::Left)
        );
        assert_eq!(
            world
                .resource::<bevy::input::mouse::AccumulatedMouseMotion>()
                .delta,
            Vec2::ZERO
        );
    }
    #[test]
    fn release_keeps_shared_and_later_foreign_assets_and_entities() {
        let mut world = World::new();
        world.init_resource::<Assets<Image>>();
        world.init_resource::<Session>();
        let shared = world.resource_mut::<Assets<Image>>().add(Image::default());
        let owned = world.resource_mut::<Assets<Image>>().add(Image::default());
        let foreign = world.resource_mut::<Assets<Image>>().add(Image::default());
        let collected = world.resource_mut::<Assets<Image>>().add(Image::default());
        world.resource_mut::<Assets<Image>>().remove(collected.id());
        world.resource_mut::<Session>().assets = Some(OwnedAssets {
            images: HashSet::from([owned.id(), collected.id()]),
            ..default()
        });
        let parent = world.spawn(Owned).id();
        let child = world.spawn((Owned, ChildOf(parent))).id();
        let other = world.spawn_empty().id();
        let pointer = world
            .spawn(bevy::window::CursorOptions {
                visible: false,
                grab_mode: bevy::window::CursorGrabMode::Locked,
                ..default()
            })
            .id();
        assert_eq!(release(&mut world), (2, 1));
        assert!(world.get_entity(parent).is_err() && world.get_entity(child).is_err());
        assert!(world.get_entity(other).is_ok());
        let cursor = world.get::<bevy::window::CursorOptions>(pointer).unwrap();
        assert!(cursor.visible);
        assert_eq!(cursor.grab_mode, bevy::window::CursorGrabMode::None);
        let images = world.resource::<Assets<Image>>();
        assert!(images.contains(shared.id()) && images.contains(foreign.id()));
        assert!(!images.contains(owned.id()));
        assert_eq!(release(&mut world), (0, 0));
    }
}
