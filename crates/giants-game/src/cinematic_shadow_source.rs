//! Shared source silhouette preparation. Clock, cursor, model and masks are caller-owned.
use crate::{
    actor_links::Projection,
    cinematic_shadow::{Declaration, SilhouetteCamera},
    procedural_aim::Matrix,
};
use std::collections::BTreeMap;

pub struct Request<'a> {
    pub declaration: Declaration,
    pub cursor_offset: usize,
    pub model_slot: usize,
    pub part_mask: u64,
    pub clocks: &'a BTreeMap<usize, f32>,
    pub locals: &'a BTreeMap<usize, Option<Matrix>>,
    pub retained_positions: Option<&'a [[f32; 3]]>,
}
pub struct Sample {
    pub position_buffer: Vec<[f32; 3]>,
    pub frame: Matrix,
    pub source_world: Matrix,
    pub camera: SilhouetteCamera,
    pub extent: [u32; 2],
    pub positions: Vec<[f32; 3]>,
}
pub fn prepare(actors: &Projection, request: Request<'_>) -> Result<Sample, String> {
    let declaration = request.declaration;
    if declaration.command_offset > request.cursor_offset || request.model_slot >= 10 {
        return Err("shadow declaration/model not reached or outside profile".into());
    }
    let actor = actors
        .actors
        .get(&declaration.actor_slot)
        .ok_or("shadow actor absent")?;
    if actor.command_offset > declaration.command_offset {
        return Err("shadow actor not declared before request".into());
    }
    let model = actor
        .class
        .specification
        .as_ref()
        .ok_or("shadow model table absent")?
        .models
        .get(request.model_slot)
        .and_then(Option::as_ref)
        .ok_or("shadow selected model absent")?;
    let mesh = &model.prepared.mesh;
    if mesh.subobjects.len() > 64 {
        return Err("shadow part mask exceeds bounded profile".into());
    }
    let clock = *request
        .clocks
        .get(&declaration.actor_slot)
        .ok_or("shadow actor clock absent")?;
    let positions = match actor.position_update(request.model_slot, clock)? {
        crate::actor_context::PositionUpdate::Replace(positions) => positions,
        crate::actor_context::PositionUpdate::Keep => request
            .retained_positions
            .ok_or("shadow retained positions required")?
            .to_vec(),
    };
    if positions.len() != mesh.positions.len() || positions.iter().flatten().any(|v| !v.is_finite())
    {
        return Err("shadow position buffer differs from selected model".into());
    }
    let placement = actors.placement_matrix(
        request.cursor_offset,
        declaration.actor_slot,
        request.clocks,
        request.locals,
    )?;
    let frame = crate::cinematic_shadow::projector_frame(placement)?;
    let source_world = actors.matrix(
        request.cursor_offset,
        declaration.actor_slot,
        request.clocks,
        request.locals,
    )?;
    let extent = declaration
        .texture_words
        .map(|word| 1u32.checked_shl(word).unwrap_or(0));
    let bounds = [
        declaration.bounds_words[0][0],
        declaration.bounds_words[0][1],
        declaration.bounds_words[1][0],
        declaration.bounds_words[1][1],
    ]
    .map(f32::from_bits);
    let camera = SilhouetteCamera::prepare(frame, bounds, extent)?;
    let mut vertices = Vec::new();
    for (part, sub) in mesh.subobjects.iter().enumerate() {
        if request.part_mask & (1u64 << part) == 0 {
            continue;
        }
        for triangle in &sub.triangles {
            if vertices.len() + 3 > 900_000 {
                return Err("shadow triangle budget exceeded".into());
            }
            for &index in triangle {
                let index = *mesh
                    .indices
                    .get(usize::from(index))
                    .ok_or("shadow mesh index absent")? as usize;
                vertices.push(*positions.get(index).ok_or("shadow position absent")?);
            }
        }
    }
    if vertices.is_empty() {
        return Err("shadow part mask contains no geometry".into());
    }
    Ok(Sample {
        position_buffer: positions,
        frame,
        source_world,
        camera,
        extent,
        positions: vertices,
    })
}
