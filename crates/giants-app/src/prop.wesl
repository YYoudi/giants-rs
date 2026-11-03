import bevy_pbr::render::mesh_functions;
// Measured mirrored staff: source texture Ã— stored GBS RGB, encoded scene target.
@group(3) @binding(0) var staff_texture:texture_2d<f32>;
@group(3) @binding(1) var staff_sampler:sampler;
@group(3) @binding(2) var<storage,read> clip_from_bevy:mat4x4<f32>;
@group(3) @binding(3) var<uniform> world_from_bevy:mat4x4<f32>;
@group(3) @binding(4) var<uniform> pixel_offset:vec4<f32>;
@group(3) @binding(5) var<uniform> fog:vec4<f32>;
@group(3) @binding(6) var<uniform> fog_color:vec4<f32>;
@group(3) @binding(7) var<storage,read> instance_matrices:array<mat4x4<f32>>;
struct Vertex {@builtin(instance_index) instance:u32,@location(0) position:vec3<f32>,@location(1) uv:vec2<f32>,@location(2) color:vec4<f32>,}
struct Output {@builtin(position) position:vec4<f32>,@location(0) uv:vec2<f32>,@location(1) color:vec3<f32>,@location(2) eye_depth:f32,}
@vertex fn vertex(v:Vertex)->Output {
    var out:Output;
    @if(SOURCE_FOLIAGE_INSTANCES)
    let world=instance_matrices[mesh_functions::get_tag(v.instance)];
    @if(!SOURCE_FOLIAGE_INSTANCES)
    let world=world_from_bevy;
    let clip=clip_from_bevy*(world*vec4<f32>(v.position,1.0));
    out.position=vec4<f32>(clip.xy+pixel_offset.xy*clip.w,clip.w-clip.z,clip.w);
    out.uv=v.uv;out.color=v.color.rgb;out.eye_depth=clip.w;return out;
}
@fragment fn fragment(in:Output)->@location(0) vec4<f32> {
    let texel=textureSample(staff_texture,staff_sampler,in.uv);
    let color=select(in.color,vec3<f32>(1.0),pixel_offset.w<0.0);
    var encoded=texel.rgb*color;
    if(pixel_offset.w==7.0){encoded=texel.rgb+in.color;}
    if(pixel_offset.w==-2.0){encoded=in.color;}
    encoded=clamp(encoded,vec3<f32>(0.0),vec3<f32>(1.0));
    if(fog.z>0.0){let visible=clamp((fog.y-in.eye_depth)/(fog.y-fog.x),0.0,1.0);encoded=mix(fog_color.rgb,encoded,visible);}
    return vec4<f32>(encoded,select(1.0,texel.a,pixel_offset.z>0.0));
}
