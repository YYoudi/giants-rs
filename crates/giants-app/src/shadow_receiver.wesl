@group(3) @binding(0) var silhouette:texture_2d<f32>;
@group(3) @binding(1) var silhouette_sampler:sampler;
@group(3) @binding(2) var<uniform> clip_from_bevy:mat4x4<f32>;
@group(3) @binding(3) var<uniform> pixel_strength:vec4<f32>;
@group(3) @binding(4) var<uniform> fog:vec4<f32>;
@group(3) @binding(5) var<uniform> fog_color:vec4<f32>;
struct Vertex {@location(0) position:vec3<f32>,@location(1) uv:vec2<f32>,}
struct Output {@builtin(position) position:vec4<f32>,@location(0) uv:vec2<f32>,@location(1) eye_depth:f32,}
@vertex fn vertex(v:Vertex)->Output {
    var out:Output;
    let clip=clip_from_bevy*vec4<f32>(v.position,1.0);
    // Same source projection arithmetic and pixel centre as the terrain base.
    out.position=vec4<f32>(clip.xy+pixel_strength.xy*clip.w,clip.w-clip.z,clip.w);
    out.uv=v.uv;out.eye_depth=clip.w;
    return out;
}
@fragment fn fragment(v:Output)->@location(0) vec4<f32> {
    var source=textureSample(silhouette,silhouette_sampler,v.uv).rgb*pixel_strength.z;
    if(fog.z>0.0){let visible=clamp((fog.y-v.eye_depth)/(fog.y-fog.x),0.0,1.0);source=mix(fog_color.rgb,source,visible);}
    // Pipeline applies ZERO / INVSRCCOLOR, retaining destination alpha.
    return vec4<f32>(source,0.0);
}
