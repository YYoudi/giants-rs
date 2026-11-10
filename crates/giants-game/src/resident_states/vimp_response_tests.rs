use super::vimp_response::{Action, Request, Services};
use super::*;
use crate::{
    vimp_response as response, vimp_vector_response as vector,
    vimp_wander::{put, scalar, word},
};
struct Provider {
    mode: i32,
    fail: bool,
    cancel: bool,
    actions: Vec<Action>,
}
impl Services for Provider {
    fn session(
        &mut self,
        _: &Session,
        _: Id,
        _: response::Predicate,
    ) -> Result<crate::actor_session_predicates::Snapshot<'_>, String> {
        Ok(crate::actor_session_predicates::Snapshot {
            mode: self.mode,
            declared_count: 0,
            current_key: 0,
            slots: &[],
        })
    }
    fn action(&mut self, s: &mut Session, id: Id, _: Id, action: Action) -> Result<(), String> {
        self.actions.push(action);
        let a = s.residents.get_mut(&id.serial).unwrap();
        let b = a.seed.storage.data_mut().unwrap();
        if let Action::Dispatch(response::Action::Nearby { kind, .. }) = action {
            assert_eq!(word(b, 0x74), kind);
            assert_eq!(word(b, 0xd4), 3);
            assert_eq!((word(b, 0xb0), word(b, 0xb8)), (1, 3));
            assert_eq!(a.construction_fields.as_ref().unwrap().control_flags, 0);
        }
        if matches!(action, Action::Vector(_)) {
            assert_eq!(word(b, 0xd4), 5);
            assert_eq!(scalar(b, 0xc4), 0.);
            assert_eq!(a.construction_fields.as_ref().unwrap().control_flags, 0);
        }
        put(b, 0xd4, 99);
        put(b, 0x3c, 123f32.to_bits());
        a.vimp.as_mut().unwrap().mode = 6;
        a.construction_fields.as_mut().unwrap().control_flags = 0x12345678;
        if self.cancel {
            s.clear();
        }
        if self.fail {
            Err("injected terminal response action failure".into())
        } else {
            Ok(())
        }
    }
}
fn provider(mode: i32) -> Provider {
    Provider {
        mode,
        fail: false,
        cancel: false,
        actions: Vec::new(),
    }
}
fn prepared(
    flags: u32,
) -> (
    Session,
    Id,
    giants_formats::global_data::GlobalData,
    crate::audio_parameters::Random,
) {
    let (mut s, _, g, _, children, random) = super::vimp_history_tests::source_births(true);
    let id = children[0];
    let a = s.residents.get_mut(&id.serial).unwrap();
    // Declared isolated response entry, not a native readiness producer.
    let f = a.construction_fields.as_mut().unwrap();
    f.control_flags = flags;
    f.channels[0][8] |= 2;
    a.channels = Some(f.channels);
    let b = a.seed.storage.data_mut().unwrap();
    assert_eq!(word(b, 0) & 0x40, 0);
    put(b, 0xd4, 0);
    put(b, 0xc4, 2f32.to_bits());
    for (i, v) in [4f32, 5., 6.].into_iter().enumerate() {
        put(b, 0xc8 + 4 * i, v.to_bits());
    }
    put(b, 0x88, 0xaabbccdd);
    s.prepare_actor_affine(id).unwrap();
    (s, id, g, random)
}
fn request(number: u64) -> Request {
    Request {
        number,
        noise_position: [20., 30., 40.],
    }
}
#[test]
#[ignore = "requires GIANTS_NPC_ROOT; owned response to actual vector composition, supplied entry/session/noise/RNG phase"]
fn source_vimp_response_composes_signal_vector_and_keeps_frame_counters_separate() {
    let (mut s, id, g, mut random) = prepared(0x20);
    let mut p = provider(0);
    let mut expected = random;
    let channels = s.get(id).unwrap().channels;
    let frames = s.get(id).unwrap().vimp_frames;
    assert!(
        s.respond_vimp_offline(id, request(0), &g, &mut random, &mut p)
            .unwrap()
    );
    for _ in 0..4 {
        expected.next_15();
    }
    assert_eq!(random.0, expected.0);
    let a = s.get(id).unwrap();
    let b = a.seed.storage.data().unwrap();
    assert_eq!(
        (word(b, 0xd4), word(b, 0x74), word(b, 0xb0), word(b, 0xb8)),
        (5, 2, 1, 0)
    );
    assert_eq!(scalar(b, 0xc4), 0.);
    assert_eq!(a.construction_fields.as_ref().unwrap().control_flags, 0);
    assert_eq!(s.vimp_response_steps(id), Some(1));
    assert_eq!(s.vimp_vector_steps(id), Some(1));
    assert_eq!(a.vimp_frames, frames);
    assert_eq!(a.channels, channels);
    assert!(p.actions.is_empty());
}
#[test]
#[ignore = "requires GIANTS_NPC_ROOT; actual inline physical/channel gates and opaque backwards bytes"]
fn source_vimp_response_retains_gates_and_noise_target_without_fabricating_links() {
    let (mut s, id, g, mut random) = prepared(2);
    let mut p = provider(0);
    let before = random.0;
    let a = s.residents.get_mut(&id.serial).unwrap();
    let b = a.seed.storage.data_mut().unwrap();
    let physical = word(b, 0);
    put(b, 0, physical | 0x40);
    assert!(
        !s.respond_vimp_offline(id, request(0), &g, &mut random, &mut p)
            .unwrap()
    );
    assert_eq!(
        s.get(id)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap()
            .control_flags,
        2
    );
    assert_eq!(random.0, before);
    let a = s.residents.get_mut(&id.serial).unwrap();
    put(a.seed.storage.data_mut().unwrap(), 0, physical);
    let f = a.construction_fields.as_mut().unwrap();
    f.channels[0][8] &= !2;
    a.channels = Some(f.channels);
    assert!(
        !s.respond_vimp_offline(id, request(1), &g, &mut random, &mut p)
            .unwrap()
    );
    assert_eq!(random.0, before);
    let a = s.residents.get_mut(&id.serial).unwrap();
    let f = a.construction_fields.as_mut().unwrap();
    f.channels[0][8] |= 2;
    a.channels = Some(f.channels);
    assert!(
        s.respond_vimp_offline(id, request(2), &g, &mut random, &mut p)
            .unwrap()
    );
    let a = s.get(id).unwrap();
    let b = a.seed.storage.data().unwrap();
    assert_eq!(a.vimp.unwrap().mode, 7);
    assert_eq!(word(b, 0x78), 0);
    assert_eq!(word(b, 0x8c), 0);
    assert_eq!(word(b, 0x88) & 0xffffff00, 0xaabbcc00);
    assert_eq!(
        [scalar(b, 0x7c), scalar(b, 0x80), scalar(b, 0x84)],
        request(0).noise_position
    );
    assert_eq!(random.0, before);
    assert_eq!(s.vimp_vector_steps(id), Some(0));
}
#[test]
#[ignore = "requires GIANTS_NPC_ROOT; terminal action callback changes must survive Outcome projection"]
fn source_vimp_response_keeps_terminal_callback_writes_on_success_and_error() {
    for fail in [false, true] {
        let (mut s, id, g, mut random) = prepared(8);
        let mut p = provider(4);
        p.fail = fail;
        let before = random.0;
        let result = s.respond_vimp_offline(id, request(0), &g, &mut random, &mut p);
        assert_eq!(result.is_err(), fail);
        let a = s.get(id).unwrap();
        let b = a.seed.storage.data().unwrap();
        assert_eq!(word(b, 0xd4), 99);
        assert_eq!(scalar(b, 0x3c), 123.);
        assert_eq!(a.vimp.unwrap().mode, 6);
        assert_eq!(
            a.construction_fields.as_ref().unwrap().control_flags,
            0x12345678
        );
        assert_ne!(random.0, before);
        assert_eq!(
            p.actions,
            [Action::Dispatch(response::Action::Nearby {
                value: 0.,
                kind: 1
            })]
        );
        assert_eq!(s.vimp_response_steps(id), Some(1));
        assert_eq!(a.vimp_frame_stopped, fail);
        assert!(
            s.respond_vimp_offline(id, request(0), &g, &mut random, &mut p)
                .is_err()
        );
        if fail {
            let after = random.0;
            assert!(
                s.respond_vimp_offline(id, request(1), &g, &mut random, &mut p)
                    .is_err()
            );
            assert_eq!(random.0, after);
        }
    }
}
#[test]
#[ignore = "requires GIANTS_NPC_ROOT; nested vector callback failure must retain both owners' prefixes"]
fn source_vimp_response_nested_vector_failure_is_not_republished_from_old_dispatch() {
    let (mut s, id, g, mut random) = prepared(0x20);
    let mut p = provider(4);
    p.fail = true;
    assert!(
        s.respond_vimp_offline(id, request(0), &g, &mut random, &mut p)
            .is_err()
    );
    let a = s.get(id).unwrap();
    let b = a.seed.storage.data().unwrap();
    assert_eq!(word(b, 0xd4), 99);
    assert_eq!(a.vimp.unwrap().mode, 6);
    assert_eq!(
        a.construction_fields.as_ref().unwrap().control_flags,
        0x12345678
    );
    assert_eq!(scalar(b, 0xc4), 0.);
    assert!(a.vimp_frame_stopped);
    assert_eq!(s.vimp_response_steps(id), Some(1));
    assert_eq!(s.vimp_vector_steps(id), Some(1));
    assert!(matches!(
        p.actions.as_slice(),
        [Action::Vector(vector::Action::Retarget { .. })]
    ));
}
#[test]
#[ignore = "requires GIANTS_NPC_ROOT; actual Session cancellation during terminal response action"]
fn source_vimp_response_cancellation_never_restores_old_outcome() {
    let (mut s, id, g, mut random) = prepared(8);
    let mut p = provider(4);
    p.cancel = true;
    assert!(
        s.respond_vimp_offline(id, request(0), &g, &mut random, &mut p)
            .is_err()
    );
    assert!(s.get(id).is_none());
    assert_eq!(s.counts().residents, 0);
    assert!(s.list(0).is_empty());
    assert!(s.specification(id).is_none());
}
#[test]
#[ignore = "requires GIANTS_NPC_ROOT; link/target, foreign, reentry and stale affine preflight preserve RNG and payload"]
fn source_vimp_response_unowned_identity_and_preflight_refusals_are_not_silent_clears() {
    let (mut s, id, g, mut random) = prepared(2);
    let mut p = provider(0);
    let before = random.0;
    let foreign = Id {
        ticket: id.ticket + 1,
        serial: id.serial,
    };
    assert!(
        s.respond_vimp_offline(foreign, request(0), &g, &mut random, &mut p)
            .is_err()
    );
    s.residents
        .get_mut(&id.serial)
        .unwrap()
        .vimp_response_running = true;
    assert!(
        s.respond_vimp_offline(id, request(0), &g, &mut random, &mut p)
            .is_err()
    );
    s.residents
        .get_mut(&id.serial)
        .unwrap()
        .vimp_response_running = false;
    for at in [0x78, 0x8c] {
        put(
            s.residents
                .get_mut(&id.serial)
                .unwrap()
                .seed
                .storage
                .data_mut()
                .unwrap(),
            at,
            1,
        );
        assert!(
            s.respond_vimp_offline(id, request(0), &g, &mut random, &mut p)
                .is_err()
        );
        assert_eq!(word(s.get(id).unwrap().seed.storage.data().unwrap(), at), 1);
        put(
            s.residents
                .get_mut(&id.serial)
                .unwrap()
                .seed
                .storage
                .data_mut()
                .unwrap(),
            at,
            0,
        );
    }
    s.residents
        .get_mut(&id.serial)
        .unwrap()
        .construction_fields
        .as_mut()
        .unwrap()
        .position[0] += 1.;
    assert!(
        s.respond_vimp_offline(id, request(0), &g, &mut random, &mut p)
            .is_err()
    );
    assert_eq!(random.0, before);
    assert_eq!(s.vimp_response_steps(id), Some(0));
    assert!(p.actions.is_empty());
}
