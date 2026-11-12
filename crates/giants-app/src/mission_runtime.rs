//! Ticket-owned mission staging. Data actors are real ECS entities; rendering,
//! behavior callbacks and playable readiness remain explicitly incomplete.
use crate::frontend_scene::{MissionCancelled, MissionPrepared, Scene};
use bevy::prelude::*;
use giants_game::{
    instances::Plan,
    mission_loading::{LaunchIntent, Phase, PreparedMission},
    mission_selection::Progress,
};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
};

type DeclaredActorPose = ([f32; 3], [f32; 3]);
type ActorId = giants_game::resident_states::Id;
pub(crate) struct CampaignCompletion {
    pub ticket: u64,
    pub current: giants_game::mission_selection::LoadRequest,
    pub next: giants_game::mission_selection::LoadRequest,
    pub carried: f32,
    pub counter: u32,
    pub required: i32,
    pub film: Option<String>,
}
mod campaign;
mod landing;
mod npc;
mod signals;
pub(crate) use npc::{Draw as NpcDraw, DrawOwner as NpcDrawOwner};
#[path = "mission_runtime/owned.rs"]
mod owned;
pub(crate) use owned::Input as OwnedMeccInput;
struct PickupPacket {
    parent: ActorId,
    child: ActorId,
    services: Vec<giants_game::resident_states::PickupService>,
    random: [u32; 3],
}
pub struct Plugin;
impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Runtime>().add_systems(
            Update,
            (drive, bind_declared_residents, audit)
                .chain()
                .after(crate::frontend_scene::runtime_events),
        );
    }
}
#[derive(Component)]
pub struct MissionOwned(pub u64);
#[derive(Component)]
pub struct ActorRef {
    pub ticket: u64,
    pub index: usize,
    pub world_declaration: bool,
}
#[derive(Component)]
pub struct OwnedActor(pub giants_game::resident_states::Id);
fn declaration_key(reference: &ActorRef) -> giants_game::declared_residents::Key {
    giants_game::declared_residents::Key {
        scope: if reference.world_declaration {
            giants_game::declared_residents::Scope::World
        } else {
            giants_game::declared_residents::Scope::Mission
        },
        index: reference.index,
    }
}
struct Payload {
    signals: signals::State,
    npcs: Result<Option<npc::State>, String>,
    impact_sounds: Vec<u32>,
    packet: Arc<PreparedMission>,
    plan: Plan,
    models: Arc<giants_game::static_geometry::Scene>,
    world: Result<Arc<giants_game::WorldAsset>, String>,
    opening: Result<giants_game::mission_cinematics::Opening, String>,
    playback: Result<Option<crate::mission_opening::Prepared>, String>,
    parameters: Result<giants_game::mission_slot::Projection, String>,
    start_candidate: Result<giants_game::player_start::Declared, String>,
    director: Result<Option<giants_game::director_initial::Prepared>, String>,
    effect_proxies: Result<Vec<giants_game::cinematic_fx::TerminalProxy>, String>,
    player: Result<Arc<giants_game::player_initial::Prepared>, String>,
    kabuto: Result<Option<giants_game::kabuto_initial::Factory>, String>,
    kabuto_body: Option<Arc<giants_game::player_body::Prepared>>,
    kabuto_camera_inspection:
        Option<Result<giants_game::kabuto_camera_inspection::Prepared, String>>,
    delphi_body: Option<Arc<giants_game::player_body::Prepared>>,

