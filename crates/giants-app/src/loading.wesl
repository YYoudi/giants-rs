import bevy_ui_render::ui_vertex_output::UiVertexOutput;

@group(1) @binding(0) var<uniform> uv_scale: vec4<f32>;
@group(1) @binding(1) var tile: texture_2d<f32>;
@group(1) @binding(2) var tile_sampler: sampler;

// D3D8 filters the encoded byte values. Decode only AFTER filtering so the
// sRGB render target's encoding restores the measured byte-space result.
fn encoded_to_linear(color: vec3<f32>) -> vec3<f32> {
    return select(color / 12.92,
        pow((color + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4)),
        color > vec3<f32>(0.04045));
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(tile, tile_sampler, in.uv * uv_scale.xy).rgb;
    return vec4<f32>(encoded_to_linear(color), 1.0);
}
