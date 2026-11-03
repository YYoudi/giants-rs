//! Explicit static owned-FX inspection in a loaded environment. No AI/mission ack.
use bevy::{asset::RenderAssetUsages, prelude::*, render::storage::ShaderBuffer};
use giants_game::{
    actor_fields::Fields,
    arithmetic::Precision,
    attached_effect_context::{Creation, Pool},
    attached_effect_draw::Geometry,
    attached_effect_object::Factory,
    resident_states::{Construction, Id, Session},
    scene_resources::SceneResources,
    state_storage::{Profile, Seed},
};
use std::{collections::BTreeMap, path::Path, sync::Arc};
struct Object {
    id: Option<Id>,
    geometry: DrawGeometry,
    buffer: Option<Handle<ShaderBuffer>>,
    draws: Vec<(usize, Handle<Mesh>)>,
}
enum DrawGeometry {
    Attached(Box<Geometry>),
    Burst(Box<giants_game::burst_effect_draw::Geometry>),
}
impl DrawGeometry {
    fn class(&self) -> i32 {
        match self {
            Self::Attached(g) => g.class,
            Self::Burst(g) => g.class,
        }
    }
    fn model(&self) -> &giants_game::scene_resources::ModelAsset {
        match self {
            Self::Attached(g) => &g.model,
            Self::Burst(g) => &g.model,
        }
    }
    fn profiles(&self) -> &[giants_game::unlit_material::Profile] {
        match self {
            Self::Attached(g) => &g.profiles,
            Self::Burst(g) => &g.profiles,
        }
    }
    fn frame(
        &self,
        session: &mut Session,
        id: Option<Id>,
        epoch: u32,
        view: [f32; 16],
    ) -> Result<giants_game::attached_effect_draw::Frame, String> {
        match self {
            Self::Attached(g) => g.frame(
                session,
                id.ok_or("attached draw identity absent")?,
                epoch,
                true,
                view,
            ),
            Self::Burst(g) => g.frame(view),
        }
    }
}
#[derive(Resource)]
pub(crate) struct State {
    session: Session,
    objects: Vec<Object>,
    spawned: bool,
    epoch: u32,
    clip: Option<Handle<ShaderBuffer>>,
    focus: Vec3,
}
impl State {
    /// Rows: parent-class effect-name tag-hex x y z yaw pitch roll scale model-slot.
    /// Every pose is an explicit inspection input, never a mission placement rule.
    pub(crate) fn load(
        root: &Path,
        world: &str,
        path: &Path,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        if std::fs::metadata(path)?.len() > 65536 {
            return Err("owned FX request exceeds inspection budget".into());
        }
        let text = std::fs::read_to_string(path)?;
        let resources = SceneResources::load(root, world, "English")?;
        let mut session = Session::new(160);
        let mut objects = vec![];
        let mut contexts = Pool::from_active_resources(160, &resources);
        let mut factory = Factory::new(&resources);
        let mut requests = 0;
        let mut focus = [0_f64; 3];
        for line in text
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty() && !s.starts_with('#'))
        {
            requests += 1;
            if requests > 64 {
                return Err("owned FX request count exceeds inspection budget".into());
            }
            let p = line.split_whitespace().collect::<Vec<_>>();
            if p.first() == Some(&"burst") {
                // burst name seed x y z slot fixed-steps; explicit inspection inputs.
                if p.len() != 8 {
                    return Err("burst row requires name/seed/position/slot/steps".into());
                }
                let mut random = giants_game::audio_parameters::Random(p[2].parse()?);
                let position = [p[3].parse::<f32>()?, p[4].parse()?, p[5].parse()?];
                let slot = p[6].parse::<usize>()?;
                let frames = p[7].parse::<u32>()?;
                let image = resources
                    .classes
                    .models
                    .effects
                    .last()
                    .ok_or("burst image absent")?
                    .image
                    .clone();
                let (creation, _) = contexts.prepare_positional(
                    p[1],
                    position,
                    giants_game::effect_initialization::Selection {
                        active_objects: 1,
                        quality: 1.,
                        lite_distance: 500.,
                        camera: [0.; 3],
                        recipient_position: position,
                    },
                    &mut random,
                )?;
                let Creation::Prepared(context) = creation else {
                    return Err("burst symbol absent".into());
                };
                let birth = contexts.consume_positional_burst(context, &resources, &mut random)?;
                let total = birth.spawns.len();
                let mut drawable = 0;
                for (spawn, matrix) in birth.spawns.into_iter().zip(birth.matrices) {
                    if spawn.hidden {
                        continue;
                    }
                    let geometry = giants_game::burst_effect_draw::Geometry::load(
                        &resources, &image, spawn, matrix, slot, frames,
                    )?;
                    if !geometry.visible() {
                        continue;
                    }
                    drawable += 1;
                    objects.push(Object {
                        id: None,
                        geometry: DrawGeometry::Burst(Box::new(geometry)),
                        buffer: None,
                        draws: vec![],
                    });
                }
                for (sum, value) in focus.iter_mut().zip(position) {
                    *sum += f64::from(value);
                }
                println!(
                    "owned_burst_prepared request={requests} births={total} drawable={drawable} supplied_steps={frames} seed_after={} contact_free_preview=true mission_actors_created=0 ready=false",
                    random.0
                );
                continue;
            }
            if p.len() != 11 {
                return Err("owned FX row requires class/name/tag/pose7/slot".into());
            }
            let class = p[0].parse::<i32>()?;
            let tag = u32::from_str_radix(p[2].trim_start_matches("0x"), 16)?;
            let pose = p[3..10]
                .iter()
                .map(|s| s.parse::<f32>())
                .collect::<Result<Vec<_>, _>>()?;
            let slot = p[10].parse::<usize>()?;
            if !pose.iter().all(|v| v.is_finite()) {
                return Err("owned FX parent pose nonfinite".into());
            }
            for (sum, value) in focus.iter_mut().zip(&pose[..3]) {
                *sum += f64::from(*value);
            }
            let program = resources
                .classes
                .models
                .first_program(class)
                .ok_or("owned FX parent class absent")?
                .program
                .as_ref()
                .map_err(Clone::clone)?;
            let seed = Seed::from_program_for(program, Profile::Debug, Precision::Double53)?;
            let spec = Arc::new(giants_game::specification::Prepared::load_registered(
                &resources, class,
            )?);
            let states = Arc::new(giants_game::state_resources::Prepared::load(
                &resources,
                class,
                Profile::Debug,
            )?);
            let bindings = resources.class_state_bindings_for(class, Profile::Debug)?;
            let flags = giants_game::object_lists::DirectFlags::project(class, program);
            let mut fields = Fields::load(&resources, class, &spec)?.placed(
                giants_game::collider_init::DeclaredPose {
                    class: class as u32,
                    position: [pose[0], pose[1], pose[2]],
                    yaw: pose[3],
                    pitch_roll: Some([pose[4], pose[5]]),
                },
            )?;
            fields.set_declared_scale(pose[6])?;
            let input = Construction {
                class,
                seed: &seed,
                bindings: &bindings,
                flags,
                specification: spec,
                state_resources: states,
                fields,
            };
            let parent = if seed.storage.state() == 6 {
                session.construct_enemy_projectile(input)?
            } else {
                session.construct(input)?
            };
            let parent_flags = session
                .get(parent)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap()
                .object_flags;
            let Creation::Prepared(context) = contexts.prepare(p[1], parent, parent_flags, tag)?
            else {
                return Err("requested owned FX symbol absent".into());
            };
            let Some(id) = contexts.activate(context, &mut session, &mut factory)? else {
                println!(
                    "owned_effect_terminal request={requests} object_created=false expected_empty=true"
                );
                continue;
            };
            let child_reference = session.attached_effect(id).unwrap().reference;
            session.prepare_actor_affine(parent)?;
            if (tag as u8 as i8) >= 0
                && !session
                    .specification(parent)
                    .unwrap()
                    .metadata
                    .references
                    .names
                    .is_empty()
            {
                session.update_current_references(parent)?;
            }
            if (child_reference as u8 as i8) >= 0 {
                session.update_current_references(id)?;
            }
            session.update_attached_effect_world(id)?;
            let geometry = Geometry::load(&resources, &session, id, slot)?;
            println!(
                "owned_effect_prepared request={requests} class={} model_slot={slot} parts={} source_clock=0 parent_fixture=true ai_ready=false emitted_particles=0",
                geometry.class,
                geometry.profiles.len()
            );
            objects.push(Object {
                id: Some(id),
                geometry: DrawGeometry::Attached(Box::new(geometry)),
                buffer: None,
                draws: vec![],
            });
        }
        if requests == 0 {
            return Err("owned FX request list is empty".into());
        }
        Ok(Self {
            session,
            objects,
            spawned: false,
            epoch: 1,
            clip: None,
            focus: Vec3::from_array(focus.map(|v| (v / requests as f64) as f32)),
        })
    }
}
pub(crate) fn install(app: &mut App, state: State) {
    if !app.is_plugin_added::<MaterialPlugin<crate::rigid_attachment_scene::Material>>() {
        app.add_plugins(MaterialPlugin::<crate::rigid_attachment_scene::Material>::default());
    }
    app.insert_resource(state)
        .add_systems(Startup, focus_camera.after(crate::world_scene::setup))
        .add_systems(
            PostUpdate,
            draw.after(bevy::transform::TransformSystems::Propagate)
                .after(bevy::camera::CameraUpdateSystems),
        );
}
fn focus_camera(
    state: Res<State>,
    scene: Res<crate::world_scene::Scene>,
    mut cameras: Query<(&mut Transform, &mut Projection), With<crate::mesh_scene::FlyCamera>>,
) {
    if scene.validation_camera.is_some() {
        return;
    }
    let Ok((mut camera, mut projection)) = cameras.single_mut() else {
        return;
    };
    let source_to_bevy = |p: Vec3| Vec3::new(p.x, p.z, -p.y);
    *camera = Transform::from_translation(source_to_bevy(state.focus + Vec3::new(0., -28., 8.)))
        .looking_at(source_to_bevy(state.focus), Vec3::Y);
    if let Projection::Perspective(p) = &mut *projection {
        p.fov = 50f32.to_radians();
    }
}
fn free_camera_matrices(camera: &GlobalTransform, projection: &Projection) -> ([f32; 16], Mat4) {
    let basis = Mat4::from_cols(Vec4::X, -Vec4::Z, Vec4::Y, Vec4::W);
    let inverse = camera.to_matrix().inverse();
    let native_view = (Mat4::from_scale(Vec3::new(1., 1., -1.)) * inverse * basis).to_cols_array();
    let depth = Mat4::from_cols(Vec4::X, Vec4::Y, -Vec4::Z, Vec4::new(0., 0., 1., 1.));
    (
        native_view,
        depth * projection.get_clip_from_view() * inverse,
    )
}
#[allow(clippy::too_many_arguments)] // Bevy resource access is explicit in this system.
fn draw(
    mut commands: Commands,
    mut state: ResMut<State>,
    scene: Res<crate::world_scene::Scene>,
    cameras: Query<(&GlobalTransform, &Projection, &Camera), With<Camera3d>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<crate::rigid_attachment_scene::Material>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
    mut exit: MessageWriter<AppExit>,
) {
    let basis = Mat4::from_cols(Vec4::X, -Vec4::Z, Vec4::Y, Vec4::W);
    let (native_view, clip, viewport) = if let Some(c) = &scene.validation_camera {
        (
            c.view,
            c.clip_from_bevy(),
            UVec2::new(c.viewport.z, c.viewport.w),
        )
    } else {
        let Ok((camera, projection, target)) = cameras.single() else {
            return;
        };
        let (native_view, clip) = free_camera_matrices(camera, projection);
        // The shared shader converts source forward-Z depth to Bevy reverse-Z.
        // Supply the inverse conversion when starting from Bevy's projection.
        (
            native_view,
            clip,
            target
                .physical_viewport_size()
                .unwrap_or(UVec2::new(1920, 1080)),
        )
    };
    let clip_buffer = if let Some(handle) = &state.clip {
        let Some(mut target) = buffers.get_mut(handle) else {
            error!("Owned FX projection buffer removed before its scene");
            exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
            return;
        };
        crate::source_view::write_uniform(&clip, &mut target);
        handle.clone()
    } else {
        let handle = buffers.add(crate::source_view::uniform_buffer(&clip));
        state.clip = Some(handle.clone());
        handle
    };
    let fog = scene.asset.config.fog;
    let first = !state.spawned;
    let epoch = state.epoch;
    let State {
        session, objects, ..
    } = &mut *state;
    let result = (|| -> Result<(), String> {
        for object in objects.iter_mut() {
            let frame = object
                .geometry
                .frame(session, object.id, epoch, native_view)?;
            let matrix = basis * Mat4::from_cols_array(&frame.matrix).transpose() * basis.inverse();
            if let Some(buffer) = &object.buffer {
                let mut target = buffers
                    .get_mut(buffer)
                    .ok_or("owned FX matrix buffer missing")?;
                crate::source_view::write_uniform(&matrix, &mut target);
                let model = object.geometry.model();
                for (part, handle) in &object.draws {
                    let positions = model.mesh.subobjects[*part]
                        .triangles
                        .iter()
                        .flatten()
                        .map(|i| {
                            let p = frame.positions[model.mesh.indices[usize::from(*i)] as usize];
                            [p[0], p[2], -p[1]]
                        })
                        .collect::<Vec<_>>();
                    meshes
                        .get_mut(handle)
                        .ok_or("owned FX mesh missing")?
                        .insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
                }
                continue;
            }
            let buffer = buffers.add(crate::source_view::uniform_buffer(&matrix));
            object.buffer = Some(buffer.clone());
            let mut textures = BTreeMap::new();
            for source in &object.geometry.model().textures {
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
                textures.insert(source.name.to_ascii_lowercase(), images.add(image));
            }
            let model = object.geometry.model();
            for (index, part) in model.mesh.subobjects.iter().enumerate() {
                let ids = part
                    .triangles
                    .iter()
                    .flatten()
                    .map(|i| usize::from(*i))
                    .collect::<Vec<_>>();
                let positions = ids
                    .iter()
                    .map(|i| {
                        let p = frame.positions[model.mesh.indices[*i] as usize];
                        [p[0], p[2], -p[1]]
                    })
                    .collect::<Vec<_>>();
                let mesh = Mesh::new(
                    bevy::render::render_resource::PrimitiveTopology::TriangleList,
                    RenderAssetUsages::default(),
                )
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
                .with_inserted_attribute(
                    Mesh::ATTRIBUTE_UV_0,
                    ids.iter().map(|i| model.mesh.uv[*i]).collect::<Vec<_>>(),
                )
                .with_inserted_attribute(
                    Mesh::ATTRIBUTE_COLOR,
                    ids.iter()
                        .map(|i| {
                            let c = model.mesh.rgb[*i];
                            [
                                c[0] as f32 / 255.,
                                c[1] as f32 / 255.,
                                c[2] as f32 / 255.,
                                1.,
                            ]
                        })
                        .collect::<Vec<_>>(),
                );
                let texture = textures
                    .get(&part.material.texture.to_ascii_lowercase())
                    .ok_or("owned FX named texture missing")?
                    .clone();
                let material = crate::rigid_attachment_scene::owned_material(
                    texture,
                    clip_buffer.clone(),
                    buffer.clone(),
                    object.geometry.profiles()[index],
                    frame.front,
                    Vec4::new(
                        1. / viewport.x as f32,
                        -1. / viewport.y as f32,
                        fog.start,
                        fog.end,
                    ),
                    Vec3::new(
                        fog.color[0] as f32 / 255.,
                        fog.color[1] as f32 / 255.,
                        fog.color[2] as f32 / 255.,
                    ),
                    !scene.no_fog,
                );
                let mesh = meshes.add(mesh);
                object.draws.push((index, mesh.clone()));
                commands.spawn((
                    Name::new(format!("Owned FX {} part {index}", object.geometry.class())),
                    Mesh3d(mesh),
                    MeshMaterial3d(materials.add(material)),
                    Transform::default(),
                    GlobalTransform::default(),
                    Visibility::Visible,
                    bevy::camera::visibility::NoFrustumCulling,
                ));
                println!(
                    "owned_effect_draw class={} part={index} vertices={} source_clock=0 emitter_simulated=false rendered_submission=true",
                    object.geometry.class(),
                    ids.len()
                );
            }
        }
        Ok(())
    })();
    match result {
        Ok(()) => {
            state.spawned = true;
            if first {
                println!(
                    "owned_effect_draws_submitted objects={} fixture=true visual_fidelity_verified=false",
                    state.objects.len()
                );
            }
        }
        Err(error) => {
            error!("Owned FX render failed: {error}");
            exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn free_camera_source_view_maps_eye_to_origin_and_forward_to_positive_z() {
        let pose = Transform::from_xyz(10., 20., 30.).looking_at(Vec3::new(-5., 17., 12.), Vec3::Y);
        let world = GlobalTransform::from(pose);
        let (native, _) = free_camera_matrices(
            &world,
            &Projection::Perspective(PerspectiveProjection::default()),
        );
        let view = Mat4::from_cols_array(&native);
        let convert = |v: Vec3| Vec3::new(v.x, -v.z, v.y);
        assert!(view.transform_point3(convert(pose.translation)).length() < 0.0001);
        let forward = view.transform_point3(convert(pose.translation + *pose.forward() * 10.));
        assert!(
            forward.x.abs() < 0.0001
                && forward.y.abs() < 0.0001
                && (forward.z - 10.).abs() < 0.0001
        );
        assert!(view.determinant() < 0.);
    }
    #[test]
    fn shared_shader_depth_conversion_preserves_bevy_projection() {
        let camera = GlobalTransform::IDENTITY;
        let projection = Projection::Perspective(PerspectiveProjection::default());
        let (_, source) = free_camera_matrices(&camera, &projection);
        for distance in [0.2, 1., 10., 1000.] {
            let p = Vec4::new(0.03, -0.01, -distance, 1.);
            let native = source * p;
            let shader = Vec4::new(native.x, native.y, native.w - native.z, native.w);
            let expected = projection.get_clip_from_view() * p;
            assert!((shader - expected).abs().max_element() < 0.0001);
        }
    }
}
