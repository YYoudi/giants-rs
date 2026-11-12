//! Persistent ordinary class73 callers. Prototype PS2 search key `ripper_move`
//! remains an unknown attribution; PC debug-1.1 0x004d61f0 is the acquired
//! callback reference. The machine's motion/failure latch stays with the actor.
use super::*;
use crate::{
    audio_parameters::Random,
    ripper_primary_turn::{FrameInput, Outcome, Services},
};
type Effect = crate::attached_effect_context::Id;

impl Session {
    /// Source ANM channels/references/events after the completed common motion,
    /// before finalization. Prototype PS2 search key `ripper_animselect` remains
    /// an unknown attribution (PC debug-1.1 secondary 0x004db670).
    pub fn advance_ripper_animation(
        &mut self,
        id: Id,
        seconds: f32,
        stamp: &mut crate::animation_epoch::Stamp,
        contexts: &mut crate::attached_effect_context::Pool<Id>,
        random: &mut Random,
    ) -> Result<Option<u32>, String> {
        let actor = self.ordinary_frame_actor(id)?;
        if self.actor_finalization(id)?.phase()
            != crate::resident_finalization::Phase::MotionFinished
            || actor.ripper_animation_consumed
            || actor.ripper_animation_failed
        {
            return Err("Ripper ANM requires unconsumed completed common motion".into());
        }
        let class = actor.class;
        let mut fields = actor
            .construction_fields
            .as_ref()
            .ok_or("Ripper ANM fields absent")?
            .clone();
        let mut channels = actor.channels.ok_or("Ripper ANM channels absent")?;
        let specification = self
            .specification(id)
            .ok_or("Ripper ANM resources absent")?
            .clone();
        let mut references = self.specifications.get(&id.serial).unwrap().0.clone();
        let mut bank = self
            .ripper_events
            .get(&class)
            .cloned()
            .unwrap_or_else(|| crate::mecc_animation_events::Bank::new(specification.clone()));
        if !bank.belongs_to(&specification) {
            return Err("Ripper ANM event resources replaced".into());
        }
        let mut durations = vec![None; 256];
        for (&slot, descriptor) in &specification.animations {
            durations[usize::from(slot)] = Some(descriptor.template.duration);
        }
        let model: [f32; 3] = self.actor_motion(id)?.rates()[..3].try_into().unwrap();
        let tick = crate::animation_channels::advance(
            &mut channels,
            &durations,
            model,
            seconds,
            crate::arithmetic::Precision::Double53,
        )?;
        if tick.missing != 0 {
            return Err("Ripper ANM source descriptor absent".into());
        }
        let mut epoch = actor.ripper_animation_epoch;
        let result: Result<(), String> = (|| {
            if tick.processed != 0 {
                epoch = Some(stamp.advance());
                references.update_current(&channels, epoch.unwrap())?;
                let prior = self.actor_finalization(id)?.state();
                bank.dispatch(
                    crate::mecc_animation_events::Input {
                        owner: id,
                        fields: &fields,
                        channels: &channels,
                        references: references.matrices(),
                        affine: prior.affine.map(|a| a.matrix),
                        // An absent acquired support needs no material lookup.
                        // A present support remains unknown until its provider runs.
                        material: if prior.fields.support == 0 {
                            crate::mecc_animation_events::Material::Acquired(None)
                        } else {
                            crate::mecc_animation_events::Material::Unknown
                        },
                    },
                    contexts,
                    random,
                )?;
            }
            Ok(())
        })();
        // Preserve clocks/references and any external event attempt on failure.
        // History rearms this phase only after successful finalization.
        fields.channels = channels;
        let actor = self.residents.get_mut(&id.serial).unwrap();
        actor.channels = Some(channels);
        actor.construction_fields = Some(fields);
        actor.ripper_animation_consumed = true;
        actor.ripper_animation_failed = result.is_err();
        actor.ripper_animation_epoch = epoch;
        self.specifications.get_mut(&id.serial).unwrap().0 = references;
        self.ripper_events.insert(class, bank);
        result.map(|()| epoch)
    }

    pub(super) fn ripper_target_shapes(
        &self,
        frame: &crate::ripper_perception::Frame<'_, Id>,
    ) -> Result<BTreeMap<Id, crate::ripper_primary_turn::TargetShape>, String> {
        self.validate_perception_frame(frame)?;
        frame
            .candidates()
            .map(|candidate| {
                let fields = self
                    .get(candidate.id)
                    .and_then(|a| a.construction_fields.as_ref())
                    .ok_or("Ripper attack target fields absent or foreign")?;
                Ok((
                    candidate.id,
                    crate::ripper_primary_turn::TargetShape {
                        scale: fields.scale,
                        center_offset: f32::from_bits(fields.configuration[22]),
                    },
                ))
            })
            .collect()
    }

    pub fn update_ripper_turn(
        &mut self,
        id: Id,
        input: FrameInput<'_, Id>,
        parameters: crate::ripper_turn_completion::Parameters,
        random: &mut Random,
        services: &mut impl Services<Id, Effect>,
    ) -> Result<Outcome, String> {
        self.update_ripper_owned_primary(
            id,
            input,
            random,
            services,
            |_, machine, input, random, services| {
                machine.advance(
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

    pub fn update_ripper_pursuit(
        &mut self,
        id: Id,
        input: FrameInput<'_, Id>,
        attack_percent: f32,
        random: &mut Random,
        services: &mut impl Services<Id, Effect>,
    ) -> Result<Outcome, String> {
        self.update_ripper_owned_primary(
            id,
            input,
            random,
            services,
            |_, machine, input, random, services| {
                machine.advance_pursuit(
                    input.context,
                    input.frame,
                    input.range,
                    attack_percent,
                    random,
                    services,
                )
            },
        )
    }

    pub fn update_ripper_attack(
        &mut self,
        id: Id,
        input: FrameInput<'_, Id>,
        parameters: crate::ripper_attack::Parameters,
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
                machine.advance_attack(
                    input.context,
                    input.frame,
                    input.range,
                    crate::ripper_primary_turn::AttackContext {
                        parameters,
                        target_shapes: &shapes,
                    },
                    random,
                    &mut bridge,
                )
            },
        )
    }
}
