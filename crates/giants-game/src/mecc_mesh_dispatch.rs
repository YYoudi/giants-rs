//! Mecc mesh-object stage, ending before the terrain/platform response stage.
//! Caller supplies live fields/list order, selected triangles and initial ground.
use crate::{
    arithmetic::Precision,
    mecc_collision_candidates::{self, Actor, Object},
    mecc_mesh_contact::{self, State},
};
pub struct MeshObject {
    pub fields: Object,
    pub triangles: Vec<[[f32; 3]; 3]>,
}
pub struct Ground {
    pub height: f32,
    pub plane: [f32; 4],
}
pub struct Stage {
    pub state: State,
    pub passes: u32,
    pub responses: u32,
    pub fallback_xy: bool,
}
pub struct DispatchResponse {
    pub response: mecc_mesh_contact::Response,
    /// Volume misses retain an earlier retry; mesh results replace it.
    pub preserve_retry: bool,
    /// The special radial rewind ends this candidate list immediately.
    pub stop_pass: bool,
}
impl From<mecc_mesh_contact::Response> for DispatchResponse {
    fn from(response: mecc_mesh_contact::Response) -> Self {
        Self {
            response,
            preserve_retry: false,
            stop_pass: false,
        }
    }
}
pub fn resolve(
    actor: u32,
    has_model: bool,
    state: State,
    ground: Ground,
    objects: &[MeshObject],
    precision: Precision,
) -> Result<Stage, &'static str> {
    resolve_selected(
        actor,
        has_model,
        state,
        ground,
        &objects.iter().collect::<Vec<_>>(),
        precision,
    )
}
/// Ordered borrowed geometry, after caller-owned class/state exclusions.
/// This avoids copying prepared vertex buffers for each environment request.
pub fn resolve_selected(
    actor: u32,
    has_model: bool,
    state: State,
    ground: Ground,
    objects: &[&MeshObject],
    precision: Precision,
) -> Result<Stage, &'static str> {
    resolve_with_response(
        actor,
        has_model,
        state,
        ground,
        objects,
        precision,
        |state, object| {
            if object.fields.collision_flags & 0x1008 == 0 {
                return Err("radial or box response required");
            }
            mecc_mesh_contact::resolve(state, object.fields.id, &object.triangles, precision)
                .map(Into::into)
        },
    )
}
/// Same ordered candidate/retry loop with an explicit acquired narrow phase.
pub fn resolve_with_response(
    actor: u32,
    has_model: bool,
    mut state: State,
    ground: Ground,
    objects: &[&MeshObject],
    precision: Precision,
    mut respond: impl FnMut(State, &MeshObject) -> Result<DispatchResponse, &'static str>,
) -> Result<Stage, &'static str> {
    if actor == 0 || !ground.height.is_finite() || !ground.plane.into_iter().all(f32::is_finite) {
        return Err("invalid mesh dispatch input");
    }
    let mut result = Stage {
        state,
        passes: 0,
        responses: 0,
        fallback_xy: false,
    };
    if state.object_flags & 0x1000_0000 == 0 && state.current == state.previous {
        return Ok(result);
    }
    state.object_flags &= !0x1000_0000;
    state.hit = 0;
    state.floor = ground.height;
    state.plane = ground.plane;
    let fields = objects.iter().map(|o| o.fields).collect::<Vec<_>>();
    let mut identities = std::collections::BTreeSet::new();
    for o in &fields {
        if o.id == 0 || !identities.insert(o.id) {
            return Err("invalid mesh dispatch object identity");
        }
    }
    let mut retry = true;
    if has_model && state.collision_flags & 0x200 == 0 {
        for _ in 0..5 {
            if !retry {
                break;
            }
            let Some(pass) = mecc_collision_candidates::select_pass(
                &Actor {
                    id: actor,
                    current: state.current,
                    previous: state.previous,
                    radius: state.radius,
                    scale: state.scale,
                    height: state.height,
                    object_flags: state.object_flags,
                    collision_flags: state.collision_flags,
                    has_model,
                },
                &fields,
                precision,
            )?
            else {
                retry = false;
                break;
            };
            result.passes += 1;
            retry = false;
            for id in pass.candidates {
                let object = objects
                    .iter()
                    .find(|o| o.fields.id == id)
                    .ok_or("mesh candidate identity absent")?;
                let response = respond(state, object)?;
                state = response.response.state;
                retry = response.response.changed || response.preserve_retry && retry;
                result.responses += 1;
                if response.stop_pass {
                    break;
                }
            }
            if state.current == state.previous {
                break;
            }
        }
        if retry {
            state.current[0] = state.previous[0];
            state.current[1] = state.previous[1];
            result.fallback_xy = true;
        }
    }
    result.state = state;
    Ok(result)
}
