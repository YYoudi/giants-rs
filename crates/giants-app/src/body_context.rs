//! Directional context overlay. Shared camera, source geometry and checked CPU
//! image/color kernels. Live light callbacks and full native parity are separate.
use bevy::{
    mesh::MeshVertexBufferLayoutRef,
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    reflect::TypePath,
    render::render_resource::{
        AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError,
    },
    shader::ShaderRef,
};

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
#[bind_group_data(Key)]
pub struct Bindings {
    pub front: bool,
    #[texture(0)]
    #[sampler(1)]
    pub base: Handle<Image>,
    #[texture(2)]
    #[sampler(3)]
    pub normal: Handle<Image>,
    #[storage(4, read_only)]
    pub clip: Handle<bevy::render::storage::ShaderBuffer>,
    #[uniform(5)]
    pub pixel: Vec4,
    #[uniform(6)]
    pub factor: Vec4,
    #[uniform(7)]
    pub fog: Vec4,
    #[uniform(8)]
    pub source_world: Mat4,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Key(bool);
impl From<&Bindings> for Key {
    fn from(value: &Bindings) -> Self {
        Self(value.front)
    }
}
pub type Material = crate::source_view::UniformMaterial<Bindings, 16>;
impl bevy::pbr::Material for Material {
    fn vertex_shader() -> ShaderRef {
        "embedded://giants_app/body_context.wesl".into()
    }
    fn fragment_shader() -> ShaderRef {
        Self::vertex_shader()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Add
    }
    fn specialize(
        _pipeline: &MaterialPipeline,
        _descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        use bevy::render::render_resource::*;
        _descriptor.vertex.buffers = vec![layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(1),
            Mesh::ATTRIBUTE_COLOR.at_shader_location(2),
        ])?];
        _descriptor.primitive.cull_mode =
            Some(crate::source_view::cull_face(_key.bind_group_data.0));
        if let Some(depth) = &mut _descriptor.depth_stencil {
            depth.depth_write_enabled = Some(false);
        }
        if let Some(fragment) = &mut _descriptor.fragment {
            for target in fragment.targets.iter_mut().flatten() {
                target.blend = Some(BlendState {
                    color: BlendComponent {
                        src_factor: BlendFactor::SrcAlpha,
                        dst_factor: BlendFactor::One,
                        operation: BlendOperation::Add,
                    },
                    alpha: BlendComponent {
                        src_factor: BlendFactor::One,
                        dst_factor: BlendFactor::One,
                        operation: BlendOperation::Add,
                    },
                });
            }
        }
        Ok(())
    }
}

pub fn colors(
    actor: &giants_game::cast::Actor,
    positions: &[[f32; 3]],
    normals: &[[f32; 3]],
    part: usize,
    light: &giants_game::lighting::Directional,
) -> Result<Vec<[f32; 4]>, &'static str> {
    let direction = light.direction.map(|v| -v);
    mesh_colors(&actor.body.mesh, positions, normals, part, direction)
}

pub(crate) fn mesh_colors(
    mesh: &giants_formats::gbs::Mesh,
    positions: &[[f32; 3]],
    normals: &[[f32; 3]],
    part: usize,
    direction: [f32; 3],
) -> Result<Vec<[f32; 4]>, &'static str> {
    let basis = giants_game::uv_basis::prepare(mesh, positions, part)?;
    mesh.subobjects[part]
        .triangles
        .iter()
        .flatten()
        .map(|&index| {
            let i = usize::from(index);
            let packed = giants_game::body_pipeline::context_color(
                direction,
                normals[i],
                basis.du[i - basis.vertex_start],
                basis.dv[i - basis.vertex_start],
            )?;
            Ok([
                ((packed >> 16) & 255) as f32 / 255.,
                ((packed >> 8) & 255) as f32 / 255.,
                (packed & 255) as f32 / 255.,
                1.,
            ])
        })
        .collect()
}

/// D's context factor carries only the product alpha. Its renderer disables
/// the GOG factor-RGB stage. Live light selection remains a separate input.
pub fn factor(
    material: &giants_formats::gbs::Material,
    light: &giants_game::lighting::Directional,
) -> Result<Vec4, &'static str> {
    if light.variant != 1
        || light
            .color
            .iter()
            .any(|v| !v.is_finite() || !(0. ..=1.).contains(v))
    {
        return Err("context primary light outside unit directional inspection profile");
    }
    let m = material.colors[2].to_le_bytes();
    let alpha = ((u32::from(m[0]).pow(2) + u32::from(m[1]).pow(2) + u32::from(m[2]).pow(2)) as f64)
        .sqrt()
        .trunc()
        .min(255.) as u32;
    let mut light_bytes = [255; 4];
    for (out, color) in light_bytes.iter_mut().zip(light.color) {
        *out = (f64::from(color) * 255.).trunc() as u8;
    }
    // The native light setter stores a clamped RGB magnitude as float alpha.
    // This is not an opaque constant: it scales the additive pass as well.
    let light_alpha = light
        .color
        .iter()
        .map(|v| f64::from(*v).powi(2))
        .sum::<f64>()
        .sqrt()
        .min(1.) as f32;
    light_bytes[3] = (f64::from(light_alpha) * 255.).trunc() as u8;
    let material_word = (material.colors[2] & 0xffffff) | (alpha << 24);
    let packed = giants_game::body_pipeline::context_factor_words(
        material_word,
        u32::from_le_bytes(light_bytes),
    );
    Ok(Vec4::new(1., 1., 1., (packed >> 24) as f32 / 255.))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_primary_light_retains_native_magnitude_alpha() {
        let material = giants_formats::gbs::Material {
            texture: String::new(),
            bump_texture: String::new(),
            falloff: 0.,
            blend: 1.,
            flags: 0xe0000020,
            colors: [0, 0x171717, 0x969696, 0x737373],
            power: 0.,
        };
        let light = giants_game::lighting::Directional {
            variant: 1,
            position: [0.; 3],
            direction: [0., 0., 1.],
            color: [0.391, 0.32, 0.211],
        };
        assert_eq!(
            factor(&material, &light).unwrap(),
            Vec4::new(1., 1., 1., 139. / 255.)
        );
    }
}
