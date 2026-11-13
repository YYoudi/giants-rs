//! Owned class90/232 state6 copy at the enemy fire helper's pre-FX boundary.
//! Ordered launch requires explicit effect/audio/reference providers. The
//! state6 flight can consume owned-list or ordinary-scene geometry. Contact
//! reactions, projectile rendering and damage availability remain separate.
use crate::{
    actor_fields::Fields,
    state_storage::{Profile, Seed},
};

/// The original allocates the class884 shell with no additional list bits,
/// then copies the resource class. That copy does not reclassify memberships.
pub const LIST_MASK: u32 = 0;
#[derive(Debug)]
pub struct Metadata {
    pub resource_class: i32,
    pub phase: Phase,
    pub effect: Option<EffectOutcome>,
    pub sound: Option<SoundOutcome>,
    pub sound_words: [u32; 3],
    pub launch: Option<Launch>,
    pub failure: Option<String>,
    pub ground_impact: Option<GroundImpact>,
    pub object_impact: Option<ObjectImpact>,
    /// Initial late collider snapshot, not current world bounds or a ready tick.
    pub initial_environment: Option<crate::collider_init::Instance>,
    pub pending_flight: Option<FlightOutcome>,
    /// Only the actual ground query creates this continuation's clock. A caller
    /// cannot replace the elapsed time/ordered areas after observing a contact.
    pub(crate) ground_flight_clock: Option<GroundFlightClock>,
    /// Advances only after the actual terrain/object flight query publishes.
    pub flight_revision: u64,
    /// Object-local clock selected after the current flight, at its real pose.
    pub animation_seconds: Option<f32>,
    pub presentation: Option<Presentation>,
    pub presentation_failure: Option<String>,
}
#[derive(Clone, Debug)]
pub(crate) struct GroundFlightClock {
    pub seconds: f32,
    pub areas: Vec<crate::object_time::SlowArea>,
}
/// Scoped clock/affine completion, not generic world-tick or impact readiness.
#[derive(Clone, Copy, Debug)]
pub struct Presentation {
    pub flight_revision: u64,
    pub epoch: Option<u32>,
}
/// Explicit single-player world view. The Session supplies its ordered owned
/// object list; mission population completeness remains the caller's contract.
pub struct FlightWorld<'a, 't> {
    pub terrain: &'a crate::segment_queries::StaticCellQuery<'t>,
    pub inverse_spacing: f32,
    pub seconds: f32,
    /// Actual mission vertex epoch and cache gate, not inferred from elapsed time.
    pub vertex_epoch: u32,
    pub force_vertices: bool,
    pub areas: &'a [crate::object_time::SlowArea],
}
/// A current ordinary scene supplies its own geometry packets; no native global
/// vertex epoch is manufactured for a partial controller or rigid declaration.
#[derive(Clone, Copy)]
pub struct SceneFlightWorld<'a, 't> {
    pub terrain: &'a crate::segment_queries::StaticCellQuery<'t>,
    pub inverse_spacing: f32,
    pub seconds: f32,
    pub areas: &'a [crate::object_time::SlowArea],
}
/// Actual query boundary. Object hits and expiry require their own reactions;
/// they are retained as pending and never acknowledged as successful damage.
#[derive(Clone, Copy, Debug)]
pub enum FlightOutcome {
    Flying,
    GroundContact {
        point: [f32; 3],
    },
    ObjectContact {
        target: crate::resident_states::Id,
        point: [f32; 3],
        tag: u32,
    },
    /// A prepared rigid declaration, not an invented Session actor. The caller
    /// must acquire its actual impact/owner continuation before acknowledging it.
    DeclaredContact {
        target: crate::static_geometry::SourceKey,
        point: [f32; 3],
        tag: u32,
    },
    Expired,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Constructed,
    Services,
    Failed,
    Launched,
    ImpactServices,
    ObjectImpactServices,
    Retiring,
    CleanupServices,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectOutcome {
    Prepared,
    Rejected,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundOutcome {
    Submitted,
    ResourceUnavailable,
    Suppressed,
}
#[derive(Clone, Debug)]
pub struct Launch {
    pub source: crate::resident_states::Id,
    pub dispatch_context: u32,
    pub rates: [f32; 4],
    pub previous_rates: [f32; 4],
    pub calculation: crate::enemy_ballistic::Launch,
}
pub struct LaunchInput {
    pub source: crate::resident_states::Id,
    pub target: [f32; 3],
    pub vertical_spread_percent: f32,
    /// Current source context is an explicit upstream input until its primary
    /// consumer owns it. Never infer it from a class or a native address.
    pub dispatch_context: u32,
}
pub struct Recipe {
    pub effect: &'static str,
    pub action: u32,
    pub cues: [u32; 2],
    pub sound_words: [u32; 3],
}
pub struct EffectRequest<'a> {
    pub recipient: crate::resident_states::Id,
    pub name: &'a str,
    pub flags: u32,
    pub tag: u32,
}
pub fn recipe(actor_class: i32) -> Result<Recipe, &'static str> {
    match actor_class {
        73 => Ok(Recipe {
            effect: "fx_ripper_medium_proj",
            action: 0,
            cues: [1250, 1251],
            sound_words: [1591, 1263, 0],
        }),
        230 => Ok(Recipe {
            effect: "fx_ripper_large_proj",
            action: 1,
            cues: [1227, 1228],
            sound_words: [1590, 1240, 1241],
        }),
        _ => Err("enemy fire recipe outside acquired state23 classes"),
    }
}
/// Implementations must perform actual preparation/admission, audio lookup and
/// current reference resolution. Missing backends return errors. Prepared FX
/// is not a rendered effect; real resource absence is not an unavailable backend.
/// EXE-side effect/reference providers share the caller's game RNG. The sound
/// backend owns its DLL/audio RNG domain; positional dispatch does not receive
/// the game RNG. Providers may update/cancel owned actors.
pub trait Services {
    fn prepare_effect(
        &mut self,
        session: &mut crate::resident_states::Session,
        request: EffectRequest<'_>,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<EffectOutcome, String>;
    fn sound(
        &mut self,
        session: &mut crate::resident_states::Session,
        cue: u32,
        position: [f32; 3],
    ) -> Result<SoundOutcome, String>;
    fn muzzle(
        &mut self,
        session: &mut crate::resident_states::Session,
        source: crate::resident_states::Id,
        reference: i32,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<[f32; 3], String>;
}
pub struct Prepared {
    pub seed: Seed,
    pub fields: Fields,
    pub metadata: Metadata,
}
pub fn prepare(seed: &Seed, fields: &Fields) -> Result<Prepared, String> {
    if seed.storage.profile() != Profile::Debug
        || seed.storage.state() != 6
        || !seed.diagnostics.is_empty()
        || !matches!(fields.class, 90 | 232)
        || fields.configuration[20] != 6
        || seed.storage.data().is_none_or(|d| d.len() != 36)
    {
        return Err("enemy projectile requires acquired D class90/232 state6 storage".into());
    }
    Ok(Prepared {
        seed: seed.instantiate()?,
        fields: fields.instantiate(),
        metadata: Metadata {
            resource_class: fields.class,
            phase: Phase::Constructed,
            effect: None,
            sound: None,
            sound_words: [0; 3],
            launch: None,
            failure: None,
            ground_impact: None,
            object_impact: None,
            initial_environment: None,
            pending_flight: None,
            ground_flight_clock: None,
            flight_revision: 0,
            animation_seconds: None,
            presentation: None,
            presentation_failure: None,
        },
    })
}

#[derive(Clone, Debug)]
pub struct GroundImpact {
    pub effect: EffectOutcome,
    pub sound: Option<SoundOutcome>,
    pub cue: Option<u32>,
}
/// A completed positive direct contact. Target identity is historical evidence,
/// not an owned transform/attachment reference. The pending query owns the live
/// target reference until this continuation completes.
#[derive(Clone, Copy, Debug)]
pub struct ObjectImpact {
    pub source: crate::resident_states::Id,
    pub target: crate::resident_states::Id,
    pub point: [f32; 3],
    pub tag: u32,
    pub amount: f32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObjectContactOutcome {
    Retained,
    Marked,
}
/// Real single-player direct-hit consumers. World suppression is queried before
/// callback dispatch. Terminal processing follows the actual source mark; no
/// default world guards, damage backend or mission owner are supplied here.
pub trait ObjectImpactServices {
    fn world_suppressed(
        &mut self,
        session: &crate::resident_states::Session,
    ) -> Result<bool, String>;
    fn hurt(
        &mut self,
        session: &mut crate::resident_states::Session,
        request: ObjectImpact,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<(), String>;
    fn terminal(
        &mut self,
        session: &mut crate::resident_states::Session,
        target: crate::resident_states::Id,
    ) -> Result<(), String>;
}
#[derive(Clone, Copy)]
pub(crate) enum RetirementKind {
    Ground,
    Object,
}
#[derive(Clone, Debug)]
pub struct ImpactEffect {
    pub name: &'static str,
    pub flags: u32,
    pub position: [f32; 3],
}
/// This helper selects by the current action, not by a shot/scene number.
pub fn impact_effect(action: u32, position: [f32; 3]) -> Result<ImpactEffect, &'static str> {
    if !position.into_iter().all(f32::is_finite) {
        return Err("nonfinite enemy impact point");
    }
    Ok(ImpactEffect {
        name: if action == 1 {
            "fx_ripper_large_explode"
        } else {
            "fx_ripper_medium_explode"
        },
        flags: 8,
        position,
    })
}
#[derive(Clone, Debug)]
pub struct RadialRequest {
    pub source: crate::resident_states::Id,
    pub other: Option<crate::resident_states::Id>,
    pub position: [f32; 3],
    /// Preserve the five source payload words in the original dispatch order;
    /// the radial consumer, not this wrapper, interprets their individual units.
    pub parameters: [u32; 5],
    pub context: u32,
}
/// A contact reaction requires actual FX admission, radial processing and audio
/// services. There is no implicit clear world or successful damage backend.
pub trait GroundImpactServices {
    fn effect(
        &mut self,
        session: &mut crate::resident_states::Session,
        request: ImpactEffect,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<EffectOutcome, String>;
    fn radial(
        &mut self,
        session: &mut crate::resident_states::Session,
        request: RadialRequest,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<(), String>;
    fn sound(
        &mut self,
        session: &mut crate::resident_states::Session,
        cue: u32,
        position: [f32; 3],
    ) -> Result<SoundOutcome, String>;
}

/// Historical alias for the shared tracked actor-release boundary.
pub use crate::actor_retirement::Services as RetirementServices;
