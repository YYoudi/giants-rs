//! Cinematic declarations and bounded object-catalog references. No VM execution.
use crate::{Error, slice, u32_at};
#[derive(Debug)]
pub struct Actor {
    pub slot: usize,
    pub director: i32,
    pub name: Option<String>,
    pub object_type: i32,
}
fn name(data: &[u8], at: usize, end: usize) -> Result<String, Error> {
    let raw = slice(
        data,
        at,
        end.checked_sub(at)
            .ok_or(Error("name outside pool"))?
            .min(128),
    )?;
    let n = raw
        .iter()
        .position(|v| *v == 0)
        .ok_or(Error("unterminated cast name"))?;
    if n == 0 || !raw[..n].is_ascii() {
        return Err(Error("invalid cast name"));
    }
    Ok(String::from_utf8(raw[..n].to_vec()).unwrap())
}
pub fn first_flick(data: &[u8]) -> Result<Vec<Actor>, Error> {
    if data.len() < 40 || u32_at(data, 0)? != 3 || u32_at(data, 4)? as usize != data.len() {
        return Err(Error("unsupported cast FLK framing"));
    }
    let end = u32_at(data, 36)? as usize;
    let count = u32_at(data, 12)? as usize;
    if !(1..=32).contains(&count) || u32_at(data, 8)? != 1 || end > data.len() {
        return Err(Error("unsupported first cast profile"));
    }
    let mut at = 24;
    let mut declarations = Vec::new();
    let mut terminated = false;
    for _ in 0..4096 {
        if at + 8 > end {
            break;
        }
        let op = u32_at(data, at)?;
        let size = u32_at(data, at + 4)? as usize;
        if !(1..=53).contains(&op) || !(2..=4096).contains(&size) || at + size * 4 > end {
            return Err(Error("invalid FLK declaration record"));
        }
        if op == 2 {
            if size != 6 {
                return Err(Error("invalid actor declaration size"));
            }
            declarations.push((
                u32_at(data, at + 8)? as i32,
                u32_at(data, at + 12)? as usize,
                u32_at(data, at + 16)? as usize,
                u32_at(data, at + 20)? as i32,
            ));
        }
        at += size * 4;
        if op == 5 {
            if size != 2 {
                return Err(Error("invalid FLK end record"));
            }
            terminated = true;
            break;
        }
    }
    if !terminated || declarations.len() != count {
        return Err(Error("incomplete first cast declarations"));
    }
    let mut actors = Vec::new();
    let mut slots = std::collections::BTreeSet::new();
    for (director, p, slot, object_type) in declarations {
        if slot >= count || !slots.insert(slot) || ![-1, 0].contains(&director) {
            return Err(Error("invalid cast slot/director"));
        }
        let label = if p == 0 {
            None
        } else {
            if p < at || p >= end {
                return Err(Error("actor name outside FLK string pool"));
            }
            Some(name(data, p, end)?)
        };
        actors.push(Actor {
            slot,
            director,
            name: label,
            object_type,
        });
    }
    actors.sort_by_key(|a| a.slot);
    Ok(actors)
}
type CatalogTables<'a> = (&'a [u8], Vec<usize>, Vec<usize>, Vec<usize>);
/// Ordered recursive dependencies from header word seven. Repeated names are
/// meaningful occurrences; this reader does not choose class override winners.
pub fn catalog_includes(data: &[u8]) -> Result<Vec<String>, Error> {
    slice(data, 0, 32)?;
    if u32_at(data, 0)? != 0x1a0002e5 {
        return Err(Error("unsupported object catalog header"));
    }
    let at = u32_at(data, 28)? as usize;
    if at == 0 {
        return Ok(Vec::new());
    }
    let count = u32_at(data, at)? as usize;
    if count > 4096 {
        return Err(Error("catalog include budget"));
    }
    let records = slice(data, at + 4, count * 32)?;
    records
        .as_chunks::<32>()
        .0
        .iter()
        .map(|record| {
            let end = record
                .iter()
                .position(|v| *v == 0)
                .ok_or(Error("unterminated catalog include"))?;
            let value = &record[..end];
            if value.is_empty()
                || !value
                    .iter()
                    .all(|v| v.is_ascii_alphanumeric() || *v == b'_')
            {
                return Err(Error("invalid catalog include basename"));
            }
            Ok(String::from_utf8(value.to_vec()).unwrap())
        })
        .collect()
}
fn catalog_tables(data: &[u8]) -> Result<CatalogTables<'_>, Error> {
    if u32_at(data, 0)? != 0x1a0002e5 {
        return Err(Error("unsupported object catalog header"));
    }
    let section = u32_at(data, 16)? as usize;
    let size = u32_at(data, section)? as usize;
    let data = slice(data, section + 4, size)?;
    let mut at = 0;
    fn table(data: &[u8], at: &mut usize) -> Result<Vec<usize>, Error> {
        let count = u32_at(data, *at)? as usize;
        *at += 4;
        if count > 4096 {
            return Err(Error("object catalog table budget"));
        }
        slice(data, *at, count * 4)?;
        let values = (0..count)
            .map(|i| u32_at(data, *at + i * 4).map(|v| v as usize))
            .collect::<Result<Vec<_>, _>>()?;
        *at += count * 4;
        Ok(values)
    }
    let names = table(data, &mut at)?;
    table(data, &mut at)?;
    let specs = table(data, &mut at)?;
    let instances = table(data, &mut at)?;
    Ok((data, names, specs, instances))
}
/// Declared instance IDs in source table order. Duplicates are retained so a
/// caller cannot accidentally turn an ambiguous catalog into a unique match.
pub fn catalog_class_ids(data: &[u8]) -> Result<Vec<i32>, Error> {
    let (data, _, _, instances) = catalog_tables(data)?;
    instances
        .into_iter()
        .map(|at| u32_at(data, at).map(|v| v as i32))
        .collect()
}
pub fn first_model(data: &[u8], object_type: i32) -> Result<String, Error> {
    let (data, names, specs, instances) = catalog_tables(data)?;
    let mut selected = None;
    for p in instances {
        if u32_at(data, p)? as i32 == object_type {
            if selected.is_some() {
                return Err(Error("ambiguous catalog object type"));
            }
            if slice(data, p + 4, 1)? != [1] {
                return Err(Error("unsupported object catalog reference prefix"));
            }
            selected = Some(u32_at(data, p + 5)?);
        }
    }
    let reference = selected.ok_or(Error("object type missing from catalog"))?;
    let mut model_ref = None;
    for p in specs {
        if u32_at(data, p)? == reference {
            if model_ref.is_some() || slice(data, p + 4, 1)? != [0x51] {
                return Err(Error("unsupported/ambiguous body spec prefix"));
            }
            model_ref = Some(u32_at(data, p + 5)?);
        }
    }
    let reference = model_ref.ok_or(Error("body spec missing from catalog"))?;
    let mut result = None;
    for p in names {
        if u32_at(data, p)? == reference {
            if result.is_some() {
                return Err(Error("ambiguous catalog model name"));
            }
            result = Some(name(data, p + 4, data.len())?);
        }
    }
    let result = result.ok_or(Error("body name missing from catalog"))?;
    if result.contains(['/', '\\', ':', '.']) {
        return Err(Error("catalog model is not a basename"));
    }
    Ok(result)
}

