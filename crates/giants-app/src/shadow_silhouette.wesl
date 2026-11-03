@group(3) @binding(0) var<uniform> source_view:mat4x4<f32>;
@group(3) @binding(1) var<uniform> source_projection:mat4x4<f32>;
@group(3) @binding(2) var<uniform> pixel_offset:vec4<f32>;
@group(3) @binding(3) var<uniform> source_world:mat4x4<f32>;
struct Vertex { @location(0) position:vec3<f32>, }
@vertex fn vertex(v:Vertex)->@builtin(position) vec4<f32> {
    let clip=source_projection*(source_view*(source_world*vec4<f32>(v.position,1.0)));
    // Same D3D pixel-centre adapter as the existing source body passes.
    return vec4<f32>(clip.xy+pixel_offset.xy*clip.w,clip.w,clip.w);
}
@fragment fn fragment()->@location(0) vec4<f32> { return vec4<f32>(1.0); }
