//! Source-declared corrections between two nodes in one animation hierarchy.
//! Preparing a correction does not attach an object or select a live actor.
use crate::{
    animation_references::{References, Resolution, node_matrix},
    procedural_aim::Matrix,
};
use giants_formats::{Error, anm::Animation, object_spec::Program};
use std::collections::BTreeMap;

/// Prepare only the source clips requested by registered correction records.
/// Preserve the captured specification context and reject missing resources.
pub fn prepare_registered(
    resources: &crate::scene_resources::SceneResources,
    class: i32,
) -> Result<Vec<Link>, String> {
    let selected = resources
        .classes
        .models
        .first_selection(class)
        .ok_or("reference correction class absent")?
        .as_ref()
        .map_err(Clone::clone)?;
    let definition = resources
        .classes
        .models
        .specs
        .get(&selected.spec_id)
        .and_then(|s| s.first())
        .ok_or("reference correction specification absent")?;
    if definition.catalog != selected.spec_catalog {
        return Err("reference correction specification context changed".into());
    }
    let program = definition.program.as_ref().map_err(Clone::clone)?;
    let mut animations = BTreeMap::new();
    for record in program.records.iter().filter(|r| r.opcode == 0x54) {
        let slot = *record.bytes.get(2).ok_or("reference correction framing")?;
        if let std::collections::btree_map::Entry::Vacant(entry) = animations.entry(slot) {
            let source = selected
                .selection
                .animations
                .iter()
                .find(|a| a.slot == slot)
                .ok_or("reference correction clip undeclared")?;
            entry.insert(
                resources
                    .animation(&source.name)
                    .map_err(|e| e.to_string())?,
            );
        }
    }
    let sources = animations
        .iter()
        .map(|(slot, animation)| (*slot, animation))
        .collect();
    prepare(program, &sources).map_err(|e| e.to_string())
}

/// Apply the prepared correction to a current raw reference, not to the whole
/// animation pose. Duplicate corrections need their ordered runtime consumer.
pub fn apply(slot: u8, current: Matrix, links: &[Link]) -> Result<Matrix, Error> {
    let mut matches = links.iter().filter(|link| link.reference_slot == slot);
    let Some(link) = matches.next() else {
        return Ok(current);
    };
    if matches.next().is_some() {
        return Err(Error("duplicate reference correction consumer unsupported"));
    }
    match &link.outcome {
        Outcome::Prepared(correction) => {
            if !current
                .values
                .into_iter()
                .chain(correction.values)
                .all(f32::is_finite)
            {
                return Err(Error("nonfinite current reference correction input"));
            }
            current.mul(*correction).map_err(Error)
        }
        _ => Err(Error("current reference correction is not prepared")),
    }
}

#[derive(Debug)]
pub enum Outcome {
    Prepared(Matrix),
    MissingAnimation,
    MissingTarget,
    MissingReference(Resolution),
}
#[derive(Debug)]
pub struct Link {
    pub source_offset: usize,
    pub reference_slot: u8,
    pub animation_slot: u8,
    pub target: String,
    pub frame: f32,
    pub tick: Option<i32>,
    pub outcome: Outcome,
}

/// The source value is an absolute frame, multiplied by ticks per frame and
/// truncated toward zero. It is not a duration or a normalized clip fraction.
pub fn frame_tick(frame: f32, ticks_per_frame: i32) -> Result<i32, Error> {
    let value = f64::from(frame) * f64::from(ticks_per_frame);
    if !frame.is_finite()
        || ticks_per_frame <= 0
        || value.trunc() < f64::from(i32::MIN)
        || value.trunc() > f64::from(i32::MAX)
    {
        return Err(Error("invalid reference-link frame"));
    }
    Ok(value.trunc() as i32)
}

pub fn prepare(
    program: &Program,
    animations: &BTreeMap<u8, &Animation>,
) -> Result<Vec<Link>, Error> {
    let references = References::from_program(program)?;
    let mut out = Vec::new();
    for record in &program.records {
        if record.opcode != 0x54 {
            continue;
        }
        if out.len() >= 128 {
            return Err(Error("reference-link budget"));
        }
        let b = &record.bytes;
        if b.len() != 23 || b[1] == 0 || b[2] >= 254 {
            return Err(Error("invalid reference-link declaration"));
        }
        let raw = &b[3..19];
        let end = raw
            .iter()
            .position(|c| *c == 0)
            .ok_or(Error("unterminated reference-link target"))?;
        if !raw[..end].is_ascii() {
            return Err(Error("non-ASCII reference-link target"));
        }
        let mut link = Link {
            source_offset: record.offset,
            reference_slot: b[1],
            animation_slot: b[2],
            target: String::from_utf8(raw[..end].to_vec()).unwrap(),
            frame: f32::from_le_bytes(b[19..23].try_into().unwrap()),
            tick: None,
            outcome: Outcome::MissingAnimation,
        };
        if !link.frame.is_finite() {
            return Err(Error("nonfinite reference-link frame"));
        }
        if let Some(animation) = animations.get(&link.animation_slot) {
            let tick = frame_tick(link.frame, animation.timeline[2])?;
            link.tick = Some(tick);
            // Resolve names independently so absence remains distinct from an
            // unsupported name-comparison profile or invalid source graph.
            let target_names = References {
                names: vec![Some(link.target.clone())],
            };
            let target = target_names.bind(animation)?.resolve(0)?.node();
            let reference = references
                .bind(animation)?
                .resolve(usize::from(link.reference_slot))?;
            link.outcome = match (target, reference.node()) {
                (Some(target), Some(reference)) => {
                    let target = node_matrix(animation, target, tick)?;
                    let reference = node_matrix(animation, reference, tick)?;
                    Outcome::Prepared(
                        reference
                            .inverse()
                            .map_err(Error)?
                            .mul(target)
                            .map_err(Error)?,
                    )
                }
                (None, _) => Outcome::MissingTarget,
                (_, None) => Outcome::MissingReference(reference),
            };
        }
        out.push(link);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_frames_are_absolute_and_truncated_with_checked_range() {
        assert_eq!(frame_tick(1.125, 160).unwrap(), 180);
        assert_eq!(frame_tick(-0.01, 160).unwrap(), -1);
        assert_eq!(frame_tick(0., 160).unwrap(), 0);
        assert!(frame_tick(f32::NAN, 160).is_err());
        assert!(frame_tick(f32::INFINITY, 160).is_err());
        assert!(frame_tick(f32::MAX, 160).is_err());
        assert!(frame_tick(1., 0).is_err());
    }
    #[test]
    fn missing_context_remains_unprepared_and_invalid_slots_are_rejected() {
        let mut bytes = vec![0x54, 2, 250];
        bytes.extend_from_slice(b"ref_thrust\0\0\0\0\0\0");
        bytes.extend_from_slice(&1_f32.to_le_bytes());
        bytes.push(255);
        let links = prepare(
            &giants_formats::object_spec::parse(&bytes).unwrap(),
            &BTreeMap::new(),
        )
        .unwrap();
        assert!(matches!(links[0].outcome, Outcome::MissingAnimation));
        assert_eq!(links[0].tick, None);
        assert!(apply(2, Matrix::identity(), &links).is_err());
        assert_eq!(
            apply(3, Matrix::identity(), &links).unwrap().values,
            Matrix::identity().values
        );
        for (reference, animation) in [(0, 1), (2, 254)] {
            bytes[1] = reference;
            bytes[2] = animation;
            assert!(
                prepare(
                    &giants_formats::object_spec::parse(&bytes).unwrap(),
                    &BTreeMap::new()
                )
                .is_err()
            );
        }
    }
}
