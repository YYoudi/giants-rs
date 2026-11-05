//! Cold Kabuto animation definitions from a user-supplied PE data section.
//! No native instructions are executed or copied. Layout recognition is not
//! cryptographic authentication; the original executable is never bundled.
//! Owns cache data and its acquired reset flag; world dispatch scheduling remains
//! separate from this resource service.
use crate::specification;
use std::{path::Path, sync::Arc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Definition {
    pub animation: u8,
    pub lower: i32,
    pub upper: i32,
    pub reference: i32,
    pub flags: u32,
    /// Opaque six-word external prefix, not a complete parameter schema,
    /// callable address or interpreted behavior.
    pub parameters: Option<[u32; 6]>,
}
#[derive(Clone, Debug)]
pub struct Definitions {
    entries: Vec<Definition>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub definition: Definition,
    /// Serialized ANM offset, never an original process pointer.
    pub node: Option<usize>,
}
pub struct ColdCache {
    specification: Arc<specification::Prepared>,
    entries: Vec<Entry>,
    node_specifications: std::collections::BTreeMap<u8, Arc<specification::Prepared>>,
}
impl ColdCache {
    pub fn specification(&self) -> &Arc<specification::Prepared> {
        &self.specification
    }
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }
    /// A skipped row can retain a node from the preceding resource context.
    /// Its owner stays explicit; an offset is never rebound to another ANM.
    pub fn node_specification(&self, animation: u8) -> Option<&Arc<specification::Prepared>> {
        self.node_specifications.get(&animation)
    }
}

