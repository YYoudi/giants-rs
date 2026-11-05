//! Current vertical slice; simulation cadence and physics remain undecided.
pub mod actor_affine;
pub mod actor_common_motion;
pub mod actor_environment;
pub mod actor_fields;
pub mod actor_finalization;
pub mod actor_references;
pub mod actor_surface_reset;
pub mod actor_terrain;
pub mod actor_volume;
pub mod actor_world_tick;
pub mod arithmetic;
pub mod attached_effect_context;
pub mod attached_effect_draw;
pub mod attached_effect_object;
pub mod attachment_animation;
pub mod attachment_geometry;
pub mod attachment_pose;
pub mod attachment_resources;
pub mod burst_effect_draw;
pub mod camera_facing;
pub mod camera_matrix;
pub mod draw_routes;
pub mod effect_animation;
pub mod effect_attachment;
pub mod effect_burst;
pub mod effect_initialization;
pub mod facing_direction;
pub mod ground_contact;
pub mod held_weapon;
pub mod hit_routing;
pub mod instances;
pub mod kabuto_player;
pub mod kabuto_resources;
pub mod kabuto_size;
pub mod mecc_collection;
pub mod mecc_contact_materials;
pub mod mecc_countdowns;
pub mod mecc_damage;
pub mod mecc_death;
pub mod mecc_fire;
pub mod mecc_fire_lock;
pub mod mecc_fire_primary;
pub mod mecc_jet;
pub mod mecc_jet_owned;
pub mod mecc_jet_presentation;
pub mod mecc_periodic_scan;
pub mod mecc_presentation;
mod mecc_secondary;
pub mod mecc_shot;
pub mod mecc_special;
pub mod mecc_target_owned;
pub mod mecc_target_query;
pub mod mecc_wield;
pub mod mecc_wield_pose;
pub mod mission_child12;
pub mod player_input_channels;
pub mod resident_environment;
pub mod resident_finalization;
pub mod resident_motion;
pub mod scalar_track;
pub mod specification;
mod state_attributes;
pub mod state_resources;
pub mod state_storage;
pub mod straight_projectile;
pub mod uncached_pose;
pub mod unlit_material;
pub mod vimp_chunks;
use giants_formats::{
    gzp::Archive,
    tga::{self, RgbImage},
};
use std::path::Path;
pub mod ambient_streams;
pub mod animation;
pub mod animation_event_dispatch;
pub mod animation_events;
pub mod animation_links;
pub mod attachments;
pub mod audio_options;
pub mod bump_image;
pub mod carried_actor;
pub mod cast;
pub mod catalog_registry;
pub mod child_creation;
pub mod cinematic_clock;
pub mod cinematic_fade;
pub mod cinematic_frame;
pub mod cinematic_fx;
pub mod cinematic_offset;
pub mod cinematic_render_policy;
pub mod cinematic_viewport;
pub mod control_layout;
pub mod control_options;
pub mod declared_mesh_scene;
pub mod decorations;
pub mod delphi_player;
pub mod delphi_resources;
pub mod delphi_special;
pub mod dependency_graph;
pub mod director_initial;
pub mod display_options;
pub mod effect_visual;
pub mod flick;
pub mod foliage;
pub mod frame_clock;
pub mod frontend;
pub mod frontend_background;
pub mod graphics_options;
pub mod lighting;
pub mod lod;
pub mod material_descriptor;
pub mod model_loading;
pub mod model_pose;
pub mod model_registry;
pub mod music_buffer;
pub mod music_context;
pub mod music_schedule;
pub mod music_stream;
pub mod object_culling;
pub mod object_links;
pub mod object_list_walk;
pub mod object_lists;
pub mod object_reference;
pub mod object_segment;
pub mod object_time;
pub mod object_transform;
pub mod procedural_aim;
pub mod procedural_bindings;
pub mod profile_store;
pub mod projectile_contacts;
pub mod projectile_retaliation;
pub mod projectile_world;
pub mod props;
pub mod reflection;
pub mod render_entry;
pub mod render_parts;
pub mod render_sort;
pub mod renderer_policy;
pub mod scene_resources;
pub mod scenery;
pub mod skinning;
pub mod smartie_carry;
pub mod smartie_drop;
pub mod static_geometry;
pub mod stream_resources;
pub mod sun_glare;
pub mod target_runtime;
pub mod targeting;
pub mod terrain_direction;
pub mod terrain_grid;
pub mod terrain_material;
pub mod terrain_surface;
pub mod terrain_uv;
pub mod terrain_visibility;
pub mod texture_registry;
pub mod uv_basis;
pub mod water;
pub mod weapons;

