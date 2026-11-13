//! Completed source-local rigid submissions in the common ticket-owned renderer.
//! Prototype PS2 `object_render__FP6OBJECTiG7f32quat` remains an unknown route.
//! This consumes actual prepared normal/material resources and current matrices;
//! it creates no actor, collision, clock, damage decision or readiness signal.
use super::*;
use bevy::render::storage::ShaderBuffer;

pub(super) enum Material {
    Lit {
        material: Handle<crate::scenery_scene::Material>,
        parameters: Handle<ShaderBuffer>,
    },
    Cube {
        material: Handle<crate::sword_scene::Material>,
        parameters: Handle<ShaderBuffer>,
    },
}

fn profile(
    draw: &crate::mission_runtime::NpcDraw,
) -> Result<&crate::mission_runtime::NpcRigidProfile, String> {
    let profile = draw.rigid.as_ref().ok_or("rigid draw profile absent")?;
    if !std::sync::Arc::ptr_eq(&profile.asset.model, &draw.geometry)
        || profile.asset.normals.is_none()
        || !profile.matrix.into_iter().all(f32::is_finite)
        || !(0. ..=1.).contains(&draw.transparency)
        || draw.texture.is_some()
    {
        return Err(
            "rigid draw requires matching source normal resources, finite opacity and current pose"
                .into(),
        );
    }
    Ok(profile)
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Appearance {
    pub alpha: f32,
    pub blend: u8,
}
/// Reuse the actor renderer's acquired object/material override and descriptor
/// kernels. Reflection remains on its independently checked cube programs.
pub(crate) fn appearance(
    asset: &giants_game::static_geometry::Asset,
    index: usize,
    transparency: f32,
    fog: bool,
) -> Result<Appearance, String> {
    if !(0. ..=1.).contains(&transparency) {
        return Err("rigid object transparency outside unit profile".into());
    }
    let source = &asset.model.mesh.subobjects[index].material;
    let texture_alpha = asset
        .model
        .alpha_textures
        .contains(&source.texture.to_ascii_lowercase());
    if transparency == 0. {
        return if let Some(descriptor) = asset.reflection_descriptor(index) {
            let (alpha, blend) = descriptor.cube_blend_profile()?;
            Ok(Appearance { alpha, blend })
        } else {
            Ok(Appearance {
                alpha: 1.,
                blend: if texture_alpha { 2 } else { 0 },
            })
        };
    }
    let pass = giants_game::render_parts::part_pass(
        giants_game::render_parts::Object {
            opacity: transparency,
            personality: 0,
            secondary_flags: 0,
            node_filter: false,
        },
        source.falloff,
        texture_alpha,
    );
    let (base_flags, constant, constant_guard) = giants_game::material_descriptor::object_base(
        giants_game::material_descriptor::InspectionInput {
            part_flags: source.flags,
            blend: source.blend,
            falloff: source.falloff,
            has_texture: !source.texture.is_empty(),
            texture_alpha,
            fog,
            pass_mask: pass.mask(),
        },
        transparency,
    )?;
    let descriptor =
        giants_game::material_descriptor::lit(giants_game::material_descriptor::LitInput {
            part_flags: source.flags,
            base_flags,
            constant,
            constant_guard,
            mode: 0x5000,
            toggle: false,
            force_auxiliary: false,
        });
    if asset.reflection_descriptor(index).is_some() {
        let (alpha, blend) = descriptor.cube_blend_profile()?;
        Ok(Appearance { alpha, blend })
    } else if !descriptor.auxiliary && descriptor.flags & 0xf00 == 0x300 {
        Ok(Appearance {
            alpha: (descriptor.constant >> 24) as f32 / 255.,
            blend: 2,
        })
    } else {
        Err("rigid fade requires an unbound material composition".into())
    }
}

#[allow(clippy::too_many_arguments)] // Shared GPU stores and independently owned source/view inputs.
pub(super) fn create(
    draw: &crate::mission_runtime::NpcDraw,
    index: usize,
    mesh: &mut Mesh,
    textures: &BTreeMap<String, Handle<Image>>,
    gpu: &mut Gpu,
    world: &crate::world_scene::Scene,
    lights: &[giants_game::lighting::Directional],
    cube: Option<&crate::reflection_cube::Build>,
) -> Result<Material, String> {
    let p = profile(draw)?;
    let normals = p
        .asset
        .vertices(index)
        .normals
        .ok_or("rigid normal buffer absent")?;
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    let part = &draw.geometry.mesh.subobjects[index];
    let texture = textures
        .get(&part.material.texture.to_ascii_lowercase())
        .cloned();
    let fog = world.asset.config.fog;
    let appearance = appearance(&p.asset, index, draw.transparency, !world.no_fog)?;
    if let Some(descriptor) = p.asset.reflection_descriptor(index) {
        descriptor.cube_blend_profile()?;
        let texture = texture.ok_or("rigid cube source texture absent")?;
        let cube = cube
            .and_then(|b| b.handle())
            .ok_or("rigid cube needs the actual world reflection resource")?;
        let parameters = gpu.buffers.add(
            crate::static_scene::rigid_cube_parameters(
                &p.asset, index, p.matrix, world, lights, fog,
            )
            .cube_factor(Some(appearance.alpha))
            .blended(appearance.blend != 0)
            .buffer(),
        );
        let material = gpu.rigid_cube.add(
            crate::sword_scene::Material::new(texture, cube, parameters.clone())
                .with_blend(appearance.blend),
        );
        Ok(Material::Cube {
            material,
            parameters,
        })
    } else {
        let parameters = gpu.buffers.add(
            crate::static_scene::rigid_lit_parameters(
                &p.asset, index, p.matrix, world, lights, fog,
            )
            .object_alpha(appearance.alpha)
            .buffer(),
        );
        let alpha = appearance.blend == 2;
        let material = gpu.rigid_lit.add(crate::scenery_scene::Material::from(
            crate::scenery_scene::Bindings {
                alpha,
                texture,
                parameters: parameters.clone(),
            },
        ));
        Ok(Material::Lit {
            material,
            parameters,
        })
    }
}

pub(super) fn attach(commands: &mut Commands, entity: Entity, material: &Material) {
    match material {
        Material::Lit { material, .. } => {
            commands
                .entity(entity)
                .insert(MeshMaterial3d(material.clone()));
        }
        Material::Cube { material, .. } => {
            commands
                .entity(entity)
                .insert(MeshMaterial3d(material.clone()));
        }
    }
}

pub(super) fn update(
    draw: &crate::mission_runtime::NpcDraw,
    index: usize,
    material: &Material,
    gpu: &mut Gpu,
    world: &crate::world_scene::Scene,
    lights: &[giants_game::lighting::Directional],
) -> Result<(), String> {
    let p = profile(draw)?;
    let fog = world.asset.config.fog;
    let appearance = appearance(&p.asset, index, draw.transparency, !world.no_fog)?;
    match material {
        Material::Lit {
            parameters,
            material,
        } => {
            gpu.rigid_lit
                .get_mut(material)
                .ok_or("rigid lit material owner removed")?
                .alpha = appearance.blend == 2;
            let mut buffer = gpu
                .buffers
                .get_mut(parameters)
                .ok_or("rigid lit parameter owner removed")?;
            crate::static_scene::rigid_lit_parameters(
                &p.asset, index, p.matrix, world, lights, fog,
            )
            .object_alpha(appearance.alpha)
            .write_buffer(&mut buffer);
        }
        Material::Cube {
            parameters,
            material,
        } => {
            gpu.rigid_cube
                .get_mut(material)
                .ok_or("rigid cube material owner removed")?
                .blend = appearance.blend;
            let mut buffer = gpu
                .buffers
                .get_mut(parameters)
                .ok_or("rigid cube parameter owner removed")?;
            crate::static_scene::rigid_cube_parameters(
                &p.asset, index, p.matrix, world, lights, fog,
            )
            .cube_factor(Some(appearance.alpha))
            .blended(appearance.blend != 0)
            .write_buffer(&mut buffer);
        }
    }
    Ok(())
}

pub(super) fn remove(gpu: &mut Gpu, material: Material) {
    match material {
        Material::Lit {
            material,
            parameters,
        } => {
            gpu.rigid_lit.remove(material.id());
            gpu.buffers.remove(parameters.id());
        }
        Material::Cube {
            material,
            parameters,
        } => {
            gpu.rigid_cube.remove(material.id());
            gpu.buffers.remove(parameters.id());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires GIANTS_BUILDING_ROOT; CPU asset ownership, not actor birth or GPU pixels"]
    fn installed_rigid_material_owners_release_buffers_and_refuse_missing_cube() {
        let root = std::path::PathBuf::from(std::env::var("GIANTS_BUILDING_ROOT").unwrap());
        let resources =
            giants_game::scene_resources::SceneResources::load(&root, "w_Story2.bin", "English")
                .unwrap();
        let matrix = giants_game::procedural_aim::Matrix::identity().values;
        let prepared = giants_game::static_geometry::Scene::constructed_rigid_draw(
            &resources,
            (false, 7),
            1382,
            matrix,
            1.,
        )
        .unwrap();
        let asset = std::sync::Arc::new(
            prepared
                .assets
                .into_iter()
                .find(|((_, slot), _)| *slot == 0)
                .unwrap()
                .1,
        );
        let cube_part = asset
            .reflection_descriptors
            .iter()
            .position(Option::is_some)
            .unwrap();
        let mut draw = crate::mission_runtime::NpcDraw {
            rigid: Some(crate::mission_runtime::NpcRigidProfile {
                asset: asset.clone(),
                matrix,
            }),
            texture: None,
            class: 1382,
            transparency: 0.,
            id: crate::mission_runtime::NpcDrawOwner::BuildingEffect(1),
            frame: 0,
            geometry: asset.model.clone(),
            positions: asset.model.mesh.positions.clone(),
            center: [0.; 3],
        };
        let menu = giants_game::mission_selection::MissionMenu::parse(
            &std::fs::read(root.join("Bin/w_Story2.bin")).unwrap(),
            0,
            true,
        )
        .unwrap();
        let packet =
            giants_game::mission_loading::prepare(&root, menu.request("Story2", 0).unwrap())
                .unwrap();
        let world = crate::world_scene::Scene {
            asset: std::sync::Arc::new(packet.world_asset(&root).unwrap()),
            oblique: false,
            validation_camera: Some(crate::validation_camera::ValidationCamera {
                view: matrix,
                projection: matrix,
                viewport: UVec4::new(0, 0, 640, 360),
            }),
            camera_source: "Rigid CPU resource control",
            camera_origin: None,
            cinematic_capture_ready: false,
            no_fog: false,
            render_terrain: false,
            render_sky: false,
            render_underside: false,
            reflection_profile: false,
            water: None,
            render_water: false,
        };
        let lights = giants_game::lighting::first_world(&world.asset.config).unwrap();
        let mut storage = World::new();
        storage.init_resource::<Assets<Mesh>>();
        storage.init_resource::<Assets<Image>>();
        storage.init_resource::<Assets<crate::actor_scene::Material>>();
        storage.init_resource::<Assets<crate::rigid_attachment_scene::Material>>();
        storage.init_resource::<Assets<crate::scenery_scene::Material>>();
        storage.init_resource::<Assets<crate::sword_scene::Material>>();
        storage.init_resource::<Assets<ShaderBuffer>>();
        let mut system = bevy::ecs::system::SystemState::<Gpu>::new(&mut storage);
        let mut gpu = system.get_mut(&mut storage).unwrap();
        let mut textures = BTreeMap::new();
        for source in &draw.geometry.textures {
            let image = Image::new(
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
            textures.insert(source.name.to_ascii_lowercase(), gpu.images.add(image));
        }
        let mut mesh = Mesh::new(
            bevy::render::render_resource::PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        let material = create(
            &draw, 0, &mut mesh, &textures, &mut gpu, &world, &lights, None,
        )
        .unwrap();
        assert!(matches!(material, Material::Lit { .. }));
        assert_eq!((gpu.rigid_lit.len(), gpu.buffers.len()), (1, 1));
        update(&draw, 0, &material, &mut gpu, &world, &lights).unwrap();
        // The old guard's zero-blend auxiliary fallback cannot silently become
        // a fading RGB program. Keep that refusal and the missing-cube case.
        draw.transparency = 0.5;
        assert!(update(&draw, 0, &material, &mut gpu, &world, &lights).is_err());
        draw.transparency = 0.;
        remove(&mut gpu, material);
        assert!(
            create(
                &draw, cube_part, &mut mesh, &textures, &mut gpu, &world, &lights, None
            )
            .is_err()
        );
        assert!(gpu.rigid_lit.is_empty() && gpu.rigid_cube.is_empty() && gpu.buffers.is_empty());

        let prepared = giants_game::static_geometry::Scene::constructed_rigid_draw(
            &resources,
            (false, 0),
            1377,
            matrix,
            1.,
        )
        .unwrap();
        let fading_asset = std::sync::Arc::new(
            prepared
                .assets
                .into_iter()
                .find(|((_, slot), _)| *slot == 0)
                .unwrap()
                .1,
        );
        draw.class = 1377;
        draw.geometry = fading_asset.model.clone();
        draw.positions = draw.geometry.mesh.positions.clone();
        draw.rigid = Some(crate::mission_runtime::NpcRigidProfile {
            asset: fading_asset,
            matrix,
        });
        for source in &draw.geometry.textures {
            let image = Image::new(
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
            textures.insert(source.name.to_ascii_lowercase(), gpu.images.add(image));
        }
        let material = create(
            &draw, 0, &mut mesh, &textures, &mut gpu, &world, &lights, None,
        )
        .unwrap();
        for (transparency, expected_alpha, straight) in
            [(0.5, 127f32 / 255., true), (1., 0., true), (0., 1., false)]
        {
            draw.transparency = transparency;
            update(&draw, 0, &material, &mut gpu, &world, &lights).unwrap();
            let Material::Lit {
                material: handle,
                parameters,
            } = &material
            else {
                panic!("lit owner changed")
            };
            assert_eq!(gpu.rigid_lit.get(handle).unwrap().alpha, straight);
            let bevy::render::storage::ShaderBufferData::Initialized { data, .. } =
                &gpu.buffers.get(parameters).unwrap().data
            else {
                panic!("uniform data absent")
            };
            let bytes: &[u8] = data;
            let alpha = f32::from_le_bytes(
                bytes[bytes.len() - 16..bytes.len() - 12]
                    .try_into()
                    .unwrap(),
            );
            assert_eq!(alpha.to_bits(), expected_alpha.to_bits());
            let cube = appearance(&asset, cube_part, transparency, true).unwrap();
            assert_eq!(cube.blend, if straight { 2 } else { 0 });
            if straight {
                assert_eq!(cube.alpha.to_bits(), expected_alpha.to_bits());
            }
            assert_eq!((gpu.rigid_lit.len(), gpu.buffers.len()), (1, 1));
        }
        draw.transparency = f32::NAN;
        assert!(profile(&draw).is_err());
        draw.transparency = 0.;
        remove(&mut gpu, material);
        assert!(gpu.rigid_lit.is_empty() && gpu.buffers.is_empty());
        assert!(gpu.rigid_cube.is_empty() && gpu.buffers.is_empty());
    }
}
