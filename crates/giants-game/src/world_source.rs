//! Explicit world selection for inspection; does not select a mission or Flick.
use std::path::Path;

#[derive(Debug, PartialEq, Eq)]
pub struct Source {
    pub name: String,
    pub catalog: String,
    pub archive: String,
}

fn requested_stem(request: &str) -> Result<String, &'static str> {
    let request = request.to_ascii_lowercase();
    let stem = request.strip_suffix(".bin").unwrap_or(&request);
    let stem = stem.strip_prefix("w_").unwrap_or(stem);
    if stem.is_empty()
        || stem.len() > 64
        || !stem.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
    {
        return Err("world selection requires an ASCII catalog identifier");
    }
    Ok(if stem == "intro" {
        "intro_island"
    } else {
        stem
    }
    .into())
}

pub fn canonical_name(request: &str) -> Result<String, &'static str> {
    let stem = requested_stem(request)?;
    Ok(if stem == "intro_island" {
        "intro".into()
    } else {
        stem
    })
}

fn select(request: &str, names: &[String]) -> Result<Source, &'static str> {
    let stem = requested_stem(request)?;
    let expected = format!("w_{stem}.bin");
    let catalogs: Vec<_> = names
        .iter()
        .filter(|n| n.eq_ignore_ascii_case(&expected))
        .collect();
    if catalogs.len() != 1 {
        return Err("world catalog absent or ambiguous");
    }
    let catalog = catalogs[0];
    let expected = format!("{}.gzp", &catalog[..catalog.len() - 4]);
    let archives: Vec<_> = names
        .iter()
        .filter(|n| n.eq_ignore_ascii_case(&expected))
        .collect();
    if archives.len() != 1 {
        return Err("world archive absent or ambiguous");
    }
    Ok(Source {
        name: canonical_name(request)?,
        catalog: catalog.clone(),
        archive: archives[0].clone(),
    })
}

pub fn resolve(root: &Path, request: &str) -> Result<Source, Box<dyn std::error::Error>> {
    requested_stem(request)?;
    let mut names = Vec::new();
    for entry in std::fs::read_dir(root.join("Bin"))? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            names.push(
                entry
                    .file_name()
                    .into_string()
                    .map_err(|_| "non-Unicode Bin filename")?,
            );
        }
    }
    Ok(select(request, &names)?)
}

/// Archive-only Bin context. The native environment reader uses its shared
/// member index rather than confining requests to the selected world's archive.
/// Loose overrides and other search tiers remain outside this adapter.
pub(crate) fn bin_resources(
    root: &Path,
) -> Result<crate::catalog_registry::ResourceIndex, Box<dyn std::error::Error>> {
    let index = crate::catalog_registry::ResourceIndex::scan(&root.join("Bin"))?;
    if !index.issues.is_empty() {
        return Err("environment archive index has unresolved reader issues".into());
    }
    Ok(index)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn aliases_preserve_actual_catalog_and_archive_identity() {
        assert_eq!(canonical_name("W_INTRO_ISLAND.BIN").unwrap(), "intro");
        assert_eq!(canonical_name("W_STORY4.BIN").unwrap(), "story4");
        let names = [
            "w_intro_island.bin",
            "w_intro_island.gzp",
            "w_Story4.bin",
            "w_Story4.gzp",
            "w_Rstory2.bin",
            "w_Rstory2.gzp",
        ]
        .map(String::from);
        assert_eq!(
            select("intro", &names).unwrap(),
            select("W_INTRO_ISLAND.BIN", &names).unwrap()
        );
        assert_eq!(select("STORY4", &names).unwrap().catalog, "w_Story4.bin");
        assert_eq!(
            select("w_Rstory2.bin", &names).unwrap().archive,
            "w_Rstory2.gzp"
        );
    }
    #[test]
    fn unsafe_absent_and_ambiguous_selections_are_refused() {
        for request in ["", "../story4", "C:\\story4", "a/b", "story4.gzp", "é"] {
            assert!(requested_stem(request).is_err(), "{request}");
        }
        assert!(select("story4", &[]).is_err());
        let names = ["w_story4.bin", "w_Story4.bin", "w_story4.gzp"].map(String::from);
        assert!(select("story4", &names).is_err());
        assert!(select("story4", &["w_story4.bin".into()]).is_err());
    }
}
