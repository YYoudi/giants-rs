//! Candidate definitions and archive members, before native load-order selection.
//! Filesystem enumeration order is never used to resolve a conflict.
use giants_formats::{cast, gzp::Archive};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub enum Resolution<'a, T> {
    Missing,
    Unique(&'a T),
    Ambiguous(&'a [T]),
}
fn resolution<T>(values: &[T]) -> Resolution<'_, T> {
    match values {
        [] => Resolution::Missing,
        [one] => Resolution::Unique(one),
        many => Resolution::Ambiguous(many),
    }
}
#[derive(Debug)]
pub struct ClassDefinition {
    pub catalog: PathBuf,
    pub class_id: i32,
    pub models: Result<Vec<cast::ModelReference>, String>,
    pub scale: Result<f32, String>,
}
#[derive(Debug)]
pub struct Issue {
    pub file: PathBuf,
    pub error: String,
}
#[derive(Default, Debug)]
pub struct ClassRegistry {
    pub definitions: BTreeMap<i32, Vec<ClassDefinition>>,
    pub issues: Vec<Issue>,
    pub catalog_files: usize,
}
fn paths(directory: &Path, extension: &str) -> std::io::Result<Vec<PathBuf>> {
    let mut result = std::fs::read_dir(directory)?
        .map(|entry| entry.map(|e| e.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    result.retain(|p| {
        p.is_file()
            && p.extension()
                .is_some_and(|e| e.eq_ignore_ascii_case(extension))
    });
    result.sort();
    Ok(result)
}
impl ClassRegistry {
    /// Index declarations without a requested-class allowlist. A unique file
    /// candidate still does not prove that the native loader registers it.
    pub fn scan(directory: &Path) -> std::io::Result<Self> {
        let mut result = Self::default();
        for path in paths(directory, "bin")? {
            if path.metadata()?.len() > 64 * 1024 * 1024 {
                result.issues.push(Issue {
                    file: path,
                    error: "catalog input exceeds budget".into(),
                });
                continue;
            }
            let data = std::fs::read(&path)?;
            if data.get(..4) != Some(&0x1a0002e5u32.to_le_bytes()) {
                continue;
            }
            result.catalog_files += 1;
            result.register(&path, &data);
        }
        Ok(result)
    }
    pub fn register(&mut self, catalog: &Path, data: &[u8]) {
        let ids = match cast::catalog_class_ids(data) {
            Ok(ids) => ids,
            Err(error) => {
                self.issues.push(Issue {
                    file: catalog.into(),
                    error: error.to_string(),
                });
                return;
            }
        };
        for class_id in ids {
            self.definitions
                .entry(class_id)
                .or_default()
                .push(ClassDefinition {
                    catalog: catalog.into(),
                    class_id,
                    models: cast::model_prefix(data, class_id).map_err(|e| e.to_string()),
                    scale: cast::class_scale(data, class_id).map_err(|e| e.to_string()),
                });
        }
    }
    pub fn resolve(&self, class_id: i32) -> Resolution<'_, ClassDefinition> {
        resolution(self.definitions.get(&class_id).map_or(&[], Vec::as_slice))
    }
}
/// Source-derived dependency visits and definitions in native append order.
/// Archive precedence and execution of class command streams are separate.
#[derive(Debug)]
pub struct WorldClassRegistry {
    pub visits: Vec<PathBuf>,
    pub classes: ClassRegistry,
    pub models: crate::model_registry::ModelRegistry,
    pub textures: crate::texture_registry::Registry,
}
impl WorldClassRegistry {
    pub fn load(directory: &Path, root: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self::load_with_entry_order(directory, root)?.0)
    }
    /// Catalog entry order differs from class append order. Sound declarations
    /// are consumed on entry; class registrations remain after their children.
    pub fn load_with_entry_order(
        directory: &Path,
        root: &str,
    ) -> Result<(Self, Vec<PathBuf>), Box<dyn std::error::Error>> {
        let mut files = BTreeMap::new();
        for path in paths(directory, "bin")? {
            let key = path
                .file_name()
                .ok_or("missing catalog filename")?
                .to_string_lossy()
                .to_ascii_lowercase();
            if files.insert(key, path).is_some() {
                return Err("case-ambiguous catalog filename".into());
            }
        }
        let mut result = Self {
            visits: Vec::new(),
            classes: ClassRegistry::default(),
            models: crate::model_registry::ModelRegistry::default(),
            textures: crate::texture_registry::Registry::default(),
        };
        let mut entries = Vec::new();
        result.visit(root, &files, &mut Vec::new(), &mut 0, &mut entries)?;
        if !result.classes.issues.is_empty() {
            return Err("invalid class table in world dependency context".into());
        }
        Ok((result, entries))
    }
    fn visit(
        &mut self,
        name: &str,
        files: &BTreeMap<String, PathBuf>,
        stack: &mut Vec<String>,
        visits: &mut usize,
        entries: &mut Vec<PathBuf>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if name.len() <= 4
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'.')
            || !name.to_ascii_lowercase().ends_with(".bin")
            || name[..name.len() - 4].contains('.')
        {
            return Err("invalid catalog filename".into());
        }
        let key = name.to_ascii_lowercase();
        *visits += 1;
        if stack.contains(&key) || stack.len() >= 64 || *visits > 4096 {
            return Err("catalog dependency cycle or visit budget".into());
        }
        let path = files.get(&key).ok_or("missing catalog dependency")?;
        if path.metadata()?.len() > 64 * 1024 * 1024 {
            return Err("catalog input exceeds budget".into());
        }
        let data = std::fs::read(path)?;
        let includes = cast::catalog_includes(&data)?;
        entries.push(path.clone());
        self.textures.register(path, &data)?;
        self.models.enter_catalog(path, &data)?;
        stack.push(key);
        for include in includes {
            self.visit(&format!("{include}.bin"), files, stack, visits, entries)?;
        }
        stack.pop();
        self.visits.push(path.clone());
        self.classes.catalog_files += 1;
        self.classes.register(path, &data);
        self.models.register(path, &data)?;
        Ok(())
    }
    /// Native object lookup scans appended blocks from the head, then entries
    /// in source order. The candidate index's ambiguity API remains available.
    /// A returned definition may still contain unsupported model/scale errors.
    pub fn first_registered(&self, id: i32) -> Option<&ClassDefinition> {
        self.classes.definitions.get(&id)?.first()
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    pub archive: PathBuf,
    pub member: String,
}
#[derive(Default, Debug)]
pub struct ResourceIndex {
    pub members: BTreeMap<String, Vec<Location>>,
    pub issues: Vec<Issue>,
    pub archive_files: usize,
}
impl ResourceIndex {
    /// Select within one Bin directory, with the native language filter and
    /// case-insensitive archive basename precedence. Other search tiers and
    /// loose-file overrides are deliberately outside this directory index.
    pub fn select_bin(&self, member: &str, language: &str) -> Result<&Location, String> {
        if language.is_empty() || !language.bytes().all(|b| b.is_ascii_alphabetic()) {
            return Err("invalid resource language".into());
        }
        let language = language.to_ascii_lowercase();
        let candidates = self
            .members
            .get(&member.to_ascii_lowercase())
            .ok_or("resource missing from archive index")?;
        let mut selected: Option<(&Location, String)> = None;
        let mut parent = None;
        for location in candidates {
            if parent.is_some_and(|p| location.archive.parent() != Some(p)) {
                return Err("archive search tiers require explicit resolution".into());
            }
            parent = location.archive.parent();
            let name = location
                .archive
                .file_name()
                .and_then(|v| v.to_str())
                .filter(|v| v.is_ascii())
                .ok_or("non-ASCII archive basename")?
                .to_ascii_lowercase();
            if name
                .strip_prefix("vo_sfx")
                .is_some_and(|tail| !tail.starts_with(&language))
            {
                continue;
            }
            match &selected {
                Some((_, old)) if name == *old => {
                    return Err("ambiguous archive/member identity".into());
                }
                Some((_, old)) if name > *old => {}
                _ => selected = Some((location, name)),
            }
        }
        selected
            .map(|(location, _)| location)
            .ok_or("resource absent for selected language".into())
    }
    pub fn read_bin(
        &self,
        member: &str,
        language: &str,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let selected = self.select_bin(member, language)?;
        let data = std::fs::read(&selected.archive)?;
        Ok(Archive::parse(&data)?.read(&selected.member)?)
    }
    /// Index metadata only; source payloads are neither extracted nor retained.
    pub fn scan(directory: &Path) -> std::io::Result<Self> {
        let mut result = Self::default();
        for path in paths(directory, "gzp")? {
            let data = std::fs::read(&path)?;
            let archive = match Archive::parse(&data) {
                Ok(archive) => archive,
                Err(error) => {
                    result.issues.push(Issue {
                        file: path,
                        error: error.to_string(),
                    });
                    continue;
                }
            };
            result.archive_files += 1;
            for entry in archive.entries() {
                result
                    .members
                    .entry(entry.name.to_ascii_lowercase())
                    .or_default()
                    .push(Location {
                        archive: path.clone(),
                        member: entry.name.clone(),
                    });
            }
        }
        Ok(result)
    }
    pub fn resolve(&self, member: &str) -> Resolution<'_, Location> {
        resolution(
            self.members
                .get(&member.to_ascii_lowercase())
                .map_or(&[], Vec::as_slice),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn catalog() -> Vec<u8> {
        let mut data = vec![0; 112];
        for (at, value) in [
            (0, 0x1a0002e5u32),
            (8, 108),
            (16, 32),
            (32, 72),
            (36, 1),
            (40, 28),
            (48, 1),
            (52, 40),
            (56, 1),
            (60, 60),
            (64, 7),
            (76, 9),
            (81, 7),
            (96, 123),
            (101, 9),
        ] {
            data[at..at + 4].copy_from_slice(&value.to_le_bytes());
        }
        data[68..74].copy_from_slice(b"mesh0\0");
        data[80] = 0x51;
        data[93] = 255;
        data[100] = 1;
        data[105] = 255;
        data
    }
    #[test]
    fn world_registry_visits_dependencies_before_local_and_retains_repeats() {
        let directory =
            std::env::temp_dir().join(format!("giants-catalog-plan-{}", std::process::id()));
        std::fs::create_dir(&directory).unwrap();
        let mut root = catalog();
        root.resize(180, 0);
        root[28..32].copy_from_slice(&112u32.to_le_bytes());
        root[112..116].copy_from_slice(&2u32.to_le_bytes());
        root[116..122].copy_from_slice(b"child\0");
        root[148..154].copy_from_slice(b"child\0");
        std::fs::write(directory.join("Root.bin"), &root).unwrap();
        std::fs::write(directory.join("Child.bin"), catalog()).unwrap();
        let (plan, entries) =
            WorldClassRegistry::load_with_entry_order(&directory, "root.bin").unwrap();
        assert_eq!(
            entries
                .iter()
                .map(|p| p.file_name().unwrap().to_str().unwrap())
                .collect::<Vec<_>>(),
            ["Root.bin", "Child.bin", "Child.bin"]
        );
        assert_eq!(
            plan.visits
                .iter()
                .map(|p| p.file_name().unwrap().to_str().unwrap())
                .collect::<Vec<_>>(),
            ["Child.bin", "Child.bin", "Root.bin"]
        );
        assert_eq!(plan.classes.definitions[&123].len(), 3);
        assert_eq!(
            plan.first_registered(123)
                .unwrap()
                .catalog
                .file_name()
                .unwrap(),
            "Child.bin"
        );
        assert!(plan.first_registered(999).is_none());
        assert!(WorldClassRegistry::load(&directory, "../Root.bin").is_err());
        assert!(WorldClassRegistry::load(&directory, "missing.bin").is_err());
        root[116..122].copy_from_slice(b"Root\0\0");
        std::fs::write(directory.join("Root.bin"), &root).unwrap();
        assert!(WorldClassRegistry::load(&directory, "Root.bin").is_err());
        std::fs::remove_file(directory.join("Root.bin")).unwrap();
        std::fs::remove_file(directory.join("Child.bin")).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }
    #[test]
    fn declarations_resolve_without_requested_class_ids_and_preserve_conflicts() {
        let mut registry = ClassRegistry::default();
        let data = catalog();
        registry.register(Path::new("first.bin"), &data);
        let Resolution::Unique(definition) = registry.resolve(123) else {
            panic!("unique definition missing")
        };
        assert_eq!(
            definition.models.as_ref().unwrap()[0].name.as_deref(),
            Some("mesh0")
        );
        assert!(matches!(registry.resolve(124), Resolution::Missing));
        registry.register(Path::new("second.bin"), &data);
        assert!(matches!(registry.resolve(123),Resolution::Ambiguous(rows) if rows.len()==2));
    }
    #[test]
    fn unsupported_models_remain_declared_and_bad_tables_are_reported() {
        let mut registry = ClassRegistry::default();
        let mut data = catalog();
        data[80] = 0x52;
        registry.register(Path::new("unsupported.bin"), &data);
        assert!(matches!(registry.resolve(123),Resolution::Unique(d) if d.models.is_err()));
        data[32..36].copy_from_slice(&1000u32.to_le_bytes());
        registry.register(Path::new("truncated.bin"), &data);
        assert_eq!(registry.issues.len(), 1);
    }
    #[test]
    fn bin_selection_uses_basename_order_language_and_rejects_unscoped_tiers() {
        let mut index = ResourceIndex::default();
        let rows = vec![
            Location {
                archive: "Bin/z.gzp".into(),
                member: "wall.tga".into(),
            },
            Location {
                archive: "Bin/A.gzp".into(),
                member: "WALL.TGA".into(),
            },
        ];
        index.members.insert("wall.tga".into(), rows);
        assert_eq!(
            index.select_bin("Wall.Tga", "English").unwrap().archive,
            Path::new("Bin/A.gzp")
        );
        index.members.get_mut("wall.tga").unwrap().reverse();
        assert_eq!(
            index.select_bin("wall.tga", "English").unwrap().archive,
            Path::new("Bin/A.gzp")
        );
        index.members.get_mut("wall.tga").unwrap()[0].archive = "Other/A.gzp".into();
        assert!(index.select_bin("wall.tga", "English").is_err());
        index.members.insert(
            "voice.wav".into(),
            vec![
                Location {
                    archive: "Bin/VO_SfxEnglish.gzp".into(),
                    member: "voice.wav".into(),
                },
                Location {
                    archive: "Bin/VO_SfxFrench.gzp".into(),
                    member: "voice.wav".into(),
                },
            ],
        );
        assert_eq!(
            index.select_bin("voice.wav", "French").unwrap().archive,
            Path::new("Bin/VO_SfxFrench.gzp")
        );
        assert!(index.select_bin("voice.wav", "Japanese").is_err());
        assert!(index.select_bin("voice.wav", "../French").is_err());
    }
    #[test]
    fn member_lookup_is_case_insensitive_and_never_chooses_archive_order() {
        let mut index = ResourceIndex::default();
        index.members.insert(
            "wall.tga".into(),
            vec![Location {
                archive: "a.gzp".into(),
                member: "Wall.TGA".into(),
            }],
        );
        assert!(matches!(index.resolve("WALL.tga"), Resolution::Unique(_)));
        index.members.get_mut("wall.tga").unwrap().push(Location {
            archive: "b.gzp".into(),
            member: "wall.tga".into(),
        });
        assert!(matches!(index.resolve("WALL.tga"),Resolution::Ambiguous(rows) if rows.len()==2));
        assert!(matches!(index.resolve("absent.tga"), Resolution::Missing));
    }
}