/// Keep the missing resource path in loader diagnostics, including nested loads.
pub(crate) fn read_asset(path: impl AsRef<Path>) -> std::io::Result<Vec<u8>> {
    let path = path.as_ref();
    std::fs::read(path)
        .map_err(|error| std::io::Error::new(error.kind(), format!("{}: {error}", path.display())))
}

pub struct WorldAsset {
    pub name: String,
    pub catalog: String,
    pub member: Vec<u8>,
    pub terrain: giants_formats::gti::Terrain,
    pub positions: Vec<[f32; 3]>,
    pub triangles: Vec<[u32; 3]>,
    pub face_classes: Vec<u8>,
    pub packed_terrain_tags: Vec<u16>,
    pub terrain_row_spans: Vec<[i32; 2]>,
    pub additional: Option<terrain_surface::Prepared>,
    pub config: giants_formats::world::WorldConfig,
    pub textures: Vec<TerrainTexture>,
    pub sky: EnvironmentAsset,
    pub underside: EnvironmentAsset,
    pub sun: [f32; 3],
}
pub struct EnvironmentAsset {
    pub mesh: giants_formats::gb2::Mesh,
    pub image: RgbImage,
}
pub struct TerrainTexture {
    pub name: String,
    pub levels: Vec<giants_formats::pixels::MipLevel>,
}
pub fn load_world(root: &Path, name: &str) -> Result<WorldAsset, Box<dyn std::error::Error>> {
    let source = world_source::resolve(root, name)?;
    let resources = world_source::bin_resources(root)?;
    let config_path = root.join("Bin").join(&source.catalog);
    let config = giants_formats::world::parse_world(&std::fs::read(config_path)?)?;
    build_world(root, &source.name, source.catalog, config, &resources)
}

