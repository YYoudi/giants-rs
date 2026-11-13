//! Class230's owned mode0/61..68 callers and actual marker/projectile constructors. Other modes,
//! live world admission and audio submission remain separate required services.
use super::*;
use crate::{
    audio_parameters::Random,
    enemy_projectile::SoundOutcome,
    ripper_primary_turn::{FrameInput, Outcome, Services, State, VariantAwakeningContext},
};
#[cfg(test)]
mod tests;
type Effect = crate::attached_effect_context::Id;

struct Bridge<'a, 'r, S> {
    session: &'a mut Session,
    owner: Id,
    factory: &'a mut crate::primary_surface_marker::Factory<'r>,
    services: &'a mut S,
}
impl<S: Services<Id, Effect>> Services<Id, Effect> for Bridge<'_, '_, S> {
    fn owner_sound(
        &mut self,
        state: &State<Id, Effect>,
        cue: u32,
        random: &mut Random,
    ) -> Result<SoundOutcome, String> {
        self.services.owner_sound(state, cue, random)
    }
    fn stop_owner_sound(
        &mut self,
        state: &State<Id, Effect>,
        cue: u32,
        random: &mut Random,
    ) -> Result<SoundOutcome, String> {
        self.services.stop_owner_sound(state, cue, random)
    }
    fn remove_effect(
        &mut self,
        state: &State<Id, Effect>,
        effect: Effect,
        random: &mut Random,
    ) -> Result<(), String> {
        self.services.remove_effect(state, effect, random)
    }
    fn construct_surface_marker(&mut self, state: &State<Id, Effect>) -> Result<Id, String> {
        let actor = self.session.ordinary_frame_actor(self.owner)?;
        let fields = actor.construction_fields.as_ref().unwrap();
        if !actor.ripper_primary_running
            || actor.class != 230
            || state.entry.class != 230
            || state.entry.mode != 62
            || state.entry.action != 0
            || state.parameter_184.to_bits() != 0
            || actor.channels.is_none_or(|c| c[0] != state.primary)
            || fields.position.map(f32::to_bits) != state.entry.position.map(f32::to_bits)
            || fields.position.map(f32::to_bits) != state.steering.position.map(f32::to_bits)
            || fields.configuration[1] != state.entry.parameter.to_bits()
            || fields.configuration[14] & !0x800 != state.steering.personality
            || (fields.object_flags ^ state.entry.flags) & !0x03000000 != 0
            || !self.session.actor_motion(self.owner)?.awaiting_motion()
            || self.session.actor_finalization(self.owner)?.phase()
                != crate::resident_finalization::Phase::MotionStarted
        {
            return Err("variant marker differs from its owned primary prefix".into());
        }
        // The actual copy writes only the destination. Unlike the class73
        // surfacing route, this caller does not reset source rates/history and
        // does not condition construction on positive world height.
        self.factory.construct(self.session, self.owner)
    }
}

