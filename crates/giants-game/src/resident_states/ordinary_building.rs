//! Class53/state7 phase ownership, independent of Ripper AI or whole-world ticks.
use super::{Id, OwnedAffine, Session, affine_pose_words};
use crate::ordinary_building_body::{Body, Resources};

impl Session {
    pub fn prepare_ordinary_building_body(
        &mut self,
        id: Id,
        collision_identity: u32,
        source: &crate::scene_resources::SceneResources,
        ground: impl FnMut([f32; 3]) -> Result<(f32, [f32; 4]), &'static str>,
    ) -> Result<(), String> {
        let actor = self.get(id).ok_or("pub body actor absent or foreign")?;
        if collision_identity == 0
            || self.residents.values().any(|r| {
                r.ordinary_building
                    .as_ref()
                    .is_some_and(|b| b.collision_identity() == collision_identity)
            })
        {
            return Err("pub collision identity is absent or already owned".into());
        }
        if !actor.acquired_construction
            || actor.class != 53
            || actor.seed.storage.profile() != crate::state_storage::Profile::Debug
            || actor.seed.storage.state() != 7
            || actor.seed.storage.data().is_some()
            || actor.ordinary_building.is_some()
            || actor.ordinary_motion.is_some()
            || actor.finalization.is_some()
            || actor.environment.is_some()
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || actor.world_tick.is_some()
            || actor.world_tick_running
        {
            return Err("pub body requires fresh source-owned construction".into());
        }
        let spec = self
            .specification(id)
            .ok_or("pub specification is not owned")?;
        let binding = source
            .classes
            .models
            .first_selection(53)
            .ok_or("pub source binding absent")?
            .as_ref()
            .map_err(Clone::clone)?;
        if binding.class_catalog != spec.class_catalog
            || binding.spec_catalog != spec.specification_catalog
            || binding.spec_id != spec.specification
            || !spec.needs.is_empty()
            || !self
                .state_resources(id)
                .ok_or("pub state resources absent")?
                .needs()
                .is_empty()
        {
            return Err("pub body resources disagree with its constructor".into());
        }
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("pub fields absent")?;
        if actor.channels != Some(fields.channels) {
            return Err("pub body requires its unchanged constructor channels".into());
        }
        let cache = if fields.object_flags & 0x20 != 0 {
            Some(self.actor_current_affine(id)?)
        } else {
            None
        };
        let (body, fields) = Body::initialize(
            collision_identity,
            fields.clone(),
            Resources::load(source)?,
            cache,
            ground,
        )?;
        let affine = OwnedAffine {
            value: body.affine()?,
            pose: affine_pose_words(&fields),
            external_orientation: false,
        };
        let actor = self.residents.get_mut(&id.serial).unwrap();
        actor.construction_fields = Some(fields);
        actor.affine_cache = Some(affine);
        actor.ordinary_building = Some(body);
        Ok(())
    }
    pub fn ordinary_building_body(&self, id: Id) -> Result<&Body, String> {
        let actor = self.get(id).ok_or("pub body actor absent or foreign")?;
        let body = actor
            .ordinary_building
            .as_ref()
            .ok_or("pub body not initialized")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("pub fields absent")?;
        if actor.class != 53
            || actor.seed.storage.state() != 7
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || actor.channels != Some(fields.channels)
            || !body.matches_header(fields)
        {
            return Err("pub body header is stale".into());
        }
        Ok(body)
    }
    /// Consume one ordinary movement/finalization phase with actual caller
    /// services. This does not acknowledge secondary callbacks or a world tick.
    pub fn advance_ordinary_building_body(
        &mut self,
        id: Id,
        number: u64,
        time: crate::resident_motion::Time<'_>,
        services: &mut impl crate::actor_finalization::Services,
    ) -> Result<bool, String> {
        if self.ordinary_building_body(id)?.steps() != number {
            return Err("pub body phase ordinal is stale or skipped".into());
        }
        let actor = self.get(id).unwrap();
        if actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || actor.world_tick_running
        {
            return Err("pub phase owner changed during its ordinary body sequence".into());
        }
        let actor = self.residents.get_mut(&id.serial).unwrap();
        let body = actor.ordinary_building.as_mut().unwrap();
        let fields = actor.construction_fields.as_mut().unwrap();
        let result = body.advance(fields, time, services);
        actor.affine_cache = if body.failure().is_none() {
            Some(OwnedAffine {
                value: body.affine()?,
                pose: affine_pose_words(fields),
                external_orientation: false,
            })
        } else {
            None
        };
        result
    }
    pub fn ordinary_building_player_mesh(
        &self,
        id: Id,
    ) -> Result<crate::player_world_collision::Object, String> {
        self.ordinary_building_body(id)?.player_mesh()
    }
}
