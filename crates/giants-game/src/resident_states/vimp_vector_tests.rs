use super::vimp_vector::{Request, Services};
use super::*;
use crate::{
    vimp_response::Predicate,
    vimp_vector_response as vector,
    vimp_wander::{put, scalar, word},
};
struct Provider {
    actions: Vec<vector::Action>,
    mode: i32,
    fail: bool,
    remove: bool,
    parent_remove: bool,
    clear: bool,
    saw_route: bool,
}
impl Services for Provider {
    fn session(
        &mut self,
        s: &Session,
        id: Id,
        q: Predicate,
    ) -> Result<crate::actor_session_predicates::Snapshot<'_>, String> {
        if matches!(q, Predicate::Second) {
            let a = s.get(id).unwrap();
            let b = a.seed.storage.data().unwrap();
            let parent = a.vimp.unwrap().born.unwrap().parent;
            let p = s.get(parent).unwrap();
            if self.actions.is_empty() {
                let selected = word(p.seed.storage.data().unwrap(), 0x2c) as usize;
                let marker = s.herd_markers(parent).unwrap()[selected].unwrap();
                assert_eq!(
                    p.construction_fields.as_ref().unwrap().position,
                    s.get(marker)
                        .unwrap()
                        .construction_fields
                        .as_ref()
                        .unwrap()
                        .position
                );
                assert!(scalar(b, 0x40).is_finite());
                self.saw_route = true;
            } else {
                assert_eq!(scalar(b, 0x3c), 123.);
            }
        }
        Ok(crate::actor_session_predicates::Snapshot {
            mode: self.mode,
            declared_count: 0,
            current_key: 17,
            slots: &[],
        })
    }
    fn action(
        &mut self,
        s: &mut Session,
        id: Id,
        parent: Id,
        action: vector::Action,
    ) -> Result<(), String> {
        self.actions.push(action);
        if let vector::Action::Heading(value) = action {
            assert_eq!(value, 42.);
        }
        let a = s.residents.get_mut(&id.serial).unwrap();
        let b = a.seed.storage.data_mut().unwrap();
        put(b, 0x3c, 123f32.to_bits());
        put(b, 0x74, 777);
        put(b, 0xb4, 42f32.to_bits());
        s.residents
            .get_mut(&parent.serial)
            .unwrap()
            .construction_fields
            .as_mut()
            .unwrap()
            .position[2] = 88.;
        if self.remove {
            s.residents.remove(&id.serial);
        }
        if self.parent_remove {
            s.residents.remove(&parent.serial);
        }
        if self.clear {
            s.clear();
        }
        if self.fail {
            return Err("injected action failure after owned writes".into());
        }
        Ok(())
    }
}
fn provider(mode: i32) -> Provider {
    Provider {
        actions: Vec::new(),
        mode,
        fail: false,
        remove: false,
        parent_remove: false,
        clear: false,
        saw_route: false,
    }
}
fn prepared() -> (
    Session,
    Id,
    Id,
    giants_formats::global_data::GlobalData,
    crate::audio_parameters::Random,
) {
    let (mut s, _, g, _, children, rng) = super::vimp_history_tests::source_births(true);
    let id = children[0];
    let parent = s.vimp(id).unwrap().born.unwrap().parent;
    let marker = s.herd_markers(parent).unwrap()[0].unwrap();
    let position = s
        .get(marker)
        .unwrap()
        .construction_fields
        .as_ref()
        .unwrap()
        .position;
    let a = s.residents.get_mut(&id.serial).unwrap();
    a.construction_fields.as_mut().unwrap().position = position;
    put(a.seed.storage.data_mut().unwrap(), 0x98, 0);
    put(
        s.residents
            .get_mut(&parent.serial)
            .unwrap()
            .seed
            .storage
            .data_mut()
            .unwrap(),
        0x2c,
        0,
    );
    s.prepare_actor_affine(id).unwrap();
    (s, id, parent, g, rng)
}
#[test]
#[ignore = "requires GIANTS_NPC_ROOT; isolated owned vector, supplied session/affine phase, no live world"]
fn source_vimp_vector_publishes_route_before_query_and_rereads_callback_changes() {
    let (mut s, id, parent, g, mut rng) = prepared();
    let mut p = provider(4);
    let frames = s.get(id).unwrap().vimp_frames;
    let channels = s.get(id).unwrap().channels;
    s.advance_vimp_vector_offline(
        id,
        Request {
            number: 0,
            origin: [1., 2., 3.],
            fallback: -1.,
        },
        &g,
        &mut rng,
        &mut p,
    )
    .unwrap();
    assert!(p.saw_route);
    assert_eq!(p.actions.len(), 2);
    assert!(matches!(p.actions[0], vector::Action::Retarget { .. }));
    assert_eq!(p.actions[1], vector::Action::Heading(42.));
    let a = s.get(id).unwrap();
    let b = a.seed.storage.data().unwrap();
    assert_eq!(scalar(b, 0x3c), 123.);
    assert_eq!(word(b, 0x74), 2);
    assert_eq!(a.vimp.unwrap().mode, 1);
    assert_eq!(scalar(b, 0xb4), 42.);
    assert_eq!(
        s.get(parent)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap()
            .position[2],
        88.
    );
    assert_eq!(a.vimp_frames, frames);
    assert_eq!(a.channels, channels);
    assert!(!a.vimp.unwrap().wander_pending);
    assert_eq!(s.vimp_vector_steps(id), Some(1));
    assert!(
        s.advance_vimp_vector_offline(
            id,
            Request {
                number: 0,
                origin: [0.; 3],
                fallback: -1.
            },
            &g,
            &mut rng,
            &mut p
        )
        .is_err()
    );
}
#[test]
#[ignore = "requires GIANTS_NPC_ROOT; retained late callback writes and frame quarantine"]
fn source_vimp_vector_late_failure_keeps_actor_parent_and_rng_and_refuses_replay() {
    let (mut s, id, parent, g, mut rng) = prepared();
    let mut p = provider(4);
    p.fail = true;
    let before = rng.0;
    assert!(
        s.advance_vimp_vector_offline(
            id,
            Request {
                number: 0,
                origin: [1., 2., 3.],
                fallback: -1.
            },
            &g,
            &mut rng,
            &mut p
        )
        .unwrap_err()
        .contains("injected action")
    );
    let b = s.get(id).unwrap().seed.storage.data().unwrap();
    assert_eq!(word(b, 0x74), 777);
    assert_eq!(scalar(b, 0x3c), 123.);
    assert_eq!(scalar(b, 0xb4), 42.);
    assert_eq!(
        s.get(parent)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap()
            .position[2],
        88.
    );
    assert_ne!(rng.0, before);
    assert_eq!(s.vimp_vector_steps(id), Some(1));
    let after = rng.0;
    assert!(
        s.advance_vimp_vector_offline(
            id,
            Request {
                number: 1,
                origin: [0.; 3],
                fallback: -1.
            },
            &g,
            &mut rng,
            &mut p
        )
        .is_err()
    );
    assert_eq!(rng.0, after);
    assert_eq!(p.actions.len(), 1);
}
#[test]
#[ignore = "requires GIANTS_NPC_ROOT; callback retirement must not restore stale actor/route"]
fn source_vimp_vector_callback_retirement_keeps_survivor_writes() {
    for parent_remove in [false, true] {
        let (mut s, id, parent, g, mut rng) = prepared();
        let mut p = provider(4);
        p.remove = !parent_remove;
        p.parent_remove = parent_remove;
        assert!(
            s.advance_vimp_vector_offline(
                id,
                Request {
                    number: 0,
                    origin: [1., 2., 3.],
                    fallback: -1.
                },
                &g,
                &mut rng,
                &mut p
            )
            .is_err()
        );
        if parent_remove {
            assert!(s.get(parent).is_none());
            let a = s.get(id).unwrap();
            assert!(a.vimp_frame_stopped);
            assert_eq!(word(a.seed.storage.data().unwrap(), 0x74), 777);
        } else {
            assert!(s.get(id).is_none());
            assert_eq!(
                s.get(parent)
                    .unwrap()
                    .construction_fields
                    .as_ref()
                    .unwrap()
                    .position[2],
                88.
            );
        }
        assert_eq!(p.actions.len(), 1);
    }
}
#[test]
#[ignore = "requires GIANTS_NPC_ROOT; actual Session cancellation from required action callback"]
fn source_vimp_vector_session_clear_does_not_restore_actors_or_resources() {
    let (mut s, id, parent, g, mut rng) = prepared();
    let mut p = provider(4);
    p.clear = true;
    assert!(
        s.advance_vimp_vector_offline(
            id,
            Request {
                number: 0,
                origin: [1., 2., 3.],
                fallback: -1.
            },
            &g,
            &mut rng,
            &mut p
        )
        .is_err()
    );
    assert!(s.get(id).is_none());
    assert!(s.get(parent).is_none());
    assert_eq!(s.counts().residents, 0);
    assert_eq!(p.actions.len(), 1);
    assert!(s.list(0).is_empty());
    assert!(s.specification(id).is_none());
}

