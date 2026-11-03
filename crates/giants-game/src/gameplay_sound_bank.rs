//! Sound preparation from the caller's actual ordered catalog visits and the
//! common Bin archive resolver. Descriptors are retained without guessed units.
use crate::catalog_registry::{Location, ResourceIndex};
use giants_formats::{sound_catalog, wav};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

pub struct Sound {
    pub binding: sound_catalog::Binding,
    pub location: Location,
    pub wave: Vec<u8>,
    pub pcm: wav::Pcm16,
}

pub struct Bank {
    pub sounds: BTreeMap<u32, Sound>,
}
/// Explicit new-runtime policy for installs containing only one voice locale.
/// This does not alter the native-compatible strict per-file archive filter.
pub fn voice_language(index: &ResourceIndex, requested: &str) -> Result<(String, bool), String> {
    if requested.is_empty() || !requested.bytes().all(|b| b.is_ascii_alphabetic()) {
        return Err("invalid voice language".into());
    }
    let mut available = BTreeSet::new();
    for location in index.members.values().flatten() {
        let Some(stem) = location.archive.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let lower = stem.to_ascii_lowercase();
        if let Some(tail) = lower.strip_prefix("vo_sfx") {
            let locale = tail
                .chars()
                .take_while(char::is_ascii_alphabetic)
                .collect::<String>();
            if !locale.is_empty() {
                available.insert(locale);
            }
        }
    }
    if available.is_empty() || available.contains(&requested.to_ascii_lowercase()) {
        return Ok((requested.into(), false));
    }
    if available.len() == 1 {
        return Ok((available.into_iter().next().unwrap(), true));
    }
    Err(
        "requested voice language absent; multiple installed voice languages require a selection"
            .into(),
    )
}

/// Metadata from the actual ordered mission catalogs. WAV payloads stay lazy.
/// A missing ID is distinct from a declared sound whose payload cannot load.
pub struct Registry {
    bindings: BTreeMap<u32, sound_catalog::Binding>,
}
impl Registry {
    pub fn prepare(catalogs: &[PathBuf], alternate: bool) -> Result<Self, String> {
        let mut definitions = Vec::new();
        for path in catalogs {
            let raw = std::fs::read(path).map_err(|e| e.to_string())?;
            definitions.extend(sound_catalog::parse_catalog(&raw).map_err(|e| e.to_string())?);
        }
        Ok(Self {
            bindings: sound_catalog::prepare_fresh(&definitions, alternate),
        })
    }
    pub fn resolve(
        &self,
        archives: &ResourceIndex,
        language: &str,
        id: u32,
    ) -> Result<Option<Sound>, String> {
        if id >= 2000 {
            return Err("sound ID outside native registry".into());
        }
        let Some(binding) = self.bindings.get(&id) else {
            return Ok(None);
        };
        let location = archives.select_bin(&binding.filename, language)?.clone();
        let wave = archives
            .read_bin(&binding.filename, language)
            .map_err(|e| e.to_string())?;
        let pcm = wav::parse(&wave).map_err(|e| format!("sound {id}: {e}"))?;
        Ok(Some(Sound {
            binding: sound_catalog::Binding {
                filename: binding.filename.clone(),
                descriptor: binding.descriptor,
            },
            location,
            wave,
            pcm,
        }))
    }
}
impl Bank {
    /// No whole-installation sound-ID merge: the caller must supply the ordered
    /// catalogs of its prepared mission. This is a fresh-bank policy; retained
    /// payloads from earlier levels are outside this constructor's contract.
    pub fn prepare(
        catalogs: &[PathBuf],
        archives: &ResourceIndex,
        language: &str,
        alternate: bool,
        requested: impl IntoIterator<Item = u32>,
    ) -> Result<Self, String> {
        let ids: BTreeSet<_> = requested.into_iter().collect();
        if ids.len() > 64 || ids.iter().any(|id| *id >= 2000) {
            return Err("gameplay sound request budget exceeded".into());
        }
        let registry = Registry::prepare(catalogs, alternate)?;
        let mut sounds = BTreeMap::new();
        for id in ids {
            let sound = registry
                .resolve(archives, language, id)?
                .ok_or_else(|| format!("sound {id} absent from loaded catalogs"))?;
            sounds.insert(id, sound);
        }
        Ok(Self { sounds })
    }
}

#[cfg(test)]
mod voice_language_tests {
    use super::*;
    #[test]
    fn absent_catalog_id_is_not_a_payload_failure_or_an_invalid_id() {
        let registry = Registry {
            bindings: BTreeMap::from([(
                123,
                sound_catalog::Binding {
                    filename: "declared.wav".into(),
                    descriptor: [1, 0, 0, 0, 0, 0],
                },
            )]),
        };
        let archives = ResourceIndex::default();
        assert!(
            registry
                .resolve(&archives, "English", 124)
                .unwrap()
                .is_none()
        );
        assert!(registry.resolve(&archives, "English", 123).is_err());
        assert!(registry.resolve(&archives, "English", 2000).is_err());
    }
    #[test]
    fn single_installed_locale_is_explicit_and_ambiguity_never_picks_an_arbitrary_bank() {
        let mut index = ResourceIndex::default();
        index.members.insert(
            "voice.wav".into(),
            vec![Location {
                archive: "Bin/VO_SfxFrench.gzp".into(),
                member: "voice.wav".into(),
            }],
        );
        assert_eq!(
            voice_language(&index, "English"),
            Ok(("french".into(), true))
        );
        assert_eq!(
            voice_language(&index, "French"),
            Ok(("French".into(), false))
        );
        index.members.get_mut("voice.wav").unwrap().push(Location {
            archive: "Bin/VO_SfxItalian.gzp".into(),
            member: "voice.wav".into(),
        });
        assert!(voice_language(&index, "English").is_err());
        assert!(voice_language(&index, "../French").is_err());
        assert!(index.select_bin("voice.wav", "English").is_err());
    }
}
