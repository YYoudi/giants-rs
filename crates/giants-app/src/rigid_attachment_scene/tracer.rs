//! Explicit viewer schedule for acquired tracer kernels; native cadence remains open.
use super::*;
use giants_game::{
    effect_tracer as history, effect_tracer_owner as owner, effect_tracer_registry as registry,
    effect_tracer_ribbon as ribbon,
};
struct Bound {
    handle: registry::Handle,
    owner: owner::Owner,
    flags: u32,
    offset: usize,
}
struct Batch {
    entity: Entity,
    mesh: Handle<Mesh>,
    material: Handle<Material>,
}
#[derive(Default)]
pub(super) struct State {
    registry: registry::Registry,
    owners: BTreeMap<(usize, u64), Bound>,
    roots: BTreeMap<usize, usize>,
    rejected: BTreeSet<(usize, u64)>,
    batch: Option<Batch>,
    steps: u64,
    generation: Option<u64>,
}
impl State {
    fn reset(&mut self) {
        self.registry = registry::Registry::default();
        self.owners.clear();
        self.roots.clear();
        self.rejected.clear();
        self.steps = 0;
        self.generation = None;
    }
    pub(super) fn entity(&self) -> Option<Entity> {
        self.batch.as_ref().map(|b| b.entity)
    }
    /// A director cut keeps the replay generation; an explicit restart changes it.
    pub(super) fn prepare(&mut self, generation: u64) -> bool {
        if self.generation == Some(generation) {
            return false;
        }
        self.reset();
        self.generation = Some(generation);
        true
    }
    pub(super) fn handoff(&mut self) -> Result<(), String> {
        println!(
            "cinematic_tracer_handoff owners_released={} slots_retained={} populated_histories={} preview_schedule=true native_actor_teardown_verified=false",
            self.owners.len(),
            self.registry.slots().count(),
            self.registry
                .slots()
                .filter(|(_, _, t)| t.count() > 1)
                .count()
        );
        for bound in self.owners.values() {
            self.registry.release(bound.handle)?;
        }
        self.owners.clear();
        self.rejected.clear();
        // Root offsets belong to the old director; its tails still draw normally,
        // but an inspection of a new root must not select them by a reused offset.
        self.roots.clear();
        self.steps = 0;
        Ok(())
    }
}
struct Sample {
    key: (usize, u64),
    configuration: history::Configuration,
    position: [f32; 3],
    offset: usize,
}
fn samples(scene: &Prepared) -> Result<Vec<Sample>, String> {
    let mut samples = Vec::new();
    for (index, instance) in scene.instances.iter().enumerate() {
        let Source::Effect { request, .. } = &instance.source else {
            continue;
        };
        if let Some(p) = &instance.projectile {
            if p.playback.matrix.is_some()
                && !p.playback.expired
                && let Some(configuration) = &p.playback.recipe.tracer
            {
                samples.push(Sample {
                    key: (index, 0),
                    configuration: configuration.clone(),
                    position: p.playback.motion.position,
                    offset: request.offset,
                });
            }
        } else if let Some(e) = &instance.emitted
            && let Some(p) = e.presentation.filter(|p| !p.expired)
            && let Some(configuration) =
                &scene.emitters[e.group].recipe.particles[e.particle].tracer
        {
            // Sampling uses the resolved source binding BEFORE the camera-facing
            // draw matrix changes its translation. Never project that draw matrix twice.
            let position = if let Some(binding) = p.attachment {
                let parent = e.parent.ok_or("tracer attachment parent absent")?;
                let own = instance
                    .animation
                    .as_ref()
                    .ok_or("tracer own animation absent")?
                    .reference(binding.child_reference as usize)?;
                let matrix = giants_game::effect_attachment::initial_child_matrix(
                    parent.matrix,
                    parent.reference,
                    own,
                    p.size,
                    parent.size,
                )?;
                [matrix.values[3], matrix.values[7], matrix.values[11]]
            } else {
                p.position
            };
            samples.push(Sample {
                key: (index, p.serial),
                configuration: configuration.clone(),
                position,
                offset: request.offset,
            });
        }
    }
    Ok(samples)
}
#[allow(clippy::too_many_arguments)]
pub(super) fn update(
    state: &mut State,
    scene: &Prepared,
    world: &crate::world_scene::Scene,
    cinematic: &crate::cinematic_camera::Cinematic,
    commands: &mut Commands,
    assets: &mut Assets<'_>,
    delta: f32,
    reset: bool,
    isolation: Option<usize>,
) -> Result<(), String> {
    if reset && state.generation != Some(cinematic.generation()) {
        state.prepare(cinematic.generation());
    }
    let samples = samples(scene)?;
    let live = samples.iter().map(|s| s.key).collect::<BTreeSet<_>>();
    // Explicit viewer release-before-birth schedule, not a native actor-order claim.
    let removed = state
        .owners
        .keys()
        .copied()
        .filter(|key| !live.contains(key))
        .collect::<Vec<_>>();
    for key in removed {
        let bound = state.owners.remove(&key).unwrap();
        state.registry.release(bound.handle)?;
        println!(
            "cinematic_tracer_released sequence={} offset={} slot={} history_retained=true native_cadence_verified=false",
            cinematic.sequence(),
            bound.offset,
            bound.handle.slot()
        );
    }
    state.rejected.retain(|key| live.contains(key));
    for sample in samples {
        if state.rejected.contains(&sample.key) {
            continue;
        }
        if !state.owners.contains_key(&sample.key) {
            if let Some(handle) = state.registry.admit(sample.configuration)? {
                let mut initial = owner::INITIAL_SAMPLE;
                if owner::consume_initial_sample(&mut initial) {
                    state.registry.sample(handle, sample.position)?;
                }
                state.roots.insert(handle.slot(), sample.offset);
                state.owners.insert(
                    sample.key,
                    Bound {
                        handle,
                        owner: owner::Owner::new(0., 0., 0.)?,
                        flags: owner::ACTIVE,
                        offset: sample.offset,
                    },
                );
                println!(
                    "cinematic_tracer_admitted sequence={} offset={} slot={} initial_position_bits={:?} backend_connected=true native_cadence_verified=false",
                    cinematic.sequence(),
                    sample.offset,
                    handle.slot(),
                    sample.position.map(f32::to_bits)
                );
            } else {
                println!(
                    "cinematic_tracer_capacity_refused sequence={} offset={} slots=30",
                    cinematic.sequence(),
                    sample.offset
                );
                state.rejected.insert(sample.key);
                continue;
            }
        }
        let bound = state.owners.get_mut(&sample.key).unwrap();
        let actions = bound.owner.post_motion(&mut bound.flags, delta)?;
        if let Some(opacity) = actions.opacity {
            state.registry.set_opacity(bound.handle, opacity)?;
        }
        if actions.sample {
            state.registry.sample(bound.handle, sample.position)?;
        }
    }
    let segments = state.registry.advance(delta)?;
    state.steps += 1;
    let Some(camera) = world.validation_camera.as_ref() else {
        return Err("tracer source camera absent".into());
    };
    let composed = giants_game::camera_matrix::compose(
        camera.view,
        Some(camera.projection),
        giants_game::arithmetic::Precision::Double53,
    );
    let mut positions = Vec::new();
    let mut colors = Vec::new();
    for (slot, segment) in &segments {
        // Released tails keep their root attribution after the owner is removed.
        if isolation.is_some()
            && state
                .roots
                .get(slot)
                .is_none_or(|offset| Some(*offset) != isolation)
        {
            continue;
        }
        let (_, _, track) = state
            .registry
            .slots()
            .find(|(i, _, _)| i == slot)
            .ok_or("tracer slot absent")?;
        let points = track.defined_points().collect::<BTreeMap<_, _>>();
        let previous = points
            .get(&segment.previous)
            .ok_or("tracer previous point absent")?;
        let current = points
            .get(&segment.current)
            .ok_or("tracer current point absent")?;
        let a = ribbon::tracer_position(composed, previous.position)?;
        let b = ribbon::tracer_position(composed, current.position)?;
        if let Some(quad) = ribbon::camera_quad(
            a,
            b,
            previous.color,
            previous.fade,
            segment.opacity,
            segment.half_width,
        )? {
            let color = [
                ((quad.rgb >> 16) & 255) as f32 / 255.,
                ((quad.rgb >> 8) & 255) as f32 / 255.,
                (quad.rgb & 255) as f32 / 255.,
                (quad.alpha >> 24) as f32 / 255.,
            ];
            for triangle in ribbon::TRIANGLES {
                for vertex in triangle {
                    positions.push(quad.positions[vertex]);
                    colors.push(color);
                }
            }
        }
    }
    let candidates = positions
        .as_chunks::<3>()
        .0
        .iter()
        .filter(|triangle| {
            !(0..6).any(|plane| {
                triangle.iter().all(|p| match plane {
                    0 => p[0] < -p[2],
                    1 => p[0] > p[2],
                    2 => p[1] < -p[2],
                    3 => p[1] > p[2],
                    4 => p[2] - (camera.projection[10] * p[2] + camera.projection[14]) < 0.,
                    _ => p[2] - (camera.projection[10] * p[2] + camera.projection[14]) > p[2],
                })
            })
        })
        .count();
    let vertices = positions.len();
    if vertices == 0 {
        if let Some(batch) = &state.batch {
            commands.entity(batch.entity).insert(Visibility::Hidden);
        }
        return Ok(());
    }
    let pixel = Vec4::new(
        1. / camera.viewport.z as f32,
        -1. / camera.viewport.w as f32,
        camera.projection[10],
        camera.projection[14],
    );
    let mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.; 2]; vertices])
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    if let Some(batch) = &state.batch {
        *assets
            .meshes
            .get_mut(&batch.mesh)
            .ok_or("tracer mesh absent")? = mesh;
        assets
            .materials
            .get_mut(&batch.material)
            .ok_or("tracer material absent")?
            .pixel_fog = pixel;
        commands.entity(batch.entity).insert(Visibility::Inherited);
    } else {
        use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
        let texture = assets.images.add(Image::new_fill(
            Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[255, 255, 255, 255],
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        ));
        let identity = assets
            .buffers
            .add(crate::source_view::uniform_buffer(&Mat4::IDENTITY));
        let material = assets.materials.add(Material::from(Bindings {
            tracer: true,
            front: false,
            blend: 2,
            texture,
            clip: identity.clone(),
            world: identity,
            pixel_fog: pixel,
            color_alpha: Vec4::ONE,
            flags: Vec4::new(1., 0., 0., 2.),
        }));
        let mesh = assets.meshes.add(mesh);
        let entity = commands
            .spawn((
                Name::new("FLK source tracer ribbons"),
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material.clone()),
                Transform::default(),
                Visibility::Inherited,
                bevy::camera::visibility::NoFrustumCulling,
            ))
            .id();
        state.batch = Some(Batch {
            entity,
            mesh,
            material,
        });
    }
    if state.steps == 1 || state.steps.is_multiple_of(30) {
        println!(
            "cinematic_tracer_draw sequence={} owners={} slots={} segment_requests={} vertices={} raster_candidates={} depth_test=false depth_write=false fog=false blend=straight_alpha source_slot_order=true native_cadence_verified=false native_pixels_verified=false",
            cinematic.sequence(),
            state.owners.len(),
            state.registry.slots().count(),
            segments.len(),
            vertices,
            candidates
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_director_handoff_releases_owners_but_preserves_draining_history() {
        let mut state = State::default();
        state.prepare(1);
        let handle = state
            .registry
            .admit(history::Configuration {
                lifetime: 1.,
                fade_start: 0.25,
                colors: [[1., 0., 0.], [0., 1., 0.]],
                min_interval: 0.01,
                half_width: 0.5,
            })
            .unwrap()
            .unwrap();
        state.registry.sample(handle, [0., 0., 10.]).unwrap();
        state.registry.advance(0.125).unwrap();
        state.registry.sample(handle, [1., 0., 10.]).unwrap();
        state.owners.insert(
            (3, 7),
            Bound {
                handle,
                owner: owner::Owner::new(0., 0., 0.).unwrap(),
                flags: owner::ACTIVE,
                offset: 476,
            },
        );
        state.roots.insert(handle.slot(), 476);
        state.handoff().unwrap();
        state.prepare(1);
        state.prepare(1); // Repeated setup and a cut stay in the same replay.
        assert!(state.owners.is_empty());
        assert!(state.roots.is_empty());
        let slots = state
            .registry
            .slots()
            .map(|(_, refs, t)| (refs, t.count()))
            .collect::<Vec<_>>();
        assert_eq!(slots, vec![(0, 3)]);
        assert_eq!(state.registry.advance(0.125).unwrap().len(), 2);
        assert!(state.registry.advance(1.).unwrap().is_empty());
        assert!(state.prepare(2)); // An explicit replay restart must clear old slots.
        assert_eq!(state.registry.slots().count(), 0);
    }
}
