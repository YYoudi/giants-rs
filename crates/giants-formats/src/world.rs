//! Bounded command-block traversal and environment data; object payloads opaque.
use crate::{Error, slice, u32_at};

#[derive(Clone, Debug)]
pub struct TerrainMaterial {
    pub name: String,
    pub scale: f32,
    pub reciprocal: f32,
}
pub struct TerrainPrefix {
    pub box_name: String,
    pub terrain: String,
    /// Declared overrides only. None does not imply a default texture or scale.
    pub materials: [Option<TerrainMaterial>; 3],
    /// Commands 114..116 are separate from the base overrides (52..54).
    pub additional_materials: [Option<TerrainMaterial>; 3],
    pub consumed_offset: usize,
    pub block_end: usize,
}
#[derive(Clone, Copy, Debug)]
pub struct Fog {
    pub start: f32,
    pub end: f32,
    pub color: [u8; 3],
}
/// Command 118. Other lens-effect records (119/120) remain opaque.
#[derive(Clone, Debug)]
pub struct SunGlare {
    pub effect: String,
    pub color: [f32; 3],
    pub exponent: f32,
    pub scale: f32,
}
/// Command 121: band tint, five tile-layer tints, then five water scalars.
#[derive(Clone, Copy, Debug)]
pub struct WaterParameters {
    pub colors: [[u8; 3]; 6],
    pub values: [f32; 5],
}
impl Default for WaterParameters {
    fn default() -> Self {
        let mut colors = [[0; 3]; 6];
        colors[0] = [0, 0, 128];
        Self {
            colors,
            values: [32., 4000., 1., 1., 1.5],
        }
    }
}
pub struct WorldConfig {
    pub prefix: TerrainPrefix,
    pub fog: Fog,
    pub water_fog: Fog,
    pub water: WaterParameters,
    pub sun_glare: Option<SunGlare>,
    pub sky: String,
    pub sea_texture: String,
    pub underside: String,
    pub sea_values: [f32; 3],
    pub mip_curve: [f32; 3],
    pub slope_thresholds: [f32; 2],
    /// Command 117 override. Missing does not establish the native default.
    pub terrain_direction_floor: Option<f32>,
    pub opcodes: Vec<(usize, u8)>,
    pub lights: Vec<Light>,
    pub scenarios: Vec<Scenario>,
    pub placements: Vec<WorldPlacement>,
    /// Command 79, ordered world-local cinematic names and an opaque string.
    pub cinematics: Vec<WorldCinematic>,
    /// Command 93 names the mission-state slots; distinct from scenarios (92).
    pub mission_names: Vec<String>,
    /// Declaration presence only; consumers of the native flag remain unknown.
    pub command74_present: bool,
    /// Bounded copied string; its downstream purpose is not established.
    pub command89: Option<String>,
}
pub struct WorldCinematic {
    pub source_offset: usize,
    pub name: String,
    pub metadata: String,
}
/// Direct world pose declarations. Omitted fields retain their template values;
/// other world commands are not replayed by this data reader.
#[derive(Clone, Debug)]
pub struct WorldPlacement {
    pub source_offset: usize,
    pub object_type: u32,
    pub position: [f32; 3],
    pub yaw: f32,
    pub pitch_roll: Option<[f32; 2]>,
    pub scale: Option<f32>,
    /// Direct command 59 override, independent of the class default.
    pub selector: Option<u8>,
    /// Direct command 82 override; behavior meaning remains unspecified.
    pub word_174: Option<u32>,
    /// Command78: object-local cinematic exclusions.
    pub cinematic_exclusions: Vec<[u8; 32]>,
    pub state_writes: Vec<WorldStateWrite>,
    /// Raw command90 header words; their interpretation belongs to the actor.
    pub values_f4: Option<[u32; 3]>,
}
/// Declared ordinary object-state stores. A consumer must validate its state;
/// state36 has a distinct command50 branch and is not covered by this projection.
#[derive(Clone, Debug)]
pub enum WorldStateWrite {
    Bytes8And28([u8; 2]),
    Word4(u32),
    Triple1c([u32; 3]),
}
#[derive(Debug)]
pub struct Scenario {
    pub kind: u8,
    pub parameter: u32,
    pub name: String,
}
/// Demonstrated world object type 1004. Other object payloads remain opaque.
#[derive(Clone, Copy, Debug)]
pub struct Light {
    pub source_offset: usize,
    pub position: [f32; 3],
    pub angles: [f32; 3],
    pub scale: f32,
    pub variant: Option<u8>,
    pub color: Option<[f32; 3]>,
}
fn floats<const N: usize>(data: &[u8], at: usize) -> Result<[f32; N], Error> {
    let mut out = [0.; N];
    for (i, value) in out.iter_mut().enumerate() {
        *value = f32::from_bits(u32_at(data, at + i * 4)?);
        if !value.is_finite() {
            return Err(Error("non-finite world scalar"));
        }
    }
    Ok(out)
}
/// Walk demonstrated record boundaries; object/gameplay payloads remain opaque.
pub fn parse_world(data: &[u8]) -> Result<WorldConfig, Error> {
    let mut prefix = terrain_prefix(data)?;
    let mut terrain_direction_floor = None;
    let bounded = &data[..prefix.block_end];
    let mut at = u32_at(data, 4)? as usize + 4;
    name(bounded, &mut at, 128)?;
    name(bounded, &mut at, 128)?;
    let mut fog = None;
    let mut water_fog = None;
    let mut water = WaterParameters::default();
    let mut sun_glare = None;
    let mut sky = None;
    let mut sea_texture = None;
    let mut underside = None;
    let mut sea_values = None;
    let mut mip_curve = [1., -0.1, -0.05];
    let mut slope_thresholds = crate::gti::DEFAULT_SLOPE_THRESHOLDS;
    let mut opcodes = Vec::new();
    let mut terminated = false;
    let mut lights: Vec<Light> = Vec::new();
    let mut scenarios = Vec::new();
    let mut current_light: Option<usize> = None;
    let mut placements: Vec<WorldPlacement> = Vec::new();
    let mut current_placement: Option<usize> = None;
    let mut cinematics = Vec::new();
    let mut mission_names = Vec::new();
    let mut command74_present = false;
    let mut command89 = None;
    while at < prefix.block_end {
        let opcode = bounded[at];
        opcodes.push((at, opcode));
        if opcodes.len() > 8192 {
            return Err(Error("world opcode budget exceeded"));
        }
        let length = match opcode {
            89 | 118 | 132 => {
                let mut p = at + 1;
                name(bounded, &mut p, 128)?;
                p - at + if opcode == 118 { 20 } else { 0 }
            }
            23 | 96 | 117 => 5,
            30 | 37 | 38 => 17,
            39 => 18,
            40 => 4,
            41 | 88 => 12,
            42 => 21,
            46 | 90 | 95 | 101 => 13,
            48 => 19,
            52..=54 | 114..=116 => 45,
            57 | 70 => 29,
            59 => 2,
            68 | 69 | 74 | 255 => 1,
            79 => 65,
            92 => 38,
            93 => 33,
            102 => 9,
            119 | 120 => 49,
            121 => 93,
            // Reuse the separately controlled table framing, without treating
            // opaque object/gameplay payloads as implemented runtime effects.
            _ => crate::world_tables::record_length(bounded, at)?,
        };
        let record = slice(bounded, at, length)?;
        match opcode {
            90 => {
                if let Some(i) = current_placement {
                    placements[i].values_f4 =
                        Some([u32_at(record, 1)?, u32_at(record, 5)?, u32_at(record, 9)?]);
                }
            }
            50 | 83 | 91 => {
                if let Some(i) = current_placement {
                    let write = match opcode {
                        50 => WorldStateWrite::Bytes8And28([record[1], record[2]]),
                        83 => WorldStateWrite::Word4(u32_at(record, 1)?),
                        _ => WorldStateWrite::Triple1c([
                            u32_at(record, 1)?,
                            u32_at(record, 5)?,
                            u32_at(record, 9)?,
                        ]),
                    };
                    placements[i].state_writes.push(write);
                }
            }
            59 => {
                if let Some(i) = current_placement {
                    placements[i].selector = Some(record[1]);
                }
                if let Some(i) = current_light {
                    lights[i].variant = Some(record[1]);
                }
            }
            82 => {
                if let Some(i) = current_placement {
                    placements[i].word_174 = Some(u32_at(record, 1)?);
                }
            }
            74 => command74_present = true,
            89 => {
                let mut p = 1;
                command89 = Some(name(record, &mut p, 128)?);
            }
            78 => {
                let index =
                    current_placement.ok_or(Error("world cinematic exclusion without object"))?;
                placements[index]
                    .cinematic_exclusions
                    .push(record[1..33].try_into().unwrap());
            }
            79 => {
                if cinematics.len() >= 128 {
                    return Err(Error("world cinematic table outside supported capacity"));
                }
                let mut cursor = 0;
                let label = name(&record[1..33], &mut cursor, 32)?;
                let mut cursor = 0;
                let metadata = name(&record[33..65], &mut cursor, 32)?;
                cinematics.push(WorldCinematic {
                    source_offset: at,
                    name: label,
                    metadata,
                });
            }
            93 => {
                if mission_names.len() >= 32 {
                    return Err(Error("world mission slots outside supported capacity"));
                }
                let mut at = 0;
                let label = name(&record[1..33], &mut at, 32)?;
                if label.is_empty() {
                    return Err(Error("empty mission slot outside supported profile"));
                }
                mission_names.push(label);
            }
            118 => {
                let mut p = 1;
                let effect = name(record, &mut p, 128)?;
                let [r, g, b, exponent, scale] = floats(record, p)?;
                sun_glare = Some(SunGlare {
                    effect,
                    color: [r, g, b],
                    exponent,
                    scale,
                });
            }
            92 => {
                if scenarios.len() >= 27 {
                    return Err(Error("world scenario budget exceeded"));
                }
                let mut p = 6;
                scenarios.push(Scenario {
                    kind: record[1],
                    parameter: u32_at(record, 2)?,
                    name: name(record, &mut p, 32)?,
                });
            }
            42 | 70 => {
                current_light = None;
                let object_type = u32_at(record, 1)?;
                let position = floats(record, 5)?;
                let yaw = floats::<1>(record, 17)?[0];
                let pitch_roll = if opcode == 70 {
                    Some(floats(record, 21)?)
                } else {
                    None
                };
                current_placement = Some(placements.len());
                placements.push(WorldPlacement {
                    source_offset: at,
                    object_type,
                    position,
                    yaw,
                    pitch_roll,
                    scale: None,
                    selector: None,
                    word_174: None,
                    state_writes: Vec::new(),
                    values_f4: None,
                    cinematic_exclusions: Vec::new(),
                });
                if object_type == 1004 {
                    let [pitch, roll] = pitch_roll.unwrap_or([0., 0.]);
                    let angles = [yaw, pitch, roll];
                    current_light = Some(lights.len());
                    lights.push(Light {
                        source_offset: at,
                        position,
                        angles,
                        scale: 1.,
                        variant: None,
                        color: None,
                    });
                }
            }
            95 => {
                if let Some(i) = current_light {
                    lights[i].color = Some(floats(record, 1)?);
                }
            }
            23 => {
                let scale = floats::<1>(record, 1)?[0];
                if let Some(i) = current_placement {
                    placements[i].scale = Some(scale);
                }
                if let Some(i) = current_light {
                    lights[i].scale = scale;
                }
            }
            41 | 88 => {
                let [start, end] = floats(record, 1)?;
                if start < 0. || start >= end {
                    return Err(Error("invalid world fog interval"));
                }
                let value = Some(Fog {
                    start,
                    end,
                    color: record[9..12].try_into().unwrap(),
                });
                if opcode == 41 {
                    fog = value;
                } else {
                    water_fog = value;
                }
            }
            30 | 37 | 38 => {
                let mut p = 0;
                let value = name(&record[1..17], &mut p, 16)?;
                if value.is_empty() {
                    return Err(Error("empty environment texture reference"));
                }
                match opcode {
                    30 => sky = Some(value),
                    37 => sea_texture = Some(value),
                    _ => underside = Some(value),
                }
            }
            46 => {
                let values = floats(record, 1)?;
                if values[1] <= 0. || !(1. / values[1]).is_finite() {
                    return Err(Error("invalid world sea scalars"));
                }
                sea_values = Some(values);
            }
            52..=54 | 114..=116 => {
                let (slots, index) = if opcode <= 54 {
                    (&mut prefix.materials, opcode - 52)
                } else {
                    (&mut prefix.additional_materials, opcode - 114)
                };
                slots[index as usize] = Some(terrain_material(record)?);
            }
            101 => mip_curve = floats(record, 1)?,
            121 => {
                let raw: [f32; 23] = floats(record, 1)?;
                // This reader supports normalized RGB input. Preserve the native
                // truncation after extended-precision multiplication, without rounding.
                if !raw[..18].iter().all(|v| (0. ..=1.).contains(v)) {
                    return Err(Error("unsupported water color range"));
                }
                for (color, source) in water.colors.iter_mut().zip(raw[..18].as_chunks::<3>().0) {
                    for (channel, value) in color.iter_mut().zip(source) {
                        *channel = (f64::from(*value) * 255.) as u8;
                    }
                }
                water.values.copy_from_slice(&raw[18..]);
            }
            102 => {
                let angles: [f32; 2] = floats(record, 1)?;
                if !angles.iter().all(|v| (0. ..=180.).contains(v)) {
                    return Err(Error("invalid slope angles"));
                }
                slope_thresholds =
                    angles.map(|v| ((f64::from(v) * f64::from(0.017453292f32)).cos()) as f32);
            }
            117 => terrain_direction_floor = Some(floats::<1>(record, 1)?[0]),
            _ => {}
        }
        at += length;
        if opcode == 255 {
            terminated = true;
            break;
        }
    }
    if !terminated || at != prefix.block_end {
        return Err(Error("world terminator does not end block"));
    }
    Ok(WorldConfig {
        prefix,
        fog: fog.ok_or(Error("missing world fog"))?,
        water_fog: water_fog.ok_or(Error("missing water fog"))?,
        water,
        sun_glare,
        sky: sky.ok_or(Error("missing sky reference"))?,
        sea_texture: sea_texture.ok_or(Error("missing sea reference"))?,
        underside: underside.ok_or(Error("missing underside reference"))?,
        sea_values: sea_values.ok_or(Error("missing sea scalars"))?,
        mip_curve,
        slope_thresholds,
        terrain_direction_floor,
        opcodes,
        lights,
        scenarios,
        placements,
        cinematics,
        mission_names,
        command74_present,
        command89,
    })
}
fn name(data: &[u8], at: &mut usize, limit: usize) -> Result<String, Error> {
    let raw = data.get(*at..).ok_or(Error("world name outside block"))?;
    let end = raw
        .iter()
        .take(limit)
        .position(|v| *v == 0)
        .ok_or(Error("unterminated world name"))?;
    if !raw[..end].is_ascii() {
        return Err(Error("non-ASCII world name"));
    }
    *at += end + 1;
    Ok(String::from_utf8(raw[..end].to_vec()).unwrap())
}
fn terrain_material(record: &[u8]) -> Result<TerrainMaterial, Error> {
    slice(record, 0, 45)?;
    let mut at = 0;
    let material_name = name(&record[1..33], &mut at, 32)?;
    let [mut scale] = floats(record, 33)?;
    if scale < 0. {
        return Err(Error("invalid terrain scale"));
    }
    if scale == 0. {
        scale = 50.;
    }
    if !(1. / scale).is_finite() {
        return Err(Error("terrain reciprocal overflows float32"));
    }
    Ok(TerrainMaterial {
        name: material_name,
        scale,
        reciprocal: 1. / scale,
    })
}
pub fn terrain_prefix(data: &[u8]) -> Result<TerrainPrefix, Error> {
    slice(data, 0, 32)?;
    if u32_at(data, 0)? != 0x1A0002E5 {
        return Err(Error("unsupported world version"));
    }
    let start = u32_at(data, 4)? as usize;
    if start < 32 {
        return Err(Error("world block overlaps header"));
    }
    let size = u32_at(data, start)? as usize;
    if size > 16 * 1024 * 1024 {
        return Err(Error("world block exceeds budget"));
    }
    let block = slice(data, start + 4, size)?;
    let mut at = 0;
    let box_name = name(block, &mut at, 128)?;
    let terrain = name(block, &mut at, 128)?;
    let mut leading_records = 0;
    while let Some(&opcode @ (130 | 132)) = block.get(at) {
        leading_records += 1;
        if leading_records > 8192 {
            return Err(Error("world opcode budget exceeded"));
        }
        if opcode == 130 {
            // Native consumes a fixed 32-byte field. Embedded opcode/NUL bytes must
            // not change framing; its downstream consumer is still separate.
            slice(block, at, 33)?;
            at += 33;
        } else {
            at += 1;
            name(block, &mut at, 128)?;
        }
    }
    if slice(block, at, 29)?[0] != 57 {
        return Err(Error("unsupported terrain prefix"));
    }
    at += 29;
    let mut materials = std::array::from_fn(|_| None);
    let mut additional_materials = std::array::from_fn(|_| None);
    // Native dispatch selects a destination by opcode, not declaration order.
    // This prefix stops at the next other command; parse_world walks the rest.
    let mut records = 0;
    while let Some(&opcode @ (52..=54 | 114..=116)) = block.get(at) {
        records += 1;
        if records > 8192 {
            return Err(Error("world opcode budget exceeded"));
        }
        let record = slice(block, at, 45)?;
        let (slots, index) = if opcode <= 54 {
            (&mut materials, opcode - 52)
        } else {
            (&mut additional_materials, opcode - 114)
        };
        slots[index as usize] = Some(terrain_material(record)?);
        at += 45;
    }
    Ok(TerrainPrefix {
        box_name,
        terrain,
        materials,
        additional_materials,
        consumed_offset: start + 4 + at,
        block_end: start + 4 + size,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixed_leading_record_does_not_resynchronize_inside_its_payload() {
        let mut data = world_fixture();
        let mut at = 36;
        name(&data, &mut at, 128).unwrap();
        name(&data, &mut at, 128).unwrap();
        let mut record = [57; 33];
        record[0] = 130;
        record[3] = 0;
        record[8] = 52;
        record[15] = 255;
        data.splice(at..at, record);
        let size = (data.len() - 36) as u32;
        data[32..36].copy_from_slice(&size.to_le_bytes());
        let parsed = parse_world(&data).unwrap();
        assert_eq!(parsed.opcodes[0], (at, 130));
        assert_eq!(parsed.opcodes[1], (at + 33, 132));
        assert_eq!(parsed.opcodes[2], (at + 40, 57));
        assert_eq!(parsed.prefix.materials[0].as_ref().unwrap().name, "ground");
        let mut truncated = data[..at + 32].to_vec();
        truncated[32..36].copy_from_slice(&((at + 32 - 36) as u32).to_le_bytes());
        assert!(terrain_prefix(&truncated).is_err());
    }
    #[test]
    fn cinematic_exclusion_is_kept_on_current_placement() {
        let mut bytes = world_fixture();
        let table_count = parse_world(&bytes).unwrap().cinematics.len();
        bytes.pop();
        bytes.push(42);
        bytes.extend(590u32.to_le_bytes());
        bytes.extend([0; 16]);
        bytes.push(78);
        let name = [b'Y'; 32];
        bytes.extend(name);
        bytes.push(255);
        let size = (bytes.len() - 36) as u32;
        bytes[32..36].copy_from_slice(&size.to_le_bytes());
        let world = parse_world(&bytes).unwrap();
        assert_eq!(
            world.placements.last().unwrap().cinematic_exclusions,
            [name]
        );
        assert_eq!(world.cinematics.len(), table_count);
    }
    #[test]
    fn cinematic_table_and_mission_state_slots_are_separate_and_bounded() {
        let mut bytes = world_fixture();
        bytes.pop();
        let offset = bytes.len();
        bytes.push(79);
        let mut field = [0; 64];
        field[..5].copy_from_slice(b"Alpha");
        field[32..36].copy_from_slice(b"Note");
        bytes.extend(field);
        bytes.push(93);
        let mut field = [0; 32];
        field[..7].copy_from_slice(b"Mission");
        bytes.extend(field);
        bytes.push(255);
        let size = (bytes.len() - 36) as u32;
        bytes[32..36].copy_from_slice(&size.to_le_bytes());
        let world = parse_world(&bytes).unwrap();
        assert_eq!(world.cinematics.len(), 2);
        assert_eq!(world.cinematics[1].source_offset, offset);
        assert_eq!(world.cinematics[1].name, "Alpha");
        assert_eq!(world.cinematics[1].metadata, "Note");
        assert_eq!(world.mission_names, ["Mission"]);
        assert!(world.scenarios.is_empty());
        bytes[offset + 1..offset + 33].fill(b'X');
        assert!(parse_world(&bytes).is_err());
        let mut bytes = world_fixture();
        bytes.pop();
        for _ in 0..33 {
            bytes.push(93);
            bytes.extend(field);
        }
        bytes.push(255);
        let size = (bytes.len() - 36) as u32;
        bytes[32..36].copy_from_slice(&size.to_le_bytes());
        assert!(parse_world(&bytes).is_err());
    }
    #[test]
    fn light_attributes_follow_the_current_object_only() {
        let mut data = world_fixture();
        data.pop();
        let at = data.len();
        data.push(70);
        data.extend(1004u32.to_le_bytes());
        for v in [1f32, 2., 3., 20., -3., 40.] {
            data.extend(v.to_le_bytes());
        }
        data.extend([59, 1, 23]);
        data.extend(2f32.to_le_bytes());
        data.push(42);
        data.extend(999u32.to_le_bytes());
        data.extend([0; 16]);
        data.extend([59, 3, 255]);
        let size = (data.len() - 36) as u32;
        data[32..36].copy_from_slice(&size.to_le_bytes());
        let world = parse_world(&data).unwrap();
        assert_eq!(world.lights.len(), 1);
        assert_eq!(world.lights[0].variant, Some(1));
        assert_eq!(world.lights[0].scale, 2.);
        assert_eq!(world.lights[0].source_offset, at);
        data[at + 5..at + 9].copy_from_slice(&f32::NAN.to_le_bytes());
        assert!(parse_world(&data).is_err());
    }
    fn world_fixture() -> Vec<u8> {
        let mut block = b"box\0island.gti\0\x84voice\0".to_vec();
        block.push(57);
        block.extend([0; 28]);
        for op in [52, 53, 54] {
            block.push(op);
            let mut record = [0; 44];
            record[..6].copy_from_slice(b"ground");
            block.extend(record);
        }
        block.push(46);
        for value in [250f32, 48., 0.02] {
            block.extend(value.to_le_bytes());
        }
        for (op, text) in [(30, "sky"), (37, "sea"), (38, "under")] {
            block.push(op);
            let mut record = [0; 16];
            record[..text.len()].copy_from_slice(text.as_bytes());
            block.extend(record);
        }
        let mut opaque = [0; 64];
        opaque[4] = 41;
        opaque[5..9].copy_from_slice(&666f32.to_le_bytes());
        opaque[9..13].copy_from_slice(&777f32.to_le_bytes());
        opaque[13..16].copy_from_slice(&[1, 2, 3]);
        block.push(79);
        block.extend(opaque);
        block.push(41);
        for value in [99f32, 9062.] {
            block.extend(value.to_le_bytes());
        }
        block.extend([13, 7, 18]);
        block.push(88);
        for value in [256f32, 8609.] {
            block.extend(value.to_le_bytes());
        }
        block.extend([1, 20, 42]);
        block.extend(b"\x76note\0");
        block.extend([0; 20]);
        block.push(255);
        let mut out = vec![0; 32];
        out[..4].copy_from_slice(&0x1A0002E5u32.to_le_bytes());
        out[4..8].copy_from_slice(&32u32.to_le_bytes());
        out.extend((block.len() as u32).to_le_bytes());
        out.extend(block);
        out
    }
    #[test]
    fn world_placements_preserve_omitted_template_fields() {
        let mut data = world_fixture();
        data.pop();
        for opcode in [42, 70] {
            data.push(opcode);
            data.extend(7u32.to_le_bytes());
            for value in [1f32, 2., 3., 90.] {
                data.extend(value.to_le_bytes());
            }
            if opcode == 70 {
                for value in [4f32, 5.] {
                    data.extend(value.to_le_bytes());
                }
                data.push(23);
                data.extend(2f32.to_le_bytes());
            }
        }
        data.push(255);
        let size = (data.len() - 36) as u32;
        data[32..36].copy_from_slice(&size.to_le_bytes());
        let config = parse_world(&data).unwrap();
        assert_eq!(config.placements.len(), 2);
        assert_eq!(config.placements[0].pitch_roll, None);
        assert_eq!(config.placements[0].scale, None);
        assert_eq!(config.placements[1].pitch_roll, Some([4., 5.]));
        assert_eq!(config.placements[1].scale, Some(2.));
        assert_eq!(config.placements[0].position, [1., 2., 3.]);
        assert_eq!(config.placements[1].yaw, 90.);
    }
    #[test]
    fn sun_glare_uses_bounded_record_and_later_override() {
        let mut data = world_fixture();
        data.pop();
        data.extend(b"\x76synthetic\0");
        let at = data.len();
        for value in [1f32, 0.7, 0.1, 6., 0.2] {
            data.extend(value.to_le_bytes());
        }
        data.push(255);
        let size = (data.len() - 36) as u32;
        data[32..36].copy_from_slice(&size.to_le_bytes());
        let glare = parse_world(&data).unwrap().sun_glare.unwrap();
        assert_eq!(glare.effect, "synthetic");
        assert_eq!(glare.color, [1., 0.7, 0.1]);
        assert_eq!(glare.exponent, 6.);
        assert_eq!(glare.scale, 0.2);
        for end in at..data.len() {
            assert!(parse_world(&data[..end]).is_err());
        }
        for offset in (0..20).step_by(4) {
            let mut invalid = data.clone();
            invalid[at + offset..at + offset + 4].copy_from_slice(&f32::NAN.to_le_bytes());
            assert!(parse_world(&invalid).is_err());
        }
    }
    #[test]
    fn scenario_record_is_bounded_and_ordered() {
        let mut data = world_fixture();
        data.pop();
        data.extend([92, 2]);
        data.extend(7u32.to_le_bytes());
        let mut label = [0; 32];
        label[..5].copy_from_slice(b"alpha");
        data.extend(label);
        data.push(255);
        let size = (data.len() - 36) as u32;
        data[32..36].copy_from_slice(&size.to_le_bytes());
        let parsed = parse_world(&data).unwrap();
        assert_eq!(parsed.scenarios.len(), 1);
        assert_eq!(parsed.scenarios[0].kind, 2);
        assert_eq!(parsed.scenarios[0].parameter, 7);
        assert_eq!(parsed.scenarios[0].name, "alpha");
        let start = data.len() - 33;
        data[start..start + 32].fill(b'x');
        assert!(parse_world(&data).is_err());
    }
    #[test]
    fn walker_skips_opaque_fake_fog_and_requires_complete_block() {
        let mut data = world_fixture();
        let world = parse_world(&data).unwrap();
        assert_eq!(world.fog.start, 99.);
        assert_eq!(world.fog.end, 9062.);
        assert_eq!(world.fog.color, [13, 7, 18]);
        assert_eq!(world.water_fog.start, 256.);
        assert_eq!(world.water_fog.color, [1, 20, 42]);
        assert_eq!(world.opcodes.iter().filter(|(_, op)| *op == 41).count(), 1);
        for end in 0..data.len() {
            assert!(parse_world(&data[..end]).is_err());
        }
        *data.last_mut().unwrap() = 23;
        assert!(parse_world(&data).is_err());
    }
    #[test]
    fn water_record_truncates_rgb_and_rejects_nonfinite_or_truncated_input() {
        let mut data = world_fixture();
        assert_eq!(parse_world(&data).unwrap().water.colors[0], [0, 0, 128]);
        data.pop();
        let at = data.len();
        data.push(121);
        for value in [0.5f32; 18].into_iter().chain([4.8, 4000., 2., 0.904, 1.5]) {
            data.extend(value.to_le_bytes());
        }
        data.push(255);
        let size = (data.len() - 36) as u32;
        data[32..36].copy_from_slice(&size.to_le_bytes());
        let parsed = parse_world(&data).unwrap();
        assert_eq!(parsed.water.colors, [[127; 3]; 6]);
        assert_eq!(parsed.water.values, [4.8, 4000., 2., 0.904, 1.5]);
        for end in at..data.len() {
            assert!(parse_world(&data[..end]).is_err());
        }
        for (offset, value) in [(1, -0.1), (1, 1.1), (1, f32::NAN), (89, f32::INFINITY)] {
            let mut invalid = data.clone();
            invalid[at + offset..at + offset + 4].copy_from_slice(&value.to_le_bytes());
            assert!(parse_world(&invalid).is_err());
        }
    }
    #[test]
    fn later_overrides_and_invalid_curve_are_explicit() {
        let mut data = world_fixture();
        data.pop();
        data.push(101);
        for value in [0.5f32, 0., 0.] {
            data.extend(value.to_le_bytes());
        }
        data.push(102);
        for value in [45f32, 70.] {
            data.extend(value.to_le_bytes());
        }
        data.push(54);
        let mut record = [0; 44];
        record[..5].copy_from_slice(b"other");
        record[32..36].copy_from_slice(&40f32.to_le_bytes());
        data.extend(record);
        data.push(255);
        let size = (data.len() - 36) as u32;
        data[32..36].copy_from_slice(&size.to_le_bytes());
        let world = parse_world(&data).unwrap();
        assert_eq!(world.mip_curve, [0.5, 0., 0.]);
        assert_eq!(world.prefix.materials[2].as_ref().unwrap().name, "other");
        assert!((world.slope_thresholds[0] - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-6);
        let at = world.opcodes.iter().find(|(_, op)| *op == 101).unwrap().0;
        data[at + 1..at + 5].copy_from_slice(&f32::NAN.to_le_bytes());
        assert!(parse_world(&data).is_err());
    }
    #[test]
    fn light_color_attaches_only_to_current_light_and_rejects_nan() {
        let mut data = world_fixture();
        data.pop();
        data.push(70);
        data.extend(1004u32.to_le_bytes());
        data.extend([0; 24]);
        let color_at = data.len();
        data.push(95);
        for v in [0.1f32, 0.2, 0.3] {
            data.extend(v.to_le_bytes());
        }
        data.push(42);
        data.extend(123u32.to_le_bytes());
        data.extend([0; 16]);
        data.push(95);
        for v in [0.7f32, 0.8, 0.9] {
            data.extend(v.to_le_bytes());
        }
        data.push(255);
        let size = (data.len() - 36) as u32;
        data[32..36].copy_from_slice(&size.to_le_bytes());
        let config = parse_world(&data).unwrap();
        assert_eq!(config.lights.len(), 1);
        assert_eq!(config.lights[0].color, Some([0.1, 0.2, 0.3]));
        data[color_at + 1..color_at + 5].copy_from_slice(&f32::NAN.to_le_bytes());
        assert!(parse_world(&data).is_err());
    }
    #[test]
    fn direct_selector_and_word_attach_to_their_world_declaration() {
        let mut data = world_fixture();
        data.pop();
        for (class, value) in [(679u32, 22u8), (7, 1)] {
            data.push(42);
            data.extend(class.to_le_bytes());
            data.extend([0; 16]);
            data.extend([59, value, 82]);
            data.extend(u32::from(value).to_le_bytes());
        }
        data.push(255);
        let size = (data.len() - 36) as u32;
        data[32..36].copy_from_slice(&size.to_le_bytes());
        let world = parse_world(&data).unwrap();
        let p = &world.placements[world.placements.len() - 2..];
        assert_eq!(
            (p[0].object_type, p[0].selector, p[0].word_174),
            (679, Some(22), Some(22))
        );
        assert_eq!(
            (p[1].object_type, p[1].selector, p[1].word_174),
            (7, Some(1), Some(1))
        );
    }
    #[test]
    fn optional_world_commands_share_bounded_framing_without_payload_resync() {
        let mut data = world_fixture();
        data.pop();
        data.extend([74, 89]);
        data.extend(b"weather\0");
        let at = data.len();
        data.push(103);
        data.extend(1u32.to_le_bytes());
        data.extend([255, 74, 89, 103]);
        data.push(255);
        let size = (data.len() - 36) as u32;
        data[32..36].copy_from_slice(&size.to_le_bytes());
        let world = parse_world(&data).unwrap();
        assert!(world.command74_present);
        assert_eq!(world.command89.as_deref(), Some("weather"));
        assert_eq!(world.opcodes[world.opcodes.len() - 2], (at, 103));
        data[at + 1..at + 5].copy_from_slice(&21u32.to_le_bytes());
        assert!(parse_world(&data).is_err());
    }
    #[test]
    fn sparse_material_slots_and_additional_overrides_keep_source_identity() {
        let mut data = world_fixture();
        let at = terrain_prefix(&data).unwrap().consumed_offset - 135;
        data.drain(at + 45..at + 135);
        let mut extra = vec![115];
        let mut payload = [0; 44];
        payload[..6].copy_from_slice(b"detail");
        payload[32..36].copy_from_slice(&80f32.to_le_bytes());
        extra.extend(payload);
        data.splice(at + 45..at + 45, extra);
        data.pop();
        data.push(114);
        payload[..6].copy_from_slice(b"later\0");
        payload[32..36].copy_from_slice(&0f32.to_le_bytes());
        data.extend(payload);
        data.push(255);
        let size = (data.len() - 36) as u32;
        data[32..36].copy_from_slice(&size.to_le_bytes());
        let prefix = terrain_prefix(&data).unwrap();
        assert!(prefix.materials[0].is_some());
        assert!(prefix.materials[1..].iter().all(Option::is_none));
        assert!(prefix.additional_materials[0].is_none());
        assert_eq!(
            prefix.additional_materials[1].as_ref().unwrap().name,
            "detail"
        );
        let world = parse_world(&data).unwrap();
        assert!(world.prefix.materials[1..].iter().all(Option::is_none));
        let value = world.prefix.additional_materials[0].as_ref().unwrap();
        assert_eq!(
            (value.name.as_str(), value.scale, value.reciprocal),
            ("later", 50., 0.02)
        );
        data[at + 45] = 113;
        assert!(parse_world(&data).is_err());
    }
    #[test]
    fn prefix_is_bounded_and_applies_zero_scale_default() {
        let mut block = b"box\0island.gti\0".to_vec();
        block.push(57);
        block.extend([0; 28]);
        for opcode in [52, 53, 54] {
            block.push(opcode);
            let mut record = [0; 44];
            record[..6].copy_from_slice(b"ground");
            block.extend(record);
        }
        let mut data = vec![0; 32];
        data[..4].copy_from_slice(&0x1A0002E5u32.to_le_bytes());
        data[4..8].copy_from_slice(&32u32.to_le_bytes());
        data.extend((block.len() as u32).to_le_bytes());
        data.extend(block);
        let result = terrain_prefix(&data).unwrap();
        assert_eq!(result.terrain, "island.gti");
        assert_eq!(result.materials[2].as_ref().unwrap().scale, 50.);
        assert_eq!(result.consumed_offset, data.len());
        for end in 0..data.len() {
            assert!(terrain_prefix(&data[..end]).is_err());
        }
        data[36] = 255;
        assert!(terrain_prefix(&data).is_err());
    }
}
