//! Independent profile-name storage. No original registry or save format is written.
use crate::{
    audio_options::AudioOptions, control_options::ControlOptions, graphics_options::GraphicsOptions,
};
use std::{
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug)]
pub struct Profiles {
    pub names: Vec<String>,
    pub active: usize,
    pub audio: Vec<AudioOptions>,
    pub graphics: Vec<GraphicsOptions>,
    pub controls: Vec<ControlOptions>,
    file: PathBuf,
}
impl Profiles {
    /// Dedicated preview identity; never replace another profile to make room.
    pub fn preview_profile(&mut self) -> Result<Option<String>, Box<dyn std::error::Error>> {
        const NAME: &str = "Preview All";
        if let Some(index) = self
            .names
            .iter()
            .position(|name| name.eq_ignore_ascii_case(NAME))
        {
            self.select(index)?;
        } else if self.names.len() < 5 {
            self.create(NAME)?;
        } else {
            return Ok(None);
        }
        Ok(Some(self.names[self.active].clone()))
    }
    pub fn open(root: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        std::fs::create_dir_all(root)?;
        let file = root.join("frontend-profiles.dat");
        let bytes = match std::fs::read(&file) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self {
                    names: Vec::new(),
                    active: 0,
                    audio: Vec::new(),
                    graphics: Vec::new(),
                    controls: Vec::new(),
                    file,
                });
            }
            Err(e) => return Err(e.into()),
        };
        if bytes.len() > 16384
            || !matches!(
                bytes.get(..8),
                Some(b"GPROF001" | b"GPROF002" | b"GPROF003" | b"GPROF004")
            )
        {
            return Err("Invalid independent profile file".into());
        }
        let mut at = 8usize;
        let mut word = || -> Result<usize, Box<dyn std::error::Error>> {
            let value = bytes.get(at..at + 4).ok_or("Truncated profile header")?;
            at += 4;
            Ok(u32::from_le_bytes(value.try_into()?) as usize)
        };
        let count = word()?;
        let active = word()?;
        if count > 5 || (count == 0 && active != 0) || (count > 0 && active >= count) {
            return Err("Invalid profile counts".into());
        }
        let version_two = bytes.get(..8) != Some(b"GPROF001");
        let version_three = matches!(bytes.get(..8), Some(b"GPROF003" | b"GPROF004"));
        let version_four = bytes.get(..8) == Some(b"GPROF004");
        let mut controls = Vec::new();
        let mut graphics = Vec::new();
        let mut audio = Vec::new();
        let mut names = Vec::new();
        for _ in 0..count {
            let len = *bytes.get(at).ok_or("Truncated profile name length")? as usize;
            at += 1;
            let name =
                std::str::from_utf8(bytes.get(at..at + len).ok_or("Truncated profile name")?)?;
            if validate_name(name)? != name
                || names
                    .iter()
                    .any(|old: &String| old.eq_ignore_ascii_case(name))
            {
                return Err("Invalid or duplicate stored profile name".into());
            }
            names.push(name.to_string());
            at += len;
            let gains = if version_two {
                let effects = f32::from_le_bytes(
                    bytes
                        .get(at..at + 4)
                        .ok_or("Truncated effects gain")?
                        .try_into()?,
                );
                let music = f32::from_le_bytes(
                    bytes
                        .get(at + 4..at + 8)
                        .ok_or("Truncated music gain")?
                        .try_into()?,
                );
                at += 8;
                AudioOptions { effects, music }
            } else {
                AudioOptions::default()
            };
            if !gains.valid() {
                return Err("Invalid stored audio gains".into());
            }
            audio.push(gains);
            let settings = if version_three {
                let value = GraphicsOptions(
                    bytes
                        .get(at..at + 10)
                        .ok_or("Truncated graphics settings")?
                        .try_into()?,
                );
                at += 10;
                value
            } else {
                GraphicsOptions::default()
            };
            if !settings.valid() {
                return Err("Invalid graphics settings".into());
            }
            graphics.push(settings);
            let mut bindings = ControlOptions::default();
            if version_four {
                bindings.1 = f32::from_le_bytes(
                    bytes
                        .get(at..at + 4)
                        .ok_or("Truncated mouse speed")?
                        .try_into()?,
                );
                bindings.2 = match bytes.get(at + 4) {
                    Some(0) => false,
                    Some(1) => true,
                    _ => return Err("Invalid mouse inversion".into()),
                };
                at += 5;
                for (unit, map) in bindings.0.iter_mut().enumerate() {
                    let len = *bytes.get(at).ok_or("Truncated binding count")? as usize;
                    at += 1;
                    if len > crate::control_options::ACTION_COUNTS[unit] {
                        return Err("Invalid binding count".into());
                    }
                    for _ in 0..len {
                        let row = *bytes.get(at).ok_or("Truncated binding index")?;
                        let len = *bytes.get(at + 1).ok_or("Truncated binding length")? as usize;
                        at += 2;
                        let value = std::str::from_utf8(
                            bytes.get(at..at + len).ok_or("Truncated binding")?,
                        )?;
                        at += len;
                        if map.insert(row, value.into()).is_some() {
                            return Err("Duplicate binding index".into());
                        }
                    }
                }
            }
            if !bindings.valid() {
                return Err("Invalid control bindings".into());
            }
            controls.push(bindings);
        }
        if at != bytes.len() {
            return Err("Trailing profile data".into());
        }
        Ok(Self {
            names,
            active,
            audio,
            graphics,
            controls,
            file,
        })
    }
    pub fn create(&mut self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let name = validate_name(name)?;
        if self.names.len() >= 5 {
            return Err("Five profiles maximum".into());
        }
        if self.names.iter().any(|old| old.eq_ignore_ascii_case(name)) {
            return Err("This profile already exists".into());
        }
        let mut next = self.clone();
        next.names.push(name.into());
        next.audio.push(AudioOptions::default());
        next.graphics.push(GraphicsOptions::default());
        next.controls.push(ControlOptions::default());
        next.active = next.names.len() - 1;
        next.save()?;
        *self = next;
        Ok(())
    }
    pub fn select(&mut self, index: usize) -> Result<(), Box<dyn std::error::Error>> {
        if index >= self.names.len() {
            return Err("Profile selection outside list".into());
        }
        let mut next = self.clone();
        next.active = index;
        next.save()?;
        *self = next;
        Ok(())
    }
    /// Delete only after the frontend's confirmation. An empty list stays empty on reopen.
    pub fn delete(&mut self, index: usize) -> Result<(), Box<dyn std::error::Error>> {
        if index >= self.names.len() {
            return Err("Profile deletion outside list".into());
        }
        let mut next = self.clone();
        next.names.remove(index);
        next.audio.remove(index);
        next.graphics.remove(index);
        next.controls.remove(index);
        next.active = if next.names.is_empty() {
            0
        } else if index < self.active {
            self.active - 1
        } else {
            self.active.min(next.names.len() - 1)
        };
        next.save()?;
        *self = next;
        Ok(())
    }
    pub fn set_audio(&mut self, gains: AudioOptions) -> Result<(), Box<dyn std::error::Error>> {
        if !gains.valid() || self.active >= self.audio.len() {
            return Err("Invalid profile audio settings".into());
        }
        let mut next = self.clone();
        next.audio[next.active] = gains;
        next.save()?;
        *self = next;
        Ok(())
    }
    pub fn set_graphics(
        &mut self,
        settings: GraphicsOptions,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if !settings.valid() || self.active >= self.graphics.len() {
            return Err("Invalid profile graphics settings".into());
        }
        let mut next = self.clone();
        next.graphics[next.active] = settings;
        next.save()?;
        *self = next;
        Ok(())
    }
    pub fn set_controls(
        &mut self,
        settings: ControlOptions,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if !settings.valid() || self.active >= self.controls.len() {
            return Err("Invalid profile controls".into());
        }
        let mut next = self.clone();
        next.controls[next.active] = settings;
        next.save()?;
        *self = next;
        Ok(())
    }
    fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut bytes = b"GPROF004".to_vec();
        bytes.extend((self.names.len() as u32).to_le_bytes());
        bytes.extend((self.active as u32).to_le_bytes());
        for (index, ((name, gains), graphics)) in self
            .names
            .iter()
            .zip(&self.audio)
            .zip(&self.graphics)
            .enumerate()
        {
            bytes.push(name.len() as u8);
            bytes.extend(name.as_bytes());
            bytes.extend(gains.effects.to_le_bytes());
            bytes.extend(gains.music.to_le_bytes());
            bytes.extend(graphics.0);
            bytes.extend(self.controls[index].1.to_le_bytes());
            bytes.push(u8::from(self.controls[index].2));
            for map in &self.controls[index].0 {
                bytes.push(map.len() as u8);
                for (&row, value) in map {
                    bytes.push(row);
                    bytes.push(value.len() as u8);
                    bytes.extend(value.as_bytes());
                }
            }
        }
        let pending = self.file.with_extension("pending");
        // Refuse another writer's pending file; never truncate it.
        let mut stream = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&pending)?;
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            stream.write_all(&bytes)?;
            stream.sync_all()?;
            drop(stream);
            std::fs::rename(&pending, &self.file)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&pending);
        }
        result
    }
}
/// Current supported subset. Full native ANSI/duplicate-name behavior is not certified.
fn validate_name(name: &str) -> Result<&str, Box<dyn std::error::Error>> {
    let name = name.trim_matches(' ');
    if name.is_empty() || name.len() > 31 || !name.bytes().all(|b| (32..=126).contains(&b)) {
        return Err("Use 1 to 31 printable ASCII characters".into());
    }
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dedicated_preview_preserves_other_profiles_and_reuses_its_identity() {
        let root = folder("preview");
        let mut profiles = Profiles::open(&root).unwrap();
        profiles.create("Existing").unwrap();
        assert_eq!(
            profiles.preview_profile().unwrap().as_deref(),
            Some("Preview All")
        );
        assert_eq!(profiles.names.len(), 2);
        let mut profiles = Profiles::open(&root).unwrap();
        profiles.preview_profile().unwrap();
        assert_eq!(profiles.names, ["Existing", "Preview All"]);
        profiles.delete(1).unwrap();
        for name in ["Second", "Third", "Fourth", "Fifth"] {
            profiles.create(name).unwrap();
        }
        let previous = profiles.names.clone();
        assert!(profiles.preview_profile().unwrap().is_none());
        assert_eq!(Profiles::open(&root).unwrap().names, previous);
        cleanup(root);
    }
    fn folder(label: &str) -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "giants-profiles-{label}-{}-{nonce}",
            std::process::id()
        ))
    }
    fn cleanup(root: PathBuf) {
        let resolved = root.canonicalize().unwrap();
        let temporary = std::env::temp_dir().canonicalize().unwrap();
        assert!(resolved.starts_with(&temporary) && resolved != temporary);
        assert!(
            resolved
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("giants-profiles-")
        );
        std::fs::remove_dir_all(resolved).unwrap();
    }
    #[test]
    fn profiles_survive_reopen_and_failed_validation_does_not_change_store() {
        let root = folder("roundtrip");
        let mut profiles = Profiles::open(&root).unwrap();
        profiles.create("Player").unwrap();
        profiles.create(" pseudoPseudo ").unwrap();
        for name in ["", "   ", "PSEUDOpseudo", "non\nname", "é"] {
            assert!(profiles.create(name).is_err());
        }
        let stored = Profiles::open(&root).unwrap();
        assert_eq!(stored.names, ["Player", "pseudoPseudo"]);
        assert_eq!(stored.active, 1);
        profiles.select(0).unwrap();
        assert_eq!(Profiles::open(&root).unwrap().active, 0);
        cleanup(root);
    }
    #[test]
    fn pending_writer_and_capacity_fail_without_state_changes() {
        let root = folder("capacity");
        let mut p = Profiles::open(&root).unwrap();
        for name in ["Player", "A", "B", "C", "D"] {
            p.create(name).unwrap();
        }
        assert!(p.create("E").is_err());
        assert_eq!(p.names.len(), 5);
        let pending = root.join("frontend-profiles.pending");
        std::fs::write(&pending, b"owned by another writer").unwrap();
        assert!(p.select(0).is_err());
        assert_eq!(p.active, 4);
        assert_eq!(std::fs::read(pending).unwrap(), b"owned by another writer");
        cleanup(root);
    }
    #[test]
    fn deletion_preserves_active_identity_and_persists_empty_list() {
        let root = folder("delete");
        let mut p = Profiles::open(&root).unwrap();
        assert!(p.names.is_empty());
        for name in ["A", "B", "C"] {
            p.create(name).unwrap();
        }
        p.delete(0).unwrap();
        assert_eq!(p.names[p.active], "C");
        p.delete(1).unwrap();
        assert_eq!(p.names[p.active], "B");
        let before = std::fs::read(root.join("frontend-profiles.dat")).unwrap();
        assert!(p.delete(99).is_err());
        assert_eq!(
            std::fs::read(root.join("frontend-profiles.dat")).unwrap(),
            before
        );
        let pending = root.join("frontend-profiles.pending");
        std::fs::write(&pending, b"other writer").unwrap();
        assert!(p.delete(0).is_err());
        assert_eq!(p.names, ["B"]);
        std::fs::remove_file(pending).unwrap();
        p.delete(0).unwrap();
        let empty = Profiles::open(&root).unwrap();
        assert!(empty.names.is_empty());
        assert_eq!(empty.active, 0);
        p.create("Again").unwrap();
        assert_eq!(Profiles::open(&root).unwrap().names, ["Again"]);
        cleanup(root);
    }
    #[test]
    fn legacy_profile_migration_and_audio_isolation_are_transactional() {
        let root = folder("audio");
        std::fs::create_dir_all(&root).unwrap();
        let file = root.join("frontend-profiles.dat");
        // Handwritten v1 fixture, independent of the v2 writer.
        std::fs::write(&file, b"GPROF001\x01\0\0\0\0\0\0\0\x03Old").unwrap();
        let mut p = Profiles::open(&root).unwrap();
        assert_eq!(p.audio, [AudioOptions::default()]);
        p.set_audio(AudioOptions {
            effects: 0.25,
            music: 0.9,
        })
        .unwrap();
        p.create("New").unwrap();
        assert_eq!(p.audio[1], AudioOptions::default());
        p.select(0).unwrap();
        assert_eq!(p.audio[0].effects, 0.25);
        let before = std::fs::read(&file).unwrap();
        assert!(
            p.set_audio(AudioOptions {
                effects: f32::NAN,
                music: 0.5
            })
            .is_err()
        );
        assert_eq!(std::fs::read(&file).unwrap(), before);
        let loaded = Profiles::open(&root).unwrap();
        assert_eq!(loaded.audio, p.audio);
        p.delete(0).unwrap();
        assert_eq!(p.audio, [AudioOptions::default()]);
        // Reject corrupt floating-point payloads instead of substituting settings.
        let mut corrupt = std::fs::read(&file).unwrap();
        let offset = 17 + "New".len();
        corrupt[offset..offset + 4].copy_from_slice(&f32::INFINITY.to_le_bytes());
        std::fs::write(&file, &corrupt).unwrap();
        assert!(Profiles::open(&root).is_err());
        assert_eq!(std::fs::read(&file).unwrap(), corrupt);
        cleanup(root);
    }
    #[test]
    fn version_two_migrates_graphics_without_losing_audio_or_profile_identity() {
        let root = folder("graphics");
        std::fs::create_dir_all(&root).unwrap();
        let file = root.join("frontend-profiles.dat");
        let mut v2 = b"GPROF002\x01\0\0\0\0\0\0\0\x03Old".to_vec();
        v2.extend(0.25f32.to_le_bytes());
        v2.extend(0.5f32.to_le_bytes());
        std::fs::write(&file, v2).unwrap();
        let mut p = Profiles::open(&root).unwrap();
        assert_eq!(p.graphics, [GraphicsOptions::default()]);
        let mut graphics = GraphicsOptions::default();
        graphics.cycle(3, 1);
        graphics.adjust_distance(-1);
        p.set_graphics(graphics).unwrap();
        p.create("New").unwrap();
        assert_eq!(p.graphics[1], GraphicsOptions::default());
        let loaded = Profiles::open(&root).unwrap();
        assert_eq!(loaded.graphics[0], graphics);
        assert_eq!(loaded.audio[0].effects, 0.25);
        p.delete(0).unwrap();
        assert_eq!(p.graphics, [GraphicsOptions::default()]);
        let mut corrupt = std::fs::read(&file).unwrap();
        corrupt[17 + "New".len() + 8 + 9] = 14;
        std::fs::write(&file, &corrupt).unwrap();
        assert!(Profiles::open(&root).is_err());
        assert_eq!(std::fs::read(&file).unwrap(), corrupt);
        cleanup(root);
    }
    #[test]
    fn version_three_binding_migration_keeps_profiles_and_character_maps_separate() {
        let root = folder("controls");
        std::fs::create_dir_all(&root).unwrap();
        let file = root.join("frontend-profiles.dat");
        let mut v3 = b"GPROF003\x01\0\0\0\0\0\0\0\x03Old".to_vec();
        v3.extend(0.2f32.to_le_bytes());
        v3.extend(0.8f32.to_le_bytes());
        v3.extend(GraphicsOptions::default().0);
        std::fs::write(&file, v3).unwrap();
        let mut p = Profiles::open(&root).unwrap();
        let mut options = ControlOptions::default();
        assert!(options.bind(0, 1, "ArrowRight"));
        assert!(options.bind(2, 0, ""));
        options.1 = 0.8;
        options.2 = true;
        p.set_controls(options.clone()).unwrap();
        p.create("New").unwrap();
        assert_eq!(p.controls[1], ControlOptions::default());
        p.select(0).unwrap();
        let loaded = Profiles::open(&root).unwrap();
        assert_eq!(loaded.controls[0], options);
        assert_eq!(loaded.audio[0].music, 0.8);
        let pending = root.join("frontend-profiles.pending");
        std::fs::write(&pending, b"other writer").unwrap();
        assert!(p.set_controls(ControlOptions::default()).is_err());
        assert_eq!(p.controls[0], options);
        std::fs::remove_file(pending).unwrap();
        p.delete(0).unwrap();
        assert_eq!(p.controls, [ControlOptions::default()]);
        let mut invalid = std::fs::read(&file).unwrap();
        *invalid.last_mut().unwrap() = 255;
        std::fs::write(&file, &invalid).unwrap();
        assert!(Profiles::open(&root).is_err());
        assert_eq!(std::fs::read(&file).unwrap(), invalid);
        cleanup(root);
    }
    #[test]
    fn corrupted_store_is_not_replaced_with_defaults() {
        let root = folder("corrupt");
        std::fs::create_dir_all(&root).unwrap();
        let file = root.join("frontend-profiles.dat");
        std::fs::write(&file, b"GPROF001\x02").unwrap();
        assert!(Profiles::open(&root).is_err());
        assert_eq!(std::fs::read(file).unwrap(), b"GPROF001\x02");
        cleanup(root);
    }
}
