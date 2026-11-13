// Synthetic callback boundaries, not ordinary projectile or gameplay proof.
fn damage_fixture(session: &mut Session) -> Id {
    let id = fixture_actor(session, 73, 23);
    session
        .residents
        .get_mut(&id.serial)
        .unwrap()
        .construction_fields
        .as_mut()
        .unwrap()
        .configuration[12] = 100f32.to_bits();
    id
}
fn damage_context() -> crate::state23_damage::Context {
    crate::state23_damage::Context {
        player_is_state1: true,
        drop_percent: 0,
        death_timer: 4.,
    }
}
#[derive(Default)]
struct DamageAccept(usize);
impl State23Services for DamageAccept {
    fn execute(
        &mut self,
        session: &mut Session,
        actor: &crate::state23_damage::State<Id>,
        _: crate::state23_damage::Service<Id>,
        _: &mut crate::audio_parameters::Random,
    ) -> Result<(), String> {
        self.0 += 1;
        assert_eq!(session.state23_combat(actor.id), Some(actor));
        Ok(())
    }
}
#[derive(Default)]
struct DamageProbe {
    calls: usize,
    fail_play: bool,
    prefix_visible: bool,
}
impl State23Services for DamageProbe {
    fn execute(
        &mut self,
        session: &mut Session,
        actor: &crate::state23_damage::State<Id>,
        service: crate::state23_damage::Service<Id>,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<(), String> {
        self.calls += 1;
        self.prefix_visible = session.state23_combat(actor.id).is_some_and(|current| {
            current.action == actor.action
                && current.mode == actor.mode
                && current.damage.to_bits() == actor.damage.to_bits()
        });
        if !self.fail_play || matches!(service, crate::state23_damage::Service::PlayAttached { .. })
        {
            random.next_15();
            return Err("supplied external service failed after its draw".into());
        }
        Ok(())
    }
}

#[test]
fn owned_ripper_damage_retains_prefix_and_refuses_replay_after_early_failure() {
    let mut session = Session::new(719);
    let id = damage_fixture(&mut session);
    session
        .residents
        .get_mut(&id.serial)
        .unwrap()
        .construction_fields
        .as_mut()
        .unwrap()
        .action = 1;
    let mut random = crate::audio_parameters::Random(17);
    let mut service = DamageProbe::default();
    assert!(
        session
            .damage_state23(id, 5., None, damage_context(), &mut random, &mut service)
            .is_err()
    );
    let state = session
        .state23_combat(id)
        .expect("actual prefix must survive failure");
    assert_eq!((state.action, state.mode, state.damage), (0, 33, 0.));
    assert!(service.prefix_visible);
    assert_eq!(
        session
            .get(id)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap()
            .action,
        0
    );
    let seed = random.0;
    assert!(
        session
            .damage_state23(id, 5., None, damage_context(), &mut random, &mut service)
            .is_err()
    );
    assert_eq!((service.calls, random.0), (1, seed));
    assert!(session.state23_damage_failure(id).is_some());
    assert!(session.ordinary_frame_actor(id).is_err());
}

#[test]
fn owned_ripper_damage_keeps_health_and_rng_when_last_cue_fails() {
    let mut session = Session::new(720);
    let id = damage_fixture(&mut session);
    let mut random = crate::audio_parameters::Random(17);
    let mut service = DamageProbe {
        fail_play: true,
        ..Default::default()
    };
    assert!(
        session
            .damage_state23(id, 5., None, damage_context(), &mut random, &mut service)
            .is_err()
    );
    assert_eq!(session.state23_combat(id).unwrap().damage, 5.);
    assert!(service.prefix_visible);
    let seed = random.0;
    let calls = service.calls;
    assert!(
        session
            .damage_state23(id, 5., None, damage_context(), &mut random, &mut service)
            .is_err()
    );
    assert_eq!((service.calls, random.0), (calls, seed));
}

#[test]
fn owned_ripper_damage_cannot_publish_into_a_replaced_session_with_colliding_ids() {
    struct Replace {
        calls: usize,
        id: Option<Id>,
    }
    impl State23Services for Replace {
        fn execute(
            &mut self,
            session: &mut Session,
            _: &crate::state23_damage::State<Id>,
            _: crate::state23_damage::Service<Id>,
            _: &mut crate::audio_parameters::Random,
        ) -> Result<(), String> {
            self.calls += 1;
            if self.calls == 1 {
                *session = Session::new(session.ticket);
                let id = damage_fixture(session);
                session
                    .residents
                    .get_mut(&id.serial)
                    .unwrap()
                    .construction_fields
                    .as_mut()
                    .unwrap()
                    .action = 19;
                self.id = Some(id);
            }
            Ok(())
        }
    }
    let mut session = Session::new(721);
    let id = damage_fixture(&mut session);
    let mut service = Replace { calls: 0, id: None };
    assert!(
        session
            .damage_state23(
                id,
                5.,
                None,
                damage_context(),
                &mut crate::audio_parameters::Random(17),
                &mut service
            )
            .is_err()
    );
    assert_eq!(service.id, Some(id));
    assert_eq!(service.calls, 1);
    assert!(session.state23_combat(id).is_none());
    assert!(session.state23_damage_failure(id).is_none());
    assert_eq!(
        session
            .get(id)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap()
            .action,
        19
    );
    session
        .damage_state23(
            id,
            5.,
            None,
            damage_context(),
            &mut crate::audio_parameters::Random(17),
            &mut DamageAccept::default(),
        )
        .unwrap();
    assert_eq!(session.state23_combat(id).unwrap().damage, 5.);
}

#[test]
fn owned_ripper_damage_class230_preserves_its_distinct_action1_prefix() {
    let mut session = Session::new(725);
    let id = fixture_actor(&mut session, 230, 23);
    let fields = session
        .residents
        .get_mut(&id.serial)
        .unwrap()
        .construction_fields
        .as_mut()
        .unwrap();
    fields.configuration[12] = 100f32.to_bits();
    fields.action = 1;
    let mut service = DamageProbe::default();
    assert!(
        session
            .damage_state23(
                id,
                5.,
                None,
                damage_context(),
                &mut crate::audio_parameters::Random(17),
                &mut service
            )
            .is_err()
    );
    let state = session.state23_combat(id).unwrap();
    assert_eq!((state.action, state.mode, state.damage), (0, 64, 0.));
    assert!(service.prefix_visible);
    assert!(session.state23_damage_failure(id).is_some());
}
#[test]
fn owned_ripper_damage_success_rearms_and_invalid_preflight_does_not_quarantine() {
    let mut session = Session::new(722);
    let id = damage_fixture(&mut session);
    let mut service = DamageAccept::default();
    let mut random = crate::audio_parameters::Random(17);
    assert!(
        session
            .damage_state23(
                id,
                f32::NAN,
                None,
                damage_context(),
                &mut random,
                &mut service
            )
            .is_err()
    );
    assert_eq!((service.0, random.0), (0, 17));
    assert!(session.state23_damage_failure(id).is_none());
    assert!(session.state23_combat(id).is_none());
    for expected in [5., 10.] {
        assert_eq!(
            session
                .damage_state23(id, 5., None, damage_context(), &mut random, &mut service)
                .unwrap(),
            crate::state23_damage::Outcome::Hurt
        );
        assert_eq!(session.state23_combat(id).unwrap().damage, expected);
        assert!(session.state23_damage_failure(id).is_none());
    }
    assert_eq!(service.0, 14);
    assert!(session.ordinary_frame_actor(id).is_ok());
}

#[test]
fn owned_ripper_damage_refuses_reentry_without_consuming_another_callback() {
    struct Reenter {
        inner: DamageAccept,
        refused: bool,
    }
    impl State23Services for Reenter {
        fn execute(
            &mut self,
            session: &mut Session,
            actor: &crate::state23_damage::State<Id>,
            _: crate::state23_damage::Service<Id>,
            random: &mut crate::audio_parameters::Random,
        ) -> Result<(), String> {
            self.refused = session
                .damage_state23(
                    actor.id,
                    1.,
                    None,
                    damage_context(),
                    random,
                    &mut self.inner,
                )
                .is_err();
            Err("supplied outer service failure".into())
        }
    }
    let mut session = Session::new(723);
    let id = damage_fixture(&mut session);
    let mut random = crate::audio_parameters::Random(17);
    let mut service = Reenter {
        inner: DamageAccept::default(),
        refused: false,
    };
    assert!(
        session
            .damage_state23(id, 5., None, damage_context(), &mut random, &mut service)
            .is_err()
    );
    assert!(service.refused);
    assert_eq!((service.inner.0, random.0), (0, 17));
    assert_eq!(session.state23_combat(id).unwrap().damage, 0.);
}

#[test]
fn owned_ripper_damage_lethal_effect_failure_cannot_invent_the_later_death_mode() {
    struct MissingEffect(usize);
    impl State23Services for MissingEffect {
        fn execute(
            &mut self,
            _: &mut Session,
            _: &crate::state23_damage::State<Id>,
            request: crate::state23_damage::Service<Id>,
            random: &mut crate::audio_parameters::Random,
        ) -> Result<(), String> {
            self.0 += 1;
            if matches!(request, crate::state23_damage::Service::Effect { .. }) {
                random.next_15();
                return Err("actual lethal effect consumer unavailable".into());
            }
            Ok(())
        }
    }
    let mut session = Session::new(724);
    let id = damage_fixture(&mut session);
    let mut random = crate::audio_parameters::Random(17);
    let mut service = MissingEffect(0);
    assert!(
        session
            .damage_state23(id, 100., None, damage_context(), &mut random, &mut service)
            .is_err()
    );
    let state = session.state23_combat(id).unwrap();
    assert_eq!(
        (state.damage, state.mode, state.flags, state.timer),
        (100., 0, 0, 0.)
    );
    let seed = random.0;
    assert!(
        session
            .damage_state23(id, 100., None, damage_context(), &mut random, &mut service)
            .is_err()
    );
    assert_eq!((service.0, random.0), (6, seed));
    assert!(session.state23_damage_failure(id).is_some());
}
