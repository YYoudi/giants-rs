//! Native marker selection and copied pose, given an ordered object list.
//! List construction, live callbacks, continuity and team formation are separate.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pose {
    /// Raw words preserve bit copies, including non-numeric diagnostic values.
    pub position: [u32; 3],
    pub angles: [u32; 3],
    pub cached: [u32; 17],
}
#[derive(Clone, Debug)]
pub struct Candidate {
    pub id: u32,
    pub class: i32,
    pub active: u32,
    pub personality: u32,
    pub selector: u32,
    pub word174: u32,
    pub pose: Pose,
}
pub fn wanted_selector(kind: i32) -> u32 {
    match kind {
        2 | 133 => 1,
        3 => 2,
        _ => 3,
    }
}
pub fn eligible(kind: i32, candidate: &Candidate) -> bool {
    eligible_fields(
        kind,
        candidate.class,
        candidate.active,
        candidate.personality,
        candidate.selector,
        candidate.word174,
    )
}
pub fn eligible_fields(
    kind: i32,
    class: i32,
    active: u32,
    personality: u32,
    selector: u32,
    word174: u32,
) -> bool {
    class == 679
        && personality & 0x0800_0000 != 0
        && active & 0x8000 == 0
        && word174 == 0
        && selector == wanted_selector(kind)
}
/// The order is a caller input, never inferred by sorting IDs or declarations.
pub fn select(kind: i32, candidates: &[Candidate]) -> Option<&Candidate> {
    candidates
        .iter()
        .find(|candidate| eligible(kind, candidate))
}
pub fn copy(kind: i32, candidates: &[Candidate], player: &mut Pose) -> Option<u32> {
    let selected = select(kind, candidates)?;
    *player = selected.pose;
    Some(selected.id)
}

#[derive(Debug, Clone)]
pub struct Declared {
    pub start_offset: usize,
    pub marker_index: usize,
    pub marker_offset: usize,
    pub kind: i32,
    pub position: [f32; 3],
    pub angles: [f32; 3],
}
/// Initial cold mission markers use their declared creation order. The exact-D
/// constructor appends to the common list, and selection stops at its first match.
/// World/child markers, other class programs, repeated starts and continuity
/// still require their own consumers.
pub fn declared(
    plan: &crate::instances::Plan,
    world: &giants_formats::world::WorldConfig,
    mission: &giants_formats::mission::Mission,
) -> Result<Declared, String> {
    declared_for_kind(plan, world, mission, 2)
}

/// Project the requested character's cold marker declarations. This does not
/// run its constructor, establish a live common list or admit player controls.
/// The existing Mecc entry point remains restricted to kind 2.
pub fn declared_for_kind(
    plan: &crate::instances::Plan,
    world: &giants_formats::world::WorldConfig,
    mission: &giants_formats::mission::Mission,
    expected_kind: i32,
) -> Result<Declared, String> {
    use giants_formats::mission::ObjectRef;
    let start = cold_start(&mission.starts, expected_kind)?;
    let kind = start.words[0] as i32;
    if plan
        .entries
        .windows(2)
        .any(|pair| pair[0].serialized.source_offset >= pair[1].serialized.source_offset)
    {
        return Err("initial marker plan does not preserve declaration order".into());
    }
    if mission.locked_children.iter().any(|p| p.object_type == 679) {
        return Err("child marker initialization and ordering remain unresolved".into());
    }
    for marker in world.placements.iter().filter(|p| p.object_type == 679) {
        if marker
            .selector
            .is_some_and(|selector| u32::from(selector) != wanted_selector(kind))
            || marker.word_174.is_some_and(|word| word != 0)
        {
            continue;
        }
        return Err(
            "potential world marker requires proven initialization and live-list order".into(),
        );
    }
    let mut selected = None;
    for entry in &plan.entries {
        if entry.serialized.object_type != 679
            || entry.serialized.source_offset >= start.source_offset
        {
            continue;
        }
        let source = entry.source.as_ref().map_err(Clone::clone)?;
        let program = &plan
            .resources()
            .classes
            .models
            .first_program(679)
            .ok_or("marker class missing")?
            .program;
        let program = program.as_ref().map_err(Clone::clone)?;
        if program
            .records
            .iter()
            .any(|r| !matches!(r.opcode, 1 | 0x1d | 0x36 | 0x22 | 0x41 | 0xff))
        {
            return Err("marker class outside controlled cold selector profile".into());
        }
        if source
            .state_seed
            .as_ref()
            .map_err(Clone::clone)?
            .storage
            .state()
            != 0
        {
            return Err("marker class state outside zero-state profile".into());
        }
        let owner = ObjectRef::Placement(entry.index);
        let selector = mission
            .object_selectors
            .iter()
            .rev()
            .find(|w| w.object == owner && w.source_offset < start.source_offset)
            .map(|w| u32::from(w.value))
            .ok_or("marker selector requires an explicit pre-start declaration")?;
        let word174 = mission
            .object_properties
            .iter()
            .rev()
            .find(|w| w.object == owner && w.source_offset < start.source_offset)
            .map_or(0, |w| w.value);
        if !eligible_fields(
            kind,
            source.class,
            source.direct_flags.active,
            source.direct_flags.personality,
            selector,
            word174,
        ) {
            continue;
        }
        let mut angles = [entry.serialized.yaw, 0., 0.];
        if let Some([pitch, roll]) = entry.serialized.pitch_roll {
            angles[1] = pitch;
            angles[2] = roll;
        }
        for write in mission
            .orientations
            .iter()
            .filter(|w| w.object == owner && w.source_offset < start.source_offset)
        {
            angles = write.angles;
        }
        selected.get_or_insert(Declared {
            start_offset: start.source_offset,
            marker_index: entry.index,
            marker_offset: entry.serialized.source_offset,
            kind,
            position: entry.serialized.position,
            angles,
        });
    }
    selected.ok_or_else(|| "no eligible declared character start marker".into())
}

fn cold_start(
    starts: &[giants_formats::mission::Start],
    expected_kind: i32,
) -> Result<&giants_formats::mission::Start, String> {
    if starts.len() != 1 {
        return Err("initial start projection requires one command 11".into());
    }
    let start = &starts[0];
    if !matches!(expected_kind, 1..=3)
        || start.words[0] as i32 != expected_kind
        || start.words[2] == 0
    {
        return Err("initial character marker relocation not requested by command 11".into());
    }
    Ok(start)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn character_preparation_cannot_reinterpret_another_character_start() {
        for kind in 1..=3 {
            let starts = [giants_formats::mission::Start {
                source_offset: 12,
                words: [kind, 0, 1, 0],
            }];
            for expected in 1..=3 {
                assert_eq!(
                    cold_start(&starts, expected).is_ok(),
                    kind as i32 == expected
                );
            }
            assert_eq!(
                wanted_selector(kind as i32),
                if kind == 2 {
                    1
                } else if kind == 3 {
                    2
                } else {
                    3
                }
            );
        }
    }
    #[test]
    fn resource_preparation_requires_one_explicit_relocation() {
        let mut starts = vec![giants_formats::mission::Start {
            source_offset: 12,
            words: [3, 0, 0, 0],
        }];
        assert!(cold_start(&starts, 3).is_err());
        starts[0].words[2] = 1;
        assert!(cold_start(&starts, 3).is_ok());
        starts.push(starts[0].clone());
        assert!(cold_start(&starts, 3).is_err());
        assert!(cold_start(&[], 3).is_err());
        assert!(cold_start(&starts[..1], 133).is_err());
    }
}
