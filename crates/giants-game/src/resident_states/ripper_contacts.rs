//! Contacts from the acquired PC state23 current frame. Prototype PS2 search
//! key `ripper_move` remains an unknown attribution; this adapter composes the
//! existing common geometry consumers, not an asserted prototype counterpart.
use super::*;

impl Session {
    /// The component supplies its completed iteration number, while Session
    /// validates the actual finished world/motion/ANM and current affine/bounds.
    /// Constructor poses, render LOD and marker geometry cannot substitute.
    pub fn ripper_contact_snapshot(
        &mut self,
        id: Id,
        number: u64,
    ) -> Result<crate::actor_contacts::Snapshot, String> {
        let actor = self.ordinary_frame_actor(id)?;
        if number == 0
            || actor.world_tick_running
            || actor.ripper_primary_running
            || actor
                .world_tick
                .as_ref()
                .is_none_or(|tick| tick.phase() != crate::actor_world_tick::Phase::Finished)
            || !actor.ripper_animation_consumed
            || actor.ripper_animation_failed
            || self.actor_motion(id)?.awaiting_motion()
            || self.actor_finalization(id)?.phase() != crate::resident_finalization::Phase::Idle
        {
            return Err("Ripper contact requires a completed ordinary frame".into());
        }
        let epoch = actor
            .ripper_animation_epoch
            .ok_or("Ripper current ANM epoch absent")?;
        let fields = actor.construction_fields.as_ref().unwrap().clone();
        let state = self.state23_snapshot(id)?;
        let bounds = self.actor_environment(id)?.geometry_fields().bounds;
        let (mesh, triangles) = self.contact_geometry(id, epoch)?;
        Ok(crate::actor_contacts::Snapshot {
            actor: crate::projectile_contacts::Actor {
                id,
                parent: None,
                class: fields.class,
                state: 23,
                context: fields.dispatch_context,
                type_word: state.mode,
                flags: DirectFlags {
                    active: fields.object_flags,
                    personality: fields.configuration[14],
                },
            },
            frame: number,
            response_flags: fields.response_flags,
            bounds: crate::volume_queries::Bounds {
                min: bounds[..3].try_into().unwrap(),
                max: bounds[3..].try_into().unwrap(),
            },
            mesh,
            triangles,
        })
    }
}
