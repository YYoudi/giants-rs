//! Physical Mecc dispatcher with explicit prepared meshes and query providers.
//! Live collider construction, radial/box responses and nonphysical actors are upstream.
use crate::{
    arithmetic::Precision,
    ground_contact,
    mecc_mesh_contact::State,
    mecc_mesh_dispatch::{self, MeshObject},
    physical_ground,
    platform_queries::PlatformSample,
};

pub struct Input {
    pub actor: u32,
    pub has_model: bool,
    pub state: State,
    pub velocity_xy: [f32; 2],
    pub gravity_mass: f32,
    pub restitution: f32,
    pub retention: f32,
    pub force_z: f32,
    pub physics_flags: u32,
    pub support: u32,
}
pub struct Output {
    pub state: State,
    pub velocity_xy: [f32; 2],
    pub force_z: f32,
    pub physics_flags: u32,
    pub support: u32,
    pub passes: u32,
    pub mesh_responses: u32,
    pub fallback_xy: bool,
    pub ground_queries: u32,
    pub terrain_responses: u32,
    pub junctions: u32,
    pub platform_queries: u32,
}
pub fn resolve(
    input: Input,
    objects: &[MeshObject],
    precision: Precision,
    mut ground: impl FnMut([f32; 2]) -> Result<physical_ground::Ground, &'static str>,
    mut platform: impl FnMut([f32; 3]) -> Result<PlatformSample, &'static str>,
) -> Result<Output, &'static str> {
    if input.actor == 0 || !input.state.physical {
        return Err("physical Mecc profile required");
    }
    let mut out = Output {
        state: input.state,
        velocity_xy: input.velocity_xy,
        force_z: input.force_z,
        physics_flags: input.physics_flags,
        support: input.support,
        passes: 0,
        mesh_responses: 0,
        fallback_xy: false,
        ground_queries: 0,
        terrain_responses: 0,
        junctions: 0,
        platform_queries: 0,
    };
    if out.state.object_flags & 0x1000_0000 == 0 && out.state.current == out.state.previous {
        return Ok(out);
    }
    out.state.object_flags &= !0x1000_0000;
    out.state.hit = 0;
    out.support = 0;
    let initial = ground([out.state.previous[0], out.state.previous[1]])?;
    out.ground_queries = 1;
    out.state.floor = initial.height;
    out.state.plane = initial.plane;
    // Native early exit is after initial ground and marker resets, before objects.
    if input.has_model && out.state.collision_flags & 0x200 != 0 {
        return Ok(out);
    }
    // Keep the force bit for the object's stage entry only. Without it an
    // originally forced stationary actor would incorrectly skip this stage.
    let forced = input.state.object_flags & 0x1000_0000;
    out.state.object_flags |= forced;
    let mesh = mecc_mesh_dispatch::resolve(
        input.actor,
        input.has_model,
        out.state,
        mecc_mesh_dispatch::Ground {
            height: initial.height,
            plane: initial.plane,
        },
        objects,
        precision,
    )?;
    out.state = mesh.state;
    out.passes = mesh.passes;
    out.mesh_responses = mesh.responses;
    out.fallback_xy = mesh.fallback_xy;
    let response = physical_ground::resolve(
        physical_ground::Input {
            position: out.state.current,
            previous: out.state.previous,
            velocity: [
                out.velocity_xy[0],
                out.velocity_xy[1],
                out.state.world_vertical_speed,
            ],
            scale: out.state.scale,
            ground_offset: out.state.ground_offset,
            center_offset: out.state.center_offset,
            gravity_mass: input.gravity_mass,
            restitution: input.restitution,
            retention: input.retention,
            force_z: out.force_z,
            flags: out.physics_flags,
            floor: out.state.floor,
            plane: out.state.plane,
        },
        precision,
        &mut ground,
    )?;
    out.state.current = response.position;
    out.state.previous = response.previous;
    out.velocity_xy = response.velocity[..2].try_into().unwrap();
    out.state.world_vertical_speed = response.velocity[2];
    out.state.floor = response.floor;
    out.state.plane = response.plane;
    out.force_z = response.force_z;
    out.physics_flags = response.flags;
    out.ground_queries += response.queries;
    out.terrain_responses = response.terrain_responses;
    out.junctions = response.junctions;
    let surface = platform(out.state.current)?;
    out.platform_queries = 1;
    let contact = ground_contact::resolve(
        ground_contact::Input {
            z: out.state.current[2],
            vertical_velocity: out.state.model_vertical_speed,
            flags: out.state.object_flags,
            lock_z: true,
            ground: out.state.floor,
            plane: out.state.plane,
            vertical_offset: out.state.ground_offset,
            platform: surface.object.map(|_| surface.height),
        },
        precision,
    )?;
    out.state.current[2] = contact.z;
    out.state.model_vertical_speed = contact.vertical_velocity;
    out.state.object_flags = contact.flags;
    out.state.floor = contact.ground;
    out.state.plane = contact.plane;
    // The selected platform is not written to the actor's support marker here.
    Ok(out)
}
