//! Source-derived catalog order and sample preparation, before a real backend.
//! Controls and dialog context are explicit; no device or playback is implied.
use crate::{
    catalog_registry::{Location, ResourceIndex, WorldClassRegistry},
    sound_bindings::Bank,
};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

struct Wave {
    location: Location,
    bytes: Vec<u8>,
}

pub struct Sample {
    pub filename: String,
    pub location: Location,
    pub wave: Vec<u8>,
    pub prepared: Option<crate::sound_sample::Prepared>,
}

pub struct PreparedBank {
    pub catalogs: Vec<PathBuf>,
    pub samples: BTreeMap<u32, Sample>,
}

pub fn load(
    root: &Path,
    catalog: &str,
    language: &str,
    alternate: bool,
    controls: u32,
) -> Result<PreparedBank, Box<dyn std::error::Error>> {
    let directory = root.join("Bin");
    let (_, catalogs) = WorldClassRegistry::load_with_entry_order(&directory, catalog)?;
    let index = ResourceIndex::scan(&directory)?;
    if !index.issues.is_empty() {
        return Err("Unsupported archive inputs for sound bank".into());
    }
    let mut archives = BTreeMap::new();
    let mut bank = Bank::<Wave, ()>::default();
    for path in &catalogs {
        let definitions = giants_formats::sound_catalog::parse_catalog(&std::fs::read(path)?)?;
        let mut failure = None;
        let staged = bank.stage(&definitions, alternate, |name| {
            let mut read = || -> Result<Wave, Box<dyn std::error::Error>> {
                let location = index.select_bin(name, language)?.clone();
                if !archives.contains_key(&location.archive) {
                    archives.insert(location.archive.clone(), std::fs::read(&location.archive)?);
                }
                let archive = giants_formats::gzp::Archive::parse(&archives[&location.archive])?;
                Ok(Wave {
                    bytes: archive.read(&location.member)?,
                    location,
                })
            };
            match read() {
                Ok(wave) => Ok(wave),
                Err(error) => {
                    failure = Some(format!("{name}: {error}"));
                    Err("Sound source read failed")
                }
            }
        });
        if let Err(error) = staged {
            return Err(failure.unwrap_or_else(|| error.into()).into());
        }
    }
    let mut samples = BTreeMap::new();
    for (id, slot) in bank.into_slots() {
        let payload = slot.payload.ok_or("Staged sound lacks a source payload")?;
        let prepared = crate::sound_sample::prepare(controls, slot.descriptor, &payload.data.bytes)
            .map_err(|error| format!("sound {id} {}: {error}", payload.filename))?;
        samples.insert(
            id,
            Sample {
                filename: payload.filename,
                location: payload.data.location,
                wave: payload.data.bytes,
                prepared,
            },
        );
    }
    Ok(PreparedBank { catalogs, samples })
}
