//! Ground-contact requests of the ordinary effect callback, after physical
//! services have published the actual contact flags. This is not a ground query
//! or a successful child factory. Literal prototype search routes `fx_logic`
//! and `fx_processOpCodes__FP6FxDataiP6OBJECT` remain unconfirmed relationships.
//! Related PC aliases are retained in the private attribution worksheets.
//! Specific search keys: `fx_setupGroundHit__FP10FXSTACKTAGi` (unknown shared
//! flag-writer relationship) and `fx_setupGroundHitCreate__FP10FXSTACKTAGii`
//! (probable relationship to the PC opcode5a block, not a PC function identity).

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Input {
    pub control_flags: u32,
    pub response_flags: u32,
    pub object_flags: u32,
    pub physical_flags: u32,
    /// Source5a stores an integer word, not a floating-point duration.
    pub remaining: i32,
    pub effect: u32,
    pub position: [u32; 3],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Emission {
    pub effect: u32,
    pub position: [u32; 3],
    pub flags: u32,
    /// Publish only after the real factory call has completed.
    pub remaining_after: i32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Retirement {
    pub allow_source_child: bool,
    pub publish_position: bool,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Decision {
    pub emission: Option<Emission>,
    pub retirement: Option<Retirement>,
}

/// Retained source relationship, separate from the actor's mutable inline
/// budget. The resolved index belongs to this exact source image, not a native
/// pointer or an identity copied between binaries.
#[derive(Clone, Debug)]
pub struct Source {
    child: Option<crate::effect_scene::DeferredEffectBinding>,
    stop_on_contact: bool,
    entry: Option<u32>,
}
impl Source {
    pub fn child(&self) -> Option<&crate::effect_scene::DeferredEffectBinding> {
        self.child.as_ref()
    }
    pub fn resolve(
        needs: &crate::effect_scene::ObjectServiceNeeds,
        image: &giants_formats::effect_image::Image,
    ) -> Result<Option<Self>, String> {
        if !needs.source16 && needs.source5a.is_none() {
            return Ok(None);
        }
        if needs.logged_context || needs.retirement.is_some() {
            return Err("ground effect also requires context/retirement services".into());
        }
        let entry = needs
            .source5a
            .as_ref()
            .map(|binding| {
                let (index, actual) = image.find(&binding.effect).ok_or("ground child absent")?;
                if actual.offset != binding.program_offset {
                    return Err("ground child source relationship changed");
                }
                u32::try_from(index).map_err(|_| "ground child source index overflow")
            })
            .transpose()?;
        Ok(Some(Self {
            child: needs.source5a.clone(),
            stop_on_contact: needs.source16,
            entry,
        }))
    }

    /// Apply the two VM writers to the actual actor. Lifetime, geometry,
    /// movement, context retirement and callback admission are separate owners.
    #[doc(alias = "fx_setupGroundHitCreate")]
    pub fn configure(
        &self,
        fields: &mut crate::actor_fields::Fields,
        seed: &mut crate::state_storage::Seed,
    ) -> Result<(), String> {
        crate::effect_actor::physical_data(seed)?;
        let data = seed.storage.data_mut().unwrap();
        if self.stop_on_contact {
            fields.control_flags |= 0x200;
        }
        if let (Some(child), Some(entry)) = (&self.child, self.entry) {
            data[0x20..0x24].copy_from_slice(&child.parameter.to_le_bytes());
            data[0x24..0x28].copy_from_slice(&entry.to_le_bytes());
            fields.control_flags |= 0x0200_0000;
        }
        Ok(())
    }

    /// Read the canonical header and inline physical contact flags. A caller
    /// must execute the real physical stage before requesting this decision.
    pub fn decision(
        &self,
        fields: &crate::actor_fields::Fields,
        seed: &crate::state_storage::Seed,
    ) -> Result<Decision, String> {
        crate::effect_actor::physical_data(seed)?;
        let data = seed.storage.data().unwrap();
        let word = |at| u32::from_le_bytes(data[at..at + 4].try_into().unwrap());
        if (fields.control_flags & 0x200 != 0) != self.stop_on_contact
            || (fields.control_flags & 0x0200_0000 != 0) != self.child.is_some()
            || self.entry.is_some_and(|entry| word(0x24) != entry)
        {
            return Err("ground effect source/header relationship changed".into());
        }
        decide(Input {
            control_flags: fields.control_flags,
            response_flags: fields.response_flags,
            object_flags: fields.object_flags,
            physical_flags: word(0xa0),
            remaining: word(0x20) as i32,
            effect: word(0x24),
            position: fields.position.map(f32::to_bits),
        })
        .map_err(str::to_owned)
    }

    /// A successful real child call is the only budget writer. A repeated or
    /// stale completion cannot consume a second unit. Failures must quarantine
    /// the composing callback, rather than replay its already executed services.
    pub fn emitted(
        &self,
        seed: &mut crate::state_storage::Seed,
        request: Emission,
    ) -> Result<(), String> {
        crate::effect_actor::physical_data(seed)?;
        let data = seed.storage.data_mut().unwrap();
        let word = |at| u32::from_le_bytes(data[at..at + 4].try_into().unwrap());
        if self.entry != Some(request.effect)
            || word(0x20) == 0
            || word(0x24) != request.effect
            || word(0x20) as i32 != request.remaining_after.wrapping_add(1)
        {
            return Err("ground child completion is stale or foreign".into());
        }
        data[0x20..0x24].copy_from_slice(&request.remaining_after.to_le_bytes());
        Ok(())
    }
}

/// Preserve the ordering: physical services -> optional child factory -> budget
/// store -> optional retirement. Unsupported bounce/surface services refuse;
/// they cannot be acknowledged by calling this decision function.
pub fn decide(input: Input) -> Result<Decision, &'static str> {
    let f = input.control_flags;
    if f & 0x0200_0700 == 0 || input.physical_flags & 4 == 0 {
        return Ok(Decision::default());
    }
    if input.physical_flags & 8 != 0 && f & 0x100 != 0 {
        return Err("effect contact bounce service required");
    }
    if f & 0x8000_0000 != 0 {
        return Err("effect contact surface service required");
    }
    let emission = (f & 0x0200_0000 != 0 && input.remaining != 0).then(|| Emission {
        effect: input.effect,
        position: input.position,
        flags: (u32::from(input.response_flags & 0x400 != 0) * 2)
            | (u32::from(input.response_flags & 0x1000 != 0) * 4)
            | u32::from(input.object_flags & 0x400 != 0),
        remaining_after: input.remaining.wrapping_sub(1),
    });
    let retirement = if f & 0x200 != 0 {
        Some(Retirement {
            allow_source_child: false,
            publish_position: false,
        })
    } else if f & 0x400 != 0 {
        Some(Retirement {
            allow_source_child: true,
            publish_position: true,
        })
    } else {
        None
    };
    Ok(Decision {
        emission,
        retirement,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires GIANTS_MISSION_OWNED_ROOT; contact flags supplied, no live ground/factory acceptance"]
    fn installed_source_budget_changes_only_after_matching_completion() {
        use crate::{
            attached_effect_context::{Creation, Pool},
            audio_parameters::Random,
        };
        let root = std::path::PathBuf::from(std::env::var_os("GIANTS_MISSION_OWNED_ROOT").unwrap());
        let resources =
            crate::scene_resources::SceneResources::load(&root, "w_Story2.bin", "English").unwrap();
        let image = &resources.classes.models.effects.last().unwrap().image;
        let offset = image.find("fxb_blood").unwrap().1.offset;
        let mut pool = Pool::<u32>::from_active_resources(80, &resources);
        let mut random = Random(23);
        let (Creation::Prepared(context), _) = pool
            .prepare_retirement_offset_invariant(Some(offset), [1., 2., 3.], 0, 0x1000, &mut random)
            .unwrap()
        else {
            panic!("source context absent")
        };
        let template = crate::effect_actor::Template::load(&resources, 916).unwrap();
        let mut configured = template
            .ground_positioned(context, pool.get(context).unwrap(), &mut random)
            .unwrap();
        let source = configured.ground.as_ref().unwrap();
        assert_eq!(configured.initial.fields.response_flags & 0x1000, 0x1000);
        assert!(
            source
                .decision(&configured.initial.fields, &configured.initial.seed)
                .unwrap()
                .emission
                .is_none()
        );
        // Explicit dependency input, not a fabricated successful ground query.
        configured.initial.seed.storage.data_mut().unwrap()[0xa0..0xa4]
            .copy_from_slice(&4u32.to_le_bytes());
        let request = source
            .decision(&configured.initial.fields, &configured.initial.seed)
            .unwrap()
            .emission
            .unwrap();
        assert_eq!(request.flags, 4);
        assert!(
            source
                .emitted(
                    &mut configured.initial.seed,
                    Emission {
                        effect: request.effect ^ 1,
                        ..request
                    }
                )
                .is_err()
        );
        assert_eq!(
            source
                .decision(&configured.initial.fields, &configured.initial.seed)
                .unwrap()
                .emission
                .unwrap(),
            request
        );
        source
            .emitted(&mut configured.initial.seed, request)
            .unwrap();
        assert!(
            source
                .emitted(&mut configured.initial.seed, request)
                .is_err()
        );
        let data = configured.initial.seed.storage.data().unwrap();
        assert_eq!(i32::from_le_bytes(data[0x20..0x24].try_into().unwrap()), 19);
    }
    #[test]
    #[ignore = "requires GIANTS_MISSION_OWNED_ROOT; constructor/callback refusal only, not live film acceptance"]
    fn installed_blood_body_uses_canonical_payload_and_requires_contact_services() {
        use crate::{
            attached_effect_context::{Creation, Pool},
            audio_parameters::Random,
            resident_states::Session,
        };
        let root = std::path::PathBuf::from(std::env::var_os("GIANTS_MISSION_OWNED_ROOT").unwrap());
        let resources =
            crate::scene_resources::SceneResources::load(&root, "w_Story2.bin", "English").unwrap();
        let image = &resources.classes.models.effects.last().unwrap().image;
        let offset = image.find("fxb_blood").unwrap().1.offset;
        let mut pool = Pool::<u32>::from_active_resources(79, &resources);
        let mut random = Random(19); // Supplied dependency fixture stream.
        let mut expected = random;
        for _ in 0..12 {
            expected.next_15();
        }
        let (Creation::Prepared(context), _) = pool
            .prepare_positional_offset(Some(offset), [1., 2., 3.], &mut random)
            .unwrap()
        else {
            panic!("source context absent")
        };
        let mut session = Session::new(79);
        let actor = pool
            .activate_ground_positioned(context, &resources, &mut random, &mut session)
            .unwrap();
        assert_eq!(random.0, expected.0);
        assert!(pool.active().is_empty());
        assert!(
            pool.activate_ground_positioned(context, &resources, &mut random, &mut session)
                .is_err()
        );
        let resident = session.get(actor).unwrap();
        assert_eq!(resident.class, 916);
        let fields = resident.construction_fields.as_ref().unwrap();
        assert_eq!(fields.position, [1., 2., 3.]);
        assert_eq!(fields.control_flags & 0x0200_0280, 0x0200_0280);
        let data = resident.seed.storage.data().unwrap();
        assert_eq!(i32::from_le_bytes(data[0x20..0x24].try_into().unwrap()), 20);
        struct Required(Vec<&'static str>);
        impl crate::resident_states::TimedEffectServices for Required {
            fn physical(
                &mut self,
                s: &mut Session,
                id: crate::resident_states::Id,
                dt: f32,
            ) -> Result<(), String> {
                self.0.push("motion");
                s.advance_timed_effect_motion(id, f64::from(dt))
            }
            fn gravity(
                &mut self,
                session: &mut Session,
                id: crate::resident_states::Id,
            ) -> Result<(), String> {
                self.0.push("gravity");
                session.apply_timed_effect_gravity(id)
            }
            fn retire(
                &mut self,
                _: &mut Session,
                _: crate::resident_states::Id,
                _: &giants_formats::effect_image::Image,
                _: Option<&crate::effect_scene::RetirementBinding>,
            ) -> Result<(), String> {
                panic!("not expired")
            }
        }
        let mut stamp = crate::animation_epoch::Stamp::default();
        let mut services = Required(Vec::new());
        session
            .advance_timed_effect(actor, 0., &mut stamp, &mut services)
            .unwrap();
        let error = session
            .advance_timed_effect(actor, 0.01, &mut stamp, &mut services)
            .unwrap_err();
        assert_eq!(services.0, ["motion", "gravity"]);
        assert!(error.contains("real contact and child services"), "{error}");
        assert!(session.timed_effect_failure(actor).is_some());
        assert!(
            session
                .advance_timed_effect(actor, 0.01, &mut stamp, &mut services)
                .is_err()
        );
        let resident = session.get(actor).unwrap();
        assert_eq!(
            i32::from_le_bytes(
                resident.seed.storage.data().unwrap()[0x20..0x24]
                    .try_into()
                    .unwrap()
            ),
            20
        );
    }
    fn input() -> Input {
        Input {
            control_flags: 0x0200_0200,
            response_flags: 0x1400,
            object_flags: 0x400,
            physical_flags: 4,
            remaining: 20,
            effect: 17,
            position: [1, 2, 3],
        }
    }
    #[test]
    fn real_contact_is_required_and_does_not_consume_budget_itself() {
        let source = input();
        assert_eq!(
            decide(Input {
                physical_flags: 0,
                ..source
            }),
            Ok(Decision::default())
        );
        let decision = decide(source).unwrap();
        assert_eq!(
            decision.emission.unwrap(),
            Emission {
                effect: 17,
                position: [1, 2, 3],
                flags: 7,
                remaining_after: 19,
            }
        );
        assert_eq!(source.remaining, 20);
        assert_eq!(
            decision.retirement,
            Some(Retirement {
                allow_source_child: false,
                publish_position: false
            })
        );
        assert!(
            decide(Input {
                remaining: 0,
                ..source
            })
            .unwrap()
            .emission
            .is_none()
        );
    }
    #[test]
    fn signed_budget_and_retirement_precedence_survive() {
        let source = input();
        assert_eq!(
            decide(Input {
                remaining: i32::MIN,
                ..source
            })
            .unwrap()
            .emission
            .unwrap()
            .remaining_after,
            i32::MAX
        );
        assert_eq!(
            decide(Input {
                control_flags: 0x600,
                ..source
            })
            .unwrap()
            .retirement,
            Some(Retirement {
                allow_source_child: false,
                publish_position: false
            })
        );
        assert_eq!(
            decide(Input {
                control_flags: 0x400,
                ..source
            })
            .unwrap()
            .retirement,
            Some(Retirement {
                allow_source_child: true,
                publish_position: true
            })
        );
    }
    #[test]
    fn unprovided_services_are_not_silently_consumed() {
        assert!(
            decide(Input {
                control_flags: 0x100,
                physical_flags: 12,
                ..input()
            })
            .is_err()
        );
        assert!(
            decide(Input {
                control_flags: 0x8200_0200,
                ..input()
            })
            .is_err()
        );
    }
}
