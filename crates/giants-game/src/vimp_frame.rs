//! Ordinary Vimp common-list iteration after a supplied world/history admission.
//! Active FX scheduling, other modes and mission-wide traversal remain separate.
use crate::{
    resident_states::{ActorWorldServices, Id, Session},
    vimp_wander::Tick,
};
pub struct Frame<'a> {
    pub number: u64,
    pub tick: Tick<'a>,
    pub controlled_actor: Option<Id>,
}
/// Read-only publication of a fully consumed local common-node iteration.
/// The number counts completed callbacks; it is not the world's frame clock.
pub struct Completed {
    pub number: u64,
    pub animation_epoch: u32,
}
pub struct Services<'a> {
    pub animation: crate::vimp_animation::Services<'a>,
    pub ground: &'a mut dyn FnMut([f32; 2]) -> Result<crate::physical_ground::Ground, &'static str>,
    /// Required only by a hurt primary. Absence is a readiness failure, not an
    /// assumed silent success for a requested actor sound stop.
    pub recovery_sounds: Option<&'a mut dyn crate::vimp_hurt::Sounds>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum History {
    Combined,
    Prepared,
    Constructor,
}
pub(crate) struct Adapter<'a, 'b, 'c> {
    pub factory: &'a crate::vimp_actor::Factory,
    pub globals: &'a giants_formats::global_data::GlobalData,
    pub frame: Frame<'a>,
    pub services: &'a mut Services<'b>,
    pub living: Option<&'c mut dyn crate::vimp_living::Services>,
}
impl ActorWorldServices for Adapter<'_, '_, '_> {
    fn controlled_actor(&self) -> Option<Id> {
        self.frame.controlled_actor
    }
    fn primary(&mut self, session: &mut Session, source: Id) -> Result<(), String> {
        session.vimp_primary_history(source)?;
        if session.vimp(source).is_some_and(|v| !v.initialized) {
            session.initialize_vimp_offline(
                source,
                self.globals,
                self.services.animation.random,
            )?;
            if session.actor_world_flags(source)? & 0xc000 != 0 {
                return Ok(());
            }
        }
        if session
            .vimp(source)
            .is_some_and(|v| matches!(v.mode, 2..=4))
        {
            let sounds = self
                .services
                .recovery_sounds
                .as_mut()
                .ok_or("Vimp recovery sound-stop service absent")?;
            return session.advance_vimp_recovery_primary_offline(
                source,
                crate::vimp_recovery::Primary {
                    number: session.vimp_recovery_primaries(source).unwrap(),
                    tick: self.frame.tick,
                },
                self.factory,
                self.globals,
                &mut crate::vimp_recovery::Services {
                    animation: crate::vimp_animation::Services {
                        blending: self.services.animation.blending,
                        contexts: self.services.animation.contexts,
                        random: self.services.animation.random,
                        stamp: self.services.animation.stamp,
                    },
                    sounds: &mut **sounds,
                    ground: self.services.ground,
                },
            );
        }
        if let Some(living) = self.living.as_deref_mut() {
            session.begin_vimp_living_primary(
                source,
                self.frame.tick,
                self.factory,
                self.globals,
                self.services.animation.random,
                living,
            )?;
        } else {
            session.begin_vimp_wander_offline(
                source,
                self.frame.tick,
                self.factory,
                self.globals,
                self.services.animation.random,
            )?;
        }
        session.advance_vimp_physical_offline(
            source,
            self.factory,
            self.globals,
            &mut *self.services.ground,
        )?;
        if let Some(living) = self.living.as_deref_mut() {
            return session.advance_vimp_living_animation(
                source,
                self.factory,
                self.globals,
                crate::vimp_animation::Services {
                    blending: self.services.animation.blending,
                    contexts: self.services.animation.contexts,
                    random: self.services.animation.random,
                    stamp: self.services.animation.stamp,
                },
                living,
            );
        }
        session.advance_vimp_animation_offline(
            source,
            self.factory,
            self.globals,
            crate::vimp_animation::Services {
                blending: self.services.animation.blending,
                contexts: self.services.animation.contexts,
                random: self.services.animation.random,
                stamp: self.services.animation.stamp,
            },
        )
    }
    fn secondary(&mut self, _: &mut Session, _: Id) -> Result<(), String> {
        Err("Vimp fallback secondary not acquired".into())
    }
    fn object_time(&mut self, _: &Session, _: Id) -> Result<f32, String> {
        Err("Vimp fallback clock not acquired".into())
    }
    fn animation(&mut self, _: &mut Session, _: Id, _: f32) -> Result<(), String> {
        Err("Vimp fallback ANM must not duplicate the primary".into())
    }
    fn finalization(&mut self, _: &mut Session, _: Id) -> Result<(), String> {
        Err("Vimp fallback finalizer not acquired".into())
    }
}
