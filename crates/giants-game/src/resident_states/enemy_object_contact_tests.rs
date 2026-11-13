use crate::enemy_projectile::{
    FlightOutcome, ObjectContactOutcome, ObjectImpact, ObjectImpactServices, Phase,
};
mod canonical_mecc {
    use super::*;
    include!("mecc_enemy_hit_tests.rs");
}

// Supplied query and service fixture, not ordinary gameplay or collision proof.
fn fixture(session: &mut Session) -> (Id, Id, crate::audio_parameters::Random) {
    let (shot, random) = impact_fixture(session);
    let target = fixture_actor(session, 219, 48);
    let actor = session.residents.get_mut(&shot.serial).unwrap();
    actor.construction_fields.as_mut().unwrap().configuration[12] = 20f32.to_bits();
    actor.construction_fields.as_mut().unwrap().position = [4., 6., 2.];
    actor.seed.storage.data_mut().unwrap()[4..8].copy_from_slice(&7f32.to_le_bytes());
    actor.enemy_projectile.as_mut().unwrap().pending_flight = Some(FlightOutcome::ObjectContact {
        target, point: [3.5, 5.5, 1.5], tag: 0x1234,
    });
    (shot, target, random)
}
#[derive(Default)]
struct Services {
    events: Vec<&'static str>,
    suppressed: bool,
    fail: Option<&'static str>,
    clear_terminal: bool,
    source: Option<Id>,
}
impl ObjectImpactServices for Services {
    fn world_suppressed(&mut self, _: &Session) -> Result<bool, String> {
        self.events.push("world");
        if self.fail == Some("world") { return Err("supplied world error".into()); }
        Ok(self.suppressed)
    }
    fn hurt(&mut self, session: &mut Session, request: ObjectImpact, random: &mut crate::audio_parameters::Random) -> Result<(), String> {
        self.events.push("hurt");
        assert_eq!(request.point, [3.5, 5.5, 1.5]);
        assert_eq!(request.tag, 0x1234);
        assert_eq!(request.amount, 20.);
        assert_eq!(session.get(request.source).unwrap().construction_fields.as_ref().unwrap().position, [4., 6., 2.]);
        assert_eq!(session.get(request.source).unwrap().construction_fields.as_ref().unwrap().object_flags & 0x8000, 0);
        // Visible irreversible service work must survive a later error.
        random.next_15();
        if self.fail == Some("hurt") { return Err("supplied hurt error".into()); }
        Ok(())
    }
    fn terminal(&mut self, session: &mut Session, target: Id) -> Result<(), String> {
        self.events.push("terminal");
        let source = self.source.unwrap();
        assert!(session.enemy_object_terminal_no_effect(source, target)?);
        if self.clear_terminal { session.clear(); }
        if self.fail == Some("terminal") { return Err("supplied terminal error".into()); }
        Ok(())
    }
}
#[test]
fn direct_order_immunities_and_object_only_retirement() {
    for suppressed in [false, true] {
        for immunity in [0, 14, 15] {
            let mut session = Session::new(1001);
            let (shot, target, mut random) = fixture(&mut session);
            if immunity != 0 {
                session.residents.get_mut(&target.serial).unwrap().construction_fields.as_mut().unwrap().configuration[immunity] |= 0x4000000;
            }
            let mut services = Services { suppressed, source: Some(shot), ..Default::default() };
            let before = random.0;
            let marked = !suppressed && immunity == 0;
            let outcome = session.complete_enemy_projectile_object_contact(shot, &mut random, &mut services).unwrap();
            assert_eq!(outcome, if marked { ObjectContactOutcome::Marked } else { ObjectContactOutcome::Retained });
            assert_eq!(services.events, if suppressed || immunity == 15 { vec!["world"] } else if immunity == 14 { vec!["world", "terminal"] } else { vec!["world", "hurt", "terminal"] });
            assert_eq!(random.0 == before, !marked);
            let actor = session.get(shot).unwrap();
            assert_eq!(&actor.seed.storage.data().unwrap()[4..8], &7f32.to_le_bytes());
            assert_eq!(actor.construction_fields.as_ref().unwrap().position, [4., 6., 2.]);
            assert_eq!(actor.construction_fields.as_ref().unwrap().object_flags & 0x8000 != 0, marked);
            let meta = session.enemy_projectile(shot).unwrap();
            assert!(meta.ground_impact.is_none());
            assert!(meta.pending_flight.is_none());
            assert_eq!(meta.object_impact.is_some(), marked);
            assert!(!session.incoming_actor_references(target).unwrap().contains(&shot));
            let event_count = services.events.len();
            assert!(session.complete_enemy_projectile_object_contact(shot, &mut random, &mut services).is_err());
            assert_eq!(services.events.len(), event_count);
            let mut release = RetirementMock::default();
            assert!(session.cleanup_enemy_projectile(shot, &mut release).is_err());
            assert_eq!(release.calls, 0);
            if marked {
                session.cleanup_enemy_projectile_object_contact(shot, &mut release).unwrap();
                assert_eq!(release.calls, 1);
                assert!(session.get(shot).is_none());
                assert!(session.get(target).is_some());
            } else {
                assert!(session.cleanup_enemy_projectile_object_contact(shot, &mut release).is_err());
                assert_eq!(release.calls, 0);
            }
        }
    }
}
#[test]
fn direct_late_failures_preserve_mark_and_prevent_replay() {
    for failure in ["world", "hurt", "terminal"] {
        let mut session = Session::new(1002);
        let (shot, target, mut random) = fixture(&mut session);
        let before = random.0;
        let mut services = Services { fail: Some(failure), source: Some(shot), ..Default::default() };
        assert!(session.complete_enemy_projectile_object_contact(shot, &mut random, &mut services).is_err());
        let meta = session.enemy_projectile(shot).unwrap();
        assert_eq!(meta.phase, Phase::Failed);
        assert!(meta.pending_flight.is_some());
        assert!(session.incoming_actor_references(target).unwrap().contains(&shot));
        assert_eq!(session.get(shot).unwrap().construction_fields.as_ref().unwrap().object_flags & 0x8000 != 0, failure == "terminal");
        assert_eq!(random.0 == before, failure == "world");
        let count = services.events.len();
        assert!(session.complete_enemy_projectile_object_contact(shot, &mut random, &mut services).is_err());
        assert_eq!(count, services.events.len());
    }
}
#[test]
fn direct_terminal_cannot_remove_session_and_report_success() {
    let mut session = Session::new(1003);
    let (shot, _, mut random) = fixture(&mut session);
    let mut services = Services { clear_terminal: true, source: Some(shot), ..Default::default() };
    assert!(session.complete_enemy_projectile_object_contact(shot, &mut random, &mut services).is_err());
    assert!(session.get(shot).is_none());
}

