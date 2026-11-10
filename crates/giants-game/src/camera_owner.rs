//! Acquired camera-owner selection prefix, before the remaining camera update.
//! Initial words, controlled/override producers and world scheduling are external.
use crate::resident_states::{Id, Session};

pub struct Input<K> {
    pub controlled: Option<K>,
    pub reference: i32,
    pub override_enabled: u32,
    pub override_owner: Option<K>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome<K> {
    NoControlledActor,
    Selected { owner: K, reference_prepared: bool },
}

pub trait Services<K> {
    fn object_flags(&mut self, owner: K) -> Result<u32, String>;
    /// Required source reference-point service. Its completed writes survive
    /// errors; a successful return precedes the reference-status publication.
    fn prepare_reference(&mut self, owner: K, reference: i32) -> Result<(), String>;
}

pub struct State<K> {
    owner: Option<K>,
    reference_status: u32,
    failure: Option<String>,
}
impl<K: Copy> State<K> {
    /// No startup default is inferred from a selected view or player class.
    pub fn new(owner: Option<K>, reference_status: u32) -> Self {
        Self {
            owner,
            reference_status,
            failure: None,
        }
    }
    pub fn owner(&self) -> Option<K> {
        self.owner
    }
    /// This word is distinct from subjective display activity. A skipped
    /// reference-point preparation does not clear its previous value.
    pub fn reference_status(&self) -> u32 {
        self.reference_status
    }
    pub fn failure(&self) -> Option<&str> {
        self.failure.as_deref()
    }