/// Consecutive model-reference records at the start of a measured class spec.
/// Record payload words are opaque; their order does not establish LOD selection.
#[derive(Debug, Clone)]
pub struct ModelReference {
    pub reference: u32,
    pub name: Option<String>,
    pub payload: [u32; 2],
}
#[derive(Debug)]
pub struct ModelPrefixRecord {
    pub reference: u32,
    pub payload: [u32; 2],
}
#[derive(Debug)]
pub struct CatalogReferences {
    pub names: Vec<(u32, Result<String, Error>)>,
    pub specs: Vec<(u32, Result<Vec<ModelPrefixRecord>, Error>)>,
    pub classes: Vec<(i32, Result<u32, Error>)>,
}
pub type CatalogClassProgram = (i32, Result<crate::class_program::Program, Error>);
pub fn catalog_class_programs(data: &[u8]) -> Result<Vec<CatalogClassProgram>, Error> {
    let (data, _, _, instances) = catalog_tables(data)?;
    instances
        .into_iter()
        .map(|p| {
            let id = u32_at(data, p)? as i32;
            let program = data
                .get(p + 4..)
                .ok_or(Error("class body outside section"))
                .and_then(crate::class_program::parse);
            Ok((id, program))
        })
        .collect()
}
pub type CatalogSpecProgram = (u32, Result<crate::object_spec::Program, Error>);
/// Native spec streams end at FF, not at the next table offset. Compiled tables
/// may share that byte with the first byte of an adjacent record's identifier.
pub fn catalog_spec_programs(data: &[u8]) -> Result<Vec<CatalogSpecProgram>, Error> {
    let (data, _, specs, _) = catalog_tables(data)?;
    specs
        .iter()
        .map(|&p| {
            let id = u32_at(data, p)?;
            let body = p.checked_add(4).ok_or(Error("spec offset overflow"))?;
            let available = data.get(body..).ok_or(Error("spec framing"))?;
            let program =
                crate::object_spec::parse(&available[..available.len().min(4 * 1024 * 1024)]);
            Ok((id, program))
        })
        .collect()
}
/// Raw references in source table order, before cross-catalog lookup. Only the
/// consecutive 0x51 model prefix and initial class 0x01 reference are decoded.
pub fn catalog_references(data: &[u8]) -> Result<CatalogReferences, Error> {
    let (data, names, specs, instances) = catalog_tables(data)?;
    let end = |start| {
        names
            .iter()
            .chain(&specs)
            .chain(&instances)
            .copied()
            .filter(|p| *p > start)
            .min()
            .unwrap_or(data.len())
    };
    let mut result = CatalogReferences {
        names: Vec::new(),
        specs: Vec::new(),
        classes: Vec::new(),
    };
    for &p in &names {
        let id = u32_at(data, p)?;
        let value = name(data, p + 4, end(p)).and_then(|value| {
            if value.contains(['/', '\\', ':', '.']) {
                Err(Error("model reference is not a basename"))
            } else {
                Ok(value)
            }
        });
        result.names.push((id, value));
    }
    for &p in &specs {
        let id = u32_at(data, p)?;
        let prefix = || -> Result<Vec<ModelPrefixRecord>, Error> {
            let mut at = p + 4;
            let limit = end(p);
            let mut values = Vec::new();
            while at < limit && slice(data, at, 1)? == [0x51] {
                if values.len() >= 16 || at + 13 > limit {
                    return Err(Error("model prefix budget/truncation"));
                }
                values.push(ModelPrefixRecord {
                    reference: u32_at(data, at + 1)?,
                    payload: [u32_at(data, at + 5)?, u32_at(data, at + 9)?],
                });
                at += 13;
            }
            if values.is_empty() {
                return Err(Error("unsupported model spec prefix"));
            }
            Ok(values)
        };
        result.specs.push((id, prefix()));
    }
    for &p in &instances {
        let id = u32_at(data, p)? as i32;
        let reference = if end(p) < p + 9 || slice(data, p + 4, 1)? != [1] {
            Err(Error("unsupported object catalog reference prefix"))
        } else {
            u32_at(data, p + 5)
        };
        result.classes.push((id, reference));
    }
    Ok(result)
}
pub fn model_prefix(data: &[u8], object_type: i32) -> Result<Vec<ModelReference>, Error> {
    // Retain the existing strict instance/spec/name checks for the first model.
    first_model(data, object_type)?;
    let (data, names, specs, instances) = catalog_tables(data)?;
    let selected = instance(data, instances.clone(), object_type)?;
    let reference = u32_at(data, selected + 5)?;
    let matching: Vec<_> = specs
        .iter()
        .copied()
        .filter(|&p| u32_at(data, p).ok() == Some(reference))
        .collect();
    if matching.len() != 1 {
        return Err(Error("ambiguous model spec"));
    }
    let start = matching[0];
    let end = names
        .iter()
        .chain(&specs)
        .chain(&instances)
        .copied()
        .filter(|&p| p > start)
        .min()
        .unwrap_or(data.len());
    let mut at = start + 4;
    let mut result = Vec::new();
    while at < end && slice(data, at, 1)? == [0x51] {
        if result.len() >= 16 || at + 13 > end {
            return Err(Error("model prefix budget/truncation"));
        }
        let id = u32_at(data, at + 1)?;
        let matching: Vec<_> = names
            .iter()
            .copied()
            .filter(|&p| u32_at(data, p).ok() == Some(id))
            .collect();
        if id != 0 && matching.len() != 1 {
            return Err(Error("missing/ambiguous model reference name"));
        }
        let name = if id == 0 {
            None
        } else {
            Some(name(data, matching[0] + 4, data.len())?)
        };
        if name
            .as_ref()
            .is_some_and(|v| v.contains(['/', '\\', ':', '.']))
        {
            return Err(Error("model reference is not a basename"));
        }
        result.push(ModelReference {
            reference: id,
            name,
            payload: [u32_at(data, at + 5)?, u32_at(data, at + 9)?],
        });
        at += 13;
    }
    Ok(result)
}

