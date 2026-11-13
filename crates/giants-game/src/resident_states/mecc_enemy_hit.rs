//! Canonical cold-player direct-hit publication. Prototype PS2 `mecc_shot`
//! remains an unknown hurt-callback correspondence, separate from firing.
use super::*;
use crate::{audio_parameters::Random, enemy_projectile, mecc_enemy_hit as receiver};

#[derive(Clone, Copy)]
pub struct Gates {
    pub globally_disabled: bool,
    /// Actual controlled-player console/completion decision, not film activity.
    pub health_suppressed: bool,
}
/// Required real world, controller and local-cue owners. This adapter has no
/// default guards, copied controller, synthetic voice acceptance or device.
pub trait Services {
    fn gates(&mut self, session: &Session, target: Id) -> Result<Gates, String>;
    /// Copy the actual health/velocity response, including health-only changes.
    /// This is not a request to subtract damage again or move the player.
    fn recoil(
        &mut self,
        session: &Session,
        response: crate::controlled_mecc::HitResponse,
    ) -> Result<(), String>;
    fn voice(&mut self, session: &Session, target: Id, cue: u32) -> Result<(), String>;
}

impl Session {
    /// Partial-controller prefix over the actual shared world-clock sample.
    /// Multiple fixed physics steps within one sample must not subtract the
    /// frame delta repeatedly. The full primary owns its separate clock path.
    pub(crate) fn advance_controlled_mecc_hit_clock(
        &mut self,
        target: Id,
        step: u64,
        sample: u64,
        seconds: f32,
    ) -> Result<bool, String> {
        self.controlled_mecc_motion(target, step)?;
        self.mecc_health(target)?;
        let actor = self.get(target).unwrap();
        if sample == 0 || actor.mecc_physics_present || !actor.mecc_hit_relations_cleared {
            return Err(
                "controlled Mecc hit clock needs its actual cold partial-controller sample".into(),
            );
        }
        if !seconds.is_finite() || seconds < 0. {
            return Err("controlled Mecc hit clock needs finite nonnegative global seconds".into());
        }
        if let Some(previous) = actor.controlled_hit_clock_sample {
            if sample < previous {
                return Err("controlled Mecc hit clock sample is stale".into());
            }
            if sample == previous {
                return Ok(false);
            }
        }
        let world = self
            .mecc_countdown_world
            .ok_or("controlled Mecc shared voice clock absent")?;
        let payload = actor.seed.storage.data().unwrap();
        let pain = f32::from_le_bytes(payload[0x1fc..0x200].try_into().unwrap());
        let output = crate::mecc_countdowns::advance(world, pain, seconds)?;
        let actor = self.residents.get_mut(&target.serial).unwrap();
        actor.seed.storage.data_mut().unwrap()[0x1fc..0x200]
            .copy_from_slice(&output.actor.to_le_bytes());
        actor.controlled_hit_clock_sample = Some(sample);
        self.mecc_countdown_world = Some(output.world);
        Ok(true)
    }

    /// Positive kind0 callback for its actual pending projectile contact with
    /// the ordinary controller's current cold Mecc. Canonical health, reset,
    /// timers and velocity publish before cue transport. The source wrapper
    /// separately owns marking, terminal processing and object retirement.
    pub fn damage_mecc_from_enemy_object(
        &mut self,
        request: enemy_projectile::ObjectImpact,
        step: u64,
        random: &mut Random,
        services: &mut impl Services,
    ) -> Result<(), String> {
        let source = self.get(request.source).ok_or("enemy Mecc source absent")?;
        let meta = source
            .enemy_projectile
            .as_ref()
            .ok_or("enemy Mecc constructor absent")?;
        let fields = source
            .construction_fields
            .as_ref()
            .ok_or("enemy source fields absent")?;
        if meta.phase != enemy_projectile::Phase::ObjectImpactServices
            || meta.mecc_receiver.is_some()
            || meta.failure.is_some()
            || !request.amount.is_finite()
            || request.amount <= 0.
            || request.amount.to_bits() != fields.configuration[12]
            || !matches!(meta.pending_flight, Some(enemy_projectile::FlightOutcome::ObjectContact { target, point, tag })
                if target == request.target && point.map(f32::to_bits) == request.point.map(f32::to_bits) && tag == request.tag)
        {
            return Err("Mecc receiver needs its unconsumed actual direct-contact request".into());
        }
        // Source/owner/state/health checks share the canonical attribution gate.
        self.enemy_projectile_mecc_source(request.target, request.source)?;
        self.residents
            .get_mut(&request.source.serial)
            .unwrap()
            .enemy_projectile
            .as_mut()
            .unwrap()
            .mecc_receiver = Some(receiver::Phase::Running);
        let result = self.damage_mecc_from_enemy_object_inner(request, step, random, services);
        if let Some(meta) = self
            .residents
            .get_mut(&request.source.serial)
            .and_then(|a| a.enemy_projectile.as_mut())
        {
            meta.mecc_receiver = Some(if result.is_ok() {
                receiver::Phase::Complete
            } else {
                receiver::Phase::Failed
            });
            if let Err(error) = &result {
                meta.phase = enemy_projectile::Phase::Failed;
                meta.failure = Some(error.clone());
            }
        }
        result
    }

