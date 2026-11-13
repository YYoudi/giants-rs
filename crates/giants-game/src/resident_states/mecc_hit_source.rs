//! Canonical enemy-shot attribution for the common Mecc receiver. Source
//! validation is not a health mutation or acknowledgement of reaction services.
use super::*;
impl Session {
    pub fn enemy_projectile_mecc_source(
        &self,
        recipient: Id,
        shot: Id,
    ) -> Result<crate::mecc_hit_source::Gate<Id>, String> {
        use crate::{
            enemy_projectile::Phase,
            mecc_hit_source::{Actor, Source, singleplayer},
        };
        self.mecc_health(recipient)?;
        let target = self.get(recipient).ok_or("Mecc recipient absent")?;
        let target_fields = target.construction_fields.as_ref().unwrap();
        let source = self.get(shot).ok_or("enemy hit source absent or foreign")?;
        let metadata = source
            .enemy_projectile
            .as_ref()
            .ok_or("enemy source constructor absent")?;
        if !source.acquired_construction
            || !matches!(source.class, 90 | 232)
            || source.seed.storage.profile() != Profile::Debug
            || source.seed.storage.state() != 6
            || source.seed.storage.data().is_none_or(|b| b.len() != 36)
            || metadata.resource_class != source.class
            || !matches!(
                metadata.phase,
                Phase::Launched | Phase::ImpactServices | Phase::ObjectImpactServices
            )
            || metadata.failure.is_some()
            || source.attachment.is_some()
            || source.presentation_parent.is_some()
        {
            return Err("Mecc source requires an actual live owned enemy projectile".into());
        }
        let launch = metadata
            .launch
            .as_ref()
            .ok_or("enemy source launch absent")?;
        let owner = self
            .get(launch.source)
            .ok_or("enemy projectile owner absent or foreign")?;
        if !owner.acquired_construction
            || !matches!(owner.class, 73 | 230)
            || owner.seed.storage.profile() != Profile::Debug
            || owner.seed.storage.state() != 23
        {
            return Err("enemy shot requires its acquired Ripper owner".into());
        }
        let source_fields = source
            .construction_fields
            .as_ref()
            .ok_or("enemy source fields absent")?;
        let owner_fields = owner
            .construction_fields
            .as_ref()
            .ok_or("enemy owner fields absent")?;
        if target_fields.class != 2
            || target_fields.configuration[20] != 2
            || source_fields.class != source.class
            || source_fields.configuration[20] != 6
            || owner_fields.class != owner.class
            || owner_fields.configuration[20] != 23
        {
            return Err("enemy Mecc source fields differ from their canonical states".into());
        }
        Ok(singleplayer(
            Actor {
                id: recipient,
                class: target.class,
                state: 2,
                context: target_fields.dispatch_context,
            },
            Some(Source {
                actor: Actor {
                    id: shot,
                    class: source.class,
                    state: 6,
                    context: source_fields.dispatch_context,
                },
                owner: Some(Actor {
                    id: launch.source,
                    class: owner.class,
                    state: 23,
                    context: owner_fields.dispatch_context,
                }),
            }),
        ))
    }
}