#[derive(Debug)]
pub struct Attachment {
    pub parent_slot: usize,
    pub parent_node: String,
    pub slot: usize,
    pub child_node: String,
}
/// Raw node binding, including a legitimate null child handle. Framing and
/// pointer bounds are checked; this does not execute a binding or resolve ANM.
#[derive(Debug, Clone)]
pub struct AttachmentDeclaration {
    pub offset: usize,
    pub parent_slot: usize,
    pub parent_node: String,
    pub slot: usize,
    pub child_node: Option<String>,
}
pub fn attachment_declarations(data: &[u8]) -> Result<Vec<AttachmentDeclaration>, Error> {
    let actors = first_flick(data)?;
    let end = u32_at(data, 36)? as usize;
    let mut at = 24;
    let mut raw = Vec::new();
    while u32_at(data, at)? != 5 {
        let size = u32_at(data, at + 4)? as usize;
        if u32_at(data, at)? == 3 {
            if size != 6 {
                return Err(Error("invalid FLK attachment size"));
            }
            raw.push((
                at,
                [
                    u32_at(data, at + 8)?,
                    u32_at(data, at + 12)?,
                    u32_at(data, at + 16)?,
                    u32_at(data, at + 20)?,
                ],
            ));
        }
        at += size * 4;
    }
    let pool = at + 8;
    let mut result = Vec::new();
    for (offset, [parent, parent_name, slot, child_name]) in raw {
        let parent_slot = parent as usize;
        let slot = slot as usize;
        if parent_slot >= actors.len()
            || slot >= actors.len()
            || (parent_name as usize) < pool
            || (child_name != 0 && (child_name as usize) < pool)
        {
            return Err(Error(
                "attachment declaration slot or string outside bounds",
            ));
        }
        result.push(AttachmentDeclaration {
            offset,
            parent_slot,
            parent_node: name(data, parent_name as usize, end)?,
            slot,
            child_node: if child_name == 0 {
                None
            } else {
                Some(name(data, child_name as usize, end)?)
            },
        });
    }
    Ok(result)
}
pub fn first_attachments(data: &[u8]) -> Result<Vec<Attachment>, Error> {
    attachments_profile(data, false, None)
}
pub fn preview_attachments(data: &[u8]) -> Result<Vec<Attachment>, Error> {
    attachments_profile(data, true, None)
}
/// Consumer-specific legacy bindings. Other classes retain their common
/// dependency requests; they are not forced through named-parent skin rules.
pub fn preview_attachments_for_classes(
    data: &[u8],
    classes: &[i32],
) -> Result<Vec<Attachment>, Error> {
    attachments_profile(data, true, Some(classes))
}
fn attachments_profile(
    data: &[u8],
    preview: bool,
    classes: Option<&[i32]>,
) -> Result<Vec<Attachment>, Error> {
    let actors = first_flick(data)?;
    let end = u32_at(data, 36)? as usize;
    let mut at = 24;
    let mut raw = Vec::new();
    while u32_at(data, at)? != 5 {
        let size = u32_at(data, at + 4)? as usize;
        if u32_at(data, at)? == 3 {
            if size != 6 {
                return Err(Error("invalid FLK attachment size"));
            }
            raw.push([
                u32_at(data, at + 8)?,
                u32_at(data, at + 12)?,
                u32_at(data, at + 16)?,
                u32_at(data, at + 20)?,
            ]);
        }
        at += size * 4;
    }
    let pool = at + 8;
    let mut result = Vec::new();
    let mut children = std::collections::BTreeSet::new();
    for [parent, parent_name, slot, child_name] in raw {
        if preview && child_name == 0 {
            eprintln!("cinematic_preview_omission attachment_slot={slot} null_handle=true");
            continue;
        }
        let parent = parent as usize;
        let slot = slot as usize;
        if slot < actors.len()
            && classes.is_some_and(|classes| !classes.contains(&actors[slot].object_type))
        {
            if parent >= actors.len()
                || parent == slot
                || (parent_name as usize) < pool
                || (child_name as usize) < pool
            {
                return Err(Error("attachment outside declared slots or string pool"));
            }
            name(data, parent_name as usize, end)?;
            name(data, child_name as usize, end)?;
            eprintln!(
                "cinematic_preview_omission attachment_slot={slot} class={} legacy_attachment_consumer=false",
                actors[slot].object_type
            );
            continue;
        }
        if parent >= actors.len()
            || slot >= actors.len()
            || parent == slot
            || actors[parent].name.is_none()
            || actors[slot].name.is_some()
            || !children.insert(slot)
            || (parent_name as usize) < pool
            || (child_name as usize) < pool
        {
            return Err(Error("attachment outside first cast profile"));
        }
        result.push(Attachment {
            parent_slot: parent,
            slot,
            parent_node: name(data, parent_name as usize, end)?,
            child_node: name(data, child_name as usize, end)?,
        });
    }
    Ok(result)
}
fn instance(data: &[u8], instances: Vec<usize>, object_type: i32) -> Result<usize, Error> {
    let matches = instances
        .into_iter()
        .filter(|p| u32_at(data, *p).ok() == Some(object_type as u32))
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err(Error("missing/ambiguous catalog class"));
    }
    let p = matches[0];
    if slice(data, p + 4, 1)? != [1] {
        return Err(Error("unsupported catalog class prefix"));
    }
    Ok(p)
}
pub fn class_scale(data: &[u8], object_type: i32) -> Result<f32, Error> {
    let (data, names, specs, instances) = catalog_tables(data)?;
    let p = instance(data, instances.clone(), object_type)?;
    let end = names
        .iter()
        .chain(&specs)
        .chain(&instances)
        .copied()
        .filter(|v| *v > p)
        .min()
        .unwrap_or(data.len());
    let mut at = p + 9;
    let mut scale = None;
    for _ in 0..128 {
        if at >= end {
            break;
        }
        let op = slice(data, at, 1)?[0];
        if op == 0xff {
            scale = Some(1.);
            break;
        }
        if op == 0x17 {
            if at + 5 > end {
                return Err(Error("truncated class scale"));
            }
            scale = Some(f32::from_bits(u32_at(data, at + 1)?));
            break;
        }
        if op == 0x7b || op == 2 {
            let start = at + if op == 2 { 5 } else { 1 };
            if start > end {
                return Err(Error("truncated class string prefix"));
            }
            let payload = slice(data, start, end - start)?;
            let count = payload
                .iter()
                .position(|b| *b == 0)
                .ok_or(Error("unterminated class string attribute"))?;
            if count > 127 || !payload[..count].is_ascii() {
                return Err(Error("class string attribute outside measured profile"));
            }
            at = start + count + 1;
            continue;
        }
        let size = match op {
            5..=7 | 9..=11 | 0xd | 0x19 | 0x25 | 0x29 | 0x31 | 0x32 | 0x39 | 0x7a => 5,
            4 | 8 | 0x1c | 0x21 | 0x26 => 9,
            0x13 | 0x1e | 0x36 => 2,
            0x14 | 0x1b | 0x1d | 0x22 | 0x2a | 0x2f | 0x49 | 0x4d | 0x7e => 1,
            0x38 => 13,
            _ => return Err(Error("class scale outside measured prefix")),
        };
        if at + size > end {
            return Err(Error("truncated class attribute prefix"));
        }
        at += size;
    }
    let scale = scale.ok_or(Error("unterminated class scale prefix"))?;
    if !scale.is_finite() || scale <= 0. {
        return Err(Error("invalid class scale"));
    }
    Ok(scale)
}
pub fn attachment_resource(data: &[u8], object_type: i32) -> Result<(String, String, f32), Error> {
    let model = first_model(data, object_type)?;
    let scale = class_scale(data, object_type)?;
    let (data, names, specs, instances) = catalog_tables(data)?;
    let p = instance(data, instances.clone(), object_type)?;
    let reference = u32_at(data, p + 5)?;
    let selected = specs
        .iter()
        .copied()
        .filter(|p| u32_at(data, *p).ok() == Some(reference))
        .collect::<Vec<_>>();
    if selected.len() != 1 {
        return Err(Error("missing/ambiguous attachment spec"));
    }
    let p = selected[0];
    let end = names
        .iter()
        .chain(&specs)
        .chain(&instances)
        .copied()
        .filter(|v| *v > p)
        .min()
        .unwrap_or(data.len());
    let mut at = p + 4;
    for _ in 0..10 {
        if slice(data, at, 1)?[0] != 0x51 {
            break;
        }
        if at + 13 > end {
            return Err(Error("truncated attachment model reference"));
        }
        at += 13;
    }
    if at + 2 > end || slice(data, at, 2)? != [0x52, 1] {
        return Err(Error("attachment animation outside measured prefix"));
    }
    let animation = name(data, at + 2, end)?;
    if !animation.to_ascii_lowercase().ends_with(".anm") || animation.contains(['/', '\\', ':']) {
        return Err(Error("attachment animation is not a basename"));
    }
    Ok((model, animation, scale))
}

