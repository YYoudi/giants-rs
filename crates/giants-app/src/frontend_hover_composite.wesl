import bevy_ui_render::ui_vertex_output::UiVertexOutput;
@group(1) @binding(0) var particles:texture_2d<f32>;
@group(1) @binding(1) var particles_sampler:sampler;
@fragment fn fragment(in:UiVertexOutput)->@location(0) vec4<f32> {
    let energy=textureSample(particles,particles_sampler,in.uv).rgb;
    // Owner-requested readability: compress accumulated energy before adding it
    // to the encoded scene, keeping an orange/red core below white text.
    let bounded=energy/(vec3<f32>(0.6)+energy);
    return vec4<f32>(bounded*vec3<f32>(0.72,0.36,0.04),1.0);
}
