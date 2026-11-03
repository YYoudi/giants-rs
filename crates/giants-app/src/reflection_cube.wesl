import bevy_ui_render::ui_vertex_output::UiVertexOutput;
@group(1) @binding(0) var cube:texture_cube<f32>;
@group(1) @binding(1) var cube_sampler:sampler;
@fragment fn fragment(in:UiVertexOutput)->@location(0) vec4<f32> {
    let tile=in.uv*vec2<f32>(3.0,2.0);
    let face=u32(floor(tile.x))+3u*u32(floor(tile.y));
    let p=fract(tile)*2.0-1.0;
    var d=vec3<f32>(1.0,-p.y,-p.x);
    if(face==1u){d=vec3<f32>(-1.0,-p.y,p.x);}
    if(face==2u){d=vec3<f32>(p.x,1.0,p.y);}
    if(face==3u){d=vec3<f32>(p.x,-1.0,-p.y);}
    if(face==4u){d=vec3<f32>(p.x,-p.y,1.0);}
    if(face==5u){d=vec3<f32>(-p.x,-p.y,-1.0);}
    let encoded=textureSampleLevel(cube,cube_sampler,d,0.0).rgb;
    let linear=select(encoded/12.92,pow((encoded+vec3<f32>(0.055))/1.055,vec3<f32>(2.4)),encoded>vec3<f32>(0.04045));
    return vec4<f32>(linear,1.0);
}
