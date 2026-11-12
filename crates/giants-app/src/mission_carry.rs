//! Ticket-owned visual pickup preview. A drawn, animated source model does not
//! acknowledge command12: live construction, callbacks and services stay pending.
use crate::{actor_scene, mission_runtime::Runtime};
use bevy::{asset::RenderAssetUsages, prelude::*};
use giants_game::{animation_loop::Clock, carried_actor::Prepared};
use std::{collections::BTreeMap, sync::Arc};

pub(crate) struct Plugin;
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct AudioUpdate;
impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<State>().add_systems(
            Update,
            (pickup_declared, sync, apply_load, update, publish_audio)
                .chain()
                .in_set(AudioUpdate)
                .after(crate::mission_player::PlayerUpdate)
                .before(crate::mission_showcase::update)
                .before(crate::source_view::ViewUpdate)
                .run_if(crate::mecc_session::frontend_available),
        );
    }
}
fn pickup_declared(
    mut runtime: ResMut<Runtime>,
    frontend: Res<crate::frontend_scene::Scene>,
    showcase: Res<crate::mission_showcase::State>,
    cinematic: Option<Res<crate::cinematic_camera::Cinematic>>,
    players: Query<(
        &crate::mission_runtime::PlayerInitial,
        &crate::mission_player::Controller,
    )>,
    mut exit: MessageWriter<AppExit>,
) {
    if cinematic.is_some() || !showcase.controls(frontend.flow.phase) {
        return;
    }
    let Some(ticket) = runtime.active_ticket() else {
        return;
    };
    let Some((_, controller)) = players
        .iter()
        .find(|(p, c)| p.ticket == ticket && c.prepared(ticket))
    else {
        return;
    };
    if let Err(error) = runtime.advance_declared_smartie_pickup(ticket, |prepared| {
        controller.carried_parent_point(prepared)
    }) {
        error!("Declared Smartie pickup failed: {error}");
        exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
    }
}
fn apply_load(
    runtime: Res<Runtime>,
    mut players: Query<(
        &crate::mission_runtime::PlayerInitial,
        &mut crate::mission_player::Controller,
    )>,
) {
    for (player, mut controller) in &mut players {
        let carried = runtime.carried_response(player.ticket) & 0x800000 != 0;
        if let Err(error) = controller.set_carried_load(carried) {
            println!(
                "mission_carried_load_refused ticket={} error={error:?}",
                player.ticket
            );
        }
    }
}
fn publish_audio(
    mut runtime: ResMut<Runtime>,
    mut state: ResMut<State>,
    players: Query<(
        &crate::mission_runtime::PlayerInitial,
        &crate::mission_player::Controller,
    )>,
    mut audio: MessageWriter<crate::mission_audio::Request>,
) {
    let Some(ticket) = runtime.active_ticket() else {
        return;
    };
    let Some((_, controller)) = players
        .iter()
        .find(|(p, c)| p.ticket == ticket && c.prepared(ticket))
    else {
        return;
    };
    let mounted = state.mounted.as_mut().filter(|m| m.ticket == ticket);
    let point = mounted.as_ref().map(|m| m.point);
    match runtime.publish_actor_audio(ticket, controller.actor_audio_position(), point) {
        Ok((parent, Some(child))) => {
            if let Some(m) = mounted
                && !m.audio_queued
            {
                audio.write(crate::mission_audio::Request {
                    ticket,
                    event: crate::mission_audio::Event::InitialCarryPreview { parent, child },
                });
                m.audio_queued = true;
                println!(
                    "mission_actor_audio_queued ticket={ticket} source=initial_carry_preview backend_started=false local_notice_pending=true"
                );
            }
        }
        Ok(_) => {}
        Err(error) => {
            if mounted.is_some_and(|m| !m.audio_queued) {
                println!(
                    "mission_actor_audio_refused ticket={ticket} error={error:?} backend_started=false"
                );
            }
        }
    }
}
#[derive(Component)]
struct DrawnCarry {
    ticket: u64,
}
struct Mounted {
    ticket: u64,
    source_offset: usize,
    prepared: Arc<Prepared>,
    entities: Vec<Entity>,
    meshes: Vec<Handle<Mesh>>,
    images: Vec<Handle<Image>>,
    materials: Vec<Handle<actor_scene::Material>>,
    clock: Clock,
    first: [f32; 3],
    point: [f32; 3],
    matrix: [f32; 16],
    updates: u32,
    moved: bool,
    animated: bool,
    initial_positions: Vec<[f32; 3]>,
    audio_queued: bool,
}
#[derive(Resource, Default)]
struct State {
    mounted: Option<Mounted>,
    failed: Option<u64>,
}

