//! Actual state6 expiry tail. Prototype PS2 `projectile_logic` and
//! `projectile_cleanup` are unknown navigation relationships, not attributions.
//! The PC identity, previous aliases and supplied-boundary controls stay private.
use super::*;
use crate::{
    audio_parameters::Random,
    enemy_projectile::{self, FlightOutcome, Phase},
};

struct View {
    position: [f32; 3],
    action: u32,
    cues: [u32; 2],
    parameters: [u32; 5],
    damage: f32,
    context: u32,
}
impl Session {
    /// Consume the real retained expiry without querying terrain, supplying a
    /// collision point or subtracting another clock. Sound precedes FX, unlike
    /// ground contact. Same-callback ground+expiry retains both receipts.
    /// The shared FX/radial/audio service protocol supplies no successful defaults.
    pub fn complete_enemy_projectile_expiry(
        &mut self,
        id: Id,
        random: &mut Random,
        services: &mut impl enemy_projectile::GroundImpactServices,
    ) -> Result<(), String> {
        let actor = self.get(id).ok_or("expiry source absent or foreign")?;
        let meta = actor
            .enemy_projectile
            .as_ref()
            .ok_or("expiry constructor absent")?;
        let after_ground = meta.ground_impact.is_some();
        self.enemy_expiry_view(
            id,
            if after_ground {
                Phase::Retiring
            } else {
                Phase::Launched
            },
            after_ground,
            None,
        )?;
        let token = Arc::new(());
        let meta = self
            .residents
            .get_mut(&id.serial)
            .unwrap()
            .enemy_projectile
            .as_mut()
            .unwrap();
        meta.phase = Phase::ExpiryServices;
        meta.expiry_attempt = Some(token.clone());
        let result: Result<(), String> = (|| {
            let view =
                self.enemy_expiry_view(id, Phase::ExpiryServices, after_ground, Some(&token))?;
            let cue = if view.cues[0] == 0 {
                None
            } else {
                Some(
                    if view.cues[1] != 0 && (random.next_15() * 100) >> 15 >= 50 {
                        view.cues[1]
                    } else {
                        view.cues[0]
                    },
                )
            };
            let sound = cue
                .map(|cue| services.sound(self, cue, view.position))
                .transpose()?;
            let view =
                self.enemy_expiry_view(id, Phase::ExpiryServices, after_ground, Some(&token))?;
            let effect = services.effect(
                self,
                enemy_projectile::impact_effect(view.action, view.position)?,
                random,
            )?;
            let view =
                self.enemy_expiry_view(id, Phase::ExpiryServices, after_ground, Some(&token))?;
            let radial = f32::from_bits(view.parameters[3]) != 0. && view.damage >= 0.;
            if radial {
                services.radial(
                    self,
                    enemy_projectile::RadialRequest {
                        source: id,
                        other: None,
                        position: view.position,
                        parameters: view.parameters,
                        context: view.context,
                    },
                    random,
                )?;
            }
            self.enemy_expiry_view(id, Phase::ExpiryServices, after_ground, Some(&token))?;
            let actor = self.residents.get_mut(&id.serial).unwrap();
            actor.construction_fields.as_mut().unwrap().object_flags |= 0x8000;
            let meta = actor.enemy_projectile.as_mut().unwrap();
            meta.expiry_impact = Some(enemy_projectile::ExpiryImpact {
                effect,
                sound,
                cue,
                radial,
                after_ground,
            });
            meta.pending_flight = None;
            meta.phase = Phase::Retiring;
            Ok(())
        })();
        if let Err(error) = &result
            && let Some(meta) = self
                .residents
                .get_mut(&id.serial)
                .and_then(|a| a.enemy_projectile.as_mut())
            && meta
                .expiry_attempt
                .as_ref()
                .is_some_and(|actual| Arc::ptr_eq(actual, &token))
        {
            meta.failure = Some(error.clone());
            meta.phase = Phase::Failed;
        }
        result
    }
    fn enemy_expiry_view(
        &self,
        id: Id,
        phase: Phase,
        after_ground: bool,
        token: Option<&Arc<()>>,
    ) -> Result<View, String> {
        let actor = self.get(id).ok_or("expiry scope removed its source")?;
        let meta = actor
            .enemy_projectile
            .as_ref()
            .ok_or("expiry source constructor removed")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("expiry source fields removed")?;
        let body = actor.seed.storage.data().ok_or("expiry payload absent")?;
        if !actor.acquired_construction
            || actor.seed.storage.profile() != Profile::Debug
            || actor.seed.storage.state() != 6
            || !matches!(actor.class, 90 | 232)
            || body.len() != 36
            || fields.class != actor.class
            || fields.configuration[20] != 6
            || meta.resource_class != actor.class
            || meta.phase != phase
            || meta.failure.is_some()
            || meta.launch.is_none()
            || meta.initial_environment.is_none()
            || meta.flight_revision == 0
            || !matches!(meta.pending_flight, Some(FlightOutcome::Expired))
            || meta.ground_flight_clock.is_some()
            || meta.object_impact.is_some()
            || meta.expiry_impact.is_some()
            || meta.ground_impact.is_some() != after_ground
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || fields.object_flags & 0x4000 != 0
            || (fields.object_flags & 0x8000 != 0) != after_ground
            || match (token, meta.expiry_attempt.as_ref()) {
                (None, None) => false,
                (Some(expected), Some(actual)) => !Arc::ptr_eq(expected, actual),
                _ => true,
            }
        {
            return Err(
                "expiry requires its actual unconsumed single-player state6 flight and scope"
                    .into(),
            );
        }
        let word = |at| u32::from_le_bytes(body[at..at + 4].try_into().unwrap());
        let parameters = [12, 20, 16, 28, 32].map(word);
        let damage = f32::from_bits(fields.configuration[12]);
        let remaining = f32::from_bits(word(4));
        if !remaining.is_finite()
            || remaining > 0.
            || !damage.is_finite()
            || !fields.position.into_iter().all(f32::is_finite)
            || !parameters
                .map(f32::from_bits)
                .into_iter()
                .all(f32::is_finite)
        {
            return Err("expiry fields outside finite acquired profile".into());
        }
        Ok(View {
            position: fields.position,
            action: fields.action,
            cues: [meta.sound_words[1], meta.sound_words[2]],
            parameters,
            damage,
            context: meta.launch.as_ref().unwrap().dispatch_context,
        })
    }
}
