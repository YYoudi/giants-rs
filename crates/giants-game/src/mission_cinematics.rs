//! Mission startup request projection. No director clock, callbacks or runtime readiness.
use giants_formats::{mission::Mission, world::WorldConfig};

#[derive(Debug)]
pub struct Opening {
    pub mission_slot: usize,
    /// Absent command and explicitly empty command remain distinct.
    pub source_offset: Option<usize>,
    pub raw_name: Option<[u8; 32]>,
    pub world_index: Option<usize>,
    pub director: Option<String>,
}
pub fn select(world: &WorldConfig, mission: &Mission, name: &str) -> Result<Opening, String> {
    let slot = crate::mission_slot::index(world, name)?;
    let mut out = Opening {
        mission_slot: slot,
        source_offset: None,
        raw_name: None,
        world_index: None,
        director: None,
    };
    let Some(request) = mission.opening_cinematics.last() else {
        return Ok(out); // No claim about constructor defaults or absence of a movie.
    };
    out.source_offset = Some(request.source_offset);
    out.raw_name = Some(request.name);
    let end = request
        .name
        .iter()
        .position(|b| *b == 0)
        .ok_or("unterminated startup cinematic outside supported profile")?;
    let label = std::str::from_utf8(&request.name[..end])
        .map_err(|_| "invalid startup cinematic encoding")?;
    if !label.is_ascii() {
        return Err("startup cinematic outside ASCII matching profile".into());
    }
    if label.is_empty() {
        return Ok(out);
    }
    let index = world
        .cinematics
        .iter()
        .position(|c| c.name.eq_ignore_ascii_case(label))
        .ok_or("startup cinematic missing from world table")?;
    let canonical = &world.cinematics[index].name;
    if !canonical
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'_')
    {
        return Err("startup cinematic identifier outside supported profile".into());
    }
    out.world_index = Some(index);
    out.director = Some(canonical.clone());
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn slot_identity_last_request_and_explicit_empty_are_preserved() {
        use giants_formats::world::*;
        let fog = Fog {
            start: 1.,
            end: 100.,
            color: [0; 3],
        };
        let world = WorldConfig {
            prefix: TerrainPrefix {
                box_name: String::new(),
                terrain: String::new(),
                materials: std::array::from_fn(|_| {
                    Some(TerrainMaterial {
                        name: String::new(),
                        scale: 1.,
                        reciprocal: 1.,
                    })
                }),
                additional_materials: std::array::from_fn(|_| None),
                consumed_offset: 0,
                block_end: 0,
            },
            fog,
            water_fog: fog,
            water: WaterParameters::default(),
            sun_glare: None,
            sky: String::new(),
            sea_texture: String::new(),
            underside: String::new(),
            sea_values: [0.; 3],
            mip_curve: [0.; 3],
            slope_thresholds: [0.; 2],
            terrain_direction_floor: None,
            opcodes: vec![],
            lights: vec![],
            scenarios: vec![],
            placements: vec![],
            cinematics: vec![
                WorldCinematic {
                    source_offset: 10,
                    name: "Alpha".into(),
                    metadata: String::new(),
                },
                WorldCinematic {
                    source_offset: 75,
                    name: "Beta".into(),
                    metadata: String::new(),
                },
            ],
            mission_names: vec!["Other".into(), "Selected".into()],
            command74_present: false,
            command89: None,
        };
        let mut bytes = vec![4, 0, 0, 0, 1, 2];
        let empty = giants_formats::mission::parse(&bytes).unwrap();
        let selection = select(&world, &empty, "Selected").unwrap();
        assert_eq!(selection.mission_slot, 1);
        assert!(selection.source_offset.is_none());
        let parameters = crate::mission_slot::project(&world, &empty, "Selected").unwrap();
        assert_eq!(parameters.index, 1);
        assert!(parameters.parameter15.is_none());
        assert!(parameters.completion_cinematic.is_none());
        assert!(crate::mission_slot::project(&world, &empty, "missing").is_err());
        let mut extra = vec![4, 0, 0, 0, 1];
        for value in [i32::MIN, 20] {
            extra.push(15);
            extra.extend(value.to_le_bytes());
        }
        extra.push(25);
        extra.extend([0xff; 32]);
        let empty_at = extra.len();
        extra.push(25);
        extra.extend([0; 32]);
        extra.push(2);
        let projected = crate::mission_slot::project(
            &world,
            &giants_formats::mission::parse(&extra).unwrap(),
            "Selected",
        )
        .unwrap();
        assert_eq!(projected.parameter15, Some((10, 20)));
        assert_eq!(projected.completion_cinematic, Some((empty_at, [0; 32])));
        bytes.pop();
        for label in [b"Alpha".as_slice(), b"bEtA".as_slice()] {
            bytes.push(21);
            let mut field = [0xa5; 32];
            field[..label.len()].copy_from_slice(label);
            field[label.len()] = 0;
            bytes.extend(field);
        }
        bytes.push(2);
        let mission = giants_formats::mission::parse(&bytes).unwrap();
        let selection = select(&world, &mission, "Selected").unwrap();
        assert_eq!(selection.mission_slot, 1);
        assert_eq!(selection.source_offset, Some(38));
        assert_eq!(selection.world_index, Some(1));
        assert_eq!(selection.director.as_deref(), Some("Beta"));
        assert_eq!(selection.raw_name.unwrap()[31], 0xa5);
        assert!(select(&world, &mission, "selected").is_err()); // Mission lookup itself is case-sensitive.
        bytes[39] = 0;
        let mission = giants_formats::mission::parse(&bytes).unwrap();
        let selection = select(&world, &mission, "Selected").unwrap();
        assert_eq!(selection.source_offset, Some(38));
        assert!(selection.raw_name.is_some());
        assert!(selection.director.is_none());
        bytes[39..71].fill(b'X');
        let mission = giants_formats::mission::parse(&bytes).unwrap();
        assert!(
            select(&world, &mission, "Selected")
                .unwrap_err()
                .contains("unterminated")
        );
    }
}
