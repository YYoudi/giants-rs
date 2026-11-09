use super::*;

/// Publishes the real secondary boundary before invoking owned health. Earlier
/// timers/health writes remain visible even if a later effect/channel fails.
pub(super) struct Damage<'a> {
    pub session: &'a mut Session,
    pub source: Id,
    pub context: crate::mecc_impact::Context,
}
impl crate::mecc_secondary::ImpactDamage for Damage<'_> {
    fn apply(
        &mut self,
        fields: &mut crate::actor_fields::Fields,
        payload: &mut [u8],
        amount: f32,
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
        let result = self
            .session
            .damage_unattributed_mecc_nonlethal_inner(
                self.source,
                16,
                amount,
                self.context.damage,
                true,
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
            });
        let actor = self.session.get(self.source).unwrap();
        *fields = actor.construction_fields.as_ref().unwrap().clone();
        payload.copy_from_slice(actor.seed.storage.data().unwrap());
        result.map(|_| ())
    }
}
