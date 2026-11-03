import bevy_ui_render::ui_vertex_output::UiVertexOutput;
struct Parameters { phases:array<vec4<f32>,5>, colors:array<vec4<f32>,5>, }
@group(1) @binding(0) var source:texture_2d<f32>;
@group(1) @binding(1) var source_sampler:sampler;
@group(1) @binding(2) var<uniform> parameters:Parameters;
fn quantize8(value:vec3<f32>)->vec3<f32> {return round(clamp(value,vec3<f32>(0.0),vec3<f32>(1.0))*255.0)/255.0;}
@fragment fn fragment(in:UiVertexOutput)->@location(0) vec4<f32> {
    // Native D3D8 integer pixel centers, with vertically inverted screen UV.
    let uv=vec2<f32>(in.uv.x-0.5/256.0,1.0-in.uv.y+0.5/256.0);
    var encoded=vec3<f32>(0.0);
    for(var i=0u;i<5u;i=i+1u) {
        let sample=textureSample(source,source_sampler,(uv+parameters.phases[i].xy)*f32(i+1u)).rgb;
        let layer=clamp(sample+parameters.colors[i].rgb,vec3<f32>(0.0),vec3<f32>(1.0));
        if(i==0u){encoded=quantize8(layer);}else{encoded=quantize8(2.0*layer*encoded);}
    }
    let linear=select(encoded/12.92,pow((encoded+vec3<f32>(0.055))/1.055,vec3<f32>(2.4)),encoded>vec3<f32>(0.04045));
    return vec4<f32>(select(linear,encoded,parameters.phases[0].z>0.0),1.0);
}
