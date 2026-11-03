//! Director-owned, source-driven silhouette textures. Terrain projection is separate.
use bevy::{
    ecs::system::SystemParam,
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};
use giants_game::{
    actor_links::{Factory, Projection},
    cinematic_shadow_source,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

#[derive(Resource)]
struct Options {
    root: PathBuf,
    models: [usize; 32],
    capture: Option<PathBuf>,
    capture_after: f32,
    receivers: bool,
}
struct Entry {
    pass: crate::shadow_silhouette::Pass,
    extent: [u32; 2],
    frames: u32,
    updates: u64,
    captured: bool,
    receiver: Option<crate::shadow_receiver::Pass>,
}
#[derive(Resource, Default)]
struct State {
    key: String,
    sources: Vec<Result<Projection, String>>,
    owner: Option<(u64, usize)>,
    entries: BTreeMap<usize, Entry>,
    buffers: BTreeMap<String, Vec<[f32; 3]>>,
    needs: BTreeSet<String>,
    sampled: Option<(i32, usize)>,
    last_second: Option<i32>,
}
#[derive(SystemParam)]
struct Inputs<'w> {
    cinematic: Option<Res<'w, crate::cinematic_camera::Cinematic>>,
    world: Option<Res<'w, crate::world_scene::Scene>>,
    instances: Option<Res<'w, crate::source_instances::Scene>>,
    pipelines: Res<'w, crate::PipelinesReady>,
    options: Res<'w, Options>,
    view: Option<Res<'w, crate::source_view::SourceView>>,
}
#[derive(SystemParam)]
struct Assets<'w> {
    meshes: ResMut<'w, bevy::asset::Assets<Mesh>>,
    materials: ResMut<'w, bevy::asset::Assets<crate::shadow_silhouette::Material>>,
    images: ResMut<'w, bevy::asset::Assets<Image>>,
    receivers: ResMut<'w, bevy::asset::Assets<crate::shadow_receiver::Material>>,
}
pub fn install(app: &mut App, root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut models = [0; 32];
    if let Ok(overrides) = std::env::var("GIANTS_SHADOW_MODEL_SLOTS") {
        for value in overrides.split(',').filter(|v| !v.is_empty()) {
            let (slot, model) = value
                .split_once(':')
                .ok_or("shadow model override needs slot:model")?;
            let slot = slot.parse::<usize>()?;
            let model = model.parse::<usize>()?;
            if slot >= 32 || model >= 10 {
                return Err("shadow model override outside profile".into());
            }
            models[slot] = model;
        }
    }
    let capture = std::env::var_os("GIANTS_SHADOW_CAPTURE_DIR").map(PathBuf::from);
    if let Some(path) = &capture {
        let evidence = std::env::current_dir()?.join("evidence").canonicalize()?;
        if path.exists()
            || !path
                .parent()
                .ok_or("shadow capture parent absent")?
                .canonicalize()?
                .starts_with(evidence)
        {
            return Err("fresh private shadow capture directory required".into());
        }
        std::fs::create_dir(path)?;
    }
    let capture_after = std::env::var("GIANTS_SHADOW_CAPTURE_AFTER_SECONDS")
        .map_or(Ok(1.), |s| s.parse::<f32>())?;
    if !capture_after.is_finite() || !(0. ..=600.).contains(&capture_after) {
        return Err("invalid shadow capture threshold".into());
    }
    crate::shadow_silhouette::install(app);
    crate::shadow_receiver::install(app);
    let receivers = match std::env::var("GIANTS_SHADOW_RECEIVERS").as_deref() {
        Ok("0") => false,
        Ok("1") | Err(std::env::VarError::NotPresent) => true,
        _ => return Err("shadow receivers expects0 or1".into()),
    };
    app.insert_resource(Options {
        root: root.into(),
        models,
        capture,
        capture_after,
        receivers,
    })
    .init_resource::<State>()
    .add_systems(PostUpdate, update.after(crate::cinematic_camera::update));
    Ok(())
}
fn retire(commands: &mut Commands, state: &mut State, slot: usize) {
    if let Some(entry) = state.entries.remove(&slot) {
        if let Some(receiver) = entry.receiver {
            commands.entity(receiver.entity).despawn();
        }
        for entity in entry.pass.entities {
            commands.entity(entity).despawn();
        }
        info!(
            "cinematic_shadow_retired slot={} updates={} director_owned=true",
            slot, entry.updates
        );
    }
}
fn clear(commands: &mut Commands, state: &mut State) {
    for slot in state.entries.keys().copied().collect::<Vec<_>>() {
        retire(commands, state, slot);
    }
    state.sampled = None;
    state.last_second = None;
}
fn need(state: &mut State, sequence: &str, slot: usize, reason: String) {
    let key = format!("{sequence}:{slot}:{reason}");
    if state.needs.insert(key) {
        warn!(
            "cinematic_shadow_need sequence={} slot={} reason={:?} texture_ready=false terrain_ready=false",
            sequence, slot, reason
        );
    }
}
fn load(
    inputs: &Inputs,
    cinematic: &crate::cinematic_camera::Cinematic,
) -> Result<Vec<Result<Projection, String>>, String> {
    let world = inputs.world.as_ref().ok_or("shadow world not mounted")?;
    let resources = if let Some(instances) = &inputs.instances {
        instances.plan.resources().clone()
    } else {
        std::sync::Arc::new(
            giants_game::scene_resources::SceneResources::load(
                &inputs.options.root,
                &world.asset.catalog,
                "English",
            )
            .map_err(|e| e.to_string())?,
        )
    };
    let effects = giants_formats::effect_image::parse(
        &std::fs::read(inputs.options.root.join("Bin/fx_world_dust.bin"))
            .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let mut factory = Factory::new(&resources, &effects);
    Ok(cinematic
        .segments()
        .iter()
        .map(|segment| {
            let path = giants_game::flick::sequence_path(&inputs.options.root, segment.sequence())
                .map_err(|e| e.to_string())?;
            let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
            factory.project(&bytes)
        })
        .collect())
}
fn update(mut commands: Commands, inputs: Inputs, mut state: ResMut<State>, mut assets: Assets) {
    let Some(cinematic) = &inputs.cinematic else {
        if state.owner.take().is_some() {
            clear(&mut commands, &mut state);
        }
        return;
    };
    let Some(world) = &inputs.world else {
        clear(&mut commands, &mut state);
        return;
    };
    let key = format!(
        "{}:{:?}",
        world.asset.catalog,
        cinematic
            .segments()
            .iter()
            .map(|s| s.sequence())
            .collect::<Vec<_>>()
    );
    if cinematic.is_added() || state.key != key {
        clear(&mut commands, &mut state);
        state.buffers.clear();
        state.needs.clear();
        state.key = key;
        state.sources = match load(&inputs, cinematic) {
            Ok(s) => s,
            Err(e) => {
                need(&mut state, cinematic.sequence(), 0, e);
                Vec::new()
            }
        };
    }
    let owner = (cinematic.generation(), cinematic.director_index());
    if state.owner != Some(owner) {
        clear(&mut commands, &mut state);
        state.owner = Some(owner);
    }
    let Some(cursor) = cinematic.reached_command_offset() else {
        clear(&mut commands, &mut state);
        return;
    };
    let seconds = cinematic.pose_seconds();
    let declarations: Vec<_> = (0..32)
        .filter_map(|slot| {
            cinematic.shadow_declaration(slot).filter(|_| {
                !cinematic.render_policy().actor_auxiliary_disabled
                    && cinematic.actor_render_visible(slot) == Some(true)
            })
        })
        .collect();
    let wanted: BTreeSet<_> = declarations.iter().map(|d| d.actor_slot).collect();
    for slot in state
        .entries
        .keys()
        .copied()
        .filter(|s| !wanted.contains(s))
        .collect::<Vec<_>>()
    {
        retire(&mut commands, &mut state, slot);
    }
    let sample_key = (cinematic.tick(), cursor);
    if state.sampled != Some(sample_key) {
        state.sampled = Some(sample_key);
        for declaration in declarations {
            let slot = declaration.actor_slot;
            let sampled = (|| -> Result<(_, String), String> {
                let source = state
                    .sources
                    .get(owner.1)
                    .ok_or("shadow source context unavailable")?
                    .as_ref()
                    .map_err(Clone::clone)?;
                let actor = source
                    .actors
                    .get(&slot)
                    .ok_or("shadow actor missing from source context")?;
                let model_slot = inputs.options.models[slot];
                let model = actor
                    .class
                    .specification
                    .as_ref()
                    .ok_or("shadow model table absent")?
                    .models
                    .get(model_slot)
                    .and_then(Option::as_ref)
                    .ok_or("shadow preview model absent")?;
                let name = model.name.to_ascii_lowercase();
                let mut clocks = BTreeMap::new();
                let mut locals = BTreeMap::new();
                for (&index, a) in &source.actors {
                    if cinematic.actor_scale(index, a.class.scale) != Some(a.class.scale) {
                        return Err("dynamic actor scale needs shadow parent-scale binding".into());
                    }
                    clocks.insert(
                        index,
                        if a.director_slot.is_some() {
                            seconds
                        } else {
                            0.
                        },
                    );
                    locals.insert(index, cinematic.shadow_local(index)?);
                }
                let sample = cinematic_shadow_source::prepare(
                    source,
                    cinematic_shadow_source::Request {
                        declaration,
                        cursor_offset: cursor,
                        model_slot,
                        part_mask: u64::MAX,
                        clocks: &clocks,
                        locals: &locals,
                        retained_positions: state.buffers.get(&name).map(Vec::as_slice),
                    },
                )?;
                Ok((sample, name))
            })();
            let (sample, name) = match sampled {
                Ok(s) => s,
                Err(e) => {
                    retire(&mut commands, &mut state, slot);
                    need(&mut state, cinematic.sequence(), slot, e);
                    continue;
                }
            };
            let receiver = (|| -> Result<_, String> {
                if !inputs.options.receivers || !world.render_terrain {
                    return Ok(None);
                }
                let view = inputs
                    .view
                    .as_ref()
                    .ok_or("shadow receiver source view unavailable")?;
                let camera = world
                    .validation_camera
                    .as_ref()
                    .ok_or("shadow receiver needs source camera")?;
                let bounds = [
                    declaration.bounds_words[0][0],
                    declaration.bounds_words[0][1],
                    declaration.bounds_words[1][0],
                    declaration.bounds_words[1][1],
                ]
                .map(f32::from_bits);
                let mapping =
                    giants_game::cinematic_shadow::Mapping::prepare(sample.frame, bounds)?;
                let geometry = giants_game::cinematic_shadow_terrain::project_loaded(
                    &world.asset.terrain,
                    &mapping,
                )?;
                let fog = &world.asset.config.fog;
                let bindings = crate::shadow_receiver::Bindings {
                    texture: Handle::default(),
                    clip: view.buffer.clone(),
                    pixel_strength: Vec4::new(
                        1. / camera.viewport.z as f32,
                        -1. / camera.viewport.w as f32,
                        160. / 255.,
                        0.,
                    ),
                    fog: Vec4::new(fog.start, fog.end, if world.no_fog { 0. } else { 1. }, 0.),
                    fog_color: Vec4::new(
                        fog.color[0] as f32 / 255.,
                        fog.color[1] as f32 / 255.,
                        fog.color[2] as f32 / 255.,
                        1.,
                    ),
                };
                Ok(Some((geometry, bindings)))
            })();
            let request = crate::shadow_silhouette::Request {
                camera: sample.camera,
                extent: sample.extent,
                positions: sample.positions,
                source_world: Mat4::from_cols_array(&sample.source_world.values).transpose(),
                layer: 32 + slot,
            };
            state.buffers.insert(name, sample.position_buffer);
            if state
                .entries
                .get(&slot)
                .is_some_and(|e| e.extent != sample.extent)
            {
                retire(&mut commands, &mut state, slot);
            }
            let result = if let Some(entry) = state.entries.get_mut(&slot) {
                let result = crate::shadow_silhouette::update(
                    &entry.pass,
                    &mut assets.meshes,
                    &mut assets.materials,
                    &request,
                );
                if result.is_ok() {
                    entry.updates += 1;
                }
                result
            } else {
                crate::shadow_silhouette::spawn(&mut commands,&mut assets.meshes,&mut assets.materials,&mut assets.images,request).map(|pass|{
                    state.entries.insert(slot,Entry {pass,extent:sample.extent,frames:0,updates:1,captured:false,receiver:None});
                    info!("cinematic_shadow_texture sequence={} slot={} model={} model_policy=explicit_preview native_lod_verified=false native_visibility_verified=false texture_ready=true terrain_ready=false",cinematic.sequence(),slot,inputs.options.models[slot]);
                })
            };
            if let Err(e) = result {
                retire(&mut commands, &mut state, slot);
                need(&mut state, cinematic.sequence(), slot, e.into());
                continue;
            }
            let entry = state.entries.get_mut(&slot).expect("prepared shadow entry");
            match receiver {
                Ok(Some((geometry, mut bindings))) if !geometry.vertices.is_empty() => {
                    bindings.texture = entry.pass.image.clone();
                    let result = if let Some(pass) = &entry.receiver {
                        crate::shadow_receiver::update(
                            pass,
                            &mut assets.meshes,
                            &mut assets.receivers,
                            &geometry,
                            bindings,
                        )
                    } else {
                        if let Some(mut image) = assets.images.get_mut(&entry.pass.image) {
                            image.sampler = bevy::image::ImageSampler::linear();
                        }
                        entry.receiver = Some(crate::shadow_receiver::spawn(
                            &mut commands,
                            &mut assets.meshes,
                            &mut assets.receivers,
                            &geometry,
                            bindings,
                        ));
                        info!(
                            "cinematic_shadow_receiver sequence={} slot={} vertices={} cells={} static_terrain=true native_scene_fidelity_verified=false",
                            cinematic.sequence(),
                            slot,
                            geometry.vertices.len(),
                            geometry.visited_cells
                        );
                        Ok(())
                    };
                    if let Err(error) = result {
                        if let Some(pass) = entry.receiver.take() {
                            commands.entity(pass.entity).despawn();
                        }
                        error!("shadow receiver update failed: {error}");
                    }
                }
                result => {
                    if let Some(pass) = entry.receiver.take() {
                        commands.entity(pass.entity).despawn();
                    }
                    if let Ok(Some((geometry, _))) = &result {
                        let key = format!("empty:{}:{slot}", cinematic.sequence());
                        if state.needs.insert(key) {
                            info!(
                                "cinematic_shadow_receiver_empty sequence={} slot={} cells={} frame_origin={:?} grid_origin={:?} grid_size={:?} grid_spacing={} static_terrain=true",
                                cinematic.sequence(),
                                slot,
                                geometry.visited_cells,
                                [
                                    sample.frame.values[3],
                                    sample.frame.values[7],
                                    sample.frame.values[11]
                                ],
                                world.asset.terrain.origin,
                                [world.asset.terrain.width, world.asset.terrain.height],
                                world.asset.terrain.spacing
                            );
                        }
                    }
                    if let Err(error) = result {
                        let key = format!("receiver:{}:{slot}:{error}", cinematic.sequence());
                        if state.needs.insert(key) {
                            warn!(
                                "cinematic_shadow_receiver_need sequence={} slot={} reason={:?} texture_allocated=true terrain_ready=false",
                                cinematic.sequence(),
                                slot,
                                error
                            );
                        }
                    }
                }
            }
        }
    }
    for (&slot, entry) in &mut state.entries {
        entry.frames = entry.frames.saturating_add(1);
        if !entry.captured
            && inputs.pipelines.0
            && entry.frames >= 4
            && seconds >= inputs.options.capture_after
            && let Some(folder) = &inputs.options.capture
        {
            entry.captured = true;
            let name = format!("g{}-d{:03}-slot{:02}", owner.0, owner.1, slot);
            let text = format!(
                "sequence={}\ntick={}\nseconds_bits={}\nmodel_slot={}\nupdates={}\nsource_camera=true\nterrain_receiver_allocated={}\nnative_terrain_verified=false\n",
                cinematic.sequence(),
                cinematic.tick(),
                seconds.to_bits(),
                inputs.options.models[slot],
                entry.updates,
                entry.receiver.is_some()
            );
            if let Err(error) = std::fs::write(folder.join(format!("{name}.txt")), text) {
                error!("Shadow capture metadata failed: {error}");
                continue;
            }
            commands
                .spawn(Screenshot::image(entry.pass.image.clone()))
                .observe(save_to_disk(folder.join(format!("{name}.png"))));
        }
    }
    let second = seconds.floor() as i32;
    if !state.entries.is_empty() && state.last_second != Some(second) {
        state.last_second = Some(second);
        info!(
            "cinematic_shadow_playback sequence={} active={} tick={} continuous=true initial_channels_profile=true terrain_policy=static_prepared_grid native_fidelity_verified=false",
            cinematic.sequence(),
            state.entries.len(),
            cinematic.tick()
        );
    }
}