/// Owned equivalent of the acquired cache flag/reset contract. Dispatching the
/// reset at the correct world boundary remains the caller's responsibility.
pub struct Lifecycle {
    definitions: Definitions,
    cache: Option<Arc<ColdCache>>,
    initialized: bool,
}
impl Lifecycle {
    pub fn new(definitions: Definitions) -> Self {
        Self {
            definitions,
            cache: None,
            initialized: false,
        }
    }
    pub fn initialized(&self) -> bool {
        self.initialized
    }
    pub fn cached(&self) -> Option<&Arc<ColdCache>> {
        self.cache.as_ref()
    }
    /// Like the acquired hook: clear only the initialized flag. Keep clamped
    /// bounds and node records until their next successful preparation.
    pub fn reset(&mut self) {
        self.initialized = false;
    }
    pub fn initialize(
        &mut self,
        specification: Arc<specification::Prepared>,
    ) -> Result<Arc<ColdCache>, String> {
        if self.initialized {
            let cache = self.cache.as_ref().unwrap();
            if !Arc::ptr_eq(cache.specification(), &specification) {
                return Err("Kabuto animation resources changed without reset".into());
            }
            return Ok(cache.clone());
        }
        let mut next = self.definitions.prepare(specification)?;
        if let Some(previous) = &self.cache {
            for entry in &mut next.entries {
                if entry.node.is_none()
                    && let Some(old) = previous
                        .entries
                        .iter()
                        .find(|e| e.definition.animation == entry.definition.animation)
                    && old.node.is_some()
                {
                    entry.node = old.node;
                    next.node_specifications.insert(
                        entry.definition.animation,
                        previous.node_specifications[&entry.definition.animation].clone(),
                    );
                }
            }
        }
        // Stage all fallible work before publishing either bounds or cache.
        self.definitions.entries = next.entries.iter().map(|e| e.definition.clone()).collect();
        let next = Arc::new(next);
        self.cache = Some(next.clone());
        self.initialized = true;
        Ok(next)
    }
}
#[cfg(test)]
use crate::executable_metadata::MAX_IMAGE;
use crate::executable_metadata::{DataSection, Layout, word};
impl DataSection<'_> {
    fn definitions(&self, table: u32, count: usize) -> Result<Definitions, String> {
        let rows = self.data(table, (count + 1).checked_mul(28).ok_or("table overflow")?)?;
        if word(rows, count * 28)? != 254 {
            return Err("missing Kabuto animation sentinel".into());
        }
        let mut entries = Vec::with_capacity(count);
        for row in rows[..count * 28].as_chunks::<28>().0 {
            let slot = word(row, 0)?;
            if slot > 255
                || slot == 254
                || entries
                    .iter()
                    .any(|e: &Definition| u32::from(e.animation) == slot)
            {
                return Err("invalid or duplicate Kabuto animation slot".into());
            }
            if word(row, 24)? != 0 {
                return Err("serialized animation cache must be cold".into());
            }
            let pointer = word(row, 20)?;
            let parameters = if pointer == 0 {
                None
            } else {
                let rva = pointer
                    .checked_sub(self.base)
                    .ok_or("parameter VA below image")?;
                let data = self.data(rva, 24)?;
                let mut values = [0; 6];
                for (i, value) in values.iter_mut().enumerate() {
                    *value = word(data, i * 4)?;
                }
                Some(values)
            };
            entries.push(Definition {
                animation: slot as u8,
                lower: word(row, 4)? as i32,
                upper: word(row, 8)? as i32,
                reference: word(row, 12)? as i32,
                flags: word(row, 16)?,
                parameters,
            });
        }
        Ok(Definitions { entries })
    }
}
impl Definitions {
    /// Explicit configurable file input; no search of archives or native launch.
    pub fn read(path: &Path) -> Result<Self, String> {
        Self::decode(&crate::executable_metadata::read(path)?)
    }
    pub fn entries(&self) -> &[Definition] {
        &self.entries
    }
    /// Three independently inspected PC metadata layouts. Header tuples select
    /// the data layout only; modified files are not authenticated by this API.
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        let image = crate::executable_metadata::Image::decode(bytes)?;
        let table = match image.layout {
            Layout::DebugReference => 0x160680,
            Layout::Retail => 0x15a4d8,
            Layout::Original => 0x15c5f8,
        };
        image.data.definitions(table, 62)
    }
    /// Atomic immutable cold preparation, reusing the shared reference resolver.
    /// Missing animation slots retain their source definition. A declared slot
    /// whose reference cannot resolve refuses preparation; no fallback node.
    pub fn prepare(
        &self,
        specification: Arc<specification::Prepared>,
    ) -> Result<ColdCache, String> {
        if specification.class != 1 || specification.specification != 1 {
            return Err("Kabuto animation cache requires class/specification1".into());
        }
        let mut caches = std::collections::BTreeMap::new();
        let mut entries = Vec::with_capacity(self.entries.len());
        let mut node_specifications = std::collections::BTreeMap::new();
        for definition in &self.entries {
            let mut entry = Entry {
                definition: definition.clone(),
                node: None,
            };
            if let Some(descriptor) = specification.animations.get(&definition.animation)
                && definition.reference >= 0
            {
                let cache = caches
                    .entry(definition.animation)
                    .or_insert_with(|| vec![None; specification.metadata.references.names.len()]);
                entry.node = specification
                    .metadata
                    .references
                    .resolve_cached(&descriptor.animation, cache, definition.reference as usize)
                    .map_err(|e| e.to_string())?
                    .node();
                if entry.node.is_none() {
                    return Err("Kabuto animation reference unavailable".into());
                }
                clamp(&mut entry.definition, descriptor.animation.timeline);
                node_specifications.insert(definition.animation, specification.clone());
            }
            entries.push(entry);
        }
        Ok(ColdCache {
            specification,
            entries,
            node_specifications,
        })
    }
}
fn clamp(definition: &mut Definition, timeline: [i32; 4]) {
    definition.lower = definition.lower.max(timeline[0]);
    definition.upper = definition.upper.min(timeline[1].wrapping_sub(1));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "Requires GIANTS_KABUTO_ROOT with supplied executable/data; no window or sound"]
    fn source_kabuto_cache_retains_bounds_and_node_owners_across_reset() {
        let root = std::path::PathBuf::from(std::env::var("GIANTS_KABUTO_ROOT").unwrap());
        let source = crate::world_source::resolve(&root, "Kstory1").unwrap();
        let menu = crate::mission_selection::MissionMenu::parse(
            &std::fs::read(root.join("Bin").join(source.catalog)).unwrap(),
            u32::MAX,
            true,
        )
        .unwrap();
        let packet =
            crate::mission_loading::prepare(&root, menu.request("Kstory1", 0).unwrap()).unwrap();
        let plan = packet.instance_plan(&root, "English").unwrap();
        let prepare = || specification::Prepared::load_registered(plan.resources(), 1).unwrap();
        let path = std::env::var_os("GIANTS_KABUTO_EXECUTABLE")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| root.join("Giants.exe"));
        let mut lifecycle = Lifecycle::new(Definitions::read(&path).unwrap());
        let spec = Arc::new(prepare());
        let cold = lifecycle.initialize(spec.clone()).unwrap();
        let warm = lifecycle.initialize(spec).unwrap();
        assert!(Arc::ptr_eq(&cold, &warm));
        assert!(lifecycle.initialize(Arc::new(prepare())).is_err());
        assert!(lifecycle.initialized());
        lifecycle.reset();
        assert!(Arc::ptr_eq(lifecycle.cached().unwrap(), &cold));
        let mut narrow_spec = prepare();
        for descriptor in narrow_spec.animations.values_mut() {
            descriptor.animation.timeline[..2].copy_from_slice(&[10000, 20000]);
        }
        let narrow = lifecycle.initialize(Arc::new(narrow_spec)).unwrap();
        for (old, next) in cold.entries().iter().zip(narrow.entries()) {
            if old.node.is_some() {
                assert_eq!(next.definition.lower, old.definition.lower.max(10000));
                assert_eq!(next.definition.upper, old.definition.upper.min(19999));
            }
        }
        lifecycle.reset();
        let mut wider_spec = prepare();
        for descriptor in wider_spec.animations.values_mut() {
            descriptor.animation.timeline[..2].copy_from_slice(&[-10000, 30000]);
        }
        let wider = lifecycle.initialize(Arc::new(wider_spec)).unwrap();
        assert_eq!(narrow.entries(), wider.entries());
        let slot = wider
            .entries()
            .iter()
            .find(|e| e.node.is_some())
            .unwrap()
            .definition
            .animation;
        lifecycle.reset();
        let mut missing_spec = prepare();
        for descriptor in missing_spec.animations.values_mut() {
            descriptor.animation.timeline[..2].copy_from_slice(&[-10000, 30000]);
        }
        missing_spec.animations.remove(&slot);
        let missing_spec = Arc::new(missing_spec);
        let missing = lifecycle.initialize(missing_spec.clone()).unwrap();
        assert_eq!(wider.entries(), missing.entries());
        assert!(Arc::ptr_eq(
            missing.node_specification(slot).unwrap(),
            wider.specification()
        ));
        assert!(!Arc::ptr_eq(
            missing.node_specification(slot).unwrap(),
            &missing_spec
        ));
        lifecycle.reset();
        let mut invalid = prepare();
        invalid.class = 0;
        assert!(lifecycle.initialize(Arc::new(invalid)).is_err());
        assert!(!lifecycle.initialized());
        assert!(Arc::ptr_eq(lifecycle.cached().unwrap(), &missing));
        let weak = Arc::downgrade(&missing);
        drop(missing);
        drop(lifecycle);
        assert!(weak.upgrade().is_none());
    }
    fn synthetic_image() -> Vec<u8> {
        let mut bytes = crate::executable_metadata::synthetic_image(Layout::DebugReference);
        let put = |bytes: &mut [u8], at, value: u32| {
            bytes[at..at + 4].copy_from_slice(&value.to_le_bytes())
        };
        let table = 4096 + 0x160680 - 1_413_120;
        for i in 0..62 {
            put(&mut bytes, table + i * 28, i as u32);
            put(&mut bytes, table + i * 28 + 12, u32::MAX);
        }
        put(&mut bytes, table + 62 * 28, 254);
        bytes
    }
    #[test]
    fn external_pe_table_uses_file_mapping_and_refuses_malformed_sections() {
        let valid = synthetic_image();
        assert_eq!(Definitions::decode(&valid).unwrap().entries().len(), 62);
        for (at, value) in [
            (136, 0),                        // Unknown layout.
            (376 + 2 * 40 + 36, 0xe0000040), // Executable data.
            (376 + 2 * 40 + 20, u32::MAX),   // Truncated raw section.
            (376 + 3 * 40 + 20, 4096),       // Aliased file ranges.
            (376 + 3 * 40 + 12, 1_413_120),  // Aliased RVAs.
            (4096 + 0x160680 - 1_413_120 + 62 * 28, 253),
            (4096 + 0x160680 - 1_413_120 + 28, 0), // Duplicate slot.
            (4096 + 0x160680 - 1_413_120 + 20, 0x401000), // Parameter outside data.
        ] {
            let mut bytes = valid.clone();
            bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
            assert!(
                Definitions::decode(&bytes).is_err(),
                "accepted mutation at {at}"
            );
        }
    }
    #[test]
    fn data_mapping_uses_rva_and_preserves_signed_ranges_and_opaque_data() {
        // Synthetic data only: raw file offsets deliberately differ from RVAs.
        let mut data = vec![0; 112];
        let put = |data: &mut [u8], at, v: u32| data[at..at + 4].copy_from_slice(&v.to_le_bytes());
        for (i, v) in [7, (-20i32) as u32, 4, (-1i32) as u32, 9, 0x402050, 0]
            .into_iter()
            .enumerate()
        {
            put(&mut data, i * 4, v);
        }
        put(&mut data, 28, 254);
        put(&mut data, 80, 123);
        let section = DataSection {
            bytes: &data,
            rva: 0x2000,
            base: 0x400000,
        };
        let defs = section.definitions(0x2000, 1).unwrap();
        let mut definition = defs.entries[0].clone();
        assert_eq!(definition.reference, -1);
        assert_eq!(definition.parameters.unwrap()[0], 123);
        clamp(&mut definition, [8, 40, 0, 0]);
        assert_eq!((definition.lower, definition.upper), (8, 4));
        assert_eq!(definition.flags, 9);
        assert!(section.definitions(0x1fff, 1).is_err());
        assert!(section.data(u32::MAX, 24).is_err());
        put(&mut data, 24, 1);
        assert!(
            DataSection {
                bytes: &data,
                rva: 0x2000,
                base: 0x400000
            }
            .definitions(0x2000, 1)
            .is_err()
        );
    }
    #[test]
    fn malformed_external_images_refuse_without_panicking() {
        for len in [0, 1, 2, 64, 256, 512, MAX_IMAGE + 1] {
            let mut data = vec![0; len];
            if len >= 2 {
                data[..2].copy_from_slice(b"MZ");
            }
            if len >= 64 {
                data[60..64].copy_from_slice(&u32::MAX.to_le_bytes());
            }
            assert!(Definitions::decode(&data).is_err());
        }
    }
}