    delphi: Result<Option<giants_game::delphi_declared::Factory>, String>,
    controller: Result<Option<crate::mecc_dev_scene::State>, String>,
    carried: Result<Option<(usize, Arc<giants_game::carried_actor::Prepared>)>, String>,
    dressing: Option<crate::mission_dressing::Prepared>,
    opening_effects_bound: bool,
}
#[derive(Component)]
pub struct PlayerInitial {
    pub ticket: u64,
    pub prepared: Arc<giants_game::player_initial::Prepared>,
}
struct StartState {
    intent: LaunchIntent,
    progress: Progress,
}
struct Pending {
    ticket: u64,
    cancel: Arc<AtomicBool>,
    receiver: Mutex<mpsc::Receiver<Result<Payload, String>>>,
    start: StartState,
}
impl Drop for Pending {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}
#[derive(Clone, Copy, Default)]
struct ActorBindings {
    player: Option<ActorId>,
    carried: Option<ActorId>,
    /// None means no completed Delphi player initialization to acknowledge.
    delphi_projection: Option<giants_game::delphi_player::ProjectionStatus>,
}
struct Active {
    ticket: u64,
    root: Entity,
    actors: usize,
    resident_states: giants_game::resident_states::Session,
    blocked: Vec<String>,
    payload: Payload,
    start: StartState,
    reported: bool,
    pickup: Option<PickupPacket>,
    variant_random: giants_game::audio_parameters::Random,
    audio_registry: Option<giants_game::gameplay_sound_bank::Registry>,
    bindings: ActorBindings,
    // Legacy motion publication is independent of constructor identity. The
    // owned driver must still reject a prior development-motion publication.
    controlled_mecc_owner: Option<ActorId>,
    owned_mecc: Option<owned::Driver>,
    landing: Option<landing::Driver>,
    campaign: Option<Result<campaign::State, String>>,
}
impl Active {
    fn player_owner(&self) -> Result<ActorId, String> {
        let class = if let Some(factory) = self.payload.delphi.as_ref().map_err(Clone::clone)? {
            factory.declared_start().kind
        } else {
            match self.payload.kabuto.as_ref().map_err(Clone::clone)? {
                Some(factory) => factory.declared_start().kind,
                None => {
                    self.payload
                        .player
                        .as_ref()
                        .map_err(Clone::clone)?
                        .start
                        .kind
                }
            }
        };
        self.bindings
            .player
            .filter(|id| {
                self.resident_states
                    .get(*id)
                    .is_some_and(|a| a.class == class)
            })
            .ok_or_else(|| "constructed player owner absent, foreign or changed".into())
    }
    fn carried_owner(&self) -> Result<Option<ActorId>, String> {
        let Some((_, prepared)) = self.payload.carried.as_ref().map_err(Clone::clone)? else {
            return Ok(None);
        };
        self.bindings
            .carried
            .filter(|id| {
                self.resident_states
                    .get(*id)
                    .is_some_and(|a| a.class == prepared.class)
            })
            .map(Some)
            .ok_or_else(|| "constructed carried owner absent, foreign or changed".into())
    }
}
#[derive(Resource, Default)]
pub struct Runtime {
    pending: Option<Pending>,
    active: Option<Active>,
    handled: Option<u64>,
    retired: Vec<u64>,
    failure: Option<(u64, String)>,
    smoke_geometry:
        std::collections::BTreeMap<(u64, ActorId), Arc<giants_game::smoke_particle_draw::Geometry>>,
}
pub(crate) struct AmbientDeclaration {
    pub key: usize,
    pub class: i32,
    pub position: [f32; 3],
    pub config: giants_game::ambient_streams::Config,
}
pub struct EnvironmentCandidate {
    pub ticket: u64,
    pub asset: Arc<giants_game::WorldAsset>,
    pub camera: crate::validation_camera::ValidationCamera,
    pub models: Arc<giants_game::static_geometry::Scene>,
    pub eye: [f32; 3],
    pub lod_scale: f32,
    pub camera_source: &'static str,
}
pub(crate) struct Startup {
    pub ticket: u64,
    pub cinematic: bool,
    pub companions: u32,
}
fn prepare_kabuto(
    root: &std::path::Path,
    plan: &Plan,
    packet: &PreparedMission,
) -> Result<Option<giants_game::kabuto_initial::Factory>, String> {
    let mission = packet.mission.as_ref().ok_or("prepared mission missing")?;
    if mission.starts.first().is_none_or(|s| s.words[0] != 1) {
        return Ok(None);
    }
    let globals = giants_formats::global_data::parse(
        &std::fs::read(root.join("Bin/GData.gbn")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    giants_game::kabuto_initial::Factory::load(
        plan,
        packet.world.as_ref().ok_or("prepared world missing")?,
        mission,
        Arc::new(globals),
    )
    .map(Some)
}
fn requests_delphi(packet: &PreparedMission) -> bool {
    packet.mission.as_ref().is_some_and(|mission| {
        mission
            .starts
            .first()
            .is_some_and(|start| start.words[0] == 3)
    })
}
fn prepare_delphi(
    root: &std::path::Path,
    plan: &Plan,
    packet: &PreparedMission,
) -> Result<Option<giants_game::delphi_declared::Factory>, String> {
    let mission = packet.mission.as_ref().ok_or("prepared mission missing")?;
    if !requests_delphi(packet) {
        return Ok(None);
    }
    let globals = giants_formats::global_data::parse(
        &std::fs::read(root.join("Bin/GData.gbn")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let weapons = giants_formats::weapons::parse_with_profile(
        &std::fs::read(root.join("Bin/weapondefs.bin")).map_err(|e| e.to_string())?,
        giants_formats::weapons::Profile::Debug,
    )
    .map_err(|e| e.to_string())?;
    giants_game::delphi_declared::Factory::load(
        plan,
        packet.world.as_ref().ok_or("prepared world missing")?,
        mission,
        &globals,
        &weapons,
    )
    .map(Some)
}
impl Runtime {
    /// Explicit diagnostic command, never ordinary startup or an initializer ack.
    /// Retain a failed attempt so a later frame cannot replay its completed stores.
    pub(crate) fn prepare_kabuto_camera_inspection(
        &mut self,
        executable: &std::path::Path,
    ) -> Result<(), String> {
        let active = self
            .active
            .as_mut()
            .ok_or("Kabuto camera inspection mission absent")?;
        if let Some(result) = &active.payload.kabuto_camera_inspection {
            let prepared = result.as_ref().map_err(Clone::clone)?;
            if active.player_owner()? != prepared.owner
                || !prepared.current(&active.resident_states)
            {
                return Err("Kabuto camera inspection owner or header changed".into());
            }
            return Ok(());
        }
        let owner = active.player_owner()?;
        let result = giants_game::kabuto_camera_inspection::Prepared::after_mode_one(
            &mut active.resident_states,
            owner,
            executable,
            [1920., 1080.],
        );
        let status = result.as_ref().map(|_| ()).map_err(Clone::clone);
        active.payload.kabuto_camera_inspection = Some(result);
        println!(
            "mission_kabuto_camera_inspection ticket={} mode_request=1 unclipped=true native_startup_verified=false initializer_acknowledged=false ready=false result={status:?}",
            active.ticket
        );
        status
    }
    pub(crate) fn delphi_body(&self) -> Option<(u64, &giants_game::player_body::Prepared)> {
        let active = self.active.as_ref()?;
        let body = active.payload.delphi_body.as_deref()?;
        (active.bindings.player == Some(body.owner) && body.current(&active.resident_states))
            .then_some((active.ticket, body))
    }
    pub(crate) fn initial_player_body(&self) -> Option<(u64, &giants_game::player_body::Prepared)> {
        self.kabuto_body().or_else(|| self.delphi_body())
    }
    pub(crate) fn kabuto_body(&self) -> Option<(u64, &giants_game::player_body::Prepared)> {
        let active = self.active.as_ref()?;
        let body = active.payload.kabuto_body.as_deref()?;
        (active.bindings.player == Some(body.owner) && body.current(&active.resident_states))
            .then_some((active.ticket, body))
    }
    pub(crate) fn cinematic_session(
        &self,
        ticket: u64,
    ) -> Option<&giants_game::resident_states::Session> {
        Some(
            &self
                .active
                .as_ref()
                .filter(|a| a.ticket == ticket)?
                .resident_states,
        )
    }
    pub(crate) fn cinematic_session_mut(
        &mut self,
        ticket: u64,
    ) -> Option<&mut giants_game::resident_states::Session> {
        Some(
            &mut self
                .active
                .as_mut()
                .filter(|a| a.ticket == ticket)?
                .resident_states,
        )
    }
    fn initialize_owned_pickup(&mut self, ticket: u64) {
        let Some(active) = self.active.as_mut().filter(|a| a.ticket == ticket) else {
            return;
        };
        let (Ok(_), Ok(Some(_))) = (&active.payload.player, &active.payload.carried) else {
            return;
        };
        let result = (|| -> Result<(), String> {
            let parent = active.player_owner()?;
            let child = active
                .carried_owner()?
                .ok_or("constructed carried owner absent")?;
            let point = active.resident_states.constructor_point(
                parent,
                giants_game::smartie_carry::MECC_PARENT_REFERENCE as i32,
            )?;
            let before = active.variant_random.0;
            let choice = active.variant_random.next_15() * 100 / 32768;
            let services = active
                .resident_states
                .pickup_fresh(child, parent, point, 0, choice)?;
            let resident = active
                .resident_states
                .get(child)
                .ok_or("pickup child retired")?;
            let fields = resident
                .construction_fields
                .as_ref()
                .ok_or("pickup fields absent")?;
            println!(
                "mission_owned_pickup_state ticket={ticket} class={} scale_bits={} behavior=19 payload_written=true binding=parent10_child1 notice_pending={} source_point={point:?} supplied_clock_word=0 clock_source=own_mission_start live_rng_phase_verified=false complete_transaction=false ready=false",
                resident.class,
                fields.scale.to_bits(),
                resident.pickup_notice_pending
            );
            active.pickup = Some(PickupPacket {
                parent,
                child,
                services,
                random: [before, choice, active.variant_random.0],
            });
            Ok(())
        })();
        if let Err(error) = result {
            active.blocked.push(format!("owned pickup state: {error}"));
            println!("mission_owned_pickup_refused ticket={ticket} error={error:?} ready=false");
        }
    }
    pub(crate) fn take_pickup_services(
        &mut self,
        ticket: u64,
        parent: ActorId,
        child: ActorId,
    ) -> Result<(Vec<giants_game::resident_states::PickupService>, [u32; 3]), String> {
        let active = self
            .active
            .as_mut()
            .filter(|a| a.ticket == ticket)
            .ok_or("pickup service ticket absent")?;
        let pending = active
            .pickup
            .as_ref()
            .ok_or("pickup services absent or already taken")?;
        if pending.parent != parent || pending.child != child {
            return Err("pickup service pair differs".into());
        }
        let pending = active.pickup.take().expect("validated pickup packet");
        Ok((pending.services, pending.random))
    }
    pub(crate) fn variant_choice(&mut self, ticket: u64) -> Result<u32, String> {
        let active = self
            .active
            .as_mut()
            .filter(|a| a.ticket == ticket)
            .ok_or("variant random ticket absent")?;
        Ok(active.variant_random.next_15() * 100 / 32768)
    }
    pub(crate) fn with_actor_random<T>(
        &mut self,
        ticket: u64,
        operation: impl FnOnce(&mut giants_game::audio_parameters::Random) -> Result<T, String>,
    ) -> Result<T, String> {
        let active = self
            .active
            .as_mut()
            .filter(|a| a.ticket == ticket)
            .ok_or("actor random ticket absent")?;
        operation(&mut active.variant_random)
    }
    pub(crate) fn carried_response(&self, ticket: u64) -> u32 {
        let Some(active) = self.active.as_ref().filter(|a| a.ticket == ticket) else {
            return 0;
        };
        active
            .player_owner()
            .ok()
            .and_then(|id| active.resident_states.get(id))
            .and_then(|r| r.construction_fields.as_ref())
            .map_or(0, |f| f.response_flags)
    }
    pub(crate) fn player_state_specification(
        &self,
        ticket: u64,
        offset: usize,
    ) -> Result<&giants_game::specification::Prepared, String> {
        let active = self
            .active
            .as_ref()
            .filter(|a| a.ticket == ticket)
            .ok_or("player state resource ticket absent")?;
        let id = active.player_owner()?;
        Ok(active
            .resident_states
            .state_resources(id)
            .ok_or("player state resources unbound")?
            .specification(offset)?
            .as_ref())
    }
    pub(crate) fn publish_controlled_mecc_motion(
        &mut self,
        ticket: u64,
        step: u64,
        frame: giants_game::controlled_mecc::Frame,
    ) -> Result<(), String> {
        let active = self
            .active
            .as_mut()
            .filter(|a| a.ticket == ticket)
            .ok_or("controlled motion ticket absent")?;
        if active.owned_mecc.is_some() {
            return Err("owned primary already owns this mission's Mecc motion".into());
        }
        let id = active.player_owner()?;
        if active
            .controlled_mecc_owner
            .is_some_and(|previous| previous != id)
        {
            return Err("controlled motion owner changed after publication".into());
        }
        active.controlled_mecc_owner = Some(id);
        active
            .resident_states
            .publish_controlled_mecc_motion(id, step, frame)?;
        let owned = active.resident_states.controlled_mecc_motion(id, step)?;
        let fields = active
            .resident_states
            .get(id)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap();
        if [1, 60, 240, 360, 480, 720].contains(&step) {
            println!(
                "mission_controlled_actor_motion ticket={ticket} step={step} source_bits={:?} actor_bits={:?} current_bits={:?} history_bits={:?} rates_bits={:?} ready=false",
                frame.position.map(f32::to_bits),
                fields.position.map(f32::to_bits),
                fields.current.map(f32::to_bits),
                fields.previous.map(f32::to_bits),
                owned.frame.model_rates.map(f32::to_bits)
            );
        }
        Ok(())
    }
    pub(crate) fn sample_world_clock(
        &mut self,
        ticket: u64,
        observed_ms: u32,
    ) -> Result<(), String> {
        let active = self
            .active
            .as_mut()
            .filter(|a| a.ticket == ticket)
            .ok_or("mission clock ticket absent")?;
        let owner = active.player_owner()?;
        active.payload.signals.sample(owner, observed_ms)
    }
    pub(crate) fn publish_player_noise(&mut self, ticket: u64) -> Result<(), String> {
        let active = self
            .active
            .as_mut()
            .filter(|a| a.ticket == ticket)
            .ok_or("mission noise ticket absent")?;
        let owner = active.player_owner()?;
        let position = active
            .resident_states
            .get(owner)
            .and_then(|a| a.construction_fields.as_ref())
            .ok_or("mission noise actor fields absent")?
            .position;
        let seconds = active.payload.signals.publish_noise(owner, position)?;
        println!(
            "mission_player_noise ticket={ticket} seconds_bits={} position_bits={:?} source=ordinary_pistol native_primary_complete=false",
            seconds.to_bits(),
            position.map(f32::to_bits)
        );
        Ok(())
    }
    pub(crate) fn controlled_jet_session(
        &self,
        ticket: u64,
    ) -> Result<(&giants_game::resident_states::Session, ActorId), String> {
        let active = self
            .active
            .as_ref()
            .filter(|a| a.ticket == ticket)
            .ok_or("normal jet ticket absent")?;
        let owner = active.player_owner()?;
        if active.owned_mecc.is_some() || active.controlled_mecc_owner != Some(owner) {
            return Err("normal jet requires the actual ordinary motion owner".into());
        }
        Ok((&active.resident_states, owner))
    }
    pub(crate) fn controlled_jet_session_mut(
        &mut self,
        ticket: u64,
    ) -> Result<(&mut giants_game::resident_states::Session, ActorId), String> {
        let (_, owner) = self.controlled_jet_session(ticket)?;
        Ok((&mut self.active.as_mut().unwrap().resident_states, owner))
    }
    pub(crate) fn pistol_projectile_visual(
        &self,
        ticket: u64,
    ) -> Result<crate::mission_projectile_scene::Visual, String> {
        let active = self
            .active
            .as_ref()
            .filter(|a| a.ticket == ticket)
            .ok_or("projectile render ticket absent")?;
        crate::mission_projectile_scene::Visual::load(active.payload.plan.resources())
    }
    pub(crate) fn publish_actor_audio(
        &mut self,
        ticket: u64,
        player: [f32; 3],
        carried: Option<[f32; 3]>,
    ) -> Result<
        (
            giants_game::resident_states::Id,
            Option<giants_game::resident_states::Id>,
        ),
        String,
    > {
        if !player
            .into_iter()
            .chain(carried.into_iter().flatten())
            .all(f32::is_finite)
        {
            return Err("nonfinite actor audio presentation pose".into());
        }
        let active = self
            .active
            .as_mut()
            .filter(|a| a.ticket == ticket)
            .ok_or("actor audio ticket absent")?;
        let parent = active.player_owner()?;
        let child = if matches!(active.payload.carried, Ok(Some(_))) {
            active.carried_owner()?
        } else {
            None
        };
        active
            .resident_states
            .set_presentation_position(parent, Some(player))?;
        if let Some(child) = child {
            active
                .resident_states
                .set_presentation_position(child, carried)?;
        }
        Ok((parent, child))
    }
    pub(crate) fn actor_audio_position(
        &self,
        ticket: u64,
        id: giants_game::resident_states::Id,
    ) -> Option<[f32; 3]> {
        let active = self.active.as_ref().filter(|a| a.ticket == ticket)?;
        if active.resident_states.vimp(id).is_some() {
            // Canonical completed NPC ownership supplies its current position;
            // a living Vimp has no independent presentation-position mirror.
            active.resident_states.completed_vimp_frame(id).ok()?;
            let fields = active
                .resident_states
                .get(id)?
                .construction_fields
                .as_ref()?;
            return (fields.object_flags & 0xc000 == 0).then_some(fields.position);
        }
        if active.resident_states.positional_impact(id).is_some() {
            let fields = active
                .resident_states
                .get(id)?
                .construction_fields
                .as_ref()?;
            return (fields.object_flags & 0x8000 == 0).then_some(fields.position);
        }
        active.resident_states.presentation_position(id)
    }
    pub(crate) fn actor_sound_route(
        &self,
        ticket: u64,
        id: giants_game::resident_states::Id,
        kind: giants_game::actor_sound::Kind,
        ready: bool,
    ) -> Result<Option<giants_game::actor_sound::Route<giants_game::resident_states::Id>>, String>
    {
        let active = self
            .active
            .as_ref()
            .filter(|a| a.ticket == ticket)
            .ok_or("actor sound ticket absent")?;
        let local = active.player_owner()?;
        active
            .resident_states
            .sound_route(id, local, kind, ready)
            .map_err(String::from)
    }
    pub(crate) fn resolve_actor_sound(
        &mut self,
        ticket: u64,
        sound: u32,
    ) -> Result<Option<giants_game::gameplay_sound_bank::Sound>, String> {
        let active = self
            .active
            .as_mut()
            .filter(|a| a.ticket == ticket)
            .ok_or("actor sound ticket not active")?;
        let resources = active.payload.plan.resources();
        if active.audio_registry.is_none() {
            active.audio_registry = Some(giants_game::gameplay_sound_bank::Registry::prepare(
                &resources.classes.visits,
                false,
            )?);
        }
        let (language, _) = giants_game::gameplay_sound_bank::voice_language(
            &resources.archives,
            &resources.language,
        )?;
        active
            .audio_registry
            .as_ref()
            .unwrap()
            .resolve(&resources.archives, &language, sound)
    }
    pub(crate) fn owned_impact_cues(&self, ticket: u64) -> Result<Vec<u32>, String> {
        let active = self
            .active
            .as_ref()
            .filter(|a| a.ticket == ticket)
            .ok_or("impact cue resource ticket absent or foreign")?;
        let Some([low, high]) = giants_game::positional_impact::cue_range(
            active.payload.plan.resources(),
            "fx_mecc_hard_land",
        )?
        else {
            return Ok(vec![]);
        };
        if high - low >= 32 {
            return Err("impact cue reservation exceeds source budget".into());
        }
        Ok((low..=high).collect())
    }
    pub(crate) fn sound_bank(
        &self,
        ticket: u64,
    ) -> Result<giants_game::gameplay_sound_bank::Bank, String> {
        let active = self
            .active
            .as_ref()
            .filter(|a| a.ticket == ticket)
            .ok_or("audio ticket not active")?;
        let resources = active.payload.plan.resources();
        // These are controlled Mecc cue recipes, not universal mission sounds.
        // Use the actual single start declaration; a failed player preparation
        // must not hide missing Mecc resources in an otherwise Mecc mission.
        let player_kind =
            active
                .payload
                .packet
                .mission
                .as_ref()
                .and_then(|m| match m.starts.as_slice() {
                    [start] => Some(start.words[0]),
                    _ => None,
                });
        let mecc_cues = player_kind == Some(2);
        let mut ids = if mecc_cues {
            vec![
                0x5f7, 0x5f8, 0x5b0, 0x5b1, 0x5b2, 0x5b3, 0x5b5, 0x5b6, 0x5b7,
            ]
        } else {
            Vec::new()
        };
        ids.extend(active.payload.impact_sounds.iter().copied());
        if mecc_cues && matches!(&active.payload.carried, Ok(Some(_))) {
            ids.extend([1510, 1705, 1706]);
        }
        if mecc_cues && let Ok(footsteps) = giants_game::mecc_footsteps::Prepared::load(resources) {
            ids.extend(footsteps.sound_ids());
        }
        println!(
            "mission_audio_player_profile ticket={ticket} declared_kind={player_kind:?} mecc_cues={mecc_cues} other_local_profiles_implemented=false generic_actor_sounds=actual_catalog_lazy_lookup"
        );
        let (language, inferred) = giants_game::gameplay_sound_bank::voice_language(
            &resources.archives,
            &resources.language,
        )?;
        println!(
            "mission_audio_language ticket={ticket} requested={} selected={language} single_installed_locale={inferred} native_fallback_claimed=false",
            resources.language
        );
        giants_game::gameplay_sound_bank::Bank::prepare(
            &resources.classes.visits,
            &resources.archives,
            &language,
            false,
            ids,
        )
    }
    pub(crate) fn flight_bank(
        &self,
        ticket: u64,
        definition: &giants_formats::weapons::Projectile,
    ) -> Result<
        (
            giants_game::projectile_flight_audio::Recipe,
            giants_game::gameplay_sound_bank::Bank,
        ),
        String,
    > {
        if definition.class != 38 || !matches!(definition.id, 17 | 42) {
            return Err("flight audio outside checked constructor profile".into());
        }
        let active = self
            .active
            .as_ref()
            .filter(|a| a.ticket == ticket)
            .ok_or("flight ticket not active")?;
        let resources = active.payload.plan.resources();
        let request = giants_game::projectile_effect_requests::constructor(definition)
            .ok_or("flight effect absent")?;
        let image = &resources
            .classes
            .models
            .effects
            .last()
            .ok_or("flight effect image absent")?
            .image;
        let (_, entry) = image
            .find(request.name)
            .ok_or("flight source program absent")?;
        let recipe = giants_game::projectile_flight_audio::Recipe::prepare(
            &image.program(entry.offset).map_err(|e| e.to_string())?,
        )?;
        let bank = giants_game::gameplay_sound_bank::Bank::prepare(
            &resources.classes.visits,
            &resources.archives,
            &resources.language,
            false,
            [recipe.sound],
        )?;
        Ok((recipe, bank))
    }
    pub(crate) fn ambient_configs(&self, ticket: u64) -> Result<Vec<AmbientDeclaration>, String> {
        let active = self
            .active
            .as_ref()
            .filter(|a| a.ticket == ticket)
            .ok_or("ambient ticket not active")?;
        let plan = &active.payload.plan;
        let mut configurations = Vec::new();
        let mut declared = 0usize;
        let mut inspect = |key, class, position, seed: &giants_game::state_storage::Seed| {
            if seed.storage.state() != 58 {
                return;
            }
            declared += 1;
            let result = seed
                .storage
                .data()
                .ok_or_else(|| "ambient storage absent".to_owned())
                .and_then(giants_game::ambient_streams::Config::from_stream_state);
            match result {
                Ok(config) => configurations.push(AmbientDeclaration {
                    key,
                    class,
                    position,
                    config,
                }),
                Err(reason) => println!(
                    "mission_ambient_unsupported ticket={ticket} emitter={key} class={class} reason={reason:?}"
                ),
            }
        };
        for (index, entry) in plan.entries.iter().enumerate() {
            if let Ok(source) = &entry.source
                && let Ok(seed) = &source.state_seed
            {
                inspect(index, source.class, source.position, seed);
            }
        }
        for (index, entry) in plan.world_entries.iter().enumerate() {
            if let Ok(source) = entry.class_input()
                && let Ok(seed) = &source.state_seed
            {
                inspect(
                    plan.entries.len() + index,
                    source.class,
                    entry.declaration.position,
                    seed,
                );
            }
        }
        println!(
            "mission_ambient_inventory ticket={ticket} declared={declared} supported={} unsupported={}",
            configurations.len(),
            declared - configurations.len()
        );
        Ok(configurations)
    }
    pub(crate) fn take_dressing(
        &mut self,
        ticket: u64,
    ) -> Option<crate::mission_dressing::Prepared> {
        self.active
            .as_mut()
            .filter(|a| a.ticket == ticket)?
            .payload
            .dressing
            .take()
    }
    pub(crate) fn failed_ticket(&self) -> Option<u64> {
        self.failure.as_ref().map(|(ticket, _)| *ticket)
    }
    /// A resident cold Mecc can be exercised without admitting unresolved mission rules.
    /// In particular, proxy34 still requires an actor-service backend.
    pub(crate) fn carried_preview(
        &self,
    ) -> Option<(u64, usize, Arc<giants_game::carried_actor::Prepared>)> {
        let active = self.active.as_ref()?;
        let (offset, prepared) = active.payload.carried.as_ref().ok()?.as_ref()?;
        Some((active.ticket, *offset, Arc::clone(prepared)))
    }
    /// The preview consumes its resident's clock. Only its loaded clip is
    /// supplied; nonzero model-speed references require a future live consumer.
    pub(crate) fn advance_carried_clock(
        &mut self,
        ticket: u64,
        dt: f32,
    ) -> Result<giants_game::animation_loop::Clock, String> {
        let active = self
            .active
            .as_mut()
            .filter(|a| a.ticket == ticket)
            .ok_or("carried clock ticket absent")?;
        let (_, prepared) = active
            .payload
            .carried
            .as_ref()
            .ok()
            .and_then(|p| p.as_ref())
            .ok_or("carried clip absent")?;
        let id = active
            .carried_owner()?
            .ok_or("constructed carried owner absent")?;
        let channels = active
            .resident_states
            .get(id)
            .and_then(|r| r.channels.as_ref())
            .ok_or("carried channels absent")?;
        let slot = u32::from(prepared.pickup_animation.slot);
        if slot >= 254
            || channels[0][0] != slot
            || channels
                .iter()
                .any(|c| c[0] != 254 && (c[0] != slot || f32::from_bits(c[5]) != 0.))
        {
            return Err("carried channels require unloaded clip or live model speed".into());
        }
        let mut durations = [None; 254];
        durations[slot as usize] = Some(prepared.template.duration);
        let tick = active.resident_states.advance_channels(
            id,
            &durations,
            [0.; 3],
            dt,
            giants_game::arithmetic::Precision::Double53,
        )?;
        if tick.missing != 0 {
            return Err("carried channel descriptor missing".into());
        }
        let channels = active
            .resident_states
            .get(id)
            .and_then(|r| r.channels.as_ref())
            .ok_or("carried clock retired")?;
        let clock = giants_game::animation_channels::clock(&channels[0]);
        let references =
            active
                .resident_states
                .update_references_unblended(id, slot as u8, clock.seconds)?;
        if references.cached_nodes == 0 {
            println!(
                "mission_actor_reference_cache ticket={ticket} class={} evaluated={} cached_nodes={} missing_nodes={} unblended=true cpu_owned=true actor_backend_ready=false",
                prepared.class,
                references.evaluated,
                references.cached_nodes,
                references.missing_nodes
            );
        }
        Ok(clock)
    }
    pub(crate) fn carried_reference(
        &self,
        ticket: u64,
    ) -> Result<giants_game::procedural_aim::Matrix, String> {
        let active = self
            .active
            .as_ref()
            .filter(|a| a.ticket == ticket)
            .ok_or("carried reference ticket absent")?;
        let id = active
            .carried_owner()?
            .ok_or("carried reference actor absent")?;
        active
            .resident_states
            .reference_matrix(
                id,
                giants_game::smartie_carry::MECC_CHILD_REFERENCE as usize,
            )
            .map_err(String::from)
    }
    pub(crate) fn mecc_preview_candidate(&self, carried_resident: bool) -> bool {
        self.active.as_ref().is_some_and(|active| {
            let p = &active.payload;
            p.packet.needs.is_empty()
                && p.world
                    .as_ref()
                    .is_ok_and(|w| !w.positions.is_empty() && !w.triangles.is_empty())
                && p.player.as_ref().is_ok_and(|p| p.start.kind == 2)
                && p.controller.is_ok()
                && p.parameters.as_ref().is_ok_and(|parameters| {
                    parameters.player_proxy34.is_none()
                        && (parameters.player_child12.is_none()
                            || carried_resident
                                && p.carried.as_ref().is_ok_and(|c| {
                                    c.as_ref().is_some_and(|(offset, actor)| {
                                        parameters.player_child12
                                            == Some((*offset, actor.class as u32))
                                    })
                                }))
                })
        })
    }
    pub(crate) fn startup(&self) -> Result<Option<Startup>, String> {
        let Some(active) = &self.active else {
            return Ok(None);
        };
        if active.start.intent == LaunchIntent::CompletionReplay {
            return Err("completion cinematic replay is an inspection context; gameplay admission is not available".into());
        }
        let p = &active.payload;
        if !p.packet.needs.is_empty() {
            return Err("mission data still has needs".into());
        }
        let world = p.world.as_ref().map_err(Clone::clone)?;
        if world.positions.is_empty() || world.triangles.is_empty() {
            return Err("mission terrain is empty".into());
        }
        if matches!(p.kabuto, Ok(Some(_))) {
            let owner = active.player_owner()?;
            active
                .resident_states
                .kabuto_group(owner)
                .ok_or("Kabuto initial group absent")?;
            return Err("Kabuto initialization and native controller are not connected yet".into());
        }
        let player = p.player.as_ref().map_err(Clone::clone)?;
        if player.start.kind != 2 || p.controller.is_err() {
            return Err("controlled Mecc is not prepared".into());
        }
        let parameters = p.parameters.as_ref().map_err(Clone::clone)?;
        if parameters.player_child12.is_some() {
            return Err("mission child12 requires an owned actor-service backend".into());
        }
        if parameters.parameter13.is_some() {
            return Err("startup parameter13 gameplay consumer is not connected".into());
        }
        if parameters.parameter15.is_some() {
            let objective = active
                .campaign
                .as_ref()
                .ok_or("collection objective owner not prepared")?
                .as_ref()
                .map_err(Clone::clone)?;
            if objective.program.quota != parameters.parameter15.map(|(_, v)| v) {
                return Err("collection objective quota differs from selected mission slot".into());
            }
            p.npcs
                .as_ref()
                .map_err(Clone::clone)?
                .as_ref()
                .ok_or("collection population owner absent")?
                .view(&active.resident_states, active.ticket)?;
        }
        if parameters.player_proxy34.is_some() {
            return Err("mission proxy34 requires an owned actor-service backend".into());
        }
        let opening = p.opening.as_ref().map_err(Clone::clone)?;
        if opening.director.is_none() && opening.source_offset.is_none() {
            return Err("absent opening request does not establish native defaults".into());
        }
        if !p.opening_effects_bound {
            return Err("opening renderer or effect dispatch is incomplete; preview only".into());
        }
        Ok(Some(Startup {
            ticket: active.ticket,
            cinematic: opening.director.is_some(),
            companions: player.requested_companions as u32,
        }))
    }
    pub(crate) fn take_opening(
        &mut self,
        ticket: u64,
    ) -> Result<Option<crate::mission_opening::Prepared>, String> {
        let active = self
            .active
            .as_mut()
            .filter(|a| a.ticket == ticket)
            .ok_or("opening ticket is not active")?;
        active
            .payload
            .playback
            .as_mut()
            .map(Option::take)
            .map_err(|e| e.clone())
    }
    pub(crate) fn idle(&self) -> bool {
        self.pending.is_none() && self.active.is_none()
    }
    pub fn active_ticket(&self) -> Option<u64> {
        self.active.as_ref().map(|a| a.ticket)
    }
    /// Partial first-living interaction, using the normal controller's actual
    /// owner, clock, noise, RNG and source Session. Other AI stays unavailable.
    #[cfg(test)]
    pub(crate) fn advance_normal_npcs(
        &mut self,
        ticket: u64,
        objects: &[giants_game::actor_environment::Collider],
    ) -> Result<Vec<crate::mission_audio::Event>, String> {
        struct Unavailable;
        impl giants_game::actor_retirement::Services for Unavailable {
            fn release(
                &mut self,
                _: &mut giants_game::resident_states::Session,
                _: giants_game::resident_states::Id,
            ) -> Result<(), String> {
                Err("CPU component requires an explicit retirement device".into())
            }
        }
        self.advance_normal_npcs_with_retirement(ticket, objects, &mut Unavailable)
    }
    pub(crate) fn advance_normal_npcs_with_retirement(
        &mut self,
        ticket: u64,
        objects: &[giants_game::actor_environment::Collider],
        device: &mut impl giants_game::actor_retirement::Services,
    ) -> Result<Vec<crate::mission_audio::Event>, String> {
        let a = self
            .active
            .as_mut()
            .filter(|a| a.ticket == ticket)
            .ok_or("NPC mission ticket absent")?;
        let owner = a.player_owner()?;
        if a.owned_mecc.is_some() || a.controlled_mecc_owner != Some(owner) {
            return Err("NPC component requires ordinary Mecc motion ownership".into());
        }
        let Some((sample, clock)) = a.payload.signals.current() else {
            return Err("NPC shared clock absent".into());
        };
        let areas = a.payload.signals.areas.snapshot(|id| {
            a.resident_states
                .get(id)
                .and_then(|a| a.construction_fields.as_ref())
                .map(|f| f.dispatch_context)
                .ok_or_else(|| "NPC slow-area owner absent".into())
        })?;
        let Some(npcs) = a.payload.npcs.as_mut().map_err(|e| e.clone())? else {
            return Ok(vec![]);
        };
        let world = a.payload.world.as_ref().map_err(|e| e.clone())?;
        npcs.advance_live(
            &mut a.resident_states,
            npc::live::Frame {
                ticket,
                sample,
                controlled: owner,
                world,
                objects,
                resources: a.payload.plan.resources(),
                tick: giants_game::vimp_wander::Tick {
                    global_delta: clock.seconds,
                    control_factor: 0.5,
                    skip_heading: false,
                    slow_areas: &areas,
                    now: clock.elapsed_seconds,
                    noise_time: a.payload.signals.noise.seconds,
                    noise_position: a.payload.signals.noise.position,
                },
            },
            &mut a.variant_random,
            device,
        )
    }
    pub(crate) fn hit_normal_vimp(
        &mut self,
        ticket: u64,
        target: giants_game::resident_states::Id,
        projectile: giants_game::mecc_pistol::WorldHit,
        camera: [f32; 3],
        device: &mut impl giants_game::actor_retirement::Services,
    ) -> Result<(bool, Vec<crate::mission_audio::Event>), String> {
        let a = self
            .active
            .as_mut()
            .filter(|a| a.ticket == ticket)
            .ok_or("Vimp hit mission ticket absent")?;
        let shooter = a.player_owner()?;
        let npcs = a
            .payload
            .npcs
            .as_mut()
            .map_err(|e| e.clone())?
            .as_mut()
            .ok_or("Vimp hit component absent")?;
        let world = a.payload.world.as_ref().map_err(|e| e.clone())?;
        npcs.hit(
            &mut a.resident_states,
            npc::live::combat::Hit {
                source: a.payload.plan.resources(),
                world,
                shooter,
                target,
                projectile,
                camera,
            },
            &mut a.variant_random,
            device,
        )
    }
    pub(crate) fn npc_target(&self, ticket: u64) -> Option<[f32; 3]> {
        let a = self
            .active
            .as_ref()
            .filter(|a| a.ticket == ticket && a.owned_mecc.is_none())?;
        let position = a
            .resident_states
            .get(a.player_owner().ok()?)?
            .construction_fields
            .as_ref()?
            .position;
        a.payload
            .npcs
            .as_ref()
            .ok()?
            .as_ref()?
            .live_target(&a.resident_states, position)
    }
    pub(crate) fn npc_reactions(&self, ticket: u64) -> Option<(u64, u64)> {
        let a = self.active.as_ref().filter(|a| a.ticket == ticket)?;
        Some(a.payload.npcs.as_ref().ok()?.as_ref()?.reactions())
    }
    pub(crate) fn meat_status(&self, ticket: u64) -> Option<(f32, u64)> {
        let a = self
            .active
            .as_ref()
            .filter(|a| a.ticket == ticket && a.owned_mecc.is_none())?;
        Some((
            a.resident_states.mecc_meat(a.player_owner().ok()?).ok()?,
            a.payload.npcs.as_ref().ok()?.as_ref()?.collected_chunks(),
        ))
    }
    pub(crate) fn take_campaign_completion(
        &mut self,
        ticket: u64,
    ) -> Result<Option<CampaignCompletion>, String> {
        let Some(a) = self.active.as_mut().filter(|a| a.ticket == ticket) else {
            return Ok(None);
        };
        let Some(c) = a.campaign.as_ref().and_then(|r| r.as_ref().ok()) else {
            return Ok(None);
        };
        if !c.completed || c.queued {
            return Ok(None);
        }
        let packet = &a.payload.packet;
        let world = packet.world.as_ref().ok_or("completed world absent")?;
        let mission = packet.mission.as_ref().ok_or("completed mission absent")?;
        let film = giants_game::mission_cinematics::select_completion(
            world,
            mission,
            &packet.request.mission_name,
        )?;
        let next = world
            .scenarios
            .get(c.program.scenario_index + 1)
            .ok_or("following source record absent")?;
        if next.kind != 2 {
            return Err(format!(
                "source record kind {} requires its continuation service",
                next.kind
            ));
        }
        let menu = giants_game::mission_selection::MissionMenu::parse(&packet.world_bytes, 0, true)
            .map_err(|e| e.to_string())?;
        let index = menu
            .entries
            .iter()
            .position(|m| m.name == next.name)
            .ok_or("next source mission absent from catalog")?;
        let request = menu
            .request(&packet.request.world, index)
            .map_err(|e| e.to_string())?;
        let carried = a.resident_states.mecc_meat(a.player_owner()?)?;
        let result = CampaignCompletion {
            ticket,
            current: packet.request.clone(),
            next: request,
            carried,
            counter: c.progress.counter,
            required: c.program.required,
            film: film.director,
        };
        a.campaign.as_mut().unwrap().as_mut().unwrap().queued = true;
        Ok(Some(result))
    }
    pub(crate) fn prepare_campaign_film(
        &self,
        ticket: u64,
        root: &std::path::Path,
    ) -> Result<mpsc::Receiver<Result<crate::mission_opening::Prepared, String>>, String> {
        let a = self
            .active
            .as_ref()
            .filter(|a| a.ticket == ticket)
            .ok_or("completion preparation ticket absent")?;
        if !a
            .campaign
            .as_ref()
            .and_then(|c| c.as_ref().ok())
            .is_some_and(|c| c.completed && c.queued)
        {
            return Err("completion preparation needs actual owned objectives".into());
        }
        let selection = a.payload.packet.completion(root)?;
        if selection.director.is_none() {
            return Err("completion source film absent".into());
        }
        let root = root.to_owned();
        let resources = Arc::clone(a.payload.plan.resources());
        let world = Arc::clone(a.payload.world.as_ref().map_err(Clone::clone)?);
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let result = (|| {
                let mut film =
                    crate::mission_opening::Prepared::load(&root, selection.director.as_deref())?
                        .ok_or("completion preparation produced no source film")?;
                film.prepare_render(&root, &resources, &world.config);
                if !film.effects_bound() {
                    return Err("completion rendering/effect owners are incomplete".into());
                }
                Ok(film)
            })();
            // A cancelled ticket drops this CPU-only result and all its resources.
            let _ = tx.send(result);
        });
        Ok(rx)
    }
    pub(crate) fn install_campaign_film(
        &mut self,
        ticket: u64,
        film: crate::mission_opening::Prepared,
    ) -> Result<(), String> {
        let a = self
            .active
            .as_mut()
            .filter(|a| a.ticket == ticket)
            .ok_or("completion installation ticket absent")?;
        if !a
            .campaign
            .as_ref()
            .and_then(|c| c.as_ref().ok())
            .is_some_and(|c| c.completed && c.queued)
        {
            return Err("completion installation lost objective ownership".into());
        }
        let playback = a.payload.playback.as_mut().map_err(|e| e.clone())?;
        if playback.is_some() {
            return Err("completion cannot overwrite an unconsumed film".into());
        }
        *playback = Some(film);
        Ok(())
    }
    pub(crate) fn collect_normal_meat(
        &mut self,
        ticket: u64,
        step: u64,
        seconds: f32,
        gameplay: bool,
    ) -> Result<Vec<crate::mission_audio::Event>, String> {
        let a = self
            .active
            .as_mut()
            .filter(|a| a.ticket == ticket)
            .ok_or("collection ticket absent")?;
        let owner = a.player_owner()?;
        if a.owned_mecc.is_some() || a.controlled_mecc_owner != Some(owner) {
            return Err("collection requires ordinary controlled Mecc".into());
        }
        if gameplay && a.campaign.is_none() {
            let result = campaign::State::prepare(&a.payload.packet, &a.payload.plan);
            match &result {
                Ok(c) => println!(
                    "mission_objectives_bound ticket={ticket} targets={} quota={:?} required={} native_world_precheck=false",
                    c.program.targets.len(),
                    c.program.quota,
                    c.program.required
                ),
                Err(reason) => {
                    println!("mission_objectives_unavailable ticket={ticket} reason={reason}")
                }
            }
            a.campaign = Some(result);
        }
        let Some(npc) = a.payload.npcs.as_mut().map_err(|e| e.clone())? else {
            return Ok(vec![]);
        };
        let (_, clock) = a
            .payload
            .signals
            .current()
            .ok_or("collection clock absent")?;
        let mut sounds = vec![];
        npc.collect(
            &mut a.resident_states,
            npc::collection::Tick {
                controlled: owner,
                ticket,
                step,
                seconds,
                now: clock.elapsed_seconds,
            },
            &mut a.variant_random,
            &mut sounds,
            if gameplay {
                a.campaign.as_mut().and_then(|r| r.as_mut().ok())
            } else {
                None
            },
        )?;
        Ok(sounds)
    }
    pub(crate) fn meat_target(&self, ticket: u64) -> Option<[f32; 3]> {
        let a = self
            .active
            .as_ref()
            .filter(|a| a.ticket == ticket && a.owned_mecc.is_none())?;
        let p = a
            .resident_states
            .get(a.player_owner().ok()?)?
            .construction_fields
            .as_ref()?
            .position;
        a.payload
            .npcs
            .as_ref()
            .ok()?
            .as_ref()?
            .meat_target(&a.resident_states, p)
    }
    pub(crate) fn campaign_return_target(&self, ticket: u64) -> Option<[f32; 3]> {
        let a = self.active.as_ref().filter(|a| a.ticket == ticket)?;
        let c = a.campaign.as_ref()?.as_ref().ok()?;
        let carried = a.resident_states.mecc_meat(a.player_owner().ok()?).ok()?;
        if f64::from(carried) < f64::from(c.program.quota?) {
            return None;
        }
        c.program
            .targets
            .iter()
            .find(|(_, t)| t.response_flags & 15 == 4)
            .map(|(_, t)| t.position)
    }
    pub(crate) fn npc_hint(&self, ticket: u64) -> Option<String> {
        let a = self
            .active
            .as_ref()
            .filter(|a| a.ticket == ticket && a.owned_mecc.is_none())?;
        let npc = a.payload.npcs.as_ref().ok()?.as_ref()?;
        let player = a
            .resident_states
            .get(a.player_owner().ok()?)?
            .construction_fields
            .as_ref()?;
        let meat = a.resident_states.mecc_meat(a.player_owner().ok()?).ok()?;
        if let Some(target) = self.campaign_return_target(ticket) {
            let dx = target[0] - player.position[0];
            let dy = target[1] - player.position[1];
            let turn =
                (dx.atan2(-dy).to_degrees() - player.angles[0] + 180.).rem_euclid(360.) - 180.;
            let direction = if turn.abs() < 10. {
                "ahead".to_owned()
            } else {
                format!(
                    "{} {:.0} deg",
                    if turn > 0. { "left" } else { "right" },
                    turn.abs()
                )
            };
            return Some(format!(
                "Meat: {meat:.0} / 20 | Return to the objective destination.\nDestination: {:.0} units, {direction}.",
                dx.hypot(dy)
            ));
        }
        let hint = npc
            .live_hint(&a.resident_states, player.position, player.angles[0])
            .unwrap_or_default();
        Some(format!(
            "Meat: {meat:.0} / 20 | Walk over settled meat to collect.\n{hint}"
        ))
    }
    /// Copied canonical particles; this read neither advances clocks nor RNG.
    pub(crate) fn npc_blood_quads(&self, ticket: u64) -> Vec<giants_game::blood_particles::Quad> {
        self.active
            .as_ref()
            .filter(|a| a.ticket == ticket)
            .and_then(|a| a.payload.npcs.as_ref().ok()?.as_ref())
            .map(npc::State::blood_quads)
            .unwrap_or_default()
    }
    pub(crate) fn npc_effect_draws(
        &self,
        ticket: u64,
        view: [f32; 16],
    ) -> Result<Vec<NpcDraw>, String> {
        let Some(npcs) = self
            .active
            .as_ref()
            .filter(|a| a.ticket == ticket)
            .and_then(|a| a.payload.npcs.as_ref().ok()?.as_ref())
        else {
            return Ok(vec![]);
        };
        npcs.effect_draws(view)
    }
    pub(crate) fn npc_draws(&mut self, ticket: u64) -> Result<Vec<NpcDraw>, String> {
        let active = self
            .active
            .as_mut()
            .filter(|a| a.ticket == ticket)
            .ok_or("NPC draw mission ticket absent")?;
        match active.payload.npcs.as_mut().map_err(|e| e.clone())? {
            None => Ok(Vec::new()),
            Some(npcs) => npcs.draws(
                &mut active.resident_states,
                ticket,
                active.payload.plan.resources(),
            ),
        }
    }
    pub(crate) fn npc_contact_snapshots(
        &mut self,
        ticket: u64,
    ) -> Result<Vec<giants_game::vimp_contacts::Snapshot>, String> {
        let active = self
            .active
            .as_mut()
            .filter(|a| a.ticket == ticket)
            .ok_or("NPC contact mission ticket absent")?;
        if active.owned_mecc.is_some()
            || !active
                .payload
                .world
                .as_ref()
                .map_err(Clone::clone)?
                .name
                .eq_ignore_ascii_case("Story2")
        {
            return Ok(Vec::new());
        }
        match active
            .payload
            .npcs
            .as_mut()
            .map_err(|error| error.clone())?
        {
            None => Ok(Vec::new()),
            Some(npcs) => npcs.contact_snapshots(
                &mut active.resident_states,
                ticket,
                active.payload.plan.resources(),
            ),
        }
    }
    pub(crate) fn messages(&self, ticket: u64) -> Option<&giants_game::mission_messages::Queue> {
        Some(
            self.active
                .as_ref()
                .filter(|a| a.ticket == ticket)?
                .resident_states
                .messages(),
        )
    }
    /// Explicit diagnostic producer, never an acknowledgement of an actor notice.
    pub(crate) fn probe_message(
        &mut self,
        ticket: u64,
        text: &[u8],
        now: f32,
    ) -> Result<(), String> {
        if !matches!(
            std::env::var("GIANTS_MESSAGE_PROBE").ok().as_deref(),
            Some("render" | "expire")
        ) {
            return Err("message probe is not enabled".into());
        }
        self.active
            .as_mut()
            .filter(|a| a.ticket == ticket)
            .ok_or("message ticket absent")?
            .resident_states
            .post_message(text, 6, now)
            .map_err(str::to_owned)
    }
    pub(crate) fn player_candidate(
        &self,
    ) -> Option<(u64, Arc<giants_game::player_initial::Prepared>)> {
        let active = self.active.as_ref()?;
        active
            .payload
            .player
            .as_ref()
            .ok()
            .map(|p| (active.ticket, Arc::clone(p)))
    }
    pub fn environment_candidate(&self) -> Result<Option<EnvironmentCandidate>, String> {
        let Some(active) = &self.active else {
            return Ok(None);
        };
        let asset = Arc::clone(active.payload.world.as_ref().map_err(Clone::clone)?);
        if let Some(result) = &active.payload.kabuto_camera_inspection {
            let camera = result.as_ref().map_err(Clone::clone)?;
            if active.player_owner()? != camera.owner || !camera.current(&active.resident_states) {
                return Err("Kabuto inspection camera owner or header changed".into());
            }
            let pose = camera.pose;
            return Ok(Some(EnvironmentCandidate {
                ticket: active.ticket,
                asset,
                camera: crate::validation_camera::ValidationCamera {
                    view: crate::mecc_dev_scene::camera_view(
                        Vec3::from(pose.eye),
                        Vec3::from(pose.forward),
                        Vec3::from(pose.up),
                    )
                    .to_cols_array(),
                    projection: camera.projection.matrix,
                    viewport: UVec4::new(0, 0, 1920, 1080),
                },
                models: Arc::clone(&active.payload.models),
                eye: pose.eye,
                lod_scale: 1.,
                camera_source: "kabuto_profile_inspection",
            }));
        }
        // Gameplay-world inspection must not require a supported opening Flick.
        // This is only a declared-start inspection camera, not a playback bind.
        if let Ok(player) = &active.payload.player {
            let pose = giants_game::mecc_camera::pose(
                player.start.position,
                player.start.angles[0],
                0.,
                false,
            )
            .map_err(str::to_owned)?;
            let view = crate::mecc_dev_scene::camera_view(
                Vec3::from(pose.eye),
                Vec3::from(pose.forward),
                Vec3::from(pose.up),
            );
            return Ok(Some(EnvironmentCandidate {
                ticket: active.ticket,
                asset,
                camera: crate::validation_camera::ValidationCamera {
                    view: view.to_cols_array(),
                    projection: giants_game::mecc_camera::projection(1920. / 1080., 25000.)
                        .map_err(str::to_owned)?,
                    viewport: UVec4::new(0, 0, 1920, 1080),
                },
                models: Arc::clone(&active.payload.models),
                eye: pose.eye,
                lod_scale: 1.,
                camera_source: "mission_player_inspection",
            }));
        }
        if let Ok(start) = &active.payload.start_candidate {
            let pose = giants_game::mecc_camera::pose(start.position, start.angles[0], 0., false)
                .map_err(str::to_owned)?;
            let view = crate::mecc_dev_scene::camera_view(
                Vec3::from(pose.eye),
                Vec3::from(pose.forward),
                Vec3::from(pose.up),
            );
            return Ok(Some(EnvironmentCandidate {
                ticket: active.ticket,
                asset,
                camera: crate::validation_camera::ValidationCamera {
                    view: view.to_cols_array(),
                    projection: giants_game::mecc_camera::projection(1920. / 1080., 100000.)
                        .map_err(str::to_owned)?,
                    viewport: UVec4::new(0, 0, 1920, 1080),
                },
                models: Arc::clone(&active.payload.models),
                eye: pose.eye,
                lod_scale: 1.,
                camera_source: "declared_start_debug_camera",
            }));
        }
        if !active
            .payload
            .director
            .as_ref()
            .is_ok_and(|d| d.as_ref().is_some_and(|d| d.binding_candidate.is_ok()))
        {
            let terrain = &asset.terrain;
            let eye = [
                terrain.origin[0] + (terrain.width - 1) as f32 * terrain.spacing * 0.5,
                terrain.origin[1] + (terrain.height - 1) as f32 * terrain.spacing * 0.5,
                terrain.height_bounds[1] + 200.,
            ];
            let view = crate::mecc_dev_scene::camera_view(
                Vec3::from(eye),
                Vec3::new(0., -1., -0.3),
                Vec3::Z,
            );
            return Ok(Some(EnvironmentCandidate {
                ticket: active.ticket,
                asset,
                camera: crate::validation_camera::ValidationCamera {
                    view: view.to_cols_array(),
                    projection: giants_game::mecc_camera::projection(1920. / 1080., 100000.)
                        .map_err(str::to_owned)?,
                    viewport: UVec4::new(0, 0, 1920, 1080),
                },
                models: Arc::clone(&active.payload.models),
                eye,
                lod_scale: 1.,
                camera_source: "terrain_debug_camera",
            }));
        }
        let director = active
            .payload
            .director
            .as_ref()
            .map_err(Clone::clone)?
            .as_ref()
            .ok_or("no selected cinematic camera candidate")?;
        let binding = director.binding_candidate.as_ref().map_err(Clone::clone)?;
        let camera = giants_game::animation::Evaluator {
            animation: &director.animation,
        }
        .camera_named_in_context(
            binding.tree.as_deref(),
            &binding.name,
            director.reset_tick,
            giants_game::cinematic_viewport::Context {
                extent: [1920, 1080],
                fraction: 1.,
            },
        )
        .map_err(|e| e.to_string())?;
        Ok(Some(EnvironmentCandidate {
            ticket: active.ticket,
            asset,
            camera: crate::cinematic_camera::convert(&camera),
            models: Arc::clone(&active.payload.models),
            eye: camera.origin,
            lod_scale: camera.lod_scale,
            camera_source: "mission_candidate",
        }))
    }
}
pub(crate) fn live(ticket: u64, current: u64, phase: Phase) -> bool {
    ticket == current
        && matches!(
            phase,
            Phase::AwaitingRuntime | Phase::Cinematic | Phase::Objectives | Phase::Gameplay
        )
}
fn clear(runtime: &mut Runtime, commands: &mut Commands) {
    runtime.pending = None;
    runtime.failure = None;
    runtime.smoke_geometry.clear();
    if let Some(mut active) = runtime.active.take() {
        let (field_actors, field_guns, field_shadows) = active.resident_states.actor_field_counts();
        let (released_state_sets, released_state_references) =
            active.resident_states.state_resource_counts();
        let released_specifications = active.resident_states.specification_count();
        let released_declarations = active.resident_states.declaration_count();
        let released = active.resident_states.clear();
        let remaining = active.resident_states.counts();
        let (remaining_fields, _, _) = active.resident_states.actor_field_counts();
        println!(
            "mission_declarations_released ticket={} bindings={} remaining={} live_callbacks=false ready=false",
            active.ticket,
            released_declarations,
            active.resident_states.declaration_count()
        );
        println!(
            "mission_actor_fields_released ticket={} actors={field_actors} gunlist_entries={field_guns} shadow_descriptors={field_shadows} remaining={remaining_fields} live_callbacks=false ready=false",
            active.ticket
        );
        println!(
            "mission_resident_states_released ticket={} residents={} bytes={} registrations={} remaining={} remaining_bytes={} remaining_registrations={} live_callbacks=false ready=false",
            active.ticket,
            released.residents,
            released.payload_bytes,
            released.registrations,
            remaining.residents,
            remaining.payload_bytes,
            remaining.registrations
        );
        println!(
            "mission_resident_specifications_released ticket={} resources={} remaining={} live_callbacks=false ready=false",
            active.ticket,
            released_specifications,
            active.resident_states.specification_count()
        );
        let (state_sets, state_references) = active.resident_states.state_resource_counts();
        println!(
            "mission_resident_state_resources_released ticket={} sets={released_state_sets} references={released_state_references} remaining_sets={state_sets} remaining_references={state_references} native_pointers_written=false ready=false",
            active.ticket
        );
        if let Ok(proxies) = &active.payload.effect_proxies {
            println!(
                "mission_cinematic_proxies_released ticket={} prepared={} active_queue={} ownership=cpu_ticket",
                active.ticket,
                proxies.len(),
                proxies.iter().filter(|p| p.active()).count()
            );
        }
        commands.entity(active.root).try_despawn();
        runtime.retired.push(active.ticket);
        println!(
            "mission_session_release_requested ticket={} owned_actors={}",
            active.ticket, active.actors
        );
    }
}
fn cancel_ticket(runtime: &mut Runtime, commands: &mut Commands, ticket: u64) {
    if runtime.pending.as_ref().is_some_and(|p| p.ticket == ticket)
        || runtime.active.as_ref().is_some_and(|a| a.ticket == ticket)
    {
        clear(runtime, commands);
    }
}
fn actor(
    commands: &mut Commands,
    root: Entity,
    reference: ActorRef,
    class: i32,
    matrix: [f32; 16],
) -> Result<(), String> {
    let basis = Mat4::from_cols(Vec4::X, -Vec4::Z, Vec4::Y, Vec4::W);
    let matrix = basis * Mat4::from_cols_array(&matrix).transpose() * basis.inverse();
    let transform = Transform::from_matrix(matrix);
    if !transform.translation.is_finite()
        || !transform.rotation.is_finite()
        || !transform.scale.is_finite()
    {
        return Err("non-finite authored pose candidate".into());
    }
    commands.spawn((
        MissionOwned(reference.ticket),
        reference,
        Name::new(format!("Source class {class} · initialization pending")),
        transform,
        GlobalTransform::default(),
        Visibility::Hidden,
        ChildOf(root),
    ));
    Ok(())
}
fn spawn(
    commands: &mut Commands,
    ticket: u64,
    payload: &mut Payload,
) -> (
    Entity,
    usize,
    Vec<String>,
    giants_game::resident_states::Session,
    ActorBindings,
) {
    payload.kabuto_camera_inspection = None;
    let root = commands
        .spawn((
            MissionOwned(ticket),
            Name::new(format!(
                "Mission {} / {}",
                payload.packet.request.world, payload.packet.request.mission_name
            )),
            Transform::default(),
            GlobalTransform::default(),
            Visibility::Hidden,
        ))
        .id();
    let mut actors = 0;
    let mut blocked = vec![
        "complete scene rendering, actor callbacks, cinematic and controller are not connected"
            .into(),
    ];
    if let Err(error) = &payload.world {
        blocked.push(format!("selected environment: {error}"));
    }
    if let Err(error) = &payload.opening {
        blocked.push(format!("mission opening: {error}"));
    }
    if let Err(error) = &payload.carried {
        blocked.push(format!("carried source-pose preview: {error}"));
    }
    match &payload.start_candidate {
        Err(error) => blocked.push(format!("initial player marker: {error}")),
        Ok(start) if start.kind == 3 => blocked.push(format!(
            "Delphi declared marker {}: native relocation and playable controller admission pending",
            start.marker_offset
        )),
        Ok(start) => blocked.push(format!(
            "initial player marker {}: continuity, team formation and live player binding pending",
            start.marker_offset
        )),
    }
    match &payload.player {
        Ok(player) => {
            let mut entity = commands.spawn((
                MissionOwned(ticket),
                PlayerInitial {
                    ticket,
                    prepared: Arc::clone(player),
                },
                Name::new("Mecc initial player / live binding pending"),
                ChildOf(root),
            ));
            match &mut payload.controller {
                Ok(controller) => {
                    if let Some(controller) = controller.take() {
                        entity.insert(crate::mission_player::Controller::new(ticket, controller));
                        println!(
                            "mission_player_controller_prepared ticket={ticket} equipment=selected_cold replay_override=false ready=false"
                        );
                    }
                }
                Err(error) => blocked.push(format!("player controller preparation: {error}")),
            }
            println!(
                "mission_player_initial ticket={ticket} class={} companions={} weapons={:?} utilities={:?} body={} parts={} state={} position={:?} cold_equipment=true continuity=false controller=false ready=false",
                player.start.kind,
                player.requested_companions,
                player.equipment.weapons,
                player.equipment.utility,
                player.body.model,
                player.body.mesh.subobjects.len(),
                player.state_seed.storage.state(),
                player.start.position
            );
            blocked.push(
                "player special children, continuity, team and controller binding pending".into(),
            );
        }
        Err(error)
            if !matches!(payload.kabuto, Ok(Some(_))) && !requests_delphi(&payload.packet) =>
        {
            blocked.push(format!("initial player construction: {error}"));
        }
        Err(_) => {}
    }
    match &payload.parameters {
        Err(error) => blocked.push(format!("mission slot parameters: {error}")),
        Ok(parameters) => {
            if parameters.parameter13.is_some() {
                blocked.push(format!(
                    "mission slot {}: parameter13 runtime consumer unresolved",
                    parameters.index
                ));
            }
            for (offset, class_id) in &parameters.linked_actors12 {
                blocked.push(format!(
                    "mission slot {}: command12 at {} class {} requires actor services",
                    parameters.index, offset, class_id
                ));
            }
            if parameters.parameter15.is_some()
                && let Err(reason) = campaign::State::prepare(&payload.packet, &payload.plan)
            {
                blocked.push(format!(
                    "mission slot {}: objective consumer unavailable: {reason}",
                    parameters.index
                ));
            }
            if parameters.completion_cinematic.is_some() {
                blocked.push(format!(
                    "mission slot {}: completion cinematic gate not connected",
                    parameters.index
                ));
            }
        }
    }
    match &payload.director {
        Err(error) => blocked.push(format!("director preparation: {error}")),
        Ok(Some(director)) => {
            match &director.effect_requests {
                Ok(requests) => {
                    for request in requests {
                        let target = request.target(payload.plan.resources());
                        let program = target.as_ref().map_err(Clone::clone).and_then(|target| {
                            payload.plan.resources().bound_effect_program(target)
                        });
                        println!(
                            "mission_cinematic_effect_request ticket={ticket} slot={} actor={} mode={} effect={:?} node={:?} target={target:?} program={:?} executed=false rendered=false ready=false",
                            request.slot,
                            request.actor,
                            request.mode,
                            request.effect,
                            request.node,
                            program.as_ref().map(|p| p.records.len())
                        );
                        if request.slot >= 0 {
                            let terminal = program
                                .as_ref()
                                .is_ok_and(giants_game::cinematic_fx::terminator_only);
                            let prepared = payload.effect_proxies.as_ref().is_ok_and(|proxies| {
                                proxies
                                    .iter()
                                    .any(|p| p.placement.slot == request.slot as usize)
                            });
                            println!(
                                "mission_cinematic_effect_profile ticket={ticket} slot={} terminator_only={terminal} placement_prepared={prepared} live_registered=false ready=false",
                                request.slot
                            );
                            blocked.push(format!(
                                "cinematic effect slot {}: {}",
                                request.slot,
                                if prepared {
                                    "normal director dispatch and owned effect updates pending"
                                } else {
                                    "execution, animated attachment and rendering pending"
                                }
                            ));
                        }
                    }
                }
                Err(error) => blocked.push(format!("cinematic effect requests: {error}")),
            }
            if let Err(error) = &director.program {
                blocked.push(format!("director execution profile: {error}"));
            }
            if let Err(error) = &director.camera_candidate {
                blocked.push(format!("initial camera candidate: {error}"));
            }
        }
        Ok(None) => {}
    }
    match &payload.effect_proxies {
        Ok(proxies) => {
            for proxy in proxies {
                println!(
                    "mission_cinematic_proxy_prepared ticket={ticket} slot={} actor={:?} node={} position_bits={:?} mode={} active_queue={} cursor={} live_registered=false ready=false",
                    proxy.placement.slot,
                    proxy.placement.actor,
                    proxy
                        .placement
                        .node
                        .as_ref()
                        .map_or("-".into(), |n| n.identity()),
                    proxy.placement.position.map(f32::to_bits),
                    proxy.placement.mode,
                    proxy.active(),
                    proxy.cursor()
                );
            }
        }
        Err(error) => blocked.push(format!("cinematic effect proxy preparation: {error}")),
    }
    for (index, entry) in payload.plan.entries.iter().enumerate() {
        let result = entry
            .source
            .as_ref()
            .map_err(Clone::clone)
            .and_then(|source| {
                actor(
                    commands,
                    root,
                    ActorRef {
                        ticket,
                        index,
                        world_declaration: false,
                    },
                    source.class,
                    entry
                        .transform(giants_game::instances::Adjustment::Authored)?
                        .matrix,
                )
            });
        match result {
            Ok(()) => actors += 1,
            Err(error) => blocked.push(format!("mission slot {index}: {error}")),
        }
    }
    for (index, entry) in payload.plan.world_entries.iter().enumerate() {
        let result = entry.class_input().and_then(|source| {
            actor(
                commands,
                root,
                ActorRef {
                    ticket,
                    index,
                    world_declaration: true,
                },
                source.class,
                entry.initial_matrix()?,
            )
        });
        match result {
            Ok(()) => actors += 1,
            Err(error) => blocked.push(format!("world slot {index}: {error}")),
        }
    }
    if let Some(mission) = &payload.packet.mission {
        for child in &mission.locked_children {
            blocked.push(format!(
                "mission child at {}: constructor/parent transform pending",
                child.source_offset
            ));
        }
    }
    for result in payload
        .plan
        .mission_state_results
        .iter()
        .chain(&payload.plan.mission_object_results)
    {
        if let Err(error) = &result.result {
            blocked.push(format!("update at {}: {error}", result.source_offset));
        }
    }
    let mut resident_states = giants_game::resident_states::Session::new(ticket);
    let mut bindings_by_role = ActorBindings::default();
    payload.delphi_body = None;
    match &payload.delphi {
        Ok(Some(factory)) => {
            let attempt = factory.construct_declared(
                &mut resident_states,
                giants_game::animation_channels::Blending::Enabled,
            );
            bindings_by_role.player = attempt.owner;
            bindings_by_role.delphi_projection = Some(attempt.projection);
            match attempt.result {
                Ok(initial) => {
                    match giants_game::player_body::Prepared::from_session(
                        payload.plan.resources(),
                        &resident_states,
                        initial.owner,
                    ) {
                        Ok(body) => payload.delphi_body = Some(Arc::new(body)),
                        Err(error) => blocked.push(format!("Delphi body preparation: {error}")),
                    }
                    println!(
                        "mission_delphi_constructed ticket={ticket} owner={:?} class=3 held={:?} copies={} position={:?} projection_checkpoint={:?} declared_placement=true native_relocation_verified=false controller=false ready=false",
                        initial.owner,
                        initial.special.held,
                        initial.special.copies.len(),
                        factory.declared_start().position,
                        attempt.projection
                    );
                    let projection = if bindings_by_role.delphi_projection
                        != Some(giants_game::delphi_player::ProjectionStatus::Completed)
                    {
                        "pre-special projection callback, "
                    } else {
                        ""
                    };
                    blocked.push(format!("Delphi {projection}live animation, physical/camera/input and controller admission pending"));
                }
                Err(error) => blocked.push(format!(
                    "Delphi cold initialization owner={:?}: {error}",
                    attempt.owner
                )),
            }
        }
        Err(error) => blocked.push(format!("Delphi preparation: {error}")),
        Ok(None) if requests_delphi(&payload.packet) => {
            blocked.push("Delphi cold factory absent for requested player".into());
        }
        Ok(None) => {}
    }
    match &payload.kabuto {
        Ok(Some(factory)) => match factory.construct_declared(
            &mut resident_states,
            giants_game::animation_channels::Blending::Enabled,
            1,
        ) {
            Ok(owner) => {
                bindings_by_role.player = Some(owner);
                match giants_game::player_body::Prepared::from_session(
                    payload.plan.resources(),
                    &resident_states,
                    owner,
                ) {
                    Ok(body) => payload.kabuto_body = Some(Arc::new(body)),
                    Err(error) => blocked.push(format!("Kabuto body preparation: {error}")),
                }
                println!(
                    "mission_kabuto_constructed ticket={ticket} owner={owner:?} class=1 position={:?} initializer_pending={:?} declared_placement=true native_relocation_verified=false controller=false ready=false",
                    factory.declared_start().position,
                    resident_states.kabuto_group(owner).unwrap().pending,
                );
                blocked.push("Kabuto projection, animation cache, material, geometry and live controller admission pending".into());
            }
            Err(error) => blocked.push(format!("Kabuto cold construction: {error}")),
        },
        Err(error) => blocked.push(format!("Kabuto preparation: {error}")),
        Ok(None) => {}
    }
    let mut register = |role: &str,
                        class: i32,
                        seed: &giants_game::state_storage::Seed,
                        bindings: &[giants_game::model_registry::StateBinding],
                        selection: Option<giants_game::animation_channels::Request>,
                        state_specifications: Option<
        &giants_game::state_resources::Specifications,
    >,
                        pose: Option<DeclaredActorPose>| {
        let result = (|| -> Result<(), String> {
            let definition = payload
                .plan
                .resources()
                .classes
                .models
                .first_program(class)
                .ok_or("resident class program absent")?;
            let program = definition.program.as_ref().map_err(Clone::clone)?;
            let flags = giants_game::object_lists::DirectFlags::project(class, program);
            let specification = Arc::new(giants_game::specification::Prepared::load_registered(
                payload.plan.resources(),
                class,
            )?);
            let specification_summary = format!(
                "mission_resident_specification ticket={ticket} role={role} class={class} specification={} models={} animations={} references={} events={} needs={} cpu_owned=true event_dispatch=false resource_pointers_bound=false ready=false",
                specification.specification,
                specification.models.iter().flatten().count(),
                specification.animations.len(),
                specification.metadata.references.names.len(),
                specification
                    .metadata
                    .events
                    .iter()
                    .map(|t| t.events.len())
                    .sum::<usize>(),
                specification.needs.len()
            );
            let states = Arc::new(giants_game::state_resources::Prepared::load_reusing(
                payload.plan.resources(),
                class,
                giants_game::state_storage::Profile::Debug,
                state_specifications,
            )?);
            let (specs, effects) = states.counts();
            let needs = states
                .needs()
                .iter()
                .map(|(offset, reason)| format!("{offset}: {reason}"))
                .collect::<Vec<_>>();
            if !needs.is_empty() {
                return Err(format!("state resources: {}", needs.join("; ")));
            }
            let shared = match state_specifications {
                Some(prepared) => prepared.iter().all(|(offset, spec)| {
                    match (spec, states.specification(*offset)) {
                        (Ok(before), Ok(after)) => Arc::ptr_eq(before, after),
                        _ => false,
                    }
                }),
                None => false,
            };
            let state_summary = format!(
                "mission_resident_state_resources ticket={ticket} role={role} class={class} specifications={specs} effects={effects} needs=0 shared_with_prepared_player={shared} native_pointers_written=false live_callbacks=false ready=false"
            );
            let template = giants_game::actor_fields::Fields::load(
                payload.plan.resources(),
                class,
                &specification,
            )?;
            let fields = match pose {
                Some((position, angles)) => {
                    template.placed(giants_game::collider_init::DeclaredPose {
                        class: class as u32,
                        position,
                        yaw: angles[0],
                        pitch_roll: Some([angles[1], angles[2]]),
                    })?
                }
                None => template.instantiate(),
            };
            let guns = fields.gunlist.len();
            let shadow = fields.shadow.is_some();
            let id = resident_states.construct(giants_game::resident_states::Construction {
                class,
                seed,
                bindings,
                flags,
                specification,
                state_resources: states,
                fields,
            })?;
            // Retain the real constructor result even if a later initializer
            // fails. A class lookup must never substitute another instance.
            match role {
                "player" => bindings_by_role.player = Some(id),
                "carried" => bindings_by_role.carried = Some(id),
                _ => {}
            }
            if role == "player" && class == 2 {
                let mut factory = giants_game::mecc_special::Factory::new(payload.plan.resources());
                let initialized = factory.initialize_player_fields(
                    &mut resident_states,
                    id,
                    giants_game::animation_channels::Blending::Enabled,
                )?;
                println!(
                    "mission_mecc_special ticket={ticket} owner={id:?} children={:?} classes=84,85,781 constructor_mask=2 context=0 equipment_controlled={} cold_phase=true player_fields=true blending=enabled projection_reset_pending=true activation_complete=false child_callbacks=false child_rendering=false ready=false",
                    initialized.children, initialized.controlled
                );
                let fields = resident_states
                    .get(id)
                    .unwrap()
                    .construction_fields
                    .as_ref()
                    .unwrap();
                println!(
                    "mission_mecc_special_source ticket={ticket} position_bits={:?} angles_bits={:?} eligibility_bit={} ready=false",
                    fields.position.map(f32::to_bits),
                    fields.angles.map(f32::to_bits),
                    fields.configuration[14] & 1
                );
                let health = resident_states.mecc_health(id)?;
                let view = resident_states.perception_subset(&[id])?;
                println!(
                    "mission_mecc_owned_health ticket={ticket} damage_bits={} limit_bits={} shield_bits={} candidates={} list1_members={} actual_owned_fields=true population_complete=false ready=false",
                    health.damage.to_bits(),
                    health.limit.to_bits(),
                    health.shield.to_bits(),
                    view.candidates.len(),
                    view.order.len()
                );
                for (index, child) in initialized.children.into_iter().enumerate() {
                    let actor = resident_states.get(child).unwrap();
                    let fields = actor.construction_fields.as_ref().unwrap();
                    let relation = resident_states.mecc_special_child(child).unwrap();
                    println!(
                        "mission_mecc_special_child ticket={ticket} index={index} class={} state={} mask={} parent_matches={} reference={} binding={} context={} flags={} channel_flags={} position_bits={:?} current_bits={:?} previous_bits={:?} angles_bits={:?} scale_bits={} ready=false",
                        actor.class,
                        actor.seed.storage.state(),
                        actor.constructor_mask,
                        relation.parent == id,
                        relation.reference_word,
                        relation.binding_word,
                        relation.context,
                        fields.object_flags,
                        actor.channels.unwrap()[0][8],
                        fields.position.map(f32::to_bits),
                        fields.current.map(f32::to_bits),
                        fields.previous.map(f32::to_bits),
                        fields.angles.map(f32::to_bits),
                        fields.scale.to_bits()
                    );
                }
            }
            if let Some(request) = selection {
                resident_states.select_primary(
                    id,
                    request,
                    giants_game::animation_channels::Blending::Disabled,
                )?;
            }
            let resident = resident_states
                .get(id)
                .ok_or("registered resident absent")?;
            println!(
                "mission_resident_state ticket={ticket} role={role} class={} state={} bytes={} references={} object_commands_pending={} mask={} live_callbacks=false resource_pointers_bound=false ready=false",
                resident.class,
                resident.seed.storage.state(),
                resident.seed.storage.data().map_or(0, <[u8]>::len),
                resident.bindings.len(),
                resident.seed.unexecuted.len(),
                resident.constructor_mask
            );
            let channels = resident
                .channels
                .as_ref()
                .ok_or("resident channels absent")?;
            println!(
                "mission_resident_channels ticket={ticket} role={role} primary={} flags={} selection_applied={} stored_transition={} channel_blending=false pose_blending=false live_callbacks=false ready=false",
                channels[0][0],
                channels[0][8],
                selection.is_some(),
                f32::from_bits(channels[0][6])
            );
            println!("{specification_summary}");
            println!("{state_summary}");
            println!(
                "mission_owned_constructor ticket={ticket} role={role} class={class} resources_published=true initial_channels_before_selection=true live_callbacks=false ready=false"
            );
            println!(
                "mission_actor_fields ticket={ticket} role={role} class={class} configuration_words=30 gunlist_entries={guns} shadow_descriptor={shadow} pose_provided={} native_pointers_written=false live_callbacks=false ready=false",
                pose.is_some()
            );
            Ok(())
        })();
        if let Err(error) = result {
            blocked.push(format!("{role} resident state: {error}"));
        }
    };
    if let Ok(player) = &payload.player {
        for (offset, specification) in &player.state_specifications {
            match specification {
                Ok(specification) => println!(
                    "mission_player_state_specification ticket={ticket} offset={offset} specification={} models={} animations={} corrections={} needs={} cpu_owned=true rendered=false actor_backend_ready=false",
                    specification.specification,
                    specification.models.iter().flatten().count(),
                    specification.animations.len(),
                    specification.corrections.len(),
                    specification.needs.len(),
                ),
                Err(error) => println!(
                    "mission_player_state_specification_refused ticket={ticket} offset={offset} reason={error:?} actor_backend_ready=false",
                ),
            }
        }
        register(
            "player",
            player.start.kind,
            &player.state_seed,
            &player.state_bindings,
            None,
            Some(&player.state_specifications),
            Some((player.start.position, player.start.angles)),
        );
    }
    if let Ok(Some((_, carried))) = &payload.carried {
        register(
            "carried",
            carried.class,
            &carried.seed,
            &carried.state_bindings,
            Some(carried.pickup_animation),
            None,
            None,
        );
    }
    if let (Ok(player), Ok(Some((_, carried)))) = (&payload.player, &payload.carried) {
        let binding = (|| -> Result<(), String> {
            let parent = bindings_by_role
                .player
                .ok_or("constructed player owner absent")?;
            let child = bindings_by_role
                .carried
                .ok_or("constructed carried owner absent")?;
            resident_states.set_presentation_parent(child, Some(parent))?;
            println!(
                "mission_actor_audio_parent ticket={ticket} child_class={} parent_class={} presentation_only=true actor_backend_ready=false",
                carried.class, player.start.kind
            );
            Ok(())
        })();
        if let Err(error) = binding {
            blocked.push(format!("actor audio parent: {error}"));
        }
    }
    let declared =
        giants_game::declared_residents::construct_acquired(&payload.plan, &mut resident_states);
    let herd_declarations = declared
        .records
        .iter()
        .filter_map(|r| r.result.as_ref().ok())
        .filter(|&&id| resident_states.herd_clock(id).is_some())
        .count();
    let herd_markers = declared
        .records
        .iter()
        .filter_map(|r| r.result.as_ref().ok())
        .filter(|&&id| resident_states.herd_marker(id).is_some())
        .count();
    let mission_markers_loaded = declared
        .records
        .iter()
        .filter(|r| r.key.scope == giants_game::declared_residents::Scope::Mission)
        .filter_map(|r| r.result.as_ref().ok())
        .filter(|&&id| resident_states.herd_marker(id).is_some())
        .count();
    println!(
        "mission_declared_residents ticket={ticket} constructed={} refused={} other_states={} declaration_bindings={} herd_declarations={herd_declarations} herd_markers={herd_markers} mission_markers_loaded={mission_markers_loaded} known_herd_world_properties_applied=true other_world_properties_pending=true generic_native_load_flags_applied=false full_population=false ready=false",
        declared.constructed(),
        declared.refused(),
        declared.other_states,
        resident_states.declaration_count()
    );
    let npc_binding = match &mut payload.npcs {
        Ok(Some(npcs)) => npcs.bind(ticket, &declared, &resident_states).map(Some),
        Ok(None) => Ok(None),
        Err(error) => Err(error.clone()),
    };
    match npc_binding {
        Ok(Some(view)) => {
            println!(
                "mission_npc_resources ticket={ticket} herds={} markers={} vimps={} effect_contexts={} registered_namespace=true callbacks_admitted=false rendered=false ready=false",
                view.herds, view.markers, view.vimps, view.effect_contexts
            );
            blocked.push("Vimp AI is not yet active in this mission".into());
        }
        Ok(None) => {}
        Err(error) => blocked.push(format!("NPC resource binding: {error}")),
    }
    for record in declared.records {
        match record.result {
            Ok(id) => println!(
                "mission_declared_resident ticket={ticket} scope={:?} index={} class={:?} id={id:?} source_offset={:?} live_callbacks=false ready=false",
                record.key.scope, record.key.index, record.class, record.source_offset
            ),
            Err(error) => blocked.push(format!(
                "{:?} declaration {} class {:?}: {error}",
                record.key.scope, record.key.index, record.class
            )),
        }
    }
    (root, actors, blocked, resident_states, bindings_by_role)
}
fn drive(
    mut commands: Commands,
    scene: Res<Scene>,
    mut runtime: ResMut<Runtime>,
    mut requested: MessageReader<MissionPrepared>,
    mut cancelled: MessageReader<MissionCancelled>,
    showcase: Res<crate::mission_showcase::State>,
) {
    let obsolete = runtime
        .pending
        .as_ref()
        .is_some_and(|p| !live(p.ticket, scene.flow.ticket, scene.flow.phase))
        || runtime
            .active
            .as_ref()
            .is_some_and(|a| !live(a.ticket, scene.flow.ticket, scene.flow.phase));
    if obsolete {
        clear(&mut runtime, &mut commands);
    }
    let cancelled_tickets: std::collections::BTreeSet<_> =
        cancelled.read().map(|event| event.0).collect();
    for ticket in &cancelled_tickets {
        cancel_ticket(&mut runtime, &mut commands, *ticket);
    }
    for event in requested.read() {
        if event.ticket != scene.flow.ticket
            || event.intent != scene.launch_intent()
            || cancelled_tickets.contains(&event.ticket)
            || scene.flow.phase != Phase::AwaitingRuntime
            || runtime.handled == Some(event.ticket)
        {
            continue;
        }
        let Some(packet) = scene.prepared_mission.as_ref() else {
            continue;
        };
        if !packet.needs.is_empty() {
            continue;
        }
        if packet.request.world != event.request.world
            || packet.request.mission_name != event.request.mission_name
            || packet.request.source_id != event.request.source_id
            || packet.request.menu_index != event.request.menu_index
        {
            continue;
        }
        clear(&mut runtime, &mut commands);
        runtime.handled = Some(event.ticket);
        let packet = Arc::clone(packet);
        let root = scene.data_root().to_owned();
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = Arc::clone(&cancel);
        let showcase = showcase.enabled();
        let intent = event.intent;
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let result = (|| {
                let plan = packet.instance_plan(&root, "English")?;
                if worker_cancel.load(Ordering::Relaxed) {
                    return Err("mission staging cancelled".into());
                }
                let world = packet.world_asset(&root).map(Arc::new);
                let dressing = world
                    .as_ref()
                    .ok()
                    .filter(|_| showcase)
                    .map(|world| crate::mission_dressing::Prepared::load(&root, &plan, world));
                let covered = dressing
                    .as_ref()
                    .map_or_else(Default::default, |d| d.covered());
                let models = Arc::new(match &world {
                    Ok(world) => {
                        giants_game::static_geometry::Scene::prepare_normal_initial_with_terrain(
                            &plan,
                            &covered,
                            &world.terrain,
                        )
                    }
                    Err(_) => {
                        giants_game::static_geometry::Scene::prepare_normal_initial(&plan, &covered)
                    }
                });
                if worker_cancel.load(Ordering::Relaxed) {
                    return Err("mission staging cancelled".into());
                }
                let opening = if intent == LaunchIntent::CompletionReplay {
                    packet.completion(&root).and_then(|selection| {
                        if selection.director.is_some() {
                            Ok(selection)
                        } else {
                            Err("selected mission has no declared completion cinematic".into())
                        }
                    })
                } else {
                    packet.opening(&root)
                };
                println!(
                    "mission_cinematic_selection intent={intent:?} source_role={} native_replay_state_reconstructed=false",
                    if intent == LaunchIntent::CompletionReplay {
                        "completion"
                    } else {
                        "startup"
                    }
                );
                let mut playback = opening
                    .as_ref()
                    .map_err(Clone::clone)
                    .and_then(|selection| {
                        crate::mission_opening::Prepared::load(&root, selection.director.as_deref())
                    });
                if let Ok(Some(playback)) = &mut playback
                    && let Ok(world) = &world
                {
                    playback.prepare_render(&root, plan.resources(), &world.config);
                }
                let parameters = packet.slot_parameters(&root);
                let carried = (|| -> Result<_, String> {
                    let mission = packet.mission.as_ref().ok_or("prepared mission missing")?;
                    let declarations = mission
                        .slot_writes
                        .iter()
                        .filter_map(|write| {
                            if let giants_formats::mission::SlotWriteKind::PlayerChild12(class) =
                                write.kind
                            {
                                Some((write.source_offset, class))
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>();
                    if declarations.is_empty() {
                        return Ok(None);
                    }
                    if declarations.len() != 1 {
                        return Err("carried preview needs ordered replacement services".into());
                    }
                    let (offset, class) = declarations[0];
                    let prepared = giants_game::carried_actor::Prepared::load(
                        plan.resources(),
                        i32::try_from(class)
                            .map_err(|_| "carried class exceeds signed class profile")?,
                    )
                    .map_err(|e| e.to_string())?;
                    Ok(Some((offset, Arc::new(prepared))))
                })();
                let opening_effects_bound = playback
                    .as_ref()
                    .is_ok_and(|p| p.as_ref().is_none_or(|p| p.effects_bound()));
                let kind = packet
                    .mission
                    .as_ref()
                    .and_then(|m| m.starts.first())
                    .map(|s| s.words[0] as i32);
                let start_candidate = (|| {
                    giants_game::player_start::declared_for_kind(
                        &plan,
                        packet.world.as_ref().ok_or("prepared world missing")?,
                        packet.mission.as_ref().ok_or("prepared mission missing")?,
                        kind.ok_or("mission player start absent")?,
                    )
                })();
                let kabuto = prepare_kabuto(&root, &plan, &packet);
                let delphi = prepare_delphi(&root, &plan, &packet);
                let npcs = npc::State::prepare(&root, &plan);
                let player = giants_game::player_initial::Prepared::load(
                    &root,
                    &plan,
                    packet.world.as_ref().ok_or("prepared world missing")?,
                    packet.mission.as_ref().ok_or("prepared mission missing")?,
                )
                .map(Arc::new)
                .map_err(|e| e.to_string());
                let controller = world.as_ref().map_err(Clone::clone).and_then(|world| {
                    if matches!(kind, Some(1 | 3)) {
                        return Ok(None);
                    }
                    let initial = player.as_ref().map_err(Clone::clone)?;
                    crate::mecc_dev_scene::State::load_mission(&root, world, &packet, initial)
                        .map(Some)
                        .map_err(|e| e.to_string())
                });
                let director = opening
                    .as_ref()
                    .map_err(Clone::clone)
                    .and_then(|selection| {
                        selection
                            .director
                            .as_deref()
                            .map(|name| giants_game::director_initial::Prepared::load(&root, name))
                            .transpose()
                    });
                let effect_proxies = opening
                    .as_ref()
                    .map_err(Clone::clone)
                    .and_then(|selection| {
                        let Some(name) = selection.director.as_deref() else {
                            return Ok(Vec::new());
                        };
                        giants_game::cinematic_fx::load_terminal_proxies(
                            &root,
                            name,
                            plan.resources(),
                            0.,
                        )
                    });
                if worker_cancel.load(Ordering::Relaxed) {
                    return Err("mission staging cancelled".into());
                }
                let impact_sounds = controller
                    .as_ref()
                    .ok()
                    .and_then(|c| c.as_ref())
                    .map_or(Vec::new(), |s| s.impact_sound_ids());
                Ok(Payload {
                    signals: Default::default(),
                    npcs,
                    impact_sounds,
                    packet,
                    plan,
                    models,
                    world,
                    opening,
                    playback,
                    parameters,
                    start_candidate,
                    director,
                    effect_proxies,
                    player,
                    kabuto,
                    kabuto_body: None,
                    kabuto_camera_inspection: None,
                    delphi_body: None,

                    delphi,
                    controller,
                    carried,
                    dressing,
                    opening_effects_bound,
                })
            })();
            let _ = tx.send(result);
        });
        runtime.pending = Some(Pending {
            ticket: event.ticket,
            cancel,
            receiver: Mutex::new(rx),
            start: StartState {
                intent: event.intent,
                progress: event.progress.clone(),
            },
        });
        println!(
            "mission_session_requested ticket={} initialization=bounded_normal_initial animation_warmup=false readiness=false",
            event.ticket
        );
    }
    let result = runtime.pending.as_ref().and_then(|p| {
        match p
            .receiver
            .lock()
            .expect("owned mission receiver")
            .try_recv()
        {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                Some(Err("mission staging worker disconnected".into()))
            }
        }
    });
    if let Some(result) = result {
        let pending = runtime.pending.take().expect("owned mission completion");
        if pending.ticket != scene.flow.ticket || scene.flow.phase != Phase::AwaitingRuntime {
            return;
        }
        match result {
            Ok(mut payload) => {
                let (root, actors, blocked, resident_states, bindings) =
                    spawn(&mut commands, pending.ticket, &mut payload);
                let campaign = Some(campaign::State::prepare(&payload.packet, &payload.plan));
                runtime.active = Some(Active {
                    ticket: pending.ticket,
                    root,
                    actors,
                    resident_states,
                    blocked,
                    payload,
                    start: StartState {
                        intent: pending.start.intent,
                        progress: pending.start.progress.clone(),
                    },
                    reported: false,
                    pickup: None,
                    variant_random: giants_game::audio_parameters::Random(1),
                    audio_registry: None,
                    bindings,
                    controlled_mecc_owner: None,
                    owned_mecc: None,
                    landing: None,
                    campaign,
                });
                runtime.initialize_owned_pickup(pending.ticket);
            }
            Err(error) => {
                eprintln!(
                    "mission_session_error ticket={} error={error}",
                    pending.ticket
                );
                runtime.failure = Some((pending.ticket, error));
            }
        }
    }
}
fn bind_declared_residents(
    mut commands: Commands,
    runtime: Res<Runtime>,
    actors: Query<(Entity, &ActorRef), Without<OwnedActor>>,
) {
    let Some(active) = &runtime.active else {
        return;
    };
    for (entity, reference) in &actors {
        if reference.ticket != active.ticket {
            continue;
        }
        if let Some(id) = active
            .resident_states
            .declaration(declaration_key(reference))
        {
            commands.entity(entity).try_insert(OwnedActor(id));
        }
    }
}
pub(crate) fn audit(
    mut runtime: ResMut<Runtime>,
    owned: Query<&MissionOwned>,
    actors: Query<&ActorRef>,
    bindings: Query<(&ActorRef, &OwnedActor)>,
) {
    runtime.retired.retain(|ticket| {
        let remaining = owned.iter().filter(|e| e.0 == *ticket).count();
        if remaining == 0 {
            println!("mission_session_released ticket={ticket} remaining_entities=0");
        }
        remaining != 0
    });
    let Some(active) = runtime.active.as_mut().filter(|a| !a.reported) else {
        return;
    };
    let count = actors.iter().filter(|a| a.ticket == active.ticket).count();
    let entities = owned.iter().filter(|e| e.0 == active.ticket).count();
    if count != active.actors {
        return;
    }
    let bound = bindings
        .iter()
        .filter(|(r, id)| {
            r.ticket == active.ticket
                && active.resident_states.declaration(declaration_key(r)) == Some(id.0)
                && active.resident_states.get(id.0).is_some()
        })
        .count();
    if bound != active.resident_states.declaration_count() {
        active.blocked.push(format!(
            "declared actor bindings incomplete: {bound}/{}",
            active.resident_states.declaration_count()
        ));
    }
    println!(
        "mission_declared_bindings ticket={} bound={} expected={} actor_entities={} complete_population=false ready=false",
        active.ticket,
        bound,
        active.resident_states.declaration_count(),
        count
    );
    match active.resident_states.entry_subset() {
        Ok(subset) => println!(
            "mission_entry_subset ticket={} owned_actors={} selected_actors={} total={} scene_actor_entities={} complete_native_list=false progression_written=false ready=false",
            active.ticket,
            subset.actors.len(),
            subset
                .actors
                .iter()
                .filter(|a| a.contribution.object_flags & 0x400 != 0)
                .count(),
            subset.total(),
            count
        ),
        Err(error) => {
            println!(
                "mission_entry_subset_refused ticket={} reason={error} progression_written=false ready=false",
                active.ticket
            );
            active.blocked.push(format!("owned entry subset: {error}"));
        }
    }
    println!(
        "mission_session_staged ticket={} entities={} actor_entities={} mission_declarations={} world_declarations={} rigid_candidates={} model_assets={} geometry_diagnostics={} blockers={} intent={:?} progress_world={} ready=false draw_entities=0 animation_warmup=false",
        active.ticket,
        entities,
        count,
        active.payload.plan.entries.len(),
        active.payload.plan.world_entries.len(),
        active.payload.models.instances.len(),
        active.payload.models.assets.len(),
        active.payload.models.diagnostics.len(),
        active.blocked.len(),
        active.start.intent,
        active.start.progress.world_word()
    );
    for blocker in &active.blocked {
        println!(
            "mission_session_blocked ticket={} reason={blocker}",
            active.ticket
        );
    }
    if let Ok(world) = &active.payload.world {
        println!(
            "mission_session_world ticket={} name={} vertices={} triangles={} terrain_textures={} sky_vertices={} underside_vertices={} ready=false draw_entities=0",
            active.ticket,
            world.name,
            world.positions.len(),
            world.triangles.len(),
            world.textures.len(),
            world.sky.mesh.positions.len(),
            world.underside.mesh.positions.len()
        );
    }
    if let Ok(opening) = &active.payload.opening {
        println!(
            "mission_session_opening ticket={} mission_slot={} source_offset={:?} world_index={:?} director={:?} normal_clock=unmeasured playback=false",
            active.ticket,
            opening.mission_slot,
            opening.source_offset,
            opening.world_index,
            opening.director
        );
    }
    if let Ok(Some(director)) = &active.payload.director {
        match &director.binding_candidate {
            Ok(binding) => println!(
                "mission_session_camera_request ticket={} command_offset={} tree_slot={} tree={:?} name={} node_offset={} callbacks_executed=false bound=false viewport_profile=fixed_inspection",
                active.ticket,
                binding.command_offset,
                binding.tree_slot,
                binding.tree,
                binding.name,
                binding.node_offset
            ),
            Err(error) => println!(
                "mission_session_camera_request ticket={} error={} bound=false",
                active.ticket, error
            ),
        }
        println!(
            "mission_session_director ticket={} counts={:?} initial_finished={} animation_bound={} initial_seconds={} initial_dt={} first_bound_reset_tick={} camera_candidate={} strict_program={} declarations={} nodes={} playback=false",
            active.ticket,
            director.initial.counts,
            director.initial.finished,
            director.initial.animation_bound,
            director.initial.seconds,
            director.initial.dt,
            director.reset_tick,
            director.camera_candidate.is_ok(),
            director.program.is_ok(),
            director.dependencies.actors.len(),
            director.animation.nodes.len()
        );
    }
    for diagnostic in &active.payload.models.diagnostics {
        println!(
            "mission_session_geometry ticket={} origin={:?} class={} result={}",
            active.ticket, diagnostic.key, diagnostic.class, diagnostic.result
        );
    }
    let invalid_references = actors
        .iter()
        .filter(|a| a.ticket == active.ticket)
        .filter(|a| {
            if a.world_declaration {
                a.index >= active.payload.plan.world_entries.len()
            } else {
                a.index >= active.payload.plan.entries.len()
            }
        })
        .count();
    let expected_players = usize::from(active.payload.player.is_ok());
    if entities != count + 1 + expected_players || invalid_references != 0 {
        eprintln!(
            "mission_session_invalid ticket={} entities={} actors={} invalid_references={}",
            active.ticket, entities, count, invalid_references
        );
    }
    active.reported = true;
}

pub(crate) struct SmokeDraw {
    pub id: ActorId,
    pub geometry: Arc<giants_game::smoke_particle_draw::Geometry>,
    pub frame: giants_game::smoke_particle_draw::Frame,
}
impl Runtime {
    pub(crate) fn smoke_draws(
        &mut self,
        view: [f32; 16],
    ) -> Result<Option<(u64, Vec<SmokeDraw>)>, String> {
        let ticket = self.active_ticket();
        self.smoke_geometry.retain(|(t, _), _| Some(*t) == ticket);
        let Some(active) = self.active.as_mut() else {
            return Ok(None);
        };
        let ids = active.resident_states.particle_draw_ids();
        self.smoke_geometry.retain(|(_, id), _| ids.contains(id));
        let mut frames = Vec::new();
        for id in ids {
            if active.resident_states.particle_draw_state(id)?.is_none() {
                self.smoke_geometry.remove(&(active.ticket, id));
                continue;
            }
            let geometry = match self.smoke_geometry.entry((active.ticket, id)) {
                std::collections::btree_map::Entry::Occupied(e) => e.get().clone(),
                std::collections::btree_map::Entry::Vacant(e) => e
                    .insert(Arc::new(giants_game::smoke_particle_draw::Geometry::load(
                        active.payload.plan.resources(),
                        &active.resident_states,
                        id,
                        0,
                    )?))
                    .clone(),
            };
            if let Some(frame) = geometry.frame(&mut active.resident_states, id, view)? {
                frames.push(SmokeDraw {
                    id,
                    geometry,
                    frame,
                });
            }
        }
        Ok(Some((active.ticket, frames)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::world::CommandQueue;

    pub(super) fn payload() -> Payload {
        use giants_game::{
            catalog_registry::WorldClassRegistry,
            model_registry::{ClassProgram, ModelRegistry},
            scene_resources::SceneResources,
        };
        let mut models = ModelRegistry::default();
        models.programs.insert(
            7,
            vec![ClassProgram {
                catalog: "synthetic.bin".into(),
                bindings: Ok(vec![]),
                program: Ok(giants_formats::class_program::parse(&[0x1e, 4, 0xff]).unwrap()),
            }],
        );
        let resources = Arc::new(SceneResources {
            world_bounds: Default::default(),
            classes: WorldClassRegistry {
                visits: vec![],
                classes: Default::default(),
                models,
                textures: Default::default(),
            },
            archives: Default::default(),
            language: "English".into(),
        });
        let mut mission = giants_formats::mission::parse(&[4, 0, 0, 0, 1, 2]).unwrap();
        mission.placements.push(giants_formats::mission::Placement {
            source_offset: 5,
            object_type: 7,
            position: [1., 2., 3.],
            yaw: 0.,
            pitch_roll: Some([0., 0.]),
            scale: Some(1.),
        });
        let world = giants_formats::world::WorldPlacement {
            selector: None,
            word_174: None,
            state_writes: Vec::new(),
            values_f4: None,
            cinematic_exclusions: Vec::new(),
            source_offset: 100,
            object_type: 7,
            position: [4., 5., 6.],
            yaw: 0.,
            pitch_roll: None,
            scale: Some(1.),
        };
        let plan = Plan::prepare_scene(&resources, &mission, &[world]);
        Payload {
            signals: Default::default(),
            npcs: Ok(None),
            impact_sounds: Vec::new(),
            dressing: None,
            opening_effects_bound: true,
            packet: Arc::new(PreparedMission {
                request: giants_game::mission_selection::LoadRequest {
                    world: "Synthetic".into(),
                    menu_index: 0,
                    source_id: 3,
                    mission_name: "fixture".into(),
                    splash: giants_game::mission_selection::Splash::Mecc,
                },
                world: None,
                mission: Some(mission),
                scenario_index: None,
                terrain: None,
                terrain_bytes: vec![],
                world_bytes: vec![],
                mission_bytes: vec![],
                needs: vec![],
            }),
            plan,
            models: Default::default(),
            world: Err("synthetic fixture has no environment assets".into()),
            opening: Err("synthetic fixture has no mission slots".into()),
            playback: Ok(None),
            parameters: Err("synthetic fixture has no mission slots".into()),
            start_candidate: Err("synthetic fixture has no start".into()),
            director: Err("synthetic fixture has no director".into()),
            effect_proxies: Ok(Vec::new()),
            controller: Ok(None),
            carried: Ok(None),
            player: Err("synthetic fixture has no player resources".into()),
            kabuto: Ok(None),
            kabuto_body: None,
            kabuto_camera_inspection: None,
            delphi_body: None,

            delphi: Ok(None),
        }
    }

    #[test]
    #[ignore = "Requires GIANTS_DELPHI_ROOT; source menu request and common spawn/roles/re-entry; all-unlocked test mask, no graphics/input claim"]
    fn source_delphi_menu_entry_constructs_declared_group_and_retains_player_role() {
        let root = std::path::PathBuf::from(std::env::var("GIANTS_DELPHI_ROOT").unwrap());
        let source = giants_game::world_source::resolve(&root, "Rstory1").unwrap();
        let menu = giants_game::mission_selection::MissionMenu::parse(
            &std::fs::read(root.join("Bin").join(source.catalog)).unwrap(),
            u32::MAX,
            true,
        )
        .unwrap();
        let mut ecs = World::new();
        let baseline = ecs.query::<Entity>().iter(&ecs).count();
        for ticket in [998, 999] {
            let packet = Arc::new(
                giants_game::mission_loading::prepare(&root, menu.request("Rstory1", 0).unwrap())
                    .unwrap(),
            );
            let plan = packet.instance_plan(&root, "English").unwrap();
            let factory = prepare_delphi(&root, &plan, &packet).unwrap().unwrap();
            let start = factory.declared_start().clone();
            let terrain = packet.terrain.as_ref().unwrap();
            let query = giants_game::terrain_queries::StaticTerrainQuery::new(
                terrain,
                1. / terrain.spacing,
                giants_game::arithmetic::Precision::Double53,
            )
            .unwrap();
            let sample = query
                .sample([start.position[0], start.position[1]])
                .unwrap();
            println!(
                "test_delphi_declared_source ticket={ticket} position={:?} terrain_height={} source_sea_scale={} native_wet_classification_verified=false",
                start.position,
                sample.height,
                packet.world.as_ref().unwrap().sea_values[0]
            );
            let mut payload = payload();
            payload.packet = packet;
            payload.plan = plan;
            payload.delphi = Ok(Some(factory));
            let mut queue = CommandQueue::default();
            let (scene_root, actors, blocked, residents, bindings) =
                spawn(&mut Commands::new(&mut queue, &ecs), ticket, &mut payload);
            queue.apply(&mut ecs);
            let owner = bindings.player.expect("actual Delphi constructor identity");
            let body = Arc::clone(payload.delphi_body.as_ref().unwrap_or_else(|| {
                panic!(
                    "actual Delphi body missing: {:?}",
                    blocked
                        .iter()
                        .filter(|s| s.starts_with("Delphi body"))
                        .collect::<Vec<_>>()
                )
            }));
            assert_eq!(body.owner, owner);
            assert!(body.current(&residents));
            assert!(!body.positions.is_empty());
            assert!(body.vertices(0).iter().flatten().all(|v| v.is_finite()));
            assert!(residents.actor_model_cache(owner, 0).unwrap().is_none());
            assert!(residents.actor_current_affine(owner).is_err());
            assert!(residents.delphi_physical(owner).is_none());
            assert_eq!(
                bindings.delphi_projection,
                Some(giants_game::delphi_player::ProjectionStatus::Pending)
            );
            let actor = residents.get(owner).unwrap();
            assert_eq!(actor.class, 3);
            assert_eq!(
                actor.construction_fields.as_ref().unwrap().position,
                start.position
            );
            let group = residents.delphi_special(owner).unwrap();
            assert_eq!(residents.get(group.held).unwrap().class, 669);
            assert!(!group.copies.is_empty());
            assert!(
                blocked
                    .iter()
                    .any(|s| s.starts_with("Delphi pre-special projection"))
            );
            assert!(
                !blocked
                    .iter()
                    .any(|s| s.starts_with("initial player construction:"))
            );
            let mut active = Active {
                ticket,
                root: scene_root,
                actors,
                resident_states: residents,
                blocked,
                payload,
                start: StartState {
                    intent: LaunchIntent::SavedMission,
                    progress: Progress::load(None, None).unwrap(),
                },
                reported: false,
                pickup: None,
                variant_random: giants_game::audio_parameters::Random(1),
                audio_registry: None,
                bindings,
                controlled_mecc_owner: None,
                owned_mecc: None,
                landing: None,
                campaign: None,
            };
            assert_eq!(active.player_owner().unwrap(), owner);
            let peer = active
                .payload
                .delphi
                .as_ref()
                .unwrap()
                .as_ref()
                .unwrap()
                .construct_declared(
                    &mut active.resident_states,
                    giants_game::animation_channels::Blending::Enabled,
                )
                .result
                .unwrap()
                .owner;
            assert_ne!(peer, owner);
            assert_eq!(active.player_owner().unwrap(), owner);
            let mut foreign = giants_game::resident_states::Session::new(ticket + 1000);
            let foreign_owner = active
                .payload
                .delphi
                .as_ref()
                .unwrap()
                .as_ref()
                .unwrap()
                .construct_declared(
                    &mut foreign,
                    giants_game::animation_channels::Blending::Enabled,
                )
                .result
                .unwrap()
                .owner;
            active.bindings.player = Some(foreign_owner);
            assert!(active.player_owner().is_err());
            active.bindings.player = Some(owner);
            assert_eq!(active.player_owner().unwrap(), owner);
            foreign.clear();
            active.resident_states.clear();
            assert!(!body.current(&active.resident_states));
            assert!(active.resident_states.get(owner).is_none());
            assert!(active.player_owner().is_err());
            assert_eq!(active.resident_states.counts().residents, 0);
            ecs.despawn(scene_root);
            assert_eq!(ecs.query::<Entity>().iter(&ecs).count(), baseline);
            active.payload.delphi = Ok(None);
            let mut queue = CommandQueue::default();
            let (missing_root, _, missing, mut residents, bindings) = spawn(
                &mut Commands::new(&mut queue, &ecs),
                ticket + 100,
                &mut active.payload,
            );
            queue.apply(&mut ecs);
            assert!(bindings.player.is_none());
            assert!(bindings.delphi_projection.is_none());
            assert!(active.payload.delphi_body.is_none());
            assert!(
                missing
                    .iter()
                    .any(|s| s == "Delphi cold factory absent for requested player")
            );
            residents.clear();
            ecs.despawn(missing_root);
            assert_eq!(ecs.query::<Entity>().iter(&ecs).count(), baseline);
        }
    }

    #[test]
    #[ignore = "Requires GIANTS_NPC_ROOT; actual Story2 menu request and canonical mission resource/declaration binding; no world callbacks, window or sound"]
    fn source_npc_menu_entry_binds_registered_resources_and_reenters() {
        let root = std::path::PathBuf::from(std::env::var_os("GIANTS_NPC_ROOT").unwrap());
        let source = giants_game::world_source::resolve(&root, "Story2").unwrap();
        let menu = giants_game::mission_selection::MissionMenu::parse(
            &std::fs::read(root.join("Bin").join(source.catalog)).unwrap(),
            u32::MAX,
            true,
        )
        .unwrap();
        let packet = Arc::new(
            giants_game::mission_loading::prepare(&root, menu.request("Story2", 0).unwrap())
                .unwrap(),
        );
        let mut world = World::new();
        let external = world.spawn(Name::new("unrelated owner")).id();
        for ticket in [3501, 3502] {
            let plan = packet.instance_plan(&root, "English").unwrap();
            let npcs = npc::State::prepare(&root, &plan).unwrap().unwrap();
            let player = giants_game::player_initial::Prepared::load(
                &root,
                &plan,
                packet.world.as_ref().unwrap(),
                packet.mission.as_ref().unwrap(),
            )
            .map(Arc::new)
            .map_err(|e| e.to_string());
            let mut payload = payload();
            payload.packet = Arc::clone(&packet);
            payload.plan = plan;
            payload.player = player;
            payload.npcs = Ok(Some(npcs));
            let mut queue = CommandQueue::default();
            let (scene_root, _, blocked, mut residents, _) =
                spawn(&mut Commands::new(&mut queue, &world), ticket, &mut payload);
            queue.apply(&mut world);
            let npcs = payload.npcs.as_mut().unwrap().as_mut().unwrap();
            let view = npcs.view(&residents, ticket).unwrap();
            assert!(view.herds > 0);
            assert!(view.markers > 0);
            assert_eq!((view.vimps, view.effect_contexts), (0, 0));
            assert!(
                blocked
                    .iter()
                    .any(|b| b == "Vimp AI is not yet active in this mission")
            );
            assert!(
                !blocked
                    .iter()
                    .any(|b| b.starts_with("NPC resource binding:")),
                "{blocked:?}"
            );
            for id in residents.list(0) {
                if let Some(clock) = residents.herd_clock(id) {
                    assert_eq!(clock.steps, 0);
                }
            }
            assert!(npcs.view(&residents, ticket + 1).is_err());
            assert!(
                npcs.bind(
                    ticket,
                    &giants_game::declared_residents::Report::default(),
                    &residents
                )
                .is_err()
            );
            assert_eq!(npcs.view(&residents, ticket).unwrap(), view);
            residents.clear();
            assert!(npcs.view(&residents, ticket).is_err());
            world.entity_mut(scene_root).despawn();
            assert!(world.get_entity(external).is_ok());
            assert_eq!(
                world
                    .query::<&MissionOwned>()
                    .iter(&world)
                    .filter(|o| o.0 == ticket)
                    .count(),
                0
            );
        }
    }
    #[test]
    #[ignore = "Requires GIANTS_KABUTO_ROOT; actual menu request, common spawn, cold ownership and second entry; no window or sound"]
    fn source_kabuto_menu_entry_constructs_owned_group_and_reenters() {
        check_source_kabuto_menu_entry(false);
    }
    #[test]
    #[ignore = "Requires GIANTS_KABUTO_ROOT and GIANTS_KABUTO_EXECUTABLE; common menu/spawn and explicit mode-1 unclipped camera, stale-owner/return/re-entry, no GPU or gameplay claim"]
    fn source_kabuto_inspection_camera_uses_owned_mode_profile_and_reenters() {
        check_source_kabuto_menu_entry(true);
    }
    fn check_source_kabuto_menu_entry(camera: bool) {
        let root = std::path::PathBuf::from(std::env::var("GIANTS_KABUTO_ROOT").unwrap());
        let source = giants_game::world_source::resolve(&root, "Kstory1").unwrap();
        let menu = giants_game::mission_selection::MissionMenu::parse(
            &std::fs::read(root.join("Bin").join(source.catalog)).unwrap(),
            u32::MAX,
            true,
        )
        .unwrap();
        let packet = Arc::new(
            giants_game::mission_loading::prepare(&root, menu.request("Kstory1", 0).unwrap())
                .unwrap(),
        );
        let plan = packet.instance_plan(&root, "English").unwrap();
        let prepared = prepare_kabuto(&root, &plan, &packet).unwrap().unwrap();
        let start = prepared.declared_start().clone();
        let mut payload = payload();
        if camera {
            payload.world = Ok(Arc::new(giants_game::load_world(&root, "Kstory1").unwrap()));
        }
        payload.packet = packet;
        payload.plan = plan;
        payload.kabuto = Ok(Some(prepared));
        let mut world = World::new();
        let baseline = world.query::<Entity>().iter(&world).count();
        for ticket in [996, 997] {
            let mut queue = CommandQueue::default();
            let (scene_root, actors, blocked, mut residents, bindings) =
                spawn(&mut Commands::new(&mut queue, &world), ticket, &mut payload);
            queue.apply(&mut world);
            let owner = bindings
                .player
                .expect("Kabuto constructor must publish its identity");
            let actor = residents.get(owner).unwrap();
            assert_eq!(actor.class, 1);
            assert_eq!(
                actor.construction_fields.as_ref().unwrap().position,
                start.position
            );
            let group = residents.kabuto_group(owner).unwrap();
            let body = payload
                .kabuto_body
                .as_ref()
                .unwrap_or_else(|| panic!("Kabuto body preparation refused: {blocked:?}"))
                .clone();
            assert_eq!(body.owner, owner);
            assert!(body.current(&residents));
            assert!(!body.positions.is_empty());
            assert!(
                body.model
                    .mesh
                    .subobjects
                    .iter()
                    .enumerate()
                    .all(|(i, _)| !body.vertices(i).is_empty())
            );
            assert_eq!(
                group.pending,
                [
                    giants_game::kabuto_initial::Pending::ProjectionReset,
                    giants_game::kabuto_initial::Pending::AnimationCache,
                    giants_game::kabuto_initial::Pending::ProceduralMaterial,
                    giants_game::kabuto_initial::Pending::GeometryRegistration,
                ]
            );
            assert!(blocked.iter().any(|s| s.starts_with("Kabuto projection")));
            assert!(
                !blocked
                    .iter()
                    .any(|s| s.starts_with("initial player construction:"))
            );
            assert_eq!(owner.ticket(), ticket);
            if camera {
                let mut runtime = Runtime {
                    active: Some(Active {
                        ticket,
                        root: scene_root,
                        actors,
                        resident_states: residents,
                        blocked,
                        payload: std::mem::replace(&mut payload, self::payload()),
                        start: StartState {
                            intent: LaunchIntent::SavedMission,
                            progress: Progress::load(None, None).unwrap(),
                        },
                        reported: false,
                        pickup: None,
                        variant_random: giants_game::audio_parameters::Random(1),
                        audio_registry: None,
                        bindings,
                        controlled_mecc_owner: None,
                        owned_mecc: None,
                        landing: None,
                        campaign: None,
                    }),
                    ..Default::default()
                };
                let executable =
                    std::path::PathBuf::from(std::env::var_os("GIANTS_KABUTO_EXECUTABLE").unwrap());
                runtime
                    .prepare_kabuto_camera_inspection(&executable)
                    .unwrap();
                let candidate = runtime.environment_candidate().unwrap().unwrap();
                assert_eq!(candidate.ticket, ticket);
                assert_eq!(candidate.camera_source, "kabuto_profile_inspection");
                assert!(candidate.eye.into_iter().all(f32::is_finite));
                assert!(candidate.camera.projection.into_iter().all(f32::is_finite));
                assert!(runtime.startup().is_err());
                runtime
                    .prepare_kabuto_camera_inspection(&executable)
                    .unwrap();
                let active = runtime.active.as_mut().unwrap();
                assert!(body.current(&active.resident_states));
                assert!(active.resident_states.camera_look_angle(owner).unwrap() < 0.);
                assert_eq!(
                    active
                        .resident_states
                        .kabuto_group(owner)
                        .unwrap()
                        .pending
                        .len(),
                    4
                );
                active.resident_states.clear();
                assert!(runtime.environment_candidate().is_err());
                assert!(
                    runtime
                        .prepare_kabuto_camera_inspection(&executable)
                        .is_err()
                );
                let active = runtime.active.take().unwrap();
                payload = active.payload;
                residents = active.resident_states;
            }
            residents.clear();
            assert!(!body.current(&residents));
            assert!(residents.get(owner).is_none());
            assert_eq!(residents.counts().residents, 0);
            world.despawn(scene_root);
            assert_eq!(world.query::<Entity>().iter(&world).count(), baseline);
        }
    }

    #[test]
    #[ignore = "Requires GIANTS_MISSION_OWNED_ROOT; actual constructor roles with duplicate-class, foreign and retired owners; no window or sound"]
    fn source_actor_roles_keep_player_identity_across_duplicate_classes_and_retirement() {
        let root = std::path::PathBuf::from(std::env::var("GIANTS_MISSION_OWNED_ROOT").unwrap());
        let request = giants_game::mission_selection::LoadRequest {
            world: "Story1".into(),
            menu_index: 0,
            source_id: 3,
            mission_name: "1c_1_firstmecc".into(),
            splash: giants_game::mission_selection::Splash::Mecc,
        };
        let packet = Arc::new(giants_game::mission_loading::prepare(&root, request).unwrap());
        let plan = packet.instance_plan(&root, "English").unwrap();
        let player = Arc::new(
            giants_game::player_initial::Prepared::load(
                &root,
                &plan,
                packet.world.as_ref().unwrap(),
                packet.mission.as_ref().unwrap(),
            )
            .unwrap(),
        );
        let mut payload = payload();
        payload.parameters = packet.slot_parameters(&root);
        payload.packet = packet;
        payload.plan = plan;
        payload.player = Ok(player.clone());
        let mut world = World::new();
        // Bevy's default query-filter resource itself has a backing entity.
        let baseline_entities = world.query::<Entity>().iter(&world).collect::<Vec<_>>();
        let mut queue = CommandQueue::default();
        let (scene_root, actors, blocked, residents, bindings) =
            spawn(&mut Commands::new(&mut queue, &world), 995, &mut payload);
        queue.apply(&mut world);
        let owner = bindings
            .player
            .expect("source player constructor must publish its identity");
        let mut runtime = Runtime {
            active: Some(Active {
                ticket: 995,
                root: scene_root,
                actors,
                resident_states: residents,
                blocked,
                payload,
                start: StartState {
                    intent: LaunchIntent::SavedMission,
                    progress: Progress::load(None, None).unwrap(),
                },
                reported: false,
                pickup: None,
                variant_random: giants_game::audio_parameters::Random(1),
                audio_registry: None,
                bindings,
                controlled_mecc_owner: None,
                owned_mecc: None,
                landing: None,
                campaign: None,
            }),
            ..Default::default()
        };
        let active = runtime.active.as_mut().unwrap();
        assert_eq!(active.player_owner().unwrap(), owner);
        assert!(active.controlled_mecc_owner.is_none());
        let fields = active
            .resident_states
            .get(owner)
            .unwrap()
            .construction_fields
            .clone()
            .unwrap();
        let specification = active.resident_states.specification(owner).unwrap().clone();
        let state_resources = active
            .resident_states
            .state_resources(owner)
            .unwrap()
            .clone();
        let program = active
            .payload
            .plan
            .resources()
            .classes
            .models
            .first_program(2)
            .unwrap()
            .program
            .as_ref()
            .unwrap();
        let flags = giants_game::object_lists::DirectFlags::project(2, program);
        let cold_fields = giants_game::actor_fields::Fields::load(
            active.payload.plan.resources(),
            2,
            &specification,
        )
        .unwrap()
        .instantiate();
        let construct = |session: &mut giants_game::resident_states::Session| {
            session
                .construct(giants_game::resident_states::Construction {
                    class: 2,
                    seed: &player.state_seed,
                    bindings: &player.state_bindings,
                    flags,
                    specification: specification.clone(),
                    state_resources: state_resources.clone(),
                    fields: cold_fields.clone(),
                })
                .unwrap()
        };
        let duplicate = construct(&mut active.resident_states);
        assert_ne!(duplicate, owner);
        assert!(active.resident_states.unique_class(2).is_err());
        let duplicate_pose = active
            .resident_states
            .get(duplicate)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap()
            .position;
        assert!(runtime.player_state_specification(995, 0x70).is_ok());
        assert_eq!(
            runtime
                .actor_sound_route(995, owner, giants_game::actor_sound::Kind::Attachment, true)
                .unwrap(),
            Some(giants_game::actor_sound::Route::Local(owner))
        );
        assert_eq!(
            runtime
                .actor_sound_route(
                    995,
                    duplicate,
                    giants_game::actor_sound::Kind::Attachment,
                    true
                )
                .unwrap(),
            Some(giants_game::actor_sound::Route::Attached(Some(duplicate)))
        );
        let mut position = fields.position;
        position[0] += 1.;
        runtime
            .publish_controlled_mecc_motion(
                995,
                1,
                giants_game::controlled_mecc::Frame {
                    before: fields.position,
                    position,
                    contact_previous: fields.position,
                    angles: fields.angles,
                    scale: fields.scale,
                    velocity: [0.; 3],
                    model_rates: [0.; 3],
                },
            )
            .unwrap();
        // The ordinary render adapter consumes constructor-bound children and
        // the real motion owner without completing or mutating an owned primary.
        assert!(runtime.pistol_projectile_visual(995).is_ok());
        assert!(runtime.pistol_projectile_visual(996).is_err());
        let jet_models = runtime.owned_jet_models(995).unwrap();
        assert!(jet_models.iter().all(|m| m.mesh.normal_count == 0));
        let (session, bound_owner) = runtime.controlled_jet_session_mut(995).unwrap();
        assert_eq!(bound_owner, owner);
        let children = session.mecc_special(owner).unwrap().children;
        let channels_before = children.map(|id| session.get(id).unwrap().channels);
        let spec = session.specification(owner).unwrap().clone();
        let animation = &spec.animations[&1].animation;
        let jet_world =
            giants_game::object_transform::euler(position, fields.angles, fields.scale).unwrap();
        {
            let (flags, boost, hidden) = (0x1000, true, [false, true, false]);
            let mut producer =
                crate::mission_player::jet::Presentation::new(session, owner).unwrap();
            let input = || crate::mission_player::jet::Parent {
                animation,
                seconds: 0.,
                world: jet_world,
                scale: fields.scale,
                flags,
                boost,
            };
            let frames = producer.advance(session, 1, 1. / 120., input()).unwrap();
            assert_eq!(
                frames.iter().map(|f| f.pose.id).collect::<Vec<_>>(),
                children
            );
            assert_eq!(
                frames.iter().map(|f| f.pose.hidden).collect::<Vec<_>>(),
                hidden
            );
            assert!(
                frames
                    .iter()
                    .zip(&jet_models)
                    .all(|(f, m)| f.positions.len() == m.mesh.positions.len())
            );
            assert!(
                frames.iter().all(|f| !f.positions.is_empty()
                    && f.pose.world.values.iter().all(|v| v.is_finite()))
            );
            assert!(producer.advance(session, 1, 1. / 120., input()).is_err());
            assert!(
                producer
                    .advance(session, 2, 1. / 120., input())
                    .err()
                    .unwrap()
                    .contains("quarantined")
            );
        }
        assert_ne!(
            channels_before,
            children.map(|id| session.get(id).unwrap().channels)
        );
        assert!(
            children
                .iter()
                .all(|&id| session.passive_world_frame(id).unwrap().next_frame == 1)
        );
        assert!(session.mecc_physical_cycle(owner).is_err());
        let active = runtime.active.as_mut().unwrap();
        assert_eq!(active.controlled_mecc_owner, Some(owner));
        assert_eq!(
            active
                .resident_states
                .get(owner)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap()
                .position,
            position
        );
        assert_eq!(
            active
                .resident_states
                .get(duplicate)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap()
                .position,
            duplicate_pose
        );
        let mut foreign = giants_game::resident_states::Session::new(996);
        active.bindings.player = Some(construct(&mut foreign));
        assert!(active.player_owner().is_err());
        active.bindings.player = Some(owner);
        active.resident_states.clear();
        let replacement = construct(&mut active.resident_states);
        assert_ne!(replacement, owner);
        assert_eq!(active.resident_states.unique_class(2).unwrap(), replacement);
        assert!(active.player_owner().is_err());
        assert!(runtime.player_state_specification(995, 0x70).is_err());
        assert!(
            runtime
                .publish_controlled_mecc_motion(
                    995,
                    2,
                    giants_game::controlled_mecc::Frame {
                        before: position,
                        position,
                        contact_previous: position,
                        angles: fields.angles,
                        scale: fields.scale,
                        velocity: [0.; 3],
                        model_rates: [0.; 3],
                    }
                )
                .is_err()
        );
        cancel_ticket(&mut runtime, &mut Commands::new(&mut queue, &world), 995);
        queue.apply(&mut world);
        assert!(runtime.active.is_none());
        assert_eq!(world.query::<&MissionOwned>().iter(&world).count(), 0);
        assert!(world.get_entity(scene_root).is_err());
        assert_eq!(
            world.query::<Entity>().iter(&world).collect::<Vec<_>>(),
            baseline_entities
        );
    }

    #[test]
    #[ignore = "Requires GIANTS_MISSION_OWNED_ROOT pointing to the owned reference clone; CPU-only, no window, sound or input"]
    fn source_owned_mecc_runtime_terrain_and_presentation() {
        check_source_owned_mecc_runtime(false);
    }

    #[test]
    #[ignore = "Requires GIANTS_MISSION_OWNED_ROOT; real ordinary body/motion/jet producer composition with supplied commands, no GPU or real input"]
    fn source_normal_mecc_body_drives_jet_presentation_across_flight_and_release() {
        let root = std::path::PathBuf::from(std::env::var("GIANTS_MISSION_OWNED_ROOT").unwrap());
        let packet = Arc::new(
            giants_game::mission_loading::prepare(
                &root,
                giants_game::mission_selection::LoadRequest {
                    world: "Story1".into(),
                    menu_index: 0,
                    source_id: 3,
                    mission_name: "1c_1_firstmecc".into(),
                    splash: giants_game::mission_selection::Splash::Mecc,
                },
            )
            .unwrap(),
        );
        let asset = Arc::new(packet.world_asset(&root).unwrap());
        let plan = packet.instance_plan(&root, "English").unwrap();
        let player = Arc::new(
            giants_game::player_initial::Prepared::load(
                &root,
                &plan,
                packet.world.as_ref().unwrap(),
                packet.mission.as_ref().unwrap(),
            )
            .unwrap(),
        );
        let mut viewer =
            crate::mecc_dev_scene::State::load_mission(&root, &asset, &packet, &player).unwrap();
        // Actual Bevy input semantics: a down/up pair in one host update has a
        // press edge but no held level. Zero-step updates must not lose it, and
        // consuming that edge must not auto-fire after the source cooldown.
        viewer.sample_pose().unwrap();
        let mut keys = ButtonInput::default();
        let mut mouse = ButtonInput::default();
        let mut controls = giants_game::control_options::ControlOptions::default();
        assert!(controls.bind(0, 0, "KeyK"));
        let mut fire_press = crate::mecc_input::ActionPress::default();
        keys.press(KeyCode::KeyK);
        keys.release(KeyCode::KeyK);
        assert!(!crate::mecc_input::pressed(
            Some(&controls),
            crate::mecc_input::Action::Fire,
            &keys,
            &mouse,
            false,
        ));
        fire_press.sample(
            Some(&controls),
            crate::mecc_input::Action::Fire,
            &keys,
            &mouse,
            true,
        );
        keys.clear();
        for _ in 0..3 {
            fire_press.sample(
                Some(&controls),
                crate::mecc_input::Action::Fire,
                &keys,
                &mouse,
                true,
            );
        }
        assert!(
            viewer
                .mission_fire_step(1. / 120., fire_press.take(false), &asset.terrain)
                .unwrap()
                .fired
        );
        for _ in 0..90 {
            assert!(
                !viewer
                    .mission_fire_step(1. / 120., fire_press.take(false), &asset.terrain)
                    .unwrap()
                    .fired
            );
        }
        mouse.press(MouseButton::Left);
        mouse.release(MouseButton::Left);
        fire_press.sample(None, crate::mecc_input::Action::Fire, &keys, &mouse, true);
        assert!(
            viewer
                .mission_fire_step(1. / 120., fire_press.take(false), &asset.terrain)
                .unwrap()
                .fired
        );
        assert_eq!(viewer.mission_pistol.as_ref().unwrap().fired, 2);
        mouse.clear();
        mouse.press(MouseButton::Left);
        mouse.release(MouseButton::Left);
        fire_press.sample(None, crate::mecc_input::Action::Fire, &keys, &mouse, true);
        fire_press.sample(None, crate::mecc_input::Action::Fire, &keys, &mouse, false);
        for _ in 0..90 {
            assert!(
                !viewer
                    .mission_fire_step(1. / 120., fire_press.take(false), &asset.terrain)
                    .unwrap()
                    .fired
            );
        }
        assert_eq!(viewer.mission_pistol.as_ref().unwrap().fired, 2);
        println!(
            "normal_fire_press_control keyboard=1 mouse=1 lost_focus_discarded=true repeated_after_cooldown=false supplied_input_flags=true os_input=false"
        );
        let mut payload = payload();
        payload.parameters = packet.slot_parameters(&root);
        payload.packet = packet;
        payload.plan = plan;
        payload.player = Ok(player);
        payload.world = Ok(asset.clone());
        let mut world = World::new();
        let mut queue = CommandQueue::default();
        let (scene_root, actors, blocked, residents, bindings) =
            spawn(&mut Commands::new(&mut queue, &world), 998, &mut payload);
        queue.apply(&mut world);
        let mut runtime = Runtime {
            active: Some(Active {
                ticket: 998,
                root: scene_root,
                actors,
                resident_states: residents,
                blocked,
                payload,
                start: StartState {
                    intent: LaunchIntent::SavedMission,
                    progress: Progress::load(None, None).unwrap(),
                },
                reported: false,
                pickup: None,
                variant_random: giants_game::audio_parameters::Random(1),
                audio_registry: None,
                bindings,
                controlled_mecc_owner: None,
                owned_mecc: None,
                landing: None,
                campaign: None,
            }),
            ..Default::default()
        };
        let active = runtime.active.as_ref().unwrap();
        let owner = active.player_owner().unwrap();
        let children = active.resident_states.mecc_special(owner).unwrap().children;
        let constructor_channels =
            children.map(|id| active.resident_states.get(id).unwrap().channels);
        let mut presentation =
            crate::mission_player::jet::Presentation::new(&active.resident_states, owner).unwrap();
        let models = runtime.owned_jet_models(998).unwrap();
        let mut visible = false;
        let mut boosted = false;
        let mut minimum_fuel = 100f32;
        let mut released = false;
        let mut descent = false;
        let mut landing_requests = 0;
        #[derive(Default)]
        struct LandingTransport {
            born: Vec<ActorId>,
            stopped: Vec<ActorId>,
            released: Vec<ActorId>,
        }
        impl giants_game::positional_impact::Cue for LandingTransport {
            fn play(
                &mut self,
                s: &mut giants_game::resident_states::Session,
                id: ActorId,
                _: u32,
                _: &mut giants_game::audio_parameters::Random,
            ) -> Result<(), String> {
                assert!(!s.positional_impact(id).unwrap().complete);
                self.born.push(id);
                Ok(())
            }
        }
        impl giants_game::positional_impact::Stop for LandingTransport {
            fn stop(
                &mut self,
                s: &mut giants_game::resident_states::Session,
                id: ActorId,
                _: u32,
            ) -> Result<(), String> {
                assert_eq!(
                    s.get(id)
                        .unwrap()
                        .construction_fields
                        .as_ref()
                        .unwrap()
                        .object_flags
                        & 0x8000,
                    0
                );
                self.stopped.push(id);
                Ok(())
            }
        }
        impl giants_game::actor_retirement::Services for LandingTransport {
            fn release(
                &mut self,
                s: &mut giants_game::resident_states::Session,
                id: ActorId,
            ) -> Result<(), String> {
                assert_ne!(
                    s.get(id)
                        .unwrap()
                        .construction_fields
                        .as_ref()
                        .unwrap()
                        .object_flags
                        & 0x8000,
                    0
                );
                self.released.push(id);
                Ok(())
            }
        }
        let mut landing_audio = LandingTransport::default();
        let mut impact_draws = 0;
        assert!(
            runtime
                .active
                .as_ref()
                .unwrap()
                .payload
                .controller
                .as_ref()
                .unwrap()
                .is_none(),
            "staging controller must have moved into its ECS owner"
        );
        assert!(!runtime.normal_landing_supported(998));
        assert!(runtime.publish_player_noise(998).is_err());
        assert!(runtime.sample_world_clock(999, 1000).is_err());
        for number in 1..=1600 {
            runtime
                .sample_world_clock(998, 1000 + number as u32)
                .unwrap();
            viewer.mission_look(Vec2::new(0.0001, 0.), false);
            let frame = viewer
                .step(
                    1. / 120.,
                    if (64..96).contains(&number) {
                        Vec2::Y
                    } else {
                        Vec2::ZERO
                    },
                    &asset.terrain,
                    (16..420).contains(&number),
                    number == 32,
                )
                .unwrap();
            runtime
                .publish_controlled_mecc_motion(998, number, frame)
                .unwrap();
            if [64, 1200].contains(&number) {
                runtime.publish_player_noise(998).unwrap();
                let noise = &runtime.active.as_ref().unwrap().payload.signals.noise;
                assert_eq!(noise.owner, Some(owner));
                assert_eq!(
                    noise.position.map(f32::to_bits),
                    frame.position.map(f32::to_bits)
                );
                assert_eq!(
                    noise.seconds.to_bits(),
                    ((f64::from(1000 + number as u32) * f64::from(0.001_f32)) as f32).to_bits()
                );
                assert!(runtime.publish_player_noise(999).is_err());
            }
            assert!(runtime.normal_landing_supported(998));
            assert!(!runtime.normal_landing_supported(999));
            let (session, bound) = runtime.controlled_jet_session_mut(998).unwrap();
            assert_eq!(bound, owner);
            let frames = presentation
                .advance(session, number, 1. / 120., viewer.normal_jet_parent())
                .unwrap();
            assert_eq!(frames.len(), models.len());
            for (frame, model) in frames.iter().zip(&models) {
                assert_eq!(frame.positions.len(), model.mesh.positions.len());
                assert!(frame.pose.world.values.iter().all(|v| v.is_finite()));
                visible |= !frame.pose.hidden;
                boosted |= frame.pose.class == 781 && !frame.pose.hidden;
            }
            minimum_fuel = minimum_fuel.min(viewer.mission_snapshot().1);
            descent |= viewer.mission_snapshot().2 == 41;
            let landing = viewer.take_landing_impact();
            if let Some(position) = landing {
                assert!(position.into_iter().all(f32::is_finite));
                assert!(number > 420, "landing must follow actual thrust release");
                landing_requests += 1;
            }
            runtime
                .advance_normal_landing(998, 1. / 120., landing, &mut landing_audio)
                .unwrap();
            let (_, draws) = runtime
                .smoke_draws(viewer.camera().0.view)
                .unwrap()
                .unwrap();
            impact_draws += draws.len();
            if number > 720 {
                released |= frames.iter().all(|frame| frame.pose.hidden);
            }
            viewer.sample_pose().unwrap();
        }
        assert!(visible && boosted && released && descent && minimum_fuel < 99.);
        assert!(
            landing_requests > 0,
            "source flight must produce an actual terrain landing request"
        );
        assert_eq!(landing_audio.born.len(), landing_requests);
        assert_eq!(landing_audio.born, landing_audio.stopped);
        assert_eq!(landing_audio.born, landing_audio.released);
        assert!(impact_draws > 0);
        assert!(!runtime.normal_landing_has_work(998));
        assert!(
            runtime
                .active
                .as_ref()
                .unwrap()
                .landing
                .as_ref()
                .unwrap()
                .contexts_idle()
        );
        let session = &runtime.active.as_ref().unwrap().resident_states;
        assert_ne!(
            constructor_channels,
            children.map(|id| session.get(id).unwrap().channels)
        );
        assert!(
            children
                .iter()
                .all(|&id| session.passive_world_frame(id).unwrap().next_frame == 1600)
        );
        assert!(session.mecc_physical_cycle(owner).is_err());
        cancel_ticket(&mut runtime, &mut Commands::new(&mut queue, &world), 998);
        queue.apply(&mut world);
        assert!(runtime.controlled_jet_session(998).is_err());
        assert!(runtime.sample_world_clock(998, 3000).is_err());
        assert!(runtime.publish_player_noise(998).is_err());
        assert!(!runtime.normal_landing_supported(998));
        assert!(world.get_entity(scene_root).is_err());
        println!(
            "normal_jet_live_body steps=1600 thrust={visible} boost={boosted} release={released} descent={descent} minimum_fuel={minimum_fuel} canonical_child_callbacks=true copied_animation_clock=false supplied_commands=true gpu=false real_input=false"
        );
    }
    #[test]
    #[ignore = "Requires owned source clone and real audio device; zero PCM only, gain zero, no window"]
    fn source_owned_mecc_jet_with_reserved_zero_pcm_device() {
        assert!(
            crate::mission_audio::muted(),
            "requires GIANTS_AUDIO_MUTE=1"
        );
        check_source_owned_mecc_runtime(true);
    }
    fn check_source_owned_mecc_runtime(device: bool) {
        let root = std::path::PathBuf::from(
            std::env::var("GIANTS_MISSION_OWNED_ROOT").expect("owned source clone required"),
        );
        let request = giants_game::mission_selection::LoadRequest {
            world: "Story1".into(),
            menu_index: 0,
            source_id: 3,
            mission_name: "1c_1_firstmecc".into(),
            splash: giants_game::mission_selection::Splash::Mecc,
        };
        let packet = Arc::new(giants_game::mission_loading::prepare(&root, request).unwrap());
        let asset = Arc::new(packet.world_asset(&root).unwrap());
        let plan = packet.instance_plan(&root, "English").unwrap();
        let player = Arc::new(
            giants_game::player_initial::Prepared::load(
                &root,
                &plan,
                packet.world.as_ref().unwrap(),
                packet.mission.as_ref().unwrap(),
            )
            .unwrap(),
        );
        let mut viewer =
            crate::mecc_dev_scene::State::load_mission(&root, &asset, &packet, &player).unwrap();
        let mut payload = payload();
        payload.carried = packet
            .mission
            .as_ref()
            .unwrap()
            .slot_writes
            .iter()
            .find_map(|w| {
                if let giants_formats::mission::SlotWriteKind::PlayerChild12(class) = w.kind {
                    Some(Ok(Some((
                        w.source_offset,
                        Arc::new(
                            giants_game::carried_actor::Prepared::load(
                                plan.resources(),
                                class as i32,
                            )
                            .unwrap(),
                        ),
                    ))))
                } else {
                    None
                }
            })
            .unwrap_or(Ok(None));
        payload.player = Ok(player);
        payload.world = Ok(asset.clone());
        payload.parameters = packet.slot_parameters(&root);
        payload.plan = plan;
        payload.packet = packet;
        let mut app = App::new();
        if device {
            app.add_plugins((
                MinimalPlugins,
                AssetPlugin::default(),
                bevy::audio::AudioPlugin {
                    global_volume: bevy::audio::GlobalVolume::new(bevy::audio::Volume::Linear(0.)),
                    ..default()
                },
            ));
        }
        let world = app.world_mut();
        let mut queue = CommandQueue::default();
        let (scene_root, actors, blocked, residents, bindings) =
            spawn(&mut Commands::new(&mut queue, world), 991, &mut payload);
        queue.apply(world);
        let mut runtime = Runtime {
            active: Some(Active {
                ticket: 991,
                root: scene_root,
                actors,
                resident_states: residents,
                blocked,
                payload,
                start: StartState {
                    intent: LaunchIntent::SavedMission,
                    progress: Progress::load(None, None).unwrap(),
                },
                reported: false,
                pickup: None,
                variant_random: giants_game::audio_parameters::Random(1),
                audio_registry: None,
                bindings,
                controlled_mecc_owner: None,
                owned_mecc: None,
                landing: None,
                campaign: None,
            }),
            ..Default::default()
        };
        runtime.initialize_owned_pickup(991);
        {
            let active = runtime.active.as_mut().unwrap();
            let owner = active.resident_states.unique_class(2).unwrap();
            let before = active
                .resident_states
                .get(owner)
                .unwrap()
                .seed
                .storage
                .data()
                .unwrap()
                .to_vec();
            assert_eq!(
                active.resident_states.mecc_selected_weapon(owner).unwrap(),
                1
            );
            assert_eq!(
                active.resident_states.mecc_ammunition(owner, 1).unwrap(),
                giants_game::mecc_ammunition::Availability {
                    amount: 1,
                    slot: giants_game::mecc_ammunition::Slot::Null
                }
            );
            active
                .resident_states
                .consume_mecc_base_pistol(owner)
                .unwrap();
            assert_eq!(
                active
                    .resident_states
                    .get(owner)
                    .unwrap()
                    .seed
                    .storage
                    .data()
                    .unwrap(),
                before
            );
            for other in active.resident_states.list(0) {
                if active.resident_states.get(other).unwrap().class != 2 {
                    assert!(active.resident_states.mecc_ammunition(other, 1).is_err());
                    assert!(active.resident_states.mecc_selected_weapon(other).is_err());
                    assert!(
                        active
                            .resident_states
                            .consume_mecc_base_pistol(other)
                            .is_err()
                    );
                }
            }
        }
        let mut audio_state = if device {
            crate::mission_audio::install_zero_reserved_fixture(app.world_mut(), 991);
            Some(bevy::ecs::system::SystemState::<
                crate::mission_audio::OwnedAudio,
            >::new(app.world_mut()))
        } else {
            None
        };
        let initial = viewer.mission_snapshot().0;
        let mut maximum_height = initial[2];
        let mut minimum_fuel = viewer.mission_snapshot().1;
        let mut first_positions = None;
        let mut pose_changed = false;
        let mut angle_changed = false;
        let mut inputs = giants_game::player_input_channels::State::default();
        let frame_limit = std::env::var("GIANTS_MECC_RUNTIME_FRAMES")
            .map(|value| {
                value
                    .parse::<u64>()
                    .expect("owned frame limit must fit u64")
            })
            .unwrap_or(128);
        assert!(
            (128..=4096).contains(&frame_limit),
            "owned frame limit outside bounded control"
        );
        let thrust_until = std::env::var("GIANTS_MECC_RUNTIME_THRUST_UNTIL")
            .map(|v| v.parse::<u64>().expect("thrust endpoint must fit u64"))
            .unwrap_or(48);
        let boost_until = std::env::var("GIANTS_MECC_RUNTIME_BOOST_UNTIL")
            .map(|v| v.parse::<u64>().expect("boost endpoint must fit u64"))
            .unwrap_or(33);
        assert!((17..=frame_limit).contains(&thrust_until));
        assert!((33..=frame_limit).contains(&boost_until));
        for number in 1..=frame_limit {
            if number == 64 {
                viewer.mission_look(Vec2::ZERO, true);
            }
            let moving = (16..96).contains(&number);
            let commands = crate::mecc_input::owned_commands(
                None,
                &mut inputs,
                if moving && !device {
                    Vec2::new(2., -1.)
                } else {
                    Vec2::ZERO
                },
                true,
                if moving && !device {
                    Vec2::Y
                } else {
                    Vec2::ZERO
                },
                device && (16..thrust_until).contains(&number),
                device && (32..boost_until).contains(&number),
            )
            .unwrap();
            let input = OwnedMeccInput {
                root: &root,
                number,
                commands,
                seconds: 1. / 60.,
                terrain: &asset.terrain,
                objects: viewer.mission_colliders(),
                materials: viewer.mission_contact_materials(),
                first_person: viewer.first_person_view(),
            };
            let result = if let Some(state) = &mut audio_state {
                let deadline = std::time::Instant::now();
                loop {
                    app.update();
                    if state.get_mut(app.world_mut()).unwrap().ready(991) {
                        break;
                    }
                    assert!(
                        deadline.elapsed() < std::time::Duration::from_secs(5),
                        "source driver reservations did not become ready"
                    );
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                let result =
                    state
                        .get_mut(app.world_mut())
                        .unwrap()
                        .with_backend(991, 0.75, |backend| {
                            runtime.advance_owned_mecc(991, input, backend)
                        });
                state.apply(app.world_mut());
                crate::mission_audio::refill_zero_reserved_fixture(app.world_mut());
                result
            } else {
                runtime.advance_owned_mecc(991, input, &mut crate::mission_audio::UnavailableDevice)
            };
            let frame = result.unwrap_or_else(|e| {
                let session=&runtime.active.as_ref().unwrap().resident_states;
                let id=session.unique_class(2).unwrap();let fields=session.get(id).unwrap().construction_fields.as_ref().unwrap();
                panic!("source owned frame{number}: {e}; flags={:x} response={:x} control={:x} action={}",fields.object_flags,fields.response_flags,fields.control_flags,fields.action)
            });
            assert_eq!(frame.number, number);
            let expected_aim = frame.world_aim.as_ref().map(|aim| aim.point);
            assert_eq!(
                expected_aim.is_some(),
                !viewer.first_person_view() && frame.world_weapon.is_some()
            );
            assert!(expected_aim.into_iter().flatten().all(f32::is_finite));
            let session = &runtime.active.as_ref().unwrap().resident_states;
            if number == 1 {
                assert!(frame.world_weapon.is_none());
            }
            if let Some(weapon) = &frame.world_weapon {
                assert_eq!((weapon.number, weapon.epoch), (number, frame.stamp));
                assert_eq!(weapon.pose.binding.parent, frame.owner);
                assert_eq!(weapon.pose.binding.reference, 1);
                assert!(!weapon.positions.is_empty());
                assert!(weapon.positions.iter().flatten().all(|v| v.is_finite()));
                assert_eq!(
                    weapon.pose.point(session, 1).unwrap().map(f32::to_bits),
                    weapon.muzzle.map(f32::to_bits),
                );
            }
            assert_eq!(frame.children.len(), 3);
            for (index, child) in frame.children.iter().enumerate() {
                assert_eq!(child.pose.class, [84, 85, 781][index]);
                assert!(!child.positions.is_empty());
                assert!(child.positions.iter().flatten().all(|v| v.is_finite()));
                let resident = runtime
                    .active
                    .as_ref()
                    .unwrap()
                    .resident_states
                    .get(child.pose.id)
                    .unwrap();
                assert_eq!(
                    resident.channels.unwrap(),
                    resident.construction_fields.as_ref().unwrap().channels
                );
            }
            maximum_height = maximum_height.max(frame.position[2]);
            minimum_fuel = minimum_fuel.min(frame.jet.fuel);
            assert!(!frame.positions.is_empty());
            let pose = frame
                .positions
                .iter()
                .map(|p| p.map(f32::to_bits))
                .collect::<Vec<_>>();
            if let Some(first) = &first_positions {
                pose_changed |= *first != pose;
            } else {
                first_positions = Some(pose);
            }
            angle_changed |= (frame.angles[0] - viewer.initial_yaw.to_degrees()).abs() > 0.01;
            let had_weapon = frame.world_weapon.is_some();
            viewer.accept_owned_pose(frame).unwrap();
            if let Some(target) = expected_aim {
                assert_eq!(
                    viewer.combat_pose().unwrap().1.map(f32::to_bits),
                    target.map(f32::to_bits)
                );
            } else {
                assert!(viewer.combat_pose().is_err());
            }
            assert!(
                viewer
                    .mission_fire_step(1. / 60., true, &asset.terrain)
                    .is_err()
            );
            viewer.sample_pose().unwrap();
            assert_eq!(viewer.weapon_matrix().is_ok(), had_weapon);
            assert_eq!(viewer.render_weapon_matrix().unwrap().is_some(), had_weapon);
            assert!(viewer.camera().1.into_iter().all(f32::is_finite));
            assert!(viewer.expanded(0).into_iter().flatten().all(f32::is_finite));
            assert!(
                viewer
                    .step(1. / 60., Vec2::ZERO, &asset.terrain, false, false)
                    .is_err()
            );
        }
        let final_position = viewer.mission_snapshot().0;
        if device {
            assert!(maximum_height > initial[2] + 1. && minimum_fuel < 99.);
            let started = crate::mission_audio::finish_zero_reserved_fixture(app.world_mut());
            assert!(
                started >= 4,
                "source jet did not start/loop/boost/release real sinks"
            );
            println!(
                "mission_owned_jet_device_source steps={frame_limit} initial_height={} maximum_height={maximum_height} minimum_fuel={minimum_fuel} real_sink_starts={started} zero_pcm=true muted=true world_ready=false",
                initial[2]
            );
        } else {
            assert!((final_position[0] - initial[0]).hypot(final_position[1] - initial[1]) > 1.);
        }
        assert!(pose_changed && (device || angle_changed) && viewer.first_person_view());
        let legacy_attempt = runtime.publish_controlled_mecc_motion(
            991,
            1,
            giants_game::controlled_mecc::Frame {
                before: final_position,
                position: final_position,
                contact_previous: final_position,
                angles: [0.; 3],
                scale: viewer.scale,
                velocity: [0.; 3],
                model_rates: [0.; 3],
            },
        );
        assert!(legacy_attempt.unwrap_err().contains("owned primary"));
        assert!(
            runtime
                .active
                .as_ref()
                .unwrap()
                .controlled_mecc_owner
                .is_none()
        );
        let active = runtime.active.as_mut().unwrap();
        let owner = active.resident_states.unique_class(2).unwrap();
        let children = active.resident_states.mecc_special(owner).unwrap().children;
        let clocks = children.map(|id| active.resident_states.get(id).unwrap().channels);
        let mut stale =
            giants_game::mecc_jet_presentation::Children::new(&active.resident_states, owner)
                .unwrap();
        assert!(
            stale
                .advance(&mut active.resident_states, 1, 1. / 60., 1)
                .is_err()
        );
        assert!(
            stale
                .advance(&mut active.resident_states, 1, 1. / 60., 1)
                .err()
                .unwrap()
                .contains("quarantined")
        );
        assert_eq!(
            clocks,
            children.map(|id| active.resident_states.get(id).unwrap().channels)
        );
        let fields = active
            .resident_states
            .get(owner)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap();
        assert_eq!(
            viewer.mission_snapshot().0.map(f32::to_bits),
            fields.position.map(f32::to_bits)
        );
        assert!(
            active
                .resident_states
                .controlled_mecc_motion(owner, frame_limit)
                .is_err()
        );
        let before = active
            .resident_states
            .get(owner)
            .unwrap()
            .seed
            .storage
            .data()
            .unwrap()
            .to_vec();
        let channels = active.resident_states.get(owner).unwrap().channels;
        let stamp = active
            .resident_states
            .get(owner)
            .unwrap()
            .mecc_animation_stamp;
        let random = active.variant_random.0;
        assert!(
            giants_game::mecc_presentation::sample(
                &mut active.resident_states,
                owner,
                frame_limit - 1,
                false
            )
            .is_err()
        );
        let repeated = giants_game::mecc_presentation::sample(
            &mut active.resident_states,
            owner,
            frame_limit,
            true,
        )
        .unwrap();
        assert!(viewer.accept_owned_pose(repeated).is_err());
        assert_eq!(
            before,
            active
                .resident_states
                .get(owner)
                .unwrap()
                .seed
                .storage
                .data()
                .unwrap()
        );
        assert_eq!(
            channels,
            active.resident_states.get(owner).unwrap().channels
        );
        assert_eq!(
            stamp,
            active
                .resident_states
                .get(owner)
                .unwrap()
                .mecc_animation_stamp
        );
        assert_eq!(random, active.variant_random.0);
        active
            .resident_states
            .update_references_unblended(owner, 1, 0.123)
            .unwrap();
        assert!(
            giants_game::mecc_presentation::sample(
                &mut active.resident_states,
                owner,
                frame_limit,
                true
            )
            .is_err()
        );
        println!(
            "mission_owned_primary_source steps={frame_limit} real_terrain=true real_colliders=true owned_positions_camera_hand=true native_input_producers=true movement_turn_view={} development_physics_refused=true ready=false",
            !device
        );
    }

    #[test]
    #[ignore = "Requires GIANTS_HERD_DECLARATION_ROOT pointing to the owned reference clone; CPU-only, no window or input"]
    fn source_herd_declarations_bind_and_release() {
        let root = std::path::PathBuf::from(
            std::env::var("GIANTS_HERD_DECLARATION_ROOT").expect("owned source clone required"),
        );
        for (catalog, mission_file, expected_count) in [
            ("w_Kstory1.bin", None, 10),
            ("w_Story1.bin", Some("wm_1c_1_firstmecc.bin"), 1),
        ] {
            let resources = Arc::new(
                giants_game::scene_resources::SceneResources::load(&root, catalog, "English")
                    .unwrap(),
            );
            let world_bytes = std::fs::read(root.join("Bin").join(catalog)).unwrap();
            let declarations = giants_formats::world::parse_world(&world_bytes).unwrap();
            let mission_bytes = mission_file
                .map(|name| std::fs::read(root.join("Bin").join(name)).unwrap())
                .unwrap_or_else(|| vec![4, 0, 0, 0, 1, 2]);
            let mission = giants_formats::mission::parse(&mission_bytes).unwrap();
            let mut payload = payload();
            payload.plan = Plan::prepare_scene(&resources, &mission, &declarations.placements);
            let packet = Arc::get_mut(&mut payload.packet).unwrap();
            packet.world = Some(declarations);
            packet.world_bytes = world_bytes;
            packet.mission = Some(mission);
            packet.mission_bytes = mission_bytes;
            let mut world = World::new();
            let external = world.spawn(Name::new("unrelated owner entity")).id();
            let mut queue = CommandQueue::default();
            let (scene_root, actors, blocked, residents, bindings) =
                spawn(&mut Commands::new(&mut queue, &world), 987, &mut payload);
            queue.apply(&mut world);
            use giants_game::declared_residents::{Key, Scope};
            let world_keys = payload
                .plan
                .world_entries
                .iter()
                .enumerate()
                .filter(|(_, entry)| matches!(entry.declaration.object_type, 49 | 679))
                .map(|(index, entry)| {
                    (
                        Key {
                            scope: Scope::World,
                            index,
                        },
                        entry.declaration.object_type,
                    )
                });
            let mission_keys = payload
                .plan
                .entries
                .iter()
                .enumerate()
                .filter(|(_, entry)| entry.serialized.object_type == 679)
                .map(|(index, entry)| {
                    (
                        Key {
                            scope: Scope::Mission,
                            index,
                        },
                        entry.serialized.object_type,
                    )
                });
            let expected = world_keys
                .chain(mission_keys)
                .map(|(key, class)| {
                    let id = residents
                        .declaration(key)
                        .expect("source typed declaration missing");
                    if class == 49 {
                        assert!(residents.herd_clock(id).is_some());
                    } else {
                        assert!(residents.herd_marker(id).is_some());
                    }
                    if key.scope == Scope::Mission {
                        let fields = residents
                            .get(id)
                            .unwrap()
                            .construction_fields
                            .as_ref()
                            .unwrap();
                        assert_eq!(fields.action, 1);
                        assert_eq!(fields.object_flags & 0x420, 0x420);
                        assert_ne!(fields.response_flags & 0x20000000, 0);
                        assert!(residents.actor_current_affine(id).is_ok());
                    }
                    (key, id)
                })
                .collect::<Vec<_>>();
            assert_eq!(expected.len(), expected_count);
            world.insert_resource(Runtime {
                active: Some(Active {
                    ticket: 987,
                    root: scene_root,
                    actors,
                    resident_states: residents,
                    blocked,
                    payload,
                    start: StartState {
                        intent: LaunchIntent::SavedMission,
                        progress: Progress::load(None, None).unwrap(),
                    },
                    reported: false,
                    pickup: None,
                    variant_random: giants_game::audio_parameters::Random(1),
                    audio_registry: None,
                    bindings,
                    controlled_mecc_owner: None,
                    owned_mecc: None,
                    landing: None,
                    campaign: None,
                }),
                ..Default::default()
            });
            let mut bind = Schedule::default();
            bind.add_systems(bind_declared_residents);
            bind.run(&mut world);
            let actual = world
                .query::<(&ActorRef, &OwnedActor)>()
                .iter(&world)
                .map(|(reference, id)| (declaration_key(reference), id.0))
                .collect::<Vec<_>>();
            for pair in expected {
                assert_eq!(actual.iter().filter(|&&p| p == pair).count(), 1);
            }
            let mut runtime = world.remove_resource::<Runtime>().unwrap();
            cancel_ticket(&mut runtime, &mut Commands::new(&mut queue, &world), 987);
            queue.apply(&mut world);
            assert!(runtime.active.is_none());
            assert_eq!(world.query::<&OwnedActor>().iter(&world).count(), 0);
            assert_eq!(world.query::<&MissionOwned>().iter(&world).count(), 0);
            assert!(world.get_entity(external).is_ok());
        }
    }

    #[test]
    fn session_spawns_source_owned_actors_and_release_preserves_other_entities() {
        let mut world = World::new();
        let external = world.spawn(Name::new("unrelated menu entity")).id();
        let mut queue = CommandQueue::default();
        let mut payload = payload();
        let (root, actors, blocked, resident_states, bindings) =
            spawn(&mut Commands::new(&mut queue, &world), 9, &mut payload);
        queue.apply(&mut world);
        assert_eq!(actors, 2); // Mission and initial world pose are separate owners.
        assert!(blocked.iter().any(|s| s.contains("actor callbacks")));
        let mut query =
            world.query::<(&MissionOwned, &ActorRef, &Transform, &ChildOf, &Visibility)>();
        for (owned, reference, transform, parent, visibility) in query.iter(&world) {
            assert_eq!(owned.0, 9);
            assert_eq!(reference.ticket, 9);
            assert_eq!(reference.index, 0);
            assert_eq!(parent.parent(), root);
            let expected = if reference.world_declaration {
                Vec3::new(4., 6., -5.)
            } else {
                Vec3::new(1., 3., -2.)
            };
            assert_eq!(transform.translation, expected);
            assert_eq!(*visibility, Visibility::Hidden);
        }
        assert_eq!(world.query::<&Mesh3d>().iter(&world).count(), 0);
        let mut runtime = Runtime {
            active: Some(Active {
                ticket: 9,
                root,
                actors,
                resident_states,
                blocked,
                payload,
                start: StartState {
                    intent: LaunchIntent::SavedMission,
                    progress: Progress::load(None, None).unwrap(),
                },
                reported: false,
                pickup: None,
                variant_random: giants_game::audio_parameters::Random(1),
                audio_registry: None,
                bindings,
                controlled_mecc_owner: None,
                owned_mecc: None,
                landing: None,
                campaign: None,
            }),
            ..Default::default()
        };
        runtime.active.as_mut().unwrap().start.intent = LaunchIntent::CompletionReplay;
        assert!(
            matches!(runtime.startup(), Err(reason) if reason.contains("replay is an inspection context"))
        );
        runtime.active.as_mut().unwrap().start.intent = LaunchIntent::SavedMission;
        cancel_ticket(&mut runtime, &mut Commands::new(&mut queue, &world), 8);
        queue.apply(&mut world);
        assert!(runtime.active.is_some());
        assert!(world.get_entity(root).is_ok());
        cancel_ticket(&mut runtime, &mut Commands::new(&mut queue, &world), 9);
        queue.apply(&mut world);
        assert!(runtime.active.is_none());
        assert_eq!(world.query::<&MissionOwned>().iter(&world).count(), 0);
        assert!(world.get_entity(external).is_ok());
        // Repeated cancellation is harmless.
        clear(&mut runtime, &mut Commands::new(&mut queue, &world));
        queue.apply(&mut world);
        assert!(world.get_entity(external).is_ok());
    }

    #[test]
    fn dropped_pending_signals_worker_and_rejects_late_delivery() {
        let cancel = Arc::new(AtomicBool::new(false));
        let (sender, receiver) = mpsc::channel();
        let pending = Pending {
            ticket: 4,
            cancel: Arc::clone(&cancel),
            receiver: Mutex::new(receiver),
            start: StartState {
                intent: LaunchIntent::NewCampaign,
                progress: Progress::load(None, None).unwrap(),
            },
        };
        assert!(!cancel.load(Ordering::Relaxed));
        drop(pending);
        assert!(cancel.load(Ordering::Relaxed));
        assert!(sender.send(Err("late result".into())).is_err());
    }

    #[test]
    fn stale_tickets_and_non_runtime_phases_are_not_live() {
        for phase in [
            Phase::AwaitingRuntime,
            Phase::Cinematic,
            Phase::Objectives,
            Phase::Gameplay,
        ] {
            assert!(live(3, 3, phase));
            assert!(!live(2, 3, phase));
        }
        for phase in [Phase::Menu, Phase::Splash, Phase::Loading, Phase::Failed] {
            assert!(!live(3, 3, phase));
        }
    }
    #[test]
    #[ignore = "Requires GIANTS_MISSION_OWNED_ROOT; canonical smoke render packets, dynamic alpha, expiry/pruning and mission retirement; no GPU"]
    fn source_owned_smoke_draws_follow_actual_actor_lifetime() {
        let root = std::path::PathBuf::from(std::env::var("GIANTS_MISSION_OWNED_ROOT").unwrap());
        let request = giants_game::mission_selection::LoadRequest {
            world: "Story1".into(),
            menu_index: 0,
            source_id: 3,
            mission_name: "1c_1_firstmecc".into(),
            splash: giants_game::mission_selection::Splash::Mecc,
        };
        let packet = Arc::new(giants_game::mission_loading::prepare(&root, request).unwrap());
        let plan = packet.instance_plan(&root, "English").unwrap();
        let mut payload = payload();
        payload.packet = packet;
        payload.plan = plan;
        let mut world = World::new();
        let mut queue = CommandQueue::default();
        let (scene_root, actors, blocked, residents, bindings) =
            spawn(&mut Commands::new(&mut queue, &world), 970, &mut payload);
        queue.apply(&mut world);
        let mut runtime = Runtime {
            active: Some(Active {
                ticket: 970,
                root: scene_root,
                actors,
                resident_states: residents,
                blocked,
                payload,
                start: StartState {
                    intent: LaunchIntent::SavedMission,
                    progress: Progress::load(None, None).unwrap(),
                },
                reported: false,
                pickup: None,
                variant_random: giants_game::audio_parameters::Random(1),
                audio_registry: None,
                bindings,
                controlled_mecc_owner: None,
                owned_mecc: None,
                landing: None,
                campaign: None,
            }),
            ..Default::default()
        };
        let active = runtime.active.as_mut().unwrap();
        let resources = active.payload.plan.resources();
        let offset = resources
            .classes
            .models
            .effects
            .iter()
            .find_map(|e| {
                e.image
                    .find("fx_mecc_smoke_lite")
                    .map(|(_, entry)| entry.offset)
            })
            .unwrap();
        let mut pool =
            giants_game::attached_effect_context::Pool::from_active_resources(970, resources);
        let mut random = giants_game::audio_parameters::Random(42);
        let (creation, _) = pool
            .prepare_retirement_offset_invariant(Some(offset), [17., -24., 30.], 0, 0, &mut random)
            .unwrap();
        let giants_game::attached_effect_context::Creation::Prepared(context) = creation else {
            panic!("source child absent")
        };
        let actor = pool
            .activate_smoke_particle(context, &mut active.resident_states, resources, &mut random)
            .unwrap();
        let view = giants_game::procedural_aim::Matrix::identity().values;
        assert!(runtime.smoke_draws(view).unwrap().unwrap().1.is_empty());
        let active = runtime.active.as_mut().unwrap();
        let mut stamp = giants_game::animation_epoch::Stamp(0);
        active
            .resident_states
            .advance_smoke_particle(actor, 0.01, None, &mut stamp)
            .unwrap();
        let saved = active
            .resident_states
            .get(actor)
            .unwrap()
            .seed
            .storage
            .data()
            .unwrap()
            .to_vec();
        let weak = Arc::downgrade(active.resident_states.specification(actor).unwrap());
        let (ticket, draws) = runtime.smoke_draws(view).unwrap().unwrap();
        assert_eq!(ticket, 970);
        assert_eq!(draws.len(), 1);
        assert_eq!(draws[0].id, actor);
        assert_eq!(draws[0].frame.positions.len(), 4);
        assert_eq!(draws[0].frame.profiles[0].blend, 2);
        assert!(draws[0].frame.profiles[0].texture_alpha);
        let geometry = draws[0].geometry.clone();
        let alpha = draws[0].frame.profiles[0].alpha;
        let active = runtime.active.as_mut().unwrap();
        assert_eq!(
            active
                .resident_states
                .get(actor)
                .unwrap()
                .seed
                .storage
                .data()
                .unwrap(),
            saved
        );

        active
            .resident_states
            .advance_smoke_particle(actor, 0.2, Some(0.2), &mut stamp)
            .unwrap();
        let (_, draws) = runtime.smoke_draws(view).unwrap().unwrap();
        assert_eq!(draws.len(), 1);
        assert!(draws[0].frame.profiles[0].alpha < alpha);
        assert!(Arc::ptr_eq(&geometry, &draws[0].geometry));
        let asset = Arc::new(giants_game::load_world(&root, "Story1").unwrap());
        runtime = crate::mission_smoke_scene::check_render_assets(runtime, asset, view);
        let active = runtime.active.as_mut().unwrap();
        active
            .resident_states
            .advance_smoke_particle(actor, 10., None, &mut stamp)
            .unwrap();
        assert!(runtime.smoke_draws(view).unwrap().unwrap().1.is_empty());
        assert!(runtime.smoke_geometry.is_empty());
        let active = runtime.active.as_mut().unwrap();
        active
            .resident_states
            .destroy_expired_smoke_particle(actor, &pool)
            .unwrap();
        assert!(weak.upgrade().is_none());
        assert!(
            geometry
                .frame(&mut active.resident_states, actor, view)
                .is_err()
        );
        runtime.active = None;
        assert!(runtime.smoke_draws(view).unwrap().is_none());
        assert!(runtime.smoke_geometry.is_empty());
    }
}
