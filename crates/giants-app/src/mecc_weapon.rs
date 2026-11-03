//! Held pistol uses the common normal preparation and fixed-function lit pass.
use bevy::{asset::RenderAssetUsages, prelude::*};
#[derive(Component)]
struct HeldDraw;
#[derive(Resource, Default)]
pub(crate) struct Draws(
    Vec<(
        usize,
        Handle<crate::actor_scene::LitMaterial>,
        Handle<bevy::render::storage::ShaderBuffer>,
    )>,
);
pub fn install(app: &mut App) {
    app.init_resource::<Draws>()
        .add_plugins(MaterialPlugin::<crate::actor_scene::LitMaterial>::default())
        .add_systems(
            Update,
            update
                .after(crate::mecc_dev_scene::update)
                .before(crate::source_view::ViewUpdate)
                .run_if(crate::mecc_session::playing),
        );
}
#[allow(clippy::too_many_arguments)]
pub(crate) fn setup(
    mut commands: Commands,
    state: Res<crate::mecc_dev_scene::State>,
    scene: Res<crate::world_scene::Scene>,
    mut draws: ResMut<Draws>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<crate::actor_scene::LitMaterial>>,
    mut buffers: ResMut<Assets<bevy::render::storage::ShaderBuffer>>,
) {
    let weapon = &state.weapon;
    let normals = giants_game::body_pipeline::prepare_normals(
        &weapon.model.mesh,
        &weapon.model.mesh.positions,
        false,
    )
    .expect("checked pistol normals");
    let mut textures = std::collections::BTreeMap::new();
    for t in &weapon.model.textures {
        let mut image = Image::new(
            bevy::render::render_resource::Extent3d {
                width: t.width,
                height: t.height,
                depth_or_array_layers: 1,
            },
            bevy::render::render_resource::TextureDimension::D2,
            t.rgba.clone(),
            bevy::render::render_resource::TextureFormat::Rgba8Unorm,
            RenderAssetUsages::default(),
        );
        image.sampler = crate::logo_material::sampler();
        textures.insert(t.name.to_ascii_lowercase(), images.add(image));
    }
    let lights =
        giants_game::lighting::first_world(&scene.asset.config).expect("checked world lights");
    let world = Mat4::from_cols_array(&state.weapon_matrix().expect("checked weapon attachment"))
        .transpose();
    let camera = state.camera().0;
    for (index, part) in weapon.model.mesh.subobjects.iter().enumerate() {
        let ids = part
            .triangles
            .iter()
            .flatten()
            .map(|i| usize::from(*i))
            .collect::<Vec<_>>();
        let mesh = Mesh::new(
            bevy::render::render_resource::PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_POSITION,
            ids.iter()
                .map(|i| weapon.model.mesh.positions[usize::from(weapon.model.mesh.indices[*i])])
                .collect::<Vec<_>>(),
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_NORMAL,
            ids.iter().map(|i| normals[*i]).collect::<Vec<_>>(),
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_UV_0,
            ids.iter()
                .map(|i| weapon.model.mesh.uv[*i])
                .collect::<Vec<_>>(),
        );
        let parameters = crate::gun_scene::source_parameters(
            &part.material,
            world,
            &camera,
            &lights,
            scene.asset.config.fog,
            scene.no_fog,
        );
        let buffer = buffers.add(parameters.buffer());
        let material = materials.add(crate::actor_scene::shared_lit_material(
            textures[&part.material.texture.to_ascii_lowercase()].clone(),
            buffer.clone(),
            false,
        ));
        draws.0.push((index, material.clone(), buffer));
        commands.spawn((
            HeldDraw,
            Name::new(format!("Mecc held pistol / {}", part.name)),
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(material),
            bevy::camera::visibility::NoFrustumCulling,
        ));
    }
    println!(
        "mecc_dev_held_weapon class={} model={} parts={} muzzle_slot=1 pose=animated_hand procedural_arm=false first_person_overlay=false",
        weapon.class,
        weapon.model.name,
        draws.0.len()
    );
}
fn update(
    state: Res<crate::mecc_dev_scene::State>,
    scene: Res<crate::world_scene::Scene>,
    draws: Res<Draws>,
    mut buffers: ResMut<Assets<bevy::render::storage::ShaderBuffer>>,
    mut exit: MessageWriter<AppExit>,
    mut draws_visibility: Query<&mut Visibility, With<HeldDraw>>,
) {
    for mut visibility in &mut draws_visibility {
        *visibility = if state.alive() && !state.first_person_view() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !state.alive() {
        return;
    }
    let matrix = match state.weapon_matrix() {
        Ok(matrix) => matrix,
        Err(error) => {
            error!("mecc_dev_weapon_failed={error}");
            exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
            return;
        }
    };
    let lights = giants_game::lighting::first_world(&scene.asset.config).expect("checked lights");
    let camera = state.camera().0;
    for (index, _material, buffer) in &draws.0 {
        if let Some(mut buffer) = buffers.get_mut(buffer) {
            crate::gun_scene::source_parameters(
                &state.weapon.model.mesh.subobjects[*index].material,
                Mat4::from_cols_array(&matrix).transpose(),
                &camera,
                &lights,
                scene.asset.config.fog,
                scene.no_fog,
            )
            .write_buffer(&mut buffer);
        }
    }
}
