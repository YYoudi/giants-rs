//! Initial lap-indicator attachments. Parent time is supplied by its prepared
//! pose; race progress, live clocks and indicator retirement remain separate.
use crate::{
    animation_references::References, procedural_aim::Matrix, scene_resources::SceneResources,
};

pub fn matrices(
    resources: &SceneResources,
    parent_matrix: [f32; 16],
    parent_tick: i32,
) -> Result<Vec<(usize, [f32; 16])>, String> {
    if !parent_matrix.iter().all(|v| v.is_finite()) {
        return Err("non-finite race gate transform".into());
    }
    let mut bindings = Vec::new();
    for class in [1147, 1160] {
        let binding = resources
            .classes
            .models
            .first_selection(class)
            .ok_or("race attachment class missing")?
            .as_ref()
            .map_err(Clone::clone)?;
        let spec = resources
            .classes
            .models
            .specs
            .get(&binding.spec_id)
            .and_then(|v| v.first())
            .ok_or("race attachment specification missing")?;
        if spec.catalog != binding.spec_catalog {
            return Err("race attachment specification context changed".into());
        }
        let channels: Vec<_> = binding
            .selection
            .animations
            .iter()
            .filter(|v| v.slot == 1)
            .collect();
        if channels.len() != 1 {
            return Err("race attachment channel1 missing or ambiguous".into());
        }
        let animation = resources
            .animation(&channels[0].name)
            .map_err(|e| e.to_string())?;
        let references = References::from_program(spec.program.as_ref().map_err(Clone::clone)?)
            .map_err(|e| e.to_string())?;
        bindings.push((animation, references));
    }
    let (child, child_names) = &bindings[1];
    let child_tick = crate::model_pose::seconds_tick(child.timeline, 0.)?;
    let inverse = child_names
        .bind(child)
        .map_err(|e| e.to_string())?
        .evaluate(0, child_tick)
        .map_err(|e| e.to_string())?
        .matrix
        .ok_or("race indicator center reference unresolved")?
        .inverse()?;
    let (parent, parent_names) = &bindings[0];
    let mut refs = parent_names.bind(parent).map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    for slot in 2..7 {
        let local = refs
            .evaluate(slot, parent_tick)
            .map_err(|e| e.to_string())?
            .matrix
            .ok_or("race gate lap reference unresolved")?;
        let world = Matrix {
            flags: 31,
            values: parent_matrix,
        }
        .mul(local)?
        .mul(inverse)?;
        result.push((slot, world.values));
    }
    Ok(result)
}