/// Common environment preparation from the selected catalog and shared Bin index.
/// Reader support does not establish native initialization or rendering fidelity.
pub(crate) fn build_world(
    root: &Path,
    name: &str,
    catalog: String,
    config: giants_formats::world::WorldConfig,
    resources: &catalog_registry::ResourceIndex,
) -> Result<WorldAsset, Box<dyn std::error::Error>> {
    let read = |member: &str| {
        let source = resources.select_bin(member, "English")?;
        println!(
            "world_resource member={} archive={}",
            source.member,
            source
                .archive
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
        );
        resources.read_bin(member, "English")
    };
    let mut suns = config.lights.iter().filter(|l| l.variant == Some(1));
    let sun_light = suns.next().ok_or("World has no supported sun object")?;
    if suns.next().is_some() {
        return Err("Multiple world suns require native ordering proof".into());
    }
    let sun = object_transform::sun(sun_light.angles, sun_light.scale)?;
    let bindings = giants_formats::gb2::bindings(&std::fs::read(root.join("Bin/GameObj.bin"))?)?;
    let binding = bindings
        .iter()
        .rev()
        .find(|b| b.id == 1)
        .ok_or("Sky binding missing")?;
    if !binding.container.eq_ignore_ascii_case("world.gb2") {
        return Err("Unsupported environment container".into());
    }
    let extra = std::fs::read(root.join("Bin/extra.gzp"))?;
    let extra = Archive::parse(&extra)?;
    let sky_mesh = giants_formats::gb2::parse(&extra.read(&binding.container)?, &binding.object)?;
    let sky_image = tga::decode(&read(&format!("{}.tga", config.sky))?)?;
    let sky = EnvironmentAsset {
        mesh: sky_mesh,
        image: sky_image,
    };
    let binding = bindings
        .iter()
        .rev()
        .find(|b| b.id == 8)
        .ok_or("Underside binding missing")?;
    if !binding.container.eq_ignore_ascii_case("world.gb2") {
        return Err("Unsupported underside container".into());
    }
    let underside = EnvironmentAsset {
        mesh: giants_formats::gb2::parse(&extra.read(&binding.container)?, &binding.object)?,
        image: tga::decode(&read(&format!("{}.tga", config.underside))?)?,
    };
    let mut textures = Vec::new();
    for material in &config.prefix.materials {
        let material = material
            .as_ref()
            .ok_or("undeclared base terrain material consumers remain unverified for rendering")?;
        let image = tga::decode(&read(&format!("{}.tga", material.name))?)?;
        if !matches!((image.width, image.height), (512, 512) | (256, 256)) {
            return Err("Terrain texture dimensions outside profile".into());
        }
        let levels = giants_formats::pixels::terrain_mipmaps(
            &image.pixels,
            image.width,
            image.height,
            config.mip_curve,
        )?;
        textures.push(TerrainTexture {
            name: material.name.clone(),
            levels,
        });
    }
    let member = read(&config.prefix.terrain)?;
    let terrain = giants_formats::gti::parse(&member)?;
    let terrain_grid::Prepared {
        terrain,
        positions,
        triangles,
        face_classes,
        packed_tags,
        row_spans,
    } = terrain_grid::prepare(terrain, config.slope_thresholds)?;
    let additional = terrain_surface::prepare_debug_hardware_with(read, &terrain, &config)?;
    Ok(WorldAsset {
        name: name.into(),
        catalog,
        member,
        terrain,
        positions,
        triangles,
        face_classes,
        packed_terrain_tags: packed_tags,
        terrain_row_spans: row_spans,
        additional,
        config,
        sun,
        textures,
        sky,
        underside,
    })
}
pub fn world_control(asset: &WorldAsset) -> Vec<u8> {
    let t = &asset.terrain;
    let mut out = b"GMTI0007".to_vec();
    for v in [t.width, t.height, t.consumed_bytes, asset.triangles.len()] {
        out.extend((v as u32).to_le_bytes());
    }
    for v in [
        t.origin[0],
        t.origin[1],
        t.spacing,
        t.height_bounds[0],
        t.height_bounds[1],
    ] {
        out.extend(v.to_le_bytes());
    }
    for v in &t.heights {
        out.extend(v.to_le_bytes());
    }
    for v in &t.colors {
        out.extend(v.to_le_bytes());
    }
    out.extend(&t.tags);
    for p in &asset.positions {
        for v in p {
            out.extend(v.to_le_bytes());
        }
    }
    for triangle in &asset.triangles {
        for v in triangle {
            out.extend(v.to_le_bytes());
        }
    }
    out.extend((t.opaque_tail.len() as u32).to_le_bytes());
    out.extend(&t.opaque_tail);
    out.extend(&asset.face_classes);
    out.extend((asset.config.prefix.consumed_offset as u32).to_le_bytes());
    out.extend((asset.config.prefix.block_end as u32).to_le_bytes());
    for (material, texture) in asset.config.prefix.materials.iter().zip(&asset.textures) {
        let material = material
            .as_ref()
            .expect("world preparation validates base materials");
        out.extend((material.name.len() as u32).to_le_bytes());
        out.extend(material.name.as_bytes());
        out.extend(material.scale.to_le_bytes());
        out.extend(material.reciprocal.to_le_bytes());
        out.extend((texture.levels.len() as u32).to_le_bytes());
        for mip in &texture.levels {
            out.extend(mip.width.to_le_bytes());
            out.extend(mip.height.to_le_bytes());
            for word in &mip.words {
                out.extend(word.to_le_bytes());
            }
        }
    }
    let c = &asset.config;
    out.extend(c.fog.start.to_le_bytes());
    out.extend(c.fog.end.to_le_bytes());
    out.extend(c.fog.color);
    out.push(0);
    for name in [&c.sky, &c.sea_texture, &c.underside] {
        out.extend((name.len() as u32).to_le_bytes());
        out.extend(name.as_bytes());
    }
    for value in c
        .sea_values
        .into_iter()
        .chain(c.mip_curve)
        .chain(c.slope_thresholds)
    {
        out.extend(value.to_le_bytes());
    }
    out.extend((c.opcodes.len() as u32).to_le_bytes());
    for (at, opcode) in &c.opcodes {
        out.extend((*at as u32).to_le_bytes());
        out.push(*opcode);
    }
    append_environment(&mut out, &asset.sky);
    out.extend(c.water_fog.start.to_le_bytes());
    out.extend(c.water_fog.end.to_le_bytes());
    out.extend(c.water_fog.color);
    out.push(0);
    append_environment(&mut out, &asset.underside);
    for color in c.water.colors {
        out.extend(color);
    }
    for value in c.water.values {
        out.extend(value.to_le_bytes());
    }
    out
}
fn append_environment(out: &mut Vec<u8>, sky: &EnvironmentAsset) {
    for name in [&sky.mesh.name, &sky.mesh.texture] {
        out.extend((name.len() as u32).to_le_bytes());
        out.extend(name.as_bytes());
    }
    out.extend((sky.mesh.positions.len() as u32).to_le_bytes());
    out.extend((sky.mesh.triangles.len() as u32).to_le_bytes());
    for p in &sky.mesh.positions {
        for v in p {
            out.extend(v.to_le_bytes());
        }
    }
    for uv in &sky.mesh.uv {
        for v in uv {
            out.extend(v.to_le_bytes());
        }
    }
    for t in &sky.mesh.triangles {
        for v in t {
            out.extend(v.to_le_bytes());
        }
    }
    out.extend(sky.image.width.to_le_bytes());
    out.extend(sky.image.height.to_le_bytes());
    out.extend(&sky.image.pixels);
}

