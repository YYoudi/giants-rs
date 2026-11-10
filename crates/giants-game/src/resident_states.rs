//! Ticket-owned copies of prepared actor payloads and ordered registrations.
//! Acquired callback fields are owned separately; general actor behavior and
//! mission readiness remain incomplete.
use crate::{
    model_registry::StateBinding,
    object_links::Links,
    object_lists::DirectFlags,
    state_storage::{Profile, Seed},
};
use std::collections::BTreeMap;
use std::sync::Arc;
mod delphi;
mod delphi_camera;
mod delphi_held;
mod herd_frame;
#[cfg(test)]
mod herd_frame_tests;
mod parent_binding;
pub use parent_binding::AttachmentWords;
#[cfg(test)]
mod delphi_target_tests;
mod kabuto;
mod mecc_impact;
pub use mecc_impact::Voice as MeccImpactVoice;
mod reference_camera;
mod state18_reference_reset;
mod subjective;
mod timed_effect;
#[cfg(test)]
mod vimp_history_tests;
#[cfg(test)]
mod vimp_newborn_tests;
pub use timed_effect::Services as TimedEffectServices;

struct OwnedSpecification(crate::actor_references::Owned);
impl std::fmt::Debug for OwnedSpecification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OwnedSpecification")
            .field("class", &self.0.specification().class)
            .field("specification", &self.0.specification().specification)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Id {
    ticket: u64,
    serial: u64,
}
impl Id {
    pub fn ticket(self) -> u64 {
        self.ticket
    }
}
/// One ticket-bound common-registration traversal. A completed/failed token
/// cannot replay; it is not a global frame or mission-readiness certificate.
#[derive(Debug)]
pub struct CommonWalk {
    ticket: u64,
    inner: crate::object_list_walk::Walk,
}
impl CommonWalk {
    pub fn phase(&self) -> crate::object_list_walk::Phase {
        self.inner.phase()
    }
    pub fn visits(&self) -> usize {
        self.inner.visits()
    }
    pub fn failure(&self) -> Option<&str> {
        self.inner.failure()
    }
}
struct CommonWalkAdapter<'a, F> {
    session: &'a mut Session,
    visitor: &'a mut F,
}
impl<F> crate::object_list_walk::Services for CommonWalkAdapter<'_, F>
where
    F: FnMut(&mut Session, Id) -> Result<crate::object_list_walk::Flow, String>,
{
    type Node = crate::object_links::LinkId;
    fn first(&self) -> Result<Option<Self::Node>, String> {
        Ok(self.session.links.first_node(0))
    }
    fn object(&self, node: Self::Node) -> Result<u64, String> {
        let serial = self.session.links.node_object(0, node)?;
        if !self.session.residents.contains_key(&serial) {
            return Err("common walk has an unowned actor".into());
        }
        Ok(serial)
    }
    fn next(&self, node: Self::Node) -> Result<Option<Self::Node>, String> {
        self.session.links.next_node(0, node).map_err(str::to_owned)
    }
    fn visit(&mut self, serial: u64) -> Result<crate::object_list_walk::Flow, String> {
        let id = Id {
            ticket: self.session.ticket,
            serial,
        };
        (self.visitor)(self.session, id)
    }
}
/// Ordered view of an explicitly requested subset of this session. This does
/// not assert that all native mission objects or lists have been populated.
pub struct OwnedPerceptionSubset {
    pub candidates: Vec<crate::ripper_perception::Candidate<Id>>,
    pub order: Vec<Id>,
}
impl OwnedPerceptionSubset {
    pub fn frame(&self) -> Result<crate::ripper_perception::Frame<'_, Id>, &'static str> {
        crate::ripper_perception::Frame::new(&self.candidates, &self.order)
    }
}
/// Explicit current single-player gates. These are not supplied success values
/// for an attacker, sound, impulse, equipment or death service.
#[derive(Clone, Copy)]
pub struct MeccDamageContext {
    pub globally_disabled: bool,
    /// Historical label for the acquired suppression input. Its actual producer
    /// and relationship to cinematic state remain unverified.
    pub cutscene: bool,
    pub shield_percent: f32,
    pub ambient_voice_timer: f32,
}
/// The reciprocal D +274 callback relationship is independent of the +210
/// transform parent. A marker does not follow its owner through this link.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SurfaceLink {
    Owner {
        marker: Id,
    },
    Marker {
        owner: Id,
        look_pitch: f32,
        mode: u32,
        timer: f32,
    },
}
pub struct ProjectileHitServices<'a, S> {
    pub routing: crate::hit_routing::Context,
    pub filter: crate::projectile_contacts::Environment,
    pub callback: crate::state23_damage::Context,
    pub random: &'a mut crate::audio_parameters::Random,
    pub blood: &'a mut crate::blood_particles::Pool,
    pub sounds: &'a mut S,
}
#[derive(Debug)]
pub enum ProjectileHit {
    NoContact,
    RangeRejected {
        target: Id,
    },
    Suppressed {
        target: Id,
    },
    Applied {
        target: Id,
        counts: crate::state23_hurt::Counts,
    },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntryActor {
    pub id: Id,
    pub class: i32,
    pub contribution: crate::mission_selection::EntryContribution,
}
/// Snapshot of this session's common list ONLY. It is not certification that
/// every object from the native world/mission/dynamic registries is present.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnedEntrySubset {
    pub actors: Vec<EntryActor>,
}
impl OwnedEntrySubset {
    pub fn total(&self) -> u32 {
        crate::mission_selection::EntryContribution::total(
            self.actors.iter().map(|a| a.contribution),
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Attachment {
    pub parent: Id,
    pub child_reference: u32,
    pub parent_reference: u32,
    pub behavior: u32,
}
#[derive(Debug, Clone, Copy)]
pub enum PickupService {
    Sound {
        owner: Id,
        id: u32,
        kind: crate::actor_sound::Kind,
    },
    LocalNotice {
        child: Id,
    },
}
/// A callback service may create/update other owned actors. The snapshot is
/// the callback's current state; its local fields are published only on success.
/// External service effects are not rolled back on a later failure.
pub trait State23Services {
    fn execute(
        &mut self,
        session: &mut Session,
        actor: &crate::state23_damage::State<Id>,
        service: crate::state23_damage::Service<Id>,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<(), String>;
}
/// Real attack service for the secondary animation callback. Resource absence
/// is a distinct result; an unavailable backend must return an error.
pub trait State23AnimationServices {
    fn fire(
        &mut self,
        session: &mut Session,
        source: Id,
        state: &crate::state23_animation::State,
        target: [f32; 3],
        vertical_spread_percent: f32,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<crate::state23_animation::FireOutcome, String>;
}
/// Required callbacks for one common-list iteration on an owned D resident.
/// The selector is an explicit current-world input; its presence is resolved
/// against this Session. No behavior, clock or finalizer is acknowledged here.
pub trait ActorWorldServices {
    fn controlled_actor(&self) -> Option<Id>;
    fn primary(&mut self, session: &mut Session, source: Id) -> Result<(), String>;
    fn secondary(&mut self, session: &mut Session, source: Id) -> Result<(), String>;
    fn object_time(&mut self, session: &Session, source: Id) -> Result<f32, String>;
    fn animation(&mut self, session: &mut Session, source: Id, seconds: f32) -> Result<(), String>;
    fn finalization(&mut self, session: &mut Session, source: Id) -> Result<(), String>;
}
struct ActorWorldAdapter<'a, S> {
    session: &'a mut Session,
    source: Id,
    services: &'a mut S,
}
impl<S: ActorWorldServices> crate::actor_world_tick::Services for ActorWorldAdapter<'_, S> {
    fn flags(&self) -> Result<u32, String> {
        self.session.actor_world_flags(self.source)
    }
    fn set_flags(&mut self, flags: u32) -> Result<(), String> {
        self.session
            .residents
            .get_mut(&self.source.serial)
            .and_then(|a| a.construction_fields.as_mut())
            .ok_or("world iteration lost its owned actor fields")?
            .object_flags = flags;
        Ok(())
    }
    fn controlled_actor_present(&self) -> bool {
        self.services
            .controlled_actor()
            .is_some_and(|id| self.session.get(id).is_some())
    }
    fn primary(&mut self) -> Result<(), String> {
        self.services.primary(self.session, self.source)
    }
    fn secondary(&mut self) -> Result<(), String> {
        self.services.secondary(self.session, self.source)
    }
    fn object_time(&mut self) -> Result<f32, String> {
        self.services.object_time(self.session, self.source)
    }
    fn animation(&mut self, seconds: f32) -> Result<(), String> {
        self.services.animation(self.session, self.source, seconds)
    }
    fn finalization(&mut self) -> Result<(), String> {
        self.services.finalization(self.session, self.source)
    }
}
struct State23AnimationAdapter<'a, S> {
    attempted: bool,
    session: &'a mut Session,
    source: Id,
    services: &'a mut S,
}
impl<S: State23AnimationServices> crate::state23_animation::Services
    for State23AnimationAdapter<'_, S>
{
    fn fire(
        &mut self,
        state: &crate::state23_animation::State,
        target: [f32; 3],
        vertical_spread_percent: f32,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<crate::state23_animation::FireOutcome, String> {
        self.attempted = true;
        self.services.fire(
            self.session,
            self.source,
            state,
            target,
            vertical_spread_percent,
            random,
        )
    }
}
struct State23Adapter<'a, S> {
    session: &'a mut Session,
    services: &'a mut S,
}
impl<S: State23Services> crate::state23_damage::Services<Id> for State23Adapter<'_, S> {
    fn execute(
        &mut self,
        actor: &crate::state23_damage::State<Id>,
        service: crate::state23_damage::Service<Id>,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<(), String> {
        self.services.execute(self.session, actor, service, random)
    }
}
#[derive(Debug)]
pub struct Resident {
    herd: Option<crate::herd_generator::Owned>,
    herd_frame: Option<crate::herd_frame::Owned>,
    herd_marker: Option<crate::herd_markers::Marker>,
    vimp: Option<crate::vimp_actor::Cold>,
    vimp_physical: Option<crate::vimp_physical::Owned>,
    vimp_frames: u64,
    vimp_prepared_history: Option<u64>,
    vimp_frame_stopped: bool,
    vimp_chunk: Option<crate::vimp_chunks::Chunk>,
    vimp_hurt_steps: u64,
    vimp_scalar_steps: [u64; 2],
    vimp_recovery_primaries: u64,
    vimp_death_proxy: Option<crate::vimp_death_proxy::Proxy>,
    pub class: i32,
    pub seed: Seed,
    /// Captured identities only; no native pointers or allocated services.
    pub bindings: Vec<StateBinding>,
    pub constructor_mask: u32,
    initial_flags: DirectFlags,
    /// Owned fields initialized by construction, then updated by acquired kernels.
    pub construction_fields: Option<crate::actor_fields::Fields>,
    pub attachment: Option<Attachment>,
    // A removed parent does not clear the acquired reference/behavior words.
    // None means no producer has supplied them, rather than assumed zeros.
    retained_attachment_words: Option<AttachmentWords>,
    pub pickup_notice_pending: bool,
    pickup_notice_delivery: Option<crate::mission_notice::Delivery>,
    /// Numeric channel state, not evaluated poses or shared resource pointers.
    pub channels: Option<crate::animation_channels::Channels>,
    /// Last actual ANM header stamp (historical name); world services allocate it.
    pub mecc_animation_stamp: u32,
    /// Owned presentation metadata; not a constructed native actor pointer.
    presentation_parent: Option<Id>,
    presentation_position: Option<[f32; 3]>,
    /// Acquired callback fields and typed relationships, separate from raw
    /// constructor bytes. No native pointer is encoded as an owned identity.
    state23_combat: Option<crate::state23_damage::State<Id>>,
    mecc_special: Option<crate::mecc_special::Initialized>,
    delphi_special: Option<crate::delphi_special::State>,
    delphi_copy_source: Option<crate::delphi_special::CopySource>,
    delphi_copy_parameter: Option<f32>,
    delphi_held_pose: Option<delphi_held::Owned>,
    delphi_physics_present: bool,
    delphi_physical: Option<crate::delphi_physical::Owned>,
    mecc_special_child: Option<crate::mecc_special::Child>,
    wield_binding: Option<crate::mecc_wield::Binding>,
    wield_failure: Option<String>,
    mecc_shot_failure: Option<String>,
    mecc_local_target: Option<crate::mecc_target_owned::Published>,
    mecc_jet: Option<crate::mecc_jet_owned::Owner>,
    mecc_contact: crate::mecc_secondary::Contact,
    mecc_damage_smoke: crate::mecc_damage_effect::State<crate::attached_effect_context::Id>,
    /// Native accumulated header damage, owned separately from the payload.
    mecc_damage: Option<f32>,
    mecc_physics_present: bool,
    mecc_physical_cycle: Option<crate::mecc_physical_cycle::Owned>,
    affine_cache: Option<OwnedAffine>,
    state23_animation_counter: i32,
    state23_animation_failed: bool,
    acquired_construction: bool,
    kabuto_player_fields: bool,
    kabuto_group: Option<crate::kabuto_initial::Group>,
    basic_projectile: Option<crate::basic_projectile::Metadata>,
    enemy_projectile: Option<crate::enemy_projectile::Metadata>,
    attached_effect: Option<crate::attached_effect_object::Metadata>,
    positional_impact: Option<crate::positional_impact::Metadata>,
    timed_effect: Option<timed_effect::Metadata>,
    environment: Option<crate::resident_environment::Prepared>,
    ordinary_motion: Option<crate::resident_motion::Owned>,
    ripper_primary:
        Option<crate::ripper_primary_turn::Machine<Id, crate::attached_effect_context::Id>>,
    ripper_primary_running: bool,
    controlled_motion: Option<crate::controlled_mecc::Owned>,
    finalization: Option<crate::resident_finalization::Owned>,
    world_tick: Option<crate::actor_world_tick::Tick>,
    world_tick_running: bool,
    surface_link: Option<SurfaceLink>,
}
#[derive(Clone, Debug)]
struct OwnedAffine {
    value: crate::actor_affine::Cache,
    pose: [u32; 7],
    external_orientation: bool,
}
struct SurfaceSource {
    position: [f32; 3],
    current: [f32; 3],
    previous: [f32; 3],
    angles: [f32; 3],
    scale: f32,
    look_pitch: f32,
    combat: crate::state23_damage::State<Id>,
}
fn affine_pose_words(fields: &crate::actor_fields::Fields) -> [u32; 7] {
    [
        fields.position[0].to_bits(),
        fields.position[1].to_bits(),
        fields.position[2].to_bits(),
        fields.angles[0].to_bits(),
        fields.angles[1].to_bits(),
        fields.angles[2].to_bits(),
        fields.scale.to_bits(),
    ]
}
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    pub residents: usize,
    pub payload_bytes: usize,
    pub registrations: usize,
}
/// Complete ownership transfer for the acquired initial state0/state2/state18/state23 profile.
/// Behavioral updates and interaction callbacks remain separate services.
pub struct Construction<'a> {
    pub class: i32,
    pub seed: &'a Seed,
    pub bindings: &'a [StateBinding],
    pub flags: DirectFlags,
    pub specification: Arc<crate::specification::Prepared>,
    pub state_resources: Arc<crate::state_resources::Prepared>,
    pub fields: crate::actor_fields::Fields,
}
struct MeccCycleServices<'a, 'r, 'c, 'f, 'v> {
    deferred: bool,
    primary: bool,
    prepared_history: bool,
    fire: Option<&'a mut crate::mecc_primary::FireServices<'f, 'r, 'c>>,
    collection: Option<crate::mecc_collection::Services<'c>>,
    countdowns: bool,
    periodic: bool,
    wield: Option<&'a mut crate::mecc_wield::Factory<'r>>,
    jet: Option<&'a mut dyn crate::mecc_jet_owned::PhaseDriver>,
    impact: Option<crate::mecc_impact::Context>,
    impact_voice: Option<&'v mut dyn MeccImpactVoice>,
    damage_display: Option<crate::mecc_damage_effect::Display>,
}
#[derive(Debug)]
pub struct Session {
    ticket: u64,
    next: u64,
    residents: BTreeMap<u64, Resident>,
    specifications: BTreeMap<u64, OwnedSpecification>,
    state_resources: BTreeMap<u64, Arc<crate::state_resources::Prepared>>,
    declarations: BTreeMap<crate::declared_residents::Key, Id>,
    messages: crate::mission_messages::Queue,
    links: Links,
    position_buffers: crate::actor_position_buffers::Bank,
    common_walk_running: bool,
    common_walk_failure: Option<String>,
    mecc_events: Option<crate::mecc_animation_events::Bank>,
    delphi_events: Option<crate::mecc_animation_events::Bank>,
    kabuto_events: Option<crate::mecc_animation_events::Bank>,
    mecc_contact_materials: Option<Arc<crate::mecc_contact_materials::Catalog>>,
    vimp_events: Option<crate::mecc_animation_events::Bank>,
    mecc_countdown_world: Option<f32>,
    mecc_shot_record: Option<crate::mecc_shot::Record>,
    mecc_reticle_owner: Option<Id>,
    mecc_target_range: Option<(Id, bool)>,
}
impl Session {
    pub fn new(ticket: u64) -> Self {
        Self {
            ticket,
            next: 0,
            residents: BTreeMap::new(),
            specifications: BTreeMap::new(),
            state_resources: BTreeMap::new(),
            declarations: BTreeMap::new(),
            messages: crate::mission_messages::Queue::default(),
            links: Links::default(),
            position_buffers: crate::actor_position_buffers::Bank::default(),
            common_walk_running: false,
            common_walk_failure: None,
            mecc_events: None,
            delphi_events: None,
            kabuto_events: None,
            mecc_contact_materials: None,
            vimp_events: None,
            mecc_countdown_world: None,
            mecc_shot_record: None,
            mecc_reticle_owner: None,
            mecc_target_range: None,
        }
    }
    /// Bind known contact names from the owning mission's prepared colliders.
    pub fn bind_mecc_contact_materials(
        &mut self,
        catalog: crate::mecc_contact_materials::Catalog,
    ) -> Result<(), String> {
        if let Some(old) = &self.mecc_contact_materials {
            if old.as_ref() != &catalog {
                return Err("Mecc contact materials cannot be rebound within a mission".into());
            }
        } else {
            self.mecc_contact_materials = Some(Arc::new(catalog));
        }
        Ok(())
    }
    /// Copy before registration. Preserve unexecuted commands and unresolved
    /// targets; owning these bytes never acknowledges actor or mission readiness.
    pub fn register(
        &mut self,
        class: i32,
        seed: &Seed,
        bindings: &[StateBinding],
        flags: DirectFlags,
    ) -> Result<Id, String> {
        self.register_with_mask(class, seed, bindings, flags, 0x8000_0000)
    }
    fn register_with_mask(
        &mut self,
        class: i32,
        seed: &Seed,
        bindings: &[StateBinding],
        flags: DirectFlags,
        requested: u32,
    ) -> Result<Id, String> {
        if seed.storage.profile() != Profile::Debug {
            return Err("resident mission states require the debug profile".into());
        }
        if bindings.len() != seed.unresolved.len()
            || seed.unresolved.iter().any(|reference| {
                bindings
                    .iter()
                    .filter(|b| b.offset == reference.offset && b.key == reference.key)
                    .count()
                    != 1
            })
        {
            return Err("resident bindings do not match the captured payload references".into());
        }
        let serial = self
            .next
            .checked_add(1)
            .ok_or("resident identity exhausted")?;
        let seed = seed.instantiate()?;
        let constructor_mask = self.links.construct(
            serial,
            class,
            u32::from(seed.storage.state()),
            flags,
            requested,
        )?;
        self.residents.insert(
            serial,
            Resident {
                herd: None,
                herd_marker: None,
                vimp: None,
                vimp_physical: None,
                herd_frame: None,
                vimp_frames: 0,
                vimp_prepared_history: None,
                vimp_frame_stopped: false,
                vimp_chunk: None,
                vimp_hurt_steps: 0,
                vimp_scalar_steps: [0; 2],
                vimp_recovery_primaries: 0,
                vimp_death_proxy: None,
                class,
                seed,
                bindings: bindings.to_vec(),
                constructor_mask,
                initial_flags: flags,
                construction_fields: None,
                state23_combat: None,
                mecc_special: None,
                delphi_special: None,
                delphi_copy_source: None,
                delphi_copy_parameter: None,
                delphi_held_pose: None,
                delphi_physics_present: false,
                delphi_physical: None,
                mecc_special_child: None,
                wield_binding: None,
                wield_failure: None,
                mecc_shot_failure: None,
                mecc_local_target: None,
                mecc_jet: None,
                mecc_contact: Default::default(),
                mecc_damage_smoke: Default::default(),
                mecc_damage: None,
                mecc_physics_present: false,
                mecc_physical_cycle: None,
                affine_cache: None,
                state23_animation_counter: 0,
                state23_animation_failed: false,
                acquired_construction: false,
                kabuto_player_fields: false,
                kabuto_group: None,
                basic_projectile: None,
                enemy_projectile: None,
                attached_effect: None,
                positional_impact: None,
                timed_effect: None,
                environment: None,
                ordinary_motion: None,
                ripper_primary: None,
                ripper_primary_running: false,
                controlled_motion: None,
                finalization: None,
                world_tick: None,
                world_tick_running: false,
                surface_link: None,
                attachment: None,
                retained_attachment_words: None,
                pickup_notice_pending: false,
                pickup_notice_delivery: None,
                channels: None,
                mecc_animation_stamp: 0,
                presentation_parent: None,
                presentation_position: None,
            },
        );
        self.next = serial;
        Ok(Id {
            ticket: self.ticket,
            serial,
        })
    }
    pub fn get(&self, id: Id) -> Option<&Resident> {
        (id.ticket == self.ticket)
            .then(|| self.residents.get(&id.serial))
            .flatten()
    }
    pub fn mecc_special(&self, id: Id) -> Option<crate::mecc_special::Initialized> {
        self.get(id)?.mecc_special
    }
    pub fn mecc_special_child(&self, id: Id) -> Option<crate::mecc_special::Child> {
        let actor = self.get(id)?;
        actor
            .mecc_special_child
            .filter(|child| actor.presentation_parent == Some(child.parent))
    }
    pub(crate) fn mirror_jet_child_channels(&mut self, id: Id) -> Result<(), String> {
        self.mecc_special_child(id)
            .ok_or("jet child binding absent")?;
        let actor = self.residents.get_mut(&id.serial).unwrap();
        let channels = actor.channels.ok_or("jet child channels absent")?;
        actor
            .construction_fields
            .as_mut()
            .ok_or("jet child fields absent")?
            .channels = channels;
        Ok(())
    }
    pub(crate) fn mirror_wield_child_channels(&mut self, id: Id) -> Result<(), String> {
        let binding = self
            .wield_binding(id)
            .ok_or("held channel binding absent")?;
        if self
            .get(id)
            .is_none_or(|actor| actor.class != 21 || actor.seed.storage.state() != 0)
            || !self
                .wield_list()?
                .iter()
                .any(|child| child.id == id && child.parent == Some(binding.parent))
        {
            return Err("held channel owner outside acquired normal pistol profile".into());
        }
        let actor = self.residents.get_mut(&id.serial).unwrap();
        actor
            .construction_fields
            .as_mut()
            .ok_or("held fields absent")?
            .channels = actor.channels.ok_or("held channels absent")?;
        Ok(())
    }
    /// Full cold special-init phase, with actual source child constructions and
    /// explicit non-base local-player identity. No activation bit is invented.
    pub fn initialize_mecc_special(
        &mut self,
        owner: Id,
        cold: &Seed,
        inputs: [Construction<'_>; 3],
        local_player: Option<Id>,
    ) -> Result<crate::mecc_special::Initialized, String> {
        self.initialize_mecc_special_fields(owner, cold, inputs, local_player, None)
    }
    pub(crate) fn initialize_cold_mecc_player_fields(
        &mut self,
        owner: Id,
        cold: &Seed,
        inputs: [Construction<'_>; 3],
        scale: f32,
        blending: crate::animation_channels::Blending,
    ) -> Result<crate::mecc_special::Initialized, String> {
        self.initialize_mecc_special_fields(
            owner,
            cold,
            inputs,
            Some(owner),
            Some((scale, blending)),
        )
    }
    fn initialize_mecc_special_fields(
        &mut self,
        owner: Id,
        cold: &Seed,
        inputs: [Construction<'_>; 3],
        local_player: Option<Id>,
        player: Option<(f32, crate::animation_channels::Blending)>,
    ) -> Result<crate::mecc_special::Initialized, String> {
        let parent = self
            .get(owner)
            .ok_or("Mecc special owner absent or foreign")?;
        if let Some(local) = local_player {
            self.get(local)
                .ok_or("Mecc local player absent or foreign")?;
        }
        let fields = parent
            .construction_fields
            .as_ref()
            .ok_or("Mecc special owner fields absent")?;
        if !parent.acquired_construction
            || parent.class != 2
            || parent.seed.storage.profile() != Profile::Debug
            || parent.seed.storage.state() != 2
            || cold.storage.profile() != Profile::Debug
            || cold.storage.state() != 2
            || cold.storage.data().is_none_or(|d| d.len() != 524)
            || parent.seed.storage.data() != cold.storage.data()
            || parent.mecc_special.is_some()
            || parent.controlled_motion.is_some()
            || parent.attachment.is_some()
            || parent.presentation_parent.is_some()
            || !fields
                .position
                .into_iter()
                .chain(fields.angles)
                .chain([fields.scale])
                .all(f32::is_finite)
        {
            return Err(
                "Mecc special phase requires fresh unparented acquired D class2 state".into(),
            );
        }
        self.specification(owner)
            .ok_or("Mecc special body absent")?;
        self.state_resources(owner)
            .ok_or("Mecc special state resources absent")?;
        let mut fields = fields.clone();
        let mut parent_channels = parent.channels.ok_or("Mecc channels absent")?;
        if let Some((scale, blending)) = player {
            if !scale.is_finite()
                || parent_channels != fields.channels
                || fields.configuration[14] & 1 != 0
                || self
                    .links
                    .memberships(owner.serial)
                    .iter()
                    .map(|(_, l)| *l)
                    .collect::<std::collections::BTreeSet<_>>()
                    != [0, 1, 2, 5].into_iter().collect()
            {
                return Err("cold player fields require unselected source Mecc constructor".into());
            }
            fields.scale = scale;
            // The acquired fresh generic-to-player path retains context zero
            // from the generic shell, rather than the Mecc class template.
            fields.dispatch_context = 0;
            fields.angles[1] = 0.;
            fields.previous = fields.current;
            fields.current = fields.position;
            fields.configuration[14] |= 1;
            crate::animation_channels::select_primary(
                &mut parent_channels,
                crate::animation_channels::Request {
                    slot: 1,
                    flags: 0x21,
                    speed_source: 0,
                    reference_speed: 0.,
                    blend_seconds: f32::from_bits(0x3e2a_aaab),
                },
                blending,
            )?;
        }
        let position = fields.position;
        let angles = fields.angles;
        let dispatch_context = fields.dispatch_context;
        let mut payload = parent.seed.storage.data().unwrap().to_vec();
        for (index, input) in inputs.iter().enumerate() {
            if input.class != [84, 85, 781][index]
                || input.seed.storage.profile() != Profile::Debug
                || input.seed.storage.state() != 0
                || input.seed.storage.data().is_some()
                || !input.seed.diagnostics.is_empty()
            {
                return Err(
                    "Mecc special children require ordered acquired84/85/781 state0 profiles"
                        .into(),
                );
            }
        }
        let mut staged = Self::new(self.ticket);
        staged.next = self.next;
        let mut ids = Vec::new();
        for (index, input) in inputs.into_iter().enumerate() {
            let id = staged.register_with_mask(
                input.class,
                input.seed,
                input.bindings,
                input.flags,
                2,
            )?;
            staged.bind_specification(id, input.specification)?;
            staged.bind_state_resources(id, input.state_resources)?;
            let mut channels = input.fields.channels;
            staged.bind_actor_fields(id, input.fields)?;
            let child = staged.residents.get_mut(&id.serial).unwrap();
            let fields = child.construction_fields.as_mut().unwrap();
            fields.dispatch_context = dispatch_context;
            if index < 2 {
                fields.previous = fields.current;
                fields.current = position;
                fields.position = position;
                fields.angles = angles;
            }
            fields.object_flags |= 0x40400000;
            channels[0][8] |= 1;
            child.channels = Some(channels);
            child.acquired_construction = true;
            child.presentation_parent = Some(owner);
            child.mecc_special_child = Some(crate::mecc_special::Child {
                parent: owner,
                reference_word: if index == 1 { 0 } else { -1 },
                binding_word: 2,
                context: fields.dispatch_context,
            });
            ids.push(id);
        }
        let write = |data: &mut [u8], at: usize, value: u32| {
            data[at..at + 4].copy_from_slice(&value.to_le_bytes())
        };
        for at in [0x60, 0x190, 0x18c, 0x188, 0x184, 0x17c, 0x178, 0x174, 0xf4] {
            write(&mut payload, at, u32::MAX);
        }
        let controlled = local_player == Some(owner);
        let equipment = crate::player_equipment::Initial::non_base(controlled).words();
        let offsets = std::iter::once(0x64).chain((0x14c..0x194).step_by(4));
        for (at, value) in offsets.zip(equipment) {
            write(&mut payload, at, value);
        }
        // Preflight every registration in a private list copy before any
        // resident, payload, identity or list is made public.
        let mut links = self.links.clone();
        for (&serial, actor) in &staged.residents {
            links.construct(
                serial,
                actor.class,
                u32::from(actor.seed.storage.state()),
                actor.initial_flags,
                2,
            )?;
        }
        if player.is_some() {
            for list in [4, 2, 3] {
                links.remove_membership(list, owner.serial);
            }
            links.insert(2, owner.serial, crate::object_links::Position::Head);
        }
        let initialized = crate::mecc_special::Initialized {
            children: ids.try_into().unwrap(),
            controlled,
            player_fields: player.is_some(),
        };
        self.links = links;
        self.next = staged.next;
        self.residents.append(&mut staged.residents);
        self.specifications.append(&mut staged.specifications);
        self.state_resources.append(&mut staged.state_resources);
        let parent = self.residents.get_mut(&owner.serial).unwrap();
        parent
            .seed
            .storage
            .data_mut()
            .unwrap()
            .copy_from_slice(&payload);
        parent.mecc_special = Some(initialized);
        if player.is_some() {
            parent.construction_fields = Some(fields);
            parent.channels = Some(parent_channels);
            // Exact cold generic allocation and complete D type change retain
            // the zero header damage. This is not a health reset for live actors.
            parent.mecc_damage = Some(0.);
        }
        Ok(initialized)
    }
    /// Refresh real owned inline Mecc parameters and current inventory mass.
    /// This precedes motion; it does not advance or complete an actor frame.
    pub fn prepare_mecc_physics(
        &mut self,
        id: Id,
        generation: u32,
        globals: &giants_formats::global_data::GlobalData,
        catalog: &giants_formats::weapons::Catalog,
    ) -> Result<crate::mecc_physics::Physics, String> {
        self.mecc_health(id)?;
        let actor = self.get(id).ok_or("Mecc actor absent or foreign")?;
        if !actor.acquired_construction
            || actor.mecc_physical_cycle.is_some()
            || actor.wield_failure.is_some()
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || actor.controlled_motion.is_some()
        {
            return Err(
                "Mecc physics requires acquired unparented player without external motion".into(),
            );
        }
        let fields = actor.construction_fields.as_ref().unwrap();
        let prepared = crate::mecc_physics::prepare(
            actor.seed.storage.data().unwrap(),
            actor.mecc_physics_present,
            fields.configuration,
            crate::mecc_physics::Refresh {
                generation,
                globals,
                catalog,
                response_flags: fields.response_flags,
                control_flags: fields.control_flags,
            },
        )?;
        let physics = prepared.physics;
        let actor = self.residents.get_mut(&id.serial).unwrap();
        actor
            .seed
            .storage
            .data_mut()
            .unwrap()
            .copy_from_slice(&prepared.payload);
        actor.construction_fields.as_mut().unwrap().configuration = prepared.configuration;
        actor.mecc_physics_present = true;
        Ok(physics)
    }
    pub fn mecc_physics(&self, id: Id) -> Result<crate::mecc_physics::Physics, String> {
        self.mecc_health(id)?;
        let actor = self.get(id).unwrap();
        if !actor.mecc_physics_present {
            return Err("owned Mecc physics not initialized".into());
        }
        crate::mecc_physics::Physics::from_payload(actor.seed.storage.data().unwrap())
    }
    pub fn prepare_mecc_physical_cycle(
        &mut self,
        id: Id,
        resources: crate::mecc_physical_cycle::Resources,
    ) -> Result<(), String> {
        let health = self.mecc_health(id)?;
        self.mecc_physics(id)?;
        let actor = self.get(id).unwrap();
        if health.damage != 0.
            || !actor.acquired_construction
            || actor.wield_failure.is_some()
            || actor.mecc_physical_cycle.is_some()
            || actor.controlled_motion.is_some()
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || !resources.belongs_to(
                self.specification(id)
                    .ok_or("physical Mecc specification absent")?,
            )
        {
            return Err("physical cycle requires a fresh source-owned unparented player".into());
        }
        let cycle = crate::mecc_physical_cycle::Owned::fresh(
            resources,
            actor.construction_fields.as_ref().unwrap(),
            actor.seed.storage.data().unwrap(),
        )?;
        self.residents
            .get_mut(&id.serial)
            .unwrap()
            .mecc_physical_cycle = Some(cycle);
        Ok(())
    }
    /// Advance only the acquired physical subcycle on owned state. ANM,
    /// secondary, jet and the complete primary remain required external phases;
    /// this operation neither marks40 nor enters the world-loop success path.
    pub fn advance_mecc_physical_cycle(
        &mut self,
        id: Id,
        environment: crate::mecc_physical_cycle::Environment<'_>,
        ground: impl FnMut([f32; 2]) -> Result<crate::physical_ground::Ground, &'static str>,
    ) -> Result<crate::mecc_physical_cycle::View, String> {
        let health = self.mecc_health(id)?;
        let actor = self.get(id).unwrap();
        if health.damage != 0. || actor.attachment.is_some() || actor.presentation_parent.is_some()
        {
            return Err("physical cycle requires acquired undamaged unparented state2".into());
        }
        let mut cycle = actor
            .mecc_physical_cycle
            .clone()
            .ok_or("Mecc physical cycle not prepared")?;
        if cycle.interleaved || cycle.world_history.is_some() {
            return Err("interleaved Mecc cannot use the separate physical projection".into());
        }
        if !cycle.belongs_to(self.specification(id).ok_or("Mecc specification absent")?) {
            return Err("Mecc physical resources are stale after specification rebind".into());
        }
        let result = cycle.advance(
            actor.construction_fields.as_ref().unwrap(),
            actor.seed.storage.data().unwrap(),
            environment,
            ground,
        );
        let view = cycle.view.clone();
        let actor = self.residents.get_mut(&id.serial).unwrap();
        actor.mecc_physical_cycle = Some(cycle);
        let output = result?;
        if let Some(value) = view.affine {
            actor.affine_cache = Some(OwnedAffine {
                value,
                pose: affine_pose_words(&output.fields),
                external_orientation: false,
            });
        }
        actor.construction_fields = Some(output.fields);
        actor
            .seed
            .storage
            .data_mut()
            .unwrap()
            .copy_from_slice(&output.payload);
        Ok(view)
    }
    pub fn mecc_physical_cycle(&self, id: Id) -> Result<&crate::mecc_physical_cycle::View, String> {
        let actor = self.get(id).ok_or("Mecc cycle actor absent or foreign")?;
        let cycle = actor
            .mecc_physical_cycle
            .as_ref()
            .ok_or("Mecc cycle absent")?;
        if cycle.failure.is_some()
            || !cycle.belongs_to(self.specification(id).ok_or("Mecc specification absent")?)
        {
            return Err("Mecc physical cycle failed".into());
        }
        Ok(&cycle.view)
    }
    pub fn mecc_physical_cycle_failure(&self, id: Id) -> Option<&str> {
        self.get(id)?
            .mecc_physical_cycle
            .as_ref()?
            .failure
            .as_deref()
    }
    /// The first real ANM tail runs before gravity/control, over the actual
    /// working physical state. The normal selection/second tail remain a
    /// projection pending the jet and complete secondary/body services.
    pub fn advance_mecc_interleaved_cycle(
        &mut self,
        id: Id,
        environment: crate::mecc_physical_cycle::Environment<'_>,
        ground: impl FnMut([f32; 2]) -> Result<crate::physical_ground::Ground, &'static str>,
        animation: crate::mecc_animation_phase::Services<'_>,
    ) -> Result<crate::mecc_physical_cycle::InterleavedProjection, String> {
        self.advance_mecc_interleaved_inner(
            id,
            environment,
            ground,
            animation,
            MeccCycleServices {
                impact: None,
                impact_voice: None,
                damage_display: None,
                deferred: false,
                fire: None,
                primary: false,
                prepared_history: false,
                collection: None,
                countdowns: false,
                periodic: false,
                wield: None,
                jet: None,
            },
        )
    }
    /// Actual ordinary secondary writes and real owned hand-object services.
    /// Jet, remaining primary writes and non-ordinary providers stay required.
    /// Once hand services start, failure retains completed phase effects and
    /// quarantines the frame; clear the whole owner world before retrying.
    pub fn advance_mecc_secondary_cycle(
        &mut self,
        id: Id,
        environment: crate::mecc_physical_cycle::Environment<'_>,
        ground: impl FnMut([f32; 2]) -> Result<crate::physical_ground::Ground, &'static str>,
        animation: crate::mecc_animation_phase::Services<'_>,
        wield: &mut crate::mecc_wield::Factory<'_>,
    ) -> Result<crate::mecc_physical_cycle::InterleavedProjection, String> {
        self.advance_mecc_interleaved_inner(
            id,
            environment,
            ground,
            animation,
            MeccCycleServices {
                impact: None,
                impact_voice: None,
                damage_display: None,
                deferred: false,
                fire: None,
                primary: false,
                prepared_history: false,
                collection: None,
                countdowns: false,
                periodic: false,
                wield: Some(wield),
                jet: None,
            },
        )
    }
    /// Normal jet phase with real owned children and queued sound sequences.
    /// Global audio, depletion and remaining primary providers stay required.
    pub fn advance_mecc_jet_cycle(
        &mut self,
        id: Id,
        environment: crate::mecc_physical_cycle::Environment<'_>,
        ground: impl FnMut([f32; 2]) -> Result<crate::physical_ground::Ground, &'static str>,
        animation: crate::mecc_animation_phase::Services<'_>,
        services: crate::mecc_jet_owned::Services<'_, '_>,
    ) -> Result<crate::mecc_physical_cycle::InterleavedProjection, String> {
        let crate::mecc_jet_owned::Services {
            wield,
            sequences,
            backend,
            input,
        } = services;
        let mut jet = crate::mecc_jet_owned::Driver {
            fire: false,
            sequences,
            backend,
            input,
        };
        self.advance_mecc_interleaved_inner(
            id,
            environment,
            ground,
            animation,
            MeccCycleServices {
                impact: None,
                impact_voice: None,
                damage_display: None,
                deferred: false,
                fire: None,
                primary: false,
                prepared_history: false,
                collection: None,
                countdowns: false,
                periodic: false,
                wield: Some(wield),
                jet: Some(&mut jet),
            },
        )
    }
    /// Initialize the explicitly supplied shared countdown once per owned world.
    /// Actor clear does not reset this process-like state; a new Session does.
    pub fn prepare_mecc_countdown_world(&mut self, timer: f32) -> Result<(), String> {
        if !timer.is_finite() || self.mecc_countdown_world.is_some() {
            return Err("shared Mecc countdown is invalid or already initialized".into());
        }
        self.mecc_countdown_world = Some(timer);
        Ok(())
    }
    pub fn mecc_countdown_world(&self) -> Option<f32> {
        self.mecc_countdown_world
    }
    /// Acquired paired countdown prefix followed by the owned jet/ANM cycle.
    /// This is not the remaining primary suffix, global sound update or Ready.
    pub fn advance_mecc_countdown_cycle(
        &mut self,
        id: Id,
        environment: crate::mecc_physical_cycle::Environment<'_>,
        ground: impl FnMut([f32; 2]) -> Result<crate::physical_ground::Ground, &'static str>,
        animation: crate::mecc_animation_phase::Services<'_>,
        services: crate::mecc_jet_owned::Services<'_, '_>,
    ) -> Result<crate::mecc_physical_cycle::InterleavedProjection, String> {
        let crate::mecc_jet_owned::Services {
            wield,
            sequences,
            backend,
            input,
        } = services;
        let mut jet = crate::mecc_jet_owned::Driver {
            fire: false,
            sequences,
            backend,
            input,
        };
        self.advance_mecc_interleaved_inner(
            id,
            environment,
            ground,
            animation,
            MeccCycleServices {
                impact: None,
                impact_voice: None,
                damage_display: None,
                deferred: false,
                fire: None,
                primary: false,
                prepared_history: false,
                collection: None,
                countdowns: true,
                periodic: false,
                wield: Some(wield),
                jet: Some(&mut jet),
            },
        )
    }
    /// Acquired periodic scan in the owned cycle. Eligible collection requests
    /// require their actual provider; unsupported requests fail and quarantine.
    pub fn advance_mecc_periodic_cycle(
        &mut self,
        id: Id,
        environment: crate::mecc_physical_cycle::Environment<'_>,
        ground: impl FnMut([f32; 2]) -> Result<crate::physical_ground::Ground, &'static str>,
        animation: crate::mecc_animation_phase::Services<'_>,
        services: crate::mecc_jet_owned::Services<'_, '_>,
    ) -> Result<crate::mecc_physical_cycle::InterleavedProjection, String> {
        let crate::mecc_jet_owned::Services {
            wield,
            sequences,
            backend,
            input,
        } = services;
        let mut jet = crate::mecc_jet_owned::Driver {
            fire: false,
            sequences,
            backend,
            input,
        };
        self.advance_mecc_interleaved_inner(
            id,
            environment,
            ground,
            animation,
            MeccCycleServices {
                impact: None,
                impact_voice: None,
                damage_display: None,
                deferred: false,
                fire: None,
                primary: false,
                prepared_history: false,
                collection: None,
                countdowns: true,
                periodic: true,
                wield: Some(wield),
                jet: Some(&mut jet),
            },
        )
    }
    /// Composed offline collection at the real periodic phase. The caller owns
    /// the authoritative world/mission/audio providers; this is not mission Ready.
    pub fn advance_mecc_collection_cycle(
        &mut self,
        id: Id,
        environment: crate::mecc_physical_cycle::Environment<'_>,
        ground: impl FnMut([f32; 2]) -> Result<crate::physical_ground::Ground, &'static str>,
        animation: crate::mecc_animation_phase::Services<'_>,
        services: crate::mecc_collection::CycleServices<'_, '_, '_>,
    ) -> Result<crate::mecc_physical_cycle::InterleavedProjection, String> {
        let crate::mecc_jet_owned::Services {
            wield,
            sequences,
            backend,
            input,
        } = services.jet;
        let mut jet = crate::mecc_jet_owned::Driver {
            fire: false,
            sequences,
            backend,
            input,
        };
        self.advance_mecc_interleaved_inner(
            id,
            environment,
            ground,
            animation,
            MeccCycleServices {
                impact: None,
                impact_voice: None,
                damage_display: None,
                deferred: false,
                fire: None,
                primary: false,
                prepared_history: false,
                countdowns: true,
                periodic: true,
                collection: Some(services.collection),
                wield: Some(wield),
                jet: Some(&mut jet),
            },
        )
    }
    /// Separate common position-history pass. Call this for the declared Mecc
    /// population before entering its world callbacks; it is not admission or
    /// certification of the surrounding world passes.
    pub fn begin_mecc_frame_history(&mut self, id: Id, number: u64) -> Result<(), String> {
        self.mecc_physical_cycle(id)?;
        let actor = self.get(id).unwrap();
        let cycle = actor.mecc_physical_cycle.as_ref().unwrap();
        if actor.world_tick_running
            || actor
                .world_tick
                .as_ref()
                .is_some_and(|t| t.phase() != crate::actor_world_tick::Phase::Finished)
            || cycle.world_stopped
            || cycle.subjective_pending.is_some()
            || cycle.view.number.checked_add(1) != Some(number)
            || cycle.world_history.is_some_and(|n| n != cycle.view.number)
            || cycle.view.number != 0 && cycle.world_history.is_none()
        {
            return Err("Mecc world history out of order, pending, failed or stopped".into());
        }
        let actor = self.residents.get_mut(&id.serial).unwrap();
        let fields = actor
            .construction_fields
            .as_mut()
            .ok_or("Mecc world fields absent")?;
        fields.previous = fields.current;
        fields.current = fields.position;
        actor.mecc_physical_cycle.as_mut().unwrap().world_history = Some(number);
        actor.world_tick = None;
        Ok(())
    }
    /// One ticket-owned iteration after separately prepared history. The real
    /// primary services own physics/ANM; the common policy controls admission,
    /// controller presence and cleanup. Failed or stopped iterations cannot
    /// replay through a new caller token or switch to a standalone subcycle.
    pub fn advance_mecc_world_offline(
        &mut self,
        id: Id,
        environment: crate::mecc_physical_cycle::Environment<'_>,
        controlled_actor: Option<Id>,
        ground: impl FnMut([f32; 2]) -> Result<crate::physical_ground::Ground, &'static str>,
        services: crate::mecc_primary::Services<'_, '_, '_>,
    ) -> Result<crate::mecc_world::Projection, String> {
        self.advance_mecc_world_offline_route(
            id,
            environment,
            controlled_actor,
            ground,
            services,
            false,
        )
    }
    fn advance_mecc_world_offline_route(
        &mut self,
        id: Id,
        environment: crate::mecc_physical_cycle::Environment<'_>,
        controlled_actor: Option<Id>,
        mut ground: impl FnMut([f32; 2]) -> Result<crate::physical_ground::Ground, &'static str>,
        services: crate::mecc_primary::Services<'_, '_, '_>,
        deferred: bool,
    ) -> Result<crate::mecc_world::Projection, String> {
        self.mecc_physical_cycle(id)?;
        let actor = self.get(id).unwrap();
        let cycle = actor.mecc_physical_cycle.as_ref().unwrap();
        if actor.world_tick_running
            || actor.world_tick.is_some()
            || cycle.world_stopped
            || cycle.subjective_pending.is_some()
            || cycle.world_history != Some(environment.number)
            || cycle.view.number.checked_add(1) != Some(environment.number)
        {
            return Err("Mecc world iteration requires fresh separate owned history".into());
        }
        self.residents
            .get_mut(&id.serial)
            .unwrap()
            .world_tick_running = true;
        let mut token = crate::actor_world_tick::Tick::new();
        let mut adapter = crate::mecc_world::Adapter {
            deferred,
            environment: Some(environment),
            services: Some(services),
            ground: &mut ground,
            controlled: controlled_actor,
            primary: None,
        };
        let result = token.run(&mut ActorWorldAdapter {
            session: self,
            source: id,
            services: &mut adapter,
        });
        let actor = self
            .residents
            .get_mut(&id.serial)
            .ok_or("Mecc world owner removed during callbacks")?;
        actor.world_tick_running = false;
        actor.world_tick = Some(token);
        if let Ok(outcome) = result {
            actor.mecc_physical_cycle.as_mut().unwrap().world_stopped =
                outcome != crate::actor_world_tick::Outcome::UpdatedByPrimary;
        }
        result.map(|outcome| crate::mecc_world::Projection {
            outcome,
            primary: adapter.primary,
        })
    }
    /// Complete acquired normal offline primary composition. Requires real
    /// hand/jet/event services and actual collection services if a scan selects
    /// an item. Other equipment, reactions, active auxiliary flags and general
    /// actor admission remain outside this profile. Marks40 at the real first
    /// ANM boundary; failure quarantines the frame with executed flags retained.
    pub fn advance_mecc_normal_primary(
        &mut self,
        id: Id,
        environment: crate::mecc_physical_cycle::Environment<'_>,
        ground: impl FnMut([f32; 2]) -> Result<crate::physical_ground::Ground, &'static str>,
        services: crate::mecc_primary::Services<'_, '_, '_>,
    ) -> Result<crate::mecc_physical_cycle::InterleavedProjection, String> {
        self.advance_mecc_primary_with_history(id, environment, ground, services, false)
    }
    pub(crate) fn advance_mecc_primary_with_history(
        &mut self,
        id: Id,
        environment: crate::mecc_physical_cycle::Environment<'_>,
        ground: impl FnMut([f32; 2]) -> Result<crate::physical_ground::Ground, &'static str>,
        services: crate::mecc_primary::Services<'_, '_, '_>,
        prepared_history: bool,
    ) -> Result<crate::mecc_physical_cycle::InterleavedProjection, String> {
        self.advance_mecc_primary_with_history_route(
            id,
            environment,
            ground,
            services,
            prepared_history,
            false,
        )
    }
    pub(crate) fn advance_mecc_primary_with_history_route(
        &mut self,
        id: Id,
        environment: crate::mecc_physical_cycle::Environment<'_>,
        ground: impl FnMut([f32; 2]) -> Result<crate::physical_ground::Ground, &'static str>,
        services: crate::mecc_primary::Services<'_, '_, '_>,
        prepared_history: bool,
        deferred: bool,
    ) -> Result<crate::mecc_physical_cycle::InterleavedProjection, String> {
        let crate::mecc_primary::Services {
            impact,
            impact_voice,
            damage_display,
            animation,
            jet,
            collection,
            mut fire,
        } = services;
        let crate::mecc_jet_owned::Services {
            wield,
            sequences,
            backend,
            input,
        } = jet;
        let mut driver = crate::mecc_jet_owned::Driver {
            fire: fire.is_some() || deferred,
            sequences,
            backend,
            input,
        };
        self.advance_mecc_interleaved_inner(
            id,
            environment,
            ground,
            animation,
            MeccCycleServices {
                impact,
                impact_voice,
                damage_display,
                deferred,
                fire: fire.as_mut(),
                primary: true,
                prepared_history,
                collection,
                countdowns: true,
                periodic: true,
                wield: Some(wield),
                jet: Some(&mut driver),
            },
        )
    }
    fn advance_mecc_interleaved_inner(
        &mut self,
        id: Id,
        environment: crate::mecc_physical_cycle::Environment<'_>,
        ground: impl FnMut([f32; 2]) -> Result<crate::physical_ground::Ground, &'static str>,
        animation: crate::mecc_animation_phase::Services<'_>,
        services: MeccCycleServices<'_, '_, '_, '_, '_>,
    ) -> Result<crate::mecc_physical_cycle::InterleavedProjection, String> {
        let crate::mecc_animation_phase::Services {
            contexts,
            random,
            stamp,
            blending,
        } = animation;
        let MeccCycleServices {
            impact,
            mut impact_voice,
            damage_display,
            deferred,
            primary,
            prepared_history,
            mut fire,
            mut collection,
            countdowns,
            periodic,
            mut wield,
            mut jet,
        } = services;
        if impact_voice.is_some() && impact.is_none() {
            return Err("impact voice requires its explicit damage/authority context".into());
        }
        if primary && environment.orientation.is_none() {
            return Err("normal Mecc primary requires its orientation input provider".into());
        }
        let fire_mode = fire.is_some() || deferred;
        if deferred && (!primary || fire.is_some()) {
            return Err("subjective primary cannot also execute immediate fire".into());
        }
        if let Some(service) = &fire {
            service.preflight(id, environment.number)?;
        }
        let collection_mode = collection.is_some();
        if let Some(c) = &collection
            && (c.world.failure.is_some()
                || self.get(c.world.scope).is_none()
                || !c.offline
                || !c.now.is_finite()
                || c.audio_key == 0)
        {
            return Err("collection cycle requires current unfailed offline services".into());
        }
        let jet_mode = jet.is_some();
        if let Some(driver) = &jet {
            driver.preflight(self, id)?;
        }
        let secondary_mode = wield.is_some();
        if let Some(factory) = &mut wield {
            factory.preflight_weapon(self, id, environment.catalog)?;
        }
        if let Some(display) = &damage_display {
            display.validate(self, id, environment.number)?;
        }
        let health = self.mecc_health(id)?;
        self.mecc_physical_cycle(id)?;
        let actor = self.get(id).unwrap();
        let mut fields = actor.construction_fields.as_ref().unwrap().clone();
        let body = actor.seed.storage.data().unwrap();
        let word = |at| u32::from_le_bytes(body[at..at + 4].try_into().unwrap());
        let mut cycle = actor.mecc_physical_cycle.clone().unwrap();
        if (health.damage != 0.
            && !(primary
                && damage_display.is_some()
                && health.damage >= 0.
                && health.damage < health.limit))
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || fields.action != 0
            || fields.control_flags
                & !(if secondary_mode {
                    crate::mecc_secondary::control_mask(fire_mode)
                } else {
                    0
                })
                != 0
            || fields.response_flags & !(0x80803000 | if fire_mode { 0x08000020 } else { 0 }) != 0
            || [word(0x180), word(0x184), word(0x188)] != [1, u32::MAX, u32::MAX]
            || (!cycle.interleaved && cycle.view.number != 0)
            || cycle.view.number != 0 && cycle.secondary != secondary_mode
            || cycle.view.number != 0 && cycle.jet != jet_mode
            || cycle.view.number != 0 && cycle.countdowns != countdowns
            || cycle.view.number != 0 && cycle.periodic != periodic
            || cycle.view.number != 0 && cycle.fire != fire_mode
            || cycle.view.number != 0 && cycle.primary != primary
            || cycle.view.number != 0 && cycle.orientation != environment.orientation.is_some()
            || cycle.world_stopped
            || cycle.subjective_pending.is_some()
            || prepared_history != cycle.world_history.is_some()
            || prepared_history && (!primary || cycle.world_history != Some(environment.number))
            || cycle.view.number != 0 && cycle.collection != collection_mode
            || cycle.view.number.checked_add(1) != Some(environment.number)
        {
            return Err("interleaved Mecc requires fresh normal unparented source profile".into());
        }
        let effect_reset = crate::mecc_auxiliary::reset_unheld(
            word(0x11c) != 0,
            environment
                .globals
                .scalar(0x8f4)
                .map_err(|e| e.to_string())?,
        )?;
        crate::mecc_auxiliary::advance_timers(
            word(0xd8),
            [f32::from_bits(word(0x194)), f32::from_bits(word(0x198))],
            0.,
        )?;
        cycle.primary = primary;
        cycle.fire = fire_mode;
        cycle.orientation = environment.orientation.is_some();
        cycle.secondary = secondary_mode;
        cycle.jet = jet_mode;
        cycle.countdowns = countdowns;
        cycle.periodic = periodic;
        cycle.collection = collection_mode;
        cycle.jet_buttons = jet.as_deref().map_or(0, |driver| driver.buttons());
        let mut commands = [0u8; 8];
        if let Some(input) = environment.orientation {
            commands[0] = input.controls[0] as u8;
            commands[1] = input.controls[1] as u8;
        }
        commands[2] = environment.controls[0] as u8;
        commands[3] = environment.controls[1] as u8;
        commands[4..8].copy_from_slice(&cycle.jet_buttons.to_le_bytes());
        let rates = [
            cycle.view.model_rates[0],
            cycle.view.model_rates[1],
            cycle.view.model_rates[2],
            cycle.view.angular_rate,
        ];
        let entry = crate::actor_primary_gate::enter(
            fields.object_flags,
            fields.configuration[14],
            commands,
            rates.map(f32::to_bits),
            cycle.view.previous_model_rates.map(f32::to_bits),
        );
        if !entry.invoke {
            return Err("Mecc primary wrapper does not dispatch this inactive/idle profile".into());
        }
        fields.configuration[14] = entry.binding_flags;
        cycle.view.previous_model_rates = entry.previous_rates.map(f32::from_bits);
        cycle.view.commands = commands;
        let mut starting_body = body.to_vec();
        let countdown = if countdowns {
            let result = crate::mecc_countdowns::advance(
                self.mecc_countdown_world
                    .ok_or("shared Mecc countdown not initialized")?,
                f32::from_bits(word(0x1fc)),
                environment.global_seconds,
            )?;
            starting_body[0x1fc..0x200].copy_from_slice(&result.actor.to_le_bytes());
            Some(result)
        } else {
            None
        };
        if secondary_mode {
            // Acquired47c892..47c8a7, before motion and the first ANM queue.
            let seconds = crate::object_time::seconds(
                environment.global_seconds,
                Some(crate::object_time::Sample {
                    owner: 0,
                    position: fields.position,
                }),
                None,
                environment.areas,
            );
            let timer = f32::from_bits(word(0xd4));
            if !timer.is_finite() || !seconds.is_finite() {
                return Err("ordinary animation prefix timer outside finite profile".into());
            }
            starting_body[0xd4..0xd8].copy_from_slice(
                &((f64::from(timer) + seconds) as f32)
                    .to_bits()
                    .to_le_bytes(),
            );
        }
        let specification = self.specification(id).unwrap().clone();
        let bank = self
            .mecc_events
            .clone()
            .unwrap_or_else(|| crate::mecc_animation_events::Bank::new(specification.clone()));
        if !bank.belongs_to(&specification) {
            return Err("interleaved Mecc event class resources differ".into());
        }
        let mut pipeline = crate::mecc_animation_phase::Pipeline {
            specification,
            references: self.specifications.get(&id.serial).unwrap().0.clone(),
            bank,
            contexts: contexts.staged_copy(),
            random: *random,
            channels: actor.channels.ok_or("interleaved Mecc channels absent")?,
            owner: id,
            stamp,
            last_stamp: actor.mecc_animation_stamp,
            blending,
            materials: self.mecc_contact_materials.clone(),
        };
        let globals = environment.globals;
        let environment_controls = environment.controls;
        let catalog = environment.catalog;
        let global_seconds = environment.global_seconds;
        let areas = environment.areas;
        let mut first = None;
        let mut primary_mark = None;
        let mut first_physics = [0; 15];
        if let Some(countdown) = countdown {
            // These real prefix effects precede physical/world callbacks. Once
            // published they survive later errors; the failed cycle cannot replay.
            self.mecc_countdown_world = Some(countdown.world);
            self.residents
                .get_mut(&id.serial)
                .unwrap()
                .seed
                .storage
                .data_mut()
                .unwrap()
                .copy_from_slice(&starting_body);
        }
        let result = cycle.advance_with_tail(
            &fields,
            &starting_body,
            environment,
            ground,
            |fields, body, view| {
                if first.is_some() {
                    return Err("first ANM boundary repeated".into());
                }
                for (index, value) in first_physics.iter_mut().enumerate() {
                    *value = u32::from_le_bytes(
                        body[0x84 + index * 4..0x88 + index * 4].try_into().unwrap(),
                    );
                }
                first = Some(pipeline.advance(fields, view, view.local_seconds)?);
                if primary {
                    // Native47d57f: after the actual first ANM call returns.
                    fields.object_flags |= 0x40;
                    primary_mark = Some(fields.object_flags);
                    self.residents
                        .get_mut(&id.serial)
                        .unwrap()
                        .construction_fields
                        .as_mut()
                        .unwrap()
                        .object_flags = fields.object_flags;
                }
                Ok(())
            },
        );
        let mut secondary_projection = None;
        let mut jet_projection = None;
        let mut fire_projection = None;
        let completed = result.and_then(|mut output| {
            let first = first.ok_or("first ANM boundary absent")?;
            if let Some(service) = &mut fire {
                // Physical forces and the first ANM precede firing. Publish
                // their real channels/references once; never evaluate ANM again
                // to manufacture a muzzle sample at this boundary.
                output.fields.channels = pipeline.channels;
                let actor = self.residents.get_mut(&id.serial).unwrap();
                actor.channels = Some(pipeline.channels);
                actor.mecc_animation_stamp = pipeline.last_stamp;
                actor.construction_fields = Some(output.fields.clone());
                actor.seed.storage.data_mut().unwrap().copy_from_slice(&output.payload);
                actor.affine_cache = Some(OwnedAffine {
                    value: cycle.view.affine.ok_or("fire physical affine absent")?,
                    pose: affine_pose_words(&output.fields), external_orientation: false,
                });
                self.specifications.get_mut(&id.serial).unwrap().0 = pipeline.references.clone();
                self.mecc_events = Some(pipeline.bank.clone());
                *contexts = pipeline.contexts.staged_copy();
                *random = pipeline.random;
                let seconds = crate::object_time::seconds(global_seconds,
                    Some(crate::object_time::Sample {owner: output.fields.dispatch_context, position: output.fields.position}), None, areas);
                let result = service.run(self,
                    crate::mecc_primary::FireFrame {
                        frame: crate::mecc_fire_primary::Frame {fields: output.fields.clone(), payload: output.payload.clone()},
                        view: &cycle.view, buttons: cycle.jet_buttons, seconds,
                        auxiliary_reset: globals.scalar(0x8f4).map_err(|e|e.to_string())?,
                    },
                    &mut pipeline.contexts, &mut pipeline.random);
                // Constructor effects/RNG survive a late sound/provider error.
                *contexts = pipeline.contexts.staged_copy();
                *random = pipeline.random;
                fire_projection = Some(result?);
                let actor = self.get(id).ok_or("fire removed its primary source")?;
                output.fields = actor.construction_fields.as_ref().unwrap().clone();
                output.payload = actor.seed.storage.data().unwrap().to_vec();
            } else if !deferred {
                for (at, value) in [0x114,0x118].into_iter().zip(effect_reset) {
                    output.payload[at..at+4].copy_from_slice(&value.to_le_bytes());
                }
            }
            if let Some(driver) = &mut jet {
                let phase = driver.run(
                    self,
                    id,
                    crate::mecc_jet_owned::Frame {
                        fields: &output.fields,
                        payload: &output.payload,
                        view: &cycle.view,
                        globals,
                        seconds: crate::object_time::seconds(
                            global_seconds,
                            Some(crate::object_time::Sample {
                                owner: 0,
                                position: output.fields.position,
                            }),
                            None,
                            areas,
                        ),
                        controls: environment_controls,
                    },
                )?;
                output.fields = phase.fields;
                output.payload = phase.payload;
                jet_projection = Some(phase.projection);
            }
            if primary {
                let health = self.mecc_health(id)?;
                let mut smoke = self.get(id).unwrap().mecc_damage_smoke.clone();
                let word = |at| u32::from_le_bytes(output.payload[at..at+4].try_into().unwrap());
                if word(0x1a0) != u32::from(smoke.binding.is_some()) || word(0x1a4) != smoke.level {
                    return Err("damage smoke payload differs from its persistent owner".into());
                }
                if let Some(display) = damage_display {
                    let result = smoke.advance(
                        crate::mecc_damage_effect::Input::from_globals(health.damage, health.limit, globals)?,
                        &mut crate::mecc_damage_effect::Prepared { reset: display.display.owns(&id),
                            pool: &mut pipeline.contexts, source: id, source_flags: output.fields.object_flags,
                            random: &mut pipeline.random },
                    );
                    output.payload[0x1a0..0x1a4].copy_from_slice(&u32::from(smoke.binding.is_some()).to_le_bytes());
                    output.payload[0x1a4..0x1a8].copy_from_slice(&smoke.level.to_le_bytes());
                    let actor = self.residents.get_mut(&id.serial).unwrap();
                    actor.mecc_damage_smoke = smoke;
                    actor.mecc_animation_stamp = pipeline.last_stamp;
                    actor.channels = Some(pipeline.channels);
                    actor.construction_fields = Some(output.fields.clone());
                    actor.seed.storage.data_mut().unwrap().copy_from_slice(&output.payload);
                    self.specifications.get_mut(&id.serial).unwrap().0 = pipeline.references.clone();
                    self.mecc_events = Some(pipeline.bank.clone());
                    *contexts = pipeline.contexts.staged_copy();
                    *random = pipeline.random;
                    result?;
                } else if health.damage != 0. || smoke.binding.is_some() || smoke.level != 0 {
                    return Err("wounded primary requires its actual damage-smoke display provider".into());
                }
            }
            if periodic {
                let timer = f32::from_le_bytes(output.payload[0x1d4..0x1d8].try_into().unwrap());
                let tick = crate::mecc_periodic_scan::plan::<Id>(timer, global_seconds, output.fields.position, &[])?;
                if tick.scanned {
                    let candidates = self.list(0).into_iter().filter(|id| self.get(*id)
                        .is_some_and(|a| (0x45d..=0x45f).contains(&a.class)))
                        .map(|id| {
                            let actor = self.get(id).ok_or("periodic candidate absent")?;
                            let f = actor.construction_fields.as_ref().ok_or("periodic candidate fields absent")?;
                            Ok(crate::mecc_periodic_scan::Candidate { id, class: actor.class,
                                flags: f.object_flags, control: f.control_flags, position: f.position })
                        }).collect::<Result<Vec<_>, String>>()?;
                    let plan = crate::mecc_periodic_scan::plan(timer, global_seconds, output.fields.position, &candidates)?;
                    if !plan.selected.is_empty() {
                        let c = collection.as_mut().ok_or("periodic collection requires actual resource/mission/audio/UI providers")?;
                        // The native resets the timer before invoking any collection.
                        output.payload[0x1d4..0x1d8].copy_from_slice(&tick.timer.to_le_bytes());
                        let actor = self.residents.get_mut(&id.serial).unwrap();
                        actor.mecc_animation_stamp = pipeline.last_stamp;
                actor.channels = Some(pipeline.channels);
                        actor.construction_fields = Some(output.fields.clone());
                        actor.seed.storage.data_mut().unwrap().copy_from_slice(&output.payload);
                        self.specifications.get_mut(&id.serial).unwrap().0 = pipeline.references.clone();
                        self.mecc_events = Some(pipeline.bank.clone());
                        *contexts = pipeline.contexts.staged_copy();
                        *random = pipeline.random;
                        for item in plan.selected {
                            let result = self.collect_vimp_chunk(id, item,
                                crate::mecc_collection::Services {
                                    world: c.world, progress: c.progress, context: c.context,
                                    catalog: c.catalog, current_player: c.current_player,
                                    now: c.now, offline: c.offline, audio_key: c.audio_key,
                                    audio: c.audio,
                                }, &mut pipeline.random);
                            // Even a late audio error has already consumed real RNG.
                            *random = pipeline.random;
                            result?;
                        }
                        let actor = self.get(id).unwrap();
                        output.fields = actor.construction_fields.as_ref().unwrap().clone();
                        output.payload = actor.seed.storage.data().unwrap().to_vec();
                    }
                }
                output.payload[0x1d4..0x1d8].copy_from_slice(&tick.timer.to_le_bytes());
            }
            let selected = if let Some(factory) = &mut wield {
                let mut contact = self.get(id).unwrap().mecc_contact.clone();
                let mut contact_checkpoint = None;
                let stage = {
                    let mut damage = impact.map(|context| mecc_impact::Damage { session: self, source: id, context, voice: impact_voice.take() });
                    crate::mecc_secondary::ordinary(
                    &output.fields,
                    &output.payload,
                    pipeline.channels,
                    &cycle.view,
                    globals,
                    crate::object_time::seconds(
                        global_seconds,
                        Some(crate::object_time::Sample {
                            owner: 0,
                            position: output.fields.position,
                        }),
                        None,
                        areas,
                    ),
                    crate::mecc_secondary::Services {
                        impact: damage.as_mut().map(|d| d as &mut dyn crate::mecc_secondary::ImpactDamage),
                        fire: fire_mode,
                        contact: Some(crate::mecc_secondary::ContactServices {
                            state: &mut contact, pool: &mut pipeline.contexts, source: id,
                            checkpoint: &mut contact_checkpoint,
                        }),
                        blending: pipeline.blending,
                        random: &mut pipeline.random,
                    },
                )
                };
                // Preserve effect ownership and earlier selector writes even if
                // the later channel setter fails. The failed cycle cannot replay.
                self.residents.get_mut(&id.serial).unwrap().mecc_contact = contact;
                if let Some((fields, payload)) = contact_checkpoint {
                    let actor = self.residents.get_mut(&id.serial).unwrap();
                    actor.channels = Some(fields.channels);
                    actor.construction_fields = Some(fields);
                    actor.seed.storage.data_mut().unwrap().copy_from_slice(&payload);
                    self.specifications.get_mut(&id.serial).unwrap().0 = pipeline.references.clone();
                    self.mecc_events = Some(pipeline.bank.clone());
                    *contexts = pipeline.contexts.staged_copy();
                }
                // A selector can consume shared RNG before a later channel error.
                *random = pipeline.random;
                let stage = stage?;
                pipeline.channels = stage.channels;
                output.fields = stage.fields;
                output.payload = stage.payload;
                // Publish completed physical/first-ANM/selection effects before
                // calling actual world constructors. A later error cannot replay.
                let actor = self.residents.get_mut(&id.serial).unwrap();
                actor.mecc_animation_stamp = pipeline.last_stamp;
                actor.channels = Some(pipeline.channels);
                actor.construction_fields = Some(output.fields.clone());
                actor
                    .seed
                    .storage
                    .data_mut()
                    .unwrap()
                    .copy_from_slice(&output.payload);
                self.specifications.get_mut(&id.serial).unwrap().0 = pipeline.references.clone();
                self.mecc_events = Some(pipeline.bank.clone());
                *contexts = pipeline.contexts.staged_copy();
                *random = pipeline.random;
                if stage.commit_weapon {
                    factory.commit_weapon(self, id, catalog)?;
                }
                let actor = self.get(id).unwrap();
                output.fields = actor.construction_fields.as_ref().unwrap().clone();
                output.payload = actor.seed.storage.data().unwrap().to_vec();
                let word = |at| u32::from_le_bytes(output.payload[at..at + 4].try_into().unwrap());
                secondary_projection = Some(crate::mecc_physical_cycle::SecondaryProjection {
                    context: stage.context,
                    words: [
                        word(0x30),
                        word(0x60),
                        word(0x68),
                        word(0x6c),
                        word(0x80),
                        word(0xd4),
                        word(0xd8),
                        output.fields.control_flags,
                    ],
                    weapon_commit: stage.commit_weapon,
                });
                stage.request
            } else {
                pipeline.select_normal(&output.fields, &cycle.view, globals)?
            };
            let second = pipeline.advance(&output.fields, &cycle.view, 0.)?;
            // Inactive auxiliary flags: actual47ecc0 tail after both ANM passes.
            let scalar_word = |at| u32::from_le_bytes(output.payload[at..at+4].try_into().unwrap());
            let local_seconds = crate::object_time::seconds(global_seconds, Some(crate::object_time::Sample { owner: 0, position: output.fields.position }), None, areas);
            let timers = crate::mecc_auxiliary::advance_timers(scalar_word(0xd8), [f32::from_bits(scalar_word(0x194)),f32::from_bits(scalar_word(0x198))], local_seconds)?;
            output.payload[0x198..0x19c].copy_from_slice(&timers[1].to_le_bytes());
            output.payload[0x194..0x198].copy_from_slice(&timers[0].to_le_bytes());
            let visits = first.visits + second.visits;
            let projection = crate::mecc_physical_cycle::ChannelProjection {
                first: first.channels,
                second: second.channels,
                selected,
                first_references: first.references,
                second_references: second.references,
                first_stamp: Some(first.stamp),
                second_stamp: Some(second.stamp),
                first_primary_flags: primary_mark,
                first_processed: first.processed,
                second_processed: second.processed,
            };
            output.fields.channels = projection.second;
            Ok((output, projection, visits))
        });
        let (output, projection, visits) = match completed {
            Ok(value) => value,
            Err(error) => {
                self.residents
                    .get_mut(&id.serial)
                    .unwrap()
                    .mecc_animation_stamp = pipeline.last_stamp;
                cycle.failure = Some(error.clone());
                self.residents
                    .get_mut(&id.serial)
                    .unwrap()
                    .mecc_physical_cycle = Some(cycle);
                return Err(error);
            }
        };
        cycle.accept_interleaved(projection.clone());
        let view = cycle.view.clone();
        let actor = self.residents.get_mut(&id.serial).unwrap();
        actor.affine_cache = Some(OwnedAffine {
            value: view.affine.unwrap(),
            pose: affine_pose_words(&output.fields),
            external_orientation: false,
        });
        actor.mecc_animation_stamp = pipeline.last_stamp;
        actor.channels = Some(projection.second);
        actor.construction_fields = Some(output.fields);
        actor
            .seed
            .storage
            .data_mut()
            .unwrap()
            .copy_from_slice(&output.payload);
        actor.mecc_physical_cycle = Some(cycle);
        self.specifications.get_mut(&id.serial).unwrap().0 = pipeline.references;
        self.mecc_events = Some(pipeline.bank);
        *contexts = pipeline.contexts;
        *random = pipeline.random;
        Ok(crate::mecc_physical_cycle::InterleavedProjection {
            fire: fire_projection,
            physical: view,
            channels: projection,
            event_visits: visits,
            first_physics,
            secondary: secondary_projection,
            jet: jet_projection,
        })
    }
    /// Source-descriptor channel/reference projection for the cold living
    /// unblended pistol profile. Events and the secondary callback remain
    /// required services; this operation never marks40.
    pub fn project_mecc_normal_channels(
        &mut self,
        id: Id,
        globals: &giants_formats::global_data::GlobalData,
    ) -> Result<crate::mecc_physical_cycle::ChannelProjection, String> {
        let health = self.mecc_health(id)?;
        self.mecc_physical_cycle(id)?;
        let actor = self.get(id).unwrap();
        let body = actor.seed.storage.data().unwrap();
        let word = |at| u32::from_le_bytes(body[at..at + 4].try_into().unwrap());
        if health.damage != 0.
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || [word(0x180), word(0x184), word(0x188)] != [1, u32::MAX, u32::MAX]
        {
            return Err(
                "normal channel projection requires undamaged unparented cold pistol".into(),
            );
        }
        let mut cycle = actor.mecc_physical_cycle.clone().unwrap();
        let mut projection = cycle.project_channels(
            actor.construction_fields.as_ref().unwrap(),
            actor.channels.ok_or("Mecc channels absent")?,
            globals,
        )?;
        let mut references = self
            .specifications
            .get(&id.serial)
            .ok_or("Mecc reference resources absent")?
            .0
            .clone();
        if projection.first_processed {
            references.update_current_unblended(&projection.first)?;
            projection.first_references = Some(references.matrices().to_vec());
        }
        if projection.second_processed {
            references.update_current_unblended(&projection.second)?;
            projection.second_references = Some(references.matrices().to_vec());
        }
        let actor = self.residents.get_mut(&id.serial).unwrap();
        actor.channels = Some(projection.second);
        actor.construction_fields.as_mut().unwrap().channels = projection.second;
        cycle.channel_projection = Some(projection.clone());
        actor.mecc_physical_cycle = Some(cycle);
        self.specifications.get_mut(&id.serial).unwrap().0 = references;
        Ok(projection)
    }
    /// Real deferred FX preparation and shared event mutations over the owned
    /// staged channel/reference views. This is not an interleaved full primary.
    pub fn project_mecc_effect_events(
        &mut self,
        id: Id,
        contexts: &mut crate::attached_effect_context::Pool<Id>,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<u32, String> {
        let health = self.mecc_health(id)?;
        self.mecc_physical_cycle(id)?;
        let actor = self.get(id).unwrap();
        let cycle = actor.mecc_physical_cycle.as_ref().unwrap();
        if cycle.interleaved {
            return Err("interleaved Mecc cannot replay the separate event projection".into());
        }
        if cycle.event_number.checked_add(1) != Some(cycle.view.number) {
            return Err("Mecc event projection skipped or repeated a step".into());
        }
        let projection = cycle
            .channel_projection
            .as_ref()
            .ok_or("Mecc channels not projected")?;
        let fields = actor.construction_fields.as_ref().unwrap();
        if health.damage != 0.
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || fields.action != 0
            || fields.control_flags != 0
            || fields.object_flags & 0x1c000 != 0
            || actor.channels != Some(projection.second)
        {
            return Err("Mecc event source state changed after channel projection".into());
        }
        let affine = self.actor_current_affine(id)?.matrix;
        if affine.map(f32::to_bits)
            != cycle
                .view
                .affine
                .ok_or("Mecc affine absent")?
                .matrix
                .map(f32::to_bits)
        {
            return Err("Mecc event pose changed after physical projection".into());
        }
        let specification = self.specification(id).unwrap();
        let mut bank = self
            .mecc_events
            .clone()
            .unwrap_or_else(|| crate::mecc_animation_events::Bank::new(specification.clone()));
        if !bank.belongs_to(specification) {
            return Err(
                "Mecc event table requires its canonical registered class resources".into(),
            );
        }
        let mut staged_contexts = contexts.staged_copy();
        let mut staged_random = *random;
        let mut visits = 0;
        for (processed, channels, references) in [
            (
                projection.first_processed,
                &projection.first,
                &projection.first_references,
            ),
            (
                projection.second_processed,
                &projection.second,
                &projection.second_references,
            ),
        ] {
            if processed {
                visits += bank.dispatch(
                    crate::mecc_animation_events::Input {
                        material: crate::mecc_animation_events::Material::Unknown,
                        owner: id,
                        fields,
                        channels,
                        references: references.as_ref().ok_or("event reference tail absent")?,
                        affine: Some(affine),
                    },
                    &mut staged_contexts,
                    &mut staged_random,
                )?;
            }
        }
        self.mecc_events = Some(bank);
        let cycle = self
            .residents
            .get_mut(&id.serial)
            .unwrap()
            .mecc_physical_cycle
            .as_mut()
            .unwrap();
        cycle.event_number = cycle.view.number;
        *contexts = staged_contexts;
        *random = staged_random;
        Ok(visits)
    }
    pub fn mecc_event_kinds(&self, id: Id, slot: u8) -> Result<Vec<u32>, String> {
        self.mecc_health(id)?;
        let bank = self
            .mecc_events
            .as_ref()
            .ok_or("Mecc event bank not prepared")?;
        if !bank.belongs_to(self.specification(id).ok_or("Mecc resources absent")?) {
            return Err("Mecc event resources differ from shared class bank".into());
        }
        bank.kinds(slot)
    }
    /// Cancel this player's prepared-only smoke before retirement. Active effects
    /// require their separate interpreter and emitted-object lifecycle.
    pub fn release_mecc_damage_smoke(
        &mut self,
        id: Id,
        contexts: &mut crate::attached_effect_context::Pool<Id>,
    ) -> Result<(), String> {
        self.mecc_health(id)?;
        let state = self.get(id).unwrap().mecc_damage_smoke.clone();
        if let Some(binding) = state.binding {
            let context = contexts
                .get(binding)
                .ok_or("damage smoke cancellation binding absent, foreign or stale")?;
            if context.recipient() != Some(id)
                || context.tag() != Some(3)
                || context.flags() != 0x20004
                || context.label != *b"FX_mecc_smoking_"
            {
                return Err("damage smoke cancellation belongs to another source/profile".into());
            }
            contexts.release_prepared(binding)?;
        }
        let actor = self.residents.get_mut(&id.serial).unwrap();
        actor.mecc_damage_smoke = Default::default();
        actor.seed.storage.data_mut().unwrap()[0x1a0..0x1a8].fill(0);
        Ok(())
    }
    pub fn mecc_health(&self, id: Id) -> Result<crate::mecc_damage::State, String> {
        let actor = self.get(id).ok_or("Mecc health actor absent or foreign")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("Mecc fields absent")?;
        let payload = actor.seed.storage.data().ok_or("Mecc payload absent")?;
        if actor.class != 2
            || actor.seed.storage.profile() != Profile::Debug
            || actor.seed.storage.state() != 2
            || payload.len() != 524
            || !actor.mecc_special.is_some_and(|phase| phase.player_fields)
        {
            return Err("Mecc health requires acquired cold-player fields".into());
        }
        let state = crate::mecc_damage::State {
            damage: actor.mecc_damage.ok_or("Mecc accumulated damage absent")?,
            limit: f32::from_bits(fields.configuration[12]),
            shield: f32::from_le_bytes(payload[0x4c..0x50].try_into().unwrap()),
            flags: fields.control_flags,
        };
        if ![state.damage, state.limit, state.shield]
            .into_iter()
            .all(f32::is_finite)
            || state.limit <= 0.
            || state.shield < 0.
        {
            return Err("owned Mecc health outside finite acquired profile".into());
        }
        Ok(state)
    }
    /// Source-absent, zero-direction, nonlethal current-D path. Present voice,
    /// gyro, shield equipment, directional impulse and death are refused before
    /// publication rather than acknowledged by an empty service.
    pub fn damage_unattributed_mecc_nonlethal(
        &mut self,
        id: Id,
        kind: i32,
        amount: f32,
        context: MeccDamageContext,
    ) -> Result<crate::mecc_damage::Result, String> {
        self.damage_unattributed_mecc_nonlethal_inner(id, kind, amount, context, false, false)
    }
    fn damage_unattributed_mecc_nonlethal_inner(
        &mut self,
        id: Id,
        kind: i32,
        amount: f32,
        context: MeccDamageContext,
        expired_impact_cooldown: bool,
        expired_shared_voice: bool,
    ) -> Result<crate::mecc_damage::Result, String> {
        let state = self.mecc_health(id)?;
        let actor = self.get(id).unwrap();
        let fields = actor.construction_fields.as_ref().unwrap();
        let payload = actor.seed.storage.data().unwrap();
        let word = |at| u32::from_le_bytes(payload[at..at + 4].try_into().unwrap());
        let cooldown = f32::from_bits(word(0x1fc));
        if !context.ambient_voice_timer.is_finite()
            || context.ambient_voice_timer < 0. && !(expired_shared_voice && kind == 16)
            || !cooldown.is_finite()
            || cooldown < 0. && !(expired_impact_cooldown && kind == 16)
            || !state.alive()
            // Acquired self-point kind16 excludes directional response even
            // when the current-history position differs from the live position.
            || kind != 16 && fields.position.map(f32::to_bits) != fields.current.map(f32::to_bits)
            || !fields.position.into_iter().all(f32::is_finite)
            || fields.control_flags & 0x20000 != 0
            || fields.configuration[14] & 0x04000000 != 0
            || fields.configuration[15] & 0x04000000 != 0
            || fields.response_flags & 15 != 0
            || actor.presentation_parent.is_some()
            || actor.attachment.is_some()
            || word(0x190) == 17
            || word(0xfc) != 0
        {
            return Err(
                "Mecc damage requires acquired inactive-service zero-direction profile".into(),
            );
        }
        let result = state.apply_for(
            Profile::Debug,
            crate::mecc_damage::Hit {
                kind,
                amount,
                globally_disabled: context.globally_disabled,
                cutscene: context.cutscene,
                allowed: true,
                shield_percent: context.shield_percent,
            },
        )?;
        if result.died {
            return Err("Mecc death requires the real actor death service".into());
        }
        let actor = self.residents.get_mut(&id.serial).unwrap();
        actor.mecc_damage = Some(result.state.damage);
        actor.construction_fields.as_mut().unwrap().control_flags = result.state.flags;
        let payload = actor.seed.storage.data_mut().unwrap();
        payload[0x4c..0x50].copy_from_slice(&result.state.shield.to_le_bytes());
        if !context.globally_disabled {
            payload[0xf4..0xf8].copy_from_slice(&u32::MAX.to_le_bytes());
        }
        Ok(result)
    }
    pub fn perception_candidate(
        &self,
        id: Id,
    ) -> Result<crate::ripper_perception::Candidate<Id>, String> {
        let actor = self.get(id).ok_or("perception actor absent or foreign")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("perception fields absent")?;
        let (damage, limit, parent) = if actor.class == 2 {
            let health = self.mecc_health(id)?;
            (
                health.damage,
                health.limit,
                actor.presentation_parent.is_some(),
            )
        } else {
            let combat = self.state23_snapshot(id)?;
            (
                combat.damage,
                combat.limit,
                combat.parent.is_some() || actor.presentation_parent.is_some(),
            )
        };
        Ok(crate::ripper_perception::Candidate {
            id,
            class: actor.class,
            state: u32::from(actor.seed.storage.state()),
            active: fields.object_flags,
            personality: fields.configuration[14],
            control: fields.control_flags,
            position: fields.position,
            damage,
            limit,
            parent_present: parent,
        })
    }
    pub fn perception_subset(&self, ids: &[Id]) -> Result<OwnedPerceptionSubset, String> {
        let unique = ids
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        if unique.len() != ids.len() {
            return Err("duplicate owned perception identity".into());
        }
        let candidates = ids
            .iter()
            .map(|&id| self.perception_candidate(id))
            .collect::<Result<_, _>>()?;
        let order = self
            .list(1)
            .into_iter()
            .filter(|id| unique.contains(id))
            .collect();
        Ok(OwnedPerceptionSubset { candidates, order })
    }
    fn validate_perception_frame(
        &self,
        frame: &crate::ripper_perception::Frame<'_, Id>,
    ) -> Result<(), String> {
        for candidate in frame.candidates() {
            let current = self.perception_candidate(candidate.id)?;
            if candidate.class != current.class
                || candidate.state != current.state
                || candidate.active != current.active
                || candidate.personality != current.personality
                || candidate.control != current.control
                || candidate.parent_present != current.parent_present
                || candidate.position.map(f32::to_bits) != current.position.map(f32::to_bits)
                || candidate.damage.to_bits() != current.damage.to_bits()
                || candidate.limit.to_bits() != current.limit.to_bits()
            {
                return Err("perception view disagrees with current owned fields or health".into());
            }
        }
        Ok(())
    }
    pub fn state23_combat(&self, id: Id) -> Option<&crate::state23_damage::State<Id>> {
        self.get(id)?.state23_combat.as_ref()
    }
    /// Own the acquired fresh constructor's rates, look pitch and commands.
    /// Never seed a running actor's history with constructor zeros.
    pub fn prepare_actor_motion(&mut self, id: Id) -> Result<(), String> {
        let actor = self.get(id).ok_or("motion actor absent or foreign")?;
        if !actor.acquired_construction
            || actor.seed.storage.state() != 23
            || actor.state23_combat.is_some()
            || actor.ordinary_motion.is_some()
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
        {
            return Err("motion requires fresh unattached acquired construction".into());
        }
        let motion = crate::resident_motion::Owned::fresh(
            actor
                .construction_fields
                .as_ref()
                .ok_or("motion fields absent")?,
        )?;
        self.residents.get_mut(&id.serial).unwrap().ordinary_motion = Some(motion);
        Ok(())
    }
    pub fn actor_motion(&self, id: Id) -> Result<&crate::resident_motion::Owned, String> {
        self.get(id)
            .ok_or("motion actor absent or foreign")?
            .ordinary_motion
            .as_ref()
            .ok_or_else(|| "ordinary motion not prepared".into())
    }
    /// Actual fresh constructor/mission-write fields, before any primary has
    /// published. The machine retains its typed FX identities and extra body
    /// scalars; no native output bytes or ready-made AI mode are imported.
    pub fn initial_ripper_primary<E>(
        &self,
        id: Id,
    ) -> Result<crate::ripper_primary_turn::State<Id, E>, String> {
        let actor = self.ordinary_frame_actor(id)?;
        let motion = self.actor_motion(id)?;
        if actor.class != 73
            || actor.state23_combat.is_some()
            || actor.surface_link.is_some()
            || !motion.awaiting_motion()
            || motion.commands() != [0; 8]
            || motion.rates() != [0.; 4]
            || motion.previous_rates() != [0.; 4]
            || motion.look_pitch() != 0.
            || self.actor_finalization(id)?.phase()
                != crate::resident_finalization::Phase::MotionStarted
        {
            return Err("initial primary requires a fresh admitted owned class73 frame".into());
        }
        let combat = self.state23_snapshot(id)?;
        let f = actor
            .construction_fields
            .as_ref()
            .ok_or("initial primary fields absent")?;
        let data = actor
            .seed
            .storage
            .data()
            .ok_or("initial primary payload absent")?;
        let scalar = |at| f32::from_le_bytes(data[at..at + 4].try_into().unwrap());
        Ok(crate::ripper_primary_turn::State {
            entry: crate::state23_entry::State {
                class: 73,
                context: f.dispatch_context,
                mode: 0,
                action: f.action,
                flags: f.object_flags,
                control: f.control_flags,
                packed: 0,
                position: f.position,
                parameter: f32::from_bits(f.configuration[1]),
                active_limit: f32::from_bits(f.configuration[11]),
                saved_parameter: scalar(0x20),
                retained_timer: scalar(0x54),
                target: None,
            },
            steering: crate::ripper_steering::State {
                position: f.position,
                heading: f.angles[0],
                flags: f.object_flags,
                personality: f.configuration[14],
                commands: [0; 8],
                target_heading: 0.,
                target_position: [0.; 3],
            },
            orbit: crate::ripper_steering::Orbit {
                angle: scalar(4),
                radius: scalar(8),
            },
            timer: scalar(0),
            effect: None,
            retaliation: combat.retaliation,
            primary: actor.channels.ok_or("initial primary channels absent")?[0],
            parameter_184: 0.,
        })
    }
    pub fn publish_initial_ripper_primary<E: Copy>(
        &mut self,
        id: Id,
        machine: &crate::ripper_primary_turn::Machine<Id, E>,
        buried_speed_percent: f32,
    ) -> Result<(), String> {
        let original = self.initial_ripper_primary::<E>(id)?;
        let s = machine.state();
        let expected = if matches!(s.entry.mode, 31 | 34 | 36) {
            (f64::from(buried_speed_percent)
                * f64::from(original.entry.parameter)
                * f64::from(0.01_f32)) as f32
        } else {
            original.entry.parameter
        };
        if !machine.awaiting_motion()
            || !buried_speed_percent.is_finite()
            || !matches!(s.entry.mode, 31 | 34 | 44)
            || s.entry.context != 100
            || s.entry.class != 73
            || s.entry.action != 0
            || s.entry.active_limit.to_bits() != 0
            || s.entry.saved_parameter.to_bits() != original.entry.parameter.to_bits()
            || s.entry.parameter.to_bits() != expected.to_bits()
            || s.entry.position.map(f32::to_bits) != original.entry.position.map(f32::to_bits)
            || s.steering.position.map(f32::to_bits) != original.entry.position.map(f32::to_bits)
            || s.steering.heading.to_bits() != original.steering.heading.to_bits()
            || s.primary != original.primary
            || s.steering.personality != original.steering.personality
            || s.entry.control != original.entry.control
            || s.entry.flags != original.entry.flags & !0x0300_0000
        {
            return Err("initial primary result disagrees with its fresh owned frame".into());
        }
        if let Some(target) = s.retaliation {
            self.get(target)
                .ok_or("initial primary target absent or foreign")?;
        }
        let mut combat = self.state23_snapshot(id)?;
        combat.mode = s.entry.mode;
        combat.timer = s.timer;
        combat.flags = s.entry.flags;
        combat.retaliation = s.retaliation;
        let actor = self.residents.get_mut(&id.serial).unwrap();
        let f = actor.construction_fields.as_mut().unwrap();
        f.object_flags = s.entry.flags;
        f.dispatch_context = s.entry.context;
        f.configuration[11] = s.entry.active_limit.to_bits();
        f.configuration[1] = s.entry.parameter.to_bits();
        actor.state23_combat = Some(combat);
        Ok(())
    }
    /// Bind the actual constructor snapshot once. The callback machine, typed
    /// effect IDs and retained body fields now live with the resident.
    pub fn prepare_ripper_primary(&mut self, id: Id) -> Result<(), String> {
        if self.get(id).is_some_and(|a| a.ripper_primary.is_some()) {
            return Err("resident primary already prepared".into());
        }
        let state = self.initial_ripper_primary(id)?;
        self.residents.get_mut(&id.serial).unwrap().ripper_primary =
            Some(crate::ripper_primary_turn::Machine::new(state));
        Ok(())
    }
    pub fn ripper_primary(
        &self,
        id: Id,
    ) -> Result<&crate::ripper_primary_turn::Machine<Id, crate::attached_effect_context::Id>, String>
    {
        self.get(id)
            .and_then(|a| a.ripper_primary.as_ref())
            .ok_or_else(|| "resident primary absent or foreign".into())
    }
    /// Use the actual static terrain producer for the owned primary. The
    /// remaining services are still required, including real resource absence.
    /// A world with active deformation must use an acquired dynamic provider.
    pub fn update_ripper_awakening_on_static_terrain(
        &mut self,
        id: Id,
        input: crate::ripper_primary_turn::AwakeningFrame<'_, Id>,
        terrain: &crate::terrain_queries::StaticTerrainQuery<'_>,
        random: &mut crate::audio_parameters::Random,
        services: &mut impl crate::ripper_primary_turn::Services<Id, crate::attached_effect_context::Id>,
    ) -> Result<crate::ripper_primary_turn::Outcome, String> {
        self.update_ripper_awakening(
            id,
            input,
            random,
            &mut crate::ripper_ground::StaticGround {
                query: terrain,
                services,
            },
        )
    }
    /// Current owned header/channels feed the persistent machine; retained body
    /// scalars and FX identities never come from an external ready-made mode.
    pub fn update_ripper_awakening(
        &mut self,
        id: Id,
        input: crate::ripper_primary_turn::AwakeningFrame<'_, Id>,
        random: &mut crate::audio_parameters::Random,
        services: &mut impl crate::ripper_primary_turn::Services<Id, crate::attached_effect_context::Id>,
    ) -> Result<crate::ripper_primary_turn::Outcome, String> {
        let crate::ripper_primary_turn::AwakeningFrame {
            context,
            frame,
            range,
            parameters,
        } = input;
        self.update_ripper_owned_primary(
            id,
            crate::ripper_primary_turn::FrameInput {
                context,
                frame,
                range,
            },
            random,
            services,
            |_, machine, input, random, services| {
                machine.advance_awakening(
                    input.context,
                    input.frame,
                    input.range,
                    parameters,
                    random,
                    services,
                )
            },
        )
    }
    pub fn update_ripper_emergence_on_static_terrain(
        &mut self,
        id: Id,
        input: crate::ripper_primary_turn::FrameInput<'_, Id>,
        buried_speed_percent: f32,
        terrain: &crate::terrain_queries::StaticTerrainQuery<'_>,
        random: &mut crate::audio_parameters::Random,
        services: &mut impl crate::ripper_primary_turn::Services<Id, crate::attached_effect_context::Id>,
    ) -> Result<crate::ripper_primary_turn::Outcome, String> {
        self.update_ripper_emergence(
            id,
            input,
            buried_speed_percent,
            random,
            &mut crate::ripper_ground::StaticGround {
                query: terrain,
                services,
            },
        )
    }
    /// Continue the same persistent actor in34; no retained pose, AI mode,
    /// target, effect handle or saved speed is supplied by this caller.
    pub fn update_ripper_emergence(
        &mut self,
        id: Id,
        input: crate::ripper_primary_turn::FrameInput<'_, Id>,
        buried_speed_percent: f32,
        random: &mut crate::audio_parameters::Random,
        services: &mut impl crate::ripper_primary_turn::Services<Id, crate::attached_effect_context::Id>,
    ) -> Result<crate::ripper_primary_turn::Outcome, String> {
        self.update_ripper_owned_primary(
            id,
            input,
            random,
            services,
            |_, machine, input, random, services| {
                machine.advance_emergence(
                    input.context,
                    input.frame,
                    input.range,
                    buried_speed_percent,
                    random,
                    services,
                )
            },
        )
    }
    /// Continue44 with the same owned machine and real marker factory. Services
    /// may mutate reciprocal relations; the common publisher preserves them.
    pub fn update_ripper_surfacing(
        &mut self,
        id: Id,
        input: crate::ripper_primary_turn::FrameInput<'_, Id>,
        parameters: crate::ripper_turn_completion::Parameters,
        random: &mut crate::audio_parameters::Random,
        factory: &mut crate::primary_surface_marker::Factory<'_>,
        dependencies: &mut impl crate::primary_surface_marker::Dependencies<
            crate::attached_effect_context::Id,
        >,
    ) -> Result<crate::ripper_primary_turn::Outcome, String> {
        self.update_ripper_owned_primary(
            id,
            input,
            random,
            &mut (factory, dependencies),
            |session, machine, input, random, (factory, dependencies)| {
                let mut bridge =
                    crate::primary_surface_marker::Bridge::new(session, factory, id, *dependencies);
                let outcome = machine.advance_surfacing(
                    input.context,
                    input.frame,
                    input.range,
                    parameters,
                    random,
                    &mut bridge,
                )?;
                if outcome == crate::ripper_primary_turn::Outcome::CompletedSurfacing {
                    bridge.finish(machine)?;
                }
                Ok(outcome)
            },
        )
    }
    fn update_ripper_owned_primary<'a, S>(
        &mut self,
        id: Id,
        input: crate::ripper_primary_turn::FrameInput<'a, Id>,
        random: &mut crate::audio_parameters::Random,
        services: &mut S,
        callback: impl FnOnce(
            &mut Session,
            &mut crate::ripper_primary_turn::Machine<Id, crate::attached_effect_context::Id>,
            crate::ripper_primary_turn::FrameInput<'a, Id>,
            &mut crate::audio_parameters::Random,
            &mut S,
        ) -> Result<crate::ripper_primary_turn::Outcome, String>,
    ) -> Result<crate::ripper_primary_turn::Outcome, String> {
        let crate::ripper_primary_turn::FrameInput { context, frame, .. } = input;
        let actor = self.ordinary_frame_actor(id)?;
        if context.parent_state.is_some() {
            return Err("unparented resident cannot accept a supplied parent state".into());
        }
        self.validate_perception_frame(frame)?;
        let machine = self.ripper_primary(id)?;
        if machine.failure().is_some()
            || machine.awaiting_motion()
            || !self.actor_motion(id)?.awaiting_motion()
            || self.actor_finalization(id)?.phase()
                != crate::resident_finalization::Phase::MotionStarted
        {
            return Err("resident primary requires an admitted unconsumed frame".into());
        }
        let mut state = machine.state().clone();
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("primary fields absent")?;
        let combat = self.state23_snapshot(id)?;
        state.entry.mode = combat.mode;
        state.timer = combat.timer;
        state.retaliation = combat.retaliation;
        state.entry.action = fields.action;
        state.entry.context = fields.dispatch_context;
        state.entry.flags = fields.object_flags;
        state.entry.control = fields.control_flags;
        state.entry.position = fields.position;
        state.entry.parameter = f32::from_bits(fields.configuration[1]);
        state.entry.active_limit = f32::from_bits(fields.configuration[11]);
        state.steering.position = fields.position;
        state.steering.heading = fields.angles[0];
        state.steering.flags = fields.object_flags;
        state.steering.personality = fields.configuration[14];
        state.steering.commands = self.actor_motion(id)?.commands();
        state.entry.packed = u32::from_le_bytes(state.steering.commands[..4].try_into().unwrap());
        state.primary = actor.channels.ok_or("primary channels absent")?[0];
        // Take only after all owned-view checks. A failed callback is always
        // returned to the resident, including its no-replay latch.
        let mut machine = self
            .residents
            .get_mut(&id.serial)
            .unwrap()
            .ripper_primary
            .take()
            .unwrap();
        self.residents
            .get_mut(&id.serial)
            .unwrap()
            .ripper_primary_running = true;
        let result = (|| {
            machine.refresh_owned_frame(state)?;
            let outcome = callback(self, &mut machine, input, random, services)?;
            // Surface construction publishes a reciprocal callback owner. Read
            // the current relations instead of overwriting them with entry data.
            let mut combat = self.state23_snapshot(id)?;
            let state = machine.state();
            combat.mode = state.entry.mode;
            combat.flags = state.entry.flags;
            combat.personality = state.steering.personality;
            combat.control = state.entry.control;
            combat.retaliation = state.retaliation;
            combat.timer = state.timer;
            let actor = self.residents.get_mut(&id.serial).unwrap();
            let fields = actor.construction_fields.as_mut().unwrap();
            fields.object_flags = state.entry.flags;
            fields.dispatch_context = state.entry.context;
            fields.control_flags = state.entry.control;
            fields.configuration[1] = state.entry.parameter.to_bits();
            fields.configuration[11] = state.entry.active_limit.to_bits();
            fields.configuration[14] = state.steering.personality;
            actor.state23_combat = Some(combat);
            Ok(outcome)
        })();
        let actor = self.residents.get_mut(&id.serial).unwrap();
        actor.ripper_primary_running = false;
        actor.ripper_primary = Some(machine);
        result
    }
    /// Consume the commands with the resident's real rates/history once, then
    /// rearm the same machine. Finalization/secondary/clocks remain real phases
    /// owned by their existing producers, not acknowledged by this consumer.
    pub fn advance_ripper_common_motion(
        &mut self,
        id: Id,
        time: crate::resident_motion::Time<'_>,
    ) -> Result<(), String> {
        let machine = self.ripper_primary(id)?;
        if !machine.awaiting_motion() {
            return Err("resident motion requires its completed primary".into());
        }
        let s = machine.state();
        let commands = crate::resident_motion::Commands {
            bytes: s.steering.commands,
            context: s.entry.context,
            target_heading: s.steering.target_heading,
            target_position: s.steering.target_position[..2].try_into().unwrap(),
        };
        self.advance_actor_common_motion(id, commands, time)?;
        let actor = self.residents.get_mut(&id.serial).unwrap();
        let fields = actor.construction_fields.as_ref().unwrap();
        actor.ripper_primary.as_mut().unwrap().accept_owned_motion(
            fields.position,
            fields.angles[0],
            fields.object_flags,
            fields.configuration[14],
        );
        Ok(())
    }
    fn ordinary_frame_actor(&self, id: Id) -> Result<&Resident, String> {
        let actor = self
            .get(id)
            .ok_or("ordinary frame actor absent or foreign")?;
        if !actor.acquired_construction
            || actor.seed.storage.profile() != Profile::Debug
            || actor.seed.storage.state() != 23
            || !matches!(actor.class, 73 | 230)
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || actor
                .state23_combat
                .as_ref()
                .is_some_and(|s| s.parent.is_some())
        {
            return Err("ordinary frame outside acquired unparented D Ripper".into());
        }
        Ok(actor)
    }
    /// Fresh physical/footprint history is acquired only before callbacks and
    /// motion. Environment and affine must already be owned and current.
    pub fn prepare_actor_finalization(&mut self, id: Id) -> Result<(), String> {
        let actor = self.ordinary_frame_actor(id)?;
        if actor.finalization.is_some()
            || actor.ordinary_motion.is_some()
            || actor.state23_combat.is_some()
        {
            return Err("finalization requires fresh initialization before callbacks".into());
        }
        let owned = crate::resident_finalization::Owned::fresh(
            actor
                .construction_fields
                .as_ref()
                .ok_or("finalization fields absent")?,
            self.actor_environment(id)?,
            self.actor_current_affine(id)?,
        )?;
        self.residents.get_mut(&id.serial).unwrap().finalization = Some(owned);
        Ok(())
    }
    pub fn actor_finalization(
        &self,
        id: Id,
    ) -> Result<&crate::resident_finalization::Owned, String> {
        self.get(id)
            .ok_or("finalization actor absent or foreign")?
            .finalization
            .as_ref()
            .ok_or_else(|| "owned finalization not prepared".into())
    }
    /// Common position history precedes the primary wrapper. This method does
    /// not execute or acknowledge the primary behavior callback.
    pub fn begin_actor_frame_history(&mut self, id: Id) -> Result<(), String> {
        let actor = self.ordinary_frame_actor(id)?;
        if actor.world_tick_running
            || actor
                .world_tick
                .as_ref()
                .is_some_and(|tick| tick.phase() != crate::actor_world_tick::Phase::Finished)
        {
            return Err(
                "world iteration failed or remains incomplete; history cannot replay it".into(),
            );
        }
        if actor
            .ordinary_motion
            .as_ref()
            .is_none_or(|m| m.awaiting_motion())
        {
            return Err("frame history requires prepared idle ordinary motion".into());
        }
        self.actor_environment(id)?;
        let actor = self.residents.get_mut(&id.serial).unwrap();
        actor
            .finalization
            .as_mut()
            .ok_or("owned finalization not prepared")?
            .begin(
                actor
                    .construction_fields
                    .as_mut()
                    .ok_or("frame fields absent")?,
            )?;
        actor.world_tick = None;
        Ok(())
    }
    pub fn actor_world_flags(&self, id: Id) -> Result<u32, String> {
        self.get(id)
            .and_then(|a| a.construction_fields.as_ref())
            .map(|f| f.object_flags)
            .ok_or_else(|| "world iteration fields absent or foreign".into())
    }
    pub fn actor_world_tick(&self, id: Id) -> Result<&crate::actor_world_tick::Tick, String> {
        self.get(id)
            .and_then(|a| a.world_tick.as_ref())
            .ok_or_else(|| "owned world iteration absent or foreign".into())
    }
    /// Run one iteration after the separate position-history pass. The token
    /// stays with its resident, including after a partially executed service
    /// failure. Re-entering with a new caller-owned token cannot replay it.
    /// List traversal and all surrounding world phases remain separate work.
    pub fn update_actor_world(
        &mut self,
        id: Id,
        services: &mut impl ActorWorldServices,
    ) -> Result<crate::actor_world_tick::Outcome, String> {
        let actor = self.ordinary_frame_actor(id)?;
        if actor.world_tick_running || actor.world_tick.is_some() {
            return Err("owned world iteration already consumed; fresh history required".into());
        }
        if actor
            .finalization
            .as_ref()
            .is_none_or(|f| f.phase != crate::resident_finalization::Phase::History)
        {
            return Err("world iteration requires the separate owned position history".into());
        }
        let mut tick = crate::actor_world_tick::Tick::new();
        self.residents
            .get_mut(&id.serial)
            .unwrap()
            .world_tick_running = true;
        let result = tick.run(&mut ActorWorldAdapter {
            session: self,
            source: id,
            services,
        });
        let actor = self
            .residents
            .get_mut(&id.serial)
            .ok_or("world iteration actor removed during its callbacks")?;
        actor.world_tick_running = false;
        actor.world_tick = Some(tick);
        result
    }
    /// Continue with actual environment/release services. A started failure is
    /// retained and latched: it cannot replay earlier external effects.
    pub fn finalize_actor(
        &mut self,
        id: Id,
        time: crate::actor_finalization::Time<'_>,
        services: &mut impl crate::actor_finalization::Services,
    ) -> Result<(), String> {
        let actor = self.ordinary_frame_actor(id)?;
        let mut state = actor
            .finalization
            .as_ref()
            .ok_or("owned finalization not prepared")?
            .input(
                actor
                    .construction_fields
                    .as_ref()
                    .ok_or("finalization fields absent")?,
                self.actor_motion(id)?.rates()[0],
            )?;
        // Bounds belong to the preceding explicit geometry phase, not to the
        // finalizer. Carry that owned buffer forward without calling it current.
        state.fields.bounds = actor
            .environment
            .as_ref()
            .ok_or("finalization environment absent")?
            .geometry_fields()
            .bounds;
        let result = crate::actor_finalization::finalize(state, time, services);
        let (state, phase, error) = match result {
            Ok(state) => (state, crate::resident_finalization::Phase::Idle, None),
            Err(failure) => {
                if !failure.phase_started {
                    return Err(failure.message);
                }
                (
                    *failure.state,
                    crate::resident_finalization::Phase::Failed,
                    Some(failure.message),
                )
            }
        };
        let actor = self.residents.get_mut(&id.serial).unwrap();
        let header = actor.construction_fields.as_mut().unwrap();
        header.position = state.fields.current;
        header.current = state.fields.previous;
        header.angles = state.fields.angles;
        header.object_flags = state.fields.object_flags;
        header.response_flags = state.fields.response_flags;
        header.configuration[14] = state.fields.collision_flags;
        actor
            .ordinary_motion
            .as_mut()
            .unwrap()
            .publish_finalized_vertical_speed(state.fields.model_vertical_speed);
        if error.is_none() {
            let value = state
                .affine
                .ok_or("ordinary finalization did not produce an affine")?;
            actor.affine_cache = Some(OwnedAffine {
                value,
                pose: affine_pose_words(header),
                external_orientation: header.object_flags & 0x10000 != 0,
            });
        }
        let owned = actor.finalization.as_mut().unwrap();
        owned.state = state;
        owned.phase = phase;
        owned.failure = error.clone();
        error.map_or(Ok(()), Err)
    }
    /// Primary wrapper prefix only. The actual behavior callback still belongs
    /// between this phase and the common motion consumer below.
    pub fn begin_actor_motion_history(
        &mut self,
        id: Id,
        commands: [u8; 8],
    ) -> Result<bool, String> {
        let actor = self.get(id).ok_or("motion actor absent or foreign")?;
        if actor
            .finalization
            .as_ref()
            .is_some_and(|f| f.phase != crate::resident_finalization::Phase::History)
        {
            return Err("owned frame requires position history before its primary prefix".into());
        }
        if actor.seed.storage.state() != 23
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || actor
                .state23_combat
                .as_ref()
                .is_some_and(|s| s.parent.is_some())
        {
            return Err("ordinary history outside acquired unattached state23".into());
        }
        let actor = self.residents.get_mut(&id.serial).unwrap();
        let admitted = actor
            .ordinary_motion
            .as_mut()
            .ok_or("ordinary motion not prepared")?
            .begin(
                actor
                    .construction_fields
                    .as_mut()
                    .ok_or("motion fields absent")?,
                commands,
            )?;
        if let Some(frame) = &mut actor.finalization {
            frame.phase = if admitted {
                crate::resident_finalization::Phase::MotionStarted
            } else {
                crate::resident_finalization::Phase::MotionFinished
            };
        }
        Ok(admitted)
    }
    /// Consume supplied behavior outputs with owned rates/history and class
    /// parameters. No primary callback, finalization or readiness is implied.
    pub fn advance_actor_common_motion(
        &mut self,
        id: Id,
        commands: crate::resident_motion::Commands,
        time: crate::resident_motion::Time<'_>,
    ) -> Result<(), String> {
        let actor = self.get(id).ok_or("motion actor absent or foreign")?;
        if actor
            .finalization
            .as_ref()
            .is_some_and(|f| f.phase != crate::resident_finalization::Phase::MotionStarted)
        {
            return Err("owned frame common motion is not pending".into());
        }
        if actor.seed.storage.state() != 23
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || actor
                .state23_combat
                .as_ref()
                .is_some_and(|s| s.parent.is_some())
        {
            return Err("ordinary motion outside acquired unattached state23".into());
        }
        let mode = actor.state23_combat.as_ref().map_or(0, |s| s.mode);
        if matches!(mode, 20 | 103 | 104 | 108) {
            return Err(
                "state23 mode bypasses common motion; its primary continuation is required".into(),
            );
        }
        let actor = self.residents.get_mut(&id.serial).unwrap();
        actor
            .ordinary_motion
            .as_mut()
            .ok_or("ordinary motion not prepared")?
            .advance(
                actor
                    .construction_fields
                    .as_mut()
                    .ok_or("motion fields absent")?,
                commands,
                time,
            )?;
        if let Some(frame) = &mut actor.finalization {
            frame.phase = crate::resident_finalization::Phase::MotionFinished;
        }
        Ok(())
    }
    /// Apply the acquired secondary callback to owned channels and combat fields.
    /// Global parameters are supplied. Parent/target views come from owned IDs.
    /// This does not run the primary behavior, advance clocks or certify readiness.
    /// A partially failed attack cannot be retried on this resident.
    pub fn update_state23_animation(
        &mut self,
        id: Id,
        mut context: crate::state23_animation::Context,
        random: &mut crate::audio_parameters::Random,
        services: &mut impl State23AnimationServices,
    ) -> Result<(), String> {
        let mut combat = self.state23_snapshot(id)?;
        let actor = self.get(id).ok_or("animation actor absent")?;
        if actor.state23_animation_failed {
            return Err("state23 animation service attempt already failed; cancellation or explicit recovery required".into());
        }
        let channels = actor.channels.ok_or("state23 animation channels absent")?;
        let parent = actor.attachment.map(|a| a.parent).or(combat.parent);
        context.parent_state = parent
            .map(|parent| {
                self.get(parent)
                    .map(|r| u32::from(r.seed.storage.state()))
                    .ok_or("state23 animation parent absent or foreign")
            })
            .transpose()?;
        if matches!(context.parent_state, Some(1 | 76)) {
            return Ok(());
        }
        context.target_position = combat
            .retaliation
            .map(|target| {
                self.get(target)
                    .and_then(|r| r.construction_fields.as_ref())
                    .map(|f| f.position)
                    .ok_or("state23 animation objective fields absent or foreign")
            })
            .transpose()?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("animation fields absent")?;
        context.leap_reference_speed =
            (f64::from(f32::from_bits(fields.configuration[9])) * f64::from(fields.scale)) as f32;
        if combat.mode == 20 && channels[0][0] != 2 && !matches!(context.parent_state, Some(1 | 76))
        {
            return Err(
                "state23 mode20 requires owned movement and typed leap resource services".into(),
            );
        }
        // Forward speed is unused by all admitted Session modes; mode20's writer
        // remains an explicit refusal until the common movement consumer owns it.
        let state = crate::state23_animation::State {
            class: actor.class,
            mode: combat.mode,
            timer: combat.timer,
            counter: actor.state23_animation_counter,
            forward_speed: 0.,
            channels,
        };
        let mut adapter = State23AnimationAdapter {
            attempted: false,
            session: self,
            source: id,
            services,
        };
        let result = crate::state23_animation::update(state, context, random, &mut adapter);
        let attempted = adapter.attempted;
        let next = match result {
            Ok(next) => next,
            Err(error) => {
                if attempted && let Some(actor) = self.residents.get_mut(&id.serial) {
                    actor.state23_animation_failed = true;
                }
                return Err(error);
            }
        };
        let actor = self
            .residents
            .get_mut(&id.serial)
            .ok_or("animation service removed its callback actor before publication")?;
        combat.mode = next.mode;
        combat.timer = next.timer;
        actor.state23_animation_counter = next.counter;
        actor.channels = Some(next.channels);
        actor.state23_combat = Some(combat);
        Ok(())
    }
    /// Publish the actual controlled physics step, including contact-corrected
    /// AC. Does not set health, channels, flags, readiness or an affine cache.
    pub fn publish_controlled_mecc_motion(
        &mut self,
        id: Id,
        step: u64,
        frame: crate::controlled_mecc::Frame,
    ) -> Result<(), String> {
        frame.validate()?;
        let actor = self.get(id).ok_or("controlled actor absent or foreign")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("controlled fields absent")?;
        if !actor.acquired_construction
            || actor.mecc_physics_present
            || actor.class != 2
            || actor.seed.storage.profile() != Profile::Debug
            || actor.seed.storage.state() != 2
            || fields.class != 2
            || fields.configuration[20] != 2
            || fields.object_flags & 0x8000 != 0
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || self.specification(id).is_none()
            || self.state_resources(id).is_none()
            || fields.scale.to_bits() != frame.scale.to_bits()
            || fields.position.map(f32::to_bits) != frame.before.map(f32::to_bits)
            || step
                != actor
                    .controlled_motion
                    .as_ref()
                    .map_or(Some(1), |m| m.step.checked_add(1))
                    .ok_or("controlled step overflow")?
        {
            return Err(
                "controlled Mecc motion requires contiguous acquired unparented state2 ownership"
                    .into(),
            );
        }
        let previous_history = fields.current;
        let actor = self.residents.get_mut(&id.serial).unwrap();
        let fields = actor.construction_fields.as_mut().unwrap();
        fields.previous = previous_history;
        fields.current = frame.contact_previous;
        fields.position = frame.position;
        fields.angles = frame.angles;
        actor.controlled_motion = Some(crate::controlled_mecc::Owned {
            step,
            frame,
            previous_history,
        });
        Ok(())
    }
    pub fn controlled_mecc_motion(
        &self,
        id: Id,
        step: u64,
    ) -> Result<&crate::controlled_mecc::Owned, String> {
        let actor = self.get(id).ok_or("controlled actor absent or foreign")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("controlled fields absent")?;
        let motion = actor
            .controlled_motion
            .as_ref()
            .ok_or("controlled motion has not been published")?;
        if actor.class != 2
            || fields.class != 2
            || fields.configuration[20] != 2
            || actor.seed.storage.profile() != Profile::Debug
            || !actor.acquired_construction
            || actor.seed.storage.state() != 2
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || fields.object_flags & 0x8000 != 0
            || !motion.current(fields, step)
        {
            return Err("controlled motion is stale or belongs to another step".into());
        }
        Ok(motion)
    }
    pub fn basic_projectile(&self, id: Id) -> Option<&crate::basic_projectile::Metadata> {
        self.get(id)?.basic_projectile.as_ref()
    }
    pub fn enemy_projectile(&self, id: Id) -> Option<&crate::enemy_projectile::Metadata> {
        self.get(id)?.enemy_projectile.as_ref()
    }
    /// Owned callback consumer, not the general projectile-hit wrapper.
    /// Services must be available before calling. On service failure the local
    /// actor is not published, but earlier external effects/RNG are not undone;
    /// callers must not blindly retry a partially executed service sequence.
    pub fn damage_state23(
        &mut self,
        id: Id,
        amount: f32,
        source: Option<Id>,
        context: crate::state23_damage::Context,
        random: &mut crate::audio_parameters::Random,
        services: &mut impl State23Services,
    ) -> Result<crate::state23_damage::Outcome, String> {
        use crate::state23_damage::Source;
        let actor = self
            .get(id)
            .ok_or("damage target is not owned by this ticket")?;
        if actor.seed.storage.state() != 23
            || !matches!(actor.class, 73 | 230)
            || !actor.acquired_construction
            || self.specification(id).is_none()
            || self.state_resources(id).is_none()
        {
            return Err("damage target lacks acquired state23 construction".into());
        }
        let source = source
            .map(|id| {
                let actor = self
                    .get(id)
                    .ok_or("damage source is not owned by this ticket")?;
                if let Some(projectile) = &actor.basic_projectile {
                    return Err(if projectile.launch.is_some() {
                        "launched projectile requires acquired contact/hit dispatch before damage"
                    } else {
                        "constructed projectile requires acquired launch/owner initialization before damage"
                    }.into());
                }
                if actor.enemy_projectile.is_some() {
                    return Err("constructed enemy projectile requires its ordered launch and hit dispatch before damage".into());
                }
                if !actor.acquired_construction
                    || actor.construction_fields.is_none()
                    || self.specification(id).is_none()
                    || self.state_resources(id).is_none()
                {
                    return Err(
                        "damage source requires a complete acquired constructor".to_string()
                    );
                }
                Ok(Source {
                    id,
                    state: u32::from(actor.seed.storage.state()),
                    owner: actor
                        .state23_combat
                        .as_ref()
                        .and_then(|s| s.owner.map(|(id, _)| id)),
                })
            })
            .transpose()?;
        let mut state = self.state23_snapshot(id)?;
        let outcome = crate::state23_damage::apply(
            &mut state,
            amount,
            source,
            context,
            random,
            &mut State23Adapter {
                session: self,
                services,
            },
        )?;
        let actor = self
            .residents
            .get_mut(&id.serial)
            .ok_or("damage service removed its callback actor before publication")?;
        let fields = actor
            .construction_fields
            .as_mut()
            .ok_or("damage service removed its callback fields before publication")?;
        fields.object_flags = state.flags;
        fields.configuration[14] = state.personality;
        fields.control_flags = state.control;
        fields.action = state.action;
        actor.state23_combat = Some(state);
        Ok(outcome)
    }
    /// Build all owned resources before making an actor visible to this session.
    /// Reuse the individual binding guards in an isolated staging session. Failed
    /// construction releases its resources and publishes no lists or identity.
    pub fn construct(&mut self, input: Construction<'_>) -> Result<Id, String> {
        self.construct_with_state_writes(input, &[])
    }
    pub fn wield_binding(&self, id: Id) -> Option<crate::mecc_wield::Binding> {
        self.get(id)?.wield_binding
    }
    /// Read the current owned firing parent/reference without resolving a
    /// parented world pose, advancing ANM, allocating a projectile or consuming
    /// ammunition. Selection alone never acknowledges a firing transaction.
    pub fn mecc_fire_lock(
        &self,
        owner: Id,
        selector: u32,
        definition: &crate::projectile_definition::Definition,
    ) -> Result<crate::mecc_fire_lock::Binding<Id>, String> {
        self.check_wield_owner(owner)?;
        if self.mecc_physical_cycle_failure(owner).is_some() {
            return Err("fire lock owner has a failed physical/primary cycle".into());
        }
        let fields = self
            .get(owner)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap();
        let held = if matches!(selector as u8, 2 | 13 | 24) {
            Vec::new()
        } else {
            self.wield_list()?
        };
        let mut helpers = Vec::new();
        if selector as u8 == 24 {
            for id in self.list(2) {
                let actor = self.get(id).ok_or("fire helper identity absent")?;
                let fields = actor
                    .construction_fields
                    .as_ref()
                    .ok_or("fire helper fields absent")?;
                if actor.class != 991 || fields.object_flags & 0x8000 != 0 {
                    continue;
                }
                let binding = actor
                    .attachment
                    .ok_or("fire helper needs an owned parent binding")?;
                if actor
                    .presentation_parent
                    .is_some_and(|parent| parent != binding.parent)
                {
                    return Err("fire helper parent bindings disagree".into());
                }
                helpers.push(crate::mecc_wield::Child {
                    id,
                    class: actor.class,
                    flags: fields.object_flags,
                    parent: Some(binding.parent),
                    reference: binding.parent_reference as i32,
                });
            }
        }
        crate::mecc_fire_lock::select(
            owner,
            fields.control_flags,
            selector,
            definition,
            &held,
            &helpers,
        )
        .map_err(str::to_owned)
    }
    pub(crate) fn mecc_jet_owner(&self, id: Id) -> Option<crate::mecc_jet_owned::Owner> {
        self.get(id)?.mecc_jet
    }
    pub fn mecc_jet_sequence(&self, id: Id) -> Option<crate::mecc_jet_owned::Id> {
        self.get(id)?.mecc_jet?.current
    }
    pub(crate) fn publish_mecc_jet_owner(
        &mut self,
        id: Id,
        owner: crate::mecc_jet_owned::Owner,
    ) -> Result<(), String> {
        self.get(id).ok_or("jet owner removed during services")?;
        self.residents.get_mut(&id.serial).unwrap().mecc_jet = Some(owner);
        Ok(())
    }
    pub(crate) fn publish_jet_children(
        &mut self,
        id: Id,
        flags: u32,
        boost: bool,
    ) -> Result<(), String> {
        let owner = self.get(id).ok_or("jet parent absent or foreign")?;
        if owner.attachment.is_some()
            || owner.presentation_parent.is_some()
            || !owner.mecc_special.is_some_and(|s| s.player_fields)
        {
            return Err("jet children require actual unmounted acquired player".into());
        }
        let mut writes = vec![];
        for child in self.list(2) {
            let actor = self.get(child).ok_or("jet child list stale")?;
            if !matches!(actor.class, 84 | 85 | 781) {
                continue;
            }
            if actor.mecc_special_child.is_some_and(|b| b.parent == id) {
                if actor.presentation_parent != Some(id) {
                    return Err("jet child binding changed outside its producer".into());
                }
                let fields = actor
                    .construction_fields
                    .as_ref()
                    .ok_or("jet child fields absent")?;
                let hidden = flags & 0x3000 == 0
                    || if boost {
                        actor.class == 85
                    } else {
                        actor.class == 781
                    };
                writes.push((
                    child,
                    if hidden {
                        fields.object_flags | 0x40000000
                    } else {
                        fields.object_flags & !0x40000000
                    },
                ));
            }
        }
        for (child, flags) in writes {
            self.residents
                .get_mut(&child.serial)
                .unwrap()
                .construction_fields
                .as_mut()
                .unwrap()
                .object_flags = flags;
        }
        Ok(())
    }
    pub fn wield_failure(&self, id: Id) -> Option<&str> {
        self.get(id)?.wield_failure.as_deref()
    }
    pub(crate) fn wield_selection(&self, id: Id) -> Result<i32, String> {
        self.mecc_selected_weapon(id)
    }
    pub fn mecc_selected_weapon(&self, id: Id) -> Result<i32, String> {
        let actor = self
            .get(id)
            .ok_or("weapon selection owner absent or foreign")?;
        if actor.class != 2 || actor.seed.storage.profile() != Profile::Debug {
            return Err("weapon selection requires a debug-profile Mecc".into());
        }
        crate::mecc_ammunition::selected(
            actor.seed.storage.state(),
            actor.seed.storage.data().unwrap_or(&[]),
        )
        .map_err(str::to_owned)
    }
    pub fn mecc_ammunition(
        &self,
        id: Id,
        icon: i32,
    ) -> Result<crate::mecc_ammunition::Availability, String> {
        let actor = self.get(id).ok_or("ammunition owner absent or foreign")?;
        if actor.class != 2 || actor.seed.storage.profile() != Profile::Debug {
            return Err("ammunition requires a debug-profile Mecc".into());
        }
        crate::mecc_ammunition::available(
            actor.seed.storage.state(),
            actor.seed.storage.data().unwrap_or(&[]),
            icon,
        )
        .map_err(str::to_owned)
    }
    pub fn consume_mecc_base_pistol(&mut self, id: Id) -> Result<(), String> {
        let selected = self.mecc_selected_weapon(id)?;
        crate::mecc_ammunition::consume_base_pistol(selected).map_err(str::to_owned)
    }
    pub(crate) fn finish_wield_commit(&mut self, id: Id, selected: i32) -> Result<(), String> {
        let actor = self.get(id).ok_or("weapon commit owner removed")?;
        if actor.seed.storage.state() != 2 {
            return Ok(());
        }
        let actor = self.residents.get_mut(&id.serial).unwrap();
        let data = actor
            .seed
            .storage
            .data_mut()
            .ok_or("weapon commit payload absent")?;
        data[0x60..0x64].copy_from_slice(&selected.to_le_bytes());
        actor
            .construction_fields
            .as_mut()
            .ok_or("weapon commit fields absent")?
            .control_flags &= 0xfffe03ff;
        Ok(())
    }
    pub(crate) fn fail_wield_owner(&mut self, id: Id, error: String) {
        if let Some(actor) = self.residents.get_mut(&id.serial) {
            actor.wield_failure = Some(error);
            if let Some(cycle) = &mut actor.mecc_physical_cycle {
                cycle.failure = Some("held-object service failed; cancellation required".into());
            }
        }
    }
    pub(crate) fn check_wield_footprint(&self, id: Id) -> Result<(), String> {
        let data = self
            .get(id)
            .ok_or("held-object owner absent or foreign")?
            .seed
            .storage
            .data()
            .ok_or("held-object owner payload absent")?;
        if data.get(0x19c..0x1a0) != Some(&[0, 0, 0, 0]) {
            return Err("held-object footprint requires its real owned context service".into());
        }
        Ok(())
    }
    pub fn query_mecc_local_target(
        &mut self,
        source: &crate::scene_resources::SceneResources,
        input: crate::mecc_target_owned::Input<'_>,
        terrain: &crate::terrain_queries::StaticTerrainQuery<'_>,
        headers: &mut impl crate::mecc_target_owned::Headers,
    ) -> Result<crate::mecc_target_owned::Output, String> {
        if self
            .get(input.source)
            .is_some_and(|a| a.presentation_parent.is_some())
        {
            return Err("local targeting source requires its unparented owned profile".into());
        }
        crate::mecc_target_owned::run(self, source, input, terrain, headers)
    }
    /// Execute the acquired local query and publish its target relationships,
    /// aim point and reticle flag. Required header/view/geometry providers stay
    /// explicit. This is not admission of local input or the fire callback.
    pub fn scan_mecc_local_target(
        &mut self,
        resources: &crate::scene_resources::SceneResources,
        input: crate::mecc_target_owned::Input<'_>,
        terrain: &crate::terrain_queries::StaticTerrainQuery<'_>,
        headers: &mut impl crate::mecc_target_owned::Headers,
    ) -> Result<crate::mecc_target_owned::Output, String> {
        let source = input.source;
        let epoch = input.vertex_epoch;
        self.check_mecc_target_publication(source)?;
        let output = self.query_mecc_local_target(resources, input, terrain, headers)?;
        self.publish_mecc_target(
            source,
            epoch,
            output.target,
            output.point,
            output.near_terrain,
        )?;
        Ok(output)
    }
    pub fn mecc_local_target(&self, source: Id) -> Option<crate::mecc_target_owned::Published> {
        self.get(source)?.mecc_local_target
    }
    /// Last explicitly scanned local owner and its terrain-proximity flag.
    pub fn mecc_local_target_in_range(&self) -> Option<(Id, bool)> {
        let (owner, in_range) = self.mecc_target_range?;
        self.get(owner)?;
        Some((owner, in_range))
    }
    pub(crate) fn publish_mecc_target_range(
        &mut self,
        source: Id,
        in_range: bool,
    ) -> Result<(), String> {
        self.get(source)
            .ok_or("target range source absent or foreign")?;
        self.mecc_target_range = Some((source, in_range));
        Ok(())
    }
    pub fn mecc_local_reticle(&self) -> Option<(Id, bool)> {
        let owner = self.mecc_reticle_owner?;
        Some((owner, self.mecc_local_target(owner)?.near_terrain))
    }
    fn check_mecc_target_publication(&self, source: Id) -> Result<(), String> {
        let actor = self
            .get(source)
            .ok_or("local target source absent or foreign")?;
        let body = actor
            .seed
            .storage
            .data()
            .ok_or("local target payload absent")?;
        if actor.class != 2
            || actor.seed.storage.state() != 2
            || actor.seed.storage.profile() != Profile::Debug
            || !actor.acquired_construction
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || body.len() != 524
            || [0x14, 0xe0, 0x120]
                .into_iter()
                .any(|at| body[at..at + 4] != [0; 4])
        {
            return Err(
                "local target publication requires its normal owned Mecc and normalized links"
                    .into(),
            );
        }
        Ok(())
    }
    fn publish_mecc_target(
        &mut self,
        source: Id,
        epoch: u32,
        target: Option<Id>,
        point: [f32; 3],
        near_terrain: bool,
    ) -> Result<(), String> {
        self.check_mecc_target_publication(source)?;
        if epoch == 0
            || !point.into_iter().all(f32::is_finite)
            || target.is_some_and(|id| self.get(id).is_none())
        {
            return Err("local target publication has unavailable identity, epoch or point".into());
        }
        // The native prepass discards a self result before writing all three links.
        let target = target.filter(|id| *id != source);
        let actor = self.residents.get_mut(&source.serial).unwrap();
        let body = actor.seed.storage.data_mut().unwrap();
        for (at, value) in [0xe4, 0xe8, 0xec].into_iter().zip(point) {
            body[at..at + 4].copy_from_slice(&value.to_le_bytes());
        }
        actor.mecc_local_target = Some(crate::mecc_target_owned::Published {
            links: [target; 3],
            point,
            near_terrain,
            vertex_epoch: epoch,
        });
        self.mecc_reticle_owner = Some(source);
        self.mecc_target_range = Some((source, false));
        Ok(())
    }
    pub(crate) fn check_target_geometry(&self, id: Id) -> Result<(), String> {
        let actor = self.get(id).ok_or("target actor absent or foreign")?;
        if !actor.acquired_construction || actor.seed.storage.profile() != Profile::Debug {
            return Err("target geometry requires an acquired debug actor".into());
        }
        Ok(())
    }
    pub(crate) fn target_query_metadata(
        &self,
        current: Id,
    ) -> Result<crate::mecc_target_owned::Metadata, String> {
        self.get(current)
            .ok_or("target current player absent or foreign")?;
        let ordered = self.list(0);
        let mut pending = ordered.clone();
        let mut actors = BTreeMap::new();
        let mut identities = BTreeMap::new();
        let key = |id: Id| {
            u32::try_from(id.serial)
                .map_err(|_| "target actor exceeds normalized identity range".to_owned())
        };
        let order = ordered
            .iter()
            .map(|&id| key(id))
            .collect::<Result<Vec<_>, _>>()?;
        let mut at = 0;
        while let Some(&id) = pending.get(at) {
            at += 1;
            let serial = key(id)?;
            if actors.contains_key(&serial) {
                continue;
            }
            let actor = self
                .get(id)
                .ok_or("target hierarchy has a foreign or missing actor")?;
            let fields = actor
                .construction_fields
                .as_ref()
                .ok_or("target list requires actual owned header fields")?;
            let model_present = self.specification(id).is_some();
            let skip = fields.object_flags & 0x8000 != 0 || !model_present || id == current;
            let mut parent = None;
            if !skip {
                for other in [
                    actor.attachment.map(|a| a.parent),
                    actor.wield_binding.map(|a| a.parent),
                    actor.mecc_special_child.map(|a| a.parent),
                ]
                .into_iter()
                .flatten()
                {
                    if parent.is_some_and(|p| p != other) {
                        return Err("target typed parent bindings disagree".into());
                    }
                    parent = Some(other);
                }
                if actor.presentation_parent.is_some_and(|p| Some(p) != parent) {
                    return Err(
                        "target presentation parent lacks an acquired native binding".into(),
                    );
                }
                if let Some(parent) = parent {
                    pending.push(parent);
                }
            }
            identities.insert(serial, id);
            actors.insert(
                serial,
                crate::mecc_target_query::Actor {
                    parent: parent.map(key).transpose()?,
                    flags: fields.object_flags,
                    personality: fields.configuration[14],
                    // query_with obtains these exclusively through the required header service.
                    context: None,
                    word_e4: None,
                    state: u32::from(actor.seed.storage.state()),
                    class: actor.class,
                    model_present,
                    geometry: None,
                },
            );
        }
        Ok(crate::mecc_target_owned::Metadata {
            actors,
            identities,
            order,
        })
    }
    /// Read acquired current context; raw registrations and uncommitted callback
    /// context changes are refused. No class805 special-header value is inferred.
    pub fn mecc_target_header(&self, id: Id) -> Result<crate::mecc_target_query::Header, String> {
        let actor = self
            .get(id)
            .ok_or("target header actor absent or foreign")?;
        if !actor.acquired_construction
            || actor.seed.storage.profile() != Profile::Debug
            || actor.ripper_primary_running
        {
            return Err("target header requires acquired owned construction".into());
        }
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("target header fields absent")?;
        let context = fields.dispatch_context;
        let disagrees = actor.vimp.is_some_and(|v| v.context != context)
            || actor.vimp_chunk.is_some_and(|v| v.context != context)
            || actor
                .mecc_special_child
                .is_some_and(|v| v.context != context)
            || actor.wield_binding.is_some_and(|v| v.context != context)
            || actor
                .basic_projectile
                .as_ref()
                .is_some_and(|v| v.dispatch_context != context)
            || actor.enemy_projectile.as_ref().is_some_and(|v| {
                v.phase == crate::enemy_projectile::Phase::Failed
                    || v.launch
                        .as_ref()
                        .is_some_and(|v| v.dispatch_context != context)
            })
            || actor
                .ripper_primary
                .as_ref()
                .is_some_and(|v| v.failure().is_some() || v.state().entry.context != context);
        if disagrees {
            return Err(
                "target header context is uncommitted or disagrees with owned producer".into(),
            );
        }
        Ok(crate::mecc_target_query::Header {
            context,
            word_e4: None,
        })
    }
    pub fn mecc_shot_failure(&self, id: Id) -> Option<&str> {
        self.get(id)?.mecc_shot_failure.as_deref()
    }
    pub(crate) fn commit_mecc_fire_frame(
        &mut self,
        id: Id,
        fields: crate::actor_fields::Fields,
        payload: Vec<u8>,
    ) -> Result<(), String> {
        self.check_wield_owner(id)?;
        if payload.len() != 524 || fields.class != 2 {
            return Err("fire frame storage/class changed".into());
        }
        let actor = self.residents.get_mut(&id.serial).unwrap();
        actor.construction_fields = Some(fields);
        actor
            .seed
            .storage
            .data_mut()
            .unwrap()
            .copy_from_slice(&payload);
        Ok(())
    }
    pub fn mecc_shot_record(&self) -> Option<crate::mecc_shot::Record> {
        self.mecc_shot_record
    }
    pub(crate) fn fail_mecc_shot(&mut self, id: Id, error: &str) {
        if let Some(actor) = self.residents.get_mut(&id.serial) {
            actor.mecc_shot_failure = Some(error.to_owned());
        }
    }
    pub(crate) fn commit_mecc_shot_record(&mut self, id: Id, counter: u32) -> Result<(), String> {
        self.check_wield_owner(id)?;
        let actor = self.residents.get_mut(&id.serial).unwrap();
        let fields = actor.construction_fields.as_mut().unwrap();
        self.mecc_shot_record = Some(crate::mecc_shot::Record {
            source: id,
            counter,
            position: fields.position,
        });
        fields.response_flags |= 0x20;
        Ok(())
    }
    pub(crate) fn check_wield_owner(&self, id: Id) -> Result<(), String> {
        let actor = self.get(id).ok_or("held-object owner absent or foreign")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("held-object owner fields absent")?;
        if actor.wield_failure.is_some() {
            return Err("held-object owner quarantined; cancellation required".into());
        }
        if !actor.acquired_construction
            || actor.class != 2
            || actor.seed.storage.profile() != Profile::Debug
            || actor.seed.storage.state() != 2
            || !actor.mecc_special.is_some_and(|s| s.player_fields)
            || fields.dispatch_context != 0
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || fields.object_flags & 0x8000 != 0
            || !fields
                .position
                .into_iter()
                .chain([fields.angles[0]])
                .all(f32::is_finite)
        {
            return Err("held-object owner requires acquired cold unparented Mecc context0".into());
        }
        // The acquired cold player constructor supplies context0. Arbitrary
        // live context/type changes are not represented by this producer.
        self.check_wield_footprint(id)
    }
    pub(crate) fn wield_list(&self) -> Result<Vec<crate::mecc_wield::Child<Id>>, String> {
        self.list(6)
            .into_iter()
            .map(|id| {
                let actor = self
                    .get(id)
                    .ok_or("held-object list contains stale actor")?;
                let fields = actor
                    .construction_fields
                    .as_ref()
                    .ok_or("held-object list fields absent")?;
                let (parent, reference) = if let Some(binding) = actor.wield_binding {
                    if actor.presentation_parent != Some(binding.parent)
                        || actor.attachment.is_none_or(|a| {
                            a.parent != binding.parent
                                || a.child_reference != 0
                                || a.parent_reference as i32 != binding.reference
                        })
                    {
                        return Err("held-object owned binding changed outside its producer".into());
                    }
                    (Some(binding.parent), binding.reference)
                } else if let Some(binding) = actor.attachment {
                    (Some(binding.parent), binding.parent_reference as i32)
                } else if actor.presentation_parent.is_some() {
                    return Err("held-object list parent lacks acquired reference binding".into());
                } else {
                    (None, 0)
                };
                Ok(crate::mecc_wield::Child {
                    id,
                    class: actor.class,
                    flags: fields.object_flags,
                    parent,
                    reference,
                })
            })
            .collect()
    }
    pub(crate) fn mark_wield_child(&mut self, id: Id) -> Result<(), String> {
        self.get(id)
            .ok_or("held-object removal actor absent or foreign")?;
        self.residents
            .get_mut(&id.serial)
            .unwrap()
            .construction_fields
            .as_mut()
            .ok_or("held-object removal fields absent")?
            .object_flags |= 0x8000;
        Ok(())
    }
    pub fn vimp_chunk(&self, id: Id) -> Option<crate::vimp_chunks::Chunk> {
        self.get(id)?.vimp_chunk
    }
    /// Execute the real offline state80 callback, retaining completed numeric
    /// writes if the required ground provider fails. No common-loop history or
    /// cleanup is fabricated here; the owner scheduler must supply those phases.
    pub fn advance_vimp_chunk(
        &mut self,
        id: Id,
        environment: crate::vimp_chunks::Environment<'_>,
        ground: impl FnMut([f32; 3]) -> Result<f32, String>,
    ) -> Result<crate::vimp_chunks::Chunk, String> {
        let actor = self.get(id).ok_or("chunk absent or foreign")?;
        if actor.seed.storage.state() != 80
            || !(1117..=1119).contains(&actor.class)
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
        {
            return Err("chunk update class/state/parent outside acquired profile".into());
        }
        let mut chunk = actor.vimp_chunk.ok_or("chunk not source constructed")?;
        let mut fields = actor
            .construction_fields
            .clone()
            .ok_or("chunk fields absent")?;
        let before_number = chunk.number();
        let result = chunk.advance(&mut fields, environment, ground);
        if result.is_err() && chunk.number() == before_number && !chunk.failed() {
            return result.map(|()| chunk);
        }
        let actor = self.residents.get_mut(&id.serial).unwrap();
        actor.vimp_chunk = Some(chunk);
        actor.construction_fields = Some(fields);
        actor.affine_cache = None;
        result.map(|()| chunk)
    }
    /// Actual offline collection body over a selected source-backed chunk.
    /// On error the supplied world is quarantined; completed effects stay owned.
    pub fn collect_vimp_chunk(
        &mut self,
        collector: Id,
        item: Id,
        services: crate::mecc_collection::Services<'_>,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<bool, String> {
        let crate::mecc_collection::Services {
            world,
            progress,
            context,
            catalog,
            current_player,
            now,
            offline,
            audio_key,
            audio,
        } = services;
        if world.failure.is_some() {
            return Err("collection world is quarantined".into());
        }
        let result: Result<bool, String> = (|| {
            if !offline || !now.is_finite() || audio_key == 0 || self.get(world.scope).is_none() {
                return Err("collection requires current owned offline world and audio key".into());
            }
            if let Some(ctx) = context
                && (!ctx.gates.time.is_finite()
                    || ctx.prefix.len() > 31
                    || !ctx.prefix.is_ascii()
                    || ctx.prefix.contains(&0))
            {
                return Err("collection mission context outside acquired profile".into());
            }
            let actor = self
                .get(collector)
                .ok_or("collection actor absent or foreign")?;
            actor
                .construction_fields
                .as_ref()
                .ok_or("collection fields absent")?;
            if actor.class != 2
                || actor.seed.storage.state() != 2
                || !actor.mecc_special.is_some_and(|s| s.player_fields)
                || actor.attachment.is_some()
                || actor.presentation_parent.is_some()
            {
                return Err(
                    "collection requires acquired unparented cold-network Mecc ownership".into(),
                );
            }
            let body = actor
                .seed
                .storage
                .data()
                .ok_or("collection payload absent")?;
            let resource = f32::from_le_bytes(body[0x50..0x54].try_into().unwrap());
            let child = self.get(item).ok_or("collection item absent or foreign")?;
            let chunk = child
                .vimp_chunk
                .ok_or("collection item not source-constructed")?;
            if chunk.failed()
                || child.seed.storage.state() != 80
                || !(1117..=1119).contains(&child.class)
            {
                return Err("collection item changed its acquired class/state".into());
            }
            let child_fields = child
                .construction_fields
                .as_ref()
                .ok_or("collection item fields absent")?;
            if child_fields.object_flags & 0x8000 != 0
                || child_fields.control_flags & 1 == 0
                || !resource.is_finite()
            {
                return Err("collection selection is retired, inactive or nonfinite".into());
            }
            if let Some(player) = current_player {
                let p = self
                    .get(player)
                    .ok_or("collection HUD player absent or foreign")?;
                if p.class != 2
                    || p.seed.storage.state() != 2
                    || !p.mecc_special.is_some_and(|s| s.player_fields)
                {
                    return Err("collection HUD requires acquired offline Mecc player".into());
                }
            }
            if resource >= 20. {
                return Ok(false);
            }
            let next = (f64::from(resource) + f64::from(chunk.amount)) as f32;
            if !next.is_finite() {
                return Err("collection resource overflow".into());
            }
            world.credit(
                chunk.amount,
                progress,
                context,
                catalog,
                &mut self.messages,
                now,
            )?;
            self.residents
                .get_mut(&item.serial)
                .unwrap()
                .construction_fields
                .as_mut()
                .unwrap()
                .object_flags |= 0x8000;
            self.residents
                .get_mut(&collector.serial)
                .unwrap()
                .seed
                .storage
                .data_mut()
                .unwrap()[0x50..0x54]
                .copy_from_slice(&next.to_le_bytes());
            let sound = if (random.next_15() * 100) >> 15 < 50 {
                0x63d
            } else {
                0x63e
            };
            let route = if current_player == Some(collector) {
                crate::audio_sequence::Route::Local
            } else {
                crate::audio_sequence::Route::Attached
            };
            audio.play(sound, audio_key, route, random)?;
            if let Some(player) = current_player {
                let p = self.get(player).unwrap().seed.storage.data().unwrap();
                world.hud = f32::from_le_bytes(p[0x50..0x54].try_into().unwrap());
            }
            Ok(true)
        })();
        if let Err(e) = &result {
            world.failure = Some(e.clone());
        }
        result
    }
    pub(crate) fn construct_herd(&mut self, input: Construction<'_>) -> Result<Id, String> {
        if input.class != 49
            || input.seed.storage.profile() != Profile::Debug
            || input.seed.storage.state() != 17
            || input.seed.storage.data().map(<[u8]>::len) != Some(52)
            || !input.seed.diagnostics.is_empty()
            || input.flags.active != input.fields.object_flags
            || input.flags.personality != input.fields.configuration[14]
        {
            return Err("herd constructor requires actual class49/state17 resources".into());
        }
        let frame = crate::herd_frame::Owned::fresh(&input.fields, &input.specification)?;
        let mut staged = Self::new(self.ticket);
        staged.next = self.next;
        let id = staged.register_animated(
            input.class,
            input.seed,
            input.bindings,
            input.flags,
            input.fields.channels,
        )?;
        staged.bind_specification(id, input.specification)?;
        staged.bind_state_resources(id, input.state_resources)?;
        staged.bind_actor_fields(id, input.fields)?;
        staged
            .residents
            .get_mut(&id.serial)
            .unwrap()
            .acquired_construction = true;
        staged.residents.get_mut(&id.serial).unwrap().herd = Some(Default::default());
        staged.residents.get_mut(&id.serial).unwrap().herd_frame = Some(frame);
        self.publish_staged(staged, id)
    }
    pub fn herd_marker(&self, id: Id) -> Option<crate::herd_markers::Marker> {
        self.get(id)?.herd_marker
    }
    pub fn vimp_death_proxy(&self, id: Id) -> Option<crate::vimp_death_proxy::Proxy> {
        self.get(id)?.vimp_death_proxy
    }
    /// Number of isolated hurt-state invocations; not completed actor frames.
    pub fn vimp_hurt_steps(&self, id: Id) -> Option<u64> {
        self.get(id)?.vimp?;
        Some(self.get(id)?.vimp_hurt_steps)
    }
    pub fn vimp_scalar_steps(
        &self,
        id: Id,
        phase: crate::vimp_living_scalars::Phase,
    ) -> Option<u64> {
        self.get(id)?.vimp?;
        Some(self.get(id)?.vimp_scalar_steps[phase.index()])
    }
    /// Isolated scalar writers in their native relative order: regeneration
    /// precedes state dispatch; movement parameters follow it. Each phase has
    /// its own invocation counter. Neither acknowledges a complete primary.
    pub fn advance_vimp_living_scalars_offline(
        &mut self,
        id: Id,
        phase: crate::vimp_living_scalars::Phase,
        tick: crate::vimp_living_scalars::Tick<'_>,
        globals: &giants_formats::global_data::GlobalData,
    ) -> Result<Option<crate::vimp_living_scalars::Movement>, String> {
        use crate::vimp_living_scalars::{Phase, movement, regenerate};
        use crate::vimp_wander::{put, scalar, word};
        let actor = self.get(id).ok_or("Vimp scalar owner absent or foreign")?;
        let mut vimp = actor.vimp.ok_or("Vimp scalar construction absent")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("Vimp scalar fields absent")?;
        let body = actor
            .seed
            .storage
            .data()
            .ok_or("Vimp scalar payload absent")?;
        if actor.class != 8
            || actor.seed.storage.state() != 21
            || body.len() != 216
            || !actor.acquired_construction
            || !vimp.initialized
            || !matches!(vimp.mode,1..=5|7..=9)
            || vimp.wander_pending
            || actor.vimp_frame_stopped
            || fields.object_flags & 0x80008000 != 0
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || actor.vimp_scalar_steps[phase.index()] != tick.number
            || tick.number == u64::MAX
        {
            return Err("Vimp scalar phase requires a contiguous initialized offline owner".into());
        }
        if !tick.global_delta.is_finite()
            || tick.global_delta < 0.
            || !fields.position.into_iter().all(f32::is_finite)
            || tick.slow_areas.iter().any(|a| {
                !a.factor.is_finite()
                    || a.factor < 0.
                    || !a.radius_squared.is_finite()
                    || a.radius_squared < 0.
                    || !a.center.into_iter().all(f32::is_finite)
            })
        {
            return Err("Vimp scalar time outside finite acquired profile".into());
        }
        let seconds = crate::object_time::seconds(
            tick.global_delta,
            Some(crate::object_time::Sample {
                owner: vimp.context,
                position: fields.position,
            }),
            None,
            tick.slow_areas,
        );
        let result = match phase {
            Phase::Regeneration => {
                vimp.damage = regenerate(
                    vimp.mode,
                    vimp.damage,
                    seconds,
                    globals.scalar(0x964).map_err(|e| e.to_string())?,
                )?;
                None
            }
            Phase::Movement => Some(movement(
                globals,
                vimp.damage,
                fields.scale,
                word(body, 0x74),
                scalar(body, 0x68),
                seconds,
            )?),
        };
        let actor = self.residents.get_mut(&id.serial).unwrap();
        if let Some(m) = result {
            let body = actor.seed.storage.data_mut().unwrap();
            for (at, v) in [
                (0x68, m.speed_68),
                (0x6c, m.parameter_6c),
                (0x70, m.parameter_70),
            ] {
                put(body, at, v.to_bits());
            }
        }
        actor.vimp = Some(vimp);
        actor.vimp_scalar_steps[phase.index()] += 1;
        Ok(result)
    }
    /// Isolated acquired hurt-state writer, reading the owned channel0 flag.
    /// This never acknowledges growth/motion/animation or a complete primary.
    pub fn advance_vimp_hurt_state_offline(
        &mut self,
        id: Id,
        tick: crate::vimp_hurt::Tick<'_>,
        random: &mut crate::audio_parameters::Random,
        sounds: &mut (impl crate::vimp_hurt::Sounds + ?Sized),
    ) -> Result<(), String> {
        use crate::vimp_wander::{put, scalar, word};
        let actor = self.get(id).ok_or("Vimp hurt owner absent or foreign")?;
        let v = actor.vimp.ok_or("Vimp hurt requires source construction")?;
        let f = actor
            .construction_fields
            .as_ref()
            .ok_or("Vimp hurt fields absent")?;
        if actor.class != 8
            || actor.seed.storage.state() != 21
            || !actor.acquired_construction
            || !v.initialized
            || !matches!(v.mode, 2..=4)
            || v.wander_pending
            || actor.vimp_frame_stopped
            || actor.vimp_hurt_steps != tick.number
            || f.object_flags & 0x80008000 != 0
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
        {
            return Err("Vimp hurt requires its unconsumed ordinary offline mode2..4 owner".into());
        }
        if !tick.global_delta.is_finite()
            || tick.global_delta < 0.
            || tick.slow_areas.iter().any(|a| {
                !a.factor.is_finite()
                    || a.factor < 0.
                    || !a.radius_squared.is_finite()
                    || a.radius_squared < 0.
                    || !a.center.into_iter().all(f32::is_finite)
            })
            || !f.position.into_iter().all(f32::is_finite)
        {
            return Err("Vimp hurt time outside finite nonnegative profile".into());
        }
        let body = actor
            .seed
            .storage
            .data()
            .ok_or("Vimp hurt payload absent")?;
        let born = v
            .born
            .ok_or("Vimp hurt requires its actual parent and EC owner")?;
        let parent = self
            .get(born.parent)
            .ok_or("Vimp hurt parent absent or foreign")?;
        if parent.class != 49 || parent.herd.is_none() {
            return Err("Vimp hurt parent is not a source herd".into());
        }
        let parent_body = parent
            .seed
            .storage
            .data()
            .ok_or("Vimp hurt parent payload absent")?;
        let mut state = crate::vimp_hurt::State {
            mode: v.mode,
            delay: born.delay,
            speed_68: scalar(body, 0x68),
        };
        let mut herd = crate::vimp_hurt::Herd {
            word_0c: word(parent_body, 0xc),
            timer_10: scalar(parent_body, 0x10),
            flags_14: word(parent_body, 0x14),
            timer_18: scalar(parent_body, 0x18),
        };
        let finished = actor.channels.ok_or("Vimp hurt channels absent")?[0][8] & 2 != 0;
        let seconds = crate::object_time::seconds(
            tick.global_delta,
            Some(crate::object_time::Sample {
                owner: v.context,
                position: f.position,
            }),
            None,
            tick.slow_areas,
        ) as f32;
        if !seconds.is_finite() {
            return Err("Vimp hurt local time overflow".into());
        }
        let result = crate::vimp_hurt::advance(
            &mut state,
            Some(&mut herd),
            seconds,
            finished,
            random,
            |selector, r| sounds.stop(id, selector, r),
        );
        let actor = self.residents.get_mut(&id.serial).unwrap();
        actor.vimp_hurt_steps += 1;
        let v = actor.vimp.as_mut().unwrap();
        v.mode = state.mode;
        v.born.as_mut().unwrap().delay = state.delay;
        put(
            actor.seed.storage.data_mut().unwrap(),
            0x68,
            state.speed_68.to_bits(),
        );
        if result.is_err() {
            actor.vimp_frame_stopped = true;
        }
        let body = self
            .residents
            .get_mut(&born.parent.serial)
            .unwrap()
            .seed
            .storage
            .data_mut()
            .unwrap();
        put(body, 0xc, herd.word_0c);
        put(body, 0x10, herd.timer_10.to_bits());
        put(body, 0x14, herd.flags_14);
        put(body, 0x18, herd.timer_18.to_bits());
        result
    }
    /// Complete ordinary offline callback for absent source/impact and a
    /// stationary recipient. Real service effects and RNG survive failure;
    /// the owner is quarantined, preventing replay of a partial sequence.
    pub fn damage_stationary_vimp_offline(
        &mut self,
        id: Id,
        kind: u32,
        amount: f32,
        random: &mut crate::audio_parameters::Random,
        services: &mut crate::vimp_damage::Services<'_, '_>,
    ) -> Result<crate::vimp_damage::Outcome, String> {
        use crate::vimp_damage::{DeathEffect, Outcome, choice};
        let actor = self.get(id).ok_or("Vimp damage owner absent or foreign")?;
        let vimp = actor
            .vimp
            .ok_or("Vimp damage requires source construction")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("Vimp fields absent")?;
        if actor.class != 8
            || actor.seed.storage.state() != 21
            || !actor.acquired_construction
            || actor.vimp_frame_stopped
            || fields.object_flags & 0x8000 != 0
        {
            return Err("Vimp damage owner outside acquired live profile".into());
        }
        if vimp.mode == 0 {
            return Ok(Outcome::Ignored);
        }
        let body = actor.seed.storage.data().ok_or("Vimp payload absent")?;
        if body.len() != 216 {
            return Err("Vimp damage payload has the wrong layout".into());
        }
        let word = |at| u32::from_le_bytes(body[at..at + 4].try_into().unwrap());
        let stage = f32::from_bits(word(0x4c));
        let origin_stage = word(0x50);
        let physical_flags = word(0);
        let limit = f32::from_bits(fields.configuration[12]);
        let blood_actor = crate::blood_particles::Actor {
            position: fields.position,
            scale: fields.scale,
            center: f32::from_bits(fields.configuration[22]),
            height: f32::from_bits(fields.configuration[23]),
        };
        if !vimp.initialized
            || vimp.wander_pending
            || fields.position != fields.current
            || ![
                limit,
                vimp.damage,
                amount,
                stage,
                fields.scale,
                blood_actor.center,
                blood_actor.height,
            ]
            .into_iter()
            .chain(fields.position)
            .all(f32::is_finite)
            || limit <= 0.
            || fields.scale <= 0.
            || blood_actor.height < 0.
        {
            return Err("Vimp damage requires initialized stationary finite source fields".into());
        }
        let sum = f64::from(vimp.damage) + f64::from(amount);
        if !(sum as f32).is_finite() {
            return Err("Vimp damage overflow".into());
        }
        if sum < f64::from(limit) && kind != 14 {
            let scale = services.globals.scalar(0xa04).map_err(|e| e.to_string())?;
            if !((f64::from(scale) * f64::from(amount)) as f32).is_finite() {
                return Err("Vimp reaction scale is outside the finite profile".into());
            }
        }
        let parent = vimp.born.map(|b| b.parent);
        if let Some(parent) = parent {
            let a = self.get(parent).ok_or("Vimp parent no longer owned")?;
            if a.class != 49 || a.herd.is_none() {
                return Err("Vimp parent is not an acquired herd".into());
            }
        }
        let position = fields.position;
        let scale = fields.scale;
        let flags = fields.object_flags;
        self.residents
            .get_mut(&id.serial)
            .unwrap()
            .vimp
            .as_mut()
            .unwrap()
            .damage = sum as f32;
        let result: Result<Outcome, String> = (|| {
            if sum < f64::from(limit) {
                let mut count = 0;
                if kind != 14 {
                    // With no source/impact and A0==AC the real impulse helper
                    // reaches its zero-vector branch without changing forces.
                    if !matches!(vimp.mode, 2..=4)
                        && flags & 0x80000000 == 0
                        && physical_flags & 0x40 == 0
                    {
                        let selector = 0x561 + choice(random);
                        services.sounds.attached(selector, id, random)?;
                        self.residents
                            .get_mut(&id.serial)
                            .unwrap()
                            .vimp
                            .as_mut()
                            .unwrap()
                            .mode = 4;
                    }
                    count = services.blood.emit(blood_actor, random)?;
                }
                let selector = 0x55b + choice(random);
                services.sounds.attached(selector, id, random)?;
                return Ok(Outcome::Hurt {
                    blood_particles: count,
                });
            }
            let angle = (f64::from(random.next_15()) * 360. * (1. / 32768.)) as f32;
            services
                .effects
                .death(DeathEffect { position, angle }, random)?;
            let selector = if origin_stage == 1_f32.to_bits() {
                0x607
            } else {
                0x60b
            } + choice(random);
            services.sounds.positional(selector, position, random)?;
            let selector = 0x54f + choice(random);
            services.sounds.positional(selector, position, random)?;
            let mut chunks = Vec::new();
            let mut proxy = None;
            if parent.is_some() && !matches!(kind, 6 | 7) {
                chunks.push(services.chunks.birth(
                    self,
                    crate::vimp_chunks::Birth {
                        position,
                        flags: 0x3c,
                        amount: 1,
                        class: 0,
                        scale,
                    },
                    services.globals,
                    random,
                )?);
                let tier = if stage > 0.5 {
                    2
                } else if stage > 0.2 {
                    1
                } else {
                    0
                };
                proxy = Some(services.proxy.spawn(self, position, tier)?);
                for threshold in [0.2, 0.5] {
                    if stage > threshold {
                        chunks.push(services.chunks.birth(
                            self,
                            crate::vimp_chunks::Birth {
                                position,
                                flags: 0x0c,
                                amount: 1,
                                class: 0,
                                scale,
                            },
                            services.globals,
                            random,
                        )?);
                    }
                }
            }
            let selector = 0x54f + choice(random);
            services.sounds.positional(selector, position, random)?;
            self.residents
                .get_mut(&id.serial)
                .unwrap()
                .construction_fields
                .as_mut()
                .unwrap()
                .object_flags |= 0x8000;
            Ok(Outcome::Died { chunks, proxy })
        })();
        if result.is_err() {
            self.residents
                .get_mut(&id.serial)
                .unwrap()
                .vimp_frame_stopped = true;
        }
        result
    }
    pub(crate) fn construct_vimp_death_proxy(
        &mut self,
        input: Construction<'_>,
        metadata: crate::vimp_death_proxy::Proxy,
    ) -> Result<Id, String> {
        if input.class != 818
            || input.seed.storage.profile() != Profile::Debug
            || input.seed.storage.state() != 78
            || !input.bindings.is_empty()
            || input
                .seed
                .storage
                .data()
                .is_none_or(|d| d.len() != 268 || d.iter().any(|&b| b != 0))
            || !input.seed.diagnostics.is_empty()
            || input.fields.object_flags != input.flags.active
            || input.fields.configuration[14] != input.flags.personality
        {
            return Err("death auxiliary requires source class818/cold78 construction".into());
        }
        let mut staged = Self::new(self.ticket);
        staged.next = self.next;
        let id = staged.register_with_mask(
            input.class,
            input.seed,
            input.bindings,
            input.flags,
            0x400,
        )?;
        let channels = input.fields.channels;
        staged.bind_specification(id, input.specification)?;
        staged.bind_state_resources(id, input.state_resources)?;
        staged.bind_actor_fields(id, input.fields)?;
        let actor = staged.residents.get_mut(&id.serial).unwrap();
        actor.channels = Some(channels);
        actor.construction_fields.as_mut().unwrap().object_flags |= 0x40000000;
        // Only the all-zero cold78 destructor profile is acquired here.
        // It has no live emitter/attachment to release; native lifecycle still
        // releases its old buffer and allocates the zeroed four-byte state84.
        actor.seed.storage.transition_with(84, |state, body| {
            if state == 78 && body.len() == 268 && body.iter().all(|&b| b == 0) {
                Ok(())
            } else {
                Err("nonempty state78 destructor is outside death auxiliary profile".into())
            }
        })?;
        actor.construction_fields.as_mut().unwrap().configuration[20] = 84;
        actor.vimp_death_proxy = Some(metadata);
        actor.acquired_construction = true;
        self.publish_staged(staged, id)
    }
    pub fn herd_markers(&self, id: Id) -> Option<&[Option<Id>]> {
        self.get(id)?.herd.as_ref()?.markers.as_deref()
    }
    pub fn herd_clock(&self, id: Id) -> Option<&crate::herd_generator::Clock> {
        Some(&self.get(id)?.herd.as_ref()?.clock)
    }
    /// Complete ordinary offline herd primary, including its first marker
    /// allocation. Child living updates and world cleanup remain separate.
    pub fn advance_herd_offline(
        &mut self,
        id: Id,
        tick: crate::herd_generator::Tick<'_>,
        factory: &crate::vimp_actor::Factory,
        globals: &giants_formats::global_data::GlobalData,
        random: &mut crate::audio_parameters::Random,
        ground: &mut dyn crate::herd_generator::Ground,
    ) -> Result<Vec<Id>, String> {
        self.herd_vimp_inputs(id)?;
        let state = self.get(id).unwrap().herd.as_ref().unwrap();
        if tick.step != state.clock.steps || state.clock.failure.is_some() {
            return Err("herd callback replay or quarantined owner".into());
        }
        if !tick.global_delta.is_finite()
            || tick.global_delta < 0.
            || tick.slow_areas.iter().any(|a| {
                !a.factor.is_finite()
                    || a.factor < 0.
                    || !a.radius_squared.is_finite()
                    || a.radius_squared < 0.
                    || !a.center.into_iter().all(f32::is_finite)
            })
        {
            return Err("herd clock outside finite nonnegative profile".into());
        }
        for at in [0xbb8, 0xbbc, 0xbc0] {
            globals.scalar(at).map_err(|e| e.to_string())?;
        }
        if !state.initialized {
            self.initialize_herd_markers(id)?;
        }
        self.residents
            .get_mut(&id.serial)
            .unwrap()
            .herd
            .as_mut()
            .unwrap()
            .clock
            .steps += 1;
        let result: Result<Vec<Id>, String> = (|| {
            let actor = self.get(id).unwrap();
            let fields = actor.construction_fields.as_ref().unwrap();
            if fields.object_flags & 0x8000 != 0 {
                return Ok(Vec::new());
            }
            let state = actor.herd.as_ref().unwrap();
            let position = fields.position;
            let action = fields.action;
            let mode = state.clock.mode;
            let delay = state.clock.delay;
            let dt = crate::object_time::seconds(
                tick.global_delta,
                Some(crate::object_time::Sample { owner: 0, position }),
                None,
                tick.slow_areas,
            );
            let actor = self.residents.get_mut(&id.serial).unwrap();
            let body = actor.seed.storage.data_mut().unwrap();
            if mode == 0 && action == 1 {
                let flags = u32::from_le_bytes(body[0x14..0x18].try_into().unwrap()) | 0x20;
                body[0x14..0x18].copy_from_slice(&flags.to_le_bytes());
            }
            if delay == 0. && mode != 0 {
                let timer = f32::from_le_bytes(body[..4].try_into().unwrap());
                let next = f64::from(timer) - dt;
                body[..4].copy_from_slice(&(next as f32).to_le_bytes());
                if next > 0. {
                    return Ok(Vec::new());
                }
                body[..4].copy_from_slice(&2_f32.to_le_bytes());
                // The native list10 scan deliberately includes marked children.
                if self.list(10).into_iter().any(|child| {
                    self.vimp(child)
                        .and_then(|v| v.born)
                        .is_some_and(|b| b.parent == id)
                }) {
                    return Ok(Vec::new());
                }
                self.residents
                    .get_mut(&id.serial)
                    .unwrap()
                    .herd
                    .as_mut()
                    .unwrap()
                    .clock
                    .delay = 5.;
                return Ok(Vec::new());
            }
            let next = f64::from(delay) - dt;
            actor.herd.as_mut().unwrap().clock.delay = next as f32;
            if next > 0. {
                return Ok(Vec::new());
            }
            let count = i32::from_le_bytes(body[8..12].try_into().unwrap());
            let index = u32::from_le_bytes(body[0x2c..0x30].try_into().unwrap()) as usize;
            let mut born = Vec::new();
            for _ in 0..count {
                let marker = self
                    .herd_markers(id)
                    .and_then(|s| s.get(index))
                    .copied()
                    .flatten()
                    .and_then(|m| self.herd_marker(m))
                    .ok_or("herd selected marker missing")?;
                let radius = f32::from_bits(marker.values[1]);
                let mut offsets = [0.; 2];
                for value in &mut offsets {
                    let low = -radius;
                    let range = (f64::from(radius) - f64::from(low)) as f32;
                    *value = (f64::from(random.next_15()) * f64::from(range) / 32768.
                        + f64::from(low)) as f32;
                }
                let z = ground.height(position)?;
                let candidate = [
                    (f64::from(position[0]) + f64::from(offsets[0])) as f32,
                    (f64::from(position[1]) + f64::from(offsets[1])) as f32,
                    z,
                ];
                let child = factory.spawn(
                    self,
                    crate::vimp_actor::Spawn {
                        parent: id,
                        position: candidate,
                        uid: 0,
                    },
                    globals,
                    random,
                )?;
                self.residents
                    .get_mut(&child.serial)
                    .unwrap()
                    .vimp
                    .as_mut()
                    .unwrap()
                    .born
                    .as_mut()
                    .unwrap()
                    .offsets = offsets;
                born.push(child);
            }
            let actor = self.residents.get_mut(&id.serial).unwrap();
            actor.seed.storage.data_mut().unwrap()[..4].copy_from_slice(&2_f32.to_le_bytes());
            let clock = &mut actor.herd.as_mut().unwrap().clock;
            clock.delay = 0.;
            if clock.mode == 0 {
                clock.mode = 1;
            }
            Ok(born)
        })();
        if let Err(e) = &result {
            self.residents
                .get_mut(&id.serial)
                .unwrap()
                .herd
                .as_mut()
                .unwrap()
                .clock
                .failure = Some(e.clone());
        }
        result
    }
    pub(crate) fn construct_herd_marker(
        &mut self,
        input: Construction<'_>,
        marker: crate::herd_markers::Marker,
    ) -> Result<Id, String> {
        self.construct_herd_marker_profile(input, marker, false)
    }
    pub(crate) fn construct_mission_herd_marker(
        &mut self,
        input: Construction<'_>,
        marker: crate::herd_markers::Marker,
    ) -> Result<Id, String> {
        self.construct_herd_marker_profile(input, marker, true)
    }
    fn construct_herd_marker_profile(
        &mut self,
        input: Construction<'_>,
        marker: crate::herd_markers::Marker,
        mission: bool,
    ) -> Result<Id, String> {
        if input.class != 679
            || input.seed.storage.profile() != Profile::Debug
            || input.seed.storage.state() != 0
            || input.seed.storage.data().is_some()
            || !input.seed.diagnostics.is_empty()
            || input.flags.active != input.fields.object_flags
            || input.flags.personality != input.fields.configuration[14]
        {
            return Err("herd marker constructor requires class679/state0 source resources".into());
        }
        let frame = crate::herd_frame::Owned::fresh(&input.fields, &input.specification)?;
        let mut staged = Self::new(self.ticket);
        staged.next = self.next;
        let id = staged.register_animated(
            input.class,
            input.seed,
            input.bindings,
            input.flags,
            input.fields.channels,
        )?;
        staged.bind_specification(id, input.specification)?;
        staged.bind_state_resources(id, input.state_resources)?;
        staged.bind_actor_fields(id, input.fields)?;
        let actor = staged.residents.get_mut(&id.serial).unwrap();
        actor.herd_marker = Some(marker);
        actor.herd_frame = Some(frame);
        actor.acquired_construction = true;
        if mission {
            let fields = actor.construction_fields.as_mut().unwrap();
            fields.mark_mission_placement();
            staged.prepare_actor_affine(id)?;
        }
        self.publish_staged(staged, id)
    }
    /// The actual first-primary marker allocation/selection prefix. It stops
    /// before ownership, clocks, spawning and later herd behavior.
    pub fn initialize_herd_markers(&mut self, id: Id) -> Result<(), String> {
        let actor = self.get(id).ok_or("herd absent or foreign")?;
        let state = actor.herd.as_ref().ok_or("herd not source constructed")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("herd fields absent")?;
        if actor.class != 49
            || actor.seed.storage.state() != 17
            || state.initialized
            || fields.object_flags & 0x8000 != 0
        {
            return Err(
                "herd marker prefix is already initialized or outside its source profile".into(),
            );
        }
        let body = actor.seed.storage.data().unwrap();
        let count = u32::from_le_bytes(body[0x1c..0x20].try_into().unwrap());
        let group = u32::from_le_bytes(body[0x20..0x24].try_into().unwrap());
        if count > 4096 {
            return Err("herd marker count outside bounded profile".into());
        }
        let mut slots = if count == 0 {
            None
        } else {
            Some(vec![None; count as usize])
        };
        let mut found = 0;
        if let Some(slots) = &mut slots {
            for candidate in self.list(0) {
                let a = self.get(candidate).ok_or("herd candidate absent")?;
                let f = a
                    .construction_fields
                    .as_ref()
                    .ok_or("herd candidate fields absent")?;
                if f.configuration[14] & 0x08000000 != 0 && f.action == group {
                    let marker = a
                        .herd_marker
                        .ok_or("eligible herd marker requires its actual field owner")?;
                    if (marker.index as i32) < 0 {
                        return Err("negative herd marker index outside acquired profile".into());
                    }
                    if marker.index < count {
                        slots[marker.index as usize] = Some(candidate);
                        found += 1;
                    }
                }
            }
        }
        let selected = slots
            .as_ref()
            .and_then(|s| s.iter().position(Option::is_some));
        let position = selected.map(|index| {
            self.get(slots.as_ref().unwrap()[index].unwrap())
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap()
                .position
        });
        let actor = self.residents.get_mut(&id.serial).unwrap();
        let fields = actor.construction_fields.as_mut().unwrap();
        if found == 0 {
            fields.object_flags |= 0x8000;
        }
        if let (Some(index), Some(position)) = (selected, position) {
            actor.seed.storage.data_mut().unwrap()[0x2c..0x30]
                .copy_from_slice(&(index as u32).to_le_bytes());
            fields.position = position;
            actor.affine_cache = None;
        }
        let state = actor.herd.as_mut().unwrap();
        state.markers = slots;
        state.initialized = true;
        Ok(())
    }
    pub(crate) fn herd_vimp_inputs(&self, id: Id) -> Result<(u32, u32), String> {
        let actor = self.get(id).ok_or("Vimp parent absent or foreign")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("Vimp parent fields absent")?;
        let body = actor
            .seed
            .storage
            .data()
            .ok_or("Vimp parent state absent")?;
        if !actor.acquired_construction
            || actor.class != 49
            || actor.seed.storage.state() != 17
            || body.len() != 52
            || u32::from_le_bytes(body[4..8].try_into().unwrap()) != 8
            || fields.object_flags & 0x8000 != 0
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
        {
            return Err("Vimp birth requires acquired configured herd parent for class8".into());
        }
        Ok((
            fields.action,
            u32::from_le_bytes(body[0x28..0x2c].try_into().unwrap()),
        ))
    }
    pub fn vimp(&self, id: Id) -> Option<crate::vimp_actor::Cold> {
        self.get(id)?.vimp
    }
    /// Ordinary mode1 prefix through steering commands and heading/look. A successful call
    /// leaves a sealed pending physical phase; it never completes an actor tick.
    pub fn begin_vimp_wander_offline(
        &mut self,
        id: Id,
        tick: crate::vimp_wander::Tick<'_>,
        factory: &crate::vimp_actor::Factory,
        globals: &giants_formats::global_data::GlobalData,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<crate::vimp_wander::PendingPhysics, String> {
        use crate::vimp_wander::{put, scalar, squared, word};
        factory.growth_bounds(self, id)?;
        let actor = self.get(id).ok_or("Vimp wander owner absent or foreign")?;
        let vimp = actor
            .vimp
            .ok_or("Vimp wander requires actual construction")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("Vimp wander fields absent")?;
        let body = actor
            .seed
            .storage
            .data()
            .ok_or("Vimp wander payload absent")?;
        let p = |at| globals.scalar(at).map_err(|e| e.to_string());
        let g = [
            0x91c, 0x968, 0x96c, 0x998, 0x99c, 0x9a4, 0x9bc, 0x994, 0x940, 0x944, 0x948, 0x6dc,
        ]
        .map(p)
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
        if !vimp.initialized
            || vimp.mode != 1
            || vimp.wander_pending
            || actor.vimp_physical.as_ref().is_some_and(|p| {
                !p.view.physics_consumed || !p.view.animation_consumed || p.view.failure.is_some()
            })
            || vimp.damage != 0.
            || fields.object_flags & 0x81008000 != 0
            || actor.attachment.is_some()
            || body.len() != 216
            || word(body, 0) & 0x40 != 0
            || word(body, 0xb0) != 0
            || word(body, 0x74) != 0
            || word(body, 0xa8) != 0
            || scalar(body, 0x58) > 0.
            || g[0] > 0.
        {
            return Err("Vimp wander requires ordinary unhurt mode1 without a pending phase or feeding timer".into());
        }
        if !tick.global_delta.is_finite()
            || tick.global_delta < 0.
            || !tick.control_factor.is_finite()
            || fields.angles[0].abs() > 1_000_000.
            || [0, 1, 3, 4, 5, 6, 7, 10]
                .into_iter()
                .any(|i| !f32::from_bits(fields.configuration[i]).is_finite())
            || [vimp.angular_speed, vimp.previous_angular_speed]
                .into_iter()
                .any(|v| !v.is_finite())
            || [tick.now, tick.noise_time]
                .into_iter()
                .chain(tick.noise_position)
                .any(|v| !v.is_finite())
            || g.iter().any(|v| !v.is_finite())
            || fields
                .position
                .into_iter()
                .chain(fields.angles)
                .any(|v| !v.is_finite() || v.abs() > 1_000_000.)
            || tick.slow_areas.iter().any(|a| {
                !a.factor.is_finite()
                    || a.factor < 0.
                    || !a.radius_squared.is_finite()
                    || a.radius_squared < 0.
                    || !a.center.into_iter().all(f32::is_finite)
            })
        {
            return Err("Vimp wander inputs outside finite acquired profile".into());
        }
        if !self.list(9).is_empty() || fields.control_flags != 0 {
            return Err(
                "Vimp wander requires the separate shockwave/threat response producer".into(),
            );
        }
        let mut prepared = body.to_vec();
        let priority = word(body, 0xd4) as i32;
        let position = fields.position;
        let peers = self.list(10);
        if priority < 5 {
            if scalar(body, 0xc4) > 0. || word(body, 0x8c) != 0 {
                return Err("Vimp linked threat response not acquired".into());
            }
            if priority < 4 {
                for other in self.list(11) {
                    let f = self
                        .get(other)
                        .and_then(|a| a.construction_fields.as_ref())
                        .ok_or("Vimp list11 fields absent")?;
                    if squared(position, f.position) < f64::from(g[3]) * f64::from(g[3]) {
                        return Err("Vimp nearby list11 response requires its real provider".into());
                    }
                }
            }
            for other in &peers {
                let a = self.get(*other).ok_or("Vimp peer absent")?;
                let v = a.vimp.ok_or("Vimp peer requires source owner")?;
                let f = a
                    .construction_fields
                    .as_ref()
                    .ok_or("Vimp peer fields absent")?;
                if priority < 3
                    && matches!(v.mode, 2..=4)
                    && squared(position, f.position) < f64::from(g[4]) * f64::from(g[4])
                {
                    return Err("Vimp peer threat response requires its real provider".into());
                }
            }
            if priority < 1
                && f64::from(tick.now) - f64::from(tick.noise_time) <= 1.
                && scalar(body, 0x3c) != tick.noise_time
            {
                put(&mut prepared, 0x3c, tick.noise_time.to_bits());
                if squared(position, tick.noise_position) < f64::from(g[5]) * f64::from(g[5]) {
                    let level = f64::from(scalar(body, 0x94)) + 1.;
                    if level >= f64::from(g[1]) {
                        return Err("Vimp noise threshold requires threat response".into());
                    }
                    put(&mut prepared, 0x94, (level as f32).to_bits());
                }
            }
        }
        let born = vimp.born.ok_or("Vimp wander requires actual herd birth")?;
        self.herd_vimp_inputs(born.parent)?;
        let slots = self
            .herd_markers(born.parent)
            .ok_or("Vimp marker table absent")?
            .to_vec();
        let current = word(body, 0x98) as usize;
        let marker_id = slots
            .get(current)
            .copied()
            .flatten()
            .ok_or("Vimp target marker absent")?;
        let marker = self
            .herd_marker(marker_id)
            .ok_or("Vimp marker metadata absent")?;
        let target = [scalar(body, 0x40), scalar(body, 0x44), scalar(body, 0x48)];
        let retargeted = squared(position, target) <= 100.;
        if retargeted && f32::from_bits(marker.values[2]) != 0. {
            let count = word(body, 0x9c).wrapping_add(u32::from(word(body, 0xa0) == 0));
            let mut total = 0;
            let mut ahead = 0;
            for peer in &peers {
                if *peer == id {
                    continue;
                }
                let a = self.get(*peer).unwrap();
                if a.vimp
                    .and_then(|v| v.born)
                    .is_some_and(|b| b.parent == born.parent)
                {
                    total += 1;
                    if (word(a.seed.storage.data().unwrap(), 0x9c) as i32) < count as i32
                        && a.attachment.is_none()
                    {
                        ahead += 1;
                    }
                }
            }
            if total != 0
                && (1. - f64::from(ahead) / f64::from(total)) * 100.
                    >= f64::from(f32::from_bits(marker.values[2]))
            {
                return Err("Vimp feeding transition requires its audio/mode providers".into());
            }
        }
        let parent_current = word(
            self.get(born.parent).unwrap().seed.storage.data().unwrap(),
            0x2c,
        ) as usize;
        let next = (1..=slots.len())
            .map(|i| (current + i) % slots.len())
            .find(|i| slots[*i].is_some())
            .ok_or("Vimp route empty")?;
        let next_id = slots[next].unwrap();
        let next_marker = self
            .herd_marker(next_id)
            .ok_or("Vimp next marker unowned")?;
        let parent_selected = if current != parent_current {
            next
        } else {
            parent_current
        };
        let pose = |id| {
            self.get(id)
                .and_then(|a| a.construction_fields.as_ref())
                .map(|f| f.position)
                .ok_or("Vimp marker pose absent")
        };
        let next_position = pose(next_id)?;
        let parent_position =
            pose(slots[parent_selected].ok_or("Vimp parent selected marker absent")?)?;
        self.advance_vimp_growth_offline(
            id,
            crate::vimp_actor::GrowthTick {
                number: vimp.growth_steps,
                global_delta: tick.global_delta,
                slow_areas: tick.slow_areas,
            },
            factory,
            globals,
            random,
        )?;
        let actor = self.get(id).unwrap();
        let mut body = actor.seed.storage.data().unwrap().to_vec();
        let mut fields = actor.construction_fields.as_ref().unwrap().clone();
        let mut vimp = actor.vimp.unwrap();
        for at in [0x3c, 0x94] {
            put(&mut body, at, word(&prepared, at));
        }
        let wide = crate::object_time::seconds(
            tick.global_delta,
            Some(crate::object_time::Sample {
                owner: vimp.context,
                position,
            }),
            None,
            tick.slow_areas,
        );
        let seconds = f64::from(wide as f32);
        let timer = f64::from(scalar(&body, 0xac)) - seconds;
        put(&mut body, 0xac, (timer as f32).to_bits());
        put(&mut body, 0xc0, g[6].to_bits());
        if timer <= 0. {
            fields.configuration[14] &= !0x400;
        }
        let alert = f64::from(scalar(&body, 0x94)) - f64::from(g[2]) * seconds;
        put(
            &mut body,
            0x94,
            (if alert < 0. { 0. } else { alert as f32 }).to_bits(),
        );
        if retargeted {
            if word(&body, 0xa0) == 0 {
                let count = word(&body, 0x9c).wrapping_add(1);
                put(&mut body, 0x9c, count);
            }
            put(&mut body, 0xa0, 0);
            put(&mut body, 0x98, next as u32);
            let (offsets, target) = crate::vimp_wander::target(
                random,
                next_position,
                f32::from_bits(next_marker.values[1]),
            );
            vimp.born.as_mut().unwrap().offsets = offsets;
            for (i, value) in target.into_iter().enumerate() {
                put(&mut body, 0x40 + i * 4, value.to_bits());
            }
        }
        let movement = crate::vimp_living_scalars::movement(
            globals,
            vimp.damage,
            fields.scale,
            word(&body, 0x74),
            scalar(&body, 0x68),
            wide,
        )?;
        for (at, v) in [
            (0x68, movement.speed_68),
            (0x6c, movement.parameter_6c),
            (0x70, movement.parameter_70),
        ] {
            put(&mut body, at, v.to_bits());
        }
        let target = [
            scalar(&body, 0x40),
            scalar(&body, 0x44),
            scalar(&body, 0x48),
        ];
        vimp.commands = crate::vimp_wander::commands(position, fields.angles[0], target);
        put(&mut body, 0x58, 0);
        put(
            &mut body,
            0x5c,
            crate::vimp_wander::interval(random, 0.5, 1.5).to_bits(),
        );
        // D 4aec80/4aef90, before setting the primary-completed bit or moving.
        // The acquired mode1 producer emits zero look/side commands.
        crate::vimp_recovery::orient(&mut fields, &mut vimp, tick)?;
        vimp.wander_pending = true;
        let result = crate::vimp_wander::PendingPhysics {
            commands: vimp.commands,
            retargeted,
        };
        let actor = self.residents.get_mut(&id.serial).unwrap();
        actor
            .seed
            .storage
            .data_mut()
            .unwrap()
            .copy_from_slice(&body);
        actor.construction_fields = Some(fields);
        actor.vimp = Some(vimp);
        if let Some(physical) = actor.vimp_physical.as_mut() {
            physical.rearm(&tick, position, vimp.context);
        } else {
            actor.vimp_physical = Some(crate::vimp_physical::Owned::new(
                &tick,
                position,
                vimp.context,
            ));
        }
        if retargeted {
            let parent = self.residents.get_mut(&born.parent.serial).unwrap();
            put(
                parent.seed.storage.data_mut().unwrap(),
                0x2c,
                parent_selected as u32,
            );
            parent.construction_fields.as_mut().unwrap().position = parent_position;
        }
        Ok(result)
    }
    pub fn vimp_recovery_primaries(&self, id: Id) -> Option<u64> {
        self.get(id)?.vimp?;
        Some(self.get(id)?.vimp_recovery_primaries)
    }
    /// Actual offline primary entered in mode2..4, including zero commands,
    /// common orientation, physical phase and ANM. No world caller/history is
    /// fabricated and a mode1 successor requires its own acquired AI producer.
    pub fn advance_vimp_recovery_primary_offline(
        &mut self,
        id: Id,
        primary: crate::vimp_recovery::Primary<'_>,
        factory: &crate::vimp_actor::Factory,
        globals: &giants_formats::global_data::GlobalData,
        services: &mut crate::vimp_recovery::Services<'_>,
    ) -> Result<(), String> {
        use crate::vimp_wander::{scalar, word};
        factory.growth_bounds(self, id)?;
        let actor = self
            .get(id)
            .ok_or("Vimp recovery owner absent or foreign")?;
        let v = actor.vimp.ok_or("Vimp recovery construction absent")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("Vimp recovery fields absent")?;
        let body = actor
            .seed
            .storage
            .data()
            .ok_or("Vimp recovery body absent")?;
        let tick = primary.tick;
        if !actor.acquired_construction
            || actor.seed.storage.state() != 21
            || body.len() != 216
            || !v.initialized
            || !matches!(v.mode, 2..=4)
            || actor.vimp_recovery_primaries != primary.number
            || primary.number == u64::MAX
            || actor.vimp_frame_stopped
            || v.wander_pending
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || actor.vimp_physical.as_ref().is_some_and(|p| {
                !p.view.physics_consumed || !p.view.animation_consumed || p.view.failure.is_some()
            })
            || fields.object_flags & 0x81008000 != 0
            || word(body, 0) & 0x40 != 0
            || word(body, 0xb0) != 0
            || word(body, 0xa8) != 0
            || word(body, 0x74) != 0
        {
            return Err(
                "Vimp recovery requires contiguous ordinary dry mode2..4 without a pending phase"
                    .into(),
            );
        }
        if fields.control_flags != 0 || !self.list(9).is_empty() {
            return Err(
                "Vimp recovery controlled-reference/threat provider is not acquired".into(),
            );
        }
        if !tick.global_delta.is_finite()
            || tick.global_delta < 0.
            || !tick.control_factor.is_finite()
            || ![v.angular_speed, v.previous_angular_speed]
                .into_iter()
                .all(f32::is_finite)
            || fields
                .position
                .into_iter()
                .chain(fields.angles)
                .any(|x| !x.is_finite() || x.abs() > 1_000_000.)
            || [0, 1, 3, 4, 5, 6, 7, 10]
                .into_iter()
                .any(|i| !f32::from_bits(fields.configuration[i]).is_finite())
            || [0x68, 0x6c, 0x70]
                .into_iter()
                .any(|at| !scalar(body, at).is_finite())
            || tick.slow_areas.iter().any(|a| {
                !a.factor.is_finite()
                    || a.factor < 0.
                    || !a.radius_squared.is_finite()
                    || a.radius_squared < 0.
                    || a.center.into_iter().any(|x| !x.is_finite())
            })
        {
            return Err("Vimp recovery time/orientation outside finite acquired profile".into());
        }
        let result = (|| {
            use crate::vimp_living_scalars::{Phase, Tick};
            let number = self.vimp_scalar_steps(id, Phase::Regeneration).unwrap();
            self.advance_vimp_living_scalars_offline(
                id,
                Phase::Regeneration,
                Tick {
                    number,
                    global_delta: tick.global_delta,
                    slow_areas: tick.slow_areas,
                },
                globals,
            )?;
            self.advance_vimp_hurt_state_offline(
                id,
                crate::vimp_hurt::Tick {
                    number: self.vimp_hurt_steps(id).unwrap(),
                    global_delta: tick.global_delta,
                    slow_areas: tick.slow_areas,
                },
                services.animation.random,
                services.sounds,
            )?;
            let number = self.vimp_scalar_steps(id, Phase::Movement).unwrap();
            self.advance_vimp_living_scalars_offline(
                id,
                Phase::Movement,
                Tick {
                    number,
                    global_delta: tick.global_delta,
                    slow_areas: tick.slow_areas,
                },
                globals,
            )?;
            let actor = self.get(id).unwrap();
            let mut fields = actor.construction_fields.as_ref().unwrap().clone();
            let mut vimp = actor.vimp.unwrap();
            let position = fields.position;
            vimp.commands = [0; 8];
            crate::vimp_recovery::orient(&mut fields, &mut vimp, tick)?;
            vimp.wander_pending = true;
            let actor = self.residents.get_mut(&id.serial).unwrap();
            actor.construction_fields = Some(fields);
            actor.vimp = Some(vimp);
            if let Some(physical) = actor.vimp_physical.as_mut() {
                physical.rearm(&tick, position, vimp.context);
            } else {
                actor.vimp_physical = Some(crate::vimp_physical::Owned::new(
                    &tick,
                    position,
                    vimp.context,
                ));
            }
            self.advance_vimp_physical_offline(id, factory, globals, &mut *services.ground)?;
            self.advance_vimp_animation_offline(
                id,
                factory,
                globals,
                crate::vimp_animation::Services {
                    blending: services.animation.blending,
                    contexts: services.animation.contexts,
                    random: services.animation.random,
                    stamp: services.animation.stamp,
                },
            )?;
            self.residents
                .get_mut(&id.serial)
                .unwrap()
                .vimp
                .as_mut()
                .unwrap()
                .wander_pending = false;
            Ok(())
        })();
        let actor = self.residents.get_mut(&id.serial).unwrap();
        match result {
            Ok(()) => {
                actor.vimp_recovery_primaries += 1;
                Ok(())
            }
            Err(error) => {
                actor.vimp_frame_stopped = true;
                Err(error)
            }
        }
    }
    pub fn vimp_physical(&self, id: Id) -> Option<&crate::vimp_physical::View> {
        self.get(id)?.vimp_physical.as_ref().map(|p| &p.view)
    }
    /// Consume the time-sealed physical phase. ANM is still pending afterward.
    /// Any attempted ground-provider failure quarantines this owner against replay.
    pub fn advance_vimp_physical_offline(
        &mut self,
        id: Id,
        factory: &crate::vimp_actor::Factory,
        globals: &giants_formats::global_data::GlobalData,
        ground: impl FnMut([f32; 2]) -> Result<crate::physical_ground::Ground, &'static str>,
    ) -> Result<(), String> {
        factory.growth_bounds(self, id)?;
        let actor = self
            .get(id)
            .ok_or("Vimp physical owner absent or foreign")?;
        let vimp = actor.vimp.ok_or("Vimp physical construction absent")?;
        if actor.attachment.is_some() || actor.seed.storage.state() != 21 {
            return Err("Vimp physical phase requires its unparented state21 body".into());
        }
        let mut owned = actor
            .vimp_physical
            .clone()
            .ok_or("Vimp sealed time/physical phase absent")?;
        if owned.view.physics_consumed || owned.view.failure.is_some() {
            return Err("Vimp physical phase consumed or quarantined".into());
        }
        let result = owned.advance(
            actor
                .construction_fields
                .as_ref()
                .ok_or("Vimp physical fields absent")?
                .clone(),
            actor
                .seed
                .storage
                .data()
                .ok_or("Vimp physical payload absent")?
                .to_vec(),
            vimp,
            globals,
            ground,
        );
        let actor = self.residents.get_mut(&id.serial).unwrap();
        match result {
            Ok((fields, body)) => {
                actor.affine_cache = owned.view.affine.map(|value| OwnedAffine {
                    value,
                    pose: affine_pose_words(&fields),
                    external_orientation: fields.object_flags & 0x10000 != 0,
                });
                actor.construction_fields = Some(fields);
                actor
                    .seed
                    .storage
                    .data_mut()
                    .unwrap()
                    .copy_from_slice(&body);
                actor.vimp_physical = Some(owned);
                Ok(())
            }
            Err(error) => {
                if let Some((fields, body)) = owned.partial.take() {
                    actor.construction_fields = Some(fields);
                    actor
                        .seed
                        .storage
                        .data_mut()
                        .unwrap()
                        .copy_from_slice(&body);
                }
                owned.view.failure = Some(error.clone());
                actor.vimp_physical = Some(owned);
                Err(error)
            }
        }
    }
    pub fn vimp_frames(&self, id: Id) -> Option<u64> {
        Some(self.get(id)?.vimp_frames)
    }
    pub(crate) fn vimp_primary_history(&mut self, id: Id) -> Result<(), String> {
        let actor = self
            .residents
            .get_mut(&id.serial)
            .ok_or("Vimp primary history owner absent")?;
        let vimp = actor
            .vimp
            .as_mut()
            .ok_or("Vimp primary history construction absent")?;
        let fields = actor
            .construction_fields
            .as_mut()
            .ok_or("Vimp primary history fields absent")?;
        if fields.object_flags & 0xc000 != 0
            || (fields.configuration[14] & 0x10 != 0 && vimp.commands == [0; 8])
        {
            return Err("Vimp common primary not admitted; fallback services required".into());
        }
        fields.configuration[14] &= !0x10;
        vimp.previous_angular_speed = vimp.angular_speed;
        if let Some(physical) = actor.vimp_physical.as_mut() {
            physical.view.previous_model_rates = physical.view.model_rates;
        }
        Ok(())
    }
    /// Seal a Vimp's position history during the separate world history pass.
    /// No clock, AI, physics, ANM or sound is advanced. The caller must still
    /// establish native world admission and visit order; this is not readiness.
    pub fn prepare_vimp_world_history(
        &mut self,
        id: Id,
        number: u64,
        factory: &crate::vimp_actor::Factory,
    ) -> Result<(), String> {
        factory.growth_bounds(self, id)?;
        self.validate_vimp_world_entry(id, number, false)?;
        let actor = self.residents.get_mut(&id.serial).unwrap();
        let fields = actor
            .construction_fields
            .as_mut()
            .ok_or("Vimp world fields absent")?;
        if actor.vimp_prepared_history.is_some() || fields.object_flags & 0xc000 != 0 {
            return Err("Vimp history already prepared or actor inactive".into());
        }
        fields.advance_position_history();
        actor.vimp_prepared_history = Some(number);
        Ok(())
    }

    fn validate_vimp_world_entry(&self, id: Id, number: u64, newborn: bool) -> Result<(), String> {
        let actor = self.get(id).ok_or("Vimp world owner absent or foreign")?;
        let vimp = actor.vimp.ok_or("Vimp world construction absent")?;
        let admitted = if newborn {
            !vimp.initialized
                && vimp.born.is_some()
                && vimp.mode == 0
                && actor.vimp_frames == 0
                && actor.attachment.is_none()
                && actor.presentation_parent.is_none()
                && actor
                    .construction_fields
                    .as_ref()
                    .is_some_and(|f| f.object_flags & 0xc000 == 0)
        } else {
            vimp.initialized
        };
        if !admitted
            || number != actor.vimp_frames
            || number == u64::MAX
            || actor.world_tick_running
            || actor.vimp_frame_stopped
            || actor
                .world_tick
                .as_ref()
                .is_some_and(|t| t.phase() != crate::actor_world_tick::Phase::Finished)
            || (actor.vimp_frames == 0 && (vimp.wander_pending || actor.vimp_physical.is_some()))
            || (actor.vimp_frames != 0
                && actor
                    .vimp_physical
                    .as_ref()
                    .is_none_or(|p| !p.view.animation_consumed || p.view.failure.is_some()))
        {
            return Err("Vimp world iteration out of order, incomplete or stopped".into());
        }
        Ok(())
    }

    /// Consume separately prepared history without copying it again during the
    /// primary pass. Missing preflight services retain the pending history;
    /// callback execution consumes it even when a later service fails.
    pub fn advance_vimp_world_after_history(
        &mut self,
        id: Id,
        frame: crate::vimp_frame::Frame<'_>,
        factory: &crate::vimp_actor::Factory,
        globals: &giants_formats::global_data::GlobalData,
        services: &mut crate::vimp_frame::Services<'_>,
    ) -> Result<crate::actor_world_tick::Outcome, String> {
        self.advance_vimp_world_with_history(
            id,
            frame,
            factory,
            globals,
            services,
            crate::vimp_frame::History::Prepared,
        )
    }

    /// First common-list callback after an actual owned herd birth. The
    /// constructor history is retained: no preceding world-history pass is
    /// invented. Initialization and living continuation share this callback,
    /// RNG and failure token. Outer world admission remains the caller's duty.
    pub fn advance_vimp_newborn_world(
        &mut self,
        id: Id,
        frame: crate::vimp_frame::Frame<'_>,
        factory: &crate::vimp_actor::Factory,
        globals: &giants_formats::global_data::GlobalData,
        services: &mut crate::vimp_frame::Services<'_>,
    ) -> Result<crate::actor_world_tick::Outcome, String> {
        self.advance_vimp_world_with_history(
            id,
            frame,
            factory,
            globals,
            services,
            crate::vimp_frame::History::Constructor,
        )
    }

    /// One common-list Vimp iteration with retained physics, supplied admission/time
    /// and acquired position/rate histories. This is not whole-world scheduling.
    pub fn advance_vimp_world_offline(
        &mut self,
        id: Id,
        frame: crate::vimp_frame::Frame<'_>,
        factory: &crate::vimp_actor::Factory,
        globals: &giants_formats::global_data::GlobalData,
        services: &mut crate::vimp_frame::Services<'_>,
    ) -> Result<crate::actor_world_tick::Outcome, String> {
        self.advance_vimp_world_with_history(
            id,
            frame,
            factory,
            globals,
            services,
            crate::vimp_frame::History::Combined,
        )
    }

    fn advance_vimp_world_with_history(
        &mut self,
        id: Id,
        frame: crate::vimp_frame::Frame<'_>,
        factory: &crate::vimp_actor::Factory,
        globals: &giants_formats::global_data::GlobalData,
        services: &mut crate::vimp_frame::Services<'_>,
        history: crate::vimp_frame::History,
    ) -> Result<crate::actor_world_tick::Outcome, String> {
        factory.growth_bounds(self, id)?;
        self.validate_vimp_world_entry(
            id,
            frame.number,
            history == crate::vimp_frame::History::Constructor,
        )?;
        let actor = self.get(id).ok_or("Vimp world owner absent or foreign")?;
        let vimp = actor.vimp.ok_or("Vimp world construction absent")?;
        if matches!(vimp.mode, 2..=4) && services.recovery_sounds.is_none() {
            return Err("Vimp recovery sound-stop service absent".into());
        }
        if actor.vimp_prepared_history
            != (history == crate::vimp_frame::History::Prepared).then_some(frame.number)
        {
            return Err("Vimp world history route or frame differs from preparation".into());
        }
        let actor = self.residents.get_mut(&id.serial).unwrap();
        let fields = actor
            .construction_fields
            .as_mut()
            .ok_or("Vimp world fields absent")?;
        if history == crate::vimp_frame::History::Combined {
            fields.advance_position_history();
        }
        actor.vimp_prepared_history = None;
        actor.vimp.as_mut().unwrap().wander_pending = false;
        actor.world_tick_running = true;
        let mut token = crate::actor_world_tick::Tick::new();
        let mut adapter = crate::vimp_frame::Adapter {
            factory,
            globals,
            frame,
            services,
        };
        let result = token.run(&mut ActorWorldAdapter {
            session: self,
            source: id,
            services: &mut adapter,
        });
        let actor = self
            .residents
            .get_mut(&id.serial)
            .ok_or("Vimp world owner removed")?;
        actor.world_tick_running = false;
        actor.world_tick = Some(token);
        if let Ok(outcome) = result {
            actor.vimp_frames += 1;
            actor.vimp_frame_stopped =
                outcome != crate::actor_world_tick::Outcome::UpdatedByPrimary;
        }
        result
    }
    pub fn vimp_event_kinds(&self, slot: u8) -> Result<Vec<u32>, String> {
        self.vimp_events
            .as_ref()
            .ok_or("Vimp event bank absent")?
            .kinds(slot)
    }
    /// Isolated hurt selector with the owned descriptors and shared random.
    /// Clocks, poses, events, physical phases and full frame remain separate.
    pub fn select_vimp_hurt_animation_offline(
        &mut self,
        id: Id,
        blending: crate::animation_channels::Blending,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<(), String> {
        let actor = self
            .get(id)
            .ok_or("Vimp hurt ANM owner absent or foreign")?;
        let v = actor.vimp.ok_or("Vimp hurt ANM construction absent")?;
        let f = actor
            .construction_fields
            .as_ref()
            .ok_or("Vimp hurt ANM fields absent")?;
        if actor.class != 8
            || actor.seed.storage.state() != 21
            || !actor.acquired_construction
            || !v.initialized
            || !matches!(v.mode, 2..=4)
            || v.wander_pending
            || actor.vimp_frame_stopped
            || f.object_flags & 0x80008000 != 0
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
        {
            return Err("Vimp hurt ANM requires its initialized offline mode2..4 owner".into());
        }
        let mut channels = actor.channels.ok_or("Vimp hurt ANM channels absent")?;
        let specification = self
            .specification(id)
            .ok_or("Vimp hurt ANM resources absent")?;
        if let Some(request) = crate::vimp_animation::hurt_request(
            v.mode,
            &channels,
            |slot| specification.animations.contains_key(&slot),
            random,
        )? {
            crate::animation_channels::select_primary(&mut channels, request, blending)?;
        }
        let actor = self.residents.get_mut(&id.serial).unwrap();
        actor.channels = Some(channels);
        actor.construction_fields.as_mut().unwrap().channels = channels;
        Ok(())
    }
    /// Real ordinary ANM phase after owned physics, with a real deferred FX pool.
    /// The caller/world phase is still sealed; no history or global tick is fabricated.
    pub fn advance_vimp_animation_offline(
        &mut self,
        id: Id,
        factory: &crate::vimp_actor::Factory,
        globals: &giants_formats::global_data::GlobalData,
        services: crate::vimp_animation::Services<'_>,
    ) -> Result<(), String> {
        let crate::vimp_animation::Services {
            blending,
            contexts,
            random,
            stamp,
        } = services;
        factory.growth_bounds(self, id)?;
        if !factory.owns_effect_contexts(contexts) {
            return Err("Vimp ANM effect namespace differs from source registration".into());
        }
        let actor = self.get(id).ok_or("Vimp ANM owner absent or foreign")?;
        let physical = actor
            .vimp_physical
            .as_ref()
            .ok_or("Vimp ANM physical phase absent")?;
        let mut vimp = actor.vimp.ok_or("Vimp ANM construction absent")?;
        if !vimp.wander_pending
            || !physical.view.physics_consumed
            || physical.view.animation_consumed
            || physical.view.failure.is_some()
            || actor.attachment.is_some()
        {
            return Err("Vimp ANM requires its unconsumed physical phase".into());
        }
        let seconds = physical.animation_seconds;
        let mut fields = actor
            .construction_fields
            .as_ref()
            .ok_or("Vimp ANM fields absent")?
            .clone();
        let body = actor.seed.storage.data().ok_or("Vimp ANM payload absent")?;
        let specification = self.specification(id).unwrap().clone();
        let mut references = self.specifications.get(&id.serial).unwrap().0.clone();
        let mut bank = self
            .vimp_events
            .clone()
            .unwrap_or_else(|| crate::mecc_animation_events::Bank::new(specification.clone()));
        if !bank.belongs_to(&specification) {
            return Err("Vimp event bank class resources differ".into());
        }
        let mut channels = actor.channels.ok_or("Vimp ANM channels absent")?;
        let prepared = (|| -> Result<_, String> {
            if let Some(request) = crate::vimp_animation::select(
                &fields,
                vimp,
                &physical.view,
                body,
                &channels,
                &specification,
                globals,
                random,
            )? {
                crate::animation_channels::select_primary(&mut channels, request, blending)?;
            }
            let mut durations = vec![None; 256];
            for (&slot, descriptor) in &specification.animations {
                durations[usize::from(slot)] = Some(descriptor.template.duration);
            }
            let tick = crate::animation_channels::advance(
                &mut channels,
                &durations,
                physical.view.model_rates,
                seconds,
                crate::arithmetic::Precision::Double53,
            )?;
            if tick.missing != 0 {
                return Err("Vimp ANM source descriptor absent".into());
            }
            if tick.processed != 0 {
                vimp.animation_stamp = stamp.advance();
                references.update_current(&channels, vimp.animation_stamp)?;
            }
            Ok(tick)
        })();
        let result = prepared.and_then(|tick| {
            if tick.processed != 0 {
                bank.dispatch(
                    crate::mecc_animation_events::Input {
                        owner: id,
                        fields: &fields,
                        channels: &channels,
                        references: references.matrices(),
                        affine: Some(physical.view.affine.ok_or("Vimp ANM affine absent")?.matrix),
                        // Actual material helper returns null when support224 is0.
                        // Ordinary Vimp physics enforces the source400 object-walk gate
                        // and refuses any support: this is acquired absence, not a default.
                        material: crate::mecc_animation_events::Material::Acquired(None),
                    },
                    contexts,
                    random,
                )?;
            }
            Ok(())
        });
        // Completed clocks/references/event visits survive a late service error;
        // the quarantine prevents replay of an already consumed random request.
        fields.channels = channels;
        let actor = self.residents.get_mut(&id.serial).unwrap();
        actor.channels = Some(channels);
        actor.vimp = Some(vimp);
        actor.construction_fields = Some(fields);
        self.specifications.get_mut(&id.serial).unwrap().0 = references;
        self.vimp_events = Some(bank);
        let physical = actor.vimp_physical.as_mut().unwrap();
        match result {
            Ok(()) => {
                physical.view.animation_consumed = true;
                Ok(())
            }
            Err(error) => {
                physical.view.failure = Some(error.clone());
                Err(error)
            }
        }
    }
    /// Actual offline maturation helper. Invocation order in the full living
    /// primary and all surrounding movement/animation phases remain separate.
    pub fn advance_vimp_growth_offline(
        &mut self,
        id: Id,
        tick: crate::vimp_actor::GrowthTick<'_>,
        factory: &crate::vimp_actor::Factory,
        globals: &giants_formats::global_data::GlobalData,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<(), String> {
        let bounds = factory.growth_bounds(self, id)?;
        let actor = self.get(id).ok_or("Vimp growth owner absent or foreign")?;
        let mut vimp = actor.vimp.ok_or("Vimp growth requires source owner")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("Vimp growth fields absent")?;
        if !vimp.initialized
            || vimp.wander_pending
            || tick.number != vimp.growth_steps
            || fields.object_flags & 0x8000 != 0
            || actor.seed.storage.state() != 21
        {
            return Err(
                "Vimp growth requires initialized live owner and contiguous invocation".into(),
            );
        }
        if !tick.global_delta.is_finite()
            || tick.global_delta < 0.
            || tick.slow_areas.iter().any(|a| {
                !a.factor.is_finite()
                    || a.factor < 0.
                    || !a.radius_squared.is_finite()
                    || a.radius_squared < 0.
                    || !a.center.into_iter().all(f32::is_finite)
            })
        {
            return Err("Vimp growth time outside finite nonnegative profile".into());
        }
        let mut body = actor
            .seed
            .storage
            .data()
            .ok_or("Vimp growth payload absent")?
            .to_vec();
        let word = |body: &[u8], at| u32::from_le_bytes(body[at..at + 4].try_into().unwrap());
        let put = |body: &mut [u8], at, value: u32| {
            body[at..at + 4].copy_from_slice(&value.to_le_bytes())
        };
        let mut fields = fields.clone();
        let mut channels = actor.channels.ok_or("Vimp growth channels absent")?;
        let mut rng = *random;
        if fields.action != 1 && word(&body, 0x60) == 0 {
            let parameter = |at| globals.scalar(at).map_err(|e| e.to_string());
            let health = [parameter(0xbbc)?, parameter(0xbc0)?];
            let rates = [
                parameter(0x970)?,
                parameter(0x9ac)?,
                parameter(0x9b0)?,
                parameter(0x9b4)?,
                parameter(0x9b8)?,
            ];
            if health.into_iter().chain(rates).any(|v| !v.is_finite()) {
                return Err("Vimp growth parameters outside finite source profile".into());
            }
            let dt = crate::object_time::seconds(
                tick.global_delta,
                Some(crate::object_time::Sample {
                    owner: vimp.context,
                    position: fields.position,
                }),
                None,
                tick.slow_areas,
            );
            let timer = f64::from(f32::from_bits(word(&body, 0x54))) - dt;
            put(&mut body, 0x54, (timer as f32).to_bits());
            if timer <= 0. {
                let stage = word(&body, 0x4c);
                if stage == 0.2_f32.to_bits() {
                    put(&mut body, 0x4c, 0.5_f32.to_bits());
                    fields.configuration[12] = health[0].to_bits();
                } else if stage == 0.5_f32.to_bits() {
                    put(&mut body, 0x4c, 1_f32.to_bits());
                    fields.configuration[12] = health[1].to_bits();
                }
                let timer = (f64::from(rng.next_15()) * 20. / 32768. + 10.) as f32;
                put(&mut body, 0x54, timer.to_bits());
            }
            let current = f32::from_bits(word(&body, 0x50));
            let stage = f32::from_bits(word(&body, 0x4c));
            if current < stage {
                let next = f64::from(current) + dt * 0.5;
                put(&mut body, 0x50, (next as f32).to_bits());
                if next >= f64::from(stage) {
                    put(&mut body, 0x50, stage.to_bits());
                    // Native refresh precedes the new geometric scale, and
                    // the direct helper does not apply the locked-radius gate.
                    vimp.radius = bounds.refresh(
                        vimp.radius,
                        fields.scale,
                        0,
                        crate::arithmetic::Precision::Double53,
                    )?;
                }
                fields.scale = (f64::from(f32::from_bits(word(&body, 0x50)))
                    * f64::from(vimp.template_scale)) as f32;
                let rate = if vimp.mode != 1 {
                    0.
                } else if word(&body, 0) & 0x40 != 0 {
                    rates[0]
                } else if vimp.damage > 0. {
                    rates[1]
                } else {
                    rates[match word(&body, 0x74) {
                        0 => 2,
                        1 => 3,
                        _ => 4,
                    }]
                };
                channels[0][5] = ((f64::from(rate) * f64::from(fields.scale)) as f32).to_bits();
            }
        }
        vimp.growth_steps += 1;
        let actor = self.residents.get_mut(&id.serial).unwrap();
        actor
            .seed
            .storage
            .data_mut()
            .unwrap()
            .copy_from_slice(&body);
        fields.channels = channels;
        actor.construction_fields = Some(fields);
        actor.channels = Some(channels);
        actor.vimp = Some(vimp);
        *random = rng;
        Ok(())
    }
    /// First offline Vimp primary through physical setup and initial target.
    /// It stops before the living-mode dispatch; it does not complete a frame.
    pub fn initialize_vimp_offline(
        &mut self,
        id: Id,
        globals: &giants_formats::global_data::GlobalData,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<(), String> {
        let actor = self
            .get(id)
            .ok_or("Vimp initialization owner absent or foreign")?;
        let mut vimp = actor.vimp.ok_or("Vimp requires source construction")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("Vimp fields absent")?;
        if vimp.initialized || vimp.mode != 0 || fields.object_flags & 0x8000 != 0 {
            return Err("Vimp initialization replay or outside cold profile".into());
        }
        let mut body = actor
            .seed
            .storage
            .data()
            .ok_or("Vimp payload absent")?
            .to_vec();
        if actor.class != 8
            || actor.seed.storage.state() != 21
            || body.len() != 216
            || body[0xa8..0xac] != [0; 4]
        {
            return Err("Vimp initialization outside acquired source profile".into());
        }
        let scalar = |at| globals.scalar(at).map_err(|e| e.to_string());
        let parameters = [
            0x9c4, 0x9c8, 0x9d0, 0x9cc, 0xa08, 0xa0c, 0x994, 0x940, 0x944, 0x948, 0x91c, 0x920,
        ]
        .map(scalar)
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
        if parameters.iter().any(|v| !v.is_finite()) {
            return Err("Vimp parameters outside finite source profile".into());
        }
        let target = if let Some(born) = vimp.born {
            self.herd_vimp_inputs(born.parent)?;
            let parent = self.get(born.parent).unwrap();
            let data = parent.seed.storage.data().unwrap();
            let current = u32::from_le_bytes(data[0x2c..0x30].try_into().unwrap()) as usize;
            let slots = self
                .herd_markers(born.parent)
                .ok_or("Vimp parent marker table absent")?;
            let selected = slots
                .get(current)
                .copied()
                .flatten()
                .ok_or("Vimp parent selected marker absent")?;
            let next = (1..=slots.len())
                .map(|i| (current + i) % slots.len())
                .find(|i| slots[*i].is_some())
                .ok_or("Vimp parent has no occupied marker")?;
            let marker_id = slots[next].unwrap();
            let marker = self
                .herd_marker(marker_id)
                .ok_or("Vimp target requires owned marker")?;
            let pose = |id| {
                self.get(id)
                    .and_then(|a| a.construction_fields.as_ref())
                    .map(|f| f.position)
                    .ok_or("Vimp marker fields absent")
            };
            Some((
                born.parent,
                next,
                pose(selected)?,
                pose(marker_id)?,
                f32::from_bits(marker.values[1]),
            ))
        } else {
            None
        };
        let put = |body: &mut [u8], at: usize, value: f32| {
            body[at..at + 4].copy_from_slice(&value.to_bits().to_le_bytes());
        };
        // 4a8740 preserves word14; all other inline physical words are reset.
        for i in 0..15 {
            if i != 5 {
                put(
                    &mut body,
                    i * 4,
                    if matches!(i, 1 | 2 | 4) { 1. } else { 0. },
                );
            }
        }
        for (at, value) in [4, 8, 0xc, 0x30, 0x34, 0x38].into_iter().zip(&parameters) {
            put(&mut body, at, *value);
        }
        for (at, value) in [0x68, 0x6c, 0x70].into_iter().zip(&parameters[7..10]) {
            put(
                &mut body,
                at,
                ((1. - f64::from(fields.scale)) * f64::from(parameters[6]) * f64::from(*value)
                    + f64::from(*value)) as f32,
            );
        }
        let mut rng = *random;
        let interval = |rng: &mut crate::audio_parameters::Random, low: f32, high: f32| {
            let range = (f64::from(high) - f64::from(low)) as f32;
            f64::from(rng.next_15()) * f64::from(range) / 32768. + f64::from(low)
        };
        put(
            &mut body,
            0x64,
            interval(&mut rng, parameters[10], parameters[11]) as f32,
        );
        let mut scale = fields.scale;
        if let Some((_, next, _, marker_position, radius)) = target {
            body[0x98..0x9c].copy_from_slice(&(next as u32).to_le_bytes());
            let angle = interval(&mut rng, 0., 360.) as f32;
            let (sin, cos) = crate::object_transform::sincos(angle);
            let offsets = [
                (interval(&mut rng, 0., radius) * cos) as f32,
                (interval(&mut rng, 0., radius) * sin) as f32,
            ];
            vimp.born.as_mut().unwrap().offsets = offsets;
            for i in 0..2 {
                put(
                    &mut body,
                    0x40 + i * 4,
                    (f64::from(marker_position[i]) + f64::from(offsets[i])) as f32,
                );
            }
            scale = (f64::from(f32::from_le_bytes(body[0x50..0x54].try_into().unwrap()))
                * f64::from(vimp.template_scale)) as f32;
            vimp.mode = 1;
        }
        vimp.initialized = true;
        let actor = self.residents.get_mut(&id.serial).unwrap();
        actor
            .seed
            .storage
            .data_mut()
            .unwrap()
            .copy_from_slice(&body);
        let fields = actor.construction_fields.as_mut().unwrap();
        fields.scale = scale;
        if target.is_none() {
            fields.object_flags |= 0x8000;
        }
        actor.vimp = Some(vimp);
        actor.affine_cache = None;
        if let Some((parent, _, position, _, _)) = target {
            let parent = self.residents.get_mut(&parent.serial).unwrap();
            parent.construction_fields.as_mut().unwrap().position = position;
            parent.affine_cache = None;
        }
        *random = rng;
        Ok(())
    }
    pub(crate) fn construct_vimp(
        &mut self,
        input: Construction<'_>,
        cold: crate::vimp_actor::Cold,
    ) -> Result<Id, String> {
        if input.class != 8
            || input.seed.storage.profile() != Profile::Debug
            || input.seed.storage.state() != 21
            || input.seed.storage.data().map(<[u8]>::len) != Some(216)
            || !input.seed.diagnostics.is_empty()
            || input.flags.active != input.fields.object_flags
            || input.flags.personality != input.fields.configuration[14]
            || cold.context != input.fields.dispatch_context
        {
            return Err("Vimp constructor requires actual class8/state21 source resources".into());
        }
        let mut staged = Self::new(self.ticket);
        staged.next = self.next;
        let id = staged.register_animated(
            input.class,
            input.seed,
            input.bindings,
            input.flags,
            input.fields.channels,
        )?;
        staged.bind_specification(id, input.specification)?;
        staged.bind_state_resources(id, input.state_resources)?;
        staged.bind_actor_fields(id, input.fields)?;
        let actor = staged.residents.get_mut(&id.serial).unwrap();
        actor.vimp = Some(cold);
        actor.acquired_construction = true;
        self.publish_staged(staged, id)
    }
    pub(crate) fn construct_vimp_chunk(
        &mut self,
        input: Construction<'_>,
        chunk: crate::vimp_chunks::Chunk,
    ) -> Result<Id, String> {
        if !(1117..=1119).contains(&input.class)
            || input.seed.storage.profile() != Profile::Debug
            || input.seed.storage.state() != 80
            || input.seed.storage.data().is_some()
            || !input.seed.diagnostics.is_empty()
            || input.flags.active != input.fields.object_flags
            || input.flags.personality != input.fields.configuration[14]
            || chunk.context != input.fields.dispatch_context
        {
            return Err("chunk constructor requires actual acquired D state80 resources".into());
        }
        let mut staged = Self::new(self.ticket);
        staged.next = self.next;
        let id =
            staged.register_with_mask(input.class, input.seed, input.bindings, input.flags, 0)?;
        staged.bind_specification(id, input.specification)?;
        staged.bind_state_resources(id, input.state_resources)?;
        let channels = input.fields.channels;
        staged.bind_actor_fields(id, input.fields)?;
        let actor = staged.residents.get_mut(&id.serial).unwrap();
        actor.channels = Some(channels);
        actor.acquired_construction = true;
        actor.vimp_chunk = Some(chunk);
        self.publish_staged(staged, id)
    }
    pub(crate) fn construct_wield_child(&mut self, input: Construction<'_>) -> Result<Id, String> {
        if input.seed.storage.profile() != Profile::Debug
            || input.seed.storage.state() != 0
            || input.seed.storage.data().is_some()
            || !input.seed.diagnostics.is_empty()
            || input.flags.active != input.fields.object_flags
            || input.flags.personality != input.fields.configuration[14]
        {
            return Err(
                "held-object constructor requires actual acquired D state0 template".into(),
            );
        }
        let mut staged = Self::new(self.ticket);
        staged.next = self.next;
        let id = staged.register_with_mask(
            input.class,
            input.seed,
            input.bindings,
            input.flags,
            0x20,
        )?;
        staged.bind_specification(id, input.specification)?;
        staged.bind_state_resources(id, input.state_resources)?;
        let channels = input.fields.channels;
        staged.bind_actor_fields(id, input.fields)?;
        let actor = staged.residents.get_mut(&id.serial).unwrap();
        actor.channels = Some(channels);
        actor.acquired_construction = true;
        self.publish_staged(staged, id)
    }
    pub(crate) fn bind_wield_child(
        &mut self,
        child: Id,
        parent: Id,
        reference: i32,
    ) -> Result<(), String> {
        self.check_wield_owner(parent)?;
        let source = self
            .get(parent)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap();
        let position = source.position;
        let yaw = source.angles[0];
        let actor = self
            .get(child)
            .ok_or("held-object binding child absent or foreign")?;
        if child == parent
            || !actor.acquired_construction
            || actor.constructor_mask != 0x20
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || actor.wield_binding.is_some()
            || actor.construction_fields.is_none()
        {
            return Err("held-object binding requires fresh owned constructor".into());
        }
        let actor = self.residents.get_mut(&child.serial).unwrap();
        let fields = actor.construction_fields.as_mut().unwrap();
        fields.position = position;
        fields.angles[0] = yaw;
        fields.object_flags |= 0x20400000;
        fields.dispatch_context = 0;
        actor.attachment = Some(Attachment {
            parent,
            child_reference: 0,
            parent_reference: reference as u32,
            behavior: 0,
        });
        actor.presentation_parent = Some(parent);
        actor.wield_binding = Some(crate::mecc_wield::Binding {
            parent,
            reference,
            context: 0,
        });
        Ok(())
    }
    pub fn attached_effect(&self, id: Id) -> Option<&crate::attached_effect_object::Metadata> {
        self.get(id)?.attached_effect.as_ref()
    }
    fn resolve_attached_effect_world(
        &self,
        id: Id,
    ) -> Result<crate::procedural_aim::Matrix, String> {
        use crate::procedural_aim::Matrix;
        let actor = self.get(id).ok_or("attached effect absent or foreign")?;
        let metadata = actor
            .attached_effect
            .as_ref()
            .ok_or("attached effect metadata absent")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("attached effect fields absent")?;
        if actor.seed.storage.state() != 78
            || actor.presentation_parent != Some(metadata.parent)
            || fields.object_flags & 0x20000000 != 0
            || fields.response_flags & (0x02000000 | 0x9000) != 0
        {
            return Err("attached world lock requires acquired ordinary persistent orientation/scale profile".into());
        }
        let parent = self.get(metadata.parent).ok_or("attached parent absent")?;
        let parent_fields = parent
            .construction_fields
            .as_ref()
            .ok_or("attached parent fields absent")?;
        let cached = self.actor_current_affine(metadata.parent)?;
        let reference = |id: Id, index: u32, parent: bool| -> Result<Matrix, String> {
            let owned = &self
                .specifications
                .get(&id.serial)
                .ok_or("attached reference owner absent")?
                .0;
            if (index as u8 as i8) < 0 || parent && owned.matrices().is_empty() {
                return Ok(Matrix::identity());
            }
            let index = usize::try_from(index).map_err(|_| "attached reference index invalid")?;
            let channels = self
                .get(id)
                .and_then(|a| a.channels.as_ref())
                .ok_or("attached reference channels absent")?;
            owned
                .current_matrices(channels)?
                .get(index)
                .copied()
                .ok_or("attached reference outside acquired array".into())
        };
        // The constructor's tag is the PARENT reference; its opcode24 operand
        // is the CHILD reference. Negative low-byte indices select no reference.
        let anchor = Matrix {
            flags: cached.flags,
            values: cached.matrix,
        }
        .mul(reference(metadata.parent, metadata.tag, true)?)
        .map_err(str::to_owned)?;
        let mut world = crate::effect_attachment::normal_matrix(
            anchor,
            fields.scale,
            parent_fields.scale,
            reference(id, metadata.reference, false)?,
        )
        .map_err(str::to_owned)?;
        world.values[11] = (f64::from(world.values[11])
            + f64::from(f32::from_bits(fields.configuration[17])))
            as f32;
        if !world.values.iter().all(|v| v.is_finite()) {
            return Err("attached world pose nonfinite".into());
        }
        Ok(world)
    }
    /// References must already have been produced from current owned channels.
    /// No inspection sample, missing cache, or initial local matrix is a fallback.
    pub fn update_attached_effect_world(
        &mut self,
        id: Id,
    ) -> Result<crate::procedural_aim::Matrix, String> {
        let world = self.resolve_attached_effect_world(id)?;
        let actor = self
            .residents
            .get_mut(&id.serial)
            .expect("validated attached effect");
        let fields = actor
            .construction_fields
            .as_mut()
            .expect("validated effect fields");
        fields.previous = fields.current;
        fields.position = [world.values[3], world.values[7], world.values[11]];
        fields.current = fields.position;
        actor
            .attached_effect
            .as_mut()
            .expect("validated effect metadata")
            .world_matrix = Some(world);
        Ok(world)
    }
    pub fn current_attached_effect_world(
        &self,
        id: Id,
    ) -> Result<crate::procedural_aim::Matrix, String> {
        let expected = self.resolve_attached_effect_world(id)?;
        let actual = self
            .attached_effect(id)
            .and_then(|m| m.world_matrix)
            .ok_or("attached world pose not produced")?;
        if actual.flags != expected.flags
            || actual.values.map(f32::to_bits) != expected.values.map(f32::to_bits)
        {
            return Err("attached world pose stale for current parent/child references".into());
        }
        Ok(actual)
    }
    /// Exact-D class219/state48 construction with the explicit mask requested
    /// by surfacing callbacks. No source pose/link or behavior is invented.
    pub fn construct_surface_marker(&mut self, input: Construction<'_>) -> Result<Id, String> {
        if input.class != 219
            || input.seed.storage.profile() != Profile::Debug
            || input.seed.storage.state() != 48
            || input.seed.storage.data().is_some()
            || !input.seed.diagnostics.is_empty()
        {
            return Err(
                "surface marker requires acquired D class219/state48 without payload".into(),
            );
        }
        let mut staged = Self::new(self.ticket);
        staged.next = self.next;
        let id = staged.register_with_mask(
            input.class,
            input.seed,
            input.bindings,
            input.flags,
            0x11,
        )?;
        staged.bind_specification(id, input.specification)?;
        staged.bind_state_resources(id, input.state_resources)?;
        let channels = input.fields.channels;
        staged.bind_actor_fields(id, input.fields)?;
        let actor = staged
            .residents
            .get_mut(&id.serial)
            .expect("staged surface marker");
        actor.channels = Some(channels);
        actor.acquired_construction = true;
        self.publish_staged(staged, id)
    }
    pub fn surface_link(&self, id: Id) -> Option<SurfaceLink> {
        self.get(id)?.surface_link
    }
    /// The two D helpers inside the mode44 primary: shift AC/B8, then reset
    /// eight motion and six physical-history words. This is not the mode44
    /// dispatcher, audio/FX submission, or a completed callback.
    pub fn reset_actor_surface_motion(&mut self, id: Id) -> Result<(), String> {
        let actor = self.ordinary_frame_actor(id)?;
        if !self.actor_motion(id)?.awaiting_motion()
            || self.actor_motion(id)?.surface_reset()
            || self.actor_finalization(id)?.phase()
                != crate::resident_finalization::Phase::MotionStarted
            || actor.construction_fields.is_none()
        {
            return Err("surface motion reset requires an admitted owned primary".into());
        }
        let actor = self.residents.get_mut(&id.serial).unwrap();
        let header = actor.construction_fields.as_mut().unwrap();
        let frame = actor.finalization.as_mut().unwrap();
        let motion = actor.ordinary_motion.as_mut().unwrap();
        let next = crate::actor_surface_reset::reset(crate::actor_surface_reset::State {
            position: header.position,
            current: header.current,
            previous: header.previous,
            rates: motion.rates(),
            previous_rates: motion.previous_rates(),
            velocity: frame.state.fields.velocity,
            previous_velocity: frame.state.previous_physical_velocity,
        });
        header.previous = next.previous;
        header.current = next.current;
        motion.reset_surface_rates(&next);
        frame.state.fields.velocity = next.velocity;
        frame.state.previous_physical_velocity = next.previous_velocity;
        // The rate at108 also has a cached finalizer view. It must agree before
        // the later motion phase supplies a new current vertical rate.
        frame.state.fields.model_vertical_speed = next.rates[0];
        Ok(())
    }
    /// Bind the acquired mode44 prelude to an admitted current source. The
    /// primary machine owns its remaining behavior fields and scalar184.
    pub(crate) fn validate_surface_entry<E>(
        &self,
        id: Id,
        snapshot: &crate::ripper_primary_turn::State<Id, E>,
    ) -> Result<(), String> {
        let actor = self.ordinary_frame_actor(id)?;
        let f = actor
            .construction_fields
            .as_ref()
            .ok_or("surface entry fields absent")?;
        if actor.class != 73
            || snapshot.entry.class != 73
            || snapshot.entry.mode != 44
            || snapshot.entry.action != 0
            || f.action != 0
            || actor
                .channels
                .as_ref()
                .is_none_or(|c| c[0] != snapshot.primary)
            || f.position.map(f32::to_bits) != snapshot.entry.position.map(f32::to_bits)
            || f.position.map(f32::to_bits) != snapshot.steering.position.map(f32::to_bits)
            || f.angles[0].to_bits() != snapshot.steering.heading.to_bits()
            || (f.object_flags ^ snapshot.entry.flags) & !0x0300_0000 != 0
            || (f.control_flags ^ snapshot.entry.control) & !0x10 != 0
            || f.configuration[14] != snapshot.steering.personality
            || f.configuration[1] != snapshot.entry.parameter.to_bits()
            || !self.actor_motion(id)?.awaiting_motion()
            || self.actor_motion(id)?.surface_reset()
            || self.actor_finalization(id)?.phase()
                != crate::resident_finalization::Phase::MotionStarted
        {
            return Err("surface entry snapshot disagrees with current owned frame".into());
        }
        if let Some(target) = snapshot.retaliation {
            self.get(target)
                .ok_or("surface entry target absent or foreign")?;
        }
        self.surface_source(id)?;
        Ok(())
    }
    pub(crate) fn publish_surface_reset<E>(
        &mut self,
        id: Id,
        snapshot: &crate::ripper_primary_turn::State<Id, E>,
    ) -> Result<(), String> {
        let actor = self.ordinary_frame_actor(id)?;
        let f = actor
            .construction_fields
            .as_ref()
            .ok_or("surface source fields absent")?;
        if actor.class != 73
            || snapshot.entry.class != 73
            || snapshot.entry.mode != 32
            || snapshot.entry.action != 0
            || f.action != 0
            || snapshot.parameter_184.to_bits() != 0
            || actor
                .channels
                .as_ref()
                .is_none_or(|c| c[0] != snapshot.primary)
            || f.position.map(f32::to_bits) != snapshot.entry.position.map(f32::to_bits)
            || f.position.map(f32::to_bits) != snapshot.steering.position.map(f32::to_bits)
            || f.angles[0].to_bits() != snapshot.steering.heading.to_bits()
            || (f.object_flags ^ snapshot.entry.flags) & !0x0300_0000 != 0
            || (f.control_flags ^ snapshot.entry.control) & !0x10 != 0
            || f.configuration[14] & !0x800 != snapshot.steering.personality
            || f.configuration[1] != snapshot.entry.parameter.to_bits()
        {
            return Err("surface reset snapshot disagrees with current owned source".into());
        }
        if let Some(target) = snapshot.retaliation {
            self.get(target).ok_or("surface target absent or foreign")?;
        }
        let mut combat = self.surface_source(id)?.combat;
        self.reset_actor_surface_motion(id)?;
        let actor = self.residents.get_mut(&id.serial).unwrap();
        let f = actor.construction_fields.as_mut().unwrap();
        f.object_flags = snapshot.entry.flags;
        f.dispatch_context = snapshot.entry.context;
        f.control_flags = snapshot.entry.control;
        f.configuration[14] = snapshot.steering.personality;
        combat.mode = snapshot.entry.mode;
        combat.flags = snapshot.entry.flags;
        combat.control = snapshot.entry.control;
        combat.personality = snapshot.steering.personality;
        combat.retaliation = snapshot.retaliation;
        actor.state23_combat = Some(combat);
        Ok(())
    }
    pub(crate) fn publish_surface_primary_result<E>(
        &mut self,
        id: Id,
        snapshot: &crate::ripper_primary_turn::State<Id, E>,
        marker: Option<Id>,
    ) -> Result<(), String> {
        let actor = self.ordinary_frame_actor(id)?;
        let f = actor
            .construction_fields
            .as_ref()
            .ok_or("surface result fields absent")?;
        if snapshot.entry.class != 73
            || !matches!(snapshot.entry.mode, 32 | 33)
            || snapshot.entry.action != 0
            || snapshot.parameter_184.to_bits() != 0
            || actor
                .channels
                .as_ref()
                .is_none_or(|c| c[0] != snapshot.primary)
            || f.position.map(f32::to_bits) != snapshot.entry.position.map(f32::to_bits)
            || f.position.map(f32::to_bits) != snapshot.steering.position.map(f32::to_bits)
            || f.angles[0].to_bits() != snapshot.steering.heading.to_bits()
            || (f.object_flags ^ snapshot.entry.flags) & !0x0300_0000 != 0
            || f.control_flags != snapshot.entry.control
            || f.configuration[14] & !0x40 != snapshot.steering.personality
            || f.configuration[1] != snapshot.entry.parameter.to_bits()
            || actor.surface_link != marker.map(|marker| SurfaceLink::Owner { marker })
            || !self.actor_motion(id)?.awaiting_motion()
            || !self.actor_motion(id)?.surface_reset()
            || self.actor_finalization(id)?.phase()
                != crate::resident_finalization::Phase::MotionStarted
        {
            return Err("surface result disagrees with its owned pending frame".into());
        }
        if let Some(target) = snapshot.retaliation {
            self.get(target)
                .ok_or("surface result target absent or foreign")?;
        }
        let mut combat = self.state23_snapshot(id)?;
        if combat.owner != marker.map(|id| (id, 219)) {
            return Err("surface result callback owner disagrees with reciprocal marker".into());
        }
        let actor = self.residents.get_mut(&id.serial).unwrap();
        let f = actor.construction_fields.as_mut().unwrap();
        f.object_flags = snapshot.entry.flags;
        f.configuration[14] = snapshot.steering.personality;
        combat.mode = snapshot.entry.mode;
        combat.flags = snapshot.entry.flags;
        combat.personality = snapshot.steering.personality;
        combat.retaliation = snapshot.retaliation;
        actor.state23_combat = Some(combat);
        Ok(())
    }
    /// Acquired mode44 suffix: copy current pose/history, scale and look pitch,
    /// initialize marker mode0, then publish reciprocal callback identities.
    /// This neither executes state48 nor acknowledges the rest of mode44.
    fn surface_source(&self, owner: Id) -> Result<SurfaceSource, String> {
        let source = self.ordinary_frame_actor(owner)?;
        if source.surface_link.is_some() {
            return Err("surface owner already linked".into());
        }
        let look_pitch = self.actor_motion(owner)?.look_pitch();
        let pose = source
            .construction_fields
            .as_ref()
            .ok_or("surface owner fields absent")?;
        let (position, current, previous, angles, scale) = (
            pose.position,
            pose.current,
            pose.previous,
            pose.angles,
            pose.scale,
        );
        // State23 already has a typed view of +274. Keep that consumer in
        // agreement instead of creating a parallel, invisible relationship.
        let combat = self.state23_snapshot(owner)?;
        if combat.owner.is_some() {
            return Err("surface owner already has a callback owner".into());
        }
        Ok(SurfaceSource {
            position,
            current,
            previous,
            angles,
            scale,
            look_pitch,
            combat,
        })
    }
    /// Atomically publish the conditional constructor and its acquired pose/link
    /// suffix. Source preflight precedes construction; no fresh orphan marker
    /// is left behind when a stale/occupied owner or resource binding refuses.
    pub fn construct_linked_surface_marker(
        &mut self,
        owner: Id,
        input: Construction<'_>,
    ) -> Result<Id, String> {
        let source = self.surface_source(owner)?;
        let marker = self.construct_surface_marker(input)?;
        // A fresh dedicated constructor guarantees the destination invariants.
        // No external callback runs between source preflight and publication.
        self.publish_surface_link(owner, marker, source);
        Ok(marker)
    }
    pub fn link_surface_marker(&mut self, owner: Id, marker: Id) -> Result<(), String> {
        let source = self.surface_source(owner)?;
        if owner == marker {
            return Err("surface owner identical to marker".into());
        }
        let destination = self.get(marker).ok_or("surface marker absent or foreign")?;
        if !destination.acquired_construction
            || destination.class != 219
            || destination.seed.storage.profile() != Profile::Debug
            || destination.seed.storage.state() != 48
            || destination.constructor_mask != 0x11
            || destination.surface_link.is_some()
            || destination.attachment.is_some()
            || destination.presentation_parent.is_some()
            || destination.affine_cache.is_some()
            || destination.environment.is_some()
            || destination.construction_fields.is_none()
        {
            return Err("surface link requires a fresh owned D marker".into());
        }
        self.publish_surface_link(owner, marker, source);
        Ok(())
    }
    fn publish_surface_link(&mut self, owner: Id, marker: Id, source: SurfaceSource) {
        // All validation precedes either mutation. Neither resources, channels,
        // membership nor the source pose is overwritten by the native suffix.
        let destination = self.residents.get_mut(&marker.serial).unwrap();
        let fields = destination.construction_fields.as_mut().unwrap();
        fields.position = source.position;
        fields.current = source.current;
        fields.previous = source.previous;
        fields.angles = source.angles;
        fields.scale = source.scale;
        destination.surface_link = Some(SurfaceLink::Marker {
            owner,
            look_pitch: source.look_pitch,
            mode: 0,
            timer: 0.,
        });
        let mut combat = source.combat;
        let actor = self.residents.get_mut(&owner.serial).unwrap();
        actor.surface_link = Some(SurfaceLink::Owner { marker });
        combat.owner = Some((marker, 219));
        actor.state23_combat = Some(combat);
    }
    /// State48 over current owned pose/channels and the real reciprocal peer.
    /// A marked object stays owned: native retirement cleanup is separate.
    pub fn update_surface_marker(
        &mut self,
        id: Id,
        global_seconds: f32,
        areas: &[crate::object_time::SlowArea],
        delay: f32,
        blending: crate::animation_channels::Blending,
    ) -> Result<bool, String> {
        self.update_surface_marker_inner(id, global_seconds, areas, delay, blending, None)
    }
    /// Advance the acquired channel-clock prefix using this actor's own
    /// descriptors. Pose/event tails and native resource retirement stay separate.
    pub fn tick_surface_marker(
        &mut self,
        id: Id,
        global_seconds: f32,
        areas: &[crate::object_time::SlowArea],
        delay: f32,
        blending: crate::animation_channels::Blending,
        clock_seconds: f32,
    ) -> Result<bool, String> {
        let body = self
            .specification(id)
            .ok_or("surface clock specification absent")?;
        let mut durations = vec![None; 256];
        for (&slot, d) in &body.animations {
            durations[usize::from(slot)] = Some(d.template.duration);
        }
        self.update_surface_marker_inner(
            id,
            global_seconds,
            areas,
            delay,
            blending,
            Some((clock_seconds, &durations)),
        )
    }
    fn update_surface_marker_inner(
        &mut self,
        id: Id,
        global_seconds: f32,
        areas: &[crate::object_time::SlowArea],
        delay: f32,
        blending: crate::animation_channels::Blending,
        clock: Option<(f32, &[Option<f32>])>,
    ) -> Result<bool, String> {
        let actor = self
            .get(id)
            .ok_or("surface update actor absent or foreign")?;
        let Some(SurfaceLink::Marker {
            owner,
            look_pitch,
            mode,
            timer,
        }) = actor.surface_link
        else {
            return Err("surface update requires acquired linked marker".into());
        };
        if actor.class != 219
            || actor.seed.storage.profile() != Profile::Debug
            || actor.seed.storage.state() != 48
            || !actor.acquired_construction
        {
            return Err("surface update outside acquired D state48".into());
        }
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("surface update fields absent")?;
        if fields.configuration[14] & 0x10 != 0
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
        {
            return Err("surface update requires acquired unparented non-idle profile".into());
        }
        if fields.object_flags & 0xc000 != 0 {
            return Ok(false);
        }
        let peer = self.get(owner).ok_or("surface owner stale")?;
        if peer.surface_link != Some(SurfaceLink::Owner { marker: id }) {
            return Err("surface owner relationship is not reciprocal".into());
        }
        let position = fields.position;
        let channels = actor.channels.ok_or("surface channels absent")?;
        let owner_view = crate::surface_marker::Owner {
            state: u32::from(peer.seed.storage.state()),
            position: peer
                .construction_fields
                .as_ref()
                .ok_or("surface peer fields absent")?
                .position,
        };
        // Dedicated constructor establishes context0; acquired callbacks do
        // not change it. This is the original object-local time query.
        let seconds = crate::object_time::seconds(
            global_seconds,
            Some(crate::object_time::Sample { owner: 0, position }),
            None,
            areas,
        );
        let next = crate::surface_marker::primary(
            crate::surface_marker::State {
                mode,
                timer,
                position,
                flags: fields.object_flags,
            },
            channels[0],
            Some(owner_view),
            delay,
            seconds,
        )?;
        let mut next_channels = if next.flags & 0xc000 == 0 {
            crate::surface_marker::secondary(next.mode, channels, blending)?
        } else {
            channels
        };
        if next.flags & 0xc000 == 0
            && let Some((dt, durations)) = clock
        {
            let tick = crate::animation_channels::advance(
                &mut next_channels,
                durations,
                [0.; 3],
                dt,
                crate::arithmetic::Precision::Double53,
            )?;
            if tick.missing != 0 {
                return Err("surface clock requires its actual animation descriptors".into());
            }
        }
        let actor = self.residents.get_mut(&id.serial).unwrap();
        actor.construction_fields.as_mut().unwrap().object_flags = next.flags;
        actor.channels = Some(next_channels);
        actor.surface_link = Some(SurfaceLink::Marker {
            owner,
            look_pitch,
            mode: next.mode,
            timer: next.timer,
        });
        Ok(true)
    }
    pub fn positional_impact(&self, id: Id) -> Option<&crate::positional_impact::Metadata> {
        self.get(id)?.positional_impact.as_ref()
    }
    pub(crate) fn construct_positional_impact(
        &mut self,
        input: Construction<'_>,
        metadata: crate::positional_impact::Metadata,
        ticket: Option<u64>,
    ) -> Result<Id, String> {
        if ticket.is_some_and(|t| t != self.ticket)
            || self.residents.values().any(|a| {
                a.positional_impact
                    .as_ref()
                    .is_some_and(|m| m.origin == metadata.origin)
            })
            || input.seed.storage.profile() != Profile::Debug
            || input.seed.storage.state() != 78
            || input.seed.storage.data().is_none_or(|d| d.len() != 268)
            || metadata.complete
            || metadata.failure.is_some()
            || metadata.matrix.flags != 0x10
            || crate::object_lists::membership(input.class, 78, input.flags) != 0
        {
            return Err(
                "positional impact requires matching session and acquired unlisted D state78 birth"
                    .into(),
            );
        }
        let mut staged = Self::new(self.ticket);
        staged.next = self.next;
        let id =
            staged.register_with_mask(input.class, input.seed, input.bindings, input.flags, 0)?;
        staged.bind_specification(id, input.specification)?;
        staged.bind_state_resources(id, input.state_resources)?;
        let fields = input.fields;
        let cache = OwnedAffine {
            value: crate::actor_affine::Cache {
                flags: metadata.matrix.flags,
                matrix: metadata.matrix.values,
            },
            pose: affine_pose_words(&fields),
            external_orientation: true,
        };
        let channels = fields.channels;
        staged.bind_actor_fields(id, fields)?;
        let actor = staged.residents.get_mut(&id.serial).unwrap();
        actor.channels = Some(channels);
        actor.affine_cache = Some(cache);
        actor.positional_impact = Some(metadata);
        actor.acquired_construction = true;
        self.publish_staged(staged, id)
    }
    pub(crate) fn positional_impact_cue(&mut self, id: Id, cue: u32) -> Result<(), String> {
        let actor = self
            .residents
            .get_mut(&id.serial)
            .filter(|a| {
                id.ticket == self.ticket
                    && a.positional_impact
                        .as_ref()
                        .is_some_and(|m| !m.complete && m.failure.is_none())
            })
            .ok_or("positional impact cue owner absent, foreign or failed")?;
        actor.seed.storage.data_mut().unwrap()[0x108..0x10c].copy_from_slice(&cue.to_le_bytes());
        Ok(())
    }
    pub(crate) fn configure_positional_impact(
        &mut self,
        id: Id,
        origin: crate::attached_effect_context::Id,
        sizes: [f32; 2],
        life: [f32; 3],
        spin: [f32; 3],
        fade: [f32; 4],
    ) -> Result<(), String> {
        let actor = self
            .get(id)
            .ok_or("positional impact removed during cue provider")?;
        let metadata = actor
            .positional_impact
            .as_ref()
            .ok_or("positional impact ownership absent")?;
        if metadata.origin != origin || metadata.complete || metadata.failure.is_some() {
            return Err("positional impact source changed during cue provider".into());
        }
        let mut fields = actor.construction_fields.as_ref().unwrap().clone();
        fields.scale = sizes[0];
        crate::effect_actor::lifetime_flags(&mut fields, life);
        if sizes[0] != sizes[1] {
            fields.control_flags |= 8;
        }
        let matrix =
            crate::effect_orientation::matrix(fields.position, fields.angles, fields.scale)
                .map_err(str::to_owned)?;
        fields.angles = crate::effect_orientation::from_matrix(matrix).map_err(str::to_owned)?;
        fields.control_flags |= 0x40000;
        let actor = self.residents.get_mut(&id.serial).unwrap();
        let data = actor.seed.storage.data_mut().unwrap();
        for (at, value) in [
            (4, life[0]),
            (8, 0.),
            (0x10, sizes[0]),
            (0x14, sizes[1]),
            (0x30, spin[1]),
            (0x34, spin[0]),
            (0x38, spin[2]),
        ] {
            data[at..at + 4].copy_from_slice(&value.to_le_bytes());
        }
        let metadata = actor.positional_impact.as_mut().unwrap();
        metadata.matrix = matrix;
        metadata.fade = fade;
        metadata.initial_opacity = if life[1] > 0. { 1. } else { 0. };
        metadata.visual = Some(crate::effect_visual::Visual::new(
            crate::effect_visual::Configuration {
                lifetime: life[0],
                fade_in: life[1],
                fade_out_start: life[2],
                start_size: sizes[0],
                end_size: sizes[1],
            },
        )?);
        actor.affine_cache = Some(OwnedAffine {
            value: crate::actor_affine::Cache {
                flags: matrix.flags,
                matrix: matrix.values,
            },
            pose: affine_pose_words(&fields),
            external_orientation: true,
        });
        actor.construction_fields = Some(fields);
        Ok(())
    }
    pub(crate) fn complete_positional_impact(&mut self, id: Id) -> Result<(), String> {
        self.get(id)
            .ok_or("positional impact absent at completion")?;
        self.residents
            .get_mut(&id.serial)
            .unwrap()
            .positional_impact
            .as_mut()
            .ok_or("positional impact ownership absent")?
            .complete = true;
        Ok(())
    }
    /// Acquired unparented, nonphysical class867 callback with its actual
    /// descriptor clocks/references and an externally owned shared ANM epoch.
    /// `seconds` is the already-produced object-local time, not wall time.
    /// Expiry marks the actor after required cue stop; retirement is separate.
    pub fn advance_positional_impact(
        &mut self,
        id: Id,
        seconds: f32,
        stamp: &mut crate::animation_epoch::Stamp,
        stop: &mut impl crate::positional_impact::Stop,
    ) -> Result<crate::positional_impact::Step, String> {
        use crate::positional_impact::Step;
        if !seconds.is_finite() || seconds < 0. {
            return Err("positional impact requires finite nonnegative object time".into());
        }
        let actor = self.get(id).ok_or("positional impact absent or foreign")?;
        let metadata = actor
            .positional_impact
            .as_ref()
            .ok_or("positional impact ownership absent")?;
        if !metadata.complete || metadata.failure.is_some() {
            return Err("positional impact birth unfinished or callback quarantined".into());
        }
        let mut fields = actor.construction_fields.as_ref().unwrap().clone();
        if fields.object_flags & 0xc000 != 0 {
            return Ok(Step::Inactive);
        }
        let mut channels = actor.channels.ok_or("positional impact channels absent")?;
        let specification = self.specification(id).unwrap().clone();
        if actor.class != 867
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || fields.action != 0
            || !matches!(fields.control_flags, 0x4002e | 0x4000e)
            || specification
                .metadata
                .events
                .iter()
                .any(|t| !t.events.is_empty())
            || channels.iter().any(|c| c[4] != 0 || c[5] != 0)
        {
            return Err(
                "positional impact callback outside acquired unparented timed profile".into(),
            );
        }
        let data = actor.seed.storage.data().unwrap();
        let word = |at| u32::from_le_bytes(data[at..at + 4].try_into().unwrap());
        let cue = word(0x108);
        let rates = [0x30, 0x34, 0x38].map(|at| f32::from_bits(word(at)));
        let mut visual = metadata
            .visual
            .clone()
            .ok_or("impact lifetime state absent")?;
        let next = visual.advance(seconds).map_err(str::to_owned)?;
        let matrix = if next == crate::effect_visual::Step::VisibleState {
            fields.scale = visual.size;
            fields.angles = crate::effect_orientation::advance(fields.angles, rates, seconds)?;
            Some(crate::effect_orientation::matrix(
                fields.position,
                fields.angles,
                fields.scale,
            )?)
        } else {
            None
        };
        crate::effect_actor::callback_entry(&mut fields, 0);
        // Publish age before the expiry services. Completed writes survive a
        // late stop failure, and the failure latch forbids repeating that stop.
        let actor = self.residents.get_mut(&id.serial).unwrap();
        actor.seed.storage.data_mut().unwrap()[8..12].copy_from_slice(&visual.age.to_le_bytes());
        let metadata = actor.positional_impact.as_mut().unwrap();
        if next == crate::effect_visual::Step::VisibleState {
            metadata.initial_opacity = visual.transparency;
        }
        metadata.visual = Some(visual);
        if let Some(matrix) = matrix {
            metadata.matrix = matrix;
            actor.affine_cache = Some(OwnedAffine {
                value: crate::actor_affine::Cache {
                    flags: matrix.flags,
                    matrix: matrix.values,
                },
                pose: affine_pose_words(&fields),
                external_orientation: true,
            });
        }
        actor.construction_fields = Some(fields);
        let result = (|| -> Result<Step, String> {
            if next == crate::effect_visual::Step::Expired {
                if cue != 0 {
                    stop.stop(self, id, cue)?;
                }
                self.residents
                    .get_mut(&id.serial)
                    .ok_or("positional impact removed during cue stop")?
                    .construction_fields
                    .as_mut()
                    .unwrap()
                    .object_flags |= 0x8000;
                return Ok(Step::Marked);
            }
            let mut durations = vec![None; 256];
            for (&slot, descriptor) in &specification.animations {
                durations[usize::from(slot)] = Some(descriptor.template.duration);
            }
            let tick = crate::animation_channels::advance(
                &mut channels,
                &durations,
                [],
                seconds,
                crate::arithmetic::Precision::Double53,
            )?;
            if tick.missing != 0 {
                return Err("positional impact ANM descriptor absent".into());
            }
            let actor = self.residents.get_mut(&id.serial).unwrap();
            actor.channels = Some(channels);
            actor.construction_fields.as_mut().unwrap().channels = channels;
            if tick.processed != 0 {
                actor.mecc_animation_stamp = stamp.advance();
                self.specifications
                    .get_mut(&id.serial)
                    .unwrap()
                    .0
                    .update_current(&channels, actor.mecc_animation_stamp)?;
            }
            Ok(if next == crate::effect_visual::Step::FirstFrame {
                Step::FirstFrame
            } else {
                Step::Updated
            })
        })();
        if let Err(ref error) = result {
            self.fail_positional_impact(id, error);
        }
        result
    }
    pub(crate) fn fail_positional_impact(&mut self, id: Id, error: &str) {
        if id.ticket == self.ticket
            && let Some(m) = self
                .residents
                .get_mut(&id.serial)
                .and_then(|a| a.positional_impact.as_mut())
        {
            m.failure = Some(error.into());
        }
    }
    fn positional_impact_cleanup_preflight(&self, id: Id, started: bool) -> Result<Counts, String> {
        let actor = self
            .get(id)
            .ok_or("impact retirement actor absent or foreign")?;
        let metadata = actor
            .positional_impact
            .as_ref()
            .ok_or("impact retirement owner absent")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("impact retirement fields absent")?;
        let data = actor
            .seed
            .storage
            .data()
            .ok_or("impact retirement payload absent")?;
        if actor.class != 867
            || actor.seed.storage.profile() != Profile::Debug
            || actor.seed.storage.state() != 78
            || !actor.acquired_construction
            || !metadata.complete
            || metadata.failure.is_some()
            || metadata.cleanup_started != started
            || data.len() != 268
            || !metadata.visual.as_ref().is_some_and(|v| {
                let lifetime = f32::from_le_bytes(data[4..8].try_into().unwrap());
                lifetime.is_finite()
                    && lifetime > 0.
                    && v.age.is_finite()
                    && v.age >= lifetime
                    && data[8..12] == v.age.to_le_bytes()
            })
            || data[0xf8..0x108].iter().any(|&b| b != 0)
            || fields.object_flags & 0x8000 == 0
            || fields.configuration[14] & 0x20 != 0
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || self.specification(id).is_none()
            || self.state_resources(id).is_none()
        {
            return Err("impact retirement outside acquired marked class867 profile".into());
        }
        if self.declarations.values().any(|&value| value == id)
            || !self.incoming_actor_references(id)?.is_empty()
        {
            return Err("impact retirement has unresolved declaration/actor references".into());
        }
        let links = self.links.memberships(id.serial);
        if links.len() != 1 || links[0].1 != 0 {
            return Err("impact retirement requires its common-only registration".into());
        }
        Ok(Counts {
            residents: 1,
            payload_bytes: 268,
            registrations: 1,
        })
    }
    /// Complete the acquired unreferenced, expired class867 core retirement.
    /// Required external release runs while the real actor/resources are owned.
    /// A late failure retains core storage and forbids replay. This does not run
    /// the enclosing world's cascade, active-context or global audio cleanup.
    pub fn cleanup_positional_impact(
        &mut self,
        id: Id,
        services: &mut impl crate::actor_retirement::Services,
    ) -> Result<Counts, String> {
        let expected = self.positional_impact_cleanup_preflight(id, false)?;
        self.residents
            .get_mut(&id.serial)
            .unwrap()
            .positional_impact
            .as_mut()
            .unwrap()
            .cleanup_started = true;
        let result = (|| -> Result<Counts, String> {
            services.release(self, id)?;
            self.positional_impact_cleanup_preflight(id, true)?;
            // Source release precedes unlink; the shared class assets and model
            // caches remain ticket-owned. The physics view aliases this payload.
            let actor = self
                .residents
                .remove(&id.serial)
                .expect("validated impact actor");
            drop(actor);
            self.state_resources
                .remove(&id.serial)
                .expect("validated impact state resources");
            self.specifications
                .remove(&id.serial)
                .expect("validated impact specification");
            assert_eq!(self.links.remove_object(id.serial), expected.registrations);
            Ok(expected)
        })();
        if let Err(ref error) = result {
            self.fail_positional_impact(id, error);
        }
        result
    }
    /// Acquired direct class3d4 birth. No attached/positional FX initializer,
    /// physical presence or originating effect context belongs to this route.
    pub(crate) fn construct_delphi_target_proxy(
        &mut self,
        input: Construction<'_>,
    ) -> Result<Id, String> {
        if input.class != 0x3d4
            || input.fields.class != input.class
            || input.seed.storage.profile() != Profile::Debug
            || input.seed.storage.state() != 78
            || input
                .seed
                .storage
                .data()
                .is_none_or(|d| d.len() != 268 || d.iter().any(|b| *b != 0))
            || !input.seed.diagnostics.is_empty()
            || !input.seed.unresolved.is_empty()
            || input.flags.active != input.fields.object_flags
            || input.flags.personality != input.fields.configuration[14]
        {
            return Err(
                "Delphi target proxy requires acquired direct zero D class3d4 resources".into(),
            );
        }
        let mut staged = Self::new(self.ticket);
        staged.next = self.next;
        let id =
            staged.register_with_mask(input.class, input.seed, input.bindings, input.flags, 0)?;
        staged.bind_specification(id, input.specification)?;
        staged.bind_state_resources(id, input.state_resources)?;
        let channels = input.fields.channels;
        staged.bind_actor_fields(id, input.fields)?;
        let actor = staged.residents.get_mut(&id.serial).unwrap();
        actor.channels = Some(channels);
        actor.acquired_construction = true;
        self.publish_staged(staged, id)
    }
    /// Acquired attached flags4 constructor and constant-scale configuration.
    /// The native list mask is zero even when the resource class suggests more.
    /// The initial matrix is local; world attachment resolution remains separate.
    pub(crate) fn construct_attached_effect(
        &mut self,
        input: Construction<'_>,
        metadata: crate::attached_effect_object::Metadata,
    ) -> Result<Id, String> {
        if self.get(metadata.parent).is_none()
            || input.seed.storage.profile() != Profile::Debug
            || input.seed.storage.state() != 78
            || input.seed.storage.data().is_none_or(|d| d.len() != 268)
            || !input.seed.diagnostics.is_empty()
        {
            return Err(
                "attached effect requires valid owned parent and D state78 resources".into(),
            );
        }
        let mut staged = Self::new(self.ticket);
        staged.next = self.next;
        let id =
            staged.register_with_mask(input.class, input.seed, input.bindings, input.flags, 0)?;
        staged.bind_specification(id, input.specification)?;
        staged.bind_state_resources(id, input.state_resources)?;
        let channels = input.fields.channels;
        staged.bind_actor_fields(id, input.fields)?;
        let actor = staged
            .residents
            .get_mut(&id.serial)
            .expect("staged attached effect");
        actor.channels = Some(channels);
        actor.presentation_parent = Some(metadata.parent);
        actor.attached_effect = Some(metadata);
        actor.acquired_construction = true;
        self.publish_staged(staged, id)
    }
    /// State6 resource copy before enemy FX, sound, muzzle and owner services.
    /// Keeps the shell's list mask rather than silently registering state6 anew.
    pub fn construct_enemy_projectile(&mut self, input: Construction<'_>) -> Result<Id, String> {
        if input.flags.active != input.fields.object_flags
            || input.flags.personality != input.fields.configuration[14]
        {
            return Err("enemy projectile template flags differ from class fields".into());
        }
        let prepared = crate::enemy_projectile::prepare(input.seed, &input.fields)?;
        let flags = DirectFlags {
            active: prepared.fields.object_flags,
            personality: prepared.fields.configuration[14],
        };
        let mut staged = Self::new(self.ticket);
        staged.next = self.next;
        let id = staged.register_with_mask(
            input.class,
            &prepared.seed,
            input.bindings,
            flags,
            crate::enemy_projectile::LIST_MASK,
        )?;
        staged.bind_specification(id, input.specification)?;
        staged.bind_state_resources(id, input.state_resources)?;
        let channels = prepared.fields.channels;
        staged.bind_actor_fields(id, prepared.fields)?;
        let actor = staged
            .residents
            .get_mut(&id.serial)
            .expect("staged enemy projectile");
        actor.channels = Some(channels);
        actor.enemy_projectile = Some(prepared.metadata);
        actor.acquired_construction = true;
        self.publish_staged(staged, id)
    }
    /// Complete the acquired enemy launch over an existing owned constructor.
    /// Providers are required; a partial external attempt cannot be replayed.
    /// This does not run state6 motion/hits or certify actor/mission readiness.
    pub(crate) fn publish_primary_shot_source<E>(
        &mut self,
        id: Id,
        snapshot: &crate::ripper_primary_turn::State<Id, E>,
        target: [f32; 3],
        owned_primary: bool,
    ) -> Result<(), String> {
        let source = self.ordinary_frame_actor(id)?;
        let f = source
            .construction_fields
            .as_ref()
            .ok_or("primary fire source fields absent")?;
        if source.class != 73
            || snapshot.entry.class != 73
            || snapshot.entry.mode != 39
            || snapshot.entry.action != 0
            || f.action != 0
            || source
                .channels
                .as_ref()
                .is_none_or(|c| c[0] != snapshot.primary)
            || f.position.map(f32::to_bits) != snapshot.entry.position.map(f32::to_bits)
            || f.position.map(f32::to_bits) != snapshot.steering.position.map(f32::to_bits)
            || f.angles[0].to_bits() != snapshot.steering.heading.to_bits()
            || (f.object_flags ^ snapshot.entry.flags) & !0x0300_0000 != 0
            || (f.control_flags ^ snapshot.entry.control) & !0x10 != 0
            || f.configuration[14] != snapshot.steering.personality
            || f.configuration[1] != snapshot.entry.parameter.to_bits()
        {
            return Err("primary shot snapshot disagrees with its owned current source".into());
        }
        if owned_primary {
            if self.actor_finalization(id)?.phase()
                != crate::resident_finalization::Phase::MotionStarted
            {
                return Err("primary shot requires admitted owned primary".into());
            }
        } else if source.finalization.is_some() || source.ordinary_motion.is_some() {
            return Err("isolated callback cannot bypass an owned frame".into());
        }
        let peer = self
            .get(snapshot.retaliation.ok_or("primary shot target absent")?)
            .ok_or("primary shot target absent or foreign")?;
        let peer = peer
            .construction_fields
            .as_ref()
            .ok_or("primary shot target fields absent")?;
        let expected = [
            peer.position[0],
            peer.position[1],
            (f64::from(peer.position[2])
                + f64::from(f32::from_bits(peer.configuration[22])) * f64::from(peer.scale))
                as f32,
        ];
        if !target.into_iter().all(f32::is_finite)
            || target.map(f32::to_bits) != expected.map(f32::to_bits)
        {
            return Err("primary shot target dimensions or pose are stale".into());
        }
        let source = self.residents.get_mut(&id.serial).unwrap();
        let f = source.construction_fields.as_mut().unwrap();
        f.object_flags = snapshot.entry.flags;
        f.control_flags = snapshot.entry.control;
        f.dispatch_context = snapshot.entry.context;
        // Pose, channels, class parameters and geometry are not overwritten.
        Ok(())
    }
    pub fn finish_enemy_projectile_fire(
        &mut self,
        id: Id,
        input: crate::enemy_projectile::LaunchInput,
        random: &mut crate::audio_parameters::Random,
        services: &mut impl crate::enemy_projectile::Services,
    ) -> Result<(), String> {
        use crate::enemy_projectile::{Launch, Phase};
        let source = self
            .get(input.source)
            .ok_or("enemy source absent or foreign")?;
        let shot = self.get(id).ok_or("enemy projectile absent or foreign")?;
        let metadata = shot
            .enemy_projectile
            .as_ref()
            .ok_or("enemy constructor absent")?;
        if id == input.source
            || !source.acquired_construction
            || source.seed.storage.state() != 23
            || source.attachment.is_some()
            || source.presentation_parent.is_some()
            || source
                .state23_combat
                .as_ref()
                .is_some_and(|s| s.parent.is_some())
            || metadata.phase != Phase::Constructed
            || metadata.resource_class != crate::enemy_ballistic::resource_class(source.class)?
            || shot.attachment.is_some()
            || shot.presentation_parent.is_some()
            || shot.seed.storage.state() != 6
            || shot.seed.storage.data().is_none_or(|d| d.len() != 36)
        {
            return Err(
                "enemy launch outside acquired fresh state6/unparented state23 ownership".into(),
            );
        }
        let fields = shot
            .construction_fields
            .as_ref()
            .ok_or("enemy fields absent")?;
        let speed = f32::from_bits(fields.configuration[1]);
        let gravity = f32::from_bits(fields.configuration[10]);
        if !input
            .target
            .into_iter()
            .chain([input.vertical_spread_percent, speed, gravity])
            .all(f32::is_finite)
            || speed <= 0.
        {
            return Err("enemy launch parameters outside finite positive-speed profile".into());
        }
        let recipe = crate::enemy_projectile::recipe(source.class)?;
        let actor = self.residents.get_mut(&id.serial).unwrap();
        actor.enemy_projectile.as_mut().unwrap().phase = Phase::Services;
        // The native action is already visible to FX preparation.
        actor.construction_fields.as_mut().unwrap().action = recipe.action;
        let result: Result<(), String> = (|| {
            let effect = services.prepare_effect(
                self,
                crate::enemy_projectile::EffectRequest {
                    recipient: id,
                    name: recipe.effect,
                    flags: 4,
                    tag: u32::MAX,
                },
                random,
            )?;
            self.residents
                .get_mut(&id.serial)
                .ok_or("effect service removed projectile")?
                .enemy_projectile
                .as_mut()
                .unwrap()
                .effect = Some(effect);
            let cue = recipe.cues[usize::from((random.next_15() * 100) >> 15 >= 50)];
            let position = self
                .get(input.source)
                .and_then(|a| a.construction_fields.as_ref())
                .ok_or("sound source removed")?
                .position;
            let sound = services.sound(self, cue, position)?;
            let actor = self
                .residents
                .get_mut(&id.serial)
                .ok_or("sound service removed projectile")?;
            let metadata = actor.enemy_projectile.as_mut().unwrap();
            metadata.sound = Some(sound);
            metadata.sound_words = recipe.sound_words;
            let muzzle = services.muzzle(self, input.source, 0, random)?;
            let heading = self
                .get(input.source)
                .and_then(|a| a.construction_fields.as_ref())
                .ok_or("reference source removed")?
                .angles[0];
            if !heading.is_finite() {
                return Err("enemy source heading is nonfinite".into());
            }
            let calculation = crate::enemy_ballistic::launch(
                crate::enemy_ballistic::Parameters {
                    muzzle,
                    target: input.target,
                    forward_speed: speed,
                    gravity,
                    vertical_spread_percent: input.vertical_spread_percent,
                },
                random,
            )?;
            let actor = self
                .residents
                .get_mut(&id.serial)
                .ok_or("reference service removed projectile")?;
            if actor.attachment.is_some() || actor.presentation_parent.is_some() {
                return Err("reference service left an unsupported parented projectile".into());
            }
            let fields = actor.construction_fields.as_mut().unwrap();
            fields.position = muzzle;
            fields.current = muzzle;
            fields.response_flags |= 0x80000000;
            fields.angles = [heading, 0., 0.];
            fields.dispatch_context = input.dispatch_context;
            let bytes = actor.seed.storage.data_mut().unwrap();
            for at in [0, 4] {
                bytes[at..at + 4].copy_from_slice(&calculation.lifetime.to_bits().to_le_bytes());
            }
            let rates = [calculation.vertical_rate, calculation.forward_rate, 0., 0.];
            let metadata = actor.enemy_projectile.as_mut().unwrap();
            metadata.launch = Some(Launch {
                source: input.source,
                dispatch_context: input.dispatch_context,
                rates,
                previous_rates: rates,
                calculation,
            });
            metadata.phase = Phase::Launched;
            Ok(())
        })();
        if let Err(error) = &result
            && let Some(actor) = self.residents.get_mut(&id.serial)
        {
            let metadata = actor.enemy_projectile.as_mut().unwrap();
            metadata.phase = Phase::Failed;
            metadata.failure = Some(error.clone());
        }
        result
    }
    /// First dirty-bounds prelude for the acquired unparented state6 shot.
    /// Executes radius and terrain preparation only, as the native initializer
    /// does. World AABB update, history and motion are separate frame phases.
    pub fn prepare_enemy_projectile_environment(
        &mut self,
        id: Id,
        resources: &crate::scene_resources::SceneResources,
        terrain: &crate::terrain_queries::StaticTerrainQuery<'_>,
    ) -> Result<(), String> {
        use crate::enemy_projectile::Phase;
        let actor = self
            .get(id)
            .ok_or("enemy environment actor absent or foreign")?;
        let metadata = actor
            .enemy_projectile
            .as_ref()
            .ok_or("enemy constructor absent")?;
        if !actor.acquired_construction
            || !matches!(actor.class, 90 | 232)
            || actor.seed.storage.profile() != Profile::Debug
            || actor.seed.storage.state() != 6
            || actor.seed.storage.data().is_none_or(|d| d.len() != 36)
            || metadata.resource_class != actor.class
            || metadata.phase != Phase::Launched
            || metadata.launch.is_none()
            || metadata.initial_environment.is_some()
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
        {
            return Err("enemy environment requires an acquired fresh unparented launch".into());
        }
        let header = actor
            .construction_fields
            .as_ref()
            .ok_or("enemy fields absent")?;
        if header.class != actor.class
            || header.configuration[20] != 6
            || header.response_flags & 0x80000000 == 0
            || header.object_flags & 0xc000 != 0
        {
            return Err("enemy environment requires dirty live fields".into());
        }
        let spec = self.specification(id).ok_or("enemy specification absent")?;
        let bounds = crate::resident_environment::Resources::load(resources, spec)?;
        let channels = actor.channels.ok_or("enemy channels absent")?;
        let selected = bounds.bounds(channels[0][0])?;
        let instance = crate::resident_environment::fresh_instance(
            u32::try_from(id.serial).map_err(|_| "enemy serial outside acquired range")?,
            header,
            channels,
        )?;
        let result = crate::collider_init::initialize(
            instance,
            giants_formats::world_bounds::Resolved {
                primary: selected.primary,
                animation: selected.radius_auxiliary,
            },
            crate::arithmetic::Precision::Double53,
            |p| {
                let sample = terrain.sample([p[0], p[1]])?;
                Ok((sample.height, sample.plane))
            },
        )?;
        if !result.initialized {
            return Err("enemy environment initializer did not execute".into());
        }
        let actor = self.residents.get_mut(&id.serial).unwrap();
        let fields = actor.construction_fields.as_mut().unwrap();
        fields.response_flags = result.instance.fields.response_flags;
        fields.configuration[14] = result.instance.fields.collision_flags;
        fields.configuration[15] = result.instance.usage[0];
        fields.configuration[16] = result.instance.usage[1];
        actor.enemy_projectile.as_mut().unwrap().initial_environment = Some(result.instance);
        Ok(())
    }
    /// Single-player state6 history, motion, owned-list query and terrain reaction.
    /// Queries every eligible member of this Session's actual list1 in order.
    /// This does not certify that mission construction populated the whole world.
    /// Missing/stale geometry fails before history/movement is published. Object
    /// contact and expiry remain pending rather than reporting successful damage.
    pub fn advance_enemy_projectile_singleplayer_owned_world(
        &mut self,
        id: Id,
        world: crate::enemy_projectile::FlightWorld<'_, '_>,
        random: &mut crate::audio_parameters::Random,
        services: &mut impl crate::enemy_projectile::GroundImpactServices,
    ) -> Result<crate::enemy_projectile::FlightOutcome, String> {
        use crate::enemy_projectile::{FlightOutcome, FlightWorld, Phase};
        let FlightWorld {
            terrain,
            inverse_spacing,
            seconds,
            vertex_epoch,
            force_vertices,
            areas,
        } = world;
        let actor = self.get(id).ok_or("flight projectile absent or foreign")?;
        let meta = actor
            .enemy_projectile
            .as_ref()
            .ok_or("enemy constructor absent")?;
        let launch = meta.launch.as_ref().ok_or("enemy launch absent")?;
        let mut header = actor
            .construction_fields
            .clone()
            .ok_or("enemy fields absent")?;
        if !actor.acquired_construction
            || actor.seed.storage.profile() != Profile::Debug
            || actor.seed.storage.state() != 6
            || !matches!(actor.class, 90 | 232)
            || meta.resource_class != actor.class
            || header.class != actor.class
            || header.configuration[20] != 6
            || meta.phase != Phase::Launched
            || meta.initial_environment.is_none()
            || !matches!(terrain.precision(), crate::arithmetic::Precision::Double53)
            || meta.pending_flight.is_some()
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || header.object_flags & 0xc000 != 0
            || header.response_flags & 0x80000000 != 0
            || header.configuration[14] & 0x10 != 0
            || self.get(launch.source).is_none()
        {
            return Err("flight outside acquired prepared single-player state6 profile".into());
        }
        let payload = actor
            .seed
            .storage
            .data()
            .filter(|d| d.len() == 36)
            .ok_or("flight payload absent")?;
        let word = |at| f32::from_le_bytes(payload[at..at + 4].try_into().unwrap());
        let time = crate::enemy_flight::Time {
            seconds,
            context: launch.dispatch_context,
            areas,
        };
        let lifetime = word(4);
        let next = crate::enemy_flight::advance(
            crate::enemy_flight::State {
                position: header.position,
                angles: header.angles,
                rates: launch.rates,
                previous_rates: launch.rates,
                response: header.response_flags,
                personality: header.configuration[14],
            },
            crate::enemy_flight::Parameters {
                braking: f32::from_bits(header.configuration[4]),
                gravity: f32::from_bits(header.configuration[10]),
                pitch_rate: word(8),
            },
            &time,
        )?;
        // Validate lifetime and clock before any external reaction or publication.
        crate::enemy_flight::countdown(lifetime, next.position, &time)?;
        let serial =
            |id: Id| u32::try_from(id.serial).map_err(|_| "flight identity outside acquired range");
        let owner = serial(launch.source)?;
        let mut objects = vec![];
        let mut identities = BTreeMap::new();
        for candidate in self.list(1) {
            let object = self.get(candidate).ok_or("unowned flight candidate")?;
            let fields = object
                .construction_fields
                .as_ref()
                .ok_or("candidate fields absent")?
                .clone();
            if object.presentation_parent.is_some() {
                return Err("flight candidate requires an acquired native parent binding".into());
            }
            let parent = object
                .attachment
                .map(|p| serial(p.parent))
                .transpose()?
                .unwrap_or(0);
            let key = serial(candidate)?;
            if !crate::object_segment::projectile_candidate(
                serial(id)?,
                owner,
                key,
                parent,
                fields.object_flags,
                fields.configuration[14],
            ) {
                continue;
            }
            let prepared = self.actor_environment(candidate)?;
            let bounds = prepared.geometry_fields().bounds;
            let mesh = if fields.configuration[14] & 0x1000000 != 0 {
                crate::object_segment::Mesh::new(key, vec![])?
            } else {
                let slot = self
                    .specification(candidate)
                    .ok_or("candidate specification absent")?
                    .metadata
                    .selection
                    .table()?
                    .collision_slot()?;
                self.prepare_actor_positions(candidate, slot, vertex_epoch, force_vertices)?;
                self.actor_current_contact_mesh(candidate)?
            };
            objects.push(crate::projectile_world::Object {
                id: key,
                parent,
                active: fields.object_flags,
                personality: fields.configuration[14],
                bounds: crate::volume_queries::Bounds {
                    min: bounds[..3].try_into().unwrap(),
                    max: bounds[3..].try_into().unwrap(),
                },
                mesh,
            });
            identities.insert(key, candidate);
        }
        // Position history is separate from the primary wrapper's rate history.
        header.previous = header.current;
        header.current = header.position;
        header.object_flags &= !0x40;
        header.position = next.position;
        header.angles = next.angles;
        header.response_flags = next.response;
        header.configuration[14] = next.personality;
        let ground = terrain.segment(header.current, &mut header.position, inverse_spacing)?;
        let objects = crate::projectile_world::query(
            serial(id)?,
            owner,
            header.current,
            header.position,
            &objects,
        )?;
        let mut outcome = if let Some(target) = objects.selected {
            FlightOutcome::ObjectContact {
                target: identities[&target],
                point: objects.point,
                tag: objects.tag,
            }
        } else if ground.is_some() {
            FlightOutcome::GroundContact {
                point: header.position,
            }
        } else {
            FlightOutcome::Flying
        };
        let (remaining, expired) =
            crate::enemy_flight::countdown(lifetime, header.position, &time)?;
        if matches!(outcome, FlightOutcome::Flying) && expired {
            outcome = FlightOutcome::Expired;
        }
        let actor = self.residents.get_mut(&id.serial).unwrap();
        actor.construction_fields = Some(header);
        let meta = actor.enemy_projectile.as_mut().unwrap();
        let launch = meta.launch.as_mut().unwrap();
        launch.previous_rates = launch.rates;
        launch.rates = next.rates;
        meta.pending_flight = (!matches!(outcome, FlightOutcome::Flying)).then_some(outcome);
        if !matches!(
            outcome,
            FlightOutcome::ObjectContact { .. } | FlightOutcome::GroundContact { .. }
        ) {
            actor.seed.storage.data_mut().unwrap()[4..8].copy_from_slice(&remaining.to_le_bytes());
        }
        if let FlightOutcome::GroundContact { point } = outcome {
            self.enemy_projectile_ground_impact(id, point, random, services)?;
            let actor = self
                .residents
                .get_mut(&id.serial)
                .ok_or("ground reaction cancelled flight")?;
            let position = actor
                .construction_fields
                .as_ref()
                .ok_or("post-contact fields absent")?
                .position;
            let (remaining, expired) = crate::enemy_flight::countdown(lifetime, position, &time)?;
            actor
                .seed
                .storage
                .data_mut()
                .ok_or("post-contact payload absent")?[4..8]
                .copy_from_slice(&remaining.to_le_bytes());
            actor.enemy_projectile.as_mut().unwrap().pending_flight =
                expired.then_some(FlightOutcome::Expired);
            if expired {
                outcome = FlightOutcome::Expired;
            }
        }
        Ok(outcome)
    }
    /// Reaction at an externally established terrain-contact point. This does
    /// not fabricate a world query, run the remaining lifetime tail, or unlink
    /// the actor. Native marking and later resource cleanup are distinct phases.
    pub fn enemy_projectile_ground_impact(
        &mut self,
        id: Id,
        point: [f32; 3],
        random: &mut crate::audio_parameters::Random,
        services: &mut impl crate::enemy_projectile::GroundImpactServices,
    ) -> Result<(), String> {
        use crate::enemy_projectile::{GroundImpact, Phase, RadialRequest};
        if let Some(pending) = self.enemy_projectile(id).and_then(|m| m.pending_flight) {
            match pending {
                crate::enemy_projectile::FlightOutcome::GroundContact { point: expected }
                    if expected.map(f32::to_bits) == point.map(f32::to_bits) => {}
                _ => {
                    return Err(
                        "ground reaction cannot bypass a pending flight continuation".into(),
                    );
                }
            }
        }

        let actor = self.get(id).ok_or("impact projectile absent or foreign")?;
        let metadata = actor
            .enemy_projectile
            .as_ref()
            .ok_or("enemy constructor absent")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("impact fields absent")?;
        let bytes = actor.seed.storage.data().ok_or("impact payload absent")?;
        if !matches!(actor.class, 90 | 232)
            || actor.seed.storage.state() != 6
            || bytes.len() != 36
            || metadata.phase != Phase::Launched
            || metadata.launch.is_none()
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || fields.object_flags & 0xc000 != 0
        {
            return Err("enemy impact outside acquired launched unparented state6 profile".into());
        }
        let power = f32::from_le_bytes(bytes[28..32].try_into().unwrap());
        let damage = f32::from_bits(fields.configuration[12]);
        if !power.is_finite() || power == 0. || !damage.is_finite() || damage < 0. {
            return Err("enemy ground reaction needs its acquired family branch".into());
        }
        crate::enemy_projectile::impact_effect(fields.action, point)?;
        let actor = self.residents.get_mut(&id.serial).unwrap();
        actor.enemy_projectile.as_mut().unwrap().phase = Phase::ImpactServices;
        actor.construction_fields.as_mut().unwrap().position = point;
        let result: Result<(), String> = (|| {
            let fields = self
                .get(id)
                .and_then(|a| a.construction_fields.as_ref())
                .ok_or("impact source removed")?;
            let effect = services.effect(
                self,
                crate::enemy_projectile::impact_effect(fields.action, fields.position)?,
                random,
            )?;
            let actor = self.get(id).ok_or("effect service removed impact source")?;
            let fields = actor
                .construction_fields
                .as_ref()
                .ok_or("impact fields removed")?;
            let body = actor.seed.storage.data().ok_or("impact payload removed")?;
            let context = actor
                .enemy_projectile
                .as_ref()
                .and_then(|m| m.launch.as_ref())
                .ok_or("impact launch removed")?
                .dispatch_context;
            let request = RadialRequest {
                source: id,
                other: None,
                position: fields.position,
                parameters: [12, 20, 16, 28, 32]
                    .map(|offset| u32::from_le_bytes(body[offset..offset + 4].try_into().unwrap())),
                context,
            };
            services.radial(self, request, random)?;
            let actor = self.get(id).ok_or("radial service removed impact source")?;
            let words = actor.enemy_projectile.as_ref().unwrap().sound_words;
            let cue = if words[1] == 0 {
                None
            } else {
                Some(if words[2] != 0 && (random.next_15() * 100) >> 15 >= 50 {
                    words[2]
                } else {
                    words[1]
                })
            };
            let sound = if let Some(cue) = cue {
                let position = actor
                    .construction_fields
                    .as_ref()
                    .ok_or("impact position removed")?
                    .position;
                Some(services.sound(self, cue, position)?)
            } else {
                None
            };
            let actor = self
                .residents
                .get_mut(&id.serial)
                .ok_or("sound service removed impact source")?;
            if actor.attachment.is_some() || actor.presentation_parent.is_some() {
                return Err("impact service left an unsupported parented projectile".into());
            }
            actor.construction_fields.as_mut().unwrap().object_flags |= 0x8000;
            let metadata = actor.enemy_projectile.as_mut().unwrap();
            metadata.ground_impact = Some(GroundImpact { effect, sound, cue });
            metadata.phase = Phase::Retiring;
            Ok(())
        })();
        if let Err(error) = &result
            && let Some(actor) = self.residents.get_mut(&id.serial)
        {
            let metadata = actor.enemy_projectile.as_mut().unwrap();
            metadata.phase = Phase::Failed;
            metadata.failure = Some(error.clone());
        }
        result
    }
    /// Conservative inventory of live typed references. Clearing a reference is
    /// a separate acquired operation; deletion must not silently orphan actors.
    pub fn incoming_actor_references(&self, id: Id) -> Result<Vec<Id>, String> {
        self.get(id).ok_or("reference target absent or foreign")?;
        let mut incoming = Vec::new();
        for (&serial, actor) in &self.residents {
            if serial == id.serial {
                continue;
            }
            let mut refers = actor.attachment.as_ref().is_some_and(|a| a.parent == id)
                || actor.presentation_parent == Some(id);
            if let Some(target) = actor.mecc_local_target {
                refers |= target.links.contains(&Some(id));
            }
            refers |= match actor.surface_link {
                Some(SurfaceLink::Owner { marker }) => marker == id,
                Some(SurfaceLink::Marker { owner, .. }) => owner == id,
                None => false,
            };
            if let Some(special) = actor.mecc_special {
                refers |= special.children.contains(&id);
            }
            if let Some(copy) = actor.delphi_copy_source {
                refers |= copy.owner == id;
            }
            if let Some(crate::delphi_special::State::Complete(special)) = &actor.delphi_special {
                refers |= special.held == id
                    || special
                        .copies
                        .iter()
                        .any(|copy| copy.actor == id || copy.held == id);
            }
            if let Some(primary) = &actor.ripper_primary {
                // A failed service can retain a newly selected target before
                // the completed combat view is published. Keep that identity
                // protected as well; never infer that failure released it.
                refers |=
                    primary.state().retaliation == Some(id) || primary.surface_marker() == Some(id);
            }
            if let Some(s) = &actor.state23_combat {
                refers |= [
                    s.parent,
                    s.owner.map(|v| v.0),
                    s.retaliation,
                    s.target.map(|t| t.id),
                    s.group,
                    s.effect,
                ]
                .contains(&Some(id));
            }
            if let Some(launch) = actor
                .basic_projectile
                .as_ref()
                .and_then(|m| m.launch.as_ref())
            {
                refers |= launch.parent == id
                    || [
                        launch.attachment_parent,
                        launch.source,
                        launch.tracking_actor,
                        launch.mesh_contacts.as_ref().and_then(|q| q.selected),
                    ]
                    .contains(&Some(id))
                    || matches!(launch.actor_hit,crate::basic_projectile::ActorHit::Applied {target} if target==id);
            }
            if let Some(launch) = actor
                .enemy_projectile
                .as_ref()
                .and_then(|m| m.launch.as_ref())
            {
                refers |= launch.source == id;
            }
            if let Some(crate::enemy_projectile::FlightOutcome::ObjectContact { target, .. }) =
                actor
                    .enemy_projectile
                    .as_ref()
                    .and_then(|m| m.pending_flight)
            {
                refers |= target == id;
            }
            if refers {
                incoming.push(Id {
                    ticket: self.ticket,
                    serial,
                });
            }
        }
        Ok(incoming)
    }
    fn enemy_cleanup_preflight(
        &self,
        id: Id,
        phase: crate::enemy_projectile::Phase,
    ) -> Result<Counts, String> {
        let actor = self.get(id).ok_or("retirement actor absent or foreign")?;
        let metadata = actor
            .enemy_projectile
            .as_ref()
            .ok_or("enemy retirement metadata absent")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("enemy retirement fields absent")?;
        if !actor.acquired_construction
            || !matches!(actor.class, 90 | 232)
            || metadata.resource_class != actor.class
            || actor.seed.storage.profile() != Profile::Debug
            || actor.seed.storage.state() != 6
            || actor.seed.storage.data().is_none_or(|b| b.len() != 36)
            || metadata.phase != phase
            || metadata.ground_impact.is_none()
            || metadata.pending_flight.is_some()
            || fields.object_flags & 0x8000 == 0
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || self.specification(id).is_none()
            || self.state_resources(id).is_none()
        {
            return Err("enemy cleanup outside acquired marked terrain-retirement profile".into());
        }
        if self.declarations.values().any(|value| *value == id)
            || !self.incoming_actor_references(id)?.is_empty()
        {
            return Err("enemy retirement has unresolved declaration/actor references".into());
        }
        let links = self.links.memberships(id.serial);
        if links.len() != 1 || links[0].1 != 0 {
            return Err(
                "enemy retirement registrations differ from the acquired common-only constructor"
                    .into(),
            );
        }
        Ok(Counts {
            residents: 1,
            payload_bytes: 36,
            registrations: 1,
        })
    }
    /// Complete acquired unreferenced terrain-retirement ownership. External
    /// cleanup is mandatory; missing/detachment failures preserve core storage.
    /// Shared model-position caches belong to the ticket and remain retained.
    pub fn cleanup_enemy_projectile(
        &mut self,
        id: Id,
        services: &mut impl crate::enemy_projectile::RetirementServices,
    ) -> Result<Counts, String> {
        use crate::enemy_projectile::Phase;
        let expected = self.enemy_cleanup_preflight(id, Phase::Retiring)?;
        self.residents
            .get_mut(&id.serial)
            .unwrap()
            .enemy_projectile
            .as_mut()
            .unwrap()
            .phase = Phase::CleanupServices;
        let result: Result<Counts, String> = (|| {
            services.release(self, id)?;
            self.enemy_cleanup_preflight(id, Phase::CleanupServices)?;
            let removed = self.links.remove_object(id.serial);
            assert_eq!(removed, expected.registrations);
            self.specifications
                .remove(&id.serial)
                .expect("validated retiring specification");
            self.state_resources
                .remove(&id.serial)
                .expect("validated retiring state resources");
            self.residents
                .remove(&id.serial)
                .expect("validated retiring resident");
            Ok(expected)
        })();
        if let Err(error) = &result
            && let Some(actor) = self.residents.get_mut(&id.serial)
        {
            let metadata = actor.enemy_projectile.as_mut().unwrap();
            metadata.phase = Phase::Failed;
            metadata.failure = Some(error.clone());
        }
        result
    }
    /// Dynamic base constructor. The named-effect request stays explicit and
    /// motion/contact callbacks remain pending; no source-owner launch is implied.
    pub fn construct_basic_projectile(
        &mut self,
        input: Construction<'_>,
        definition: Arc<crate::projectile_definition::Definition>,
        dispatch_context: u32,
        camera_origin: [f32; 3],
    ) -> Result<Id, String> {
        if input.flags.active != input.fields.object_flags
            || input.flags.personality != input.fields.configuration[14]
        {
            return Err("basic projectile template flags differ from class fields".into());
        }
        let prepared = crate::basic_projectile::prepare(
            input.seed,
            &input.fields,
            definition,
            dispatch_context,
            camera_origin,
        )?;
        let flags = DirectFlags {
            active: prepared.fields.object_flags,
            personality: prepared.fields.configuration[14],
        };
        let mut staged = Self::new(self.ticket);
        staged.next = self.next;
        let id = staged.register_with_mask(
            input.class,
            &prepared.seed,
            input.bindings,
            flags,
            crate::basic_projectile::LIST_MASK,
        )?;
        staged.bind_specification(id, input.specification)?;
        staged.bind_state_resources(id, input.state_resources)?;
        let channels = prepared.fields.channels;
        staged.bind_actor_fields(id, prepared.fields)?;
        let actor = staged
            .residents
            .get_mut(&id.serial)
            .expect("staged projectile");
        actor.channels = Some(channels);
        actor.basic_projectile = Some(prepared.metadata);
        actor.acquired_construction = true;
        self.publish_staged(staged, id)
    }
    /// Acquired D mode2 straight launch after the current parent-reference
    /// service has resolved a point. Motion/contact dispatch is still pending.
    /// Raw native actor pointers stay zero; their identities are owned metadata.
    pub fn launch_basic_projectile(
        &mut self,
        id: Id,
        input: crate::basic_projectile::LaunchInput,
    ) -> Result<(), String> {
        self.launch_basic_projectile_with_bounds(id, input, None)
    }
    /// Scene-aware launch; bounds retain their prepared specification identity.
    pub fn launch_basic_projectile_in_world(
        &mut self,
        id: Id,
        input: crate::basic_projectile::LaunchInput,
        bounds: &crate::projectile_launch::Bounds,
    ) -> Result<(), String> {
        self.launch_basic_projectile_with_bounds(id, input, Some(bounds))
    }
    fn launch_basic_projectile_with_bounds(
        &mut self,
        id: Id,
        input: crate::basic_projectile::LaunchInput,
        resource_bounds: Option<&crate::projectile_launch::Bounds>,
    ) -> Result<(), String> {
        for related in [Some(input.parent), input.tracking_actor]
            .into_iter()
            .flatten()
        {
            let actor = self
                .get(related)
                .ok_or("launch relation absent or foreign")?;
            if related == id
                || !actor.acquired_construction
                || actor.construction_fields.is_none()
                || self.specification(related).is_none()
                || self.state_resources(related).is_none()
            {
                return Err("launch requires constructed parent and tracking identities".into());
            }
        }
        let actor = self.get(id).ok_or("launch projectile absent or foreign")?;
        let metadata = actor
            .basic_projectile
            .as_ref()
            .ok_or("basic constructor absent")?;
        let body = actor
            .seed
            .storage
            .data()
            .ok_or("projectile storage absent")?;
        if metadata.launch.is_some()
            || actor.seed.storage.state() != 77
            || body.len() != 500
            || body[0x1cc..0x1d0] != [0; 4]
            || metadata.definition.flags() & 1 != 0
            || f32::from_bits(metadata.definition.word(0x58)?) != 0.
            || metadata.definition.effect(3).is_some()
        {
            return Err("launch outside acquired fresh straight/nonhoming profile".into());
        }
        let spec = self
            .specification(id)
            .ok_or("launch body resources absent")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("launch fields absent")?;
        let channels = actor.channels.as_ref().ok_or("launch channels absent")?;
        // Class38 starts on descriptor1. The historical isolated control used
        // zero auxiliary bounds; actual scenes may load a nonzero ABX entry.
        if channels != &fields.channels
            || channels[0][0] != 1
            || !spec.animations.contains_key(&1)
            || self
                .specifications
                .get(&id.serial)
                .ok_or("launch cache absent")?
                .0
                .sampled()
        {
            return Err("launch bounds require acquired cold class38 auxiliary descriptor1".into());
        }
        let reference_selector = if spec.metadata.references.names.is_empty() {
            -1
        } else {
            0
        };
        let straight = crate::projectile_launch::Straight::new(
            input.resolved_reference,
            input.target,
            f32::from_bits(metadata.definition.word(8)?),
        )?;
        let bounds = if let Some(resources) = resource_bounds {
            if !Arc::ptr_eq(spec, &resources.specification) {
                return Err("launch bounds belong to another prepared specification".into());
            }
            crate::collision_bounds::cached(
                Some(crate::collision_bounds::Models {
                    primary: resources.models.primary,
                    auxiliary: resources.models.current_auxiliary,
                    slot: 1,
                }),
                straight.matrix.values[..12].try_into().unwrap(),
                crate::arithmetic::Precision::Double53,
            )?
        } else {
            [
                straight.origin[0],
                straight.origin[1],
                straight.origin[2],
                straight.origin[0],
                straight.origin[1],
                straight.origin[2],
            ]
        };
        let mut fields = actor
            .construction_fields
            .as_ref()
            .ok_or("launch fields absent")?
            .clone();
        fields.previous = fields.current;
        fields.current = straight.origin;
        fields.position = straight.origin;
        fields.response_flags |= 0x9000;
        fields.object_flags |= 0x10000;
        let launch = crate::basic_projectile::Launch {
            parent: input.parent,
            attachment_parent: Some(input.parent),
            source: None,
            mesh_contacts: None,
            actor_hit: crate::basic_projectile::ActorHit::Pending,
            tracking_actor: input.tracking_actor,
            reference: input.reference,
            reference_selector,
            current_matrix: straight.matrix,
            velocity: straight.velocity,
            previous_velocity: [0.; 3],
            ground: 0.,
            ground_plane: [0.; 4],
            straight,
            bounds,
        };
        // Every fallible validation precedes these owned writes.
        let actor = self
            .residents
            .get_mut(&id.serial)
            .expect("validated projectile");
        actor.construction_fields = Some(fields);
        actor
            .basic_projectile
            .as_mut()
            .expect("validated metadata")
            .launch = Some(launch);
        Ok(())
    }
    /// Temporary D reference lock for the acquired class38 no-child-reference
    /// profile. The reference service supplies the parent's CURRENT world point,
    /// separately from the point used at launch. This is a late lock operation,
    /// not a motion tick or a firing transaction. Orientation/velocity survive.
    pub fn release_basic_projectile(
        &mut self,
        id: Id,
        parent: Id,
        resolved_reference: [f32; 3],
    ) -> Result<(), String> {
        let owner = self.get(parent).ok_or("release parent absent or foreign")?;
        if parent == id || !owner.acquired_construction {
            return Err("release requires the constructed launch parent".into());
        }
        let actor = self.get(id).ok_or("release projectile absent or foreign")?;
        let launch = actor
            .basic_projectile
            .as_ref()
            .and_then(|m| m.launch.as_ref())
            .ok_or("release requires a launched projectile")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("release fields absent")?;
        let references = &self
            .specification(parent)
            .ok_or("release parent resources absent")?
            .metadata
            .references
            .names;
        if (launch.reference as u8 as i8) >= 0
            && !references.is_empty()
            && usize::try_from(launch.reference)
                .ok()
                .and_then(|i| references.get(i))
                .is_none()
        {
            return Err("release parent reference outside acquired array".into());
        }
        if launch.attachment_parent != Some(parent)
            || launch.reference_selector != -1
            || actor.seed.storage.state() != 77
            || fields.response_flags & 0x9000 != 0x9000
            || !resolved_reference.into_iter().all(f32::is_finite)
        {
            return Err("release outside acquired temporary translation lock profile".into());
        }
        let mut position = resolved_reference;
        position[2] =
            (f64::from(position[2]) + f64::from(f32::from_bits(fields.configuration[17]))) as f32;
        if !position.into_iter().all(f32::is_finite) {
            return Err("release position outside finite profile".into());
        }
        let actor = self
            .residents
            .get_mut(&id.serial)
            .expect("validated projectile");
        let fields = actor
            .construction_fields
            .as_mut()
            .expect("validated fields");
        fields.previous = fields.current;
        fields.current = position;
        fields.position = position;
        fields.response_flags &= !0x1000;
        let launch = actor
            .basic_projectile
            .as_mut()
            .expect("validated metadata")
            .launch
            .as_mut()
            .expect("validated launch");
        launch.attachment_parent = None;
        for (at, value) in [3, 7, 11].into_iter().zip(position) {
            launch.current_matrix.values[at] = value;
        }
        Ok(())
    }
    /// Shared D force/motion stage only. Current terrain, contact callbacks,
    /// cache production and state77 lifetime/animation still belong to their
    /// later ordered stages; this operation does not acknowledge those services.
    pub fn advance_basic_projectile_motion(
        &mut self,
        id: Id,
        global_delta: f32,
        areas: &[crate::object_time::SlowArea],
    ) -> Result<(), String> {
        let actor = self.get(id).ok_or("motion projectile absent or foreign")?;
        let metadata = actor
            .basic_projectile
            .as_ref()
            .ok_or("basic constructor absent")?;
        let launch = metadata
            .launch
            .as_ref()
            .ok_or("motion requires a launched projectile")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("motion fields absent")?;
        let body = actor.seed.storage.data().ok_or("motion payload absent")?;
        if actor.seed.storage.state() != 77 || body.len() != 500 {
            return Err("motion outside acquired D state77 storage".into());
        }
        let float = |at| f32::from_le_bytes(body[at..at + 4].try_into().unwrap());
        let state = crate::mecc_motion::Motion {
            position: fields.position,
            velocity: launch.velocity,
            previous_velocity: launch.previous_velocity,
            force: [float(0x34), float(0x38), float(0x3c)],
            mass: float(0x24),
            speed_limit: float(0x44),
            force_time_limit: float(0x48),
            stored_dt: float(0x30),
        };
        let frozen = launch.attachment_parent.is_some();
        let delta = if frozen {
            0.
        } else {
            crate::object_time::seconds(
                global_delta,
                Some(crate::object_time::Sample {
                    owner: metadata.dispatch_context,
                    position: fields.position,
                }),
                None,
                areas,
            )
        };
        let motion = crate::mecc_motion::integrate_with_time(
            state,
            delta,
            frozen,
            fields.response_flags & 0x1000 != 0,
            crate::arithmetic::Precision::Double53,
        )?;
        let actor = self
            .residents
            .get_mut(&id.serial)
            .expect("validated projectile");
        actor
            .construction_fields
            .as_mut()
            .expect("validated fields")
            .position = motion.position;
        let launch = actor
            .basic_projectile
            .as_mut()
            .expect("validated metadata")
            .launch
            .as_mut()
            .expect("validated launch");
        launch.velocity = motion.velocity;
        launch.previous_velocity = motion.previous_velocity;
        let body = actor.seed.storage.data_mut().expect("validated storage");
        for (at, value) in [
            (0x30, motion.stored_dt),
            (0x34, motion.force[0]),
            (0x38, motion.force[1]),
            (0x3c, motion.force[2]),
            (0x48, motion.force_time_limit),
        ] {
            body[at..at + 4].copy_from_slice(&value.to_le_bytes());
        }
        Ok(())
    }
    /// Acquired D static terrain/physical-ground stage. The query must describe
    /// a prepared terrain, with dynamic ripple disabled. This does not perform
    /// the preceding swept-segment query, object contacts or state77 hit dispatch.
    pub fn resolve_basic_projectile_ground(
        &mut self,
        id: Id,
        query: &crate::terrain_queries::StaticTerrainQuery<'_>,
    ) -> Result<crate::physical_ground::Output, String> {
        let output = self.prepare_basic_projectile_ground(id, query)?;
        self.commit_basic_projectile_ground(id, &output);
        Ok(output)
    }
    fn prepare_basic_projectile_ground(
        &self,
        id: Id,
        query: &crate::terrain_queries::StaticTerrainQuery<'_>,
    ) -> Result<crate::physical_ground::Output, String> {
        if !query.classified_topology()
            || !matches!(query.precision(), crate::arithmetic::Precision::Double53)
        {
            return Err(
                "D projectile ground requires post-classified terrain at53-bit precision".into(),
            );
        }
        let actor = self.get(id).ok_or("ground projectile absent or foreign")?;
        let launch = actor
            .basic_projectile
            .as_ref()
            .and_then(|m| m.launch.as_ref())
            .ok_or("ground requires a launched projectile")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("ground fields absent")?;
        let body = actor.seed.storage.data().ok_or("ground payload absent")?;
        if actor.seed.storage.state() != 77 || body.len() != 500 {
            return Err("ground outside acquired D state77 storage".into());
        }
        let float = |at| f32::from_le_bytes(body[at..at + 4].try_into().unwrap());
        let input = crate::physical_ground::Input {
            position: fields.position,
            previous: fields.current,
            velocity: launch.velocity,
            scale: fields.scale,
            ground_offset: f32::from_bits(fields.configuration[17]),
            center_offset: f32::from_bits(fields.configuration[22]),
            gravity_mass: float(0x20),
            restitution: float(0x28),
            retention: float(0x2c),
            force_z: float(0x3c),
            flags: u32::from_le_bytes(body[0x1c..0x20].try_into().unwrap()),
            floor: launch.ground,
            plane: launch.ground_plane,
        };
        let output = crate::physical_ground::resolve(
            input,
            crate::arithmetic::Precision::Double53,
            |point| {
                crate::physical_ground::Ground::from_sample(
                    query.sample(point)?,
                    query.dimensions()[0],
                )
            },
        )
        .map_err(str::to_owned)?;
        Ok(output)
    }
    fn commit_basic_projectile_ground(&mut self, id: Id, output: &crate::physical_ground::Output) {
        let actor = self
            .residents
            .get_mut(&id.serial)
            .expect("validated projectile");
        let fields = actor
            .construction_fields
            .as_mut()
            .expect("validated fields");
        fields.position = output.position;
        fields.current = output.previous;
        let launch = actor
            .basic_projectile
            .as_mut()
            .expect("validated metadata")
            .launch
            .as_mut()
            .expect("validated launch");
        launch.velocity = output.velocity;
        launch.ground = output.floor;
        launch.ground_plane = output.plane;
        let body = actor.seed.storage.data_mut().expect("validated payload");
        body[0x1c..0x20].copy_from_slice(&output.flags.to_le_bytes());
        body[0x3c..0x40].copy_from_slice(&output.force_z.to_le_bytes());
    }
    /// Native ordering: sweep a temporary endpoint, resolve physical ground at
    /// the unshortened position, then conditionally apply the swept endpoint.
    /// Object contacts, pose/lifetime and state77 hit callbacks remain separate.
    pub fn resolve_basic_projectile_terrain(
        &mut self,
        id: Id,
        cells: &crate::segment_queries::StaticCellQuery<'_>,
        ground: &crate::terrain_queries::StaticTerrainQuery<'_>,
        inverse_spacing: f32,
    ) -> Result<crate::basic_projectile::TerrainStage, String> {
        if !std::ptr::eq(cells.terrain(), ground.terrain())
            || !matches!(cells.precision(), crate::arithmetic::Precision::Double53)
        {
            return Err("projectile terrain stages require the same static grid at53 bits".into());
        }
        let actor = self.get(id).ok_or("terrain projectile absent or foreign")?;
        let metadata = actor
            .basic_projectile
            .as_ref()
            .ok_or("basic constructor absent")?;
        if metadata.launch.is_none() {
            return Err("terrain requires a launched projectile".into());
        }
        let body = actor.seed.storage.data().ok_or("terrain payload absent")?;
        if actor.seed.storage.state() != 77 || body.len() != 500 {
            return Err("terrain outside acquired D state77 storage".into());
        }
        let flags = u32::from_le_bytes(body[0x5c..0x60].try_into().unwrap());
        let old_flags = u32::from_le_bytes(body[..4].try_into().unwrap());
        if flags & 0x800 != 0 || flags & 0x220 == 0 || old_flags & 2 != 0 {
            return Err("projectile terrain outside acquired live swept-ground profile".into());
        }
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("terrain fields absent")?;
        let mut endpoint = fields.position;
        let sweep = cells
            .segment(fields.current, &mut endpoint, inverse_spacing)
            .map_err(str::to_owned)?;
        let physical = self.prepare_basic_projectile_ground(id, ground)?;
        let hit = sweep.is_some() || physical.flags & 8 != 0;
        if hit && body[0x1a8..0x1ac] != [0; 4] {
            return Err("projectile terrain hit requires its pending effect service".into());
        }
        let next_flags = (old_flags & !0x100) | if hit { 0x100 } else { 0 };
        self.commit_basic_projectile_ground(id, &physical);
        let actor = self
            .residents
            .get_mut(&id.serial)
            .expect("validated projectile");
        if sweep.is_some() && flags & 0x420 != 0 {
            actor
                .construction_fields
                .as_mut()
                .expect("validated fields")
                .position = endpoint;
        }
        if hit && flags & 0x400 != 0 {
            actor
                .basic_projectile
                .as_mut()
                .expect("validated metadata")
                .launch
                .as_mut()
                .expect("validated launch")
                .velocity = [0.; 3];
        }
        actor.seed.storage.data_mut().expect("validated payload")[..4]
            .copy_from_slice(&next_flags.to_le_bytes());
        Ok(crate::basic_projectile::TerrainStage {
            sweep,
            swept_endpoint: endpoint,
            physical,
            hit,
        })
    }
    /// Acquired post-launch attribution store of the normal firing wrapper.
    /// The caller supplies that wrapper's source identity. This does not fire,
    /// acknowledge ammunition/cooldown/audio or complete the firing transaction.
    pub fn attribute_basic_projectile(&mut self, id: Id, source: Id) -> Result<(), String> {
        let owner = self
            .get(source)
            .ok_or("projectile source absent or foreign")?;
        if id == source
            || !owner.acquired_construction
            || owner.construction_fields.is_none()
            || self.specification(source).is_none()
            || self.state_resources(source).is_none()
        {
            return Err("projectile attribution requires a constructed source identity".into());
        }
        let launch = self
            .get(id)
            .and_then(|actor| actor.basic_projectile.as_ref())
            .and_then(|metadata| metadata.launch.as_ref())
            .ok_or("attribution requires a launched projectile")?;
        if launch.source.is_some() {
            return Err("initial projectile source already assigned".into());
        }
        self.residents
            .get_mut(&id.serial)
            .expect("validated projectile")
            .basic_projectile
            .as_mut()
            .expect("validated metadata")
            .launch
            .as_mut()
            .expect("validated launch")
            .source = Some(source);
        Ok(())
    }
    /// State77 mesh-contact query over a supplied posed membership snapshot.
    /// Ownership, class, state, flags and native-parent identity must match this
    /// session. Context words, bounds, poses and list completeness remain input
    /// services. Raw native pointer words remain zero; results use owned IDs.
    pub fn query_basic_projectile_contacts(
        &mut self,
        id: Id,
        frame: &crate::projectile_contacts::Frame<'_, Id>,
        environment: crate::projectile_contacts::Environment,
    ) -> Result<crate::projectile_contacts::Query<Id>, String> {
        for view in frame.actors() {
            let actor = self
                .get(view.id)
                .ok_or("contact snapshot contains a foreign identity")?;
            let fields = actor
                .construction_fields
                .as_ref()
                .ok_or("contact actor fields absent")?;
            let parent = actor.attachment.map(|a| a.parent).or_else(|| {
                actor
                    .basic_projectile
                    .as_ref()
                    .and_then(|p| p.launch.as_ref())
                    .and_then(|l| l.attachment_parent)
            });
            if !actor.acquired_construction
                || actor.class != view.class
                || u32::from(actor.seed.storage.state()) != view.state
                || view.parent != parent
                || fields.object_flags != view.flags.active
                || fields.configuration[14] != view.flags.personality
            {
                return Err("contact snapshot differs from current owned actor fields".into());
            }
        }
        let actor = self.get(id).ok_or("contact projectile absent or foreign")?;
        let metadata = actor
            .basic_projectile
            .as_ref()
            .ok_or("basic constructor absent")?;
        let launch = metadata
            .launch
            .as_ref()
            .ok_or("contacts require a launched projectile")?;
        let source = launch
            .source
            .ok_or("contacts require post-launch shooter attribution")?;
        if self.get(source).is_none() {
            return Err("projectile source no longer owned".into());
        }
        let body = actor.seed.storage.data().ok_or("contact payload absent")?;
        if actor.seed.storage.state() != 77
            || body.len() != 500
            || body[0x10..0x14] != [0; 4]
            || u32::from_le_bytes(body[0x5c..0x60].try_into().unwrap()) & 0x10 == 0
        {
            return Err("contacts outside acquired state77 mesh-list profile".into());
        }
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("contact projectile fields absent")?;
        let result = frame
            .query(
                crate::projectile_contacts::Shot {
                    id,
                    source: Some(source),
                    context: metadata.dispatch_context,
                    start: fields.current,
                    end: fields.position,
                },
                environment,
            )
            .map_err(str::to_owned)?;
        self.residents
            .get_mut(&id.serial)
            .expect("validated projectile")
            .basic_projectile
            .as_mut()
            .expect("validated metadata")
            .launch
            .as_mut()
            .expect("validated launch")
            .mesh_contacts = Some(result.clone());
        Ok(result)
    }
    fn hit_actor(&self, id: Id) -> Result<crate::hit_routing::Actor<Id>, String> {
        let actor = self.get(id).ok_or("hit actor absent or foreign")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("hit actor fields absent")?;
        if !actor.acquired_construction {
            return Err("hit actor lacks acquired construction".into());
        }
        Ok(crate::hit_routing::Actor {
            id,
            parent: actor.attachment.map(|a| a.parent).or_else(|| {
                actor
                    .basic_projectile
                    .as_ref()
                    .and_then(|p| p.launch.as_ref())
                    .and_then(|l| l.attachment_parent)
            }),
            personality: fields.configuration[14],
            reception: fields.configuration[15],
        })
    }
    fn state23_snapshot(&self, id: Id) -> Result<crate::state23_damage::State<Id>, String> {
        use crate::state23_damage::State;
        let actor = self.get(id).ok_or("state23 actor absent or foreign")?;
        if actor.seed.storage.state() != 23
            || !matches!(actor.class, 73 | 230)
            || !actor.acquired_construction
            || self.specification(id).is_none()
            || self.state_resources(id).is_none()
        {
            return Err("state23 snapshot requires acquired class73/230 construction".into());
        }
        let f = actor
            .construction_fields
            .as_ref()
            .ok_or("state23 fields absent")?;
        if let Some(old) = &actor.state23_combat {
            let mut state = old.clone();
            state.flags = f.object_flags;
            state.personality = f.configuration[14];
            state.control = f.control_flags;
            state.action = f.action;
            state.position = f.position;
            return Ok(state);
        }
        let p = actor.seed.storage.data().ok_or("state23 payload absent")?;
        if p.len() != 88
            || [12, 16, 0x24, 0x38]
                .iter()
                .any(|&at| p[at..at + 4] != [0; 4])
        {
            return Err("state23 relationships require typed binding".into());
        }
        Ok(State {
            id,
            class: actor.class,
            limit: f32::from_bits(f.configuration[12]),
            damage: 0.,
            mode: 0,
            action: f.action,
            flags: f.object_flags,
            personality: f.configuration[14],
            control: f.control_flags,
            timer: 0.,
            position: f.position,
            parent: None,
            owner: None,
            retaliation: None,
            target: None,
            group: None,
            group_index: i32::from_le_bytes(p[0x40..0x44].try_into().unwrap()),
            effect: None,
        })
    }
    /// Fresh supplied contact -> offline direct wrapper route -> concrete hurt
    /// services -> owned victim/neighbour publication. This is not a full tick,
    /// live list/pose producer, effect dispatch or projectile retirement.
    pub fn dispatch_basic_projectile_hurt<S: crate::state23_hurt::Sounds<Id>>(
        &mut self,
        id: Id,
        frame: &crate::projectile_contacts::Frame<'_, Id>,
        services: &mut ProjectileHitServices<'_, S>,
    ) -> Result<ProjectileHit, String> {
        use crate::{basic_projectile::ActorHit, hit_routing::Route};
        if services.filter.mode != services.routing.mode {
            return Err("contact and hit contexts disagree on current mode".into());
        }
        if services.routing.mode == 4 {
            return Err("projectile hit requires mode4 network services".into());
        }
        let projectile = self.get(id).ok_or("hit projectile absent or foreign")?;
        let metadata = projectile
            .basic_projectile
            .as_ref()
            .ok_or("basic metadata absent")?;
        let launch = metadata.launch.as_ref().ok_or("hit requires launch")?;
        if launch.actor_hit != ActorHit::Pending {
            return Err("projectile hit attempt already consumed; do not retry".into());
        }
        let owner = launch.source.ok_or("hit requires shooter attribution")?;
        let payload = projectile
            .seed
            .storage
            .data()
            .ok_or("projectile payload absent")?;
        if payload.len() != 500 {
            return Err("hit payload differs from state77 profile".into());
        }
        let range_flags = u32::from_le_bytes(payload[0x5c..0x60].try_into().unwrap());
        let radius = f32::from_le_bytes(payload[0x84..0x88].try_into().unwrap());
        let anchor = std::array::from_fn(|i| {
            f32::from_le_bytes(payload[0x1e8 + i * 4..0x1ec + i * 4].try_into().unwrap())
        });
        let amount = f32::from_le_bytes(payload[0x98..0x9c].try_into().unwrap());
        if !amount.is_finite() || amount <= 0. || payload[0x1a8..0x1ac] != [0; 4] {
            return Err(
                "hit outside positive direct-damage and empty pending-effect profile".into(),
            );
        }
        let query = self.query_basic_projectile_contacts(id, frame, services.filter)?;
        let Some(target) = query.selected else {
            return Ok(ProjectileHit::NoContact);
        };
        let owner_flags = self
            .get(owner)
            .and_then(|a| a.construction_fields.as_ref())
            .ok_or("shooter fields absent")?
            .configuration[14];
        if crate::hit_routing::range_rejected(
            range_flags,
            owner_flags,
            radius,
            anchor,
            query.point,
        )? {
            let projectile = self.residents.get_mut(&id.serial).unwrap();
            let body = projectile.seed.storage.data_mut().unwrap();
            let flags = u32::from_le_bytes(body[..4].try_into().unwrap()) & !0x100;
            body[..4].copy_from_slice(&flags.to_le_bytes());
            body[0x80..0x84].fill(0);
            projectile
                .basic_projectile
                .as_mut()
                .unwrap()
                .launch
                .as_mut()
                .unwrap()
                .actor_hit = ActorHit::Suppressed;
            return Ok(ProjectileHit::RangeRejected { target });
        }
        let route =
            crate::hit_routing::resolve(self.hit_actor(target)?, 0, services.routing, |who| {
                self.hit_actor(who)
            })?;
        if matches!(route, Route::Suppressed) {
            let projectile = self
                .residents
                .get_mut(&id.serial)
                .expect("owned projectile");
            projectile
                .construction_fields
                .as_mut()
                .expect("queried fields")
                .position = query.point;
            projectile
                .basic_projectile
                .as_mut()
                .unwrap()
                .launch
                .as_mut()
                .unwrap()
                .actor_hit = ActorHit::Suppressed;
            return Ok(ProjectileHit::Suppressed { target });
        }
        if route != (Route::Dispatch { target, kind: 0 })
            || self.hit_actor(target)?.parent.is_some()
        {
            return Err(
                "forwarded/finalized/attached hit requires its remaining actor services".into(),
            );
        }
        let mut states = Vec::new();
        for who in frame.members() {
            let actor = self.get(who).ok_or("contact member no longer owned")?;
            if actor.seed.storage.state() != 23
                || states
                    .iter()
                    .any(|s: &crate::state23_damage::State<Id>| s.id == who)
            {
                continue;
            }
            if self.hit_actor(who)?.parent.is_some() {
                return Err(
                    "attached state23 credit member requires typed attachment integration".into(),
                );
            }
            states.push(self.state23_snapshot(who)?);
        }
        let victim_index = states
            .iter()
            .position(|s| s.id == target)
            .ok_or("selected target lacks acquired state23 backend")?;
        let mut victim = states[victim_index].clone();
        if victim.damage >= victim.limit
            || f64::from(victim.damage) + f64::from(amount) >= f64::from(victim.limit)
        {
            return Err("dead/lethal contact requires mission and destruction services".into());
        }
        let mut views = states
            .iter()
            .map(|s| crate::projectile_retaliation::Target {
                id: s.id,
                state: 23,
                position: s.position,
                retaliation: s.retaliation,
            })
            .collect::<Vec<_>>();
        let fields = self
            .get(target)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap();
        let shape = crate::state23_hurt::Shape {
            scale: fields.scale,
            center: f32::from_bits(fields.configuration[22]),
            height: f32::from_bits(fields.configuration[23]),
        };
        // Consume the attempt before external sound/pool services can fail.
        // Partial mutations below are published even on error, never retried.
        let projectile = self.residents.get_mut(&id.serial).unwrap();
        projectile.construction_fields.as_mut().unwrap().position = query.point;
        projectile
            .basic_projectile
            .as_mut()
            .unwrap()
            .launch
            .as_mut()
            .unwrap()
            .actor_hit = ActorHit::Started;
        let result = crate::state23_hurt::apply(
            &mut victim,
            amount,
            Some(crate::state23_damage::Source {
                id,
                state: 77,
                owner: Some(owner),
            }),
            services.callback,
            services.random,
            &mut crate::state23_hurt::Services {
                blood: services.blood,
                actors: &mut views,
                shape,
                sounds: services.sounds,
            },
        );
        for (i, mut state) in states.into_iter().enumerate() {
            if i == victim_index {
                state = victim.clone();
            } else {
                state.retaliation = views[i].retaliation;
            }
            let actor = self
                .residents
                .get_mut(&state.id.serial)
                .expect("prevalidated callback actor");
            let fields = actor
                .construction_fields
                .as_mut()
                .expect("prevalidated fields");
            fields.object_flags = state.flags;
            fields.configuration[14] = state.personality;
            fields.control_flags = state.control;
            fields.action = state.action;
            actor.state23_combat = Some(state);
        }
        let (_, counts) = result?;
        self.residents
            .get_mut(&id.serial)
            .unwrap()
            .basic_projectile
            .as_mut()
            .unwrap()
            .launch
            .as_mut()
            .unwrap()
            .actor_hit = ActorHit::Applied { target };
        Ok(ProjectileHit::Applied { target, counts })
    }
    fn construct_with_state_writes(
        &mut self,
        input: Construction<'_>,
        writes: &[&giants_formats::mission::StateWriteKind],
    ) -> Result<Id, String> {
        self.construct_with_placement(input, writes, None)
    }
    fn construct_with_placement(
        &mut self,
        input: Construction<'_>,
        writes: &[&giants_formats::mission::StateWriteKind],
        initial_pose: Option<crate::uncached_pose::Pose>,
    ) -> Result<Id, String> {
        let kabuto_cold = input.class == 1
            && input.seed.storage.state() == 1
            && input.seed.storage.profile() == Profile::Debug
            && writes.is_empty();
        if (!matches!(input.seed.storage.state(), 0 | 2 | 18 | 23) && !kabuto_cold)
            || !input.seed.diagnostics.is_empty()
        {
            return Err(
                "owned constructor outside acquired ordinary or cold Kabuto profile".into(),
            );
        }
        self.construct_prechecked(input, writes, initial_pose)
    }
    /// Explicit cold support route. Ordinary declarations retain their existing
    /// profile gate; no update or initialization is acknowledged by this entry.
    pub(crate) fn construct_delphi_support_cold(
        &mut self,
        input: Construction<'_>,
    ) -> Result<Id, String> {
        crate::delphi_support::validate(&input)?;
        self.construct_prechecked(input, &[], None)
    }
    fn construct_prechecked(
        &mut self,
        input: Construction<'_>,
        writes: &[&giants_formats::mission::StateWriteKind],
        initial_pose: Option<crate::uncached_pose::Pose>,
    ) -> Result<Id, String> {
        if input.seed.storage.state() == 0 && input.seed.storage.data().is_some() {
            return Err("state0 construction must not allocate a payload".into());
        }
        let mut staged = Self::new(self.ticket);
        staged.next = self.next;
        let id = staged.register_animated(
            input.class,
            input.seed,
            input.bindings,
            input.flags,
            input.fields.channels,
        )?;
        staged.bind_specification(id, input.specification)?;
        staged.bind_state_resources(id, input.state_resources)?;
        staged.bind_actor_fields(id, input.fields)?;
        if let Some(initial) = initial_pose {
            // The source placement writes its first affine before later
            // command9 scale overrides. Preserve that real cache and let the
            // ordinary stale-pose guard require a later writer when needed.
            let fields = staged
                .residents
                .get_mut(&id.serial)
                .unwrap()
                .construction_fields
                .as_mut()
                .unwrap();
            let final_scale = fields.scale;
            fields.mark_mission_placement();
            fields.scale = initial.scale;
            staged.prepare_actor_affine(id)?;
            staged
                .residents
                .get_mut(&id.serial)
                .unwrap()
                .construction_fields
                .as_mut()
                .unwrap()
                .scale = final_scale;
        }
        // Construction precedes mission writes. Keep updates private until
        // every write and typed-resource check succeeds.
        if !writes.is_empty() {
            let actor = staged.residents.get_mut(&id.serial).expect("staged actor");
            for write in writes {
                actor.seed.apply_mission_write(write)?;
                if actor.seed.unresolved.len() != actor.bindings.len()
                    || actor.seed.unresolved.iter().any(|r| {
                        !actor
                            .bindings
                            .iter()
                            .any(|b| b.offset == r.offset && b.key == r.key)
                    })
                {
                    return Err(
                        "mission write replaces a typed state resource; rebinding is pending"
                            .into(),
                    );
                }
            }
        }
        staged
            .residents
            .get_mut(&id.serial)
            .expect("staged actor")
            .acquired_construction = true;
        self.publish_staged(staged, id)
    }
    fn publish_staged(&mut self, mut staged: Self, id: Id) -> Result<Id, String> {
        let actor = staged.get(id).ok_or("staged constructor actor absent")?;
        // Every fallible resource operation above precedes publication. The
        // destination list insertion validates before changing any membership.
        self.links.construct(
            id.serial,
            actor.class,
            u32::from(actor.seed.storage.state()),
            actor.initial_flags,
            actor.constructor_mask,
        )?;
        self.residents.insert(
            id.serial,
            staged
                .residents
                .remove(&id.serial)
                .expect("staged resident"),
        );
        self.specifications.insert(
            id.serial,
            staged
                .specifications
                .remove(&id.serial)
                .expect("staged specification"),
        );
        self.state_resources.insert(
            id.serial,
            staged
                .state_resources
                .remove(&id.serial)
                .expect("staged state resources"),
        );
        self.next = id.serial;
        Ok(id)
    }
    /// Publish a declaration identity only after the complete acquired
    /// construction succeeds. Duplicate keys never allocate another actor.
    pub fn construct_declaration(
        &mut self,
        key: crate::declared_residents::Key,
        input: Construction<'_>,
    ) -> Result<Id, String> {
        self.construct_declaration_with_state_writes(key, input, &[])
    }
    pub fn construct_declaration_with_state_writes(
        &mut self,
        key: crate::declared_residents::Key,
        input: Construction<'_>,
        writes: &[&giants_formats::mission::StateWriteKind],
    ) -> Result<Id, String> {
        // The generic main-loader profiles currently acquired here are
        // ordinary state0/state23. Other character constructors retain their
        // own startup ordering until their complete load path is acquired.
        let mission_header = key.scope == crate::declared_residents::Scope::Mission
            && matches!(input.seed.storage.state(), 0 | 23);
        self.construct_source_declaration(key, |session| {
            let id = session.construct_with_state_writes(input, writes)?;
            if mission_header {
                session
                    .residents
                    .get_mut(&id.serial)
                    .unwrap()
                    .construction_fields
                    .as_mut()
                    .unwrap()
                    .mark_mission_placement();
            }
            Ok(id)
        })
    }
    /// Original mission root placement phase for the generic acquired profiles.
    /// The source class scale precedes later overrides; a resulting old cache
    /// must remain unavailable through actor_current_affine until rewritten.
    pub(crate) fn construct_mission_declaration_with_pose(
        &mut self,
        key: crate::declared_residents::Key,
        input: Construction<'_>,
        writes: &[&giants_formats::mission::StateWriteKind],
        initial: crate::uncached_pose::Pose,
    ) -> Result<Id, String> {
        if key.scope != crate::declared_residents::Scope::Mission
            || !matches!(input.seed.storage.state(), 0 | 23)
            || initial.position.map(f32::to_bits) != input.fields.position.map(f32::to_bits)
            || initial.angles.map(f32::to_bits) != input.fields.angles.map(f32::to_bits)
        {
            return Err("mission initial affine requires matching acquired root placement".into());
        }
        self.construct_source_declaration(key, |session| {
            session.construct_with_placement(input, writes, Some(initial))
        })
    }
    /// Trusted acquired constructors publish the source key only after their
    /// staged actor construction succeeds. Duplicate keys cannot allocate.
    pub(crate) fn construct_source_declaration(
        &mut self,
        key: crate::declared_residents::Key,
        construct: impl FnOnce(&mut Self) -> Result<Id, String>,
    ) -> Result<Id, String> {
        if self.declarations.contains_key(&key) {
            return Err("actor declaration already constructed".into());
        }
        let id = construct(self)?;
        assert!(
            self.get(id).is_some(),
            "source constructor returned a foreign actor"
        );
        self.declarations.insert(key, id);
        Ok(id)
    }
    pub fn declaration(&self, key: crate::declared_residents::Key) -> Option<Id> {
        self.declarations.get(&key).copied()
    }
    pub fn declaration_count(&self) -> usize {
        self.declarations.len()
    }
    /// Initial constructor channels precede presentation/interaction selection.
    /// This executes the measured numeric setter, not evaluated pose blending.
    pub fn select_primary(
        &mut self,
        id: Id,
        request: crate::animation_channels::Request,
        blending: crate::animation_channels::Blending,
    ) -> Result<(), &'static str> {
        self.get(id).ok_or("selection actor absent or foreign")?;
        let channels = self
            .residents
            .get_mut(&id.serial)
            .and_then(|r| r.channels.as_mut())
            .ok_or("selection actor channels absent")?;
        crate::animation_channels::select_primary(channels, request, blending)
    }
    /// Own immutable CPU class resources separately from the state payload.
    /// Native state pointers, event dispatch and live callbacks remain unbound.
    pub fn bind_specification(
        &mut self,
        id: Id,
        specification: Arc<crate::specification::Prepared>,
    ) -> Result<(), &'static str> {
        let resident = self.get(id).ok_or("resource resident absent or foreign")?;
        if resident.class != specification.class {
            return Err("resource specification belongs to another class");
        }
        if !specification.needs.is_empty() {
            return Err("resource specification has unresolved preparation needs");
        }
        if self.specifications.contains_key(&id.serial) {
            return Err("resource specification already bound");
        }
        let references = crate::actor_references::Owned::new(specification)?;
        self.specifications
            .insert(id.serial, OwnedSpecification(references));
        Ok(())
    }
    pub fn specification(&self, id: Id) -> Option<&Arc<crate::specification::Prepared>> {
        self.get(id)?;
        self.specifications
            .get(&id.serial)
            .map(|owned| owned.0.specification())
    }
    pub fn specification_count(&self) -> usize {
        self.specifications.len()
    }
    pub fn bind_actor_fields(
        &mut self,
        id: Id,
        fields: crate::actor_fields::Fields,
    ) -> Result<(), &'static str> {
        let resident = self
            .get(id)
            .ok_or("actor field resident absent or foreign")?;
        if fields.class != resident.class
            || fields.configuration[20] != u32::from(resident.seed.storage.state())
            || fields.object_flags != resident.initial_flags.active
            || fields.configuration[14] != resident.initial_flags.personality
        {
            return Err("actor constructor fields differ from registered class/state/flags");
        }
        if fields.response_flags & 0x8000_0000 == 0
            || fields.configuration[19] != 0
            || fields.configuration[29] != 0
            || fields.configuration[18] as usize != fields.gunlist.len()
        {
            return Err("actor constructor fields contain an invalid copy or resource view");
        }
        if resident.construction_fields.is_some() {
            return Err("actor constructor fields already bound");
        }
        if self.specification(id).is_none() {
            return Err("actor constructor body resources absent");
        }
        self.residents
            .get_mut(&id.serial)
            .ok_or("actor field resident absent")?
            .construction_fields = Some(fields);
        Ok(())
    }
    pub fn actor_field_counts(&self) -> (usize, usize, usize) {
        self.residents
            .values()
            .filter_map(|r| r.construction_fields.as_ref())
            .fold((0, 0, 0), |(actors, guns, shadows), f| {
                (
                    actors + 1,
                    guns + f.gunlist.len(),
                    shadows + usize::from(f.shadow.is_some()),
                )
            })
    }
    /// Exact initial reference service over the constructor's real zeroed cache.
    /// A sampled cache or an unprovided affine cache needs its actual live path.
    pub fn constructor_point(&self, id: Id, index: i32) -> Result<[f32; 3], String> {
        let resident = self.get(id).ok_or("constructor reference actor absent")?;
        let fields = resident
            .construction_fields
            .as_ref()
            .ok_or("constructor reference fields absent")?;
        let cache = &self
            .specifications
            .get(&id.serial)
            .ok_or("constructor reference resource absent")?
            .0;
        if cache.sampled() || fields.object_flags & 0x20 != 0 {
            return Err("constructor point requires the initial uncached actor profile".into());
        }
        let references = cache
            .matrices()
            .iter()
            .map(|m| m.values)
            .collect::<Vec<_>>();
        Ok(crate::object_reference::point(
            &crate::object_reference::Object {
                position: fields.position,
                angles: fields.angles,
                scale: fields.scale,
                flags: fields.object_flags,
                cached: [0.; 16],
                references: &references,
            },
            index,
        )?)
    }
    /// Apply the fresh command12 field setup and measured state18 pickup kernel
    /// to actual owned resources. The point/clock/random are supplied services.
    /// Sound and local-notice requests remain explicit, not successful callbacks.
    pub fn pickup_fresh(
        &mut self,
        child: Id,
        parent: Id,
        point: [f32; 3],
        time_word: u32,
        random_100: u32,
    ) -> Result<Vec<PickupService>, String> {
        use crate::smartie_carry as carry;
        if child == parent {
            return Err("pickup identities coincide".into());
        }
        let child_actor = self.get(child).ok_or("pickup child absent or foreign")?;
        let parent_actor = self.get(parent).ok_or("pickup parent absent or foreign")?;
        if child_actor.attachment.is_some() {
            return Err("pickup requires a fresh unattached actor".into());
        }
        let mut fields = child_actor
            .construction_fields
            .clone()
            .ok_or("pickup child not constructed")?;
        let parent_fields = parent_actor
            .construction_fields
            .as_ref()
            .ok_or("pickup parent not constructed")?;
        if child_actor.seed.storage.state() != 18
            || parent_actor.seed.storage.state() != 2
            || parent_fields.response_flags & 0x800000 != 0
        {
            return Err("pickup requires fresh state18 / unoccupied Mecc".into());
        }
        if self
            .specification(child)
            .is_none_or(|s| !s.animations.contains_key(&41))
        {
            return Err("pickup animation41 resources absent".into());
        }
        let data = child_actor
            .seed
            .storage
            .data()
            .ok_or("pickup state18 payload absent")?;
        if child_actor.bindings.iter().any(|b| {
            [0x18..0x2c, 0x50..0x54, 0x94..0x98]
                .iter()
                .any(|r| b.offset < r.end && b.offset + 4 > r.start)
        }) {
            return Err("pickup state resource overlaps mutable payload fields".into());
        }
        let mut payload = carry::Payload::read(data)?;
        let mut next_data = data.to_vec();
        let mut channels = child_actor.channels.ok_or("pickup channels absent")?;
        // Known fresh constructor fields: no parent/binding, footprint or extra
        // links. Dynamic/replacement actors must use their own lifecycle path.
        let mut actor = carry::Child {
            identity: child.serial,
            state: 18,
            object_flags: fields.object_flags | 0x400,
            collision_flags: fields.configuration[14],
            behavior: 3,
            scale: fields.scale,
            position: fields.position,
            current: fields.current,
            previous: fields.previous,
            channels: [0; 4],
            timer: 0,
            parent: None,
            binding: None,
            footprint: None,
            has_links: false,
        };
        if fields.action != 0 || fields.control_flags & 8 != 0 {
            return Err("pickup child outside fresh constructor action/control profile".into());
        }
        let mut owner = carry::Owner {
            identity: parent.serial,
            state: 2,
            response_flags: parent_fields.response_flags,
        };
        let requests = carry::attach_mecc_at_point(
            &mut actor,
            &mut payload,
            &mut owner,
            carry::Input {
                local_player: Some(parent.serial),
                time_word,
                random_100,
            },
            point,
        )?;
        crate::animation_channels::select_primary(
            &mut channels,
            carry::MECC_PICKUP_ANIMATION,
            crate::animation_channels::Blending::Disabled,
        )?;
        payload.write(&mut next_data)?;
        let mut services = Vec::new();
        for request in requests {
            match request {
                carry::Request::InvalidateFootprint(None) => {} // Actual fresh cache is absent.
                carry::Request::InvalidateFootprint(Some(_)) => {
                    return Err("fresh pickup unexpectedly owns a footprint".into());
                }
                carry::Request::AttachmentSound { owner: who, id }
                | carry::Request::OwnerSound { owner: who, id } => {
                    let kind = if matches!(request, carry::Request::AttachmentSound { .. }) {
                        crate::actor_sound::Kind::Attachment
                    } else {
                        crate::actor_sound::Kind::Owner
                    };
                    let owner = if who == parent.serial {
                        parent
                    } else if who == child.serial {
                        child
                    } else {
                        return Err("pickup sound owner outside session pair".into());
                    };
                    services.push(PickupService::Sound { owner, id, kind });
                }
                carry::Request::LocalPickup { child: who } => {
                    if who != child.serial {
                        return Err("pickup notice owner differs".into());
                    }
                    services.push(PickupService::LocalNotice { child });
                }
            }
        }
        fields.object_flags = actor.object_flags;
        fields.configuration[14] = actor.collision_flags;
        fields.scale = actor.scale;
        fields.position = actor.position;
        fields.current = actor.current;
        fields.previous = actor.previous;
        fields.action = 3;
        fields.control_flags |= 0x20;
        self.set_presentation_parent(child, Some(parent))?;
        let child_actor = self
            .residents
            .get_mut(&child.serial)
            .expect("validated child");
        child_actor
            .seed
            .storage
            .data_mut()
            .expect("validated payload")
            .copy_from_slice(&next_data);
        child_actor.construction_fields = Some(fields);
        child_actor.channels = Some(channels);
        child_actor.attachment = Some(Attachment {
            parent,
            child_reference: carry::MECC_CHILD_REFERENCE,
            parent_reference: carry::MECC_PARENT_REFERENCE,
            behavior: actor.behavior,
        });
        child_actor.presentation_parent = Some(parent);
        child_actor.presentation_position = Some(actor.position);
        child_actor.pickup_notice_pending = true;
        child_actor.pickup_notice_delivery = Some(crate::mission_notice::Delivery::default());
        self.residents
            .get_mut(&parent.serial)
            .expect("validated parent")
            .construction_fields
            .as_mut()
            .expect("validated fields")
            .response_flags = owner.response_flags;
        Ok(services)
    }
    /// Execute only against a caller's real mission state and actual services.
    /// A failed lookup/post keeps the request, including its consumed counter.
    pub fn deliver_pickup_notice(
        &mut self,
        id: Id,
        state: &mut crate::mission_notice::State,
        context: Option<crate::mission_notice::Context<'_>>,
        lookup: impl FnMut(&[u8]) -> Result<Option<Vec<u8>>, String>,
        post: impl FnMut(&crate::mission_notice::Notice) -> Result<(), String>,
    ) -> Result<(), String> {
        self.get(id).ok_or("notice actor absent or foreign")?;
        let actor = self.residents.get_mut(&id.serial).expect("validated actor");
        let delivery = actor
            .pickup_notice_delivery
            .as_mut()
            .ok_or("actor has no pickup notice request")?;
        delivery.advance(state, context, lookup, post)?;
        actor.pickup_notice_pending = !delivery.finished();
        Ok(())
    }
    pub fn messages(&self) -> &crate::mission_messages::Queue {
        &self.messages
    }
    pub fn post_message(&mut self, text: &[u8], route: u32, now: f32) -> Result<(), &'static str> {
        self.messages.post(Some(text), route, now)
    }
    /// Accept into this ticket's real queue. Context and time remain mandatory
    /// caller inputs; acceptance does not certify a visible HUD message.
    pub fn deliver_pickup_notice_to_queue(
        &mut self,
        id: Id,
        state: &mut crate::mission_notice::State,
        context: Option<crate::mission_notice::Context<'_>>,
        catalog: &giants_formats::gtext::Catalog,
        now: f32,
    ) -> Result<(), String> {
        self.get(id).ok_or("notice actor absent or foreign")?;
        let actor = self.residents.get_mut(&id.serial).expect("validated actor");
        let delivery = actor
            .pickup_notice_delivery
            .as_mut()
            .ok_or("actor has no pickup notice request")?;
        delivery.advance(
            state,
            context,
            |key| Ok(catalog.get(key).map(<[u8]>::to_vec)),
            |notice| self.messages.post_notice(notice, now),
        )?;
        actor.pickup_notice_pending = !delivery.finished();
        Ok(())
    }
    /// Bind typed resources at their captured state offsets, leaving the byte
    /// payload unchanged. This does not instantiate native pointers or callbacks.
    pub fn bind_state_resources(
        &mut self,
        id: Id,
        resources: Arc<crate::state_resources::Prepared>,
    ) -> Result<(), &'static str> {
        let resident = self
            .get(id)
            .ok_or("state resource resident absent or foreign")?;
        if !resources.matches(
            resident.class,
            resident.seed.storage.profile(),
            &resident.bindings,
        ) {
            return Err("state resources differ from resident capture");
        }
        if !resources.needs().is_empty() {
            return Err("state resource preparation incomplete");
        }
        if self.state_resources.contains_key(&id.serial) {
            return Err("state resources already bound");
        }
        self.state_resources.insert(id.serial, resources);
        Ok(())
    }
    pub fn state_resources(&self, id: Id) -> Option<&Arc<crate::state_resources::Prepared>> {
        self.get(id)?;
        self.state_resources.get(&id.serial)
    }
    /// Ordinary subjective resource, distinct from the body's specification.
    pub fn mecc_subjective_specification(
        &self,
        id: Id,
    ) -> Result<&Arc<crate::specification::Prepared>, String> {
        let actor = self.get(id).ok_or("subjective source absent or foreign")?;
        if actor.class != 2
            || actor.seed.storage.profile() != Profile::Debug
            || actor.seed.storage.state() != 2
            || actor
                .construction_fields
                .as_ref()
                .is_none_or(|f| f.configuration[20] != 2)
        {
            return Err("subjective source outside acquired ordinary Mecc profile".into());
        }
        self.state_resources(id)
            .ok_or("subjective state resources absent")?
            .specification(0x70)
    }
    /// Publish the subjective ANM into the source's shared reference allocation.
    /// This does not certify the resulting bytes as a pose of the body channels.
    pub fn update_mecc_subjective_references(
        &mut self,
        id: Id,
        context: &mut crate::actor_references::Owned,
        channels: &crate::animation_channels::Channels,
        epoch: u32,
    ) -> Result<crate::actor_references::Update, String> {
        if !Arc::ptr_eq(
            context.specification(),
            self.mecc_subjective_specification(id)?,
        ) {
            return Err("subjective reference context belongs to another resource".into());
        }
        let storage = &mut self
            .specifications
            .get_mut(&id.serial)
            .ok_or("subjective source reference allocation absent")?
            .0;
        context.update_shared_current(storage, channels, epoch)
    }
    pub fn state_resource_counts(&self) -> (usize, usize) {
        (
            self.state_resources.len(),
            self.state_resources
                .values()
                .map(|r| {
                    let (specs, effects) = r.counts();
                    specs + effects
                })
                .sum(),
        )
    }
    pub fn update_references_unblended(
        &mut self,
        id: Id,
        slot: u8,
        seconds: f32,
    ) -> Result<crate::actor_references::Update, String> {
        self.get(id).ok_or("reference actor absent or foreign")?;
        self.specifications
            .get_mut(&id.serial)
            .ok_or("reference actor resources absent")?
            .0
            .update_unblended(slot, seconds)
    }
    /// Refresh references from owned current channels in the acquired single
    /// frame profile. This does not advance clocks or run procedural callbacks.
    pub fn update_current_references(
        &mut self,
        id: Id,
    ) -> Result<crate::actor_references::Update, String> {
        let channels = self
            .get(id)
            .and_then(|a| a.channels)
            .ok_or("current reference actor/channels absent or foreign")?;
        self.specifications
            .get_mut(&id.serial)
            .ok_or("current reference resources absent")?
            .0
            .update_current_unblended(&channels)
    }
    /// Evaluate current ordinary mixed references using a caller-owned native
    /// epoch. Clock advancement, callbacks and world admission remain separate.
    pub fn update_current_references_at(
        &mut self,
        id: Id,
        epoch: u32,
    ) -> Result<crate::actor_references::Update, String> {
        let channels = self
            .get(id)
            .and_then(|a| a.channels)
            .ok_or("current reference actor/channels absent or foreign")?;
        self.specifications
            .get_mut(&id.serial)
            .ok_or("current reference resources absent")?
            .0
            .update_current(&channels, epoch)
    }
    /// Resolve a current world reference over owned animation and affine caches.
    /// Native origin/outside-reference semantics remain distinct from a missing
    /// producer for an in-range animated slot.
    pub fn current_reference_point(&self, id: Id, index: i32) -> Result<[f32; 3], String> {
        self.reference_point_at(id, index, false)
    }
    /// A primary may read its previous finalization's affine before common
    /// motion. This is not permission to consume a current collision shape.
    pub fn current_primary_reference_point(&self, id: Id, index: i32) -> Result<[f32; 3], String> {
        self.ordinary_frame_actor(id)?;
        if self.actor_finalization(id)?.phase()
            != crate::resident_finalization::Phase::MotionStarted
        {
            return Err("primary reference requires an admitted owned primary phase".into());
        }
        let actor = self.get(id).unwrap();
        let header = actor
            .construction_fields
            .as_ref()
            .ok_or("primary reference fields absent")?;
        let prior = &self.actor_finalization(id)?.state().fields;
        if header.position.map(f32::to_bits) != prior.current.map(f32::to_bits)
            || header.angles.map(f32::to_bits) != prior.angles.map(f32::to_bits)
            || header.scale.to_bits() != prior.scale.to_bits()
        {
            return Err("primary reference pose changed before common motion".into());
        }
        self.reference_point_at(id, index, true)
    }
    fn reference_point_at(&self, id: Id, index: i32, primary: bool) -> Result<[f32; 3], String> {
        let actor = self
            .get(id)
            .ok_or("current reference actor absent or foreign")?;
        if actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || actor
                .state23_combat
                .as_ref()
                .is_some_and(|s| s.parent.is_some())
        {
            return Err("parented reference requires its world-pose producer".into());
        }
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("current reference fields absent")?;
        let cache = &self
            .specifications
            .get(&id.serial)
            .ok_or("current reference specification absent")?
            .0;
        let in_range =
            (index as u8 as i8) >= 0 && index >= 0 && (index as usize) < cache.matrices().len();
        let matrices = if in_range {
            cache.current_matrices(
                actor
                    .channels
                    .as_ref()
                    .ok_or("current reference channels absent")?,
            )?
        } else {
            &[]
        };
        let references = matrices.iter().map(|m| m.values).collect::<Vec<_>>();
        let cached = if fields.object_flags & 0x20 != 0 {
            self.cached_affine_at(id, primary)?.matrix
        } else {
            [0.; 16]
        };
        Ok(crate::object_reference::point(
            &crate::object_reference::Object {
                position: fields.position,
                angles: fields.angles,
                scale: fields.scale,
                flags: fields.object_flags,
                cached,
                references: &references,
            },
            index,
        )?)
    }
    pub fn reference_matrix(
        &self,
        id: Id,
        reference: usize,
    ) -> Result<crate::procedural_aim::Matrix, &'static str> {
        self.get(id).ok_or("reference actor absent or foreign")?;
        self.specifications
            .get(&id.serial)
            .ok_or("reference actor resources absent")?
            .0
            .matrix(reference)
    }
    /// Current local matrices consumed by presentation, without advancing ANM.
    pub fn current_reference_matrices(
        &self,
        id: Id,
    ) -> Result<&[crate::procedural_aim::Matrix], String> {
        let actor = self.get(id).ok_or("current reference owner absent")?;
        let channels = actor
            .channels
            .as_ref()
            .ok_or("current reference channels absent")?;
        self.specifications
            .get(&id.serial)
            .ok_or("current reference specification absent")?
            .0
            .current_matrices(channels)
    }
    pub fn reference_cache_storage(
        &self,
        id: Id,
    ) -> Result<crate::actor_references::Storage<'_>, String> {
        self.get(id)
            .ok_or("reference storage owner absent or foreign")?;
        let storage = &self
            .specifications
            .get(&id.serial)
            .ok_or("reference storage absent")?
            .0;
        Ok(crate::actor_references::Storage {
            sampled: storage.sampled(),
            matrices: storage.matrices(),
        })
    }
    pub fn set_presentation_parent(
        &mut self,
        id: Id,
        parent: Option<Id>,
    ) -> Result<(), &'static str> {
        self.get(id).ok_or("presentation actor absent")?;
        let mut next = parent;
        let mut remaining = self.residents.len();
        while let Some(node) = next {
            if node == id || remaining == 0 {
                return Err("presentation attachment cycle");
            }
            next = self
                .get(node)
                .ok_or("presentation parent absent or foreign")?
                .presentation_parent;
            remaining -= 1;
        }
        self.residents
            .get_mut(&id.serial)
            .ok_or("presentation actor absent")?
            .presentation_parent = parent;
        Ok(())
    }
    pub fn set_presentation_position(
        &mut self,
        id: Id,
        position: Option<[f32; 3]>,
    ) -> Result<(), &'static str> {
        if position.is_some_and(|p| !p.into_iter().all(f32::is_finite)) {
            return Err("nonfinite presentation sound position");
        }
        self.get(id).ok_or("presentation actor absent or foreign")?;
        self.residents
            .get_mut(&id.serial)
            .ok_or("presentation actor absent")?
            .presentation_position = position;
        Ok(())
    }
    pub fn presentation_position(&self, id: Id) -> Option<[f32; 3]> {
        self.get(id)?.presentation_position
    }
    pub fn sound_route(
        &self,
        id: Id,
        local_player: Id,
        kind: crate::actor_sound::Kind,
        descriptor_ready: bool,
    ) -> Result<Option<crate::actor_sound::Route<Id>>, &'static str> {
        self.get(id).ok_or("sound actor absent or foreign")?;
        self.get(local_player)
            .ok_or("sound local player absent or foreign")?;
        crate::actor_sound::resolve(
            kind,
            Some(id),
            Some(local_player),
            false,
            descriptor_ready,
            |node| {
                Ok(self
                    .get(node)
                    .ok_or("sound attachment absent or foreign")?
                    .presentation_parent)
            },
        )
    }
    /// Refuse ambiguity rather than ticking an arbitrary actor of this class.
    pub fn unique_class(&self, class: i32) -> Result<Id, &'static str> {
        let mut matches = self.residents.iter().filter(|(_, r)| r.class == class);
        let (&serial, _) = matches.next().ok_or("resident class absent")?;
        if matches.next().is_some() {
            return Err("resident class is ambiguous");
        }
        Ok(Id {
            ticket: self.ticket,
            serial,
        })
    }
    pub fn advance_channels(
        &mut self,
        id: Id,
        durations: &[Option<f32>],
        model: [f32; 3],
        dt: f32,
        precision: crate::arithmetic::Precision,
    ) -> Result<crate::animation_channels::Tick, &'static str> {
        if id.ticket != self.ticket {
            return Err("animation resident belongs to another ticket");
        }
        let channels = self
            .residents
            .get_mut(&id.serial)
            .and_then(|r| r.channels.as_mut())
            .ok_or("animated resident absent")?;
        crate::animation_channels::advance(channels, durations, model, dt, precision)
    }
    /// Current owned channels and immutable resources feed the shared model
    /// buffer. Epoch/gate provenance and live behavior selection are upstream.
    pub fn prepare_actor_positions(
        &mut self,
        id: Id,
        model_slot: usize,
        epoch: u32,
        force: bool,
    ) -> Result<crate::actor_position_buffers::Update, String> {
        let actor = self.get(id).ok_or("position actor absent or foreign")?;
        let channels = actor.channels.ok_or("position actor channels absent")?;
        let spec = self
            .specification(id)
            .ok_or("position actor specification absent")?
            .clone();
        self.position_buffers
            .update(&spec, model_slot, &channels, epoch, force)
    }
    pub fn actor_model_positions(
        &self,
        id: Id,
        model_slot: usize,
    ) -> Result<Option<&[[f32; 3]]>, String> {
        let spec = self
            .specification(id)
            .ok_or("position actor absent,foreign or without specification")?;
        self.position_buffers.positions(spec, model_slot)
    }
    pub fn position_buffer_count(&self) -> usize {
        self.position_buffers.len()
    }
    pub fn actor_model_cache(
        &self,
        id: Id,
        model_slot: usize,
    ) -> Result<Option<crate::vertex_cache::Cache>, String> {
        let spec = self
            .specification(id)
            .ok_or("position actor absent,foreign or without specification")?;
        self.position_buffers.cache(spec, model_slot)
    }
    /// Run the acquired ordinary affine writer from this actor's current
    /// fields. Call order remains the mission scheduler's responsibility.
    pub fn prepare_actor_affine(&mut self, id: Id) -> Result<(), String> {
        let actor = self.get(id).ok_or("affine actor absent or foreign")?;
        if actor
            .finalization
            .as_ref()
            .is_some_and(|f| f.phase != crate::resident_finalization::Phase::Idle)
        {
            return Err("ordinary frame affine requires successful finalization".into());
        }
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("affine actor fields absent")?;
        if actor.attachment.is_some() || actor.presentation_parent.is_some() {
            return Err("parented actor requires its acquired world-affine service".into());
        }
        let (active, value) = crate::actor_affine::update(
            crate::uncached_pose::Pose {
                position: fields.position,
                angles: fields.angles,
                scale: fields.scale,
            },
            fields.object_flags,
            actor.affine_cache.as_ref().map(|c| c.value),
            crate::arithmetic::Precision::Double53,
        )
        .map_err(str::to_owned)?;
        let cache = OwnedAffine {
            value,
            pose: affine_pose_words(fields),
            external_orientation: active & 0x10000 != 0,
        };
        let actor = self
            .residents
            .get_mut(&id.serial)
            .expect("validated affine actor");
        actor.construction_fields.as_mut().unwrap().object_flags = active;
        actor.affine_cache = Some(cache);
        Ok(())
    }
    /// Consume the owned cache only while it still corresponds to current
    /// actor fields; stale caches never become an implicit bind/identity pose.
    pub fn actor_current_affine(&self, id: Id) -> Result<crate::actor_affine::Cache, String> {
        self.cached_affine_at(id, false)
    }
    fn cached_affine_at(
        &self,
        id: Id,
        primary: bool,
    ) -> Result<crate::actor_affine::Cache, String> {
        let actor = self
            .get(id)
            .ok_or("current contact actor absent or foreign")?;
        if actor.finalization.as_ref().is_some_and(|f| {
            f.phase
                != if primary {
                    crate::resident_finalization::Phase::MotionStarted
                } else {
                    crate::resident_finalization::Phase::Idle
                }
        }) {
            return Err("ordinary frame affine unavailable before successful finalization".into());
        }
        if actor.attachment.is_some() || actor.presentation_parent.is_some() {
            return Err("parented contact actor requires its world-affine service".into());
        }
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("current contact fields absent")?;
        let cache = actor
            .affine_cache
            .as_ref()
            .ok_or("current contact affine not prepared")?;
        if fields.object_flags & 0x20 == 0
            || cache.pose != affine_pose_words(fields)
            || cache.external_orientation != (fields.object_flags & 0x10000 != 0)
        {
            return Err("current contact affine is stale".into());
        }
        Ok(cache.value)
    }
    /// Explicit late initialization over current owned constructor fields.
    /// No behavior, animation advancement or mission readiness is invoked.
    /// Resources and terrain must come from the owning mission context; this
    /// method does not infer a world from the ticket.
    pub fn prepare_actor_environment(
        &mut self,
        id: Id,
        resources: &crate::scene_resources::SceneResources,
        terrain: &crate::terrain_queries::StaticTerrainQuery<'_>,
    ) -> Result<(), String> {
        let actor = self.get(id).ok_or("environment actor absent or foreign")?;
        if !actor.acquired_construction
            || actor.seed.storage.state() != 23
            || !matches!(actor.class, 73 | 230)
            || actor.state23_combat.is_some()
            || actor.environment.is_some()
            || actor.ordinary_motion.is_some()
        {
            return Err(
                "environment requires fresh acquired Ripper construction before callbacks".into(),
            );
        }
        let fields = actor
            .construction_fields
            .clone()
            .ok_or("environment fields absent")?;
        let channels = actor.channels.ok_or("environment channels absent")?;
        let cache = if fields.object_flags & 0x20 != 0 {
            Some(self.actor_current_affine(id)?)
        } else {
            None
        };
        let body = self
            .specification(id)
            .ok_or("environment specification absent")?;
        let prepared = crate::resident_environment::Resources::load(resources, body)?;
        let (environment, fields) = crate::resident_environment::Prepared::initialize(
            u32::try_from(id.serial).map_err(|_| "environment serial outside acquired range")?,
            fields,
            channels,
            prepared,
            cache,
            |p| {
                let s = terrain.sample([p[0], p[1]])?;
                Ok((s.height, s.plane))
            },
        )?;
        let actor = self
            .residents
            .get_mut(&id.serial)
            .expect("validated environment actor");
        actor.construction_fields = Some(fields);
        actor.environment = Some(environment);
        Ok(())
    }
    /// Validate current fields, primary slot and affine before returning any
    /// geometry; old constructor/bind poses are never substituted.
    pub fn actor_environment(
        &self,
        id: Id,
    ) -> Result<&crate::resident_environment::Prepared, String> {
        let actor = self.get(id).ok_or("environment actor absent or foreign")?;
        if actor
            .finalization
            .as_ref()
            .is_some_and(|f| f.phase != crate::resident_finalization::Phase::Idle)
        {
            return Err(
                "ordinary frame geometry unavailable before successful finalization".into(),
            );
        }
        let header = actor
            .construction_fields
            .as_ref()
            .ok_or("environment fields absent")?;
        let slot = actor.channels.ok_or("environment channels absent")?[0][0];
        let cache = self.actor_current_affine(id)?;
        let prepared = actor
            .environment
            .as_ref()
            .ok_or("resident environment not prepared")?;
        prepared.validate(header, slot, cache)?;
        Ok(prepared)
    }
    /// Explicit moving-geometry phase over an already initialized resident.
    /// Current affine and completed common motion are required; no behavior,
    /// terrain finalization, radius initialization or readiness is performed.
    pub fn refresh_actor_environment_geometry(&mut self, id: Id) -> Result<(), String> {
        let actor = self.get(id).ok_or("geometry actor absent or foreign")?;
        if actor
            .finalization
            .as_ref()
            .is_some_and(|f| f.phase != crate::resident_finalization::Phase::Idle)
        {
            return Err("ordinary frame geometry requires successful finalization".into());
        }
        if !actor.acquired_construction
            || actor.seed.storage.profile() != crate::state_storage::Profile::Debug
            || actor.seed.storage.state() != 23
            || !matches!(actor.class, 73 | 230)
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || actor
                .state23_combat
                .as_ref()
                .is_some_and(|s| s.parent.is_some())
            || actor
                .ordinary_motion
                .as_ref()
                .is_some_and(|m| m.awaiting_motion())
        {
            return Err(
                "geometry refresh outside acquired unattached completed state23 phase".into(),
            );
        }
        let cache = self.actor_current_affine(id)?;
        let actor = self.residents.get_mut(&id.serial).unwrap();
        actor
            .environment
            .as_mut()
            .ok_or("resident environment not prepared")?
            .refresh_geometry(
                actor
                    .construction_fields
                    .as_ref()
                    .ok_or("geometry fields absent")?,
                actor.channels.ok_or("geometry channels absent")?[0][0],
                cache,
            )
    }
    pub fn actor_environment_collider(
        &self,
        id: Id,
    ) -> Result<crate::actor_environment::Collider, String> {
        let prepared = self.actor_environment(id)?;
        let active = self
            .get(id)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap()
            .object_flags;
        prepared.collider(active, self.actor_current_affine(id)?)
    }
    pub fn actor_current_contact_mesh(
        &self,
        id: Id,
    ) -> Result<crate::object_segment::Mesh, String> {
        let value = self.actor_current_affine(id)?;
        let active = self
            .get(id)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap()
            .object_flags;
        let pose = crate::contact_pose::Cached::new(active, value.flags, value.matrix)?;
        self.actor_contact_mesh(id, pose)
    }
    /// Snapshot the dedicated collision model from an already prepared owned
    /// buffer. The current affine actor pose remains an explicit service input.
    pub fn actor_contact_mesh(
        &self,
        id: Id,
        pose: crate::contact_pose::Cached,
    ) -> Result<crate::object_segment::Mesh, String> {
        let actor = self.get(id).ok_or("contact actor absent or foreign")?;
        let active = actor
            .construction_fields
            .as_ref()
            .ok_or("contact actor fields absent")?
            .object_flags;
        if active & 0x20 == 0 {
            return Err("owned contact actor has no acquired affine cache".into());
        }
        let spec = self
            .specification(id)
            .ok_or("contact actor specification absent")?;
        let slot = spec.metadata.selection.table()?.collision_slot()?;
        let model = spec
            .models
            .get(slot)
            .and_then(Option::as_ref)
            .ok_or("dedicated collision model absent")?;
        let positions = self
            .position_buffers
            .positions(spec, slot)?
            .ok_or("dedicated collision positions not prepared")?;
        let serial = u32::try_from(id.serial)
            .map_err(|_| "contact mesh identity exceeds acquired profile")?;
        crate::object_segment::Mesh::from_local_positions(
            serial,
            &model.prepared.mesh,
            positions,
            pose,
        )
        .map_err(str::to_owned)
    }
    /// The caller must prepare the source channels and apply the requested
    /// selection successfully before this ownership transfer.
    pub fn register_animated(
        &mut self,
        class: i32,
        seed: &Seed,
        bindings: &[StateBinding],
        flags: DirectFlags,
        channels: crate::animation_channels::Channels,
    ) -> Result<Id, String> {
        let id = self.register(class, seed, bindings, flags)?;
        self.residents
            .get_mut(&id.serial)
            .expect("new owned resident")
            .channels = Some(channels);
        Ok(id)
    }
    pub fn list(&self, list: u32) -> Vec<Id> {
        self.links
            .list(list)
            .into_iter()
            .map(|serial| Id {
                ticket: self.ticket,
                serial,
            })
            .collect()
    }
    pub fn new_common_walk(&self, visit_budget: usize) -> Result<CommonWalk, String> {
        if self.common_walk_running || self.common_walk_failure.is_some() {
            return Err("common walk running or failed; cancellation required".into());
        }
        Ok(CommonWalk {
            ticket: self.ticket,
            inner: crate::object_list_walk::Walk::new(visit_budget)?,
        })
    }
    /// Capture the head at entry, then resolve each successor AFTER the visitor.
    /// The visitor must execute its acquired phase or return an error. This
    /// method never supplies history, behavior or a successful default callback.
    /// Failure preserves effects and quarantines further walks until clear().
    pub fn walk_common(
        &mut self,
        walk: &mut CommonWalk,
        visitor: &mut impl FnMut(&mut Session, Id) -> Result<crate::object_list_walk::Flow, String>,
    ) -> Result<crate::object_list_walk::Outcome, String> {
        if walk.ticket != self.ticket {
            return Err("common walk belongs to another ticket".into());
        }
        if walk.phase() != crate::object_list_walk::Phase::Fresh {
            return Err("common walk already consumed".into());
        }
        if self.common_walk_running || self.common_walk_failure.is_some() {
            return Err("common walk running or failed; cancellation required".into());
        }
        self.common_walk_running = true;
        let result = walk.inner.run(&mut CommonWalkAdapter {
            session: self,
            visitor,
        });
        self.common_walk_running = false;
        if let Err(error) = &result {
            self.common_walk_failure = Some(error.clone());
        }
        result
    }
    pub fn common_walk_failure(&self) -> Option<&str> {
        self.common_walk_failure.as_deref()
    }
    /// Compose the mutable common-list traversal with the acquired per-actor
    /// iteration. Histories and surrounding world services must already exist.
    /// Unsupported active actors fail explicitly rather than being skipped.
    pub fn update_common_world(
        &mut self,
        walk: &mut CommonWalk,
        services: &mut impl ActorWorldServices,
    ) -> Result<crate::object_list_walk::Outcome, String> {
        self.walk_common(walk, &mut |session, id| {
            if session.actor_world_flags(id)? & 0xc000 != 0 {
                return Ok(crate::object_list_walk::Flow::Continue);
            }
            let outcome = session.update_actor_world(id, services)?;
            Ok(
                if outcome == crate::actor_world_tick::Outcome::WorldAborted {
                    crate::object_list_walk::Flow::Stop
                } else {
                    crate::object_list_walk::Flow::Continue
                },
            )
        })
    }
    pub fn entry_subset(&self) -> Result<OwnedEntrySubset, String> {
        let mut actors = vec![];
        for id in self.list(0) {
            let actor = self
                .get(id)
                .ok_or("entry subset has an unowned registration")?;
            let fields = actor
                .construction_fields
                .as_ref()
                .ok_or_else(|| format!("entry subset actor {id:?} fields are not bound"))?;
            actors.push(EntryActor {
                id,
                class: actor.class,
                contribution: fields.entry_contribution(),
            });
        }
        Ok(OwnedEntrySubset { actors })
    }
    pub fn counts(&self) -> Counts {
        Counts {
            residents: self.residents.len(),
            payload_bytes: self
                .residents
                .values()
                .map(|r| r.seed.storage.data().map_or(0, <[u8]>::len))
                .sum(),
            registrations: (0..15).map(|list| self.links.list(list).len()).sum(),
        }
    }
    /// Unlink before dropping Rust-owned payloads and resource caches. This
    /// cancels Session ownership; native world retirement services are separate.
    pub fn clear(&mut self) -> Counts {
        let counts = self.counts();
        for serial in self.residents.keys() {
            self.links.remove_object(*serial);
        }
        self.residents.clear();
        self.specifications.clear();
        self.position_buffers.clear();
        self.state_resources.clear();
        self.declarations.clear();
        self.messages = crate::mission_messages::Queue::default();
        self.mecc_events = None;
        self.kabuto_events = None;
        self.delphi_events = None;
        self.mecc_contact_materials = None;
        self.vimp_events = None;
        self.mecc_reticle_owner = None;
        self.mecc_target_range = None;
        // A visitor can cancel while walk_common is on the stack. Its running
        // lease is released by the outer call, never by clearing registries.
        self.common_walk_failure = None;
        counts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_target_publication_owns_links_preserves_payload_and_replaces_misses() {
        let mut session = Session::new(921);
        let owner = fixture_actor(&mut session, 2, 2);
        let first = fixture_actor(&mut session, 73, 23);
        let second = fixture_actor(&mut session, 73, 23);
        let before = session
            .get(owner)
            .unwrap()
            .seed
            .storage
            .data()
            .unwrap()
            .to_vec();
        let point = [-0., 15., -3.5];
        session
            .publish_mecc_target(owner, 17, Some(first), point, true)
            .unwrap();
        let published = session.mecc_local_target(owner).unwrap();
        assert_eq!(published.links, [Some(first); 3]);
        assert!(crate::mecc_fire_primary::check_local_target_profile(&session, owner).is_err());
        assert_eq!(published.point.map(f32::to_bits), point.map(f32::to_bits));
        assert_eq!(published.vertex_epoch, 17);
        assert_eq!(session.mecc_local_reticle(), Some((owner, true)));
        assert_eq!(session.mecc_local_target_in_range(), Some((owner, false)));
        session.publish_mecc_target_range(owner, true).unwrap();
        assert_eq!(session.incoming_actor_references(first).unwrap(), [owner]);
        let mut expected = before;
        for (at, value) in [0xe4, 0xe8, 0xec].into_iter().zip(point) {
            expected[at..at + 4].copy_from_slice(&value.to_le_bytes());
        }
        assert_eq!(
            session.get(owner).unwrap().seed.storage.data().unwrap(),
            expected
        );
        session
            .publish_mecc_target(owner, 18, Some(second), [9., 8., 7.], false)
            .unwrap();
        assert!(session.incoming_actor_references(first).unwrap().is_empty());
        assert_eq!(session.incoming_actor_references(second).unwrap(), [owner]);
        session
            .publish_mecc_target(owner, 19, None, [1., 2., 3.], false)
            .unwrap();
        assert_eq!(session.mecc_local_target(owner).unwrap().links, [None; 3]);
        assert_eq!(session.mecc_local_target_in_range(), Some((owner, false)));
        assert!(crate::mecc_fire_primary::check_local_target_profile(&session, owner).is_ok());
        assert!(
            session
                .incoming_actor_references(second)
                .unwrap()
                .is_empty()
        );
        session
            .publish_mecc_target(owner, 20, Some(owner), [4., 5., 6.], true)
            .unwrap();
        assert_eq!(session.mecc_local_target(owner).unwrap().links, [None; 3]);
        session.clear();
        assert!(session.mecc_local_target(owner).is_none());
        assert!(session.mecc_local_reticle().is_none());
        assert!(session.mecc_local_target_in_range().is_none());
    }
    #[test]
    fn local_target_publication_rejects_invalid_inputs_without_replacing_state() {
        let mut session = Session::new(922);
        let owner = fixture_actor(&mut session, 2, 2);
        let target = fixture_actor(&mut session, 73, 23);
        let foreign = fixture_actor(&mut Session::new(923), 73, 23);
        session
            .publish_mecc_target(owner, 1, Some(target), [1., 2., 3.], true)
            .unwrap();
        let state = session.mecc_local_target(owner);
        let bytes = session
            .get(owner)
            .unwrap()
            .seed
            .storage
            .data()
            .unwrap()
            .to_vec();
        for (source, epoch, selected, point) in [
            (owner, 2, Some(foreign), [0.; 3]),
            (foreign, 2, Some(target), [0.; 3]),
            (target, 2, None, [0.; 3]),
            (owner, 0, None, [0.; 3]),
            (owner, 2, None, [f32::NAN, 0., 0.]),
        ] {
            assert!(
                session
                    .publish_mecc_target(source, epoch, selected, point, false)
                    .is_err()
            );
            assert_eq!(session.mecc_local_target(owner), state);
            assert_eq!(
                session.get(owner).unwrap().seed.storage.data().unwrap(),
                bytes
            );
            assert_eq!(session.mecc_local_reticle(), Some((owner, true)));
        }
        // Raw identities are never silently adopted or converted into typed ones.
        session
            .residents
            .get_mut(&owner.serial)
            .unwrap()
            .seed
            .storage
            .data_mut()
            .unwrap()[0xe0..0xe4]
            .copy_from_slice(&1_u32.to_le_bytes());
        assert!(
            session
                .publish_mecc_target(owner, 2, None, [0.; 3], false)
                .is_err()
        );
        assert_eq!(session.mecc_local_target(owner), state);
    }
    #[test]
    fn current_affine_requires_preparation_rejects_stale_pose_and_preserves_failure_state() {
        let mut session = Session::new(95);
        let id = session
            .register(73, &seed(23), &[], DirectFlags::default())
            .unwrap();
        assert!(session.prepare_actor_affine(id).is_err());
        // Synthetic current fields test ownership and invalidation only; the
        // resource-backed control independently exercises real construction.
        session
            .residents
            .get_mut(&id.serial)
            .unwrap()
            .construction_fields = Some(crate::actor_fields::Fields {
            class: 73,
            dispatch_context: 0,
            configuration: [0; 30],
            scale: 1.,
            object_flags: 0,
            response_flags: 0,
            control_flags: 0,
            action: 0,
            shadow_reciprocal: 0.,
            shadow: None,
            gunlist: Arc::from([]),
            channels: [[0; 9]; 4],
            position: [1., 2., 3.],
            current: [0.; 3],
            previous: [0.; 3],
            angles: [0.; 3],
        });
        assert!(session.actor_current_affine(id).is_err());
        session.prepare_actor_affine(id).unwrap();
        assert_eq!(session.actor_current_affine(id).unwrap().matrix[3], 1.);
        session
            .residents
            .get_mut(&id.serial)
            .unwrap()
            .construction_fields
            .as_mut()
            .unwrap()
            .position[0] = 4.;
        assert!(session.actor_current_affine(id).is_err());
        session.prepare_actor_affine(id).unwrap();
        let before = session.actor_current_affine(id).unwrap();
        assert_eq!(before.matrix[3], 4.);
        session
            .residents
            .get_mut(&id.serial)
            .unwrap()
            .construction_fields
            .as_mut()
            .unwrap()
            .angles[0] = f32::NAN;
        assert!(session.prepare_actor_affine(id).is_err());
        assert_eq!(
            session
                .get(id)
                .unwrap()
                .affine_cache
                .as_ref()
                .unwrap()
                .value
                .matrix,
            before.matrix
        );
        assert!(session.actor_current_affine(id).is_err());
        session
            .residents
            .get_mut(&id.serial)
            .unwrap()
            .construction_fields
            .as_mut()
            .unwrap()
            .angles[0] = 0.;
        session
            .residents
            .get_mut(&id.serial)
            .unwrap()
            .presentation_parent = Some(id);
        assert!(session.prepare_actor_affine(id).is_err());
        assert!(session.actor_current_affine(id).is_err());
        session.clear();
        assert!(session.actor_current_affine(id).is_err());
        assert!(session.prepare_actor_affine(id).is_err());
    }
    fn fixture_actor(session: &mut Session, class: i32, state: u8) -> Id {
        use crate::model_registry::{ClassProgram, ModelRegistry};
        let mut models = ModelRegistry::default();
        models.programs.insert(
            class,
            vec![ClassProgram {
                catalog: "fixture.bin".into(),
                program: Ok(giants_formats::class_program::parse(&[0x1e, state, 255]).unwrap()),
                bindings: Ok(vec![]),
            }],
        );
        let resources = crate::scene_resources::SceneResources {
            world_bounds: Default::default(),
            classes: crate::catalog_registry::WorldClassRegistry {
                visits: vec![],
                classes: Default::default(),
                textures: Default::default(),
                models,
            },
            archives: Default::default(),
            language: "English".into(),
        };
        let mut configuration = [0; 30];
        configuration[20] = u32::from(state);
        if state == 6 {
            configuration[1] = 50_f32.to_bits();
            configuration[10] = 10_f32.to_bits();
        }
        let fields = crate::actor_fields::Fields {
            class,
            dispatch_context: 0,
            configuration,
            scale: 1.,
            object_flags: 0,
            response_flags: 0x80000000,
            control_flags: 0,
            action: 0,
            shadow_reciprocal: 0.,
            shadow: None,
            gunlist: Arc::from([]),
            channels: crate::actor_context::initial_channels(Some(1), false),
            position: [1., 2., 3.],
            current: [0.; 3],
            previous: [0.; 3],
            angles: [0.; 3],
        };
        let input = Construction {
            class,
            seed: &seed(state),
            bindings: &[],
            flags: DirectFlags::default(),
            specification: specification(class, vec![]),
            state_resources: Arc::new(
                crate::state_resources::Prepared::load(&resources, class, Profile::Debug).unwrap(),
            ),
            fields,
        };
        if state == 6 {
            session.construct_enemy_projectile(input).unwrap()
        } else if class == 219 && state == 48 {
            session.construct_surface_marker(input).unwrap()
        } else {
            session.construct(input).unwrap()
        }
    }
    fn animation_actor(session: &mut Session) -> Id {
        fixture_actor(session, 73, 23)
    }
    #[test]
    fn mission_declaration_header_follows_validated_construction_and_preserves_scope() {
        use crate::declared_residents::{Key, Scope};
        let mut session = Session::new(187);
        for state in [0, 23] {
            let template = fixture_actor(&mut session, 73, state);
            let body = session.specification(template).unwrap().clone();
            let resources = session.state_resources(template).unwrap().clone();
            let mut fields = session
                .get(template)
                .unwrap()
                .construction_fields
                .clone()
                .unwrap();
            fields.configuration[13] = 17;
            let seed = seed(state);
            let make = |fields| Construction {
                class: 73,
                seed: &seed,
                bindings: &[],
                flags: DirectFlags::default(),
                specification: body.clone(),
                state_resources: resources.clone(),
                fields,
            };
            let world_key = Key {
                scope: Scope::World,
                index: usize::from(state),
            };
            let world = session
                .construct_declaration(world_key, make(fields.clone()))
                .unwrap();
            let mission_key = Key {
                scope: Scope::Mission,
                index: usize::from(state),
            };
            let before = session.counts();
            let next = session.next;
            let mut invalid = fields.clone();
            invalid.configuration[19] = 1;
            assert!(
                session
                    .construct_declaration(mission_key, make(invalid))
                    .is_err()
            );
            assert_eq!(session.counts(), before);
            assert_eq!(session.next, next);
            assert!(session.declaration(mission_key).is_none());
            let mission = session
                .construct_declaration(mission_key, make(fields.clone()))
                .unwrap();
            let w = session
                .get(world)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap();
            let m = session
                .get(mission)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap();
            assert_eq!(w.object_flags, fields.object_flags);
            assert_eq!(w.response_flags, fields.response_flags);
            assert_eq!(m.entry_contribution().word_1c4, 17);
            assert_eq!(
                crate::mission_selection::EntryContribution::total([
                    w.entry_contribution(),
                    m.entry_contribution()
                ]),
                17
            );
            assert_eq!(m.channels, w.channels);
            assert_eq!(m.configuration, w.configuration);
            let before = session.counts();
            assert!(
                session
                    .construct_declaration(mission_key, make(fields))
                    .is_err()
            );
            assert_eq!(session.counts(), before);
            assert_eq!(session.declaration(mission_key), Some(mission));
        }
    }
    #[test]
    fn source_mission_affine_retains_initial_scale_and_refuses_stale_or_failed_publication() {
        use crate::declared_residents::{Key, Scope};
        let mut session = Session::new(188);
        let template = fixture_actor(&mut session, 73, 0);
        let body = session.specification(template).unwrap().clone();
        let resources = session.state_resources(template).unwrap().clone();
        let fields = session
            .get(template)
            .unwrap()
            .construction_fields
            .clone()
            .unwrap();
        let initial = crate::uncached_pose::Pose {
            position: fields.position,
            angles: fields.angles,
            scale: fields.scale,
        };
        let seed = seed(0);
        let make = |fields| Construction {
            class: 73,
            seed: &seed,
            bindings: &[],
            flags: DirectFlags::default(),
            specification: body.clone(),
            state_resources: resources.clone(),
            fields,
        };
        let key = Key {
            scope: Scope::Mission,
            index: 0,
        };
        let before = session.counts();
        let next = session.next;
        assert!(
            session
                .construct_mission_declaration_with_pose(
                    key,
                    make(fields.clone()),
                    &[],
                    crate::uncached_pose::Pose {
                        scale: f32::MAX,
                        ..initial
                    },
                )
                .is_err()
        );
        assert_eq!(session.counts(), before);
        assert_eq!(session.next, next);
        assert!(session.declaration(key).is_none());
        let mut changed = fields.clone();
        changed.scale = 3.;
        let id = session
            .construct_mission_declaration_with_pose(key, make(changed), &[], initial)
            .unwrap();
        assert!(session.actor_current_affine(id).is_err());
        let retained = session.get(id).unwrap().affine_cache.as_ref().unwrap();
        assert_eq!(retained.value.matrix[0], 1.);
        assert_eq!(retained.pose[6], 1_f32.to_bits());
        assert_eq!(
            session
                .get(id)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap()
                .scale,
            3.
        );
        session.prepare_actor_affine(id).unwrap();
        assert_eq!(session.actor_current_affine(id).unwrap().matrix[0], 3.);
        let fresh = session
            .construct_mission_declaration_with_pose(
                Key { index: 1, ..key },
                make(fields.clone()),
                &[],
                initial,
            )
            .unwrap();
        assert_eq!(
            session.actor_current_affine(fresh).unwrap().matrix[3],
            fields.position[0]
        );
        let before = session.counts();
        assert!(
            session
                .construct_mission_declaration_with_pose(key, make(fields), &[], initial)
                .is_err()
        );
        assert_eq!(session.counts(), before);
    }
    #[test]
    fn target_headers_require_owned_identity_and_keep_special_header_unknown() {
        let mut session = Session::new(185);
        let id = fixture_actor(&mut session, 805, 0);
        session
            .residents
            .get_mut(&id.serial)
            .unwrap()
            .construction_fields
            .as_mut()
            .unwrap()
            .dispatch_context = u32::MAX;
        let header = session.mecc_target_header(id).unwrap();
        assert_eq!(header.context, u32::MAX);
        assert!(header.word_e4.is_none());
        session
            .residents
            .get_mut(&id.serial)
            .unwrap()
            .ripper_primary_running = true;
        assert!(session.mecc_target_header(id).is_err());
        session
            .residents
            .get_mut(&id.serial)
            .unwrap()
            .ripper_primary_running = false;
        assert!(
            session
                .mecc_target_header(Id { ticket: 186, ..id })
                .is_err()
        );
        let raw = session
            .register(805, &seed(0), &[], DirectFlags::default())
            .unwrap();
        assert!(session.mecc_target_header(raw).is_err());
        session.clear();
        assert!(session.mecc_target_header(id).is_err());
    }
    #[test]
    fn cold_player_late_child_failure_preserves_pose_channels_and_list_order() {
        let mut sources = Session::new(160);
        let ids = [84, 85, 781].map(|class| fixture_actor(&mut sources, class, 0));
        let child_seed = seed(0);
        let input = |index: usize, bad: bool| {
            let source = sources.get(ids[index]).unwrap();
            let mut fields = source.construction_fields.clone().unwrap();
            if bad {
                fields.class = -1;
            }
            Construction {
                class: source.class,
                seed: &child_seed,
                bindings: &[],
                flags: source.initial_flags,
                specification: sources.specification(ids[index]).unwrap().clone(),
                state_resources: sources.state_resources(ids[index]).unwrap().clone(),
                fields,
            }
        };
        let mut session = Session::new(161);
        let owner = fixture_actor(&mut session, 2, 2);
        session
            .links
            .insert(1, owner.serial, crate::object_links::Position::Tail);
        let actor = session.residents.get_mut(&owner.serial).unwrap();
        let f = actor.construction_fields.as_mut().unwrap();
        f.configuration[12] = 100_f32.to_bits();
        f.angles = [23., 24., 25.];
        f.current = [3., 4., 5.];
        let old_fields = f.clone();
        let old_channels = actor.channels;
        let old_counts = session.counts();
        let old_lists = (0..15).map(|list| session.list(list)).collect::<Vec<_>>();
        assert!(
            session
                .initialize_cold_mecc_player_fields(
                    owner,
                    &seed(2),
                    [input(0, false), input(1, false), input(2, true)],
                    1.5,
                    crate::animation_channels::Blending::Enabled,
                )
                .is_err()
        );
        assert_eq!(session.counts(), old_counts);
        assert_eq!(
            (0..15).map(|list| session.list(list)).collect::<Vec<_>>(),
            old_lists
        );
        let actor = session.get(owner).unwrap();
        let f = actor.construction_fields.as_ref().unwrap();
        assert_eq!(f.angles, old_fields.angles);
        assert_eq!(f.current, old_fields.current);
        assert_eq!(f.previous, old_fields.previous);
        assert_eq!(f.configuration, old_fields.configuration);
        assert_eq!(actor.channels, old_channels);
        let initialized = session
            .initialize_cold_mecc_player_fields(
                owner,
                &seed(2),
                [input(0, false), input(1, false), input(2, false)],
                1.5,
                crate::animation_channels::Blending::Enabled,
            )
            .unwrap();
        assert!(initialized.player_fields && initialized.controlled);
        assert_eq!(
            session.list(2),
            [&[owner], initialized.children.as_slice()].concat()
        );
        assert_eq!(session.list(1), [owner]);
        assert_eq!(session.list(5), [owner]);
        let f = session
            .get(owner)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap();
        assert_eq!(f.angles, [23., 0., 25.]);
        assert_eq!(f.previous, old_fields.current);
        assert_eq!(f.current, old_fields.position);
        assert_eq!(f.configuration[14], old_fields.configuration[14] | 1);
        let stale = session.perception_subset(&[owner]).unwrap();
        let context = MeccDamageContext {
            globally_disabled: false,
            cutscene: false,
            shield_percent: 90.,
            ambient_voice_timer: 0.,
        };
        session
            .damage_unattributed_mecc_nonlethal(owner, 41, 12., context)
            .unwrap();
        assert_eq!(session.mecc_health(owner).unwrap().damage, 12.);
        assert!(
            session
                .validate_perception_frame(&stale.frame().unwrap())
                .is_err()
        );
        let current = session.perception_subset(&[owner]).unwrap();
        session
            .validate_perception_frame(&current.frame().unwrap())
            .unwrap();
        assert!(session.perception_subset(&[owner, owner]).is_err());
        assert!(
            session
                .perception_subset(&[Id {
                    ticket: 160,
                    serial: owner.serial
                }])
                .is_err()
        );
        let body = session
            .get(owner)
            .unwrap()
            .seed
            .storage
            .data()
            .unwrap()
            .to_vec();
        let flags = session.mecc_health(owner).unwrap().flags;
        let counts = session.counts();
        for (amount, context) in [
            (1000., context),
            (f32::NAN, context),
            (
                1.,
                MeccDamageContext {
                    ambient_voice_timer: -1.,
                    ..context
                },
            ),
        ] {
            assert!(
                session
                    .damage_unattributed_mecc_nonlethal(owner, 41, amount, context)
                    .is_err()
            );
            assert_eq!(session.mecc_health(owner).unwrap().damage, 12.);
            assert_eq!(session.mecc_health(owner).unwrap().flags, flags);
            assert_eq!(
                session.get(owner).unwrap().seed.storage.data().unwrap(),
                body
            );
            assert_eq!(session.counts(), counts);
        }

        let before = session.counts();
        assert!(
            session
                .initialize_cold_mecc_player_fields(
                    owner,
                    &seed(2),
                    [input(0, false), input(1, false), input(2, false)],
                    1.5,
                    crate::animation_channels::Blending::Enabled,
                )
                .is_err()
        );
        assert_eq!(session.counts(), before);
    }
    #[test]
    fn mecc_special_late_failure_is_atomic_and_retry_owns_all_three_children() {
        let mut fixtures = Session::new(158);
        let classes = [84, 85, 781];
        let sources = classes.map(|class| fixture_actor(&mut fixtures, class, 0));
        let child_seed = seed(0);
        let input = |index: usize, bad: bool| {
            let id = sources[index];
            let source = fixtures.get(id).unwrap();
            let mut fields = source.construction_fields.clone().unwrap();
            fields.dispatch_context = index as u32 + 1;
            if bad {
                fields.class = 0;
            }
            Construction {
                class: classes[index],
                seed: &child_seed,
                bindings: &[],
                flags: source.initial_flags,
                specification: fixtures.specification(id).unwrap().clone(),
                state_resources: fixtures.state_resources(id).unwrap().clone(),
                fields,
            }
        };
        let mut session = Session::new(159);
        let owner = fixture_actor(&mut session, 2, 2);
        session
            .residents
            .get_mut(&owner.serial)
            .unwrap()
            .construction_fields
            .as_mut()
            .unwrap()
            .dispatch_context = 0x8055_ff42;
        let cold = seed(2);
        let before = session.counts();
        let lists = (0..15).map(|list| session.list(list)).collect::<Vec<_>>();
        let payload = session
            .get(owner)
            .unwrap()
            .seed
            .storage
            .data()
            .unwrap()
            .to_vec();
        let flags = session
            .get(owner)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap()
            .configuration[14];
        let resources = (0..3)
            .map(|i| Arc::strong_count(fixtures.specification(sources[i]).unwrap()))
            .collect::<Vec<_>>();
        assert!(
            session
                .initialize_mecc_special(
                    owner,
                    &cold,
                    [input(0, false), input(1, false), input(2, true)],
                    Some(owner)
                )
                .is_err()
        );
        assert_eq!(session.counts(), before);
        assert_eq!(
            (0..15).map(|list| session.list(list)).collect::<Vec<_>>(),
            lists
        );
        assert_eq!(
            session.get(owner).unwrap().seed.storage.data().unwrap(),
            payload
        );
        assert!(session.mecc_special(owner).is_none());
        assert_eq!(
            (0..3)
                .map(|i| Arc::strong_count(fixtures.specification(sources[i]).unwrap()))
                .collect::<Vec<_>>(),
            resources
        );
        let initialized = session
            .initialize_mecc_special(
                owner,
                &cold,
                [input(0, false), input(1, false), input(2, false)],
                Some(owner),
            )
            .unwrap();
        assert_eq!(
            initialized.children.map(|id| id.serial),
            [owner.serial + 1, owner.serial + 2, owner.serial + 3]
        );
        assert_eq!(
            session.incoming_actor_references(owner).unwrap(),
            initialized.children
        );
        for id in initialized.children {
            assert_eq!(session.incoming_actor_references(id).unwrap(), [owner]);
            assert_eq!(session.mecc_target_header(id).unwrap().context, 0x8055_ff42);
            assert_eq!(session.mecc_special_child(id).unwrap().context, 0x8055_ff42);
        }
        assert_eq!(
            session
                .get(owner)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap()
                .configuration[14],
            flags
        );
        assert!(
            session
                .initialize_mecc_special(
                    owner,
                    &cold,
                    [input(0, false), input(1, false), input(2, false)],
                    Some(owner)
                )
                .is_err()
        );
        assert_eq!(session.counts().residents, 4);
        assert!(
            session
                .mecc_special(Id {
                    ticket: 158,
                    serial: owner.serial
                })
                .is_none()
        );
        session.clear();
        assert_eq!(session.counts().residents, 0);
        assert!(session.mecc_special(owner).is_none());
        assert!(
            session
                .mecc_special_child(initialized.children[0])
                .is_none()
        );
    }
    #[test]
    fn persistent_primary_preserves_pure_retry_and_latches_actual_audio_failure() {
        struct FailAudio(u32);
        impl crate::ripper_primary_turn::Services<Id, crate::attached_effect_context::Id> for FailAudio {
            fn owner_sound(
                &mut self,
                _: &crate::ripper_primary_turn::State<Id, crate::attached_effect_context::Id>,
                cue: u32,
                _: &mut crate::audio_parameters::Random,
            ) -> Result<crate::enemy_projectile::SoundOutcome, String> {
                assert_eq!(cue, 1258);
                self.0 += 1;
                Err("actual audio failure".into())
            }
            fn remove_effect(
                &mut self,
                _: &crate::ripper_primary_turn::State<Id, crate::attached_effect_context::Id>,
                _: crate::attached_effect_context::Id,
                _: &mut crate::audio_parameters::Random,
            ) -> Result<(), String> {
                Err("unexpected retirement".into())
            }
        }
        let mut session = Session::new(155);
        let id = animation_actor(&mut session);
        session.prepare_actor_motion(id).unwrap();
        session.begin_actor_motion_history(id, [0; 8]).unwrap();
        // Explicit unit fixture for an admitted frame. The independent control
        // uses actual resource/bounds/ground initialization instead.
        session.residents.get_mut(&id.serial).unwrap().finalization =
            Some(crate::resident_finalization::Owned {
                state: crate::actor_finalization::State {
                    fields: crate::collider_copy::Fields::zeroed(),
                    previous_physical_velocity: [0.; 3],
                    friction: 0.,
                    parent_present: false,
                    footprint: None,
                    affine: None,
                    physical_argument_word: None,
                },
                phase: crate::resident_finalization::Phase::MotionStarted,
                failure: None,
            });
        session.prepare_ripper_primary(id).unwrap();
        assert!(session.prepare_ripper_primary(id).is_err());
        let frame = crate::ripper_perception::Frame::new(&[], &[]).unwrap();
        let mut context = crate::state23_entry::Context {
            paused: false,
            native_mode: 4,
            parent_state: None,
            player: None,
            delta: 0.1,
        };
        let parameters = crate::ripper_primary_turn::AwakeningContext {
            camera: [0.; 3],
            buried_speed_percent: 50.,
            parameters: crate::ripper_awakening::Parameters {
                camera_range: 100.,
                slope_decision: 0.,
                orbit_radius: [20., 40.],
                seconds: 1.,
            },
        };
        let mut audio = FailAudio(0);
        let mut random = crate::audio_parameters::Random(1);
        for foreign in [false, true] {
            let h = session
                .get(id)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap();
            let candidates = [crate::ripper_perception::Candidate {
                id: if foreign {
                    Id {
                        ticket: 156,
                        serial: id.serial,
                    }
                } else {
                    id
                },
                class: 73,
                state: 23,
                active: h.object_flags,
                personality: h.configuration[14],
                control: h.control_flags,
                position: if foreign { h.position } else { [999.; 3] },
                damage: 0.,
                limit: 0.,
                parent_present: false,
            }];
            let bad_frame = crate::ripper_perception::Frame::new(&candidates, &[]).unwrap();
            assert!(
                session
                    .update_ripper_awakening(
                        id,
                        crate::ripper_primary_turn::AwakeningFrame {
                            context,
                            frame: &bad_frame,
                            range: 500.,
                            parameters
                        },
                        &mut random,
                        &mut audio
                    )
                    .is_err()
            );
            assert!(session.ripper_primary(id).unwrap().failure().is_none());
            assert_eq!(audio.0, 0);
        }

        assert!(
            session
                .update_ripper_awakening(
                    id,
                    crate::ripper_primary_turn::AwakeningFrame {
                        context,
                        frame: &frame,
                        range: 500.,
                        parameters
                    },
                    &mut random,
                    &mut audio
                )
                .is_err()
        );
        assert!(session.ripper_primary(id).unwrap().failure().is_none());
        assert_eq!(audio.0, 0);
        context.native_mode = 0;
        assert_eq!(
            session
                .update_ripper_awakening(
                    id,
                    crate::ripper_primary_turn::AwakeningFrame {
                        context,
                        frame: &frame,
                        range: 500.,
                        parameters
                    },
                    &mut random,
                    &mut audio
                )
                .unwrap_err(),
            "actual audio failure"
        );
        assert_eq!(
            session.ripper_primary(id).unwrap().failure(),
            Some("actual audio failure")
        );
        assert_eq!(
            session.ripper_primary(id).unwrap().state().entry.context,
            100
        );
        assert!(!session.get(id).unwrap().ripper_primary_running);
        assert!(session.mecc_target_header(id).is_err());
        assert!(
            session
                .update_ripper_awakening(
                    id,
                    crate::ripper_primary_turn::AwakeningFrame {
                        context,
                        frame: &frame,
                        range: 500.,
                        parameters
                    },
                    &mut random,
                    &mut audio
                )
                .is_err()
        );
        assert!(
            session
                .advance_ripper_common_motion(id, motion_time())
                .is_err()
        );
        assert_eq!((audio.0, random.0), (1, 1));
        assert!(Session::new(156).ripper_primary(id).is_err());
        let target = animation_actor(&mut session);
        // Explicit typed-reference fixture, including the interval before the
        // completed combat view exists. Destruction must remain conservative.
        let mut retained = session.ripper_primary(id).unwrap().state().clone();
        retained.retaliation = Some(target);
        session
            .residents
            .get_mut(&id.serial)
            .unwrap()
            .ripper_primary = Some(crate::ripper_primary_turn::Machine::new(retained));
        assert_eq!(session.incoming_actor_references(target).unwrap(), vec![id]);
        session.clear();
        assert!(session.ripper_primary(id).is_err());
    }
    #[test]
    fn marker_clock_missing_descriptor_preserves_entire_pending_update() {
        let mut session = Session::new(154);
        let owner = animation_actor(&mut session);
        let marker = fixture_actor(&mut session, 219, 48);
        session.prepare_actor_motion(owner).unwrap();
        session.link_surface_marker(owner, marker).unwrap();
        let fields = session
            .get(marker)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap()
            .words();
        let channels = session.get(marker).unwrap().channels;
        let link = session.surface_link(marker);
        let counts = session.counts();
        assert_eq!(
            session
                .tick_surface_marker(
                    marker,
                    0.1,
                    &[],
                    1.,
                    crate::animation_channels::Blending::Enabled,
                    0.1
                )
                .unwrap_err(),
            "surface clock requires its actual animation descriptors"
        );
        assert_eq!(
            session
                .get(marker)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap()
                .words(),
            fields
        );
        assert_eq!(session.get(marker).unwrap().channels, channels);
        assert_eq!(session.surface_link(marker), link);
        assert_eq!(session.counts(), counts);
    }
    #[test]
    fn surface_lifecycle_uses_owner_xy_waits_past_zero_then_marks_without_unlinking() {
        use crate::animation_channels::Blending;
        let mut session = Session::new(153);
        let owner = animation_actor(&mut session);
        let marker = fixture_actor(&mut session, 219, 48);
        session.prepare_actor_motion(owner).unwrap();
        session.link_surface_marker(owner, marker).unwrap();
        let initial = session
            .get(marker)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap()
            .position;
        session
            .residents
            .get_mut(&marker.serial)
            .unwrap()
            .channels
            .as_mut()
            .unwrap()[0] = [11, 0, 0, 0, 0, 0, 0, 0, 2];
        assert!(
            session
                .update_surface_marker(marker, 0.25, &[], 1., Blending::Disabled)
                .unwrap()
        );
        assert!(matches!(
            session.surface_link(marker),
            Some(SurfaceLink::Marker { mode: 1, .. })
        ));
        session
            .residents
            .get_mut(&owner.serial)
            .unwrap()
            .construction_fields
            .as_mut()
            .unwrap()
            .position[2] += 5.;
        session
            .update_surface_marker(marker, 0.25, &[], 1., Blending::Disabled)
            .unwrap();
        assert!(matches!(
            session.surface_link(marker),
            Some(SurfaceLink::Marker { mode: 1, .. })
        ));
        session
            .residents
            .get_mut(&owner.serial)
            .unwrap()
            .construction_fields
            .as_mut()
            .unwrap()
            .position[0] += 1.;
        session
            .update_surface_marker(marker, 0.25, &[], 1., Blending::Disabled)
            .unwrap();
        session
            .update_surface_marker(marker, 1., &[], 1., Blending::Disabled)
            .unwrap();
        assert!(matches!(
            session.surface_link(marker),
            Some(SurfaceLink::Marker {
                mode: 3,
                timer: 0.,
                ..
            })
        ));
        session
            .update_surface_marker(marker, f32::from_bits(1), &[], 1., Blending::Disabled)
            .unwrap();
        assert!(matches!(
            session.surface_link(marker),
            Some(SurfaceLink::Marker { mode: 2, .. })
        ));
        let a = session.residents.get_mut(&marker.serial).unwrap();
        assert_eq!(a.channels.unwrap()[0][0], 12);
        a.channels.as_mut().unwrap()[0][8] |= 2;
        let counts = session.counts();
        session
            .update_surface_marker(marker, 0.25, &[], 1., Blending::Disabled)
            .unwrap();
        assert_ne!(
            session
                .get(marker)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap()
                .object_flags
                & 0x8000,
            0
        );
        assert!(
            !session
                .update_surface_marker(marker, 0.25, &[], 1., Blending::Disabled)
                .unwrap()
        );
        assert_eq!(session.counts(), counts);
        assert_eq!(
            session
                .get(marker)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap()
                .position,
            initial
        );
        assert_eq!(
            session.incoming_actor_references(marker).unwrap(),
            vec![owner]
        );
        assert_eq!(
            session.incoming_actor_references(owner).unwrap(),
            vec![marker]
        );
    }
    #[test]
    fn linked_surface_construction_refuses_before_publishing_orphan_or_advancing_identity() {
        let mut session = Session::new(152);
        let owner = animation_actor(&mut session);
        session.prepare_actor_motion(owner).unwrap();
        let template = fixture_actor(&mut session, 219, 48);
        let body = session.specification(template).unwrap().clone();
        let states = session.state_resources(template).unwrap().clone();
        let fields = session
            .get(template)
            .unwrap()
            .construction_fields
            .clone()
            .unwrap();
        let mut bad = fields.clone();
        bad.configuration[19] = 123;
        let before = session.counts();
        let next = session.next;
        let seed = seed(48);
        assert!(
            session
                .construct_linked_surface_marker(
                    owner,
                    Construction {
                        class: 219,
                        seed: &seed,
                        bindings: &[],
                        flags: DirectFlags::default(),
                        specification: body.clone(),
                        state_resources: states.clone(),
                        fields: bad,
                    }
                )
                .is_err()
        );
        assert_eq!(session.counts(), before);
        assert_eq!(session.next, next);
        assert!(session.surface_link(owner).is_none());
        assert!(session.state23_combat(owner).is_none());
        let make = || Construction {
            class: 219,
            seed: &seed,
            bindings: &[],
            flags: DirectFlags::default(),
            specification: body.clone(),
            state_resources: states.clone(),
            fields: fields.clone(),
        };
        let marker = session
            .construct_linked_surface_marker(owner, make())
            .unwrap();
        assert_eq!(
            session.surface_link(owner),
            Some(SurfaceLink::Owner { marker })
        );
        let before = session.counts();
        let next = session.next;
        assert!(
            session
                .construct_linked_surface_marker(owner, make())
                .is_err()
        );
        assert_eq!(session.counts(), before);
        assert_eq!(session.next, next);
        assert!(session.surface_link(template).is_none());
        assert_eq!(
            session.incoming_actor_references(owner).unwrap(),
            vec![marker]
        );
    }
    #[test]
    fn surface_reset_preserves_pitch_commands_and_pending_phase_but_clears_physical_history() {
        let mut session = Session::new(151);
        let id = animation_actor(&mut session);
        assert!(session.reset_actor_surface_motion(id).is_err());
        session.prepare_actor_motion(id).unwrap();
        let fields = session
            .residents
            .get_mut(&id.serial)
            .unwrap()
            .construction_fields
            .as_mut()
            .unwrap();
        fields.configuration[0] = 100_f32.to_bits();
        fields.configuration[1] = 50_f32.to_bits();
        session.begin_actor_motion_history(id, [0; 8]).unwrap();
        let mut commands = motion_commands();
        commands.bytes[1] = 32;
        session
            .advance_actor_common_motion(id, commands, motion_time())
            .unwrap();
        assert!(
            session
                .actor_motion(id)
                .unwrap()
                .rates()
                .iter()
                .any(|r| *r != 0.)
        );
        assert_ne!(session.actor_motion(id).unwrap().look_pitch(), 0.);
        session
            .begin_actor_motion_history(id, commands.bytes)
            .unwrap();
        // Explicit finalization fixture: this unit checks ownership/phase
        // plumbing, not native environment preparation or living mission AI.
        let actor = session.residents.get_mut(&id.serial).unwrap();
        let fields = actor.construction_fields.as_mut().unwrap();
        fields.current = [11., 12., 13.];
        fields.previous = [-4., -5., -6.];
        actor.finalization = Some(crate::resident_finalization::Owned {
            state: crate::actor_finalization::State {
                fields: crate::collider_copy::Fields {
                    velocity: [3., 4., 5.],
                    model_vertical_speed: 19.,
                    ..crate::collider_copy::Fields::zeroed()
                },
                previous_physical_velocity: [6., 7., 8.],
                friction: 0.,
                parent_present: false,
                footprint: Some(9),
                affine: None,
                physical_argument_word: None,
            },
            phase: crate::resident_finalization::Phase::MotionStarted,
            failure: None,
        });
        let before = session
            .get(id)
            .unwrap()
            .construction_fields
            .clone()
            .unwrap();
        let pitch = session.actor_motion(id).unwrap().look_pitch();
        let commands_before = session.actor_motion(id).unwrap().commands();
        session.reset_actor_surface_motion(id).unwrap();
        let actor = session.get(id).unwrap();
        let after = actor.construction_fields.as_ref().unwrap();
        assert_eq!(after.current, before.position);
        assert_eq!(after.previous, before.current);
        let mut expected = before;
        expected.previous = expected.current;
        expected.current = expected.position;
        assert_eq!(after.words(), expected.words());
        let motion = session.actor_motion(id).unwrap();
        assert_eq!(motion.rates(), [0.; 4]);
        assert_eq!(motion.previous_rates(), [0.; 4]);
        assert_eq!(motion.look_pitch(), pitch);
        assert_eq!(motion.commands(), commands_before);
        assert!(motion.awaiting_motion());
        let frame = actor.finalization.as_ref().unwrap();
        assert_eq!(
            frame.phase(),
            crate::resident_finalization::Phase::MotionStarted
        );
        assert_eq!(frame.state.fields.velocity, [0.; 3]);
        assert_eq!(frame.state.previous_physical_velocity, [0.; 3]);
        assert_eq!(frame.state.fields.model_vertical_speed, 0.);
        assert_eq!(frame.state.footprint, Some(9));
        let after = after.words();
        assert!(session.reset_actor_surface_motion(id).is_err());
        assert_eq!(
            session
                .get(id)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap()
                .words(),
            after
        );
        session
            .advance_actor_common_motion(id, commands, motion_time())
            .unwrap();
        assert_eq!(
            session.actor_finalization(id).unwrap().phase(),
            crate::resident_finalization::Phase::MotionFinished
        );
    }
    #[test]
    fn surface_links_copy_owned_running_pitch_without_creating_a_transform_parent() {
        let mut session = Session::new(149);
        let owner = animation_actor(&mut session);
        let marker = fixture_actor(&mut session, 219, 48);
        session.prepare_actor_motion(owner).unwrap();
        let fields = session
            .residents
            .get_mut(&owner.serial)
            .unwrap()
            .construction_fields
            .as_mut()
            .unwrap();
        fields.configuration[1] = 50_f32.to_bits();
        session.begin_actor_motion_history(owner, [0; 8]).unwrap();
        let mut commands = motion_commands();
        commands.bytes = [0, 32, 0, 0, 0, 0, 0, 0];
        session
            .advance_actor_common_motion(owner, commands, motion_time())
            .unwrap();
        let pitch = session.actor_motion(owner).unwrap().look_pitch();
        assert_ne!(pitch, 0.);
        session.link_surface_marker(owner, marker).unwrap();
        assert_eq!(
            session.surface_link(marker),
            Some(SurfaceLink::Marker {
                owner,
                look_pitch: pitch,
                mode: 0,
                timer: 0.,
            })
        );
        assert_eq!(session.get(marker).unwrap().presentation_parent, None);
        assert_eq!(session.get(owner).unwrap().presentation_parent, None);
        assert!(session.get(marker).unwrap().attachment.is_none());
        assert_eq!(
            session.incoming_actor_references(owner).unwrap(),
            vec![marker]
        );
        assert_eq!(
            session.incoming_actor_references(marker).unwrap(),
            vec![owner]
        );
        let copied = session
            .get(marker)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap()
            .position;
        session
            .residents
            .get_mut(&owner.serial)
            .unwrap()
            .construction_fields
            .as_mut()
            .unwrap()
            .position = [999.; 3];
        assert_eq!(
            session
                .get(marker)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap()
                .position,
            copied
        );
        session.clear();
        assert!(session.surface_link(marker).is_none());
    }
    #[test]
    fn surface_link_refuses_existing_callback_owner_and_prepared_marker_atomically() {
        let mut session = Session::new(150);
        let owner = animation_actor(&mut session);
        let marker = fixture_actor(&mut session, 219, 48);
        session.prepare_actor_motion(owner).unwrap();
        session.prepare_actor_affine(marker).unwrap();
        let before = session
            .get(marker)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap()
            .words();
        assert!(session.link_surface_marker(owner, marker).is_err());
        assert!(session.surface_link(owner).is_none());
        assert!(session.surface_link(marker).is_none());
        assert!(session.state23_combat(owner).is_none());
        assert_eq!(
            session
                .get(marker)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap()
                .words(),
            before
        );
        session
            .residents
            .get_mut(&marker.serial)
            .unwrap()
            .affine_cache = None;
        let mut combat = session.state23_snapshot(owner).unwrap();
        combat.owner = Some((marker, 219));
        session
            .residents
            .get_mut(&owner.serial)
            .unwrap()
            .state23_combat = Some(combat.clone());
        assert!(session.link_surface_marker(owner, marker).is_err());
        assert_eq!(session.state23_combat(owner), Some(&combat));
        assert!(session.surface_link(owner).is_none());
        assert!(session.surface_link(marker).is_none());
        assert_eq!(
            session
                .get(marker)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap()
                .words(),
            before
        );
    }
    #[test]
    fn primary_reference_reads_only_unchanged_pre_motion_cache_without_unlocking_colliders() {
        let mut session = Session::new(147);
        let id = animation_actor(&mut session);
        session.prepare_actor_affine(id).unwrap();
        let pose = session
            .get(id)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap()
            .position;
        session.residents.get_mut(&id.serial).unwrap().finalization =
            Some(crate::resident_finalization::Owned {
                state: crate::actor_finalization::State {
                    fields: crate::collider_copy::Fields {
                        current: pose,
                        scale: 1.,
                        ..crate::collider_copy::Fields::zeroed()
                    },
                    previous_physical_velocity: [0.; 3],
                    friction: 0.,
                    parent_present: false,
                    footprint: None,
                    affine: None,
                    physical_argument_word: None,
                },
                phase: crate::resident_finalization::Phase::MotionStarted,
                failure: None,
            });
        assert!(session.actor_current_affine(id).is_err());
        assert!(session.current_reference_point(id, 0).is_err());
        assert!(session.prepare_actor_affine(id).is_err());
        // Synthetic spec has no in-range slot0: native semantics use origin.
        assert_eq!(
            session.current_primary_reference_point(id, 0).unwrap(),
            pose
        );
        session
            .residents
            .get_mut(&id.serial)
            .unwrap()
            .construction_fields
            .as_mut()
            .unwrap()
            .position[0] += 1.;
        assert!(session.current_primary_reference_point(id, 0).is_err());
        session
            .residents
            .get_mut(&id.serial)
            .unwrap()
            .construction_fields
            .as_mut()
            .unwrap()
            .object_flags &= !0x20;
        assert!(session.current_primary_reference_point(id, 0).is_err());
        session
            .residents
            .get_mut(&id.serial)
            .unwrap()
            .construction_fields
            .as_mut()
            .unwrap()
            .position = pose;
        for phase in [
            crate::resident_finalization::Phase::History,
            crate::resident_finalization::Phase::MotionFinished,
            crate::resident_finalization::Phase::Failed,
        ] {
            session
                .residents
                .get_mut(&id.serial)
                .unwrap()
                .finalization
                .as_mut()
                .unwrap()
                .phase = phase;
            assert!(session.current_primary_reference_point(id, 0).is_err());
        }
        session.clear();
        assert!(session.current_primary_reference_point(id, 0).is_err());
    }

    #[test]
    fn common_walk_visits_new_resident_and_refuses_foreign_reentrant_and_replayed_calls() {
        use crate::object_list_walk::{Flow, Outcome};
        let mut session = Session::new(173);
        let first = animation_actor(&mut session);
        let second = animation_actor(&mut session);
        let mut walk = session.new_common_walk(16).unwrap();
        let mut foreign = Session::new(174).new_common_walk(16).unwrap();
        assert!(
            session
                .walk_common(&mut foreign, &mut |_, _| panic!("foreign visitor"))
                .is_err()
        );
        let mut visited = Vec::new();
        let mut created = None;
        let result = session
            .walk_common(&mut walk, &mut |session, id| {
                visited.push(id);
                assert!(session.new_common_walk(16).is_err());
                if id == first {
                    created = Some(animation_actor(session));
                }
                Ok(Flow::Continue)
            })
            .unwrap();
        assert_eq!(result, Outcome::Completed);
        assert_eq!(visited, [first, second, created.unwrap()]);
        assert_eq!(walk.visits(), 3);
        assert!(
            session
                .walk_common(&mut walk, &mut |_, _| panic!("replayed visitor"))
                .is_err()
        );
        assert!(session.common_walk_failure().is_none());
        assert!(session.new_common_walk(16).is_ok());
    }
    #[test]
    fn common_walk_failure_quarantines_new_tokens_until_cancellation() {
        use crate::object_list_walk::Flow;
        let mut session = Session::new(175);
        animation_actor(&mut session);
        let mut walk = session.new_common_walk(16).unwrap();
        let mut pending = session.new_common_walk(16).unwrap();
        let before = session.counts().residents;
        assert_eq!(
            session.walk_common(&mut walk, &mut |session, _| {
                animation_actor(session);
                Err("phase provider unavailable".into())
            }),
            Err("phase provider unavailable".into())
        );
        assert_eq!(session.counts().residents, before + 1);
        assert_eq!(
            session.common_walk_failure(),
            Some("phase provider unavailable")
        );
        assert!(session.new_common_walk(16).is_err());
        assert!(
            session
                .walk_common(&mut pending, &mut |_, _| panic!("fresh token bypass"))
                .is_err()
        );
        session.clear();
        assert!(session.common_walk_failure().is_none());
        assert!(
            session
                .walk_common(&mut walk, &mut |_, _| panic!("old failed token"))
                .is_err()
        );
        let mut fresh = session.new_common_walk(16).unwrap();
        assert_eq!(
            session
                .walk_common(&mut fresh, &mut |_, _| Ok(Flow::Continue))
                .unwrap(),
            crate::object_list_walk::Outcome::Completed
        );
        assert_eq!(fresh.visits(), 0);
    }
    #[test]
    fn common_walk_can_cancel_and_stop_but_latches_unbounded_growth_at_host_budget() {
        use crate::object_list_walk::{Flow, Outcome};
        let mut session = Session::new(176);
        animation_actor(&mut session);
        let mut stop = session.new_common_walk(2).unwrap();
        assert_eq!(
            session
                .walk_common(&mut stop, &mut |session, _| {
                    session.clear();
                    assert!(session.new_common_walk(2).is_err());
                    Ok(Flow::Stop)
                })
                .unwrap(),
            Outcome::Stopped
        );
        assert!(session.common_walk_failure().is_none());
        animation_actor(&mut session);
        let mut grow = session.new_common_walk(2).unwrap();
        assert_eq!(
            session.walk_common(&mut grow, &mut |session, _| {
                animation_actor(session);
                Ok(Flow::Continue)
            }),
            Err("list walk visit budget exhausted".into())
        );
        assert_eq!(grow.visits(), 2);
        assert_eq!(session.counts().residents, 3);
        assert_eq!(
            session.common_walk_failure(),
            Some("list walk visit budget exhausted")
        );
        assert!(session.new_common_walk(2).is_err());
    }
    #[test]
    fn common_world_aborts_before_later_actors_or_cleanup_when_selector_disappears() {
        struct GoneSelector {
            selected: Option<Id>,
            primary_calls: usize,
        }
        impl ActorWorldServices for GoneSelector {
            fn controlled_actor(&self) -> Option<Id> {
                self.selected
            }
            fn primary(&mut self, _: &mut Session, _: Id) -> Result<(), String> {
                self.primary_calls += 1;
                self.selected = None;
                Ok(())
            }
            fn secondary(&mut self, _: &mut Session, _: Id) -> Result<(), String> {
                panic!("secondary after abort")
            }
            fn object_time(&mut self, _: &Session, _: Id) -> Result<f32, String> {
                panic!("clock after abort")
            }
            fn animation(&mut self, _: &mut Session, _: Id, _: f32) -> Result<(), String> {
                panic!("animation after abort")
            }
            fn finalization(&mut self, _: &mut Session, _: Id) -> Result<(), String> {
                panic!("finalization after abort")
            }
        }
        let mut session = Session::new(177);
        let first = animation_actor(&mut session);
        let second = animation_actor(&mut session);
        for id in [first, second] {
            let actor = session.residents.get_mut(&id.serial).unwrap();
            actor.construction_fields.as_mut().unwrap().object_flags = 0x840;
            // Synthetic phase admission: selector/lifecycle coverage only.
            actor.finalization = Some(history_fixture(
                actor,
                crate::resident_finalization::Phase::History,
            ));
        }
        let mut walk = session.new_common_walk(16).unwrap();
        let mut services = GoneSelector {
            selected: Some(first),
            primary_calls: 0,
        };
        assert_eq!(
            session
                .update_common_world(&mut walk, &mut services)
                .unwrap(),
            crate::object_list_walk::Outcome::Stopped
        );
        assert_eq!(walk.visits(), 1);
        assert_eq!(services.primary_calls, 1);
        assert_eq!(session.actor_world_flags(first).unwrap(), 0x800);
        assert_eq!(session.actor_world_flags(second).unwrap(), 0x840);
        assert!(session.actor_world_tick(second).is_err());
        assert!(session.common_walk_failure().is_none());
    }
    fn history_fixture(
        actor: &Resident,
        phase: crate::resident_finalization::Phase,
    ) -> crate::resident_finalization::Owned {
        let fields = actor.construction_fields.as_ref().unwrap();
        crate::resident_finalization::Owned {
            state: crate::actor_finalization::State {
                fields: crate::collider_copy::Fields {
                    current: fields.position,
                    previous: fields.current,
                    ..crate::collider_copy::Fields::zeroed()
                },
                previous_physical_velocity: [0.; 3],
                friction: 0.,
                parent_present: false,
                footprint: None,
                affine: None,
                physical_argument_word: None,
            },
            phase,
            failure: None,
        }
    }
    #[test]
    fn position_history_keeps_world_flags_for_the_later_primary_pass() {
        let mut session = Session::new(171);
        let id = animation_actor(&mut session);
        let actor = session.residents.get_mut(&id.serial).unwrap();
        actor.construction_fields.as_mut().unwrap().object_flags = 0x840;
        let mut history = history_fixture(actor, crate::resident_finalization::Phase::Idle);
        let fields = actor.construction_fields.as_mut().unwrap();
        let before = fields.clone();
        history.begin(fields).unwrap();
        assert_eq!(fields.object_flags, 0x840);
        assert_eq!(fields.current, before.position);
        assert_eq!(fields.previous, before.current);
        assert!(history.begin(fields).is_err());
    }
    #[test]
    fn owned_world_failure_cannot_reenter_or_restart_through_fresh_history() {
        struct MissingClock {
            source: Id,
            calls: usize,
        }
        impl ActorWorldServices for MissingClock {
            fn controlled_actor(&self) -> Option<Id> {
                Some(self.source)
            }
            fn primary(&mut self, session: &mut Session, source: Id) -> Result<(), String> {
                self.calls += 1;
                // Reentrant execution must be refused before any callback.
                assert!(session.update_actor_world(source, self).is_err());
                assert_eq!(session.actor_world_flags(source)?, 0x800);
                session
                    .residents
                    .get_mut(&source.serial)
                    .unwrap()
                    .construction_fields
                    .as_mut()
                    .unwrap()
                    .action = 77;
                Ok(())
            }
            fn secondary(&mut self, _: &mut Session, _: Id) -> Result<(), String> {
                self.calls += 1;
                Ok(())
            }
            fn object_time(&mut self, _: &Session, _: Id) -> Result<f32, String> {
                self.calls += 1;
                Err("clock provider unavailable".into())
            }
            fn animation(&mut self, _: &mut Session, _: Id, _: f32) -> Result<(), String> {
                panic!("animation cannot run after clock failure")
            }
            fn finalization(&mut self, _: &mut Session, _: Id) -> Result<(), String> {
                panic!("finalization cannot run after clock failure")
            }
        }
        let mut session = Session::new(172);
        let id = animation_actor(&mut session);
        let actor = session.residents.get_mut(&id.serial).unwrap();
        actor.construction_fields.as_mut().unwrap().object_flags = 0x840;
        // Synthetic phase admission: this unit tests lifecycle and failures,
        // not construction, environment, clock or native behavior fidelity.
        actor.finalization = Some(history_fixture(
            actor,
            crate::resident_finalization::Phase::History,
        ));
        let mut services = MissingClock {
            source: id,
            calls: 0,
        };
        assert_eq!(
            session.update_actor_world(id, &mut services),
            Err("clock provider unavailable".into())
        );
        assert_eq!(
            session.actor_world_tick(id).unwrap().failure(),
            Some("clock provider unavailable")
        );
        assert_eq!(
            session
                .get(id)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap()
                .action,
            77
        );
        assert!(session.update_actor_world(id, &mut services).is_err());
        assert!(session.begin_actor_frame_history(id).is_err());
        assert_eq!(services.calls, 3);
        assert_eq!(session.actor_world_flags(id).unwrap(), 0x800);
        session.clear();
        assert!(session.actor_world_tick(id).is_err());
    }
    fn motion_time() -> crate::resident_motion::Time<'static> {
        crate::resident_motion::Time {
            seconds: 0.1,
            areas: &[],
            control_factor: 1.,
            running_factor: 1.,
            skip_heading: false,
            controlled_actor: false,
        }
    }
    fn motion_commands() -> crate::resident_motion::Commands {
        crate::resident_motion::Commands {
            bytes: [0, 0, 32, 0, 0, 0, 0, 0],
            context: 100,
            target_heading: 0.,
            target_position: [0.; 2],
        }
    }
    #[test]
    fn owned_motion_history_persists_and_contact_cache_becomes_stale() {
        let mut session = Session::new(71);
        let id = animation_actor(&mut session);
        let fields = session
            .residents
            .get_mut(&id.serial)
            .unwrap()
            .construction_fields
            .as_mut()
            .unwrap();
        fields.configuration[0] = 100_f32.to_bits();
        fields.configuration[1] = 50_f32.to_bits();
        session.prepare_actor_motion(id).unwrap();
        session.prepare_actor_affine(id).unwrap();
        let before = session
            .get(id)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap()
            .clone();
        for expected_previous in [0., 10.] {
            assert!(session.begin_actor_motion_history(id, [0; 8]).unwrap());
            assert_eq!(
                session.actor_motion(id).unwrap().previous_rates()[1],
                expected_previous
            );
            assert!(session.begin_actor_motion_history(id, [0; 8]).is_err());
            session
                .advance_actor_common_motion(id, motion_commands(), motion_time())
                .unwrap();
            assert!(!session.actor_motion(id).unwrap().awaiting_motion());
            assert!(
                session
                    .advance_actor_common_motion(id, motion_commands(), motion_time())
                    .is_err()
            );
        }
        let after = session
            .get(id)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap();
        assert_eq!(after.current, before.current);
        assert_eq!(after.previous, before.previous);
        assert_eq!(after.object_flags, before.object_flags);
        assert!(after.position[1] < before.position[1]);
        assert!(session.actor_current_affine(id).is_err());
        session.prepare_actor_affine(id).unwrap();
        assert!(session.actor_current_affine(id).is_ok());
    }
    #[test]
    fn owned_motion_idle_gate_uses_all_eight_commands_and_preserves_history() {
        let mut session = Session::new(72);
        let id = animation_actor(&mut session);
        session.prepare_actor_motion(id).unwrap();
        session
            .residents
            .get_mut(&id.serial)
            .unwrap()
            .construction_fields
            .as_mut()
            .unwrap()
            .configuration[14] |= 0x10;
        assert!(!session.begin_actor_motion_history(id, [0; 8]).unwrap());
        assert!(!session.actor_motion(id).unwrap().awaiting_motion());
        let mut commands = [0; 8];
        commands[7] = 1;
        assert!(session.begin_actor_motion_history(id, commands).unwrap());
        assert_eq!(
            session
                .get(id)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap()
                .configuration[14]
                & 0x10,
            0
        );
        session
            .advance_actor_common_motion(id, motion_commands(), motion_time())
            .unwrap();
        session
            .residents
            .get_mut(&id.serial)
            .unwrap()
            .construction_fields
            .as_mut()
            .unwrap()
            .object_flags |= 0x8000;
        assert!(!session.begin_actor_motion_history(id, commands).unwrap());
    }
    #[test]
    fn owned_motion_errors_publish_no_step_and_keep_history_phase() {
        let mut session = Session::new(73);
        let id = animation_actor(&mut session);
        session.prepare_actor_motion(id).unwrap();
        assert!(
            session
                .advance_actor_common_motion(id, motion_commands(), motion_time())
                .is_err()
        );
        session.begin_actor_motion_history(id, [0; 8]).unwrap();
        let before = session
            .get(id)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap()
            .words();
        let mut bad = motion_time();
        bad.seconds = f32::NAN;
        assert!(
            session
                .advance_actor_common_motion(id, motion_commands(), bad)
                .is_err()
        );
        assert_eq!(
            session
                .get(id)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap()
                .words(),
            before
        );
        assert_eq!(session.actor_motion(id).unwrap().rates(), [0.; 4]);
        assert!(session.actor_motion(id).unwrap().awaiting_motion());
        session
            .advance_actor_common_motion(id, motion_commands(), motion_time())
            .unwrap();
    }
    #[test]
    fn owned_motion_rejects_foreign_cancelled_parented_and_bypass_mode() {
        let mut session = Session::new(74);
        let id = animation_actor(&mut session);
        let mut foreign = Session::new(75);
        assert!(foreign.prepare_actor_motion(id).is_err());
        assert!(foreign.begin_actor_motion_history(id, [0; 8]).is_err());
        session.prepare_actor_motion(id).unwrap();
        assert!(session.prepare_actor_motion(id).is_err());
        session
            .residents
            .get_mut(&id.serial)
            .unwrap()
            .presentation_parent = Some(id);
        assert!(session.begin_actor_motion_history(id, [0; 8]).is_err());
        session
            .residents
            .get_mut(&id.serial)
            .unwrap()
            .presentation_parent = None;
        let mut state = session.state23_snapshot(id).unwrap();
        state.parent = Some(id);
        session
            .residents
            .get_mut(&id.serial)
            .unwrap()
            .state23_combat = Some(state.clone());
        assert!(session.begin_actor_motion_history(id, [0; 8]).is_err());
        state.parent = None;
        state.mode = 20;
        session
            .residents
            .get_mut(&id.serial)
            .unwrap()
            .state23_combat = Some(state);
        session.begin_actor_motion_history(id, [0; 8]).unwrap();
        assert!(
            session
                .advance_actor_common_motion(id, motion_commands(), motion_time())
                .is_err()
        );
        assert!(session.actor_motion(id).unwrap().awaiting_motion());
        session.clear();
        assert!(session.actor_motion(id).is_err());
        assert!(
            session
                .advance_actor_common_motion(id, motion_commands(), motion_time())
                .is_err()
        );
    }
    fn controlled_frame() -> crate::controlled_mecc::Frame {
        crate::controlled_mecc::Frame {
            before: [1., 2., 3.],
            position: [2., 3., 4.],
            contact_previous: [1.5, 2., 3.],
            angles: [90., 0., 0.],
            scale: 1.,
            velocity: [1., 2., 3.],
            model_rates: [3., 2., 1.],
        }
    }
    #[test]
    fn controlled_motion_keeps_contact_history_and_requires_fresh_affine() {
        let mut session = Session::new(181);
        let id = fixture_actor(&mut session, 2, 2);
        session.prepare_actor_affine(id).unwrap();
        let counts = session.counts();
        let spec = session.specification(id).unwrap().clone();
        let payload = session
            .get(id)
            .unwrap()
            .seed
            .storage
            .data()
            .unwrap()
            .to_vec();
        let flags = session
            .get(id)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap()
            .object_flags;
        let first = controlled_frame();
        session
            .publish_controlled_mecc_motion(id, 1, first)
            .unwrap();
        let fields = session
            .get(id)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap();
        assert_eq!(fields.position, first.position);
        assert_eq!(
            fields.current, first.contact_previous,
            "contact correction must survive publication"
        );
        assert_eq!(fields.previous, [0.; 3]);
        assert_eq!(
            fields.object_flags, flags,
            "do not fabricate cache or dirty flags"
        );
        assert!(session.actor_current_affine(id).is_err());
        assert_eq!(
            session
                .controlled_mecc_motion(id, 1)
                .unwrap()
                .frame
                .model_rates,
            first.model_rates
        );
        assert!(session.controlled_mecc_motion(id, 2).is_err());
        session.prepare_actor_affine(id).unwrap();
        assert_eq!(
            session.actor_current_affine(id).unwrap().matrix[3],
            first.position[0]
        );
        let second = crate::controlled_mecc::Frame {
            before: first.position,
            position: [3., 4., 5.],
            ..first
        };
        session
            .publish_controlled_mecc_motion(id, 2, second)
            .unwrap();
        assert_eq!(
            session
                .get(id)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap()
                .previous,
            first.contact_previous
        );
        assert!(session.controlled_mecc_motion(id, 1).is_err());
        assert!(session.controlled_mecc_motion(id, 2).is_ok());
        assert_eq!(session.counts(), counts);
        assert!(Arc::ptr_eq(session.specification(id).unwrap(), &spec));
        assert_eq!(
            session.get(id).unwrap().seed.storage.data().unwrap(),
            payload
        );
    }
    #[test]
    fn controlled_motion_refuses_discontinuity_foreign_parent_and_invalid_fields_atomically() {
        let mut session = Session::new(182);
        let id = fixture_actor(&mut session, 2, 2);
        let first = controlled_frame();
        for (step, frame) in [
            (0, first),
            (2, first),
            (
                1,
                crate::controlled_mecc::Frame {
                    before: [0.; 3],
                    ..first
                },
            ),
            (1, crate::controlled_mecc::Frame { scale: 2., ..first }),
            (
                1,
                crate::controlled_mecc::Frame {
                    velocity: [f32::NAN; 3],
                    ..first
                },
            ),
        ] {
            assert!(
                session
                    .publish_controlled_mecc_motion(id, step, frame)
                    .is_err()
            );
            assert_eq!(
                session
                    .get(id)
                    .unwrap()
                    .construction_fields
                    .as_ref()
                    .unwrap()
                    .position,
                first.before
            );
            assert!(session.controlled_mecc_motion(id, 1).is_err());
        }
        assert!(
            Session::new(183)
                .publish_controlled_mecc_motion(id, 1, first)
                .is_err()
        );
        let parent = fixture_actor(&mut session, 2, 2);
        session.set_presentation_parent(id, Some(parent)).unwrap();
        assert!(
            session
                .publish_controlled_mecc_motion(id, 1, first)
                .is_err()
        );
        session.set_presentation_parent(id, None).unwrap();
        session
            .publish_controlled_mecc_motion(id, 1, first)
            .unwrap();
        assert!(
            session
                .publish_controlled_mecc_motion(id, 1, first)
                .is_err()
        );
        session.set_presentation_parent(id, Some(parent)).unwrap();
        assert!(session.controlled_mecc_motion(id, 1).is_err());
        session.set_presentation_parent(id, None).unwrap();
        assert!(session.controlled_mecc_motion(id, 1).is_ok());
        session
            .residents
            .get_mut(&id.serial)
            .unwrap()
            .construction_fields
            .as_mut()
            .unwrap()
            .position[0] += 1.;
        assert!(session.controlled_mecc_motion(id, 1).is_err());
        session.clear();
        assert!(session.controlled_mecc_motion(id, 1).is_err());
    }
    struct FireServices {
        events: Vec<&'static str>,
        fail: Option<&'static str>,
        cancel: bool,
        expected_action: u32,
        audio_random: crate::audio_parameters::Random,
    }
    impl crate::enemy_projectile::Services for FireServices {
        fn prepare_effect(
            &mut self,
            session: &mut Session,
            r: crate::enemy_projectile::EffectRequest<'_>,
            random: &mut crate::audio_parameters::Random,
        ) -> Result<crate::enemy_projectile::EffectOutcome, String> {
            self.events.push("effect");
            assert_eq!(r.flags, 4);
            assert_eq!(r.tag, u32::MAX);
            assert_eq!(
                session
                    .get(r.recipient)
                    .unwrap()
                    .construction_fields
                    .as_ref()
                    .unwrap()
                    .action,
                self.expected_action
            );
            for _ in 0..2 {
                random.next_15();
            }
            if self.cancel {
                session.clear();
            }
            if self.fail == Some("effect") {
                return Err("effect backend failed after an external attempt".into());
            }
            Ok(crate::enemy_projectile::EffectOutcome::Rejected)
        }
        fn sound(
            &mut self,
            _: &mut Session,
            _: u32,
            _: [f32; 3],
        ) -> Result<crate::enemy_projectile::SoundOutcome, String> {
            self.events.push("sound");
            // Synthetic loaded audio service, distinct from the EXE RNG.
            self.audio_random.next_15();
            if self.fail == Some("sound") {
                return Err("sound backend failed".into());
            }
            Ok(crate::enemy_projectile::SoundOutcome::ResourceUnavailable)
        }
        fn muzzle(
            &mut self,
            _: &mut Session,
            _: Id,
            reference: i32,
            random: &mut crate::audio_parameters::Random,
        ) -> Result<[f32; 3], String> {
            self.events.push("muzzle");
            assert_eq!(reference, 0);
            for _ in 0..4 {
                random.next_15();
            }
            if self.fail == Some("muzzle") {
                return Err("reference backend failed".into());
            }
            Ok([0.; 3])
        }
    }
    fn fire_input(source: Id) -> crate::enemy_projectile::LaunchInput {
        crate::enemy_projectile::LaunchInput {
            source,
            target: [30., 40., 20.],
            vertical_spread_percent: 0.,
            dispatch_context: 0x12345678,
        }
    }
    fn fire_services(action: u32) -> FireServices {
        FireServices {
            events: vec![],
            fail: None,
            cancel: false,
            expected_action: action,
            audio_random: crate::audio_parameters::Random(1),
        }
    }
    impl crate::enemy_projectile::GroundImpactServices for FireServices {
        fn effect(
            &mut self,
            session: &mut Session,
            request: crate::enemy_projectile::ImpactEffect,
            random: &mut crate::audio_parameters::Random,
        ) -> Result<crate::enemy_projectile::EffectOutcome, String> {
            self.events.push("impact_effect");
            assert_eq!(request.flags, 8);
            assert_eq!(
                request.name,
                if self.expected_action == 1 {
                    "fx_ripper_large_explode"
                } else {
                    "fx_ripper_medium_explode"
                }
            );
            assert_eq!(request.position, [4., 5., 0.]);
            for _ in 0..2 {
                random.next_15();
            }
            if self.cancel {
                session.clear();
            }
            if self.fail == Some("impact_effect") {
                return Err("impact effect backend failed".into());
            }
            Ok(crate::enemy_projectile::EffectOutcome::Rejected)
        }
        fn radial(
            &mut self,
            session: &mut Session,
            request: crate::enemy_projectile::RadialRequest,
            random: &mut crate::audio_parameters::Random,
        ) -> Result<(), String> {
            self.events.push("radial");
            assert_eq!(request.context, 0x12345678);
            assert_eq!(request.position, [4., 5., 0.]);
            assert!(request.other.is_none());
            assert!(session.enemy_projectile(request.source).is_some());
            for _ in 0..3 {
                random.next_15();
            }
            if self.fail == Some("radial") {
                return Err("radial backend failed".into());
            }
            Ok(())
        }
        fn sound(
            &mut self,
            _: &mut Session,
            _: u32,
            position: [f32; 3],
        ) -> Result<crate::enemy_projectile::SoundOutcome, String> {
            self.events.push("impact_sound");
            assert_eq!(position, [4., 5., 0.]);
            self.audio_random.next_15();
            if self.fail == Some("impact_sound") {
                return Err("impact sound backend failed".into());
            }
            Ok(crate::enemy_projectile::SoundOutcome::ResourceUnavailable)
        }
    }
    fn impact_fixture(session: &mut Session) -> (Id, crate::audio_parameters::Random) {
        let source = fixture_actor(session, 230, 23);
        let shot = fixture_actor(session, 232, 6);
        session
            .residents
            .get_mut(&shot.serial)
            .unwrap()
            .seed
            .storage
            .data_mut()
            .unwrap()[28..32]
            .copy_from_slice(&(-1f32).to_le_bytes());
        let mut random = crate::audio_parameters::Random(17);
        session
            .finish_enemy_projectile_fire(
                shot,
                fire_input(source),
                &mut random,
                &mut fire_services(1),
            )
            .unwrap();
        (shot, random)
    }
    #[derive(Default)]
    struct RetirementMock {
        calls: usize,
        fail: bool,
        cancel: bool,
        reference_from: Option<Id>,
    }
    impl crate::enemy_projectile::RetirementServices for RetirementMock {
        fn release(&mut self, session: &mut Session, id: Id) -> Result<(), String> {
            self.calls += 1;
            assert_eq!(
                session.enemy_projectile(id).unwrap().phase,
                crate::enemy_projectile::Phase::CleanupServices
            );
            if let Some(other) = self.reference_from {
                session.set_presentation_parent(other, Some(id))?;
            }
            if self.cancel {
                session.clear();
            }
            if self.fail {
                return Err("tracked external release failed".into());
            }
            Ok(())
        }
    }
    fn retiring_fixture(session: &mut Session) -> Id {
        let (shot, mut random) = impact_fixture(session);
        session
            .enemy_projectile_ground_impact(shot, [4., 5., 0.], &mut random, &mut fire_services(1))
            .unwrap();
        shot
    }
    #[test]
    fn pending_enemy_branches_cannot_be_replaced_by_ground_or_cleanup_success() {
        use crate::enemy_projectile::FlightOutcome;
        let mut session = Session::new(177);
        let (shot, mut random) = impact_fixture(&mut session);
        for pending in [
            FlightOutcome::Expired,
            FlightOutcome::ObjectContact {
                target: shot,
                point: [4., 5., 0.],
                tag: 0,
            },
            FlightOutcome::GroundContact {
                point: [9., 5., 0.],
            },
        ] {
            session
                .residents
                .get_mut(&shot.serial)
                .unwrap()
                .enemy_projectile
                .as_mut()
                .unwrap()
                .pending_flight = Some(pending);
            let before = random.0;
            let mut services = fire_services(1);
            assert!(
                session
                    .enemy_projectile_ground_impact(shot, [4., 5., 0.], &mut random, &mut services)
                    .is_err()
            );
            assert_eq!(random.0, before);
            assert!(services.events.is_empty());
        }
        session
            .residents
            .get_mut(&shot.serial)
            .unwrap()
            .enemy_projectile
            .as_mut()
            .unwrap()
            .pending_flight = None;
        session
            .enemy_projectile_ground_impact(shot, [4., 5., 0.], &mut random, &mut fire_services(1))
            .unwrap();
        session
            .residents
            .get_mut(&shot.serial)
            .unwrap()
            .enemy_projectile
            .as_mut()
            .unwrap()
            .pending_flight = Some(FlightOutcome::Expired);
        let mut release = RetirementMock::default();
        assert!(
            session
                .cleanup_enemy_projectile(shot, &mut release)
                .is_err()
        );
        assert_eq!(release.calls, 0);
        assert!(session.get(shot).is_some());
    }
    #[test]
    fn pending_enemy_object_contact_keeps_its_target_owned() {
        let mut session = Session::new(178);
        let target = retiring_fixture(&mut session);
        let (shot, _) = impact_fixture(&mut session);
        session
            .residents
            .get_mut(&shot.serial)
            .unwrap()
            .enemy_projectile
            .as_mut()
            .unwrap()
            .pending_flight = Some(crate::enemy_projectile::FlightOutcome::ObjectContact {
            target,
            point: [1., 2., 3.],
            tag: 0,
        });
        assert!(
            session
                .incoming_actor_references(target)
                .unwrap()
                .contains(&shot)
        );
        let mut release = RetirementMock::default();
        assert!(
            session
                .cleanup_enemy_projectile(target, &mut release)
                .is_err()
        );
        assert_eq!(release.calls, 0);
        assert!(session.get(target).is_some());
    }
    #[test]
    fn enemy_cleanup_releases_only_target_ownership_and_never_reuses_identity() {
        let mut session = Session::new(159);
        let shot = retiring_fixture(&mut session);
        let source = session
            .enemy_projectile(shot)
            .unwrap()
            .launch
            .as_ref()
            .unwrap()
            .source;
        let body = session.specification(shot).unwrap().clone();
        let states = session.state_resources(shot).unwrap().clone();
        let survivor = session.specification(source).unwrap().clone();
        let before = (
            Arc::strong_count(&body),
            Arc::strong_count(&states),
            Arc::strong_count(&survivor),
        );
        let counts = session.counts();
        let order = session
            .links
            .list(0)
            .into_iter()
            .filter(|id| *id != shot.serial)
            .collect::<Vec<_>>();
        let mut services = RetirementMock::default();
        assert_eq!(
            session
                .cleanup_enemy_projectile(shot, &mut services)
                .unwrap(),
            Counts {
                residents: 1,
                payload_bytes: 36,
                registrations: 1
            }
        );
        assert_eq!(services.calls, 1);
        assert!(session.get(shot).is_none());
        assert!(session.specification(shot).is_none() && session.state_resources(shot).is_none());
        assert_eq!(
            session.counts(),
            Counts {
                residents: counts.residents - 1,
                payload_bytes: counts.payload_bytes - 36,
                registrations: counts.registrations - 1
            }
        );
        assert_eq!(session.links.list(0), order);
        assert_eq!(
            (
                Arc::strong_count(&body),
                Arc::strong_count(&states),
                Arc::strong_count(&survivor)
            ),
            (before.0 - 1, before.1 - 1, before.2)
        );
        let next = fixture_actor(&mut session, 232, 6);
        assert!(next.serial > shot.serial);
        assert!(
            session
                .cleanup_enemy_projectile(shot, &mut services)
                .is_err()
        );
        assert_eq!(services.calls, 1);
        assert!(session.get(source).is_some());
    }
    #[test]
    fn enemy_cleanup_preflight_preserves_unmarked_foreign_and_referenced_actors() {
        let mut session = Session::new(160);
        let (shot, mut random) = impact_fixture(&mut session);
        let mut services = RetirementMock::default();
        assert!(
            session
                .cleanup_enemy_projectile(shot, &mut services)
                .is_err()
        );
        assert!(
            Session::new(161)
                .cleanup_enemy_projectile(shot, &mut services)
                .is_err()
        );
        session
            .enemy_projectile_ground_impact(shot, [4., 5., 0.], &mut random, &mut fire_services(1))
            .unwrap();
        let source = session
            .enemy_projectile(shot)
            .unwrap()
            .launch
            .as_ref()
            .unwrap()
            .source;
        session.set_presentation_parent(source, Some(shot)).unwrap();
        let counts = session.counts();
        assert_eq!(session.incoming_actor_references(shot).unwrap(), [source]);
        assert!(
            session
                .cleanup_enemy_projectile(shot, &mut services)
                .is_err()
        );
        assert_eq!(services.calls, 0);
        assert_eq!(session.counts(), counts);
        assert_eq!(
            session.enemy_projectile(shot).unwrap().phase,
            crate::enemy_projectile::Phase::Retiring
        );
        session.set_presentation_parent(source, None).unwrap();
        assert!(
            session
                .cleanup_enemy_projectile(shot, &mut services)
                .is_ok()
        );
        assert_eq!(services.calls, 1);
    }
    #[test]
    fn enemy_cleanup_failed_external_release_keeps_core_storage_without_replay() {
        let mut session = Session::new(162);
        let shot = retiring_fixture(&mut session);
        let counts = session.counts();
        let mut services = RetirementMock {
            fail: true,
            ..Default::default()
        };
        assert!(
            session
                .cleanup_enemy_projectile(shot, &mut services)
                .is_err()
        );
        assert_eq!(session.counts(), counts);
        assert!(session.specification(shot).is_some());
        assert_eq!(
            session.enemy_projectile(shot).unwrap().phase,
            crate::enemy_projectile::Phase::Failed
        );
        assert!(
            session
                .cleanup_enemy_projectile(shot, &mut services)
                .is_err()
        );
        assert_eq!(services.calls, 1);
        session.clear();
        assert_eq!(session.counts(), Counts::default());
    }
    #[test]
    fn enemy_cleanup_rechecks_references_and_detects_external_cancellation() {
        for cancel in [false, true] {
            let mut session = Session::new(163);
            let shot = retiring_fixture(&mut session);
            let source = session
                .enemy_projectile(shot)
                .unwrap()
                .launch
                .as_ref()
                .unwrap()
                .source;
            let counts = session.counts();
            let mut services = RetirementMock {
                cancel,
                reference_from: if cancel { None } else { Some(source) },
                ..Default::default()
            };
            assert!(
                session
                    .cleanup_enemy_projectile(shot, &mut services)
                    .is_err()
            );
            assert_eq!(services.calls, 1);
            if cancel {
                assert_eq!(session.counts(), Counts::default());
                assert!(session.get(shot).is_none());
            } else {
                assert_eq!(session.counts(), counts);
                assert_eq!(session.incoming_actor_references(shot).unwrap(), [source]);
                assert_eq!(
                    session.enemy_projectile(shot).unwrap().phase,
                    crate::enemy_projectile::Phase::Failed
                );
            }
        }
    }
    #[test]
    fn enemy_ground_reaction_orders_services_and_marks_without_unlinking() {
        use crate::enemy_projectile::Phase;
        let mut session = Session::new(155);
        let (shot, mut random) = impact_fixture(&mut session);
        let counts = session.counts();
        let mut expected = random;
        for _ in 0..6 {
            expected.next_15();
        }
        let mut services = fire_services(1);
        session
            .enemy_projectile_ground_impact(shot, [4., 5., 0.], &mut random, &mut services)
            .unwrap();
        assert_eq!(services.events, ["impact_effect", "radial", "impact_sound"]);
        assert_eq!(random.0, expected.0);
        assert_eq!(session.counts(), counts);
        assert_eq!(
            session
                .get(shot)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap()
                .object_flags
                & 0x8000,
            0x8000
        );
        let metadata = session.enemy_projectile(shot).unwrap();
        assert_eq!(metadata.phase, Phase::Retiring);
        assert_eq!(
            metadata.ground_impact.as_ref().unwrap().effect,
            crate::enemy_projectile::EffectOutcome::Rejected
        );
        let before = (random.0, services.audio_random.0, services.events.len());
        assert!(
            session
                .enemy_projectile_ground_impact(shot, [4., 5., 0.], &mut random, &mut services)
                .is_err()
        );
        assert_eq!(
            (random.0, services.audio_random.0, services.events.len()),
            before
        );
    }
    #[test]
    fn enemy_ground_reaction_latches_failed_external_attempts() {
        for failure in ["impact_effect", "radial", "impact_sound"] {
            let mut session = Session::new(156);
            let (shot, mut random) = impact_fixture(&mut session);
            let mut services = fire_services(1);
            services.fail = Some(failure);
            assert!(
                session
                    .enemy_projectile_ground_impact(shot, [4., 5., 0.], &mut random, &mut services)
                    .is_err()
            );
            assert_eq!(
                session.enemy_projectile(shot).unwrap().phase,
                crate::enemy_projectile::Phase::Failed
            );
            assert_eq!(
                session
                    .get(shot)
                    .unwrap()
                    .construction_fields
                    .as_ref()
                    .unwrap()
                    .object_flags
                    & 0x8000,
                0
            );
            let before = (random.0, services.events.len());
            assert!(
                session
                    .enemy_projectile_ground_impact(shot, [4., 5., 0.], &mut random, &mut services)
                    .is_err()
            );
            assert_eq!((random.0, services.events.len()), before);
        }
    }
    #[test]
    fn enemy_ground_reaction_preflight_and_cancellation_never_acknowledge_retirement() {
        let mut session = Session::new(157);
        let (shot, mut random) = impact_fixture(&mut session);
        let mut services = fire_services(1);
        let before = random.0;
        assert!(
            session
                .enemy_projectile_ground_impact(
                    shot,
                    [f32::NAN, 5., 0.],
                    &mut random,
                    &mut services
                )
                .is_err()
        );
        assert!(
            Session::new(158)
                .enemy_projectile_ground_impact(shot, [4., 5., 0.], &mut random, &mut services)
                .is_err()
        );
        assert_eq!(random.0, before);
        assert!(services.events.is_empty());
        assert_eq!(
            session.enemy_projectile(shot).unwrap().phase,
            crate::enemy_projectile::Phase::Launched
        );
        services.cancel = true;
        assert!(
            session
                .enemy_projectile_ground_impact(shot, [4., 5., 0.], &mut random, &mut services)
                .is_err()
        );
        assert!(session.get(shot).is_none());
        assert_eq!(services.events, ["impact_effect"]);
    }
    #[test]
    fn enemy_fire_preserves_order_separate_audio_rng_and_native_rate_family() {
        let mut session = Session::new(151);
        let source = fixture_actor(&mut session, 230, 23);
        let shot = fixture_actor(&mut session, 232, 6);
        let mut random = crate::audio_parameters::Random(17);
        let mut services = fire_services(1);
        session
            .finish_enemy_projectile_fire(shot, fire_input(source), &mut random, &mut services)
            .unwrap();
        assert_eq!(services.events, ["effect", "sound", "muzzle"]);
        let mut expected = crate::audio_parameters::Random(17);
        for _ in 0..8 {
            expected.next_15();
        }
        assert_eq!(random.0, expected.0);
        let mut audio_expected = crate::audio_parameters::Random(1);
        audio_expected.next_15();
        assert_eq!(services.audio_random.0, audio_expected.0);
        let metadata = session.enemy_projectile(shot).unwrap();
        let launch = metadata.launch.as_ref().unwrap();
        assert_eq!(metadata.phase, crate::enemy_projectile::Phase::Launched);
        assert_eq!(
            metadata.effect,
            Some(crate::enemy_projectile::EffectOutcome::Rejected)
        );
        assert_eq!(launch.rates, [25., 50., 0., 0.]);
        assert_eq!(launch.rates, launch.previous_rates);
        assert_eq!(launch.source, source);
        assert_eq!(launch.dispatch_context, 0x12345678);
        assert_eq!(
            session.mecc_target_header(shot).unwrap().context,
            0x12345678
        );
        assert_eq!(
            &session.get(shot).unwrap().seed.storage.data().unwrap()[..8],
            [2_f32.to_le_bytes(), 2_f32.to_le_bytes()].concat()
        );
        let before = random.0;
        assert!(
            session
                .finish_enemy_projectile_fire(shot, fire_input(source), &mut random, &mut services)
                .is_err()
        );
        assert_eq!(random.0, before);
        assert_eq!(services.events.len(), 3);
    }
    #[test]
    fn enemy_fire_external_failures_latch_without_replay_or_numeric_publication() {
        for failure in ["effect", "sound", "muzzle"] {
            let mut session = Session::new(152);
            let source = animation_actor(&mut session);
            let shot = fixture_actor(&mut session, 90, 6);
            let initial = session
                .get(shot)
                .unwrap()
                .seed
                .storage
                .data()
                .unwrap()
                .to_vec();
            let mut random = crate::audio_parameters::Random(1);
            let mut services = fire_services(0);
            services.fail = Some(failure);
            assert!(
                session
                    .finish_enemy_projectile_fire(
                        shot,
                        fire_input(source),
                        &mut random,
                        &mut services
                    )
                    .is_err()
            );
            let metadata = session.enemy_projectile(shot).unwrap();
            assert_eq!(metadata.phase, crate::enemy_projectile::Phase::Failed);
            assert!(metadata.launch.is_none() && metadata.failure.is_some());
            assert_eq!(
                session.get(shot).unwrap().seed.storage.data().unwrap(),
                initial
            );
            let before = (random.0, services.audio_random.0, services.events.len());
            assert!(
                session
                    .finish_enemy_projectile_fire(
                        shot,
                        fire_input(source),
                        &mut random,
                        &mut services
                    )
                    .is_err()
            );
            assert_eq!(
                (random.0, services.audio_random.0, services.events.len()),
                before
            );
        }
    }
    #[test]
    fn enemy_fire_cancellation_and_preflight_refusals_do_not_acknowledge_launch() {
        let mut session = Session::new(153);
        let source = animation_actor(&mut session);
        let shot = fixture_actor(&mut session, 90, 6);
        let mut foreign = Session::new(154);
        let mut random = crate::audio_parameters::Random(1);
        let mut services = fire_services(0);
        assert!(
            foreign
                .finish_enemy_projectile_fire(shot, fire_input(source), &mut random, &mut services)
                .is_err()
        );
        let mut bad = fire_input(source);
        bad.target[0] = f32::NAN;
        assert!(
            session
                .finish_enemy_projectile_fire(shot, bad, &mut random, &mut services)
                .is_err()
        );
        assert_eq!(random.0, 1);
        assert!(services.events.is_empty());
        assert_eq!(
            session.enemy_projectile(shot).unwrap().phase,
            crate::enemy_projectile::Phase::Constructed
        );
        session.set_presentation_parent(shot, Some(source)).unwrap();
        assert!(
            session
                .finish_enemy_projectile_fire(shot, fire_input(source), &mut random, &mut services)
                .is_err()
        );
        assert_eq!(random.0, 1);
        session.set_presentation_parent(shot, None).unwrap();
        services.cancel = true;
        assert!(
            session
                .finish_enemy_projectile_fire(shot, fire_input(source), &mut random, &mut services)
                .is_err()
        );
        assert!(session.enemy_projectile(shot).is_none());
        assert_eq!(services.events, ["effect"]);
    }
    struct AnimationService {
        calls: u32,
        refuse: bool,
    }
    impl State23AnimationServices for AnimationService {
        fn fire(
            &mut self,
            session: &mut Session,
            source: Id,
            _: &crate::state23_animation::State,
            target: [f32; 3],
            _: f32,
            random: &mut crate::audio_parameters::Random,
        ) -> Result<crate::state23_animation::FireOutcome, String> {
            assert!(session.get(source).is_some());
            assert_eq!(target, [1., 2., 3.]);
            self.calls += 1;
            random.next_15();
            if self.refuse {
                Err("projectile backend partially failed".into())
            } else {
                Ok(crate::state23_animation::FireOutcome::ResourceUnavailable)
            }
        }
    }
    fn animation_context() -> crate::state23_animation::Context {
        crate::state23_animation::Context {
            parent_state: None,
            target_position: None,
            leap_reference_speed: 0.,
            leap_model_speed: 0.,
            vertical_spread_percent: 10.,
            cooldown: [1., 2.],
            repeat_timer: 3.,
            blending: crate::animation_channels::Blending::Enabled,
        }
    }
    #[test]
    fn animation_publication_uses_owned_channels_and_refuses_foreign_ids_or_missing_objectives() {
        let mut session = Session::new(96);
        let actor = animation_actor(&mut session);
        let mut combat = session.state23_snapshot(actor).unwrap();
        combat.mode = 31;
        session
            .residents
            .get_mut(&actor.serial)
            .unwrap()
            .state23_combat = Some(combat.clone());
        let mut random = crate::audio_parameters::Random(7);
        let mut service = AnimationService {
            calls: 0,
            refuse: false,
        };
        session
            .update_state23_animation(actor, animation_context(), &mut random, &mut service)
            .unwrap();
        assert_eq!(session.get(actor).unwrap().channels.unwrap()[0][0], 42);
        assert_eq!(session.state23_combat(actor).unwrap().mode, 31);
        assert_eq!(service.calls, 0);
        let mut foreign = Session::new(97);
        let other = animation_actor(&mut foreign);
        let old = session.get(actor).unwrap().channels;
        combat.retaliation = Some(other);
        session
            .residents
            .get_mut(&actor.serial)
            .unwrap()
            .state23_combat = Some(combat);
        assert!(
            session
                .update_state23_animation(actor, animation_context(), &mut random, &mut service)
                .is_err()
        );
        assert!(
            session
                .update_state23_animation(other, animation_context(), &mut random, &mut service)
                .is_err()
        );
        assert_eq!(session.get(actor).unwrap().channels, old);
        assert_eq!(random.0, 7);
        assert!(!session.get(actor).unwrap().state23_animation_failed);
        session.clear();
        assert!(
            session
                .update_state23_animation(actor, animation_context(), &mut random, &mut service)
                .is_err()
        );
    }
    #[test]
    fn animation_failed_external_service_latches_without_publishing_or_replaying_it() {
        let mut session = Session::new(98);
        let actor = animation_actor(&mut session);
        let target = animation_actor(&mut session);
        let mut combat = session.state23_snapshot(actor).unwrap();
        combat.mode = 21;
        combat.retaliation = Some(target);
        let r = session.residents.get_mut(&actor.serial).unwrap();
        r.state23_combat = Some(combat);
        let c = r.channels.as_mut().unwrap();
        c[0][0] = 50;
        c[0][1] = 0x3edd_ddde;
        c[0][2] = 0x3edd_dddd;
        c[0][8] = 0;
        let old = *c;
        let mut random = crate::audio_parameters::Random(7);
        let mut service = AnimationService {
            calls: 0,
            refuse: true,
        };
        assert!(
            session
                .update_state23_animation(actor, animation_context(), &mut random, &mut service)
                .is_err()
        );
        assert_eq!(session.get(actor).unwrap().channels.unwrap(), old);
        assert_eq!(session.get(actor).unwrap().state23_animation_counter, 0);
        let phase = random.0;
        assert!(
            session
                .update_state23_animation(actor, animation_context(), &mut random, &mut service)
                .is_err()
        );
        assert_eq!(service.calls, 1);
        assert_eq!(random.0, phase);
        assert!(session.get(actor).unwrap().state23_animation_failed);
        let success = animation_actor(&mut session);
        let mut state = session.state23_snapshot(success).unwrap();
        state.mode = 21;
        state.retaliation = Some(target);
        let r = session.residents.get_mut(&success.serial).unwrap();
        r.state23_combat = Some(state);
        let c = r.channels.as_mut().unwrap();
        c[0][0] = 50;
        c[0][1] = 0x3edd_ddde;
        c[0][2] = 0x3edd_dddd;
        c[0][8] = 0;
        service.refuse = false;
        session
            .update_state23_animation(success, animation_context(), &mut random, &mut service)
            .unwrap();
        assert_eq!(session.state23_combat(success).unwrap().timer, 3.);
        assert_eq!(session.get(success).unwrap().state23_animation_counter, -1);
        assert!(!session.get(success).unwrap().state23_animation_failed);
        let fresh = animation_actor(&mut session);
        let mut state = session.state23_snapshot(fresh).unwrap();
        state.mode = 20;
        session
            .residents
            .get_mut(&fresh.serial)
            .unwrap()
            .state23_combat = Some(state);
        assert!(
            session
                .update_state23_animation(fresh, animation_context(), &mut random, &mut service)
                .is_err()
        );
        assert!(!session.get(fresh).unwrap().state23_animation_failed);
    }
    fn specification(class: i32, needs: Vec<String>) -> Arc<crate::specification::Prepared> {
        let selection = crate::model_registry::SelectionSpec {
            slots: vec![],
            animations: vec![],
            effects: vec![],
            non_model_commands: vec![],
            diagnostics: vec![],
        };
        Arc::new(crate::specification::Prepared {
            class,
            class_catalog: "class.bin".into(),
            specification: 9,
            specification_catalog: "spec.bin".into(),
            metadata: crate::specification::Metadata::prepare(
                &giants_formats::object_spec::parse(&[255]).unwrap(),
                &selection,
                &giants_formats::effect_image::parse(&[1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])
                    .unwrap(),
            )
            .unwrap(),
            models: vec![],
            animations: Default::default(),
            corrections: vec![],
            needs,
        })
    }
    #[test]
    fn owned_resources_reject_foreign_classes_needs_and_rebinding_then_release() {
        let mut session = Session::new(50);
        let source = seed(18);
        let first = session
            .register(583, &source, &[], DirectFlags::default())
            .unwrap();
        assert!(
            session
                .bind_specification(first, specification(11, vec![]))
                .is_err()
        );
        assert!(
            session
                .bind_specification(first, specification(583, vec!["missing skin".into()]))
                .is_err()
        );
        assert_eq!(session.specification_count(), 0);
        let shared = specification(583, vec![]);
        let weak = Arc::downgrade(&shared);
        session.bind_specification(first, shared.clone()).unwrap();
        assert!(session.reference_matrix(first, 0).is_err());
        assert_eq!(
            session
                .update_references_unblended(first, 254, f32::NAN)
                .unwrap(),
            crate::actor_references::Update::default()
        );
        assert!(session.update_references_unblended(first, 1, 0.).is_err());
        assert!(session.reference_matrix(first, 0).is_err());
        assert!(session.bind_specification(first, shared.clone()).is_err());
        let second = session
            .register(583, &source, &[], DirectFlags::default())
            .unwrap();
        session.bind_specification(second, shared.clone()).unwrap();
        assert!(Arc::ptr_eq(
            session.specification(first).unwrap(),
            session.specification(second).unwrap()
        ));
        let mut foreign = Session::new(51);
        let foreign_id = foreign
            .register(583, &source, &[], DirectFlags::default())
            .unwrap();
        assert!(
            session
                .bind_specification(foreign_id, shared.clone())
                .is_err()
        );
        assert!(session.specification(foreign_id).is_none());
        assert!(
            session
                .update_references_unblended(foreign_id, 254, 0.)
                .is_err()
        );
        assert!(session.reference_matrix(foreign_id, 0).is_err());
        drop(shared);
        assert!(weak.upgrade().is_some());
        session.clear();
        assert_eq!(session.specification_count(), 0);
        assert!(session.specification(first).is_none());
        assert!(weak.upgrade().is_none());
    }
    fn seed(state: u8) -> Seed {
        Seed::from_program_for(
            &giants_formats::class_program::parse(&[0x1e, state, 0xff]).unwrap(),
            Profile::Debug,
            crate::arithmetic::Precision::Double53,
        )
        .unwrap()
    }
    #[test]
    fn entry_subset_refuses_unbound_fields_without_changing_registrations() {
        let source = seed(18);
        let mut session = Session::new(72);
        let id = session
            .register(583, &source, &[], DirectFlags::default())
            .unwrap();
        let before = session.counts();
        assert!(session.entry_subset().is_err());
        assert_eq!(session.counts(), before);
        assert_eq!(session.list(0), [id]);
        session.clear();
        assert!(session.entry_subset().unwrap().actors.is_empty());
    }
    #[test]
    fn independent_payloads_ordered_lists_and_ticket_retirement() {
        let mut source = seed(18);
        source.storage.data_mut().unwrap()[0] = 42;
        let mut session = Session::new(10);
        let mut channels = crate::actor_context::initial_channels(Some(1), false);
        let first = session
            .register_animated(583, &source, &[], DirectFlags::default(), channels)
            .unwrap();
        channels[0][0] = 99;
        assert_eq!(
            session.get(first).unwrap().channels.as_ref().unwrap()[0][0],
            1
        );
        assert_eq!(channels[0][0], 99);
        assert_eq!(session.unique_class(583), Ok(first));
        session
            .advance_channels(
                first,
                &[None, Some(1.)],
                [0.; 3],
                0.25,
                crate::arithmetic::Precision::Double53,
            )
            .unwrap();
        assert_eq!(
            crate::animation_channels::clock(
                &session.get(first).unwrap().channels.as_ref().unwrap()[0]
            )
            .seconds,
            0.25
        );
        let second = session
            .register(583, &source, &[], DirectFlags::default())
            .unwrap();
        assert!(session.unique_class(583).is_err());
        source.storage.data_mut().unwrap()[0] = 99;
        assert_eq!(
            session.get(first).unwrap().seed.storage.data().unwrap()[0],
            42
        );
        assert_eq!(session.list(0), [first, second]);
        assert_eq!(session.list(12), [first, second]);
        assert_eq!(
            session.counts(),
            Counts {
                residents: 2,
                payload_bytes: 400,
                registrations: 8
            }
        );
        let mut next = Session::new(11);
        let fresh = next
            .register(583, &source, &[], DirectFlags::default())
            .unwrap();
        assert!(next.get(first).is_none());
        assert!(session.get(fresh).is_none());
        assert!(
            session
                .advance_channels(
                    fresh,
                    &[None, Some(1.)],
                    [0.; 3],
                    0.25,
                    crate::arithmetic::Precision::Double53
                )
                .is_err()
        );
        session.clear();
        assert_eq!(session.counts(), Counts::default());
        assert!(session.get(first).is_none());
        assert!(session.list(12).is_empty());
        let replacement = session
            .register(583, &source, &[], DirectFlags::default())
            .unwrap();
        assert_ne!(replacement, first);
        assert!(session.get(first).is_none());
        assert!(next.get(fresh).is_some());
    }
    #[test]
    fn presentation_sound_graph_is_owned_acyclic_and_retired_with_its_ticket() {
        let seed = seed(18);
        let mut session = Session::new(8);
        let parent = session
            .register(2, &seed, &[], DirectFlags::default())
            .unwrap();
        let child = session
            .register(583, &seed, &[], DirectFlags::default())
            .unwrap();
        session
            .set_presentation_parent(child, Some(parent))
            .unwrap();
        assert!(
            session
                .set_presentation_parent(parent, Some(child))
                .is_err()
        );
        session
            .set_presentation_position(child, Some([1., 2., 3.]))
            .unwrap();
        assert_eq!(
            session.sound_route(child, parent, crate::actor_sound::Kind::Attachment, true),
            Ok(Some(crate::actor_sound::Route::Local(parent)))
        );
        assert_eq!(
            session.sound_route(child, parent, crate::actor_sound::Kind::Owner, true),
            Ok(Some(crate::actor_sound::Route::Attached(Some(child))))
        );
        assert!(
            session
                .set_presentation_position(child, Some([f32::NAN, 0., 0.]))
                .is_err()
        );
        assert_eq!(session.presentation_position(child), Some([1., 2., 3.]));
        let mut foreign = Session::new(9);
        let other = foreign
            .register(2, &seed, &[], DirectFlags::default())
            .unwrap();
        assert!(session.set_presentation_parent(child, Some(other)).is_err());
        assert!(
            session
                .set_presentation_position(other, Some([0.; 3]))
                .is_err()
        );
        session.clear();
        assert_eq!(session.presentation_position(child), None);
        assert!(
            session
                .sound_route(child, parent, crate::actor_sound::Kind::Owner, true)
                .is_err()
        );
    }
    #[test]
    fn invalid_profile_or_binding_map_never_registers_partial_data() {
        let mut session = Session::new(3);
        let program = giants_formats::class_program::parse(&[
            0x1e, 2, 0x1f, 20, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 0xff,
        ])
        .unwrap();
        let debug = Seed::from_program_for(
            &program,
            Profile::Debug,
            crate::arithmetic::Precision::Double53,
        )
        .unwrap();
        let binding = StateBinding {
            offset: 0x70,
            key: crate::state_storage::ReferenceKey::Spec(20),
            target: Err("absent at registration".into()),
        };
        for bindings in [vec![], vec![binding.clone(), binding.clone()]] {
            assert!(
                session
                    .register(2, &debug, &bindings, DirectFlags::default())
                    .is_err()
            );
            assert_eq!(session.counts(), Counts::default());
        }
        assert!(
            session
                .register(
                    2,
                    &Seed::from_program(&program).unwrap(),
                    std::slice::from_ref(&binding),
                    DirectFlags::default()
                )
                .is_err()
        );
        let id = session
            .register(2, &debug, &[binding], DirectFlags::default())
            .unwrap();
        assert_eq!(id.serial, 1);
        assert!(session.get(id).unwrap().bindings[0].target.is_err());
    }
}

#[cfg(test)]
#[path = "mecc_world_tests.rs"]
mod mecc_world_tests;
