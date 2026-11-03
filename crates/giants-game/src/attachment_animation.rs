//! Standalone initial accessory loops. This admits vertex preparation only;
//! actor services, event dispatch, native scheduling and LOD remain separate.
use crate::{actor_context::Prepared, effect_animation::Playback};

/// Vertex-only preview of a constructed single loop, including lit model zero.
/// Class state services and native LOD selection are not executed here. Events
/// on the selected channel and procedural corrections remain explicit refusals.
pub fn initial_vertex_loop(actor: &Prepared) -> Result<Option<Playback>, String> {
    if actor.channels[0][0] == 254 {
        return Ok(None);
    }
    let initial = Prepared::class_initial(actor.class.clone());
    if actor.inline.is_some()
        || actor.director_slot.is_some()
        || actor.channels != initial.channels
        || !actor.needs.is_empty()
    {
        return Err("vertex loop requires unsupported constructed channels".into());
    }
    let spec = actor
        .class
        .specification
        .as_ref()
        .ok_or("vertex loop specification absent")?;
    let slot = u8::try_from(actor.channels[0][0]).map_err(|e| e.to_string())?;
    if spec
        .metadata
        .events
        .iter()
        .any(|track| track.slot == slot && !track.events.is_empty())
        || !spec.corrections.is_empty()
    {
        return Err("vertex loop requires selected-channel events or corrections".into());
    }
    let model = spec
        .models
        .first()
        .and_then(Option::as_ref)
        .ok_or("vertex loop model zero absent")?;
    if model.prepared.skin.is_none() {
        return Ok(None);
    }
    Playback::new(initial)
}

pub fn initial_unlit_loop(
    program: &giants_formats::class_program::Program,
    actor: &Prepared,
) -> Result<Option<Playback>, String> {
    if actor.channels[0][0] == 254 {
        return Ok(None);
    }
    let initial = Prepared::class_initial(actor.class.clone());
    if actor.inline.is_some()
        || actor.director_slot.is_some()
        || actor.channels != initial.channels
        || actor.class.state != 0
        || !actor.needs.is_empty()
        || program.records.iter().any(|r| {
            !matches!(
                r.opcode,
                1 | 0x13 | 0x14 | 0x16 | 0x1d | 0x22 | 0x36 | 0x17 | 0xff
            )
        })
    {
        return Err("accessory loop requires unsupported actor state or channels".into());
    }
    let spec = actor
        .class
        .specification
        .as_ref()
        .ok_or("accessory specification absent")?;
    let model = spec
        .models
        .first()
        .and_then(Option::as_ref)
        .ok_or("accessory model zero absent")?;
    if spec.models.iter().flatten().count() != 1
        || model.prepared.mesh.normal_count != 0
        || spec.metadata.events.iter().any(|t| !t.events.is_empty())
        || !spec.corrections.is_empty()
    {
        return Err(
            "accessory loop requires normals, additional models, events or corrections".into(),
        );
    }
    // Unskinned meshes retain their buffers; a declared descriptor alone does
    // not justify allocating a vertex clock or claiming a visible animation.
    if model.prepared.skin.is_none() {
        return Ok(None);
    }
    Playback::new(initial)
}
