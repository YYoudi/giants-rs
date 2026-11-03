// Measured mirrored staff: source texture × stored GBS RGB, encoded scene target.
@group(3) @binding(0) var staff_texture:texture_2d<f32>;
@group(3) @binding(1) var staff_sampler:sampler;
@group(3) @binding(2) var<uniform> clip_from_bevy:mat4x4<f32>;
@group(3) @binding(3) var<uniform> world_from_bevy:mat4x4<f32>;
@group(3) @binding(4) var<uniform> pixel_offset:vec4<f32>;
struct Vertex {@location(0) position:vec3<f32>,@location(1) uv:vec2<f32>,@location(2) color:vec4<f32>,}
struct Output {@builtin(position) position:vec4<f32>,@location(0) uv:vec2<f32>,@location(1) color:vec3<f32>,}
@vertex fn vertex(v:Vertex)->Output {
    var out:Output;
    let clip=clip_from_bevy*(world_from_bevy*vec4<f32>(v.position,1.0));
    out.position=vec4<f32>(clip.xy+pixel_offset.xy*clip.w,clip.w-clip.z,clip.w);
    out.uv=v.uv;out.color=v.color.rgb;return out;
}
@fragment fn fragment(in:Output)->@location(0) vec4<f32> {
    return vec4<f32>(textureSample(staff_texture,staff_sampler,in.uv).rgb*in.color,1.0);
}