#[test]
#[ignore = "requires GIANTS_NPC_ROOT; foreign, stale affine, nonzero hint and reentry preflight"]
fn source_vimp_vector_preflight_does_not_consume_random_or_replace_missing_owners() {
    let (mut s, id, _, g, mut rng) = prepared();
    let mut p = provider(0);
    let before = rng.0;
    let foreign = Id {
        ticket: id.ticket + 1,
        serial: id.serial,
    };
    assert!(
        s.advance_vimp_vector_offline(
            foreign,
            Request {
                number: 0,
                origin: [0.; 3],
                fallback: -1.
            },
            &g,
            &mut rng,
            &mut p
        )
        .is_err()
    );
    s.residents.get_mut(&id.serial).unwrap().vimp_vector_running = true;
    assert!(
        s.advance_vimp_vector_offline(
            id,
            Request {
                number: 0,
                origin: [0.; 3],
                fallback: -1.
            },
            &g,
            &mut rng,
            &mut p
        )
        .is_err()
    );
    s.residents.get_mut(&id.serial).unwrap().vimp_vector_running = false;
    put(
        s.residents
            .get_mut(&id.serial)
            .unwrap()
            .seed
            .storage
            .data_mut()
            .unwrap(),
        0xa8,
        1,
    );
    assert!(
        s.advance_vimp_vector_offline(
            id,
            Request {
                number: 0,
                origin: [0.; 3],
                fallback: -1.
            },
            &g,
            &mut rng,
            &mut p
        )
        .is_err()
    );
    put(
        s.residents
            .get_mut(&id.serial)
            .unwrap()
            .seed
            .storage
            .data_mut()
            .unwrap(),
        0xa8,
        0,
    );
    s.residents
        .get_mut(&id.serial)
        .unwrap()
        .construction_fields
        .as_mut()
        .unwrap()
        .position[0] += 1.;
    assert!(
        s.advance_vimp_vector_offline(
            id,
            Request {
                number: 0,
                origin: [0.; 3],
                fallback: -1.
            },
            &g,
            &mut rng,
            &mut p
        )
        .is_err()
    );
    assert_eq!(rng.0, before);
    assert_eq!(s.vimp_vector_steps(id), Some(0));
    assert!(p.actions.is_empty());
}

