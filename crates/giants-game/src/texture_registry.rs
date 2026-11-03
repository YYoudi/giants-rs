//! Preorder catalog registration; newest table first, source order within it.
use std::path::{Path, PathBuf};

/// Result of the measured registered-texture file-loading branch. The caller
/// must supply the backend result and global option word; neither is guessed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileChoice {
    Existing,
    Tga,
    Dds,
}
pub fn registered_file_choice(
    data_present: bool,
    backend_result: i32,
    flags: u8,
    dds_exists: bool,
    options: u32,
) -> FileChoice {
    if data_present {
        FileChoice::Existing
    } else if backend_result == 0 && flags & 4 == 0 && dds_exists && options & 0x2000 != 0 {
        FileChoice::Dds
    } else {
        FileChoice::Tga
    }
}

#[derive(Debug)]
pub struct Definition {
    pub catalog: PathBuf,
    pub table_offset: usize,
    pub entry: usize,
    pub declaration: giants_formats::texture_table::Entry,
}
#[derive(Default, Debug)]
pub struct Registry {
    pub tables: Vec<(PathBuf, Vec<Definition>)>,
}
impl Registry {
    pub fn register(&mut self, path: &Path, data: &[u8]) -> Result<(), giants_formats::Error> {
        if self.tables.iter().any(|(old, _)| {
            old.to_string_lossy()
                .eq_ignore_ascii_case(&path.to_string_lossy())
        }) {
            return Ok(());
        }
        let table = giants_formats::texture_table::catalog(data)?;
        let entries = table
            .entries
            .into_iter()
            .enumerate()
            .map(|(entry, declaration)| Definition {
                catalog: path.into(),
                table_offset: table.offset,
                entry,
                declaration,
            })
            .collect();
        self.tables.push((path.into(), entries));
        Ok(())
    }
    pub fn lookup(&self, name: &str) -> Option<&Definition> {
        self.tables
            .iter()
            .rev()
            .flat_map(|(_, entries)| entries)
            .find(|d| d.declaration.name.eq_ignore_ascii_case(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn newest_table_first_entry_and_repeated_catalog_is_not_reregistered() {
        fn data(names: &[&str]) -> Vec<u8> {
            let mut raw = vec![0; 32];
            raw[..4].copy_from_slice(&0x1a0002e5u32.to_le_bytes());
            raw[8..12].copy_from_slice(&32u32.to_le_bytes());
            raw.extend((names.len() as u32).to_le_bytes());
            for name in names {
                raw.extend([0, 0, (name.len() + 1) as u8]);
                raw.extend(name.as_bytes());
                raw.push(0);
            }
            raw
        }
        let mut registry = Registry::default();
        registry
            .register(Path::new("root.bin"), &data(&["leaf"]))
            .unwrap();
        registry
            .register(Path::new("child.bin"), &data(&["LEAF", "leaf"]))
            .unwrap();
        registry.register(Path::new("ROOT.BIN"), &[]).unwrap();
        let target = registry.lookup("lEaF").unwrap();
        assert_eq!(target.catalog, Path::new("child.bin"));
        assert_eq!(target.entry, 0);
        assert!(registry.lookup("missing").is_none());
        assert_eq!(registry.tables.len(), 2);
    }
}
