//! Source health object required by the class73/230 lethal callback.
//! Prototype PS2 `ripper_drop_health__FP6OBJECT` (`ripper_drop_health`) is a
//! navigation candidate only; its
//! correspondence to the PC callback or state37 primary remains unknown.

//! Constructor, owned primary prefix and Mecc collection contracts. Ordinary
//! world/FX/audio/stat providers and their runtime scheduling remain explicit.
use crate::{
    actor_fields::Fields,
    arithmetic::Precision,
    attached_effect_context::Creation,
    audio_parameters::Random,
    model_registry::StateBinding,
    object_lists::DirectFlags,
    resident_states::{Construction, Id, Session},
    scene_resources::SceneResources,
    state_storage::{Profile, Seed},
};
use std::sync::Arc;
pub(crate) mod primary;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrimaryPhase {
    Idle,
    Running,
    CollectionPending,
    Collecting,
    Failed,
}

/// World inputs remain explicit; this is not a global-clock producer.
pub struct PrimaryInput<'a> {
    pub step: u64,
    pub global_seconds: f32,
    pub areas: &'a [crate::object_time::SlowArea],
    pub paused: bool,
    pub force_ground: bool,
}
pub trait PrimaryServices {
    fn ground(&mut self, session: &Session, item: Id, position: [f32; 3]) -> Result<f32, String>;
    /// Execute the actual ordered eligible-world obstruction query. Missing
    /// geometry is an error, never an empty/no-contact result.
    /// The source list5 query excludes self, parented and excluded-owner actors,
    /// requires personality bit17 and clear flags4000c000, then uses the actual
    /// segment/AABB and required triangle test (except personality bit24).
    fn blocked(
        &mut self,
        session: &Session,
        item: Id,
        from: [f32; 3],
        to: [f32; 3],
    ) -> Result<bool, String>;
    /// Source expiry marks first, then checks/emits its actual notification.
    /// Missing notification ownership is an error, not assumed single-player.
    /// This does not release the item or its attached effect.
    fn expired(&mut self, session: &Session, item: Id) -> Result<(), String>;
}
#[derive(Debug)]
pub struct PendingCollection {
    pub(crate) item: Id,
    pub(crate) step: u64,
    /// Source-bound pre-movement packet; the primary's later pose writes do not
    /// authorize a fabricated refreshed affine or common finalization replay.
    pub(crate) bounds: [f32; 6],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Statistic {
    Medium,
    Large,
}
impl Statistic {
    pub fn source_name(self) -> &'static str {
        match self {
            Self::Medium => "InstantHealthMed",
            Self::Large => "InstantHealthLarge",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollectionOutcome {
    NotCollected,
    Collected,
}
pub trait CollectionServices {
    fn local_cue(
        &mut self,
        session: &Session,
        item: Id,
        recipient: Id,
        cue: u32,
    ) -> Result<(), String>;
    /// Copy the already committed canonical response; never subtract again.
    fn copy_health(
        &mut self,
        session: &Session,
        response: crate::controlled_mecc::HitResponse,
    ) -> Result<(), String>;
    /// Resolve the actual named statistic and increment it by one.
    fn statistic(
        &mut self,
        session: &Session,
        recipient: Id,
        statistic: Statistic,
    ) -> Result<(), String>;
}
impl PendingCollection {
    pub fn item(&self) -> Id {
        self.item
    }
    pub fn step(&self) -> u64 {
        self.step
    }
}
#[derive(Debug)]
pub enum PrimaryPrefix {
    Paused,
    Collection(PendingCollection),
}

/// Bound to the actual source type36 declaration. An absent/unacquired field is
/// an error, separate from a declared family that does not admit the Mecc.
#[derive(Clone, Copy, Debug)]
pub struct PickupDefinition {
    player_type: i32,
}
impl PickupDefinition {
    pub fn load(catalog: &giants_formats::weapons::Catalog) -> Result<Self, String> {
        let icon = catalog
            .icons
            .get(&36)
            .filter(|icon| icon.id == 36)
            .ok_or("Ripper health item36 definition absent")?;
        Ok(Self {
            player_type: icon
                .player_type
                .ok_or("Ripper health player-family declaration absent or unacquired")?,
        })
    }
    /// Bounded PC type36 eligibility, not contact, healing or retirement.
    /// Other player families keep their separate native admission rules.
    pub fn mecc_eligible(self, health: crate::mecc_damage::State) -> Result<bool, String> {
        if ![health.damage, health.limit, health.shield]
            .into_iter()
            .all(f32::is_finite)
            || health.limit <= 0.
            || health.shield < 0.
        {
            return Err("Ripper health eligibility requires finite actual Mecc health".into());
        }
        Ok(matches!(self.player_type, 1 | 2) && health.damage > 0. && health.damage < health.limit)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Owned {
    pub owner: Id,
    pub amount: i32,
    pub owner_delay: f32,
    pub lifetime: f32,
    pub context: Option<crate::attached_effect_context::Id>,
    /// Constructor/amount/control suffix only, not a completed physical frame
    /// or active/running effect.
    pub initialized: bool,
    pub failed: bool,
    pub velocity: [f32; 3],
    pub floor: f32,
    pub primary_phase: PrimaryPhase,
    pub primary_step: Option<u64>,
    pub primary_recipient: Option<Id>,
    /// Native source birth has no excluded pickup owner. This is distinct
    /// from `owner`, which retains the Ripper that created the item.
    pub exclusion_owner: Option<Id>,
}
/// Actual mode4 attachment; Prepared retains a context, not GPU readiness.
pub trait Effects {
    fn attach(
        &mut self,
        session: &mut Session,
        item: Id,
        name: &'static str,
        random: &mut Random,
    ) -> Result<Creation, String>;
}
pub struct Factory {
    seed: Seed,
    bindings: Vec<StateBinding>,
    fields: Fields,
    flags: DirectFlags,
    body: Arc<crate::specification::Prepared>,
    states: Arc<crate::state_resources::Prepared>,
    geometry: crate::resident_environment::Resources,
    amount: [i32; 2],
}
impl Factory {
    /// Execute the owned state37 animation/affine/motion/timer prefix. An
    /// unpaused result remains pending until the real collection tail runs.
    /// No completed physical frame or healing follows from this call.
    pub fn advance_primary_prefix(
        &self,
        session: &mut Session,
        item: Id,
        input: PrimaryInput<'_>,
        services: &mut impl PrimaryServices,
    ) -> Result<PrimaryPrefix, String> {
        session.advance_ripper_health_prefix(item, &self.body, &self.geometry, input, services)
    }
    /// Finish the retained primary with the ordinary controller's current
    /// contact packet. Marking a collected item does not retire its FX/owners.
    pub fn finish_collection(
        &self,
        session: &mut Session,
        pending: &PendingCollection,
        player: &crate::controlled_mecc_contacts::Snapshot,
        definition: PickupDefinition,
        services: &mut impl CollectionServices,
    ) -> Result<CollectionOutcome, String> {
        session.finish_ripper_health_collection(&self.body, pending, player, definition, services)
    }
    pub fn load(
        resources: &SceneResources,
        globals: &giants_formats::global_data::GlobalData,
    ) -> Result<Self, String> {
        let program = resources
            .classes
            .models
            .first_program(785)
            .ok_or("Ripper health class785 absent")?
            .program
            .as_ref()
            .map_err(Clone::clone)?;
        let seed = Seed::from_program_for(program, Profile::Debug, Precision::Double53)?;
        let bindings = resources.class_state_bindings_for(785, Profile::Debug)?;
        if seed.storage.state() != 0
            || seed.storage.data().is_some()
            || !seed.diagnostics.is_empty()
            || !seed.unresolved.is_empty()
            || !bindings.is_empty()
        {
            return Err("Ripper health requires acquired empty cold0 construction".into());
        }
        let body = Arc::new(crate::specification::Prepared::load_registered(
            resources, 785,
        )?);
        let states = Arc::new(crate::state_resources::Prepared::load(
            resources,
            785,
            Profile::Debug,
        )?);
        if !body.needs.is_empty() || !states.needs().is_empty() {
            return Err("Ripper health source resources incomplete".into());
        }
        let fields = Fields::load(resources, 785, &body)?;
        let geometry = crate::resident_environment::Resources::load(resources, &body)?;
        let amount =
            [0x1078, 0x107c].map(|at| globals.words.get(at / 4).copied().map(|w| w as i32));
        let [Some(medium), Some(large)] = amount else {
            return Err("Ripper health quantities absent".into());
        };
        Ok(Self {
            seed,
            bindings,
            fields,
            flags: DirectFlags::project(785, program),
            body,
            states,
            geometry,
            amount: [medium, large],
        })
    }
    /// Current source bounds only. The caller must prepare the owned affine;
    /// this does not advance state37, establish contact or acknowledge healing.
    pub fn bounds(&self, session: &Session, item: Id) -> Result<[f32; 6], String> {
        let slot = session.ripper_health_geometry_source(item, &self.body)?;
        let bounds = self.geometry.bounds(slot)?;
        let affine = session.actor_current_affine(item)?;
        crate::collision_bounds::cached(
            Some(crate::collision_bounds::Models {
                primary: bounds.primary,
                auxiliary: bounds.current_auxiliary,
                slot,
            }),
            affine.matrix[..12].try_into().unwrap(),
            Precision::Double53,
        )
        .map_err(str::to_owned)
    }
    /// Only an active lethal callback can construct this child. A late effect
    /// error retains its published identity and marks its incomplete suffix.
    pub fn spawn(
        &self,
        session: &mut Session,
        owner: Id,
        random: &mut Random,
        effects: &mut impl Effects,
    ) -> Result<Id, String> {
        let state = session
            .state23_combat(owner)
            .ok_or("Ripper health source absent")?;
        let large = match state.class {
            73 => false,
            230 => true,
            _ => return Err("Ripper health source class invalid".into()),
        };
        let id = session.construct_ripper_health_drop(
            owner,
            Construction {
                class: 785,
                seed: &self.seed,
                bindings: &self.bindings,
                flags: self.flags,
                specification: self.body.clone(),
                state_resources: self.states.clone(),
                fields: self.fields.instantiate(),
            },
        )?;
        let result = effects.attach(
            session,
            id,
            if large {
                "fx_ripper_large_health"
            } else {
                "fx_ripper_medium_health"
            },
            random,
        );
        session.finish_ripper_health_drop(
            id,
            owner,
            self.amount[usize::from(large)],
            large,
            result,
        )?;
        Ok(id)
    }
}

#[cfg(test)]
pub(crate) mod tests;
