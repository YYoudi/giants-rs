//! Source-owned cinematic effect support. Literal prototype search route
//! `FLK_FxSpawn__FP9FLK_FlickiiiiPc` is not a confirmed PC attribution.
//! PC uses the loaded scalar class0 and forces unit header scale; the prototype
//! uses a different support class. This owner never registers an ordinary actor.
use crate::{
    actor_fields::Fields, procedural_aim::Matrix, resident_states::CinematicSupport,
    scene_resources::SceneResources,
};

pub struct Support {
    owner: CinematicSupport,
    fields: Fields,
    matrix: Matrix,
}

impl Support {
    /// The caller supplies its actual resolved source pose, not a manufactured
    /// position or identity matrix for a missing actor/reference.
    pub fn prepare(
        resources: &SceneResources,
        owner: CinematicSupport,
        matrix: Matrix,
    ) -> Result<Self, String> {
        if matrix.flags & 0x20 != 0 || !matrix.values.iter().all(|v| v.is_finite()) {
            return Err("cinematic support needs its actual finite affine".into());
        }
        let mut fields = Fields::load_class_zero(resources)?;
        fields.scale = 1.;
        fields.previous = fields.current;
        fields.position = [matrix.values[3], matrix.values[7], matrix.values[11]];
        fields.current = fields.position;
        Ok(Self {
            owner,
            fields,
            matrix,
        })
    }
    pub fn owner(&self) -> CinematicSupport {
        self.owner
    }
    pub fn position(&self) -> [f32; 3] {
        self.fields.position
    }
    pub fn object_flags(&self) -> u32 {
        self.fields.object_flags
    }
    pub fn scale(&self) -> f32 {
        self.fields.scale
    }
    pub fn matrix(&self) -> Matrix {
        self.matrix
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires GIANTS_MISSION_OWNED_ROOT; real resources/supplied source pose, not live FLK or GPU acceptance"]
    fn installed_source3_snapshots_real_class_zero_support_and_keeps_context_binding() {
        use crate::{
            attached_effect_context::{Binding, Creation, Pool},
            audio_parameters::Random,
            resident_states::{CinematicScope, Session},
        };
        let root = std::path::PathBuf::from(std::env::var_os("GIANTS_MISSION_OWNED_ROOT").unwrap());
        let resources = SceneResources::load(&root, "w_Story2.bin", "English").unwrap();
        let image = &resources.classes.models.effects.last().unwrap().image;
        let bytes = std::fs::read(crate::flick::sequence_path(&root, "story2_savewife_6").unwrap())
            .unwrap();
        let metadata = giants_formats::flick::discover_dependencies(&bytes).unwrap();
        let actors = crate::actor_links::Factory::new(&resources, image)
            .project(&bytes)
            .unwrap();
        let clocks = actors.actors.keys().map(|&slot| (slot, 0.)).collect();
        let locals = actors.actors.keys().map(|&slot| (slot, None)).collect();
        let requests = crate::cinematic_fx::declarations(&bytes, &metadata).unwrap();
        let mut contexts = Pool::from_active_resources(83, &resources);
        let mut random = Random(41); // Explicit dependency fixture stream.
        let world = crate::load_world(&root, "Story2").unwrap();
        let query = crate::terrain_queries::StaticTerrainQuery::new(
            &world.terrain,
            1. / world.terrain.spacing,
            crate::arithmetic::Precision::Double53,
        )
        .unwrap();
        for name in ["fxb_blood", "fxb_blooddrop"] {
            let request = requests
                .iter()
                .find(|r| r.effect.as_deref() == Some(name))
                .unwrap();
            let placement = request
                .placement(&actors, &clocks, &locals)
                .unwrap()
                .unwrap();
            let owner = CinematicSupport {
                scope: CinematicScope {
                    ticket: 83,
                    director: 4,
                    generation: 1,
                },
                slot: placement.slot,
            };
            let support = Support::prepare(&resources, owner, placement.matrix).unwrap();
            assert_eq!(
                support.position().map(f32::to_bits),
                placement.position.map(f32::to_bits)
            );
            assert_eq!(support.scale(), 1.);
            let (Creation::Prepared(context), _) = contexts
                .prepare_cinematic_source3(name, owner, support.object_flags(), &mut random)
                .unwrap()
            else {
                panic!("source context absent")
            };
            let actual = contexts.get(context).unwrap();
            assert!(matches!(actual.binding,Binding::Attached {recipient,..} if recipient==owner));
            assert_eq!(actual.flags(), 0x8004);
            let foreign = Support::prepare(
                &resources,
                CinematicSupport {
                    slot: owner.slot + 1,
                    ..owner
                },
                placement.matrix,
            )
            .unwrap();
            let before = random.0;
            let invalid = if name == "fxb_blood" {
                crate::timed_effect_object::Timed::ground_cinematic(
                    &resources,
                    context,
                    actual,
                    &foreign,
                    &mut random,
                )
            } else {
                crate::timed_effect_object::Timed::surface_cinematic(
                    &resources,
                    context,
                    actual,
                    &foreign,
                    &mut random,
                    |xy| {
                        crate::physical_ground::Ground::from_sample(
                            query.sample(xy)?,
                            world.terrain.width,
                        )
                    },
                )
            };
            assert!(invalid.is_err());
            assert_eq!(before, random.0);
            let body = if name == "fxb_blood" {
                crate::timed_effect_object::Timed::ground_cinematic(
                    &resources,
                    context,
                    actual,
                    &support,
                    &mut random,
                )
            } else {
                crate::timed_effect_object::Timed::surface_cinematic(
                    &resources,
                    context,
                    actual,
                    &support,
                    &mut random,
                    |xy| {
                        crate::physical_ground::Ground::from_sample(
                            query.sample(xy)?,
                            world.terrain.width,
                        )
                    },
                )
            }
            .unwrap();
            assert!(matches!(
                body.object().placement,
                crate::effect_actor::Placement::Position(_)
            ));
            assert!(matches!(
                contexts.get(context).unwrap().binding,
                Binding::Attached { .. }
            ));
            if name == "fxb_blood" {
                assert_eq!(
                    body.object().fields.position.map(f32::to_bits),
                    support.position().map(f32::to_bits)
                );
            }
            let mut session = Session::new(owner.scope.ticket);
            let id = session
                .adopt_cinematic_timed_positioned(body, owner.scope)
                .unwrap();
            contexts.release_prepared(context).unwrap();
            assert!(session.cinematic_timed_support(id).is_none());
            session.cancel_cinematic_scope(owner.scope).unwrap();
            assert!(session.get(id).is_none());
        }
        assert!(contexts.active().is_empty());
    }
}
