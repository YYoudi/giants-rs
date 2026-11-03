import bevy_pbr::render::{mesh_functions,view_transformations};
@group(3) @binding(0) var sky_texture:texture_2d<f32>;
@group(3) @binding(1) var sky_sampler:sampler;
@group(3) @binding(2) var<uniform> clip_from_bevy:mat4x4<f32>;
@group(3) @binding(3) var<uniform> validation:vec4<f32>;
struct Vertex {@builtin(instance_index) instance:u32,@location(0) position:vec3<f32>,@location(1) uv:vec2<f32>,}
struct Output {@builtin(position) position:vec4<f32>,@location(0) uv:vec2<f32>,}
@vertex fn vertex(v:Vertex)->Output {
    var out:Output;
    let world=mesh_functions::mesh_position_local_to_world(mesh_functions::get_world_from_local(v.instance),vec4<f32>(v.position,1.0));
    out.position=view_transformations::position_world_to_clip(world.xyz);
    if(validation.x>0.0){out.position=clip_from_bevy*vec4<f32>(v.position,1.0);out.position=vec4<f32>(out.position.xy+validation.yz*out.position.w,out.position.w-out.position.z,out.position.w);}
    out.uv=v.uv;return out;
}
@fragment fn fragment(in:Output)->@location(0) vec4<f32> {
    let encoded=textureSample(sky_texture,sky_sampler,in.uv).rgb;
    return vec4<f32>(encoded,1.0);
}
