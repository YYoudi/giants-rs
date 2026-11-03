//! Owned offline normal-pistol request with required targeting/providers.
//! The caller still owns eligibility, aiming, cooldown storage, world admission
//! and ammunition. The normal aim service remains a required boundary.
use crate::{
    actor_fields::Fields,
    audio_parameters::Random,
    resident_states::{Construction, Id, Session},
    scene_resources::SceneResources,
    state_storage::{Profile, Seed},
};
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
pub struct Record {
    pub source: Id,
    pub counter: u32,
    pub position: [f32; 3],
}
pub struct Input {
    pub source: Id,
    pub cooldown: Option<f32>,
    /// Direct only with tracking_actor; otherwise the aim provider produces it.
    pub target: [f32; 3],
    pub tracking_actor: Option<Id>,
    pub range: f32,
    pub camera_origin: [f32; 3],
    /// Supplied world counter, whose gameplay meaning is not inferred here.
    pub counter: u32,
}
pub struct Shot {
    pub actor: Id,
    pub cooldown: Option<f32>,
    pub effect: crate::attached_effect_context::Creation,
    pub cue: u32,
    pub sound: crate::enemy_projectile::SoundOutcome,
}
pub trait Services {
    /// Actual normal targeting branch, including view/ray acquisition and its
    /// fallback. A missing targeting producer must fail, not choose the camera.
    fn aim(
        &mut self,
        session: &mut Session,
        source: Id,
        muzzle: [f32; 3],
        range: f32,
    ) -> Result<[f32; 3], String>;
    /// Resolve this actual selected parent/reference at the caller's phase.
    fn muzzle(
        &mut self,
        session: &Session,
        binding: crate::mecc_fire_lock::Binding<Id>,
    ) -> Result<[f32; 3], String>;
    /// Perform named constructor-effect preparation; unavailable backends fail.
    fn effect(
        &mut self,
        session: &mut Session,
        recipient: Id,
        name: &str,
        random: &mut Random,
    ) -> Result<crate::attached_effect_context::Creation, String>;
    fn sound(
        &mut self,
        session: &mut Session,
        source: Id,
        cue: u32,
    ) -> Result<crate::enemy_projectile::SoundOutcome, String>;
}
/// Real deferred constructor-effect preparation over the owning world's pool.
/// Targeting and sound remain mandatory providers; no device is acknowledged.
pub struct ContextServices<'a, S> {
    pub contexts: &'a mut crate::attached_effect_context::Pool<Id>,
    pub other: &'a mut S,
}
impl<S: Services> Services for ContextServices<'_, S> {
    fn aim(
        &mut self,
        session: &mut Session,
        source: Id,
        muzzle: [f32; 3],
        range: f32,
    ) -> Result<[f32; 3], String> {
        self.other.aim(session, source, muzzle, range)
    }
    fn muzzle(
        &mut self,
        session: &Session,
        binding: crate::mecc_fire_lock::Binding<Id>,
    ) -> Result<[f32; 3], String> {
        self.other.muzzle(session, binding)
    }
    fn effect(
        &mut self,
        session: &mut Session,
        recipient: Id,
        name: &str,
        _: &mut Random,
    ) -> Result<crate::attached_effect_context::Creation, String> {
        let metadata = session
            .basic_projectile(recipient)
            .ok_or("constructor effect recipient absent")?;
        if metadata.definition.id != 17
            || metadata.launch.is_some()
            || metadata.constructor_effect.as_deref() != Some(name)
        {
            return Err("constructor effect outside fresh normal-pistol service".into());
        }
        let fields = session
            .get(recipient)
            .and_then(|a| a.construction_fields.as_ref())
            .ok_or("constructor effect fields absent")?;
        self.contexts
            .prepare(name, recipient, fields.object_flags, 0)
    }
    fn sound(
        &mut self,
        session: &mut Session,
        source: Id,
        cue: u32,
    ) -> Result<crate::enemy_projectile::SoundOutcome, String> {
        self.other.sound(session, source, cue)
    }
}
pub struct Factory {
    seed: Seed,
    fields: Fields,
    bindings: Vec<crate::model_registry::StateBinding>,
    flags: crate::object_lists::DirectFlags,
    specification: Arc<crate::specification::Prepared>,
    states: Arc<crate::state_resources::Prepared>,
    definition: Arc<crate::projectile_definition::Definition>,
    bounds: crate::projectile_launch::Bounds,
}
impl Factory {
    pub fn load(
        source: &SceneResources,
        declarations: &crate::projectile_definition::DebugDefinitions,
    ) -> Result<Self, String> {
        if declarations
            .catalog
            .icons
            .get(&1)
            .is_none_or(|icon| icon.projectile != 17)
        {
            return Err("base-pistol icon must resolve the acquired projectile declaration".into());
        }
        let definition = Arc::new(
            declarations
                .entries
                .get(&17)
                .ok_or("normal pistol declaration absent")?
                .clone(),
        );
        if definition.flags() & 1 != 0
            || definition.word(0x58)? != 0
            || definition.effect(3).is_some()
        {
            return Err("normal shot requires the acquired straight nonhoming definition".into());
        }
        let program = source
            .classes
            .models
            .first_program(38)
            .ok_or("normal projectile class absent")?
            .program
            .as_ref()
            .map_err(Clone::clone)?;
        let specification = Arc::new(crate::specification::Prepared::load_registered(source, 38)?);
        let fields = Fields::load(source, 38, &specification)?;
        let seed = Seed::from_program_for(
            program,
            Profile::Debug,
            crate::arithmetic::Precision::Double53,
        )?;
        crate::basic_projectile::prepare(&seed, &fields, definition.clone(), 0, [0.; 3])?;
        let bounds = crate::projectile_launch::Bounds::load(source, specification.clone())?;
        Ok(Self {
            seed,
            fields,
            specification,
            definition,
            bounds,
            bindings: source.class_state_bindings_for(38, Profile::Debug)?,
            flags: crate::object_lists::DirectFlags::project(38, program),
            states: Arc::new(crate::state_resources::Prepared::load(
                source,
                38,
                Profile::Debug,
            )?),
        })
    }
    pub fn definition(&self) -> &crate::projectile_definition::Definition {
        &self.definition
    }
    /// A positive clock returns without provider calls or RNG consumption.
    /// Failures after allocation starts quarantine this source.
    pub fn attempt(
        &self,
        session: &mut Session,
        input: Input,
        random: &mut Random,
        services: &mut impl Services,
    ) -> Result<Option<Shot>, String> {
        session.check_wield_owner(input.source)?;
        if session.mecc_shot_failure(input.source).is_some()
            || session.mecc_physical_cycle_failure(input.source).is_some()
            || !input
                .target
                .into_iter()
                .chain(input.camera_origin)
                .chain([input.range])
                .chain(input.cooldown)
                .all(f32::is_finite)
        {
            return Err(
                "normal shot requires an unfailed source and finite direct-target inputs".into(),
            );
        }
        if input.cooldown.is_some_and(|clock| clock > 0.) {
            return Ok(None);
        }
        let binding = session.mecc_fire_lock(input.source, 17, &self.definition)?;
        let muzzle = services.muzzle(session, binding)?;
        let target = if input.tracking_actor.is_some() {
            input.target
        } else {
            services.aim(session, input.source, muzzle, input.range)?
        };
        crate::projectile_launch::Straight::new(
            muzzle,
            target,
            f32::from_bits(self.definition.word(8)?),
        )?;
        let result: Result<Option<Shot>, String> = (|| {
            let actor = session.construct_basic_projectile(
                Construction {
                    class: 38,
                    seed: &self.seed,
                    bindings: &self.bindings,
                    flags: self.flags,
                    specification: self.specification.clone(),
                    state_resources: self.states.clone(),
                    fields: self.fields.instantiate(),
                },
                self.definition.clone(),
                0,
                input.camera_origin,
            )?;
            let effect = if let Some(name) = self.definition.effect(0) {
                services.effect(session, actor, name, random)?
            } else {
                crate::attached_effect_context::Creation::Absent
            };
            // The launch helper reads the selected parent again after the
            // constructor's effect service. Never reuse the wrapper's earlier
            // aiming sample across that service boundary.
            let muzzle = services.muzzle(session, binding)?;
            session.launch_basic_projectile_in_world(
                actor,
                crate::basic_projectile::LaunchInput {
                    parent: binding.parent,
                    tracking_actor: input.tracking_actor,
                    reference: binding.reference,
                    resolved_reference: muzzle,
                    target,
                },
                &self.bounds,
            )?;
            session.attribute_basic_projectile(actor, input.source)?;
            session.commit_mecc_shot_record(input.source, input.counter)?;
            let cooldown = input.cooldown.map(|clock| {
                (f64::from(clock)
                    + f64::from(f32::from_bits(
                        self.definition
                            .word(0x44)
                            .expect("validated declaration interval"),
                    ))) as f32
            });
            let cue = crate::mecc_audio::pistol_request(17, (random.next_15() * 100) >> 15)?;
            let sound = services.sound(session, input.source, cue)?;
            Ok(Some(Shot {
                actor,
                cooldown,
                effect,
                cue,
                sound,
            }))
        })();
        if let Err(error) = &result {
            session.fail_mecc_shot(input.source, error);
        }
        result
    }
}
