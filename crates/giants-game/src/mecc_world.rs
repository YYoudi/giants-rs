//! One declared offline Mecc iteration, after the separate common-history pass.
//! Population, traversal, children and surrounding world phases remain external.
use crate::resident_states::{ActorWorldServices, Id, Session};

pub struct Projection {
    pub outcome: crate::actor_world_tick::Outcome,
    pub primary: Option<crate::mecc_physical_cycle::InterleavedProjection>,
}

pub(crate) struct Adapter<'e, 'a, 'r, 'c, 'g> {
    pub environment: Option<crate::mecc_physical_cycle::Environment<'e>>,
    pub services: Option<crate::mecc_primary::Services<'a, 'r, 'c>>,
    pub ground: &'g mut dyn FnMut([f32; 2]) -> Result<crate::physical_ground::Ground, &'static str>,
    pub controlled: Option<Id>,
    pub primary: Option<crate::mecc_physical_cycle::InterleavedProjection>,
}
impl ActorWorldServices for Adapter<'_, '_, '_, '_, '_> {
    fn controlled_actor(&self) -> Option<Id> {
        self.controlled
    }
    fn primary(&mut self, session: &mut Session, source: Id) -> Result<(), String> {
        self.primary = Some(
            session.advance_mecc_primary_with_history(
                source,
                self.environment
                    .take()
                    .ok_or("Mecc primary environment already consumed")?,
                &mut *self.ground,
                self.services
                    .take()
                    .ok_or("Mecc primary services already consumed")?,
                true,
            )?,
        );
        Ok(())
    }
    fn secondary(&mut self, _: &mut Session, _: Id) -> Result<(), String> {
        Err("Mecc world fallback secondary is outside the acquired primary profile".into())
    }
    fn object_time(&mut self, _: &Session, _: Id) -> Result<f32, String> {
        Err("Mecc world fallback clock must not duplicate the primary".into())
    }
    fn animation(&mut self, _: &mut Session, _: Id, _: f32) -> Result<(), String> {
        Err("Mecc world fallback ANM must not duplicate the primary".into())
    }
    fn finalization(&mut self, _: &mut Session, _: Id) -> Result<(), String> {
        Err("Mecc world fallback finalization is outside the acquired profile".into())
    }
}
