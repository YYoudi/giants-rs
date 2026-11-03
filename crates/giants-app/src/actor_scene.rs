//! Animated body geometry and the measured texture/source-RGB base pass.
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
use giants_game::body_pipeline::{Normals, Surface};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MaterialKey(u8);
impl From<&Bindings> for MaterialKey {
    fn from(material: &Bindings) -> Self {
        Self(if material.blend {
            2
        } else if material.force_additive || material.validation.w < 1. {
            1
        } else {
            0
        })
    }
}
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
#[bind_group_data(MaterialKey)]
pub struct Bindings {
    #[texture(0)]
    #[sampler(1)]
    pub texture: Handle<Image>,
    #[storage(2, read_only)]
    pub clip_from_bevy: Handle<bevy::render::storage::ShaderBuffer>,
    #[uniform(3)]
    pub validation: Vec4,
    #[uniform(4)]
    pub fog: Vec4,
    #[uniform(5)]
    pub fog_color: Vec4,
    pub blend: bool,
    pub force_additive: bool,
}
pub type Material = crate::source_view::UniformMaterial<Bindings>;
impl bevy::pbr::Material for Material {
    fn vertex_shader() -> ShaderRef {
        "embedded://giants_app/actor.wesl".into()
    }
    fn fragment_shader() -> ShaderRef {
        "embedded://giants_app/actor.wesl".into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        if self.blend {
            AlphaMode::Blend
        } else if self.force_additive || self.validation.w < 1. {
            AlphaMode::Add
        } else {
            AlphaMode::Opaque
        }
    }
    fn specialize(
        _: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.vertex.buffers = vec![layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(1),
            Mesh::ATTRIBUTE_COLOR.at_shader_location(2),
        ])?];
        descriptor.primitive.cull_mode = Some(bevy::render::render_resource::Face::Back);
        if key.bind_group_data.0 != 0 {
            use bevy::render::render_resource::{
                BlendComponent, BlendFactor, BlendOperation, BlendState,
            };
            if let Some(depth) = &mut descriptor.depth_stencil {
                depth.depth_write_enabled = Some(false);
            }
            if let Some(fragment) = &mut descriptor.fragment {
                for target in fragment.targets.iter_mut().flatten() {
                    target.blend = Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::SrcAlpha,
                            dst_factor: if key.bind_group_data.0 == 2 {
                                BlendFactor::OneMinusSrcAlpha
                            } else {
                                BlendFactor::One
                            },
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent {
                            // Retain opaque camera alpha after straight blending.
                            // Native X8 targets discard this channel; Bevy uses it
                            // when composing the encoded world camera.
                            src_factor: if key.bind_group_data.0 == 2 {
                                BlendFactor::One
                            } else {
                                BlendFactor::Zero
                            },
                            dst_factor: if key.bind_group_data.0 == 2 {
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
        Ok(())
    }
}
pub type Scene = crate::cinematic_scene::Scenes<Bodies>;
/// Animated normal geometry uses the measured fixed-function scenery material,
/// with normals rebuilt from each animated pose.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LitKey(u8, bool);
impl From<&LitBindings> for LitKey {
    fn from(value: &LitBindings) -> Self {
        Self(
            if value.additive {
                1
            } else if value.alpha {
                2
            } else {
                0
            },
            value.front,
        )
    }
}
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
#[bind_group_data(LitKey)]
pub struct LitBindings {
    alpha: bool,
    additive: bool,
    front: bool,
    #[texture(0)]
    #[sampler(1)]
    texture: Handle<Image>,
    #[storage(2, read_only)]
    parameters: Handle<bevy::render::storage::ShaderBuffer>,
}
pub type LitMaterial = crate::source_view::UniformMaterial<LitBindings>;
pub(crate) fn shared_lit_material(
    texture: Handle<Image>,
    parameters: Handle<bevy::render::storage::ShaderBuffer>,
    alpha: bool,
) -> LitMaterial {
    LitMaterial::from(LitBindings {
        alpha,
        additive: false,
        front: false,
        texture,
        parameters,
    })
}
impl LitMaterial {
    pub(crate) fn with_additive(mut self, additive: bool) -> Self {
        self.additive = additive;
        self
    }
    pub(crate) fn with_front(mut self, front: bool) -> Self {
        self.front = front;
        self
    }
    pub(crate) fn front(&self) -> bool {
        self.front
    }
    pub(crate) fn set_front(&mut self, front: bool) {
        self.front = front;
    }
}
impl bevy::pbr::Material for LitMaterial {
    fn vertex_shader() -> ShaderRef {
        "embedded://giants_app/gun.wesl".into()
    }
    fn fragment_shader() -> ShaderRef {
        Self::vertex_shader()
    }
    fn alpha_mode(&self) -> AlphaMode {
        if self.additive {
            AlphaMode::Add
        } else if self.alpha {
            AlphaMode::Blend
        } else {
            AlphaMode::Opaque
        }
    }
    fn specialize(
        _: &MaterialPipeline,
        d: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        d.vertex.buffers = vec![layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(2),
        ])?];
        d.primitive.cull_mode = Some(crate::source_view::cull_face(key.bind_group_data.1));
        if key.bind_group_data.0 != 0
            && let Some(depth) = &mut d.depth_stencil
        {
            depth.depth_write_enabled = Some(false);
        }
        if key.bind_group_data.0 == 1
            && let Some(fragment) = &mut d.fragment
        {
            use bevy::render::render_resource::{
                BlendComponent, BlendFactor, BlendOperation, BlendState,
            };
            for target in fragment.targets.iter_mut().flatten() {
                target.blend = Some(BlendState {
                    color: BlendComponent {
                        src_factor: BlendFactor::SrcAlpha,
                        dst_factor: BlendFactor::One,
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
pub struct Bodies {
    pub(crate) attachment_requests: Vec<giants_game::attachment_resources::Request>,
    actors: Vec<giants_game::cast::Actor>,
    surfaces: Vec<Vec<Result<Surface, &'static str>>>,
    initial: Vec<Vec<[f32; 3]>>,
    draws: Vec<(usize, usize, Handle<Mesh>)>,
    lit_materials: Vec<(usize, usize, Handle<LitMaterial>)>,
    rgb_materials: Vec<(usize, usize, Handle<Material>, u32)>,
    cube_materials: Vec<(usize, usize, f32, u8, Handle<crate::sword_scene::Material>)>,
    context_materials: Vec<Handle<crate::body_context::Material>>,
    context_images: std::collections::BTreeMap<(i32, String), giants_formats::tga::RgbImage>,
    lights: Vec<giants_game::lighting::Directional>,
    fog: giants_game::lighting::Fog,
    tick: i32,
}
impl Scene {
    pub fn needs_cube(&self) -> bool {
        self.any(|body| {
            body.surfaces
                .iter()
                .flatten()
                .any(|surface| surface.as_ref().is_ok_and(|s| s.cube_profile().is_some()))
        })
    }
    pub fn load(
        root: &std::path::Path,
        cinematic: &crate::cinematic_camera::Cinematic,
        slot: Option<usize>,
        resources: &giants_game::scene_resources::SceneResources,
        config: &giants_formats::world::WorldConfig,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut cache = giants_game::cast::RegisteredBodies::new(resources);
        let mut attachments = giants_game::attachment_resources::Registered::new(resources);
        Self::prepare(cinematic, |segment| {
            Bodies::load(root, segment, slot, &mut cache, &mut attachments, config)
        })
    }
}
#[derive(Component)]
pub struct Rendered;
impl Bodies {
    fn load(
        root: &std::path::Path,
        cinematic: &crate::cinematic_camera::Segment,
        slot: Option<usize>,
        cache: &mut giants_game::cast::RegisteredBodies<'_>,
        attachments: &mut giants_game::attachment_resources::Registered<'_>,
        config: &giants_formats::world::WorldConfig,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let bytes = std::fs::read(giants_game::flick::sequence_path(
            root,
            cinematic.sequence(),
        )?)?;
        let cast = cache.sequence(&bytes)?;
        let attachment_requests = attachments.sequence(&bytes)?;
        for request in &attachment_requests {
            let pose = request.matrix(cinematic.animation(), cinematic.tick());
            let error = request.asset.as_ref().err().cloned().or_else(|| pose.err());
            println!(
                "cinematic_common_attachment_request sequence={} actor_slot={} class={} offset={} links={} geometry_decoded={} inspection_matrix_calculated={} common_created=false common_drawn=false reason={:?}",
                cinematic.sequence(),
                request.slot,
                request.class,
                request.offset,
                request.links.len(),
                request.asset.is_ok(),
                error.is_none(),
                error.unwrap_or_default()
            );
            if let Ok(asset) = &request.asset {
                println!(
                    "cinematic_child_handle_policy sequence={} actor_slot={} class={} policy={:?} profile=initial_declared_resources live=false",
                    cinematic.sequence(),
                    request.slot,
                    request.class,
                    asset.child_handle_policy
                );
                let mut inputs = vec![
                    (
                        "class_catalog",
                        asset.source.class_catalog.clone(),
                        String::new(),
                    ),
                    (
                        "spec_catalog",
                        asset.source.spec_catalog.clone(),
                        String::new(),
                    ),
                    (
                        "mesh",
                        asset.source.mesh.archive.clone(),
                        asset.source.mesh.member.clone(),
                    ),
                ];
                if let Some(a) = &asset.source.animation {
                    inputs.push(("animation", a.archive.clone(), a.member.clone()));
                }
                for t in asset.source.textures.values() {
                    inputs.push(("texture", t.archive.clone(), t.member.clone()));
                }
                for (role, file, member) in inputs {
                    println!(
                        "cinematic_common_attachment_input sequence={} actor_slot={} class={} role={} file={:?} member={:?}",
                        cinematic.sequence(),
                        request.slot,
                        request.class,
                        role,
                        file.file_name()
                            .ok_or("attachment input filename absent")?
                            .to_string_lossy(),
                        member
                    );
                }
            }
        }
        for request in &cast.requests {
            println!(
                "cinematic_asset_request sequence={} offset={} actor_slot={} class={} named={} result={:?}",
                cinematic.sequence(),
                request.offset,
                request.slot,
                request.class,
                request.tree.is_some(),
                request.result
            );
            if request.result.is_err() && !cinematic.preview() {
                return Err(format!("registered body request failed: {:?}", request.result).into());
            }
        }
        let mut actors = cast.actors;
        actors.retain(|actor| slot.is_none_or(|slot| actor.slot == slot));
        let mut surfaces = Vec::new();
        let lights = giants_game::lighting::first_world(config)?;
        let fog = config.fog;
        let mut context_images = std::collections::BTreeMap::new();
        for actor in &actors {
            if let Some(source) = &actor.body.source {
                for (role, path) in [
                    ("class_catalog", &source.class_catalog),
                    ("spec_catalog", &source.spec_catalog),
                    ("mesh_archive", &source.mesh.archive),
                    ("skin_archive", &source.skin.archive),
                ] {
                    println!(
                        "cinematic_body_input sequence={} actor_slot={} class={} role={} file={}",
                        cinematic.sequence(),
                        actor.slot,
                        actor.object_type,
                        role,
                        path.file_name()
                            .expect("source resource basename")
                            .to_string_lossy()
                    );
                }
                for texture in source.textures.values() {
                    println!(
                        "cinematic_body_input sequence={} actor_slot={} class={} role=texture_archive file={}",
                        cinematic.sequence(),
                        actor.slot,
                        actor.object_type,
                        texture
                            .archive
                            .file_name()
                            .expect("source texture basename")
                            .to_string_lossy()
                    );
                }
                for request in source.secondary_textures.values() {
                    if let Some(selected) = &request.selected {
                        println!(
                            "cinematic_body_input sequence={} actor_slot={} class={} role=secondary_archive file={}",
                            cinematic.sequence(),
                            actor.slot,
                            actor.object_type,
                            selected
                                .archive
                                .file_name()
                                .expect("secondary image archive")
                                .to_string_lossy()
                        );
                    }
                    if let Err(error) = &request.decoded {
                        eprintln!(
                            "cinematic_secondary_image_error sequence={} actor_slot={} member={:?} reason={error:?}",
                            cinematic.sequence(),
                            actor.slot,
                            request.member
                        );
                    }
                }
            }
            // Explicit measured benchmark inspection settings, not inferred
            // live game settings. Each part keeps its own routing decision.
            let normals = Normals::select(false, actor.body.mesh.normal_count != 0, false, true);
            let parts: Vec<_> = actor.body.mesh.subobjects.iter().enumerate().map(|(part, sub)| {
                if !sub.material.bump_texture.is_empty() {
                    println!("cinematic_part_dependency sequence={} actor_slot={} class={} part={} role=secondary_image member={:?}",
                        cinematic.sequence(), actor.slot, actor.object_type, part, format!("{}.tga", sub.material.bump_texture));
                }
                let mut result = giants_game::body_pipeline::preview_surface(normals, &sub.material,
                    actor.body.alpha_textures.contains(&sub.material.texture.to_ascii_lowercase()));
                if result == Ok(Surface::Context) {
                    let key = (actor.object_type,sub.material.bump_texture.to_ascii_lowercase());
                    let request = actor.body.source.as_ref().and_then(|source| source.secondary_textures.get(&key.1));
                    result = match request.and_then(|request| request.decoded.as_ref().ok()) {
                        Some(giants_formats::tga::Image::Plane8(image)) => {
                            crate::body_context::factor(&sub.material,&lights[0]).and_then(|_| {
                                if let std::collections::btree_map::Entry::Vacant(entry) = context_images.entry(key) {
                                    entry.insert(giants_game::bump_image::normal_map(image,true)?);
                                }
                                Ok(Surface::Context)
                            })
                        }
                        _ => Err("context image is missing or outside plane8 profile"),
                    };
                }
                println!("cinematic_part_request sequence={} actor_slot={} class={} part={} flags={:08x} route={:?} result={:?}",
                    cinematic.sequence(), actor.slot, actor.object_type, part, sub.material.flags,
                    normals.part(sub.material.flags), result);
                result
            }).collect();
            if !cinematic.preview()
                && let Some(error) = parts.iter().find_map(|part| part.as_ref().err())
            {
                return Err(format!(
                    "Actor part capability pending: class={} reason={error}",
                    actor.object_type
                )
                .into());
            }
            println!(
                "cinematic_body_parts sequence={} actor_slot={} class={} supported={} deferred={} model_slot=0",
                cinematic.sequence(),
                actor.slot,
                actor.object_type,
                parts.iter().filter(|p| p.is_ok()).count(),
                parts.iter().filter(|p| p.is_err()).count()
            );
            surfaces.push(parts);
        }
        let tick = cinematic.tick();
        let initial = actors
            .iter()
            .map(|a| a.positions(cinematic.animation(), tick))
            .collect::<Result<Vec<_>, _>>()?;
        for (actor_index, (actor, positions)) in actors.iter().zip(&initial).enumerate() {
            if actor.body.mesh.normal_count != 0 {
                let normals = prepared_normals(actor, positions)?;
                for (part, surface) in surfaces[actor_index].iter().enumerate() {
                    if *surface == Ok(Surface::Context) {
                        crate::body_context::colors(actor, positions, &normals, part, &lights[0])?;
                    }
                }
            }
            // Resource selection precedes part routing, generated context images
            // and vertex-color validation. Report their actual final CPU stage,
            // without claiming an entity, GPU submission or native policy.
            if let Some(source) = &actor.body.source {
                for request in source.secondary_textures.values() {
                    let (kind, size) = request
                        .decoded
                        .as_ref()
                        .map(|image| (image.profile(), image.dimensions()))
                        .unwrap_or(("unknown", [0, 0]));
                    let mut referenced_parts = 0;
                    let mut context_parts = 0;
                    for (part, sub) in actor.body.mesh.subobjects.iter().enumerate() {
                        if !sub.material.bump_texture.is_empty()
                            && format!("{}.tga", sub.material.bump_texture)
                                .eq_ignore_ascii_case(&request.member)
                        {
                            referenced_parts += 1;
                            if surfaces[actor_index][part] == Ok(Surface::Context)
                                && context_images.contains_key(&(
                                    actor.object_type,
                                    sub.material.bump_texture.to_ascii_lowercase(),
                                ))
                            {
                                context_parts += 1;
                            }
                        }
                    }
                    let prepared = request.decoded.is_ok()
                        && referenced_parts > 0
                        && context_parts == referenced_parts;
                    println!(
                        "cinematic_secondary_image sequence={} actor_slot={} class={} member={:?} selected={} kind={} decoded={} width={} height={} referenced_parts={} preview_context_parts={} cpu_prepared={} consumer_pending={} native_policy_verified=false",
                        cinematic.sequence(),
                        actor.slot,
                        actor.object_type,
                        request.member,
                        request.selected.is_some(),
                        kind,
                        request.decoded.is_ok(),
                        size[0],
                        size[1],
                        referenced_parts,
                        context_parts,
                        prepared,
                        !prepared
                    );
                }
            }
        }
        println!(
            "cinematic_bodies={} rigid_skin=true lod=0 geometry_source=flk_catalog_skn_anm material_profile=common_rgb_normals_opaque_cube per_part_capabilities=true attachments=false flk_vm=false",
            actors.len()
        );
        Ok(Self {
            attachment_requests,
            actors,
            surfaces,
            initial,
            draws: Vec::new(),
            lit_materials: Vec::new(),
            rgb_materials: Vec::new(),
            cube_materials: Vec::new(),
            context_materials: Vec::new(),
            context_images,
            lights,
            fog,
            tick,
        })
    }
}
#[derive(SystemParam)]
pub struct AssetsForScene<'w> {
    meshes: ResMut<'w, Assets<Mesh>>,
    images: ResMut<'w, Assets<Image>>,
    materials: ResMut<'w, Assets<Material>>,
    lit_materials: ResMut<'w, Assets<LitMaterial>>,
    cube_materials: Option<ResMut<'w, Assets<crate::sword_scene::Material>>>,
    context_materials: ResMut<'w, Assets<crate::body_context::Material>>,
    buffers: ResMut<'w, Assets<bevy::render::storage::ShaderBuffer>>,
}
fn expanded(actor: &giants_game::cast::Actor, positions: &[[f32; 3]], sub: usize) -> Vec<[f32; 3]> {
    actor.body.mesh.subobjects[sub]
        .triangles
        .iter()
        .flatten()
        .map(|i| {
            let p = positions[actor.body.mesh.indices[usize::from(*i)] as usize];
            if actor.body.mesh.normal_count != 0 {
                p
            } else {
                [p[0], p[2], -p[1]]
            }
        })
        .collect()
}
fn prepared_normals(
    actor: &giants_game::cast::Actor,
    positions: &[[f32; 3]],
) -> Result<Vec<[f32; 3]>, Box<dyn std::error::Error>> {
    let mesh = &actor.body.mesh;
    let profile = Normals::select(false, mesh.normal_count != 0, false, true);
    let flags = mesh
        .subobjects
        .iter()
        .fold(0, |flags, part| flags | part.material.flags);
    giants_game::body_pipeline::prepare_normals(
        mesh,
        positions,
        profile.contextual_preparation(flags),
    )
}
fn rgb_parameters(
    actor: &giants_game::cast::Actor,
    sub: usize,
    transparency: f32,
    no_fog: bool,
) -> Result<(f32, bool), &'static str> {
    let material = &actor.body.mesh.subobjects[sub].material;
    let texture_alpha = actor
        .body
        .alpha_textures
        .contains(&material.texture.to_ascii_lowercase());
    if transparency == 0. {
        if let Some(profile) = giants_game::body_pipeline::additive_rgb(material, texture_alpha) {
            return Ok((profile.alpha, false));
        }
        // Preserve the independently scoped neutral-object preview profile.
        let alpha = if material.flags & 1 != 0 || (actor.object_type == 3 && material.falloff > 0.)
        {
            ((1. - material.falloff) * 255.) as u8 as f32 / 255.
        } else {
            1.
        };
        return Ok((
            alpha,
            texture_alpha || (actor.object_type == 3 && material.falloff > 0.),
        ));
    }
    let pass = giants_game::render_parts::part_pass(
        giants_game::render_parts::Object {
            opacity: transparency,
            personality: 0,
            secondary_flags: 0,
            node_filter: false,
        },
        material.falloff,
        texture_alpha,
    );
    let (flags, constant, _) = giants_game::material_descriptor::object_base(
        giants_game::material_descriptor::InspectionInput {
            part_flags: material.flags,
            blend: material.blend,
            falloff: material.falloff,
            has_texture: !material.texture.is_empty(),
            texture_alpha,
            fog: !no_fog,
            pass_mask: pass.mask(),
        },
        transparency,
    )?;
    match flags & 0xf00 {
        0x100 => Ok(((constant >> 24) as f32 / 255., false)),
        0x300 => Ok(((constant >> 24) as f32 / 255., true)),
        _ => Err("object transparency outside measured RGB blend profile"),
    }
}
#[derive(SystemParam)]
pub(crate) struct SetupContext<'w> {
    view: Res<'w, crate::source_view::SourceView>,
    cube: Option<Res<'w, crate::reflection_cube::Build>>,
    exit: MessageWriter<'w, AppExit>,
}
pub fn setup(
    mut commands: Commands,
    mut scene: ResMut<Scene>,
    world: Res<crate::world_scene::Scene>,
    mut assets: AssetsForScene,
    cinematic: Res<crate::cinematic_camera::Cinematic>,
    context: SetupContext,
) {
    let SetupContext {
        view,
        cube,
        mut exit,
    } = context;
    let camera = world
        .validation_camera
        .as_ref()
        .expect("body scene requires cinematic camera");
    let mut texture_handles = std::collections::BTreeMap::new();
    let mut draws = Vec::new();
    let mut lit_materials = Vec::new();
    let mut rgb_materials = Vec::new();
    let mut cube_materials = Vec::new();
    let mut context_materials = Vec::new();
    let mut context_textures = std::collections::BTreeMap::new();
    for (key, source) in &scene.context_images {
        let alpha = source.alpha.as_ref().expect("normal image alpha");
        let rgba = source
            .pixels
            .as_chunks::<3>()
            .0
            .iter()
            .zip(alpha)
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
        println!(
            "cinematic_context_image_ready sequence={} class={} member={:?} converted=true staged=true",
            cinematic.sequence(),
            key.0,
            format!("{}.tga", key.1)
        );
    }
    for (index, actor) in scene.actors.iter().enumerate() {
        let normals = if actor.body.mesh.normal_count != 0 {
            Some(
                prepared_normals(actor, &scene.initial[index])
                    .expect("validated animated normal definitions"),
            )
        } else {
            None
        };
        for texture in &actor.body.textures {
            let key = (actor.object_type, texture.name.to_ascii_lowercase());
            if texture_handles.contains_key(&key) {
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
            texture_handles.insert(key, assets.images.add(image));
        }
        // The checked null-texture diffuse path needs no source image. A unit
        // sample leaves the existing diffuse-only RGB shader result unchanged.
        if actor.body.mesh.subobjects.iter().any(|p| {
            giants_game::body_pipeline::untextured_diffuse(&p.material)
                || giants_game::body_pipeline::lit_null_base(&p.material)
        }) {
            texture_handles
                .entry((actor.object_type, String::new()))
                .or_insert_with(|| {
                    assets.images.add(Image::new_fill(
                        bevy::render::render_resource::Extent3d {
                            width: 1,
                            height: 1,
                            depth_or_array_layers: 1,
                        },
                        bevy::render::render_resource::TextureDimension::D2,
                        &[255, 255, 255, 255],
                        bevy::render::render_resource::TextureFormat::Rgba8Unorm,
                        RenderAssetUsages::default(),
                    ))
                });
        }
        for (sub_index, sub) in actor.body.mesh.subobjects.iter().enumerate() {
            if cinematic.actor_transparency(actor.slot).unwrap_or(0.) != 0.
                && scene.surfaces[index][sub_index] != Ok(Surface::Rgb)
            {
                error!("Actor transparency requires an unimplemented non-RGB material consumer");
                exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
                return;
            }
            let Ok(surface) = scene.surfaces[index][sub_index] else {
                continue;
            };
            let uv: Vec<[f32; 2]> = sub
                .triangles
                .iter()
                .flatten()
                .map(|i| actor.body.mesh.uv[usize::from(*i)])
                .collect();
            let mut color: Vec<[f32; 4]> = sub
                .triangles
                .iter()
                .flatten()
                .map(|i| {
                    if sub.material.flags & 0x20 != 0 {
                        let rgb = actor.body.mesh.rgb[usize::from(*i)];
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
                .collect();
            if surface == Surface::Context {
                color = crate::body_context::colors(
                    actor,
                    &scene.initial[index],
                    normals.as_ref().expect("context normals"),
                    sub_index,
                    &scene.lights[0],
                )
                .expect("validated context vertex colors");
            }
            let mut mesh = Mesh::new(
                PrimitiveTopology::TriangleList,
                RenderAssetUsages::default(),
            )
            .with_inserted_attribute(
                Mesh::ATTRIBUTE_POSITION,
                expanded(actor, &scene.initial[index], sub_index),
            )
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
            .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, color);
            if let Some(normals) = &normals {
                mesh.insert_attribute(
                    Mesh::ATTRIBUTE_NORMAL,
                    sub.triangles
                        .iter()
                        .flatten()
                        .map(|i| normals[usize::from(*i)])
                        .collect::<Vec<_>>(),
                );
            }
            let handle = assets.meshes.add(mesh);
            draws.push((index, sub_index, handle.clone()));
            if surface == Surface::Context {
                let material = assets
                    .context_materials
                    .add(crate::body_context::Material::from(
                        crate::body_context::Bindings {
                            front: false,
                            source_world: Mat4::IDENTITY,
                            base: texture_handles
                                [&(actor.object_type, sub.material.texture.to_ascii_lowercase())]
                                .clone(),
                            normal: context_textures[&(
                                actor.object_type,
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
                            factor: crate::body_context::factor(&sub.material, &scene.lights[0])
                                .expect("context primary profile"),
                            fog: Vec4::new(
                                scene.fog.start,
                                scene.fog.end,
                                if world.no_fog { 0. } else { 1. },
                                0.,
                            ),
                        },
                    ));
                context_materials.push(material.clone());
                commands.spawn((
                    Rendered,
                    crate::cinematic_visibility::ActorSlot(actor.slot),
                    Name::new(format!("FLK context overlay {} / {}", actor.slot, sub.name)),
                    Mesh3d(handle.clone()),
                    MeshMaterial3d(material),
                    bevy::camera::visibility::NoFrustumCulling,
                ));
            }
            if let Some((factor, blend)) = surface.cube_profile() {
                let parameters = crate::sword_scene::source_parameters(
                    &sub.material,
                    Mat4::IDENTITY,
                    camera,
                    &scene.lights,
                    scene.fog,
                    world.no_fog,
                )
                .cube_factor(Some(factor))
                .blended(blend != 0);
                let parameters = assets.buffers.add(parameters.buffer());
                let material = assets
                    .cube_materials
                    .as_mut()
                    .expect("cube material installed")
                    .add(
                        crate::sword_scene::Material::new(
                            texture_handles
                                [&(actor.object_type, sub.material.texture.to_ascii_lowercase())]
                                .clone(),
                            cube.as_ref()
                                .and_then(|build| build.handle())
                                .expect("prepared body reflection cube"),
                            parameters,
                        )
                        .with_blend(blend),
                    );
                cube_materials.push((index, sub_index, factor, blend, material.clone()));
                commands.spawn((
                    Rendered,
                    crate::cinematic_visibility::ActorSlot(actor.slot),
                    Name::new(format!("FLK cube actor {} / {}", actor.slot, sub.name)),
                    Mesh3d(handle),
                    MeshMaterial3d(material),
                    bevy::camera::visibility::NoFrustumCulling,
                ));
                continue;
            }
            if surface == Surface::Lit
                || surface == Surface::Context
                || surface.lit_additive().is_some()
            {
                let alpha = actor
                    .body
                    .alpha_textures
                    .contains(&sub.material.texture.to_ascii_lowercase());
                let parameters = crate::gun_scene::source_parameters(
                    &sub.material,
                    Mat4::IDENTITY,
                    camera,
                    &scene.lights,
                    scene.fog,
                    world.no_fog,
                )
                .white_texture_factor(alpha)
                .untextured(sub.material.texture.is_empty())
                .suppress_primary_diffuse(surface == Surface::Context);
                let parameters = parameters.additive_profile(surface.lit_additive());
                let parameters = assets.buffers.add(parameters.buffer());
                let material = assets.lit_materials.add(LitMaterial::from(LitBindings {
                    alpha,
                    additive: surface.lit_additive().is_some(),
                    front: false,
                    texture: texture_handles
                        [&(actor.object_type, sub.material.texture.to_ascii_lowercase())]
                        .clone(),
                    parameters,
                }));
                lit_materials.push((index, sub_index, material.clone()));
                commands.spawn((
                    Rendered,
                    crate::cinematic_visibility::ActorSlot(actor.slot),
                    Name::new(format!("FLK lit actor {} / {}", actor.slot, sub.name)),
                    Mesh3d(handle),
                    MeshMaterial3d(material),
                    bevy::camera::visibility::NoFrustumCulling,
                ));
                continue;
            }
            let transparency = cinematic.actor_transparency(actor.slot).unwrap_or(0.);
            let (alpha, straight) =
                match rgb_parameters(actor, sub_index, transparency, world.no_fog) {
                    Ok(value) => value,
                    Err(error) => {
                        error!("RGB actor transparency failed: {error}");
                        exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
                        return;
                    }
                };
            let material = assets.materials.add(Material::from(Bindings {
                texture: texture_handles
                    [&(actor.object_type, sub.material.texture.to_ascii_lowercase())]
                    .clone(),
                clip_from_bevy: view.buffer.clone(),
                fog: Vec4::new(
                    scene.fog.start,
                    scene.fog.end,
                    if world.no_fog { 0. } else { 1. },
                    0.,
                ),
                fog_color: if sub.material.flags & 1 != 0 {
                    Vec4::ZERO
                } else {
                    Vec4::new(
                        scene.fog.color[0] as f32 / 255.,
                        scene.fog.color[1] as f32 / 255.,
                        scene.fog.color[2] as f32 / 255.,
                        0.,
                    )
                },
                blend: straight,
                force_additive: giants_game::body_pipeline::additive_rgb(&sub.material, false)
                    .is_some(),
                validation: Vec4::new(
                    1.,
                    1. / camera.viewport.z as f32,
                    -1. / camera.viewport.w as f32,
                    alpha,
                ),
            }));
            rgb_materials.push((index, sub_index, material.clone(), transparency.to_bits()));
            if transparency != 0.
                || cinematic
                    .program()
                    .commands
                    .iter()
                    .any(|c| c.opcode == 42 && c.words[0] == actor.slot as u32)
            {
                println!(
                    "cinematic_actor_transparency sequence={} actor_slot={} part={} value_bits={} factor_bits={} straight={}",
                    cinematic.sequence(),
                    actor.slot,
                    sub_index,
                    transparency.to_bits(),
                    alpha.to_bits(),
                    straight
                );
            }
            commands.spawn((
                Rendered,
                crate::cinematic_visibility::ActorSlot(actor.slot),
                Name::new(format!(
                    "FLK actor {} {} / {}",
                    actor.slot, actor.tree, sub.name
                )),
                Mesh3d(handle),
                MeshMaterial3d(material),
                if transparency == 1. {
                    Visibility::Hidden
                } else {
                    Visibility::Inherited
                },
                bevy::camera::visibility::NoFrustumCulling,
            ));
        }
    }
    scene.draws = draws;
    scene.lit_materials = lit_materials;
    scene.rgb_materials = rgb_materials;
    scene.cube_materials = cube_materials;
    scene.context_materials = context_materials;
    println!(
        "cinematic_common_attachment_state sequence={} director={} retained={} common_created=false common_drawn=false",
        cinematic.sequence(),
        cinematic.director_index(),
        scene.attachment_requests.len()
    );
    println!(
        "cinematic_context_draws sequence={} overlays={} direction=primary_variant1_world_baked mode=directional_dot3",
        cinematic.sequence(),
        scene.context_materials.len()
    );
    info!(
        "cinematic_render_ready=actor_scene director={} meshes={}",
        cinematic.director_index(),
        scene.draws.len()
    );
    scene.initial.clear();
}
pub fn update(
    cinematic: Res<crate::cinematic_camera::Cinematic>,
    world: Res<crate::world_scene::Scene>,
    mut scene: ResMut<Scene>,
    mut assets: AssetsForScene,
    mut exit: MessageWriter<AppExit>,
    rendered: Query<Entity, With<Rendered>>,
) {
    let profile_started = std::time::Instant::now();
    if rendered.iter().count() != scene.draws.len() + scene.context_materials.len() {
        error!("Director render entity count differs: actor_scene");
        exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
        return;
    }

    for (index, actor) in scene.actors.iter().enumerate() {
        if cinematic.actor_transparency(actor.slot).unwrap_or(0.) != 0.
            && scene.surfaces[index]
                .iter()
                .any(|surface| *surface != Ok(Surface::Rgb))
        {
            error!("Actor transparency requires an unimplemented non-RGB material consumer");
            exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
            return;
        }
    }
    let changes: Vec<_> = scene
        .rgb_materials
        .iter()
        .enumerate()
        .filter_map(|(entry, (index, sub, handle, last))| {
            let actor = &scene.actors[*index];
            let value = cinematic.actor_transparency(actor.slot).unwrap_or(0.);
            (value.to_bits() != *last).then_some((entry, *index, *sub, handle.clone(), value))
        })
        .collect();
    for (entry, index, sub, handle, value) in changes {
        let actor = &scene.actors[index];
        let (alpha, straight) = match rgb_parameters(actor, sub, value, world.no_fog) {
            Ok(parameters) => parameters,
            Err(error) => {
                error!("RGB actor transparency update failed: {error}");
                exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
                return;
            }
        };
        if let Some(mut material) = assets.materials.get_mut(&handle) {
            material.validation.w = alpha;
            material.blend = straight;
        }
        let slot = actor.slot;
        scene.rgb_materials[entry].3 = value.to_bits();
        println!(
            "cinematic_actor_transparency sequence={} actor_slot={} part={} value_bits={} factor_bits={} straight={}",
            cinematic.sequence(),
            slot,
            sub,
            value.to_bits(),
            alpha.to_bits(),
            straight
        );
    }
    let tick = cinematic.tick();
    if tick == scene.tick && !world.is_changed() {
        return;
    }
    let mut poses = Vec::new();
    let mut normals = Vec::new();
    for actor in &scene.actors {
        match actor.positions(cinematic.animation(), tick) {
            Ok(p) => {
                let n = if actor.body.mesh.normal_count != 0 {
                    match prepared_normals(actor, &p) {
                        Ok(n) => Some(n),
                        Err(error) => {
                            error!("Actor normal calculation failed: {}", error);
                            exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
                            return;
                        }
                    }
                } else {
                    None
                };
                normals.push(n);
                poses.push(p);
            }
            Err(error) => {
                error!("Actor skinning failed: {}", error);
                exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
                return;
            }
        }
    }
    for (actor, sub, handle) in &scene.draws {
        if let Some(mut mesh) = assets.meshes.get_mut(handle) {
            mesh.insert_attribute(
                Mesh::ATTRIBUTE_POSITION,
                expanded(&scene.actors[*actor], &poses[*actor], *sub),
            );
            if let Some(normals) = &normals[*actor] {
                mesh.insert_attribute(
                    Mesh::ATTRIBUTE_NORMAL,
                    scene.actors[*actor].body.mesh.subobjects[*sub]
                        .triangles
                        .iter()
                        .flatten()
                        .map(|i| normals[usize::from(*i)])
                        .collect::<Vec<_>>(),
                );
                if scene.surfaces[*actor][*sub] == Ok(Surface::Context) {
                    let colors = match crate::body_context::colors(
                        &scene.actors[*actor],
                        &poses[*actor],
                        normals,
                        *sub,
                        &scene.lights[0],
                    ) {
                        Ok(colors) => colors,
                        Err(error) => {
                            error!("Context color preparation failed: {}", error);
                            exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
                            return;
                        }
                    };
                    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
                }
            }
        }
    }
    if let Some(camera) = &world.validation_camera {
        for (actor, sub, handle) in &scene.lit_materials {
            if let Some(material) = assets.lit_materials.get(handle) {
                let handle = material.parameters.clone();
                let parameters = crate::gun_scene::source_parameters(
                    &scene.actors[*actor].body.mesh.subobjects[*sub].material,
                    Mat4::IDENTITY,
                    camera,
                    &scene.lights,
                    scene.fog,
                    world.no_fog,
                )
                .white_texture_factor(material.alpha)
                .untextured(
                    scene.actors[*actor].body.mesh.subobjects[*sub]
                        .material
                        .texture
                        .is_empty(),
                )
                .suppress_primary_diffuse(scene.surfaces[*actor][*sub] == Ok(Surface::Context));
                let parameters = parameters.additive_profile(
                    scene.surfaces[*actor][*sub]
                        .ok()
                        .and_then(Surface::lit_additive),
                );
                if let Some(mut buffer) = assets.buffers.get_mut(&handle) {
                    parameters.write_buffer(&mut buffer);
                }
            }
        }
        for (actor, sub, factor, blend, handle) in &scene.cube_materials {
            if let Some(material) = assets
                .cube_materials
                .as_ref()
                .and_then(|materials| materials.get(handle))
            {
                let buffer = material.parameters.clone();
                let parameters = crate::sword_scene::source_parameters(
                    &scene.actors[*actor].body.mesh.subobjects[*sub].material,
                    Mat4::IDENTITY,
                    camera,
                    &scene.lights,
                    scene.fog,
                    world.no_fog,
                )
                .cube_factor(Some(*factor))
                .blended(*blend != 0);
                if let Some(mut buffer) = assets.buffers.get_mut(&buffer) {
                    parameters.write_buffer(&mut buffer);
                }
            }
        }
    }
    scene.tick = tick;
    if let Some(camera) = &world.validation_camera {
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
        for handle in &scene.context_materials {
            if let Some(mut material) = assets.context_materials.get_mut(handle)
                && (material.pixel != pixel || material.fog != fog)
            {
                material.pixel = pixel;
                material.fog = fog;
            }
        }
    }
    if std::env::var_os("GIANTS_PROFILE_FRAME").is_some() {
        info!(
            "frame_cpu_system=actor_scene micros={}",
            profile_started.elapsed().as_micros()
        );
    }
}

pub fn switch(
    mut commands: Commands,
    cinematic: Res<crate::cinematic_camera::Cinematic>,
    mut scene: ResMut<Scene>,
    rendered: Query<Entity, With<Rendered>>,
    mut exit: MessageWriter<AppExit>,
) {
    match scene.select(cinematic.director_index()) {
        Ok(false) => return,
        Ok(true) => {}
        Err(error) => {
            error!("Director scene selection failed: {}", error);
            exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
            return;
        }
    }
    for entity in &rendered {
        commands.entity(entity).despawn();
    }

    scene.initial = match scene
        .actors
        .iter()
        .map(|a| a.positions(cinematic.animation(), cinematic.tick()))
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(poses) => poses,
        Err(error) => {
            error!("Director body setup failed: {}", error);
            exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
            return;
        }
    };
    scene.draws.clear();
    scene.lit_materials.clear();
    scene.rgb_materials.clear();
    scene.cube_materials.clear();
    scene.context_materials.clear();

    scene.tick = i32::MIN;
    info!(
        "cinematic_scene_switch=actor_scene director={} sequence={}",
        cinematic.director_index(),
        cinematic.sequence()
    );
    commands.run_system_cached(setup);
}
