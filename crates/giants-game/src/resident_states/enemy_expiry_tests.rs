// Supplied expiry boundary, not a flight query, complete world or device.
fn expiry_fixture(session: &mut Session, class: i32) -> Id {
    let source = fixture_actor(session, if class == 90 { 73 } else { 230 }, 23);
    let shot = fixture_actor(session, class, 6);
    session
        .residents
        .get_mut(&shot.serial)
        .unwrap()
        .seed
        .storage
        .data_mut()
        .unwrap()[28..32]
        .copy_from_slice(&(-1f32).to_le_bytes());
    session
        .finish_enemy_projectile_fire(
            shot,
            fire_input(source),
            &mut crate::audio_parameters::Random(17),
            &mut fire_services(u32::from(class == 232)),
        )
        .unwrap();
    let actor = session.residents.get_mut(&shot.serial).unwrap();
    actor.construction_fields.as_mut().unwrap().position = [4., 5., 0.];
    actor.construction_fields.as_mut().unwrap().object_flags = 0;
    actor.seed.storage.data_mut().unwrap()[4..8].copy_from_slice(&(-0.25f32).to_le_bytes());
    let meta = actor.enemy_projectile.as_mut().unwrap();
    meta.pending_flight = Some(crate::enemy_projectile::FlightOutcome::Expired);
    meta.flight_revision = 1;
    meta.initial_environment = Some(crate::collider_init::Instance {
        fields: crate::collider_copy::Fields::zeroed(),
        state: 6,
        usage: [0; 2],
        footprint: 0,
    });
    shot
}

#[test]
fn enemy_expiry_orders_consumers_and_requires_its_own_retirement() {
    for class in [90, 232] {
        let mut session = Session::new(1901);
        let shot = expiry_fixture(&mut session, class);
        let mut services = fire_services(u32::from(class == 232));
        let mut random = crate::audio_parameters::Random(10000);
        session
            .complete_enemy_projectile_expiry(shot, &mut random, &mut services)
            .unwrap();
        assert_eq!(services.events, ["impact_sound", "impact_effect", "radial"]);
        let meta = session.enemy_projectile(shot).unwrap();
        assert!(meta.ground_impact.is_none() && meta.object_impact.is_none());
        assert!(!meta.expiry_impact.as_ref().unwrap().after_ground);
        assert!(meta.expiry_impact.as_ref().unwrap().radial);
        assert_eq!(
            session.get(shot).unwrap().seed.storage.data().unwrap()[4..8],
            (-0.25f32).to_le_bytes()
        );
        let mut retirement = RetirementMock::default();
        assert!(
            session
                .cleanup_enemy_projectile(shot, &mut retirement)
                .is_err()
        );
        assert!(
            session
                .cleanup_enemy_projectile_object_contact(shot, &mut retirement)
                .is_err()
        );
        assert_eq!(retirement.calls, 0);
        session
            .cleanup_enemy_projectile_expiry(shot, &mut retirement)
            .unwrap();
        assert_eq!(retirement.calls, 1);
        assert!(session.get(shot).is_none());
    }
}

#[test]
fn enemy_expiry_after_ground_keeps_both_tails_and_consumes_no_second_clock() {
    let mut session = Session::new(1902);
    let shot = expiry_fixture(&mut session, 232);
    let actor = session.residents.get_mut(&shot.serial).unwrap();
    actor.seed.storage.data_mut().unwrap()[4..8].copy_from_slice(&0f32.to_le_bytes());
    let meta = actor.enemy_projectile.as_mut().unwrap();
    meta.pending_flight = Some(crate::enemy_projectile::FlightOutcome::GroundContact {
        point: [4., 5., 0.],
    });
    meta.ground_flight_clock = Some(crate::enemy_projectile::GroundFlightClock {
        seconds: 0.25,
        areas: vec![],
    });
    let mut services = fire_services(1);
    let mut random = crate::audio_parameters::Random(10000);
    assert!(matches!(
        session
            .complete_enemy_projectile_ground_flight(shot, &mut random, &mut services)
            .unwrap(),
        crate::enemy_projectile::FlightOutcome::Expired
    ));
    session
        .complete_enemy_projectile_expiry(shot, &mut random, &mut services)
        .unwrap();
    assert_eq!(
        services.events,
        [
            "impact_effect",
            "radial",
            "impact_sound",
            "impact_sound",
            "impact_effect",
            "radial"
        ]
    );
    assert!(
        session
            .enemy_projectile(shot)
            .unwrap()
            .ground_impact
            .is_some()
    );
    assert!(
        session
            .enemy_projectile(shot)
            .unwrap()
            .expiry_impact
            .as_ref()
            .unwrap()
            .after_ground
    );
    assert_eq!(
        session.get(shot).unwrap().seed.storage.data().unwrap()[4..8],
        (-0.25f32).to_le_bytes()
    );
    assert!(
        session
            .cleanup_enemy_projectile(shot, &mut RetirementMock::default())
            .is_err()
    );
    session
        .cleanup_enemy_projectile_expiry(shot, &mut RetirementMock::default())
        .unwrap();
}