    fn damage_mecc_from_enemy_object_inner(
        &mut self,
        request: enemy_projectile::ObjectImpact,
        step: u64,
        random: &mut Random,
        services: &mut impl Services,
    ) -> Result<(), String> {
        let gates = services.gates(self, request.target)?;
        if gates.globally_disabled {
            return Ok(());
        }
        let health = self.mecc_health(request.target)?;
        let target = self.get(request.target).unwrap();
        if !target.mecc_hit_relations_cleared {
            return Err(
                "Mecc enemy hit needs acquired cold associated-player/voice ownership".into(),
            );
        }
        let payload = target.seed.storage.data().unwrap();
        let word = |at| u32::from_le_bytes(payload[at..at + 4].try_into().unwrap());
        // Existing cold equipment profile: a potential gyro continuation cannot
        // be admitted as ordinary unshielded damage.
        if word(0xfc) != 0 || word(0x190) == 17 {
            return Err("Mecc enemy hit requires the actual gyro/equipment continuation".into());
        }
        let motion = self.controlled_mecc_motion(request.target, step)?.frame;
        let fields = target.construction_fields.as_ref().unwrap();
        let recipient = crate::mecc_hit_source::Actor {
            id: request.target,
            class: target.class,
            state: 2,
            context: fields.dispatch_context,
        };
        let recipient_motion = crate::damage_impulse::Motion {
            current: fields.position,
            previous: fields.current,
        };
        let source = self.get(request.source).unwrap();
        let source_fields = source.construction_fields.as_ref().unwrap();
        let owner_id = source
            .enemy_projectile
            .as_ref()
            .unwrap()
            .launch
            .as_ref()
            .unwrap()
            .source;
        let owner = self.get(owner_id).unwrap();
        let source_identity = crate::mecc_hit_source::Source {
            actor: crate::mecc_hit_source::Actor {
                id: request.source,
                class: source.class,
                state: 6,
                context: source_fields.dispatch_context,
            },
            owner: Some(crate::mecc_hit_source::Actor {
                id: owner_id,
                class: owner.class,
                state: 23,
                context: owner.construction_fields.as_ref().unwrap().dispatch_context,
            }),
        };
        let source_motion = crate::damage_impulse::Motion {
            current: source_fields.position,
            previous: source_fields.current,
        };
        let mut state = receiver::State {
            health,
            velocity: motion.velocity,
            pain_timer: f32::from_bits(word(0x1fc)),
            shared_timer: self
                .mecc_countdown_world
                .ok_or("Mecc enemy shared voice clock absent")?,
            callback_reset_word: word(0xf4),
        };
        let mut machine = receiver::Machine::default();
        let mut bridge = Bridge {
            session: self,
            target: request.target,
            step,
            services,
            source: request.source,
            controller: crate::controlled_mecc::HitState {
                health,
                velocity: motion.velocity,
            },
        };
        let result = machine.run(
            &mut state,
            receiver::Request {
                recipient,
                source: source_identity,
                associated_player: None,
                kind: 0,
                amount: request.amount,
                globally_disabled: false,
                suppressed: gates.health_suppressed,
                gyro_present: false,
                critical_voice_present: false,
                recipient_motion,
                source_motion,
                impact: Some(request.point),
                // Kind0 consumes source motion; the radial center is not read.
                radial_center: recipient_motion.current,
            },
            random,
            &mut bridge,
        );
        // Retain every completed health/timer/impulse mutation on late failure.
        bridge
            .session
            .publish_enemy_mecc_state(request.target, step, state)?;
        result?;
        bridge.recoil(state)
    }

    fn publish_enemy_mecc_state(
        &mut self,
        target: Id,
        step: u64,
        state: receiver::State,
    ) -> Result<(), String> {
        self.mecc_health(target)?;
        self.controlled_mecc_motion(target, step)?;
        let actor = self.residents.get_mut(&target.serial).unwrap();
        actor.mecc_damage = Some(state.health.damage);
        actor.construction_fields.as_mut().unwrap().control_flags = state.health.flags;
        let payload = actor.seed.storage.data_mut().unwrap();
        payload[0x4c..0x50].copy_from_slice(&state.health.shield.to_le_bytes());
        payload[0xf4..0xf8].copy_from_slice(&state.callback_reset_word.to_le_bytes());
        payload[0x1fc..0x200].copy_from_slice(&state.pain_timer.to_le_bytes());
        actor.controlled_motion.as_mut().unwrap().frame.velocity = state.velocity;
        self.mecc_countdown_world = Some(state.shared_timer);
        Ok(())
    }
}
struct Bridge<'a, S> {
    session: &'a mut Session,
    target: Id,
    step: u64,
    services: &'a mut S,
    source: Id,
    controller: crate::controlled_mecc::HitState,
}
impl<S: Services> Bridge<'_, S> {
    fn recoil(&mut self, state: receiver::State) -> Result<(), String> {
        let after = crate::controlled_mecc::HitState {
            health: state.health,
            velocity: state.velocity,
        };
        if !after.same_bits(self.controller) {
            self.services.recoil(
                self.session,
                crate::controlled_mecc::HitResponse {
                    source: self.source,
                    owner: self.target,
                    step: self.step,
                    before: self.controller,
                    after,
                },
            )?;
            self.controller = after;
        }
        Ok(())
    }
}
impl<S: Services> receiver::Services<Id> for Bridge<'_, S> {
    fn retaliation(&mut self, _: Id, _: Option<Id>) -> Result<(), String> {
        Err("cold Mecc callback unexpectedly requested an associated-player store".into())
    }
    fn voice(&mut self, cue: u32, state: &receiver::State, _: &Random) -> Result<(), String> {
        self.session
            .publish_enemy_mecc_state(self.target, self.step, *state)?;
        self.recoil(*state)?;
        self.services.voice(self.session, self.target, cue)
    }
}
