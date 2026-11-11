//! Mission-owned clip-space blood billboards. Normal NPC particles are copied
//! from the mission component, which alone advances their wounds and clocks.
//! The optional probe retains its separate supplied-burst simulation.
use bevy::{
    asset::RenderAssetUsages,
    mesh::{MeshVertexAttribute, MeshVertexBufferLayoutRef},
    prelude::*,
    reflect::TypePath,
    render::render_resource::*,
    shader::ShaderRef,
};
use giants_game::{
    audio_parameters::Random,
    blood_particles::{Actor, Configuration, Pool},
};
const OFFSET: MeshVertexAttribute =
    MeshVertexAttribute::new("BloodClipOffset", 0xb100d, VertexFormat::Float32x2);
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
struct Bindings {
    #[texture(0)]
    #[sampler(1)]
    image: Handle<Image>,
    #[storage(2, read_only)]
    view: Handle<bevy::render::storage::ShaderBuffer>,
    #[uniform(3)]
    pixel: Vec4,
}
type Material = crate::source_view::UniformMaterial<Bindings>;
impl bevy::pbr::Material for Material {
    fn vertex_shader() -> ShaderRef {
        "embedded://giants_app/blood.wesl".into()
    }
    fn fragment_shader() -> ShaderRef {
        Self::vertex_shader()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
    fn specialize(
        _: &bevy::pbr::MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.vertex.buffers = vec![layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(1),
            Mesh::ATTRIBUTE_COLOR.at_shader_location(2),
            OFFSET.at_shader_location(3),
        ])?];
        descriptor.primitive.cull_mode = None;
        if let Some(depth) = &mut descriptor.depth_stencil {
            depth.depth_write_enabled = Some(false);
        }
        if let Some(fragment) = &mut descriptor.fragment {
            for target in fragment.targets.iter_mut().flatten() {
                target.blend = Some(BlendState {
                    color: BlendComponent {
                        src_factor: BlendFactor::SrcAlpha,
                        dst_factor: BlendFactor::OneMinusSrcAlpha,
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
        Ok(())
    }
}
struct Mounted {
    ticket: u64,
    pool: Pool,
    entity: Entity,
    mesh: Handle<Mesh>,
    material: Handle<Material>,
    image: Handle<Image>,
    bursts: usize,
    frames: u32,
    probe: bool,
    npc_reported: bool,
}
#[derive(Resource)]
pub(crate) struct State {
    mounted: Option<Mounted>,
    probe: bool,
    probe_fired: bool,
    probe_ready: bool,
    pose_frames: u32,
}
impl Default for State {
    fn default() -> Self {
        let pose_frames = std::env::var("GIANTS_BLOOD_PROBE_FRAMES").map_or(9, |s| {
            s.parse().expect("blood probe frames must be an integer")
        });
        assert!(
            (1..=120).contains(&pose_frames),
            "blood probe frames outside 1..120"
        );
        Self {
            mounted: None,
            probe: std::env::var("GIANTS_BLOOD_PROBE").ok().as_deref() == Some("pose"),
            probe_fired: false,
            probe_ready: false,
            pose_frames,
        }
    }
}
impl State {
    pub(crate) fn capture_pending(&self) -> bool {
        self.probe && !self.probe_ready
    }
}
pub struct Plugin;
impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<Material>::default())
            .init_resource::<State>()
            .add_systems(
                Update,
                update
                    .after(crate::mission_player::PlayerUpdate)
                    .before(crate::source_view::ViewUpdate),
            );
    }
}
fn release(world: &mut World) {
    let Some(m) = world.resource_mut::<State>().mounted.take() else {
        return;
    };
    world.despawn(m.entity);
    world.resource_mut::<Assets<Mesh>>().remove(m.mesh.id());
    world
        .resource_mut::<Assets<Material>>()
        .remove(m.material.id());
    world.resource_mut::<Assets<Image>>().remove(m.image.id());
    println!(
        "mission_blood_released ticket={} bursts={} active={} remaining_assets=0 native_pixels_verified=false",
        m.ticket,
        m.bursts,
        m.pool.active().count()
    );
}
fn mount(world: &mut World, ticket: u64) -> Result<(), String> {
    let root = world
        .resource::<crate::frontend_scene::Scene>()
        .data_root()
        .to_owned();
    let bytes = std::fs::read(root.join("Bin/GData.gbn")).map_err(|e| e.to_string())?;
    let globals = giants_formats::global_data::parse(&bytes).map_err(|e| e.to_string())?;
    let config = Configuration::from_globals(&globals)?;
    let pool = Pool::new(config)?;
    let index = giants_game::catalog_registry::ResourceIndex::scan(&root.join("Bin"))
        .map_err(|e| e.to_string())?;
    let bytes = index
        .read_bin(giants_game::blood_particles::TEXTURE, "English")
        .map_err(|e| e.to_string())?;
    let giants_formats::tga::Image::TrueColor(decoded) =
        giants_formats::tga::decode_typed(&bytes).map_err(|e| e.to_string())?
    else {
        return Err("blood texture requires true-color upload".into());
    };
    let mut rgba = Vec::with_capacity(decoded.width as usize * decoded.height as usize * 4);
    for (i, rgb) in decoded.pixels.as_chunks::<3>().0.iter().enumerate() {
        rgba.extend_from_slice(rgb);
        rgba.push(decoded.alpha.as_ref().map_or(255, |a| a[i]));
    }
    let image = world.resource_mut::<Assets<Image>>().add(Image::new(
        Extent3d {
            width: decoded.width,
            height: decoded.height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba,
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::default(),
    ));
    let view = world
        .resource::<crate::source_view::SourceView>()
        .buffer
        .clone();
    let material = world
        .resource_mut::<Assets<Material>>()
        .add(Material::from(Bindings {
            image: image.clone(),
            view,
            pixel: Vec4::new(1. / 1920., -1. / 1080., 0., 0.),
        }));
    let mesh = world.resource_mut::<Assets<Mesh>>().add(Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    ));
    let entity = world
        .spawn((
            crate::mission_runtime::MissionOwned(ticket),
            Name::new("Source blood particle batch"),
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            bevy::camera::visibility::NoFrustumCulling,
        ))
        .id();
    world.resource_mut::<State>().mounted = Some(Mounted {
        ticket,
        pool,
        entity,
        mesh,
        material,
        image,
        bursts: 0,
        frames: 0,
        probe: false,
        npc_reported: false,
    });
    println!(
        "mission_blood_bound ticket={ticket} capacity=500 count_min={} count_max={} texture_source=true native_pixels_verified=false",
        config.count[0], config.count[1]
    );
    Ok(())
}
/// Concrete birth service. The caller supplies the shared actor RNG; shader,
/// emission and lifetime ownership are all scoped to the selected mission.
pub(crate) fn emit(
    world: &mut World,
    ticket: u64,
    actor: Actor,
    random: &mut Random,
) -> Result<usize, String> {
    if world
        .resource::<crate::mission_environment::State>()
        .mounted_ticket()
        != Some(ticket)
    {
        return Err("blood service mission environment is not mounted".into());
    }
    let old = world.resource::<State>().mounted.as_ref().map(|m| m.ticket);
    if old.is_some_and(|old| old != ticket) {
        release(world);
    }
    if world.resource::<State>().mounted.is_none() {
        mount(world, ticket)?;
    }
    let mut state = world.resource_mut::<State>();
    let m = state.mounted.as_mut().ok_or("blood pool absent")?;
    let count = m.pool.emit(actor, random)?;
    m.bursts += 1;
    println!(
        "mission_blood_emit ticket={ticket} particles={count} active={} callback_acknowledged=false",
        m.pool.active().count()
    );
    Ok(count)
}
fn update(world: &mut World) {
    let live = world
        .get_resource::<crate::mission_runtime::Runtime>()
        .and_then(|r| r.active_ticket());
    if world
        .resource::<State>()
        .mounted
        .as_ref()
        .is_some_and(|m| Some(m.ticket) != live)
    {
        release(world);
    }
    let Some(ticket) = live else {
        return;
    };
    if world.resource::<State>().probe && !world.resource::<State>().probe_fired {
        let settled = world
            .query::<&crate::mission_player::Controller>()
            .iter(world)
            .any(|c| c.prepared(ticket) && c.diagnostic_motion_complete());
        let camera = world
            .get_resource::<crate::world_scene::Scene>()
            .and_then(|s| s.validation_camera.clone());
        if settled && let Some(camera) = camera {
            let point = Mat4::from_cols_array(&camera.view)
                .inverse()
                .transform_point3(Vec3::new(0., 3., 30.));
            let result =
                world.resource_scope(|world, mut runtime: Mut<crate::mission_runtime::Runtime>| {
                    runtime.with_actor_random(ticket, |random| {
                        emit(
                            world,
                            ticket,
                            Actor {
                                position: point.to_array(),
                                scale: 1.,
                                center: 0.,
                                height: 8.,
                            },
                            random,
                        )
                    })
                });
            if let Err(error) = result {
                eprintln!("mission_blood_probe_failed reason={error:?}");
                world.write_message(AppExit::error());
                return;
            }
            let mut state = world.resource_mut::<State>();
            state.probe_fired = true;
            state.mounted.as_mut().expect("emitted pool").probe = true;
        }
    }
    let npc_quads = world
        .resource::<crate::mission_runtime::Runtime>()
        .npc_blood_quads(ticket);
    let npc_quad_count = npc_quads.len();
    if !npc_quads.is_empty()
        && world.resource::<State>().mounted.is_none()
        && let Err(error) = mount(world, ticket)
    {
        eprintln!("mission_blood_mount_failed reason={error}");
        world.write_message(AppExit::error());
        return;
    }
    if world.resource::<State>().mounted.is_none() {
        return;
    }
    let scene = world.resource::<crate::world_scene::Scene>();
    let asset = scene.asset.clone();
    let viewport = scene
        .validation_camera
        .as_ref()
        .map_or(UVec4::new(0, 0, 1920, 1080), |c| c.viewport);
    let dt = world.resource::<Time>().delta_secs();
    let result: Result<_, String> =
        world.resource_scope(|world, mut runtime: Mut<crate::mission_runtime::Runtime>| {
            let mut state = world.resource_mut::<State>();
            let pose_frames = state.pose_frames;
            let m = state.mounted.as_mut().ok_or("blood pool absent")?;
            if m.bursts > 0 && (!m.probe || m.frames < pose_frames) {
                let query = giants_game::terrain_queries::StaticTerrainQuery::new(
                    &asset.terrain,
                    (1. / f64::from(asset.terrain.spacing)) as f32,
                    giants_game::arithmetic::Precision::Double53,
                )
                .map_err(str::to_owned)?;
                let step = if m.probe { 1. / 60. } else { dt };
                runtime.with_actor_random(ticket, |random| {
                    m.pool.update(
                        random,
                        |_| Ok(step),
                        |p| {
                            query
                                .sample([p[0], p[1]])
                                .map(|s| s.height)
                                .map_err(str::to_owned)
                        },
                    )
                })?;
                m.pool.purge();
                m.frames += 1;
            }
            let mut quads = m.pool.quads().collect::<Vec<_>>();
            quads.extend(npc_quads);
            if m.probe && m.frames >= pose_frames && quads.is_empty() {
                return Err("blood diagnostic burst expired before its pose".into());
            }
            if quads.iter().any(|q| !q.textured) {
                return Err("blood quad lost its source texture binding".into());
            }
            let ready = m.probe && m.frames >= pose_frames && !quads.is_empty();
            Ok((m.mesh.clone(), m.material.clone(), quads, ready))
        });
    let (mesh, material, quads, ready) = match result {
        Ok(value) => value,
        Err(error) => {
            eprintln!("mission_blood_update_failed reason={error:?}");
            world.write_message(AppExit::error());
            return;
        }
    };
    let mut positions = vec![];
    let mut uv = vec![];
    let mut colors = vec![];
    let mut offsets = vec![];
    let mut indices = vec![];
    for q in &quads {
        let base = positions.len() as u32;
        let [x, y, z] = q.position;
        for (offset, tex) in [
            ([q.half_width, q.half_height], [1., 1.]),
            ([-q.half_width, q.half_height], [0., 1.]),
            ([-q.half_width, -q.half_height], [0., 0.]),
            ([q.half_width, -q.half_height], [1., 0.]),
        ] {
            positions.push([x, z, -y]);
            offsets.push(offset);
            uv.push(tex);
            colors.push([1., 1., 1., f32::from(q.alpha) / 255.]);
        }
        indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    {
        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        let Some(mut mesh) = meshes.get_mut(&mesh) else {
            return;
        };
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
        mesh.insert_attribute(OFFSET, offsets);
        mesh.insert_indices(bevy::mesh::Indices::U32(indices));
    }
    let pixel = Vec4::new(1. / viewport.z as f32, -1. / viewport.w as f32, 0., 0.);
    if world
        .resource::<Assets<Material>>()
        .get(&material)
        .is_some_and(|m| m.pixel != pixel)
    {
        world
            .resource_mut::<Assets<Material>>()
            .get_mut(&material)
            .expect("owned material")
            .pixel = pixel;
    }
    let mut state = world.resource_mut::<State>();
    if npc_quad_count > 0
        && let Some(m) = state.mounted.as_mut()
        && !m.npc_reported
    {
        println!(
            "mission_npc_blood_uploaded ticket={ticket} quads={npc_quad_count} source_texture=true synthetic_burst=false native_pixels_verified=false"
        );
        m.npc_reported = true;
    }
    if ready && !state.probe_ready {
        println!(
            "mission_blood_probe_ready ticket={ticket} quads={} source_parameters=true source_texture=true synthetic_burst=true native_pixels_verified=false",
            quads.len()
        );
        state.probe_ready = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn release_removes_only_blood_ownership_and_preserves_shared_assets() {
        let mut world = World::new();
        world.init_resource::<Assets<Mesh>>();
        world.init_resource::<Assets<Image>>();
        world.init_resource::<Assets<Material>>();
        world.init_resource::<State>();
        let shared = world.resource_mut::<Assets<Image>>().add(Image::default());
        let image = world.resource_mut::<Assets<Image>>().add(Image::default());
        let mesh = world.resource_mut::<Assets<Mesh>>().add(Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        ));
        let material = world
            .resource_mut::<Assets<Material>>()
            .add(Material::from(Bindings {
                image: image.clone(),
                view: Default::default(),
                pixel: Vec4::ZERO,
            }));
        let entity = world.spawn_empty().id();
        let unrelated = world.spawn_empty().id();
        let pool = Pool::new(Configuration {
            initial_delay: 0.2,
            fade_seconds: 1.6,
            gravity: 55.,
            friction: 1.,
            size: [0.15, 0.35],
            vertical_speed: [25., 45.],
            count: [20, 50],
            spread: 35.,
        })
        .unwrap();
        world.resource_mut::<State>().mounted = Some(Mounted {
            ticket: 7,
            pool,
            entity,
            mesh: mesh.clone(),
            material: material.clone(),
            image: image.clone(),
            bursts: 0,
            frames: 0,
            probe: false,
            npc_reported: false,
        });
        release(&mut world);
        assert!(world.resource::<State>().mounted.is_none());
        assert!(world.get_entity(entity).is_err());
        assert!(world.get_entity(unrelated).is_ok());
        assert!(!world.resource::<Assets<Mesh>>().contains(mesh.id()));
        assert!(!world.resource::<Assets<Material>>().contains(material.id()));
        assert!(!world.resource::<Assets<Image>>().contains(image.id()));
        assert!(world.resource::<Assets<Image>>().contains(shared.id()));
        release(&mut world);
        assert!(world.get_entity(unrelated).is_ok());
    }
}
