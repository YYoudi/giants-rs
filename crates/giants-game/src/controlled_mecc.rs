//! Controller-owned motion projection, not a complete native state2 callback.
//! Pose/rates come from the actual physics producer after contact. Header caches
//! remain separate phases and must reject the old pose after publication.
#[derive(Clone, Copy, Debug)]
pub struct Frame {
    pub before: [f32; 3],
    pub position: [f32; 3],
    pub contact_previous: [f32; 3],
    pub angles: [f32; 3],
    pub scale: f32,
    pub velocity: [f32; 3],
    pub model_rates: [f32; 3],
}
#[derive(Clone, Copy, Debug)]
pub struct Owned {
    pub step: u64,
    pub frame: Frame,
    pub previous_history: [f32; 3],
}
/// Fields actually shared by the partial controller and canonical hit receiver.
#[derive(Clone, Copy, Debug)]
pub struct HitState {
    pub health: crate::mecc_damage::State,
    pub velocity: [f32; 3],
}
impl HitState {
    pub fn same_bits(self, other: Self) -> bool {
        self.health.damage.to_bits() == other.health.damage.to_bits()
            && self.health.limit.to_bits() == other.health.limit.to_bits()
            && self.health.shield.to_bits() == other.health.shield.to_bits()
            && self.health.flags == other.health.flags
            && self.velocity.map(f32::to_bits) == other.velocity.map(f32::to_bits)
    }
}
#[derive(Clone, Copy, Debug)]
pub struct HitPose {
    pub position: [f32; 3],
    pub angles: [f32; 3],
    pub scale: f32,
}
/// Produced only while the real attributed callback is running. Its before
/// state prevents double damage/copy, and its after state must already exist in
/// the canonical Session. Deferred delivery after source retirement is refused.
#[derive(Clone, Copy, Debug)]
pub struct HitResponse {
    pub(crate) source: crate::resident_states::Id,
    pub(crate) owner: crate::resident_states::Id,
    pub(crate) step: u64,
    pub(crate) before: HitState,
    pub(crate) after: HitState,
    pub(crate) origin: HitOrigin,
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum HitOrigin {
    Projectile,
    RipperMelee { epoch: u64 },
}
impl HitResponse {
    pub fn source(self) -> crate::resident_states::Id {
        self.source
    }
    pub fn owner(self) -> crate::resident_states::Id {
        self.owner
    }
    pub fn step(self) -> u64 {
        self.step
    }
    pub fn before(self) -> HitState {
        self.before
    }
    pub fn after(self) -> HitState {
        self.after
    }
    pub fn validate_controller(
        self,
        session: &crate::resident_states::Session,
        expected_owner: crate::resident_states::Id,
        pose: HitPose,
        current: HitState,
    ) -> Result<HitState, String> {
        use crate::enemy_projectile::{FlightOutcome, Phase};
        if expected_owner != self.owner
            || !current.same_bits(self.before)
            || self.before.same_bits(self.after)
            || !self.before.health.alive()
            || !self.after.health.alive()
        {
            return Err(
                "Mecc controller response owner/before state is foreign, changed or consumed"
                    .into(),
            );
        }
        match self.origin {
            HitOrigin::Projectile => {
                let meta = session
                    .enemy_projectile(self.source)
                    .ok_or("Mecc response source absent")?;
                if meta.phase != Phase::ObjectImpactServices
                    || meta.mecc_receiver != Some(crate::mecc_enemy_hit::Phase::Running)
                    || !matches!(meta.pending_flight, Some(FlightOutcome::ObjectContact {target,..}) if target == self.owner)
                {
                    return Err("Mecc response needs its actual active source callback".into());
                }
                session.enemy_projectile_mecc_source(self.owner, self.source)?;
            }
            HitOrigin::RipperMelee { epoch } => {
                session.validate_ripper_melee_response(
                    self.source,
                    self.owner,
                    self.step,
                    epoch,
                )?;
            }
        }
        let actual = session.controlled_mecc_motion(self.owner, self.step)?;
        if actual.frame.position.map(f32::to_bits) != pose.position.map(f32::to_bits)
            || actual.frame.angles.map(f32::to_bits) != pose.angles.map(f32::to_bits)
            || actual.frame.scale.to_bits() != pose.scale.to_bits()
            || !self.after.same_bits(HitState {
                health: session.mecc_health(self.owner)?,
                velocity: actual.frame.velocity,
            })
        {
            return Err(
                "Mecc response pose/health/velocity differs from the current canonical step".into(),
            );
        }
        Ok(self.after)
    }
}
impl Frame {
    pub(crate) fn validate(&self) -> Result<(), &'static str> {
        if !self
            .before
            .into_iter()
            .chain(self.position)
            .chain(self.contact_previous)
            .chain(self.angles)
            .chain([self.scale])
            .chain(self.velocity)
            .chain(self.model_rates)
            .all(f32::is_finite)
            || self.scale <= 0.
            || self.angles.iter().any(|v| v.abs() > 1_000_000.)
        {
            return Err("controlled Mecc projection outside finite acquired pose profile");
        }
        Ok(())
    }
}
impl Owned {
    pub(crate) fn current(&self, fields: &crate::actor_fields::Fields, step: u64) -> bool {
        self.step == step
            && fields.position.map(f32::to_bits) == self.frame.position.map(f32::to_bits)
            && fields.current.map(f32::to_bits) == self.frame.contact_previous.map(f32::to_bits)
            && fields.previous.map(f32::to_bits) == self.previous_history.map(f32::to_bits)
            && fields.angles.map(f32::to_bits) == self.frame.angles.map(f32::to_bits)
            && fields.scale.to_bits() == self.frame.scale.to_bits()
    }
}