/// Selector read by the catalog loader before recursive includes. Other signed
/// values retain the currently loaded effect image.
pub fn effect_selector(data: &[u8]) -> Result<Option<u8>, Error> {
    if u32_at(data, 0)? != 0x1a0002e5 {
        return Err(Error("unsupported catalog header"));
    }
    let at = u32_at(data, 20)? as usize;
    let mode = u32_at(data, at)? as i32;
    Ok(match mode {
        0..=2 => Some(mode as u8),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn model_prefix_preserves_variants_and_unnamed_references() {
        fn add(section: &mut Vec<u8>, data: &[u8]) -> usize {
            let at = section.len();
            section.extend(data);
            at
        }
        let mut section = vec![0; 32];
        let a = add(
            &mut section,
            &[10u32.to_le_bytes().as_slice(), b"part\0"].concat(),
        );
        let b = add(
            &mut section,
            &[11u32.to_le_bytes().as_slice(), b"part_l1\0"].concat(),
        );
        let mut spec = 7u32.to_le_bytes().to_vec();
        for reference in [10u32, 11] {
            spec.push(0x51);
            spec.extend(reference.to_le_bytes());
            spec.extend([0; 8]);
        }
        spec.push(0xff);
        let spec_at = add(&mut section, &spec);
        let instance = add(
            &mut section,
            &[
                99u32.to_le_bytes().as_slice(),
                &[1],
                7u32.to_le_bytes().as_slice(),
            ]
            .concat(),
        );
        for (at, value) in [
            (0, 2),
            (4, a),
            (8, b),
            (12, 0),
            (16, 1),
            (20, spec_at),
            (24, 1),
            (28, instance),
        ] {
            word(&mut section, at, value as u32);
        }
        let mut data = vec![0; 32];
        word(&mut data, 0, 0x1a0002e5);
        word(&mut data, 16, 32);
        data.extend((section.len() as u32).to_le_bytes());
        data.extend(section);
        let references = model_prefix(&data, 99).unwrap();
        assert_eq!(references.len(), 2);
        assert_eq!(references[1].name.as_deref(), Some("part_l1"));
        for end in 32..data.len() {
            assert!(model_prefix(&data[..end], 99).is_err());
        }
        word(&mut data, 36 + spec_at + 18, 0);
        assert!(model_prefix(&data, 99).unwrap()[1].name.is_none());
        word(&mut data, 36 + 8, a as u32);
        assert!(model_prefix(&data, 99).is_err());
    }
    #[test]
    fn extended_class_prefix_is_bounded_before_scale() {
        let mut prefix = vec![0x1b, 0x1d, 0x2a, 0x2f, 0x49, 0x4d, 0x7e];
        for (opcode, payload) in [
            (0x1c, 8),
            (0x21, 8),
            (0x25, 4),
            (0x26, 8),
            (0x31, 4),
            (0x32, 4),
            (0x38, 12),
            (0x39, 4),
            (0x7a, 4),
        ] {
            prefix.push(opcode);
            prefix.extend(vec![0; payload]);
        }
        prefix.extend(b"\x7bsynthetic\0");
        prefix.extend(b"\x02\0\0\0\0synthetic-shadow\0");
        prefix.push(0x17);
        prefix.extend(1.25f32.to_le_bytes());
        let make = |prefix: &[u8]| {
            let mut section = Vec::new();
            for word in [0u32, 0, 0, 1, 20, 123] {
                section.extend(word.to_le_bytes());
            }
            section.push(1);
            section.extend(7u32.to_le_bytes());
            section.extend(prefix);
            let mut data = vec![0; 32];
            data[..4].copy_from_slice(&0x1a0002e5u32.to_le_bytes());
            data[16..20].copy_from_slice(&32u32.to_le_bytes());
            data.extend((section.len() as u32).to_le_bytes());
            data.extend(section);
            data
        };
        assert_eq!(class_scale(&make(&prefix), 123).unwrap(), 1.25);
        for end in 0..prefix.len() {
            assert!(class_scale(&make(&prefix[..end]), 123).is_err());
        }
        prefix[0] = 0xfe;
        assert!(class_scale(&make(&prefix), 123).is_err());
        assert_eq!(class_scale(&make(&[0x1d, 0xff]), 123).unwrap(), 1.);
    }
    fn word(data: &mut [u8], at: usize, value: u32) {
        data[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }
    #[test]
    fn declarations_are_bounded_by_program_end_and_string_pool() {
        let mut data = vec![0; 128];
        for (at, v) in [
            (0, 3),
            (4, 128),
            (8, 1),
            (12, 1),
            (24, 1),
            (28, 4),
            (36, 80),
            (40, 2),
            (44, 6),
            (48, 0),
            (52, 72),
            (56, 0),
            (60, 123),
            (64, 5),
            (68, 2),
        ] {
            word(&mut data, at, v);
        }
        data[72..78].copy_from_slice(b"Actor\0");
        let actors = first_flick(&data).unwrap();
        assert_eq!(actors[0].name.as_deref(), Some("Actor"));
        assert_eq!(actors[0].object_type, 123);
        for (at, v) in [(52, 44), (52, 80), (56, 1), (44, 5), (68, 3), (12, 2)] {
            let mut bad = data.clone();
            word(&mut bad, at, v);
            assert!(first_flick(&bad).is_err());
        }
    }
    #[test]
    fn attachment_links_reject_invalid_slots_and_string_pointers() {
        let mut data = vec![0; 192];
        for (at, value) in [
            (0, 3),
            (4, 192),
            (8, 1),
            (12, 2),
            (24, 1),
            (28, 4),
            (36, 192),
            (40, 2),
            (44, 6),
            (48, 0),
            (52, 120),
            (56, 0),
            (60, 123),
            (64, 2),
            (68, 6),
            (72, u32::MAX),
            (76, 0),
            (80, 1),
            (84, 456),
            (88, 3),
            (92, 6),
            (96, 0),
            (100, 126),
            (104, 1),
            (108, 131),
            (112, 5),
            (116, 2),
        ] {
            word(&mut data, at, value);
        }
        data[120..126].copy_from_slice(b"actor\0");
        data[126..131].copy_from_slice(b"hand\0");
        data[131..136].copy_from_slice(b"grip\0");
        let links = first_attachments(&data).unwrap();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].parent_node, "hand");
        assert_eq!(links[0].child_node, "grip");
        let declarations = attachment_declarations(&data).unwrap();
        assert_eq!(declarations[0].offset, 88);
        assert_eq!(declarations[0].child_node.as_deref(), Some("grip"));
        let mut null_handle = data.clone();
        word(&mut null_handle, 108, 0);
        let declarations = attachment_declarations(&null_handle).unwrap();
        assert_eq!(declarations[0].parent_node, "hand");
        assert!(declarations[0].child_node.is_none());
        assert!(first_attachments(&null_handle).is_err());
        let mut inline_parent = data.clone();
        word(&mut inline_parent, 52, 0);
        word(&mut inline_parent, 60, u32::MAX);
        assert!(preview_attachments(&inline_parent).is_err());
        assert!(preview_attachments_for_classes(&inline_parent, &[456]).is_err());
        assert!(
            preview_attachments_for_classes(&inline_parent, &[665])
                .unwrap()
                .is_empty()
        );
        word(&mut inline_parent, 100, 88);
        assert!(preview_attachments_for_classes(&inline_parent, &[665]).is_err());
        for (at, value) in [(96, 2), (104, 2), (100, 88), (108, 192)] {
            let mut bad = data.clone();
            word(&mut bad, at, value);
            assert!(attachment_declarations(&bad).is_err());
        }
        for (at, value) in [(96, 1), (104, 0), (104, 2), (100, 88), (108, 192), (92, 7)] {
            let mut bad = data.clone();
            word(&mut bad, at, value);
            assert!(first_attachments(&bad).is_err());
        }
    }
    #[test]
    fn attachment_resource_bounds_animation_and_scale() {
        let mut data = vec![0; 180];
        let b = 36;
        word(&mut data, 0, 0x1a0002e5);
        word(&mut data, 16, 32);
        word(&mut data, 32, 144);
        for (at, v) in [
            (0, 1),
            (4, 28),
            (8, 0),
            (12, 1),
            (16, 40),
            (20, 1),
            (24, 100),
            (28, 7),
            (40, 9),
            (45, 7),
            (100, 123),
            (105, 9),
        ] {
            word(&mut data, b + at, v);
        }
        data[b + 32..b + 38].copy_from_slice(b"mesh0\0");
        data[b + 44] = 0x51;
        data[b + 57..b + 69].copy_from_slice(b"\x52\x01model.anm\0");
        data[b + 104] = 1;
        data[b + 109] = 0x17;
        word(&mut data, b + 110, 0.6f32.to_bits());
        data[b + 114] = 255;
        let (model, animation, scale) = attachment_resource(&data, 123).unwrap();
        assert_eq!((model.as_str(), animation.as_str()), ("mesh0", "model.anm"));
        assert_eq!(scale, 0.6);
        let mut bad = data.clone();
        word(&mut bad, b + 110, f32::NAN.to_bits());
        assert!(attachment_resource(&bad, 123).is_err());
        let mut bad = data.clone();
        bad[b + 58] = 2;
        assert!(attachment_resource(&bad, 123).is_err());
        let mut bad = data;
        bad[b + 59] = b'/';
        assert!(attachment_resource(&bad, 123).is_err());
    }
    #[test]
    fn catalog_dependencies_preserve_order_duplicates_and_reject_bad_records() {
        let mut data = vec![0; 132];
        word(&mut data, 0, 0x1a0002e5);
        word(&mut data, 28, 32);
        word(&mut data, 32, 3);
        data[36..42].copy_from_slice(b"first\0");
        data[68..75].copy_from_slice(b"second\0");
        data[100..106].copy_from_slice(b"first\0");
        assert_eq!(
            catalog_includes(&data).unwrap(),
            ["first", "second", "first"]
        );
        for at in [36, 40, 68] {
            let mut bad = data.clone();
            bad[at] = b'/';
            assert!(catalog_includes(&bad).is_err());
        }
        let mut bad = data.clone();
        bad[36..68].fill(b'a');
        assert!(catalog_includes(&bad).is_err());
        assert!(catalog_includes(&data[..131]).is_err());
        word(&mut data, 32, 4097);
        assert!(catalog_includes(&data).is_err());
        word(&mut data, 28, 0);
        assert!(catalog_includes(&data).unwrap().is_empty());
    }
    #[test]
    fn spec_terminator_may_share_the_next_record_identifier_byte() {
        let mut data = vec![0; 108];
        let b = 36;
        for (at, v) in [
            (0, 0x1a0002e5u32),
            (16, 32),
            (32, 72),
            (b, 1),
            (b + 4, 28),
            (b + 12, 1),
            (b + 16, 40),
            (b + 20, 1),
            (b + 24, 57),
            (b + 28, 7),
            (b + 40, 9),
            (b + 45, 7),
            (b + 49, 60f32.to_bits()),
            (b + 57, 255),
            (b + 62, 9),
        ] {
            word(&mut data, at, v);
        }
        data[b + 32..b + 38].copy_from_slice(b"mesh0\0");
        data[b + 44] = 0x51;
        data[b + 61] = 1;
        data[b + 66] = 255;
        let programs = catalog_spec_programs(&data).unwrap();
        let program = programs[0].1.as_ref().unwrap();
        assert_eq!(program.consumed, 14);
        assert_eq!(program.records.last().unwrap().opcode, 255);
        data[b + 57] = 0x7e;
        assert!(catalog_spec_programs(&data).unwrap()[0].1.is_err());
    }
    #[test]
    fn catalog_follows_instance_spec_and_model_references() {
        let mut data = vec![0; 108];
        word(&mut data, 0, 0x1a0002e5);
        word(&mut data, 16, 32);
        word(&mut data, 32, 72);
        let b = 36;
        for (at, v) in [
            (0, 1),
            (4, 28),
            (8, 0),
            (12, 1),
            (16, 40),
            (20, 1),
            (24, 60),
            (28, 7),
            (40, 9),
            (45, 7),
            (60, 123),
            (65, 9),
        ] {
            word(&mut data, b + at, v);
        }
        data[b + 32..b + 38].copy_from_slice(b"mesh0\0");
        data[b + 44] = 0x51;
        data[b + 57] = 255;
        data[b + 64] = 1;
        data[b + 69] = 255;
        assert_eq!(first_model(&data, 123).unwrap(), "mesh0");
        assert!(first_model(&data, 124).is_err());
        for (at, v) in [(b + 16, 4000), (b + 45, 8), (b + 65, 10), (32, 1000)] {
            let mut bad = data.clone();
            word(&mut bad, at, v);
            assert!(first_model(&bad, 123).is_err());
        }
        let mut unsupported = data;
        unsupported[b + 44] = 0x52;
        assert!(first_model(&unsupported, 123).is_err());
    }
}
