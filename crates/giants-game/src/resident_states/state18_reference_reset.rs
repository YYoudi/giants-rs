use super::*;
use crate::state18_reference_reset::{self as reset, Candidate, Plan, Preferred};

impl Session {
    /// Read-only route over actual owned header metadata and list13. The
    /// preferred relationship is an explicit caller input until its producer
    /// and lifetime are bound. A plan publishes no actor or readiness state.
    pub fn plan_state18_reference_reset(
        &self,
        source: Id,
        preferred: Option<Id>,
    ) -> Result<Plan<Id>, String> {
        let actor = self
            .get(source)
            .ok_or("state18 reset source absent or foreign")?;
        if actor.seed.storage.profile() != Profile::Debug
            || actor.seed.storage.state() != 18
            || actor.seed.storage.data().is_none_or(|d| d.len() != 200)
        {
            return Err("state18 reset planning outside acquired header profile".into());
        }
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("state18 reset source fields absent")?;
        reset::choose(
            fields.action,
            fields.dispatch_context,
            || {
                preferred
                    .map(|id| {
                        let actor = self
                            .get(id)
                            .ok_or("preferred reset actor absent or foreign")?;
                        if actor.seed.storage.profile() != Profile::Debug {
                            return Err("preferred reset actor profile unacquired".into());
                        }
                        Ok(Preferred {
                            identity: id,
                            state: u32::from(actor.seed.storage.state()),
                        })
                    })
                    .transpose()
            },
            std::iter::once_with(|| self.list(13)).flatten().map(|id| {
                let actor = self.get(id).ok_or("reset list actor absent or foreign")?;
                let fields = actor
                    .construction_fields
                    .as_ref()
                    .ok_or("reset list actor fields absent")?;
                Ok(Candidate {
                    identity: id,
                    context: fields.dispatch_context,
                })
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn header(s: &mut Session, state: u8, action: u32, context: u32, active: u32) -> Id {
        let seed = Seed::from_program_for(
            &giants_formats::class_program::parse(&[0x1e, state, 0xff]).unwrap(),
            Profile::Debug,
            crate::arithmetic::Precision::Double53,
        )
        .unwrap();
        let class = 1000 + i32::from(state);
        let id = s
            .register(
                class,
                &seed,
                &[],
                DirectFlags {
                    active,
                    personality: 0,
                },
            )
            .unwrap();
        let mut configuration = [0; 30];
        configuration[20] = u32::from(state);
        // Declared header-only fixture, not a resource constructor or live actor.
        s.residents.get_mut(&id.serial).unwrap().construction_fields =
            Some(crate::actor_fields::Fields {
                class,
                dispatch_context: context,
                configuration,
                scale: 1.,
                object_flags: active,
                response_flags: 0x80000000,
                control_flags: 0,
                action,
                shadow_reciprocal: 0.,
                shadow: None,
                shadow_texture: None,
                gunlist: Default::default(),
                channels: [[0; 9]; 4],
                position: [1., 2., 3.],
                current: [4., 5., 6.],
                previous: [7., 8., 9.],
                angles: [0.; 3],
            });
        id
    }
    #[test]
    fn owned_route_preserves_list_order_and_refuses_foreign_used_metadata() {
        let mut s = Session::new(91);
        let source = header(&mut s, 18, 10, 0x80000001, 0);
        let first = header(&mut s, 74, 0, 50, 0x4000);
        let matching = header(&mut s, 32, 0, 0x80000001, 0x40000000);
        let preferred = header(&mut s, 82, 0, 1, 0);
        let mut other = Session::new(92);
        let foreign = header(&mut other, 82, 0, 1, 0);
        let list = s.list(13);
        assert_eq!(list, [first, matching, preferred]);
        let payload = s.get(source).unwrap().seed.storage.data().unwrap().to_vec();
        assert_eq!(
            s.plan_state18_reference_reset(source, None).unwrap(),
            Plan::Reset(matching)
        );
        assert_eq!(
            s.plan_state18_reference_reset(source, Some(preferred))
                .unwrap(),
            Plan::Reset(preferred)
        );
        assert!(
            s.plan_state18_reference_reset(source, Some(foreign))
                .is_err()
        );
        assert!(s.plan_state18_reference_reset(foreign, None).is_err());
        assert_eq!(s.list(13), list);
        assert_eq!(s.get(source).unwrap().seed.storage.data().unwrap(), payload);
        let f = s
            .residents
            .get_mut(&source.serial)
            .unwrap()
            .construction_fields
            .as_mut()
            .unwrap();
        f.action = 0;
        assert_eq!(
            s.plan_state18_reference_reset(source, Some(foreign))
                .unwrap(),
            Plan::Unchanged
        );
        s.residents
            .get_mut(&source.serial)
            .unwrap()
            .construction_fields
            .as_mut()
            .unwrap()
            .action = 10;
        s.residents
            .get_mut(&first.serial)
            .unwrap()
            .construction_fields
            .as_mut()
            .unwrap()
            .dispatch_context = 0x80000001;
        assert_eq!(
            s.plan_state18_reference_reset(source, None).unwrap(),
            Plan::Reset(first)
        );
    }
}
