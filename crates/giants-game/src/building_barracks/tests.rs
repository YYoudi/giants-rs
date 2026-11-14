use super::*;
#[derive(Default)]
struct InitialRecorder {
    calls: Vec<(u32, i32, i32, u32, u32)>,
    refuse: bool,
}
impl InitialSound<u32> for InitialRecorder {
    fn object_sound(&mut self, s: &State<u32>, cue: u32) -> Result<(), String> {
        self.calls
            .push((cue, s.selector, s.remaining, s.phase, s.damage.to_bits()));
        if self.refuse {
            Err("required object sound refused".into())
        } else {
            Ok(())
        }
    }
}
fn initial_state(selector: i32) -> State<u32> {
    State {
        position: [1., 2., 3.],
        target: None,
        phase: 0,
        selector,
        damage: 15.,
        limit: 500.,
        mark_flags: 2,
        remaining: 99,
    }
}
#[test]
fn health_frames_keep_wide_boundary_and_actual_zero_damage_thresholds() {
    let thresholds = crate::building_damage::Thresholds([75., 50., 15., 0.]);
    assert_eq!(health_frame(24., 100., thresholds).unwrap().fraction, 1.);
    assert_eq!(health_frame(25., 100., thresholds).unwrap().fraction, 0.75);
    assert_eq!(
        crate::building_damage::House {
            damage: 24.,
            limit: 100.
        }
        .animation_fraction(thresholds)
        .unwrap(),
        0.75
    );
    assert_eq!(
        health_frame(0., 0., thresholds).unwrap(),
        HealthFrame {
            fraction: 1.,
            terminal: false
        }
    );
    assert_eq!(
        health_frame(101., 100., thresholds).unwrap(),
        HealthFrame {
            fraction: 0.,
            terminal: true
        }
    );
    let modified = crate::building_damage::Thresholds([99.5, 50., 15., 0.]);
    assert_eq!(health_frame(0., 100., modified).unwrap().fraction, 0.995);
    let mut s = initial_state(0);
    s.limit = 100.;
    let mut channels = [[0; 9]; 4];
    initialize(
        &mut s,
        1060,
        InitialParameters {
            mini: 3,
            barracks: 7,
            health: modified,
        },
        false,
        &mut channels,
        &mut InitialRecorder::default(),
    )
    .unwrap();
    assert_eq!(channels[0][1], 0.995_f32.to_bits());
}
#[test]
fn initialization_remaps_active_variants_and_preserves_sound_before_stock_order() {
    for (class, active, stock, cue) in [
        (1060, 0, 3, 0x6bf),
        (686, 2, 7, 0x6bf),
        (1226, 4, 12, 0x4db),
    ] {
        for selector in 0..=5 {
            let mut s = initial_state(selector);
            let mut c = [[0; 9]; 4];
            let mut sound = InitialRecorder::default();
            initialize(
                &mut s,
                class,
                InitialParameters {
                    mini: 3,
                    barracks: 7,
                    health: crate::building_damage::Thresholds([75., 50., 15., 0.]),
                },
                false,
                &mut c,
                &mut sound,
            )
            .unwrap();
            let (expected_selector, expected_stock, expected_cue) = if matches!(selector, 0 | 2 | 4)
            {
                (active, stock, cue)
            } else {
                (selector, 0, if selector == 5 { 0x4db } else { 0x6bf })
            };
            assert_eq!(sound.calls, [(expected_cue, expected_selector, 99, 1, 0)]);
            assert_eq!(
                (s.selector, s.remaining, s.phase, s.damage),
                (expected_selector, expected_stock, 1, 0.)
            );
            assert_eq!((c[0][0], c[0][1], c[0][8]), (1, 1_f32.to_bits(), 0x184));
        }
    }
}
#[test]
fn initialization_suppression_preserves_bytes_and_sound_failure_retains_prefix() {
    let mut s = initial_state(2);
    let mut c = [[0x12345678; 9]; 4];
    let original = c;
    let mut sound = InitialRecorder {
        refuse: true,
        ..Default::default()
    };
    let stocks = InitialParameters {
        mini: 3,
        barracks: 7,
        health: crate::building_damage::Thresholds([75., 50., 15., 0.]),
    };
    assert!(!initialize(&mut s, 1060, stocks, true, &mut c, &mut sound).unwrap());
    assert_eq!(c, original);
    assert_eq!((s.phase, s.damage, s.remaining), (0, 15., 99));
    assert!(sound.calls.is_empty());
    assert!(initialize(&mut s, 1060, stocks, false, &mut c, &mut sound).is_err());
    assert_eq!((s.phase, s.damage, s.selector, s.remaining), (1, 0., 0, 99));
    assert_eq!(c[0][1], 1_f32.to_bits());
    assert!(initialize(&mut s, 1060, stocks, false, &mut c, &mut sound).is_err());
    assert_eq!(sound.calls.len(), 1);
}
fn candidate(position: [f32; 3], data: Data<u32>) -> Candidate<u32> {
    let reaper = matches!(data, Data::Reaperman { .. });
    Candidate {
        class: if reaper { 215 } else { 230 },
        state: if reaper { 51 } else { 23 },
        mode: 103,
        control_flags: 0,
        position,
        data,
    }
}
fn reaper(target: Option<u32>) -> Data<u32> {
    Data::Reaperman {
        opaque_0: 0,
        target,
        flags_1b8: 0,
        opaque_1f0: 0,
        opaque_228: 0,
    }
}
#[test]
fn horizontal_alerts_keep_distinct_edges_existing_targets_and_mode_gates() {
    let mut actors = [
        candidate([70., 0., 0.], Data::Ripper { target: None }),
        candidate([69., 0., 10000.], Data::Ripper { target: None }),
        candidate([69., 0., 0.], Data::Ripper { target: Some(42) }),
        candidate([70., 0., 10000.], reaper(Some(42))),
        candidate([1., 0., 0.], reaper(Some(42))),
    ];
    actors[4].mode = 102;
    alert_team_from_building([0.; 3], 70., 7, &mut actors).unwrap();
    alert_rippers([0.; 3], 70., 7, &mut actors).unwrap();
    assert!(matches!(actors[0].data, Data::Ripper { target: None }));
    assert!(matches!(actors[1].data, Data::Ripper { target: Some(7) }));
    assert!(matches!(actors[2].data, Data::Ripper { target: Some(42) }));
    assert!(matches!(
        actors[3].data,
        Data::Reaperman {
            target: Some(7),
            ..
        }
    ));
    assert_eq!(actors[3].control_flags, 0x21);
    assert!(matches!(
        actors[4].data,
        Data::Reaperman {
            target: Some(42),
            ..
        }
    ));
    assert_eq!(actors[4].control_flags, 0);
    actors[4].data = Data::Other;
    actors[1].data = Data::Ripper { target: None };
    assert!(alert_rippers([0.; 3], 70., 7, &mut actors).is_err());
    assert!(matches!(actors[1].data, Data::Ripper { target: None }));
}
#[derive(Default)]
struct Recorder {
    events: Vec<String>,
    fail_at_remaining: Option<i32>,
}
impl Services<u32> for Recorder {
    fn alert_team(&mut self, s: &State<u32>, _: f32) -> Result<(), String> {
        self.events.push(format!(
            "team:{}:{}:{}",
            s.phase,
            s.damage,
            s.target.unwrap()
        ));
        Ok(())
    }
    fn alert_rippers(&mut self, _: &State<u32>, _: f32) -> Result<(), String> {
        self.events.push("rippers".into());
        Ok(())
    }
    fn object_sound(&mut self, _: u32) -> Result<(), String> {
        self.events.push("hit sound".into());
        Ok(())
    }
    fn explosion(&mut self, kind: Explosion, _: [f32; 3]) -> Result<(), String> {
        self.events.push(kind.name().into());
        Ok(())
    }
    fn positional_sound(&mut self, _: u32, _: [f32; 3]) -> Result<(), String> {
        self.events.push("death sound".into());
        Ok(())
    }
    fn mission_trigger(&mut self, s: &State<u32>) -> Result<(), String> {
        self.events.push(format!("mission:{}", s.remaining));
        if self.fail_at_remaining == Some(s.remaining) {
            return Err("required mission consumer unavailable".into());
        }
        Ok(())
    }
    fn retire(&mut self) -> Result<(), String> {
        self.events.push("retire".into());
        Ok(())
    }
}
#[test]
fn lethal_hit_preserves_source_order_unclamped_damage_and_failed_prefix() {
    let mut state = State {
        position: [1., 2., 3.],
        target: Some(42),
        phase: 1,
        selector: 0,
        damage: 39.,
        limit: 40.,
        mark_flags: 2,
        remaining: 3,
    };
    let mut services = Recorder {
        fail_at_remaining: Some(2),
        ..Default::default()
    };
    assert!(hit(&mut state, 7., None, 7, 50., &mut services).is_err());
    assert_eq!(state.damage, 46.);
    assert_eq!(state.remaining, 2);
    assert_eq!(
        services.events,
        [
            "team:2:39:7",
            "rippers",
            "fx_minibarracks",
            "death sound",
            "mission:3",
            "mission:2"
        ]
    );
    let prefix = services.events.clone();
    assert!(!hit(&mut state, 7., None, 7, 50., &mut services).unwrap());
    assert_eq!(services.events, prefix);
}
