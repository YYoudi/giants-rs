//! Menu-to-loader handoff. Runtime acknowledgements, never timers, open gameplay.
use crate::mission_selection::{LoadRequest, MissionMenu, Progress, WorldAccess};
use giants_formats::{gtext::TextTable, gzp::Archive, tga::RgbImage};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LaunchIntent {
    NewCampaign,
    SavedMission,
    /// Source-declared completion film in a fresh mission inspection context.
    CompletionReplay,
}

#[derive(Clone, Debug)]
pub struct WorldEntry {
    pub id: usize,
    pub name: String,
    pub title: String,
}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 31
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
}
pub fn world_list(data: &[u8], text: &TextTable) -> Result<Vec<WorldEntry>, String> {
    let word = |at| {
        data.get(at..at + 4)
            .map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize)
            .ok_or("Truncated world list")
    };
    if word(0)? != 0x1a0003d0 || word(4)? + 8 != data.len() {
        return Err("Unsupported world list".into());
    }
    let count = word(8)?;
    if count > 256 || 12 + count * 4 > data.len() {
        return Err("World-list budget".into());
    }
    let mut result = Vec::new();
    for i in 0..count {
        let at = 8 + word(12 + i * 4)?;
        if at < 12 + count * 4 {
            return Err("World name overlaps index".into());
        }
        let tail = data.get(at..).ok_or("Invalid world offset")?;
        let n = tail
            .iter()
            .take(32)
            .position(|b| *b == 0)
            .ok_or("World identifier too long")?;
        let name = std::str::from_utf8(&tail[..n])
            .map_err(|e| e.to_string())?
            .to_owned();
        if !identifier(&name) {
            return Err("Invalid world identifier".into());
        }
        let id = word(at + n + 5)?; // NUL, source flags u32, source ID u32.
        if result
            .iter()
            .any(|w: &WorldEntry| w.id == id || w.name.eq_ignore_ascii_case(&name))
        {
            return Err("Ambiguous world identity".into());
        }
        let title = english(text, &format!("WN{name}")).unwrap_or_else(|| name.clone());
        result.push(WorldEntry { id, name, title });
    }
    Ok(result)
}
fn english(text: &TextTable, key: &str) -> Option<String> {
    text.get(key.as_bytes())
        .and_then(|b| b.is_ascii().then(|| String::from_utf8(b.clone()).unwrap()))
}

/// Profile identity, not its position in a list, owns the progress sidecar.
/// No native registry, original saves or GPROF004 bytes are changed.
pub fn progress_path(user_root: &Path, profile: &str) -> PathBuf {
    let key = profile
        .as_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    user_root
        .join("campaign-progress")
        .join(format!("{key}.sav"))
}