pub struct LogoAsset {
    pub member: Vec<u8>,
    pub mesh: giants_formats::gbs::Mesh,
    pub normals: Vec<[f32; 3]>,
}
pub struct LogoTexture {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub words: Vec<u16>,
    pub rgba: Vec<u8>,
}
pub fn load_logo_textures(root: &Path) -> Result<Vec<LogoTexture>, Box<dyn std::error::Error>> {
    let data = std::fs::read(root.join("Bin/xx_giants_logo_3d.gzp"))?;
    let archive = Archive::parse(&data)?;
    let mut result = Vec::new();
    for (name, width, height) in [("Giants_logo_512", 512, 256), ("Giants_logo_rep", 128, 128)] {
        let member = archive.read(&format!("{name}.tga"))?;
        let image = tga::decode(&member)?;
        if (image.width, image.height) != (width, height) {
            return Err("Logo texture outside measured dimensions".into());
        }
        let words = giants_formats::pixels::pack_rgb565(&image.pixels)?;
        let rgba = giants_formats::pixels::rgba8(&words);
        result.push(LogoTexture {
            name: name.into(),
            width,
            height,
            words,
            rgba,
        });
    }
    Ok(result)
}
pub fn load_logo(root: &Path) -> Result<LogoAsset, Box<dyn std::error::Error>> {
    let path = root.join("Bin").join("xx_giants_logo_3d.gzp");
    let data = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let member = Archive::parse(&data)?.read("Giants_logo_3D.gbs")?;
    let mesh = giants_formats::gbs::parse(&member)?;
    let normals = mesh.render_normals()?;
    Ok(LogoAsset {
        member,
        mesh,
        normals,
    })
}