#[test]
fn enemy_expiry_retains_partial_rng_and_refuses_replay_or_scope_replacement() {
    for (stage, count) in [("impact_sound", 1), ("impact_effect", 3), ("radial", 6)] {
        let mut session = Session::new(1903);
        let shot = expiry_fixture(&mut session, 232);
        let mut services = fire_services(1);
        services.fail = Some(stage);
        let mut random = crate::audio_parameters::Random(10000);
        let mut expected = random;
        for _ in 0..count {
            expected.next_15();
        }
        assert!(
            session
                .complete_enemy_projectile_expiry(shot, &mut random, &mut services)
                .is_err()
        );
        assert_eq!(random.0, expected.0);
        assert_eq!(
            session.enemy_projectile(shot).unwrap().phase,
            crate::enemy_projectile::Phase::Failed
        );
        assert!(
            session
                .enemy_projectile(shot)
                .unwrap()
                .expiry_impact
                .is_none()
        );
        let before = services.events.len();
        assert!(
            session
                .complete_enemy_projectile_expiry(shot, &mut random, &mut services)
                .is_err()
        );
        assert_eq!(services.events.len(), before);
        assert_eq!(random.0, expected.0);
    }
    struct Replacement;
    impl crate::enemy_projectile::GroundImpactServices for Replacement {
        fn sound(
            &mut self,
            _: &mut Session,
            _: u32,
            _: [f32; 3],
        ) -> Result<crate::enemy_projectile::SoundOutcome, String> {
            Ok(crate::enemy_projectile::SoundOutcome::Submitted)
        }
        fn effect(
            &mut self,
            session: &mut Session,
            _: crate::enemy_projectile::ImpactEffect,
            _: &mut crate::audio_parameters::Random,
        ) -> Result<crate::enemy_projectile::EffectOutcome, String> {
            *session = Session::new(1903);
            expiry_fixture(session, 232);
            Ok(crate::enemy_projectile::EffectOutcome::Rejected)
        }
        fn radial(
            &mut self,
            _: &mut Session,
            _: crate::enemy_projectile::RadialRequest,
            _: &mut crate::audio_parameters::Random,
        ) -> Result<(), String> {
            panic!("removed scope must not reach radial")
        }
    }
    let mut session = Session::new(1903);
    let shot = expiry_fixture(&mut session, 232);
    assert!(
        session
            .complete_enemy_projectile_expiry(
                shot,
                &mut crate::audio_parameters::Random(17),
                &mut Replacement
            )
            .is_err()
    );
    let replacement = session.enemy_projectile(shot).unwrap();
    assert_eq!(replacement.phase, crate::enemy_projectile::Phase::Launched);
    assert!(replacement.failure.is_none() && replacement.expiry_attempt.is_none());
}

#[derive(Default)]
struct ExpiryControlServices {
    events: Vec<&'static str>,
    cue: Option<u32>,
}

#[test]
fn enemy_expiry_missing_cue_and_preflight_do_not_invent_work() {
    let mut session = Session::new(1905);
    let shot = expiry_fixture(&mut session, 232);
    let actor = session.residents.get_mut(&shot.serial).unwrap();
    actor.enemy_projectile.as_mut().unwrap().sound_words[1] = 0;
    actor.seed.storage.data_mut().unwrap()[28..32].copy_from_slice(&0f32.to_le_bytes());
    let mut random = crate::audio_parameters::Random(10000);
    let mut services = ExpiryControlServices::default();
    session
        .complete_enemy_projectile_expiry(shot, &mut random, &mut services)
        .unwrap();
    assert_eq!(random.0, 10000);
    assert_eq!(services.events, ["fx"]);
    let receipt = session
        .enemy_projectile(shot)
        .unwrap()
        .expiry_impact
        .as_ref()
        .unwrap();
    assert!(receipt.cue.is_none() && receipt.sound.is_none() && !receipt.radial);
    for invalid in 0..3 {
        let mut session = Session::new(1905);
        let shot = expiry_fixture(&mut session, 232);
        let meta = session
            .residents
            .get_mut(&shot.serial)
            .unwrap()
            .enemy_projectile
            .as_mut()
            .unwrap();
        match invalid {
            0 => meta.pending_flight = None,
            1 => meta.flight_revision = 0,
            _ => meta.initial_environment = None,
        }
        let mut services = ExpiryControlServices::default();
        assert!(
            session
                .complete_enemy_projectile_expiry(shot, &mut random, &mut services)
                .is_err()
        );
        assert!(services.events.is_empty());
        assert_eq!(random.0, 10000);
    }
}