pub struct Campaign {
    root: PathBuf,
    user_root: PathBuf,
    pub text: TextTable,
    pub worlds: Vec<WorldEntry>,
    pub menu: Option<MissionMenu>,
    pub world: Option<WorldEntry>,
    pub progress: Progress,
    showcase: bool,
}
impl Campaign {
    pub fn open(root: &Path, user_root: &Path) -> Result<Self, String> {
        for i in 2..=5 {
            if std::fs::read_dir(root.join("Bin"))
                .map_err(|e| e.to_string())?
                .filter_map(Result::ok)
                .any(|e| {
                    e.file_name()
                        .to_str()
                        .is_some_and(|n| n.eq_ignore_ascii_case(&format!("WorldList{i}.bin")))
                })
            {
                return Err("Additional world-list catalogs need an explicit merge adapter".into());
            }
        }
        let text = giants_formats::gtext::parse(
            &std::fs::read(root.join("Bin/GTextEnglish.bin")).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        let worlds = world_list(
            &std::fs::read(root.join("Bin/WorldList.bin")).map_err(|e| e.to_string())?,
            &text,
        )?;
        Ok(Self {
            root: root.into(),
            user_root: user_root.into(),
            text,
            worlds,
            menu: None,
            world: None,
            progress: Progress::load(None, None).unwrap(),
            showcase: false,
        })
    }
    pub fn select_profile(&mut self, name: &str) -> Result<(), String> {
        let path = progress_path(&self.user_root, name);
        let bytes = match std::fs::read(&path) {
            Ok(b) => Some(b),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.to_string()),
        };
        self.progress = Progress::load(bytes.as_ref().map(|_| 17), bytes.as_deref())
            .map_err(|e| e.to_string())?;
        self.showcase = match std::fs::read(path.with_extension("showcase")) {
            Ok(bytes) if bytes == b"INDEPENDENT-SHOWCASE-1\n" => true,
            Ok(_) => return Err("Invalid independent showcase marker".into()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
            Err(e) => return Err(e.to_string()),
        };
        self.world = None;
        self.menu = None;
        Ok(())
    }
    pub fn available_worlds(&self) -> Result<Vec<&WorldEntry>, String> {
        let count = match self
            .progress
            .world_count(self.showcase)
            .map_err(|e| e.to_string())?
        {
            WorldAccess::NoProgress => 0,
            WorldAccess::Prefix(n) => n,
            WorldAccess::AllCatalogWorlds => {
                return Ok(self
                    .worlds
                    .iter()
                    .filter(|w| self.progress.mission_word(w.id).is_ok_and(|v| v != 0))
                    .collect());
            }
        };
        Ok(self.worlds.iter().filter(|w| w.id < count).collect())
    }
    pub fn select_world(&mut self, index: usize) -> Result<(), String> {
        let world = self
            .available_worlds()?
            .get(index)
            .copied()
            .ok_or("No accessible world at index")?
            .clone();
        let menu = MissionMenu::parse(
            &self.catalog(&world.name)?,
            self.progress
                .mission_word(world.id)
                .map_err(|e| e.to_string())?,
            self.showcase,
        )
        .map_err(|e| e.to_string())?;
        self.world = Some(world);
        self.menu = Some(menu);
        Ok(())
    }
    pub fn request(&self, index: usize) -> Result<LoadRequest, String> {
        self.menu
            .as_ref()
            .ok_or("Mission menu missing")?
            .request(&self.world.as_ref().ok_or("World missing")?.name, index)
            .map_err(|e| e.to_string())
    }
    /// Menu choices retain the original mission index and context. Only exact
    /// source-declared completion names are accepted; no startup aliases or
    /// fallback mission slots are invented for replay.
    pub fn completion_replays(&self) -> Result<Vec<LoadRequest>, String> {
        let world = self.world.as_ref().ok_or("World missing")?;
        let decoded = giants_formats::world::parse_world(&self.catalog(&world.name)?)
            .map_err(|e| e.to_string())?;
        let menu = self.menu.as_ref().ok_or("Mission menu missing")?;
        let mut requests = Vec::new();
        for index in 0..menu.entries.len() {
            let request = self.request(index)?;
            let mission = giants_formats::mission::parse(&read_member(
                &self.root,
                &format!("wm_{}.bin", request.mission_name),
            )?)
            .map_err(|e| e.to_string())?;
            if crate::mission_cinematics::select_completion(
                &decoded,
                &mission,
                &request.mission_name,
            )?
            .director
            .is_some()
            {
                requests.push(request);
            }
        }
        Ok(requests)
    }
    /// New campaign requests the first source mission without overwriting progress.
    pub fn new_campaign(&self) -> Result<LoadRequest, String> {
        let first = self
            .worlds
            .iter()
            .find(|w| w.id == 0)
            .ok_or("Initial world missing")?;
        MissionMenu::parse(&self.catalog(&first.name)?, 1, false)
            .map_err(|e| e.to_string())?
            .request(&first.name, 0)
            .map_err(|e| e.to_string())
    }
    fn catalog(&self, name: &str) -> Result<Vec<u8>, String> {
        read_member(&self.root, &format!("w_{name}.bin"))
    }
    pub fn mission_title(&self, name: &str) -> String {
        // Do not invent a translation key: untranslated identifiers remain visible.
        english(&self.text, name).unwrap_or_else(|| name.to_owned())
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    /// Persist only actual reached-entry bits in the independent profile sidecar.
    /// The caller must have observed that entry's real runtime admission. Census,
    /// native registry, original saves and inventory snapshots are not fabricated.
    pub fn record_reached(&mut self, profile: &str, request: &LoadRequest) -> Result<(), String> {
        if profile.is_empty() || !profile.is_ascii() {
            return Err("invalid campaign profile".into());
        }
        let world = self
            .worlds
            .iter()
            .find(|w| w.name.eq_ignore_ascii_case(&request.world))
            .ok_or("reached world absent")?;
        let menu =
            MissionMenu::parse(&self.catalog(&world.name)?, 0, true).map_err(|e| e.to_string())?;
        let expected = menu
            .request(&world.name, request.menu_index)
            .map_err(|e| e.to_string())?;
        if expected != *request {
            return Err("reached request differs from source catalog".into());
        }
        let mut progress = self.progress.clone();
        progress
            .mark_reached(world.id, usize::from(request.source_id))
            .map_err(|e| e.to_string())?;
        let path = progress_path(&self.user_root, profile);
        std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
        let temporary = path.with_extension("reached-tmp");
        std::fs::write(&temporary, progress.bytes()).map_err(|e| e.to_string())?;
        std::fs::rename(&temporary, &path).map_err(|e| e.to_string())?;
        self.progress = progress;
        Ok(())
    }
    /// Independent showcase progress only. Does not restore a native game,
    /// inventory, mission completion or playable capability.
    pub fn unlock_profile(&mut self, profile: &str) -> Result<usize, String> {
        if profile.is_empty() || !profile.is_ascii() {
            return Err("Invalid showcase profile identity".into());
        }
        let mut bytes = Progress::load(None, None).unwrap().bytes().to_vec();
        let mut worlds = 0u32;
        let mut missions = 0;
        for world in &self.worlds {
            let menu = MissionMenu::parse(&self.catalog(&world.name)?, 0, true)
                .map_err(|e| e.to_string())?;
            // Empty multiplayer catalogs are not single-player level entries.
            if menu.declared_count == 0 {
                continue;
            }
            if world.id >= 31 {
                return Err("Showcase world exceeds progress storage".into());
            }
            if menu.declared_count > crate::mission_selection::MENU_LIMIT {
                return Err("Showcase mission count exceeds the campaign menu budget".into());
            }
            worlds |= 1 << world.id;
            let bits = (1u32 << menu.declared_count) - 1;
            bytes[4 + world.id * 4..8 + world.id * 4].copy_from_slice(&bits.to_le_bytes());
            missions += menu.entries.len();
        }
        bytes[..4].copy_from_slice(&worlds.to_le_bytes());
        let path = progress_path(&self.user_root, profile);
        std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
        let temporary = path.with_extension("showcase-tmp");
        std::fs::write(&temporary, bytes).map_err(|e| e.to_string())?;
        std::fs::rename(temporary, &path).map_err(|e| e.to_string())?;
        std::fs::write(path.with_extension("showcase"), b"INDEPENDENT-SHOWCASE-1\n")
            .map_err(|e| e.to_string())?;
        self.select_profile(profile)?;
        Ok(missions)
    }
}

fn read_member(root: &Path, name: &str) -> Result<Vec<u8>, String> {
    // Exact case-insensitive basename resolution, ambiguity is an error.
    let matches = std::fs::read_dir(root.join("Bin"))
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .filter(|e| {
            e.file_name()
                .to_str()
                .is_some_and(|n| n.eq_ignore_ascii_case(name))
        })
        .map(|e| e.path())
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err(format!("Missing or ambiguous resource {name}"));
    }
    std::fs::read(&matches[0]).map_err(|e| e.to_string())
}

pub fn splash(root: &Path, request: &LoadRequest) -> Result<RgbImage, String> {
    let raw = read_member(root, "xx_giants_logo_3d.gzp")?;
    let archive = Archive::parse(&raw).map_err(|e| e.to_string())?;
    let bytes = archive
        .read(&format!("{}.tga", request.splash.asset_stem()))
        .map_err(|e| e.to_string())?;
    let image = giants_formats::tga::decode(&bytes).map_err(|e| e.to_string())?;
    if (image.width, image.height) != (640, 480) {
        return Err("Unsupported loading image size".into());
    }
    Ok(image)
}

pub struct PreparedMission {
    pub request: LoadRequest,
    pub world: Option<giants_formats::world::WorldConfig>,
    pub mission: Option<giants_formats::mission::Mission>,
    pub scenario_index: Option<usize>,
    pub terrain: Option<giants_formats::gti::Terrain>,
    pub terrain_bytes: Vec<u8>,
    pub world_bytes: Vec<u8>,
    pub mission_bytes: Vec<u8>,
    pub needs: Vec<String>,
}
impl PreparedMission {
    pub fn player_start_candidate(
        &self,
        root: &Path,
    ) -> Result<crate::player_start::Declared, String> {
        let plan = self.instance_plan(root, "English")?;
        crate::player_start::declared(
            &plan,
            self.world.as_ref().ok_or("prepared world missing")?,
            self.mission.as_ref().ok_or("prepared mission missing")?,
        )
    }
    pub fn slot_parameters(&self, root: &Path) -> Result<crate::mission_slot::Projection, String> {
        self.validate_inputs(root)?;
        crate::mission_slot::project(
            self.world.as_ref().ok_or("prepared world missing")?,
            self.mission.as_ref().ok_or("prepared mission missing")?,
            &self.request.mission_name,
        )
    }
    pub fn opening(&self, root: &Path) -> Result<crate::mission_cinematics::Opening, String> {
        self.validate_inputs(root)?;
        crate::mission_cinematics::select(
            self.world.as_ref().ok_or("prepared world missing")?,
            self.mission.as_ref().ok_or("prepared mission missing")?,
            &self.request.mission_name,
        )
    }
    pub fn completion(&self, root: &Path) -> Result<crate::mission_cinematics::Opening, String> {
        self.validate_inputs(root)?;
        crate::mission_cinematics::select_completion(
            self.world.as_ref().ok_or("prepared world missing")?,
            self.mission.as_ref().ok_or("prepared mission missing")?,
            &self.request.mission_name,
        )
    }
    fn validate_inputs(&self, root: &Path) -> Result<(), String> {
        if !self.needs.is_empty() {
            return Err("prepared mission still has unresolved reader needs".into());
        }
        if !identifier(&self.request.world) || !identifier(&self.request.mission_name) {
            return Err("invalid prepared mission identity".into());
        }
        if read_member(root, &format!("w_{}.bin", self.request.world))? != self.world_bytes
            || read_member(root, &format!("wm_{}.bin", self.request.mission_name))?
                != self.mission_bytes
        {
            return Err("prepared mission inputs changed before instance planning".into());
        }
        Ok(())
    }
    /// Environment assets for this exact selected packet. Does not choose a
    /// camera, run the mission or authorize a playable transition.
    pub fn world_asset(&self, root: &Path) -> Result<crate::WorldAsset, String> {
        self.validate_inputs(root)?;
        let config =
            giants_formats::world::parse_world(&self.world_bytes).map_err(|e| e.to_string())?;
        crate::world_source::resolve(root, &self.request.world).map_err(|e| e.to_string())?;
        let resources = crate::world_source::bin_resources(root).map_err(|e| e.to_string())?;
        if resources
            .read_bin(&config.prefix.terrain, "English")
            .map_err(|e| e.to_string())?
            != self.terrain_bytes
        {
            return Err("prepared terrain bytes changed before environment preparation".into());
        }
        crate::build_world(
            root,
            &self.request.world,
            format!("w_{}.bin", self.request.world),
            config,
            &resources,
        )
        .map_err(|e| e.to_string())
    }
    /// CPU declarations/state projection for a future scene consumer. This does
    /// not create actors, run callbacks, or acknowledge runtime readiness.
    pub fn instance_plan(
        &self,
        root: &Path,
        language: &str,
    ) -> Result<crate::instances::Plan, String> {
        self.validate_inputs(root)?;
        let world_catalog = format!("w_{}.bin", self.request.world);
        let mission = self
            .mission
            .as_ref()
            .ok_or("prepared mission has no decoded script")?;
        let resources = std::sync::Arc::new(
            crate::scene_resources::SceneResources::load(root, &world_catalog, language)
                .map_err(|e| e.to_string())?,
        );
        let world = self
            .world
            .as_ref()
            .ok_or("prepared mission has no decoded world")?;
        Ok(crate::instances::Plan::prepare_scene(
            &resources,
            mission,
            &world.placements,
        ))
    }
}
pub fn prepare(root: &Path, request: LoadRequest) -> Result<PreparedMission, String> {
    if !identifier(&request.world) || !identifier(&request.mission_name) {
        return Err("Invalid load request".into());
    }
    let world_bytes = read_member(root, &format!("w_{}.bin", request.world))?;
    let mission_bytes = read_member(root, &format!("wm_{}.bin", request.mission_name))?;
    let mut needs = Vec::new();
    let world = match giants_formats::world::parse_world(&world_bytes) {
        Ok(w) => Some(w),
        Err(e) => {
            needs.push(format!("world command profile: {e}"));
            None
        }
    };
    let mission = match giants_formats::mission::parse(&mission_bytes) {
        Ok(m) => Some(m),
        Err(e) => {
            needs.push(format!("mission command profile: {e}"));
            None
        }
    };
    let matches = world
        .as_ref()
        .map(|w| {
            w.scenarios
                .iter()
                .enumerate()
                .filter(|(_, s)| s.name.eq_ignore_ascii_case(&request.mission_name))
                .map(|(i, _)| i)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let scenario_index = if matches.len() == 1 {
        Some(matches[0])
    } else {
        needs.push("mission-to-scenario identity unresolved".into());
        None
    };
    let terrain_name = terrain_name(&world_bytes)?;
    crate::world_source::resolve(root, &request.world).map_err(|e| e.to_string())?;
    let resources = crate::world_source::bin_resources(root).map_err(|e| e.to_string())?;
    let terrain_bytes = resources
        .read_bin(&terrain_name, "English")
        .map_err(|e| e.to_string())?;
    let terrain = match giants_formats::gti::parse(&terrain_bytes) {
        Ok(t) => Some(t),
        Err(e) => {
            needs.push(format!("terrain profile: {e}"));
            None
        }
    };
    Ok(PreparedMission {
        request,
        world,
        mission,
        scenario_index,
        terrain,
        terrain_bytes,
        world_bytes,
        mission_bytes,
        needs,
    })
}

/// The two initial NUL-terminated names precede material commands. Reading them
/// does not assert support for the remaining environment/gameplay stream.
fn terrain_name(bytes: &[u8]) -> Result<String, String> {
    let word = |at| {
        bytes
            .get(at..at + 4)
            .map(|v| u32::from_le_bytes(v.try_into().unwrap()) as usize)
            .ok_or("Truncated world header")
    };
    if ![0x1a0002e1, 0x1a0002e2, 0x1a0002e3, 0x1a0002e5].contains(&word(0)?) {
        return Err("Unsupported world header".into());
    }
    let start = word(4)?;
    if start < 32 {
        return Err("World block overlaps header".into());
    }
    let size = word(start)?;
    if size > 16 * 1024 * 1024 {
        return Err("World block budget".into());
    }
    let block = bytes
        .get(start + 4..start + 4 + size)
        .ok_or("Truncated world block")?;
    let first = block
        .iter()
        .take(128)
        .position(|v| *v == 0)
        .ok_or("Invalid box name")?;
    let tail = &block[first + 1..];
    let end = tail
        .iter()
        .take(128)
        .position(|v| *v == 0)
        .ok_or("Invalid terrain name")?;
    let name = std::str::from_utf8(&tail[..end]).map_err(|e| e.to_string())?;
    if !name.ends_with(".gti")
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.'))
    {
        return Err("Invalid terrain member name".into());
    }
    Ok(name.to_owned())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Menu,
    Splash,
    Loading,
    AwaitingRuntime,
    Cinematic,
    Objectives,
    Gameplay,
    Failed,
}
#[derive(Debug)]
pub struct Flow {
    pub phase: Phase,
    pub ticket: u64,
    draws: u8,
    gameplay_ready: bool,
    pub error: Option<String>,
}
impl Default for Flow {
    fn default() -> Self {
        Self {
            phase: Phase::Menu,
            ticket: 0,
            draws: 0,
            gameplay_ready: false,
            error: None,
        }
    }
}
impl Flow {
    pub fn begin(&mut self) -> u64 {
        self.ticket += 1;
        self.phase = Phase::Splash;
        self.draws = 0;
        self.gameplay_ready = false;
        self.error = None;
        self.ticket
    }
    /// Presentation acknowledgements, not elapsed seconds. Starts CPU work after
    /// the native minimum of three draws; GPU readiness is a separate concern.
    pub fn presented(&mut self) -> bool {
        if self.phase != Phase::Splash {
            return false;
        }
        self.draws += 1;
        if self.draws < 3 {
            return false;
        }
        self.phase = Phase::Loading;
        true
    }
    pub fn prepared(&mut self, ticket: u64) -> bool {
        self.advance(ticket, Phase::Loading, Phase::AwaitingRuntime)
    }
    pub fn runtime_ready(&mut self, ticket: u64, cinematic: bool, gameplay_ready: bool) -> bool {
        if self.ticket != ticket || self.phase != Phase::AwaitingRuntime {
            return false;
        }
        self.gameplay_ready = gameplay_ready;
        self.phase = if cinematic {
            Phase::Cinematic
        } else {
            Phase::Objectives
        };
        true
    }
    pub fn cinematic_finished(&mut self, ticket: u64) -> bool {
        self.advance(ticket, Phase::Cinematic, Phase::Objectives)
    }
    /// A user skip ends presentation, without granting gameplay readiness.
    pub fn cinematic_skipped(&mut self, ticket: u64) -> bool {
        self.advance(ticket, Phase::Cinematic, Phase::Objectives)
    }
    pub fn gameplay_ready(&mut self, ticket: u64) -> bool {
        if self.ticket != ticket || !matches!(self.phase, Phase::Cinematic | Phase::Objectives) {
            return false;
        }
        self.gameplay_ready = true;
        true
    }
    pub fn dismiss_objectives(&mut self, ticket: u64) -> bool {
        self.gameplay_ready && self.advance(ticket, Phase::Objectives, Phase::Gameplay)
    }
    fn advance(&mut self, ticket: u64, from: Phase, to: Phase) -> bool {
        if self.ticket != ticket || self.phase != from {
            return false;
        }
        self.phase = to;
        true
    }
    pub fn fail(&mut self, ticket: u64, error: String) {
        if self.ticket == ticket {
            self.phase = Phase::Failed;
            self.error = Some(error);
        }
    }
    pub fn cancel(&mut self) {
        self.ticket += 1;
        self.phase = Phase::Menu;
        self.gameplay_ready = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn skipping_requires_current_cinematic_and_does_not_grant_a_controller() {
        let mut flow = Flow::default();
        let ticket = flow.begin();
        assert!(!flow.cinematic_skipped(ticket));
        for _ in 0..3 {
            flow.presented();
        }
        assert!(flow.prepared(ticket));
        assert!(flow.runtime_ready(ticket, true, false));
        assert!(!flow.cinematic_skipped(ticket + 1));
        assert!(flow.cinematic_skipped(ticket));
        assert_eq!(flow.phase, Phase::Objectives);
        assert!(!flow.dismiss_objectives(ticket));
        assert!(flow.gameplay_ready(ticket));
        assert!(flow.dismiss_objectives(ticket));
        assert!(!flow.cinematic_skipped(ticket));
    }
    #[test]
    fn showcase_progress_reopens_all_catalog_missions_without_changing_other_profiles() {
        let folder = std::env::temp_dir().join(format!(
            "giants-showcase-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(folder.join("Bin")).unwrap();
        let user = folder.join("user");
        let mut catalog = vec![0u8; 32];
        catalog[..4].copy_from_slice(&0x1a0002e1u32.to_le_bytes());
        catalog[24..28].copy_from_slice(&32u32.to_le_bytes());
        catalog.extend(2u32.to_le_bytes());
        for name in ["First", "Second"] {
            let mut row = [0; 33];
            row[0] = 7;
            row[1..1 + name.len()].copy_from_slice(name.as_bytes());
            catalog.extend(row);
        }
        std::fs::write(folder.join("Bin/w_Test.bin"), &catalog).unwrap();
        std::fs::write(folder.join("Bin/w_End.bin"), &catalog).unwrap();
        std::fs::create_dir_all(user.join("campaign-progress")).unwrap();
        let original = progress_path(&user, "Other");
        std::fs::write(&original, b"preserve unrelated profile").unwrap();
        let mut campaign = Campaign {
            root: folder.clone(),
            user_root: user,
            text: Default::default(),
            worlds: vec![
                WorldEntry {
                    id: 0,
                    name: "Test".into(),
                    title: "Test world".into(),
                },
                WorldEntry {
                    id: 18,
                    name: "End".into(),
                    title: "Ending world".into(),
                },
            ],
            menu: None,
            world: None,
            progress: Progress::load(None, None).unwrap(),
            showcase: false,
        };
        assert_eq!(campaign.unlock_profile("Showcase").unwrap(), 4);
        campaign.select_profile("Showcase").unwrap();
        assert_eq!(campaign.available_worlds().unwrap().len(), 2);
        campaign.select_world(1).unwrap();
        assert_eq!(campaign.request(1).unwrap().mission_name, "Second");
        assert_eq!(
            std::fs::read(original).unwrap(),
            b"preserve unrelated profile"
        );
        campaign.select_profile("Fresh").unwrap();
        assert!(campaign.available_worlds().unwrap().is_empty());
        std::fs::remove_dir_all(folder).unwrap();
    }
    #[test]
    fn instance_planning_rejects_reader_needs_and_changed_input_identity() {
        let mut prepared = PreparedMission {
            request: LoadRequest {
                world: "Alpha".into(),
                menu_index: 0,
                source_id: 0,
                mission_name: "First".into(),
                splash: crate::mission_selection::Splash::Mecc,
            },
            world: None,
            mission: Some(giants_formats::mission::parse(&[4, 0, 0, 0, 1, 2]).unwrap()),
            scenario_index: Some(0),
            terrain: None,
            terrain_bytes: vec![],
            world_bytes: b"expected-world".to_vec(),
            mission_bytes: b"expected-mission".to_vec(),
            needs: vec!["reader capability missing".into()],
        };
        assert!(
            prepared
                .instance_plan(Path::new("missing"), "English")
                .err()
                .unwrap()
                .contains("reader needs")
        );
        assert!(
            prepared
                .world_asset(Path::new("missing"))
                .err()
                .unwrap()
                .contains("reader needs")
        );
        prepared.needs.clear();
        prepared.request.world = "../Alpha".into();
        assert!(
            prepared
                .world_asset(Path::new("missing"))
                .err()
                .unwrap()
                .contains("invalid prepared")
        );
        prepared.request.world = "Alpha".into();
        let temp = std::env::temp_dir();
        let folder = temp.join(format!(
            "mission-identity-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&folder).unwrap();
        std::fs::create_dir(folder.join("Bin")).unwrap();
        std::fs::write(folder.join("Bin/w_Alpha.bin"), b"different-world").unwrap();
        assert!(
            prepared
                .world_asset(&folder)
                .err()
                .unwrap()
                .contains("inputs changed")
        );
        assert!(
            prepared
                .instance_plan(&folder, "English")
                .err()
                .unwrap()
                .contains("inputs changed")
        );
        let resolved = folder.canonicalize().unwrap();
        assert!(resolved.starts_with(temp.canonicalize().unwrap()));
        std::fs::remove_dir_all(resolved).unwrap();
    }
    #[test]
    fn world_names_ids_offsets_and_terrain_prefix_are_bounded() {
        let mut b = Vec::new();
        for n in [0x1a0003d0u32, 22, 1, 8] {
            b.extend(n.to_le_bytes());
        }
        b.extend(b"Alpha\0");
        b.extend(0u32.to_le_bytes());
        b.extend(7u32.to_le_bytes());
        let table = TextTable::new();
        let w = world_list(&b, &table).unwrap();
        assert_eq!((w[0].id, w[0].name.as_str()), (7, "Alpha"));
        for (at, value) in [(0, 0u32), (4, 999), (8, 257), (12, 0), (12, 0xffff)] {
            let mut bad = b.clone();
            bad[at..at + 4].copy_from_slice(&value.to_le_bytes());
            assert!(world_list(&bad, &table).is_err());
        }
        let mut world = vec![0; 36];
        world[..4].copy_from_slice(&0x1a0002e5u32.to_le_bytes());
        world[4..8].copy_from_slice(&32u32.to_le_bytes());
        let block = b"box\0terrain.gti\0";
        world[32..36].copy_from_slice(&(block.len() as u32).to_le_bytes());
        world.extend(block);
        assert_eq!(terrain_name(&world).unwrap(), "terrain.gti");
        world.pop();
        assert!(terrain_name(&world).is_err());
    }
    #[test]
    fn gameplay_requires_real_runtime_and_cinematic_acknowledgements() {
        let mut f = Flow::default();
        let t = f.begin();
        assert!(!f.dismiss_objectives(t));
        assert!(!f.presented());
        assert!(!f.presented());
        assert!(f.presented());
        assert!(f.prepared(t));
        assert!(!f.cinematic_finished(t));
        assert!(!f.dismiss_objectives(t));
        assert!(f.runtime_ready(t, true, true));
        assert!(!f.dismiss_objectives(t));
        assert!(f.cinematic_finished(t));
        assert!(f.dismiss_objectives(t));
        assert_eq!(f.phase, Phase::Gameplay);
        f.cancel();
        assert!(!f.runtime_ready(t, false, true));
        let next = f.begin();
        for _ in 0..3 {
            f.presented();
        }
        f.prepared(next);
        f.runtime_ready(next, false, false);
        assert!(!f.dismiss_objectives(next));
        assert_eq!(f.phase, Phase::Objectives);
        assert!(f.gameplay_ready(next));
        assert!(f.dismiss_objectives(next));
    }
    #[test]
    fn profile_identity_and_stale_load_results_stay_separate() {
        assert_ne!(
            progress_path(Path::new("user"), "A"),
            progress_path(Path::new("user"), "a")
        );
        assert!(
            !progress_path(Path::new("user"), "../A")
                .to_string_lossy()
                .contains("..")
        );
        let mut f = Flow::default();
        let old = f.begin();
        f.cancel();
        let current = f.begin();
        f.fail(old, "old error".into());
        assert_eq!(f.phase, Phase::Splash);
        assert_eq!(f.ticket, current);
    }
}