impl Session {
    pub fn update_ripper_variant_melee(
        &mut self,
        id: Id,
        input: FrameInput<'_, Id>,
        parameters: crate::ripper_primary_turn::VariantMeleeContext,
        random: &mut Random,
        services: &mut impl Services<Id, Effect>,
    ) -> Result<Outcome, String> {
        self.update_ripper_owned_primary(
            id,
            input,
            random,
            services,
            |_, machine, input, random, services| {
                machine.advance_variant_melee(input, parameters, random, services)
            },
        )
    }
    pub fn update_ripper_variant_close(
        &mut self,
        id: Id,
        input: FrameInput<'_, Id>,
        parameters: crate::ripper_primary_turn::VariantCloseContext,
        random: &mut Random,
        services: &mut impl Services<Id, Effect>,
    ) -> Result<Outcome, String> {
        self.update_ripper_owned_primary(
            id,
            input,
            random,
            services,
            |_, machine, input, random, services| {
                machine.advance_variant_close(input, parameters, random, services)
            },
        )
    }
    pub fn update_ripper_variant_cooldown(
        &mut self,
        id: Id,
        input: FrameInput<'_, Id>,
        parameters: crate::ripper_primary_turn::VariantCooldownContext<'_>,
        random: &mut Random,
        services: &mut impl Services<Id, Effect>,
    ) -> Result<Outcome, String> {
        self.update_ripper_owned_primary(
            id,
            input,
            random,
            services,
            |_, machine, input, random, services| {
                machine.advance_variant_cooldown(input, parameters, random, services)
            },
        )
    }
    /// The owned primary reads target dimensions from this Session and launches
    /// through the same real state6 factory as the class73 caller.
    pub fn update_ripper_variant_attack(
        &mut self,
        id: Id,
        input: FrameInput<'_, Id>,
        parameters: crate::ripper_attack::VariantParameters,
        factory: &mut crate::primary_enemy_fire::Factory<'_>,
        random: &mut Random,
        dependencies: &mut impl crate::primary_enemy_fire::Dependencies<Effect>,
    ) -> Result<Outcome, String> {
        let shapes = self.ripper_target_shapes(input.frame)?;
        self.update_ripper_owned_primary(
            id,
            input,
            random,
            &mut (factory, dependencies),
            |session, machine, input, random, (factory, dependencies)| {
                let mut bridge = crate::primary_enemy_fire::Bridge::new(
                    session,
                    factory,
                    id,
                    crate::primary_enemy_fire::Scope::OwnedPrimary,
                    *dependencies,
                );
                machine.advance_variant_attack(
                    input,
                    crate::ripper_primary_turn::VariantAttackContext {
                        parameters,
                        target_shapes: &shapes,
                    },
                    random,
                    &mut bridge,
                )
            },
        )
    }
    pub fn update_ripper_variant_pursuit(
        &mut self,
        id: Id,
        input: FrameInput<'_, Id>,
        parameters: crate::ripper_primary_turn::VariantDecisionContext,
        random: &mut Random,
        services: &mut impl Services<Id, Effect>,
    ) -> Result<Outcome, String> {
        self.update_ripper_owned_primary(
            id,
            input,
            random,
            services,
            |_, machine, input, random, services| {
                machine.advance_variant_pursuit(input, parameters, random, services)
            },
        )
    }
    pub fn update_ripper_variant_decision(
        &mut self,
        id: Id,
        input: FrameInput<'_, Id>,
        parameters: crate::ripper_primary_turn::VariantDecisionContext,
        random: &mut Random,
        services: &mut impl Services<Id, Effect>,
    ) -> Result<Outcome, String> {
        self.update_ripper_owned_primary(
            id,
            input,
            random,
            services,
            |_, machine, input, random, services| {
                machine.advance_variant_decision(input, parameters, random, services)
            },
        )
    }
    pub fn update_ripper_variant_turn(
        &mut self,
        id: Id,
        input: FrameInput<'_, Id>,
        random: &mut Random,
        services: &mut impl Services<Id, Effect>,
    ) -> Result<Outcome, String> {
        self.update_ripper_owned_primary(
            id,
            input,
            random,
            services,
            |_, machine, input, random, services| {
                machine.advance_variant_turn(input, random, services)
            },
        )
    }
    pub fn update_ripper_variant_awakening(
        &mut self,
        id: Id,
        input: FrameInput<'_, Id>,
        parameters: VariantAwakeningContext<'_>,
        factory: &mut crate::primary_surface_marker::Factory<'_>,
        random: &mut Random,
        services: &mut impl Services<Id, Effect>,
    ) -> Result<Outcome, String> {
        self.update_ripper_owned_primary(
            id,
            input,
            random,
            services,
            |session, machine, input, random, services| {
                let mut bridge = Bridge {
                    session,
                    owner: id,
                    factory,
                    services,
                };
                machine.advance_variant_awakening(input, parameters, random, &mut bridge)
            },
        )
    }
}