/// Diagnostic interchange for independent controls, not an on-disk game format.
pub fn mesh_control(asset: &LogoAsset) -> Vec<u8> {
    let mesh = &asset.mesh;
    let mut out = b"GBSC01\0\0".to_vec();
    for count in [
        mesh.positions.len(),
        mesh.indices.len(),
        mesh.subobjects.len(),
    ] {
        out.extend((count as u32).to_le_bytes());
    }
    for p in &mesh.positions {
        for v in p {
            out.extend(v.to_le_bytes());
        }
    }
    for i in &mesh.indices {
        out.extend(i.to_le_bytes());
    }
    for uv in &mesh.uv {
        for v in uv {
            out.extend(v.to_le_bytes());
        }
    }
    for rgb in &mesh.rgb {
        out.extend(rgb);
    }
    for normal in &asset.normals {
        for v in normal {
            out.extend(v.to_le_bytes());
        }
    }
    for sub in &mesh.subobjects {
        out.extend((sub.triangles.len() as u32).to_le_bytes());
        for tri in &sub.triangles {
            for index in tri {
                out.extend(index.to_le_bytes());
            }
        }
    }
    out
}

pub struct LoadingAsset {
    pub member: Vec<u8>,
    pub image: RgbImage,
}
pub fn load_mecc(root: &Path) -> Result<LoadingAsset, Box<dyn std::error::Error>> {
    let path = root.join("Bin").join("xx_giants_logo_3d.gzp");
    let data = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let member = Archive::parse(&data)?.read("mecc_loading.tga")?;
    let image = tga::decode(&member)?;
    if (image.width, image.height) != (640, 480) {
        return Err("Only measured Mecc 640x480 profile supported".into());
    }
    Ok(LoadingAsset { member, image })
}

/// Source crop and measured screen rectangle. Screen coordinates use native pixel centers.
pub struct LoadingTile {
    pub source: [u32; 4],
    pub screen: [f32; 4],
}
pub fn loading_tiles() -> Vec<LoadingTile> {
    let mut tiles = Vec::new();
    for row in 0..4 {
        for column in 0..5 {
            let height = (480 - row * 128).min(128);
            tiles.push(LoadingTile {
                source: [column * 128, row * 128, 128, height],
                screen: [
                    480.0 + column as f32 * 192.0,
                    270.0 + row as f32 * 144.0,
                    192.0,
                    height as f32 * 144.0 / 128.0,
                ],
            });
        }
    }
    tiles
}
#[cfg(test)]
mod tests {
    #[test]
    fn measured_tiles_cover_native_rectangle() {
        let tiles = super::loading_tiles();
        assert_eq!(tiles.len(), 20);
        assert_eq!(tiles[0].screen, [480., 270., 192., 144.]);
        assert_eq!(tiles[19].source, [512, 384, 128, 96]);
        assert_eq!(tiles[19].screen, [1248., 702., 192., 108.]);
    }
}

pub mod mission_selection;

pub mod actor_context;
pub mod actor_links;
pub mod actor_sound;
pub mod animation_channels;
pub mod animation_loop;
pub mod animation_references;
pub mod animation_template;
pub mod base_layout;
pub mod base_layout_scene;
pub mod body_pipeline;
pub mod class_template;
pub mod collider_copy;
pub mod collision_bounds;
pub mod collision_geometry;
pub mod collision_radius;
pub mod collision_world;
pub mod declared_residents;
pub mod footprint_builder;
pub mod footprint_policy;
pub mod mecc_collision_candidates;
pub mod mecc_control_force;
pub mod mecc_control_target;
pub mod mecc_gravity;
pub mod mecc_initial;
pub mod mecc_mesh_contact;
pub mod mecc_mesh_dispatch;
pub mod mecc_motion;
pub mod mecc_physical_cycle;
pub mod mecc_physical_dispatch;
pub mod mecc_physics;
pub mod mission_cinematics;
pub mod mission_loading;
pub mod mission_proxy;
pub mod mission_slot;
pub mod mission_state;
pub mod physical_ground;
pub mod plane_response;
pub mod platform_queries;
pub mod player_collision_radius;
pub mod player_equipment;
pub mod player_initial;
pub mod player_mesh_contact;
pub mod player_start;
pub mod player_world_collision;
pub mod segment_queries;
pub mod terrain_queries;
pub mod vertex_cache;
pub mod volume_queries;