// Full dispatcher/callback observations stay private. These explicitly supplied
// world guards and motion fixtures do not install an ordinary world owner.
#[test]
#[ignore = "Requires GIANTS_ENEMY_OBJECT_CONTROLS; private original dispatcher observations"]
fn native_direct_dispatcher_controls() {
    use crate::{audio_parameters::Random, damage_impulse::Motion, mecc_enemy_hit, mecc_hit_source};
    struct NoAux;
    impl mecc_enemy_hit::Services<Id> for NoAux {
        fn retaliation(&mut self, _: Id, _: Option<Id>) -> Result<(), String> {
            panic!("supplied cold recipient has no associated player")
        }
        fn voice(&mut self, _: u32, _: &mecc_enemy_hit::State, _: &Random) -> Result<(), String> {
            panic!("supplied zero timers must not produce a voice")
        }
    }
    struct NativeServices {
        guards: [u32; 5],
        state: mecc_enemy_hit::State,
        visits: [u32; 2],
        source: Id,
    }
    impl ObjectImpactServices for NativeServices {
        fn world_suppressed(&mut self, _: &Session) -> Result<bool, String> {
            Ok(self.guards[0] != 0 || self.guards[1] != 0)
        }
        fn hurt(&mut self, session: &mut Session, request: ObjectImpact, random: &mut Random) -> Result<(), String> {
            self.visits[0] += 1;
            let actor = |id| {
                let a = session.get(id).unwrap();
                mecc_hit_source::Actor { id, class: a.class, state: u32::from(a.seed.storage.state()), context: a.construction_fields.as_ref().unwrap().dispatch_context }
            };
            let motion = |id| {
                let f = session.get(id).unwrap().construction_fields.as_ref().unwrap();
                Motion { current: f.position, previous: f.current }
            };
            let owner = session.enemy_projectile(request.source).unwrap().launch.as_ref().unwrap().source;
            let mut machine = mecc_enemy_hit::Machine::default();
            machine.run(&mut self.state, mecc_enemy_hit::Request {
                recipient: actor(request.target),
                source: mecc_hit_source::Source { actor: actor(request.source), owner: Some(actor(owner)) },
                associated_player: None, kind: 0, amount: request.amount,
                globally_disabled: self.guards[2] != 0, suppressed: self.guards[3] != 0,
                gyro_present: false, critical_voice_present: false,
                recipient_motion: motion(request.target), source_motion: motion(request.source),
                impact: Some(request.point), radial_center: motion(request.target).current,
            }, random, &mut NoAux)
        }
        fn terminal(&mut self, session: &mut Session, target: Id) -> Result<(), String> {
            self.visits[1] += 1;
            assert!(session.enemy_object_terminal_no_effect(self.source, target)?);
            Ok(())
        }
    }
    let path = std::env::var("GIANTS_ENEMY_OBJECT_CONTROLS").expect("private controls path");
    let contents = std::fs::read_to_string(path).unwrap();
    let mut count = 0;
    for line in contents.lines().filter(|s| !s.is_empty() && !s.starts_with('#')) {
        let words: Vec<u32> = line.split_whitespace().map(|s| u32::from_str_radix(s, 16).unwrap()).collect();
        assert_eq!(words.len(), 19);
        let guards: [u32; 5] = words[..5].try_into().unwrap();
        let mut session = Session::new(1010 + count);
        let (shot, mut random) = impact_fixture(&mut session);
        let target = fixture_actor(&mut session, 2, 2);
        let owner = session.enemy_projectile(shot).unwrap().launch.as_ref().unwrap().source;
        session.residents.get_mut(&owner.serial).unwrap().construction_fields.as_mut().unwrap().dispatch_context = 100;
        let f = session.residents.get_mut(&target.serial).unwrap().construction_fields.as_mut().unwrap();
        f.position = [8., 10., 4.]; f.current = [7., 9., 3.];
        if guards[4] == 1 { f.configuration[15] |= 0x4000000; }
        if guards[4] == 2 { f.configuration[14] |= 0x4000000; }
        let source = session.residents.get_mut(&shot.serial).unwrap();
        let f = source.construction_fields.as_mut().unwrap();
        f.position = [4., 6., 2.]; f.current = [1., 2., 1.]; f.dispatch_context = 100;
        f.configuration[12] = 20f32.to_bits();
        source.seed.storage.data_mut().unwrap()[4..8].copy_from_slice(&7f32.to_le_bytes());
        source.enemy_projectile.as_mut().unwrap().pending_flight = Some(FlightOutcome::ObjectContact { target, point: [3.5, 5.5, 1.5], tag: 0x1234 });
        random.0 = 17;
        let mut services = NativeServices {
            guards, source: shot, visits: [0; 2],
            state: mecc_enemy_hit::State {
                health: crate::mecc_damage::State { damage: 0., limit: 100., shield: 0., flags: 0 },
                velocity: [1., 2., 3.], pain_timer: 0., shared_timer: 0., callback_reset_word: 42,
            },
        };
        session.complete_enemy_projectile_object_contact(shot, &mut random, &mut services).unwrap();
        let source = session.get(shot).unwrap();
        let f = source.construction_fields.as_ref().unwrap();
        let mut actual = vec![services.state.health.damage.to_bits(), services.state.health.flags];
        actual.extend(services.state.velocity.map(f32::to_bits));
        actual.push(f.object_flags);
        actual.extend(f.position.map(f32::to_bits));
        actual.push(u32::from_le_bytes(source.seed.storage.data().unwrap()[4..8].try_into().unwrap()));
        actual.extend([services.state.callback_reset_word, random.0]);
        actual.extend(services.visits);
        assert_eq!(actual, words[5..], "private direct dispatcher row {count}, guards {guards:?}");
        count += 1;
    }
    assert_eq!(count, 48);
}
