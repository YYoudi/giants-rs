// Supplied owned-frame boundary, source mode and services. These controls use
// the canonical public primary/receiver path; they establish no world admission.
fn canonical_melee_fixture(session: &mut Session) -> (Id, Id, Random) {
    let (_, target, random) = controlled_fixture(session);
    let source = fixture_actor(session, 230, 23);
    let actor = session.residents.get_mut(&source.serial).unwrap();
    let fields = actor.construction_fields.as_mut().unwrap();
    fields.position = [4., 6., 2.];
    fields.current = [1., 2., 1.];
    fields.dispatch_context = 100;
    fields.configuration[12] = 100f32.to_bits();
    fields.channels[0][0] = 10;
    fields.channels[0][8] = 0x22;
    actor.channels = Some(fields.channels);
    session.prepare_actor_motion(source).unwrap();
    session.begin_actor_motion_history(source, [0; 8]).unwrap();
    session
        .residents
        .get_mut(&source.serial)
        .unwrap()
        .finalization = Some(crate::resident_finalization::Owned {
        state: crate::actor_finalization::State {
            fields: crate::collider_copy::Fields::zeroed(),
            previous_physical_velocity: [0.; 3],
            friction: 0.,
            parent_present: false,
            footprint: None,
            affine: None,
            physical_argument_word: None,
        },
        phase: crate::resident_finalization::Phase::MotionStarted,
        failure: None,
    });
    session.prepare_ripper_primary(source).unwrap();
    let mut combat = session.state23_snapshot(source).unwrap();
    combat.mode = 68;
    combat.retaliation = Some(target);
    session
        .residents
        .get_mut(&source.serial)
        .unwrap()
        .state23_combat = Some(combat);
    (source, target, random)
}
#[derive(Default)]
struct CanonicalMeleeHost {
    world: bool,
    disabled: bool,
    suppressed: bool,
    fail: Option<&'static str>,
    events: Vec<&'static str>,
    local: Option<crate::controlled_mecc::HitState>,
    response: Option<crate::controlled_mecc::HitResponse>,
    replacement_ticket: Option<u64>,
}
fn melee_pose() -> crate::controlled_mecc::HitPose {
    crate::controlled_mecc::HitPose {
        position: [8., 10., 4.],
        angles: [0.; 3],
        scale: 1.5,
    }
}
impl crate::ripper_primary_turn::Services<Id, crate::attached_effect_context::Id>
    for CanonicalMeleeHost
{
    fn owner_sound(
        &mut self,
        _: &crate::ripper_primary_turn::State<Id, crate::attached_effect_context::Id>,
        _: u32,
        _: &mut Random,
    ) -> Result<crate::enemy_projectile::SoundOutcome, String> {
        Err("unexpected owner cue".into())
    }
    fn remove_effect(
        &mut self,
        _: &crate::ripper_primary_turn::State<Id, crate::attached_effect_context::Id>,
        _: crate::attached_effect_context::Id,
        _: &mut Random,
    ) -> Result<(), String> {
        Err("unexpected effect retirement".into())
    }
}
impl MeccEnemyHitServices for CanonicalMeleeHost {
    fn gates(&mut self, _: &Session, _: Id) -> Result<MeccEnemyHitGates, String> {
        self.events.push("gates");
        if self.fail == Some("gates") {
            return Err("supplied gate refusal".into());
        }
        Ok(MeccEnemyHitGates {
            globally_disabled: self.disabled,
            health_suppressed: self.suppressed,
        })
    }
    fn recoil(
        &mut self,
        session: &Session,
        response: crate::controlled_mecc::HitResponse,
    ) -> Result<(), String> {
        self.events.push("recoil");
        assert!(session.enemy_projectile(response.source()).is_none());
        self.response = Some(response);
        if self.fail == Some("recoil") {
            return Err("supplied actual controller refusal".into());
        }
        let local = self.local.ok_or("supplied controller state absent")?;
        assert!(
            crate::controlled_mecc::HitResponse {
                origin: crate::controlled_mecc::HitOrigin::Projectile,
                ..response
            }
            .validate_controller(session, response.owner(), melee_pose(), local)
            .is_err()
        );
        if let crate::controlled_mecc::HitOrigin::RipperMelee { epoch } = response.origin {
            assert!(
                crate::controlled_mecc::HitResponse {
                    origin: crate::controlled_mecc::HitOrigin::RipperMelee { epoch: epoch + 1 },
                    ..response
                }
                .validate_controller(session, response.owner(), melee_pose(), local)
                .is_err()
            );
        }
        let after = response.validate_controller(session, response.owner(), melee_pose(), local)?;
        assert!(
            response
                .validate_controller(session, response.owner(), melee_pose(), after)
                .is_err()
        );
        self.local = Some(after);
        Ok(())
    }
    fn voice(&mut self, _: &Session, _: Id, _: u32) -> Result<(), String> {
        self.events.push("voice");
        if self.fail == Some("voice") {
            Err("supplied cue refusal".into())
        } else {
            Ok(())
        }
    }
}
impl RipperMeleeServices for CanonicalMeleeHost {
    fn world_suppressed(&mut self, _: &Session) -> Result<bool, String> {
        self.events.push("world");
        if self.fail == Some("world") {
            Err("supplied world refusal".into())
        } else {
            Ok(self.world)
        }
    }
    fn terminal(&mut self, session: &mut Session, source: Id, target: Id) -> Result<(), String> {
        self.events.push("terminal");
        if self.fail == Some("cancel") {
            if let Some(ticket) = self.replacement_ticket {
                *session = Session::new(ticket);
                // Populate the same serial with a different actor and ticket.
                // The old primary must not be restored into this new scope.
                for _ in 0..source.serial {
                    fixture_actor(session, 79, 0);
                }
            } else {
                session.clear();
            }
            return Ok(());
        }
        assert!(session.ripper_melee_terminal_no_effect(source, target)?);
        if self.fail == Some("terminal") {
            Err("supplied terminal refusal".into())
        } else {
            Ok(())
        }
    }
}
fn run_canonical_melee(
    session: &mut Session,
    source: Id,
    target: Id,
    random: &mut Random,
    services: &mut CanonicalMeleeHost,
) -> Result<crate::ripper_primary_turn::Outcome, String> {
    let peers = session.perception_subset(&[target])?;
    let frame = peers.frame()?;
    session.update_ripper_variant_melee_with_mecc(
        source,
        crate::ripper_primary_turn::FrameInput {
            context: crate::state23_entry::Context {
                paused: false,
                native_mode: 0,
                parent_state: None,
                player: None,
                delta: 0.1,
            },
            frame: &frame,
            range: 500.,
        },
        crate::ripper_primary_turn::VariantMeleeContext {
            attack_range: 20.,
            damage: 35.,
        },
        1,
        random,
        services,
    )
}
fn local_melee_state(session: &Session, target: Id) -> crate::controlled_mecc::HitState {
    crate::controlled_mecc::HitState {
        health: session.mecc_health(target).unwrap(),
        velocity: session
            .controlled_mecc_motion(target, 1)
            .unwrap()
            .frame
            .velocity,
    }
}
fn remove_supplied_ai_aliases(session: &mut Session, source: Id) {
    // Isolate the operation's reference policy from the supplied AI aliases;
    // this is not an executed actor-retirement or gameplay transition.
    let actor = session.residents.get_mut(&source.serial).unwrap();
    actor.ripper_primary = None;
    actor.state23_combat.as_mut().unwrap().retaliation = None;
}
#[test]
fn canonical_kind3_copies_actual_health_velocity_once_and_finishes_generic_tail() {
    let mut session = Session::new(1801);
    let (source, target, mut random) = canonical_melee_fixture(&mut session);
    let mut host = CanonicalMeleeHost {
        local: Some(local_melee_state(&session, target)),
        ..Default::default()
    };
    run_canonical_melee(&mut session, source, target, &mut random, &mut host).unwrap();
    assert_eq!(host.events, ["world", "gates", "recoil", "terminal"]);
    assert_eq!(session.mecc_health(target).unwrap().damage, 35.);
    assert_eq!(session.mecc_health(target).unwrap().flags, 0);
    assert!(
        host.local
            .unwrap()
            .same_bits(local_melee_state(&session, target))
    );
    assert_eq!(session.state23_combat(source).unwrap().mode, 66);
    assert!(session.enemy_projectile(source).is_none());
    assert!(
        session
            .ripper_melee_terminal_no_effect(source, target)
            .is_err()
    );
    assert_eq!(
        session
            .get(source)
            .unwrap()
            .ripper_melee
            .as_ref()
            .unwrap()
            .phase,
        super::super::super::ripper_melee::Phase::Complete
    );
    let response = host.response.unwrap();
    assert!(
        response
            .validate_controller(&session, target, melee_pose(), response.before())
            .is_err()
    );
    assert!(run_canonical_melee(&mut session, source, target, &mut random, &mut host).is_err());
    assert_eq!(host.events.len(), 4);
    remove_supplied_ai_aliases(&mut session, source);
    assert!(
        !session
            .incoming_actor_references(target)
            .unwrap()
            .contains(&source)
    );
}

