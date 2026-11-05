//! Demand-created unlit particle slots, independent of interval or scene names.
use super::*;

pub(super) fn instances(scene: &mut Prepared) -> Result<Vec<(usize, usize)>, String> {
    let mut slots = BTreeMap::<(usize, usize), Vec<usize>>::new();
    for (index, instance) in scene.instances.iter().enumerate() {
        if let Some(emitted) = &instance.emitted
            && (scene.emitters[emitted.group].position.is_some()
                || (scene.emitters[emitted.group].parent.is_some()
                    && scene.emitters[emitted.group].reference.is_none()))
        {
            let indices = slots.entry((emitted.group, emitted.particle)).or_default();
            if emitted.slot != indices.len() {
                return Err("growing render slot order differs".into());
            }
            indices.push(index);
        }
    }
    let mut grown = Vec::new();
    for ((group, particle), indices) in slots {
        let Some(runtime) = &scene.emitters[group].runtime else {
            continue;
        };
        let count = *runtime
            .slot_counts()
            .get(particle)
            .ok_or("growing particle kind absent")?;
        let prototype = *indices.first().ok_or("growing particle prototype absent")?;
        for slot in indices.len()..count {
            let p = &scene.instances[prototype];
            if p.parts
                .iter()
                .any(|(_, part)| !matches!(part, Part::Unlit(_)))
            {
                return Err("growing particles require admitted unlit parts".into());
            }
            let instance = Instance {
                source: p.source.clone(),
                geometry: p.geometry.clone(),
                parts: p.parts.clone(),
                normals: None,
                facing: p.facing,
                animation: Some(
                    p.animation
                        .as_ref()
                        .ok_or("growing animation absent")?
                        .fresh_instance()?,
                ),
                timed: None,
                released: None,
                projectile: None,
                burst: None,
                emitted: Some(Emitted {
                    group,
                    particle,
                    slot,
                    presentation: None,
                    parent: None,
                }),
                retirement_child: None,
                mixed: None,
                detached: false,
                creation_position: None,
            };
            let index = scene.instances.len();
            scene.instances.push(instance);
            grown.push((index, prototype));
        }
        if count > indices.len() {
            println!(
                "cinematic_particle_pool_grew emitter={group} particle={particle} render_slots_before={} render_slots_after={count} native_capacity_verified=false",
                indices.len()
            );
        }
    }
    Ok(grown)
}

pub(super) fn draws(
    scene: &mut Prepared,
    grown: &[(usize, usize)],
    cinematic: &crate::cinematic_camera::Cinematic,
    world: &crate::world_scene::Scene,
    commands: &mut Commands,
    assets: &mut Assets<'_>,
) -> Result<(), String> {
    let camera = world
        .validation_camera
        .as_ref()
        .ok_or("growing camera absent")?;
    for &(index, prototype) in grown {
        let instance = &scene.instances[index];
        let (source, front) =
            instance.render_matrix(cinematic.animation(), cinematic.tick(), camera.view)?;
        let matrix = world_matrix(source);
        let buffer = assets
            .buffers
            .add(crate::source_view::uniform_buffer(&matrix));
        let parts = scene
            .draws
            .iter()
            .filter(|draw| draw.instance == prototype)
            .map(|draw| {
                let DrawMaterial::Unlit(material) = &draw.material else {
                    return Err("growing draw material unsupported");
                };
                Ok((draw.part, draw.mesh.clone(), material.clone()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        if parts.is_empty() {
            return Err("growing prototype draws absent".into());
        }
        for (part, mesh, material) in parts {
            let mesh = assets
                .meshes
                .get(&mesh)
                .ok_or("growing prototype mesh absent")?
                .clone();
            let mesh = assets.meshes.add(mesh);
            let mut material = assets
                .materials
                .get(&material)
                .ok_or("growing prototype material absent")?
                .0
                .clone();
            material.world = buffer.clone();
            material.front = front;
            let material = assets.materials.add(Material::from(material));
            let entity = commands
                .spawn((
                    Rendered,
                    Name::new(format!(
                        "FLK growing particle {} / {part}",
                        instance.source.class()
                    )),
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(material.clone()),
                    Transform::from_matrix(matrix),
                    if instance.visible(cinematic) {
                        Visibility::Inherited
                    } else {
                        Visibility::Hidden
                    },
                    bevy::camera::visibility::NoFrustumCulling,
                ))
                .id();
            scene.draws.push(Draw {
                route: 0,
                texture_frames: None,
                instance: index,
                entity,
                part,
                material: DrawMaterial::Unlit(material),
                mesh,
            });
        }
        scene.motion.push(Motion {
            instance: index,
            buffer: Some(buffer),
            matrix,
        });
    }
    Ok(())
}
