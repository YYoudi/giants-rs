use super::*;
use crate::actor_fields::Fields;

#[derive(Default)]
struct Backend {
    seen: Vec<(State<u8>, Binding)>,
    fail: bool,
}

// This synthetic profile exercises the direct handoff routes. Every service
// outside that profile explicitly refuses, including unused network/field reads.
impl actor_reference_detach::Services<u8, State<u8>> for Backend {
    fn fields(&self, _: u8) -> Result<&Fields, String> {
        Err("unexpected field read".into())
    }
    fn fields_mut(&mut self, _: u8) -> Result<&mut Fields, String> {
        Err("unexpected field write".into())
    }
    fn state(&self, _: u8) -> Result<u32, String> {
        Err("unexpected state read".into())
    }
    fn parent_reference(&self, _: u8) -> Result<i32, String> {
        Err("unexpected parent read".into())
    }
    fn clear_parent(&mut self, _: u8) -> Result<(), String> {
        Err("unexpected parent clear".into())
    }
    fn set_state89_parameter(&mut self, _: u8, _: f32) -> Result<(), String> {
        Err("unexpected parameter store".into())
    }
    fn reset_state18(&mut self, _: &mut State<u8>, _: u8) -> Result<(), String> {
        Err("unexpected reset".into())
    }
    fn state3_scale(&self, _: u8) -> Result<f32, String> {
        Err("unexpected state3 read".into())
    }
}
impl Services<u8> for Backend {
    fn session_mode(&self) -> Result<u32, String> {
        Ok(0)
    }
    fn notification_context(&self, _: u8) -> Result<NotificationContext<'_>, String> {
        Err("unexpected network read".into())
    }
    fn physical_flags(&self, _: u8) -> Result<u32, String> {
        Err("unexpected physical read".into())
    }
    fn type_id(&self, _: u8) -> Result<u32, String> {
        Err("unexpected type read".into())
    }
    fn owner_word(&self, _: u8) -> Result<u32, String> {
        Err("unexpected owner read".into())
    }
    fn set_owner_word(&mut self, _: u8, _: u32) -> Result<(), String> {
        Err("unexpected owner store".into())
    }
    fn construct(&mut self, _: &mut State<u8>, _: u8, _: u32, _: u32) -> Result<u8, String> {
        Err("unexpected construction".into())
    }
    fn notify_created(&mut self, _: &mut State<u8>, _: u8, _: CreatedNotice) -> Result<(), String> {
        Err("unexpected creation notice".into())
    }
    fn notify_release(&mut self, _: &mut State<u8>, _: u8, _: Notice<u8>) -> Result<(), String> {
        Err("unexpected release notice".into())
    }
    fn action(&mut self, _: &mut State<u8>, _: Request<u8>) -> Result<(), String> {
        Err("unexpected action".into())
    }
    fn bind(
        &mut self,
        state: &mut State<u8>,
        source: u8,
        child: u8,
        input: Binding,
    ) -> Result<(), String> {
        assert_eq!((source, child), (1, 2));
        self.seen.push((*state, input));
        state.reference = Some(9);
        state.held = [Some(7), Some(8)];
        if self.fail {
            Err("declared late bind error".into())
        } else {
            Ok(())
        }
    }
}

fn state() -> State<u8> {
    State {
        reference: Some(2),
        held: [None, Some(3)],
        reference_scale: -0.,
        held_scales: [11., 12.],
    }
}

#[test]
fn store_publishes_the_free_slot_before_binding_and_keeps_callback_changes() {
    let mut state = state();
    let mut world = Backend::default();
    let input = Input {
        decision: 11,
        previous: 0,
        current: 4799,
        finished: false,
    };
    assert_eq!(
        advance(&mut state, 1, input, &mut world).unwrap(),
        Next::ContinueCaller
    );
    assert!(world.seen.is_empty());
    assert_eq!(
        advance(
            &mut state,
            1,
            Input {
                current: 4800,
                ..input
            },
            &mut world
        )
        .unwrap(),
        Next::Processed
    );
    let (seen, request) = &world.seen[0];
    assert_eq!(seen.reference, None);
    assert_eq!(seen.held, [Some(2), Some(3)]);
    assert_eq!(seen.held_scales[0].to_bits(), (-0f32).to_bits());
    assert_eq!(seen.held_scales[1], 12.);
    assert_eq!(request.parent_reference, 0);
    assert!(request.notify);
    assert_eq!(state.reference, Some(9));
    assert_eq!(state.held, [Some(7), Some(8)]);
}

#[test]
fn return_clears_the_captured_slot_after_success_but_preserves_late_failure() {
    for fail in [false, true] {
        let mut state = State {
            reference: None,
            held: [Some(2), Some(3)],
            reference_scale: 1.,
            held_scales: [-2., 4.],
        };
        let mut world = Backend {
            fail,
            ..Default::default()
        };
        let result = advance(
            &mut state,
            1,
            Input {
                decision: 10,
                previous: i32::MIN,
                current: 4160,
                finished: true,
            },
            &mut world,
        );
        let (seen, request) = &world.seen[0];
        assert_eq!(seen.reference, Some(2));
        assert_eq!(seen.held, [Some(2), Some(3)]);
        assert_eq!(seen.reference_scale, -2.);
        assert_eq!(request.parent_reference, 3);
        assert_eq!(state.reference, Some(9));
        assert_eq!(state.held, [if fail { Some(7) } else { None }, Some(8)]);
        if fail {
            assert!(result.is_err());
        } else {
            assert_eq!(result.unwrap(), Next::ContinueCaller);
        }
    }
}

#[test]
fn full_decision_and_signed_crossing_gates_do_not_read_unused_services() {
    let mut world = Backend::default();
    let mut state = State {
        held: [Some(2), None],
        ..state()
    };
    let input = Input {
        decision: 0x1000000a,
        previous: 0,
        current: 4160,
        finished: false,
    };
    assert_eq!(
        advance(&mut state, 1, input, &mut world).unwrap(),
        Next::ContinueCaller
    );
    assert_eq!(
        advance(
            &mut state,
            1,
            Input {
                decision: 10,
                current: i32::MIN,
                ..input
            },
            &mut world
        )
        .unwrap(),
        Next::Processed
    );
    assert_eq!(
        advance(
            &mut state,
            1,
            Input {
                decision: 10,
                previous: 4160,
                ..input
            },
            &mut world
        )
        .unwrap(),
        Next::Processed
    );
    assert!(world.seen.is_empty());
    assert_eq!(state.held, [Some(2), None]);
}