struct Quiet;
impl Services for Quiet {
    fn session(
        &mut self,
        _: &Session,
        _: Id,
        _: Predicate,
    ) -> Result<crate::actor_session_predicates::Snapshot<'_>, String> {
        Ok(crate::actor_session_predicates::Snapshot {
            mode: 0,
            declared_count: 0,
            current_key: 0,
            slots: &[],
        })
    }
    fn action(&mut self, _: &mut Session, _: Id, _: Id, _: vector::Action) -> Result<(), String> {
        Err("unexpected action in declared quiet session".into())
    }
}
#[test]
#[ignore = "requires GIANTS_NPC_ROOT; actual initialized cohort, supplied quiet session and affine phase, no primary/world/render proof"]
fn source_vimp_vector_uses_actual_cohort_fields_and_routes_without_extra_draws() {
    let (mut s, _, globals, _, children, mut random) =
        super::vimp_history_tests::source_births(true);
    let mut expected = random;
    let mut count = 0;
    for id in children {
        s.prepare_actor_affine(id).unwrap();
        let old_frames = s.get(id).unwrap().vimp_frames;
        let old_channels = s.get(id).unwrap().channels;
        s.advance_vimp_vector_offline(
            id,
            Request {
                number: 0,
                origin: [1., 2., 3.],
                fallback: -1.,
            },
            &globals,
            &mut random,
            &mut Quiet,
        )
        .unwrap();
        for _ in 0..4 {
            expected.next_15();
        }
        assert_eq!(random.0, expected.0);
        assert_eq!(s.vimp_vector_steps(id), Some(1));
        let a = s.get(id).unwrap();
        assert_eq!(a.vimp_frames, old_frames);
        assert_eq!(a.channels, old_channels);
        assert_eq!(a.vimp.unwrap().mode, 1);
        let body = a.seed.storage.data().unwrap();
        assert_eq!(
            (word(body, 0x74), word(body, 0xb0), word(body, 0xb8)),
            (2, 1, 0)
        );
        count += 1;
    }
    assert_eq!(count, 14);
}
