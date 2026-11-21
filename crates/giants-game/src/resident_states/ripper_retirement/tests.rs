//! Core retirement boundaries. Terminal state/witness and external release are
//! supplied fixtures; the ordinary lethal/corpse producers need rendered proof.
use super::*;
use crate::scene_resources::SceneResources;

fn terminal(
    resources: &SceneResources,
    class: i32,
) -> (Session, Id, crate::declared_residents::Key) {
    let mut session = Session::new(9847);
    let key = crate::declared_residents::Key {
        scope: crate::declared_residents::Scope::Mission,
        index: 7,
    };
    let id = session
        .construct_source_declaration(key, |s| {
            Ok(crate::ripper_health_drop::tests::source_at(
                resources,
                s,
                class,
                [4., 5., 5.],
            ))
        })
        .unwrap();
    let terrain = giants_formats::gti::Terrain {
        source_flags: 3,
        width: 4,
        height: 2,
        origin: [0.; 2],
        spacing: 10.,
        height_bounds: [0.; 2],
        fallback_texture: String::new(),
        heights: vec![0.; 8],
        colors: vec![0; 8],
        tags: vec![6; 8],
        consumed_bytes: 0,
        opaque_tail: vec![],
    };
    let ground = crate::terrain_queries::StaticTerrainQuery::new(
        &terrain,
        0.1,
        crate::arithmetic::Precision::Double53,
    )
    .unwrap();
    session.prepare_actor_affine(id).unwrap();
    session
        .prepare_actor_environment(id, resources, &ground)
        .unwrap();
    session.prepare_actor_finalization(id).unwrap();
    session.prepare_actor_motion(id).unwrap();
    session.begin_actor_frame_history(id).unwrap();
    session.begin_actor_motion_history(id, [0; 8]).unwrap();
    session.prepare_ripper_primary(id).unwrap();
    let mut state = session.state23_snapshot(id).unwrap();
    state.damage = state.limit;
    state.mode = 94;
    state.flags |= 0x8000;
    let actor = session.residents.get_mut(&id.serial).unwrap();
    actor.construction_fields.as_mut().unwrap().object_flags = state.flags;
    actor.state23_combat = Some(state);
    actor.ripper_retirement.marked = true;

    struct Inactive;
    impl crate::actor_world_tick::Services for Inactive {
        fn flags(&self) -> Result<u32, String> {
            Ok(0x8000)
        }
        fn set_flags(&mut self, _: u32) -> Result<(), String> {
            Err("unexpected active write".into())
        }
        fn primary(&mut self) -> Result<(), String> {
            Err("unexpected primary".into())
        }
        fn controlled_actor_present(&self) -> bool {
            true
        }
        fn secondary(&mut self) -> Result<(), String> {
            Err("unexpected secondary".into())
        }
        fn object_time(&mut self) -> Result<f32, String> {
            Err("unexpected clock".into())
        }
        fn animation(&mut self, _: f32) -> Result<(), String> {
            Err("unexpected animation".into())
        }
        fn finalization(&mut self) -> Result<(), String> {
            Err("unexpected finalization".into())
        }
    }
    let mut tick = crate::actor_world_tick::Tick::new();
    assert_eq!(
        tick.run(&mut Inactive).unwrap(),
        crate::actor_world_tick::Outcome::Inactive
    );
    session.residents.get_mut(&id.serial).unwrap().world_tick = Some(tick);
    (session, id, key)
}

struct Release {
    calls: usize,
    fail: bool,
}
impl crate::actor_retirement::Services for Release {
    fn release(&mut self, session: &mut Session, id: Id) -> Result<(), String> {
        assert!(session.get(id).is_some() && session.specification(id).is_some());
        assert_eq!(session.links.memberships(id.serial).len(), 3);
        self.calls += 1;
        if self.fail {
            Err("supplied late release failure".into())
        } else {
            Ok(())
        }
    }
}

