//! Localized voice lookup for the measured basename-indexed stream layout.
//! Ambiguous entries are refused: native filesystem enumeration precedence is
//! not established for duplicate basenames within a priority group.
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Default)]
pub struct Voices {
    localized: BTreeMap<String, Vec<PathBuf>>,
    shared: BTreeMap<String, Vec<PathBuf>>,
    language: String,
}
fn scan(root: &Path) -> std::io::Result<BTreeMap<String, Vec<PathBuf>>> {
    let mut result: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
    if !root.try_exists()? {
        return Ok(result);
    }
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() || entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        for file in std::fs::read_dir(entry.path())? {
            let file = file?;
            if !file.file_type()?.is_file() {
                continue;
            }
            let name = file.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            if name.starts_with('.') || !name.to_ascii_lowercase().ends_with(".wav") {
                continue;
            }
            result
                .entry(name.to_ascii_lowercase())
                .or_default()
                .push(file.path());
        }
    }
    Ok(result)
}
impl Voices {
    pub fn load(root: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let languages = Self::languages(root)?;
        // Preserve this runtime's established English default when installed.
        // A single other locale supports installations such as the debug copy.
        // This is a runtime policy, not inferred native enumeration precedence.
        if languages
            .iter()
            .any(|(name, _)| name.eq_ignore_ascii_case("English"))
        {
            return Self::load_language(root, "English");
        }
        if languages.len() > 1 {
            return Err("Multiple installed voice languages require an explicit selection".into());
        }
        if let Some((language, _)) = languages.first() {
            Self::load_language(root, language)
        } else {
            Ok(Self {
                localized: BTreeMap::new(),
                shared: scan(&root.join("Stream"))?,
                language: "Shared".into(),
            })
        }
    }
    fn languages(root: &Path) -> std::io::Result<Vec<(String, PathBuf)>> {
        let mut result = Vec::new();
        for entry in std::fs::read_dir(root)? {
            let entry = entry?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            if entry.file_type()?.is_dir()
                && name.len() > 6
                && name
                    .get(..6)
                    .is_some_and(|s| s.eq_ignore_ascii_case("Stream"))
                && name[6..].bytes().all(|c| c.is_ascii_alphabetic())
            {
                result.push((name[6..].to_owned(), entry.path()));
            }
        }
        result.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(result)
    }
    pub fn load_language(root: &Path, language: &str) -> Result<Self, Box<dyn std::error::Error>> {
        if language.is_empty() || !language.bytes().all(|c| c.is_ascii_alphabetic()) {
            return Err("Invalid voice language".into());
        }
        let languages = Self::languages(root)?;
        let candidates: Vec<_> = languages
            .iter()
            .filter(|(name, _)| name.eq_ignore_ascii_case(language))
            .collect();
        if candidates.len() != 1 {
            return Err("Voice language is missing or ambiguous".into());
        }
        let selected = candidates[0];
        Ok(Self {
            localized: scan(&selected.1)?,
            shared: scan(&root.join("Stream"))?,
            language: selected.0.clone(),
        })
    }
    pub fn language(&self) -> &str {
        &self.language
    }
    pub fn resolve(&self, name: &str) -> Result<&Path, Box<dyn std::error::Error>> {
        if name.is_empty()
            || name.len() > 100
            || !name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
        {
            return Err("Invalid voice identifier".into());
        }
        let key = format!("{}.wav", name.to_ascii_lowercase());
        let candidates = self
            .localized
            .get(&key)
            .or_else(|| self.shared.get(&key))
            .ok_or_else(|| format!("Voice resource not indexed: {key}"))?;
        if candidates.len() != 1 {
            return Err(format!(
                "Ambiguous voice resource: {key} ({} candidates)",
                candidates.len()
            )
            .into());
        }
        Ok(&candidates[0])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn installed_locale_is_selected_without_cross_language_fallback() {
        let id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base = std::env::temp_dir().canonicalize().unwrap();
        let root = base.join(format!("giants-stream-{}-{id}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        std::fs::create_dir_all(root.join("streamFrench/BAZ")).unwrap();
        std::fs::create_dir_all(root.join("Stream/Story1")).unwrap();
        std::fs::write(root.join("streamFrench/BAZ/BAZ003.wav"), []).unwrap();
        std::fs::write(root.join("Stream/Story1/F_Start.wav"), []).unwrap();
        let voices = Voices::load(&root).unwrap();
        assert_eq!(voices.language(), "French");
        assert_eq!(
            voices.resolve("baz003").unwrap(),
            root.join("streamFrench/BAZ/BAZ003.wav")
        );
        assert_eq!(
            voices.resolve("f_start").unwrap(),
            root.join("Stream/Story1/F_Start.wav")
        );
        std::fs::create_dir(root.join("StreamEnglish")).unwrap();
        let english = Voices::load(&root).unwrap();
        assert_eq!(english.language(), "English");
        assert!(english.resolve("baz003").is_err());
        assert_eq!(
            Voices::load_language(&root, "french").unwrap().language(),
            "French"
        );
        assert!(Voices::load_language(&root, "../French").is_err());
        std::fs::remove_dir(root.join("StreamEnglish")).unwrap();
        std::fs::create_dir(root.join("StreamGerman")).unwrap();
        assert!(Voices::load(&root).is_err());
        let checked = root.canonicalize().unwrap();
        assert!(checked.starts_with(&base) && checked == root);
        std::fs::remove_dir_all(checked).unwrap();
    }
    #[test]
    fn language_precedes_shared_and_duplicates_are_not_guessed() {
        let mut voices = Voices::default();
        voices
            .shared
            .insert("line.wav".into(), vec!["shared.wav".into()]);
        assert_eq!(voices.resolve("LINE").unwrap(), Path::new("shared.wav"));
        voices
            .localized
            .insert("line.wav".into(), vec!["english.wav".into()]);
        assert_eq!(voices.resolve("line").unwrap(), Path::new("english.wav"));
        voices
            .localized
            .get_mut("line.wav")
            .unwrap()
            .push("other.wav".into());
        assert!(voices.resolve("line").is_err());
        assert!(voices.resolve("missing").is_err());
    }
    #[test]
    fn names_never_escape_the_index() {
        let voices = Voices::default();
        for name in ["", "../line", "a/b", "a\\b", "x.wav", "a:b"] {
            assert_eq!(
                voices.resolve(name).unwrap_err().to_string(),
                "Invalid voice identifier"
            );
        }
    }
}
