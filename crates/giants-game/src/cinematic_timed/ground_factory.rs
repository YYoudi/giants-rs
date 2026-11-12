//! Real contact child construction from the parent's exact source image, and
//! the source16 non-respawn retirement subset. This does not bind a whole film.
use crate::{
    attached_effect_context::{Creation, Pool},
    audio_parameters::Random,
    effect_ground_emission::{Emission, Retirement, Source},
    resident_states::{CinematicScope, CinematicSupport, Id, Session},
    scene_resources::SceneResources,
};

pub struct Factory<'a> {
    pub scope: CinematicScope,
    pub contexts: &'a mut Pool<CinematicSupport>,
    pub resources: &'a SceneResources,
    pub random: &'a mut Random,
    pub newborns: &'a mut Vec<Id>,
}

impl super::world::GroundEffects for Factory<'_> {
    fn emit(
        &mut self,
        session: &mut Session,
        parent: Id,
        image: &giants_formats::effect_image::Image,
        source: &Source,
        request: Emission,
        terrain: &giants_formats::gti::Terrain,
    ) -> Result<Id, String> {
        if session.cinematic_timed_scope(parent) != Some(self.scope)
            || !self
                .resources
                .classes
                .models
                .effects
                .last()
                .is_some_and(|e| e.image.same_image(image))
        {
            return Err("ground factory source/scope differs from its actual parent".into());
        }
        let child = source
            .child()
            .ok_or("ground factory source binding absent")?;
        let (index, entry) = image
            .find(&child.effect)
            .ok_or("ground factory source child absent")?;
        if index as u32 != request.effect || entry.offset != child.program_offset {
            return Err("ground factory packet differs from source child".into());
        }
        let (Creation::Prepared(context), _) =
            self.contexts
                .prepare_ground_offset_invariant(entry.offset, request, self.random)?
        else {
            return Err("ground factory did not prepare its actual source context".into());
        };
        let result = (|| {
            let query = crate::terrain_queries::StaticTerrainQuery::new(
                terrain,
                1. / terrain.spacing,
                crate::arithmetic::Precision::Double53,
            )?;
            let body = crate::timed_effect_object::Timed::surface_positioned(
                self.resources,
                context,
                self.contexts
                    .get(context)
                    .ok_or("ground child context lost")?,
                self.random,
                |xy| crate::physical_ground::Ground::from_sample(query.sample(xy)?, terrain.width),
            )?;
            session.adopt_cinematic_timed_positioned(body, self.scope)
        })();
        self.contexts.release_prepared(context)?;
        let child = result?;
        self.newborns.push(child);
        Ok(child)
    }
    fn retire(
        &mut self,
        session: &mut Session,
        parent: Id,
        request: Retirement,
    ) -> Result<(), String> {
        if session.cinematic_timed_scope(parent) != Some(self.scope)
            || request.allow_source_child
            || request.publish_position
            || session
                .timed_effect_ordinary_header(parent)
                .is_none_or(|h| h.footprint.is_some())
        {
            return Err("ground retirement requires another source/footprint service".into());
        }
        let (seed, fields) = session.timed_effect_contact_fields(parent)?;
        let data = seed
            .storage
            .data()
            .ok_or("ground retirement payload absent")?;
        if fields.response_flags & 2 != 0 || data[0x108..0x10c].iter().any(|&v| v != 0) {
            return Err("ground retirement requires respawn/association services".into());
        }
        // Native selected retirement reaches the ordinary deferred-delete mark;
        // the composing scope keeps destruction/resource retirement separate.
        fields.object_flags |= 0x8000;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires GIANTS_MISSION_OWNED_ROOT; constructor inheritance/query-failure controls, no live FLK/GPU claim"]
    fn installed_surface_inheritance_and_late_query_rng_are_preserved() {
        let root = std::path::PathBuf::from(std::env::var_os("GIANTS_MISSION_OWNED_ROOT").unwrap());
        let resources = SceneResources::load(&root, "w_Story2.bin", "English").unwrap();
        let world = crate::load_world(&root, "Story2").unwrap();
        let query = crate::terrain_queries::StaticTerrainQuery::new(
            &world.terrain,
            1. / world.terrain.spacing,
            crate::arithmetic::Precision::Double53,
        )
        .unwrap();
        let image = &resources.classes.models.effects.last().unwrap().image;
        let (index, entry) = image.find("fxb_blooddrop").unwrap();
        let mut contexts = Pool::<CinematicSupport>::from_active_resources(82, &resources);
        for flags in 0..8 {
            let mut random = Random(31);
            let mut expected = random;
            for _ in 0..4 {
                expected.next_15();
            }
            let request = Emission {
                effect: index as u32,
                position: [1f32, 2., 3.].map(f32::to_bits),
                flags,
                remaining_after: 19,
            };
            let (Creation::Prepared(context), _) = contexts
                .prepare_ground_offset_invariant(entry.offset, request, &mut random)
                .unwrap()
            else {
                panic!("source context absent")
            };
            let body = crate::timed_effect_object::Timed::surface_positioned(
                &resources,
                context,
                contexts.get(context).unwrap(),
                &mut random,
                |xy| {
                    crate::physical_ground::Ground::from_sample(
                        query.sample(xy)?,
                        world.terrain.width,
                    )
                },
            )
            .unwrap();
            let f = &body.object().fields;
            assert_eq!(
                f.response_flags & 0x1000,
                if flags & 4 != 0 { 0x1000 } else { 0 }
            );
            assert_eq!(
                f.response_flags & 0x400,
                if flags & 2 != 0 { 0x400 } else { 0 }
            );
            assert_eq!(
                f.object_flags & 0x400,
                if flags & 1 != 0 { 0x400 } else { 0 }
            );
            assert_eq!(f.control_flags & 0x80, 0);
            assert_eq!(f.current.map(f32::to_bits), f.position.map(f32::to_bits));
            assert_eq!(random.0, expected.0);
            contexts.release_prepared(context).unwrap();
        }
        let mut random = Random(37);
        let mut expected = random;
        for _ in 0..4 {
            expected.next_15();
        }
        let request = Emission {
            effect: index as u32,
            position: [1f32, 2., 3.].map(f32::to_bits),
            flags: 4,
            remaining_after: 19,
        };
        let (Creation::Prepared(context), _) = contexts
            .prepare_ground_offset_invariant(entry.offset, request, &mut random)
            .unwrap()
        else {
            panic!("source context absent")
        };
        let mut calls = 0;
        let result = crate::timed_effect_object::Timed::surface_positioned(
            &resources,
            context,
            contexts.get(context).unwrap(),
            &mut random,
            |_| {
                calls += 1;
                Err("supplied query failure")
            },
        );
        assert!(result.is_err());
        assert_eq!(calls, 1);
        assert_eq!(
            random.0, expected.0,
            "late query failure rewound real active draws"
        );
        contexts.release_prepared(context).unwrap();
        assert!(contexts.active().is_empty());
    }
}
