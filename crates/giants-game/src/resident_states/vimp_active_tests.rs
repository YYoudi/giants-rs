use super::*;
use crate::{
    vimp_active_response::Action,
    vimp_response::Predicate,
    vimp_wander::{put, word},
};
struct Provider {
    seconds: f64,
    actions: Vec<Action>,
    slots: Vec<u32>,
    network: bool,
    change: bool,
    heading_error: bool,
    action_error: bool,
    cancel: bool,
    calls: Vec<&'static str>,
}
impl VimpActiveServices for Provider {
    fn session(
        &mut self,
        _: &Session,
        _: Id,
        q: Predicate,
    ) -> Result<crate::actor_session_predicates::Snapshot<'_>, String> {
        self.calls.push(match q {
            Predicate::First => "first",
            Predicate::Second => "second",
        });
        Ok(crate::actor_session_predicates::Snapshot {
            mode: if self.network { 4 } else { 0 },
            declared_count: 256,
            current_key: 17,
            slots: &self.slots,
        })
    }
    fn seconds(&mut self, _: &Session, _: Id) -> Result<f64, String> {
        self.calls.push("clock");
        Ok(self.seconds)
    }
    fn orient(&mut self, s: &mut Session, id: Id) -> Result<(), String> {
        self.calls.push("orient");
        if self.change {
            let a = s.residents.get_mut(&id.serial).unwrap();
            a.construction_fields.as_mut().unwrap().angles[0] = 47.;
            let v = a.vimp.as_mut().unwrap();
            v.angular_speed = 8.;
            v.commands[0] = 9;
            v.born.as_mut().unwrap().delay = 0.25;
            put(a.seed.storage.data_mut().unwrap(), 0xd4, 77);
        }
        if self.cancel {
            s.clear();
        }
        if self.heading_error {
            Err("declared heading failure".into())
        } else {
            Ok(())
        }
    }
    fn action(&mut self, s: &mut Session, id: Id, action: Action) -> Result<(), String> {
        self.calls.push("action");
        self.actions.push(action);
        let a = s.residents.get_mut(&id.serial).unwrap();
        let b = a.seed.storage.data_mut().unwrap();
        match action {
            Action::Heading(_) => assert_eq!(word(b, 0xb8), 2),
            Action::Finished => {
                assert_eq!((word(b, 0xb0), word(b, 0x74), word(b, 0xd4)), (0, 0, 0))
            }
        }
        put(b, 0xd4, 99);
        a.vimp.as_mut().unwrap().mode = 6;
        if self.cancel {
            s.clear();
        }
        if self.action_error {
            Err("declared terminal action failure".into())
        } else {
            Ok(())
        }
    }
}
fn prepared(
    stage: u32,
) -> (
    Session,
    Id,
    giants_formats::global_data::GlobalData,
    crate::audio_parameters::Random,
    Provider,
) {
    let (mut s, _, g, _, children, r) = super::vimp_history_tests::source_births(true);
    let id = children[0];
    let a = s.residents.get_mut(&id.serial).unwrap();
    let v = a.vimp.as_mut().unwrap();
    assert!(v.response_turn_timer.is_none());
    let mut slots = vec![0; 256];
    slots[usize::from(v.born.unwrap().uid as u8)] = 17;
    // Declared isolated active entry, not an acquired whole-primary producer.
    let heading = a.construction_fields.as_ref().unwrap().angles[0];
    let b = a.seed.storage.data_mut().unwrap();
    for (at, value) in [
        (0xb0, 1),
        (0x74, 2),
        (0xb8, stage),
        (0xb4, heading.to_bits()),
        (0xd4, 5),
    ] {
        put(b, at, value);
    }
    (
        s,
        id,
        g,
        r,
        Provider {
            seconds: 0.01,
            actions: vec![],
            slots,
            network: false,
            change: false,
            heading_error: false,
            action_error: false,
            cancel: false,
            calls: vec![],
        },
    )
}
#[test]
#[ignore = "requires GIANTS_NPC_ROOT; actual source owners with declared active/clock/session inputs"]
fn source_vimp_active_arrival_owns_timer_across_invocations_and_refuses_replay() {
    let (mut s, id, g, mut r, mut p) = prepared(0);
    s.advance_vimp_active_response_offline(id, 0, &g, &mut r, &mut p)
        .unwrap();
    let timer = s.vimp(id).unwrap().response_turn_timer.unwrap();
    assert_eq!(
        word(s.get(id).unwrap().seed.storage.data().unwrap(), 0xb8),
        1
    );
    s.advance_vimp_active_response_offline(id, 1, &g, &mut r, &mut p)
        .unwrap();
    assert_eq!(
        s.vimp(id).unwrap().response_turn_timer,
        Some((f64::from(timer) - p.seconds) as f32)
    );
    assert_eq!(s.vimp_active_steps(id), Some(2));
    let before = r.0;
    assert!(
        s.advance_vimp_active_response_offline(id, 1, &g, &mut r, &mut p)
            .is_err()
    );
    assert_eq!(r.0, before);
    assert_eq!(s.vimp_frames(id), Some(0));
}
#[test]
#[ignore = "requires GIANTS_NPC_ROOT; absent producer is not a zero timer"]
fn source_vimp_active_unknown_timer_refuses_at_read_and_quarantines() {
    let (mut s, id, g, mut r, mut p) = prepared(1);
    let error = s
        .advance_vimp_active_response_offline(id, 0, &g, &mut r, &mut p)
        .unwrap_err();
    assert!(error.contains("producer absent"));
    assert_eq!(p.calls, ["first"]);
    assert_eq!(s.vimp(id).unwrap().response_turn_timer, None);
    assert!(s.get(id).unwrap().vimp_frame_stopped);
    assert_eq!(s.vimp_active_steps(id), Some(1));
    assert!(
        s.advance_vimp_active_response_offline(id, 1, &g, &mut r, &mut p)
            .is_err()
    );
}
#[test]
#[ignore = "requires GIANTS_NPC_ROOT; foreign, pending and reentrant preflight has no effects"]
fn source_vimp_active_preflight_preserves_timer_payload_and_random() {
    let (mut s, id, g, mut r, mut p) = prepared(0);
    let before = s.get(id).unwrap().seed.storage.data().unwrap().to_vec();
    let seed = r.0;
    let foreign = Id {
        ticket: id.ticket ^ 1,
        ..id
    };
    assert!(
        s.advance_vimp_active_response_offline(foreign, 0, &g, &mut r, &mut p)
            .is_err()
    );
    for pending in [true, false] {
        let a = s.residents.get_mut(&id.serial).unwrap();
        a.vimp_prepared_history = pending.then_some(0);
        a.vimp_active_running = !pending;
        assert!(
            s.advance_vimp_active_response_offline(id, 0, &g, &mut r, &mut p)
                .is_err()
        );
    }
    let a = s.get(id).unwrap();
    assert_eq!(a.seed.storage.data().unwrap(), before);
    assert_eq!(a.vimp.unwrap().response_turn_timer, None);
    assert_eq!(r.0, seed);
    assert!(p.calls.is_empty());
    assert_eq!(s.vimp_active_steps(id), Some(0));
}
#[test]
#[ignore = "requires GIANTS_NPC_ROOT; source owner with mutable required heading callback"]
fn source_vimp_active_heading_rereads_callback_writes_on_success_and_error() {
    for fail in [false, true] {
        let (mut s, id, g, mut r, mut p) = prepared(3);
        p.change = true;
        p.heading_error = fail;
        p.seconds = 0.;
        assert_eq!(
            s.advance_vimp_active_response_offline(id, 0, &g, &mut r, &mut p)
                .is_err(),
            fail
        );
        let a = s.get(id).unwrap();
        assert_eq!(a.construction_fields.as_ref().unwrap().angles[0], 47.);
        let v = a.vimp.unwrap();
        assert_eq!(
            (v.angular_speed, v.commands[0], v.born.unwrap().delay),
            (8., 9, 0.25)
        );
        assert_eq!(word(a.seed.storage.data().unwrap(), 0xd4), 77);
        assert_eq!(a.vimp_frame_stopped, fail);
        assert_eq!(
            p.calls,
            if fail {
                vec!["orient"]
            } else {
                vec!["orient", "first", "clock"]
            }
        );
    }
}
#[test]
#[ignore = "requires GIANTS_NPC_ROOT; declared network action callbacks retain terminal effects"]
fn source_vimp_active_terminal_actions_keep_prefix_and_callback_effects() {
    for stage in [1, 3] {
        for fail in [false, true] {
            let (mut s, id, g, mut r, mut p) = prepared(stage);
            p.network = true;
            p.seconds = 100.;
            p.action_error = fail;
            s.residents
                .get_mut(&id.serial)
                .unwrap()
                .vimp
                .as_mut()
                .unwrap()
                .response_turn_timer = Some(0.25);
            let result = s.advance_vimp_active_response_offline(id, 0, &g, &mut r, &mut p);
            assert_eq!(result.is_err(), fail);
            assert_eq!(p.actions.len(), 1);
            let a = s.get(id).unwrap();
            assert_eq!(word(a.seed.storage.data().unwrap(), 0xd4), 99);
            assert_eq!(a.vimp.unwrap().mode, 6);
            assert_eq!(a.vimp_frame_stopped, fail);
        }
    }
}
#[test]
#[ignore = "requires GIANTS_NPC_ROOT; callback cancellation must never restore a retired owner"]
fn source_vimp_active_cancellation_never_resurrects_retired_owner() {
    for stage in [1, 3] {
        let (mut s, id, g, mut r, mut p) = prepared(stage);
        p.network = true;
        p.seconds = 100.;
        p.cancel = true;
        s.residents
            .get_mut(&id.serial)
            .unwrap()
            .vimp
            .as_mut()
            .unwrap()
            .response_turn_timer = Some(0.25);
        let result = s.advance_vimp_active_response_offline(id, 0, &g, &mut r, &mut p);
        // Stage1's action is terminal; stage3's earlier heading cancellation
        // refuses before the next predicate. Neither may resurrect the owner.
        assert_eq!(result.is_err(), stage == 3);
        assert!(s.get(id).is_none());
    }
}
