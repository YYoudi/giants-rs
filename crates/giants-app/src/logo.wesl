import bevy_pbr::render::{mesh_functions,view_transformations};

struct Light {
    direction: vec4<f32>, diffuse: vec4<f32>, specular: vec4<f32>, ambient: vec4<f32>,
}
struct Parameters {
    emissive: vec4<f32>, ambient: vec4<f32>, diffuse: vec4<f32>, specular: vec4<f32>,
    camera: vec4<f32>, power: vec4<f32>, light0: Light, light1: Light,
}
@group(3) @binding(0) var<uniform> material: Parameters;
@group(3) @binding(1) var base: texture_2d<f32>;
@group(3) @binding(2) var base_sampler: sampler;
struct Vertex {
    @builtin(instance_index) instance: u32,
    @location(0) position: vec3<f32>, @location(1) normal: vec3<f32>, @location(2) uv: vec2<f32>,
}
struct Output {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>, @location(1) diffuse: vec3<f32>, @location(2) specular: vec3<f32>,
}
fn unit(v: vec3<f32>) -> vec3<f32> {
    let length = length(v);
    if length == 0.0 { return vec3<f32>(0.0); }
    return v / length;
}
@vertex
fn vertex(v: Vertex) -> Output {
    var out: Output;
    let world = mesh_functions::mesh_position_local_to_world(mesh_functions::get_world_from_local(v.instance),vec4<f32>(v.position,1.0));
    let normal = unit(mesh_functions::mesh_normal_local_to_world(v.normal,v.instance));
    let viewer = unit(material.camera.xyz-world.xyz);
    var diffuse = material.emissive.rgb;
    var specular = vec3<f32>(0.0);
    let lights = array<Light,2>(material.light0,material.light1);
    for (var i=0u; i<2u; i++) {
        let light = lights[i];
        let direction = unit(-light.direction.xyz);
        let cosine = max(dot(normal,direction),0.0);
        diffuse += material.ambient.rgb*light.ambient.rgb + cosine*material.diffuse.rgb*light.diffuse.rgb;
        if cosine>0.0 {
            let half_vector=unit(viewer+direction);
            specular += pow(max(dot(normal,half_vector),0.0),material.power.x)*material.specular.rgb*light.specular.rgb;
        }
    }
    out.position=view_transformations::position_world_to_clip(world.xyz);
    out.uv=v.uv;
    out.diffuse=clamp(diffuse,vec3<f32>(0.0),vec3<f32>(1.0));
    out.specular=clamp(specular,vec3<f32>(0.0),vec3<f32>(1.0));
    return out;
}
fn encoded_to_linear(color: vec3<f32>) -> vec3<f32> {
    return select(color/12.92,pow((color+vec3<f32>(0.055))/1.055,vec3<f32>(2.4)),color>vec3<f32>(0.04045));
}
@fragment
fn fragment(in: Output) -> @location(0) vec4<f32> {
    // First native stage only. The second-stage cube is intentionally not fabricated.
    let first=clamp(4.0*textureSample(base,base_sampler,in.uv).rgb*in.diffuse,vec3<f32>(0.0),vec3<f32>(1.0));
    let encoded=clamp(first+in.specular,vec3<f32>(0.0),vec3<f32>(1.0));
    return vec4<f32>(encoded_to_linear(encoded),1.0);
}
