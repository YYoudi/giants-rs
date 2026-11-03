import bevy_pbr::render::{mesh_functions,view_transformations};
@group(3) @binding(0) var<uniform> tint: vec4<f32>;
@group(3) @binding(1) var terrain_texture: texture_2d<f32>;
@group(3) @binding(2) var terrain_sampler: sampler;
@group(3) @binding(3) var<uniform> clip_from_bevy: mat4x4<f32>;
@group(3) @binding(4) var<uniform> validation: vec4<f32>;
@group(3) @binding(5) var<uniform> uv_from_bevy: mat4x4<f32>;
@group(3) @binding(6) var<uniform> fog_parameters: vec4<f32>;
@group(3) @binding(7) var<uniform> fog_color: vec4<f32>;
@group(3) @binding(8) var<uniform> additional_uv_from_bevy: mat4x4<f32>;
@group(3) @binding(9) var additional_texture: texture_2d<f32>;
@group(3) @binding(10) var additional_sampler: sampler;
@group(3) @binding(11) var<uniform> surface: vec4<f32>;
struct Vertex {
    @builtin(instance_index) instance: u32,
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) secondary_color: vec4<f32>,
}
struct Output {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) eye_depth: f32,
    @location(3) secondary_color: vec3<f32>,
    @location(4) additional_uv: vec2<f32>,
}
@vertex
fn vertex(v: Vertex) -> Output {
    var out: Output;
    let world=mesh_functions::mesh_position_local_to_world(mesh_functions::get_world_from_local(v.instance),vec4<f32>(v.position,1.0));
    out.position=view_transformations::position_world_to_clip(world.xyz);
    out.color=v.color;
    out.uv=(uv_from_bevy*vec4<f32>(v.position,1.0)).xy;
    out.additional_uv=(additional_uv_from_bevy*vec4<f32>(v.position,1.0)).xy;
    out.secondary_color=v.secondary_color.rgb;
    if(validation.x>0.0){
        out.position=clip_from_bevy*vec4<f32>(v.position,1.0);
        // D3D8 uses near=0/far=1; Bevy's opaque pipeline uses reversed depth.
        out.position=vec4<f32>(out.position.xy+validation.yz*out.position.w,out.position.w-out.position.z,out.position.w);
    }
    // Carry eye depth explicitly: using reciprocal fragment position.w failed
    // the distant D3D12 control. Perspective interpolation preserves clip W.
    out.eye_depth=out.position.w;
    return out;
}
@fragment
fn fragment(in: Output) -> @location(0) vec4<f32> {
    // The Rgba8Unorm scene target stores encoded RGB; presentation decodes once.
    let texel=textureSample(terrain_texture,terrain_sampler,in.uv).rgb;
    var encoded=clamp((in.color.rgb+texel)*tint.rgb,vec3<f32>(0.0),vec3<f32>(1.0));
    if(surface.x>0.0){
        let normal=textureSample(additional_texture,additional_sampler,in.additional_uv).rgb;
        let directional=clamp(4.0*dot(normal-vec3<f32>(0.5),in.color.rgb-vec3<f32>(0.5)),0.0,1.0);
        let secondary=select(vec3<f32>(0.0),in.secondary_color,surface.y>0.0);
        encoded=clamp((2.0*directional*texel+secondary)*tint.rgb,vec3<f32>(0.0),vec3<f32>(1.0));
    }
    if(fog_parameters.z>0.0){
        let distance=in.eye_depth;
        let visible=clamp((fog_parameters.y-distance)/(fog_parameters.y-fog_parameters.x),0.0,1.0);
        encoded=mix(fog_color.rgb,encoded,visible);
    }
    return vec4<f32>(encoded,1.0);
}
