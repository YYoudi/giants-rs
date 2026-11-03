// Measured spear profile: SELECTARG1 texture, opaque, linear eye-depth fog.
@group(3) @binding(0) var base:texture_2d<f32>;
@group(3) @binding(1) var base_sampler:sampler;
@group(3) @binding(2) var<uniform> clip_world:mat4x4<f32>;
@group(3) @binding(3) var<uniform> pixel_fog:vec4<f32>;
@group(3) @binding(4) var<uniform> fog_color:vec4<f32>;
struct Vertex {@location(0) position:vec3<f32>,@location(1) uv:vec2<f32>,}
struct Output {@builtin(position) position:vec4<f32>,@location(0) uv:vec2<f32>,@location(1) depth:f32,}
@vertex fn vertex(v:Vertex)->Output {
 var out:Output;let clip=clip_world*vec4<f32>(v.position,1.0);
 out.position=vec4<f32>(clip.xy+pixel_fog.xy*clip.w,clip.w-clip.z,clip.w);
 out.uv=v.uv;out.depth=clip.w;return out;
}
@fragment fn fragment(in:Output)->@location(0) vec4<f32>{
 var rgb=textureSample(base,base_sampler,in.uv).rgb;
 if(fog_color.w>0.0){let visible=clamp((pixel_fog.w-in.depth)/(pixel_fog.w-pixel_fog.z),0.0,1.0);rgb=mix(fog_color.rgb,rgb,visible);}
 return vec4<f32>(rgb,1.0);
}