pub(crate) fn resident(world: &World, ticket: u64) -> bool {
    let Some(mounted) = world
        .get_resource::<State>()
        .and_then(|s| s.mounted.as_ref())
        .filter(|m| m.ticket == ticket)
    else {
        return false;
    };
    !mounted.entities.is_empty()
        && mounted.matrix.into_iter().all(f32::is_finite)
        && mounted.point.into_iter().all(f32::is_finite)
        && mounted.entities.len() == mounted.prepared.body.mesh.subobjects.len()
        && mounted.entities.iter().all(|e| {
            world
                .get::<DrawnCarry>(*e)
                .is_some_and(|p| p.ticket == ticket)
                && world.get::<Mesh3d>(*e).is_some()
        })
        && mounted
            .meshes
            .iter()
            .all(|h| world.resource::<Assets<Mesh>>().contains(h.id()))
}

fn vertices(
    prepared: &Prepared,
    part: usize,
    positions: &[[f32; 3]],
    matrix: [f32; 16],
) -> Vec<[f32; 3]> {
    prepared.body.mesh.subobjects[part]
        .triangles
        .iter()
        .flatten()
        .map(|id| {
            let p = positions[usize::from(prepared.body.mesh.indices[usize::from(*id)])];
            let p = std::array::from_fn::<_, 3, _>(|r| {
                let m = &matrix[r * 4..r * 4 + 4];
                (((f64::from(m[0]) * f64::from(p[0]) + f64::from(m[1]) * f64::from(p[1]))
                    + f64::from(m[2]) * f64::from(p[2]))
                    + f64::from(m[3])) as f32
            });
            [p[0], p[2], -p[1]]
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn sync(
    mut commands: Commands,
    runtime: Res<Runtime>,
    players: Query<(
        &crate::mission_runtime::PlayerInitial,
        &crate::mission_player::Controller,
    )>,
    scene: Option<Res<crate::world_scene::Scene>>,
    view: Res<crate::source_view::SourceView>,
    mut state: ResMut<State>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<actor_scene::Material>>,
) {
    let candidate = runtime.carried_preview();
    let wanted = candidate.as_ref().map(|c| c.0);
    if state
        .mounted
        .as_ref()
        .is_some_and(|m| Some(m.ticket) != wanted)
    {
        let old = state.mounted.take().unwrap();
        for entity in old.entities {
            commands.entity(entity).try_despawn();
        }
        for h in &old.meshes {
            meshes.remove(h.id());
        }
        for h in &old.images {
            images.remove(h.id());
        }
        for h in &old.materials {
            materials.remove(h.id());
        }
        assert!(old.meshes.iter().all(|h| !meshes.contains(h.id())));
        assert!(old.images.iter().all(|h| !images.contains(h.id())));
        assert!(old.materials.iter().all(|h| !materials.contains(h.id())));
        println!(
            "mission_carry_preview_released ticket={} source_offset={} meshes={} images={} materials={} updates={} moved={} animated={} remaining_assets=0 actor_backend_ready=false",
            old.ticket,
            old.source_offset,
            old.meshes.len(),
            old.images.len(),
            old.materials.len(),
            old.updates,
            old.moved,
            old.animated
        );
    }
    if state.failed.is_some_and(|ticket| Some(ticket) != wanted) {
        state.failed = None;
    }
    if state.mounted.is_some() {
        return;
    }
    let (Some((ticket, source_offset, prepared)), Some(scene)) = (candidate, scene) else {
        return;
    };
    if state.failed == Some(ticket) {
        return;
    }
    let Some((_, controller)) = players
        .iter()
        .find(|(p, c)| p.ticket == ticket && c.prepared(ticket))
    else {
        return;
    };
    let result = (|| -> Result<_, Box<dyn std::error::Error>> {
        if prepared.body.mesh.normal_count != 0 {
            return Err("carried preview requires checked unlit body surfaces".into());
        }
        for part in &prepared.body.mesh.subobjects {
            let surface = giants_game::body_pipeline::preview_surface(
                giants_game::body_pipeline::Normals::select(false, false, false, true),
                &part.material,
                prepared
                    .body
                    .alpha_textures
                    .contains(&part.material.texture.to_ascii_lowercase()),
            )?;
            if surface != giants_game::body_pipeline::Surface::Rgb {
                return Err("carried preview part requires another body renderer".into());
            }
        }
        Ok((
            prepared.positions(0.)?,
            controller.carried_matrix(&prepared, 0.)?,
        ))
    })();
    let (positions, matrix) = match result {
        Ok(v) => v,
        Err(e) => {
            println!(
                "mission_carry_preview_refused ticket={ticket} source_offset={source_offset} error={e:?} actor_backend_ready=false"
            );
            state.failed = Some(ticket);
            return;
        }
    };
    let point = [matrix[3], matrix[7], matrix[11]];
    let mut mounted = Mounted {
        ticket,
        source_offset,
        prepared,
        entities: Vec::new(),
        meshes: Vec::new(),
        images: Vec::new(),
        materials: Vec::new(),
        clock: Clock::default(),
        first: point,
        point,
        matrix,
        updates: 0,
        moved: false,
        animated: false,
        initial_positions: positions.clone(),
        audio_queued: false,
    };
    let mut textures = BTreeMap::new();
    for source in &mounted.prepared.body.textures {
        let mut image = Image::new(
            bevy::render::render_resource::Extent3d {
                width: source.width,
                height: source.height,
                depth_or_array_layers: 1,
            },
            bevy::render::render_resource::TextureDimension::D2,
            source.rgba.clone(),
            bevy::render::render_resource::TextureFormat::Rgba8Unorm,
            RenderAssetUsages::default(),
        );
        image.sampler = crate::logo_material::sampler();
        let h = images.add(image);
        textures.insert(source.name.to_ascii_lowercase(), h.clone());
        mounted.images.push(h);
    }
    for (index, part) in mounted.prepared.body.mesh.subobjects.iter().enumerate() {
        let ids = part
            .triangles
            .iter()
            .flatten()
            .map(|i| usize::from(*i))
            .collect::<Vec<_>>();
        let colors = ids
            .iter()
            .map(|i| {
                if part.material.flags & 0x20 != 0 {
                    let rgb = mounted.prepared.body.mesh.rgb[*i];
                    [
                        rgb[0] as f32 / 255.,
                        rgb[1] as f32 / 255.,
                        rgb[2] as f32 / 255.,
                        1.,
                    ]
                } else {
                    [1.; 4]
                }
            })
            .collect::<Vec<_>>();
        let mesh = meshes.add(
            Mesh::new(
                bevy::render::render_resource::PrimitiveTopology::TriangleList,
                RenderAssetUsages::default(),
            )
            .with_inserted_attribute(
                Mesh::ATTRIBUTE_POSITION,
                vertices(&mounted.prepared, index, &positions, matrix),
            )
            .with_inserted_attribute(
                Mesh::ATTRIBUTE_UV_0,
                ids.iter()
                    .map(|i| mounted.prepared.body.mesh.uv[*i])
                    .collect::<Vec<_>>(),
            )
            .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors),
        );
        let fog = scene.asset.config.fog;
        let alpha = if part.material.flags & 1 != 0 {
            ((1. - part.material.falloff) * 255.) as u8 as f32 / 255.
        } else {
            1.
        };
        let material = materials.add(actor_scene::Material::from(actor_scene::Bindings {
            texture: textures[&part.material.texture.to_ascii_lowercase()].clone(),
            clip_from_bevy: view.buffer.clone(),
            validation: Vec4::new(1., 1. / 1920., -1. / 1080., alpha),
            fog: Vec4::new(fog.start, fog.end, 1., 0.),
            fog_color: if part.material.flags & 1 != 0 {
                Vec4::ZERO
            } else {
                Vec4::new(
                    fog.color[0] as f32 / 255.,
                    fog.color[1] as f32 / 255.,
                    fog.color[2] as f32 / 255.,
                    0.,
                )
            },
            force_additive: false,
            blend: mounted
                .prepared
                .body
                .alpha_textures
                .contains(&part.material.texture.to_ascii_lowercase()),
        }));
        let entity = commands
            .spawn((
                DrawnCarry { ticket },
                crate::mission_runtime::MissionOwned(ticket),
                Name::new(format!("Carried source pose preview / {}", part.name)),
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material.clone()),
                Visibility::Hidden,
                bevy::camera::visibility::NoFrustumCulling,
            ))
            .id();
        mounted.entities.push(entity);
        mounted.meshes.push(mesh);
        mounted.materials.push(material);
    }
    println!(
        "mission_carry_preview_bound ticket={ticket} source_offset={source_offset} class={} parts={} textures={} reference={:?} point={point:?} parent_corrections={} child_corrections={} raw_parent_pose=true unblended_animation={} actor_backend_ready=false animation_flags={} reference_speed={} transition_seconds={} blend_applied=false",
        mounted.prepared.class,
        mounted.entities.len(),
        mounted.images.len(),
        mounted.prepared.parent_reference,
        mounted.prepared.parent_corrections,
        mounted.prepared.child_corrections,
        mounted.prepared.pickup_animation.slot,
        mounted.prepared.pickup_animation.flags,
        mounted.prepared.pickup_animation.reference_speed,
        mounted.prepared.pickup_animation.blend_seconds
    );
    state.mounted = Some(mounted);
}

#[allow(clippy::too_many_arguments)]
fn update(
    mut runtime: ResMut<Runtime>,
    frontend: Res<crate::frontend_scene::Scene>,
    showcase: Res<crate::mission_showcase::State>,
    cinematic: Option<Res<crate::cinematic_camera::Cinematic>>,
    time: Res<Time>,
    mut state: ResMut<State>,
    players: Query<(
        &crate::mission_runtime::PlayerInitial,
        &crate::mission_player::Controller,
    )>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut draws: Query<(&DrawnCarry, &mut Visibility)>,
) {
    let Some(mounted) = state.mounted.as_mut() else {
        return;
    };
    if runtime.active_ticket() != Some(mounted.ticket) {
        return;
    }
    let visible = cinematic.is_none()
        && (showcase.is_mecc_preview()
            || showcase.is_free()
            || frontend.flow.phase == giants_game::mission_loading::Phase::Gameplay);
    for (draw, mut visibility) in &mut draws {
        if draw.ticket == mounted.ticket {
            *visibility = if visible {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
    }
    if !showcase.controls(frontend.flow.phase) || cinematic.is_some() {
        return;
    }
    let Some((_, controller)) = players
        .iter()
        .find(|(p, c)| p.ticket == mounted.ticket && c.prepared(mounted.ticket))
    else {
        return;
    };
    let result = (|| -> Result<_, Box<dyn std::error::Error>> {
        mounted.clock =
            runtime.advance_carried_clock(mounted.ticket, time.delta_secs().clamp(0., 0.1))?;
        Ok((
            controller.carried_matrix_from_reference(
                &mounted.prepared,
                runtime.carried_reference(mounted.ticket)?,
            )?,
            mounted.prepared.positions(mounted.clock.seconds)?,
        ))
    })();
    let (matrix, positions) = match result {
        Ok(v) => v,
        Err(e) => {
            println!(
                "mission_carry_preview_update_refused ticket={} error={e:?} actor_backend_ready=false",
                mounted.ticket
            );
            return;
        }
    };
    for (part, h) in mounted.meshes.iter().enumerate() {
        if let Some(mut mesh) = meshes.get_mut(h) {
            mesh.insert_attribute(
                Mesh::ATTRIBUTE_POSITION,
                vertices(&mounted.prepared, part, &positions, matrix),
            );
        }
    }
    let point = [matrix[3], matrix[7], matrix[11]];
    mounted.point = point;
    mounted.matrix = matrix;
    mounted.updates += 1;
    mounted.moved |= point
        .iter()
        .zip(mounted.first)
        .any(|(a, b)| (a - b).abs() > 0.01);
    mounted.animated |= positions != mounted.initial_positions;
    if [1, 60, 240, 480].contains(&mounted.updates) {
        println!(
            "mission_carry_reference_lock ticket={} parent_reference={} child_reference={} current_child_reference=true matrix_finite={} actor_backend_ready=false declared_parent_correction={}",
            mounted.ticket,
            giants_game::smartie_carry::MECC_PARENT_REFERENCE,
            giants_game::smartie_carry::MECC_CHILD_REFERENCE,
            mounted.matrix.into_iter().all(f32::is_finite),
            mounted.prepared.parent_reference_links.iter().any(|link| {
                u32::from(link.reference_slot) == giants_game::smartie_carry::MECC_PARENT_REFERENCE
            })
        );
        println!(
            "mission_carry_preview_update ticket={} updates={} point={point:?} seconds={} moved={} animated={} actor_backend_ready=false owned_clock=true reference_lock=true",
            mounted.ticket, mounted.updates, mounted.clock.seconds, mounted.moved, mounted.animated
        );
    }
}
