//! Source base-layout initialization preview. No AI, mission readiness or
//! live visibility is inferred from these construction requests.
use crate::{
    animation_references::References,
    procedural_aim::Matrix,
    scene_resources::{SceneResources, SourcePlacement},
};

pub struct Request {
    pub reference: usize,
    pub source: SourcePlacement,
}
pub fn requests(
    resources: &SceneResources,
    layout: &SourcePlacement,
    matrix: [f32; 16],
) -> Result<Vec<Request>, String> {
    if !matrix.iter().all(|v| v.is_finite()) {
        return Err("non-finite layout transform".into());
    }
    if layout.class != 985
        || layout.direct_flags.active & 0x8000 != 0
        || layout
            .state_seed
            .as_ref()
            .map_err(Clone::clone)?
            .storage
            .state()
            != 91
    {
        return Err("base layout outside checked Reaper disposition profile".into());
    }
    let selected = resources
        .classes
        .models
        .first_selection(layout.class)
        .ok_or("layout selection missing")?
        .as_ref()
        .map_err(Clone::clone)?;
    let spec = resources
        .classes
        .models
        .specs
        .get(&selected.spec_id)
        .and_then(|v| v.first())
        .ok_or("layout specification missing")?;
    if spec.catalog != selected.spec_catalog {
        return Err("layout specification context changed".into());
    }
    let names = References::from_program(spec.program.as_ref().map_err(Clone::clone)?)
        .map_err(|e| e.to_string())?;
    let channels = selected
        .selection
        .animations
        .iter()
        .filter(|a| a.slot == 1)
        .collect::<Vec<_>>();
    if channels.len() != 1 {
        return Err("layout channel1 missing or ambiguous".into());
    }
    let animation = resources
        .animation(&channels[0].name)
        .map_err(|e| e.to_string())?;
    let tick = crate::model_pose::seconds_tick(animation.timeline, 0.05)?;
    let mut references = names.bind(&animation).map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    for (slot, class) in [(2, 729), (0, 858), (14, 851), (15, 851)] {
        let local = references
            .evaluate(slot, tick)
            .map_err(|e| e.to_string())?
            .matrix
            .ok_or("layout required reference is unresolved")?;
        let world = Matrix {
            flags: 31,
            values: matrix,
        }
        .mul(local)?;
        let m = world.values;
        // Preview angle conversion; the native lookup approximation remains
        // separate. Positions retain the checked source matrix arithmetic.
        let mut angle = f64::from(-m[5]).atan2(f64::from(-m[1])).to_degrees();
        if angle < 0. {
            angle += 360.;
        }
        let yaw = (angle + 90.) as f32;
        let placement = giants_formats::mission::Placement {
            source_offset: layout.source_offset,
            object_type: class,
            position: [m[3], m[7], m[11]],
            yaw,
            pitch_roll: None,
            scale: None,
        };
        result.push(Request {
            reference: slot,
            source: resources
                .placement_input(&placement)
                .map_err(|e| e.to_string())?,
        });
    }
    Ok(result)
}
