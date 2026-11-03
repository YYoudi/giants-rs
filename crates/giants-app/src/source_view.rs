//! Shared source projection: changing the camera does not invalidate materials.
use bevy::{prelude::*, render::storage::ShaderBuffer};

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ViewUpdate;

#[derive(bevy::ecs::system::SystemParam)]
pub struct PixelMaterials<'w> {
    rgb: Option<ResMut<'w, Assets<crate::prop_scene::Material>>>,
    actors: Option<ResMut<'w, Assets<crate::actor_scene::Material>>>,
    terrain: Option<ResMut<'w, Assets<crate::world_scene::WorldColorMaterial>>>,
}

#[derive(Resource)]
pub struct SourceView {
    pub buffer: Handle<ShaderBuffer>,
    pub source_buffer: Handle<ShaderBuffer>,
    pub transforms: Handle<ShaderBuffer>,
    clip: Mat4,
    viewport: UVec2,
}
/// Precompose source coordinates identically for the lit base and its overlay.
/// Algebraically equivalent shader-side axis changes can round depth differently.
pub fn source_clip(clip_from_bevy: Mat4) -> Mat4 {
    clip_from_bevy * Mat4::from_cols(Vec4::X, -Vec4::Z, Vec4::Y, Vec4::W)
}
impl FromWorld for SourceView {
    fn from_world(world: &mut World) -> Self {
        let clip = world
            .get_resource::<crate::world_scene::Scene>()
            .and_then(|scene| scene.validation_camera.as_ref())
            .map_or(Mat4::IDENTITY, |camera| camera.clip_from_bevy());
        let mut projection = ShaderBuffer::from(vec![clip.to_cols_array()]);
        projection.buffer_usage |= bevy::render::render_resource::BufferUsages::UNIFORM;
        let buffer = world.resource_mut::<Assets<ShaderBuffer>>().add(projection);
        let mut source_projection = ShaderBuffer::from(vec![source_clip(clip).to_cols_array()]);
        source_projection.buffer_usage |= bevy::render::render_resource::BufferUsages::UNIFORM;
        let source_buffer = world
            .resource_mut::<Assets<ShaderBuffer>>()
            .add(source_projection);
        let transforms = world
            .resource_mut::<Assets<ShaderBuffer>>()
            .add(ShaderBuffer::from(vec![Mat4::IDENTITY.to_cols_array()]));
        let viewport = world
            .get_resource::<crate::world_scene::Scene>()
            .and_then(|scene| scene.validation_camera.as_ref())
            .map_or(UVec2::ZERO, |camera| {
                UVec2::new(camera.viewport.z, camera.viewport.w)
            });
        Self {
            buffer,
            source_buffer,
            transforms,
            clip,
            viewport,
        }
    }
}
pub fn update(
    scene: Option<Res<crate::world_scene::Scene>>,
    mut view: ResMut<SourceView>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
    mut pixels: PixelMaterials,
) {
    let Some(camera) = scene.as_ref().and_then(|s| s.validation_camera.as_ref()) else {
        return;
    };
    let clip = camera.clip_from_bevy();
    let viewport = UVec2::new(camera.viewport.z, camera.viewport.w);
    if view.viewport != viewport {
        let x = 1. / viewport.x as f32;
        let y = -1. / viewport.y as f32;
        if let Some(rgb) = pixels.rgb.as_mut() {
            for (_, material) in rgb.iter_mut() {
                material.pixel_offset.x = x;
                material.pixel_offset.y = y;
            }
        }
        if let Some(actors) = pixels.actors.as_mut() {
            for (_, material) in actors.iter_mut() {
                material.validation.y = x;
                material.validation.z = y;
            }
        }
        if let Some(terrain) = pixels.terrain.as_mut() {
            for (_, material) in terrain.iter_mut() {
                material.validation.y = x;
                material.validation.z = y;
            }
        }
        view.viewport = viewport;
    }
    if view.clip == clip {
        return;
    }
    {
        let mut buffer = buffers.get_mut(&view.buffer).expect("Source view buffer");
        buffer.clear();
        buffer.extend_from_slice(&[clip.to_cols_array()]);
    }
    let mut source_buffer = buffers
        .get_mut(&view.source_buffer)
        .expect("Source-coordinate view buffer");
    source_buffer.clear();
    source_buffer.extend_from_slice(&[source_clip(clip).to_cols_array()]);
    view.clip = clip;
}

/// Delegate resource tracking to Bevy and bind selected slots as stable uniforms.
/// The bit mask defaults to slot 2; terrain uses slots 3, 5 and 8.
/// This keeps shader arithmetic in the uniform address space of the reference
/// implementation instead of changing it to per-invocation storage loads.
#[derive(Asset, TypePath, Debug, Clone, Deref, DerefMut)]
pub struct UniformMaterial<
    T: Asset + bevy::render::render_resource::AsBindGroup,
    const SLOTS: u32 = 4,