    pub fn select(
        &mut self,
        input: Input<K>,
        services: &mut impl Services<K>,
    ) -> Result<Outcome<K>, String> {
        if self.failure.is_some() {
            return Err("camera owner selection is quarantined".into());
        }
        let result = self.select_inner(input, services);
        if let Err(error) = &result {
            self.failure = Some(error.clone());
        }
        result
    }
    fn select_inner(
        &mut self,
        input: Input<K>,
        services: &mut impl Services<K>,
    ) -> Result<Outcome<K>, String> {
        let Some(controlled) = input.controlled else {
            return Ok(Outcome::NoControlledActor);
        };
        let retained = if input.reference >= 0 {
            match self.owner {
                Some(owner) => services.object_flags(owner)? & 0x8000 == 0,
                None => false,
            }
        } else {
            false
        };
        let owner = if retained {
            self.owner.unwrap()
        } else if input.override_enabled != 0 {
            input.override_owner.unwrap_or(controlled)
        } else {
            controlled
        };
        self.owner = Some(owner);
        let reference_prepared =
            input.reference >= 0 && services.object_flags(owner)? & 0x8000 == 0;
        if reference_prepared {
            services.prepare_reference(owner, input.reference)?;
            self.reference_status = 1;
        }
        Ok(Outcome::Selected {
            owner,
            reference_prepared,
        })
    }
}

impl State<Id> {
    /// Bind the existing pre-world consumer to this selected owner. The caller
    /// still owns real mode/force/profile production and the enclosing order.
    pub fn preworld<'a>(
        &self,
        session: &Session,
        modes: &'a crate::camera_mode::State,
        forced: u32,
        profile: u8,
        controlled: Option<Id>,
    ) -> Result<crate::mecc_subjective::PreWorldCamera<'a>, String> {
        let owner = self
            .owner_from_session(session)?
            .ok_or("pre-world selected camera owner absent")?;
        Ok(crate::mecc_subjective::PreWorldCamera {
            modes,
            forced,
            profile,
            controlled,
            owner,
        })
    }

    /// Actual Session headers supply flags. The reference-point body is still
    /// required, not replaced by a prepared mesh or a successful no-op.
    pub fn select_from_session(
        &mut self,
        session: &mut Session,
        input: Input<Id>,
        prepare: &mut impl FnMut(&mut Session, Id, i32) -> Result<(), String>,
    ) -> Result<Outcome<Id>, String> {
        struct Bound<'a, F> {
            session: &'a mut Session,
            prepare: &'a mut F,
        }
        impl<F: FnMut(&mut Session, Id, i32) -> Result<(), String>> Services<Id> for Bound<'_, F> {
            fn object_flags(&mut self, owner: Id) -> Result<u32, String> {
                self.session
                    .get(owner)
                    .ok_or("camera selection actor absent or foreign")?
                    .construction_fields
                    .as_ref()
                    .map(|f| f.object_flags)
                    .ok_or_else(|| "camera selection header absent".into())
            }
            fn prepare_reference(&mut self, owner: Id, reference: i32) -> Result<(), String> {
                (self.prepare)(self.session, owner, reference)
            }
        }
        let outcome = self.select(input, &mut Bound { session, prepare })?;
        if let Outcome::Selected { owner, .. } = outcome
            && session.get(owner).is_none()
        {
            // Independent-runtime lifetime guard, including a negative-index
            // selection or a successful callback that retired its owner.
            let error = "selected camera owner absent, foreign or retired".to_string();
            self.failure = Some(error.clone());
            return Err(error);
        }
        Ok(outcome)
    }
    /// Consumer-side lifetime check; callbacks may have removed the owner.
    pub fn owner_from_session(&self, session: &Session) -> Result<Option<Id>, String> {
        if self.failure.is_some() {
            return Err("camera owner selection is quarantined".into());
        }
        self.owner
            .map(|id| {
                session
                    .get(id)
                    .map(|_| id)
                    .ok_or_else(|| "camera owner absent or foreign".into())
            })
            .transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Provider {
        flags: [u32; 3],
        calls: Vec<(u32, i32)>,
        fail: bool,
    }
    impl Services<u32> for Provider {
        fn object_flags(&mut self, owner: u32) -> Result<u32, String> {
            Ok(self.flags[owner as usize])
        }
        fn prepare_reference(&mut self, owner: u32, reference: i32) -> Result<(), String> {
            self.calls.push((owner, reference));
            if self.fail {
                Err("late reference error".into())
            } else {
                Ok(())
            }
        }
    }
    #[test]
    fn retained_owner_precedes_override_and_skips_keep_reference_status() {
        let mut state = State::new(Some(1), 7);
        let mut p = Provider {
            flags: [0, 0, 0x8000],
            calls: vec![],
            fail: false,
        };
        assert_eq!(
            state
                .select(
                    Input {
                        controlled: Some(0),
                        reference: 4,
                        override_enabled: 1,
                        override_owner: Some(2)
                    },
                    &mut p
                )
                .unwrap(),
            Outcome::Selected {
                owner: 1,
                reference_prepared: true
            }
        );
        assert_eq!(p.calls, [(1, 4)]);
        assert_eq!(state.reference_status(), 1);
        assert_eq!(
            state
                .select(
                    Input {
                        controlled: Some(0),
                        reference: -1,
                        override_enabled: 1,
                        override_owner: Some(2)
                    },
                    &mut p
                )
                .unwrap(),
            Outcome::Selected {
                owner: 2,
                reference_prepared: false
            }
        );
        assert_eq!(state.reference_status(), 1);
        assert_eq!(p.calls.len(), 1);
    }
    #[test]
    fn no_controlled_actor_preserves_owner_and_calls_no_provider() {
        let mut state = State::new(Some(100), 7);
        let mut p = Provider {
            flags: [0; 3],
            calls: vec![],
            fail: false,
        };
        assert_eq!(
            state
                .select(
                    Input {
                        controlled: None,
                        reference: 0,
                        override_enabled: 1,
                        override_owner: Some(200)
                    },
                    &mut p
                )
                .unwrap(),
            Outcome::NoControlledActor
        );
        assert_eq!(state.owner(), Some(100));
        assert_eq!(state.reference_status(), 7);
    }
    #[test]
    fn failed_reference_retains_new_owner_and_old_status_without_replay() {
        let mut state = State::new(None, 7);
        let mut p = Provider {
            flags: [0; 3],
            calls: vec![],
            fail: true,
        };
        let input = || Input {
            controlled: Some(0),
            reference: 4,
            override_enabled: 1,
            override_owner: Some(2),
        };
        assert!(state.select(input(), &mut p).is_err());
        assert_eq!(state.owner(), Some(2));
        assert_eq!(state.reference_status(), 7);
        assert!(state.select(input(), &mut p).is_err());
        assert_eq!(p.calls, [(2, 4)]);
    }
}
