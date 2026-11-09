use super::*;

/// Required local cue transport. Implementations must resolve the actual owner
/// and acknowledge their real source/device outcome. VM timing is not audio RNG.
pub trait Voice {
    fn play(&mut self, session: &Session, source: Id, sound: u32) -> Result<(), String>;
}

impl Session {
    /// Owned nonlethal impact damage followed by its acquired local cue/timer
    /// tail. Earlier damage and completed timing remain published on late failure.
    /// This is not authority production, active FX execution or a world update.
    pub fn damage_mecc_impact_nonlethal(
        &mut self,
        source: Id,
        amount: f32,
        context: crate::mecc_impact::Context,
        random: &mut crate::audio_parameters::Random,
        voice: &mut (impl Voice + ?Sized),
    ) -> Result<crate::mecc_damage::Result, String> {
        let shared_timer = self
            .mecc_countdown_world
            .ok_or("impact voice shared countdown owner absent")?;
        if !context.offline
            || shared_timer.to_bits() != context.damage.ambient_voice_timer.to_bits()
        {
            return Err("impact voice requires matching offline shared timer ownership".into());
        }
        let result = self.damage_unattributed_mecc_nonlethal_inner(
            source,
            16,
            amount,
            context.damage,
            true,
            true,
        )?;
        if context.damage.globally_disabled || result.effective_amount <= 0. {
            return Ok(result);
        }
        let payload = self.get(source).unwrap().seed.storage.data().unwrap();
        let mut state = crate::mecc_impact_voice::State {
            damage: result.state.damage,
            limit: result.state.limit,
            pain_timer: f32::from_le_bytes(payload[0x1fc..0x200].try_into().unwrap()),
            shared_timer,
        };
        let outcome =
            crate::mecc_impact_voice::after_damage(&mut state, random, |sound, state, _| {
                self.publish_impact_voice_timer(source, state.pain_timer)?;
                self.mecc_countdown_world = Some(state.shared_timer);
                voice.play(self, source, sound)
            });
        self.mecc_countdown_world = Some(state.shared_timer);
        self.publish_impact_voice_timer(source, state.pain_timer)?;
        outcome.map(|()| result)
    }

    fn publish_impact_voice_timer(&mut self, source: Id, timer: f32) -> Result<(), String> {
        if self.get(source).is_none() {
            return Err("impact voice owner retired or foreign during cue".into());
        }
        let payload = self
            .residents
            .get_mut(&source.serial)
            .unwrap()
            .seed
            .storage
            .data_mut()
            .ok_or("impact voice owner payload absent after cue")?;
        if payload.len() != 524 {
            return Err("impact voice owner payload changed during cue".into());
        }
        payload[0x1fc..0x200].copy_from_slice(&timer.to_le_bytes());
        Ok(())
    }
}

/// Publishes the real secondary boundary before invoking owned health. Earlier
/// timers/health writes remain visible even if a later effect/channel fails.
pub(super) struct Damage<'a, 'v> {
    pub session: &'a mut Session,
    pub source: Id,
    pub context: crate::mecc_impact::Context,
    pub voice: Option<&'v mut dyn Voice>,
}
impl crate::mecc_secondary::ImpactDamage for Damage<'_, '_> {
    fn apply(
        &mut self,
        fields: &mut crate::actor_fields::Fields,
        payload: &mut [u8],
        amount: f32,
    ) -> Result<(), String> {
        self.apply_inner(fields, payload, amount, None)
    }
    fn apply_with_random(
        &mut self,
        fields: &mut crate::actor_fields::Fields,
        payload: &mut [u8],
        amount: f32,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<(), String> {
        self.apply_inner(fields, payload, amount, Some(random))
    }
}
impl Damage<'_, '_> {
    fn apply_inner(
        &mut self,
        fields: &mut crate::actor_fields::Fields,
        payload: &mut [u8],
        amount: f32,
        random: Option<&mut crate::audio_parameters::Random>,
    ) -> Result<(), String> {
        if !self.context.offline {
            return Err("impact damage requires the acquired offline authority profile".into());
        }
        let actor = self
            .session
            .get(self.source)
            .ok_or("impact damage owner absent or foreign")?;
        if actor.class != 2
            || payload.len() != 524
            || actor
                .seed
                .storage
                .data()
                .is_none_or(|p| p.len() != payload.len())
        {
            return Err("impact damage requires the actual normal Mecc payload".into());
        }
        let actor = self.session.residents.get_mut(&self.source.serial).unwrap();
        actor.channels = Some(fields.channels);
        actor.construction_fields = Some(fields.clone());
        actor
            .seed
            .storage
            .data_mut()
            .unwrap()
            .copy_from_slice(payload);
        let result = if let Some(voice) = self.voice.as_deref_mut() {
            let mut context = self.context;
            context.damage.ambient_voice_timer = self
                .session
                .mecc_countdown_world
                .ok_or("impact voice shared countdown owner absent")?;
            self.session.damage_mecc_impact_nonlethal(
                self.source,
                amount,
                context,
                random.ok_or("impact voice requires actual VM random owner")?,
                voice,
            )
        } else {
            self.session
                .damage_unattributed_mecc_nonlethal_inner(
                    self.source,
                    16,
                    amount,
                    self.context.damage,
                    true,
                    false,
                )
                .and_then(|result| {
                    let payload = self
                        .session
                        .get(self.source)
                        .unwrap()
                        .seed
                        .storage
                        .data()
                        .unwrap();
                    let cooldown = f32::from_le_bytes(payload[0x1fc..0x200].try_into().unwrap());
                    if !self.context.damage.globally_disabled
                        && result.effective_amount > 0.
                        && cooldown < 0.
                    {
                        return Err(
                            "impact health committed; actual pain voice service required".into(),
                        );
                    }
                    Ok(result)
                })
        };
        let actor = self.session.get(self.source).unwrap();
        *fields = actor.construction_fields.as_ref().unwrap().clone();
        payload.copy_from_slice(actor.seed.storage.data().unwrap());
        result.map(|_| ())
    }
}
