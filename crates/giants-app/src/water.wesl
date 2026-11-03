import bevy_pbr::render::{mesh_functions,view_transformations};
@group(3) @binding(0) var tile:texture_2d<f32>;
@group(3) @binding(1) var tile_sampler:sampler;
@group(3) @binding(2) var<uniform> clip_from_bevy:mat4x4<f32>;
@group(3) @binding(3) var<uniform> validation:vec4<f32>;
@group(3) @binding(4) var<uniform> fog:vec4<f32>;
@group(3) @binding(5) var<uniform> tint:vec4<f32>;
struct Vertex {@builtin(instance_index) instance:u32,@location(0) position:vec3<f32>,@location(1) color:vec4<f32>,@location(2) uv:vec2<f32>,}
struct Output {@builtin(position) position:vec4<f32>,@location(0) uv:vec2<f32>,@location(1) alpha:f32,@location(2) eye_depth:f32,}
@vertex fn vertex(v:Vertex)->Output {
    var out:Output;
    let world=mesh_functions::mesh_position_local_to_world(mesh_functions::get_world_from_local(v.instance),vec4<f32>(v.position,1.0));
    out.position=view_transformations::position_world_to_clip(world.xyz);
    if(validation.x>0.0){out.position=clip_from_bevy*vec4<f32>(v.position,1.0);out.position=vec4<f32>(out.position.xy+validation.yz*out.position.w,out.position.w-out.position.z,out.position.w);}
    out.uv=v.uv;out.alpha=v.color.a;out.eye_depth=out.position.w;return out;
}
@fragment fn fragment(in:Output)->@location(0) vec4<f32> {
    let sample=textureSample(tile,tile_sampler,in.uv).rgb;
    let visible=select(1.0,clamp((fog.y-in.eye_depth)/(fog.y-fog.x),0.0,1.0),fog.z>0.0);
    return vec4<f32>((sample+tint.rgb-sample*tint.rgb)*visible,in.alpha);
}