#[test]
fn enemy_expiry_retirement_never_removes_or_poison_replaced_scope() {
    struct ReplaceAtRelease;
    impl crate::actor_retirement::Services for ReplaceAtRelease {
        fn release(&mut self, session: &mut Session, _: Id) -> Result<(), String> {
            *session = Session::new(1906);
            let shot = expiry_fixture(session, 232);
            session.complete_enemy_projectile_expiry(
                shot,
                &mut crate::audio_parameters::Random(17),
                &mut ExpiryControlServices::default(),
            )?;
            Ok(())
        }
    }
    let mut session = Session::new(1906);
    let shot = expiry_fixture(&mut session, 232);
    session
        .complete_enemy_projectile_expiry(
            shot,
            &mut crate::audio_parameters::Random(17),
            &mut ExpiryControlServices::default(),
        )
        .unwrap();
    assert!(
        session
            .cleanup_enemy_projectile_expiry(shot, &mut ReplaceAtRelease)
            .is_err()
    );
    let replacement = session.enemy_projectile(shot).unwrap();
    assert_eq!(replacement.phase, crate::enemy_projectile::Phase::Retiring);
    assert!(replacement.failure.is_none() && replacement.expiry_impact.is_some());
}
impl crate::enemy_projectile::GroundImpactServices for ExpiryControlServices {
    fn sound(
        &mut self,
        _: &mut Session,
        cue: u32,
        _: [f32; 3],
    ) -> Result<crate::enemy_projectile::SoundOutcome, String> {
        self.events.push("sound");
        self.cue = Some(cue);
        Ok(crate::enemy_projectile::SoundOutcome::ResourceUnavailable)
    }
    fn effect(
        &mut self,
        _: &mut Session,
        _: crate::enemy_projectile::ImpactEffect,
        _: &mut crate::audio_parameters::Random,
    ) -> Result<crate::enemy_projectile::EffectOutcome, String> {
        self.events.push("fx");
        Ok(crate::enemy_projectile::EffectOutcome::Rejected)
    }
    fn radial(
        &mut self,
        session: &mut Session,
        request: crate::enemy_projectile::RadialRequest,
        _: &mut crate::audio_parameters::Random,
    ) -> Result<(), String> {
        self.events.push("radial");
        assert!(session.enemy_projectile_radial_no_effect_invariant(&request, Some(100.))?);
        Ok(())
    }
}

#[test]
#[ignore = "requires private identity-bound original expiry-tail controls"]
fn enemy_expiry_original_tail_controls() {
    let rows = std::fs::read_to_string(
        std::env::var("GIANTS_ENEMY_EXPIRY_CONTROLS").expect("private controls path"),
    )
    .unwrap();
    let mut count = 0;
    for row in rows.lines() {
        let w: Vec<u32> = row
            .split_whitespace()
            .map(|v| u32::from_str_radix(v, 16).unwrap())
            .collect();
        assert_eq!(w.len(), 12);
        let mut session = Session::new(1904);
        let shot = expiry_fixture(&mut session, w[0] as i32);
        if w[2] != 0 {
            session
                .residents
                .get_mut(&shot.serial)
                .unwrap()
                .enemy_projectile
                .as_mut()
                .unwrap()
                .pending_flight = None;
            session
                .enemy_projectile_ground_impact(
                    shot,
                    [4., 5., 0.],
                    &mut crate::audio_parameters::Random(17),
                    &mut fire_services(w[1]),
                )
                .unwrap();
            session
                .residents
                .get_mut(&shot.serial)
                .unwrap()
                .enemy_projectile
                .as_mut()
                .unwrap()
                .pending_flight = Some(crate::enemy_projectile::FlightOutcome::Expired);
        }
        let actor = session.residents.get_mut(&shot.serial).unwrap();
        actor.construction_fields.as_mut().unwrap().configuration[12] = w[5];
        actor.seed.storage.data_mut().unwrap()[28..32].copy_from_slice(&w[4].to_le_bytes());
        actor.enemy_projectile.as_mut().unwrap().sound_words[1..3].copy_from_slice(&w[6..8]);
        let mut random = crate::audio_parameters::Random(w[3]);
        let mut services = ExpiryControlServices::default();
        session
            .complete_enemy_projectile_expiry(shot, &mut random, &mut services)
            .unwrap();
        let receipt = session
            .enemy_projectile(shot)
            .unwrap()
            .expiry_impact
            .as_ref()
            .unwrap();
        assert_eq!(
            [
                session
                    .get(shot)
                    .unwrap()
                    .construction_fields
                    .as_ref()
                    .unwrap()
                    .object_flags,
                random.0,
                services.cue.unwrap_or(0),
                u32::from(receipt.radial)
            ],
            w[8..12]
        );
        assert_eq!(
            services.events,
            if receipt.radial {
                vec!["sound", "fx", "radial"]
            } else {
                vec!["sound", "fx"]
            }
        );
        count += 1;
    }
    assert!(count >= 64);
}
