// Native blood corners are offsets in source clip space, before division.
import bevy_pbr::render::{mesh_functions,view_transformations};
@group(3) @binding(0) var image:texture_2d<f32>;
@group(3) @binding(1) var image_sampler:sampler;
@group(3) @binding(2) var<uniform> clip_from_bevy:mat4x4<f32>;
@group(3) @binding(3) var<uniform> pixel:vec4<f32>;
struct Vertex {@builtin(instance_index) instance:u32,@location(0) position:vec3<f32>,@location(1) uv:vec2<f32>,@location(2) color:vec4<f32>,@location(3) offset:vec2<f32>,}
struct Output {@builtin(position) position:vec4<f32>,@location(0) uv:vec2<f32>,@location(1) alpha:f32,}
@vertex fn vertex(v:Vertex)->Output {
    var out:Output;
    let p=clip_from_bevy*vec4<f32>(v.position,1.0);
    out.position=vec4<f32>(p.xy+v.offset+pixel.xy*p.w,p.w-p.z,p.w);
    out.uv=v.uv;out.alpha=v.color.a;return out;
}
@fragment fn fragment(v:Output)->@location(0) vec4<f32> {
    let texel=textureSample(image,image_sampler,v.uv);
    return vec4<f32>(texel.rgb,texel.a*v.alpha);
}
