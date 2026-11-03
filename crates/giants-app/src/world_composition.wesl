import bevy_ui_render::ui_vertex_output::UiVertexOutput;
@group(1) @binding(0) var encoded_world:texture_2d<f32>;
@group(1) @binding(1) var encoded_sampler:sampler;
@group(1) @binding(2) var<uniform> glare:vec4<f32>;
@group(1) @binding(3) var<uniform> glare_viewport:vec4<f32>;
@group(1) @binding(4) var<uniform> frame_viewport:vec4<f32>;
@group(1) @binding(5) var<uniform> render_flags:vec4<f32>;
@group(1) @binding(6) var<uniform> fade:vec4<f32>;
@group(1) @binding(7) var<uniform> initial_fade:vec4<f32>;
@fragment fn fragment(in:UiVertexOutput)->@location(0) vec4<f32> {
    let point=in.uv*vec2<f32>(1920.0,1080.0);
    let in_viewport=all(point>=glare_viewport.xy)&&all(point<glare_viewport.xy+glare_viewport.zw);
    let veil=select(vec3<f32>(0.0),glare.rgb,in_viewport&&render_flags.x==0.0);
    let sampled=clamp(textureSample(encoded_world,encoded_sampler,in.uv).rgb+veil,vec3<f32>(0.0),vec3<f32>(1.0));
    let in_frame=all(point>=frame_viewport.xy)&&all(point<frame_viewport.xy+frame_viewport.zw);
    let mixed=sampled*(1.0-fade.a)+fade.rgb*fade.a;
    // Preserve the encoded eight-bit target store before display conversion.
    let faded=select(sampled,floor(mixed*255.0+vec3<f32>(0.5))/255.0,fade.a>0.0);
    // Native drawing applies the D8 envelope after the E8 colored overlay.
    let initial_mixed=faded*(1.0-initial_fade.a)+initial_fade.rgb*initial_fade.a;
    let final_faded=select(faded,floor(initial_mixed*255.0+vec3<f32>(0.5))/255.0,initial_fade.a>0.0);
    let encoded=select(vec3<f32>(0.0),final_faded,frame_viewport.z==0.0||in_frame);
    let linear=select(encoded/12.92,pow((encoded+vec3<f32>(0.055))/1.055,vec3<f32>(2.4)),encoded>vec3<f32>(0.04045));
    return vec4<f32>(linear,1.0);
}
