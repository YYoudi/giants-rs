//! Read-only native campaign progress and mission-menu inputs.
//! This does not execute a world loader, restore inventories or finish cinematics.
use giants_formats::Error;

pub const SAVE_VERSION: u32 = 17;
pub const SAVE_BYTES: usize = 0x23084;
pub const WORLD_SLOTS: usize = 32;
pub const MENU_LIMIT: usize = 18;

#[derive(Clone, Debug)]
pub struct Progress {
    bytes: Vec<u8>,
}
/// Exact input words of the post-load native list consumer. The contribution's
/// broader actor meaning remains unverified; this is not health or inventory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EntryContribution {
    pub object_flags: u32,
    pub word_1c4: u32,
}
impl EntryContribution {
    pub fn total(actors: impl IntoIterator<Item = Self>) -> u32 {
        actors
            .into_iter()
            .filter(|a| a.object_flags & 0x400 != 0)
            .fold(0u32, |sum, a| sum.wrapping_add(a.word_1c4))
    }
}
impl Progress {
    /// Match the native reset before any attempted read. Version lives outside
    /// mission.sav, in profile settings; it is not a file header.
    pub fn load(version: Option<u32>, file: Option<&[u8]>) -> Result<Self, Error> {
        let mut bytes = vec![0; SAVE_BYTES];
        bytes[0x4084..0x1f084].fill(0xff);
        if version == Some(SAVE_VERSION)
            && let Some(file) = file
        {
            // Native short fread leaves its reset tail; reject it here instead.
            if file.len() != SAVE_BYTES {
                return Err(Error("mission progress size mismatch"));
            }
            bytes.copy_from_slice(file);
        }
        Ok(Self { bytes })
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    /// Only the two reachability-bit stores from mission startup. This does not
    /// acknowledge the complete actor census, level score or inventory save.
    pub fn mark_reached(&mut self, world: usize, mission: usize) -> Result<(), Error> {
        if world >= MENU_LIMIT || mission >= 32 {
            return Err(Error("reached mission outside campaign menu profile"));
        }
        let at = 4 + world * 4;
        let worlds = self.world_word() | (1u32 << world);
        let missions = self.mission_word(world)? | (1u32 << mission);
        self.bytes[..4].copy_from_slice(&worlds.to_le_bytes());
        self.bytes[at..at + 4].copy_from_slice(&missions.to_le_bytes());
        Ok(())
    }
    /// Record a successfully loaded entry, not a victory. The caller must
    /// provide the COMPLETE list consumed by native startup; [] means a known
    /// empty list, never missing actors. This mutates storage only, not a save
    /// file, mission readiness or the native loading cursor.
    pub fn record_loaded(
        &mut self,
        world: usize,
        mission: usize,
        actors: &[EntryContribution],
    ) -> Result<u32, Error> {
        // Campaign menu profile. Native storage has 32 world slots, but high
        // world bits are outside the supported menu domain (see world_count).
        if world >= MENU_LIMIT || mission >= 32 {
            return Err(Error("loaded entry outside campaign progress profile"));
        }
        let total = EntryContribution::total(actors.iter().copied());
        let mission_at = 4 + world * 4;
        let census_at = 0x1084 + (world * 32 + mission) * 4;
        let worlds = self.world_word() | (1u32 << world);
        let missions = self.mission_word(world)? | (1u32 << mission);
        self.bytes[..4].copy_from_slice(&worlds.to_le_bytes());
        self.bytes[mission_at..mission_at + 4].copy_from_slice(&missions.to_le_bytes());
        self.bytes[census_at..census_at + 4].copy_from_slice(&total.to_le_bytes());
        Ok(total)
    }
    pub fn world_word(&self) -> u32 {
        word(&self.bytes, 0).unwrap()
    }
    pub fn mission_word(&self, world: usize) -> Result<u32, Error> {
        if world >= WORLD_SLOTS {
            return Err(Error("campaign world slot outside progress"));
        }
        word(&self.bytes, 4 + world * 4)
    }
    /// The menu counts up to the highest set world bit, not its population.
    /// Negative native words do not terminate the arithmetic-shift loop.
    pub fn world_count(&self, cheat: bool) -> Result<WorldAccess, Error> {
        if cheat {
            return Ok(WorldAccess::AllCatalogWorlds);
        }
        let bits = self.world_word();
        if bits == 0 {
            return Ok(WorldAccess::NoProgress);
        }
        if bits & 0x8000_0000 != 0 {
            return Err(Error(
                "negative world progress would not terminate native menu",
            ));
        }
        Ok(WorldAccess::Prefix(
            (32 - bits.leading_zeros() as usize).min(MENU_LIMIT),
        ))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorldAccess {
    NoProgress,
    Prefix(usize),
    /// Uses the native cheat catalog route, which can also expose noncampaign worlds.
    AllCatalogWorlds,
}

fn word(bytes: &[u8], at: usize) -> Result<u32, Error> {
    Ok(u32::from_le_bytes(
        bytes
            .get(at..at + 4)
            .ok_or(Error("truncated mission menu input"))?
            .try_into()
            .unwrap(),
    ))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MissionEntry {
    /// Byte supplied by the catalog; preserve it separately from the menu index.
    pub source_id: u8,
    pub name: String,
}
#[derive(Clone, Debug)]
pub struct MissionMenu {
    pub declared_count: usize,
    pub entries: Vec<MissionEntry>,
    /// Native returns one selectable row when a nonempty catalog has zero progress,
    /// even though no name record was read. Do not invent its contents.
    pub unnamed_fallback: bool,
}
impl MissionMenu {
    pub fn parse(catalog: &[u8], mission_word: u32, cheat: bool) -> Result<Self, Error> {
        if catalog.len() < 32
            || ![0x1a0002e1, 0x1a0002e2, 0x1a0002e3, 0x1a0002e5].contains(&word(catalog, 0)?)
        {
            return Err(Error("unsupported mission menu catalog"));
        }
        let at = word(catalog, 24)? as usize;
        if at < 32 {
            return Err(Error("mission menu overlaps catalog header"));
        }
        let count = word(catalog, at)? as usize;
        if count > 32 {
            return Err(Error(
                "mission menu count outside supported campaign domain",
            ));
        }
        let mut remaining = mission_word as i32;
        let mut visible = 0;
        for _ in 0..count {
            if remaining != 0 || cheat {
                visible += 1;
                if !cheat {
                    while remaining & 1 == 0 {
                        remaining >>= 1;
                    }
                    remaining >>= 1;
                }
            }
        }
        visible = visible.min(MENU_LIMIT);
        let mut entries = Vec::new();
        // Native consumes a contiguous prefix of records; holes in the progress
        // word are not used as catalog indexes.
        for i in 0..visible {
            let start = at + 4 + i * 33;
            let row = catalog
                .get(start..start + 33)
                .ok_or(Error("truncated visible mission entry"))?;
            let end = row[1..]
                .iter()
                .position(|b| *b == 0)
                .ok_or(Error("unterminated mission identifier"))?;
            let name = &row[1..1 + end];
            if name.is_empty() || !name.iter().all(|b| b.is_ascii_alphanumeric() || *b == b'_') {
                return Err(Error("unsupported mission identifier"));
            }
            entries.push(MissionEntry {
                source_id: row[0],
                name: String::from_utf8(name.to_vec()).unwrap(),
            });
        }
        Ok(Self {
            declared_count: count,
            entries,
            unnamed_fallback: count > 0 && visible == 0,
        })
    }
    pub fn selectable_count(&self) -> usize {
        if self.unnamed_fallback {
            1
        } else {
            self.entries.len()
        }
    }
    /// UI activation supplies a zero-based index. Missing fallback text remains an
    /// explicit error until its native callback has supplied a meaningful entry.
    pub fn request(&self, world: &str, index: usize) -> Result<LoadRequest, Error> {
        if world.is_empty()
            || world.len() > 31
            || !world
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_')
        {
            return Err(Error("unsupported world identifier"));
        }
        let entry = self
            .entries
            .get(index)
            .ok_or(Error("mission selection has no named entry"))?;
        Ok(LoadRequest {
            world: world.to_owned(),
            menu_index: index,
            source_id: entry.source_id,
            mission_name: entry.name.clone(),
            splash: Splash::for_world(world),
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Splash {
    Mecc,
    Reaper,
    Kabuto,
    Giants,
}
impl Splash {
    pub fn for_world(world: &str) -> Self {
        match world.as_bytes().first().map(u8::to_ascii_uppercase) {
            Some(b'K') => Self::Kabuto,
            Some(b'R') => Self::Reaper,
            Some(b'I') => Self::Giants,
            _ => Self::Mecc,
        }
    }
    pub fn asset_stem(self) -> &'static str {
        match self {
            Self::Mecc => "mecc_loading",
            Self::Reaper => "reaper_loading",
            Self::Kabuto => "kabuto_loading",
            Self::Giants => "giants_loading",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadRequest {
    pub world: String,
    pub menu_index: usize,
    pub source_id: u8,
    pub mission_name: String,
    pub splash: Splash,
}

impl LoadRequest {
    /// Source-encoded loading caption. Fonts/encoding remain the renderer's job.
    /// Native world-name lookup falls back to the identifier when WN is absent.
    pub fn loading_caption(
        &self,
        text: &giants_formats::gtext::TextTable,
    ) -> Result<Vec<u8>, Error> {
        let format = text
            .get(b"WorldLoading".as_slice())
            .ok_or(Error("missing loading caption format"))?;
        let mut substitutions = format.windows(2).enumerate().filter(|(_, b)| *b == b"%s");
        let at = substitutions
            .next()
            .ok_or(Error("unsupported loading caption format"))?
            .0;
        if substitutions.next().is_some()
            || format[..at].contains(&b'%')
            || format[at + 2..].contains(&b'%')
        {
            return Err(Error("unsupported loading caption format"));
        }
        let key = format!("WN{}", self.world);
        let title = text
            .get(key.as_bytes())
            .map(Vec::as_slice)
            .unwrap_or(self.world.as_bytes());
        let mut caption = format[..at].to_vec();
        caption.extend(title);
        caption.extend(&format[at + 2..]);
        if caption.len() > 1024 {
            return Err(Error("loading caption exceeds independent budget"));
        }
        Ok(caption)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reaching_an_entry_preserves_census_inventory_and_other_worlds() {
        let bytes = (0..SAVE_BYTES)
            .map(|i| (i.wrapping_mul(37) & 255) as u8)
            .collect::<Vec<_>>();
        let mut progress = Progress::load(Some(SAVE_VERSION), Some(&bytes)).unwrap();
        progress.mark_reached(2, 5).unwrap();
        let mut expected = bytes.clone();
        let word_at = |at| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
        expected[..4].copy_from_slice(&(word_at(0) | (1 << 2)).to_le_bytes());
        expected[12..16].copy_from_slice(&(word_at(12) | (1 << 5)).to_le_bytes());
        assert_eq!(progress.bytes(), expected);
        let before = progress.bytes().to_vec();
        assert!(progress.mark_reached(MENU_LIMIT, 0).is_err());
        assert!(progress.mark_reached(0, 32).is_err());
        assert_eq!(progress.bytes(), before);
    }
    #[test]
    fn version_is_external_and_tail_reset_is_not_all_zero() {
        let empty = Progress::load(None, None).unwrap();
        assert!(empty.bytes()[..0x4084].iter().all(|b| *b == 0));
        assert!(empty.bytes()[0x4084..0x1f084].iter().all(|b| *b == 255));
        assert!(empty.bytes()[0x1f084..].iter().all(|b| *b == 0));
        assert_eq!(empty.world_count(false).unwrap(), WorldAccess::NoProgress);
        assert_eq!(
            empty.world_count(true).unwrap(),
            WorldAccess::AllCatalogWorlds
        );
        assert_eq!(
            Progress::load(Some(16), Some(&[1])).unwrap().bytes(),
            empty.bytes()
        );
        assert!(Progress::load(Some(17), Some(&[1])).is_err());
        assert!(empty.mission_word(32).is_err());
    }
    #[test]
    fn sparse_progress_produces_prefix_and_keeps_identifiers_separate() {
        let mut bytes = vec![0; SAVE_BYTES];
        bytes[..4].copy_from_slice(&8u32.to_le_bytes());
        bytes[4..8].copy_from_slice(&10u32.to_le_bytes());
        let progress = Progress::load(Some(17), Some(&bytes)).unwrap();
        assert_eq!(progress.world_count(false).unwrap(), WorldAccess::Prefix(4));
        let mut catalog = vec![0; 36 + 3 * 33];
        catalog[..4].copy_from_slice(&0x1a0002e5u32.to_le_bytes());
        catalog[24..28].copy_from_slice(&32u32.to_le_bytes());
        catalog[32..36].copy_from_slice(&3u32.to_le_bytes());
        for i in 0..3 {
            catalog[36 + i * 33] = (8 - i) as u8;
            catalog[37 + i * 33] = b'a' + i as u8;
        }
        let menu = MissionMenu::parse(&catalog, progress.mission_word(0).unwrap(), false).unwrap();
        assert_eq!(menu.entries.len(), 2);
        let request = menu.request("Story1", 1).unwrap();
        assert_eq!(
            (
                request.menu_index,
                request.source_id,
                request.mission_name.as_str()
            ),
            (1, 7, "b")
        );
        assert!(menu.request("../Story1", 0).is_err());
        assert!(menu.request("Story1", 2).is_err());
        let fallback = MissionMenu::parse(&catalog, 0, false).unwrap();
        assert_eq!(fallback.selectable_count(), 1);
        assert!(fallback.request("Story1", 0).is_err());
        assert_eq!(
            MissionMenu::parse(&catalog, 0x8000_0000, false)
                .unwrap()
                .entries
                .len(),
            3
        );
    }
    #[test]
    fn loaded_entry_updates_bits_and_filtered_wrapping_contribution_only() {
        let initial = vec![0x5a; SAVE_BYTES];
        let mut progress = Progress::load(Some(SAVE_VERSION), Some(&initial)).unwrap();
        let total = progress
            .record_loaded(
                3,
                31,
                &[
                    EntryContribution {
                        object_flags: 0x400,
                        word_1c4: u32::MAX,
                    },
                    EntryContribution {
                        object_flags: 0,
                        word_1c4: 100,
                    },
                    EntryContribution {
                        object_flags: 0x401,
                        word_1c4: 3,
                    },
                ],
            )
            .unwrap();
        assert_eq!(total, 2);
        assert_eq!(progress.world_word(), 0x5a5a5a5a | 8);
        assert_eq!(progress.mission_word(3).unwrap(), 0xda5a5a5a);
        let census = 0x1084 + (3 * 32 + 31) * 4;
        for (i, byte) in initial.iter().enumerate() {
            if !(0..4).contains(&i) && !(16..20).contains(&i) && !(census..census + 4).contains(&i)
            {
                assert_eq!(progress.bytes()[i], *byte);
            }
        }
        let before = progress.bytes().to_vec();
        for (world, mission) in [(MENU_LIMIT, 0), (0, 32), (usize::MAX, usize::MAX)] {
            assert!(progress.record_loaded(world, mission, &[]).is_err());
            assert_eq!(progress.bytes(), before);
        }
    }
    #[test]
    fn caption_keeps_source_encoding_and_rejects_other_format_directives() {
        let request = LoadRequest {
            world: "Story1".into(),
            menu_index: 0,
            source_id: 0,
            mission_name: "test".into(),
            splash: Splash::Mecc,
        };
        let mut table = giants_formats::gtext::TextTable::new();
        table.insert(b"WorldLoading".to_vec(), b"Loading %s".to_vec());
        assert_eq!(request.loading_caption(&table).unwrap(), b"Loading Story1");
        table.insert(b"WNStory1".to_vec(), b"Title\xe9".to_vec());
        assert_eq!(
            request.loading_caption(&table).unwrap(),
            b"Loading Title\xe9"
        );
        table.insert(b"WorldLoading".to_vec(), b"%s %n".to_vec());
        assert!(request.loading_caption(&table).is_err());
    }
}
