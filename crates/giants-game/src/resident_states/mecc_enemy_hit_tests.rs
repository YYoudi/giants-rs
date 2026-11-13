use crate::{audio_parameters::Random, controlled_mecc::Frame};

// Explicit cold construction and current motion fixture. Required world gates,
// controller and cue consumers below are supplied controls, not game services.
fn controlled_fixture(session: &mut Session) -> (Id, Id, Random) {
    controlled_fixture_with_clock(session, Some(0.))
}
fn controlled_fixture_with_clock(session: &mut Session, shared: Option<f32>) -> (Id, Id, Random) {
    let mut templates = Session::new(1100);
    let children = [84, 85, 781].map(|class| fixture_actor(&mut templates, class, 0));
    let child_seed = seed(0);
    let inputs = children.map(|id| {
        let a = templates.get(id).unwrap();
        Construction {
            class: a.class,
            seed: &child_seed,
            bindings: &[],
            flags: a.initial_flags,
            specification: templates.specification(id).unwrap().clone(),
            state_resources: templates.state_resources(id).unwrap().clone(),
            fields: a.construction_fields.as_ref().unwrap().clone(),
        }
    });
    let target = fixture_actor(session, 2, 2);
    session
        .links
        .insert(1, target.serial, crate::object_links::Position::Tail);
    session
        .residents
        .get_mut(&target.serial)
        .unwrap()
        .construction_fields
        .as_mut()
        .unwrap()
        .configuration[12] = 100f32.to_bits();
    session
        .initialize_cold_mecc_player_fields(
            target,
            &seed(2),
            inputs,
            1.5,
            crate::animation_channels::Blending::Enabled,
        )
        .unwrap();
    let f = session
        .get(target)
        .unwrap()
        .construction_fields
        .as_ref()
        .unwrap()
        .clone();
    session
        .publish_controlled_mecc_motion(
            target,
            1,
            Frame {
                before: f.position,
                position: [8., 10., 4.],
                contact_previous: [7., 9., 3.],
                angles: f.angles,
                scale: f.scale,
                velocity: [1., 2., 3.],
                model_rates: [0.; 3],
            },
        )
        .unwrap();
    if let Some(value) = shared {
        session.prepare_mecc_countdown_world(value).unwrap();
    }
    session
        .residents
        .get_mut(&target.serial)
        .unwrap()
        .seed
        .storage
        .data_mut()
        .unwrap()[0xf4..0xf8]
        .copy_from_slice(&42u32.to_le_bytes());
    let (shot, mut random) = impact_fixture(session);
    let a = session.residents.get_mut(&shot.serial).unwrap();
    let f = a.construction_fields.as_mut().unwrap();
    f.configuration[12] = 20f32.to_bits();
    f.position = [4., 6., 2.];
    f.current = [1., 2., 1.];
    f.dispatch_context = 100;
    a.seed.storage.data_mut().unwrap()[4..8].copy_from_slice(&7f32.to_le_bytes());
    a.enemy_projectile.as_mut().unwrap().pending_flight = Some(FlightOutcome::ObjectContact {
        target,
        point: [3.5, 5.5, 1.5],
        tag: 0x1234,
    });
    random.0 = 17;
    (shot, target, random)
}
#[derive(Default)]
struct Receiver {
    disabled: bool,
    suppressed: bool,
    fail: Option<&'static str>,
    events: Vec<&'static str>,
    cues: Vec<u32>,
}
impl MeccEnemyHitServices for Receiver {
    fn gates(&mut self, _: &Session, _: Id) -> Result<MeccEnemyHitGates, String> {
        self.events.push("gates");
        if self.fail == Some("gates") {
            return Err("supplied real gate failure".into());
        }
        Ok(MeccEnemyHitGates {
            globally_disabled: self.disabled,
            health_suppressed: self.suppressed,
        })
    }
    fn recoil(
        &mut self,
        session: &Session,
        target: Id,
        step: u64,
        velocity: [f32; 3],
    ) -> Result<(), String> {
        self.events.push("recoil");
        assert_eq!(
            session.mecc_health(target)?.damage,
            if self.suppressed { 0. } else { 20. }
        );
        assert_eq!(
            session.controlled_mecc_motion(target, step)?.frame.velocity,
            velocity
        );
        assert_ne!(velocity, [1., 2., 3.]);
        assert_eq!(
            session
                .get(target)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap()
                .position,
            [8., 10., 4.]
        );
        if self.fail == Some("recoil") {
            return Err("supplied controller failure".into());
        }
        Ok(())
    }
    fn voice(&mut self, session: &Session, target: Id, cue: u32) -> Result<(), String> {
        self.events.push("voice");
        self.cues.push(cue);
        assert_eq!(
            session.mecc_health(target)?.damage,
            if self.suppressed { 0. } else { 20. }
        );
        assert_eq!(self.events.iter().filter(|&&s| s == "recoil").count(), 1);
        if self.fail == Some("voice") {
            return Err("supplied actual voice failure".into());
        }
        Ok(())
    }
}
#[derive(Default)]
struct Direct {
    receiver: Receiver,
    world_suppressed: bool,
    source: Option<Id>,
    terminal_count: u32,
}
impl ObjectImpactServices for Direct {
    fn world_suppressed(&mut self, _: &Session) -> Result<bool, String> {
        Ok(self.world_suppressed)
    }
    fn hurt(
        &mut self,
        session: &mut Session,
        request: ObjectImpact,
        random: &mut Random,
    ) -> Result<(), String> {
        session.damage_mecc_from_enemy_object(request, 1, random, &mut self.receiver)
    }
    fn terminal(&mut self, session: &mut Session, target: Id) -> Result<(), String> {
        self.terminal_count += 1;
        assert!(session.enemy_object_terminal_no_effect(self.source.unwrap(), target)?);
        Ok(())
    }
}
#[test]
fn canonical_direct_health_motion_and_suppression_publish_before_consumers() {
    for disabled in [false, true] {
        for suppressed in [false, true] {
            let mut session = Session::new(1101);
            let (shot, target, mut random) = controlled_fixture(&mut session);
            let mut services = Direct {
                source: Some(shot),
                receiver: Receiver {
                    disabled,
                    suppressed,
                    ..Default::default()
                },
                ..Default::default()
            };
            session
                .complete_enemy_projectile_object_contact(shot, &mut random, &mut services)
                .unwrap();
            let health = session.mecc_health(target).unwrap();
            assert_eq!(health.damage, if disabled || suppressed { 0. } else { 20. });
            assert_eq!(health.flags, if disabled { 0 } else { 16 });
            assert_eq!(
                session
                    .controlled_mecc_motion(target, 1)
                    .unwrap()
                    .frame
                    .velocity
                    == [1., 2., 3.],
                disabled
            );
            assert_eq!(
                services.receiver.events,
                if disabled {
                    vec!["gates"]
                } else {
                    vec!["gates", "recoil"]
                }
            );
            assert_eq!(services.terminal_count, 1);
            assert_eq!(random.0, 17);
            let source = session.get(shot).unwrap();
            assert_ne!(
                source.construction_fields.as_ref().unwrap().object_flags & 0x8000,
                0
            );
            assert_eq!(
                source.enemy_projectile.as_ref().unwrap().mecc_receiver,
                Some(crate::mecc_enemy_hit::Phase::Complete)
            );
        }
    }
}
#[test]
fn canonical_direct_failed_controller_or_voice_retains_state_without_replay() {
    for failure in ["gates", "recoil", "voice"] {
        let mut session = Session::new(1102);
        let (shot, target, mut random) = controlled_fixture(&mut session);
        session
            .residents
            .get_mut(&target.serial)
            .unwrap()
            .seed
            .storage
            .data_mut()
            .unwrap()[0x1fc..0x200]
            .copy_from_slice(&(-0.1f32).to_le_bytes());
        let mut services = Direct {
            source: Some(shot),
            receiver: Receiver {
                fail: Some(failure),
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(
            session
                .complete_enemy_projectile_object_contact(shot, &mut random, &mut services)
                .is_err()
        );
        assert_eq!(
            session.mecc_health(target).unwrap().damage,
            if failure == "gates" { 0. } else { 20. }
        );
        assert_eq!(
            session
                .controlled_mecc_motion(target, 1)
                .unwrap()
                .frame
                .velocity
                == [1., 2., 3.],
            failure == "gates"
        );
        assert_eq!(session.enemy_projectile(shot).unwrap().phase, Phase::Failed);
        assert_eq!(
            session.enemy_projectile(shot).unwrap().mecc_receiver,
            Some(crate::mecc_enemy_hit::Phase::Failed)
        );
        assert_eq!(
            session
                .get(shot)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap()
                .object_flags
                & 0x8000,
            0
        );
        assert_eq!(services.terminal_count, 0);
        let before = services.receiver.events.len();
        assert!(
            session
                .complete_enemy_projectile_object_contact(shot, &mut random, &mut services)
                .is_err()
        );
        assert_eq!(services.receiver.events.len(), before);
        assert_eq!(random.0, 17);
    }
}
#[test]
fn canonical_direct_successful_voice_keeps_separate_timer_publication() {
    let mut session = Session::new(1103);
    let (shot, target, mut random) = controlled_fixture(&mut session);
    session.mecc_countdown_world = Some(-0.1);
    session
        .residents
        .get_mut(&target.serial)
        .unwrap()
        .seed
        .storage
        .data_mut()
        .unwrap()[0x1fc..0x200]
        .copy_from_slice(&(-0.1f32).to_le_bytes());
    let mut services = Direct {
        source: Some(shot),
        ..Default::default()
    };
    session
        .complete_enemy_projectile_object_contact(shot, &mut random, &mut services)
        .unwrap();
    assert_eq!(
        services.receiver.events,
        ["gates", "recoil", "voice", "voice"]
    );
    assert_eq!(services.receiver.cues.len(), 2);
    assert_eq!(session.mecc_countdown_world(), Some(10.));
    let body = session.get(target).unwrap().seed.storage.data().unwrap();
    assert!(f32::from_le_bytes(body[0x1fc..0x200].try_into().unwrap()) >= 1.);
    assert_ne!(random.0, 17);
}

#[test]
fn normal_hit_clock_consumes_samples_once_and_keeps_shared_value_on_reentry() {
    use crate::controlled_mecc_hit_clock::Clock;
    let mut clock = Clock::default();
    assert_eq!(clock.saved(), 10.);
    let mut first = Session::new(1301);
    let (_, target, random) = controlled_fixture_with_clock(&mut first, None);
    assert!(clock.advance(&mut first, target, 1, 1, 0.25).unwrap());
    assert_eq!(first.mecc_countdown_world(), Some(9.75));
    assert!(!clock.advance(&mut first, target, 1, 1, 0.25).unwrap());
    let frame = first.controlled_mecc_motion(target, 1).unwrap().frame;
    first
        .publish_controlled_mecc_motion(
            target,
            2,
            Frame {
                before: frame.position,
                ..frame
            },
        )
        .unwrap();
    assert!(!clock.advance(&mut first, target, 2, 1, 0.25).unwrap());
    assert!(clock.advance(&mut first, target, 2, 2, 0.5).unwrap());
    assert_eq!(first.mecc_countdown_world(), Some(9.25));
    let body = first.get(target).unwrap().seed.storage.data().unwrap();
    assert_eq!(
        f32::from_le_bytes(body[0x1fc..0x200].try_into().unwrap()),
        -0.75
    );
    assert_eq!(random.0, 17);
    assert!(clock.release(&first, target.ticket()).unwrap());
    first.clear();
    let mut second = Session::new(1302);
    let (_, target2, _) = controlled_fixture_with_clock(&mut second, None);
    assert!(clock.advance(&mut second, target2, 1, 1, 0.125).unwrap());
    assert_eq!(second.mecc_countdown_world(), Some(9.125));
    assert!(clock.release(&second, target2.ticket()).unwrap());
    assert_eq!(clock.saved(), 9.125);
}

#[test]
fn normal_hit_clock_preserves_actual_post_cue_reset_and_refuses_foreign_owners() {
    use crate::controlled_mecc_hit_clock::Clock;
    let mut clock = Clock::default();
    let mut session = Session::new(1303);
    let (shot, target, mut random) = controlled_fixture_with_clock(&mut session, None);
    clock.advance(&mut session, target, 1, 1, 10.25).unwrap();
    let mut foreign = Session::new(1304);
    let (_, other, _) = controlled_fixture_with_clock(&mut foreign, None);
    assert!(clock.advance(&mut foreign, other, 1, 1, 0.25).is_err());
    assert!(foreign.mecc_countdown_world().is_none());
    assert!(clock.release(&session, other.ticket()).is_err());
    assert!(clock.release(&foreign, target.ticket()).is_err());
    let mut services = Direct {
        source: Some(shot),
        ..Default::default()
    };
    session
        .complete_enemy_projectile_object_contact(shot, &mut random, &mut services)
        .unwrap();
    assert_eq!(services.receiver.cues.len(), 2);
    assert_eq!(session.mecc_countdown_world(), Some(10.));
    assert!(clock.release(&session, target.ticket()).unwrap());
    assert_eq!(clock.saved(), 10.);
    assert!(clock.failure().is_none());
}

#[test]
fn normal_hit_clock_never_replaces_an_unowned_timer_and_latches_late_refusal() {
    use crate::controlled_mecc_hit_clock::Clock;
    let mut clock = Clock::default();
    let mut occupied = Session::new(1305);
    let (_, target, _) = controlled_fixture(&mut occupied);
    assert!(clock.advance(&mut occupied, target, 1, 1, 0.25).is_err());
    assert_eq!(occupied.mecc_countdown_world(), Some(0.));
    assert_eq!(clock.saved(), 10.);
    let mut session = Session::new(1306);
    let (_, target, _) = controlled_fixture_with_clock(&mut session, None);
    assert!(clock.advance(&mut session, target, 1, 2, 0.25).unwrap());
    assert!(clock.advance(&mut session, target, 1, 1, 0.25).is_err());
    assert!(clock.failure().is_some());
    assert_eq!(session.mecc_countdown_world(), Some(9.75));
    assert!(clock.advance(&mut session, target, 1, 3, 0.25).is_err());
    assert!(clock.release(&session, target.ticket()).unwrap());
    assert_eq!(clock.saved(), 9.75);
    assert!(clock.failure().is_some());
}

#[test]
#[ignore = "Requires GIANTS_MECC_HIT_CLOCK_CONTROLS; private original countdown fragment with two original global-clock calls"]
fn native_normal_hit_clock_prefix_controls() {
    let contents =
        std::fs::read_to_string(std::env::var("GIANTS_MECC_HIT_CLOCK_CONTROLS").unwrap()).unwrap();
    let mut count = 0;
    for line in contents
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
    {
        let words: Vec<u32> = line
            .split_whitespace()
            .map(|s| u32::from_str_radix(s, 16).unwrap())
            .collect();
        assert_eq!(words.len(), 5);
        let mut session = Session::new(1400 + count);
        let (_, target, _) =
            controlled_fixture_with_clock(&mut session, Some(f32::from_bits(words[0])));
        session
            .residents
            .get_mut(&target.serial)
            .unwrap()
            .seed
            .storage
            .data_mut()
            .unwrap()[0x1fc..0x200]
            .copy_from_slice(&words[1].to_le_bytes());
        assert!(
            session
                .advance_controlled_mecc_hit_clock(target, 1, 1, f32::from_bits(words[2]))
                .unwrap()
        );
        let body = session.get(target).unwrap().seed.storage.data().unwrap();
        assert_eq!(
            [
                session.mecc_countdown_world().unwrap().to_bits(),
                u32::from_le_bytes(body[0x1fc..0x200].try_into().unwrap())
            ],
            words[3..],
            "native clock row {count}"
        );
        count += 1;
    }
    assert_eq!(count, 36);
}

#[test]
#[ignore = "Requires GIANTS_ENEMY_OBJECT_CONTROLS; actual cold Session receiver and private original dispatcher observations"]
fn native_canonical_direct_dispatcher_controls() {
    let path = std::env::var("GIANTS_ENEMY_OBJECT_CONTROLS").expect("private controls path");
    let contents = std::fs::read_to_string(path).unwrap();
    let mut count = 0;
    for line in contents
        .lines()
        .filter(|s| !s.is_empty() && !s.starts_with('#'))
    {
        let words: Vec<u32> = line
            .split_whitespace()
            .map(|s| u32::from_str_radix(s, 16).unwrap())
            .collect();
        assert_eq!(words.len(), 19);
        let mut session = Session::new(1200 + count);
        let (shot, target, mut random) = controlled_fixture(&mut session);
        let f = session
            .residents
            .get_mut(&target.serial)
            .unwrap()
            .construction_fields
            .as_mut()
            .unwrap();
        if words[4] == 1 {
            f.configuration[15] |= 0x4000000;
        }
        if words[4] == 2 {
            f.configuration[14] |= 0x4000000;
        }
        let mut services = Direct {
            source: Some(shot),
            world_suppressed: words[0] != 0 || words[1] != 0,
            receiver: Receiver {
                disabled: words[2] != 0,
                suppressed: words[3] != 0,
                ..Default::default()
            },
            ..Default::default()
        };
        session
            .complete_enemy_projectile_object_contact(shot, &mut random, &mut services)
            .unwrap();
        let health = session.mecc_health(target).unwrap();
        let mut actual = vec![health.damage.to_bits(), health.flags];
        actual.extend(
            session
                .controlled_mecc_motion(target, 1)
                .unwrap()
                .frame
                .velocity
                .map(f32::to_bits),
        );
        let source = session.get(shot).unwrap();
        let f = source.construction_fields.as_ref().unwrap();
        actual.push(f.object_flags);
        actual.extend(f.position.map(f32::to_bits));
        actual.push(u32::from_le_bytes(
            source.seed.storage.data().unwrap()[4..8]
                .try_into()
                .unwrap(),
        ));
        let payload = session.get(target).unwrap().seed.storage.data().unwrap();
        actual.push(u32::from_le_bytes(payload[0xf4..0xf8].try_into().unwrap()));
        actual.push(random.0);
        actual.push(
            services
                .receiver
                .events
                .iter()
                .filter(|&&e| e == "gates")
                .count() as u32,
        );
        actual.push(services.terminal_count);
        assert_eq!(
            actual,
            words[5..],
            "canonical original dispatcher row {count}"
        );
        count += 1;
    }
    assert_eq!(count, 48);
}
