struct Matrix { value:mat4x4<f32>, }
@group(3) @binding(0) var base:texture_2d<f32>;
@group(3) @binding(1) var base_sampler:sampler;
@group(3) @binding(2) var<uniform> view:Matrix;
@group(3) @binding(3) var<uniform> world:Matrix;
@group(3) @binding(4) var<uniform> pixel_fog:vec4<f32>;
@group(3) @binding(5) var<uniform> color_alpha:vec4<f32>;
@group(3) @binding(6) var<uniform> flags:vec4<f32>;
struct Vertex { @location(0) position:vec3<f32>,@location(1) uv:vec2<f32>,@location(2) color:vec4<f32>, }
struct Output { @builtin(position) position:vec4<f32>,@location(0) uv:vec2<f32>,@location(1) color:vec4<f32>,@location(2) depth:f32, }
@vertex fn vertex(v:Vertex)->Output {
    var out:Output;
    let clip=view.value*(world.value*vec4<f32>(v.position,1.0));
    out.position=vec4<f32>(clip.xy+pixel_fog.xy*clip.w,clip.w-clip.z,clip.w);
    if(flags.w==2.0){
        // Tracer positions already contain source clip X/Y and homogeneous W.
        let w=v.position.z;
        out.position=vec4<f32>(v.position.xy+pixel_fog.xy*w,w-(pixel_fog.z*w+pixel_fog.w),w);
        out.depth=w;
    }
    out.uv=v.uv;out.color=v.color;out.depth=select(clip.w,v.position.z,flags.w==2.0);return out;
}
@fragment fn fragment(v:Output)->@location(0) vec4<f32> {
    let sampled=textureSample(base,base_sampler,v.uv);
    var rgb=sampled.rgb;
    if(flags.x>1.0){rgb=clamp(rgb+v.color.rgb,vec3<f32>(0.0),vec3<f32>(1.0));}
    else if(flags.x>0.0){rgb*=v.color.rgb;}
    if(flags.y>0.0){let visible=clamp((pixel_fog.w-v.depth)/(pixel_fog.w-pixel_fog.z),0.0,1.0);rgb=mix(color_alpha.rgb,rgb,visible);}
    var alpha=color_alpha.a*v.color.a;
    if(flags.z>0.0){
        // Source diffuse-alpha + texture-alpha selects their product and
        // ignores TFACTOR. Without texture alpha the factor still applies.
        if(flags.w>0.0){alpha=v.color.a;}
        alpha*=sampled.a;
    }
    return vec4<f32>(rgb,alpha);
}