#[test]
#[ignore = "Requires GIANTS_NPC_ROOT; actual marker constructor with supplied mode68, frame and receiver guards, not ordinary combat acceptance"]
fn source_kind3_accepts_reciprocal_surface_marker_without_using_it_as_hit_owner() {
    let root = std::path::PathBuf::from(std::env::var_os("GIANTS_NPC_ROOT").unwrap());
    let resources =
        crate::scene_resources::SceneResources::load(&root, "w_Story2.bin", "English").unwrap();
    for broken in [false, true] {
        let mut session = Session::new(1840 + u64::from(broken));
        let (source, target, mut random) = canonical_melee_fixture(&mut session);
        let marker = crate::primary_surface_marker::Factory::new(&resources)
            .construct(&mut session, source)
            .unwrap();
        let links = (session.surface_link(source), session.surface_link(marker));
        if broken {
            session.residents.get_mut(&marker.serial).unwrap().surface_link = None;
        }
        let mut host = CanonicalMeleeHost {
            local: Some(local_melee_state(&session, target)),
            ..Default::default()
        };
        let result = run_canonical_melee(&mut session, source, target, &mut random, &mut host);
        if broken {
            assert!(result.is_err());
            assert!(host.events.is_empty());
            assert_eq!(session.mecc_health(target).unwrap().damage, 0.);
        } else {
            result.unwrap();
            assert_eq!(host.events, ["world", "gates", "recoil", "terminal"]);
            assert_eq!(session.mecc_health(target).unwrap().damage, 35.);
            assert_eq!(session.state23_combat(source).unwrap().owner, Some((marker, 219)));
            assert_eq!((session.surface_link(source), session.surface_link(marker)), links);
            assert_eq!(host.response.unwrap().source(), source);
        }
    }
}
#[test]
#[ignore = "Requires GIANTS_MECC_KIND3_CONTROLS; private original class230 kind3 dispatcher comparisons with supplied owned mode68/frame and all consumer gates; not ordinary gameplay"]
fn native_canonical_kind3_controls() {
    let contents = std::fs::read_to_string(
        std::env::var("GIANTS_MECC_KIND3_CONTROLS").expect("private kind3 controls path"),
    )
    .unwrap();
    let mut count = 0;
    for line in contents
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
    {
        let w: Vec<u32> = line
            .split_whitespace()
            .map(|s| u32::from_str_radix(s, 16).unwrap())
            .collect();
        assert_eq!(w.len(), 20);
        if w[19] != 230 {
            continue;
        }
        let mut session = Session::new(1840 + count);
        let (source, target, mut random) = canonical_melee_fixture(&mut session);
        let mut host = CanonicalMeleeHost {
            world: w[0] != 0 || w[1] != 0,
            disabled: w[2] != 0,
            suppressed: w[3] != 0,
            local: Some(local_melee_state(&session, target)),
            ..Default::default()
        };
        run_canonical_melee(&mut session, source, target, &mut random, &mut host).unwrap();
        let state = local_melee_state(&session, target);
        let mut actual = vec![state.health.damage.to_bits(), state.health.flags];
        actual.extend(state.velocity.map(f32::to_bits));
        assert_eq!(actual, w[5..10], "original canonical kind3 row {count}");
        let body = session.get(target).unwrap().seed.storage.data().unwrap();
        assert_eq!(
            (
                u32::from_le_bytes(body[0xf4..0xf8].try_into().unwrap()),
                random.0
            ),
            (w[15], w[16])
        );
        assert_eq!(
            host.events
                .iter()
                .filter(|&&event| event == "terminal")
                .count() as u32,
            w[18]
        );
        assert!(host.local.unwrap().same_bits(state));
        count += 1;
    }
    assert_eq!(count, 4);
}
#[test]
fn canonical_kind3_preserves_real_callback_world_and_immunity_gate_order() {
    for case in 0..5 {
        let mut session = Session::new(1810 + case);
        let (source, target, mut random) = canonical_melee_fixture(&mut session);
        let mut host = CanonicalMeleeHost {
            world: case == 0,
            disabled: case == 1,
            suppressed: case == 2,
            local: Some(local_melee_state(&session, target)),
            ..Default::default()
        };
        if case >= 3 {
            session
                .residents
                .get_mut(&target.serial)
                .unwrap()
                .construction_fields
                .as_mut()
                .unwrap()
                .configuration[if case == 3 { 14 } else { 15 }] |= 0x4000000;
        }
        run_canonical_melee(&mut session, source, target, &mut random, &mut host).unwrap();
        assert_eq!(session.mecc_health(target).unwrap().damage, 0.);
        assert_eq!(
            host.events,
            match case {
                0 | 4 => vec!["world"],
                1 => vec!["world", "gates", "terminal"],
                2 => vec!["world", "gates", "recoil", "terminal"],
                _ => vec!["world", "terminal"],
            }
        );
        assert_eq!(random.0, 17);
    }
}
#[test]
fn canonical_kind3_stationary_source_copies_health_even_without_impulse() {
    let mut session = Session::new(1819);
    let (source, target, mut random) = canonical_melee_fixture(&mut session);
    let fields = session
        .residents
        .get_mut(&source.serial)
        .unwrap()
        .construction_fields
        .as_mut()
        .unwrap();
    fields.current = fields.position;
    let before = local_melee_state(&session, target);
    let mut host = CanonicalMeleeHost {
        local: Some(before),
        ..Default::default()
    };
    run_canonical_melee(&mut session, source, target, &mut random, &mut host).unwrap();
    assert_eq!(host.events, ["world", "gates", "recoil", "terminal"]);
    let after = host.local.unwrap();
    assert_eq!(after.health.damage, 35.);
    assert_eq!(after.velocity, before.velocity);
    assert!(after.same_bits(local_melee_state(&session, target)));
    assert_eq!(random.0, 17);
}
#[test]
fn canonical_kind3_late_failures_retain_health_copy_and_refuse_replay() {
    for fail in ["world", "gates", "recoil", "voice", "terminal"] {
        let mut session = Session::new(1820);
        let (source, target, mut random) = canonical_melee_fixture(&mut session);
        if fail == "voice" {
            session
                .advance_controlled_mecc_hit_clock(target, 1, 1, 1.)
                .unwrap();
        }
        let before = local_melee_state(&session, target);
        let mut host = CanonicalMeleeHost {
            fail: Some(fail),
            local: Some(before),
            ..Default::default()
        };
        assert!(
            run_canonical_melee(&mut session, source, target, &mut random, &mut host).is_err(),
            "{fail}"
        );
        assert_eq!(
            session.mecc_health(target).unwrap().damage,
            if matches!(fail, "world" | "gates") {
                0.
            } else {
                35.
            }
        );
        assert_eq!(
            host.local.unwrap().health.damage,
            if matches!(fail, "world" | "gates" | "recoil") {
                0.
            } else {
                35.
            }
        );
        assert!(session.ripper_primary(source).unwrap().failure().is_some());
        let effects = (host.events.len(), random.0);
        assert!(run_canonical_melee(&mut session, source, target, &mut random, &mut host).is_err());
        assert_eq!((host.events.len(), random.0), effects);
        remove_supplied_ai_aliases(&mut session, source);
        assert!(
            session
                .incoming_actor_references(target)
                .unwrap()
                .contains(&source)
        );
    }
}
#[test]
fn canonical_kind3_scope_cancellation_cannot_restore_retired_source_or_response() {
    for replacement_ticket in [None, Some(1831)] {
        let mut session = Session::new(1830);
        let (source, target, mut random) = canonical_melee_fixture(&mut session);
        let mut host = CanonicalMeleeHost {
            fail: Some("cancel"),
            local: Some(local_melee_state(&session, target)),
            replacement_ticket,
            ..Default::default()
        };
        assert!(run_canonical_melee(&mut session, source, target, &mut random, &mut host).is_err());
        if let Some(ticket) = replacement_ticket {
            let foreign = session
                .get(Id {
                    ticket,
                    serial: source.serial,
                })
                .unwrap();
            assert_eq!(foreign.class, 79);
            assert!(foreign.ripper_primary.is_none());
            assert!(!foreign.ripper_primary_running);
        } else {
            assert_eq!(session.counts().residents, 0);
        }
        let response = host.response.unwrap();
        assert!(
            response
                .validate_controller(&session, target, melee_pose(), response.before())
                .is_err()
        );
    }
}
