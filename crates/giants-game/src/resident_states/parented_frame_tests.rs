//! Source-backed special children; world admission and the parent turn are separate.
use super::*;
use crate::{actor_fields::Fields, arithmetic::Precision, scene_resources::SceneResources};

struct Unused;
impl crate::herd_generator::Ground for Unused {
    fn height(&mut self, _: [f32; 3]) -> Result<f32, String> {
        Err("parented state0 must not generate a herd".into())
    }
}
impl crate::actor_finalization::Services for Unused {
    fn release_footprint(&mut self, _: Option<u32>) -> Result<(), String> {
        Err("stationary parented child must not release a footprint".into())
    }
    fn environment(&mut self, _: &mut crate::actor_finalization::State) -> Result<(), String> {
        Err("parented finalization must not enter the environment".into())
    }
}

#[test]
#[ignore = "Requires GIANTS_NPC_ROOT; acquired special constructor, supplied admission/control/time, no parent turn or ordinary gameplay"]
fn source_parented_state0_callbacks_keep_binding_without_inventing_world_affine() {
    let root = std::path::PathBuf::from(std::env::var_os("GIANTS_NPC_ROOT").unwrap());
    let resources = SceneResources::load(&root, "w_Story2.bin", "English").unwrap();
    let program = resources
        .classes
        .models
        .first_program(2)
        .unwrap()
        .program
        .as_ref()
        .unwrap();
    let seed = Seed::from_program_for(program, Profile::Debug, Precision::Double53).unwrap();
    let bindings = resources
        .class_state_bindings_for(2, Profile::Debug)
        .unwrap();
    let flags = DirectFlags::project(2, program);
    let spec = Arc::new(crate::specification::Prepared::load_registered(&resources, 2).unwrap());
    let states =
        Arc::new(crate::state_resources::Prepared::load(&resources, 2, Profile::Debug).unwrap());
    let template = Fields::load(&resources, 2, &spec).unwrap();
    let factory = crate::vimp_actor::Factory::load_registered(&resources).unwrap();
    let globals =
        giants_formats::global_data::parse(&std::fs::read(root.join("Bin/GData.gbn")).unwrap())
            .unwrap();
    let mut retired = Vec::new();
    for ticket in [683, 684] {
        let mut session = Session::new(ticket);
        let owner = session
            .construct(Construction {
                class: 2,
                seed: &seed,
                bindings: &bindings,
                flags,
                specification: spec.clone(),
                state_resources: states.clone(),
                fields: template
                    .placed(crate::collider_init::DeclaredPose {
                        class: 2,
                        position: [100., 200., 300.],
                        yaw: 0.,
                        pitch_roll: Some([0., 0.]),
                    })
                    .unwrap(),
            })
            .unwrap();
        let group = crate::mecc_special::Factory::new(&resources)
            .initialize(&mut session, owner, Some(owner))
            .unwrap();
        let parent = session
            .get(owner)
            .unwrap()
            .construction_fields
            .clone()
            .unwrap();
        let mut random = crate::audio_parameters::Random(314);
        let mut stamp = crate::animation_epoch::Stamp(0);
        for id in &retired {
            assert!(session.passive_world_frame(*id).is_err());
        }
        for (number, seconds) in [0., 1. / 60., 0.4, 0.].into_iter().enumerate() {
            for id in group.children {
                session
                    .prepare_passive_world_history(id, number as u64)
                    .unwrap();
            }
            for id in group.children {
                let before = session.get(id).unwrap().channels.unwrap();
                let epoch = session.get(id).unwrap().mecc_animation_stamp;
                let binding = session.mecc_special_child(id).unwrap();
                let projection = session
                    .advance_passive_world_after_history(
                        id,
                        crate::passive_frame::Frame {
                            number: number as u64,
                            global_seconds: seconds,
                            areas: &[],
                            controlled: Some(owner),
                        },
                        &mut crate::passive_frame::Services {
                            factory: &factory,
                            globals: &globals,
                            random: &mut random,
                            ground: &mut Unused,
                            stamp: &mut stamp,
                            finalization: &mut Unused,
                        },
                    )
                    .unwrap();
                assert_eq!(
                    projection.outcome,
                    crate::actor_world_tick::Outcome::Updated
                );
                assert!(projection.born.is_empty());
                assert_eq!(session.mecc_special_child(id), Some(binding));
                let actor = session.get(id).unwrap();
                let view = session.passive_world_frame(id).unwrap();
                assert_eq!(view.next_frame, number as u64 + 1);
                assert!(view.finalization.parent_present);
                assert!(view.finalization.affine.is_none());
                assert!(actor.affine_cache.is_none());
                assert_ne!(
                    actor.construction_fields.as_ref().unwrap().object_flags & 0x20,
                    0
                );
                assert!(session.actor_current_affine(id).is_err());
                assert_eq!(
                    session.current_reference_matrices(id).unwrap().len(),
                    usize::from(actor.class == 85)
                );
                if number == 3 {
                    assert_eq!(actor.channels.unwrap(), before);
                    assert_eq!(actor.mecc_animation_stamp, epoch);
                }
                let epoch = actor.mecc_animation_stamp;
                session
                    .prepare_actor_positions(id, 0, epoch, false)
                    .unwrap();
                assert!(
                    !session
                        .actor_model_positions(id, 0)
                        .unwrap()
                        .unwrap()
                        .is_empty()
                );
                assert!(
                    session
                        .prepare_passive_world_history(id, number as u64)
                        .is_err()
                );
            }
        }
        let actual = session
            .get(owner)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap();
        assert_eq!(actual.channels, parent.channels);
        assert_eq!(actual.position, parent.position);
        assert_eq!(random.0, 314);
        // A supplied presentation change must not detach the typed constructor
        // binding and silently admit the child as an ordinary unparented actor.
        let child = group.children[0];
        session.set_presentation_parent(child, None).unwrap();
        assert!(session.passive_world_frame(child).is_err());
        session.set_presentation_parent(child, Some(owner)).unwrap();
        assert!(session.passive_world_frame(child).is_ok());
        assert!(session.prepare_actor_affine(child).is_err());
        session.clear();
        for id in group.children {
            assert!(session.passive_world_frame(id).is_err());
            retired.push(id);
        }
    }
}
