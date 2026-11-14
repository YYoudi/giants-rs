//! Canonical class230 mode68 -> direct kind3 -> cold controlled-Mecc path.
//! Prototype PS2 ripper_move/mecc_shot/hurt_object remain unknown name routes.
//! This supplies no world gates, controller, sound device or mission terminal.
use super::*;
// Prototype PS2 search routes `ripper_move` and `hurt_object` remain unknown
// correspondences. The surface relationship below is the acquired PC contract.
use crate::{audio_parameters::Random, controlled_mecc::HitOrigin, ripper_primary_turn as primary};
type Effect = crate::attached_effect_context::Id;
type State = primary::State<Id, Effect>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Phase {
    Wrapper,
    Receiver,
    Terminal,
    Complete,
    Failed,
}
#[derive(Clone, Debug)]
pub(super) struct Attempt {
    pub epoch: u64,
    pub target: Id,
    pub step: u64,
    pub phase: Phase,
    pub failure: Option<String>,
}
/// All dependencies must be supplied by their real owners. A no-effect terminal
/// proof is available for the actual target; possible credit needs its owner.
pub trait Services: primary::Services<Id, Effect> + MeccEnemyHitServices {
    fn world_suppressed(&mut self, session: &Session) -> Result<bool, String>;
    fn terminal(&mut self, session: &mut Session, source: Id, target: Id) -> Result<(), String>;
}

