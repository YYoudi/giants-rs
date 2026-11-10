//! Complete acquired detached child program for Mecc damage smoke.
//! Preparation, active construction and later world admission stay distinct.
use giants_formats::effect_program::Program;

pub(crate) struct Recipe {
    pub class: i32,
    life: [f32; 3],
    sizes: [[f32; 2]; 2],
    gravity: [f32; 2],
    side: [f32; 2],
}
impl Recipe {
    pub fn project(program: &Program) -> Result<Self, String> {
        if program.records.iter().map(|r| r.opcode).collect::<Vec<_>>()
            != [3, 6, 4, 0x37, 0x0f, 0x80, 7, 2]
        {
            return Err("smoke child outside acquired complete program".into());
        }
        let word = |index: usize, at: usize| -> Result<u32, String> {
            Ok(u32::from_le_bytes(
                program.records[index]
                    .bytes
                    .get(at..at + 4)
                    .ok_or("smoke child operand truncated")?
                    .try_into()
                    .unwrap(),
            ))
        };
        let class = word(0, 1)? as i32;
        let life = [word(2, 1)?, word(2, 5)?, word(2, 9)?].map(f32::from_bits);
        let sizes = [[word(3, 1)?, word(3, 5)?], [word(3, 9)?, word(3, 13)?]]
            .map(|r| r.map(f32::from_bits));
        let gravity = [word(4, 1)?, word(4, 5)?].map(f32::from_bits);
        let side = [word(5, 1)?, word(5, 5)?].map(f32::from_bits);
        crate::effect_actor::lifetime_fade(life)?;
        if class < 0
            || sizes
                .into_iter()
                .flatten()
                .any(|s| !s.is_finite() || s <= 0.)
            || gravity.into_iter().chain(side).any(|v| !v.is_finite())
        {
            return Err("smoke child scalar outside acquired finite profile".into());
        }
        Ok(Self {
            class,
            life,
            sizes,
            gravity,
            side,
        })
    }
    /// Active draws only; the originating pool already executed its null pass.
    pub fn activate(
        &self,
        random: &mut crate::audio_parameters::Random,
    ) -> (crate::effect_visual::Configuration, f32, f32) {
        let mut sample = |range| crate::timed_effect::sample(&mut random.0, range);
        let sizes = self.sizes.map(&mut sample);
        let gravity = sample(self.gravity);
        let side = sample(self.side);
        (
            crate::effect_visual::Configuration {
                lifetime: self.life[0],
                fade_in: self.life[1],
                fade_out_start: self.life[2],
                start_size: sizes[0],
                end_size: sizes[1],
            },
            gravity,
            side,
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        attached_effect_context::{Creation, Pool},
        audio_parameters::Random,
        resident_states::{self, Session},
        scene_resources::SceneResources,
    };
    struct MissingPhysical {
        calls: usize,
    }
    impl crate::resident_states::TimedEffectServices for MissingPhysical {
        fn physical(
            &mut self,
            session: &mut Session,
            actor: resident_states::Id,
            _: f32,
        ) -> Result<(), String> {
            self.calls += 1;
            let (seed, fields) = session.timed_effect_physical_fields(actor)?;
            assert_ne!(fields.control_flags & 0x40, 0);
            assert_ne!(fields.control_flags & 0x100000, 0);
            assert_eq!(fields.control_flags & 8, 0);
            assert_eq!(
                f32::from_le_bytes(seed.storage.data().unwrap()[8..12].try_into().unwrap()),
                0.125
            );
            Err("declared missing velocity/physical owner".into())
        }
        fn retire(
            &mut self,
            _: &mut Session,
            _: resident_states::Id,
            _: &giants_formats::effect_image::Image,
            _: Option<&crate::effect_scene::RetirementBinding>,
        ) -> Result<(), String> {
            Err("retirement outside this prefix control".into())
        }
    }
    #[test]
    #[ignore = "Requires GIANTS_MECC_WORLD_ROOT; actual smoke child ownership and retained pre-velocity failure, no physics or renderer"]
    fn source_mecc_smoke_particle_owns_active_draws_and_retains_required_physical_failure() {
        let root = std::path::PathBuf::from(std::env::var_os("GIANTS_MECC_WORLD_ROOT").unwrap());
        let resources = SceneResources::load(&root, "w_Story2.bin", "English").unwrap();
        for name in [
            "fx_mecc_smoke_lite",
            "fx_mecc_smoke_medium",
            "fx_mecc_smoke_heavy",
        ] {
            let offset = resources
                .classes
                .models
                .effects
                .iter()
                .find_map(|e| e.image.find(name).map(|(_, entry)| entry.offset))
                .unwrap();
            let mut pool = Pool::<resident_states::Id>::from_active_resources(921, &resources);
            let mut session = Session::new(922);
            let mut random = Random(42);
            let (creation, _) = pool
                .prepare_retirement_offset_invariant(
                    Some(offset),
                    [17., -24., 30.],
                    0,
                    0,
                    &mut random,
                )
                .unwrap();
            let Creation::Prepared(context) = creation else {
                panic!("missing source child")
            };
            let saved = random;
            let mut altered = pool.get(context).unwrap().clone();
            let mut program = altered.program.as_ref().unwrap().as_ref().clone();
            program.records[3].opcode = 9;
            altered.program = Ok(std::sync::Arc::new(program));
            assert!(
                crate::timed_effect_object::Timed::smoke_positioned(
                    &resources,
                    context,
                    &altered,
                    &mut random
                )
                .is_err()
            );
            assert_eq!(saved.0, random.0);
            assert!(pool.get(context).is_some());
            let actor = pool
                .activate_smoke_particle(context, &mut session, &resources, &mut random)
                .unwrap();
            let mut expected = saved;
            for _ in 0..4 {
                expected.next_15();
            }
            assert_eq!(random.0, expected.0);
            assert!(pool.get(context).is_none());
            assert_eq!(session.timed_effect_origin(actor), Some(context));
            assert!(
                pool.activate_smoke_particle(context, &mut session, &resources, &mut random)
                    .is_err()
            );
            assert_eq!(random.0, expected.0);
            assert_eq!(
                session
                    .get(actor)
                    .unwrap()
                    .construction_fields
                    .as_ref()
                    .unwrap()
                    .position,
                [17., -24., 30.]
            );
            let mut stamp = crate::animation_epoch::Stamp(0);
            let mut missing = MissingPhysical { calls: 0 };
            assert_eq!(
                session
                    .advance_timed_effect(actor, 0.01, &mut stamp, &mut missing)
                    .unwrap(),
                crate::timed_effect_object::Step::FirstFrame
            );
            assert_eq!(missing.calls, 0);
            assert!(
                session
                    .advance_timed_effect(actor, 0.125, &mut stamp, &mut missing)
                    .is_err()
            );
            assert_eq!(missing.calls, 1);
            assert!(session.timed_effect_failure(actor).is_some());
            let size = session
                .get(actor)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap()
                .scale;
            assert!(
                session
                    .advance_timed_effect(actor, 0.125, &mut stamp, &mut missing)
                    .is_err()
            );
            assert_eq!(missing.calls, 1);
            assert_eq!(
                session
                    .get(actor)
                    .unwrap()
                    .construction_fields
                    .as_ref()
                    .unwrap()
                    .scale,
                size
            );
        }
    }
}