#[test]
#[ignore = "Requires GIANTS_NPC_ROOT; installed class73/230 core ownership with supplied terminal witness/external release; native unreferenced core observations, no death/corpse/runtime/GPU proof"]
fn source_ripper_corpse_retirement_keeps_identity_and_nonreplayable_release() {
    let root = std::path::PathBuf::from(std::env::var_os("GIANTS_NPC_ROOT").unwrap());
    let resources = SceneResources::load(&root, "w_Story2.bin", "English").unwrap();
    for class in [73, 230] {
        for fail in [false, true] {
            let (mut session, id, key) = terminal(&resources, class);
            let other = crate::ripper_health_drop::tests::source_at(
                &resources,
                &mut session,
                73,
                [8., 5., 5.],
            );
            let before = session.counts();
            let other_links = session.links.memberships(other.serial);
            let expected = Counts {
                residents: 1,
                payload_bytes: 88,
                registrations: 3,
            };
            session
                .residents
                .get_mut(&id.serial)
                .unwrap()
                .ripper_retirement
                .marked = false;
            assert!(session.check_dead_ripper_retirement(id, key).is_err());
            session
                .residents
                .get_mut(&id.serial)
                .unwrap()
                .ripper_retirement
                .marked = true;
            session
                .residents
                .get_mut(&other.serial)
                .unwrap()
                .cold_alert_target = Some(id);
            assert!(
                session
                    .check_dead_ripper_retirement(id, key)
                    .unwrap_err()
                    .contains("incoming")
            );
            session
                .residents
                .get_mut(&other.serial)
                .unwrap()
                .cold_alert_target = None;
            assert_eq!(
                session.check_dead_ripper_retirement(id, key).unwrap(),
                expected
            );
            let mut release = Release { calls: 0, fail };
            let result = session.cleanup_dead_ripper(id, key, &mut release);
            assert_eq!(release.calls, 1);
            assert!(session.get(other).is_some());
            assert_eq!(session.links.memberships(other.serial), other_links);
            if fail {
                assert!(result.is_err());
                assert!(session.get(id).is_some());
                assert_eq!(session.counts(), before);
                assert_eq!(session.declaration(key), Some(id));
                assert!(session.retired_declaration(key).is_none());
            } else {
                assert_eq!(result.unwrap(), expected);
                assert!(session.get(id).is_none());
                assert!(session.specification(id).is_none());
                assert!(session.state_resources(id).is_none());
                assert!(session.links.memberships(id.serial).is_empty());
                assert_eq!(session.counts().residents, before.residents - 1);
                assert!(session.declaration(key).is_none());
                assert_eq!(session.retired_declaration(key), Some(id));
                let mut allocated = false;
                assert!(
                    session
                        .construct_source_declaration(key, |_| {
                            allocated = true;
                            Ok(other)
                        })
                        .is_err()
                );
                assert!(!allocated);
            }
            assert!(session.cleanup_dead_ripper(id, key, &mut release).is_err());
            assert_eq!(release.calls, 1);
            session.clear();
            assert!(session.retired_declaration(key).is_none());
        }
        struct Replace<'a>(&'a SceneResources, i32);
        impl crate::actor_retirement::Services for Replace<'_> {
            fn release(&mut self, session: &mut Session, id: Id) -> Result<(), String> {
                let (replacement, new_id, _) = terminal(self.0, self.1);
                assert_eq!(id, new_id, "same numeric identity, different owned attempt");
                *session = replacement;
                Ok(())
            }
        }
        let (mut session, id, key) = terminal(&resources, class);
        assert!(
            session
                .cleanup_dead_ripper(id, key, &mut Replace(&resources, class))
                .is_err()
        );
        assert!(session.get(id).is_some());
        assert!(
            session.check_dead_ripper_retirement(id, key).is_ok(),
            "replacement must not be deleted or poisoned by the old release attempt"
        );
    }
}
