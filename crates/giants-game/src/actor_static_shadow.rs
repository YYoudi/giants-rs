//! Initial registered static-shadow declaration, not a rendered shadow.
//! Prototype PS2 `object_copy_personality` / `texture_add_file` are unknown
//! navigation routes; their names do not establish a PC identity or layout.
use std::{path::PathBuf, sync::Arc};

/// Shared identity from the actual ordered texture registry. This admits only
/// cold registered descriptors, not missing-name placeholders, generated
/// textures or the native private-copy branch of a dynamic shadow.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegisteredShadow {
    pub catalog: PathBuf,
    pub entry: usize,
    pub name: String,
}

pub fn prepare(
    record: &giants_formats::class_program::Record,
    registry: &crate::texture_registry::Registry,
) -> Result<(f32, Option<Arc<RegisteredShadow>>), String> {
    if record.opcode != 2 || record.bytes.first() != Some(&2) {
        return Err("static shadow requires its declaration".into());
    }
    let divisor = f32::from_le_bytes(
        record
            .bytes
            .get(1..5)
            .ok_or("static shadow divisor absent")?
            .try_into()
            .unwrap(),
    );
    if !divisor.is_finite() {
        return Err("nonfinite static shadow divisor outside acquired profile".into());
    }
    let tail = record.bytes.get(5..).ok_or("static shadow name absent")?;
    if tail.is_empty()
        || tail.len() > 128
        || tail.last() != Some(&0)
        || tail[..tail.len() - 1].contains(&0)
        || !tail.is_ascii()
    {
        return Err("static shadow name framing outside acquired profile".into());
    }
    let reciprocal = if divisor == 0. {
        0.
    } else {
        (1. / f64::from(divisor)) as f32
    };
    let name = std::str::from_utf8(&tail[..tail.len() - 1]).unwrap();
    let binding = if name.is_empty() {
        None
    } else {
        let definition = registry.lookup(name).ok_or(
            "static shadow requires an existing registered texture; placeholder allocation unbound",
        )?;
        Some(Arc::new(RegisteredShadow {
            catalog: definition.catalog.clone(),
            entry: definition.entry,
            name: definition.declaration.name.clone(),
        }))
    };
    Ok((reciprocal, binding))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_declaration_clears_binding_but_preserves_signed_reciprocal() {
        let program = giants_formats::class_program::parse(&[2, 0, 0, 0, 192, 0, 255]).unwrap();
        let (reciprocal, binding) = prepare(
            &program.records[0],
            &crate::texture_registry::Registry::default(),
        )
        .unwrap();
        assert_eq!(reciprocal.to_bits(), (-0.5_f32).to_bits());
        assert!(binding.is_none());
        let missing = giants_formats::class_program::parse(&[2, 0, 0, 0, 0, b'x', 0, 255]).unwrap();
        assert!(
            prepare(
                &missing.records[0],
                &crate::texture_registry::Registry::default()
            )
            .is_err()
        );
    }

    #[test]
    #[ignore = "requires GIANTS_BUILDING_ROOT; initial source fields/copies, no GPU or shadow draw"]
    fn installed_entrance_retains_registered_shadow_and_shared_copy_identity() {
        let root = std::path::PathBuf::from(std::env::var_os("GIANTS_BUILDING_ROOT").unwrap());
        let menu = crate::mission_selection::MissionMenu::parse(
            &std::fs::read(root.join("Bin/w_Story2.bin")).unwrap(),
            0,
            true,
        )
        .unwrap();
        let packet =
            crate::mission_loading::prepare(&root, menu.request("Story2", 0).unwrap()).unwrap();
        let plan = packet.instance_plan(&root, "English").unwrap();
        let resources = plan.resources();
        let spec = crate::specification::Prepared::load_registered(resources, 544).unwrap();
        let fields = crate::actor_fields::Fields::load(resources, 544, &spec).unwrap();
        let binding = fields.shadow_texture.as_ref().unwrap();
        let definition = resources.classes.textures.lookup(&binding.name).unwrap();
        assert_eq!(binding.catalog, definition.catalog);
        assert_eq!(binding.entry, definition.entry);
        assert_eq!(fields.shadow_reciprocal.to_bits(), 0);
        assert!(fields.shadow.is_none());
        let copy = fields.instantiate();
        assert!(Arc::ptr_eq(binding, copy.shadow_texture.as_ref().unwrap()));
        let stage = crate::building_geometry::Stage::health(resources, 544, 0.75).unwrap();
        assert!(!stage.identities.is_empty());
    }
}
