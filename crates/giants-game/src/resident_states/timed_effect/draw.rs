//! Acquired common unlit draw inputs for canonical mixed bodies. Prototype PS2
//! `object_render` is an unknown search route; body creation is not draw readiness.
use super::*;
impl Session {
    pub fn mixed_particle_draw_state(
        &self,
        id: Id,
    ) -> Result<Option<crate::smoke_particle_draw::State>, String> {
        let actor = self.get(id).ok_or("mixed draw actor absent or foreign")?;
        let m = actor
            .timed_effect
            .as_ref()
            .ok_or("mixed draw owner absent")?;
        if !m.runtime.mixed_draw()
            || m.cinematic_scope.is_some()
            || !matches!(m.placement, Placement::Position(_))
        {
            return Err("mixed draw outside acquired ordinary canonical profile".into());
        }
        if m.runtime.failure.is_some() || !m.runtime.idle() || m.ordinary.preparing() {
            return Err("mixed draw callback/preparation pending or quarantined".into());
        }
        if !m.ordinary.prepared || m.immediate.is_some() || m.runtime.hidden {
            return Ok(None);
        }
        let f = actor
            .construction_fields
            .as_ref()
            .ok_or("mixed draw fields absent")?;
        if !f.scale.is_finite()
            || f.scale <= 0.
            || actor.seed.storage.state() != 78
            || f.class != actor.class
            || !self.list(0).contains(&id)
        {
            return Err("mixed draw header/registration changed".into());
        }
        let routes = crate::draw_routes::admit(78, f.object_flags, m.render_flags);
        let passes = u8::from(routes.ordinary) + u8::from(routes.effect);
        if passes == 0 {
            return Ok(None);
        }
        Ok(Some(crate::smoke_particle_draw::State {
            class: actor.class,
            passes,
            world: m.matrix,
            personality: f.configuration[14],
            position: f.position,
            scale: f.scale,
            angle: f.angles[0],
            transparency: m.runtime.transparency,
            epoch: m.runtime.epoch,
        }))
    }
}
