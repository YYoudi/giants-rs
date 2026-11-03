struct Parameters { world:mat4x4<f32>, pixel_alpha:vec4<f32>, }
@group(3) @binding(0) var image:texture_2d<f32>;
@group(3) @binding(1) var image_sampler:sampler;
@group(3) @binding(2) var<uniform> parameters:Parameters;
@group(3) @binding(3) var<uniform> clip:mat4x4<f32>;
struct Vertex { @location(0) position:vec3<f32>, @location(1) uv:vec2<f32>, }
struct Output { @builtin(position) position:vec4<f32>, @location(0) uv:vec2<f32>, }
@vertex fn vertex(v:Vertex)->Output {
    var out:Output;
    let p=clip*parameters.world*vec4<f32>(v.position,1.0);
    out.position=vec4<f32>(p.xy+parameters.pixel_alpha.xy*p.w,p.w-p.z,p.w);
    out.uv=v.uv;return out;
}
@fragment fn fragment(in:Output)->@location(0) vec4<f32> {
    return vec4<f32>(textureSample(image,image_sampler,in.uv).rgb,parameters.pixel_alpha.z);
}