struct Bridge<'a, S> {
    session: &'a mut Session,
    source: Id,
    step: u64,
    parameters: primary::VariantMeleeContext,
    services: &'a mut S,
}
impl<S: Services> primary::Services<Id, Effect> for Bridge<'_, S> {
    fn owner_sound(
        &mut self,
        state: &State,
        cue: u32,
        random: &mut Random,
    ) -> Result<crate::enemy_projectile::SoundOutcome, String> {
        self.services.owner_sound(state, cue, random)
    }
    fn stop_owner_sound(
        &mut self,
        state: &State,
        cue: u32,
        random: &mut Random,
    ) -> Result<crate::enemy_projectile::SoundOutcome, String> {
        self.services.stop_owner_sound(state, cue, random)
    }
    fn remove_effect(
        &mut self,
        state: &State,
        effect: Effect,
        random: &mut Random,
    ) -> Result<(), String> {
        self.services.remove_effect(state, effect, random)
    }
    fn melee(
        &mut self,
        state: &State,
        target: Id,
        amount: f32,
        random: &mut Random,
    ) -> Result<(), String> {
        self.session.begin_ripper_melee(
            self.source,
            target,
            self.step,
            self.parameters,
            state,
            amount,
        )?;
        let result = self.run(target, amount, random);
        self.session
            .get(self.source)
            .ok_or("kind3 source scope removed during its callback")?;
        let attempt = self
            .session
            .residents
            .get_mut(&self.source.serial)
            .and_then(|actor| actor.ripper_melee.as_mut())
            .ok_or("kind3 source operation removed during its callback")?;
        attempt.phase = if result.is_ok() {
            Phase::Complete
        } else {
            Phase::Failed
        };
        attempt.failure = result.as_ref().err().cloned();
        result
    }
}
impl<S: Services> Bridge<'_, S> {
    fn run(&mut self, target: Id, amount: f32, random: &mut Random) -> Result<(), String> {
        if self.services.world_suppressed(self.session)? {
            return Ok(());
        }
        let fields = self
            .session
            .get(target)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap();
        if fields.configuration[15] & 0x4000000 != 0 {
            return Ok(());
        }
        if fields.configuration[14] & 0x4000000 == 0 {
            let attempt = self
                .session
                .residents
                .get_mut(&self.source.serial)
                .unwrap()
                .ripper_melee
                .as_mut()
                .unwrap();
            attempt.phase = Phase::Receiver;
            let epoch = attempt.epoch;
            self.session.damage_mecc_from_source_inner(
                mecc_enemy_hit::Hit {
                    source: self.source,
                    target,
                    amount,
                    point: None,
                    origin: HitOrigin::RipperMelee { epoch },
                },
                self.step,
                random,
                self.services,
            )?;
        }
        self.session
            .residents
            .get_mut(&self.source.serial)
            .unwrap()
            .ripper_melee
            .as_mut()
            .unwrap()
            .phase = Phase::Terminal;
        self.services.terminal(self.session, self.source, target)
    }
}
impl Session {
    /// The same real owned primary/ANM/frame path, with a canonical kind3 Mecc
    /// consumer instead of the offline required-service boundary. No projectile
    /// identity, retained query point or projectile cleanup is synthesized.
    pub fn update_ripper_variant_melee_with_mecc(
        &mut self,
        source: Id,
        input: primary::FrameInput<'_, Id>,
        parameters: primary::VariantMeleeContext,
        step: u64,
        random: &mut Random,
        services: &mut impl Services,
    ) -> Result<primary::Outcome, String> {
        self.update_ripper_owned_primary(
            source,
            input,
            random,
            services,
            |session, machine, input, random, services| {
                machine.advance_variant_melee(
                    input,
                    parameters,
                    random,
                    &mut Bridge {
                        session,
                        source,
                        step,
                        parameters,
                        services,
                    },
                )
            },
        )
    }
    fn begin_ripper_melee(
        &mut self,
        source: Id,
        target: Id,
        step: u64,
        parameters: primary::VariantMeleeContext,
        state: &State,
        amount: f32,
    ) -> Result<(), String> {
        self.mecc_health(target)?;
        self.controlled_mecc_motion(target, step)?;
        let recipient = self.get(target).ok_or("melee target absent")?;
        let recipient_fields = recipient.construction_fields.as_ref().unwrap();
        if recipient.attachment.is_some()
            || recipient.presentation_parent.is_some()
            || recipient_fields.class != 2
            || recipient_fields.configuration[20] != 2
        {
            return Err("kind3 Mecc adapter requires its actual unparented controller".into());
        }
        let actor = self.ordinary_frame_actor(source)?;
        let fields = actor.construction_fields.as_ref().unwrap();
        let combat = self.state23_snapshot(source)?;
        // The surface callback link is not the transform/firing parent. An
        // ordinarily surfaced Ripper retains this reciprocal marker while it
        // attacks. Accept only the actual owned constructor relationship;
        // arbitrary callback-owner words are still outside this adapter.
        let surface_link_matches = match (combat.owner, actor.surface_link) {
            (None, None) => true,
            (Some((marker, 219)), Some(SurfaceLink::Owner { marker: linked }))
                if marker == linked =>
            {
                self.get(marker).is_some_and(|marker| {
                    marker.acquired_construction
                        && marker.class == 219
                        && marker.seed.storage.profile() == Profile::Debug
                        && marker.seed.storage.state() == 48
                        && marker.constructor_mask == 0x11
                        && matches!(marker.surface_link,
                            Some(SurfaceLink::Marker { owner, .. }) if owner == source)
                        && marker.attachment.is_none()
                        && marker.presentation_parent.is_none()
                })
            }
            _ => false,
        };
        let delta = std::array::from_fn::<_, 3, _>(|i| {
            f64::from(recipient.construction_fields.as_ref().unwrap().position[i])
                - f64::from(fields.position[i])
        });
        let distance = ((delta[0] * delta[0] + delta[1] * delta[1]) + delta[2] * delta[2]) as f32;
        if !actor.ripper_primary_running
            || actor.ripper_primary_epoch == 0
            || actor.ripper_melee.is_some()
            || !actor.acquired_construction
            || actor.class != 230
            || actor.seed.storage.profile() != Profile::Debug
            || actor.seed.storage.state() != 23
            || actor.enemy_projectile.is_some()
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || combat.mode != 68
            || !surface_link_matches
            || combat.parent.is_some()
            || !combat.damage.is_finite()
            || !combat.limit.is_finite()
            || combat.limit <= 0.
            || combat.damage >= combat.limit
            || state.entry.class != 230
            || state.entry.mode != 66
            || state.entry.action != 0
            || state.retaliation != Some(target)
            || state.primary[8] & 2 == 0
            || actor.channels.is_none_or(|c| c[0] != state.primary)
            || fields.class != 230
            || fields.configuration[20] != 23
            || fields.action != 0
            || fields.position.map(f32::to_bits) != state.entry.position.map(f32::to_bits)
            || fields.position.map(f32::to_bits) != state.steering.position.map(f32::to_bits)
            || fields.dispatch_context != state.entry.context
            || fields.configuration[14] != state.steering.personality
            || (fields.object_flags ^ state.entry.flags) & !0x03000000 != 0
            || fields.configuration[1] != state.entry.parameter.to_bits()
            || fields.angles[0].to_bits() != state.steering.heading.to_bits()
            || !amount.is_finite()
            || amount <= 0.
            || amount.to_bits() != parameters.damage.to_bits()
            || !distance.is_finite()
            || f64::from(distance) >= f64::from(parameters.attack_range).powi(2)
            || !self.actor_motion(source)?.awaiting_motion()
            || self.actor_finalization(source)?.phase()
                != crate::resident_finalization::Phase::MotionStarted
        {
            return Err(
                "kind3 request differs from its actual finished owned mode68 primary/target/range"
                    .into(),
            );
        }
        let epoch = actor.ripper_primary_epoch;
        let actor = self.residents.get_mut(&source.serial).unwrap();
        let fields = actor.construction_fields.as_mut().unwrap();
        fields.object_flags = state.entry.flags;
        fields.control_flags = state.entry.control;
        let combat = actor.state23_combat.as_mut().unwrap();
        combat.mode = state.entry.mode;
        combat.flags = state.entry.flags;
        combat.control = state.entry.control;
        combat.retaliation = state.retaliation;
        actor.ripper_melee = Some(Attempt {
            epoch,
            target,
            step,
            phase: Phase::Wrapper,
            failure: None,
        });
        Ok(())
    }
    pub(crate) fn validate_ripper_melee_response(
        &self,
        source: Id,
        target: Id,
        step: u64,
        epoch: u64,
    ) -> Result<(), String> {
        let actor = self.get(source).ok_or("melee response source absent")?;
        let attempt = actor
            .ripper_melee
            .as_ref()
            .ok_or("melee response operation absent")?;
        if !actor.ripper_primary_running
            || actor.ripper_primary_epoch != epoch
            || attempt.epoch != epoch
            || attempt.target != target
            || attempt.step != step
            || attempt.phase != Phase::Receiver
            || attempt.failure.is_some()
        {
            return Err(
                "melee response needs its actual current primary/receiver operation".into(),
            );
        }
        let combat = self.state23_snapshot(source)?;
        if combat.mode != 66 || combat.retaliation != Some(target) {
            return Err("melee response source changed during the callback".into());
        }
        self.mecc_health(target)?;
        self.controlled_mecc_motion(target, step)?;
        Ok(())
    }
    /// Read-only no-effect proof for this actual active generic post-hurt tail.
    /// A possible treasury or mission credit still requires its real consumer.
    pub fn ripper_melee_terminal_no_effect(&self, source: Id, target: Id) -> Result<bool, String> {
        let actor = self.get(source).ok_or("melee terminal source absent")?;
        let attempt = actor
            .ripper_melee
            .as_ref()
            .ok_or("melee terminal operation absent")?;
        if !actor.ripper_primary_running
            || actor.ripper_primary_epoch != attempt.epoch
            || attempt.target != target
            || attempt.phase != Phase::Terminal
            || attempt.failure.is_some()
        {
            return Err("melee terminal proof needs its actual active operation".into());
        }
        self.post_hurt_target_no_effect(target)
    }
}