>(pub T);
impl<T: Asset + bevy::render::render_resource::AsBindGroup, const SLOTS: u32> From<T>
    for UniformMaterial<T, SLOTS>
{
    fn from(value: T) -> Self {
        Self(value)
    }
}
impl<T: Asset + bevy::render::render_resource::AsBindGroup, const SLOTS: u32>
    bevy::render::render_resource::AsBindGroup for UniformMaterial<T, SLOTS>
{
    type Data = T::Data;
    type Param = T::Param;
    fn label() -> &'static str {
        T::label()
    }
    fn bind_group_data(&self) -> Self::Data {
        self.0.bind_group_data()
    }
    fn build_bind_group(
        &self,
        layout: &bevy::render::render_resource::BindGroupLayout,
        device: &bevy::render::renderer::RenderDevice,
        param: &mut bevy::ecs::system::SystemParamItem<'_, '_, Self::Param>,
        force_no_bindless: bool,
        output: &mut bevy::render::render_resource::BindGroupBuilder,
    ) -> Result<(), bevy::render::render_resource::AsBindGroupError> {
        self.0
            .build_bind_group(layout, device, param, force_no_bindless, output)
    }
    fn bind_group_layout_entries(
        device: &bevy::render::renderer::RenderDevice,
        force_no_bindless: bool,
    ) -> Vec<bevy::render::render_resource::BindGroupLayoutEntry> {
        use bevy::render::render_resource::{BindingType, BufferBindingType};
        let mut entries = T::bind_group_layout_entries(device, force_no_bindless);
        let mut found = 0;
        for entry in &mut entries {
            if entry.binding < 32 && SLOTS & (1 << entry.binding) != 0 {
                let BindingType::Buffer { ref mut ty, .. } = entry.ty else {
                    panic!("Shared uniform must be a buffer");
                };
                *ty = BufferBindingType::Uniform;
                found |= 1 << entry.binding;
            }
        }
        assert_eq!(
            found, SLOTS,
            "Shared uniform slots must exist in the layout"
        );
        entries
    }
}

pub fn uniform_buffer<T>(value: &T) -> ShaderBuffer
where
    T: bevy::render::render_resource::ShaderType
        + bevy::render::render_resource::encase::internal::WriteInto,
{
    let mut encoded = bevy::render::render_resource::encase::UniformBuffer::new(Vec::<u8>::new());
    encoded.write(value).expect("Source uniform layout");
    let mut buffer = ShaderBuffer::from(encoded.into_inner());
    buffer.buffer_usage |= bevy::render::render_resource::BufferUsages::UNIFORM;
    buffer
}
pub fn write_uniform<T>(value: &T, buffer: &mut ShaderBuffer)
where
    T: bevy::render::render_resource::ShaderType
        + bevy::render::render_resource::encase::internal::WriteInto,
{
    let mut encoded = bevy::render::render_resource::encase::UniformBuffer::new(Vec::<u8>::new());
    encoded.write(value).expect("Source uniform layout");
    buffer.clear();
    buffer.extend_from_slice(&encoded.into_inner());
}

/// D wrapper/device adapter: a reflected matrix selects front-face culling.
/// The caller has already applied any personality override to this boolean.
pub(crate) fn cull_face(front: bool) -> bevy::render::render_resource::Face {
    if front {
        bevy::render::render_resource::Face::Front
    } else {
        bevy::render::render_resource::Face::Back
    }
}

#[cfg(test)]
mod cull_tests {
    use super::*;
    use bevy::pbr::Material;
    #[test]
    fn source_winding_keeps_lighting_and_reflection_blend_modes_independent() {
        use bevy::render::render_resource::Face;
        // Exact D wrapper/device adapter fixtures; reflected, no personality override.
        for (front, face) in [(false, Face::Back), (true, Face::Front)] {
            assert_eq!(cull_face(front), face);
            for alpha in [false, true] {
                let mut m = crate::actor_scene::shared_lit_material(
                    Default::default(),
                    Default::default(),
                    alpha,
                )
                .with_front(front);
                let expected = if alpha {
                    AlphaMode::Blend
                } else {
                    AlphaMode::Opaque
                };
                assert_eq!(m.alpha_mode(), expected);
                assert_eq!(m.front(), front);
                m.set_front(!front);
                assert_eq!(m.front(), !front);
                assert_eq!(m.alpha_mode(), expected);
            }
            for (blend, expected) in [
                (0, AlphaMode::Opaque),
                (1, AlphaMode::Add),
                (2, AlphaMode::Blend),
            ] {
                let mut m = crate::sword_scene::Material::new(
                    Default::default(),
                    Default::default(),
                    Default::default(),
                )
                .with_blend(blend)
                .with_front(front);
                assert_eq!(m.alpha_mode(), expected);
                assert_eq!(m.front(), front);
                m.set_front(!front);
                assert_eq!(m.front(), !front);
                assert_eq!(m.alpha_mode(), expected);
            }
        }
    }
}
