import bevy_ui_render::ui_vertex_output::UiVertexOutput;
@group(1) @binding(0) var image:texture_2d<f32>;
@group(1) @binding(1) var image_sampler:sampler;
@group(1) @binding(2) var<uniform> opacity:array<vec4<f32>,100>;
@group(1) @binding(3) var<uniform> slot:vec4<u32>;
@fragment fn fragment(in:UiVertexOutput)->@location(0) vec4<f32> {
    let texel=textureSample(image,image_sampler,mix(vec2<f32>(0.0005),vec2<f32>(0.9995),in.uv));
    return vec4<f32>(texel.rgb,texel.a*opacity[slot.x].x);
}
