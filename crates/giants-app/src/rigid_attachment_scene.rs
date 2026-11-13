//! Common unlit attached-object inspection. Geometry/material capabilities and
//! actual specialized-consumer claims select objects, never shot-specific lists.
mod burst_owners;
pub(crate) mod capture;
mod growing;
mod mixed;
mod projection;
pub(crate) mod texture_animation;
mod tracer;
use bevy::{
    asset::RenderAssetUsages,
    ecs::system::SystemParam,
    mesh::MeshVertexBufferLayoutRef,
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    reflect::TypePath,
    render::render_resource::{
        AsBindGroup, PrimitiveTopology, RenderPipelineDescriptor, SpecializedMeshPipelineError,
    },
    shader::ShaderRef,
};
use giants_game::{
    attachment_geometry::Geometry, attachment_resources::Request, unlit_material::Profile,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Key {
    tracer: bool,
    front: bool,
    blend: u8,
}
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
#[bind_group_data(Key)]
pub struct Bindings {
    tracer: bool,
    front: bool,
    blend: u8,
    #[texture(0)]
    #[sampler(1)]
    texture: Handle<Image>,
    #[storage(2, read_only)]
    clip: Handle<bevy::render::storage::ShaderBuffer>,
    #[storage(3, read_only)]
    world: Handle<bevy::render::storage::ShaderBuffer>,
    #[uniform(4)]
    pixel_fog: Vec4,
    #[uniform(5)]
    color_alpha: Vec4,
    #[uniform(6)]
    flags: Vec4,
}
impl From<&Bindings> for Key {
    fn from(v: &Bindings) -> Self {
        Self {
            tracer: v.tracer,
            front: v.front,
            blend: v.blend,
        }
    }
}
pub type Material = crate::source_view::UniformMaterial<Bindings, 12>;
impl Bindings {
    pub(crate) fn set_owned_texture(&mut self, texture: Handle<Image>) {
        self.texture = texture;
    }
    pub(crate) fn set_owned_fog(&mut self, start: f32, end: f32, rgb: Vec3, enabled: bool) {
        self.pixel_fog.z = start;
        self.pixel_fog.w = end;
        self.color_alpha.x = rgb.x;
        self.color_alpha.y = rgb.y;
        self.color_alpha.z = rgb.z;
        self.flags.y = f32::from(u8::from(enabled));
    }
    pub(crate) fn set_owned_profile(&mut self, profile: Profile) {
        self.blend = profile.blend;
        self.color_alpha.w = profile.alpha;
        self.flags.x = if profile.add_vertex_rgb {
            2.
        } else {
            f32::from(u8::from(profile.vertex_rgb))
        };
        self.flags.z = f32::from(u8::from(profile.texture_alpha));
    }
    pub(crate) fn set_owned_vertex_alpha(&mut self, enabled: bool) {
        self.flags.w = f32::from(u8::from(enabled));
    }
    pub(crate) fn set_draw_view(&mut self, front: bool, width: f32, height: f32) {
        self.front = front;
        self.pixel_fog.x = 1. / width;
        self.pixel_fog.y = -1. / height;
    }
}
/// Shared unlit material binding for owned gameplay FX; no Flick source needed.
#[allow(clippy::too_many_arguments)] // Shared bindings without a second shader/material type.
pub(crate) fn owned_material(
    texture: Handle<Image>,
    clip: Handle<bevy::render::storage::ShaderBuffer>,
    world: Handle<bevy::render::storage::ShaderBuffer>,
    profile: Profile,
    front: bool,
    pixel_fog: Vec4,
    fog_rgb: Vec3,
    fog_enabled: bool,
) -> Material {
    Material::from(Bindings {
        tracer: false,
        texture,
        clip,
        world,
        front,
        blend: profile.blend,
        pixel_fog,
        color_alpha: Vec4::new(fog_rgb.x, fog_rgb.y, fog_rgb.z, profile.alpha),
        flags: Vec4::new(
            if profile.add_vertex_rgb {
                2.
            } else {
                f32::from(u8::from(profile.vertex_rgb))
            },
            f32::from(u8::from(fog_enabled)),
            f32::from(u8::from(profile.texture_alpha)),
            0.,
        ),
    })
}
impl bevy::pbr::Material for Material {
    fn vertex_shader() -> ShaderRef {
        "embedded://giants_app/rigid_attachment.wesl".into()
    }
    fn fragment_shader() -> ShaderRef {
        Self::vertex_shader()
    }
    fn alpha_mode(&self) -> AlphaMode {
        match self.blend {
            1 => AlphaMode::Add,
            2 => AlphaMode::Blend,
            _ => AlphaMode::Opaque,
        }
    }
    fn specialize(
        _: &MaterialPipeline,
        d: &mut RenderPipelineDescriptor,
        l: &MeshVertexBufferLayoutRef,
        k: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        use bevy::render::render_resource::*;
        d.vertex.buffers = vec![l.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(1),
            Mesh::ATTRIBUTE_COLOR.at_shader_location(2),
        ])?];
        d.primitive.cull_mode = Some(crate::source_view::cull_face(k.bind_group_data.front));
        if k.bind_group_data.blend != 0 {
            if let Some(depth) = &mut d.depth_stencil {
                depth.depth_write_enabled = Some(false);
            }
            if let Some(fragment) = &mut d.fragment {
                for t in fragment.targets.iter_mut().flatten() {
                    t.blend = Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::SrcAlpha,
                            dst_factor: if k.bind_group_data.blend == 2 {
                                BlendFactor::OneMinusSrcAlpha
                            } else {
                                BlendFactor::One
                            },
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent {
                            src_factor: if k.bind_group_data.blend == 2 {
                                BlendFactor::One
                            } else {
                                BlendFactor::Zero
                            },
                            dst_factor: if k.bind_group_data.blend == 2 {
                                BlendFactor::OneMinusSrcAlpha
                            } else {
                                BlendFactor::One
                            },
                            operation: BlendOperation::Add,
                        },
                    });
                }
            }
        }
        if k.bind_group_data.tracer
            && let Some(depth) = &mut d.depth_stencil
        {
            depth.depth_compare = Some(CompareFunction::Always);
            depth.depth_write_enabled = Some(false);
        }
        Ok(())
    }
}
struct Instance {
    source: Source,
    geometry: Arc<Geometry>,
    parts: Vec<(usize, Part)>,
    normals: Option<Vec<[f32; 3]>>,
    facing: Option<giants_game::attachment_pose::InitialFacing>,
    animation: Option<giants_game::effect_animation::Playback>,
    timed: Option<giants_game::timed_effect::Playback>,
    released: Option<Released>,
    projectile: Option<Projectile>,
    burst: Option<Burst>,
    emitted: Option<Emitted>,
    retirement_child: Option<usize>,
    canonical_only: bool,
    mixed: Option<mixed::Slot>,
    detached: bool,
    creation_position: Option<[f32; 3]>,
    /// Complete constant attached recipe; the live queue supplies traversal state.
    persistent_attachment: bool,
}
struct Emitted {
    group: usize,
    particle: usize,
    slot: usize,
    presentation: Option<giants_game::effect_emitter_runtime::Presentation>,
    parent: Option<mixed::AttachmentParent>,
}
struct Emitter {
    source: Source,
    recipe: Arc<giants_game::effect_emitter::Recipe>,
    reserve: usize,
    runtime: Option<giants_game::effect_emitter_runtime::Runtime>,
    parent: Option<usize>,
    reference: Option<mixed::ReferenceBinding>,
    position: Option<mixed::PositionBinding>,
}
struct PhasedEffect {
    parent: usize,
    sequence: Arc<giants_game::effect_sequence::Sequence>,
    cursor: Option<giants_game::effect_sequence::Cursor>,
    origin: Option<[f32; 3]>,
}
struct Burst {
    plan: Arc<giants_game::effect_burst::Plan>,
    references: Arc<giants_game::effect_burst::References>,
    index: usize,
    playback: Option<giants_game::effect_burst::Playback>,
    phase: Option<(usize, usize)>,
    services: Vec<usize>,
    contact_seed: Option<giants_game::effect_contact::Seed>,
    texture_frames: Option<Arc<Vec<giants_game::LogoTexture>>>,
    hidden_sound_classes: Arc<BTreeSet<i32>>,
}
struct Projectile {
    playback: giants_game::effect_projectile::Playback,
    retirement_group: Option<usize>,
    parent_own_reference: giants_game::procedural_aim::Matrix,
    parent_reference: giants_game::procedural_aim::Matrix,
    child_reference: giants_game::procedural_aim::Matrix,
    parent_size: f32,
}
struct Released {
    parent: Option<usize>,
    playback: giants_game::released_effect::Playback,
    routes: Option<giants_game::draw_routes::VerticalParticle>,
    last_route_mask: Option<u32>,
}
impl Released {
    fn uses_vertex_alpha(&self) -> bool {
        // The shader program is prepared before callback-owned fade state exists.
        self.playback.recipe.vertical_fade.is_some()
    }
    /// Defined callback outputs only; inspecting them never advances playback.
    fn vertical_state_words(&self) -> Option<[u32; 10]> {
        let visual = self.playback.visual.as_ref()?;
        let vertical = self.playback.vertical.as_ref()?;
        let position = self.playback.motion.position.map(f32::to_bits);
        Some([
            visual.age.to_bits(),
            self.playback.size().to_bits(),
            self.playback.transparency().to_bits(),
            u32::from(self.playback.expired),
            position[0],
            position[1],
            position[2],
            vertical.elapsed.to_bits(),
            vertical.values[0].to_bits(),
            vertical.values[1].to_bits(),
        ])
    }
}
#[derive(Clone)]
enum Source {
    Actor(Request),
    Effect {
        request: giants_formats::flick::EffectCommand,
        asset: Arc<giants_game::attachment_resources::Asset>,
        actors: Arc<giants_game::actor_links::Projection>,
        class: i32,
    },
}
impl Source {
    fn class(&self) -> i32 {
        match self {
            Self::Actor(r) => r.class,
            Self::Effect { class, .. } => *class,
        }
    }
    fn slot(&self) -> usize {
        match self {
            Self::Actor(r) => r.slot,
            Self::Effect { request, .. } => request.slot as usize,
        }
    }
    fn asset(&self) -> &giants_game::attachment_resources::Asset {
        match self {
            Self::Actor(r) => r.asset.as_ref().expect("prepared asset"),
            Self::Effect { asset, .. } => asset,
        }
    }
    fn matrix(
        &self,
        animation: &giants_game::animation::Animation,
        tick: i32,
    ) -> Result<([f32; 16], u32), String> {
        match self {
            Self::Actor(r) => r.matrix(animation, tick),
            Self::Effect {
                request, actors, ..
            } => {
                let [start, _, tpf, fps] = animation.timeline;
                let seconds = ((tick - start) as f64 / (f64::from(tpf) * f64::from(fps))) as f32;
                Self::effect_matrix(actors, request, seconds)
            }
        }
    }
    fn effect_matrix(
        actors: &giants_game::actor_links::Projection,
        request: &giants_formats::flick::EffectCommand,
        seconds: f32,
    ) -> Result<([f32; 16], u32), String> {
        let clocks = actors
            .actors
            .iter()
            .map(|(slot, a)| {
                (
                    *slot,
                    if a.director_slot.is_some() {
                        seconds
                    } else {
                        0.
                    },
                )
            })
            .collect();
        let locals = actors.actors.keys().map(|slot| (*slot, None)).collect();
        let matrix = actors.effect_anchor(
            usize::MAX,
            request.actor,
            request.reference.as_deref(),
            &clocks,
            &locals,
        )?;
        Ok((matrix.values, matrix.flags))
    }
    fn particle_position(
        &self,
        cinematic: &crate::cinematic_camera::Cinematic,
    ) -> Result<[f32; 3], String> {
        let Self::Effect {
            actors, request, ..
        } = self
        else {
            return Err("source particle requires an effect command".into());
        };
        let (seconds, generation) = cinematic.segments()[cinematic.director_index()]
            .effect_sample(request.offset)
            .ok_or("particle source request has no clock sample")?;
        let (values, flags) = Self::effect_matrix(actors, request, seconds)?;
        let position = giants_game::effect_attachment::source_creation_position(
            generation,
            request.actor,
            giants_game::procedural_aim::Matrix { values, flags },
        );
        println!(
            "cinematic_particle_support_sample sequence={} offset={} seconds_bits={} generation={} cold={} native_cadence_verified=false",
            cinematic.sequence(),
            request.offset,
            seconds.to_bits(),
            generation,
            generation == 0 && request.actor >= 0
        );
        Ok(position)
    }
    fn visible(&self, cinematic: &crate::cinematic_camera::Cinematic) -> bool {
        match self {
            Self::Actor(request) => cinematic.actor_requested(request.slot) == Some(true),
            Self::Effect { request, .. } => cinematic.segments()[cinematic.director_index()]
                .effect_requests()
                .iter()
                .any(|(_, r)| r.offset == request.offset),
        }
    }
}
impl Instance {
    fn has_vertical_vertex_alpha(&self) -> bool {
        self.released
            .as_ref()
            .is_some_and(Released::uses_vertex_alpha)
    }
    fn visible(&self, cinematic: &crate::cinematic_camera::Cinematic) -> bool {
        if self.canonical_only {
            return false;
        }
        if let Some(mixed) = &self.mixed {
            mixed.presentation.is_some_and(|p| p.visible)
        } else if let Some(emitted) = &self.emitted {
            emitted.presentation.is_some_and(|p| !p.expired)
        } else if let Some(burst) = &self.burst {
            burst.playback.as_ref().is_some_and(|p| p.visible())
        } else if let Some(projectile) = &self.projectile {
            projectile.playback.matrix.is_some() && !projectile.playback.expired
        } else if let Some(released) = &self.released {
            released.playback.visible()
        } else {
            self.source.visible(cinematic) && self.timed.as_ref().is_none_or(|p| p.visible())
        }
    }
    fn transparency(&self) -> Option<f32> {
        self.mixed
            .as_ref()
            .and_then(|m| m.presentation.map(|p| p.transparency))
            .or_else(|| {
                self.projectile
                    .as_ref()
                    .and_then(|p| p.playback.visual.as_ref().map(|v| v.transparency))
                    .or_else(|| {
                        self.emitted
                            .as_ref()
                            .and_then(|e| e.presentation.map(|p| p.transparency))
                    })
                    .or_else(|| {
                        self.burst
                            .as_ref()
                            .and_then(|b| b.playback.as_ref().map(|p| p.visual.transparency))
                    })
                    .or_else(|| self.timed.as_ref().map(|p| p.transparency()))
                    .or_else(|| self.released.as_ref().map(|p| p.playback.transparency()))
            })
    }
    fn render_matrix(
        &self,
        animation: &giants_game::animation::Animation,
        tick: i32,
        view: [f32; 16],
    ) -> Result<([f32; 16], bool), String> {
        self.render_matrix_with_world(animation, tick, view, None, None)
    }
    fn render_matrix_with_world(
        &self,
        animation: &giants_game::animation::Animation,
        tick: i32,
        view: [f32; 16],
        owned: Option<giants_game::procedural_aim::Matrix>,
        owned_size: Option<f32>,
    ) -> Result<([f32; 16], bool), String> {
        let (mut binding, mut flags) = if let Some(matrix) = owned {
            (matrix.values, matrix.flags)
        } else if let Some(mixed) = &self.mixed {
            let m = mixed
                .presentation
                .map_or_else(giants_game::procedural_aim::Matrix::identity, |p| p.matrix);
            (m.values, m.flags)
        } else if let Some(emitted) = &self.emitted {
            let p = emitted.presentation;
            let matrix = if let Some(p) = p.filter(|p| !p.expired && p.attachment.is_some()) {
                let parent = emitted.parent.ok_or("paired current parent pose absent")?;
                let own = self
                    .animation
                    .as_ref()
                    .ok_or("paired child pose absent")?
                    .reference(
                        p.attachment.ok_or("paired binding absent")?.child_reference as usize,
                    )?;
                // The original effect-list loop resolves the attachment after
                // state78's angular matrix. Do not draw the temporary spin basis.
                giants_game::effect_attachment::initial_child_matrix(
                    parent.matrix,
                    parent.reference,
                    own,
                    p.size,
                    parent.size,
                )
                .map_err(str::to_owned)?
            } else {
                giants_game::effect_orientation::matrix(
                    p.map_or([0.; 3], |p| p.position),
                    p.and_then(|p| p.angles).unwrap_or([0.; 3]),
                    p.map_or(1., |p| p.size),
                )
                .map_err(str::to_owned)?
            };
            (matrix.values, matrix.flags)
        } else if let Some(burst) = &self.burst {
            let matrix = burst
                .playback
                .as_ref()
                .map(|p| p.matrix())
                .transpose()
                .map_err(str::to_owned)?
                .unwrap_or_else(giants_game::procedural_aim::Matrix::identity);
            (matrix.values, matrix.flags)
        } else if let Some(projectile) = &self.projectile {
            let matrix = projectile
                .playback
                .world_matrix()
                .unwrap_or_else(giants_game::procedural_aim::Matrix::identity);
            (matrix.values, matrix.flags)
        } else if let Some(released) = &self.released {
            (
                giants_game::object_transform::euler(
                    released.playback.motion.position,
                    [0.; 3],
                    released.playback.size(),
                )
                .map_err(str::to_owned)?,
                18,
            )
        } else if self.detached {
            (
                giants_game::object_transform::euler(
                    self.creation_position.unwrap_or([0.; 3]),
                    [0.; 3],
                    self.timed.as_ref().map_or(1., |p| p.size()),
                )
                .map_err(str::to_owned)?,
                18,
            )
        } else if let Source::Actor(request) = &self.source {
            request.matrix_with_playback(animation, tick, self.animation.as_ref())?
        } else {
            self.source.matrix(animation, tick)?
        };
        if let Some(timed) = &self.timed
            && owned.is_none()
            && !self.detached
            && self.facing.is_none()
        {
            // Flick creates a separate support object with unit scale. The
            // carrier's scale is already present in its anchor matrix; it is
            // not the support scale used by the normal-effect size ratio.
            let support_scale = 1.;
            let reference = self
                .animation
                .as_ref()
                .ok_or("timed effect own reference animation absent")?
                .reference(0)?;
            let matrix = giants_game::effect_attachment::normal_matrix(
                giants_game::procedural_aim::Matrix {
                    flags,
                    values: binding,
                },
                timed.size(),
                support_scale,
                reference,
            )
            .map_err(str::to_owned)?;
            binding = matrix.values;
            flags = matrix.flags;
        }
        let (matrix, front) = if let Some(mut facing) = self.facing {
            if let Some(size) = owned_size {
                facing.scale = size;
            } else if let Some(mixed) = &self.mixed {
                facing.scale = mixed.presentation.map_or(1., |p| p.size);
            } else if let Some(emitted) = &self.emitted {
                facing.scale = emitted.presentation.map_or(1., |p| p.size);
            } else if let Some(burst) = &self.burst {
                facing.scale = burst.playback.as_ref().map_or(1., |p| p.visual.size);
            } else if let Some(released) = &self.released {
                facing.scale = released.playback.size();
            } else if let Some(timed) = &self.timed {
                facing.scale = timed.size();
            }
            (
                if let Some(mixed) = &self.mixed {
                    giants_game::camera_facing::draw_matrix(
                        binding,
                        facing.personality,
                        giants_game::camera_facing::Input {
                            view,
                            position: [binding[3], binding[7], binding[11]],
                            scale: facing.scale,
                            angle: mixed
                                .presentation
                                .and_then(|p| p.angles)
                                .map_or(0., |a| a[2]),
                        },
                        giants_game::arithmetic::Precision::Double53,
                    )
                    .map_err(str::to_owned)?
                } else if let Some(p) = self.emitted.as_ref().and_then(|e| e.presentation)
                    && let Some(angles) = p.angles
                {
                    giants_game::camera_facing::draw_matrix(
                        binding,
                        facing.personality,
                        giants_game::camera_facing::Input {
                            view,
                            // The effect-list refresh updates an attached
                            // object's position from its resolved matrix before
                            // the facing draw prefix. Birth position is stale
                            // while the parent/reference animation moves.
                            position: [binding[3], binding[7], binding[11]],
                            scale: p.size,
                            angle: angles[2],
                        },
                        giants_game::arithmetic::Precision::Double53,
                    )
                    .map_err(str::to_owned)?
                } else if let Some(burst) = &self.burst {
                    giants_game::camera_facing::draw_matrix(
                        binding,
                        facing.personality,
                        giants_game::camera_facing::Input {
                            view,
                            position: [binding[3], binding[7], binding[11]],
                            scale: facing.scale,
                            angle: burst
                                .playback
                                .as_ref()
                                .and_then(|p| p.angles)
                                .map_or(0., |a| a[2]),
                        },
                        giants_game::arithmetic::Precision::Double53,
                    )
                    .map_err(str::to_owned)?
                } else {
                    facing.draw_matrix(binding, view).map_err(str::to_owned)?
                },
                facing.front(flags),
            )
        } else {
            (binding, flags & 8 != 0)
        };
        Ok((matrix, front))
    }
}
fn unlit_effect_instance(
    class: i32,
    facing: Option<giants_game::attachment_pose::InitialFacing>,
    request: &giants_formats::flick::EffectCommand,
    actors: Arc<giants_game::actor_links::Projection>,
    effects: &giants_formats::effect_image::Image,
    resources: &giants_game::scene_resources::SceneResources,
    registered: &mut giants_game::attachment_resources::Registered<'_>,
) -> Result<Instance, String> {
    let asset = registered.asset(class)?;
    if asset.model.mesh.normal_count != 0 {
        return Err("composite inspection requires unlit geometry".into());
    }
    let parts = asset
        .model
        .mesh
        .subobjects
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let alpha = asset
                .model
                .alpha_textures
                .contains(&p.material.texture.to_ascii_lowercase());
            giants_game::unlit_material::prepare_with_texture_alpha(&p.material, alpha)
                .map(|p| (i, Part::Unlit(p)))
                .map_err(str::to_owned)
        })
        .collect::<Result<Vec<_>, _>>()?;
    if parts.is_empty() {
        return Err("composite object has no drawable parts".into());
    }
    let actor = giants_game::actor_context::Prepared::class_initial(Arc::new(
        giants_game::actor_context::Class::load(resources, class, effects)?,
    ));
    let geometry = Arc::new(giants_game::attachment_geometry::prepare_actor_initial(
        &asset, &actor,
    )?);
    let animation = giants_game::effect_animation::Playback::new(actor)?;
    Ok(Instance {
        source: Source::Effect {
            request: request.clone(),
            asset,
            actors,
            class,
        },
        geometry,
        parts,
        normals: None,
        facing,
        animation,
        timed: None,
        released: None,
        projectile: None,
        burst: None,
        emitted: None,
        retirement_child: None,
        canonical_only: false,
        mixed: None,
        detached: false,
        creation_position: None,
        persistent_attachment: false,
    })
}
fn periodic_instances(
    recipe: giants_game::effect_emitter::Recipe,
    request: &giants_formats::flick::EffectCommand,
    actors: Arc<giants_game::actor_links::Projection>,
    effects: &giants_formats::effect_image::Image,
    resources: &giants_game::scene_resources::SceneResources,
    registered: &mut giants_game::attachment_resources::Registered<'_>,
    allocation: (usize, usize),
) -> Result<(Emitter, Vec<Instance>), String> {
    let (group, reserve) = allocation;
    let root_program = resources
        .classes
        .models
        .first_program(recipe.class)
        .ok_or("periodic support class absent")?
        .program
        .as_ref()
        .map_err(Clone::clone)?;
    giants_game::released_effect::state78_facing(recipe.class, root_program, true)
        .map_err(str::to_owned)?;
    let mut instances = Vec::new();
    let mut prototypes = BTreeMap::new();
    for (particle, specification) in recipe.particles.iter().enumerate() {
        let class = specification.class;
        if let std::collections::btree_map::Entry::Vacant(entry) = prototypes.entry(class) {
            let program = resources
                .classes
                .models
                .first_program(class)
                .ok_or("emitted class absent")?
                .program
                .as_ref()
                .map_err(Clone::clone)?;
            if recipe
                .particles
                .iter()
                .any(|p| p.class == class && p.tracer.is_some() && !p.size_configured)
                && program.declared_scale().map_err(|e| e.to_string())? != 1.
            {
                return Err("emitted light without size curve has non-unit class scale".into());
            }
            let facing = giants_game::released_effect::state78_facing(class, program, true)
                .map_err(str::to_owned)?;
            entry.insert(unlit_effect_instance(
                class,
                facing,
                request,
                actors.clone(),
                effects,
                resources,
                registered,
            )?);
        }
        let prototype = &prototypes[&class];
        if specification.tracer.is_some()
            && !specification.size_configured
            && !prototype.facing.is_some_and(|f| f.scale == 1.)
        {
            return Err(
                "emitted light without size curve requires checked unit constructed size".into(),
            );
        }
        if let Some(tracer) = &specification.tracer {
            println!(
                "cinematic_emitted_tracer_pending effect={:?} offset={} particle={} class={} configuration_words={:?} tracer_backend_ready=false native_service_complete=false",
                request.name,
                request.offset,
                particle,
                class,
                tracer.configured_words()
            );
        }
        for slot in 0..reserve {
            instances.push(Instance {
                source: prototype.source.clone(),
                geometry: prototype.geometry.clone(),
                parts: prototype.parts.clone(),
                normals: None,
                facing: prototype.facing,
                animation: Some(
                    prototype
                        .animation
                        .as_ref()
                        .ok_or("emitted animation absent")?
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
                canonical_only: false,
                mixed: None,
                detached: false,
                creation_position: None,
                persistent_attachment: false,
            });
        }
    }
    let source = Source::Effect {
        request: request.clone(),
        asset: registered.asset(recipe.class)?,
        actors,
        class: recipe.class,
    };
    Ok((
        Emitter {
            source,
            recipe: Arc::new(recipe),
            reserve,
            runtime: None,
            parent: None,
            reference: None,
            position: None,
        },
        instances,
    ))
}
fn burst_instances(
    plan: giants_game::effect_burst::Plan,
    request: &giants_formats::flick::EffectCommand,
    actors: Arc<giants_game::actor_links::Projection>,
    effects: &giants_formats::effect_image::Image,
    resources: &giants_game::scene_resources::SceneResources,
    registered: &mut giants_game::attachment_resources::Registered<'_>,
) -> Result<Vec<Instance>, String> {
    let mut seed = 1;
    let spawns = plan.create(&mut seed).map_err(str::to_owned)?;
    let references = Arc::new(giants_game::effect_burst::References::load(
        resources, effects, &spawns,
    )?);
    let plan = Arc::new(plan);
    let mut prototypes = BTreeMap::new();
    let mut texture_frames = BTreeMap::new();
    let mut contact_profiles = BTreeMap::new();
    let mut hidden_sound_classes = BTreeSet::new();
    for spawn in spawns.iter().filter(|s| burst_owners::admitted(s)) {
        let program = resources
            .classes
            .models
            .first_program(spawn.class)
            .ok_or("hidden sound owner class absent")?
            .program
            .as_ref()
            .map_err(Clone::clone)?;
        match giants_game::released_effect::state78_burst_facing(spawn.class, program) {
            Ok(_) => {
                hidden_sound_classes.insert(spawn.class);
            }
            Err(reason) => println!(
                "cinematic_burst_hidden_owner_deferred class={} reason={reason:?}",
                spawn.class
            ),
        }
    }
    let hidden_sound_classes = Arc::new(hidden_sound_classes);
    for spawn in spawns
        .iter()
        .filter(|s| s.contact_requested && s.physics_enabled)
    {
        if let std::collections::btree_map::Entry::Vacant(entry) =
            contact_profiles.entry(spawn.class)
        {
            let program = resources
                .classes
                .models
                .first_program(spawn.class)
                .ok_or("burst contact class absent")?
                .program
                .as_ref()
                .map_err(Clone::clone)?;
            let profile = match giants_game::effect_contact::Seed::from_class(program) {
                Ok(seed) => Some(seed),
                Err(reason) => {
                    println!(
                        "cinematic_burst_contact_class_deferred effect={:?} offset={} class={} reason={reason:?}",
                        request.name, request.offset, spawn.class
                    );
                    None
                }
            };
            entry.insert(profile);
        }
    }
    for spawn in spawns.iter().filter(|s| !s.hidden) {
        if let std::collections::btree_map::Entry::Vacant(entry) = prototypes.entry(spawn.class) {
            let program = resources
                .classes
                .models
                .first_program(spawn.class)
                .ok_or("burst class absent")?
                .program
                .as_ref()
                .map_err(Clone::clone)?;
            let facing = giants_game::released_effect::state78_burst_facing(spawn.class, program)
                .map_err(str::to_owned)?;
            entry.insert(unlit_effect_instance(
                spawn.class,
                facing,
                request,
                actors.clone(),
                effects,
                resources,
                registered,
            )?);
        }
    }
    for spawn in &spawns {
        if let Some(configuration) = spawn.texture_animation {
            let prototype = prototypes
                .get(&spawn.class)
                .ok_or("animated texture burst has no drawable prototype")?;
            let frames =
                texture_animation::load(configuration, &prototype.source.asset().model, resources)?;
            println!(
                "cinematic_texture_animation_prepared offset={} class={} frames={} rate_bits={} native_raster_verified=false",
                request.offset,
                spawn.class,
                frames.len(),
                configuration.rate.to_bits()
            );
            texture_frames.insert((spawn.class, configuration.frames), Arc::new(frames));
        }
    }
    spawns
        .iter()
        .enumerate()
        .filter(|(_, s)| !s.hidden)
        .map(|(index, spawn)| {
            let prototype = &prototypes[&spawn.class];
            let Source::Effect {
                request,
                asset,
                actors,
                class,
            } = &prototype.source
            else {
                return Err("burst prototype source absent".into());
            };
            // Share decoded geometry/class data, retain each object's animation and
            // physics separately. Preparing hundreds of children must not reparse
            // the same class and model for every instance.
            let animation = prototype
                .animation
                .as_ref()
                .map(giants_game::effect_animation::Playback::fresh_instance)
                .transpose()?;
            Ok(Instance {
                source: Source::Effect {
                    request: request.clone(),
                    asset: asset.clone(),
                    actors: actors.clone(),
                    class: *class,
                },
                geometry: prototype.geometry.clone(),
                parts: prototype.parts.clone(),
                normals: None,
                facing: prototype.facing,
                animation,
                timed: None,
                released: None,
                projectile: None,
                burst: Some(Burst {
                    plan: plan.clone(),
                    references: references.clone(),
                    index,
                    playback: None,
                    phase: None,
                    services: Vec::new(),
                    hidden_sound_classes: hidden_sound_classes.clone(),
                    contact_seed: contact_profiles.get(&spawn.class).copied().flatten(),
                    texture_frames: spawn
                        .texture_animation
                        .and_then(|c| texture_frames.get(&(spawn.class, c.frames)).cloned()),
                }),
                emitted: None,
                retirement_child: None,
                canonical_only: false,
                mixed: None,
                detached: false,
                creation_position: None,
                persistent_attachment: false,
            })
        })
        .collect()
}
fn prepare_retirement(
    parent: usize,
    base: usize,
    emitter_base: usize,
    instance: &Instance,
    effects: &giants_formats::effect_image::Image,
    resources: &giants_game::scene_resources::SceneResources,
    registered: &mut giants_game::attachment_resources::Registered<'_>,
) -> Result<(PhasedEffect, Vec<Instance>, Vec<Emitter>), String> {
    let projectile = instance
        .projectile
        .as_ref()
        .ok_or("retirement projectile absent")?;
    let sequence = Arc::new(
        giants_game::effect_sequence::Sequence::project(
            effects,
            &projectile.playback.recipe.retirement_child,
        )
        .map_err(str::to_owned)?,
    );
    let Source::Effect {
        request, actors, ..
    } = &instance.source
    else {
        return Err("retirement source absent".into());
    };
    let mut output = Vec::new();
    let mut emitters = Vec::new();
    let mut seed = 1;
    for (phase_index, phase) in sequence.phases.iter().enumerate() {
        let spawns = phase.plan.create(&mut seed).map_err(str::to_owned)?;
        let objects = burst_instances(
            phase.plan.clone(),
            request,
            actors.clone(),
            effects,
            resources,
            registered,
        )?;
        let first = output.len();
        let end = first + objects.len();
        output.extend(objects);
        for local in first..end {
            let body_index = output[local]
                .burst
                .as_ref()
                .ok_or("retirement body absent")?
                .index;
            output[local].burst.as_mut().expect("body").phase = Some((usize::MAX, phase_index));
            for need in &spawns[body_index].child_services {
                let recipe = need
                    .emission(spawns[body_index].class, effects)
                    .map_err(str::to_owned)?;
                let reserve = recipe.reserve(0.).map_err(str::to_owned)?;
                let group = emitter_base + emitters.len();
                let (mut emitter, slots) = periodic_instances(
                    recipe,
                    request,
                    actors.clone(),
                    effects,
                    resources,
                    registered,
                    (group, reserve),
                )?;
                emitter.parent = Some(base + local);
                output[local]
                    .burst
                    .as_mut()
                    .expect("body")
                    .services
                    .push(group);
                emitters.push(emitter);
                output.extend(slots);
            }
        }
    }
    Ok((
        PhasedEffect {
            parent,
            sequence,
            cursor: None,
            origin: None,
        },
        output,
        emitters,
    ))
}
fn composite_instance(
    object: &giants_game::effect_composition::Object,
    request: &giants_formats::flick::EffectCommand,
    actors: Arc<giants_game::actor_links::Projection>,
    effects: &giants_formats::effect_image::Image,
    resources: &giants_game::scene_resources::SceneResources,
    registered: &mut giants_game::attachment_resources::Registered<'_>,
) -> Result<Instance, String> {
    let class_program = resources
        .classes
        .models
        .first_program(object.timed.class)
        .ok_or("composite class absent")?
        .program
        .as_ref()
        .map_err(Clone::clone)?;
    let facing =
        giants_game::released_effect::state78_facing(object.timed.class, class_program, true)
            .map_err(str::to_owned)?;
    let mut instance = unlit_effect_instance(
        object.timed.class,
        facing,
        request,
        actors,
        effects,
        resources,
        registered,
    )?;
    instance.timed = Some(giants_game::timed_effect::Playback::new(
        object.timed.clone(),
    ));
    instance.detached = !object.attached;
    if object.hidden {
        instance.parts.clear();
    }
    Ok(instance)
}
#[allow(clippy::too_many_arguments)]
fn projectile_instance(
    parent: &giants_game::effect_composition::Object,
    child: &giants_game::effect_composition::Immediate,
    request: &giants_formats::flick::EffectCommand,
    actors: Arc<giants_game::actor_links::Projection>,
    effects: &giants_formats::effect_image::Image,
    resources: &giants_game::scene_resources::SceneResources,
    registered: &mut giants_game::attachment_resources::Registered<'_>,
) -> Result<Instance, String> {
    if !parent.attached || parent.timed.sample_size || parent.timed.range != [1.; 2] {
        return Err("projectile parent outside checked unit attached profile".into());
    }
    let (_, entry) = effects
        .find(&child.name)
        .ok_or("projectile program absent")?;
    let recipe = giants_game::effect_projectile::Recipe::project(
        &effects.program(entry.offset).map_err(|e| e.to_string())?,
    )
    .map_err(str::to_owned)?;
    let class_program = resources
        .classes
        .models
        .first_program(recipe.class)
        .ok_or("projectile class absent")?
        .program
        .as_ref()
        .map_err(Clone::clone)?;
    recipe
        .validate_class(class_program)
        .map_err(str::to_owned)?;
    if giants_game::object_lists::DirectFlags::project(recipe.class, class_program).personality
        & 0x84
        != 0
    {
        return Err("projectile camera-facing personality outside checked profile".into());
    }
    let mut instance = unlit_effect_instance(
        recipe.class,
        None,
        request,
        actors,
        effects,
        resources,
        registered,
    )?;
    if instance.facing.is_some() {
        return Err("projectile camera-facing class outside checked profile".into());
    }
    let parent_actor = giants_game::actor_context::Prepared::class_initial(Arc::new(
        giants_game::actor_context::Class::load(resources, parent.timed.class, effects)?,
    ));
    let parent_animation = giants_game::effect_animation::Playback::new(parent_actor)?
        .ok_or("projectile parent reference animation absent")?;
    let own = instance
        .animation
        .as_ref()
        .ok_or("projectile reference animation absent")?;
    let mut playback = giants_game::effect_projectile::Playback::new(recipe);
    playback
        .configure_class(class_program)
        .map_err(str::to_owned)?;
    if playback.recipe.contact_requested {
        playback
            .bind_static_contact(
                giants_game::effect_contact::Seed::from_projectile_class(class_program)
                    .map_err(str::to_owned)?,
            )
            .map_err(str::to_owned)?;
    }
    instance.projectile = Some(Projectile {
        parent_own_reference: parent_animation.reference(0)?,
        parent_reference: parent_animation.reference(child.reference as usize)?,
        child_reference: own.reference(child.operand as usize)?,
        parent_size: 1.,
        playback,
        retirement_group: None,
    });
    Ok(instance)
}
fn released_instance(
    parent: &Instance,
    parent_index: usize,
    effects: &giants_formats::effect_image::Image,
    resources: &giants_game::scene_resources::SceneResources,
    registered: &mut giants_game::attachment_resources::Registered<'_>,
) -> Result<Instance, String> {
    let name = &parent
        .timed
        .as_ref()
        .ok_or("retirement parent has no timed recipe")?
        .recipe
        .child;
    let (_, entry) = effects.find(name).ok_or("retirement program absent")?;
    let recipe = giants_game::released_effect::Recipe::project(
        &effects.program(entry.offset).map_err(|e| e.to_string())?,
    )
    .map_err(str::to_owned)?;
    let Source::Effect {
        request, actors, ..
    } = &parent.source
    else {
        return Err("retirement owner is not an effect".into());
    };
    let source = Source::Effect {
        request: request.clone(),
        asset: registered.asset(recipe.class)?,
        actors: actors.clone(),
        class: recipe.class,
    };
    particle_instance(source, recipe, Some(parent_index), effects, resources)
}
fn routed_vertical_materials(parts: &[(usize, Part)]) -> bool {
    !parts.is_empty()
        && parts.iter().all(|(_, part)| {
            // Existing material mode1 is SrcAlpha/One; mode2 is straight alpha.
            matches!(part,Part::Unlit(p) if p.blend==1 && p.texture_alpha)
        })
}
fn particle_instance(
    source: Source,
    recipe: giants_game::released_effect::Recipe,
    parent: Option<usize>,
    effects: &giants_formats::effect_image::Image,
    resources: &giants_game::scene_resources::SceneResources,
) -> Result<Instance, String> {
    let program = resources
        .classes
        .models
        .first_program(recipe.class)
        .ok_or("released class absent")?
        .program
        .as_ref()
        .map_err(Clone::clone)?;
    let facing = recipe.facing(program).map_err(str::to_owned)?;
    let asset = source.asset();
    if recipe.vertical_fade.is_some() {
        let bounds = asset.model.mesh.bounds().map_err(|e| e.to_string())?;
        giants_game::vertical_fade::alpha(bounds[2], [bounds[2], bounds[5]], [0.; 2])
            .map_err(str::to_owned)?;
    }
    if asset.model.mesh.normal_count != 0 {
        return Err("released effect requires unlit geometry".into());
    }
    let parts = asset
        .model
        .mesh
        .subobjects
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let alpha = asset
                .model
                .alpha_textures
                .contains(&p.material.texture.to_ascii_lowercase());
            giants_game::unlit_material::prepare_with_texture_alpha(&p.material, alpha)
                .map(|p| (i, Part::Unlit(p)))
                .map_err(str::to_owned)
        })
        .collect::<Result<Vec<_>, _>>()?;
    if parts.is_empty() {
        return Err("released effect has no drawable parts".into());
    }
    let actor = giants_game::actor_context::Prepared::class_initial(Arc::new(
        giants_game::actor_context::Class::load(resources, recipe.class, effects)?,
    ));
    let geometry = Arc::new(giants_game::attachment_geometry::prepare_actor_initial(
        asset, &actor,
    )?);
    let animation = giants_game::effect_animation::Playback::new(actor)?;
    let routes = recipe
        .vertical_fade
        .is_some()
        .then(|| giants_game::draw_routes::VerticalParticle::project(program))
        .flatten()
        .filter(|_| routed_vertical_materials(&parts));
    if routes.is_some() {
        println!(
            "cinematic_particle_draw_routes_prepared class={} parts={} source_registration=true native_sort_order_verified=false",
            recipe.class,
            parts.len()
        );
    }
    Ok(Instance {
        source,
        geometry,
        parts,
        normals: None,
        facing,
        animation,
        timed: None,
        released: Some(Released {
            parent,
            playback: giants_game::released_effect::Playback::new(recipe),
            routes,
            last_route_mask: None,
        }),
        projectile: None,
        burst: None,
        emitted: None,
        retirement_child: None,
        canonical_only: false,
        mixed: None,
        detached: false,
        creation_position: None,
        persistent_attachment: false,
    })
}
struct Draw {
    instance: usize,
    entity: Entity,
    part: usize,
    material: DrawMaterial,
    mesh: Handle<Mesh>,
    texture_frames: Option<Vec<Handle<Image>>>,
    /// Zero is the existing consumer; bits1/2 select the acquired traversals.
    route: u32,
}
#[derive(Clone, Copy)]
enum Part {
    Unlit(Profile),
    Lit {
        alpha: bool,
        additive: Option<(f32, bool)>,
    },
    Context,
    Cube {
        factor: f32,
        blend: u8,
    },
}
enum DrawMaterial {
    Unlit(Handle<Material>),
    Lit {
        material: Handle<crate::actor_scene::LitMaterial>,
        parameters: Handle<bevy::render::storage::ShaderBuffer>,
        alpha: bool,
        context: bool,
        additive: Option<(f32, bool)>,
    },
    Context(Handle<crate::body_context::Material>),
    Cube {
        material: Handle<crate::sword_scene::Material>,
        parameters: Handle<bevy::render::storage::ShaderBuffer>,
        factor: f32,
        blend: u8,
    },
}
struct Motion {
    instance: usize,
    buffer: Option<Handle<bevy::render::storage::ShaderBuffer>>,
    matrix: Mat4,
}
pub struct Prepared {
    instances: Vec<Instance>,
    draws: Vec<Draw>,
    motion: Vec<Motion>,
    fog: giants_game::lighting::Fog,
    lights: Arc<Vec<giants_game::lighting::Directional>>,
    tick: i32,
    generation: u64,
    effect_seed: u32,
    hidden_sound_owners: burst_owners::Owners,
    tracers: tracer::State,
    emitters: Vec<Emitter>,
    phased: Vec<PhasedEffect>,
    mixed: Vec<mixed::Group>,
    context_images: BTreeMap<(i32, String), giants_formats::tga::RgbImage>,
}
pub type Scene = crate::cinematic_scene::Scenes<Prepared>;
fn world_matrix(m: [f32; 16]) -> Mat4 {
    let c = Mat4::from_cols(Vec4::X, -Vec4::Z, Vec4::Y, Vec4::W);
    let inverse = Mat4::from_cols(Vec4::X, Vec4::Z, -Vec4::Y, Vec4::W);
    c * Mat4::from_cols_array(&m).transpose() * inverse
}
fn context_colors(
    instance: &Instance,
    part: usize,
    cinematic: &crate::cinematic_camera::Cinematic,
    light: &giants_game::lighting::Directional,
) -> Result<Vec<[f32; 4]>, String> {
    let (matrix, flags) = instance
        .source
        .matrix(cinematic.animation(), cinematic.tick())?;
    let direction =
        giants_game::body_pipeline::local_context_direction(matrix, flags, light.direction)
            .map_err(str::to_owned)?;
    crate::body_context::mesh_colors(
        &instance.source.asset().model.mesh,
        &instance.geometry.positions,
        instance.normals.as_ref().ok_or("context normals absent")?,
        part,
        direction,
    )
    .map_err(str::to_owned)
}
impl Scene {
    /// Asset admission only; GPU upload and live drawing are checked separately.
    pub(crate) fn covers_canonical_effects(
        &self,
        director: usize,
        bindings: &giants_game::cinematic_timed::Bindings,
    ) -> bool {
        let Some(scene) = self.entry(director) else {
            return false;
        };
        bindings.geometry_roles().all(|(offset, class)| {
            scene.instances.iter().any(|instance| {
                matches!(&instance.source, Source::Effect { request, class: actual, .. }
                    if request.offset == offset && *actual == class)
                    && !instance.geometry.positions.is_empty()
                    && !instance.parts.is_empty()
                    && !instance.source.asset().model.mesh.indices.is_empty()
            })
        })
    }
    pub fn burst_audio_owner(
        &self,
        key: &crate::cinematic_effect_audio::Key,
    ) -> Option<crate::cinematic_effect_audio::OwnerState> {
        if key.family != "burst"
            || key.generation != self.generation
            || key.director != self.active_index()
        {
            return None;
        }
        let identity = (key.command, key.group, key.phase, key.object);
        if let Some(owner) = self.hidden_sound_owners.get(identity) {
            return Some(owner);
        }
        self.instances.iter().find_map(|i| {
            let b = i.burst.as_ref()?;
            let Source::Effect { request, .. } = &i.source else {
                return None;
            };
            let phase = b.phase.unwrap_or((usize::MAX, 0));
            if request.offset != key.command
                || phase != (key.group, key.phase)
                || b.index != key.object
            {
                return None;
            }
            let p = b.playback.as_ref()?;
            Some(crate::cinematic_effect_audio::OwnerState {
                position: p.motion.position,
                valid: !p.expired,
            })
        })
    }
    /// Capture a developed particle instead of its almost empty first frame.
    /// This observation threshold never changes the particle's simulation.
    pub fn released_effect_capture_ready(&self) -> bool {
        self.particle_capture_ready(true)
    }
    pub fn source_particle_capture_ready(&self) -> bool {
        self.particle_capture_ready(false)
    }
    fn particle_capture_ready(&self, has_parent: bool) -> bool {
        self.instances.iter().any(|i| {
            i.released.as_ref().is_some_and(|p| {
                p.parent.is_some() == has_parent
                    && p.playback.visible()
                    && p.playback.transparency() < 1.
                    && p.playback.visual.as_ref().is_some_and(|v| {
                        v.age >= 0.15_f32.min(p.playback.recipe.visual.lifetime * 0.25)
                    })
            })
        })
    }
    pub fn burst_effect_capture_ready(&self) -> bool {
        mixed::capture_ready(self)
            || self.instances.iter().any(|i| {
                i.burst
                    .as_ref()
                    .and_then(|b| b.playback.as_ref())
                    .is_some_and(|p| {
                        p.visible()
                            && p.visual.transparency < 1.
                            && p.visual.age >= 0.15_f32.min(p.spawn.visual.lifetime * 0.25)
                    })
            })
    }
    pub fn periodic_effect_capture_ready(&self) -> bool {
        self.instances.iter().any(|i| {
            i.emitted
                .as_ref()
                .and_then(|e| e.presentation)
                .is_some_and(|p| !p.expired && p.transparency < 1. && p.age >= 0.05)
        })
    }
    pub fn projectile_effect_capture_ready(&self) -> bool {
        self.instances.iter().any(|i| {
            i.projectile.as_ref().is_some_and(|p| {
                p.playback.matrix.is_some() && !p.playback.expired && p.playback.callbacks >= 4
            })
        })
    }
    pub fn has_visible_timed_effect(&self) -> bool {
        mixed::finite_capture_ready(self)
            || self.instances.iter().any(|i| {
                !i.parts.is_empty()
                    && i.timed
                        .as_ref()
                        .is_some_and(|p| p.visible() && p.transparency() < 1.)
            })
    }
    /// Snapshot an active declared attachment loop without waiting for the
    /// director endpoint or changing its clock/camera/visibility.
    pub fn attachment_loop_capture_ready(
        &self,
        cinematic: &crate::cinematic_camera::Cinematic,
    ) -> bool {
        self.instances.iter().any(|i| {
            matches!(&i.source, Source::Actor(r) if cinematic.actor_render_visible(r.slot) == Some(true))
                && !i.parts.is_empty()
                && i.visible(cinematic)
                && i.animation.as_ref().is_some_and(|a| {
                    a.updates >= 2 && a.clock.seconds >= 0.05_f32.min(a.duration * 0.25)
                })
        })
    }
    pub fn needs_cube(&self) -> bool {
        self.any(|s| {
            s.instances
                .iter()
                .any(|i| i.parts.iter().any(|(_, p)| matches!(p, Part::Cube { .. })))
        })
    }
    pub fn load(
        root: &std::path::Path,
        cinematic: &crate::cinematic_camera::Cinematic,
        slot: Option<usize>,
        resources: &giants_game::scene_resources::SceneResources,
        actors: &crate::actor_scene::Scene,
        owners: &crate::cinematic_scene::AttachmentOwners,
        config: &giants_formats::world::WorldConfig,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let lights = giants_game::lighting::first_world(config)?;
        let fog = config.fog;
        let lights = Arc::new(lights);
        let effects = giants_formats::effect_image::parse(&std::fs::read(
            root.join("Bin/fx_world_dust.bin"),
        )?)?;
        let mut actor_factory = giants_game::actor_links::Factory::new(resources, &effects);
        let mut index = 0;
        Self::prepare(cinematic, |segment| {
            let bytes =
                std::fs::read(giants_game::flick::sequence_path(root, segment.sequence())?)?;
            let constructed = Arc::new(actor_factory.project(&bytes)?);
            let requests = actors
                .entry(index)
                .ok_or("body dependency segment absent")?
                .attachment_requests
                .clone();
            let director = index;
            index += 1;
            let mut instances = Vec::new();
            let mut emitters = Vec::new();
            let mut mixed = Vec::new();
            let mut context_images = BTreeMap::new();
            for mut request in requests {
                if slot.is_some_and(|s| s != request.slot) {
                    continue;
                }
                let owner = owners.owner(director, request.slot);
                println!(
                    "cinematic_attachment_consumer sequence={} director={} actor_slot={} class={} prepared_owner={}",
                    segment.sequence(),
                    director,
                    request.slot,
                    request.class,
                    owner.unwrap_or("common")
                );
                if owner.is_some() {
                    println!(
                        "cinematic_rigid_attachment_deferred sequence={} actor_slot={} class={} reason=existing_specialized_consumer",
                        segment.sequence(),
                        request.slot,
                        request.class
                    );
                    continue;
                }
                let result = (|| -> Result<Instance, String> {
                    if !segment.preview() {
                        return Err(
                            "common rigid rendering requires explicit partial preview".into()
                        );
                    }
                    let initial_actor = constructed
                        .actors
                        .get(&request.slot)
                        .ok_or("constructed attachment actor absent")?;
                    if request.resolve_initial_child_handle(initial_actor)? {
                        println!(
                            "cinematic_constructed_child_handle sequence={} actor_slot={} class={} channel=254 child_handle=null profile=source_initial_actor live=false",
                            segment.sequence(),
                            request.slot,
                            request.class
                        );
                    }
                    request.matrix(segment.animation(), segment.tick())?;
                    let asset = request.asset.as_ref().map_err(Clone::clone)?;
                    let program = resources
                        .classes
                        .models
                        .first_program(request.class)
                        .ok_or("class program missing")?
                        .program
                        .as_ref()
                        .map_err(Clone::clone)?;
                    let facing = giants_game::attachment_pose::InitialFacing::project_constructed(
                        program,
                        initial_actor,
                        asset.animation.is_some(),
                    )?;
                    if facing.is_some()
                        && asset.animation.is_some()
                        && constructed.unexecuted.iter().any(|(_, op)| *op == 33)
                    {
                        return Err(
                            "animated facing fields require live offset/scale inputs".into()
                        );
                    }
                    if let Some(facing) = facing {
                        println!(
                            "cinematic_rigid_facing_fields sequence={} actor_slot={} personality={} scale_bits={} angle_bits=0 profile=constructed_initial_fields live=false",
                            segment.sequence(),
                            request.slot,
                            facing.personality,
                            facing.scale.to_bits()
                        );
                    }
                    let actor = constructed
                        .actors
                        .get(&request.slot)
                        .ok_or("constructed attachment actor absent")?;
                    if actor.class.declaration_class != request.class {
                        return Err("constructed attachment class differs".into());
                    }
                    let geometry = Arc::new(
                        giants_game::attachment_geometry::prepare_actor_initial(asset, actor)?,
                    );
                    let animation = if facing.is_some() && asset.animation.is_some() {
                        let playback = giants_game::effect_animation::Playback::new(
                            giants_game::actor_context::Prepared::class_initial(
                                actor.class.clone(),
                            ),
                        )?
                        .ok_or("constructed facing loop absent")?;
                        println!(
                            "cinematic_attachment_loop_prepared sequence={} actor_slot={} class={} initial_fields=true native_cadence_verified=false",
                            segment.sequence(),
                            request.slot,
                            request.class
                        );
                        Some(playback)
                    } else {
                        let playback =
                            giants_game::attachment_animation::initial_vertex_loop(actor);
                        match playback {
                            Ok(playback) => {
                                if playback.is_some() {
                                    println!(
                                        "cinematic_attachment_loop_prepared sequence={} actor_slot={} class={} initial_fields=true native_cadence_verified=false actor_services_verified=false",
                                        segment.sequence(),
                                        request.slot,
                                        request.class
                                    );
                                }
                                playback
                            }
                            Err(reason) => {
                                println!(
                                    "cinematic_attachment_loop_deferred sequence={} actor_slot={} class={} static_preview=true reason={reason}",
                                    segment.sequence(),
                                    request.slot,
                                    request.class
                                );
                                None
                            }
                        }
                    };
                    println!(
                        "cinematic_rigid_actor_context sequence={} actor_slot={} class={} channel={} profile={} own_seconds=0 live=false",
                        segment.sequence(),
                        request.slot,
                        request.class,
                        actor.channels[0][0],
                        geometry.profile
                    );
                    if let Some(skin) = &geometry.skin {
                        println!(
                            "cinematic_rigid_geometry sequence={} class={} profile={} file={:?} member={:?} own_tick=0",
                            segment.sequence(),
                            request.class,
                            geometry.profile,
                            skin.archive
                                .file_name()
                                .ok_or("skin archive filename absent")?
                                .to_string_lossy(),
                            skin.member
                        );
                    }
                    let normals = if asset.model.mesh.normal_count != 0 {
                        if facing.is_some() {
                            return Err("normal attachment camera-facing outside shared material capability".into());
                        }
                        let profile =
                            giants_game::body_pipeline::Normals::select(false, true, false, true);
                        let flags = asset
                            .model
                            .mesh
                            .subobjects
                            .iter()
                            .fold(0, |flags, p| flags | p.material.flags);
                        Some(
                            giants_game::body_pipeline::prepare_normals(
                                &asset.model.mesh,
                                &geometry.positions,
                                profile.contextual_preparation(flags),
                            )
                            .map_err(|e| e.to_string())?,
                        )
                    } else {
                        None
                    };
                    let parts=asset.model.mesh.subobjects.iter().enumerate().filter_map(|(i,p)| {
                        let alpha=asset.model.alpha_textures.contains(&p.material.texture.to_ascii_lowercase());
                        let result=if normals.is_some() {
                            giants_game::body_pipeline::preview_surface(giants_game::body_pipeline::Normals::select(false,true,false,true),&p.material,alpha).and_then(|surface|match surface {
                                giants_game::body_pipeline::Surface::Lit=>Ok(Part::Lit{alpha,additive:None}),
                                surface if surface.lit_additive().is_some()=>Ok(Part::Lit{alpha:false,additive:surface.lit_additive()}),
                                giants_game::body_pipeline::Surface::Context=>{
                                    let key=(request.class,p.material.bump_texture.to_ascii_lowercase());
                                    if let std::collections::btree_map::Entry::Vacant(entry)=context_images.entry(key) {
                                        let image=resources.image_request(&p.material.bump_texture);
                                        let source=image.selected.as_ref().ok_or("attachment context image source missing")?;
                                        println!("cinematic_rigid_context_input sequence={} actor_slot={} class={} file={:?} member={:?}",segment.sequence(),request.slot,request.class,source.archive.file_name().unwrap_or_default(),source.member);
                                        let giants_formats::tga::Image::Plane8(plane)=image.decoded.map_err(|_|"attachment context image decode failed")? else {return Err("attachment context image requires plane8")};
                                        entry.insert(giants_game::bump_image::normal_map(&plane,true)?);
                                    }
                                    crate::body_context::factor(&p.material,&lights[0])?;
                                    let (m,f)=request.matrix(segment.animation(),segment.tick()).map_err(|_|"attachment context matrix unavailable")?;
                                    let direction=giants_game::body_pipeline::local_context_direction(m,f,lights[0].direction)?;
                                    crate::body_context::mesh_colors(&asset.model.mesh,&geometry.positions,normals.as_ref().expect("context normals"),i,direction)?;
                                    Ok(Part::Context)
                                },
                                giants_game::body_pipeline::Surface::Cube{factor}=>Ok(Part::Cube{factor,blend:0}),
                                giants_game::body_pipeline::Surface::AdditiveCube{factor}=>Ok(Part::Cube{factor,blend:1}),
                                giants_game::body_pipeline::Surface::AlphaCube{factor}=>Ok(Part::Cube{factor,blend:2}),
                                _=>Err("normal attachment context capability not connected"),
                            })
                        } else {giants_game::unlit_material::prepare_with_texture_alpha(&p.material,alpha).map(Part::Unlit)};
                        match result {Ok(profile)=>{
                            let surface=match profile {Part::Unlit(_)=>"unlit",Part::Lit{..}=>"lit",Part::Context=>"context",Part::Cube{blend:1,..}=>"additive_cube",Part::Cube{blend:2,..}=>"alpha_cube",Part::Cube{..}=>"cube"};
                            println!("cinematic_rigid_part_capability sequence={} actor_slot={} part={} surface={} normals={}",segment.sequence(),request.slot,i,surface,normals.is_some());
                            Some((i,profile))},Err(error)=>{println!("cinematic_rigid_part_deferred sequence={} actor_slot={} part={} reason={:?}",segment.sequence(),request.slot,i,error);None}}
                    }).collect::<Vec<_>>();
                    if parts.is_empty() {
                        return Err("no supported unlit material parts".into());
                    }
                    Ok(Instance {
                        source: Source::Actor(request.clone()),
                        geometry,
                        parts,
                        normals,
                        facing,
                        animation,
                        timed: None,
                        released: None,
                        projectile: None,
                        burst: None,
                        emitted: None,
                        retirement_child: None,
                        canonical_only: false,
                        mixed: None,
                        detached: false,
                        creation_position: None,
                        persistent_attachment: false,
                    })
                })();
                match result {
                    Ok(instance) => {
                        println!(
                            "cinematic_rigid_attachment_prepared sequence={} actor_slot={} class={} parts={} geometry={} own_tick=0",
                            segment.sequence(),
                            request.slot,
                            request.class,
                            instance.parts.len(),
                            instance.geometry.profile
                        );
                        instances.push(instance);
                    }
                    Err(error) => println!(
                        "cinematic_rigid_attachment_deferred sequence={} actor_slot={} class={} reason={:?}",
                        segment.sequence(),
                        request.slot,
                        request.class,
                        error
                    ),
                }
            }
            let mut registered = giants_game::attachment_resources::Registered::new(resources);
            for request in giants_formats::flick::effect_commands(&bytes)? {
                let result = (|| -> Result<Vec<Instance>, String> {
                    if !segment.preview() || slot.is_some() || request.slot < 0 {
                        return Err(
                            "FX inspection requires partial preview and no actor filter".into()
                        );
                    }
                    if constructed.unexecuted.iter().any(|(_, op)| *op == 33) {
                        return Err(
                            "persistent FX actor offsets require live transform inputs".into()
                        );
                    }
                    let name = request
                        .name
                        .as_deref()
                        .ok_or("numeric FX selection unresolved")?;
                    let (_, entry) = effects.find(name).ok_or("effect symbol absent")?;
                    let program = effects.program(entry.offset).map_err(|e| e.to_string())?;
                    if request.mode == 1
                        && let Ok(objects) = giants_game::effect_composition::project(&program)
                    {
                        let mut prepared = objects
                            .iter()
                            .map(|object| {
                                composite_instance(
                                    object,
                                    &request,
                                    constructed.clone(),
                                    &effects,
                                    resources,
                                    &mut registered,
                                )
                            })
                            .collect::<Result<Vec<_>, _>>()?;
                        for (index, object) in objects.iter().enumerate() {
                            println!(
                                "cinematic_composite_object_prepared sequence={} offset={} object={} class={} detached={} live_callback=false native_raster_verified=false",
                                segment.sequence(),
                                request.offset,
                                index,
                                object.timed.class,
                                !object.attached
                            );
                            if object.hidden {
                                println!(
                                    "cinematic_composite_hidden_support_prepared sequence={} offset={} object={} class={} drawable_parts=0",
                                    segment.sequence(),
                                    request.offset,
                                    index,
                                    object.timed.class
                                );
                            }
                            for child in &object.immediate {
                                match projectile_instance(
                                    object,
                                    child,
                                    &request,
                                    constructed.clone(),
                                    &effects,
                                    resources,
                                    &mut registered,
                                ) {
                                    Ok(instance) => {
                                        let recipe = &instance
                                            .projectile
                                            .as_ref()
                                            .expect("prepared projectile")
                                            .playback
                                            .recipe;
                                        println!(
                                            "cinematic_projectile_effect_prepared sequence={} offset={} object={} child={:?} class={} attached_children={} retirement_child={:?} contacts_pending=true native_cadence_verified=false native_raster_verified=false",
                                            segment.sequence(),
                                            request.offset,
                                            index,
                                            child.name,
                                            recipe.class,
                                            recipe.attached_children.len(),
                                            recipe.retirement_child
                                        );
                                        println!(
                                            "cinematic_projectile_profile sequence={} offset={} class={} lifetime_bits={} tracer_pending={}",
                                            segment.sequence(),
                                            request.offset,
                                            recipe.class,
                                            recipe.visual.map_or(0, |v| v.lifetime.to_bits()),
                                            recipe.tracer.is_some()
                                        );
                                        prepared.push(instance);
                                    }
                                    Err(error) => println!(
                                        "cinematic_effect_child_deferred sequence={} offset={} object={} child={:?} reference={} operand={} reason={error:?}",
                                        segment.sequence(),
                                        request.offset,
                                        index,
                                        child.name,
                                        child.reference,
                                        child.operand
                                    ),
                                }
                            }
                        }
                        if objects.iter().any(|o| !o.immediate.is_empty()) {
                            println!(
                                "cinematic_effect_inspection_deferred sequence={} offset={} reason=\"composite projectile contacts, attached trails and retirement incomplete; visual objects prepared\"",
                                segment.sequence(),
                                request.offset
                            );
                        }
                        return Ok(prepared);
                    }
                    if request.mode == 0
                        && let Ok(recipe) =
                            giants_game::effect_emitter::Recipe::project(&effects, name)
                    {
                        let duration = giants_game::animation_template::duration(
                            segment.animation().timeline,
                            giants_game::arithmetic::Precision::Double53,
                        )
                        .map_err(|e| e.to_string())?;
                        let reserve = recipe.reserve(duration).map_err(str::to_owned)?;
                        let (emitter, prepared) = periodic_instances(
                            recipe,
                            &request,
                            constructed.clone(),
                            &effects,
                            resources,
                            &mut registered,
                            (emitters.len(), reserve),
                        )?;
                        println!(
                            "cinematic_periodic_effect_prepared sequence={} offset={} descendants={} reserve_per_descendant={} drawable_slots={} request_satisfied=false native_cadence_verified=false native_raster_verified=false",
                            segment.sequence(),
                            request.offset,
                            emitter.recipe.particles.len(),
                            reserve,
                            prepared.len()
                        );
                        println!(
                            "cinematic_effect_inspection_deferred sequence={} offset={} reason=\"periodic visual preview prepared; native scheduling and cross-director ownership incomplete\"",
                            segment.sequence(),
                            request.offset
                        );
                        emitters.push(emitter);
                        return Ok(prepared);
                    }
                    if request.mode == 0
                        && let Ok(plan) = giants_game::effect_burst::Plan::project(&effects, name)
                    {
                        let mut seed = 1;
                        let spawns = plan.create(&mut seed).map_err(str::to_owned)?;
                        let prepared = burst_instances(
                            plan,
                            &request,
                            constructed.clone(),
                            &effects,
                            resources,
                            &mut registered,
                        )?;
                        println!(
                            "cinematic_burst_effect_prepared sequence={} offset={} spawns={} supports={} drawable={} sound_pending={} texture_animation_pending={} contacts_pending=true density_policy=full_detail_preview native_cadence_verified=false native_raster_verified=false",
                            segment.sequence(),
                            request.offset,
                            spawns.len(),
                            spawns.iter().filter(|s| s.hidden).count(),
                            prepared.len(),
                            spawns.iter().map(|s| s.sounds.len()).sum::<usize>(),
                            spawns
                                .iter()
                                .filter(|s| s.texture_animation.is_some())
                                .count()
                                .saturating_sub(
                                    prepared
                                        .iter()
                                        .filter(|i| i
                                            .burst
                                            .as_ref()
                                            .is_some_and(|b| b.texture_frames.is_some()))
                                        .count()
                                )
                        );
                        println!(
                            "cinematic_effect_inspection_deferred sequence={} offset={} reason=\"burst contacts, sounds, native texture upload, cadence and density policy incomplete; visual objects prepared\"",
                            segment.sequence(),
                            request.offset
                        );
                        return Ok(prepared);
                    }
                    if let Ok(recipe) = giants_game::released_effect::Recipe::project(&program) {
                        let source = Source::Effect {
                            request: request.clone(),
                            asset: registered.asset(recipe.class)?,
                            actors: constructed.clone(),
                            class: recipe.class,
                        };
                        let instance =
                            particle_instance(source, recipe, None, &effects, resources)?;
                        println!(
                            "cinematic_particle_effect_prepared sequence={} offset={} class={} detached=true native_cadence_verified=false",
                            segment.sequence(),
                            request.offset,
                            instance.source.class()
                        );
                        return Ok(vec![instance]);
                    }
                    if request.mode != 1 {
                        return Err("persistent FX inspection requires attached mode".into());
                    }
                    let (recipe, timed) =
                        match giants_game::effect_attachment::Recipe::project(&program) {
                            Ok(recipe) => (recipe, None),
                            Err(_) => {
                                let timed = giants_game::timed_effect::Recipe::project(&program)
                                    .map_err(str::to_owned)?;
                                let recipe = giants_game::effect_attachment::Recipe {
                                    class: timed.class,
                                    opaque: timed.opaque,
                                    scale: 1.,
                                };
                                if effects.find(&timed.child).is_none() {
                                    return Err("retirement child effect symbol absent".into());
                                }
                                (
                                    recipe,
                                    Some(giants_game::timed_effect::Playback::new(timed)),
                                )
                            }
                        };
                    let asset = registered.asset(recipe.class)?;
                    let class_program = resources
                        .classes
                        .models
                        .first_program(recipe.class)
                        .ok_or("effect class absent")?
                        .program
                        .as_ref()
                        .map_err(Clone::clone)?;
                    let facing = if let Some(timed) = &timed {
                        timed
                            .recipe
                            .validate_class(class_program)
                            .map_err(str::to_owned)?;
                        None
                    } else {
                        Some(recipe.facing(class_program).map_err(str::to_owned)?)
                    };
                    if asset.model.mesh.normal_count != 0 {
                        return Err("persistent FX inspection requires unlit geometry".into());
                    }
                    let parts = asset
                        .model
                        .mesh
                        .subobjects
                        .iter()
                        .enumerate()
                        .map(|(i, p)| {
                            let alpha = asset
                                .model
                                .alpha_textures
                                .contains(&p.material.texture.to_ascii_lowercase());
                            giants_game::unlit_material::prepare_with_texture_alpha(
                                &p.material,
                                alpha,
                            )
                            .map(|p| (i, Part::Unlit(p)))
                            .map_err(str::to_owned)
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    if parts.is_empty() {
                        return Err("effect has no drawable parts".into());
                    }
                    let actor = giants_game::actor_context::Prepared::class_initial(Arc::new(
                        giants_game::actor_context::Class::load(resources, recipe.class, &effects)?,
                    ));
                    let geometry = Arc::new(
                        giants_game::attachment_geometry::prepare_actor_initial(&asset, &actor)?,
                    );
                    let animation = giants_game::effect_animation::Playback::new(actor)?;
                    let source = Source::Effect {
                        request: request.clone(),
                        asset,
                        actors: constructed.clone(),
                        class: recipe.class,
                    };
                    source.matrix(segment.animation(), segment.tick())?;
                    println!(
                        "cinematic_effect_inspection_prepared sequence={} offset={} class={} opaque={} scale_bits={} lifecycle={} own_animation={} cull={} live_callback=false native_raster_verified=false",
                        segment.sequence(),
                        request.offset,
                        recipe.class,
                        recipe.opaque,
                        recipe.scale.to_bits(),
                        if timed.is_some() {
                            "timed_viewer"
                        } else {
                            "segment"
                        },
                        if animation.is_some() {
                            "viewer_clock"
                        } else {
                            "initial_frame"
                        },
                        if timed.is_some() {
                            "wrapper_fields"
                        } else {
                            "wrapper_cw"
                        }
                    );
                    Ok(vec![Instance {
                        source,
                        geometry,
                        parts,
                        normals: None,
                        facing,
                        animation,
                        persistent_attachment: timed.is_none(),
                        timed,
                        released: None,
                        projectile: None,
                        burst: None,
                        emitted: None,
                        retirement_child: None,
                        canonical_only: false,
                        mixed: None,
                        detached: false,
                        creation_position: None,
                    }])
                })();
                match result {
                    Ok(prepared) => {
                        for mut instance in prepared {
                            let child = if instance
                                .timed
                                .as_ref()
                                .is_some_and(|p| !p.recipe.child.is_empty())
                            {
                                match released_instance(
                                    &instance,
                                    instances.len(),
                                    &effects,
                                    resources,
                                    &mut registered,
                                ) {
                                    Ok(child) => {
                                        instance.retirement_child = Some(instances.len() + 1);
                                        println!(
                                            "cinematic_released_effect_prepared sequence={} offset={} class={} lifecycle=retirement viewer_schedule=true native_cadence_verified=false",
                                            segment.sequence(),
                                            request.offset,
                                            child.source.class()
                                        );
                                        Some(child)
                                    }
                                    Err(error) => {
                                        println!(
                                            "cinematic_released_effect_deferred sequence={} offset={} reason={error:?}",
                                            segment.sequence(),
                                            request.offset
                                        );
                                        None
                                    }
                                }
                            } else {
                                None
                            };
                            instances.push(instance);
                            if let Some(child) = child {
                                instances.push(child);
                            }
                        }
                    }
                    Err(error) => {
                        let preparation = if segment.preview()
                            && slot.is_none()
                            && request.slot >= 0
                            && !constructed.unexecuted.iter().any(|(_, op)| *op == 33)
                        {
                            mixed::prepare(
                                &request,
                                constructed.clone(),
                                &effects,
                                resources,
                                &config.prefix.terrain,
                                &mut registered,
                                mixed.len(),
                                instances.len(),
                                emitters.len(),
                                segment.sequence(),
                            )
                        } else {
                            Err("mixed preview requires supported partial source context".into())
                        };
                        match preparation {
                            Ok((groups, objects, services)) => {
                                mixed.extend(groups);
                                instances.extend(objects);
                                emitters.extend(services);
                            }
                            Err(mixed_error) => println!(
                                "cinematic_effect_inspection_deferred sequence={} offset={} reason={:?} mixed_reason={:?}",
                                segment.sequence(),
                                request.offset,
                                error,
                                mixed_error
                            ),
                        }
                    }
                }
            }
            if let Ok(bindings) = giants_game::cinematic_timed::Bindings::load(resources, &bytes) {
                for request in giants_formats::flick::effect_commands(&bytes)? {
                    if let Some(class) = bindings.contact_child_class(request.offset) {
                        let program = resources
                            .classes
                            .models
                            .first_program(class)
                            .ok_or("canonical contact-child class absent")?
                            .program
                            .as_ref()
                            .map_err(Clone::clone)?;
                        let facing =
                            giants_game::released_effect::state78_burst_facing(class, program)?;
                        let mut child = unlit_effect_instance(
                            class,
                            facing,
                            &request,
                            constructed.clone(),
                            &effects,
                            resources,
                            &mut registered,
                        )?;
                        child.canonical_only = true;
                        child.detached = true;
                        println!(
                            "cinematic_contact_child_geometry sequence={} offset={} class={class} canonical_only=true body_born=false",
                            segment.sequence(),
                            request.offset
                        );
                        instances.push(child);
                    }
                }
            }
            let mut phased = Vec::new();
            let original_count = instances.len();
            for parent in 0..original_count {
                if !instances[parent].projectile.as_ref().is_some_and(|p| {
                    (p.playback.recipe.visual.is_some() || p.playback.recipe.contact_requested)
                        && !p.playback.recipe.retirement_child.is_empty()
                }) {
                    continue;
                }
                match prepare_retirement(
                    parent,
                    instances.len(),
                    emitters.len(),
                    &instances[parent],
                    &effects,
                    resources,
                    &mut registered,
                ) {
                    Ok((effect, mut objects, services)) => {
                        let group = phased.len();
                        for object in &mut objects {
                            if let Some(burst) = &mut object.burst {
                                let (_, phase) = burst.phase.expect("retirement phase");
                                burst.phase = Some((group, phase));
                            }
                        }
                        println!(
                            "cinematic_retirement_prepared sequence={} parent={} phases={} drawable_slots={} child_emitters={} density_policy=full_detail_preview native_density_verified=false request_satisfied=false offset={}",
                            segment.sequence(),
                            parent,
                            effect.sequence.phases.len(),
                            objects.len(),
                            services.len(),
                            match &instances[parent].source {
                                Source::Effect { request, .. } => request.offset,
                                _ => unreachable!(),
                            }
                        );
                        instances.extend(objects);
                        emitters.extend(services);
                        phased.push(effect);
                    }
                    Err(error) => {
                        let Source::Effect {
                            request, actors, ..
                        } = &instances[parent].source
                        else {
                            unreachable!()
                        };
                        let mut child_request = request.clone();
                        child_request.name = Some(
                            instances[parent]
                                .projectile
                                .as_ref()
                                .expect("projectile")
                                .playback
                                .recipe
                                .retirement_child
                                .clone(),
                        );
                        let child_group = mixed.len();
                        let prepared = mixed::prepare(
                            &child_request,
                            actors.clone(),
                            &effects,
                            resources,
                            &config.prefix.terrain,
                            &mut registered,
                            child_group,
                            instances.len(),
                            emitters.len(),
                            segment.sequence(),
                        )
                        .and_then(|(mut groups, objects, services)| {
                            mixed::wait_for_trigger(&mut groups)?;
                            Ok((groups, objects, services))
                        });
                        match prepared {
                            Ok((groups, objects, services)) => {
                                println!(
                                    "cinematic_projectile_mixed_retirement_prepared sequence={} parent={parent} group={child_group} offset={} effect={:?} request_satisfied=false native_cadence_verified=false",
                                    segment.sequence(),
                                    request.offset,
                                    child_request.name
                                );
                                instances[parent]
                                    .projectile
                                    .as_mut()
                                    .expect("projectile")
                                    .retirement_group = Some(child_group);
                                mixed.extend(groups);
                                instances.extend(objects);
                                emitters.extend(services);
                            }
                            Err(mixed_error) => println!(
                                "cinematic_retirement_deferred sequence={} parent={} reason={error:?} mixed_reason={mixed_error:?}",
                                segment.sequence(),
                                parent
                            ),
                        }
                    }
                }
            }
            Ok(Prepared {
                instances,
                draws: Vec::new(),
                motion: Vec::new(),
                fog,
                lights: lights.clone(),
                tick: segment.tick(),
                generation: 0,
                effect_seed: 1,
                hidden_sound_owners: burst_owners::Owners::default(),
                tracers: tracer::State::default(),
                emitters,
                phased,
                mixed,
                context_images,
            })
        })
    }
}
#[derive(Component)]
pub struct Rendered;
#[derive(SystemParam)]
pub struct Assets<'w> {
    meshes: ResMut<'w, bevy::prelude::Assets<Mesh>>,
    images: ResMut<'w, bevy::prelude::Assets<Image>>,
    materials: ResMut<'w, bevy::prelude::Assets<Material>>,
    lit_materials: ResMut<'w, bevy::prelude::Assets<crate::actor_scene::LitMaterial>>,
    cube_materials: Option<ResMut<'w, bevy::prelude::Assets<crate::sword_scene::Material>>>,
    buffers: ResMut<'w, bevy::prelude::Assets<bevy::render::storage::ShaderBuffer>>,
    context_materials: ResMut<'w, bevy::prelude::Assets<crate::body_context::Material>>,
}
fn pixel(
    camera: &crate::validation_camera::ValidationCamera,
    fog: giants_game::lighting::Fog,
) -> Vec4 {
    Vec4::new(
        1. / camera.viewport.z as f32,
        -1. / camera.viewport.w as f32,
        fog.start,
        fog.end,
    )
}
pub fn setup(
    mut commands: Commands,
    mut scene: ResMut<Scene>,
    world: Res<crate::world_scene::Scene>,
    cinematic: Res<crate::cinematic_camera::Cinematic>,
    view: Res<crate::source_view::SourceView>,
    mut assets: Assets,
    cube: Option<Res<crate::reflection_cube::Build>>,
) {
    for instance in &mut scene.instances {
        if let Some(animation) = &mut instance.animation {
            animation.reset();
        }
    }
    scene.effect_seed = 1;
    scene.hidden_sound_owners.clear();
    mixed::reset(&mut scene);
    if scene.tracers.prepare(cinematic.generation())
        && let Some(entity) = scene.tracers.entity()
    {
        commands.entity(entity).insert(Visibility::Hidden);
    }
    for emitter in &mut scene.emitters {
        emitter.runtime = None;
        if let Some(reference) = &mut emitter.reference {
            reference.channel = None;
            reference.parent = None;
        }
    }
    for instance in &mut scene.instances {
        if let Some(timed) = &mut instance.timed {
            timed.reset();
        }
        if let Some(projectile) = &mut instance.projectile {
            projectile.playback.reset();
        }
        if let Some(burst) = &mut instance.burst {
            burst.playback = None;
        }
        if let Some(emitted) = &mut instance.emitted {
            emitted.presentation = None;
            emitted.parent = None;
        }
        if let Some(released) = &mut instance.released {
            released.playback.reset();
            released.last_route_mask = None;
        }
    }
    scene.generation = cinematic.generation();
    let camera = world
        .validation_camera
        .as_ref()
        .expect("rigid attachments need source camera");
    let mut textures = BTreeMap::new();
    let mut context_textures = BTreeMap::new();
    for (key, source) in &scene.context_images {
        let rgba = source
            .pixels
            .as_chunks::<3>()
            .0
            .iter()
            .zip(source.alpha.as_ref().expect("normal image alpha"))
            .flat_map(|(rgb, a)| [rgb[0], rgb[1], rgb[2], *a])
            .collect();
        let mut image = Image::new(
            bevy::render::render_resource::Extent3d {
                width: source.width,
                height: source.height,
                depth_or_array_layers: 1,
            },
            bevy::render::render_resource::TextureDimension::D2,
            rgba,
            bevy::render::render_resource::TextureFormat::Rgba8Unorm,
            RenderAssetUsages::default(),
        );
        image.sampler = crate::logo_material::sampler();
        context_textures.insert(key.clone(), assets.images.add(image));
    }
    let mut draws = Vec::new();
    let mut motion = Vec::new();
    for (i, instance) in scene.instances.iter().enumerate() {
        let asset = instance.source.asset();
        let (source_matrix, front) = instance
            .render_matrix(cinematic.animation(), cinematic.tick(), camera.view)
            .expect("prepared binding");
        let matrix = world_matrix(source_matrix);
        let source_world = Mat4::from_cols_array(&source_matrix).transpose();
        let buffer = instance
            .parts
            .iter()
            .any(|(_, p)| matches!(p, Part::Unlit(_)))
            .then(|| {
                assets
                    .buffers
                    .add(crate::source_view::uniform_buffer(&matrix))
            });
        motion.push(Motion {
            instance: i,
            buffer: buffer.clone(),
            matrix,
        });
        for texture in asset.model.textures.iter().chain(
            instance
                .burst
                .as_ref()
                .and_then(|b| b.texture_frames.as_deref())
                .into_iter()
                .flatten(),
        ) {
            let key = (instance.source.class(), texture.name.to_ascii_lowercase());
            if textures.contains_key(&key) {
                continue;
            }
            let mut image = Image::new(
                bevy::render::render_resource::Extent3d {
                    width: texture.width,
                    height: texture.height,
                    depth_or_array_layers: 1,
                },
                bevy::render::render_resource::TextureDimension::D2,
                texture.rgba.clone(),
                bevy::render::render_resource::TextureFormat::Rgba8Unorm,
                RenderAssetUsages::default(),
            );
            image.sampler = crate::logo_material::sampler();
            textures.insert(key, assets.images.add(image));
        }
        for (part, profile) in &instance.parts {
            let sub = &asset.model.mesh.subobjects[*part];
            let indices: Vec<_> = sub
                .triangles
                .iter()
                .flatten()
                .map(|v| usize::from(*v))
                .collect();
            let positions: Vec<_> = indices
                .iter()
                .map(|v| {
                    let p = instance.geometry.positions[usize::from(asset.model.mesh.indices[*v])];
                    if instance.normals.is_some() {
                        p
                    } else {
                        [p[0], p[2], -p[1]]
                    }
                })
                .collect();
            let uv: Vec<_> = indices.iter().map(|v| asset.model.mesh.uv[*v]).collect();
            let mut colors: Vec<_> = indices
                .iter()
                .map(|v| {
                    let c = asset.model.mesh.rgb[*v];
                    [
                        c[0] as f32 / 255.,
                        c[1] as f32 / 255.,
                        c[2] as f32 / 255.,
                        1.,
                    ]
                })
                .collect();
            if matches!(profile, Part::Context) {
                colors = context_colors(instance, *part, &cinematic, &scene.lights[0])
                    .expect("prepared context colors");
            }
            let mut mesh = Mesh::new(
                PrimitiveTopology::TriangleList,
                RenderAssetUsages::default(),
            )
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
            .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors);
            if let Some(normals) = &instance.normals {
                mesh.insert_attribute(
                    Mesh::ATTRIBUTE_NORMAL,
                    indices.iter().map(|v| normals[*v]).collect::<Vec<_>>(),
                );
            }
            let texture_frames = instance
                .burst
                .as_ref()
                .and_then(|b| b.texture_frames.as_ref())
                .map(|frames| {
                    frames
                        .iter()
                        .map(|frame| {
                            textures[&(instance.source.class(), frame.name.to_ascii_lowercase())]
                                .clone()
                        })
                        .collect::<Vec<_>>()
                });
            let texture = texture_frames
                .as_ref()
                .map(|frames| frames[0].clone())
                .unwrap_or_else(|| {
                    textures[&(
                        instance.source.class(),
                        sub.material.texture.to_ascii_lowercase(),
                    )]
                        .clone()
                });
            let mesh = assets.meshes.add(mesh);
            let entity = commands
                .spawn((
                    Rendered,
                    if instance.visible(&cinematic) {
                        Visibility::Inherited
                    } else {
                        Visibility::Hidden
                    },
                    Name::new(format!(
                        "FLK common rigid {} / {}",
                        instance.source.slot(),
                        sub.name
                    )),
                    Mesh3d(mesh.clone()),
                    Transform::from_matrix(matrix),
                    bevy::camera::visibility::NoFrustumCulling,
                ))
                .id();
            if matches!(instance.source, Source::Actor(_)) {
                commands
                    .entity(entity)
                    .insert(crate::cinematic_visibility::ActorSlot(
                        instance.source.slot(),
                    ));
            }
            let material = match profile {
                Part::Unlit(profile) => {
                    let material = assets.materials.add(Material::from(Bindings {
                        tracer: false,
                        front,
                        blend: profile.blend,
                        texture,
                        clip: view.buffer.clone(),
                        world: buffer.as_ref().expect("unlit instance matrix").clone(),
                        pixel_fog: pixel(camera, scene.fog),
                        color_alpha: Vec3::from_array(scene.fog.color.map(|c| c as f32 / 255.))
                            .extend(profile.alpha),
                        flags: Vec4::new(
                            if profile.add_vertex_rgb {
                                2.
                            } else if profile.vertex_rgb {
                                1.
                            } else {
                                0.
                            },
                            if world.no_fog { 0. } else { 1. },
                            if profile.texture_alpha { 1. } else { 0. },
                            if instance.has_vertical_vertex_alpha() {
                                1.
                            } else {
                                0.
                            },
                        ),
                    }));
                    commands
                        .entity(entity)
                        .insert(MeshMaterial3d(material.clone()));
                    DrawMaterial::Unlit(material)
                }
                Part::Lit { .. } | Part::Context => {
                    let alpha = matches!(profile, Part::Lit { alpha: true, .. });
                    let additive = match profile {
                        Part::Lit { additive, .. } => *additive,
                        _ => None,
                    };
                    let context = matches!(profile, Part::Context);
                    let parameters = crate::gun_scene::source_parameters(
                        &sub.material,
                        source_world,
                        camera,
                        &scene.lights,
                        scene.fog,
                        world.no_fog,
                    )
                    .white_texture_factor(alpha)
                    .suppress_primary_diffuse(context)
                    .additive_profile(additive);
                    let parameters = assets.buffers.add(parameters.buffer());
                    let material = assets.lit_materials.add(
                        crate::actor_scene::shared_lit_material(
                            texture.clone(),
                            parameters.clone(),
                            alpha,
                        )
                        .with_front(front)
                        .with_additive(additive.is_some()),
                    );
                    commands
                        .entity(entity)
                        .insert(MeshMaterial3d(material.clone()));
                    DrawMaterial::Lit {
                        material,
                        parameters,
                        alpha,
                        context,
                        additive,
                    }
                }
                Part::Cube { factor, blend } => {
                    let parameters = crate::sword_scene::source_parameters(
                        &sub.material,
                        source_world,
                        camera,
                        &scene.lights,
                        scene.fog,
                        world.no_fog,
                    )
                    .cube_factor(Some(*factor))
                    .blended(*blend != 0);
                    let parameters = assets.buffers.add(parameters.buffer());
                    let material = assets
                        .cube_materials
                        .as_mut()
                        .expect("shared cube material installed")
                        .add(
                            crate::sword_scene::Material::new(
                                texture,
                                cube.as_ref()
                                    .and_then(|c| c.handle())
                                    .expect("attachment reflection cube"),
                                parameters.clone(),
                            )
                            .with_blend(*blend)
                            .with_front(front),
                        );
                    commands
                        .entity(entity)
                        .insert(MeshMaterial3d(material.clone()));
                    DrawMaterial::Cube {
                        material,
                        parameters,
                        factor: *factor,
                        blend: *blend,
                    }
                }
            };
            if matches!(profile, Part::Context) {
                let context_material =
                    assets
                        .context_materials
                        .add(crate::body_context::Material::from(
                            crate::body_context::Bindings {
                                front,
                                source_world,
                                base: textures[&(
                                    instance.source.class(),
                                    sub.material.texture.to_ascii_lowercase(),
                                )]
                                    .clone(),
                                normal: context_textures[&(
                                    instance.source.class(),
                                    sub.material.bump_texture.to_ascii_lowercase(),
                                )]
                                    .clone(),
                                clip: view.source_buffer.clone(),
                                pixel: Vec4::new(
                                    1. / camera.viewport.z as f32,
                                    -1. / camera.viewport.w as f32,
                                    0.,
                                    0.,
                                ),
                                factor: crate::body_context::factor(
                                    &sub.material,
                                    &scene.lights[0],
                                )
                                .expect("prepared context primary"),
                                fog: Vec4::new(
                                    scene.fog.start,
                                    scene.fog.end,
                                    if world.no_fog { 0. } else { 1. },
                                    0.,
                                ),
                            },
                        ));
                let overlay = commands
                    .spawn((
                        Rendered,
                        crate::cinematic_visibility::ActorSlot(instance.source.slot()),
                        Name::new(format!(
                            "FLK common context {} / {}",
                            instance.source.slot(),
                            sub.name
                        )),
                        Mesh3d(mesh.clone()),
                        MeshMaterial3d(context_material.clone()),
                        Transform::from_matrix(matrix),
                        if instance.visible(&cinematic) {
                            Visibility::Inherited
                        } else {
                            Visibility::Hidden
                        },
                        bevy::camera::visibility::NoFrustumCulling,
                    ))
                    .id();
                draws.push(Draw {
                    instance: i,
                    entity: overlay,
                    material: DrawMaterial::Context(context_material),
                    part: *part,
                    mesh: mesh.clone(),
                    texture_frames: None,
                    route: 0,
                });
                println!(
                    "cinematic_rigid_context_draw sequence={} actor_slot={} part={} local_direction=true source_matrix=true two_pass=true",
                    cinematic.sequence(),
                    instance.source.slot(),
                    part
                );
            }
            let routed = instance.persistent_attachment
                || instance
                    .released
                    .as_ref()
                    .is_some_and(|r| r.routes.is_some());
            if routed {
                let DrawMaterial::Unlit(handle) = &material else {
                    unreachable!("admitted particle material");
                };
                let extra = commands
                    .spawn((
                        Rendered,
                        Name::new("FLK ordinary effect traversal"),
                        Mesh3d(mesh.clone()),
                        MeshMaterial3d(handle.clone()),
                        Transform::from_matrix(matrix),
                        Visibility::Hidden,
                        bevy::camera::visibility::NoFrustumCulling,
                    ))
                    .id();
                draws.push(Draw {
                    instance: i,
                    entity: extra,
                    material: DrawMaterial::Unlit(handle.clone()),
                    part: *part,
                    mesh: mesh.clone(),
                    texture_frames: None,
                    route: 1,
                });
            }
            draws.push(Draw {
                instance: i,
                entity,
                material,
                part: *part,
                mesh,
                texture_frames,
                route: if routed { 2 } else { 0 },
            });
        }
    }
    println!(
        "cinematic_rigid_attachment_draws sequence={} director={} instances={} meshes={} profile=static_own_animation source_bindings=true",
        cinematic.sequence(),
        cinematic.director_index(),
        scene.instances.len(),
        draws.len()
    );
    scene.draws = draws;
    scene.motion = motion;
}
fn queue_burst_sounds(
    audio: &mut Vec<crate::cinematic_effect_audio::Request>,
    cinematic: &crate::cinematic_camera::Cinematic,
    command: usize,
    group: usize,
    phase: usize,
    spawns: &[giants_game::effect_burst::Spawn],
    origin: [f32; 3],
) {
    for (object, spawn) in spawns.iter().enumerate() {
        for (record, sound) in spawn.sounds.iter().enumerate() {
            audio.push(crate::cinematic_effect_audio::Request {
                key: crate::cinematic_effect_audio::Key {
                    family: "burst",
                    generation: cinematic.generation(),
                    director: cinematic.director_index(),
                    command,
                    group,
                    phase,
                    object,
                    record,
                },
                sequence: cinematic.sequence().into(),
                id: sound.id,
                binding: if sound.attached {
                    crate::cinematic_effect_audio::Binding::BurstObject
                } else {
                    crate::cinematic_effect_audio::Binding::UnownedPosition
                },
                position: (!sound.attached).then_some(origin),
            });
        }
    }
}
#[allow(clippy::too_many_arguments)]
pub fn update(
    mut commands: Commands,
    mut scene: ResMut<Scene>,
    world: Res<crate::world_scene::Scene>,
    cinematic: Res<crate::cinematic_camera::Cinematic>,
    mut assets: Assets,
    mut transforms: Query<(&mut Transform, &mut Visibility), With<Rendered>>,
    mut exit: MessageWriter<AppExit>,
    mut audio: Option<ResMut<crate::cinematic_effect_audio::Queue>>,
    mut last_paused_sample: Local<Option<(u64, usize, i32)>>,
    isolation: Option<Res<capture::EffectIsolation>>,
    persistent: Option<Res<crate::mission_effects::State>>,
    runtime: Option<Res<crate::mission_runtime::Runtime>>,
) {
    // Reflection preparation can run update before setup installs these draws.
    // Do not start effect lifetimes in that provisional, subsequently reset scene.
    if scene.draws.is_empty() {
        return;
    }
    if transforms.iter().count() != scene.draws.len() {
        error!("Director render entity count differs: rigid_attachment_scene");
        exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
        return;
    }
    let reset = scene.generation != cinematic.generation();
    let sample_key = (
        cinematic.generation(),
        cinematic.director_index(),
        cinematic.tick(),
    );
    let paused_sample = cinematic.paused() && *last_paused_sample != Some(sample_key);
    if paused_sample {
        *last_paused_sample = Some(sample_key);
        // The normal early return below must remain intact: observing a paused
        // state must not run constructors or callbacks again with a zero delta.
        if let Some(camera) = world.validation_camera.as_ref() {
            for draw in &scene.draws {
                let instance = &scene.instances[draw.instance];
                let (Some(mixed), Source::Effect { request, .. }) =
                    (&instance.mixed, &instance.source)
                else {
                    continue;
                };
                match instance.render_matrix(cinematic.animation(), cinematic.tick(), camera.view) {
                    Ok((matrix, _)) => println!(
                        "cinematic_mixed_paused_matrix sequence={} offset={} group={} phase={} object={} class={} part={} tick={} visible={} matrix_bits={:?} gpu_submission_verified=false native_pixels_verified=false",
                        cinematic.sequence(),
                        request.offset,
                        mixed.group,
                        mixed.phase,
                        mixed.object,
                        instance.source.class(),
                        draw.part,
                        cinematic.tick(),
                        instance.visible(&cinematic),
                        matrix.map(f32::to_bits)
                    ),
                    Err(reason) => {
                        println!("cinematic_mixed_paused_matrix_refused reason={reason:?}")
                    }
                }
            }
        }
    }
    let animation_dt = cinematic.animation_dt();
    // Some(None) means an actual canonical family owns this draw but its body
    // is not born/prepared yet. Never fall back to an independent viewer clock.
    let timed_frames = scene
        .instances
        .iter()
        .enumerate()
        .filter_map(|(index, instance)| {
            let Source::Effect { request, .. } = &instance.source else {
                return None;
            };
            let frame = persistent.as_ref().and_then(|state| {
                runtime.as_ref().and_then(|runtime| {
                    state.timed_frame(
                        runtime,
                        cinematic.director_index(),
                        request.offset,
                        instance.source.class(),
                    )
                })
            });
            frame.map(|frame| (index, frame))
        })
        .collect::<BTreeMap<_, _>>();
    let animating = animation_dt > 0.
        && scene.instances.iter().any(|i| {
            (i.animation.is_some()
                || i.timed.is_some()
                || i.projectile.is_some()
                || i.burst.is_some()
                || i.emitted.is_some()
                || i.mixed.is_some()
                || i.released.as_ref().is_some_and(|p| !p.playback.expired))
                && i.source.visible(&cinematic)
        });
    if scene.tick == cinematic.tick()
        && !world.is_changed()
        && !reset
        && !animating
        && !persistent.as_ref().is_some_and(|p| p.is_changed())
    {
        return;
    }
    if reset {
        scene.effect_seed = 1;
        scene.hidden_sound_owners.clear();
    }
    let mut seed = scene.effect_seed;
    let mut audio_requests = Vec::new();
    let canonical_offsets = timed_frames
        .keys()
        .filter_map(|&index| match &scene.instances[index].source {
            Source::Effect { request, .. } => Some(request.offset),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    if let Err(error) = mixed::update(
        &mut scene,
        &cinematic,
        reset,
        animation_dt,
        &mut seed,
        &mut audio_requests,
        &canonical_offsets,
    ) {
        error!("Mixed effect preview failed: {error}");
        exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
        return;
    }
    if let Some(audio) = audio.as_mut() {
        audio.0.extend(audio_requests);
    }
    // Retirement queues the child below. It receives its first positive viewer
    // callback on the next update; this is an explicit preview schedule.
    for (index, instance) in scene.instances.iter_mut().enumerate() {
        if timed_frames.contains_key(&index) {
            continue;
        }
        let root_reached = instance.released.as_ref().is_some_and(|r| {
            r.parent.is_none() && (reset || r.playback.visual.is_none() && !r.playback.expired)
        }) && instance.source.visible(&cinematic);
        let origin = if root_reached && animation_dt > 0. {
            match instance.source.particle_position(&cinematic) {
                Ok(position) => Some(position),
                Err(error) => {
                    error!("Particle position failed: {error}");
                    exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
                    return;
                }
            }
        } else {
            None
        };
        if let Some(released) = &mut instance.released {
            if reset {
                released.playback.reset();
                released.last_route_mask = None;
            }
            if released.parent.is_none()
                && released.playback.visual.is_none()
                && !released.playback.expired
                && let Some(position) = origin
            {
                let seed_before = seed;
                if let Err(error) = released.playback.request_with_seed(position, &mut seed) {
                    error!("Particle request failed: {error}");
                    exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
                    return;
                }
                if released.routes.is_some() {
                    let Source::Effect { request, .. } = &instance.source else {
                        unreachable!()
                    };
                    println!(
                        "cinematic_particle_seed sequence={} offset={} before={} after={} position_bits={:?} native_global_rng_verified=false",
                        cinematic.sequence(),
                        request.offset,
                        seed_before,
                        seed,
                        position.map(f32::to_bits)
                    );
                }
                println!(
                    "cinematic_particle_effect_requested sequence={} offset={} class={} position_bits={:?} detached=true native_cadence_verified=false",
                    cinematic.sequence(),
                    match &instance.source {
                        Source::Effect { request, .. } => request.offset,
                        _ => unreachable!(),
                    },
                    instance.source.class(),
                    released.playback.motion.position.map(f32::to_bits)
                );
            }
            let was_expired = released.playback.expired;
            let advanced = match released.playback.advance(animation_dt) {
                Ok(advanced) => advanced,
                Err(error) => {
                    error!("Released effect failed: {error}");
                    exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
                    return;
                }
            };
            if advanced
                && released.routes.is_some()
                && let Some(words) = released.vertical_state_words()
            {
                let Source::Effect { request, .. } = &instance.source else {
                    unreachable!()
                };
                println!(
                    "cinematic_particle_step sequence={} offset={} dt_bits={} state_words={:?} native_cadence_verified=false",
                    cinematic.sequence(),
                    request.offset,
                    animation_dt.to_bits(),
                    words
                );
            }
            if let Some(profile) = released.routes {
                let mask = profile
                    .routes(
                        released.playback.requested(),
                        released.playback.visual.is_some(),
                        released.playback.expired,
                    )
                    .mask();
                if released.last_route_mask != Some(mask) {
                    let Source::Effect { request, .. } = &instance.source else {
                        unreachable!();
                    };
                    println!(
                        "cinematic_particle_draw_routes sequence={} offset={} class={} mask={} requested={} first_callback_seen={} expired={} native_cadence_verified=false native_sort_order_verified=false",
                        cinematic.sequence(),
                        request.offset,
                        instance.source.class(),
                        mask,
                        released.playback.requested(),
                        released.playback.visual.is_some(),
                        released.playback.expired
                    );
                    released.last_route_mask = Some(mask);
                }
            }
            if !was_expired && released.playback.expired {
                if let Some(parent) = released.parent {
                    println!(
                        "cinematic_released_effect_expired sequence={} parent={} class={} native_cadence_verified=false",
                        cinematic.sequence(),
                        parent,
                        instance.source.class()
                    );
                } else {
                    println!(
                        "cinematic_particle_effect_expired sequence={} offset={} class={} native_cadence_verified=false",
                        cinematic.sequence(),
                        match &instance.source {
                            Source::Effect { request, .. } => request.offset,
                            _ => unreachable!(),
                        },
                        instance.source.class()
                    );
                }
            }
        }
    }
    let mut retirements = Vec::new();
    for (parent_index, instance) in scene.instances.iter_mut().enumerate() {
        if timed_frames.contains_key(&parent_index) {
            continue;
        }
        let reached = instance.source.visible(&cinematic);
        if reset {
            instance.creation_position = None;
        }
        if instance.detached && reached && instance.creation_position.is_none() {
            match instance.source.particle_position(&cinematic) {
                Ok(position) => instance.creation_position = Some(position),
                Err(error) => {
                    error!("Detached timed effect position failed: {error}");
                    exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
                    return;
                }
            }
        }
        let Some(timed) = &mut instance.timed else {
            continue;
        };
        if reset {
            timed.reset();
        }
        let was_expired = timed.expired;
        let first = timed.visual.is_none() && reached;
        if let Err(error) = timed.advance(reached, animation_dt, &mut seed) {
            error!("Timed effect failed: {}", error);
            exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
            return;
        }
        if first || !was_expired && timed.expired {
            let offset = match &instance.source {
                Source::Effect { request, .. } => request.offset,
                _ => unreachable!(),
            };
            println!(
                "cinematic_timed_effect sequence={} offset={} class={} created={} expired={} size_bits={} transparency_bits={} retirement_child_pending={} native_cadence_verified=false native_world_matrix_verified=false",
                cinematic.sequence(),
                offset,
                instance.source.class(),
                first,
                timed.expired,
                timed.size().to_bits(),
                timed.transparency().to_bits(),
                if instance.retirement_child.is_some() || timed.recipe.child.is_empty() {
                    "none"
                } else {
                    &timed.recipe.child
                }
            );
        }
        if !was_expired
            && timed.expired
            && let Some(child) = instance.retirement_child
        {
            let position = if let Some(position) = instance.creation_position {
                position
            } else {
                let anchor = match instance
                    .source
                    .matrix(cinematic.animation(), cinematic.tick())
                {
                    Ok((matrix, _)) => matrix,
                    Err(error) => {
                        error!("Retirement position failed: {error}");
                        exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
                        return;
                    }
                };
                [anchor[3], anchor[7], anchor[11]]
            };
            retirements.push((parent_index, child, position));
        }
    }
    for (parent, child, position) in retirements {
        let released = scene.instances[child]
            .released
            .as_mut()
            .expect("prepared retirement child");
        if released.parent != Some(parent) {
            error!("Retirement parent mismatch");
            exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
            return;
        }
        if let Err(error) = released.playback.request(position) {
            error!("Retirement request failed: {error}");
            exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
            return;
        }
        println!(
            "cinematic_released_effect_requested sequence={} parent={} class={} position_bits={:?} detached=true schedule=next_positive_viewer_update native_cadence_verified=false",
            cinematic.sequence(),
            parent,
            scene.instances[child].source.class(),
            position.map(f32::to_bits)
        );
    }
    let terrain_query = if scene.instances.iter().any(|i| {
        i.burst.as_ref().is_some_and(|b| b.contact_seed.is_some())
            || i.projectile
                .as_ref()
                .is_some_and(|p| p.playback.contact.is_some())
    }) {
        match giants_game::terrain_queries::StaticTerrainQuery::new(
            &world.asset.terrain,
            (1. / f64::from(world.asset.terrain.spacing)) as f32,
            giants_game::arithmetic::Precision::Double53,
        ) {
            Ok(query) => Some(query),
            Err(reason) => {
                error!("Static effect terrain unavailable: {reason}");
                exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
                return;
            }
        }
    } else {
        None
    };
    let mut retirement_prepared = scene
        .phased
        .iter()
        .map(|effect| effect.parent)
        .collect::<BTreeSet<_>>();
    for (index, instance) in scene.instances.iter().enumerate() {
        if instance
            .projectile
            .as_ref()
            .is_some_and(|p| p.retirement_group.is_some())
        {
            retirement_prepared.insert(index);
        }
    }
    let mut projectile_retirements = Vec::new();
    for (instance_index, instance) in scene.instances.iter_mut().enumerate() {
        if reset && let Some(projectile) = &mut instance.projectile {
            projectile.playback.reset();
        }
        let spawn = instance
            .projectile
            .as_ref()
            .is_some_and(|p| p.playback.matrix.is_none())
            && instance.source.visible(&cinematic)
            && animation_dt > 0.;
        let source_class = instance.source.class();
        let result = (|| -> Result<(), String> {
            let birth = if spawn {
                let Source::Effect {
                    request, actors, ..
                } = &instance.source
                else {
                    return Err("projectile source absent".into());
                };
                let (seconds, _) = cinematic.segments()[cinematic.director_index()]
                    .effect_sample(request.offset)
                    .ok_or("projectile source clock absent")?;
                let (values, flags) = Source::effect_matrix(actors, request, seconds)?;
                let p = instance.projectile.as_ref().expect("prepared projectile");
                let parent = giants_game::effect_attachment::normal_matrix(
                    giants_game::procedural_aim::Matrix { values, flags },
                    p.parent_size,
                    1.,
                    p.parent_own_reference,
                )
                .map_err(str::to_owned)?;
                Some(
                    giants_game::effect_attachment::initial_child_matrix(
                        parent,
                        p.parent_reference,
                        p.child_reference,
                        1.,
                        p.parent_size,
                    )
                    .map_err(str::to_owned)?,
                )
            } else {
                None
            };
            let Some(projectile) = &mut instance.projectile else {
                return Ok(());
            };
            if let Some(matrix) = birth {
                projectile.playback.request(matrix).map_err(str::to_owned)?;
            }
            let was_expired = projectile.playback.expired;
            if projectile.playback.contact.is_some() {
                let query = terrain_query
                    .as_ref()
                    .ok_or("projectile terrain query absent")?;
                projectile
                    .playback
                    .advance_static(animation_dt, |point| {
                        giants_game::physical_ground::Ground::from_sample(
                            query.sample(point)?,
                            world.asset.terrain.width,
                        )
                    })
                    .map_err(str::to_owned)?;
            } else {
                projectile
                    .playback
                    .advance_free(animation_dt)
                    .map_err(str::to_owned)?;
            }
            if !was_expired && projectile.playback.expired {
                if let Some(group) = projectile.retirement_group {
                    projectile_retirements.push((
                        instance_index,
                        group,
                        projectile.playback.motion.position,
                    ));
                }
                let Source::Effect { request, .. } = &instance.source else {
                    unreachable!()
                };
                println!(
                    "cinematic_projectile_effect_expired sequence={} offset={} class={} retirement={:?} retirement_preview_prepared={} static_contact_impact={} native_retirement_complete=false tracer_backend_ready=false",
                    cinematic.sequence(),
                    request.offset,
                    source_class,
                    projectile.playback.recipe.retirement_child,
                    retirement_prepared.contains(&instance_index),
                    projectile
                        .playback
                        .contact
                        .as_ref()
                        .is_some_and(|c| c.flags & 4 != 0)
                );
            }
            if birth.is_some()
                || projectile.playback.callbacks > 0
                    && projectile.playback.callbacks.is_multiple_of(30)
            {
                println!(
                    "cinematic_projectile_effect_motion sequence={} offset={} class={} created={} callbacks={} position_bits={:?} static_contact_bound={} object_contacts_pending=true attached_trails_pending=true retirement_preview_prepared={} native_retirement_complete=false native_angles_verified=false native_cadence_verified=false",
                    cinematic.sequence(),
                    match &instance.source {
                        Source::Effect { request, .. } => request.offset,
                        _ => unreachable!(),
                    },
                    source_class,
                    birth.is_some(),
                    projectile.playback.callbacks,
                    projectile.playback.motion.position.map(f32::to_bits),
                    projectile.playback.contact.is_some(),
                    retirement_prepared.contains(&instance_index)
                );
            }
            Ok(())
        })();
        if let Err(error) = result {
            error!("Projectile preview failed: {error}");
            exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
            return;
        }
    }
    for (parent, group, position) in projectile_retirements {
        if let Err(error) = mixed::queue_root(&mut scene, group, position, &mut seed) {
            error!("Projectile retirement preparation failed: {error}");
            exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
            return;
        }
        println!(
            "cinematic_projectile_mixed_retirement_queued sequence={} parent={parent} group={group} position_bits={:?} schedule=next_positive_viewer_update native_cadence_verified=false",
            cinematic.sequence(),
            position.map(f32::to_bits)
        );
    }
    if reset {
        for instance in &mut scene.instances {
            if let Some(burst) = &mut instance.burst {
                burst.playback = None;
            }
        }
    }
    let mut phase_requests = Vec::new();
    for index in 0..scene.phased.len() {
        if reset {
            scene.phased[index].cursor = None;
            scene.phased[index].origin = None;
        }
        if animation_dt <= 0. {
            continue;
        }
        let parent = scene.phased[index].parent;
        let projectile = scene.instances[parent]
            .projectile
            .as_ref()
            .expect("phased projectile");
        let expired = projectile.playback.expired;
        let position = projectile.playback.motion.position;
        let offset = match &scene.instances[parent].source {
            Source::Effect { request, .. } => request.offset,
            _ => unreachable!(),
        };
        let effect = &mut scene.phased[index];
        let result = if effect.cursor.is_none() && expired {
            effect.origin = Some(position);
            effect.sequence.start(&mut seed).map(|(spawns, cursor)| {
                effect.cursor = Some(cursor);
                Some((0, spawns))
            })
        } else if let Some(cursor) = &mut effect.cursor {
            let phase = cursor.next;
            cursor
                .advance(&effect.sequence, animation_dt, &mut seed)
                .map(|spawns| spawns.map(|s| (phase, s)))
        } else {
            Ok(None)
        };
        match result {
            Ok(Some((phase, spawns))) => {
                println!(
                    "cinematic_retirement_phase sequence={} parent={} group={} phase={} spawns={} origin_bits={:?} density_policy=full_detail_preview native_density_verified=false native_cadence_verified=false offset={}",
                    cinematic.sequence(),
                    parent,
                    index,
                    phase,
                    spawns.len(),
                    effect.origin.expect("phase origin").map(f32::to_bits),
                    offset
                );
                phase_requests.push(((index, phase), spawns));
            }
            Ok(None) => {}
            Err(error) => {
                error!("Retirement phase failed: {error}");
                exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
                return;
            }
        }
    }
    let mut phase_births = BTreeMap::new();
    for (key, spawns) in phase_requests {
        let Some(reference) = scene
            .instances
            .iter()
            .filter_map(|i| i.burst.as_ref())
            .find(|b| b.phase == Some(key))
            .map(|b| b.references.clone())
        else {
            error!("Retirement phase drawable references absent");
            exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
            return;
        };
        match reference.matrices(&spawns, scene.phased[key.0].origin.expect("phase origin")) {
            Ok(matrices) => {
                phase_births.insert(key, (spawns, matrices));
            }
            Err(error) => {
                error!("Retirement matrices failed: {error}");
                exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
                return;
            }
        }
    }
    let mut births = BTreeMap::new();
    let mut birth_origins = BTreeMap::new();
    for instance in &scene.instances {
        let Some(burst) = &instance.burst else {
            continue;
        };
        let Source::Effect { request, .. } = &instance.source else {
            unreachable!();
        };
        if burst.phase.is_some()
            || burst.playback.is_some()
            || !instance.source.visible(&cinematic)
            || animation_dt <= 0.
            || births.contains_key(&request.offset)
        {
            continue;
        }
        let result = (|| -> Result<_, String> {
            let origin = instance.source.particle_position(&cinematic)?;
            let spawns = burst.plan.create(&mut seed).map_err(str::to_owned)?;
            let matrices = burst.references.matrices(&spawns, origin)?;
            println!(
                "cinematic_burst_effect_requested sequence={} offset={} spawns={} origin_bits={:?} contacts_pending=true sounds_pending=true texture_animation_pending=true native_density_verified=false native_cadence_verified=false",
                cinematic.sequence(),
                request.offset,
                spawns.len(),
                origin.map(f32::to_bits)
            );
            Ok((spawns, matrices, origin))
        })();
        match result {
            Ok((spawns, matrices, origin)) => {
                birth_origins.insert(request.offset, origin);
                births.insert(request.offset, (spawns, matrices));
            }
            Err(error) => {
                error!("Burst creation failed: {error}");
                exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
                return;
            }
        }
    }
    let hidden_result = (|| -> Result<(), String> {
        for (&offset, (spawns, matrices)) in &births {
            let classes = scene
                .instances
                .iter()
                .find_map(|i| {
                    let Source::Effect { request, .. } = &i.source else {
                        return None;
                    };
                    i.burst
                        .as_ref()
                        .filter(|b| b.phase.is_none() && request.offset == offset)
                        .map(|b| b.hidden_sound_classes.clone())
                })
                .ok_or("hidden owner source profile absent")?;
            scene
                .hidden_sound_owners
                .start((offset, usize::MAX, 0), spawns, matrices, &classes)?;
        }
        for (&(group, phase), (spawns, matrices)) in &phase_births {
            let classes = scene
                .instances
                .iter()
                .filter_map(|i| i.burst.as_ref())
                .find(|b| b.phase == Some((group, phase)))
                .map(|b| b.hidden_sound_classes.clone())
                .ok_or("hidden phase profile absent")?;
            let Source::Effect { request, .. } =
                &scene.instances[scene.phased[group].parent].source
            else {
                unreachable!();
            };
            let offset = request.offset;
            scene
                .hidden_sound_owners
                .start((offset, group, phase), spawns, matrices, &classes)?;
        }
        scene.hidden_sound_owners.advance(animation_dt)
    })();
    if let Err(reason) = hidden_result {
        error!("Hidden burst owner update failed: {reason}");
        exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
        return;
    }
    if let Some(audio) = audio.as_mut() {
        for (&offset, (spawns, _)) in &births {
            queue_burst_sounds(
                &mut audio.0,
                &cinematic,
                offset,
                usize::MAX,
                0,
                spawns,
                birth_origins[&offset],
            );
        }
        for (&(group, phase), (spawns, _)) in &phase_births {
            let Source::Effect { request, .. } =
                &scene.instances[scene.phased[group].parent].source
            else {
                unreachable!()
            };
            queue_burst_sounds(
                &mut audio.0,
                &cinematic,
                request.offset,
                group,
                phase,
                spawns,
                scene.phased[group].origin.expect("phase origin"),
            );
        }
    }
    let service_recipes = scene
        .emitters
        .iter()
        .map(|e| e.recipe.clone())
        .collect::<Vec<_>>();
    for instance in &mut scene.instances {
        let Some(burst) = &mut instance.burst else {
            continue;
        };
        let Source::Effect { request, class, .. } = &instance.source else {
            unreachable!();
        };
        let result = (|| -> Result<(), String> {
            if burst.playback.is_none()
                && let Some((spawns, matrices)) = burst
                    .phase
                    .and_then(|key| phase_births.get(&key))
                    .or_else(|| {
                        burst
                            .phase
                            .is_none()
                            .then(|| births.get(&request.offset))
                            .flatten()
                    })
            {
                let spawn = spawns
                    .get(burst.index)
                    .ok_or("burst spawn index absent")?
                    .clone();
                let matrix = *matrices
                    .get(burst.index)
                    .ok_or("burst matrix index absent")?;
                let mut words = vec![matrix.flags];
                words.extend(matrix.values.map(f32::to_bits));
                println!(
                    "cinematic_burst_object_requested sequence={} offset={} object={} class={} parent={:?} reference={} own_reference={} matrix_words={:?} size_bits={} lifetime_bits={} speed_bits={} spin_bits={:?} native_cadence_verified=false",
                    cinematic.sequence(),
                    request.offset,
                    burst.index,
                    class,
                    spawn.parent,
                    spawn.reference,
                    spawn.own_reference,
                    words,
                    spawn.visual.start_size.to_bits(),
                    spawn.visual.lifetime.to_bits(),
                    spawn.speed.to_bits(),
                    spawn.spin.map(f32::to_bits)
                );
                let mut playback = giants_game::effect_burst::Playback::with_bound_emitters(
                    spawn,
                    matrix,
                    &burst
                        .services
                        .iter()
                        .map(|&i| service_recipes[i].clone())
                        .collect::<Vec<_>>(),
                )
                .map_err(str::to_owned)?;
                if playback.spawn.contact_requested
                    && playback.spawn.physics_enabled
                    && let Some(seed) = burst.contact_seed
                {
                    match playback.bind_static_contact(seed) {
                        Ok(()) => println!(
                            "cinematic_burst_static_contact_bound sequence={} offset={} object={} class={} static_terrain=true native_contacts_complete=false",
                            cinematic.sequence(),
                            request.offset,
                            burst.index,
                            class
                        ),
                        Err(reason) => println!(
                            "cinematic_burst_contact_deferred sequence={} offset={} object={} class={} reason={reason:?}",
                            cinematic.sequence(),
                            request.offset,
                            burst.index,
                            class
                        ),
                    }
                }
                burst.playback = Some(playback);
            }
            if let Some(playback) = &mut burst.playback {
                let previous_texture_frame = playback.texture_animation.as_ref().map(|c| c.frame());
                let expired = playback.expired;
                let responses = playback.contact.as_ref().map_or(0, |c| c.responses);
                if playback.contact.is_some() {
                    let query = terrain_query
                        .as_ref()
                        .ok_or("bound effect terrain query absent")?;
                    playback
                        .advance_static(animation_dt, |point| {
                            giants_game::physical_ground::Ground::from_sample(
                                query.sample(point)?,
                                world.asset.terrain.width,
                            )
                        })
                        .map_err(str::to_owned)?;
                } else {
                    playback.advance_free(animation_dt).map_err(str::to_owned)?;
                }
                if let Some(clock) = &playback.texture_animation
                    && previous_texture_frame != Some(clock.frame())
                {
                    println!(
                        "cinematic_texture_animation_frame sequence={} offset={} object={} frame={} elapsed_bits={} native_cadence_verified=false",
                        cinematic.sequence(),
                        request.offset,
                        burst.index,
                        clock.frame(),
                        clock.elapsed().to_bits()
                    );
                }
                if responses == 0
                    && let Some(contact) = &playback.contact
                    && contact.responses > 0
                {
                    println!(
                        "cinematic_burst_static_contact_response sequence={} offset={} object={} class={} steps={} flags={} position_bits={:?} native_scene_fidelity_verified=false",
                        cinematic.sequence(),
                        request.offset,
                        burst.index,
                        class,
                        contact.steps,
                        contact.flags,
                        playback.motion.position.map(f32::to_bits)
                    );
                }
                if !expired && playback.expired {
                    println!(
                        "cinematic_burst_object_expired sequence={} offset={} object={} class={} contacts_pending=true native_cadence_verified=false",
                        cinematic.sequence(),
                        request.offset,
                        burst.index,
                        class
                    );
                }
            }
            Ok(())
        })();
        if let Err(error) = result {
            error!("Burst preview failed: {error}");
            exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
            return;
        }
    }
    for group in 0..scene.emitters.len() {
        let parent = scene.emitters[group].parent;
        let source_state = parent.and_then(|p| {
            scene.instances[p]
                .burst
                .as_ref()
                .and_then(|b| b.playback.as_ref())
                .map(|p| (p.motion.position, !p.expired))
                .or_else(|| {
                    scene.instances[p]
                        .mixed
                        .as_ref()
                        .and_then(|m| m.presentation)
                        .map(|m| {
                            (
                                [m.matrix.values[3], m.matrix.values[7], m.matrix.values[11]],
                                !m.expired,
                            )
                        })
                })
        });
        let reference_sample = match scene.emitters[group].reference.as_ref() {
            Some(binding) => match mixed::reference_sample(&scene.mixed, binding) {
                Ok(sample) => sample,
                Err(error) => {
                    error!("Reference guide sample failed: {error}");
                    exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
                    return;
                }
            },
            None => None,
        };
        let position_sample = match scene.emitters[group].position.as_ref() {
            Some(binding) => match mixed::position_sample(&scene.mixed, binding, &cinematic) {
                Ok(sample) => sample,
                Err(error) => {
                    error!("Position guide sample failed: {error}");
                    exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
                    return;
                }
            },
            None => None,
        };
        let emitter = &mut scene.emitters[group];
        if reset {
            emitter.runtime = None;
            if let Some(reference) = &mut emitter.reference {
                reference.channel = None;
                reference.parent = None;
            }
        }
        let reached = if emitter.position.is_some() {
            position_sample.is_some()
        } else if emitter.reference.is_some() {
            reference_sample.is_some()
        } else if parent.is_some() {
            source_state.is_some()
        } else {
            emitter.source.visible(&cinematic)
        };
        if emitter.runtime.is_none() && animation_dt > 0. && reached {
            let result = if let Some(sample) = &position_sample {
                giants_game::effect_emitter_runtime::Runtime::sampled_growing_at(
                    emitter.recipe.clone(),
                    sample.interval,
                    sample.constructor_position,
                )
                .map_err(str::to_owned)
            } else if parent.is_some() && emitter.reference.is_none() {
                giants_game::effect_emitter_runtime::Runtime::fixed_growing(
                    emitter.recipe.clone(),
                    emitter.reserve,
                )
                .map_err(str::to_owned)
            } else if emitter.reference.is_some() {
                giants_game::effect_emitter_runtime::Runtime::fixed(
                    emitter.recipe.clone(),
                    emitter.reserve,
                )
                .map_err(str::to_owned)
            } else {
                emitter
                    .source
                    .particle_position(&cinematic)
                    .and_then(|position| {
                        giants_game::effect_emitter_runtime::Runtime::new(
                            emitter.recipe.clone(),
                            emitter.reserve,
                            position,
                            &mut seed,
                        )
                        .map_err(str::to_owned)
                    })
            };
            match result {
                Ok(runtime) => {
                    let Source::Effect { request, .. } = &emitter.source else {
                        unreachable!()
                    };
                    println!(
                        "cinematic_periodic_effect_started sequence={} offset={} class={} parent={:?} native_cadence_verified=false request_satisfied=false",
                        cinematic.sequence(),
                        request.offset,
                        emitter.recipe.class,
                        parent
                    );
                    emitter.runtime = Some(runtime);
                    if let (Some(binding), Some(sample)) = (&emitter.position, &position_sample) {
                        println!(
                            "cinematic_position_effect_started sequence={} offset={} emitter={group} child={} interval_bits={} guide_group={} guide_phase={} guide_object={} constructor_position_bits={:?} native_cadence_verified=false request_satisfied=false",
                            cinematic.sequence(),
                            request.offset,
                            emitter.recipe.child,
                            sample.interval.to_bits(),
                            binding.group,
                            binding.phase,
                            binding.object,
                            sample.constructor_position.map(f32::to_bits)
                        );
                    }
                    if let (Some(binding), Some(sample)) =
                        (&mut emitter.reference, &reference_sample)
                    {
                        binding.channel = Some(sample.channel);
                        println!(
                            "cinematic_reference_effect_started sequence={} offset={} child={} interval_bits={} guide_group={} guide_phase={} guide_object={} native_cadence_verified=false request_satisfied=false paired={}",
                            cinematic.sequence(),
                            request.offset,
                            emitter.recipe.child,
                            sample.channel.interval.to_bits(),
                            binding.group,
                            binding.phase,
                            binding.object,
                            matches!(
                                sample.channel.mode,
                                giants_game::effect_reference_emission::Mode::Paired(_)
                            )
                        );
                    }
                }
                Err(error) => {
                    error!("Periodic creation failed: {error}");
                    exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
                    return;
                }
            }
        }
        if let Some(runtime) = &mut emitter.runtime {
            let result = if let Some(sample) = &position_sample {
                runtime.advance_source(animation_dt, sample.position, sample.emitting, &mut seed)
            } else if let Some(binding) = &mut emitter.reference {
                binding.parent = reference_sample.as_ref().and_then(|s| s.parent.clone());
                match (&mut binding.channel, &reference_sample) {
                    (Some(channel), Some(sample))
                        if matches!(
                            channel.mode,
                            giants_game::effect_reference_emission::Mode::Paired(_)
                        ) =>
                    {
                        runtime.advance_paired(
                            animation_dt,
                            channel,
                            &sample.points,
                            sample.emitting,
                            &mut seed,
                        )
                    }
                    (Some(channel), Some(sample)) => runtime.advance_reference(
                        animation_dt,
                        channel,
                        &sample.points,
                        sample.emitting,
                        &mut seed,
                    ),
                    _ => Err("reference guide runtime state absent"),
                }
            } else if let Some((position, emitting)) = source_state {
                runtime.advance_source(animation_dt, position, emitting, &mut seed)
            } else {
                runtime.advance(animation_dt, &mut seed)
            };
            match result {
                Ok(events) => {
                    let Source::Effect { request, .. } = &emitter.source else {
                        unreachable!()
                    };
                    for event in events {
                        if emitter.position.is_some() {
                            println!(
                                "cinematic_position_particle sequence={} offset={} emitter={group} child={} kind={} serial={} position_bits={:?} native_cadence_verified=false native_raster_verified=false",
                                cinematic.sequence(),
                                request.offset,
                                emitter.recipe.child,
                                event.kind,
                                event.serial,
                                event.position.map(f32::to_bits)
                            );
                        }
                        if emitter.reference.is_some() {
                            println!(
                                "cinematic_reference_particle sequence={} offset={} child={} kind={} serial={} position_bits={:?} native_cadence_verified=false native_raster_verified=false paired={}",
                                cinematic.sequence(),
                                request.offset,
                                emitter.recipe.child,
                                event.kind,
                                event.serial,
                                event.position.map(f32::to_bits),
                                emitter
                                    .reference
                                    .as_ref()
                                    .and_then(|b| b.channel)
                                    .is_some_and(|c| matches!(
                                        c.mode,
                                        giants_game::effect_reference_emission::Mode::Paired(_)
                                    ))
                            );
                        }
                        println!(
                            "cinematic_periodic_particle sequence={} offset={} kind={} descendant={} serial={} position_bits={:?} native_cadence_verified=false",
                            cinematic.sequence(),
                            request.offset,
                            event.kind,
                            event.particle,
                            event.serial,
                            event.position.map(f32::to_bits)
                        );
                    }
                }
                Err(error) => {
                    error!(
                        "Periodic update failed: {error}; sequence={} emitter={group} class={} child={} reserve={} slots={:?} parent={parent:?} reference={} position={} dt_bits={} source_state={:?}",
                        cinematic.sequence(),
                        emitter.recipe.class,
                        emitter.recipe.child,
                        emitter.reserve,
                        runtime.slot_counts(),
                        emitter.reference.is_some(),
                        emitter.position.is_some(),
                        animation_dt.to_bits(),
                        runtime.source_state(),
                    );
                    exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
                    return;
                }
            }
        }
    }
    let grown = match growing::instances(&mut scene) {
        Ok(grown) => grown,
        Err(error) => {
            error!("Growing particle instances failed: {error}");
            exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
            return;
        }
    };
    for index in 0..scene.instances.len() {
        let Some(emitted) = &scene.instances[index].emitted else {
            continue;
        };
        let (group, particle, slot) = (emitted.group, emitted.particle, emitted.slot);
        let previous = emitted.presentation.map(|p| p.serial);
        let presentation = scene.emitters[group]
            .runtime
            .as_ref()
            .and_then(|r| r.presentation(particle, slot));
        let parent = presentation
            .filter(|p| !p.expired)
            .and_then(|p| p.attachment)
            .and_then(|binding| {
                let parent = scene.emitters[group].reference.as_ref()?.parent.as_ref()?;
                Some(mixed::AttachmentParent {
                    matrix: parent.matrix,
                    size: parent.size,
                    reference: *parent.references.get(&binding.parent_reference)?,
                })
            });
        let instance = &mut scene.instances[index];
        if presentation.is_some_and(|p| Some(p.serial) != previous)
            && let Some(animation) = &mut instance.animation
        {
            animation.reset();
        }
        instance
            .emitted
            .as_mut()
            .expect("emitted slot")
            .presentation = presentation;
        instance.emitted.as_mut().expect("emitted slot").parent = parent;
    }
    scene.effect_seed = seed;
    let mut tracers = std::mem::take(&mut scene.tracers);
    let tracer_result = tracer::update(
        &mut tracers,
        &scene,
        &world,
        &cinematic,
        &mut commands,
        &mut assets,
        animation_dt,
        reset,
        isolation.as_ref().map(|i| i.offset),
    );
    scene.tracers = tracers;
    if let Err(error) = tracer_result {
        error!("Tracer preview failed: {error}");
        exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
        return;
    }
    if let Err(error) = growing::draws(
        &mut scene,
        &grown,
        &cinematic,
        &world,
        &mut commands,
        &mut assets,
    ) {
        error!("Growing particle draws failed: {error}");
        exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
        return;
    }
    let mut deformations = BTreeMap::new();
    for (index, instance) in scene.instances.iter_mut().enumerate() {
        // The director's enabled field gates its ordinary object clocks too.
        // Full transparency is only a draw gate. Reset still initializes every
        // clock below, and source-owned effects keep their separate lifecycle.
        let owned = match &instance.source {
            Source::Effect { request, .. } => persistent.as_ref().and_then(|state| {
                runtime.as_ref().and_then(|runtime| {
                    state.persistent_frame(runtime, cinematic.director_index(), request.offset)
                })
            }),
            Source::Actor(_) => None,
        };
        let active = timed_frames.get(&index).map_or_else(
            || instance.visible(&cinematic),
            |frame| frame.is_some_and(|frame| frame.object_flags & 0xc000 == 0),
        ) && owned.is_none_or(|frame| frame.is_some())
            && match &instance.source {
                Source::Actor(request) => cinematic.actor_visible(request.slot) == Some(true),
                Source::Effect { .. } => true,
            };
        let Some(animation) = &mut instance.animation else {
            continue;
        };
        if reset {
            animation.reset();
        }
        let advanced = if let Some(frame) = timed_frames.get(&index) {
            if let Some(frame) = frame {
                animation.clock = giants_game::animation_channels::clock(&frame.channels[0]);
                animation.updates = u64::from(frame.epoch);
            }
            Ok(active && (animation_dt > 0. || reset))
        } else if let Some(Some(frame)) = owned {
            // The owned object's channel is authoritative; do not tick a
            // second independent animation clock in the renderer.
            animation.clock = giants_game::animation_channels::clock(&frame.channels[0]);
            animation.updates = frame.updates;
            Ok(active && (animation_dt > 0. || reset))
        } else if active {
            animation.advance(animation_dt)
        } else {
            Ok(false)
        };
        let positions = advanced.map_err(str::to_owned).and_then(|advanced| {
            if advanced || reset {
                animation.positions().map(Some)
            } else {
                Ok(None)
            }
        });
        match positions {
            Ok(Some(giants_game::actor_context::PositionUpdate::Replace(positions))) => {
                if animation.updates == 1
                    || animation.updates.is_multiple_of(60)
                    || animation.clock.wrapped
                    || reset
                {
                    let (tag, identity) = match &instance.source {
                        Source::Effect { request, .. } => (
                            "cinematic_effect_animation",
                            format!("offset={}", request.offset),
                        ),
                        Source::Actor(request) => (
                            "cinematic_attachment_animation",
                            format!("actor_slot={} class={}", request.slot, request.class),
                        ),
                    };
                    println!(
                        "{tag} sequence={} {identity} seconds_bits={} updates={} viewer_dt_bits={} native_cadence_verified=false",
                        cinematic.sequence(),
                        animation.clock.seconds.to_bits(),
                        animation.updates,
                        animation_dt.to_bits()
                    );
                }
                if instance.normals.is_some() {
                    let mesh = &instance.source.asset().model.mesh;
                    let flags = mesh
                        .subobjects
                        .iter()
                        .fold(0, |flags, part| flags | part.material.flags);
                    let profile =
                        giants_game::body_pipeline::Normals::select(false, true, false, true);
                    match giants_game::body_pipeline::prepare_normals(
                        mesh,
                        &positions,
                        profile.contextual_preparation(flags),
                    ) {
                        Ok(normals) => instance.normals = Some(normals),
                        Err(error) => {
                            error!("Animated attachment normals failed: {error}");
                            exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
                            return;
                        }
                    }
                    // Context colors use current geometry, not the original
                    // zero-clock snapshot. Each instance owns its deformation.
                    instance.geometry = Arc::new(Geometry {
                        positions: positions.clone(),
                        skin: instance.geometry.skin.clone(),
                        profile: instance.geometry.profile,
                    });
                }
                deformations.insert(index, positions);
            }
            Ok(_) => {}
            Err(error) => {
                error!("Effect animation failed: {}", error);
                exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
                return;
            }
        }
    }
    scene.generation = cinematic.generation();
    for draw in &scene.draws {
        // The ordinary route shares the effect route's mesh and material data.
        if draw.route == 1 {
            continue;
        }
        if let Some(positions) = deformations.get(&draw.instance) {
            let asset = scene.instances[draw.instance].source.asset();
            let sub = &asset.model.mesh.subobjects[draw.part];
            let vertices: Vec<_> = sub
                .triangles
                .iter()
                .flatten()
                .map(|index| {
                    let p = positions[usize::from(asset.model.mesh.indices[usize::from(*index)])];
                    if scene.instances[draw.instance].normals.is_some() {
                        p
                    } else {
                        [p[0], p[2], -p[1]]
                    }
                })
                .collect();
            let Some(mut mesh) = assets.meshes.get_mut(&draw.mesh) else {
                error!("Effect animation mesh absent");
                exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
                return;
            };
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
            if let Some(normals) = &scene.instances[draw.instance].normals {
                mesh.insert_attribute(
                    Mesh::ATTRIBUTE_NORMAL,
                    sub.triangles
                        .iter()
                        .flatten()
                        .map(|i| normals[usize::from(*i)])
                        .collect::<Vec<_>>(),
                );
            }
        }
        if let Some(vertical) = timed_frames
            .get(&draw.instance)
            .and_then(|frame| frame.as_ref())
            .and_then(|frame| frame.vertical.as_ref())
            .or_else(|| {
                // Canonical offsets never advance a second preview fade clock.
                if timed_frames.contains_key(&draw.instance) {
                    return None;
                }
                scene.instances[draw.instance]
                    .released
                    .as_ref()
                    .and_then(|r| r.playback.vertical.as_ref())
            })
        {
            let asset = scene.instances[draw.instance].source.asset();
            let sub = &asset.model.mesh.subobjects[draw.part];
            let result = (|| -> Result<(), String> {
                let bounds = asset.model.mesh.bounds().map_err(|e| e.to_string())?;
                let mut mesh = assets
                    .meshes
                    .get_mut(&draw.mesh)
                    .ok_or("vertical fade mesh absent")?;
                let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
                    mesh.attribute(Mesh::ATTRIBUTE_POSITION)
                else {
                    return Err("vertical fade position format absent".into());
                };
                let colors = sub
                    .triangles
                    .iter()
                    .flatten()
                    .enumerate()
                    .map(|(i, index)| {
                        let rgb = asset.model.mesh.rgb[usize::from(*index)];
                        let alpha = vertical
                            .alpha(positions[i][1], [bounds[2], bounds[5]])
                            .map_err(str::to_owned)?;
                        Ok([
                            rgb[0] as f32 / 255.,
                            rgb[1] as f32 / 255.,
                            rgb[2] as f32 / 255.,
                            alpha as f32 / 255.,
                        ])
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
                Ok(())
            })();
            if let Err(error) = result {
                error!("Vertical fade failed: {error}");
                exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
                return;
            }
        }
    }
    let camera = world
        .validation_camera
        .as_ref()
        .expect("rigid source camera");
    let matrices = scene
        .instances
        .iter()
        .enumerate()
        .map(|(index, i)| {
            let owned = match &i.source {
                Source::Effect { request, .. } => persistent
                    .as_ref()
                    .and_then(|state| {
                        runtime.as_ref().and_then(|runtime| {
                            state.persistent_frame(
                                runtime,
                                cinematic.director_index(),
                                request.offset,
                            )
                        })
                    })
                    .flatten(),
                Source::Actor(_) => None,
            };
            i.render_matrix_with_world(
                cinematic.animation(),
                cinematic.tick(),
                camera.view,
                timed_frames
                    .get(&index)
                    .and_then(|frame| frame.map(|frame| frame.world))
                    .or_else(|| owned.map(|frame| frame.world)),
                timed_frames
                    .get(&index)
                    .and_then(|frame| frame.map(|frame| frame.scale)),
            )
        })
        .collect::<Result<Vec<_>, _>>();
    let matrices = match matrices {
        Ok(v) => v,
        Err(e) => {
            error!("Rigid attachment matrix failed: {}", e);
            exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
            return;
        }
    };
    let mut moved = BTreeSet::new();
    for m in &mut scene.motion {
        let matrix = world_matrix(matrices[m.instance].0);
        if m.matrix != matrix {
            if let Some(handle) = &m.buffer
                && let Some(mut b) = assets.buffers.get_mut(handle)
            {
                crate::source_view::write_uniform(&matrix, &mut b);
            }
            m.matrix = matrix;
            moved.insert(m.instance);
        }
    }
    for draw in &scene.draws {
        let (source_matrix, front) = matrices[draw.instance];
        let matrix = world_matrix(source_matrix);
        if let Ok((mut t, mut visibility)) = transforms.get_mut(draw.entity) {
            if moved.contains(&draw.instance) {
                *t = Transform::from_matrix(matrix);
            }
            if matches!(scene.instances[draw.instance].source, Source::Effect { .. }) {
                let previous = *visibility;
                let instance = &scene.instances[draw.instance];
                let owned = match &instance.source {
                    Source::Effect { request, .. } => persistent.as_ref().and_then(|state| {
                        runtime.as_ref().and_then(|runtime| {
                            state.persistent_frame(
                                runtime,
                                cinematic.director_index(),
                                request.offset,
                            )
                        })
                    }),
                    Source::Actor(_) => None,
                };
                let canonical = timed_frames.get(&draw.instance);
                let route_enabled = if let Some(frame) = canonical {
                    match frame {
                        Some(frame) if frame.vertical.is_some() => {
                            let eye = cinematic.camera().expect("prepared source camera").origin;
                            match frame.draw_routes(
                                instance.source.class(),
                                eye,
                                cinematic.ordinary_range(),
                            ) {
                                Ok(routes) => routes.mask() & draw.route != 0,
                                Err(error) => {
                                    error!("Detached timed effect admission failed: {error}");
                                    exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
                                    return;
                                }
                            }
                        }
                        Some(frame) => draw.route == 0 && frame.object_flags & 0xc000 == 0,
                        None => false,
                    }
                } else if instance.persistent_attachment {
                    match owned {
                        Some(Some(frame)) => {
                            let eye = cinematic.camera().expect("prepared source camera").origin;
                            match frame.draw_routes(eye, cinematic.ordinary_range()) {
                                Ok(routes) => routes.mask() & draw.route != 0,
                                Err(error) => {
                                    error!("Persistent effect admission failed: {error}");
                                    exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
                                    return;
                                }
                            }
                        }
                        Some(None) => false,
                        // Isolated inspection has no owned world traversal state.
                        // Retain its single existing draw, with no ordinary claim.
                        None => draw.route == 2,
                    }
                } else {
                    draw.route == 0
                        || instance.released.as_ref().is_some_and(|r| {
                            r.routes.is_some_and(|profile| {
                                profile
                                    .routes(
                                        r.playback.requested(),
                                        r.playback.visual.is_some(),
                                        r.playback.expired,
                                    )
                                    .mask()
                                    & draw.route
                                    != 0
                            })
                        })
                };
                let visible = canonical.map_or_else(
                    || instance.visible(&cinematic),
                    |frame| frame.is_some_and(|frame| frame.object_flags & 0xc000 == 0),
                );
                *visibility =
                    if visible && route_enabled && owned.is_none_or(|frame| frame.is_some()) {
                        Visibility::Inherited
                    } else {
                        Visibility::Hidden
                    };
                if let Some(Some(frame)) = canonical
                    && (previous != *visibility || moved.contains(&draw.instance))
                    && let Source::Effect { request, .. } = &instance.source
                {
                    println!(
                        "mission_timed_draw ticket={} director={} offset={} class={} owner={:?} entity={:?} visible={} epoch={} matrix_bits={:?} canonical_body=true viewer_simulation=false native_pixels_verified=false",
                        persistent.as_ref().unwrap().ticket(),
                        cinematic.director_index(),
                        request.offset,
                        instance.source.class(),
                        frame.owner,
                        draw.entity,
                        *visibility != Visibility::Hidden,
                        frame.epoch,
                        frame.world.values.map(f32::to_bits)
                    );
                }
                if let Some(Some(frame)) = owned
                    && (previous != *visibility || frame.updates == 1)
                    && let Source::Effect { request, .. } = &instance.source
                {
                    println!(
                        "mission_persistent_draw ticket={} director={} offset={} class={} part={} route={} entity={:?} visible={} updates={} matrix_bits={:?} native_pixels_verified=false",
                        persistent.as_ref().unwrap().ticket(),
                        cinematic.director_index(),
                        request.offset,
                        instance.source.class(),
                        draw.part,
                        draw.route,
                        draw.entity,
                        *visibility != Visibility::Hidden,
                        frame.updates,
                        source_matrix.map(f32::to_bits)
                    );
                }
                if (previous != *visibility || reset)
                    && let Some(mixed) = &scene.instances[draw.instance].mixed
                {
                    let Source::Effect { request, .. } = &scene.instances[draw.instance].source
                    else {
                        unreachable!()
                    };
                    println!(
                        "cinematic_mixed_draw_state sequence={} offset={} group={} phase={} object={} class={} part={} entity={:?} visible={} tick={} paused_sample={} matrix_bits={:?} native_pixels_verified=false",
                        cinematic.sequence(),
                        request.offset,
                        mixed.group,
                        mixed.phase,
                        mixed.object,
                        scene.instances[draw.instance].source.class(),
                        draw.part,
                        draw.entity,
                        *visibility != Visibility::Hidden,
                        cinematic.tick(),
                        paused_sample,
                        source_matrix.map(f32::to_bits)
                    );
                }
                if (previous != *visibility || reset)
                    && let Some(emitted) = &scene.instances[draw.instance].emitted
                    && let Some(presentation) = emitted.presentation
                    && scene.emitters[emitted.group].position.is_some()
                {
                    let Source::Effect { request, .. } = &scene.instances[draw.instance].source
                    else {
                        unreachable!()
                    };
                    println!(
                        "cinematic_position_draw_state sequence={} offset={} emitter={} class={} serial={} part={} entity={:?} visible={} native_pixels_verified=false",
                        cinematic.sequence(),
                        request.offset,
                        emitted.group,
                        scene.instances[draw.instance].source.class(),
                        presentation.serial,
                        draw.part,
                        draw.entity,
                        *visibility != Visibility::Hidden
                    );
                }
                if (previous != *visibility || reset)
                    && let Some(emitted) = &scene.instances[draw.instance].emitted
                    && let Some(presentation) = emitted.presentation
                    && let Some(binding) = presentation.attachment
                {
                    let Source::Effect { request, .. } = &scene.instances[draw.instance].source
                    else {
                        unreachable!()
                    };
                    println!(
                        "cinematic_paired_draw_state sequence={} offset={} emitter={} class={} serial={} child_reference={} parent_reference={} part={} entity={:?} visible={} matrix_bits={:?} native_pixels_verified=false",
                        cinematic.sequence(),
                        request.offset,
                        emitted.group,
                        scene.instances[draw.instance].source.class(),
                        presentation.serial,
                        binding.child_reference,
                        binding.parent_reference,
                        draw.part,
                        draw.entity,
                        *visibility != Visibility::Hidden,
                        source_matrix.map(f32::to_bits)
                    );
                }
            }
        }
        let source_world = Mat4::from_cols_array(&source_matrix).transpose();
        if draw.route == 1 {
            continue;
        }
        let sub = &scene.instances[draw.instance]
            .source
            .asset()
            .model
            .mesh
            .subobjects[draw.part];
        match &draw.material {
            DrawMaterial::Lit {
                material,
                parameters,
                alpha,
                context,
                additive,
            } => {
                if assets
                    .lit_materials
                    .get(material)
                    .is_some_and(|m| m.front() != front)
                {
                    assets
                        .lit_materials
                        .get_mut(material)
                        .expect("prepared lit material")
                        .set_front(front);
                }
                if let Some(mut buffer) = assets.buffers.get_mut(parameters) {
                    crate::gun_scene::source_parameters(
                        &sub.material,
                        source_world,
                        camera,
                        &scene.lights,
                        scene.fog,
                        world.no_fog,
                    )
                    .white_texture_factor(*alpha)
                    .suppress_primary_diffuse(*context)
                    .additive_profile(*additive)
                    .write_buffer(&mut buffer);
                }
                continue;
            }
            DrawMaterial::Context(handle) => {
                if moved.contains(&draw.instance) || deformations.contains_key(&draw.instance) {
                    let colors = match context_colors(
                        &scene.instances[draw.instance],
                        draw.part,
                        &cinematic,
                        &scene.lights[0],
                    ) {
                        Ok(colors) => colors,
                        Err(error) => {
                            error!("Attachment context color failed: {error}");
                            exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
                            return;
                        }
                    };
                    if let Some(mut mesh) = assets.meshes.get_mut(&draw.mesh) {
                        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
                    }
                }
                let pixel = Vec4::new(
                    1. / camera.viewport.z as f32,
                    -1. / camera.viewport.w as f32,
                    0.,
                    0.,
                );
                let fog = Vec4::new(
                    scene.fog.start,
                    scene.fog.end,
                    if world.no_fog { 0. } else { 1. },
                    0.,
                );
                if assets.context_materials.get(handle).is_some_and(|m| {
                    m.source_world != source_world
                        || m.front != front
                        || m.pixel != pixel
                        || m.fog != fog
                }) {
                    let mut material = assets
                        .context_materials
                        .get_mut(handle)
                        .expect("prepared context material");
                    material.source_world = source_world;
                    material.front = front;
                    material.pixel = pixel;
                    material.fog = fog;
                }
                continue;
            }
            DrawMaterial::Cube {
                material,
                parameters,
                factor,
                blend,
                ..
            } => {
                if assets
                    .cube_materials
                    .as_ref()
                    .and_then(|a| a.get(material))
                    .is_some_and(|m| m.front() != front)
                {
                    assets
                        .cube_materials
                        .as_mut()
                        .expect("prepared cube materials")
                        .get_mut(material)
                        .expect("prepared cube material")
                        .set_front(front);
                }
                if let Some(mut buffer) = assets.buffers.get_mut(parameters) {
                    crate::sword_scene::source_parameters(
                        &sub.material,
                        source_world,
                        camera,
                        &scene.lights,
                        scene.fog,
                        world.no_fog,
                    )
                    .cube_factor(Some(*factor))
                    .blended(*blend != 0)
                    .write_buffer(&mut buffer);
                }
                continue;
            }
            DrawMaterial::Unlit(_) => {}
        }
        let DrawMaterial::Unlit(material) = &draw.material else {
            unreachable!()
        };
        let p = pixel(camera, scene.fog);
        let fog = if world.no_fog { 0. } else { 1. };
        let transparency = timed_frames
            .get(&draw.instance)
            .and_then(|frame| frame.map(|frame| frame.transparency))
            .or_else(|| scene.instances[draw.instance].transparency());
        let opacity = if let Some(transparency) = transparency {
            match giants_game::unlit_material::prepare_with_object_transparency(
                &sub.material,
                scene.instances[draw.instance]
                    .source
                    .asset()
                    .model
                    .alpha_textures
                    .contains(&sub.material.texture.to_ascii_lowercase()),
                transparency,
            ) {
                Ok(profile) => Some((profile.alpha, profile.blend)),
                Err(error) => {
                    error!("Timed effect material failed: {}", error);
                    exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
                    return;
                }
            }
        } else {
            None
        };
        let changed = assets.materials.get(material).is_some_and(|material| {
            material.pixel_fog != p
                || material.flags.y != fog
                || material.front != front
                || opacity.is_some_and(|(alpha, blend)| {
                    material.color_alpha.w != alpha || material.blend != blend
                })
        });
        let texture = draw.texture_frames.as_ref().and_then(|frames| {
            let frame = scene.instances[draw.instance]
                .burst
                .as_ref()
                .and_then(|b| b.playback.as_ref())
                .and_then(|p| p.texture_animation.as_ref())
                .map_or(0, |clock| clock.frame()) as usize;
            frames.get(frame)
        });
        let texture_changed = texture.is_some_and(|texture| {
            assets
                .materials
                .get(material)
                .is_some_and(|m| m.texture != *texture)
        });
        if (changed || texture_changed)
            && let Some(mut material) = assets.materials.get_mut(material)
        {
            if let Some(texture) = texture {
                material.texture = texture.clone();
                if texture_changed
                    && let Some(burst) = &scene.instances[draw.instance].burst
                    && let Some(clock) = burst
                        .playback
                        .as_ref()
                        .and_then(|p| p.texture_animation.as_ref())
                    && let Source::Effect { request, .. } = &scene.instances[draw.instance].source
                {
                    println!(
                        "cinematic_texture_animation_bound sequence={} offset={} object={} part={} frame={} native_raster_verified=false",
                        cinematic.sequence(),
                        request.offset,
                        burst.index,
                        draw.part,
                        clock.frame()
                    );
                }
            }
            material.pixel_fog = p;
            material.flags.y = fog;
            material.front = front;
            if let Some((alpha, blend)) = opacity {
                material.color_alpha.w = alpha;
                material.blend = blend;
            }
        }
    }
    scene.tick = cinematic.tick();
}
pub fn switch(
    mut commands: Commands,
    mut scene: ResMut<Scene>,
    cinematic: Res<crate::cinematic_camera::Cinematic>,
    rendered: Query<Entity, With<Rendered>>,
    mut exit: MessageWriter<AppExit>,
) {
    let carried = if scene.active_index() != cinematic.director_index() {
        let mut value = std::mem::take(&mut scene.tracers);
        if let Err(error) = value.handoff() {
            error!("Tracer handoff failed: {error}");
            exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
            return;
        }
        // Source-marked finite effects retire at the native director cut.
        // Keep preloaded resources, but discard outgoing runtime/pending tokens
        // now rather than leaving them in the inactive cached scene entry.
        mixed::reset(&mut scene);
        println!(
            "cinematic_mixed_director_cleanup director={} prepared_contexts={} pending_children_cleared=true scope=source_marked_preview native_loaded_actor_teardown_verified=false",
            scene.active_index(),
            scene.mixed.len()
        );
        Some(value)
    } else {
        None
    };
    match scene.select(cinematic.director_index()) {
        Ok(false) => return,
        Ok(true) => {}
        Err(e) => {
            error!("Rigid attachment director selection failed: {}", e);
            exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
            return;
        }
    }
    if let Some(value) = carried {
        scene.tracers = value;
    }
    for entity in &rendered {
        commands.entity(entity).despawn();
    }
    scene.draws.clear();
    scene.motion.clear();
    scene.tick = i32::MIN;
    info!(
        "cinematic_scene_switch=rigid_attachment_scene director={} sequence={}",
        cinematic.director_index(),
        cinematic.sequence()
    );
    commands.run_system_cached(setup);
}

#[cfg(test)]
mod particle_route_tests {
    #[test]
    fn shader_alpha_program_exists_before_first_callback_and_survives_reset() {
        let recipe = giants_game::released_effect::Recipe {
            class: 900,
            visual: giants_game::effect_visual::Configuration {
                lifetime: 1.,
                fade_in: 0.,
                fade_out_start: 1.,
                start_size: 1.,
                end_size: 1.,
            },
            gravity_mass: 0.,
            size_curve: giants_game::effect_visual::SizeCurve::Linear,
            initial_offset: [0.; 3],
            random: None,
            vertical_fade: Some(giants_game::vertical_fade::Configuration {
                start: [0., 1.],
                end: [0., 1.],
                duration: 1.,
            }),
        };
        let mut released = super::Released {
            parent: None,
            playback: giants_game::released_effect::Playback::new(recipe),
            routes: None,
            last_route_mask: None,
        };
        assert!(released.playback.vertical.is_none());
        assert!(released.uses_vertex_alpha());
        released.playback.reset();
        assert!(released.uses_vertex_alpha());
        released.playback.recipe.vertical_fade = None;
        assert!(!released.uses_vertex_alpha());
    }
    #[test]
    fn source_additive_material_is_admitted_without_changing_its_blend() {
        let source = giants_formats::gbs::Material {
            texture: "synthetic".into(),
            bump_texture: String::new(),
            falloff: 0.01,
            blend: 0.,
            flags: 0x40000001,
            colors: [0; 4],
            power: 0.,
        };
        let additive =
            giants_game::unlit_material::prepare_with_texture_alpha(&source, true).unwrap();
        assert_eq!(additive.blend, 1);
        assert!(super::routed_vertical_materials(&[(
            0,
            super::Part::Unlit(additive)
        )]));
        let alpha = giants_game::unlit_material::prepare_with_texture_alpha(
            &giants_formats::gbs::Material {
                flags: 0x40000000,
                ..source
            },
            true,
        )
        .unwrap();
        assert_eq!(alpha.blend, 2);
        assert!(!super::routed_vertical_materials(&[(
            0,
            super::Part::Unlit(alpha)
        )]));
        assert!(!super::routed_vertical_materials(&[]));
    }
}
