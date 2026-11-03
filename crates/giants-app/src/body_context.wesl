struct View { clip:mat4x4<f32>, }
@group(3) @binding(0) var base:texture_2d<f32>;
@group(3) @binding(1) var base_sampler:sampler;
@group(3) @binding(2) var normal_image:texture_2d<f32>;
@group(3) @binding(3) var normal_sampler:sampler;
@group(3) @binding(4) var<uniform> view:View;
@group(3) @binding(5) var<uniform> pixel:vec4<f32>;
@group(3) @binding(6) var<uniform> factor:vec4<f32>;
@group(3) @binding(7) var<uniform> fog:vec4<f32>;
@group(3) @binding(8) var<uniform> source_world:mat4x4<f32>;
struct Vertex { @location(0) position:vec3<f32>,@location(1) uv:vec2<f32>,@location(2) color:vec4<f32>, }
struct Output { @builtin(position) position:vec4<f32>,@location(0) uv:vec2<f32>,@location(1) color:vec3<f32>,@location(2) depth:f32, }
@vertex fn vertex(v:Vertex)->Output {
    var out:Output;
    let p=source_world*vec4<f32>(v.position,1.0);
    let clip=view.clip*p;
    out.position=vec4<f32>(clip.xy+pixel.xy*clip.w,clip.w-clip.z,clip.w);
    out.uv=v.uv;out.color=v.color.rgb;out.depth=clip.w;return out;
}
@fragment fn fragment(v:Output)->@location(0) vec4<f32> {
    let n=textureSample(normal_image,normal_sampler,v.uv).rgb;
    let dot3=clamp(4.0*dot(n-vec3<f32>(0.5),v.color-vec3<f32>(0.5)),0.0,1.0);
    let stage1=clamp(2.0*textureSample(base,base_sampler,v.uv).rgb*dot3,vec3<f32>(0.0),vec3<f32>(1.0));
    // D's checked renderer disables stage2. Its factor supplies blend alpha;
    // the older GOG modulation by factor RGB is a different program.
    var rgb=stage1;
    if(fog.z>0.0){rgb*=clamp((fog.y-v.depth)/(fog.y-fog.x),0.0,1.0);}
    return vec4<f32>(rgb,factor.a);
}
