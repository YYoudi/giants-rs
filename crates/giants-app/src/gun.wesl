// Measured fixed-function gun profile. Operate on encoded RGB like D3D8.
struct Light { direction:vec4<f32>,color:vec4<f32>,}
struct Parameters {
    world:mat4x4<f32>,view:mat4x4<f32>,clip:mat4x4<f32>,normal_eye:mat4x4<f32>,
    emissive:vec4<f32>,ambient:vec4<f32>,diffuse:vec4<f32>,specular:vec4<f32>,
    lights:array<Light,3>,fog:vec4<f32>,fog_color:vec4<f32>,pixel_power:vec4<f32>,composition:vec4<f32>,
}
@group(3) @binding(0) var base:texture_2d<f32>;
@group(3) @binding(1) var base_sampler:sampler;
@group(3) @binding(2) var<uniform> material:Parameters;
struct Vertex {@location(0) position:vec3<f32>,@location(1) normal:vec3<f32>,@location(2) uv:vec2<f32>,}
struct Output {@builtin(position) position:vec4<f32>,@location(0) uv:vec2<f32>,@location(1) diffuse:vec3<f32>,@location(2) specular:vec3<f32>,@location(3) eye_depth:f32,}
fn unit(v:vec3<f32>)->vec3<f32>{let n=length(v);if(n==0.0){return vec3<f32>(0.0);}return v/n;}
@vertex fn vertex(v:Vertex)->Output {
    var out:Output;
    let world=material.world*vec4<f32>(v.position,1.0);
    let eye=material.view*world;
    let normal=unit((material.normal_eye*vec4<f32>(v.normal,0.0)).xyz);
    let viewer=unit(-eye.xyz);
    var diffuse=material.emissive.rgb;
    var specular=vec3<f32>(0.0);
    for(var i=0u;i<3u;i++){
        let light=material.lights[i];let direction=-unit((material.view*vec4<f32>(light.direction.xyz,0.0)).xyz);
        let cosine=max(dot(normal,direction),0.0);
        diffuse+=material.ambient.rgb*light.color.rgb+(cosine*light.color.w)*material.diffuse.rgb*light.color.rgb;
        if(cosine>0.0 && material.pixel_power.z>0.0){let halfway=unit(viewer+direction);specular+=pow(max(dot(normal,halfway),0.0),material.pixel_power.z)*material.specular.rgb*light.color.rgb;}
    }
    let clip=material.clip*world;
    out.position=vec4<f32>(clip.xy+material.pixel_power.xy*clip.w,clip.w-clip.z,clip.w);
    out.uv=v.uv;out.diffuse=clamp(diffuse,vec3<f32>(0.0),vec3<f32>(1.0));out.specular=clamp(specular,vec3<f32>(0.0),vec3<f32>(1.0));out.eye_depth=clip.w;return out;
}
@fragment fn fragment(in:Output)->@location(0) vec4<f32>{
    let texel=textureSample(base,base_sampler,in.uv);
    var encoded=clamp(clamp(4.0*texel.rgb*in.diffuse,vec3<f32>(0.0),vec3<f32>(1.0))+in.specular,vec3<f32>(0.0),vec3<f32>(1.0));
    if(material.pixel_power.w>0.0 || material.composition.y>0.0){encoded=clamp(texel.rgb+in.specular,vec3<f32>(0.0),vec3<f32>(1.0));}
    // Null-texture diffuse selection: the sample and secondary color do not
    // contribute. No source texture is supplied for this profile.
    if(material.pixel_power.w<0.0){encoded=in.diffuse;}
    if(material.fog.z>0.0){let visible=clamp((material.fog.y-in.eye_depth)/(material.fog.y-material.fog.x),0.0,1.0);encoded=mix(material.fog_color.rgb,encoded,visible);}
    let alpha=select(1.0,texel.a,material.pixel_power.w==1.0);
    return vec4<f32>(encoded,alpha*material.composition.x);
}
