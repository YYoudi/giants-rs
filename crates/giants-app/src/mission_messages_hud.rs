//! Owned message queue consumer. Native draw requests, approximate borrowed-font
//! raster; live actor-notice context is not supplied by this presentation layer.
use bevy::prelude::*;
use giants_game::message_presentation::{Draw, Palette, commands};

#[derive(Resource, Default)]
struct State {
    mounted: Option<Mounted>,
    failed: Option<u64>,
}
struct Mounted {
    ticket: u64,
    camera: Entity,
    labels: Vec<Entity>,
    palette: Palette,
    last: Option<(Vec<Draw>, u32)>,
}
#[derive(Component)]
struct MessageLabel;
pub struct Plugin;
impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<State>().add_systems(
            Update,
            update
                .after(crate::mission_player::PlayerUpdate)
                .run_if(crate::mecc_session::frontend_available),
        );
    }
}
fn release(world: &mut World) {
    let Some(m) = world.resource_mut::<State>().mounted.take() else {
        return;
    };
    for label in m.labels {
        world.despawn(label);
    }
    world.despawn(m.camera);
    println!(
        "mission_messages_released ticket={} remaining_labels={} shared_font_preserved=true",
        m.ticket,
        world
            .query_filtered::<Entity, With<MessageLabel>>()
            .iter(world)
            .count()
    );
}
fn update(world: &mut World) {
    let frontend = world.resource::<crate::frontend_scene::Scene>();
    let ticket = frontend.flow.ticket;
    let permitted = world
        .resource::<crate::mission_showcase::State>()
        .controls(frontend.flow.phase)
        && world
            .resource::<crate::mission_runtime::Runtime>()
            .active_ticket()
            == Some(ticket);
    let wanted = if permitted {
        world
            .query::<&crate::mission_player::Controller>()
            .iter(world)
            .find(|c| c.prepared(ticket))
            .map(|c| (ticket, c.message_time()))
    } else {
        None
    };
    if world
        .resource::<State>()
        .mounted
        .as_ref()
        .is_some_and(|m| Some(m.ticket) != wanted.map(|v| v.0))
    {
        release(world);
    }
    let Some((ticket, now)) = wanted else {
        world.resource_mut::<State>().failed = None;
        return;
    };
    if world.resource::<State>().failed == Some(ticket) {
        return;
    }
    if world.resource::<State>().mounted.is_none() {
        let palette = match world
            .resource::<crate::frontend_scene::Scene>()
            .message_palette()
        {
            Ok(palette) => palette,
            Err(reason) => {
                world.resource_mut::<State>().failed = Some(ticket);
                println!("mission_messages_refused ticket={ticket} reason={reason}");
                return;
            }
        };
        let camera = world
            .spawn((
                crate::mission_runtime::MissionOwned(ticket),
                Camera2d,
                Camera {
                    order: 102,
                    clear_color: ClearColorConfig::None,
                    ..default()
                },
                Msaa::Off,
            ))
            .id();
        world.resource_mut::<State>().mounted = Some(Mounted {
            ticket,
            camera,
            labels: vec![],
            palette,
            last: None,
        });
        println!(
            "mission_messages_mounted ticket={ticket} source=owned_queue clock=own_player_simulation live_clock_phase_verified=false native_font_raster_verified=false"
        );
        if let Ok(mode) = std::env::var("GIANTS_MESSAGE_PROBE")
            && matches!(mode.as_str(), "render" | "expire")
        {
            let time = if mode == "expire" { now - 7. } else { now };
            for text in [
                b"DIAGNOSTIC: message queue render probe".as_slice(),
                b"\x0bLarge text probe",
                b"\x0cCentered text probe",
            ] {
                world
                    .resource_mut::<crate::mission_runtime::Runtime>()
                    .probe_message(ticket, text, time)
                    .expect("owned finite probe clock");
            }
            println!(
                "mission_message_probe ticket={ticket} mode={mode} posts=3 route=6 synthetic=true actor_notice_consumed=false os_input=false"
            );
        }
    }
    let rows = {
        let mounted = world.resource::<State>().mounted.as_ref().unwrap();
        commands(
            world
                .resource::<crate::mission_runtime::Runtime>()
                .messages(ticket)
                .unwrap(),
            &mounted.palette,
            now,
            false,
            false,
        )
    };
    let rows = match rows {
        Ok(rows) => rows,
        Err(reason) => {
            println!("mission_messages_refused ticket={ticket} reason={reason}");
            release(world);
            world.resource_mut::<State>().failed = Some(ticket);
            return;
        }
    };
    let height = world
        .query::<&Window>()
        .iter(world)
        .next()
        .map_or(1080., Window::height);
    if world
        .resource::<State>()
        .mounted
        .as_ref()
        .unwrap()
        .last
        .as_ref()
        .is_some_and(|(previous, bits)| previous == &rows && *bits == height.to_bits())
    {
        return;
    }
    let (camera, old) = {
        let mut state = world.resource_mut::<State>();
        let mounted = state.mounted.as_mut().unwrap();
        (mounted.camera, std::mem::take(&mut mounted.labels))
    };
    for entity in old {
        world.despawn(entity);
    }
    let font = world
        .resource::<crate::frontend_scene::InspectionFont>()
        .0
        .clone();
    let mut labels = vec![];
    for row in &rows {
        if !row
            .text
            .iter()
            .all(|&b| matches!(b, 32..=126 | 9 | 10 | 13))
        {
            println!(
                "mission_message_text_refused ticket={ticket} reason=unverified_text_encoding_or_control"
            );
            continue;
        }
        let color = row.color;
        let entity = world
            .spawn((
                crate::mission_runtime::MissionOwned(ticket),
                MessageLabel,
                UiTargetCamera(camera),
                Node {
                    position_type: PositionType::Absolute,
                    left: percent(if row.centered { 0. } else { row.x * 100. }),
                    top: px(height * (row.y - row.size * 15. / 24.)),
                    width: percent(if row.centered {
                        100.
                    } else {
                        (1. - row.x) * 100.
                    }),
                    justify_content: if row.centered {
                        JustifyContent::Center
                    } else {
                        JustifyContent::Start
                    },
                    ..default()
                },
            ))
            .id();
        // NoWrap text has intrinsic glyph width; center that child in a full
        // width row rather than relying on paragraph justification alone.
        world.spawn((
            ChildOf(entity),
            Text::new(String::from_utf8(row.text.clone()).unwrap()),
            TextFont::from_font_size(FontSize::Px(height * row.size)).with_font(font.clone()),
            TextColor(Color::srgb(
                ((color >> 16) & 255) as f32 / 255.,
                ((color >> 8) & 255) as f32 / 255.,
                (color & 255) as f32 / 255.,
            )),
            TextLayout::new(Justify::Left, LineBreak::NoWrap),
        ));
        labels.push(entity);
    }
    println!(
        "mission_messages_draw ticket={ticket} requests={} labels={} route_colors=source_table native_draw_requests=true native_font_raster_verified=false",
        rows.len(),
        labels.len()
    );
    let mut state = world.resource_mut::<State>();
    let mounted = state.mounted.as_mut().unwrap();
    mounted.labels = labels;
    mounted.last = Some((rows, height.to_bits()));
}