pub mod collider_init;

pub mod mecc_camera;
pub mod mecc_camera_update;
pub mod mecc_camera_wall;
pub mod mecc_load_mass;
pub mod object_time_precision;

pub mod mecc_control_state;
pub mod mecc_inventory;
pub mod mecc_location_dispatch;
pub mod mecc_locomotion;
pub mod mecc_model_motion;

pub mod actor_primary_gate;
pub mod animation_epoch;
pub mod audio_parameters;
pub mod audio_sequence;
pub mod audio_spatial;
pub mod basic_projectile;
pub mod cinematic_credits;
pub mod cinematic_sound;
pub mod gameplay_sound_bank;
pub mod mecc_aim;
pub mod mecc_ammunition;
mod mecc_animation_events;
pub mod mecc_animation_phase;
pub mod mecc_audio;
pub mod mecc_auxiliary;
pub mod mecc_footsteps;
pub mod mecc_impacts;
pub mod mecc_pistol;
pub mod mecc_primary;
pub mod mecc_world;
pub mod positional_effect_audio;
pub mod primary_enemy_fire;
pub mod primary_surface_marker;
pub mod projectile_definition;
pub mod projectile_effect_requests;
pub mod projectile_launch;
pub mod sound_bindings;
pub mod sound_descriptor;
pub mod sound_playback;
pub mod sound_resources;
pub mod sound_sample;
pub mod world_source;

pub mod timed_effect;

pub mod cinematic_ordinary;
pub mod cinematic_shadow;
pub mod cinematic_shadow_source;
pub mod cinematic_shadow_terrain;
pub mod cinematic_text;
pub mod effect_composition;
pub mod effect_contact;
pub mod effect_emission;
pub mod effect_emitter;
pub mod effect_emitter_runtime;
pub mod effect_orientation;
pub mod effect_projectile;
pub mod effect_reference_emission;
pub mod effect_scene;
pub mod effect_scene_runtime;
pub mod effect_sequence;
pub mod projectile_flight_audio;

pub mod actor_position_buffers;
pub mod animation_frames;
pub mod animation_mixing;
pub mod backpack_drop;
pub mod blood_particles;
pub mod contact_pose;
pub mod effect_surface;
pub mod first_person;
pub mod font_atlas;
pub mod font_layout;
pub mod font_rasterizer;
pub mod message_presentation;
pub mod mission_messages;
pub mod mission_notice;
pub mod pose_mixing;
pub mod released_effect;
pub mod resident_states;
pub mod ripper_attack;
pub mod ripper_awakening;
pub mod ripper_dirt;
pub mod ripper_emergence;
pub mod ripper_ground;
pub mod ripper_perception;
pub mod ripper_primary_turn;
pub mod ripper_pursuit;
pub mod ripper_steering;
pub mod ripper_turn_completion;
pub mod state23_animation;
pub mod state23_damage;
pub mod state23_entry;
pub mod state23_hurt;
pub mod subjective_player;
pub mod surface_marker;
pub mod vertical_fade;

pub mod enemy_ballistic;
pub mod enemy_projectile;

pub mod enemy_flight;

pub mod controlled_mecc;

pub mod vimp_actor;
pub mod vimp_animation;
pub mod vimp_damage;
pub mod vimp_death_proxy;
pub mod vimp_frame;
pub mod vimp_hurt;
pub mod vimp_living_scalars;
pub mod vimp_physical;
pub mod vimp_recovery;
pub mod vimp_wander;

pub mod herd_generator;

pub mod effect_texture_animation;
pub mod effect_tracer;
pub mod effect_tracer_owner;
pub mod effect_tracer_registry;
pub mod effect_tracer_ribbon;
pub mod herd_markers;
pub mod sound_device;
pub mod sound_owner;
pub mod sound_spatial;
