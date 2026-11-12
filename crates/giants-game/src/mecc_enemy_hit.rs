//! Source-bearing, offline normal-Mecc nonlethal kind0/2 receiver.
//! Prototype PS2 `mecc_shot` is an unknown hurt-callback correspondence;
//! `mecc_shot.rs` separately owns firing. Source identity, current geometry,
//! equipment and world gates must come from their real owners. This composes
//! acquired health, direction and voice services, not a playable hit backend.
use crate::{
    audio_parameters::Random, damage_impulse, mecc_damage, mecc_hit_source, mecc_impact_voice,
    state_storage::Profile,
};

#[derive(Clone, Copy, Debug)]
pub struct State {
    pub health: mecc_damage::State,
    pub velocity: [f32; 3],
    pub pain_timer: f32,
    pub shared_timer: f32,
    /// Independent payload word reset by the callback. No further semantic
    /// meaning is assigned; its PC field alias remains in the private record.
    pub callback_reset_word: u32,
}
pub struct Request<I> {
    pub recipient: mecc_hit_source::Actor<I>,
    pub source: mecc_hit_source::Source<I>,
    pub associated_player: Option<I>,
    pub kind: u32,
    pub amount: f32,
    pub globally_disabled: bool,
    /// Actual target-specific console/completion suppression result, including
    /// the controlled-player predicate. Neither film activity nor the raw world
    /// word alone supplies that decision for another actor.
    pub suppressed: bool,
    pub gyro_present: bool,
    pub critical_voice_present: bool,
    pub recipient_motion: damage_impulse::Motion,
    pub source_motion: damage_impulse::Motion,
    pub impact: Option<[f32; 3]>,
    pub radial_center: [f32; 3],
}
pub trait Services<I> {
    /// The actual associated-player owner stores the candidate only into its
    /// empty retaliation slot. This is not an actor-header transform parent.
    fn retaliation(&mut self, player: I, candidate: Option<I>) -> Result<(), String>;
    /// Transport sees the committed health/impulse and pre-cue timers. Playback
    /// and audio variation remain the real device's separate responsibilities.
    fn voice(&mut self, cue: u32, state: &State, random: &Random) -> Result<(), String>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Fresh,
    Running,
    Complete,
    Failed,
}
pub struct Machine {
    phase: Phase,
    failure: Option<String>,
}
impl Default for Machine {
    fn default() -> Self {
        Self {
            phase: Phase::Fresh,
            failure: None,
        }
    }
}
impl Machine {
    pub fn phase(&self) -> Phase {
        self.phase
    }
    pub fn failure(&self) -> Option<&str> {
        self.failure.as_deref()
    }

    /// One callback only. A failure retains preceding publication/service/RNG
    /// effects and prohibits replay. Caller must retain the mutated state even
    /// when an actual voice/retaliation consumer fails.
    pub fn run<I: Copy + Eq>(
        &mut self,
        state: &mut State,
        request: Request<I>,
        random: &mut Random,
        services: &mut impl Services<I>,
    ) -> Result<(), String> {
        if self.phase != Phase::Fresh {
            return Err("Mecc enemy hit already consumed or failed".into());
        }
        self.phase = Phase::Running;
        let result = Self::run_inner(state, request, random, services);
        match &result {
            Ok(()) => self.phase = Phase::Complete,
            Err(error) => {
                self.phase = Phase::Failed;
                self.failure = Some(error.clone());
            }
        }
        result
    }
    fn run_inner<I: Copy + Eq>(
        state: &mut State,
        request: Request<I>,
        random: &mut Random,
        services: &mut impl Services<I>,
    ) -> Result<(), String> {
        if request.globally_disabled {
            return Ok(());
        }
        if request.recipient.class != 2
            || request.recipient.state != 2
            || !matches!(request.source.actor.class, 90 | 232)
            || request.source.actor.state != 6
            || request
                .source
                .owner
                .is_none_or(|owner| !matches!(owner.class, 73 | 230) || owner.state != 23)
            || !matches!(request.kind, 0 | 2)
            || !request.amount.is_finite()
            || request.amount < 0.
            || !state.health.alive()
            || state.health.flags & 0x800000 != 0
            || request.gyro_present
            || request.critical_voice_present
        {
            return Err("Mecc enemy hit requires live unshielded non-gyro normal kind0/2 without critical-voice owner".into());
        }
        let gate = mecc_hit_source::singleplayer(request.recipient, Some(request.source));
        let health = state.health.apply_for(
            Profile::Debug,
            mecc_damage::Hit {
                kind: request.kind as i32,
                amount: request.amount,
                globally_disabled: false,
                cutscene: request.suppressed,
                allowed: gate.damage_allowed,
                shield_percent: 0.,
            },
        )?;
        if health.died {
            return Err("Mecc enemy hit requires the actual lethal continuation".into());
        }
        // Nonlethal bounded-profile preflight precedes externally visible work.
        // Once admitted, native retaliation precedes health and reaction flags.
        if let Some(player) = request.associated_player {
            services.retaliation(player, gate.retaliation)?;
        }
        state.health = health.state;
        state.callback_reset_word = u32::MAX;
        // Health suppression and a denied same-context hit do not skip impulse.
        state.velocity = damage_impulse::apply(
            state.velocity,
            damage_impulse::Hit {
                kind: request.kind,
                horizontal: health.effective_amount,
                vertical: health.effective_amount,
                recipient: request.recipient_motion,
                source: Some(request.source_motion),
                impact: request.impact,
                radial_center: request.radial_center,
            },
        )?;
        if health.effective_amount > 0. {
            let mut voice = mecc_impact_voice::State {
                damage: state.health.damage,
                limit: state.health.limit,
                pain_timer: state.pain_timer,
                shared_timer: state.shared_timer,
            };
            let result =
                mecc_impact_voice::after_damage(&mut voice, random, |cue, current, rng| {
                    state.pain_timer = current.pain_timer;
                    state.shared_timer = current.shared_timer;
                    services.voice(cue, state, rng)
                });
            state.pain_timer = voice.pain_timer;
            state.shared_timer = voice.shared_timer;
            result?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
